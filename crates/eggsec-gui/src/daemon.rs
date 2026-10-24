//! The opt-in daemon lifecycle: spawn a daemon the operator named, and own it.
//!
//! # The default is "connect to a running daemon"
//!
//! The GUI does not spawn a daemon unless the operator asks it to, by naming
//! the daemon executable with `--daemon-bin`. There is no `PATH` lookup, no
//! sidecar convention, and no well-known directory: an unnamed daemon
//! executable is never found, because a GUI that silently starts *some*
//! daemon is a GUI that can start the wrong one.
//!
//! # The pre-flight check is a connection, never a `stat`
//!
//! The daemon's own server unlinks the socket path before it binds
//! (`crates/eggsec-daemon/src/server.rs`), so a second daemon started at the
//! same path **steals a live socket** out from under its clients. The only
//! way to tell a live daemon from a socket file left behind by a crashed one
//! is to connect to it, so the liveness probe is a connection attempt and a
//! health exchange — never a check that the file exists.
//!
//! # Readiness is a handshake, not a log line
//!
//! The daemon's "Daemon listening on ..." line is a log format, not a
//! contract, so the GUI never waits for it. Readiness is a successful
//! connection that completes the protocol handshake, bounded by
//! [`READINESS_TIMEOUT`] and retried every [`READINESS_POLL_INTERVAL`].
//!
//! # The child does not outlive the GUI
//!
//! The GUI terminates the daemon it started: SIGTERM first — the daemon's
//! own shutdown path — escalating to an unconditional kill only if the
//! daemon ignores it. SIGHUP is never sent: the daemon does not handle it.
//! A readiness timeout terminates the child before the error is returned, so
//! a failed start leaves no orphan behind.
//!
//! The GUI reports a daemon that dies; it never silently respawns one.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eggsec_daemon_protocol::protocol::DAEMON_PROTOCOL_VERSION;
use eggsec_frontend_client::DaemonClientError;
use eggsec_frontend_client::DaemonRuntimeClient;

use crate::connection::gui_identity;

/// How long the GUI waits for a spawned daemon to become connectable.
pub const READINESS_TIMEOUT: Duration = Duration::from_secs(60);

/// How often the GUI retries the readiness probe while it waits.
pub const READINESS_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// How long a single liveness or readiness probe may take.
///
/// The probe is bounded so a daemon that accepts the connection and then
/// says nothing cannot stall the readiness loop.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// How long a supervised daemon may take to exit after SIGTERM.
pub const TERMINATION_GRACE: Duration = Duration::from_secs(5);

/// The daemon the GUI is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonAttachment {
    /// Not connected to any daemon.
    Disconnected,
    /// Attached to a daemon the operator started.
    Operator,
    /// Supervising a daemon this GUI started.
    Supervised,
}

/// The argv the GUI spawns a daemon with.
///
/// The argv is a **vector**: no shell string is ever built, so a socket path
/// or a binary path with a space in it cannot become an injection surface.
pub fn spawn_argv(daemon_bin: &Path, socket_path: &str) -> Vec<OsString> {
    vec![
        daemon_bin.as_os_str().to_owned(),
        OsString::from("--full-executor"),
        OsString::from("--socket-path"),
        OsString::from(socket_path),
    ]
}

/// Probe the daemon on `socket_path`.
///
/// The probe is a **connection attempt** plus a health exchange, never a
/// `stat` of the socket file. A stale socket file left behind by a crashed
/// daemon is not a live daemon, and only a connection tells them apart.
///
/// `Ok(protocol_version)` means something answered on the socket path.
pub async fn probe_daemon(socket_path: &str) -> Result<u32, DaemonClientError> {
    let client = DaemonRuntimeClient::connect(socket_path, gui_identity()).await?;
    let health = client.health().await?;
    Ok(health.protocol_version)
}

/// Whether a daemon is already serving `socket_path`.
///
/// This is the pre-flight check the spawn route runs before it starts
/// anything: a daemon that is already there must be connected to, not
/// displaced.
pub async fn daemon_is_live(socket_path: &str) -> bool {
    match tokio::time::timeout(PROBE_TIMEOUT, probe_daemon(socket_path)).await {
        Ok(Ok(_protocol_version)) => true,
        Ok(Err(error)) => {
            tracing::debug!(socket_path, %error, "no daemon answered on the socket path");
            false
        }
        Err(_elapsed) => {
            tracing::warn!(socket_path, "the daemon liveness probe timed out");
            false
        }
    }
}

/// Spawn the daemon the operator named, and wait for it to become ready.
///
/// The child is terminated before an error is returned, so a failed start
/// never leaves an orphan behind.
pub async fn spawn_and_supervise(
    daemon_bin: &Path,
    socket_path: &str,
) -> Result<SupervisedDaemon, DaemonLifecycleError> {
    let argv = spawn_argv(daemon_bin, socket_path);
    let mut command = tokio::process::Command::new(&argv[0]);
    command.args(&argv[1..]);
    // The child's own output is not read: the GUI never scrapes it for a
    // readiness signal, and a pipe nobody drains would eventually block the
    // child. The daemon's exit status is the diagnostic the GUI reports.
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    // The last-resort net: a dropped handle kills the child rather than
    // leaving it running with nobody watching it.
    command.kill_on_drop(true);

    let mut child = command
        .spawn()
        .map_err(|error| DaemonLifecycleError::Spawn {
            binary: daemon_bin.to_path_buf(),
            error,
        })?;
    let pid = child.id().unwrap_or(0);

    let readiness =
        tokio::time::timeout(READINESS_TIMEOUT, wait_until_ready(&mut child, socket_path)).await;

    match readiness {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            terminate_child(&mut child).await;
            return Err(error);
        }
        Err(_elapsed) => {
            terminate_child(&mut child).await;
            return Err(DaemonLifecycleError::ReadinessTimeout {
                seconds: READINESS_TIMEOUT.as_secs(),
                socket_path: socket_path.to_owned(),
            });
        }
    }

    Ok(SupervisedDaemon {
        pid,
        socket_path: socket_path.to_owned(),
        child: Arc::new(Mutex::new(Some(child))),
    })
}

/// Wait until the spawned daemon answers the protocol handshake.
async fn wait_until_ready(
    child: &mut tokio::process::Child,
    socket_path: &str,
) -> Result<(), DaemonLifecycleError> {
    loop {
        match tokio::time::timeout(PROBE_TIMEOUT, probe_daemon(socket_path)).await {
            Ok(Ok(protocol_version)) => {
                if protocol_version != DAEMON_PROTOCOL_VERSION {
                    return Err(DaemonLifecycleError::ProtocolMismatch {
                        found: protocol_version,
                        expected: DAEMON_PROTOCOL_VERSION,
                    });
                }
                return Ok(());
            }
            Ok(Err(error)) => {
                tracing::debug!(socket_path, %error, "the spawned daemon is not ready yet");
            }
            Err(_elapsed) => {
                tracing::debug!(socket_path, "the daemon readiness probe timed out");
            }
        }

        // A child that exited will never become ready, so the wait reports
        // the exit instead of running out the clock.
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(DaemonLifecycleError::Exited {
                    status: status.to_string(),
                });
            }
            Ok(None) => {}
            Err(error) => {
                return Err(DaemonLifecycleError::Exited {
                    status: format!("the daemon's exit status could not be read: {error}"),
                });
            }
        }

        tokio::time::sleep(READINESS_POLL_INTERVAL).await;
    }
}

/// Terminate a child the GUI owns, after the readiness wait failed.
async fn terminate_child(child: &mut tokio::process::Child) {
    let Some(pid) = child.id() else {
        return;
    };
    match send_terminate(pid) {
        TerminateSignal::Sent => {
            if let Err(error) = child.start_kill() {
                tracing::warn!(pid, %error, "the daemon could not be killed after the terminate signal");
            }
        }
        TerminateSignal::AlreadyGone => {}
        TerminateSignal::Unsupported => {
            if let Err(error) = child.start_kill() {
                tracing::warn!(pid, %error, "the daemon could not be killed");
            }
        }
    }
    if let Err(error) = child.wait().await {
        tracing::warn!(pid, %error, "the daemon's exit could not be awaited");
    }
}

/// What happened to the signal the GUI wanted to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminateSignal {
    /// The signal was delivered.
    Sent,
    /// The daemon was already gone.
    AlreadyGone,
    /// This platform has no such signal.
    Unsupported,
}

/// Send SIGTERM to a process.
///
/// The daemon's own shutdown path is SIGTERM (and ctrl_c), so it is the
/// first signal the GUI sends. SIGHUP is never sent: the daemon does not
/// handle it, so it would kill the daemon without a clean shutdown.
#[cfg(unix)]
fn send_terminate(pid: u32) -> TerminateSignal {
    // SAFETY: `kill` with SIGTERM is a plain signal delivery; the only
    // requirements are a valid pid and the signal being deliverable. A
    // failure is reported by the return value, not by undefined behaviour.
    if unsafe { libc::kill(pid as i32, libc::SIGTERM) } == 0 {
        TerminateSignal::Sent
    } else {
        TerminateSignal::AlreadyGone
    }
}

/// Send SIGTERM to a process.
#[cfg(not(unix))]
fn send_terminate(_pid: u32) -> TerminateSignal {
    TerminateSignal::Unsupported
}

/// A daemon this GUI started and owns.
#[derive(Debug, Clone)]
pub struct SupervisedDaemon {
    /// The child's process id.
    pid: u32,
    /// The socket path the daemon serves.
    socket_path: String,
    /// The child handle, shared so the GUI's shutdown path can escalate.
    child: Arc<Mutex<Option<tokio::process::Child>>>,
}

impl SupervisedDaemon {
    /// The child's process id.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// The socket path the supervised daemon serves.
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    /// Terminate the daemon: SIGTERM first, an unconditional kill only if
    /// the daemon ignores it.
    ///
    /// The daemon's exit status is reported, so an operator learns whether
    /// the daemon shut down cleanly or had to be killed.
    pub async fn terminate(&self) -> TerminationOutcome {
        let Some(mut child) = self.take_child() else {
            return TerminationOutcome::AlreadyGone;
        };

        match send_terminate(self.pid) {
            TerminateSignal::Sent => {}
            TerminateSignal::AlreadyGone => return TerminationOutcome::AlreadyGone,
            TerminateSignal::Unsupported => {
                tracing::warn!(
                    pid = self.pid,
                    "this platform has no terminate signal: the supervised daemon is force-killed \
                     instead, so it cannot shut down cleanly"
                );
                return self.force_kill(&mut child).await;
            }
        }

        match tokio::time::timeout(TERMINATION_GRACE, child.wait()).await {
            Ok(Ok(status)) => TerminationOutcome::Exited(status.to_string()),
            Ok(Err(error)) => TerminationOutcome::Failed(error.to_string()),
            Err(_elapsed) => {
                tracing::warn!(
                    pid = self.pid,
                    "the supervised daemon ignored the terminate signal: escalating to a kill"
                );
                self.force_kill(&mut child).await
            }
        }
    }

    /// Send the terminate signal without waiting for the daemon to exit.
    ///
    /// The GUI's shutdown path cannot await — the runtime is going away — so
    /// the signal is sent synchronously and the escalation is scheduled on
    /// whatever runtime is still alive.
    pub fn signal_terminate(&self) -> TerminateSignal {
        send_terminate(self.pid)
    }

    /// Kill the daemon unconditionally.
    async fn force_kill(&self, child: &mut tokio::process::Child) -> TerminationOutcome {
        if let Err(error) = child.start_kill() {
            return TerminationOutcome::Failed(error.to_string());
        }
        match tokio::time::timeout(TERMINATION_GRACE, child.wait()).await {
            Ok(Ok(status)) => TerminationOutcome::Killed(status.to_string()),
            Ok(Err(error)) => TerminationOutcome::Failed(error.to_string()),
            Err(_elapsed) => TerminationOutcome::Failed(
                "the daemon did not exit after the kill signal".to_owned(),
            ),
        }
    }

    /// Take the child handle out of the shared slot.
    ///
    /// The handle is moved rather than borrowed so that no lock is held
    /// across an await.
    fn take_child(&self) -> Option<tokio::process::Child> {
        match self.child.lock() {
            Ok(mut guard) => guard.take(),
            Err(error) => {
                tracing::error!(%error, "the supervised daemon's handle is poisoned");
                None
            }
        }
    }
}

/// How a supervised daemon's termination ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationOutcome {
    /// The daemon was already gone.
    AlreadyGone,
    /// The daemon exited after the terminate signal.
    Exited(String),
    /// The daemon had to be killed.
    Killed(String),
    /// The termination failed.
    Failed(String),
}

impl TerminationOutcome {
    /// Whether the daemon is gone.
    pub fn is_gone(&self) -> bool {
        !matches!(self, TerminationOutcome::Failed(_))
    }
}

/// Everything that can go wrong on the spawn-and-supervise route.
#[derive(Debug)]
pub enum DaemonLifecycleError {
    /// The daemon executable could not be started.
    Spawn {
        /// The executable the operator named.
        binary: PathBuf,
        /// Why it could not be started.
        error: std::io::Error,
    },
    /// The daemon exited before it became ready.
    Exited {
        /// The child's exit status.
        status: String,
    },
    /// The daemon did not become ready within the readiness timeout.
    ReadinessTimeout {
        /// How long the GUI waited.
        seconds: u64,
        /// The socket the GUI waited on.
        socket_path: String,
    },
    /// The daemon speaks a different protocol version.
    ProtocolMismatch {
        /// The version the daemon speaks.
        found: u32,
        /// The version this GUI speaks.
        expected: u32,
    },
}

impl fmt::Display for DaemonLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaemonLifecycleError::Spawn { binary, error } => write!(
                f,
                "the daemon executable {} could not be started: {error}",
                binary.display()
            ),
            DaemonLifecycleError::Exited { status } => write!(
                f,
                "the daemon failed to start: the process exited with status {status}"
            ),
            DaemonLifecycleError::ReadinessTimeout {
                seconds,
                socket_path,
            } => write!(
                f,
                "the daemon failed to start: it did not answer on {socket_path} within {seconds} \
                 seconds"
            ),
            DaemonLifecycleError::ProtocolMismatch { found, expected } => write!(
                f,
                "the daemon failed to start: it speaks daemon protocol version {found}, but this \
                 GUI speaks version {expected}"
            ),
        }
    }
}

impl std::error::Error for DaemonLifecycleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spawn_argv_is_a_vector_and_names_the_socket() {
        let argv = spawn_argv(Path::new("/opt/eggsec/bin/daemon"), "/tmp/eggsec.sock");
        assert_eq!(argv.len(), 4);
        assert_eq!(argv[0], OsString::from("/opt/eggsec/bin/daemon"));
        assert_eq!(argv[1], OsString::from("--full-executor"));
        assert_eq!(argv[2], OsString::from("--socket-path"));
        assert_eq!(argv[3], OsString::from("/tmp/eggsec.sock"));
    }

    #[test]
    fn the_spawn_argv_never_builds_a_shell_string() {
        // A socket path with a space in it stays one argument: nothing is
        // ever handed to a shell.
        let argv = spawn_argv(Path::new("/opt/my daemons/eggsec"), "/tmp/a b.sock");
        assert_eq!(argv[0], OsString::from("/opt/my daemons/eggsec"));
        assert_eq!(argv[3], OsString::from("/tmp/a b.sock"));
        assert_eq!(argv.len(), 4);
    }

    #[test]
    fn the_attachment_states_are_distinct() {
        assert_ne!(DaemonAttachment::Disconnected, DaemonAttachment::Operator);
        assert_ne!(DaemonAttachment::Operator, DaemonAttachment::Supervised);
        assert_ne!(DaemonAttachment::Disconnected, DaemonAttachment::Supervised);
    }

    #[test]
    fn the_attachment_states_serialize_in_snake_case() {
        let json = serde_json::to_string(&DaemonAttachment::Supervised).expect("serialize");
        assert_eq!(json, "\"supervised\"");
        let json = serde_json::to_string(&DaemonAttachment::Disconnected).expect("serialize");
        assert_eq!(json, "\"disconnected\"");
        let json = serde_json::to_string(&DaemonAttachment::Operator).expect("serialize");
        assert_eq!(json, "\"operator\"");
    }

    #[test]
    fn the_readiness_bounds_are_named_and_bounded() {
        assert!(READINESS_TIMEOUT <= Duration::from_secs(300));
        assert!(READINESS_TIMEOUT >= Duration::from_secs(30));
        assert!(READINESS_POLL_INTERVAL < READINESS_TIMEOUT);
    }

    #[test]
    fn a_liveness_probe_on_a_dead_socket_reports_no_daemon() {
        // Nothing is listening on this path, so the probe must say so. The
        // probe is a connection attempt: a socket file that does not exist
        // cannot answer, and a stale file would answer the same way.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a current-thread runtime");
        let alive = runtime.block_on(daemon_is_live("/tmp/eggsec-gui-no-such-socket.sock"));
        assert!(
            !alive,
            "nothing answers on a socket path that does not exist"
        );
    }

    #[test]
    #[cfg(unix)]
    fn a_process_that_exits_is_reported_as_a_failed_start() {
        // A process that exits immediately is not a daemon that is still
        // starting. The GUI must say so, with the child's exit status, and
        // must not leave the process behind.
        if !Path::new("/bin/echo").exists() {
            return;
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a current-thread runtime");
        let error = runtime
            .block_on(spawn_and_supervise(
                Path::new("/bin/echo"),
                "/tmp/eggsec-gui-no-such-socket.sock",
            ))
            .expect_err("a process that exits is not a daemon");

        let message = error.to_string();
        assert!(
            message.contains("the daemon failed to start"),
            "unexpected error: {message}"
        );
        assert!(
            message.contains("exited"),
            "the child's exit status must be reported: {message}"
        );
        assert!(
            !message.contains("still starting"),
            "a dead process must not be reported as starting: {message}"
        );
    }
}
