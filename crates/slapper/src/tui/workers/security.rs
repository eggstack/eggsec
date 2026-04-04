use crate::tui::workers::TaskResult;

pub async fn run_hunt_task(
    target: String,
    config: crate::hunt::HuntConfig,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::hunt::run_hunt;

    let _ = progress_tx.send((0, 5)).await;
    let report = run_hunt(&target, config).await?;
    let _ = progress_tx.send((5, 5)).await;
    let _ = result_tx.send(TaskResult::Hunt(report)).await;
    Ok(())
}

#[cfg(feature = "headless-browser")]
pub async fn run_browser_task(
    target: String,
    config: crate::browser::BrowserConfig,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::browser::run_browser_scan;

    let _ = progress_tx.send((0, 3)).await;
    let report = run_browser_scan(&target, config).await?;
    let _ = progress_tx.send((3, 3)).await;
    let _ = result_tx.send(TaskResult::Browser(report)).await;
    Ok(())
}

pub async fn run_compliance_task(
    target: String,
    framework: crate::compliance::ComplianceFramework,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::compliance::generate_compliance_report;
    use crate::types::Severity;

    let _ = progress_tx.send((0, 1)).await;
    let findings = vec![Severity::High, Severity::Medium, Severity::Low];
    let report = generate_compliance_report(&target, framework, &findings).await?;
    let _ = progress_tx.send((1, 1)).await;
    let _ = result_tx.send(TaskResult::Compliance(report)).await;
    Ok(())
}

pub async fn run_storage_task(
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let _ = result_tx.send(TaskResult::Storage).await;
    Ok(())
}

pub async fn run_integrations_task(
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let _ = result_tx.send(TaskResult::Integrations).await;
    Ok(())
}

pub async fn run_workflow_task(
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let _ = result_tx.send(TaskResult::Workflow).await;
    Ok(())
}

pub async fn run_vuln_task(
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let _ = result_tx.send(TaskResult::Vuln).await;
    Ok(())
}
