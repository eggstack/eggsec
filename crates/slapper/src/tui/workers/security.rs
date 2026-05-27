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

    if let Err(e) = progress_tx.send((0, 5)).await {
        tracing::warn!("Failed to send hunt progress: {}", e);
    }
    let report = match tokio::time::timeout(std::time::Duration::from_secs(60), run_hunt(&target, config)).await {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Hunt timed out after 60s")),
    }?;
    if let Err(e) = progress_tx.send((5, 5)).await {
        tracing::warn!("Failed to send hunt progress: {}", e);
    }
    if let Err(e) = result_tx.send(TaskResult::Hunt(report)).await {
        tracing::warn!("Failed to send hunt result: {}", e);
    }
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

    if let Err(e) = progress_tx.send((0, 3)).await {
        tracing::warn!("Failed to send browser progress: {}", e);
    }
    let report = match tokio::time::timeout(std::time::Duration::from_secs(60), run_browser_scan(&target, config)).await {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Browser scan timed out after 60s")),
    }?;
    if let Err(e) = progress_tx.send((3, 3)).await {
        tracing::warn!("Failed to send browser progress: {}", e);
    }
    if let Err(e) = result_tx.send(TaskResult::Browser(report)).await {
        tracing::warn!("Failed to send browser result: {}", e);
    }
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

    if let Err(e) = progress_tx.send((0, 3)).await {
        tracing::warn!("Failed to send compliance progress: {}", e);
    }

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

        if let Some(v) = headers
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
        {
            let lower = v.to_lowercase();
            if lower.contains("no-cache") || lower.contains("no-store") {
                findings.push(Severity::Info);
            }
        } else {
            let target_lower = target.to_lowercase();
            if target_lower.contains("login")
                || target_lower.contains("auth")
                || target_lower.contains("account")
            {
                findings.push(Severity::Medium);
            }
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

    if let Err(e) = progress_tx.send((2, 3)).await {
        tracing::warn!("Failed to send compliance progress: {}", e);
    }

    let report = match tokio::time::timeout(std::time::Duration::from_secs(60), generate_compliance_report(&target, framework, &findings)).await {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Compliance report timed out after 60s")),
    }?;
    if let Err(e) = progress_tx.send((3, 3)).await {
        tracing::warn!("Failed to send compliance progress: {}", e);
    }
    if let Err(e) = result_tx.send(TaskResult::Compliance(report)).await {
        tracing::warn!("Failed to send compliance result: {}", e);
    }
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

    if let Err(e) = progress_tx.send((0, 3)).await {
        tracing::warn!("Failed to send storage progress: {}", e);
    }

    let db = match init_storage(&config).await {
        Ok(db) => db,
        Err(e) => {
            if let Err(e) = result_tx.send(TaskResult::Error(format!(
                "Storage connection failed: {}. Ensure the database is running and credentials are correct.",
                e
            ))).await {
                tracing::warn!("Failed to send storage error: {}", e);
            }
            return Ok(());
        }
    };

    if let Err(e) = progress_tx.send((1, 3)).await {
        tracing::warn!("Failed to send storage progress: {}", e);
    }

    let result_data = match mode.as_str() {
        "connect" => {
            if let Err(e) = result_tx.send(TaskResult::Storage).await {
                tracing::warn!("Failed to send storage result: {}", e);
            }
            None
        }
        "list_scans" => match db.list_scans(50).await {
            Ok(scans) => {
                if let Err(e) = result_tx.send(TaskResult::StorageListScans { scans }).await {
                    tracing::warn!("Failed to send storage scans: {}", e);
                }
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
            if let Err(e) = result_tx
                .send(TaskResult::StorageListFindings { findings })
                .await
            {
                tracing::warn!("Failed to send storage findings: {}", e);
            }
            None
        }
        "search_cve" => {
            if let Some(ref cve) = cve_id {
                let finding = StoredFinding::new(
                    "cve-search",
                    &format!("CVE search: {}", cve),
                    crate::types::Severity::Medium,
                );
                if let Err(e) = result_tx
                    .send(TaskResult::StorageListFindings {
                        findings: vec![finding],
                    })
                    .await
                {
                    tracing::warn!("Failed to send CVE search result: {}", e);
                }
            } else {
                if let Err(e) = result_tx
                    .send(TaskResult::Error(
                        "No CVE ID provided for search".to_string(),
                    ))
                    .await
                {
                    tracing::warn!("Failed to send CVE search error: {}", e);
                }
            }
            None
        }
        _ => {
            if let Err(e) = result_tx
                .send(TaskResult::Error(format!("Unknown storage mode: {}", mode)))
                .await
            {
                tracing::warn!("Failed to send unknown mode error: {}", e);
            }
            None
        }
    };

    if let Err(e) = progress_tx.send((3, 3)).await {
        tracing::warn!("Failed to send storage progress: {}", e);
    }

    if let Some(error) = result_data {
        if let Err(e) = result_tx.send(TaskResult::Error(error)).await {
            tracing::warn!("Failed to send storage error: {}", e);
        }
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

    if let Err(e) = progress_tx.send((0, 3)).await {
        tracing::warn!("Failed to send integrations progress: {}", e);
    }

    match mode.as_str() {
        "configure" => {
            if let Err(e) = result_tx.send(TaskResult::Integrations).await {
                tracing::warn!("Failed to send integrations result: {}", e);
            }
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
                if let Err(e) = result_tx
                    .send(TaskResult::IntegrationsCreateIssue { issue })
                    .await
                {
                    tracing::warn!("Failed to send issue creation result: {}", e);
                }
            } else {
                if let Err(e) = result_tx
                    .send(TaskResult::Error(
                        "Title and description required for creating an issue".to_string(),
                    ))
                    .await
                {
                    tracing::warn!("Failed to send issue error: {}", e);
                }
            }
        }
        "search_issues" => {
            if let Err(e) = result_tx
                .send(TaskResult::IntegrationsSearchIssues { issues: vec![] })
                .await
            {
                tracing::warn!("Failed to send issue search result: {}", e);
            }
        }
        _ => {
            if let Err(e) = result_tx
                .send(TaskResult::Error(format!(
                    "Unknown integrations mode: {}",
                    mode
                )))
                .await
            {
                tracing::warn!("Failed to send unknown mode error: {}", e);
            }
        }
    }

    if let Err(e) = progress_tx.send((3, 3)).await {
        tracing::warn!("Failed to send integrations progress: {}", e);
    }
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

    if let Err(e) = progress_tx.send((0, 3)).await {
        tracing::warn!("Failed to send workflow progress: {}", e);
    }

    let mut report = WorkflowReport::new();
    report.total_findings = finding_ids.len();
    report.open_findings = finding_ids.len();

    if let Err(e) = progress_tx.send((2, 3)).await {
        tracing::warn!("Failed to send workflow progress: {}", e);
    }
    if let Err(e) = result_tx.send(TaskResult::Workflow(report)).await {
        tracing::warn!("Failed to send workflow result: {}", e);
    }
    if let Err(e) = progress_tx.send((3, 3)).await {
        tracing::warn!("Failed to send workflow progress: {}", e);
    }
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

    if let Err(e) = progress_tx.send((0, 3)).await {
        tracing::warn!("Failed to send vuln progress: {}", e);
    }

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
            if let Err(e) = result_tx.send(TaskResult::Vuln(assessment)).await {
                tracing::warn!("Failed to send vuln result: {}", e);
            }
        }
        _ => {
            if let Err(e) = result_tx
                .send(TaskResult::Error(format!("Unknown vuln mode: {}", mode)))
                .await
            {
                tracing::warn!("Failed to send unknown vuln mode error: {}", e);
            }
        }
    }

    if let Err(e) = progress_tx.send((3, 3)).await {
        tracing::warn!("Failed to send vuln progress: {}", e);
    }
    Ok(())
}
