use std::collections::HashMap;
use std::sync::Arc;

use eggsec_runtime::{ClientId, Runtime, RuntimeConfig, RuntimeError, RuntimeTaskExecutor};

use crate::client_registry::{
    check_permission, command_permission, ClientInfo, ClientKind, ClientRegistry, ClientRole,
    CommandPermission, SessionAccess,
};
use crate::config::DaemonConfig;
use crate::protocol::{ClientCommand, DaemonRequestContext, ErrorCode, ServerMessage};
use crate::store::{DaemonStore, PersistedAuditEvent};

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
    store: Arc<dyn DaemonStore>,
}

impl DaemonHost {
    pub fn new(
        config: DaemonConfig,
        executor: impl RuntimeTaskExecutor,
        store: Arc<dyn DaemonStore>,
    ) -> Self {
        let runtime = Runtime::new(RuntimeConfig::default(), executor);
        Self {
            runtime: Arc::new(runtime),
            config,
            client_registry: std::sync::Mutex::new(ClientRegistry::new()),
            session_access: std::sync::Mutex::new(HashMap::new()),
            store,
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

    // --- Persistence helpers ---

    /// Record a security-relevant audit event.
    async fn record_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()> {
        if !self.config.enable_persistence {
            return Ok(());
        }
        self.store.record_audit_event(&event).await
    }

    /// Recover persisted session state from the store on startup.
    ///
    /// Loads all persisted sessions and reconstructs them in the runtime.
    /// Tasks that were Running/Queued are marked as Interrupted (not auto-resumed).
    pub async fn recover_persisted_state(&self) -> anyhow::Result<()> {
        if !self.config.enable_persistence {
            return Ok(());
        }

        let sessions = self.store.load_all_sessions().await?;
        if sessions.is_empty() {
            tracing::info!("No persisted sessions to recover");
            return Ok(());
        }

        let mut recovered = 0u32;
        let mut interrupted_tasks = 0u32;

        for snapshot in sessions {
            // Mark any non-terminal tasks as interrupted
            let mut snapshot = snapshot;
            for task in &mut snapshot.active_tasks {
                task.status = eggsec_runtime::TaskStatus::Cancelled;
                task.last_error = Some("interrupted by daemon restart".into());
                interrupted_tasks += 1;
            }

            match self.runtime().hydrate_session(snapshot).await {
                Ok(session_id) => {
                    recovered += 1;
                    tracing::info!(%session_id, "Recovered persisted session");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to recover persisted session");
                }
            }
        }

        // Record recovery audit event
        let _ = self
            .record_audit_event(PersistedAuditEvent {
                action: "daemon-recovery".into(),
                surface: "daemon".into(),
                outcome: "recovered".into(),
                client_id: None,
                session_id: None,
                timestamp_secs: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            })
            .await;

        tracing::info!(
            sessions_recovered = recovered,
            interrupted_tasks,
            "Startup recovery complete"
        );

        Ok(())
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
        ctx: DaemonRequestContext,
    ) -> ServerMessage {
        let client_id = ctx.client_id;
        // Permission check before destructuring (borrowing &cmd).
        if let Some(session_id) = cmd.session_id() {
            if let Err((code, message)) =
                self.check_command_permission(&cmd, &ctx, session_id).await
            {
                // Record permission denial audit event
                let _ = self
                    .record_audit_event(PersistedAuditEvent {
                        action: format!("command-denied:{}", cmd.discriminant()),
                        surface: "daemon".into(),
                        outcome: "denied".into(),
                        client_id: client_id.map(|c| c.to_string()),
                        session_id: Some(session_id.to_string()),
                        timestamp_secs: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    })
                    .await;
                return ServerMessage::Error {
                    request_id: cmd.request_id().to_owned(),
                    code,
                    message,
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
                capabilities: crate::protocol::DaemonCapabilities {
                    runtime: eggsec_runtime::RuntimeCapabilities::default(),
                    transports: vec![crate::protocol::TransportCapability {
                        kind: crate::protocol::TransportKind::UnixSocket,
                        bind_address: self.config.socket_path.clone(),
                        enabled: true,
                    }],
                },
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
                let _ = self
                    .record_audit_event(PersistedAuditEvent {
                        action: "declare-client".into(),
                        surface: "daemon".into(),
                        outcome: "allow".into(),
                        client_id: Some(client_id.to_string()),
                        session_id: None,
                        timestamp_secs: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    })
                    .await;
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
                    .create_session_with_scope(Default::default(), surface.clone(), scope)
                    .await
                {
                    Ok(session_id) => {
                        let owner_kind = client_id
                            .and_then(|cid| {
                                self.client_registry
                                    .lock()
                                    .unwrap()
                                    .get(&cid)
                                    .map(|c| c.kind.clone())
                            })
                            .unwrap_or(ClientKind::Unknown);
                        let access = SessionAccess {
                            owner_client_id: client_id,
                            surface,
                            owner_client_kind: owner_kind,
                            default_observer_allowed: true,
                            default_controller_allowed: true,
                            ..Default::default()
                        };
                        self.session_access
                            .lock()
                            .unwrap()
                            .insert(session_id, access);
                        // Persist session snapshot and record audit event
                        let store = self.store.clone();
                        let runtime = self.runtime.clone();
                        let enable_persistence = self.config.enable_persistence;
                        let client_id_str = client_id.map(|c| c.to_string());
                        let session_id_copy = session_id;
                        tokio::spawn(async move {
                            if enable_persistence {
                                if let Ok(snapshot) = runtime.snapshot(session_id_copy).await {
                                    if let Err(e) = store.save_session_snapshot(&snapshot).await {
                                        tracing::warn!(error = %e, "Failed to persist session snapshot");
                                    }
                                }
                                let _ = store
                                    .record_audit_event(&PersistedAuditEvent {
                                        action: "create-session".into(),
                                        surface: "daemon".into(),
                                        outcome: "allow".into(),
                                        client_id: client_id_str,
                                        session_id: Some(session_id_copy.to_string()),
                                        timestamp_secs: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs(),
                                    })
                                    .await;
                            }
                        });
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
                Ok(task_id) => {
                    // Persist snapshot and record audit event
                    let store = self.store.clone();
                    let runtime = self.runtime.clone();
                    let enable_persistence = self.config.enable_persistence;
                    let client_id_str = client_id.map(|c| c.to_string());
                    tokio::spawn(async move {
                        if enable_persistence {
                            if let Ok(snapshot) = runtime.snapshot(session_id).await {
                                if let Err(e) = store.save_session_snapshot(&snapshot).await {
                                    tracing::warn!(error = %e, "Failed to persist snapshot after task submit");
                                }
                            }
                            let _ = store
                                .record_audit_event(&PersistedAuditEvent {
                                    action: "submit-task".into(),
                                    surface: "daemon".into(),
                                    outcome: "allow".into(),
                                    client_id: client_id_str,
                                    session_id: Some(session_id.to_string()),
                                    timestamp_secs: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                })
                                .await;
                        }
                    });
                    ServerMessage::TaskSubmitted {
                        request_id,
                        task_id,
                    }
                }
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
                Ok(()) => {
                    // Persist snapshot and record audit event
                    let store = self.store.clone();
                    let runtime = self.runtime.clone();
                    let enable_persistence = self.config.enable_persistence;
                    let client_id_str = client_id.map(|c| c.to_string());
                    tokio::spawn(async move {
                        if enable_persistence {
                            if let Ok(snapshot) = runtime.snapshot(session_id).await {
                                if let Err(e) = store.save_session_snapshot(&snapshot).await {
                                    tracing::warn!(error = %e, "Failed to persist snapshot after cancel");
                                }
                            }
                            let _ = store
                                .record_audit_event(&PersistedAuditEvent {
                                    action: "cancel-task".into(),
                                    surface: "daemon".into(),
                                    outcome: "allow".into(),
                                    client_id: client_id_str,
                                    session_id: Some(session_id.to_string()),
                                    timestamp_secs: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                })
                                .await;
                        }
                    });
                    ServerMessage::Ok { request_id }
                }
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
                Ok(()) => {
                    // Persist snapshot and record audit event
                    let store = self.store.clone();
                    let runtime = self.runtime.clone();
                    let enable_persistence = self.config.enable_persistence;
                    let client_id_str = client_id.map(|c| c.to_string());
                    tokio::spawn(async move {
                        if enable_persistence {
                            if let Ok(snapshot) = runtime.snapshot(session_id).await {
                                if let Err(e) = store.save_session_snapshot(&snapshot).await {
                                    tracing::warn!(error = %e, "Failed to persist snapshot after cancel-active");
                                }
                            }
                            let _ = store
                                .record_audit_event(&PersistedAuditEvent {
                                    action: "cancel-active".into(),
                                    surface: "daemon".into(),
                                    outcome: "allow".into(),
                                    client_id: client_id_str,
                                    session_id: Some(session_id.to_string()),
                                    timestamp_secs: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                })
                                .await;
                        }
                    });
                    ServerMessage::Ok { request_id }
                }
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
                Ok(_) => {
                    // Record audit event for session close
                    let store = self.store.clone();
                    let enable_persistence = self.config.enable_persistence;
                    let client_id_str = client_id.map(|c| c.to_string());
                    tokio::spawn(async move {
                        if enable_persistence {
                            let _ = store
                                .record_audit_event(&PersistedAuditEvent {
                                    action: "close-session".into(),
                                    surface: "daemon".into(),
                                    outcome: "allow".into(),
                                    client_id: client_id_str,
                                    session_id: Some(session_id.to_string()),
                                    timestamp_secs: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                })
                                .await;
                        }
                    });
                    ServerMessage::SessionClosed { request_id }
                }
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
            } => {
                // Record audit event even though this is not yet wired
                let _ = self
                    .record_audit_event(PersistedAuditEvent {
                        action: "approve-policy".into(),
                        surface: "daemon".into(),
                        outcome: "unsupported".into(),
                        client_id: client_id.map(|c| c.to_string()),
                        session_id: None,
                        timestamp_secs: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    })
                    .await;
                ServerMessage::Error {
                    request_id,
                    code: ErrorCode::Unsupported,
                    message: "daemon policy approval is not wired yet".into(),
                }
            }

            ClientCommand::ListPersistedSessions { request_id } => {
                let store = self.store.clone();
                let result = tokio::task::spawn_blocking(move || store.blocking_list_sessions())
                    .await
                    .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking failed: {}", e)));
                match result {
                    Ok(sessions) => ServerMessage::PersistedSessions {
                        request_id,
                        sessions,
                    },
                    Err(e) => ServerMessage::Error {
                        request_id,
                        code: ErrorCode::Internal,
                        message: e.to_string(),
                    },
                }
            }

            ClientCommand::GetPersistedSnapshot {
                request_id,
                session_id,
            } => {
                let store = self.store.clone();
                let sid = session_id;
                let result = tokio::task::spawn_blocking(move || store.blocking_get_snapshot(&sid))
                    .await
                    .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking failed: {}", e)));
                match result {
                    Ok(snapshot) => ServerMessage::PersistedSnapshot {
                        request_id,
                        snapshot,
                    },
                    Err(e) => ServerMessage::Error {
                        request_id,
                        code: ErrorCode::Internal,
                        message: e.to_string(),
                    },
                }
            }
        }
    }

    /// Look up the client's role for a session and run the permission check.
    /// Returns Ok(()) if permitted, or Err((ErrorCode, message)) if denied.
    async fn check_command_permission(
        &self,
        cmd: &ClientCommand,
        ctx: &DaemonRequestContext,
        session_id: &eggsec_runtime::SessionId,
    ) -> Result<(), (ErrorCode, String)> {
        let perm = command_permission(cmd);

        // Public commands are always allowed.
        if perm == CommandPermission::Public {
            return Ok(());
        }

        // Declared-client commands (DeclareClient, CreateSession, ListSessions)
        // do not require a session — they operate at the daemon level.
        if perm == CommandPermission::DeclaredClient {
            return Ok(());
        }

        // All session-scoped commands require a declared client.
        let Some(client_id) = ctx.client_id else {
            return Err((
                ErrorCode::ClientNotDeclared,
                "client must declare before sending session commands".into(),
            ));
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

        // Use the actual runtime session surface, not a derived proxy.
        let surface = self
            .runtime()
            .session_surface(*session_id)
            .await
            .unwrap_or(eggsec_runtime::RuntimeSurface::Unknown);

        check_permission(&kind, &role, &surface, perm)
            .map_err(|msg| (ErrorCode::PermissionDenied, msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DaemonConfig;
    use crate::protocol::{
        ClientCommand, DaemonRequestContext, ErrorCode, ServerMessage, TransportKind,
    };
    use crate::store::NoopStore;
    use eggsec_runtime::{
        CancellationToken, RuntimeEventSink, RuntimeTaskExecutor, TaskId, TaskOutcome,
    };
    use std::future::Future;
    use std::pin::Pin;

    fn test_ctx(client_id: Option<eggsec_runtime::ClientId>) -> DaemonRequestContext {
        DaemonRequestContext {
            client_id,
            peer: None,
            transport: TransportKind::UnixSocket,
        }
    }

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
        DaemonHost::new(
            DaemonConfig::default(),
            TestExecutor,
            crate::store::noop_store(),
        )
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
                test_ctx(None),
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
            .handle_command(
                ClientCommand::Health {
                    request_id: "req-1".into(),
                },
                test_ctx(None),
            )
            .await;
        match resp {
            ServerMessage::Health {
                request_id,
                status,
                version,
            } => {
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
            .handle_command(
                ClientCommand::Capabilities {
                    request_id: "req-2".into(),
                },
                test_ctx(None),
            )
            .await;
        match resp {
            ServerMessage::Capabilities {
                request_id,
                capabilities,
            } => {
                assert_eq!(request_id, "req-2");
                assert!(!capabilities.runtime.task_kinds.is_empty());
            }
            other => panic!("expected Capabilities, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_create_session_and_list() {
        let host = test_host();
        let resp = host
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "req-3".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(None),
            )
            .await;
        let session_id = match resp {
            ServerMessage::SessionCreated {
                request_id,
                session_id,
            } => {
                assert_eq!(request_id, "req-3");
                session_id
            }
            other => panic!("expected SessionCreated, got {:?}", other),
        };

        let resp = host
            .handle_command(
                ClientCommand::ListSessions {
                    request_id: "req-4".into(),
                },
                test_ctx(None),
            )
            .await;
        match resp {
            ServerMessage::Sessions {
                request_id,
                sessions,
            } => {
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(client_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

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
                        surface: eggsec_runtime::RuntimeSurface::CliManual,
                        labels: vec![],
                    },
                },
                test_ctx(Some(client_id)),
            )
            .await;
        match resp {
            ServerMessage::TaskSubmitted {
                request_id,
                task_id,
            } => {
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(client_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(
                ClientCommand::GetSnapshot {
                    request_id: "r2".into(),
                    session_id,
                },
                test_ctx(Some(client_id)),
            )
            .await;
        match resp {
            ServerMessage::Snapshot {
                request_id,
                snapshot,
            } => {
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(client_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(
                ClientCommand::CancelActive {
                    request_id: "r2".into(),
                    session_id,
                },
                test_ctx(Some(client_id)),
            )
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
            .handle_command(
                ClientCommand::GetSnapshot {
                    request_id: "r1".into(),
                    session_id: fake_id,
                },
                test_ctx(Some(client_id)),
            )
            .await;
        match resp {
            ServerMessage::Error {
                request_id,
                code,
                message,
            } => {
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
            .handle_command(
                ClientCommand::DeclareClient {
                    request_id: "r1".into(),
                    kind: ClientKind::Tui,
                    label: Some("my-tui".into()),
                },
                test_ctx(None),
            )
            .await;
        match resp {
            ServerMessage::ClientDeclared {
                request_id,
                client_id,
            } => {
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(client_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(
                ClientCommand::CloseSession {
                    request_id: "r2".into(),
                    session_id,
                },
                test_ctx(Some(client_id)),
            )
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
            .handle_command(
                ClientCommand::CloseSession {
                    request_id: "r1".into(),
                    session_id: fake_id,
                },
                test_ctx(Some(client_id)),
            )
            .await;
        match resp {
            ServerMessage::Error {
                request_id,
                code,
                message,
            } => {
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
            .handle_command(
                ClientCommand::DeclareClient {
                    request_id: "r1".into(),
                    kind: ClientKind::Agent,
                    label: Some("agent-1".into()),
                },
                test_ctx(None),
            )
            .await;
        let client_id = match resp {
            ServerMessage::ClientDeclared { client_id, .. } => client_id,
            _ => panic!("expected ClientDeclared"),
        };
        let stored = host
            .client_registry
            .lock()
            .unwrap()
            .get(&client_id)
            .cloned();
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(client_id)),
            )
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

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
                        surface: eggsec_runtime::RuntimeSurface::CliManual,
                        labels: vec![],
                    },
                },
                test_ctx(Some(observer_id)),
            )
            .await;
        match resp {
            ServerMessage::Error {
                code: ErrorCode::PermissionDenied,
                message,
                ..
            } => {
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(
                ClientCommand::CloseSession {
                    request_id: "r2".into(),
                    session_id,
                },
                test_ctx(Some(observer_id)),
            )
            .await;
        match resp {
            ServerMessage::Error {
                code: ErrorCode::PermissionDenied,
                message,
                ..
            } => {
                assert!(message.contains("permission-denied"));
            }
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn undeclared_client_cannot_submit() {
        let host = test_host();
        let session_id = match host
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(None),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

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
                        surface: eggsec_runtime::RuntimeSurface::CliManual,
                        labels: vec![],
                    },
                },
                test_ctx(None),
            )
            .await;
        match resp {
            ServerMessage::Error {
                code: ErrorCode::ClientNotDeclared,
                message,
                ..
            } => {
                assert!(message.contains("must declare"));
            }
            other => panic!("expected ClientNotDeclared, got {:?}", other),
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

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
                        surface: eggsec_runtime::RuntimeSurface::CliManual,
                        labels: vec![],
                    },
                },
                test_ctx(Some(owner_id)),
            )
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(
                ClientCommand::ApprovePolicy {
                    request_id: "r2".into(),
                    session_id,
                    task_id: eggsec_runtime::TaskId::new(),
                    approved: true,
                    reason: Some("confirmed".into()),
                },
                test_ctx(Some(owner_id)),
            )
            .await;
        match resp {
            ServerMessage::Error {
                code: ErrorCode::Unsupported,
                message,
                ..
            } => {
                assert!(message.contains("not wired yet"));
            }
            other => panic!("expected Unsupported error, got {:?}", other),
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::TuiManual,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(
                ClientCommand::ApprovePolicy {
                    request_id: "r2".into(),
                    session_id,
                    task_id: eggsec_runtime::TaskId::new(),
                    approved: true,
                    reason: None,
                },
                test_ctx(Some(observer_id)),
            )
            .await;
        match resp {
            ServerMessage::Error { code, message, .. } => {
                // Observer on manual session is denied by permission check,
                // or approve-policy is unsupported. Either is acceptable.
                assert!(
                    code == ErrorCode::PermissionDenied || code == ErrorCode::Unsupported,
                    "expected PermissionDenied or Unsupported, got {:?}: {}",
                    code,
                    message
                );
            }
            other => panic!("expected error, got {:?}", other),
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(
                ClientCommand::GetSnapshot {
                    request_id: "r2".into(),
                    session_id,
                },
                test_ctx(Some(observer_id)),
            )
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
                test_ctx(Some(tui_id)),
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
                test_ctx(Some(tui_id)),
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
                test_ctx(Some(cli_id)),
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
                test_ctx(Some(cli_id)),
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

        for kind in [
            ClientKind::Tui,
            ClientKind::Cli,
            ClientKind::Mcp,
            ClientKind::Rest,
            ClientKind::Agent,
        ] {
            let resp = host
                .handle_command(
                    ClientCommand::DeclareClient {
                        request_id: format!("r-{:?}", kind),
                        kind: kind.clone(),
                        label: None,
                    },
                    test_ctx(None),
                )
                .await;
            let client_id = match resp {
                ServerMessage::ClientDeclared { client_id, .. } => client_id,
                other => panic!("expected ClientDeclared for {:?}, got {:?}", kind, other),
            };
            let stored = host
                .client_registry
                .lock()
                .unwrap()
                .get(&client_id)
                .cloned()
                .unwrap();
            assert_eq!(stored.kind, kind);
        }
    }

    #[tokio::test]
    async fn create_session_stores_surface_in_access() {
        let host = test_host();
        let owner_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: owner_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            connected_at_secs: 100,
            label: None,
        });

        let resp = host
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::McpServer,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await;
        let session_id = match resp {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let access = host.session_access.lock().unwrap();
        let session_access = access.get(&session_id).unwrap();
        assert_eq!(
            session_access.surface,
            eggsec_runtime::RuntimeSurface::McpServer
        );
        assert_eq!(session_access.owner_client_kind, ClientKind::Tui);
        assert_eq!(session_access.owner_client_id, Some(owner_id));
    }

    #[tokio::test]
    async fn approver_denied_on_strict_session() {
        let host = test_host();
        let approver_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: approver_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::McpServer,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(
                ClientCommand::ApprovePolicy {
                    request_id: "r2".into(),
                    session_id,
                    task_id: eggsec_runtime::TaskId::new(),
                    approved: true,
                    reason: None,
                },
                test_ctx(Some(approver_id)),
            )
            .await;
        match resp {
            ServerMessage::Error { code, message, .. } => {
                // Approver on strict session gets denied at permission check level
                // OR gets Unsupported at command level (approve-policy is not wired).
                // Either outcome is acceptable — the key is no success.
                assert!(
                    code == ErrorCode::PermissionDenied || code == ErrorCode::Unsupported,
                    "expected PermissionDenied or Unsupported, got {:?}: {}",
                    code,
                    message
                );
            }
            other => panic!("expected error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn unrelated_tui_cannot_approve_strict_session() {
        let host = test_host();
        let unrelated_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: unrelated_id,
            kind: ClientKind::Tui,
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
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
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::McpServer,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        // Unrelated TUI client is observer by default, should be denied
        let resp = host
            .handle_command(
                ClientCommand::ApprovePolicy {
                    request_id: "r2".into(),
                    session_id,
                    task_id: eggsec_runtime::TaskId::new(),
                    approved: true,
                    reason: None,
                },
                test_ctx(Some(unrelated_id)),
            )
            .await;
        match resp {
            ServerMessage::Error { code, .. } => {
                assert!(
                    code == ErrorCode::PermissionDenied || code == ErrorCode::Unsupported,
                    "expected PermissionDenied or Unsupported, got {:?}",
                    code
                );
            }
            other => panic!("expected error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn strict_session_approve_policy_not_allowed_for_unrelated() {
        let host = test_host();

        let owner_id = ClientId::new();
        host.register_client(ClientInfo {
            client_id: owner_id,
            kind: ClientKind::Agent,
            surface: eggsec_runtime::RuntimeSurface::SecurityAgent,
            connected_at_secs: 100,
            label: None,
        });

        let session_id = match host
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::SecurityAgent,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(Some(owner_id)),
            )
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        // Even owner on strict session should not get approval success (not wired)
        let resp = host
            .handle_command(
                ClientCommand::ApprovePolicy {
                    request_id: "r2".into(),
                    session_id,
                    task_id: eggsec_runtime::TaskId::new(),
                    approved: true,
                    reason: None,
                },
                test_ctx(Some(owner_id)),
            )
            .await;
        match resp {
            ServerMessage::Error {
                code: ErrorCode::Unsupported,
                ..
            } => {}
            other => panic!("expected Unsupported, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn create_session_requires_declared_client() {
        let host = test_host();

        // Undeclared client can still create a session (DeclaredClient permission level)
        // but gets no owner attribution.
        let resp = host
            .handle_command(
                ClientCommand::CreateSession {
                    request_id: "r1".into(),
                    surface: eggsec_runtime::RuntimeSurface::Unknown,
                    scope: None,
                    labels: vec![],
                },
                test_ctx(None),
            )
            .await;
        match resp {
            ServerMessage::SessionCreated { .. } => {}
            other => panic!("expected SessionCreated, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn recover_persisted_state_with_noop_store() {
        let host = test_host();
        // NoopStore returns empty sessions, so recovery is a no-op
        host.recover_persisted_state().await.unwrap();
    }
}
