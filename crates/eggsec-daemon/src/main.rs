use std::sync::Arc;

use anyhow::{bail, Result};
use clap::Parser;
use tokio_util::sync::CancellationToken;

use eggsec_daemon::config::DaemonConfig;
use eggsec_daemon::host::DaemonHost;
use eggsec_daemon::server::run_server;
use eggsec_daemon::store::{DaemonStore, NoopStore};
use eggsec_runtime::RuntimeEvent;

#[derive(Parser)]
#[command(
    name = "eggsec-daemon",
    about = "Eggsec daemon — local-only runtime host over Unix domain socket"
)]
struct DaemonArgs {
    /// Unix socket path for client connections.
    #[arg(short, long, default_value = "/tmp/eggsec-daemon.sock")]
    socket_path: String,

    /// Maximum number of concurrent client connections.
    #[arg(short = 'm', long, default_value_t = 10)]
    max_clients: usize,

    /// Directory for persistent state (sessions, audit log).
    /// Defaults to ~/.local/share/eggsec/daemon/ if not set.
    #[arg(short, long)]
    data_dir: Option<String>,

    /// Disable persistence (no-op store regardless of data-dir).
    #[arg(long)]
    no_persistence: bool,

    /// Log level filter (trace, debug, info, warn, error).
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Enable full task execution through the Eggsec engine.
    /// The binary must be BUILT with the `service` cargo feature (or its
    /// deprecated `full-executor` alias) for this flag to have any effect;
    /// a binary built without it accepts the flag and falls back to a
    /// no-op executor with a warning.
    #[arg(long)]
    full_executor: bool,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = DaemonArgs::parse();

    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| args.log_level.parse().unwrap_or_else(|_| "info".into())),
        )
        .init();

    let config = DaemonConfig {
        socket_path: args.socket_path,
        max_clients: args.max_clients,
        data_dir: args.data_dir,
        enable_persistence: !args.no_persistence,
        ..Default::default()
    };

    tracing::info!("Starting eggsec daemon on {}", config.socket_path);

    // Refuse to displace a live daemon before touching the store or the
    // socket. `run_server` unlinks the socket path before binding, so a
    // second daemon that started here would silently steal the first
    // daemon's socket out from under its clients.
    refuse_live_daemon(&config.socket_path).await?;

    // Set up persistence store if enabled
    let store: Arc<dyn DaemonStore> = if config.enable_persistence {
        let data_dir = config
            .data_dir
            .as_ref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                std::path::PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("eggsec")
                    .join("daemon")
            });
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            tracing::warn!(error = %e, path = %data_dir.display(), "Failed to create data directory, persistence disabled");
            Arc::new(NoopStore)
        } else {
            let db_path = data_dir.join("eggsec-daemon.sqlite");
            match eggsec_daemon::store::sqlite::SqliteStore::new(&db_path) {
                Ok(s) => {
                    tracing::info!(path = %db_path.display(), "Persistence store opened");
                    Arc::new(s)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to open persistence store, using no-op store");
                    Arc::new(NoopStore)
                }
            }
        }
    } else {
        Arc::new(NoopStore)
    };

    // Create the host, optionally with full executor.
    let host = if args.full_executor {
        #[cfg(feature = "service")]
        {
            let executor = eggsec::runtime_bridge::EggsecRuntimeExecutor::new();
            tracing::info!("Full executor enabled — real task dispatch active");
            Arc::new(DaemonHost::new(config, executor, store))
        }
        #[cfg(not(feature = "service"))]
        {
            tracing::warn!(
                "--full-executor flag set but this binary was built WITHOUT the `service` cargo \
                 feature, so no engine executor is linked in — falling back to the no-op \
                 executor. Build a daemon that can execute tasks with: \
                 cargo install --path crates/eggsec-daemon --features service --locked"
            );
            Arc::new(DaemonHost::new_noop(config, store))
        }
    } else {
        Arc::new(DaemonHost::new_noop(config, store))
    };

    // Recover persisted sessions from a previous daemon instance
    if let Err(e) = host.recover_persisted_state().await {
        tracing::warn!(error = %e, "Failed to recover persisted state");
    }

    let shutdown = CancellationToken::new();

    // Handle SIGINT and SIGTERM for graceful shutdown
    {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            wait_for_shutdown_signal().await;
            tracing::info!("Received shutdown signal");
            shutdown.cancel();
        });
    }

    // Spawn background task to persist terminal runtime events.
    // Also runs a periodic snapshot sweep as a safety net against broadcast
    // channel overflow (RecvError::Lagged) which could cause missed events.
    {
        let host = host.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            let mut receiver = host.runtime().subscribe().await;
            let mut sweep_interval = tokio::time::interval(std::time::Duration::from_secs(5));
            sweep_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        tracing::debug!("Event persistence task shutting down");
                        break;
                    }
                    _ = sweep_interval.tick() => {
                        // Periodic sweep: persist snapshots for all non-closed sessions
                        // with active tasks. This catches any terminal events missed by
                        // broadcast channel overflow.
                        let sessions = host.runtime().list_sessions().await;
                        for summary in &sessions {
                            if summary.active_count > 0 {
                                if let Ok(snapshot) = host.runtime().snapshot(summary.session_id).await {
                                    if let Err(e) = host.store().save_session_snapshot(&snapshot).await {
                                        tracing::warn!(error = %e, session_id = %summary.session_id, "Failed to persist snapshot during sweep");
                                    }
                                }
                            }
                        }
                    }
                    event = receiver.recv() => {
                        let event = match event {
                            Some(e) => e,
                            None => {
                                tracing::debug!("Event channel closed, stopping persistence task");
                                break;
                            }
                        };
                        let session_id = match &event {
                            RuntimeEvent::TaskCompleted { session_id, .. } => *session_id,
                            RuntimeEvent::TaskFailed { session_id, .. } => *session_id,
                            RuntimeEvent::TaskCancelled { session_id, .. } => *session_id,
                            _ => continue,
                        };
                        match host.runtime().snapshot(session_id).await {
                            Ok(snapshot) => {
                                if let Err(e) = host.store().save_session_snapshot(&snapshot).await {
                                    tracing::warn!(error = %e, %session_id, "Failed to persist snapshot on terminal event");
                                } else {
                                    tracing::debug!(%session_id, "Persisted snapshot after terminal event");
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, %session_id, "Failed to snapshot on terminal event");
                            }
                        }
                    }
                }
            }
        });
    }

    run_server(host, shutdown).await?;

    tracing::info!("Daemon stopped.");
    Ok(())
}

/// Refuse to displace a live daemon that already serves `socket_path`.
///
/// The check is a connection plus a `Health` exchange, never a `stat`: a
/// socket file left behind by a crashed daemon is indistinguishable from a
/// live one by file existence, and only a connection tells them apart. A
/// stale file is therefore *not* a refusal — the server's own unlink removes
/// it before binding.
async fn refuse_live_daemon(socket_path: &str) -> Result<()> {
    match eggsec_daemon::host::probe_daemon_liveness(
        socket_path,
        eggsec_daemon::host::PREFLIGHT_PROBE_TIMEOUT,
    )
    .await
    {
        eggsec_daemon::host::DaemonLiveness::Live { protocol_version } => {
            bail!(
                "refusing to displace the live daemon already serving {socket_path} \
                 (protocol version {protocol_version}); stop it first"
            )
        }
        eggsec_daemon::host::DaemonLiveness::NotLive => Ok(()),
    }
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM, whichever fires first.
#[cfg(unix)]
async fn wait_for_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to install SIGTERM handler");
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!(error = %e, "Failed to install Ctrl-C handler");
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

#[cfg(not(unix))]
async fn wait_for_shutdown_signal() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %e, "Ctrl-C handler failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_daemon::client::DaemonClient;
    use eggsec_daemon::host::{DaemonLiveness, PREFLIGHT_PROBE_TIMEOUT};
    use eggsec_daemon_protocol::protocol::ServerMessage;

    /// A unique socket path under the system temp directory.
    ///
    /// The path stays short on purpose: a Unix socket path is bounded by
    /// `sun_path`.
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

    /// The daemon binary's start path refuses to displace a live daemon, and
    /// the live daemon keeps its socket.
    #[tokio::test]
    async fn the_daemon_binary_refuses_to_displace_a_live_daemon() {
        let socket_path = temp_socket("bin-live");
        let host = std::sync::Arc::new(DaemonHost::new_noop(
            DaemonConfig {
                socket_path: socket_path.clone(),
                ..Default::default()
            },
            std::sync::Arc::new(eggsec_daemon::store::NoopStore),
        ));
        let shutdown = CancellationToken::new();
        let server = {
            let host = host.clone();
            let shutdown = shutdown.clone();
            tokio::spawn(async move { run_server(host, shutdown).await })
        };

        // Wait until the daemon answers on the path.
        let mut live = false;
        for _ in 0..200 {
            if matches!(
                eggsec_daemon::host::probe_daemon_liveness(&socket_path, PREFLIGHT_PROBE_TIMEOUT)
                    .await,
                DaemonLiveness::Live { .. }
            ) {
                live = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(live, "the daemon never came up on {socket_path}");

        let refusal = refuse_live_daemon(&socket_path).await;
        let error = refusal.expect_err("the start path must refuse a live daemon");
        let message = error.to_string();
        assert!(
            message.contains(&socket_path),
            "the refusal must name the socket path, got: {message}"
        );

        // The live daemon still owns the socket: it was not stolen.
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
}
