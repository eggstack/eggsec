//! The end-to-end refusal on the CLI's start path.
//!
//! A second `eggsec daemon start` against a path a live daemon is serving
//! must **fail**: the command exits non-zero and prints the daemon's own
//! actionable refusal — the path, the fact that a live daemon holds it, and
//! what the operator can do. The live daemon keeps serving.

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use eggsec_daemon::client::DaemonClient;
use eggsec_daemon_protocol::protocol::ServerMessage;

/// The CLI binary under test.
const CLI_BIN: &str = env!("CARGO_BIN_EXE_eggsec");

/// A unique socket path under the system temp directory.
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

/// Wait until the daemon on `socket_path` answers `Health`.
async fn wait_for_health(socket_path: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if let Ok(mut client) = DaemonClient::connect(socket_path).await {
            if let Ok(ServerMessage::Health { .. }) = client.health().await {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the daemon never answered Health on {socket_path}");
}

/// Start a daemon on `socket_path` and wait until it is up.
async fn start_daemon(socket_path: &str) -> Child {
    let child = Command::new(CLI_BIN)
        .args(["daemon", "start", "--socket", socket_path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the CLI must start");
    wait_for_health(socket_path).await;
    child
}

/// A second start against a live daemon's path is refused, and the refusal
/// reaches the operator.
#[tokio::test]
async fn a_second_start_against_a_live_daemon_is_refused() {
    let socket_path = temp_socket("cli-refusal");
    let mut first = start_daemon(&socket_path).await;

    // The second start: the pre-flight finds the live daemon and refuses.
    let output = Command::new(CLI_BIN)
        .args(["daemon", "start", "--socket", &socket_path])
        .output()
        .expect("the CLI must run");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(
        !output.status.success(),
        "the second start must fail, stderr: {stderr}"
    );
    assert!(
        stderr.contains(&socket_path),
        "the refusal must name the socket path, stderr: {stderr}"
    );
    assert!(
        stderr.contains("live daemon"),
        "the refusal must say a live daemon holds the path, stderr: {stderr}"
    );
    assert!(
        stderr.contains("stop it first"),
        "the refusal must name a remedy, stderr: {stderr}"
    );

    // The live daemon still serves: the refusal displaced nothing.
    wait_for_health(&socket_path).await;

    let _ = first.kill();
    let _ = first.wait();
}
