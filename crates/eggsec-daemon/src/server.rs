use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::error::DaemonError;
use crate::host::DaemonHost;
use crate::protocol::{
    ClientCommand, DaemonRequestContext, ErrorCode, ServerMessage, TransportKind,
};

/// RAII guard that cancels a `CancellationToken` when dropped.
struct CancelOnDrop(CancellationToken);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// Run the Unix socket server, dispatching commands to the host.
///
/// Listens on the path from `host.config().socket_path`. Accepts
/// connections in a loop, reading one JSON line per command and writing
/// one JSON line per response. The loop exits when `shutdown` is
/// cancelled.
pub async fn run_server(
    host: Arc<DaemonHost>,
    shutdown: CancellationToken,
) -> Result<(), DaemonError> {
    let socket_path = &host.config().socket_path;
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;

    let max_clients = host.config().max_clients;
    let semaphore = Arc::new(Semaphore::new(max_clients));
    tracing::info!(
        "Daemon listening on {} (max_clients={})",
        socket_path,
        max_clients
    );

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        let permit = match semaphore.clone().try_acquire_owned() {
                            Ok(permit) => permit,
                            Err(_) => {
                                tracing::warn!("Max clients ({}) reached, rejecting connection", max_clients);
                                // Drop the stream — client gets a connection reset.
                                drop(stream);
                                continue;
                            }
                        };
                        let host = host.clone();
                        tokio::spawn(handle_client(host, stream, permit));
                    }
                    Err(e) => tracing::error!("Accept error: {}", e),
                }
            }
            _ = shutdown.cancelled() => {
                tracing::info!("Shutdown signal received");
                break;
            }
        }
    }

    let _ = std::fs::remove_file(socket_path);
    Ok(())
}

/// Extract the session_id from a `RuntimeEvent`.
pub(crate) fn event_session_id(
    event: &eggsec_runtime::RuntimeEvent,
) -> Option<&eggsec_runtime::SessionId> {
    use eggsec_runtime::RuntimeEvent::*;
    match event {
        SessionCreated { session_id }
        | Snapshot { session_id, .. }
        | TaskQueued { session_id, .. }
        | TaskStarted { session_id, .. }
        | TaskProgress { session_id, .. }
        | TaskLog { session_id, .. }
        | PolicyDecisionRequired { session_id, .. }
        | TaskCompleted { session_id, .. }
        | TaskFailed { session_id, .. }
        | TaskCancelled { session_id, .. }
        | Audit { session_id, .. }
        | SessionClosed { session_id } => Some(session_id),
    }
}

/// Handle a single client connection.
///
/// Reads JSON lines from the client, dispatches each as a `ClientCommand`,
/// and writes the corresponding `ServerMessage` as a JSON line. The loop
/// exits when the client disconnects (read returns `None` or error).
///
/// The `_permit` argument is an `OwnedSemaphorePermit` that is held for the
/// lifetime of the connection. When it is dropped (client disconnects or
/// handler returns), the permit is automatically released back to the
/// semaphore.
async fn handle_client(
    host: Arc<DaemonHost>,
    stream: tokio::net::UnixStream,
    _permit: tokio::sync::OwnedSemaphorePermit,
) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half).lines();

    let mut local_client_id: Option<eggsec_runtime::ClientId> = None;

    let make_ctx = |client_id: Option<eggsec_runtime::ClientId>| DaemonRequestContext {
        client_id,
        peer: None,
        transport: TransportKind::UnixSocket,
    };

    loop {
        let line = match reader.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(e) => {
                tracing::warn!("Read error: {}", e);
                break;
            }
        };

        if line.is_empty() {
            continue;
        }

        let cmd: ClientCommand = match serde_json::from_str(&line) {
            Ok(cmd) => cmd,
            Err(e) => {
                let err_resp = ServerMessage::Error {
                    request_id: String::new(),
                    code: ErrorCode::InvalidRequest,
                    message: format!("invalid command: {}", e),
                };
                if write_message(&mut write_half, &err_resp).await.is_err() {
                    break;
                }
                continue;
            }
        };

        // If this is a DeclareClient, capture the returned client_id for this connection.
        if let ClientCommand::DeclareClient { .. } = &cmd {
            let resp = host.handle_command(cmd, make_ctx(local_client_id)).await;
            if let ServerMessage::ClientDeclared { client_id, .. } = &resp {
                local_client_id = Some(*client_id);
            }
            if write_message(&mut write_half, &resp).await.is_err() {
                break;
            }
            continue;
        }

        // Handle Subscribe specially — it starts a long-lived event stream.
        if let ClientCommand::Subscribe {
            request_id,
            session_id,
        } = cmd
        {
            // Acknowledge the subscribe request
            let ack = ServerMessage::Ok { request_id };
            if write_message(&mut write_half, &ack).await.is_err() {
                break;
            }

            let mut receiver = host.runtime().subscribe().await;
            let cancel = CancellationToken::new();
            let cancel_clone = cancel.clone();
            let _cancel_guard = CancelOnDrop(cancel);

            // Streaming loop: forward matching events and handle further commands.
            // write_half is moved into this block — this function returns after.
            loop {
                tokio::select! {
                    event = receiver.recv() => {
                        match event {
                            Some(event) => {
                                if event_session_id(&event) == Some(&session_id) {
                                    let msg = ServerMessage::RuntimeEvent {
                                        session_id,
                                        event,
                                    };
                                    if write_message(&mut write_half, &msg).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                    line = reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                if line.is_empty() {
                                    continue;
                                }
                                let cmd: ClientCommand = match serde_json::from_str(&line) {
                                    Ok(cmd) => cmd,
                                    Err(e) => {
                                        let err_resp = ServerMessage::Error {
                                            request_id: String::new(),
                                            code: ErrorCode::InvalidRequest,
                                            message: format!("invalid command: {}", e),
                                        };
                                        if write_message(&mut write_half, &err_resp).await.is_err() {
                                            break;
                                        }
                                        continue;
                                    }
                                };
                                let resp = host.handle_command(cmd, make_ctx(local_client_id)).await;
                                if write_message(&mut write_half, &resp).await.is_err() {
                                    break;
                                }
                            }
                            Ok(None) => break,
                            Err(e) => {
                                tracing::warn!("Read error during subscribe: {}", e);
                                break;
                            }
                        }
                    }
                    _ = cancel_clone.cancelled() => {
                        break;
                    }
                }
            }

            return;
        }

        // Non-subscribe commands: dispatch and respond inline
        let resp = host.handle_command(cmd, make_ctx(local_client_id)).await;
        if write_message(&mut write_half, &resp).await.is_err() {
            break;
        }
    }
}

/// Write a `ServerMessage` as a single JSON line followed by `\n`.
async fn write_message(
    writer: &mut (impl AsyncWriteExt + Unpin),
    msg: &ServerMessage,
) -> Result<(), DaemonError> {
    let mut json = serde_json::to_string(msg)?;
    json.push('\n');
    writer.write_all(json.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DaemonConfig;
    use crate::host::DaemonHost;
    use crate::protocol::{ClientCommand, ErrorCode, ServerMessage};
    use eggsec_runtime::{
        CancellationToken, RuntimeError, RuntimeEventSink, RuntimeTaskExecutor, TaskId, TaskOutcome,
    };
    use std::future::Future;
    use std::pin::Pin;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

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

    async fn start_server() -> (Arc<DaemonHost>, String, CancellationToken) {
        let socket_path = format!("/tmp/eggsec-test-{}.sock", uuid::Uuid::new_v4());
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
        (host, socket_path, shutdown)
    }

    async fn connect(
        socket_path: &str,
    ) -> (
        tokio::net::unix::OwnedWriteHalf,
        tokio::io::Lines<BufReader<tokio::net::unix::OwnedReadHalf>>,
    ) {
        loop {
            match UnixStream::connect(socket_path).await {
                Ok(stream) => {
                    let (read_half, write_half) = stream.into_split();
                    let reader = BufReader::new(read_half).lines();
                    return (write_half, reader);
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        }
    }

    async fn send_command(
        write_half: &mut tokio::net::unix::OwnedWriteHalf,
        read_lines: &mut tokio::io::Lines<BufReader<tokio::net::unix::OwnedReadHalf>>,
        cmd: &ClientCommand,
    ) -> ServerMessage {
        let json = serde_json::to_string(cmd).unwrap();
        write_half.write_all(json.as_bytes()).await.unwrap();
        write_half.write_all(b"\n").await.unwrap();
        write_half.flush().await.unwrap();

        let line = read_lines
            .next_line()
            .await
            .unwrap()
            .expect("expected response");
        serde_json::from_str(&line).unwrap()
    }

    #[tokio::test]
    async fn server_health_roundtrip() {
        let (_host, socket_path, shutdown) = start_server().await;
        let (mut write_half, mut read_lines) = connect(&socket_path).await;

        let resp = send_command(
            &mut write_half,
            &mut read_lines,
            &ClientCommand::Health {
                request_id: "test-1".into(),
            },
        )
        .await;

        match resp {
            ServerMessage::Health {
                request_id,
                status,
                version,
                protocol_version,
            } => {
                assert_eq!(request_id, "test-1");
                assert_eq!(status, "ok");
                assert!(!version.is_empty());
                assert_eq!(protocol_version, crate::protocol::DAEMON_PROTOCOL_VERSION);
            }
            other => panic!("expected Health, got {:?}", other),
        }

        shutdown.cancel();
    }

    #[tokio::test]
    async fn server_create_and_list_sessions() {
        let (_host, socket_path, shutdown) = start_server().await;
        let (mut write_half, mut read_lines) = connect(&socket_path).await;

        let resp = send_command(
            &mut write_half,
            &mut read_lines,
            &ClientCommand::CreateSession {
                request_id: "r1".into(),
                surface: eggsec_runtime::RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            },
        )
        .await;
        let session_id = match resp {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            other => panic!("expected SessionCreated, got {:?}", other),
        };

        let resp = send_command(
            &mut write_half,
            &mut read_lines,
            &ClientCommand::ListSessions {
                request_id: "r2".into(),
            },
        )
        .await;
        match resp {
            ServerMessage::Sessions { sessions, .. } => {
                assert!(sessions.iter().any(|s| s.session_id == session_id));
            }
            other => panic!("expected Sessions, got {:?}", other),
        }

        shutdown.cancel();
    }

    #[tokio::test]
    async fn server_invalid_json_returns_error() {
        let (_host, socket_path, shutdown) = start_server().await;
        let (mut write_half, mut read_lines) = connect(&socket_path).await;

        write_half.write_all(b"not json\n").await.unwrap();
        write_half.flush().await.unwrap();

        let line = read_lines
            .next_line()
            .await
            .unwrap()
            .expect("expected error response");
        let resp: ServerMessage = serde_json::from_str(&line).unwrap();

        match resp {
            ServerMessage::Error { code, message, .. } => {
                assert_eq!(code, ErrorCode::InvalidRequest);
                assert!(message.contains("invalid command"));
            }
            other => panic!("expected Error, got {:?}", other),
        }

        shutdown.cancel();
    }

    #[tokio::test]
    async fn server_shutdown_signal() {
        let (_host, socket_path, shutdown) = start_server().await;
        let (_write_half, _read_lines) = connect(&socket_path).await;

        assert!(std::path::Path::new(&socket_path).exists());

        shutdown.cancel();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert!(!std::path::Path::new(&socket_path).exists());
    }

    #[tokio::test]
    async fn subscribe_receives_task_events() {
        use crate::client_registry::ClientKind;
        use crate::protocol::ClientCommand;
        use eggsec_runtime::request::{PortScanParams, RunRequest, TaskKind};
        use eggsec_runtime::{ClientId, RuntimeSurface};

        let (_host, socket_path, shutdown) = start_server().await;
        let (mut write_half, mut read_lines) = connect(&socket_path).await;

        // 0. Declare a client so permission checks pass.
        let _decl = send_command(
            &mut write_half,
            &mut read_lines,
            &ClientCommand::DeclareClient {
                request_id: "d1".into(),
                kind: ClientKind::Cli,
                label: Some("test-cli".into()),
            },
        )
        .await;

        // 1. Create a session.
        let create_resp = send_command(
            &mut write_half,
            &mut read_lines,
            &ClientCommand::CreateSession {
                request_id: "c1".into(),
                surface: RuntimeSurface::Unknown,
                scope: None,
                labels: vec![],
            },
        )
        .await;
        let session_id = match create_resp {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            other => panic!("expected SessionCreated, got {:?}", other),
        };

        // 2. Subscribe to events.
        let sub_resp = send_command(
            &mut write_half,
            &mut read_lines,
            &ClientCommand::Subscribe {
                request_id: "s1".into(),
                session_id,
            },
        )
        .await;
        assert!(matches!(sub_resp, ServerMessage::Ok { .. }));

        // 3. Submit a task (TestExecutor completes immediately).
        let submit_resp = send_command(
            &mut write_half,
            &mut read_lines,
            &ClientCommand::SubmitTask {
                request_id: "t1".into(),
                session_id,
                request: RunRequest {
                    task_kind: TaskKind::PortScan(PortScanParams {
                        target: "127.0.0.1".into(),
                        ports: None,
                        scan_type: None,
                        timeout_ms: None,
                    }),
                    requested_by: Some(ClientId::new()),
                    surface: RuntimeSurface::CliManual,
                    labels: vec![],
                },
            },
        )
        .await;
        let task_id = match submit_resp {
            ServerMessage::TaskSubmitted { task_id, .. } => task_id,
            other => panic!("expected TaskSubmitted, got {:?}", other),
        };

        // 4. Read messages until we get RuntimeEvent variants.
        let mut saw_queued = false;
        let mut saw_completed = false;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while tokio::time::Instant::now() < deadline {
            let read_result = tokio::time::timeout(
                std::time::Duration::from_millis(200),
                read_lines.next_line(),
            )
            .await;
            match read_result {
                Ok(Ok(Some(line))) => {
                    let msg: ServerMessage = serde_json::from_str(&line).unwrap();
                    if let ServerMessage::RuntimeEvent { event, .. } = msg {
                        match event {
                            eggsec_runtime::RuntimeEvent::TaskQueued { task_id: tid, .. } => {
                                assert_eq!(tid, task_id);
                                saw_queued = true;
                            }
                            eggsec_runtime::RuntimeEvent::TaskCompleted {
                                task_id: tid, ..
                            } => {
                                assert_eq!(tid, task_id);
                                saw_completed = true;
                            }
                            _ => {}
                        }
                    }
                    if saw_queued && saw_completed {
                        break;
                    }
                }
                _ => continue,
            }
        }
        assert!(saw_queued, "should receive TaskQueued event");
        assert!(saw_completed, "should receive TaskCompleted event");

        shutdown.cancel();
    }

    #[tokio::test]
    async fn two_subscribers_receive_same_events() {
        use crate::client_registry::ClientKind;
        use crate::protocol::ClientCommand;
        use eggsec_runtime::request::{PortScanParams, RunRequest, TaskKind};
        use eggsec_runtime::RuntimeSurface;

        let (_host, socket_path, shutdown) = start_server().await;

        // Client A: declare, create session, subscribe.
        let (mut wa, mut ra) = connect(&socket_path).await;
        let _da = send_command(
            &mut wa,
            &mut ra,
            &ClientCommand::DeclareClient {
                request_id: "da".into(),
                kind: ClientKind::Tui,
                label: Some("tui-a".into()),
            },
        )
        .await;
        let create_a = send_command(
            &mut wa,
            &mut ra,
            &ClientCommand::CreateSession {
                request_id: "ca".into(),
                surface: RuntimeSurface::TuiManual,
                scope: None,
                labels: vec![],
            },
        )
        .await;
        let session_id = match create_a {
            ServerMessage::SessionCreated { session_id, .. } => session_id,
            other => panic!("expected SessionCreated, got {:?}", other),
        };
        let sub_a = send_command(
            &mut wa,
            &mut ra,
            &ClientCommand::Subscribe {
                request_id: "sa".into(),
                session_id,
            },
        )
        .await;
        assert!(matches!(sub_a, ServerMessage::Ok { .. }));

        // Client B: declare, subscribe to same session.
        let (mut wb, mut rb) = connect(&socket_path).await;
        let _db = send_command(
            &mut wb,
            &mut rb,
            &ClientCommand::DeclareClient {
                request_id: "db".into(),
                kind: ClientKind::Cli,
                label: Some("cli-b".into()),
            },
        )
        .await;
        let sub_b = send_command(
            &mut wb,
            &mut rb,
            &ClientCommand::Subscribe {
                request_id: "sb".into(),
                session_id,
            },
        )
        .await;
        assert!(matches!(sub_b, ServerMessage::Ok { .. }));

        // Submit a task through client A.
        let submit_resp = send_command(
            &mut wa,
            &mut ra,
            &ClientCommand::SubmitTask {
                request_id: "t1".into(),
                session_id,
                request: RunRequest {
                    task_kind: TaskKind::PortScan(PortScanParams {
                        target: "127.0.0.1".into(),
                        ports: None,
                        scan_type: None,
                        timeout_ms: None,
                    }),
                    requested_by: None,
                    surface: RuntimeSurface::CliManual,
                    labels: vec![],
                },
            },
        )
        .await;
        let task_id = match submit_resp {
            ServerMessage::TaskSubmitted { task_id, .. } => task_id,
            other => panic!("expected TaskSubmitted, got {:?}", other),
        };

        // Both subscribers should receive TaskQueued and TaskCompleted.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);

        let mut a_saw_queued = false;
        let mut a_saw_completed = false;
        let mut b_saw_queued = false;
        let mut b_saw_completed = false;

        while tokio::time::Instant::now() < deadline {
            if a_saw_queued && a_saw_completed && b_saw_queued && b_saw_completed {
                break;
            }
            tokio::select! {
                line = tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    ra.next_line(),
                ) => {
                    if let Ok(Ok(Some(line))) = line {
                        if let Ok(ServerMessage::RuntimeEvent { event, .. }) = serde_json::from_str::<ServerMessage>(&line) {
                            match event {
                                eggsec_runtime::RuntimeEvent::TaskQueued { task_id: tid, .. } if tid == task_id => a_saw_queued = true,
                                eggsec_runtime::RuntimeEvent::TaskCompleted { task_id: tid, .. } if tid == task_id => a_saw_completed = true,
                                _ => {}
                            }
                        }
                    }
                }
                line = tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    rb.next_line(),
                ) => {
                    if let Ok(Ok(Some(line))) = line {
                        if let Ok(ServerMessage::RuntimeEvent { event, .. }) = serde_json::from_str::<ServerMessage>(&line) {
                            match event {
                                eggsec_runtime::RuntimeEvent::TaskQueued { task_id: tid, .. } if tid == task_id => b_saw_queued = true,
                                eggsec_runtime::RuntimeEvent::TaskCompleted { task_id: tid, .. } if tid == task_id => b_saw_completed = true,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        assert!(a_saw_queued, "client A should receive TaskQueued");
        assert!(a_saw_completed, "client A should receive TaskCompleted");
        assert!(b_saw_queued, "client B should receive TaskQueued");
        assert!(b_saw_completed, "client B should receive TaskCompleted");

        shutdown.cancel();
    }
}
