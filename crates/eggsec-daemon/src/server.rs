use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::error::DaemonError;
use crate::host::{probe_daemon_liveness, DaemonHost, DaemonLiveness, PREFLIGHT_PROBE_TIMEOUT};
use crate::protocol::{
    ClientCommand, DaemonRequestContext, ErrorCode, ServerMessage, TransportKind,
};

/// Maximum size of a single JSON-line request. Prevents a client from
/// streaming unbounded bytes without a newline and exhausting memory.
const MAX_LINE_LEN: usize = 1024 * 1024;

/// Read one `\n`-terminated line with a hard byte cap of [`MAX_LINE_LEN`].
///
/// Returns `Ok(Some(line))`, or `Ok(None)` on clean EOF. Returns an error on
/// I/O failure, invalid UTF-8, or when the peer exceeds the length cap
/// (previously `BufReader::lines()` would buffer such input without bound).
async fn read_bounded_line(
    reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
) -> std::io::Result<Option<String>> {
    let overflow = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("line exceeds maximum length of {} bytes", MAX_LINE_LEN),
        )
    };
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unterminated frame: missing newline",
                ))
            };
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(pos) => {
                if bytes.len() + pos > MAX_LINE_LEN {
                    return Err(overflow());
                }
                bytes.extend_from_slice(&available[..pos]);
                reader.consume(pos + 1);
                return String::from_utf8(bytes)
                    .map(Some)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e));
            }
            None => {
                let n = available.len();
                if bytes.len() + n > MAX_LINE_LEN {
                    return Err(overflow());
                }
                bytes.extend_from_slice(available);
                reader.consume(n);
            }
        }
    }
}

/// RAII guard that cancels a `CancellationToken` when dropped.
struct CancelOnDrop(CancellationToken);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// Classify a line that failed to deserialize as a [`ClientCommand`].
///
/// A client routes responses by request id, so a refusal MUST carry the
/// request id it was asked about. An answer with an empty request id never
/// resolves the pending request and the client waits out its timeout
/// instead of learning that its command was refused.
///
/// Two refusals are distinguished:
///
/// * `ErrorCode::UnsupportedCommand` — the line names a command this daemon
///   does not know. A newer client talking to an older daemon lands here,
///   and the client can degrade instead of failing.
/// * `ErrorCode::InvalidRequest` — the line is malformed, or it names a
///   command this daemon knows but whose payload does not fit.
///
/// The request id and the command type are recovered by a best-effort parse
/// of the raw line; when they cannot be read, the request id is empty.
fn refused_command_response(line: &str, parse_error: &serde_json::Error) -> ServerMessage {
    let parsed: serde_json::Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => {
            return ServerMessage::Error {
                request_id: String::new(),
                code: ErrorCode::InvalidRequest,
                message: format!("invalid command: {parse_error}"),
            };
        }
    };

    let request_id = parsed
        .get("request_id")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();

    let command_type = parsed.get("type").and_then(|value| value.as_str());
    match command_type {
        Some(command_type) if !ClientCommand::command_names().contains(&command_type) => {
            ServerMessage::Error {
                request_id,
                code: ErrorCode::UnsupportedCommand,
                message: format!("unsupported command: {command_type}"),
            }
        }
        _ => ServerMessage::Error {
            request_id,
            code: ErrorCode::InvalidRequest,
            message: format!("invalid command: {parse_error}"),
        },
    }
}

/// Run the Unix socket server, dispatching commands to the host.
///
/// Listens on the path from `host.config().socket_path`. Accepts
/// connections in a loop, reading one JSON line per command and writing
/// one JSON line per response. The loop exits when `shutdown` is
/// cancelled.
///
/// **The bind contract.** The server binds the socket path *without
/// unlinking it first*, and refuses when the path is held by a live daemon.
/// The sequence is:
///
/// 1. `UnixListener::bind(socket_path)`.
///    - Success → this daemon owns the path. Nothing was removed, so nothing
///      could have been displaced.
///    - Failure with `AddrInUse` → the path is occupied; go to step 2.
///    - Any other error → return it unchanged.
/// 2. Ask whether the occupant is live, with the **shared** probe
///    (`probe_daemon_liveness`) and the pre-flight's own bound. No second
///    liveness notion is introduced.
///    - `Live` → **refuse** ([`DaemonError::SocketPathHeld`]). The socket file
///      is *not* removed: the live daemon's clients depend on it.
///    - `NotLive` → the path holds no daemon; go to step 3.
/// 3. Reclaim the stale path, **once**: remove the file (ignoring
///    `NotFound`), then bind again.
///    - Success → this daemon owns the path.
///    - Failure with `AddrInUse` → **refuse**
///      ([`DaemonError::SocketPathTaken`]): another daemon took the path in
///      between. This daemon does not unlink again and does not take the
///      socket.
///    - Any other error → return it.
///
/// The residual is stated honestly: between the probe in step 2 and the
/// `remove_file` in step 3, a third actor could remove the stale path and a
/// live daemon could bind it. The window is one `remove_file` call wide.
pub async fn run_server(
    host: Arc<DaemonHost>,
    shutdown: CancellationToken,
) -> Result<(), DaemonError> {
    let socket_path = &host.config().socket_path;
    let listener = match UnixListener::bind(socket_path) {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            match probe_daemon_liveness(socket_path, PREFLIGHT_PROBE_TIMEOUT).await {
                DaemonLiveness::Live { protocol_version } => {
                    tracing::warn!(
                        path = %socket_path,
                        protocol_version,
                        "Refusing to displace the live daemon that holds the socket path"
                    );
                    return Err(DaemonError::SocketPathHeld {
                        socket_path: socket_path.clone(),
                        protocol_version,
                    });
                }
                DaemonLiveness::NotLive => {
                    // The path holds no daemon: reclaim it once, then bind.
                    match std::fs::remove_file(socket_path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => {
                            tracing::warn!(path = %socket_path, error = %error, "Failed to remove the stale daemon socket");
                            return Err(error.into());
                        }
                    }
                    match UnixListener::bind(socket_path) {
                        Ok(listener) => listener,
                        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                            tracing::warn!(
                                path = %socket_path,
                                "Another daemon took the socket path while this daemon was reclaiming it"
                            );
                            return Err(DaemonError::SocketPathTaken {
                                socket_path: socket_path.clone(),
                            });
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
            }
        }
        Err(error) => return Err(error.into()),
    };

    // Restrict the socket to its owner. With a permissive umask the socket
    // inherits world-connectable permissions, which would let any local user
    // connect and drive scans as the daemon owner.
    let permissions = std::fs::Permissions::from_mode(0o600);
    if let Err(e) = std::fs::set_permissions(socket_path, permissions) {
        tracing::warn!(
            "Failed to restrict daemon socket {} to 0600: {}",
            socket_path,
            e
        );
    }

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

    if let Err(error) = std::fs::remove_file(socket_path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(path = %socket_path, error = %error, "Failed to remove daemon socket during shutdown");
        }
    }
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
    let mut reader = BufReader::new(read_half);

    let mut local_client_id: Option<eggsec_runtime::ClientId> = None;

    let make_ctx = |client_id: Option<eggsec_runtime::ClientId>| DaemonRequestContext {
        client_id,
        peer: None,
        transport: TransportKind::UnixSocket,
    };

    loop {
        // Idle bound for the command loop: a client that connects and then
        // stalls would otherwise hold its semaphore permit indefinitely.
        const IDLE_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
        let line =
            match tokio::time::timeout(IDLE_READ_TIMEOUT, read_bounded_line(&mut reader)).await {
                Ok(Ok(Some(line))) => line,
                Ok(Ok(None)) => break,
                Ok(Err(e)) => {
                    tracing::warn!("Read error: {}", e);
                    let err_resp = ServerMessage::Error {
                        request_id: String::new(),
                        code: ErrorCode::InvalidRequest,
                        message: format!("invalid frame (max line length {} bytes)", MAX_LINE_LEN),
                    };
                    if write_message(&mut write_half, &err_resp).await.is_err() {
                        break;
                    }
                    break;
                }
                Err(_) => {
                    tracing::debug!("Client idle timeout (300s); closing connection");
                    break;
                }
            };

        if line.is_empty() {
            continue;
        }

        let cmd: ClientCommand = match serde_json::from_str(&line) {
            Ok(cmd) => cmd,
            Err(e) => {
                let err_resp = refused_command_response(&line, &e);
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
                    line = read_bounded_line(&mut reader) => {
                        match line {
                            Ok(Some(line)) => {
                                if line.is_empty() {
                                    continue;
                                }
                                let cmd: ClientCommand = match serde_json::from_str(&line) {
                                    Ok(cmd) => cmd,
                                    Err(e) => {
                                        let err_resp = refused_command_response(&line, &e);
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
            _context: eggsec_runtime::RuntimeExecutionContext,
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

    /// The desktop GUI's own client path, end to end against a live daemon
    /// host over the real Unix socket.
    ///
    /// This is the GUI crate's boundary exercised from the daemon's side:
    /// the GUI cannot depend on the daemon crate, so the test that proves
    /// the two fit together lives here. It covers the client's own identity
    /// — its `Gui` kind, its `gui-` request-id namespace, and its
    /// `DesktopGuiManual` session — and the catalog the GUI renders.
    #[tokio::test]
    async fn the_gui_client_path_works_end_to_end() {
        use eggsec_frontend_client::{ClientIdentity, DaemonRuntimeClient, RuntimeClient};

        let store = std::sync::Arc::new(
            crate::store::SqliteStore::new_in_memory().expect("an in-memory store"),
        );
        let socket_path = format!("/tmp/eggsec-gui-test-{}.sock", uuid::Uuid::new_v4());
        let host = Arc::new(DaemonHost::new(
            DaemonConfig {
                socket_path: socket_path.clone(),
                enable_persistence: true,
                ..Default::default()
            },
            TestExecutor,
            store.clone(),
        ));
        let shutdown = CancellationToken::new();

        let host_clone = host.clone();
        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            run_server(host_clone, shutdown_clone).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = DaemonRuntimeClient::connect(
            &socket_path,
            ClientIdentity {
                kind: eggsec_daemon_protocol::client_registry::ClientKind::Gui,
                label: "eggsec-gui".to_owned(),
                request_prefix: "gui-".to_owned(),
            },
        )
        .await
        .expect("the GUI client connects");

        // The daemon accepts the GUI's declaration and answers its
        // handshake.
        let health = client.health().await.expect("the daemon answers health");
        assert_eq!(
            health.protocol_version,
            eggsec_daemon_protocol::protocol::DAEMON_PROTOCOL_VERSION
        );

        let capabilities = client
            .daemon_capabilities()
            .await
            .expect("the daemon answers capabilities");
        assert!(
            capabilities.operation_schema,
            "the daemon publishes the operation schema the GUI renders"
        );

        let catalog = client
            .describe_operations()
            .await
            .expect("the daemon publishes the catalog");
        assert_eq!(catalog.task_kinds.len(), 30);
        assert!(!catalog.operations.is_empty());

        // The GUI opens its own surface, with the scope it constructs
        // itself.
        let session_id = client
            .create_session(
                eggsec_runtime::RuntimeSurface::DesktopGuiManual,
                Some(eggsec_runtime::session::SessionScope {
                    is_explicit: false,
                    source: "gui-default".to_owned(),
                    path: None,
                }),
                vec![],
            )
            .await
            .expect("the daemon opens a desktop-GUI session");

        let snapshot = client
            .snapshot(session_id)
            .await
            .expect("the GUI session resolves");
        assert_eq!(snapshot.session_id, session_id);

        // The audit trail records the GUI's surface label, not a blanket
        // "daemon".
        let mut surfaces: Vec<String> = Vec::new();
        for _ in 0..200 {
            surfaces = store.audit_surfaces();
            if surfaces
                .iter()
                .any(|surface| surface == "desktop-gui-manual")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(
            surfaces
                .iter()
                .any(|surface| surface == "desktop-gui-manual"),
            "a GUI session must be auditable as desktop-gui-manual, got {surfaces:?}"
        );

        shutdown.cancel();
        let _ = std::fs::remove_file(&socket_path);
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

    /// A command the daemon does not know is refused as
    /// `UnsupportedCommand`, and the refusal carries the request id the
    /// client asked with.
    ///
    /// The request id matters as much as the code: the client routes
    /// responses by request id, so a refusal without one never resolves the
    /// pending request and the client waits out its timeout instead of
    /// learning that its command was refused.
    #[tokio::test]
    async fn server_unknown_command_is_refused_with_its_request_id() {
        let (_host, socket_path, shutdown) = start_server().await;
        let (mut write_half, mut read_lines) = connect(&socket_path).await;

        write_half
            .write_all(b"{\"type\":\"NoSuchCommand\",\"request_id\":\"gui-42\"}\n")
            .await
            .unwrap();
        write_half.flush().await.unwrap();

        let line = read_lines
            .next_line()
            .await
            .unwrap()
            .expect("expected error response");
        let resp: ServerMessage = serde_json::from_str(&line).unwrap();

        match resp {
            ServerMessage::Error {
                request_id,
                code,
                message,
            } => {
                assert_eq!(
                    request_id, "gui-42",
                    "the refusal must echo the request id so the client can route it"
                );
                assert_eq!(code, ErrorCode::UnsupportedCommand);
                assert!(
                    message.contains("unsupported command: NoSuchCommand"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected Error, got {:?}", other),
        }

        shutdown.cancel();
    }

    /// A command the daemon *does* know, sent with a payload that does not
    /// fit, is a malformed request rather than an unknown command — and its
    /// request id is recovered all the same.
    #[tokio::test]
    async fn server_malformed_known_command_is_an_invalid_request() {
        let (_host, socket_path, shutdown) = start_server().await;
        let (mut write_half, mut read_lines) = connect(&socket_path).await;

        // `GetSnapshot` needs a session id; without one the line does not
        // deserialize, but the command itself is one this daemon knows.
        write_half
            .write_all(b"{\"type\":\"GetSnapshot\",\"request_id\":\"gui-43\"}\n")
            .await
            .unwrap();
        write_half.flush().await.unwrap();

        let line = read_lines
            .next_line()
            .await
            .unwrap()
            .expect("expected error response");
        let resp: ServerMessage = serde_json::from_str(&line).unwrap();

        match resp {
            ServerMessage::Error {
                request_id,
                code,
                message,
            } => {
                assert_eq!(request_id, "gui-43");
                assert_eq!(code, ErrorCode::InvalidRequest);
                assert!(
                    message.contains("invalid command"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected Error, got {:?}", other),
        }

        shutdown.cancel();
    }

    /// The refusal classification, as a unit: an unknown command type is
    /// `UnsupportedCommand`, everything else is `InvalidRequest`, and the
    /// request id is recovered whenever the line carries a usable one.
    #[test]
    fn refused_command_response_classifies_the_failure() {
        let parse_error = |line: &str| {
            serde_json::from_str::<ClientCommand>(line).expect_err("the line must not deserialize")
        };

        let unknown = refused_command_response(
            r#"{"type":"NoSuchCommand","request_id":"r-1"}"#,
            &parse_error(r#"{"type":"NoSuchCommand","request_id":"r-1"}"#),
        );
        match unknown {
            ServerMessage::Error {
                request_id,
                code,
                message,
            } => {
                assert_eq!(request_id, "r-1");
                assert_eq!(code, ErrorCode::UnsupportedCommand);
                assert_eq!(message, "unsupported command: NoSuchCommand");
            }
            other => panic!("expected Error, got {:?}", other),
        }

        let known_but_malformed =
            refused_command_response(r#"{"type":"Health"}"#, &parse_error(r#"{"type":"Health"}"#));
        match known_but_malformed {
            ServerMessage::Error {
                request_id,
                code,
                message,
            } => {
                assert_eq!(request_id, "");
                assert_eq!(code, ErrorCode::InvalidRequest);
                assert!(message.contains("invalid command"));
            }
            other => panic!("expected Error, got {:?}", other),
        }

        // A line that is not JSON at all has no type and no request id.
        let not_json = refused_command_response("not json", &parse_error("not json"));
        match not_json {
            ServerMessage::Error {
                request_id, code, ..
            } => {
                assert_eq!(request_id, "");
                assert_eq!(code, ErrorCode::InvalidRequest);
            }
            other => panic!("expected Error, got {:?}", other),
        }

        // A non-string request id cannot be recovered, so the refusal
        // carries an empty one rather than a wrong one.
        let non_string_request_id = refused_command_response(
            r#"{"type":"NoSuchCommand","request_id":7}"#,
            &parse_error(r#"{"type":"NoSuchCommand","request_id":7}"#),
        );
        match non_string_request_id {
            ServerMessage::Error {
                request_id, code, ..
            } => {
                assert_eq!(request_id, "");
                assert_eq!(code, ErrorCode::UnsupportedCommand);
            }
            other => panic!("expected Error, got {:?}", other),
        }
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
                        concurrency: None,
                        udp: None,
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
                        concurrency: None,
                        udp: None,
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
