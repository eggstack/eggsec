use std::sync::Arc;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use eggsec_daemon::config::DaemonConfig;
use eggsec_daemon::host::DaemonHost;
use eggsec_daemon::server::run_server;
use eggsec_daemon::store::{DaemonStore, NoopStore};

/// Eggsec daemon — local-only runtime host over Unix domain socket.
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let socket_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/eggsec-daemon.sock".into());

    let config = DaemonConfig {
        socket_path,
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

    // Use a no-op executor for now; real dispatch will be wired in a later phase.
    let host = Arc::new(DaemonHost::new(config, NoopExecutor, store));

    // Recover persisted sessions from a previous daemon instance
    if let Err(e) = host.recover_persisted_state().await {
        tracing::warn!(error = %e, "Failed to recover persisted state");
    }

    let shutdown = CancellationToken::new();

    // Handle SIGINT/SIGTERM for graceful shutdown
    {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Received shutdown signal");
            shutdown.cancel();
        });
    }

    run_server(host, shutdown).await?;

    tracing::info!("Daemon stopped.");
    Ok(())
}

/// Placeholder executor that rejects all tasks.
///
/// Real executor wiring (via `eggsec::dispatch::dispatch_inner`) will be
/// added in a later phase. For now this allows the daemon to start and
/// accept protocol commands (health, capabilities, session management)
/// without requiring the full `eggsec` crate dependency.
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
                    Output = Result<eggsec_runtime::TaskOutcome, eggsec_runtime::RuntimeError>,
                > + Send
                + 'static,
        >,
    > {
        Box::pin(async { Err(eggsec_runtime::RuntimeError::UnsupportedTaskKind) })
    }
}
