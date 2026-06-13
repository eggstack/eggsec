use super::TaskResult;
use tracing;

pub async fn run_intercept_task(
    config: super::TaskConfig,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let (listen_addr, dry_run, max_flows, target) = match config {
        super::TaskConfig::Intercept {
            listen_addr,
            dry_run,
            max_flows,
            target,
        } => (listen_addr, dry_run, max_flows, target),
        _ => {
            tracing::error!("Intercept worker received wrong config variant");
            return Err(anyhow::anyhow!("Wrong config for intercept worker"));
        }
    };

    tracing::info!(
        "Intercept worker starting: listen_addr={}, dry_run={}, max_flows={}",
        listen_addr,
        dry_run,
        max_flows
    );

    let _ = progress_tx.send((0, 0)).await;

    let mut session = eggsec::proxy::intercept::types::InterceptSession::new(&listen_addr, dry_run);
    session.target = target;

    let _ = progress_tx.send((1, 1)).await;

    if let Err(e) = result_tx.send(TaskResult::Intercept(session)).await {
        tracing::error!("Failed to send intercept result: {}", e);
    }

    Ok(())
}
