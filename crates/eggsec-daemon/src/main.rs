use std::sync::Arc;

use anyhow::Result;
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

    /// Enable full task execution (requires `full-executor` feature).
    /// When enabled, the daemon dispatches real tasks through the Eggsec engine.
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
        #[cfg(feature = "full-executor")]
        {
            let executor = eggsec::runtime_bridge::EggsecRuntimeExecutor::new();
            tracing::info!("Full executor enabled — real task dispatch active");
            Arc::new(DaemonHost::new(config, executor, store))
        }
        #[cfg(not(feature = "full-executor"))]
        {
            tracing::warn!("--full-executor flag set but full-executor feature not enabled, using no-op executor");
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

/// Wait for SIGINT (Ctrl+C) or SIGTERM, whichever fires first.
#[cfg(unix)]
async fn wait_for_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to install SIGTERM handler");
            let _ = tokio::signal::ctrl_c().await;
            return;
        }
    };
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
}

#[cfg(not(unix))]
async fn wait_for_shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
