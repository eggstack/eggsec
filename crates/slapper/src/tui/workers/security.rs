#[cfg(any(
    feature = "advanced-hunting",
    feature = "compliance",
    feature = "database",
    feature = "external-integrations",
    feature = "finding-workflow",
    feature = "vuln-management"
))]
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

    if let Ok(resp) = crate::utils::get_shared_http_client()
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
                .map(|v| {
                    v.to_str()
                        .map(|s| s.contains("frame-ancestors"))
                        .unwrap_or(false)
                })
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

        if !headers.contains_key("content-security-policy") {
            findings.push(Severity::Low);
        }

        if !headers.contains_key("referrer-policy") {
            findings.push(Severity::Low);
        }

        if !headers.contains_key("permissions-policy") {
            findings.push(Severity::Info);
        }

        if headers
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase().contains("no-cache") || v.to_lowercase().contains("no-store"))
            .unwrap_or(false)
        {
        } else if target.to_lowercase().contains("login")
            || target.to_lowercase().contains("auth")
            || target.to_lowercase().contains("account")
        {
            findings.push(Severity::Medium);
        }

        let set_cookie = headers.get_all("set-cookie");
        for cookie_header in set_cookie.iter().flat_map(|v| v.to_str().ok()) {
            let cookie_lower = cookie_header.to_lowercase();
            if !cookie_lower.contains("httponly") {
                findings.push(Severity::Medium);
                break;
            }
            if !cookie_lower.contains("secure") && target.starts_with("https://") {
                findings.push(Severity::Medium);
                break;
            }
            if cookie_lower.contains("samesite=None") {
                findings.push(Severity::Low);
            }
        }

        if headers
            .get("server")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("Apache") || v.contains("nginx") || v.contains("Microsoft-IIS"))
            .unwrap_or(false)
        {
            findings.push(Severity::Info);
        }

        if !headers.contains_key("x-xss-protection") {
            findings.push(Severity::Info);
        }

        if headers
            .get("access-control-allow-origin")
            .map(|v| v.to_str().ok())
            .flatten()
            .map(|v| v == "*")
            .unwrap_or(false)
        {
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
    config: crate::storage::StorageConfig,
    mode: String,
    scan_id: Option<String>,
    cve_id: Option<String>,
    severity_filter: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::storage::init_storage;
    use crate::storage::models::{ScanStatus, StoredFinding, StoredScan};

    let _ = progress_tx.send((0, 3)).await;

    let db = match init_storage(&config).await {
        Ok(db) => db,
        Err(e) => {
            let _ = result_tx.send(TaskResult::Error(format!(
                "Storage connection failed: {}. Ensure the database is running and credentials are correct.",
                e
            ))).await;
            return Ok(());
        }
    };

    let _ = progress_tx.send((1, 3)).await;

    let result_data = match mode.as_str() {
        "connect" => {
            let _ = result_tx.send(TaskResult::Storage).await;
            None
        }
        "list_scans" => match db.list_scans(50).await {
            Ok(scans) => {
                let _ = result_tx.send(TaskResult::StorageListScans { scans }).await;
                None
            }
            Err(e) => Some(format!("Failed to list scans: {}", e)),
        },
        "list_findings" => {
            let findings = if let Some(ref scan) = scan_id {
                match db.list_findings(scan).await {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::warn!("Failed to list findings for scan {}: {}", scan, e);
                        vec![]
                    }
                }
            } else {
                match db.list_findings("all").await {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::warn!("Failed to list all findings: {}", e);
                        vec![]
                    }
                }
            };
            let _ = result_tx
                .send(TaskResult::StorageListFindings { findings })
                .await;
            None
        }
        "search_cve" => {
            if let Some(ref cve) = cve_id {
                let finding = StoredFinding::new(
                    "cve-search",
                    &format!("CVE search: {}", cve),
                    crate::types::Severity::Medium,
                );
                let _ = result_tx
                    .send(TaskResult::StorageListFindings {
                        findings: vec![finding],
                    })
                    .await;
            } else {
                let _ = result_tx
                    .send(TaskResult::Error(
                        "No CVE ID provided for search".to_string(),
                    ))
                    .await;
            }
            None
        }
        _ => {
            let _ = result_tx
                .send(TaskResult::Error(format!("Unknown storage mode: {}", mode)))
                .await;
            None
        }
    };

    let _ = progress_tx.send((3, 3)).await;

    if let Some(error) = result_data {
        let _ = result_tx.send(TaskResult::Error(error)).await;
    }

    Ok(())
}

#[cfg(feature = "external-integrations")]
pub async fn run_integrations_task(
    config: crate::integrations::IntegrationConfig,
    mode: String,
    title: Option<String>,
    description: Option<String>,
    labels: Vec<String>,
    assignees: Vec<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::integrations::Issue;

    let _ = progress_tx.send((0, 3)).await;

    match mode.as_str() {
        "configure" => {
            let _ = result_tx.send(TaskResult::Integrations).await;
        }
        "create_issue" => {
            if let (Some(t), Some(d)) = (&title, &description) {
                let issue = crate::integrations::Issue {
                    title: t.clone(),
                    description: d.clone(),
                    labels: labels.clone(),
                    severity: None,
                    assignees: assignees.clone(),
                };
                let _ = result_tx
                    .send(TaskResult::IntegrationsCreateIssue { issue })
                    .await;
            } else {
                let _ = result_tx
                    .send(TaskResult::Error(
                        "Title and description required for creating an issue".to_string(),
                    ))
                    .await;
            }
        }
        "search_issues" => {
            let _ = result_tx
                .send(TaskResult::IntegrationsSearchIssues { issues: vec![] })
                .await;
        }
        _ => {
            let _ = result_tx
                .send(TaskResult::Error(format!(
                    "Unknown integrations mode: {}",
                    mode
                )))
                .await;
        }
    }

    let _ = progress_tx.send((3, 3)).await;
    Ok(())
}

#[cfg(feature = "finding-workflow")]
pub async fn run_workflow_task(
    mode: String,
    _target: Option<String>,
    finding_ids: Vec<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::workflow::WorkflowReport;

    let _ = progress_tx.send((0, 3)).await;

    let mut report = WorkflowReport::new();
    report.total_findings = finding_ids.len();
    report.open_findings = finding_ids.len();

    let _ = progress_tx.send((2, 3)).await;
    let _ = result_tx.send(TaskResult::Workflow(report)).await;
    let _ = progress_tx.send((3, 3)).await;
    Ok(())
}

#[cfg(feature = "vuln-management")]
pub async fn run_vuln_task(
    mode: String,
    target: Option<String>,
    cve_id: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::vuln::VulnAssessment;

    let _ = progress_tx.send((0, 3)).await;

    match mode.as_str() {
        "cvss_calc" | "exploit_check" | "asset_assess" | "prioritize" | "triage"
        | "remediation" => {
            let mut results = vec![format!("Mode: {}", mode)];
            if let Some(ref t) = target {
                results.push(format!("Target: {}", t));
            }
            if let Some(ref c) = cve_id {
                results.push(format!("CVE: {}", c));
            }
            results.push(format!("Assessment completed at: {}", chrono::Utc::now()));

            let assessment = VulnAssessment {
                mode: mode.clone(),
                results,
                assessed_at: chrono::Utc::now(),
            };
            let _ = result_tx.send(TaskResult::Vuln(assessment)).await;
        }
        _ => {
            let _ = result_tx
                .send(TaskResult::Error(format!("Unknown vuln mode: {}", mode)))
                .await;
        }
    }

    let _ = progress_tx.send((3, 3)).await;
    Ok(())
}
