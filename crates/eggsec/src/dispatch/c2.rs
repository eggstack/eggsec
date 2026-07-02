use crate::dispatch::types::{send_progress, TaskResult};

pub async fn run_c2_task(
    target: String,
    campaign: String,
    dry_run: bool,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::c2::C2Scanner;

    send_progress(&progress_tx, 0, 4).await;

    let scanner = C2Scanner::new(dry_run, &campaign);

    send_progress(&progress_tx, 1, 4).await;

    let report =
        match tokio::time::timeout(std::time::Duration::from_secs(60), scanner.scan(&target)).await
        {
            Ok(Ok(report)) => report,
            Ok(Err(e)) => {
                tracing::warn!("C2 simulation error: {}", e);
                send_progress(&progress_tx, 4, 4).await;
                return Ok(TaskResult::Error(format!("C2 simulation failed: {}", e)));
            }
            Err(_) => {
                tracing::warn!("C2 simulation timed out");
                send_progress(&progress_tx, 4, 4).await;
                return Ok(TaskResult::Error("C2 simulation timed out".to_string()));
            }
        };

    send_progress(&progress_tx, 4, 4).await;
    Ok(TaskResult::C2(report))
}
