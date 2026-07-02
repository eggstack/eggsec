use crate::dispatch::types::{send_progress, send_result, TaskResult};

#[cfg(any(
    feature = "advanced-hunting",
    feature = "compliance",
    feature = "database",
    feature = "external-integrations",
    feature = "finding-workflow",
    feature = "vuln-management",
    feature = "headless-browser",
    feature = "wireless"
))]
#[cfg(feature = "advanced-hunting")]
pub async fn run_hunt_task(
    target: String,
    config: crate::hunt::HuntConfig,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::hunt::run_hunt;

    send_progress(&progress_tx, 0, 5).await;
    let report = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        run_hunt(&target, config),
    )
    .await
    {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Hunt timed out after 60s")),
    };
    send_progress(&progress_tx, 5, 5).await;
    send_result(&result_tx, TaskResult::Hunt(report)).await;
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

    send_progress(&progress_tx, 0, 3).await;
    let report = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        run_browser_scan(&target, config),
    )
    .await
    {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Browser scan timed out after 60s")),
    };
    send_progress(&progress_tx, 3, 3).await;
    send_result(&result_tx, TaskResult::Browser(report)).await;
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

    send_progress(&progress_tx, 0, 3).await;

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

        if let Some(v) = headers.get("cache-control").and_then(|v| v.to_str().ok()) {
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
        tracing::debug!("Compliance preflight request to {} failed", target);
        findings.push(Severity::High);
    }

    if findings.is_empty() {
        findings.push(Severity::Info);
    }

    send_progress(&progress_tx, 2, 3).await;

    let report = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        generate_compliance_report(&target, framework, &findings),
    )
    .await
    {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Compliance report timed out after 60s")),
    };
    send_progress(&progress_tx, 3, 3).await;
    send_result(&result_tx, TaskResult::Compliance(report)).await;
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
    use crate::findings::lifecycle::StoredFinding;
    use crate::storage::init_storage;
    use crate::storage::models::{ScanStatus, StoredScan};

    let result_tx_timeout = result_tx.clone();
    match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        async move {
    send_progress(&progress_tx, 0, 3).await;

    let db = match init_storage(&config).await {
        Ok(db) => db,
        Err(e) => {
            send_result(&result_tx, TaskResult::Error(format!(
                "Storage connection failed: {}. Ensure the database is running and credentials are correct.",
                e
            ))).await;
            return Ok(());
        }
    };

    send_progress(&progress_tx, 1, 3).await;

    let result_data = match mode.as_str() {
        "connect" => {
            send_result(&result_tx, TaskResult::Storage).await;
            None
        }
        "list_scans" => match db.list_scans(50).await {
            Ok(scans) => {
                send_result(&result_tx, TaskResult::StorageListScans { scans }).await;
                None
            }
            Err(e) => Some(format!("Failed to list scans: {}", e)),
        },
        "list_findings" => {
            let findings = if let Some(ref scan) = scan_id {
                match db.list_findings(scan, 0, 1000).await {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::warn!("Failed to list findings for scan {}: {}", scan, e);
                        vec![]
                    }
                }
            } else {
                match db.list_all_findings(0, 1000).await {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::warn!("Failed to list all findings: {}", e);
                        vec![]
                    }
                }
            };
            send_result(&result_tx, TaskResult::StorageListFindings { findings }).await;
            None
        }
        "search_cve" => {
            if let Some(ref cve) = cve_id {
                let finding = crate::findings::Finding {
                    id: uuid::Uuid::new_v4().to_string(),
                    fingerprint: String::new(),
                    title: format!("CVE search: {}", cve),
                    description: format!("Search results for {}", cve),
                    severity: crate::types::Severity::Medium,
                    confidence: crate::findings::Confidence::Informational,
                    finding_type: crate::findings::FindingType::ScanResult,
                    cwe: None,
                    owasp: None,
                    cve: Some(cve.clone()),
                    affected_asset: crate::findings::AffectedAsset {
                        asset_type: "cve_search".to_string(),
                        identifier: cve.clone(),
                        host: None,
                        port: None,
                        protocol: None,
                    },
                    location: crate::findings::FindingLocation::default(),
                    evidence: vec![],
                    reproduction: None,
                    remediation: None,
                    discovered_at: chrono::Utc::now(),
                    source: crate::findings::FindingSource {
                        tool: "eggsec".to_string(),
                        module: "storage".to_string(),
                        run_id: None,
                    },
                    tags: vec![],
                    metadata: serde_json::Value::Null,
                };
                let stored = StoredFinding::new(finding, "");
                send_result(
                    &result_tx,
                    TaskResult::StorageListFindings {
                        findings: vec![stored],
                    },
                )
                .await;
            } else {
                send_result(
                    &result_tx,
                    TaskResult::Error("No CVE ID provided for search".to_string()),
                )
                .await;
            }
            None
        }
        _ => {
            send_result(
                &result_tx,
                TaskResult::Error(format!("Unknown storage mode: {}", mode)),
            )
            .await;
            None
        }
    };

    send_progress(&progress_tx, 3, 3).await;

    if let Some(error) = result_data {
        send_result(&result_tx, TaskResult::Error(error)).await;
    }

    Ok(())
        },
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Storage task timed out after 60s");
            send_result(&result_tx_timeout, TaskResult::Error("Storage task timed out".to_string())).await;
            Ok(())
        }
    }
}

#[cfg(feature = "external-integrations")]
pub async fn run_integrations_task(
    config: crate::integrations::IntegrationConfig,
    mode: String,
    title: Option<String>,
    description: Option<String>,
    labels: Vec<String>,
    assignees: Vec<String>,
    search_query: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::integrations::{Issue, IssueTracker};

    let result_tx_timeout = result_tx.clone();
    match tokio::time::timeout(std::time::Duration::from_secs(60), async move {
        send_progress(&progress_tx, 0, 3).await;

        let tracker: Option<Box<dyn IssueTracker>> = if let Some(jira_config) = config.jira {
            Some(Box::new(crate::integrations::jira::JiraClient::new(
                jira_config,
            )))
        } else if let Some(github_config) = config.github {
            Some(Box::new(crate::integrations::github::GitHubClient::new(
                github_config,
            )))
        } else if let Some(gitlab_config) = config.gitlab {
            Some(Box::new(crate::integrations::gitlab::GitLabClient::new(
                gitlab_config,
            )))
        } else {
            None
        };

        let tracker = match tracker {
            Some(t) => t,
            None => {
                send_result(
                    &result_tx,
                    TaskResult::Error(
                        "No tracker configured. Set Jira, GitHub, or GitLab credentials."
                            .to_string(),
                    ),
                )
                .await;
                return Ok(());
            }
        };

        send_progress(&progress_tx, 1, 3).await;

        match mode.as_str() {
            "configure" => {
                send_result(&result_tx, TaskResult::Integrations).await;
            }
            "create_issue" => match (&title, &description) {
                (Some(t), Some(d)) if !t.is_empty() && !d.is_empty() => {
                    let issue = Issue {
                        id: None,
                        title: t.clone(),
                        description: d.clone(),
                        labels: labels.clone(),
                        severity: None,
                        assignees: assignees.clone(),
                        status: None,
                        url: None,
                        created_at: None,
                    };
                    match tracker.create_issue(&issue).await {
                        Ok(id) => {
                            let created = Issue {
                                id: Some(id.clone()),
                                ..issue
                            };
                            send_result(
                                &result_tx,
                                TaskResult::IntegrationsCreateIssue { issue: created },
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to create issue: {}", e);
                            send_result(
                                &result_tx,
                                TaskResult::Error(format!("Failed to create issue: {}", e)),
                            )
                            .await;
                        }
                    }
                }
                _ => {
                    send_result(
                        &result_tx,
                        TaskResult::Error(
                            "Title and description required for creating an issue".to_string(),
                        ),
                    )
                    .await;
                }
            },
            "search_issues" => {
                let query = search_query.as_deref().unwrap_or("");
                if query.is_empty() {
                    send_result(
                        &result_tx,
                        TaskResult::Error(
                            "Search query required (enter in Search Query field)".to_string(),
                        ),
                    )
                    .await;
                } else {
                    match tracker.search_issues(query).await {
                        Ok(issues) => {
                            send_result(
                                &result_tx,
                                TaskResult::IntegrationsSearchIssues { issues },
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to search issues: {}", e);
                            send_result(
                                &result_tx,
                                TaskResult::Error(format!("Failed to search issues: {}", e)),
                            )
                            .await;
                        }
                    }
                }
            }
            _ => {
                send_result(
                    &result_tx,
                    TaskResult::Error(format!("Unknown integrations mode: {}", mode)),
                )
                .await;
            }
        }

        send_progress(&progress_tx, 3, 3).await;
        Ok(())
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Integrations task timed out after 60s");
            send_result(
                &result_tx_timeout,
                TaskResult::Error("Integrations task timed out".to_string()),
            )
            .await;
            Ok(())
        }
    }
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

    send_progress(&progress_tx, 0, 3).await;

    let mut report = WorkflowReport::new();
    report.total_findings = finding_ids.len();
    report.open_findings = finding_ids.len();

    send_progress(&progress_tx, 2, 3).await;
    send_result(&result_tx, TaskResult::Workflow(report)).await;
    send_progress(&progress_tx, 3, 3).await;
    Ok(())
}

#[cfg(feature = "vuln-management")]
pub async fn run_vuln_task(
    mode: String,
    target: Option<String>,
    cve_id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    cvss_vector: Option<String>,
    asset_type: Option<String>,
    severity: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::types::Severity;
    use crate::vuln::asset::assess_asset;
    use crate::vuln::prioritizer::prioritize_findings;
    use crate::vuln::triage::triage_finding;
    use crate::vuln::{AssetCriticality, CvssScore, ExploitInfo, Remediation, VulnAssessment};

    let result_tx_timeout = result_tx.clone();
    match tokio::time::timeout(std::time::Duration::from_secs(120), async move {
        send_progress(&progress_tx, 0, 3).await;

        let mut assessment = VulnAssessment::new(&mode);

        match mode.as_str() {
            "cvss_calc" => {
                let vector = cvss_vector
                    .as_deref()
                    .unwrap_or("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H");
                match CvssScore::from_vector(vector) {
                    Ok(cvss) => {
                        assessment.summary.push(format!(
                            "CVSS 3.1 Score: {:.1} ({})",
                            cvss.base_score,
                            cvss.severity()
                        ));
                        assessment.summary.push(format!("Vector: {}", cvss.vector));
                        assessment.cvss_score = Some(cvss);
                    }
                    Err(e) => {
                        assessment.summary.push(format!("Error: {}", e));
                    }
                }
            }
            "exploit_check" => {
                let cve = cve_id.as_deref().unwrap_or("CVE-2021-44228");
                let info = ExploitInfo::assess(cve);
                assessment.summary.push(format!("Exploitability: {}", cve));
                assessment.summary.push(format!(
                    "Public Exploit: {}",
                    if info.has_public_exploit { "Yes" } else { "No" }
                ));
                assessment.summary.push(format!(
                    "CISA KEV: {}",
                    if info.in_cisa_kev { "Yes" } else { "No" }
                ));
                assessment
                    .summary
                    .push(format!("Exploit Score: {:.1}", info.exploit_score));
                assessment.exploit_info = Some(info);
            }
            "asset_assess" => {
                let target_str = target.as_deref().unwrap_or("unknown");
                let atype = asset_type.as_deref().unwrap_or("web_server");
                let asset = assess_asset(target_str, atype);
                assessment
                    .summary
                    .push(format!("Asset: {}", asset.asset_id));
                assessment
                    .summary
                    .push(format!("Overall Score: {:.1}", asset.overall_score));
                assessment.summary.push(format!(
                    "Technology: {:.1} | Environment: {:.1} | Data: {:.1} | Users: {:.1}",
                    asset.technology_score,
                    asset.environment_score,
                    asset.data_sensitivity,
                    asset.user_base
                ));
                assessment.asset_criticality = Some(asset);
            }
            "prioritize" => {
                let title_str = title.as_deref().unwrap_or("Untitled finding");
                let sev = severity
                    .as_deref()
                    .and_then(|s| match s.to_lowercase().as_str() {
                        "critical" => Some(Severity::Critical),
                        "high" => Some(Severity::High),
                        "medium" => Some(Severity::Medium),
                        "low" => Some(Severity::Low),
                        _ => Some(Severity::Info),
                    })
                    .unwrap_or(Severity::High);
                let findings = vec![("find-1".to_string(), title_str.to_string(), sev, None)];
                let prioritized = prioritize_findings(&findings);
                assessment
                    .summary
                    .push(format!("Prioritized {} finding(s):", prioritized.len()));
                for f in &prioritized {
                    assessment.summary.push(format!(
                        "  #{} [{}] {} - Risk: {:.1} ({:?})",
                        f.priority_rank,
                        f.severity,
                        f.title,
                        f.risk_score.combined_score,
                        f.risk_score.priority_level
                    ));
                }
                assessment.prioritized_findings = prioritized;
            }
            "triage" => {
                let finding_id_str = cve_id.as_deref().unwrap_or("find-1");
                let title_str = title.as_deref().unwrap_or("Untitled finding");
                let desc_str = description.as_deref().unwrap_or("");
                let sev = severity
                    .as_deref()
                    .and_then(|s| match s.to_lowercase().as_str() {
                        "critical" => Some(Severity::Critical),
                        "high" => Some(Severity::High),
                        "medium" => Some(Severity::Medium),
                        "low" => Some(Severity::Low),
                        _ => Some(Severity::Info),
                    })
                    .unwrap_or(Severity::Medium);
                let cvss = cvss_vector
                    .as_deref()
                    .and_then(|v| CvssScore::from_vector(v).ok().map(|s| s.base_score));
                let result = triage_finding(finding_id_str, title_str, desc_str, sev, cvss);
                assessment
                    .summary
                    .push(format!("Triage: {}", result.finding_id));
                assessment
                    .summary
                    .push(format!("Status: {:?}", result.triage_status));
                assessment
                    .summary
                    .push(format!("Confidence: {:.0}%", result.confidence * 100.0));
                assessment
                    .summary
                    .push(format!("Reason: {}", result.reason));
                assessment.triage_results.push(result);
            }
            "remediation" => {
                let title_str = title.as_deref().unwrap_or("Untitled finding");
                let sev = severity
                    .as_deref()
                    .and_then(|s| match s.to_lowercase().as_str() {
                        "critical" => Some(Severity::Critical),
                        "high" => Some(Severity::High),
                        "medium" => Some(Severity::Medium),
                        "low" => Some(Severity::Low),
                        _ => Some(Severity::Info),
                    })
                    .unwrap_or(Severity::Medium);
                let rem = Remediation::for_finding("find-1", title_str, sev);
                assessment
                    .summary
                    .push(format!("Remediation: {}", rem.title));
                assessment
                    .summary
                    .push(format!("Priority: {:?}", rem.priority));
                assessment
                    .summary
                    .push(format!("Effort: {:.1} hours", rem.effort_hours));
                assessment.summary.push("Steps:".to_string());
                for (i, step) in rem.steps.iter().enumerate() {
                    assessment.summary.push(format!("  {}. {}", i + 1, step));
                }
                assessment.remediation_plans.push(rem);
            }
            _ => {
                send_result(
                    &result_tx,
                    TaskResult::Error(format!("Unknown vuln mode: {}", mode)),
                )
                .await;
                send_progress(&progress_tx, 3, 3).await;
                return Ok(());
            }
        }

        send_progress(&progress_tx, 2, 3).await;
        send_result(&result_tx, TaskResult::Vuln(assessment)).await;
        send_progress(&progress_tx, 3, 3).await;
        Ok(())
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Vuln task timed out after 120s");
            send_result(
                &result_tx_timeout,
                TaskResult::Error("Vuln task timed out".to_string()),
            )
            .await;
            Ok(())
        }
    }
}

#[cfg(feature = "wireless")]
pub async fn run_wireless_task(
    interface: String,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    send_progress(&progress_tx, 0, 2).await;

    let scanner = crate::wireless::WirelessScanner::new();
    let scanner = scanner.with_interface(interface);
    let scan_res = tokio::time::timeout(std::time::Duration::from_secs(30), scanner.scan(10))
        .await
        .map_err(|_| anyhow::anyhow!("Wireless scan timed out after 30s"))
        .and_then(|r| r.map_err(|e| anyhow::anyhow!("Wireless scan failed: {}", e)));

    send_progress(&progress_tx, 2, 2).await;

    match scan_res {
        Ok(r) => {
            send_result(&result_tx, TaskResult::Wireless(r)).await;
        }
        Err(e) => {
            send_result(&result_tx, TaskResult::Error(e.to_string())).await;
        }
    }
    Ok(())
}

#[cfg(feature = "wireless-advanced")]
pub async fn run_wireless_active_task(
    interface: String,
    attack_type: String,
    bssid: Option<String>,
    client: Option<String>,
    frame_count: u64,
    rate_limit: u64,
    dry_run: bool,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::wireless::active::attacks::deauth::{run_deauth, run_disassoc};
    use crate::wireless::active::ActiveAttackConfig;

    send_progress(&progress_tx, 0, 2).await;

    let parsed_bssid = bssid.as_deref().and_then(ActiveAttackConfig::parse_mac);
    let parsed_client = client.as_deref().and_then(ActiveAttackConfig::parse_mac);

    let config = ActiveAttackConfig {
        interface: interface.clone(),
        bssid: parsed_bssid,
        client: parsed_client,
        reason_code: 7,
        max_frames: frame_count.min(1000),
        frames_per_second: rate_limit.min(100),
        dry_run,
    };

    let result = match attack_type.as_str() {
        "disassoc" => {
            tokio::time::timeout(
                std::time::Duration::from_secs(60),
                run_disassoc(&config, config.client.is_none()),
            )
            .await
        }
        _ => {
            tokio::time::timeout(
                std::time::Duration::from_secs(60),
                run_deauth(&config, config.client.is_none()),
            )
            .await
        }
    };

    send_progress(&progress_tx, 2, 2).await;

    match result {
        Ok(Ok(attack_result)) => {
            send_result(&result_tx, TaskResult::WirelessActive(attack_result)).await;
        }
        Ok(Err(e)) => {
            send_result(
                &result_tx,
                TaskResult::Error(format!("Active wireless attack failed: {e}")),
            )
            .await;
        }
        Err(_) => {
            send_result(
                &result_tx,
                TaskResult::Error("Active wireless attack timed out after 60s".to_string()),
            )
            .await;
        }
    }
    Ok(())
}
