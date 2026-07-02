use crate::dispatch::types::{send_progress, send_result, TaskResult};

pub async fn run_c2_task(
    target: String,
    campaign: String,
    dry_run: bool,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
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
                send_result(
                    &result_tx,
                    TaskResult::Error(format!("C2 simulation failed: {}", e)),
                )
                .await;
                return Ok(());
            }
            Err(_) => {
                send_result(
                    &result_tx,
                    TaskResult::Error("C2 simulation timed out".to_string()),
                )
                .await;
                return Ok(());
            }
        };

    send_progress(&progress_tx, 4, 4).await;
    send_result(&result_tx, TaskResult::C2(report)).await;
    Ok(())
}
