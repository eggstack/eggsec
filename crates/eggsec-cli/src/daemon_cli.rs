//! Daemon client CLI commands.
//!
//! Provides `daemon`, `session`, and `task` subcommands that communicate
//! with a running eggsec daemon over Unix domain socket.

use anyhow::{bail, Result};
use eggsec::cli::{Cli, Commands};
use eggsec_daemon::client::DaemonClient;
use eggsec_daemon::host::{DaemonHost, DaemonLiveness};
use eggsec_daemon_protocol::protocol::{ServerMessage, DAEMON_PROTOCOL_VERSION};
use eggsec_runtime::RuntimeSurface;
use eggsec_ui_model::{EventView, SessionSummaryView, SessionView};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// How long the daemon start path waits for the daemon to answer `Health`.
///
/// The wait is bounded so a daemon that never comes up fails the command
/// instead of hanging the operator's shell. The bound sits inside the
/// workspace's 30-300 s band for long-lived spawned work, and it is a
/// parameter of [`wait_for_daemon_ready`] so a test can exercise the bound
/// without `tokio`'s `test-util` feature.
pub const DAEMON_READINESS_TIMEOUT: Duration = Duration::from_secs(60);

/// How long the start path waits between readiness probes.
///
/// The first probe is immediate, so this interval only governs how quickly a
/// slow daemon is noticed. It is deliberately a whole second: a readiness
/// poll is not the fixed sleep the milestone removed, and keeping it above a
/// second keeps the two visibly distinct.
const READINESS_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// How long the start path waits for the server task to report its result
/// after the shutdown token is cancelled.
const SERVER_STOP_TIMEOUT: Duration = Duration::from_secs(10);

/// The outcome of a daemon start attempt.
///
/// The outcome is the only source of truth for what the command prints: the
/// success message is produced from [`DaemonStartOutcome::Ready`] and from
/// nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DaemonStartOutcome {
    /// The daemon answered `Health` on the socket path.
    Ready {
        socket_path: String,
        protocol_version: u32,
        advertised_task_kinds: usize,
    },
    /// A live daemon already serves the socket path.
    Refused {
        socket_path: String,
        protocol_version: u32,
    },
    /// The daemon did not come up.
    Failed { socket_path: String, cause: String },
}

/// What a readiness probe learned about the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReadyProbe {
    protocol_version: u32,
    advertised_task_kinds: usize,
}

/// Why a readiness wait gave up.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReadyFailure {
    /// The daemon did not answer within the bound.
    TimedOut { timeout: Duration },
    /// The shutdown token was cancelled during the wait.
    Cancelled,
    /// The server task finished before the daemon answered.
    ServerFailed { cause: String },
    /// The daemon speaks a protocol version this client does not.
    ProtocolMismatch { found: u32, expected: u32 },
}

/// The daemon server a start attempt spawned.
///
/// The result channel is kept so the command observes the server's own
/// result on shutdown instead of dropping it — a bind failure has to reach
/// the operator, not vanish into a detached task.
struct DaemonServer {
    result: tokio::sync::oneshot::Receiver<Result<(), eggsec_daemon::error::DaemonError>>,
}

impl DaemonServer {
    /// Wait for the server's own result after the shutdown was requested.
    ///
    /// The result may already have been taken: the server task can fail
    /// before the daemon answers — the bind guard refuses a path a live
    /// daemon holds — and tokio's oneshot **panics** when a completed
    /// receiver is polled again. The result is therefore taken with
    /// `try_recv` first, and awaited only while it is still pending.
    async fn stop(mut self) {
        match self.result.try_recv() {
            Ok(Ok(())) => {
                tracing::info!("Daemon server stopped cleanly");
                return;
            }
            Ok(Err(error)) => {
                tracing::warn!(error = %error, "Daemon server stopped with an error");
                return;
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                tracing::warn!("Daemon server result could not be received");
                return;
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
        }
        match tokio::time::timeout(SERVER_STOP_TIMEOUT, self.result).await {
            Ok(Ok(Ok(()))) => tracing::info!("Daemon server stopped cleanly"),
            Ok(Ok(Err(error))) => {
                tracing::warn!(error = %error, "Daemon server stopped with an error")
            }
            Ok(Err(error)) => {
                tracing::warn!(error = %error, "Daemon server result could not be received")
            }
            Err(_) => tracing::warn!("Daemon server did not report a result within its bound"),
        }
    }
}

/// Returns `true` if the command is a daemon client command.
pub fn is_daemon_command(cmd: &Commands) -> bool {
    matches!(
        cmd,
        Commands::Daemon(_) | Commands::Session(_) | Commands::Task(_)
    )
}

/// Dispatch a daemon client command.
pub async fn handle_daemon_command(cmd: &Commands, cli: &Cli) -> Result<()> {
    let socket_path = cli.socket.clone();
    match cmd {
        Commands::Daemon(args) => handle_daemon(args, &socket_path, cli.json).await,
        Commands::Session(args) => handle_session(args, &socket_path, cli.json).await,
        Commands::Task(args) => handle_task(args, &socket_path, cli.json).await,
        _ => bail!("not a daemon command"),
    }
}

async fn connect(socket_path: &str) -> Result<DaemonClient> {
    DaemonClient::connect(socket_path).await
}

/// Start the daemon on `socket_path` and wait until it is demonstrably up.
///
/// The returned outcome is the only source of truth for what the command
/// prints. The start path hosts the **engine** executor — the CLI already
/// links the engine with the `cli` feature, so the CLI-hosted daemon can
/// execute what it advertises instead of rejecting every task. It does
/// **not** persist sessions: the store is a no-op store, and the daemon
/// binary is the persistent one.
async fn start_daemon(
    socket_path: &str,
    config: eggsec_daemon::config::DaemonConfig,
    shutdown: CancellationToken,
    readiness_timeout: Duration,
) -> (DaemonStartOutcome, Option<DaemonServer>) {
    // Refuse to displace a live daemon. The server's own bind guard refuses
    // too — it binds without unlinking and asks the same probe — but this
    // pre-flight answers first, so the operator is told before the server
    // task is even spawned.
    match eggsec_daemon::host::probe_daemon_liveness(
        socket_path,
        eggsec_daemon::host::PREFLIGHT_PROBE_TIMEOUT,
    )
    .await
    {
        DaemonLiveness::Live { protocol_version } => {
            return (
                DaemonStartOutcome::Refused {
                    socket_path: socket_path.to_string(),
                    protocol_version,
                },
                None,
            );
        }
        DaemonLiveness::NotLive => {}
    }

    let host = std::sync::Arc::new(DaemonHost::new(
        config,
        eggsec::runtime_bridge::EggsecRuntimeExecutor::new(),
        eggsec_daemon::store::noop_store(),
    ));
    let shutdown_for_server = shutdown.clone();
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        // The accept loop is long-lived by nature — it lives until the
        // shutdown token is cancelled — so it carries no timeout wrapper.
        let result = eggsec_daemon::server::run_server(host, shutdown_for_server).await;
        // Report the server's own result so a bind failure ends the start
        // path with the daemon's error instead of a timeout.
        if result_tx.send(result).is_err() {
            tracing::debug!("The daemon start path dropped the server's result");
        }
    });
    let mut server = DaemonServer { result: result_rx };

    match wait_for_daemon_ready(
        socket_path,
        readiness_timeout,
        &shutdown,
        &mut server.result,
    )
    .await
    {
        Ok(probe) => (
            DaemonStartOutcome::Ready {
                socket_path: socket_path.to_string(),
                protocol_version: probe.protocol_version,
                advertised_task_kinds: probe.advertised_task_kinds,
            },
            Some(server),
        ),
        Err(failure) => {
            let cause = match failure {
                ReadyFailure::TimedOut { timeout } => {
                    format!("the daemon did not answer Health within {timeout:?}")
                }
                ReadyFailure::Cancelled => {
                    "the start was cancelled before the daemon answered Health".to_string()
                }
                ReadyFailure::ServerFailed { cause } => cause,
                ReadyFailure::ProtocolMismatch { found, expected } => format!(
                    "the daemon reports protocol version {found}, but this client speaks version {expected}"
                ),
            };
            // Stop the half-started daemon so it does not linger on the path.
            shutdown.cancel();
            let server = Some(server);
            (
                DaemonStartOutcome::Failed {
                    socket_path: socket_path.to_string(),
                    cause,
                },
                server,
            )
        }
    }
}

/// Wait until the daemon on `socket_path` answers `Health`.
///
/// The criterion is a protocol exchange, never a log scrape: the daemon's
/// readiness line is a log format, not a contract, and the CLI hosts the
/// server in-process, so there is no child stdout to read at all.
///
/// The wait ends three ways: the daemon answers (`Ok`), the server task
/// fails (`ServerFailed`, carrying the daemon's own error), the shutdown
/// token is cancelled (`Cancelled`), or the bound elapses (`TimedOut`).
async fn wait_for_daemon_ready(
    socket_path: &str,
    timeout: Duration,
    cancel: &CancellationToken,
    server_result: &mut tokio::sync::oneshot::Receiver<
        Result<(), eggsec_daemon::error::DaemonError>,
    >,
) -> Result<ReadyProbe, ReadyFailure> {
    let wait = async {
        loop {
            match probe_ready(socket_path).await {
                Ok(probe) => return Ok(probe),
                Err(ProbeFailure::ProtocolMismatch { found, expected }) => {
                    // A version mismatch is not transient: report it rather
                    // than retrying until the bound elapses.
                    return Err(ReadyFailure::ProtocolMismatch { found, expected });
                }
                Err(ProbeFailure::Unreachable(error)) => {
                    tracing::debug!(path = %socket_path, error = %error, "Daemon is not ready yet");
                }
            }
            tokio::select! {
                biased;
                result = &mut *server_result => {
                    return Err(ReadyFailure::ServerFailed {
                        cause: describe_server_result(result),
                    });
                }
                _ = cancel.cancelled() => {
                    return Err(ReadyFailure::Cancelled);
                }
                _ = tokio::time::sleep(READINESS_POLL_INTERVAL) => {}
            }
        }
    };

    match tokio::time::timeout(timeout, wait).await {
        Ok(result) => result,
        Err(_) => Err(ReadyFailure::TimedOut { timeout }),
    }
}

/// Why a readiness probe could not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProbeFailure {
    /// The daemon did not answer usefully. This is transient: the start
    /// path retries until its bound elapses.
    Unreachable(String),
    /// The daemon answered, but speaks a protocol version this client does
    /// not. This is not transient.
    ProtocolMismatch { found: u32, expected: u32 },
}

/// Ask the daemon on `socket_path` whether it is up.
///
/// The probe is read-only: `Health` and `Capabilities` need no client
/// declaration and write nothing, so a probe creates no session, no task,
/// and no audit record.
async fn probe_ready(socket_path: &str) -> Result<ReadyProbe, ProbeFailure> {
    let probe = async {
        let mut client = DaemonClient::connect(socket_path)
            .await
            .map_err(|error| ProbeFailure::Unreachable(error.to_string()))?;
        let protocol_version = match client
            .health()
            .await
            .map_err(|error| ProbeFailure::Unreachable(error.to_string()))?
        {
            ServerMessage::Health {
                protocol_version, ..
            } => protocol_version,
            other => {
                return Err(ProbeFailure::Unreachable(format!(
                    "unexpected response to Health: {other:?}"
                )))
            }
        };
        // The version is checked the moment the daemon answers: a daemon
        // this client does not understand must be reported, not accepted
        // and then queried further.
        if protocol_version != DAEMON_PROTOCOL_VERSION {
            return Err(ProbeFailure::ProtocolMismatch {
                found: protocol_version,
                expected: DAEMON_PROTOCOL_VERSION,
            });
        }
        let advertised_task_kinds = match client
            .capabilities()
            .await
            .map_err(|error| ProbeFailure::Unreachable(error.to_string()))?
        {
            ServerMessage::Capabilities { capabilities, .. } => {
                capabilities.runtime.task_kinds.len()
            }
            other => {
                return Err(ProbeFailure::Unreachable(format!(
                    "unexpected response to Capabilities: {other:?}"
                )))
            }
        };
        Ok(ReadyProbe {
            protocol_version,
            advertised_task_kinds,
        })
    };

    match tokio::time::timeout(eggsec_daemon::host::PREFLIGHT_PROBE_TIMEOUT, probe).await {
        Ok(result) => result,
        Err(_) => Err(ProbeFailure::Unreachable(
            "the readiness probe did not complete within its bound".to_string(),
        )),
    }
}

/// Describe the server task's own result for the operator.
fn describe_server_result(
    result: Result<
        Result<(), eggsec_daemon::error::DaemonError>,
        tokio::sync::oneshot::error::RecvError,
    >,
) -> String {
    match result {
        Ok(Ok(())) => "the daemon server stopped before it answered Health".to_string(),
        Ok(Err(error)) => format!("the daemon server failed: {error}"),
        Err(error) => format!("the daemon server result could not be received: {error}"),
    }
}

/// The success message for a start outcome, or `None` for a failure.
///
/// Only the ready outcome produces a message: the failure outcomes become
/// the command's error, so a start that did not come up never prints a
/// success line.
fn start_success_message(outcome: &DaemonStartOutcome, json: bool) -> Option<String> {
    match outcome {
        DaemonStartOutcome::Ready {
            socket_path,
            protocol_version,
            advertised_task_kinds,
        } => {
            if json {
                Some(
                    serde_json::json!({
                        "status": "started",
                        "socket": socket_path,
                        "protocol_version": protocol_version,
                        "advertised_task_kinds": advertised_task_kinds,
                    })
                    .to_string(),
                )
            } else {
                Some(format!(
                    "Daemon is listening on {socket_path} — protocol version {protocol_version}, \
                     {advertised_task_kinds} advertised task kinds.\n\
                     An advertised capability is not a promise that this build can execute it: \
                     see docs/DAEMON.md for what each kind needs.\n\
                     This daemon does not persist sessions — run the `eggsec-daemon` binary for \
                     a persistent daemon."
                ))
            }
        }
        DaemonStartOutcome::Refused { .. } | DaemonStartOutcome::Failed { .. } => None,
    }
}

/// The failure message for a start outcome, or `None` for a success.
fn start_failure_message(outcome: &DaemonStartOutcome) -> Option<String> {
    match outcome {
        DaemonStartOutcome::Refused {
            socket_path,
            protocol_version,
        } => Some(eggsec_daemon::error::socket_path_held_message(
            socket_path,
            protocol_version,
        )),
        DaemonStartOutcome::Failed { socket_path, cause } => Some(format!(
            "could not bring up a daemon on {socket_path}: {cause}"
        )),
        DaemonStartOutcome::Ready { .. } => None,
    }
}

/// Wait for the operator's interrupt (SIGINT or SIGTERM).
#[cfg(unix)]
async fn wait_for_interrupt() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to install SIGTERM handler; waiting for Ctrl-C only");
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!(error = %e, "Ctrl-C handler failed");
            }
            return;
        }
    };
    tokio::select! {
        _ = sigterm.recv() => {}
        result = tokio::signal::ctrl_c() => {
            if let Err(e) = result {
                tracing::error!(error = %e, "Ctrl-C handler failed");
            }
        }
    }
}

/// Wait for the operator's interrupt (SIGINT).
#[cfg(not(unix))]
async fn wait_for_interrupt() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %e, "Ctrl-C handler failed");
    }
}

async fn handle_daemon(
    args: &eggsec::cli::DaemonArgs,
    socket_path: &str,
    json: bool,
) -> Result<()> {
    match &args.subcommand {
        eggsec::cli::DaemonSubcommand::Start { socket } => {
            let path = socket.as_deref().unwrap_or(socket_path).to_string();
            let config = eggsec_daemon::config::DaemonConfig {
                socket_path: path.clone(),
                ..Default::default()
            };
            let shutdown = CancellationToken::new();
            let (outcome, server) =
                start_daemon(&path, config, shutdown.clone(), DAEMON_READINESS_TIMEOUT).await;

            // The success message exists only for the ready outcome. The
            // failure outcomes become the command's error, so the operator
            // sees the cause once, with a non-zero exit.
            if let Some(message) = start_success_message(&outcome, json) {
                println!("{message}");
            }
            match outcome {
                DaemonStartOutcome::Ready { .. } => {
                    // Keep running until interrupted.
                    wait_for_interrupt().await;
                    shutdown.cancel();
                    if let Some(server) = server {
                        server.stop().await;
                    }
                    if json {
                        println!(r#"{{"status":"stopped"}}"#);
                    } else {
                        println!("Daemon stopped.");
                    }
                }
                _ => {
                    if let Some(server) = server {
                        server.stop().await;
                    }
                    let message = start_failure_message(&outcome)
                        .unwrap_or_else(|| "the daemon did not come up".to_string());
                    bail!("{message}")
                }
            }
        }
        eggsec::cli::DaemonSubcommand::Status { socket } => {
            let path = socket.as_deref().unwrap_or(socket_path);
            let mut client = connect(path).await?;
            // Declare client kind
            if let Err(e) = client
                .declare_client(
                    eggsec_daemon_protocol::client_registry::ClientKind::Cli,
                    Some("eggsec-cli".into()),
                )
                .await
            {
                tracing::warn!(error = %e, "Failed to declare client to daemon (continuing)");
            }
            let resp = client.health().await?;
            match resp {
                ServerMessage::Health {
                    status, version, ..
                } => {
                    if json {
                        println!(
                            r#"{{"status":"{}","version":"{}","socket":"{}"}}"#,
                            status, version, path
                        );
                    } else {
                        println!("Daemon status: {} (v{})", status, version);
                    }
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
        eggsec::cli::DaemonSubcommand::Stop { socket } => {
            let path = socket.as_deref().unwrap_or(socket_path);
            // Send a health check to verify daemon is running, then we can't
            // really stop it from the client side without a dedicated command.
            // For now, just report status.
            match connect(path).await {
                Ok(mut client) => {
                    if let Err(e) = client
                        .declare_client(
                            eggsec_daemon_protocol::client_registry::ClientKind::Cli,
                            Some("eggsec-cli".into()),
                        )
                        .await
                    {
                        tracing::warn!(error = %e, "Failed to declare client to daemon (continuing)");
                    }
                    match client.health().await {
                        Ok(_) => {
                            if json {
                                println!(
                                    r#"{{"status":"running","message":"daemon is running (stop via SIGTERM or ctrl-c on the daemon process)"}}"#
                                );
                            } else {
                                println!("Daemon is running. Stop it via SIGTERM or ctrl-c on the daemon process.");
                            }
                        }
                        Err(e) => {
                            bail!("Daemon not reachable: {}", e);
                        }
                    }
                }
                Err(e) => {
                    bail!("Could not connect to daemon at {}: {}", path, e);
                }
            }
        }
        eggsec::cli::DaemonSubcommand::History { json } => {
            let path = socket_path;
            let mut client = connect(path).await?;
            if let Err(e) = client
                .declare_client(
                    eggsec_daemon_protocol::client_registry::ClientKind::Cli,
                    Some("cli-history".into()),
                )
                .await
            {
                tracing::warn!(error = %e, "Failed to declare client to daemon (continuing)");
            }
            let resp = client.list_persisted_sessions().await?;
            match resp {
                ServerMessage::PersistedSessions { sessions, .. } => {
                    if *json {
                        println!("{}", serde_json::to_string_pretty(&sessions)?);
                    } else {
                        if sessions.is_empty() {
                            println!("No persisted sessions found.");
                        } else {
                            println!(
                                "{:<40} {:<15} {:<8} {:<8}",
                                "SESSION ID", "SURFACE", "ACTIVE", "DONE"
                            );
                            println!("{}", "-".repeat(73));
                            for s in &sessions {
                                let view = SessionSummaryView::from(s);
                                println!(
                                    "{:<40} {:<15} {:<8} {:<8}",
                                    view.session_id,
                                    view.surface_label,
                                    view.active_count,
                                    view.completed_count
                                );
                            }
                        }
                    }
                }
                ServerMessage::Error { code, message, .. } => {
                    bail!("Daemon error ({:?}): {}", code, message);
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
        eggsec::cli::DaemonSubcommand::Show { session_id, json } => {
            let path = socket_path;
            let mut client = connect(path).await?;
            if let Err(e) = client
                .declare_client(
                    eggsec_daemon_protocol::client_registry::ClientKind::Cli,
                    Some("cli-show".into()),
                )
                .await
            {
                tracing::warn!(error = %e, "Failed to declare client to daemon (continuing)");
            }
            let sid: eggsec_runtime::SessionId = session_id.parse()?;
            let resp = client.get_persisted_snapshot(sid).await?;
            match resp {
                ServerMessage::PersistedSnapshot {
                    snapshot: Some(snapshot),
                    ..
                } => {
                    if *json {
                        println!("{}", serde_json::to_string_pretty(&snapshot)?);
                    } else {
                        let view = SessionView::from(&snapshot);
                        println!("Session: {}", view.session_id);
                        println!("Surface: {}", view.surface_label);
                        if let Some(ref scope) = view.scope {
                            println!("Scope: {} (explicit: {})", scope.source, scope.is_explicit);
                        }
                        println!("Active tasks: {}", view.active_tasks.len());
                        for t in &view.active_tasks {
                            println!(
                                "  [{}] {} - {}",
                                t.task_id, t.request_summary, t.status_label
                            );
                        }
                        println!("Completed tasks: {}", view.completed_tasks.len());
                        for t in &view.completed_tasks {
                            println!(
                                "  [{}] {} - {}",
                                t.task_id, t.request_summary, t.status_label
                            );
                        }
                    }
                }
                ServerMessage::PersistedSnapshot { snapshot: None, .. } => {
                    println!("No persisted snapshot found for session {}", session_id);
                }
                ServerMessage::Error { code, message, .. } => {
                    bail!("Daemon error ({:?}): {}", code, message);
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
    }
    Ok(())
}

async fn handle_session(
    args: &eggsec::cli::SessionArgs,
    socket_path: &str,
    json: bool,
) -> Result<()> {
    let mut client = connect(socket_path).await?;
    // Declare client kind
    if let Err(e) = client
        .declare_client(
            eggsec_daemon_protocol::client_registry::ClientKind::Cli,
            Some("eggsec-cli".into()),
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to declare client to daemon (continuing)");
    }
    match &args.subcommand {
        eggsec::cli::SessionSubcommand::List => {
            let resp = client.list_sessions().await?;
            match resp {
                ServerMessage::Sessions { sessions, .. } => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&sessions)?);
                    } else {
                        if sessions.is_empty() {
                            println!("No active sessions.");
                        } else {
                            println!(
                                "{:<38} {:<15} {:<8} {:<8}",
                                "SESSION ID", "SURFACE", "ACTIVE", "DONE"
                            );
                            println!("{}", "-".repeat(71));
                            for s in &sessions {
                                let view = SessionSummaryView::from(s);
                                println!(
                                    "{:<38} {:<15} {:<8} {:<8}",
                                    view.session_id,
                                    view.surface_label,
                                    view.active_count,
                                    view.completed_count
                                );
                            }
                        }
                    }
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
        eggsec::cli::SessionSubcommand::Create { surface } => {
            let surface = parse_surface(surface.as_deref())?;
            let surface_label = surface.label().to_string();
            let resp = client.create_session(surface, None, vec![]).await?;
            match resp {
                ServerMessage::SessionCreated { session_id, .. } => {
                    if json {
                        println!(
                            r#"{{"session_id":"{}","surface":"{}"}}"#,
                            session_id, surface_label
                        );
                    } else {
                        println!(
                            "Session created: {} (surface: {})",
                            session_id, surface_label
                        );
                    }
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
        eggsec::cli::SessionSubcommand::Snapshot { session_id } => {
            let sid: eggsec_runtime::SessionId = session_id.parse()?;
            let resp = client.get_snapshot(sid).await?;
            match resp {
                ServerMessage::Snapshot { snapshot, .. } => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&snapshot)?);
                    } else {
                        let view = SessionView::from(&snapshot);
                        println!("Session: {}", view.session_id);
                        println!("Surface: {}", view.surface_label);
                        if let Some(ref scope) = view.scope {
                            println!("Scope: {} (explicit: {})", scope.source, scope.is_explicit);
                        }
                        println!("Active tasks: {}", view.active_tasks.len());
                        for t in &view.active_tasks {
                            println!(
                                "  [{}] {} - {}",
                                t.task_id, t.request_summary, t.status_label
                            );
                        }
                        for t in &view.completed_tasks {
                            println!(
                                "  [{}] {} - {}",
                                t.task_id, t.request_summary, t.status_label
                            );
                        }
                    }
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
    }
    Ok(())
}

async fn handle_task(args: &eggsec::cli::TaskArgs, socket_path: &str, json: bool) -> Result<()> {
    let mut client = connect(socket_path).await?;
    // Declare client kind
    if let Err(e) = client
        .declare_client(
            eggsec_daemon_protocol::client_registry::ClientKind::Cli,
            Some("eggsec-cli".into()),
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to declare client to daemon (continuing)");
    }
    match &args.subcommand {
        eggsec::cli::TaskSubcommand::Submit {
            session_id,
            kind,
            target,
        } => {
            let sid: eggsec_runtime::SessionId = session_id.parse()?;
            let request = build_run_request(kind, target)?;
            let resp = client.submit_task(sid, request).await?;
            match resp {
                ServerMessage::TaskSubmitted { task_id, .. } => {
                    if json {
                        println!(
                            r#"{{"task_id":"{}","session_id":"{}","kind":"{}"}}"#,
                            task_id, session_id, kind
                        );
                    } else {
                        println!(
                            "Task {} submitted to session {} (kind: {})",
                            task_id, session_id, kind
                        );
                    }
                }
                ServerMessage::Error { code, message, .. } => {
                    bail!("Daemon error ({:?}): {}", code, message);
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
        eggsec::cli::TaskSubcommand::Cancel {
            session_id,
            task_id,
        } => {
            let sid: eggsec_runtime::SessionId = session_id.parse()?;
            let tid: eggsec_runtime::TaskId = task_id.parse()?;
            let resp = client.cancel_task(sid, tid).await?;
            match resp {
                ServerMessage::Ok { .. } => {
                    if json {
                        println!(
                            r#"{{"status":"cancelled","session_id":"{}","task_id":"{}"}}"#,
                            session_id, task_id
                        );
                    } else {
                        println!("Task {} cancelled.", task_id);
                    }
                }
                ServerMessage::Error { code, message, .. } => {
                    bail!("Daemon error ({:?}): {}", code, message);
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
        eggsec::cli::TaskSubcommand::Result {
            session_id,
            task_id,
        } => {
            let sid: eggsec_runtime::SessionId = session_id.parse()?;
            let tid: eggsec_runtime::TaskId = task_id.parse()?;
            let resp = client.get_task_result(sid, tid).await?;
            match resp {
                ServerMessage::TaskResult {
                    status, outcome, ..
                } => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "session_id": session_id,
                                "task_id": task_id,
                                "status": status,
                                "outcome": outcome,
                            }))?
                        );
                    } else {
                        println!("Task {} in session {}: {:?}", task_id, session_id, status);
                        match outcome {
                            Some(o) => println!("Outcome: {}", serde_json::to_string_pretty(&o)?),
                            None => println!("Outcome: pending (task not yet complete)"),
                        }
                    }
                }
                ServerMessage::Error { code, message, .. } => {
                    bail!("Daemon error ({:?}): {}", code, message);
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
        eggsec::cli::TaskSubcommand::Watch { session_id } => {
            let sid: eggsec_runtime::SessionId = session_id.parse()?;
            let mut receiver = client.subscribe(sid).await?;
            // Stream events until interrupted
            loop {
                tokio::select! {
                    event = receiver.recv() => {
                        match event {
                            Some(event) => {
                                if json {
                                    println!("{}", serde_json::to_string(&event)?);
                                } else {
                                    print_event(&event);
                                }
                            }
                            None => {
                                if json {
                                    println!(r#"{{"status":"stream_ended"}}"#);
                                } else {
                                    println!("Event stream ended.");
                                }
                                break;
                            }
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_surface(s: Option<&str>) -> Result<RuntimeSurface> {
    match s.unwrap_or("cli-manual") {
        "cli-manual" => Ok(RuntimeSurface::CliManual),
        "cli-manual-strict" => Ok(RuntimeSurface::CliManualStrict),
        "ci" => Ok(RuntimeSurface::Ci),
        "mcp-server" => Ok(RuntimeSurface::McpServer),
        "rest-api" => Ok(RuntimeSurface::RestApi),
        "grpc-api" => Ok(RuntimeSurface::GrpcApi),
        "security-agent" => Ok(RuntimeSurface::SecurityAgent),
        other => bail!(
            "Unknown surface '{}'. Valid surfaces: cli-manual, cli-manual-strict, ci, mcp-server, rest-api, grpc-api, security-agent",
            other
        ),
    }
}

fn build_run_request(kind: &str, target: &str) -> Result<eggsec_runtime::RunRequest> {
    use eggsec_runtime::request::*;
    let task_kind = match kind {
        "port-scan" => TaskKind::PortScan(PortScanParams {
            target: target.into(),
            ports: None,
            scan_type: None,
            timeout_ms: None,
            ..Default::default()
        }),
        "endpoint-scan" => TaskKind::EndpointScan(EndpointScanParams {
            target: target.into(),
            methods: None,
            wordlist: None,
            ..Default::default()
        }),
        "fingerprint" => TaskKind::Fingerprint(FingerprintParams {
            target: target.into(),
            ..Default::default()
        }),
        "fuzz" => TaskKind::Fuzz(FuzzParams {
            target: target.into(),
            payload_type: None,
            threads: None,
            ..Default::default()
        }),
        "waf" => TaskKind::Waf(WafParams {
            target: target.into(),
            ..Default::default()
        }),
        "recon" => TaskKind::Recon(ReconParams {
            target: target.into(),
            modules: None,
        }),
        "load-test" => TaskKind::LoadTest(LoadTestParams {
            target: target.into(),
            method: "GET".into(),
            connections: None,
            duration_secs: None,
            rate_limit: None,
            ..Default::default()
        }),
        "pipeline" => TaskKind::Pipeline(PipelineParams {
            target: target.into(),
            profile: None,
            output_format: None,
            output_file: None,
            session_path: None,
        }),
        "auth-test" => TaskKind::AuthTest(AuthTestParams {
            target: target.into(),
            username: None,
            credential_list: None,
            ..Default::default()
        }),
        "hunt" => TaskKind::Hunt(HuntParams {
            target: target.into(),
            hunt_type: None,
        }),
        other => bail!(
            "Unknown task kind '{}'. Valid kinds: port-scan, endpoint-scan, fingerprint, fuzz, waf, recon, load-test, pipeline, auth-test, hunt",
            other
        ),
    };
    Ok(eggsec_runtime::RunRequest {
        task_kind,
        requested_by: None,
        surface: RuntimeSurface::CliManual,
        labels: vec![],
    })
}

fn print_event(event: &eggsec_runtime::RuntimeEvent) {
    let view = EventView::from(event);
    if let Some(ref msg) = view.message {
        println!("[{}] {}", view.event_type, msg);
    } else {
        println!("[{}]", view.event_type);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_daemon_command_matches_daemon() {
        let cmd = Commands::Daemon(eggsec::cli::DaemonArgs {
            subcommand: eggsec::cli::DaemonSubcommand::Status { socket: None },
        });
        assert!(is_daemon_command(&cmd));
    }

    #[test]
    fn is_daemon_command_matches_session() {
        let cmd = Commands::Session(eggsec::cli::SessionArgs {
            subcommand: eggsec::cli::SessionSubcommand::List,
        });
        assert!(is_daemon_command(&cmd));
    }

    #[test]
    fn is_daemon_command_matches_task() {
        let cmd = Commands::Task(eggsec::cli::TaskArgs {
            subcommand: eggsec::cli::TaskSubcommand::Cancel {
                session_id: "test".into(),
                task_id: "test".into(),
            },
        });
        assert!(is_daemon_command(&cmd));
    }

    #[test]
    fn is_daemon_command_rejects_non_daemon() {
        // Doctor is a simple command with no args
        assert!(!is_daemon_command(&Commands::Doctor));
    }

    #[test]
    fn parse_surface_valid() {
        assert_eq!(
            parse_surface(Some("cli-manual")).unwrap(),
            RuntimeSurface::CliManual
        );
        assert_eq!(parse_surface(Some("ci")).unwrap(), RuntimeSurface::Ci);
        assert_eq!(
            parse_surface(Some("rest-api")).unwrap(),
            RuntimeSurface::RestApi
        );
    }

    #[test]
    fn parse_surface_default() {
        assert_eq!(parse_surface(None).unwrap(), RuntimeSurface::CliManual);
    }

    #[test]
    fn parse_surface_invalid() {
        assert!(parse_surface(Some("tui-manual")).is_err());
        assert!(parse_surface(Some("invalid")).is_err());
    }

    #[test]
    fn build_request_port_scan() {
        let req = build_run_request("port-scan", "10.0.0.1").unwrap();
        assert!(matches!(
            req.task_kind,
            eggsec_runtime::TaskKind::PortScan(_)
        ));
        assert_eq!(req.surface, RuntimeSurface::CliManual);
    }

    #[test]
    fn build_request_fuzz() {
        let req = build_run_request("fuzz", "http://example.com").unwrap();
        assert!(matches!(req.task_kind, eggsec_runtime::TaskKind::Fuzz(_)));
    }

    #[test]
    fn build_request_invalid_kind() {
        assert!(build_run_request("invalid", "target").is_err());
    }

    /// A unique socket path under the system temp directory.
    ///
    /// The path stays short on purpose: a Unix socket path is bounded by
    /// `sun_path`.
    fn temp_socket(label: &str) -> String {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!(
            "{}/eggsec-m009-cli-{}-{}-{}.sock",
            std::env::temp_dir().display(),
            label,
            std::process::id(),
            counter
        )
    }

    /// Serve a daemon that answers `Health` with a protocol version this
    /// client does not speak.
    ///
    /// The handshake is written by hand so the test can pin a daemon that
    /// speaks a different protocol version without a second implementation
    /// of the protocol. Every connection is answered, so a probe that
    /// arrives at any time is served.
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
                let command =
                    serde_json::from_str::<eggsec_daemon_protocol::protocol::ClientCommand>(&line)
                        .ok();
                let request_id = command
                    .as_ref()
                    .map(|command| command.request_id().to_owned())
                    .unwrap_or_else(|| "foreign".to_string());
                // The daemon answers every command correctly; only its
                // protocol version is wrong, so the readiness check cannot
                // pass for the wrong reason.
                let response = match &command {
                    Some(eggsec_daemon_protocol::protocol::ClientCommand::Capabilities {
                        ..
                    }) => eggsec_daemon_protocol::protocol::ServerMessage::Capabilities {
                        request_id,
                        capabilities: eggsec_daemon_protocol::protocol::DaemonCapabilities {
                            runtime: eggsec_runtime::RuntimeCapabilities::canonical(),
                            transports: vec![],
                            operation_schema: true,
                            enabled_features: None,
                            report_forms: None,
                        },
                    },
                    _ => eggsec_daemon_protocol::protocol::ServerMessage::Health {
                        request_id,
                        status: "ok".into(),
                        version: "0.0.0-foreign".into(),
                        protocol_version: eggsec_daemon_protocol::protocol::DAEMON_PROTOCOL_VERSION
                            + 1,
                    },
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

    /// A host that answers `Health` and `Capabilities` over the real socket.
    ///
    /// The engine executor is the one the CLI's start path hosts, so the
    /// published list is the one a real CLI-hosted daemon publishes.
    fn cli_host(socket_path: &str) -> eggsec_daemon::host::DaemonHost {
        eggsec_daemon::host::DaemonHost::new(
            eggsec_daemon::config::DaemonConfig {
                socket_path: socket_path.to_string(),
                ..Default::default()
            },
            eggsec::runtime_bridge::EggsecRuntimeExecutor::new(),
            eggsec_daemon::store::noop_store(),
        )
    }

    /// The CLI's daemon-start path links the engine executor and carries no
    /// rejecting executor.
    #[tokio::test]
    async fn the_cli_daemon_start_path_links_the_engine_executor() {
        // Source-level: the CLI carries no rejecting executor. The needles
        // are assembled at runtime so this test does not itself contain the
        // literals it forbids.
        let rejecting_struct = ["struct ", "Noop", "Executor"].concat();
        let rejecting_error = ["RuntimeError::", "Unsupported", "TaskKind"].concat();
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&src_dir).expect("read the CLI's source directory") {
            let entry = entry.expect("read a source entry");
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read a source file");
            if source.contains(&rejecting_struct) {
                offenders.push(format!("{}: rejecting executor struct", path.display()));
            }
            if source.contains(&rejecting_error) {
                offenders.push(format!("{}: rejecting executor error", path.display()));
            }
        }
        assert!(
            offenders.is_empty(),
            "the CLI must carry no rejecting daemon executor: {offenders:?}"
        );

        // And the host the CLI's start path builds publishes a non-empty list.
        let host = cli_host(&temp_socket("capabilities"));
        let response = host
            .handle_command(
                eggsec_daemon_protocol::protocol::ClientCommand::Capabilities {
                    request_id: "cli-capabilities".into(),
                },
                eggsec_daemon_protocol::protocol::DaemonRequestContext {
                    client_id: None,
                    peer: None,
                    transport: eggsec_daemon_protocol::protocol::TransportKind::UnixSocket,
                },
            )
            .await;
        match response {
            eggsec_daemon_protocol::protocol::ServerMessage::Capabilities {
                capabilities, ..
            } => {
                assert!(
                    !capabilities.runtime.task_kinds.is_empty(),
                    "the CLI-hosted daemon must publish a non-empty task-kind list"
                );
            }
            other => panic!("expected Capabilities, got {other:?}"),
        }
    }

    /// The readiness wait's timeout is a named constant inside the
    /// workspace's 30-300 s band, and it is a parameter of the helper, so a
    /// test can exercise the bound without `tokio`'s `test-util` feature.
    #[test]
    fn the_readiness_wait_has_a_bounded_timeout() {
        assert!(
            DAEMON_READINESS_TIMEOUT >= Duration::from_secs(30),
            "the readiness timeout must be at least 30s, got {DAEMON_READINESS_TIMEOUT:?}"
        );
        assert!(
            DAEMON_READINESS_TIMEOUT <= Duration::from_secs(300),
            "the readiness timeout must be at most 300s, got {DAEMON_READINESS_TIMEOUT:?}"
        );
    }

    /// A start path whose socket cannot bind fails, and no success text is
    /// produced.
    #[tokio::test]
    async fn the_start_path_does_not_report_success_when_the_socket_fails_to_bind() {
        // A path whose parent directory does not exist can never be bound.
        let socket_path = format!(
            "{}/eggsec-m009-missing-{}-{}/daemon.sock",
            std::env::temp_dir().display(),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        );
        let config = eggsec_daemon::config::DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        };
        let shutdown = CancellationToken::new();
        let (outcome, _server) = start_daemon(
            &socket_path,
            config,
            shutdown.clone(),
            Duration::from_secs(5),
        )
        .await;

        assert!(
            matches!(outcome, DaemonStartOutcome::Failed { .. }),
            "the start path must fail, got {outcome:?}"
        );
        assert!(
            start_success_message(&outcome, false).is_none(),
            "a failed start must produce no success text"
        );
        assert!(
            start_success_message(&outcome, true).is_none(),
            "a failed start must produce no JSON payload"
        );
        let failure = start_failure_message(&outcome).expect("a failure message");
        assert!(
            failure.contains(&socket_path),
            "the failure message must name the socket path, got: {failure}"
        );
    }

    /// A failed server task must not panic the stop path.
    ///
    /// The readiness wait takes the server's result when the server task
    /// fails; the later `stop()` must not poll that completed receiver
    /// again — tokio's oneshot panics when it is.
    #[tokio::test]
    async fn the_stop_path_survives_a_server_task_that_already_failed() {
        let socket_path = format!(
            "{}/eggsec-m012-missing-{}-{}/daemon.sock",
            std::env::temp_dir().display(),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        );
        let config = eggsec_daemon::config::DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        };
        let shutdown = CancellationToken::new();
        let (outcome, server) = start_daemon(
            &socket_path,
            config,
            shutdown.clone(),
            Duration::from_secs(5),
        )
        .await;

        assert!(
            matches!(outcome, DaemonStartOutcome::Failed { .. }),
            "the start path must fail, got {outcome:?}"
        );
        let server = server.expect("a failed start keeps the server handle");
        // The stop path must not panic: the result was already taken.
        server.stop().await;
    }

    /// The readiness wait gives up within its bound when no daemon appears.
    #[tokio::test]
    async fn the_readiness_wait_times_out_when_no_daemon_appears() {
        let socket_path = temp_socket("timeout");
        let shutdown = CancellationToken::new();
        let (_keep_alive, mut result) = tokio::sync::oneshot::channel();

        let started = std::time::Instant::now();
        let outcome = wait_for_daemon_ready(
            &socket_path,
            Duration::from_millis(250),
            &shutdown,
            &mut result,
        )
        .await;
        let elapsed = started.elapsed();

        assert_eq!(
            outcome,
            Err(ReadyFailure::TimedOut {
                timeout: Duration::from_millis(250)
            }),
            "the wait must time out when no daemon appears"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "the wait must honour its own bound, took {elapsed:?}"
        );
    }

    /// The readiness wait succeeds against a live daemon and reports the
    /// protocol version.
    #[tokio::test]
    async fn the_readiness_wait_succeeds_against_a_live_daemon() {
        let socket_path = temp_socket("ready");
        let host = std::sync::Arc::new(cli_host(&socket_path));
        let shutdown = CancellationToken::new();
        let server_shutdown = shutdown.clone();
        let server =
            tokio::spawn(
                async move { eggsec_daemon::server::run_server(host, server_shutdown).await },
            );

        let probe_shutdown = CancellationToken::new();
        let (_keep_alive, mut result) = tokio::sync::oneshot::channel();
        let outcome = wait_for_daemon_ready(
            &socket_path,
            Duration::from_secs(10),
            &probe_shutdown,
            &mut result,
        )
        .await;

        match outcome {
            Ok(probe) => {
                assert_eq!(
                    probe.protocol_version,
                    eggsec_daemon_protocol::protocol::DAEMON_PROTOCOL_VERSION
                );
                assert!(
                    probe.advertised_task_kinds > 0,
                    "a live daemon must advertise a non-empty task-kind list"
                );
            }
            Err(failure) => panic!("the readiness wait must succeed, got {failure:?}"),
        }

        shutdown.cancel();
        match server.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("the server failed: {error}"),
            Err(error) => panic!("the server task failed: {error}"),
        }
    }

    /// A started daemon answers `Health` with the protocol version and
    /// `Capabilities` with the conservative set.
    #[tokio::test]
    async fn a_started_daemon_answers_capabilities_with_a_non_empty_list() {
        let socket_path = temp_socket("started");
        let host = std::sync::Arc::new(cli_host(&socket_path));
        let shutdown = CancellationToken::new();
        let server_shutdown = shutdown.clone();
        let server =
            tokio::spawn(
                async move { eggsec_daemon::server::run_server(host, server_shutdown).await },
            );

        // Wait for the socket to accept a connection.
        let mut client = None;
        for _ in 0..200 {
            if let Ok(connected) = DaemonClient::connect(&socket_path).await {
                client = Some(connected);
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        let mut client = client.expect("the daemon must accept connections");

        match client.health().await.expect("the daemon answers Health") {
            ServerMessage::Health {
                protocol_version, ..
            } => {
                assert_eq!(
                    protocol_version,
                    eggsec_daemon_protocol::protocol::DAEMON_PROTOCOL_VERSION
                );
            }
            other => panic!("expected Health, got {other:?}"),
        }
        match client
            .capabilities()
            .await
            .expect("the daemon answers Capabilities")
        {
            ServerMessage::Capabilities { capabilities, .. } => {
                assert!(!capabilities.runtime.task_kinds.is_empty());
                let canonical = eggsec_runtime::RuntimeCapabilities::canonical();
                assert_eq!(
                    capabilities.runtime.task_kinds.len(),
                    canonical.task_kinds.len()
                );
            }
            other => panic!("expected Capabilities, got {other:?}"),
        }

        shutdown.cancel();
        match server.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("the server failed: {error}"),
            Err(error) => panic!("the server task failed: {error}"),
        }
    }

    /// A stale socket file does not block a start: the pre-flight reads it
    /// as "not live" and the server's own unlink clears it.
    #[tokio::test]
    async fn a_stale_socket_does_not_block_a_start() {
        let socket_path = temp_socket("stale");
        {
            // Binding and dropping leaves the socket file in place with
            // nothing listening on it.
            let listener =
                tokio::net::UnixListener::bind(&socket_path).expect("bind a stale socket");
            drop(listener);
        }
        assert!(
            std::path::Path::new(&socket_path).exists(),
            "the stale socket file must remain after the listener is dropped"
        );

        let config = eggsec_daemon::config::DaemonConfig {
            socket_path: socket_path.clone(),
            ..Default::default()
        };
        let shutdown = CancellationToken::new();
        let (outcome, server) = start_daemon(
            &socket_path,
            config,
            shutdown.clone(),
            Duration::from_secs(10),
        )
        .await;

        match &outcome {
            DaemonStartOutcome::Ready {
                socket_path: path, ..
            } => {
                assert_eq!(path, &socket_path);
            }
            other => panic!("a stale socket must not block the start, got {other:?}"),
        }
        assert!(
            start_success_message(&outcome, false).is_some(),
            "a ready start produces its success text"
        );

        // The fresh daemon answers on the path.
        let mut client = DaemonClient::connect(&socket_path)
            .await
            .expect("the fresh daemon must accept connections");
        assert!(matches!(
            client
                .health()
                .await
                .expect("the fresh daemon must answer Health"),
            ServerMessage::Health { .. }
        ));

        shutdown.cancel();
        if let Some(server) = server {
            server.stop().await;
        }
    }

    /// Cancelling the shutdown token during the readiness wait ends the
    /// attempt with the failed outcome.
    #[tokio::test]
    async fn the_readiness_wait_is_cancelled_by_the_shutdown_token() {
        let socket_path = temp_socket("cancelled");
        let shutdown = CancellationToken::new();
        shutdown.cancel();
        let (_keep_alive, mut result) = tokio::sync::oneshot::channel();

        let outcome = wait_for_daemon_ready(
            &socket_path,
            Duration::from_secs(10),
            &shutdown,
            &mut result,
        )
        .await;

        assert_eq!(outcome, Err(ReadyFailure::Cancelled));
    }

    /// The CLI's pre-flight probe is bounded: a daemon that accepts the
    /// connection but never answers must not hang the start path.
    #[tokio::test]
    async fn the_preflight_probe_is_bounded() {
        let socket_path = temp_socket("hung-preflight");
        let listener = tokio::net::UnixListener::bind(&socket_path).expect("bind a socket");
        // Accept the connection but never answer: the daemon looks alive on
        // the socket and is silent on the wire.
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                tokio::time::sleep(Duration::from_secs(30)).await;
                drop(stream);
            }
        });

        let started = std::time::Instant::now();
        let outcome =
            eggsec_daemon::host::probe_daemon_liveness(&socket_path, Duration::from_millis(250))
                .await;
        let elapsed = started.elapsed();

        assert_eq!(outcome, eggsec_daemon::host::DaemonLiveness::NotLive);
        assert!(
            elapsed < Duration::from_secs(5),
            "the pre-flight probe must honour its own bound, took {elapsed:?}"
        );
        // The bound the start path actually uses is a short one, not an
        // unbounded wait.
        assert!(
            eggsec_daemon::host::PREFLIGHT_PROBE_TIMEOUT <= Duration::from_secs(30),
            "the pre-flight probe's bound must be short"
        );
    }

    /// The readiness check reports a protocol-version mismatch rather than
    /// accepting it silently.
    #[tokio::test]
    async fn the_readiness_check_reports_a_protocol_version_mismatch() {
        let socket_path = temp_socket("foreign-version");
        let server = serve_a_foreign_protocol_version(&socket_path);

        let shutdown = CancellationToken::new();
        let (_keep_alive, mut result) = tokio::sync::oneshot::channel();
        let outcome = wait_for_daemon_ready(
            &socket_path,
            Duration::from_secs(10),
            &shutdown,
            &mut result,
        )
        .await;

        assert_eq!(
            outcome,
            Err(ReadyFailure::ProtocolMismatch {
                found: eggsec_daemon_protocol::protocol::DAEMON_PROTOCOL_VERSION + 1,
                expected: eggsec_daemon_protocol::protocol::DAEMON_PROTOCOL_VERSION,
            }),
            "a version mismatch must be reported, not silently accepted"
        );

        server.abort();
    }

    /// The socket is owner-only: a permissive umask must not leave it
    /// world-connectable.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_socket_is_created_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let socket_path = temp_socket("perms");
        let host = std::sync::Arc::new(cli_host(&socket_path));
        let shutdown = CancellationToken::new();
        let server_shutdown = shutdown.clone();
        let server =
            tokio::spawn(
                async move { eggsec_daemon::server::run_server(host, server_shutdown).await },
            );

        let mut mode = None;
        for _ in 0..200 {
            if let Ok(metadata) = std::fs::metadata(&socket_path) {
                mode = Some(metadata.permissions().mode() & 0o777);
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert_eq!(mode, Some(0o600), "the socket must be owner-only");

        shutdown.cancel();
        match server.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("the server failed: {error}"),
            Err(error) => panic!("the server task failed: {error}"),
        }
    }

    /// The pre-flight refusal names the path, the live daemon, and both
    /// remedies.
    #[test]
    fn the_preflight_refusal_is_actionable() {
        let outcome = DaemonStartOutcome::Refused {
            socket_path: "/run/eggsec.sock".to_string(),
            protocol_version: 2,
        };
        let message = start_failure_message(&outcome).expect("a refusal has a message");
        assert!(
            message.contains("/run/eggsec.sock"),
            "the refusal must name the socket path, got: {message}"
        );
        assert!(
            message.contains("live daemon"),
            "the refusal must say a live daemon holds the path, got: {message}"
        );
        assert!(
            message.contains("protocol version 2"),
            "the refusal must give the live daemon's protocol version, got: {message}"
        );
        assert!(
            message.contains("stop it first"),
            "the refusal must name the first remedy, got: {message}"
        );
        assert!(
            message.contains(eggsec_runtime::DAEMON_SOCKET_ENV_VAR),
            "the refusal must name the environment override, got: {message}"
        );
    }

    /// The server's own refusal survives the CLI's rendering.
    ///
    /// `describe_server_result` must not flatten the daemon's actionable
    /// refusal into something unactionable.
    #[test]
    fn the_servers_refusal_survives_the_rendering() {
        let cause =
            describe_server_result(Ok(Err(eggsec_daemon::error::DaemonError::SocketPathHeld {
                socket_path: "/run/eggsec.sock".to_string(),
                protocol_version: 2,
            })));
        assert!(
            cause.contains("/run/eggsec.sock"),
            "the rendered cause must name the socket path, got: {cause}"
        );
        assert!(
            cause.contains("live daemon"),
            "the rendered cause must say a live daemon holds the path, got: {cause}"
        );
        assert!(
            cause.contains("stop it first"),
            "the rendered cause must name a remedy, got: {cause}"
        );
        assert!(
            cause.contains(eggsec_runtime::DAEMON_SOCKET_ENV_VAR),
            "the rendered cause must name the environment override, got: {cause}"
        );
    }
}
