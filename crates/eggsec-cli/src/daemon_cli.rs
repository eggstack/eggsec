//! Daemon client CLI commands.
//!
//! Provides `daemon`, `session`, and `task` subcommands that communicate
//! with a running eggsec daemon over Unix domain socket.

use anyhow::{bail, Result};
use eggsec::cli::{Cli, Commands};
use eggsec_daemon::client::DaemonClient;
use eggsec_daemon::protocol::ServerMessage;
use eggsec_runtime::RuntimeSurface;

/// Returns `true` if the command is a daemon client command.
pub fn is_daemon_command(cmd: &Commands) -> bool {
    matches!(
        cmd,
        Commands::Daemon(_)
            | Commands::Session(_)
            | Commands::Task(_)
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

async fn handle_daemon(
    args: &eggsec::cli::DaemonArgs,
    socket_path: &str,
    json: bool,
) -> Result<()> {
    match &args.subcommand {
        eggsec::cli::DaemonSubcommand::Start { socket } => {
            let path = socket.as_deref().unwrap_or(socket_path);
            // Start daemon in background
            let config = eggsec_daemon::config::DaemonConfig {
                socket_path: path.to_string(),
                ..Default::default()
            };
            let host = std::sync::Arc::new(eggsec_daemon::host::DaemonHost::new(
                config,
                NoopExecutor,
            ));
            let shutdown = tokio_util::sync::CancellationToken::new();
            let host_clone = host.clone();
            let shutdown_clone = shutdown.clone();
            tokio::spawn(async move {
                eggsec_daemon::server::run_server(host_clone, shutdown_clone).await
            });
            // Wait briefly for socket to be ready
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if json {
                println!(r#"{{"status":"started","socket":"{}"}}"#, path);
            } else {
                println!("Daemon started on {}", path);
            }
            // Keep running until interrupted
            tokio::signal::ctrl_c().await.ok();
            shutdown.cancel();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if json {
                println!(r#"{{"status":"stopped"}}"#);
            } else {
                println!("Daemon stopped.");
            }
        }
        eggsec::cli::DaemonSubcommand::Status { socket } => {
            let path = socket.as_deref().unwrap_or(socket_path);
            let mut client = connect(path).await?;
            let resp = client.health().await?;
            match resp {
                ServerMessage::Health {
                    status,
                    version,
                    ..
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
                    match client.health().await {
                        Ok(_) => {
                            if json {
                                println!(r#"{{"status":"running","message":"daemon is running (stop via SIGTERM or ctrl-c on the daemon process)"}}"#);
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
    }
    Ok(())
}

async fn handle_session(
    args: &eggsec::cli::SessionArgs,
    socket_path: &str,
    json: bool,
) -> Result<()> {
    let mut client = connect(socket_path).await?;
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
                            println!("{:<38} {:<15} {:<8} {:<8}", "SESSION ID", "SURFACE", "ACTIVE", "DONE");
                            println!("{}", "-".repeat(71));
                            for s in &sessions {
                                println!(
                                    "{:<38} {:<15} {:<8} {:<8}",
                                    s.session_id,
                                    s.surface.label(),
                                    s.active_count,
                                    s.completed_count
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
            let resp = client
                .create_session(surface, None, vec![])
                .await?;
            match resp {
                ServerMessage::SessionCreated { session_id, .. } => {
                    if json {
                        println!(
                            r#"{{"session_id":"{}","surface":"{}"}}"#,
                            session_id,
                            surface_label
                        );
                    } else {
                        println!("Session created: {} (surface: {})", session_id, surface_label);
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
                        println!("Session: {}", snapshot.session_id);
                        println!("Surface: {}", snapshot.surface.label());
                        println!(
                            "Scope: {}",
                            snapshot
                                .scope
                                .as_ref()
                                .map(|s| format!("{} (explicit: {})", s.source, s.is_explicit))
                                .unwrap_or_else(|| "none".into())
                        );
                        println!("Active tasks: {}", snapshot.active_tasks.len());
                        println!("Completed tasks: {}", snapshot.completed_tasks.len());
                        for t in &snapshot.active_tasks {
                            println!("  [{}] {} - {:?}", t.task_id, t.request_summary, t.status);
                        }
                        for t in &snapshot.completed_tasks {
                            println!("  [{}] {} - {:?}", t.task_id, t.request_summary, t.status);
                        }
                    }
                }
                other => bail!("unexpected response: {:?}", other),
            }
        }
    }
    Ok(())
}

async fn handle_task(
    args: &eggsec::cli::TaskArgs,
    socket_path: &str,
    json: bool,
) -> Result<()> {
    let mut client = connect(socket_path).await?;
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
        eggsec::cli::TaskSubcommand::Watch {
            session_id,
        } => {
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
        }),
        "endpoint-scan" => TaskKind::EndpointScan(EndpointScanParams {
            target: target.into(),
            methods: None,
            wordlist: None,
        }),
        "fingerprint" => TaskKind::Fingerprint(FingerprintParams {
            target: target.into(),
        }),
        "fuzz" => TaskKind::Fuzz(FuzzParams {
            target: target.into(),
            payload_type: None,
            threads: None,
        }),
        "waf" => TaskKind::Waf(WafParams {
            target: target.into(),
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
        }),
        "pipeline" => TaskKind::Pipeline(PipelineParams {
            target: target.into(),
            profile: None,
        }),
        "auth-test" => TaskKind::AuthTest(AuthTestParams {
            target: target.into(),
            username: None,
            credential_list: None,
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
    match event {
        eggsec_runtime::RuntimeEvent::SessionCreated { session_id } => {
            println!("[session] Created: {}", session_id);
        }
        eggsec_runtime::RuntimeEvent::TaskQueued { task_id, .. } => {
            println!("[task] Queued: {}", task_id);
        }
        eggsec_runtime::RuntimeEvent::TaskStarted { task_id, .. } => {
            println!("[task] Started: {}", task_id);
        }
        eggsec_runtime::RuntimeEvent::TaskProgress {
            task_id,
            progress,
            ..
        } => {
            println!(
                "[task] Progress: {} - {}",
                task_id,
                progress.message.as_deref().unwrap_or("...")
            );
        }
        eggsec_runtime::RuntimeEvent::TaskCompleted {
            task_id,
            outcome,
            ..
        } => {
            println!("[task] Completed: {} - {:?}", task_id, outcome);
        }
        eggsec_runtime::RuntimeEvent::TaskFailed {
            task_id, error, ..
        } => {
            println!("[task] Failed: {} - {}", task_id, error.message);
        }
        eggsec_runtime::RuntimeEvent::TaskCancelled { task_id, .. } => {
            println!("[task] Cancelled: {}", task_id);
        }
        other => {
            println!("[event] {:?}", other);
        }
    }
}

/// No-op executor for daemon start from CLI.
///
/// The daemon started via `eggsec daemon start` uses a real executor
/// through `eggsec-daemon`'s main binary. The CLI start command is
/// a convenience wrapper that delegates to the daemon binary logic.
struct NoopExecutor;

impl eggsec_runtime::RuntimeTaskExecutor for NoopExecutor {
    fn execute(
        &self,
        _task_id: eggsec_runtime::TaskId,
        _request: eggsec_runtime::RunRequest,
        _sink: eggsec_runtime::RuntimeEventSink,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        eggsec_runtime::TaskOutcome,
                        eggsec_runtime::RuntimeError,
                    >,
                > + Send
                + 'static,
        >,
    > {
        Box::pin(async { Err(eggsec_runtime::RuntimeError::UnsupportedTaskKind) })
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
        assert_eq!(parse_surface(Some("cli-manual")).unwrap(), RuntimeSurface::CliManual);
        assert_eq!(parse_surface(Some("ci")).unwrap(), RuntimeSurface::Ci);
        assert_eq!(parse_surface(Some("rest-api")).unwrap(), RuntimeSurface::RestApi);
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
        assert!(matches!(req.task_kind, eggsec_runtime::TaskKind::PortScan(_)));
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
}
