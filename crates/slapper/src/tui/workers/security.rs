#[cfg(any(feature = "advanced-hunting", feature = "compliance", feature = "database", feature = "external-integrations", feature = "finding-workflow", feature = "vuln-management"))]
use crate::tui::workers::TaskResult;

#[cfg(feature = "advanced-hunting")]
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

#[cfg(feature = "compliance")]
pub async fn run_compliance_task(
    target: String,
    framework: crate::compliance::ComplianceFramework,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::compliance::generate_compliance_report;
    use crate::types::Severity;

    let _ = progress_tx.send((0, 3)).await;

    let mut findings = Vec::new();

    if let Ok(resp) = reqwest::Client::new()
        .get(&target)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        let headers = resp.headers();
        let status = resp.status();

        if !target.starts_with("https://") {
            findings.push(Severity::High);
        }

        if !headers.contains_key("strict-transport-security") {
            findings.push(Severity::Medium);
        }

        if !headers.contains_key("x-content-type-options") {
            findings.push(Severity::Low);
        }

        if !headers.contains_key("x-frame-options")
            && !headers
                .get("content-security-policy")
                .map(|v| v.to_str().map(|s| s.contains("frame-ancestors")).unwrap_or(false))
                .unwrap_or(false)
        {
            findings.push(Severity::Medium);
        }

        if headers.contains_key("server") || headers.contains_key("x-powered-by") {
            findings.push(Severity::Low);
        }

        if status.is_server_error() {
            findings.push(Severity::High);
        }

        if status.is_client_error() && status.as_u16() != 404 {
            findings.push(Severity::Medium);
        }
    } else {
        findings.push(Severity::High);
    }

    if findings.is_empty() {
        findings.push(Severity::Info);
    }

    let _ = progress_tx.send((2, 3)).await;

    let report = generate_compliance_report(&target, framework, &findings).await?;
    let _ = progress_tx.send((3, 3)).await;
    let _ = result_tx.send(TaskResult::Compliance(report)).await;
    Ok(())
}

#[cfg(feature = "database")]
pub async fn run_storage_task(
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let _ = result_tx.send(TaskResult::Storage).await;
    Ok(())
}

#[cfg(feature = "external-integrations")]
pub async fn run_integrations_task(
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let _ = result_tx.send(TaskResult::Integrations).await;
    Ok(())
}

#[cfg(feature = "finding-workflow")]
pub async fn run_workflow_task(
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let _ = result_tx.send(TaskResult::Workflow).await;
    Ok(())
}

#[cfg(feature = "vuln-management")]
pub async fn run_vuln_task(
    _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    let _ = result_tx.send(TaskResult::Vuln).await;
    Ok(())
}
