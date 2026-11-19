//! Daemon-mediated policy approval.
//!
//! The daemon holds an operation at its confirmation point, exposes the
//! decision, accepts the client's answer, and dispatches the held operation.
//! These tests pin that behaviour end to end at the daemon's command
//! surface, and pin the two refusals that must survive it: an answer for a
//! task with nothing pending, and an answer from a client without the
//! approver role.
//!
//! The executor here is a deliberate stand-in: it holds at the confirmation
//! point through the runtime's approval gate, exactly as the engine's
//! executor does, without depending on the engine or on the network.

use std::sync::Arc;

use eggsec_daemon::config::DaemonConfig;
use eggsec_daemon::host::DaemonHost;
use eggsec_daemon::store::{DaemonStore, PersistedAuditEvent};
use eggsec_daemon_protocol::client_registry::ClientKind;
use eggsec_daemon_protocol::protocol::{
    ClientCommand, DaemonRequestContext, ErrorCode, ServerMessage, TransportKind,
};
use eggsec_runtime::{
    ApprovalAnswer, ClientId, PolicyPrompt, RunRequest, RuntimeEvent, RuntimeEventSink,
    RuntimeExecutionContext, RuntimeSurface, SessionId, TaskCapability, TaskId, TaskKind,
    TaskOutcome,
};
use tokio_util::sync::CancellationToken;

/// An executor that holds every task at its confirmation point.
///
/// It stands in for the engine's executor: it registers a pending decision
/// through the runtime's approval gate, announces it, and waits. The task
/// only proceeds once a client answers.
struct HoldingExecutor;

impl eggsec_runtime::RuntimeTaskExecutor for HoldingExecutor {
    /// Declares one kind, so the daemon publishes and admits it.
    ///
    /// A rejecting executor MUST declare the empty set explicitly: the trait
    /// default is the canonical list, so "says nothing" would mean "declares
    /// everything".
    fn supported_task_kinds(&self) -> Vec<TaskCapability> {
        vec![TaskCapability::new("port-scan", "Port scanning")]
    }

    fn execute(
        &self,
        task_id: TaskId,
        _request: RunRequest,
        context: RuntimeExecutionContext,
        sink: RuntimeEventSink,
        cancel: CancellationToken,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<TaskOutcome, eggsec_runtime::RuntimeError>>
                + Send
                + 'static,
        >,
    > {
        Box::pin(async move {
            let prompt = PolicyPrompt {
                message: "port-scan needs an operator's confirmation".into(),
                confirmation_class: Some("out-of-scope".into()),
                requires_explicit_approval: true,
            };
            let answer = context
                .approval
                .request(&sink, task_id, prompt, cancel)
                .await?;
            match answer {
                ApprovalAnswer::Approved { .. } => {
                    Ok(TaskOutcome::Text("the held operation ran".into()))
                }
                ApprovalAnswer::Denied { reason } => {
                    Err(eggsec_runtime::RuntimeError::DispatchFailed(format!(
                        "the operator declined the policy confirmation: {}",
                        reason.unwrap_or_else(|| "no reason given".into())
                    )))
                }
            }
        })
    }
}

/// A store that keeps the audit events written to it.
#[derive(Default)]
struct AuditStore {
    events: std::sync::Mutex<Vec<PersistedAuditEvent>>,
}

#[async_trait::async_trait]
impl DaemonStore for AuditStore {
    async fn save_session_snapshot(
        &self,
        _snapshot: &eggsec_runtime::SessionSnapshot,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_session_snapshot(
        &self,
        _session_id: SessionId,
    ) -> anyhow::Result<Option<eggsec_runtime::SessionSnapshot>> {
        Ok(None)
    }

    async fn load_all_sessions(&self) -> anyhow::Result<Vec<eggsec_runtime::SessionSnapshot>> {
        Ok(Vec::new())
    }

    async fn record_audit_event(&self, event: &PersistedAuditEvent) -> anyhow::Result<()> {
        let mut events = self
            .events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        events.push(event.clone());
        Ok(())
    }

    async fn delete_session(&self, _session_id: SessionId) -> anyhow::Result<()> {
        Ok(())
    }

    fn blocking_list_sessions(&self) -> anyhow::Result<Vec<eggsec_runtime::SessionSummary>> {
        Ok(Vec::new())
    }

    fn blocking_get_snapshot(
        &self,
        _session_id: &SessionId,
    ) -> anyhow::Result<Option<eggsec_runtime::SessionSnapshot>> {
        Ok(None)
    }
}

/// A host with a holding executor and an audit store.
fn host_with_store() -> (DaemonHost, Arc<AuditStore>) {
    let store = Arc::new(AuditStore::default());
    let host = DaemonHost::new(
        DaemonConfig::default(),
        HoldingExecutor,
        store.clone() as Arc<dyn DaemonStore>,
    );
    (host, store)
}

/// The context a declared client sends commands with.
fn ctx(client_id: Option<ClientId>) -> DaemonRequestContext {
    DaemonRequestContext {
        client_id,
        peer: None,
        transport: TransportKind::UnixSocket,
    }
}

/// Declare a client and create a manual session, returning both ids.
async fn declare_and_create(host: &DaemonHost) -> (ClientId, SessionId) {
    let declared = host
        .handle_command(
            ClientCommand::DeclareClient {
                request_id: "declare".into(),
                kind: ClientKind::Gui,
                label: Some("approval-test".into()),
            },
            ctx(None),
        )
        .await;
    let client_id = match declared {
        ServerMessage::ClientDeclared { client_id, .. } => client_id,
        other => panic!("the client must be declared, got {other:?}"),
    };

    let created = host
        .handle_command(
            ClientCommand::CreateSession {
                request_id: "create".into(),
                surface: RuntimeSurface::DesktopGuiManual,
                scope: None,
                labels: vec![],
            },
            ctx(Some(client_id)),
        )
        .await;
    let session_id = match created {
        ServerMessage::SessionCreated { session_id, .. } => session_id,
        other => panic!("the session must be created, got {other:?}"),
    };
    (client_id, session_id)
}

/// A port-scan request.
fn request() -> RunRequest {
    RunRequest {
        task_kind: TaskKind::PortScan(eggsec_runtime::request::PortScanParams {
            target: "127.0.0.1".into(),
            ..Default::default()
        }),
        requested_by: None,
        surface: RuntimeSurface::DesktopGuiManual,
        labels: vec![],
    }
}

/// Wait for the daemon's runtime to announce a pending decision.
async fn await_prompt(host: &DaemonHost, session_id: SessionId) -> (TaskId, PolicyPrompt) {
    let mut events = host.runtime().subscribe().await;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let event = tokio::time::timeout_at(deadline, events.recv())
            .await
            .expect("the prompt must be announced in time")
            .expect("the event stream must stay open");
        if let RuntimeEvent::PolicyDecisionRequired {
            session_id: announced_session,
            task_id: Some(task_id),
            prompt,
        } = event
        {
            assert_eq!(
                announced_session, session_id,
                "the prompt must belong to the session that is waiting"
            );
            return (task_id, prompt);
        }
    }
}

/// The end-to-end proof: a manual client answers a pending decision and the
/// held operation then runs.
#[tokio::test]
async fn a_held_operation_resumes_when_the_client_answers() {
    let (host, _store) = host_with_store();
    let (client_id, session_id) = declare_and_create(&host).await;

    let submitted = host
        .handle_command(
            ClientCommand::SubmitTask {
                request_id: "submit".into(),
                session_id,
                request: request(),
            },
            ctx(Some(client_id)),
        )
        .await;
    let task_id = match submitted {
        ServerMessage::TaskSubmitted { task_id, .. } => task_id,
        other => panic!("the task must be admitted, got {other:?}"),
    };

    // The operation is held, and the decision is exposed.
    let (_announced_task, prompt) = await_prompt(&host, session_id).await;
    assert!(
        prompt.message.contains("port-scan"),
        "the prompt must name the operation: {}",
        prompt.message
    );
    assert!(prompt.requires_explicit_approval);
    assert!(
        host.runtime()
            .pending_approval(session_id, task_id)
            .is_some(),
        "the daemon must expose the pending decision"
    );

    // The client answers.
    let answered = host
        .handle_command(
            ClientCommand::ApprovePolicy {
                request_id: "approve".into(),
                session_id,
                task_id,
                approved: true,
                reason: Some("the target is a lab host".into()),
            },
            ctx(Some(client_id)),
        )
        .await;
    assert!(
        matches!(answered, ServerMessage::Ok { .. }),
        "the answer must be accepted, got {answered:?}"
    );

    // The held operation runs.
    let mut events = host.runtime().subscribe().await;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let event = tokio::time::timeout_at(deadline, events.recv())
            .await
            .expect("the operation must finish in time")
            .expect("the event stream must stay open");
        match event {
            RuntimeEvent::TaskCompleted { outcome, .. } => {
                assert!(
                    matches!(&outcome, TaskOutcome::Text(text) if text == "the held operation ran"),
                    "the held operation must have run: {outcome:?}"
                );
                break;
            }
            RuntimeEvent::TaskFailed { error, .. } => {
                panic!("the held operation must not fail: {}", error.message)
            }
            _ => continue,
        }
    }
}

/// A refused operation stays refused: the operator's "no" fails the task.
#[tokio::test]
async fn a_refused_operation_stays_refused() {
    let (host, _store) = host_with_store();
    let (client_id, session_id) = declare_and_create(&host).await;

    host.handle_command(
        ClientCommand::SubmitTask {
            request_id: "submit".into(),
            session_id,
            request: request(),
        },
        ctx(Some(client_id)),
    )
    .await;
    let (task_id, _prompt) = await_prompt(&host, session_id).await;

    let answered = host
        .handle_command(
            ClientCommand::ApprovePolicy {
                request_id: "deny".into(),
                session_id,
                task_id,
                approved: false,
                reason: Some("not this time".into()),
            },
            ctx(Some(client_id)),
        )
        .await;
    assert!(
        matches!(answered, ServerMessage::Ok { .. }),
        "the refusal must be accepted, got {answered:?}"
    );

    let mut events = host.runtime().subscribe().await;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let event = tokio::time::timeout_at(deadline, events.recv())
            .await
            .expect("the task must finish in time")
            .expect("the event stream must stay open");
        match event {
            RuntimeEvent::TaskFailed { error, .. } => {
                assert!(
                    error.message.contains("not this time"),
                    "the failure must carry the operator's reason: {}",
                    error.message
                );
                break;
            }
            RuntimeEvent::TaskCompleted { .. } => panic!("a refused operation must not run"),
            _ => continue,
        }
    }
}

/// Answering a task with nothing pending is refused as invalid state.
#[tokio::test]
async fn answering_a_task_with_nothing_pending_is_refused() {
    let (host, _store) = host_with_store();
    let (client_id, session_id) = declare_and_create(&host).await;

    let answered = host
        .handle_command(
            ClientCommand::ApprovePolicy {
                request_id: "approve".into(),
                session_id,
                task_id: TaskId::new(),
                approved: true,
                reason: None,
            },
            ctx(Some(client_id)),
        )
        .await;
    match answered {
        ServerMessage::Error { code, message, .. } => {
            assert_eq!(code, ErrorCode::InvalidState);
            assert!(
                message.contains("no pending policy decision"),
                "the refusal must say what is wrong: {message}"
            );
        }
        other => panic!("the answer must be refused, got {other:?}"),
    }
}

/// The answer is audited: who answered, for which session, on which surface,
/// and with what outcome.
#[tokio::test]
async fn the_answer_is_audited_with_the_answering_client() {
    let (host, store) = host_with_store();
    let (client_id, session_id) = declare_and_create(&host).await;

    host.handle_command(
        ClientCommand::SubmitTask {
            request_id: "submit".into(),
            session_id,
            request: request(),
        },
        ctx(Some(client_id)),
    )
    .await;
    let (task_id, _prompt) = await_prompt(&host, session_id).await;

    host.handle_command(
        ClientCommand::ApprovePolicy {
            request_id: "approve".into(),
            session_id,
            task_id,
            approved: true,
            reason: Some("the target is a lab host".into()),
        },
        ctx(Some(client_id)),
    )
    .await;

    let events = store
        .events
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let approval = events
        .iter()
        .find(|event| event.action == "approve-policy")
        .expect("the answer must be audited");

    assert_eq!(approval.outcome, "allow");
    assert_eq!(
        approval.client_id.as_deref(),
        Some(client_id.to_string().as_str()),
        "the audit must name the answering client"
    );
    assert_eq!(
        approval.session_id.as_deref(),
        Some(session_id.to_string().as_str()),
        "the audit must name the session"
    );
    assert_eq!(
        approval.surface, "desktop-gui-manual",
        "the audit must carry the session's real surface"
    );
}

/// A client without the approver role cannot answer.
#[tokio::test]
async fn a_client_without_the_approver_role_cannot_answer() {
    let (host, _store) = host_with_store();
    let (client_id, session_id) = declare_and_create(&host).await;

    host.handle_command(
        ClientCommand::SubmitTask {
            request_id: "submit".into(),
            session_id,
            request: request(),
        },
        ctx(Some(client_id)),
    )
    .await;
    let (task_id, _prompt) = await_prompt(&host, session_id).await;

    // A second client declares itself and tries to answer a decision that is
    // not its own.
    let declared = host
        .handle_command(
            ClientCommand::DeclareClient {
                request_id: "declare-other".into(),
                kind: ClientKind::Gui,
                label: Some("other-client".into()),
            },
            ctx(None),
        )
        .await;
    let other_client = match declared {
        ServerMessage::ClientDeclared { client_id, .. } => client_id,
        other => panic!("the client must be declared, got {other:?}"),
    };

    let answered = host
        .handle_command(
            ClientCommand::ApprovePolicy {
                request_id: "approve".into(),
                session_id,
                task_id,
                approved: true,
                reason: None,
            },
            ctx(Some(other_client)),
        )
        .await;
    match answered {
        ServerMessage::Error { code, .. } => {
            assert_eq!(code, ErrorCode::PermissionDenied);
        }
        other => panic!("the answer must be refused, got {other:?}"),
    }

    // The decision is still pending: the refused answer changed nothing.
    assert!(
        host.runtime()
            .pending_approval(session_id, task_id)
            .is_some(),
        "a refused answer must not resolve the decision"
    );
}

/// The client's own path, end to end against a live daemon over the real
/// Unix socket: the operator sees the prompt, answers it through the
/// frontend-neutral contract, and the held operation runs.
#[tokio::test]
async fn the_client_answers_a_pending_decision_over_the_socket() {
    use eggsec_frontend_client::{ClientIdentity, DaemonRuntimeClient, RuntimeClient};

    // A Unix socket path is bounded by `sun_path`, so the path stays short.
    let socket_path = format!(
        "/tmp/eggsec-approval-{}-{}.sock",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let host = Arc::new(DaemonHost::new(
        DaemonConfig {
            socket_path: socket_path.clone(),
            enable_persistence: false,
            ..Default::default()
        },
        HoldingExecutor,
        eggsec_daemon::store::noop_store(),
    ));
    let shutdown = CancellationToken::new();
    let host_clone = host.clone();
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        eggsec_daemon::server::run_server(host_clone, shutdown_clone)
            .await
            .expect("the daemon serves");
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = DaemonRuntimeClient::connect(
        &socket_path,
        ClientIdentity {
            kind: ClientKind::Gui,
            label: "eggsec-gui".to_owned(),
            request_prefix: "gui-".to_owned(),
        },
    )
    .await
    .expect("the GUI client connects");

    let session_id = client
        .create_session(RuntimeSurface::DesktopGuiManual, None, vec![])
        .await
        .expect("the daemon opens a desktop-GUI session");

    // Subscribe before submitting, so the prompt cannot be missed.
    let mut events = client
        .subscribe(session_id)
        .await
        .expect("the client subscribes");

    let task_id = client
        .submit(session_id, request())
        .await
        .expect("the task is admitted");

    // The prompt reaches the client.
    let prompt = loop {
        match events.recv().await.expect("the prompt must be announced") {
            RuntimeEvent::PolicyDecisionRequired { prompt, .. } => break prompt,
            _ => continue,
        }
    };
    assert!(
        prompt.message.contains("port-scan"),
        "the prompt must name the operation: {}",
        prompt.message
    );

    // The operator answers through the client contract.
    client
        .approve_task(
            session_id,
            task_id,
            true,
            Some("the target is a lab host".into()),
        )
        .await
        .expect("the answer must reach the daemon");

    // The held operation runs.
    loop {
        match events.recv().await.expect("the operation must finish") {
            RuntimeEvent::TaskCompleted { outcome, .. } => {
                assert!(
                    matches!(&outcome, TaskOutcome::Text(text) if text == "the held operation ran"),
                    "the held operation must have run: {outcome:?}"
                );
                break;
            }
            RuntimeEvent::TaskFailed { error, .. } => {
                panic!("the held operation must not fail: {}", error.message)
            }
            _ => continue,
        }
    }

    shutdown.cancel();
}
