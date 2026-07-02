use std::collections::HashMap;
use std::sync::Arc;

use eggsec_runtime::{ClientId, Runtime, RuntimeConfig, RuntimeError, RuntimeTaskExecutor};

use crate::client_registry::{ClientInfo, ClientKind, ClientRegistry, ClientRole, SessionAccess};
use crate::config::DaemonConfig;
use crate::protocol::{ClientCommand, ErrorCode, ServerMessage};

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
    connected_clients: std::sync::Mutex<HashMap<eggsec_runtime::ClientId, ClientRole>>,
}

impl DaemonHost {
    pub fn new(config: DaemonConfig, executor: impl RuntimeTaskExecutor) -> Self {
        let runtime = Runtime::new(RuntimeConfig::default(), executor);
        Self {
            runtime: Arc::new(runtime),
            config,
            client_registry: std::sync::Mutex::new(ClientRegistry::new()),
            session_access: std::sync::Mutex::new(HashMap::new()),
            connected_clients: std::sync::Mutex::new(HashMap::new()),
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
    pub async fn handle_command(&self, cmd: ClientCommand) -> ServerMessage {
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
                            owner_client_id: None,
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

            ClientCommand::Subscribe { request_id, .. } => ServerMessage::Error {
                request_id,
                code: ErrorCode::Internal,
                message: "subscribe is handled at the transport level".into(),
            },
        }
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

    #[tokio::test]
    async fn handle_health() {
        let host = test_host();
        let resp = host
            .handle_command(ClientCommand::Health {
                request_id: "req-1".into(),
            })
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
            .handle_command(ClientCommand::Capabilities {
                request_id: "req-2".into(),
            })
            .await;
        match resp {
            ServerMessage::Capabilities {
                request_id,
                capabilities,
            } => {
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
            })
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
            .handle_command(ClientCommand::ListSessions {
                request_id: "req-4".into(),
            })
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

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            })
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
            })
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

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            })
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::GetSnapshot {
                request_id: "r2".into(),
                session_id,
            })
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

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            })
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::CancelActive {
                request_id: "r2".into(),
                session_id,
            })
            .await;
        match resp {
            ServerMessage::Ok { request_id } => {
                assert_eq!(request_id, "r2");
            }
            other => panic!("expected Ok, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_session_not_found() {
        let host = test_host();
        let fake_id = eggsec_runtime::SessionId::new();
        let resp = host
            .handle_command(ClientCommand::GetSnapshot {
                request_id: "r1".into(),
                session_id: fake_id,
            })
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
        use crate::client_registry::ClientKind;

        let host = test_host();
        let resp = host
            .handle_command(ClientCommand::DeclareClient {
                request_id: "r1".into(),
                kind: ClientKind::Tui,
                label: Some("my-tui".into()),
            })
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

        let session_id = match host
            .handle_command(ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            })
            .await
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };

        let resp = host
            .handle_command(ClientCommand::CloseSession {
                request_id: "r2".into(),
                session_id,
            })
            .await;
        match resp {
            ServerMessage::SessionClosed { request_id } => {
                assert_eq!(request_id, "r2");
            }
            other => panic!("expected SessionClosed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_close_session_not_found() {
        let host = test_host();
        let fake_id = eggsec_runtime::SessionId::new();
        let resp = host
            .handle_command(ClientCommand::CloseSession {
                request_id: "r1".into(),
                session_id: fake_id,
            })
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
        use crate::client_registry::ClientKind;

        let host = test_host();
        let resp = host
            .handle_command(ClientCommand::DeclareClient {
                request_id: "r1".into(),
                kind: ClientKind::Agent,
                label: Some("agent-1".into()),
            })
            .await;
        let client_id = match resp {
            ServerMessage::ClientDeclared { client_id, .. } => client_id,
            _ => panic!("expected ClientDeclared"),
        };
        let stored = host.client_registry.lock().unwrap().get(&client_id).cloned();
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().kind, ClientKind::Agent);
    }
}
