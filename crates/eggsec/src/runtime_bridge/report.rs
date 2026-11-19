//! Engine-side report bridge: a completed task result → the canonical report
//! document.
//!
//! The runtime owns task *lifecycle*; the engine owns report *semantics*. This
//! module is the engine half of that split: it turns a completed
//! [`TaskResult`] into the canonical [`ScanReportData`] document that the
//! runtime's bounded report store retains and the daemon serves.
//!
//! # Purity
//!
//! The bridge is pure. It performs no I/O, reads no state, consults no clock
//! and opens no network connection, so the same result always yields the same
//! document. A result whose type carries no scan timestamp therefore yields a
//! document whose `timestamp` field is empty rather than one stamped with the
//! bridge's own clock reading — a timestamp the result does not know is a
//! value the bridge would have to invent.
//!
//! The per-domain bridges this module reuses are the engine's existing ones
//! and are called verbatim; those carry their own report timestamps.
//!
//! # Scope
//!
//! The bridge covers the result families that have a natural report form:
//!
//! | family | report form |
//! |---|---|
//! | port scan | target, open ports, detected services, duration |
//! | pipeline | target, open ports, services, prioritized findings, duration |
//! | fingerprint | target, detected services, duration |
//! | fuzz | target, the session's vulnerable payloads |
//! | recon | target, detected secrets and takeover candidates |
//! | endpoint scan | target, the interesting endpoints it found |
//! | graphql, oauth | target, the vulnerability classes the run found |
//! | auth test | target, the findings the run produced |
//! | nse, hunt, browser, compliance | target, the findings the run produced |
//! | wireless, wireless-active, db-pentest, c2 | the engine's existing per-domain bridges, reused verbatim |
//!
//! Every other [`TaskResult`] variant yields `None`. "No report available" is
//! the honest answer for a result with no report form; a lossy document
//! pretending to be a report is not.
//!
//! # The verdict gate
//!
//! [`report_for_result`] is the *how*: it matches on the result's variant and
//! builds the document. [`report_for_operation`] is the *whether*: it consults
//! the coverage table — the single source of truth in
//! [`super::report_coverage`] — and only then builds.
//!
//! The two cannot silently disagree: a `Canonical` verdict whose result has
//! no arm is a bridge/table disagreement, and it is logged at error level
//! rather than swallowed.
//!
//! # Relationship to the task envelope
//!
//! This is a **separate** path from [`crate::dispatch::task_result_envelope`].
//! The envelope's `payload` stays `json!({})` and its `artifacts` stay empty:
//! the report body is derived here and handed to the runtime's bounded report
//! store, never folded into the envelope.

use crate::dispatch::TaskResult;
use crate::output::convert::{FindingData, PortData, ScanReportData, ServiceData};
use crate::scanner::{FingerprintResults, PortResult, PortScanResults, ServiceFingerprint};

/// Build the canonical report document for a completed task result.
///
/// Pure: the same result always yields the same document, and no state is
/// read or written. `None` means this result has no report form — the
/// caller reports "no report available" rather than inventing one.
pub fn report_for_result(result: &TaskResult) -> Option<ScanReportData> {
    match result {
        TaskResult::PortScan(scan) => Some(port_scan_report(scan)),
        TaskResult::Pipeline(report) => Some(pipeline_report(report)),
        TaskResult::Fingerprint(results) => Some(fingerprint_report(results)),
        TaskResult::Fuzz(session) => Some(fuzz_report(session)),
        TaskResult::Recon(recon) => Some(recon_report(recon)),
        TaskResult::EndpointScan(results) => Some(endpoint_scan_report(results)),
        TaskResult::GraphQl(results) => Some(graphql_report(results)),
        TaskResult::OAuth(results) => Some(oauth_report(results)),
        TaskResult::Auth(report) => Some(auth_report(report)),
        #[cfg(feature = "wireless")]
        TaskResult::Wireless(scan) => Some(crate::wireless::to_scan_report_data(scan)),
        #[cfg(feature = "wireless-advanced")]
        TaskResult::WirelessActive(attack) => {
            Some(crate::wireless::active::to_active_scan_report_data(attack))
        }
        #[cfg(feature = "db-pentest")]
        TaskResult::DbPentest(report) => Some(crate::db_pentest::to_scan_report_data_db(report)),
        #[cfg(feature = "c2")]
        TaskResult::C2(report) => Some(crate::c2::to_scan_report_data(report)),
        #[cfg(feature = "nse")]
        TaskResult::Nse(results) => Some(nse_report(results)),
        #[cfg(feature = "advanced-hunting")]
        TaskResult::Hunt(report) => Some(hunt_report(report)),
        #[cfg(feature = "headless-browser")]
        TaskResult::Browser(report) => Some(browser_report(report)),
        #[cfg(feature = "compliance")]
        TaskResult::Compliance(report) => Some(compliance_report(report)),
        _ => None,
    }
}

/// The report document for a completed result, if this operation has one.
///
/// The verdict comes from the coverage table — the single source of truth —
/// and the document comes from the result. `operation_id` is the canonical
/// operation identity of the task kind that produced the result
/// (`TaskKind::operation_id()`).
///
/// The function stays pure: no I/O, no state, no clock. Logging is not a
/// side effect on the document.
pub fn report_for_operation(operation_id: &str, result: &TaskResult) -> Option<ScanReportData> {
    let Some(row) = super::report_coverage::coverage_for_operation(operation_id) else {
        // The coverage table covers every canonical operation id, so this is
        // an internal inconsistency rather than a fact about the result.
        tracing::error!(
            operation_id,
            "the report coverage table has no row for this operation; no report was built"
        );
        return None;
    };

    if row.form == super::report_coverage::ReportForm::NoCleanMapping {
        // The table names the fact the result is missing; the operator can
        // find it in the log without the bridge inventing a document.
        tracing::debug!(
            operation_id,
            reason = row.reason,
            "this operation has no clean report mapping; no report was built"
        );
        return None;
    }

    match report_for_result(result) {
        Some(document) => Some(document),
        None => {
            // A canonical verdict with no bridge arm is a bridge/table
            // disagreement. It is an internal error, so it is loud: the
            // static guard makes it unreachable in the source, and this log
            // is the belt to that braces.
            tracing::error!(
                operation_id,
                variant = super::report_coverage::result_variant_name(result),
                "the report bridge has no arm for this canonical result variant; no report was \
                 built"
            );
            None
        }
    }
}

/// The canonical `scan_type` discriminators this bridge emits.
mod scan_type {
    /// A port scan.
    pub const PORT_SCAN: &str = "port-scan";
    /// A full pipeline run.
    pub const PIPELINE: &str = "pipeline";
    /// A service fingerprint run.
    pub const FINGERPRINT: &str = "fingerprint";
    /// A fuzzing session.
    pub const FUZZ: &str = "fuzz";
    /// A reconnaissance run.
    pub const RECON: &str = "recon";
    /// An endpoint discovery run.
    pub const ENDPOINT_SCAN: &str = "endpoint-scan";
    /// A GraphQL testing run.
    pub const GRAPHQL: &str = "graphql";
    /// An OAuth testing run.
    pub const OAUTH: &str = "oauth";
    /// An authentication testing run.
    pub const AUTH_TEST: &str = "auth-test";
    /// An NSE script run.
    #[cfg(feature = "nse")]
    pub const NSE: &str = "nse";
    /// A vulnerability-hunting run.
    #[cfg(feature = "advanced-hunting")]
    pub const HUNT: &str = "hunt";
    /// A headless-browser run.
    #[cfg(feature = "headless-browser")]
    pub const BROWSER: &str = "browser";
    /// A compliance-checking run.
    #[cfg(feature = "compliance")]
    pub const COMPLIANCE: &str = "compliance";
}

/// Build the report document for a port scan.
///
/// The scanner's own `open_ports` collection is mapped verbatim, verdict and
/// all: a UDP run fills that collection with a per-port verdict, and dropping
/// the non-open rows would misreport the run.
fn port_scan_report(scan: &PortScanResults) -> ScanReportData {
    let open_ports: Vec<PortData> = scan.open_ports.iter().map(port_data).collect();

    // The scanner records the detected service name per port; the report
    // lists the distinct names it detected.
    let mut services: Vec<ServiceData> = Vec::new();
    for port in &scan.open_ports {
        let name = port.service.trim();
        if name.is_empty() {
            continue;
        }
        if services.iter().any(|s| s.service == name) {
            continue;
        }
        services.push(ServiceData {
            service: name.to_string(),
            version: None,
            banner: None,
        });
    }

    ScanReportData {
        target: scan.host.clone(),
        scan_type: scan_type::PORT_SCAN.to_string(),
        timestamp: String::new(),
        findings: Vec::new(),
        open_ports,
        services,
        duration_ms: scan.duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for a pipeline run.
///
/// The pipeline's findings are the ones its vulnerability assessment
/// prioritized; a run without an assessment simply carries no findings.
fn pipeline_report(report: &crate::pipeline::PipelineReport) -> ScanReportData {
    let findings: Vec<FindingData> = report
        .vuln_assessment
        .iter()
        .flat_map(|assessment| assessment.prioritized_findings.iter())
        .map(|finding| FindingData {
            title: finding.title.clone(),
            severity: finding.severity.as_str().to_string(),
            category: "pipeline-vulnerability".to_string(),
            description: format!(
                "Priority {} finding from the pipeline's vulnerability assessment.",
                finding.priority_rank
            ),
            location: report.target.clone(),
            evidence: Some(format!(
                "finding_id={} combined_score={:.1}",
                finding.finding_id, finding.risk_score.combined_score
            )),
            remediation: None,
            cwe_ids: Vec::new(),
        })
        .collect();

    ScanReportData {
        target: report.target.clone(),
        scan_type: scan_type::PIPELINE.to_string(),
        timestamp: report
            .manifest
            .as_ref()
            .map(|manifest| manifest.started_at.to_rfc3339())
            .unwrap_or_default(),
        findings,
        open_ports: report.open_ports.iter().map(port_data).collect(),
        services: report.services.iter().map(service_data).collect(),
        duration_ms: report.total_duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for a service fingerprint run.
///
/// A fingerprint run knows which services it identified but not the port
/// verdict behind them, so the report lists the services and no ports.
fn fingerprint_report(results: &FingerprintResults) -> ScanReportData {
    ScanReportData {
        target: results.host.clone(),
        scan_type: scan_type::FINGERPRINT.to_string(),
        timestamp: String::new(),
        findings: Vec::new(),
        open_ports: Vec::new(),
        services: results.results.iter().map(service_data).collect(),
        duration_ms: results.duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for a fuzzing session.
///
/// Only the payloads the session found vulnerable become findings; a payload
/// that was sent and rejected is not a finding.
fn fuzz_report(session: &crate::fuzzer::engine::FuzzSession) -> ScanReportData {
    let findings: Vec<FindingData> = session
        .results
        .iter()
        .filter(|result| result.is_vulnerable())
        .map(|result| {
            let category = result
                .owasp_category
                .clone()
                .unwrap_or_else(|| "fuzz".to_string());
            let mut evidence = vec![format!("payload_type={}", result.payload.payload_type)];
            if !result.leaks_found.is_empty() {
                evidence.push(format!("leaks={}", result.leaks_found.join(", ")));
            }
            if result.is_anomaly {
                evidence.push("timing_anomaly=true".to_string());
            }
            if result.is_redos_suspected {
                evidence.push("redos_suspected=true".to_string());
            }
            FindingData {
                title: format!("{} payload finding", result.payload.payload_type),
                severity: result.detected_severity.as_str().to_string(),
                category,
                description: result.payload.description.clone(),
                location: session.target_url.clone(),
                evidence: Some(evidence.join("; ")),
                remediation: None,
                cwe_ids: Vec::new(),
            }
        })
        .collect();

    ScanReportData {
        target: session.target_url.clone(),
        scan_type: scan_type::FUZZ.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: session.duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for a reconnaissance run.
///
/// Two recon facts are findings: a credential pattern detected in a response,
/// and a subdomain that a known service would let an attacker take over.
/// A takeover candidate that probed safe or unknown is not a finding.
fn recon_report(recon: &crate::recon::FullReconResult) -> ScanReportData {
    let mut findings: Vec<FindingData> = Vec::new();

    if let Some(secrets) = &recon.secrets {
        for secret in secrets {
            findings.push(FindingData {
                title: format!("{} detected", secret.secret_type),
                severity: secret.severity.as_str().to_string(),
                category: "recon-secret".to_string(),
                description: secret.description.clone(),
                location: secret.location.clone(),
                evidence: Some(secret.value_preview.clone()),
                remediation: None,
                cwe_ids: Vec::new(),
            });
        }
    }

    if let Some(takeovers) = &recon.takeover {
        for takeover in takeovers {
            if takeover.status != crate::recon::takeover::TakeoverStatus::Vulnerable {
                continue;
            }
            findings.push(FindingData {
                title: format!(
                    "Subdomain takeover candidate for {}",
                    takeover.target.subdomain
                ),
                severity: takeover.severity.as_str().to_string(),
                category: "recon-takeover".to_string(),
                description: match &takeover.service {
                    Some(service) => format!("Takeover by {service}."),
                    None => "Subdomain takeover candidate.".to_string(),
                },
                location: takeover.target.subdomain.clone(),
                evidence: Some(takeover.evidence.clone()),
                remediation: None,
                cwe_ids: Vec::new(),
            });
        }
    }

    ScanReportData {
        target: recon.target.clone(),
        scan_type: scan_type::RECON.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 0,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for an endpoint discovery run.
///
/// Only the **interesting** endpoints become findings: an uninteresting
/// endpoint is a path the scanner reached, not a fact worth reporting (the
/// same rule the fuzz bridge applies to a rejected payload).
///
/// The severity is the class constant `info`: a discovered endpoint is a
/// fact about the target, not a vulnerability, and the severity is a property
/// of the class rather than of the target.
fn endpoint_scan_report(results: &crate::scanner::EndpointScanResults) -> ScanReportData {
    let findings: Vec<FindingData> = results
        .results
        .iter()
        .filter(|endpoint| endpoint.interesting)
        .map(|endpoint| {
            let mut description =
                format!("status {} {}", endpoint.status_code, endpoint.status_text);
            if let Some(redirect) = &endpoint.redirect {
                description.push_str(&format!("; redirects to {redirect}"));
            }
            FindingData {
                title: format!("interesting endpoint {}", endpoint.path),
                severity: "info".to_string(),
                category: "endpoint-discovery".to_string(),
                description,
                location: endpoint.path.clone(),
                evidence: None,
                remediation: None,
                cwe_ids: Vec::new(),
            }
        })
        .collect();

    ScanReportData {
        target: results.base_url.clone(),
        scan_type: scan_type::ENDPOINT_SCAN.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: results.duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for a GraphQL testing run.
///
/// The result carries no severity of its own, so the bridge assigns the
/// per-class constant: `medium` for a depth-limit bypass and for an alias
/// overload, `high` for an injection. The severity is a property of the
/// vulnerability class, never of the target.
fn graphql_report(results: &crate::dispatch::GraphQlResults) -> ScanReportData {
    let mut findings: Vec<FindingData> = Vec::new();

    if results.depth_limit_bypassed {
        findings.push(FindingData {
            title: "GraphQL depth limit bypassed".to_string(),
            severity: "medium".to_string(),
            category: "graphql-depth-limit".to_string(),
            description: "The endpoint answered a query deeper than its configured depth limit."
                .to_string(),
            location: results.target.clone(),
            evidence: None,
            remediation: None,
            cwe_ids: Vec::new(),
        });
    }

    if results.alias_overload_vulnerable {
        findings.push(FindingData {
            title: "GraphQL alias overload accepted".to_string(),
            severity: "medium".to_string(),
            category: "graphql-alias-overload".to_string(),
            description: "The endpoint accepted a query that overloaded it with aliases."
                .to_string(),
            location: results.target.clone(),
            evidence: None,
            remediation: None,
            cwe_ids: Vec::new(),
        });
    }

    for injection in &results.injection_findings {
        findings.push(FindingData {
            title: "GraphQL injection".to_string(),
            severity: "high".to_string(),
            category: "graphql-injection".to_string(),
            description: injection.clone(),
            location: results.target.clone(),
            evidence: None,
            remediation: None,
            cwe_ids: Vec::new(),
        });
    }

    ScanReportData {
        target: results.target.clone(),
        scan_type: scan_type::GRAPHQL.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: results.duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for an OAuth testing run.
///
/// The result carries no severity of its own, so the bridge assigns the
/// per-class constant: `medium` for an open redirect and for a weak state
/// parameter, `high` for a scope weakness and for a grant weakness.
fn oauth_report(results: &crate::dispatch::OAuthResults) -> ScanReportData {
    let mut findings: Vec<FindingData> = Vec::new();

    let mut push = |vulnerability: &String, severity: &str, category: &str| {
        findings.push(FindingData {
            title: vulnerability.clone(),
            severity: severity.to_string(),
            category: category.to_string(),
            description: vulnerability.clone(),
            location: results.target.clone(),
            evidence: None,
            remediation: None,
            cwe_ids: Vec::new(),
        });
    };

    for vulnerability in &results.redirect_vulnerabilities {
        push(vulnerability, "medium", "oauth-redirect");
    }
    for vulnerability in &results.scope_vulnerabilities {
        push(vulnerability, "high", "oauth-scope");
    }
    for vulnerability in &results.state_vulnerabilities {
        push(vulnerability, "medium", "oauth-state");
    }
    for vulnerability in &results.grant_vulnerabilities {
        push(vulnerability, "high", "oauth-grant");
    }

    ScanReportData {
        target: results.target.clone(),
        scan_type: scan_type::OAUTH.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: results.duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for an authentication testing run.
///
/// Each finding carries its own severity, title, description and
/// recommendation, so the bridge invents nothing here.
fn auth_report(report: &crate::auth::AuthTestReport) -> ScanReportData {
    let findings: Vec<FindingData> = report
        .findings
        .iter()
        .map(|finding| FindingData {
            title: finding.title.clone(),
            severity: finding.severity.as_str().to_string(),
            category: format!("auth-{}", auth_test_type_name(&finding.test_type)),
            description: finding.description.clone(),
            location: report.target.clone(),
            evidence: None,
            remediation: Some(finding.recommendation.clone()),
            cwe_ids: Vec::new(),
        })
        .collect();

    ScanReportData {
        target: report.target.clone(),
        scan_type: scan_type::AUTH_TEST.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 0,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// The report document's category suffix for one authentication test type.
///
/// The name is presentation, not policy: it names the test that produced the
/// finding so an operator can tell a brute-force finding from a
/// session-fixation one.
fn auth_test_type_name(test_type: &crate::auth::AuthTestType) -> &'static str {
    use crate::auth::AuthTestType;
    match test_type {
        AuthTestType::BruteForce => "brute-force",
        AuthTestType::CredentialStuffing => "credential-stuffing",
        AuthTestType::AccountLockout => "account-lockout",
        AuthTestType::RateLimitBypass => "rate-limit-bypass",
        AuthTestType::MfaBypass => "mfa-bypass",
        AuthTestType::SessionFixation => "session-fixation",
        AuthTestType::TimingAttack => "timing-attack",
        AuthTestType::PasswordPolicy => "password-policy",
    }
}

/// Build the report document for an NSE script run.
///
/// The severity of each evidence item is the one the engine's own NSE bridge
/// assigns to that evidence kind, so the two paths cannot disagree.
#[cfg(feature = "nse")]
fn nse_report(results: &crate::dispatch::NseResults) -> ScanReportData {
    let mut findings: Vec<FindingData> = Vec::new();

    if let Some(report) = &results.report {
        for evidence in &report.evidence {
            findings.push(FindingData {
                title: evidence.title.clone(),
                severity: crate::nse_bridge::evidence_kind_to_severity(&evidence.kind)
                    .as_str()
                    .to_string(),
                category: format!("nse-{}", evidence.kind),
                description: evidence.summary.clone(),
                location: results.target.clone(),
                evidence: evidence.raw_excerpt.clone(),
                remediation: None,
                cwe_ids: evidence.references.clone(),
            });
        }
    }

    ScanReportData {
        target: results.target.clone(),
        scan_type: scan_type::NSE.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 0,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for a vulnerability-hunting run.
///
/// Each of the five finding collections contributes its findings, with their
/// own severity. The SPA routes of a browser run are routes, not findings, and
/// are not reported here either.
#[cfg(feature = "advanced-hunting")]
fn hunt_report(report: &crate::hunt::HuntReport) -> ScanReportData {
    let mut findings: Vec<FindingData> = Vec::new();

    for chain in &report.attack_chains {
        findings.push(FindingData {
            title: chain.name.clone(),
            severity: chain.severity.as_str().to_string(),
            category: "hunt-attack-chain".to_string(),
            description: chain.description.clone(),
            location: report.target.clone(),
            evidence: Some(format!("{} steps in the chain", chain.steps.len())),
            remediation: Some(chain.remediation.clone()),
            cwe_ids: Vec::new(),
        });
    }

    for flaw in &report.business_logic {
        findings.push(FindingData {
            title: format!("business logic flaw {}", flaw.id),
            severity: flaw.severity.as_str().to_string(),
            category: "hunt-business-logic".to_string(),
            description: flaw.description.clone(),
            location: flaw.location.clone(),
            evidence: Some(flaw.evidence.clone()),
            remediation: Some(flaw.remediation.clone()),
            cwe_ids: Vec::new(),
        });
    }

    for race in &report.race_conditions {
        findings.push(FindingData {
            title: format!("race condition {}", race.id),
            severity: race.severity.as_str().to_string(),
            category: "hunt-race-condition".to_string(),
            description: race.description.clone(),
            location: race.endpoint.clone(),
            evidence: Some(race.evidence.clone()),
            remediation: None,
            cwe_ids: Vec::new(),
        });
    }

    for bypass in &report.authz_bypasses {
        findings.push(FindingData {
            title: format!("authorization bypass {}", bypass.id),
            severity: bypass.severity.as_str().to_string(),
            category: "hunt-authz-bypass".to_string(),
            description: bypass.description.clone(),
            location: bypass.endpoint.clone(),
            evidence: Some(bypass.evidence.clone()),
            remediation: Some(bypass.remediation.clone()),
            cwe_ids: Vec::new(),
        });
    }

    for issue in &report.session_issues {
        findings.push(FindingData {
            title: format!("session issue {}", issue.id),
            severity: issue.severity.as_str().to_string(),
            category: "hunt-session-issue".to_string(),
            description: issue.description.clone(),
            location: report.target.clone(),
            evidence: Some(issue.evidence.clone()),
            remediation: Some(issue.remediation.clone()),
            cwe_ids: Vec::new(),
        });
    }

    ScanReportData {
        target: report.target.clone(),
        scan_type: scan_type::HUNT.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 0,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for a headless-browser run.
///
/// The DOM XSS findings and the client-side issues become findings, with
/// their own severity, location, evidence and remediation. The SPA routes the
/// run discovered are routes, not findings, and are not reported.
#[cfg(feature = "headless-browser")]
fn browser_report(report: &crate::browser::BrowserReport) -> ScanReportData {
    let mut findings: Vec<FindingData> = Vec::new();

    for finding in &report.dom_xss {
        findings.push(FindingData {
            title: format!("DOM XSS finding {}", finding.id),
            severity: finding.severity.as_str().to_string(),
            category: "browser-dom-xss".to_string(),
            description: finding.description.clone(),
            location: finding.location.clone(),
            evidence: Some(finding.evidence.clone()),
            remediation: Some(finding.remediation.clone()),
            cwe_ids: Vec::new(),
        });
    }

    for issue in &report.client_issues {
        findings.push(FindingData {
            title: format!("client-side issue: {}", issue.issue_type),
            severity: issue.severity.as_str().to_string(),
            category: "browser-client-issue".to_string(),
            description: issue.description.clone(),
            location: issue.location.clone(),
            evidence: Some(issue.evidence.clone()),
            remediation: Some(issue.remediation.clone()),
            cwe_ids: Vec::new(),
        });
    }

    ScanReportData {
        target: report.target.clone(),
        scan_type: scan_type::BROWSER.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 0,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Build the report document for a compliance-checking run.
///
/// Only the **failed** requirements become findings: a requirement the
/// framework passed is not a fact about the target worth reporting, and a
/// requirement marked not applicable is not a failure either.
#[cfg(feature = "compliance")]
fn compliance_report(report: &crate::compliance::ComplianceReport) -> ScanReportData {
    let findings: Vec<FindingData> = report
        .findings
        .iter()
        .filter(|finding| finding.status == crate::compliance::ComplianceStatus::Fail)
        .map(|finding| FindingData {
            title: format!("{} requirement failed", finding.requirement_id),
            severity: finding.severity.as_str().to_string(),
            category: format!("compliance-{}", report.framework.to_lowercase()),
            description: finding.description.clone(),
            location: report.target.clone(),
            evidence: None,
            remediation: Some(finding.remediation.clone()),
            cwe_ids: Vec::new(),
        })
        .collect();

    ScanReportData {
        target: report.target.clone(),
        scan_type: scan_type::COMPLIANCE.to_string(),
        timestamp: String::new(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 0,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Map one scanned port onto the report model's port record.
fn port_data(port: &PortResult) -> PortData {
    PortData {
        port: port.port,
        status: port.status.as_str().to_string(),
        protocol: Some(port.protocol.as_str().to_string()),
        state: None,
        service: Some(port.service.clone()).filter(|service| !service.is_empty()),
        version: None,
        banner: None,
    }
}

/// Map one identified service onto the report model's service record.
fn service_data(service: &ServiceFingerprint) -> ServiceData {
    ServiceData {
        service: service.service.clone(),
        version: service.version.clone(),
        banner: service.banner.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_bridge::report_coverage;

    use crate::scanner::ports::PortStatus;

    fn port(port_number: u16, status: PortStatus, service: &str) -> PortResult {
        PortResult {
            port: port_number,
            status,
            protocol: crate::scanner::ports::PortProtocol::Tcp,
            service: service.to_string(),
        }
    }

    #[test]
    fn a_port_scan_reports_its_target_ports_and_services() {
        let scan = PortScanResults {
            host: "10.0.0.1".into(),
            ports_scanned: 1000,
            open_ports: vec![
                port(22, PortStatus::Open, "ssh"),
                port(80, PortStatus::Open, "http"),
                port(443, PortStatus::Open, "http"),
            ],
            total_open_ports: 3,
            results_truncated: false,
            duration_ms: 500,
            spoof_stats: None,
            #[cfg(feature = "udp-scan")]
            udp_host_state: None,
            #[cfg(feature = "udp-scan")]
            udp_evidence: None,
        };

        let report =
            report_for_result(&TaskResult::PortScan(scan)).expect("a port scan has a report");

        assert_eq!(report.target, "10.0.0.1");
        assert_eq!(report.scan_type, "port-scan");
        assert_eq!(report.duration_ms, 500);
        assert_eq!(report.open_ports.len(), 3);
        assert_eq!(report.open_ports[0].port, 22);
        assert_eq!(report.open_ports[0].status, "open");
        assert_eq!(report.open_ports[0].protocol.as_deref(), Some("tcp"));
        assert_eq!(report.open_ports[0].service.as_deref(), Some("ssh"));
        assert_eq!(
            report.services.len(),
            2,
            "the same detected service is listed once"
        );
        assert_eq!(report.services[0].service, "ssh");
    }

    #[test]
    fn a_port_scan_keeps_the_verdict_of_every_scanned_port() {
        let scan = PortScanResults {
            host: "10.0.0.2".into(),
            ports_scanned: 2,
            open_ports: vec![
                port(53, PortStatus::Filtered, ""),
                port(161, PortStatus::OpenFiltered, "snmp"),
            ],
            total_open_ports: 2,
            results_truncated: false,
            duration_ms: 10,
            spoof_stats: None,
            #[cfg(feature = "udp-scan")]
            udp_host_state: None,
            #[cfg(feature = "udp-scan")]
            udp_evidence: None,
        };

        let report =
            report_for_result(&TaskResult::PortScan(scan)).expect("a port scan has a report");

        assert_eq!(report.open_ports.len(), 2);
        assert_eq!(report.open_ports[0].status, "filtered");
        assert_eq!(report.open_ports[1].status, "open|filtered");
        assert_eq!(
            report.open_ports[0].service, None,
            "a port with no detected service carries no service name"
        );
    }

    #[test]
    fn a_pipeline_reports_its_target_ports_services_and_findings() {
        let mut report = crate::pipeline::PipelineReport {
            target: "example.com".into(),
            total_duration_ms: 4200,
            stage_results: Vec::new(),
            open_ports: vec![port(443, PortStatus::Open, "https")],
            services: vec![ServiceFingerprint {
                port: 443,
                service: "https".into(),
                banner: Some("nginx".into()),
                version: Some("1.25.0".into()),
                product: None,
                extra: None,
                confidence: 90,
            }],
            endpoints: Vec::new(),
            checkpoint_error: None,
            manifest: None,
            vuln_assessment: None,
            load_test_results: None,
            #[cfg(feature = "web-proxy")]
            web_proxy_report: None,
        };

        let mut assessment = crate::vuln::VulnAssessment::new("standard");
        assessment
            .prioritized_findings
            .push(crate::vuln::PrioritizedFinding {
                finding_id: "F-1".into(),
                title: "Outdated TLS configuration".into(),
                severity: crate::types::Severity::Medium,
                risk_score: crate::vuln::RiskScore {
                    cvss_score: 5.0,
                    exploitability_score: 2.0,
                    asset_criticality: 1.0,
                    combined_score: 4.5,
                    priority_level: crate::vuln::PriorityLevel::P2,
                },
                exploit_info: None,
                asset_criticality: None,
                priority_rank: 1,
            });
        report.vuln_assessment = Some(assessment);

        let document =
            report_for_result(&TaskResult::Pipeline(report)).expect("a pipeline run has a report");

        assert_eq!(document.target, "example.com");
        assert_eq!(document.scan_type, "pipeline");
        assert_eq!(document.duration_ms, 4200);
        assert_eq!(document.open_ports.len(), 1);
        assert_eq!(document.services.len(), 1);
        assert_eq!(document.services[0].version.as_deref(), Some("1.25.0"));
        assert_eq!(document.findings.len(), 1);
        assert_eq!(document.findings[0].title, "Outdated TLS configuration");
        assert_eq!(document.findings[0].severity, "medium");
        assert_eq!(document.findings[0].category, "pipeline-vulnerability");
    }

    #[test]
    fn a_fingerprint_reports_its_detected_services() {
        let results = FingerprintResults {
            host: "10.0.0.3".into(),
            ports_scanned: 100,
            services_identified: 1,
            total_services_identified: 1,
            duration_ms: 120,
            results: vec![ServiceFingerprint {
                port: 22,
                service: "ssh".into(),
                banner: Some("OpenSSH 9.6".into()),
                version: Some("9.6".into()),
                product: None,
                extra: None,
                confidence: 80,
            }],
        };

        let document = report_for_result(&TaskResult::Fingerprint(results))
            .expect("a fingerprint has a report");

        assert_eq!(document.target, "10.0.0.3");
        assert_eq!(document.scan_type, "fingerprint");
        assert_eq!(document.duration_ms, 120);
        assert_eq!(document.services.len(), 1);
        assert_eq!(document.services[0].service, "ssh");
        assert_eq!(document.services[0].banner.as_deref(), Some("OpenSSH 9.6"));
        assert!(
            document.open_ports.is_empty(),
            "a fingerprint run knows no port verdict"
        );
    }

    #[test]
    fn a_fuzz_session_reports_only_its_vulnerable_payloads() {
        let vulnerable = crate::fuzzer::engine::FuzzResult {
            payload: crate::fuzzer::payloads::Payload {
                payload_type: crate::fuzzer::payloads::PayloadType::Sqli,
                payload: "1' OR '1'='1".into(),
                description: "SQL injection probe".into(),
                severity: crate::types::Severity::High,
                tags: vec![],
            },
            status_code: 200,
            response_time_ms: 12,
            response_length: Some(1024),
            response_body: None,
            is_waf_blocked: false,
            is_anomaly: false,
            is_redos_suspected: false,
            leaks_found: vec!["password".into()],
            error: None,
            owasp_category: Some("A03:2021".into()),
            detected_severity: crate::types::Severity::High,
        };
        let rejected = crate::fuzzer::engine::FuzzResult {
            payload: crate::fuzzer::payloads::Payload {
                payload_type: crate::fuzzer::payloads::PayloadType::Xss,
                payload: "<script>".into(),
                description: "XSS probe".into(),
                severity: crate::types::Severity::Medium,
                tags: vec![],
            },
            status_code: 200,
            response_time_ms: 8,
            response_length: Some(2048),
            response_body: None,
            is_waf_blocked: false,
            is_anomaly: false,
            is_redos_suspected: false,
            leaks_found: Vec::new(),
            error: None,
            owasp_category: None,
            detected_severity: crate::types::Severity::Info,
        };

        let results = vec![vulnerable, rejected];
        let owasp_summary = crate::fuzzer::engine::OwaspSummary::from_results(&results);

        let session = crate::fuzzer::engine::FuzzSession {
            target_url: "http://example.com/api".into(),
            mode: "quick".into(),
            payload_type: "sqli".into(),
            total_payloads: 2,
            successful_requests: 2,
            failed_requests: 0,
            waf_bypasses: 0,
            potential_leaks: 1,
            time_anomalies: 0,
            redos_suspected: 0,
            duration_ms: 900,
            total_requests: 2,
            findings: 1,
            results,
            owasp_summary,
            baseline: None,
        };

        let document =
            report_for_result(&TaskResult::Fuzz(session)).expect("a fuzz session has a report");

        assert_eq!(document.target, "http://example.com/api");
        assert_eq!(document.scan_type, "fuzz");
        assert_eq!(document.duration_ms, 900);
        assert_eq!(
            document.findings.len(),
            1,
            "a payload that was rejected is not a finding"
        );
        assert_eq!(document.findings[0].severity, "high");
        assert_eq!(document.findings[0].category, "A03:2021");
        assert_eq!(document.findings[0].description, "SQL injection probe");
    }

    #[test]
    fn a_recon_run_reports_its_secrets_and_takeover_candidates() {
        let mut recon = crate::recon::FullReconResult {
            target: "example.com".into(),
            domain: None,
            ip_address: None,
            tech_stack: None,
            tech_error: None,
            reverse_dns: None,
            reverse_dns_error: None,
            geolocation: None,
            geoip_error: None,
            whois: None,
            whois_error: None,
            subdomains: None,
            subdomains_error: None,
            ssl_analysis: None,
            ssl_error: None,
            dns_records: None,
            dns_records_error: None,
            js_analysis: None,
            js_error: None,
            wayback: None,
            wayback_error: None,
            #[cfg(feature = "cloud")]
            cloud: None,
            #[cfg(feature = "cloud")]
            cloud_error: None,
            content: None,
            content_error: None,
            cors: None,
            cors_error: None,
            email_discovery: None,
            email_error: None,
            threat_intel: None,
            threat_intel_error: None,
            cve_mapping: None,
            cve_error: None,
            takeover: None,
            takeover_error: None,
            secrets: None,
        };

        recon.secrets = Some(vec![crate::recon::secrets::SecretFinding {
            secret_type: crate::recon::secrets::SecretType::AwsAccessKey,
            value_preview: "AKIA****".into(),
            location: "https://example.com/app.js".into(),
            confidence: crate::recon::secrets::Confidence::High,
            severity: crate::types::Severity::High,
            description: "AWS access key in a response body".into(),
        }]);

        recon.takeover = Some(vec![
            crate::recon::takeover::TakeoverResult {
                target: crate::recon::takeover::TakeoverTarget {
                    subdomain: "files.example.com".into(),
                    cname: Some("s3.amazonaws.com".into()),
                    ns: None,
                },
                status: crate::recon::takeover::TakeoverStatus::Vulnerable,
                service: Some("AWS S3".into()),
                evidence: "bucket does not exist".into(),
                severity: crate::types::Severity::High,
            },
            crate::recon::takeover::TakeoverResult {
                target: crate::recon::takeover::TakeoverTarget {
                    subdomain: "safe.example.com".into(),
                    cname: None,
                    ns: None,
                },
                status: crate::recon::takeover::TakeoverStatus::Safe,
                service: None,
                evidence: "no takeover indicator".into(),
                severity: crate::types::Severity::Info,
            },
        ]);

        let document =
            report_for_result(&TaskResult::Recon(recon)).expect("a recon run has a report");

        assert_eq!(document.target, "example.com");
        assert_eq!(document.scan_type, "recon");
        assert_eq!(
            document.findings.len(),
            2,
            "a safe takeover candidate is not a finding"
        );
        assert_eq!(document.findings[0].category, "recon-secret");
        assert_eq!(document.findings[0].severity, "high");
        assert_eq!(document.findings[1].category, "recon-takeover");
        assert_eq!(
            document.findings[1].location, "files.example.com",
            "the takeover finding is located at the vulnerable subdomain"
        );
    }

    #[test]
    fn a_result_without_a_report_form_yields_no_document() {
        let result = TaskResult::Error("something went wrong".into());
        assert!(
            report_for_result(&result).is_none(),
            "an error carries no report form"
        );
    }

    #[test]
    fn a_load_test_carries_no_report_form() {
        let result = TaskResult::LoadTest(crate::loadtest::metrics::LoadTestResults {
            target_url: "http://example.com".into(),
            total_requests: 5000,
            successful_requests: 4900,
            failed_requests: 100,
            total_duration_ms: 30000,
            requests_per_second: 150.0,
            latency_min_ms: 10.0,
            latency_max_ms: 500.0,
            latency_mean_ms: 45.2,
            latency_p50_ms: 38.0,
            latency_p90_ms: 90.0,
            latency_p95_ms: 120.0,
            latency_p99_ms: 250.0,
            status_codes: Default::default(),
            errors: vec![],
            error_kinds: Default::default(),
        });
        assert!(
            report_for_result(&result).is_none(),
            "a load test has no canonical report form"
        );
    }

    /// A representative result for one of the excluded variants.
    ///
    /// `None` means the variant does not exist in this build: the payload
    /// type is behind the same feature gate as the variant itself.
    #[allow(clippy::too_many_lines)]
    fn representative_result(variant: &str) -> Option<TaskResult> {
        Some(match variant {
            "LoadTest" => TaskResult::LoadTest(crate::loadtest::metrics::LoadTestResults {
                target_url: "https://example.test".to_string(),
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                total_duration_ms: 0,
                requests_per_second: 0.0,
                latency_min_ms: 0.0,
                latency_max_ms: 0.0,
                latency_mean_ms: 0.0,
                latency_p50_ms: 0.0,
                latency_p90_ms: 0.0,
                latency_p95_ms: 0.0,
                latency_p99_ms: 0.0,
                status_codes: Default::default(),
                errors: vec![],
                error_kinds: Default::default(),
            }),
            "WafDetection" => TaskResult::WafDetection(crate::waf::WafDetectionResult {
                waf_name: None,
                confidence: 0,
                request_error: None,
                matched_headers: vec![],
                matched_cookies: vec![],
                matched_patterns: vec![],
                server_header: None,
                status_code: 0,
            }),
            "WafBypass" => TaskResult::WafBypass {
                detection: crate::waf::WafDetectionResult {
                    waf_name: None,
                    confidence: 0,
                    request_error: None,
                    matched_headers: vec![],
                    matched_cookies: vec![],
                    matched_patterns: vec![],
                    server_header: None,
                    status_code: 0,
                },
                bypasses: vec![],
            },
            "WafStress" => TaskResult::WafStress(vec![]),
            "PacketCapture" => TaskResult::PacketCapture {
                packets_captured: 0,
                output_file: None,
            },
            "PacketTraceroute" => TaskResult::PacketTraceroute { hops: vec![] },
            "PacketSend" => TaskResult::PacketSend {
                packets_sent: 0,
                bytes_sent: 0,
            },
            "Error" => TaskResult::Error("the task failed".to_string()),
            #[cfg(feature = "stress-testing")]
            "StressTest" => TaskResult::StressTest {
                target: "example.test".to_string(),
                stats: crate::stress::StressStats {
                    duration_ms: 0,
                    packets_sent: 0,
                    bytes_sent: 0,
                    errors: 0,
                },
            },
            #[cfg(feature = "database")]
            "Storage" => TaskResult::Storage,
            #[cfg(feature = "database")]
            "StorageListScans" => TaskResult::StorageListScans { scans: vec![] },
            #[cfg(feature = "database")]
            "StorageListFindings" => TaskResult::StorageListFindings { findings: vec![] },
            #[cfg(feature = "external-integrations")]
            "Integrations" => TaskResult::Integrations,
            #[cfg(feature = "external-integrations")]
            "IntegrationsCreateIssue" => TaskResult::IntegrationsCreateIssue {
                issue: crate::integrations::Issue {
                    id: None,
                    title: "a tracker issue".to_string(),
                    description: "a description".to_string(),
                    labels: vec![],
                    severity: None,
                    assignees: vec![],
                    status: None,
                    url: None,
                    created_at: None,
                },
            },
            #[cfg(feature = "external-integrations")]
            "IntegrationsSearchIssues" => TaskResult::IntegrationsSearchIssues { issues: vec![] },
            #[cfg(feature = "finding-workflow")]
            "Workflow" => TaskResult::Workflow(crate::workflow::WorkflowReport::default()),
            #[cfg(feature = "vuln-management")]
            "Vuln" => TaskResult::Vuln(crate::vuln::VulnAssessment::new("standard")),
            #[cfg(feature = "web-proxy")]
            "Intercept" => {
                TaskResult::Intercept(crate::proxy::intercept::types::InterceptSession {
                    listen_addr: "127.0.0.1:0".to_string(),
                    ca_fingerprint: String::new(),
                    dry_run: true,
                    started_at: String::new(),
                    ended_at: String::new(),
                    target: None,
                    flows: vec![],
                    manipulations: vec![],
                    flow_actions: vec![],
                    budget: Default::default(),
                })
            }
            _ => return None,
        })
    }

    /// An excluded operation produces no document — for every variant it can
    /// produce, and in every build those variants exist in.
    #[test]
    fn an_excluded_operation_produces_no_document() {
        for row in report_coverage::REPORT_COVERAGE {
            if row.form != report_coverage::ReportForm::NoCleanMapping {
                continue;
            }
            for variant in row.variants {
                let Some(result) = representative_result(variant) else {
                    assert!(
                        !report_coverage::RESULT_VARIANT_NAMES.contains(variant),
                        "`{variant}` exists in this build, so the test must exercise it"
                    );
                    continue;
                };
                assert!(
                    report_for_operation(row.operation_id, &result).is_none(),
                    "`{variant}` is excluded by `{}` and must produce no document",
                    row.operation_id
                );
                assert!(
                    report_for_result(&result).is_none(),
                    "`{variant}` must have no bridge arm"
                );
            }
        }
    }

    /// A canonical verdict whose result has no arm is an internal error: it
    /// is logged at error level, never swallowed, and never panics.
    #[test]
    fn a_canonical_operation_with_no_bridge_arm_is_loud() {
        let recorder = std::sync::Arc::new(RecordingSubscriber::default());
        {
            let _guard = tracing::subscriber::with_default(recorder.clone(), || {
                assert!(
                    report_for_operation("scan-ports", &TaskResult::Error("boom".to_string()))
                        .is_none(),
                    "a canonical verdict with no bridge arm yields no document"
                );
            });
        }

        let events = recorder
            .events
            .lock()
            .expect("the recorder is not poisoned")
            .clone();
        assert!(
            events
                .iter()
                .any(|(level, message)| *level == tracing::Level::ERROR
                    && message.contains("no arm")),
            "the disagreement is logged at error level, got {events:?}"
        );
    }

    /// The new documents carry no timestamp: the result types have no clock
    /// of their own, and the bridge invents none.
    #[test]
    fn the_new_documents_carry_no_timestamp() {
        let endpoint = report_for_result(&TaskResult::EndpointScan(
            crate::scanner::EndpointScanResults {
                base_url: "https://example.test".to_string(),
                endpoints_scanned: 1,
                endpoints_found: 1,
                total_endpoints_matched: 1,
                interesting_findings: 1,
                duration_ms: 7,
                results: vec![crate::scanner::EndpointResult {
                    path: "/admin".to_string(),
                    status_code: 200,
                    status_text: "OK".to_string(),
                    content_length: Some(4),
                    response_time_ms: 3,
                    redirect: None,
                    interesting: true,
                }],
            },
        ))
        .expect("an endpoint scan has a report form");
        assert_eq!(endpoint.timestamp, "");
        assert_eq!(endpoint.target, "https://example.test");

        let graphql = report_for_result(&TaskResult::GraphQl(crate::dispatch::GraphQlResults {
            target: "https://example.test/graphql".to_string(),
            introspection_enabled: false,
            depth_limit_bypassed: true,
            alias_overload_vulnerable: false,
            injection_findings: vec![],
            total_requests: 3,
            errors: 0,
            duration_ms: 11,
        }))
        .expect("a graphql run has a report form");
        assert_eq!(graphql.timestamp, "");
        assert_eq!(graphql.findings.len(), 1);
        assert_eq!(graphql.findings[0].severity, "medium");
        assert_eq!(graphql.findings[0].category, "graphql-depth-limit");

        let oauth = report_for_result(&TaskResult::OAuth(crate::dispatch::OAuthResults {
            target: "https://example.test/oauth".to_string(),
            redirect_vulnerabilities: vec![
                "the authorization endpoint redirects freely".to_string()
            ],
            scope_vulnerabilities: vec![],
            state_vulnerabilities: vec![],
            grant_vulnerabilities: vec![],
            total_requests: 2,
            errors: 0,
            duration_ms: 5,
        }))
        .expect("an oauth run has a report form");
        assert_eq!(oauth.timestamp, "");
        assert_eq!(oauth.findings.len(), 1);
        assert_eq!(oauth.findings[0].severity, "medium");
        assert_eq!(oauth.findings[0].category, "oauth-redirect");

        let auth = report_for_result(&TaskResult::Auth(crate::auth::AuthTestReport {
            target: "https://example.test/login".to_string(),
            tests_run: vec![],
            brute_force: None,
            credential_stuffing: None,
            lockout_detection: None,
            rate_limit: None,
            mfa: None,
            session: None,
            timing: None,
            password_policy: None,
            total_attempts: 0,
            findings: vec![crate::auth::AuthFinding {
                test_type: crate::auth::AuthTestType::BruteForce,
                severity: crate::types::Severity::High,
                title: "the account lockout never triggers".to_string(),
                description: "the account survived a thousand attempts".to_string(),
                recommendation: "lock the account after five failures".to_string(),
            }],
        }))
        .expect("an auth run has a report form");
        assert_eq!(auth.timestamp, "");
        assert_eq!(auth.findings.len(), 1);
        assert_eq!(auth.findings[0].severity, "high");
        assert_eq!(auth.findings[0].category, "auth-brute-force");
        assert_eq!(
            auth.findings[0].remediation.as_deref(),
            Some("lock the account after five failures")
        );
    }

    /// An endpoint scan reports only the **interesting** endpoints.
    #[test]
    fn an_endpoint_scan_reports_its_interesting_endpoints() {
        let document = report_for_result(&TaskResult::EndpointScan(
            crate::scanner::EndpointScanResults {
                base_url: "https://example.test".to_string(),
                endpoints_scanned: 2,
                endpoints_found: 2,
                total_endpoints_matched: 1,
                interesting_findings: 1,
                duration_ms: 42,
                results: vec![
                    crate::scanner::EndpointResult {
                        path: "/admin".to_string(),
                        status_code: 200,
                        status_text: "OK".to_string(),
                        content_length: Some(4),
                        response_time_ms: 3,
                        redirect: Some("/login".to_string()),
                        interesting: true,
                    },
                    crate::scanner::EndpointResult {
                        path: "/static/app.css".to_string(),
                        status_code: 200,
                        status_text: "OK".to_string(),
                        content_length: Some(9),
                        response_time_ms: 1,
                        redirect: None,
                        interesting: false,
                    },
                ],
            },
        ))
        .expect("an endpoint scan has a report form");

        assert_eq!(document.scan_type, "endpoint-scan");
        assert_eq!(document.duration_ms, 42);
        assert_eq!(document.findings.len(), 1, "only the interesting endpoint");
        assert_eq!(document.findings[0].location, "/admin");
        assert_eq!(document.findings[0].severity, "info");
        assert_eq!(document.findings[0].category, "endpoint-discovery");
        assert!(
            document.findings[0].description.contains("/login"),
            "the redirect is part of the description: {}",
            document.findings[0].description
        );
    }

    /// The NSE arm reuses the engine's own severity mapping for an evidence
    /// kind, so the two paths cannot disagree.
    #[cfg(feature = "nse")]
    #[test]
    fn an_nse_report_carries_the_evidence_items_own_severity() {
        let mut report = eggsec_nse::NseRunReport::new("example.test", "http-title.nse");
        report = report.with_evidence(vec![eggsec_nse::NseEvidenceItem {
            id: "nse-ev-0".to_string(),
            kind: eggsec_nse::NseEvidenceKind::VulnerabilitySignal,
            title: "a version string suggests an outdated server".to_string(),
            summary: "the server reports an end-of-life version".to_string(),
            target: "example.test".to_string(),
            port: Some(80),
            service: Some("http".to_string()),
            confidence: "likely".to_string(),
            source: "http-title.nse".to_string(),
            raw_excerpt: Some("Server: nginx/1.4.0".to_string()),
            references: vec!["CWE-1104".to_string()],
            tags: vec![],
        }]);

        let document = report_for_result(&TaskResult::Nse(crate::dispatch::NseResults {
            target: "example.test".to_string(),
            script: "http-title.nse".to_string(),
            output: String::new(),
            errors: String::new(),
            success: true,
            report: Some(report),
        }))
        .expect("an nse run has a report form");

        assert_eq!(document.scan_type, "nse");
        assert_eq!(document.findings.len(), 1);
        assert_eq!(
            document.findings[0].severity,
            crate::nse_bridge::evidence_kind_to_severity(
                &eggsec_nse::NseEvidenceKind::VulnerabilitySignal
            )
            .as_str()
        );
        assert_eq!(
            document.findings[0].evidence.as_deref(),
            Some("Server: nginx/1.4.0")
        );
        assert_eq!(document.findings[0].cwe_ids, vec!["CWE-1104".to_string()]);
    }

    /// The hunt arm carries every finding collection, with each finding's
    /// own severity.
    #[cfg(feature = "advanced-hunting")]
    #[test]
    fn a_hunt_report_carries_every_finding_collection() {
        let mut report = crate::hunt::HuntReport::new("example.test");
        report
            .business_logic
            .push(crate::hunt::business::BusinessLogicFlaw {
                id: "bl-1".to_string(),
                flaw_type: crate::hunt::business::FlawType::PriceManipulation,
                severity: crate::types::Severity::High,
                description: "the cart accepts a negative quantity".to_string(),
                location: "/cart".to_string(),
                evidence: "a negative quantity".to_string(),
                remediation: "reject a non-positive quantity".to_string(),
                cvss_score: None,
            });
        report.authz_bypasses.push(crate::hunt::authz::AuthzBypass {
            id: "ab-1".to_string(),
            bypass_type: crate::hunt::authz::BypassType::Idor,
            severity: crate::types::Severity::Critical,
            description: "another user's order is readable".to_string(),
            endpoint: "/orders/1".to_string(),
            evidence: "a foreign order id".to_string(),
            remediation: "check the owner on every read".to_string(),
            cvss_score: None,
        });

        let document =
            report_for_result(&TaskResult::Hunt(report)).expect("a hunt run has a report form");

        assert_eq!(document.scan_type, "hunt");
        assert_eq!(document.findings.len(), 2);
        assert_eq!(document.findings[0].severity, "high");
        assert_eq!(document.findings[0].category, "hunt-business-logic");
        assert_eq!(document.findings[0].location, "/cart");
        assert_eq!(document.findings[1].severity, "critical");
        assert_eq!(document.findings[1].category, "hunt-authz-bypass");
        assert_eq!(document.findings[1].location, "/orders/1");
    }

    /// The browser arm carries the DOM XSS findings and the client-side
    /// issues, and not the SPA routes.
    #[cfg(feature = "headless-browser")]
    #[test]
    fn a_browser_report_carries_the_xss_and_client_findings() {
        let mut report = crate::browser::BrowserReport::new("https://example.test");
        report.dom_xss.push(crate::browser::xss_dom::DomXssFinding {
            id: "xss-1".to_string(),
            source: "location.hash".to_string(),
            sink: "innerHTML".to_string(),
            location: "https://example.test/#".to_string(),
            severity: crate::types::Severity::High,
            description: "the hash is written into the DOM".to_string(),
            evidence: "a reflected payload".to_string(),
            remediation: "use textContent".to_string(),
            cvss_score: None,
        });
        report
            .client_issues
            .push(crate::browser::client_checks::ClientIssue {
                id: "ci-1".to_string(),
                issue_type: crate::browser::client_checks::ClientIssueType::MissingSRI,
                severity: crate::types::Severity::Medium,
                location: "https://example.test/app.js".to_string(),
                description: "the script is loaded without a subresource integrity check"
                    .to_string(),
                evidence: "a cross-origin script tag".to_string(),
                remediation: "add an integrity attribute".to_string(),
                cvss_score: None,
            });

        let document = report_for_result(&TaskResult::Browser(report))
            .expect("a browser run has a report form");

        assert_eq!(document.scan_type, "browser");
        assert_eq!(document.findings.len(), 2);
        assert_eq!(document.findings[0].severity, "high");
        assert_eq!(document.findings[0].category, "browser-dom-xss");
        assert_eq!(document.findings[1].severity, "medium");
        assert_eq!(document.findings[1].category, "browser-client-issue");
    }

    /// The compliance arm carries the **failed** requirements only.
    #[cfg(feature = "compliance")]
    #[test]
    fn a_compliance_report_carries_the_failed_requirements_only() {
        let document = report_for_result(&TaskResult::Compliance(
            crate::compliance::ComplianceReport {
                framework: "OWASP".to_string(),
                target: "https://example.test".to_string(),
                overall_score: 0.5,
                total_requirements: 2,
                passed: 1,
                failed: 1,
                findings: vec![
                    crate::compliance::ComplianceFinding {
                        requirement_id: "A01".to_string(),
                        description: "the injection protection is missing".to_string(),
                        severity: crate::types::Severity::High,
                        status: crate::compliance::ComplianceStatus::Fail,
                        remediation: "validate the input".to_string(),
                    },
                    crate::compliance::ComplianceFinding {
                        requirement_id: "A02".to_string(),
                        description: "the crypto is adequate".to_string(),
                        severity: crate::types::Severity::Info,
                        status: crate::compliance::ComplianceStatus::Pass,
                        remediation: String::new(),
                    },
                ],
            },
        ))
        .expect("a compliance run has a report form");

        assert_eq!(document.scan_type, "compliance");
        assert_eq!(
            document.findings.len(),
            1,
            "a passing requirement is not a finding"
        );
        assert_eq!(document.findings[0].severity, "high");
        assert_eq!(document.findings[0].category, "compliance-owasp");
        assert!(
            document.findings[0].title.contains("A01"),
            "the requirement is named: {}",
            document.findings[0].title
        );
    }

    /// A subscriber that records the events a test emits.
    ///
    /// The engine's `tracing-subscriber` dependency is optional, so the test
    /// records the events with a subscriber of its own rather than adding a
    /// dependency to the crate.
    #[derive(Debug, Default)]
    struct RecordingSubscriber {
        events: std::sync::Mutex<Vec<(tracing::Level, String)>>,
    }

    impl tracing::Subscriber for RecordingSubscriber {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            let mut message = MessageVisitor(String::new());
            event.record(&mut message);
            let mut events = self.events.lock().expect("the recorder is not poisoned");
            events.push((*event.metadata().level(), message.0));
        }

        fn enter(&self, _span: &tracing::span::Id) {}

        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// Collects an event's message field.
    struct MessageVisitor(String);

    impl tracing::field::Visit for MessageVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0 = format!("{value:?}");
            }
        }
    }
}
