//! The server-side bind guard.
//!
//! `run_server` binds the socket path **without unlinking it first**. When
//! the path is held by a live daemon, the server refuses — and the refusal
//! does **not** remove the live daemon's socket file. When the path holds no
//! daemon (a stale socket file, or a plain file), the server reclaims it once
//! and binds.
//!
//! These tests start a **second server** against a live path, because probing
//! twice cannot observe a bind behaviour: the defect this milestone fixes was
//! invisible to a probe-only test.

use std::sync::Arc;
use std::time::Duration;

use eggsec_daemon::client::DaemonClient;
use eggsec_daemon::config::DaemonConfig;
use eggsec_daemon::error::DaemonError;
use eggsec_daemon::host::{probe_daemon_liveness, DaemonHost, DaemonLiveness};
use eggsec_daemon::server::run_server;
use eggsec_daemon::store::noop_store;
use eggsec_daemon_protocol::protocol::ServerMessage;
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
    /// Declares no support.
    fn supported_task_kinds(&self) -> Vec<eggsec_runtime::TaskCapability> {
        Vec::new()
    }

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

/// Build a unique socket path under the system temp directory.
///
/// The path stays short on purpose: a Unix socket path is bounded by
/// `sun_path`.
fn temp_socket(label: &str) -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!(
        "{}/eggsec-m012-{}-{}-{}.sock",
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

/// A daemon host on `socket_path`.
fn daemon_host(socket_path: &str) -> Arc<DaemonHost> {
    Arc::new(DaemonHost::new(
        DaemonConfig {
            socket_path: socket_path.to_owned(),
            ..Default::default()
        },
        RejectingExecutor,
        noop_store(),
    ))
}

/// Two servers, one path: exactly one binds, the other is refused, and the
/// survivor keeps serving.
///
/// This is the regression evidence the pre-flight's probe-only test could not
/// give: the second **server** reaches a bind, and the bind refuses.
#[tokio::test]
async fn two_servers_one_path_exactly_one_binds() {
    let socket_path = temp_socket("two-servers");
    let host = daemon_host(&socket_path);
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    // The second server reaches a bind against the live path.
    let second = daemon_host(&socket_path);
    let second_shutdown = CancellationToken::new();
    let refusal = run_server(second, second_shutdown).await;

    let error = refusal.expect_err("the second server must be refused");
    assert!(
        matches!(error, DaemonError::SocketPathHeld { .. }),
        "the refusal must be the live-daemon refusal, got: {error}"
    );
    let message = error.to_string();
    assert!(
        message.contains(&socket_path),
        "the refusal must name the socket path, got: {message}"
    );
    assert!(
        message.contains("live daemon"),
        "the refusal must say a live daemon holds the path, got: {message}"
    );
    assert!(
        message.contains("stop it first"),
        "the refusal must name a remedy, got: {message}"
    );

    // The live daemon keeps its socket file: the refusal did not unlink it.
    assert!(
        std::path::Path::new(&socket_path).exists(),
        "a refused bind must not remove the live daemon's socket file"
    );

    // And it keeps serving.
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

/// A refused bind leaves the live daemon's socket file owner-only.
#[cfg(unix)]
#[tokio::test]
async fn a_refused_bind_leaves_the_socket_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let socket_path = temp_socket("refused-perms");
    let host = daemon_host(&socket_path);
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    let refusal = run_server(daemon_host(&socket_path), CancellationToken::new()).await;
    assert!(
        refusal.is_err(),
        "the second server must be refused, got: {refusal:?}"
    );

    let mode = std::fs::metadata(&socket_path)
        .expect("the socket's metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        mode, 0o600,
        "the live daemon's socket must still be owner-only after a refusal"
    );

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
}

/// A stale socket file is reclaimed by the bind, and the daemon serves.
#[tokio::test]
async fn a_stale_socket_file_is_reclaimed_by_the_bind() {
    let socket_path = temp_socket("stale");
    // A socket file with no listener behind it: what a crashed daemon leaves
    // behind.
    let listener = tokio::net::UnixListener::bind(&socket_path).expect("bind a socket");
    drop(listener);
    assert!(
        std::path::Path::new(&socket_path).exists(),
        "the stale socket file must remain on the path"
    );

    let host = daemon_host(&socket_path);
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    let mut client = DaemonClient::connect(&socket_path)
        .await
        .expect("the daemon must accept connections");
    assert!(matches!(
        client
            .health()
            .await
            .expect("the daemon must answer Health"),
        ServerMessage::Health { .. }
    ));

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
}

/// A plain file at the path is reclaimed the same way a stale socket is.
#[tokio::test]
async fn a_plain_file_at_the_path_is_reclaimed_by_the_bind() {
    let socket_path = temp_socket("plain-file");
    std::fs::write(&socket_path, b"not a socket").expect("write the file");

    let host = daemon_host(&socket_path);
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

/// A bind that fails for another reason leaves the path alone.
///
/// The guard only unlinks a path the probe has shown to hold no daemon, and
/// a bind that never got as far as `AddrInUse` proves nothing about the
/// path's state.
#[tokio::test]
async fn a_bind_failure_that_is_not_addr_in_use_leaves_the_path_alone() {
    let directory = temp_socket("missing-dir");
    let socket_path = format!("{directory}/nested/daemon.sock");
    // The parent directory does not exist, so the bind fails with a kind that
    // is not `AddrInUse`.
    let error = run_server(daemon_host(&socket_path), CancellationToken::new())
        .await
        .expect_err("the bind must fail");
    assert!(
        matches!(error, DaemonError::Io(_)),
        "the bind error must be reported as it is, got: {error}"
    );
    assert!(
        !std::path::Path::new(&socket_path).exists(),
        "the failed bind must not create the path"
    );
}

/// The concurrent-start race, repeated.
///
/// Each round starts two servers against one path. Exactly one binds, the
/// other refuses, and the survivor serves. A server never removes another's
/// socket: the refused server leaves the file in place.
#[tokio::test]
async fn many_rounds_bind_exactly_one_server() {
    for round in 0..20 {
        let socket_path = temp_socket(&format!("race-{round}"));
        let host = daemon_host(&socket_path);
        let shutdown = CancellationToken::new();
        let server = {
            let host = host.clone();
            let shutdown = shutdown.clone();
            tokio::spawn(async move { run_server(host, shutdown).await })
        };
        wait_for_live_socket(&socket_path).await;

        let second = run_server(daemon_host(&socket_path), CancellationToken::new()).await;
        assert!(
            second.is_err(),
            "round {round}: the second server must be refused, got: {second:?}"
        );
        assert!(
            std::path::Path::new(&socket_path).exists(),
            "round {round}: the refused server must not remove the socket file"
        );

        let mut client = DaemonClient::connect(&socket_path)
            .await
            .expect("the survivor must still accept connections");
        assert!(
            matches!(
                client
                    .health()
                    .await
                    .expect("the survivor must answer Health"),
                ServerMessage::Health { .. }
            ),
            "round {round}: the survivor must keep serving"
        );

        shutdown.cancel();
        match server.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("round {round}: the server failed: {error}"),
            Err(error) => panic!("round {round}: the server task failed: {error}"),
        }
    }
}

/// The stale-then-race shape: two servers against a path holding a stale
/// file. Exactly one binds; the other refuses; the survivor serves.
#[tokio::test]
async fn a_stale_path_with_two_servers_binds_once() {
    let socket_path = temp_socket("stale-race");
    let listener = tokio::net::UnixListener::bind(&socket_path).expect("bind a socket");
    drop(listener);

    let host = daemon_host(&socket_path);
    let shutdown = CancellationToken::new();
    let server = {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { run_server(host, shutdown).await })
    };
    wait_for_live_socket(&socket_path).await;

    let second = run_server(daemon_host(&socket_path), CancellationToken::new()).await;
    assert!(
        second.is_err(),
        "the second server must be refused, got: {second:?}"
    );

    let mut client = DaemonClient::connect(&socket_path)
        .await
        .expect("the survivor must still accept connections");
    assert!(matches!(
        client
            .health()
            .await
            .expect("the survivor must answer Health"),
        ServerMessage::Health { .. }
    ));

    shutdown.cancel();
    match server.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("the server failed: {error}"),
        Err(error) => panic!("the server task failed: {error}"),
    }
}
