use crate::workers::TaskResult;

pub async fn run_c2_task(
    target: String,
    campaign: String,
    dry_run: bool,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use eggsec::c2::C2Scanner;

    if let Err(e) = progress_tx.send((0, 4)).await {
        tracing::warn!("Failed to send C2 progress: {}", e);
    }

    let scanner = C2Scanner::new(dry_run, &campaign);

    if let Err(e) = progress_tx.send((1, 4)).await {
        tracing::warn!("Failed to send C2 progress: {}", e);
    }

    let report = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        scanner.scan(&target),
    )
    .await
    {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => {
            tracing::warn!("C2 simulation error: {}", e);
            let _ = result_tx
                .send(TaskResult::Error(format!("C2 simulation failed: {}", e)))
                .await;
            return Ok(());
        }
        Err(_) => {
            let _ = result_tx
                .send(TaskResult::Error("C2 simulation timed out".to_string()))
                .await;
            return Ok(());
        }
    };

    if let Err(e) = progress_tx.send((4, 4)).await {
        tracing::warn!("Failed to send C2 progress: {}", e);
    }
    if let Err(e) = result_tx.send(TaskResult::C2(report)).await {
        tracing::warn!("Failed to send C2 result: {}", e);
    }
    Ok(())
}
