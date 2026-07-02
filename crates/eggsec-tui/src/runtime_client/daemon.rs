use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eggsec_runtime::{
    request::RuntimeSurface, session::SessionScope, RunRequest, RuntimeCapabilities, SessionId,
    SessionSnapshot, SessionSummary, TaskId,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{oneshot, Mutex};

use eggsec_daemon::protocol::{ClientCommand, ServerMessage};

use super::{RuntimeClientFuture, RuntimeEventReceiverHandle, TuiRuntimeClient};

#[derive(Debug, thiserror::Error)]
pub enum DaemonClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Connection closed")]
    ConnectionClosed,
}

struct DaemonConnection {
    _socket_path: String,
    request_counter: AtomicU64,
    writer: Mutex<tokio::net::unix::OwnedWriteHalf>,
    pending_responses: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, oneshot::Sender<ServerMessage>>>,
    >,
}

/// Runtime client that connects to a local daemon via Unix socket.
#[derive(Clone)]
pub struct DaemonRuntimeClient {
    inner: Arc<DaemonConnection>,
}

impl DaemonRuntimeClient {
    /// Connect to a daemon at the given socket path.
    pub async fn connect(socket_path: &str) -> Result<Self, DaemonClientError> {
        let stream = UnixStream::connect(socket_path).await?;
        let (read_half, write_half) = stream.into_split();

        let pending_responses: std::sync::Arc<
            std::sync::Mutex<std::collections::HashMap<String, oneshot::Sender<ServerMessage>>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

        let pending_clone = pending_responses.clone();

        // Spawn reader task to route responses to pending senders
        tokio::spawn(async move {
            let mut reader = BufReader::new(read_half).lines();
            loop {
                match reader.next_line().await {
                    Ok(Some(line)) => {
                        if line.is_empty() {
                            continue;
                        }
                        let msg: ServerMessage = match serde_json::from_str(&line) {
                            Ok(msg) => msg,
                            Err(e) => {
                                tracing::warn!("Failed to parse daemon message: {}", e);
                                continue;
                            }
                        };
                        if let ServerMessage::RuntimeEvent { .. } = &msg {
                            // RuntimeEvents are routed via subscribe-specific channels
                            tracing::debug!("Received runtime event from daemon");
                        } else {
                            let request_id = match &msg {
                                ServerMessage::Ok { request_id }
                                | ServerMessage::Error { request_id, .. }
                                | ServerMessage::SessionCreated { request_id, .. }
                                | ServerMessage::Sessions { request_id, .. }
                                | ServerMessage::Snapshot { request_id, .. }
                                | ServerMessage::TaskSubmitted { request_id, .. }
                                | ServerMessage::Capabilities { request_id, .. }
                                | ServerMessage::Health { request_id, .. } => request_id.clone(),
                                ServerMessage::RuntimeEvent { .. } => unreachable!(),
                            };
                            let mut pending = pending_clone.lock().unwrap();
                            if let Some(sender) = pending.remove(&request_id) {
                                let _ = sender.send(msg);
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::info!("Daemon connection closed");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("Daemon read error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            inner: Arc::new(DaemonConnection {
                _socket_path: socket_path.to_string(),
                request_counter: AtomicU64::new(0),
                writer: Mutex::new(write_half),
                pending_responses,
            }),
        })
    }

    fn next_request_id(&self) -> String {
        let id = self.inner.request_counter.fetch_add(1, Ordering::Relaxed);
        format!("tui-{}", id)
    }

    async fn send_command(&self, cmd: ClientCommand) -> Result<ServerMessage, DaemonClientError> {
        let request_id = match &cmd {
            ClientCommand::Health { request_id }
            | ClientCommand::Capabilities { request_id }
            | ClientCommand::CreateSession { request_id, .. }
            | ClientCommand::ListSessions { request_id }
            | ClientCommand::GetSnapshot { request_id, .. }
            | ClientCommand::SubmitTask { request_id, .. }
            | ClientCommand::CancelTask { request_id, .. }
            | ClientCommand::CancelActive { request_id, .. }
            | ClientCommand::Subscribe { request_id, .. } => request_id.clone(),
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.inner.pending_responses.lock().unwrap();
            pending.insert(request_id, tx);
        }

        let json = serde_json::to_string(&cmd)?;
        {
            let mut writer = self.inner.writer.lock().await;
            writer.write_all(json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| DaemonClientError::Protocol("response timeout".into()))?
            .map_err(|_| DaemonClientError::ConnectionClosed)
    }

    /// Subscribe to runtime events for a session.
    pub async fn subscribe_events(
        &self,
        session_id: SessionId,
    ) -> Result<RuntimeEventReceiverHandle, DaemonClientError> {
        let cmd = ClientCommand::Subscribe {
            request_id: self.next_request_id(),
            session_id,
        };

        let json = serde_json::to_string(&cmd)?;
        {
            let mut writer = self.inner.writer.lock().await;
            writer.write_all(json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        // For MVP, return a receiver that will get events from the main reader task.
        // The main reader task needs to be enhanced to route RuntimeEvent messages
        // to session-specific channels. For now, rely on snapshot polling for daemon mode.
        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tracing::info!("Subscribed to daemon events for session {}", session_id);
        Ok(RuntimeEventReceiverHandle::new(rx))
    }
}

impl TuiRuntimeClient for DaemonRuntimeClient {
    fn capabilities(&self) -> RuntimeClientFuture<RuntimeCapabilities> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::Capabilities {
                request_id: client.next_request_id(),
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Capabilities { capabilities, .. }) => Ok(capabilities),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn create_session(
        &self,
        surface: RuntimeSurface,
        scope: Option<SessionScope>,
        labels: Vec<String>,
    ) -> RuntimeClientFuture<SessionId> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::CreateSession {
                request_id: client.next_request_id(),
                surface,
                scope,
                labels,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::SessionCreated { session_id, .. }) => Ok(session_id),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn list_sessions(&self) -> RuntimeClientFuture<Vec<SessionSummary>> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::ListSessions {
                request_id: client.next_request_id(),
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Sessions { sessions, .. }) => Ok(sessions),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn snapshot(&self, session_id: SessionId) -> RuntimeClientFuture<SessionSnapshot> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::GetSnapshot {
                request_id: client.next_request_id(),
                session_id,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Snapshot { snapshot, .. }) => Ok(snapshot),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn submit(&self, session_id: SessionId, request: RunRequest) -> RuntimeClientFuture<TaskId> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::SubmitTask {
                request_id: client.next_request_id(),
                session_id,
                request,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::TaskSubmitted { task_id, .. }) => Ok(task_id),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn cancel(&self, session_id: SessionId, task_id: TaskId) -> RuntimeClientFuture<()> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::CancelTask {
                request_id: client.next_request_id(),
                session_id,
                task_id,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Ok { .. }) => Ok(()),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn cancel_active(&self, session_id: SessionId) -> RuntimeClientFuture<()> {
        let client = self.clone();
        Box::pin(async move {
            let cmd = ClientCommand::CancelActive {
                request_id: client.next_request_id(),
                session_id,
            };
            match client.send_command(cmd).await {
                Ok(ServerMessage::Ok { .. }) => Ok(()),
                Ok(ServerMessage::Error { message, .. }) => Err(message),
                Ok(msg) => Err(format!("unexpected response: {:?}", msg)),
                Err(e) => Err(e.to_string()),
            }
        })
    }

    fn subscribe(&self, session_id: SessionId) -> RuntimeClientFuture<RuntimeEventReceiverHandle> {
        let client = self.clone();
        Box::pin(async move {
            client
                .subscribe_events(session_id)
                .await
                .map_err(|e| e.to_string())
        })
    }
}
