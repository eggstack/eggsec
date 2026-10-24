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
//! | wireless, wireless-active, db-pentest, c2 | the engine's existing per-domain bridges, reused verbatim |
//!
//! Every other [`TaskResult`] variant yields `None`. "No report available" is
//! the honest answer for a result with no report form; a lossy document
//! pretending to be a report is not.
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
        _ => None,
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
}
