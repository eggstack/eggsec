//! The daemon binary and the socket environment variable.
//!
//! The environment variable's name and the precedence rule live in one crate
//! (`eggsec-runtime`). These tests observe the binary's **resolved** socket
//! path in a child process, because the variable is process-global: the
//! in-process tests never set it.
//!
//! Two observations are used:
//!
//! - the binary's `--help` prints the default it resolved, and
//! - the socket the binary actually binds is the path it was told to use.

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use eggsec_daemon::host::{probe_daemon_liveness, DaemonLiveness};
use eggsec_daemon_protocol::protocol::ServerMessage;

/// The daemon binary under test.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_eggsec-daemon");

/// The canonical default the binary must fall back to.
///
/// The canonical crate owns the literal; this test names its constant.
const BUILT_IN_DEFAULT: &str = eggsec_runtime::DEFAULT_DAEMON_SOCKET_PATH;

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

/// The default the binary's `--help` reports.
fn reported_default(env_value: Option<&str>) -> String {
    let mut command = Command::new(DAEMON_BIN);
    command
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match env_value {
        Some(value) => {
            command.env(eggsec_runtime::DAEMON_SOCKET_ENV_VAR, value);
        }
        None => {
            command.env_remove(eggsec_runtime::DAEMON_SOCKET_ENV_VAR);
        }
    }
    let output = command.output().expect("the daemon binary must run");
    let help = String::from_utf8_lossy(&output.stdout).to_string();
    let marker = "[default: ";
    let start = help
        .find(marker)
        .expect("the help text must report the resolved default")
        + marker.len();
    let end = help[start..].find(']').expect("the default must be closed") + start;
    help[start..end].to_owned()
}

/// Start the daemon binary on `socket_path` and wait until it answers.
///
/// The child is **returned**: the caller owns it and is responsible for
/// killing and waiting for it. The lint that asks every spawn to be waited
/// on does not apply to a helper that hands the child over.
#[allow(clippy::zombie_processes)]
async fn start_daemon(socket_path: &str, env_value: Option<&str>) -> Child {
    let mut command = Command::new(DAEMON_BIN);
    command
        .arg("--socket-path")
        .arg(socket_path)
        // No persistence: the test must not touch the operator's home.
        .arg("--no-persistence")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match env_value {
        Some(value) => {
            command.env(eggsec_runtime::DAEMON_SOCKET_ENV_VAR, value);
        }
        None => {
            command.env_remove(eggsec_runtime::DAEMON_SOCKET_ENV_VAR);
        }
    }
    let mut child = command.spawn().expect("the daemon binary must start");
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if matches!(
            probe_daemon_liveness(socket_path, Duration::from_millis(200)).await,
            DaemonLiveness::Live { .. }
        ) {
            return child;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    panic!("the daemon never came up on {socket_path}");
}

/// Wait until the daemon on `socket_path` answers `Health`.
async fn wait_for_health(socket_path: &str) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(mut client) = eggsec_daemon::client::DaemonClient::connect(socket_path).await {
            if let Ok(ServerMessage::Health { .. }) = client.health().await {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the daemon never answered Health on {socket_path}");
}

/// Without the environment variable, the binary resolves the built-in default.
#[test]
fn without_the_environment_variable_the_binary_uses_the_built_in_default() {
    assert_eq!(reported_default(None), BUILT_IN_DEFAULT);
}

/// With the environment variable set, the binary resolves the operator's path.
#[test]
fn the_environment_variable_overrides_the_built_in_default() {
    assert_eq!(
        reported_default(Some("/run/user/1000/eggsec.sock")),
        "/run/user/1000/eggsec.sock"
    );
}

/// An empty environment value is absent, not an empty path.
#[test]
fn an_empty_environment_value_falls_back_to_the_built_in_default() {
    assert_eq!(reported_default(Some("")), BUILT_IN_DEFAULT);
}

/// A whitespace-only environment value is absent too.
#[test]
fn a_whitespace_only_environment_value_falls_back_to_the_built_in_default() {
    assert_eq!(reported_default(Some("   ")), BUILT_IN_DEFAULT);
}

/// An explicit flag beats the environment variable.
///
/// The observation is the socket the daemon actually binds: the flag's path,
/// not the environment's.
#[tokio::test]
async fn an_explicit_flag_beats_the_environment_variable() {
    let explicit = temp_socket("explicit");
    let from_env = temp_socket("from-env");
    let mut child = start_daemon(&explicit, Some(&from_env)).await;

    assert!(
        std::path::Path::new(&explicit).exists(),
        "the daemon must bind the flag's path"
    );
    assert!(
        !std::path::Path::new(&from_env).exists(),
        "the daemon must not bind the environment's path"
    );
    wait_for_health(&explicit).await;

    let _ = child.kill();
    let _ = child.wait();
}

/// The daemon binary's own restart: a daemon that is killed leaves its socket
/// file behind, and the next daemon reclaims the path and serves.
#[tokio::test]
async fn the_binary_reclaims_the_path_of_a_killed_daemon() {
    let socket_path = temp_socket("restart");
    let mut child = start_daemon(&socket_path, None).await;
    wait_for_health(&socket_path).await;

    // An unclean stop: the socket file stays behind.
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        std::path::Path::new(&socket_path).exists(),
        "the killed daemon must leave its socket file behind"
    );

    // The next daemon reclaims the path and serves.
    let mut restarted = start_daemon(&socket_path, None).await;
    wait_for_health(&socket_path).await;

    let _ = restarted.kill();
    let _ = restarted.wait();
}
