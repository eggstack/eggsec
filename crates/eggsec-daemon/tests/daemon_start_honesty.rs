//! Start-path honesty for the daemon.
//!
//! The daemon's socket path is a shared resource: `run_server` unlinks it
//! before binding, so a second start can silently steal a live daemon's
//! socket. These tests pin the pre-flight liveness check that keeps that
//! from happening, the socket's ownership, and the protocol's stability.

use std::sync::Arc;
use std::time::Duration;

use eggsec_daemon::client::DaemonClient;
use eggsec_daemon::config::DaemonConfig;
use eggsec_daemon::host::{
    probe_daemon_liveness, DaemonHost, DaemonLiveness, PREFLIGHT_PROBE_TIMEOUT,
};
use eggsec_daemon::server::run_server;
use eggsec_daemon::store::{noop_store, DaemonStore, PersistedAuditEvent};
use eggsec_daemon_protocol::protocol::{
    ClientCommand, DaemonCapabilities, ServerMessage, DAEMON_PROTOCOL_VERSION,
};
use eggsec_runtime::{
    RuntimeError, RuntimeEventSink, RuntimeExecutionContext, RuntimeTaskExecutor, TaskId,
    TaskOutcome,
};
use tokio_util::sync::CancellationToken;

/// An executor that rejects every task.
///
/// It exists so the tests exercise a daemon that *can* answer `Health` and
/// `Capabilities` without depending on the engine.
struct RejectingExecutor;

impl RuntimeTaskExecutor for RejectingExecutor {
    fn execute(
        &self,
        _task_id: TaskId,
        _request: eggsec_runtime::RunRequest,
        _context: RuntimeExecutionContext,
        _sink: RuntimeEventSink,
        _cancel: CancellationToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>,
    > {
        Box::pin(async { Err(RuntimeError::UnsupportedTaskKind) })
    }
}

/// A store that counts the audit events written to it.
#[derive(Default)]
struct CountingStore {
    audit_events: std::sync::Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl DaemonStore for CountingStore {
    async fn save_session_snapshot(
        &self,
        _snapshot: &eggsec_runtime::SessionSnapshot,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_session_snapshot(
        &self,
        _session_id: eggsec_runtime::SessionId,
    ) -> anyhow::Result<Option<eggsec_runtime::SessionSnapshot>> {
        Ok(None)
    }

    async fn load_all_sessions(&self) -> anyhow::Result<Vec<eggsec_runtime::SessionSnapshot>> {
        Ok(Vec::new())
    }

    async fn record_audit_event(&self, event: &PersistedAuditEvent) -> anyhow::Result<()> {
        let mut events = self
            .audit_events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        events.push(event.action.clone());
        Ok(())
    }

    async fn delete_session(&self, _session_id: eggsec_runtime::SessionId) -> anyhow::Result<()> {
        Ok(())
    }

    fn blocking_list_sessions(&self) -> anyhow::Result<Vec<eggsec_runtime::SessionSummary>> {
        Ok(Vec::new())
    }

    fn blocking_get_snapshot(
        &self,
        _session_id: &eggsec_runtime::SessionId,
    ) -> anyhow::Result<Option<eggsec_runtime::SessionSnapshot>> {
        Ok(None)
    }
}

/// Build a unique socket path under the system temp directory.
///
/// The path stays short on purpose: a Unix socket path is bounded by
/// `sun_path`, and a long temp directory would push the tests over it.
fn temp_socket(label: &str) -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!(
        "{}/eggsec-m009-{}-{}-{}.sock",
        std::env::temp_dir().display(),
        label,
        std::process::id(),
        counter
    )
}

/// Wait until the daemon's socket accepts a connection.
async fn wait_for_live_socket(socket_path: &str) {
    for _ in 0..200 {
        if matches!(
            probe_daemon_liveness(socket_path, Duration::from_millis(200)).await,
            DaemonLiveness::Live { .. }
        ) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the daemon never came up on {socket_path}");
}

/// A live daemon refuses to be displaced: the pre-flight reports it, and the
/// live daemon still answers afterwards.
#[tokio::test]
async fn the_preflight_refuses_to_displace_a_live_daemon() {
    let socket_path = temp_socket("live");
    let host = Arc::new(DaemonHost::new(
        DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        },
        RejectingExecutor,
        noop_store(),
    ));
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    let outcome = probe_daemon_liveness(&socket_path, PREFLIGHT_PROBE_TIMEOUT).await;
    assert_eq!(
        outcome,
        DaemonLiveness::Live {
            protocol_version: DAEMON_PROTOCOL_VERSION
        },
        "a live daemon must be detected as live"
    );

    // The live daemon's socket was not stolen: it still answers Health.
    let mut client = DaemonClient::connect(&socket_path)
        .await
        .expect("the live daemon must still accept connections");
    assert!(matches!(
        client
            .health()
            .await
            .expect("the live daemon must answer Health"),
        ServerMessage::Health { .. }
    ));

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
}

/// A socket file left behind by a crashed daemon is not a live daemon: the
/// pre-flight must let the start proceed so the server's own unlink can
/// clean it up.
#[tokio::test]
async fn the_preflight_treats_a_stale_socket_as_not_live() {
    let socket_path = temp_socket("stale");
    {
        // Binding and dropping leaves the socket file in place with nothing
        // listening on it: exactly what a crashed daemon leaves behind.
        let listener = tokio::net::UnixListener::bind(&socket_path).expect("bind a stale socket");
        drop(listener);
    }
    assert!(
        std::path::Path::new(&socket_path).exists(),
        "the stale socket file must remain after the listener is dropped"
    );

    let outcome = probe_daemon_liveness(&socket_path, PREFLIGHT_PROBE_TIMEOUT).await;
    assert_eq!(
        outcome,
        DaemonLiveness::NotLive,
        "a stale socket file must not read as a live daemon"
    );

    // The start proceeds and rebinds the path.
    let host = Arc::new(DaemonHost::new(
        DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        },
        RejectingExecutor,
        noop_store(),
    ));
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
}

/// The probe is bounded: a daemon that accepts the connection but never
/// answers must not hang the start path.
#[tokio::test]
async fn the_preflight_probe_is_bounded() {
    let socket_path = temp_socket("hung");
    let listener = tokio::net::UnixListener::bind(&socket_path).expect("bind a socket");
    // Accept the connection but never answer: the daemon looks alive on the
    // socket and is silent on the wire.
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            // Hold the connection open without writing a response.
            tokio::time::sleep(Duration::from_secs(30)).await;
            drop(stream);
        }
    });

    let started = std::time::Instant::now();
    let outcome = probe_daemon_liveness(&socket_path, Duration::from_millis(250)).await;
    let elapsed = started.elapsed();

    assert_eq!(outcome, DaemonLiveness::NotLive);
    assert!(
        elapsed < Duration::from_secs(5),
        "the probe must honour its own bound, took {elapsed:?}"
    );
}

/// Two concurrent starts against one socket path.
///
/// The pre-flight narrows the race window; it does not close it, because
/// `run_server` unlinks the socket path before binding (the server-side
/// hardening is deferred). What this milestone guarantees is that neither
/// start reports a false success: both callers see the live daemon and
/// refuse, and the live daemon keeps its socket.
#[tokio::test]
async fn two_concurrent_starts_against_one_socket_path() {
    let socket_path = temp_socket("race");
    let host = Arc::new(DaemonHost::new(
        DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        },
        RejectingExecutor,
        noop_store(),
    ));
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    let first = tokio::spawn({
        let socket_path = socket_path.clone();
        async move { probe_daemon_liveness(&socket_path, PREFLIGHT_PROBE_TIMEOUT).await }
    });
    let second = tokio::spawn({
        let socket_path = socket_path.clone();
        async move { probe_daemon_liveness(&socket_path, PREFLIGHT_PROBE_TIMEOUT).await }
    });
    let first = first.await.expect("the first caller must finish");
    let second = second.await.expect("the second caller must finish");

    assert!(
        matches!(first, DaemonLiveness::Live { .. }),
        "the first caller must see the live daemon"
    );
    assert!(
        matches!(second, DaemonLiveness::Live { .. }),
        "the second caller must see the live daemon"
    );

    // The live daemon still owns the path: at most one daemon serves it.
    let mut client = DaemonClient::connect(&socket_path)
        .await
        .expect("the live daemon must still accept connections");
    assert!(matches!(
        client
            .health()
            .await
            .expect("the live daemon must answer Health"),
        ServerMessage::Health { .. }
    ));

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
}

/// The daemon removes its socket when it shuts down.
#[tokio::test]
async fn the_socket_is_removed_on_shutdown() {
    let socket_path = temp_socket("shutdown");
    let host = Arc::new(DaemonHost::new(
        DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        },
        RejectingExecutor,
        noop_store(),
    ));
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;
    assert!(std::path::Path::new(&socket_path).exists());

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
    assert!(
        !std::path::Path::new(&socket_path).exists(),
        "the socket must be removed on shutdown"
    );
}

/// The socket is owner-only: a permissive umask must not leave it
/// world-connectable.
#[cfg(unix)]
#[tokio::test]
async fn the_socket_is_created_with_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let socket_path = temp_socket("perms");
    let host = Arc::new(DaemonHost::new(
        DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        },
        RejectingExecutor,
        noop_store(),
    ));
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    let mode = std::fs::metadata(&socket_path)
        .expect("the socket's metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "the socket must be owner-only");

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
}

/// The liveness probe is a read-only check: it creates no session and no
/// audit record.
#[tokio::test]
async fn the_preflight_probe_creates_no_session_and_no_audit_record() {
    let socket_path = temp_socket("probe");
    let store = Arc::new(CountingStore::default());
    let host = Arc::new(DaemonHost::new(
        DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        },
        RejectingExecutor,
        store.clone(),
    ));
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    let outcome = probe_daemon_liveness(&socket_path, PREFLIGHT_PROBE_TIMEOUT).await;
    assert!(matches!(outcome, DaemonLiveness::Live { .. }));

    let sessions = host.runtime().list_sessions().await;
    assert!(
        sessions.is_empty(),
        "the probe must not create a session, found {sessions:?}"
    );
    let audit_events = store
        .audit_events
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert!(
        audit_events.is_empty(),
        "the probe must not write an audit record, found {audit_events:?}"
    );

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
}

/// The protocol version is unchanged: the daemon still answers with the
/// version the protocol crate defines.
#[tokio::test]
async fn the_protocol_version_is_unchanged() {
    assert_eq!(DAEMON_PROTOCOL_VERSION, 2);

    let socket_path = temp_socket("version");
    let host = Arc::new(DaemonHost::new(
        DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        },
        RejectingExecutor,
        noop_store(),
    ));
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    let mut client = DaemonClient::connect(&socket_path)
        .await
        .expect("connect to the daemon");
    match client.health().await.expect("the daemon answers Health") {
        ServerMessage::Health {
            protocol_version, ..
        } => {
            assert_eq!(protocol_version, DAEMON_PROTOCOL_VERSION);
        }
        other => panic!("expected Health, got {other:?}"),
    }

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
}

/// The `Capabilities` message shape is unchanged: the milestone changes the
/// published **value**, never the **shape**.
#[tokio::test]
async fn the_capabilities_message_shape_is_unchanged() {
    let host = DaemonHost::new(DaemonConfig::default(), RejectingExecutor, noop_store());
    let response = host
        .handle_command(
            ClientCommand::Capabilities {
                request_id: "shape".into(),
            },
            eggsec_daemon_protocol::protocol::DaemonRequestContext {
                client_id: None,
                peer: None,
                transport: eggsec_daemon_protocol::protocol::TransportKind::UnixSocket,
            },
        )
        .await;

    let value = match response {
        ServerMessage::Capabilities { capabilities, .. } => capabilities,
        other => panic!("expected Capabilities, got {other:?}"),
    };
    let encoded = serde_json::to_value(&value).expect("serialize the capabilities");
    let mut keys: Vec<&str> = encoded
        .as_object()
        .expect("the capabilities are an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["operation_schema", "runtime", "transports"]);

    let runtime = encoded
        .get("runtime")
        .and_then(serde_json::Value::as_object)
        .expect("the runtime capabilities are an object");
    let mut runtime_keys: Vec<&str> = runtime.keys().map(String::as_str).collect();
    runtime_keys.sort_unstable();
    assert_eq!(
        runtime_keys,
        vec![
            "supports_cancellation",
            "supports_multiple_active_tasks",
            "supports_multiple_sessions",
            "task_kinds",
            "transports",
        ]
    );

    // The typed value still carries the same shape.
    let _: DaemonCapabilities = value;
}

/// Serve a daemon that answers `Health` with a protocol version this client
/// does not speak.
///
/// The handshake is written by hand so the test can pin an *older* (or
/// newer) daemon without a second implementation of the protocol. Every
/// connection is answered, so a probe that arrives at any time is served.
fn serve_a_foreign_protocol_version(socket_path: &str) -> tokio::task::JoinHandle<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let listener = tokio::net::UnixListener::bind(socket_path).expect("bind a socket");
    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(error) => {
                    tracing::warn!(error = %error, "the foreign daemon failed to accept");
                    return;
                }
            };
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).await.is_err() {
                continue;
            }
            let request_id = serde_json::from_str::<ClientCommand>(&line)
                .map(|command| command.request_id().to_owned())
                .unwrap_or_else(|_| "foreign".to_string());
            let response = ServerMessage::Health {
                request_id,
                status: "ok".into(),
                version: "0.0.0-foreign".into(),
                protocol_version: DAEMON_PROTOCOL_VERSION + 1,
            };
            let mut encoded = match serde_json::to_string(&response) {
                Ok(encoded) => encoded,
                Err(error) => {
                    tracing::warn!(error = %error, "the foreign daemon failed to encode Health");
                    continue;
                }
            };
            encoded.push('\n');
            let mut stream = reader.into_inner();
            if let Err(error) = stream.write_all(encoded.as_bytes()).await {
                tracing::warn!(error = %error, "the foreign daemon failed to answer Health");
            }
        }
    })
}

/// A daemon that speaks a different protocol version is still **live**: the
/// pre-flight refuses to displace it rather than stealing its socket.
#[tokio::test]
async fn an_older_daemon_on_the_socket_is_detected_as_live() {
    let socket_path = temp_socket("foreign");
    let server = serve_a_foreign_protocol_version(&socket_path);

    // Wait until the foreign daemon is serving.
    let mut outcome = DaemonLiveness::NotLive;
    for _ in 0..40 {
        outcome = probe_daemon_liveness(&socket_path, PREFLIGHT_PROBE_TIMEOUT).await;
        if matches!(outcome, DaemonLiveness::Live { .. }) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(
        outcome,
        DaemonLiveness::Live {
            protocol_version: DAEMON_PROTOCOL_VERSION + 1
        },
        "a daemon on a different protocol version is still live"
    );

    server.abort();
}
