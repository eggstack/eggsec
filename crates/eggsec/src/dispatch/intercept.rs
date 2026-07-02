use crate::dispatch::types::{send_progress, send_result, TaskResult};

pub async fn run_intercept_task(
    listen_addr: String,
    dry_run: bool,
    max_flows: u64,
    target: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    tracing::info!(
        "Intercept worker starting: listen_addr={}, dry_run={}, max_flows={}",
        listen_addr,
        dry_run,
        max_flows
    );

    send_progress(&progress_tx, 0, 0).await;

    let mut session = crate::proxy::intercept::types::InterceptSession::new(&listen_addr, dry_run);
    session.target = target;

    send_progress(&progress_tx, 1, 1).await;
    send_result(&result_tx, TaskResult::Intercept(session)).await;

    Ok(())
}
