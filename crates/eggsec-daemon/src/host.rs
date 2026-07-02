use std::collections::HashMap;
use std::sync::Arc;

use eggsec_runtime::{ClientId, Runtime, RuntimeConfig, RuntimeError, RuntimeTaskExecutor};

use crate::client_registry::{check_permission, ClientInfo, ClientKind, ClientRegistry, ClientRole, SessionAccess};
use crate::config::DaemonConfig;
use crate::protocol::{ClientCommand, ErrorCode, ServerMessage};

/// Map a `ClientCommand` variant to its permission string for `check_permission`.
fn command_permission_name(cmd: &ClientCommand) -> &str {
    match cmd {
        ClientCommand::Health { .. } => "health",
        ClientCommand::Capabilities { .. } => "capabilities",
        ClientCommand::DeclareClient { .. } => "declare-client",
        ClientCommand::CreateSession { .. } => "create-session",
        ClientCommand::ListSessions { .. } => "list-sessions",
        ClientCommand::GetSnapshot { .. } => "get-snapshot",
        ClientCommand::SubmitTask { .. } => "submit-task",
        ClientCommand::CancelTask { .. } => "cancel-task",
        ClientCommand::CancelActive { .. } => "cancel-active",
        ClientCommand::Subscribe { .. } => "subscribe",
        ClientCommand::CloseSession { .. } => "close-session",
        ClientCommand::ApprovePolicy { .. } => "approve-policy",
    }
}

/// Wraps the eggsec runtime with daemon configuration and command dispatch.
///
/// `DaemonHost` is the bridge between the IPC protocol and the runtime.
/// It holds an `Arc<Runtime>` and a `DaemonConfig`, and exposes
/// `handle_command` which maps `ClientCommand` variants to runtime calls
/// and returns `ServerMessage` responses.
pub struct DaemonHost {
    runtime: Arc<Runtime>,
    config: DaemonConfig,
    client_registry: std::sync::Mutex<ClientRegistry>,
    session_access: std::sync::Mutex<HashMap<eggsec_runtime::SessionId, SessionAccess>>,
}

impl DaemonHost {
    pub fn new(config: DaemonConfig, executor: impl RuntimeTaskExecutor) -> Self {
        let runtime = Runtime::new(RuntimeConfig::default(), executor);
        Self {
            runtime: Arc::new(runtime),
            config,
            client_registry: std::sync::Mutex::new(ClientRegistry::new()),
            session_access: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    pub fn config(&self) -> &DaemonConfig {
        &self.config
    }

    pub fn register_client(&self, info: ClientInfo) -> ClientId {
        let client_id = info.client_id;
        self.client_registry.lock().unwrap().register(info);
        client_id
    }

    pub fn client_role_for_session(
        &self,
        client_id: &ClientId,
        session_id: &eggsec_runtime::SessionId,
    ) -> ClientRole {
        let access = self.session_access.lock().unwrap();
        if let Some(session_access) = access.get(session_id) {
            if session_access.owner_client_id == Some(*client_id) {
                return ClientRole::Owner;
            }
            for rule in &session_access.allowed_clients {
                if rule.client_id == *client_id {
                    return rule.role.clone();
                }
            }
        }
        ClientRole::Observer
    }

    /// Dispatch a client command to the runtime and return a response.
    ///
    /// Every response carries the same `request_id` from the incoming command.
    /// The optional `client_id` is used for permission checks. Commands that
    /// require a specific role (controller, owner) return `PermissionDenied`
    /// if the client lacks the required role.
    pub async fn handle_command(
        &self,
        cmd: ClientCommand,
        client_id: Option<ClientId>,
    ) -> ServerMessage {
        // Permission check before destructuring (borrowing &cmd).
        if let Some(session_id) = cmd.session_id() {
            if let Err(denied) = self.check_command_permission(&cmd, client_id, session_id) {
                return ServerMessage::Error {
                    request_id: cmd.request_id().to_owned(),
                    code: ErrorCode::PermissionDenied,
                    message: denied,
                };
            }
        }

        match cmd {
            ClientCommand::Health { request_id } => ServerMessage::Health {
                request_id,
                status: "ok".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },

            ClientCommand::Capabilities { request_id } => ServerMessage::Capabilities {
                request_id,
                capabilities: eggsec_runtime::RuntimeCapabilities::default(),
            },

            ClientCommand::DeclareClient {
                request_id,
                kind,
                label,
            } => {
                let client_id = ClientId::new();
                let info = ClientInfo {
                    client_id,
                    kind,
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    connected_at_secs: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    label,
                };
                self.register_client(info);
                ServerMessage::ClientDeclared {
                    request_id,
                    client_id,
                }
            }

            ClientCommand::CreateSession {
                request_id,
                surface,
                scope,
                labels: _,
            } => {
                let surface = if surface == eggsec_runtime::RuntimeSurface::Unknown {
                    self.config.default_surface.clone()
                } else {
                    surface
                };
                match self
                    .runtime()
                    .create_session_with_scope(Default::default(), surface, scope)
                    .await
                {
                    Ok(session_id) => {
                        let access = SessionAccess {
                            owner_client_id: client_id,
                            default_observer_allowed: true,
                            default_controller_allowed: true,
                            ..Default::default()
                        };
                        self.session_access
                            .lock()
                            .unwrap()
                            .insert(session_id, access);
                        ServerMessage::SessionCreated {
                            request_id,
                            session_id,
                        }
                    }
                    Err(e) => ServerMessage::Error {
                        request_id,
                        code: ErrorCode::Internal,
                        message: e.to_string(),
                    },
                }
            }

            ClientCommand::ListSessions { request_id } => {
                let sessions = self.runtime().list_sessions().await;
                ServerMessage::Sessions {
                    request_id,
                    sessions,
                }
            }

            ClientCommand::GetSnapshot {
                request_id,
                session_id,
            } => match self.runtime().snapshot(session_id).await {
                Ok(snapshot) => ServerMessage::Snapshot {
                    request_id,
                    snapshot,
                },
                Err(e) => ServerMessage::Error {
                    request_id,
                    code: ErrorCode::SessionNotFound,
                    message: e.to_string(),
                },
            },

            ClientCommand::SubmitTask {
                request_id,
                session_id,
                request,
            } => match self.runtime().submit(session_id, request).await {
                Ok(task_id) => ServerMessage::TaskSubmitted {
                    request_id,
                    task_id,
                },
                Err(e) => {
                    let code = match &e {
                        RuntimeError::SessionNotFound(_) => ErrorCode::SessionNotFound,
                        RuntimeError::UnsupportedTaskKind => ErrorCode::InvalidRequest,
                        _ => ErrorCode::Internal,
                    };
                    ServerMessage::Error {
                        request_id,
                        code,
                        message: e.to_string(),
                    }
                }
            },

            ClientCommand::CancelTask {
                request_id,
                session_id,
                task_id,
            } => match self.runtime().cancel(session_id, task_id).await {
                Ok(()) => ServerMessage::Ok { request_id },
                Err(e) => {
                    let code = match &e {
                        RuntimeError::SessionNotFound(_) => ErrorCode::SessionNotFound,
                        RuntimeError::TaskNotFound(_) => ErrorCode::TaskNotFound,
                        RuntimeError::TaskAlreadyCompleted(_) => ErrorCode::TaskAlreadyCompleted,
                        _ => ErrorCode::Internal,
                    };
                    ServerMessage::Error {
                        request_id,
                        code,
                        message: e.to_string(),
                    }
                }
            },

            ClientCommand::CancelActive {
                request_id,
                session_id,
            } => match self.runtime().cancel_active(session_id).await {
                Ok(()) => ServerMessage::Ok { request_id },
                Err(e) => ServerMessage::Error {
                    request_id,
                    code: ErrorCode::SessionNotFound,
                    message: e.to_string(),
                },
            },

            ClientCommand::Subscribe { request_id, .. } => ServerMessage::Error {
                request_id,
                code: ErrorCode::Internal,
                message: "subscribe is handled at the transport level".into(),
            },

            ClientCommand::CloseSession {
                request_id,
                session_id,
            } => match self.runtime().snapshot(session_id).await {
                Ok(_) => ServerMessage::SessionClosed { request_id },
                Err(e) => ServerMessage::Error {
                    request_id,
                    code: ErrorCode::SessionNotFound,
                    message: e.to_string(),
                },
            },

            ClientCommand::ApprovePolicy {
                request_id,
                session_id: _,
                task_id: _,
                approved: _,
                reason: _,
            } => ServerMessage::Ok { request_id },
        }
    }

    /// Look up the client's role for a session and run the permission check.
    /// Returns Ok(()) if permitted, or Err(message) if denied.
    fn check_command_permission(
        &self,
        cmd: &ClientCommand,
        client_id: Option<ClientId>,
        session_id: &eggsec_runtime::SessionId,
    ) -> Result<(), String> {
        let perm_name = command_permission_name(cmd);
        if matches!(
            perm_name,
            "health" | "capabilities" | "list-sessions" | "declare-client" | "create-session"
        ) {
            return Ok(());
        }
        let Some(client_id) = client_id else {
            return Err("permission-denied: client not declared".into());
        };
        // If the session doesn't exist in the access table, let the runtime
        // return SessionNotFound — don't block on permissions for a ghost session.
        {
            let access = self.session_access.lock().unwrap();
            if !access.contains_key(session_id) {
                return Ok(());
            }
        }
        let role = self.client_role_for_session(&client_id, session_id);
        let kind = self
            .client_registry
            .lock()
            .unwrap()
            .get(&client_id)
            .map(|c| c.kind.clone())
            .unwrap_or(ClientKind::Unknown);
        let surface = self
            .session_access
            .lock()
            .unwrap()
            .get(session_id)
            .map(|access| {
                if access.default_controller_allowed {
                    eggsec_runtime::RuntimeSurface::TuiManual
                } else {
                    eggsec_runtime::RuntimeSurface::McpServer
                }
            })
            .unwrap_or(eggsec_runtime::RuntimeSurface::Unknown);
        check_permission(&kind, &role, &surface, perm_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DaemonConfig;
    use crate::protocol::{ClientCommand, ErrorCode, ServerMessage};
    use eggsec_runtime::{
        CancellationToken, RuntimeEventSink, RuntimeTaskExecutor, TaskId, TaskOutcome,
    };
    use std::future::Future;
    use std::pin::Pin;

    struct TestExecutor;

    impl RuntimeTaskExecutor for TestExecutor {
        fn execute(
            &self,
            _task_id: TaskId,
            _request: eggsec_runtime::RunRequest,
            _sink: RuntimeEventSink,
            _cancel: CancellationToken,
        ) -> Pin<Box<dyn Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>>
        {
            Box::pin(async { Ok(TaskOutcome::Text("test-result".into())) })
        }
    }

    fn test_host() -> DaemonHost {
        DaemonHost::new(DaemonConfig::default(), TestExecutor)
    }

    /// Declare a test client and return its ID.
    async fn declare_test_client(host: &DaemonHost) -> ClientId {
        match host
            .handle_command(
                ClientCommand::DeclareClient {
                    request_id: "declare-1".into(),
                    kind: ClientKind::Tui,
                    label: Some("test-tui".into()),
                },
                None,
            )
            .await
        {
            ServerMessage::ClientDeclared { client_id, .. } => client_id,
            other => panic!("expected ClientDeclared, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_health() {
        let host = test_host();
        let resp = host
            .handle_command(ClientCommand::Health { request_id: "req-1".into() }, None)
            .await;
        match resp {
            ServerMessage::Health { request_id, status, version } => {
                assert_eq!(request_id, "req-1");
                assert_eq!(status, "ok");
                assert!(!version.is_empty());
            }
            other => panic!("expected Health, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_capabilities() {
        let host = test_host();
        let resp = host
            .handle_command(ClientCommand::Capabilities { request_id: "req-2".into() }, None)
            .await;
        match resp {
            ServerMessage::Capabilities { request_id, capabilities } => {
                assert_eq!(request_id, "req-2");
                assert!(!capabilities.task_kinds.is_empty());
            }
            other => panic!("expected Capabilities, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_create_session_and_list() {
        let host = test_host();
        let resp = host
            .handle_command(ClientCommand::CreateSession {
                request_id: "req-3".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, None)
            .await;
        let session_id = match resp {
            ServerMessage::SessionCreated { request_id, session_id } => {
                assert_eq!(request_id, "req-3");
                session_id
            }
            other => panic!("expected SessionCreated, got {:?}", other),
        };

        let resp = host
            .handle_command(ClientCommand::ListSessions { request_id: "req-4".into() }, None)
            .await;
        match resp {
            ServerMessage::Sessions { request_id, sessions } => {
                assert_eq!(request_id, "req-4");
                assert!(sessions.iter().any(|s| s.session_id == session_id));
            }
            other => panic!("expected Sessions, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_submit_task() {
        let host = test_host();
        let client_id = declare_test_client(&host).await;
        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(client_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::SubmitTask {
                request_id: "r2".into(),
                session_id,
                request: eggsec_runtime::RunRequest {
                    task_kind: eggsec_runtime::TaskKind::PortScan(
                        eggsec_runtime::request::PortScanParams {
                            target: "10.0.0.1".into(),
                            ports: Some("80".into()),
                            scan_type: None,
                            timeout_ms: None,
                        },
                    ),
                    requested_by: None,
                    surface: eggsec_runtime::RuntimeSurface::CliManual,
                    labels: vec![],
                },
            }, Some(client_id))
            .await;
        match resp {
            ServerMessage::TaskSubmitted { request_id, task_id } => {
                assert_eq!(request_id, "r2");
                let _ = task_id;
            }
            other => panic!("expected TaskSubmitted, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_get_snapshot() {
        let host = test_host();
        let client_id = declare_test_client(&host).await;
        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(client_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::GetSnapshot { request_id: "r2".into(), session_id }, Some(client_id))
            .await;
        match resp {
            ServerMessage::Snapshot { request_id, snapshot } => {
                assert_eq!(request_id, "r2");
                assert_eq!(snapshot.session_id, session_id);
                assert!(snapshot.active_tasks.is_empty());
                assert!(snapshot.completed_tasks.is_empty());
            }
            other => panic!("expected Snapshot, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_cancel_active_empty_session() {
        let host = test_host();
        let client_id = declare_test_client(&host).await;
        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(client_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::CancelActive { request_id: "r2".into(), session_id }, Some(client_id))
            .await;
        match resp {
            ServerMessage::Ok { request_id } => assert_eq!(request_id, "r2"),
            other => panic!("expected Ok, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_session_not_found() {
        let host = test_host();
        let client_id = declare_test_client(&host).await;
        let fake_id = eggsec_runtime::SessionId::new();
        let resp = host
            .handle_command(ClientCommand::GetSnapshot { request_id: "r1".into(), session_id: fake_id }, Some(client_id))
            .await;
        match resp {
            ServerMessage::Error { request_id, code, message } => {
                assert_eq!(request_id, "r1");
                assert_eq!(code, ErrorCode::SessionNotFound);
                assert!(message.contains("not found"));
            }
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_declare_client() {
        let host = test_host();
        let resp = host
            .handle_command(ClientCommand::DeclareClient {
                request_id: "r1".into(),
                kind: ClientKind::Tui,
                label: Some("my-tui".into()),
            }, None)
            .await;
        match resp {
            ServerMessage::ClientDeclared { request_id, client_id } => {
                assert_eq!(request_id, "r1");
                let _ = client_id;
            }
            other => panic!("expected ClientDeclared, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_close_session() {
        let host = test_host();
        let client_id = declare_test_client(&host).await;
        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(client_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::CloseSession { request_id: "r2".into(), session_id }, Some(client_id))
            .await;
        match resp {
            ServerMessage::SessionClosed { request_id } => assert_eq!(request_id, "r2"),
            other => panic!("expected SessionClosed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_close_session_not_found() {
        let host = test_host();
        let client_id = declare_test_client(&host).await;
        let fake_id = eggsec_runtime::SessionId::new();
        let resp = host
            .handle_command(ClientCommand::CloseSession { request_id: "r1".into(), session_id: fake_id }, Some(client_id))
            .await;
        match resp {
            ServerMessage::Error { request_id, code, message } => {
                assert_eq!(request_id, "r1");
                assert_eq!(code, ErrorCode::SessionNotFound);
                assert!(message.contains("not found"));
            }
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_declare_client_stores_in_registry() {
        let host = test_host();
        let resp = host
            .handle_command(ClientCommand::DeclareClient {
                request_id: "r1".into(),
                kind: ClientKind::Agent,
                label: Some("agent-1".into()),
            }, None)
            .await;
        let client_id = match resp {
            ServerMessage::ClientDeclared { client_id, .. } => client_id,
            _ => panic!("expected ClientDeclared"),
        };
        let stored = host.client_registry.lock().unwrap().get(&client_id).cloned();
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().kind, ClientKind::Agent);
    }

    #[tokio::test]
    async fn create_session_records_owner_client_id() {
        let host = test_host();
        let client_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: None,
        });

        let resp = host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(client_id))
            .await;
        let session_id = match resp {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let access = host.session_access.lock().unwrap();
        let session_access = access.get(&session_id).unwrap();
        assert_eq!(session_access.owner_client_id, Some(client_id));
    }

    #[tokio::test]
    async fn observer_cannot_submit_task() {
        let host = test_host();
        let observer_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: observer_id,
            kind: ClientKind::Cli,
            surface: eggsec_runtime::RuntimeSurface::CliManual,
            connected_at_secs: 100,
            label: None,
        });

        let owner_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: owner_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: None,
        });

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(owner_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::SubmitTask {
                request_id: "r2".into(),
                session_id,
                request: eggsec_runtime::RunRequest {
                    task_kind: eggsec_runtime::TaskKind::PortScan(
                        eggsec_runtime::request::PortScanParams {
                            target: "10.0.0.1".into(),
                            ports: Some("80".into()),
                            scan_type: None,
                            timeout_ms: None,
                        },
                    ),
                    requested_by: None,
                    surface: eggsec_runtime::RuntimeSurface::CliManual,
                    labels: vec![],
                },
            }, Some(observer_id))
            .await;
        match resp {
            ServerMessage::Error { code: ErrorCode::PermissionDenied, message, .. } => {
                assert!(message.contains("permission-denied"));
            }
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn observer_cannot_close_session() {
        let host = test_host();
        let observer_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: observer_id,
            kind: ClientKind::Cli,
            surface: eggsec_runtime::RuntimeSurface::CliManual,
            connected_at_secs: 100,
            label: None,
        });

        let owner_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: owner_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: None,
        });

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(owner_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::CloseSession { request_id: "r2".into(), session_id }, Some(observer_id))
            .await;
        match resp {
            ServerMessage::Error { code: ErrorCode::PermissionDenied, message, .. } => {
                assert!(message.contains("permission-denied"));
            }
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn undeclared_client_cannot_submit() {
        let host = test_host();
        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, None)
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::SubmitTask {
                request_id: "r2".into(),
                session_id,
                request: eggsec_runtime::RunRequest {
                    task_kind: eggsec_runtime::TaskKind::PortScan(
                        eggsec_runtime::request::PortScanParams {
                            target: "10.0.0.1".into(),
                            ports: Some("80".into()),
                            scan_type: None,
                            timeout_ms: None,
                        },
                    ),
                    requested_by: None,
                    surface: eggsec_runtime::RuntimeSurface::CliManual,
                    labels: vec![],
                },
            }, None)
            .await;
        match resp {
            ServerMessage::Error { code: ErrorCode::PermissionDenied, message, .. } => {
                assert!(message.contains("client not declared"));
            }
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn owner_can_submit_task() {
        let host = test_host();
        let owner_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: owner_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: None,
        });

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(owner_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::SubmitTask {
                request_id: "r2".into(),
                session_id,
                request: eggsec_runtime::RunRequest {
                    task_kind: eggsec_runtime::TaskKind::PortScan(
                        eggsec_runtime::request::PortScanParams {
                            target: "10.0.0.1".into(),
                            ports: Some("80".into()),
                            scan_type: None,
                            timeout_ms: None,
                        },
                    ),
                    requested_by: None,
                    surface: eggsec_runtime::RuntimeSurface::CliManual,
                    labels: vec![],
                },
            }, Some(owner_id))
            .await;
        match resp {
            ServerMessage::TaskSubmitted { .. } => {}
            other => panic!("expected TaskSubmitted, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn approve_policy_owner_allowed() {
        let host = test_host();
        let owner_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: owner_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: None,
        });

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(owner_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::ApprovePolicy {
                request_id: "r2".into(),
                session_id,
                task_id: eggsec_runtime::TaskId::new(),
                approved: true,
                reason: Some("confirmed".into()),
            }, Some(owner_id))
            .await;
        match resp {
            ServerMessage::Ok { request_id } => assert_eq!(request_id, "r2"),
            other => panic!("expected Ok, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn approve_policy_observer_denied() {
        let host = test_host();
        let observer_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: observer_id,
            kind: ClientKind::Cli,
            surface: eggsec_runtime::RuntimeSurface::CliManual,
            connected_at_secs: 100,
            label: None,
        });

        let owner_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: owner_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: None,
        });

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(owner_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::ApprovePolicy {
                request_id: "r2".into(),
                session_id,
                task_id: eggsec_runtime::TaskId::new(),
                approved: true,
                reason: None,
            }, Some(observer_id))
            .await;
        match resp {
            ServerMessage::Error { code: ErrorCode::PermissionDenied, message, .. } => {
                assert!(message.contains("permission-denied"));
            }
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn observer_can_get_snapshot() {
        let host = test_host();
        let observer_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: observer_id,
            kind: ClientKind::Cli,
            surface: eggsec_runtime::RuntimeSurface::CliManual,
            connected_at_secs: 100,
            label: None,
        });

        let owner_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: owner_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: None,
        });

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            }, Some(owner_id))
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::GetSnapshot { request_id: "r2".into(), session_id }, Some(observer_id))
            .await;
        match resp {
            ServerMessage::Snapshot { .. } => {}
            other => panic!("expected Snapshot, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn tui_client_creates_session_as_owner() {
        let host = test_host();
        let tui_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: tui_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: Some("my-tui".into()),
        });

        let session_id = match host
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::TuiManual,
                    scope: None,
                    labels: vec![],
                },
                Some(tui_id),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        // TUI client should be the owner — can submit tasks.
        let resp = host
            .handle_command(
                ClientCommand::SubmitTask {
                    request_id: "r2".into(),
                    session_id,
                    request: eggsec_runtime::RunRequest {
                        task_kind: eggsec_runtime::TaskKind::PortScan(
                            eggsec_runtime::request::PortScanParams {
                                target: "10.0.0.1".into(),
                                ports: Some("80".into()),
                                scan_type: None,
                                timeout_ms: None,
                            },
                        ),
                        requested_by: None,
                        surface: eggsec_runtime::RuntimeSurface::TuiManual,
                        labels: vec![],
                    },
                },
                Some(tui_id),
            )
            .await;
        assert!(
            matches!(resp, ServerMessage::TaskSubmitted { .. }),
            "TUI owner should be able to submit tasks, got {:?}",
            resp
        );
    }

    #[tokio::test]
    async fn cli_client_creates_session_as_owner() {
        let host = test_host();
        let cli_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: cli_id,
            kind: ClientKind::Cli,
            surface: eggsec_runtime::RuntimeSurface::CliManual,
            connected_at_secs: 100,
            label: Some("my-cli".into()),
        });

        let session_id = match host
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::CliManual,
                    scope: None,
                    labels: vec![],
                },
                Some(cli_id),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        // CLI client should be the owner — can cancel tasks.
        let resp = host
            .handle_command(
                ClientCommand::CancelActive {
                    request_id: "r2".into(),
                    session_id,
                },
                Some(cli_id),
            )
            .await;
        assert!(
            matches!(resp, ServerMessage::Ok { .. }),
            "CLI owner should be able to cancel active, got {:?}",
            resp
        );
    }

    #[tokio::test]
    async fn client_kind_stored_in_registry() {
        let host = test_host();

        for kind in [ClientKind::Tui, ClientKind::Cli, ClientKind::Mcp, ClientKind::Rest, ClientKind::Agent] {
            let resp = host
                .handle_command(
                    ClientCommand::DeclareClient {
                        request_id: format!("r-{:?}", kind),
                        kind: kind.clone(),
                        label: None,
                    },
                    None,
                )
                .await;
            let client_id = match resp {
                ServerMessage::ClientDeclared { client_id, .. } => client_id,
                other => panic!("expected ClientDeclared for {:?}, got {:?}", kind, other),
            };
            let stored = host.client_registry.lock().unwrap().get(&client_id).cloned().unwrap();
            assert_eq!(stored.kind, kind);
        }
    }
}
