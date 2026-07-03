//! Daemon client library for communicating with a running eggsec daemon.
//!
//! Connects over Unix domain socket using the JSON line protocol.
//! Each command is sent as a single JSON line followed by `\n`.
//! Responses are read as single JSON lines.

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::protocol::{ClientCommand, ServerMessage};

type Lines = tokio::io::Lines<BufReader<tokio::net::unix::OwnedReadHalf>>;

/// A client connected to an eggsec daemon over Unix socket.
pub struct DaemonClient {
    write_half: tokio::net::unix::OwnedWriteHalf,
    read_lines: Option<Lines>,
}

impl DaemonClient {
    /// Connect to a daemon at the given Unix socket path.
    pub async fn connect(socket_path: &str) -> Result<Self> {
        let stream = UnixStream::connect(socket_path).await.map_err(|e| {
            anyhow::anyhow!("Failed to connect to daemon at {}: {}", socket_path, e)
        })?;
        let (read_half, write_half) = stream.into_split();
        let read_lines = BufReader::new(read_half).lines();
        Ok(Self {
            write_half,
            read_lines: Some(read_lines),
        })
    }

    /// Send a command and wait for the response.
    async fn send_command(&mut self, cmd: ClientCommand) -> Result<ServerMessage> {
        let json = serde_json::to_string(&cmd)?;
        self.write_half.write_all(json.as_bytes()).await?;
        self.write_half.write_all(b"\n").await?;
        self.write_half.flush().await?;

        let lines = self
            .read_lines
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("client is in subscribe mode"))?;
        let line = lines
            .next_line()
            .await?
            .ok_or_else(|| anyhow::anyhow!("daemon connection closed"))?;
        let msg: ServerMessage = serde_json::from_str(&line)?;
        Ok(msg)
    }

    /// Check daemon health.
    pub async fn health(&mut self) -> Result<ServerMessage> {
        self.send_command(ClientCommand::Health {
            request_id: uuid::Uuid::new_v4().to_string(),
        })
        .await
    }

    /// Declare the client type and label to the daemon.
    pub async fn declare_client(
        &mut self,
        kind: crate::client_registry::ClientKind,
        label: Option<String>,
    ) -> Result<ServerMessage> {
        self.send_command(ClientCommand::DeclareClient {
            request_id: uuid::Uuid::new_v4().to_string(),
            kind,
            label,
        })
        .await
    }

    /// List all active sessions.
    pub async fn list_sessions(&mut self) -> Result<ServerMessage> {
        self.send_command(ClientCommand::ListSessions {
            request_id: uuid::Uuid::new_v4().to_string(),
        })
        .await
    }

    /// Create a new session.
    pub async fn create_session(
        &mut self,
        surface: eggsec_runtime::RuntimeSurface,
        scope: Option<eggsec_runtime::SessionScope>,
        labels: Vec<String>,
    ) -> Result<ServerMessage> {
        self.send_command(ClientCommand::CreateSession {
            request_id: uuid::Uuid::new_v4().to_string(),
            surface,
            scope,
            labels,
        })
        .await
    }

    /// Get a snapshot of a session.
    pub async fn get_snapshot(
        &mut self,
        session_id: eggsec_runtime::SessionId,
    ) -> Result<ServerMessage> {
        self.send_command(ClientCommand::GetSnapshot {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id,
        })
        .await
    }

    /// Submit a task to a session.
    pub async fn submit_task(
        &mut self,
        session_id: eggsec_runtime::SessionId,
        request: eggsec_runtime::RunRequest,
    ) -> Result<ServerMessage> {
        self.send_command(ClientCommand::SubmitTask {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id,
            request,
        })
        .await
    }

    /// Cancel a specific task.
    pub async fn cancel_task(
        &mut self,
        session_id: eggsec_runtime::SessionId,
        task_id: eggsec_runtime::TaskId,
    ) -> Result<ServerMessage> {
        self.send_command(ClientCommand::CancelTask {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id,
            task_id,
        })
        .await
    }

    /// Close a session.
    pub async fn close_session(
        &mut self,
        session_id: eggsec_runtime::SessionId,
    ) -> Result<ServerMessage> {
        self.send_command(ClientCommand::CloseSession {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id,
        })
        .await
    }

    /// List all persisted sessions from the daemon store.
    pub async fn list_persisted_sessions(&mut self) -> Result<ServerMessage> {
        self.send_command(ClientCommand::ListPersistedSessions {
            request_id: uuid::Uuid::new_v4().to_string(),
        })
        .await
    }

    /// Get a persisted snapshot by session ID from the daemon store.
    pub async fn get_persisted_snapshot(
        &mut self,
        session_id: eggsec_runtime::SessionId,
    ) -> Result<ServerMessage> {
        self.send_command(ClientCommand::GetPersistedSnapshot {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id,
        })
        .await
    }

    /// Subscribe to events for a session. Returns an event receiver.
    ///
    /// The subscribe command sends an OK acknowledgement, then the daemon
    /// streams `RuntimeEvent` messages for the given session.
    pub async fn subscribe(
        &mut self,
        session_id: eggsec_runtime::SessionId,
    ) -> Result<tokio::sync::mpsc::Receiver<eggsec_runtime::RuntimeEvent>> {
        self.send_command(ClientCommand::Subscribe {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id,
        })
        .await?;

        // Take ownership of the read_lines for streaming.
        let mut read_lines = self
            .read_lines
            .take()
            .ok_or_else(|| anyhow::anyhow!("already subscribed"))?;
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        tokio::spawn(async move {
            while let Ok(Some(line)) = read_lines.next_line().await {
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<ServerMessage>(&line) {
                    Ok(ServerMessage::RuntimeEvent { event, .. }) => {
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });
        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DaemonConfig;
    use crate::host::DaemonHost;
    use crate::server::run_server;
    use eggsec_runtime::{
        CancellationToken, RuntimeEventSink, RuntimeTaskExecutor, TaskId, TaskOutcome,
    };
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    struct TestExecutor;

    impl RuntimeTaskExecutor for TestExecutor {
        fn execute(
            &self,
            _task_id: TaskId,
            _request: eggsec_runtime::RunRequest,
            _sink: RuntimeEventSink,
            _cancel: CancellationToken,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<TaskOutcome, eggsec_runtime::RuntimeError>>
                    + Send
                    + 'static,
            >,
        > {
            Box::pin(async { Ok(TaskOutcome::Text("test-result".into())) })
        }
    }

    async fn start_server() -> (String, CancellationToken) {
        let socket_path = format!("/tmp/eggsec-client-test-{}.sock", uuid::Uuid::new_v4());
        let config = DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        };
        let host = Arc::new(DaemonHost::new(
            config,
            TestExecutor,
            crate::store::noop_store(),
        ));
        let shutdown = CancellationToken::new();

        let host_clone = host.clone();
        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            run_server(host_clone, shutdown_clone).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        (socket_path, shutdown)
    }

    #[tokio::test]
    async fn client_health_roundtrip() {
        let (socket_path, shutdown) = start_server().await;
        let mut client = DaemonClient::connect(&socket_path).await.unwrap();
        let resp = client.health().await.unwrap();
        match resp {
            ServerMessage::Health {
                status, version, ..
            } => {
                assert_eq!(status, "ok");
                assert!(!version.is_empty());
            }
            other => panic!("expected Health, got {:?}", other),
        }
        shutdown.cancel();
    }

    #[tokio::test]
    async fn client_create_session_roundtrip() {
        let (socket_path, shutdown) = start_server().await;
        let mut client = DaemonClient::connect(&socket_path).await.unwrap();
        let resp = client
            .create_session(eggsec_runtime::RuntimeSurface::CliManual, None, vec![])
            .await
            .unwrap();
        match resp {
            ServerMessage::SessionCreated { .. } => {}
            other => panic!("expected SessionCreated, got {:?}", other),
        }
        shutdown.cancel();
    }

    #[tokio::test]
    async fn client_connect_failure() {
        let result = DaemonClient::connect("/tmp/nonexistent-daemon.sock").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn client_declare_client_roundtrip() {
        let (socket_path, shutdown) = start_server().await;
        let mut client = DaemonClient::connect(&socket_path).await.unwrap();
        let resp = client
            .declare_client(
                crate::client_registry::ClientKind::Tui,
                Some("my-tui".into()),
            )
            .await
            .unwrap();
        match resp {
            ServerMessage::ClientDeclared {
                request_id,
                client_id,
            } => {
                assert!(!request_id.is_empty());
                let _ = client_id;
            }
            other => panic!("expected ClientDeclared, got {:?}", other),
        }
        shutdown.cancel();
    }

    #[tokio::test]
    async fn client_close_session_roundtrip() {
        let (socket_path, shutdown) = start_server().await;
        let mut client = DaemonClient::connect(&socket_path).await.unwrap();
        // Declare client first so subsequent commands have a client_id.
        let _decl = client
            .declare_client(
                crate::client_registry::ClientKind::Cli,
                Some("test-cli".into()),
            )
            .await
            .unwrap();
        let session_id = match client
            .create_session(eggsec_runtime::RuntimeSurface::CliManual, None, vec![])
            .await
            .unwrap()
        {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected SessionCreated"),
        };
        let resp = client.close_session(session_id).await.unwrap();
        match resp {
            ServerMessage::SessionClosed { .. } => {}
            other => panic!("expected SessionClosed, got {:?}", other),
        }
        shutdown.cancel();
    }
}
