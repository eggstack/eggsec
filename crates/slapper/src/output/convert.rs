use serde::{Deserialize, Serialize};
use std::fs;

fn parse_severity(value: &str) -> crate::types::Severity {
    crate::types::Severity::parse_or_default(value)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReportData {
    pub target: String,
    pub scan_type: String,
    pub timestamp: String,
    pub findings: Vec<FindingData>,
    pub open_ports: Vec<PortData>,
    pub services: Vec<ServiceData>,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wireless_networks: Vec<WirelessNetworkReportData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingData {
    pub title: String,
    pub severity: String,
    pub category: String,
    pub description: String,
    pub location: String,
    pub evidence: Option<String>,
    pub remediation: Option<String>,
    #[serde(alias = "cve_ids")]
    pub cwe_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortData {
    pub port: u16,
    pub status: String,
    pub protocol: Option<String>,
    pub state: Option<String>,
    pub service: Option<String>,
    pub version: Option<String>,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceData {
    pub service: String,
    pub version: Option<String>,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessNetworkReportData {
    pub ssid: String,
    pub bssid: String,
    pub channel: u8,
    pub security_type: String,
    pub signal_strength: i32,
    pub last_seen: String,
}

pub fn load_scan_report(path: &str) -> Result<ScanReportData, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
}

pub fn convert_to_junit(report: &ScanReportData) -> Result<String, String> {
    use super::junit::{JUnitBuilder, JUnitTestResult};

    let mut builder = JUnitBuilder::new("Slapper Security Scan");

    for finding in &report.findings {
        let result = if matches!(
            parse_severity(&finding.severity),
            crate::types::Severity::Critical | crate::types::Severity::High
        ) {
            JUnitTestResult::Failed {
                message: finding.description.clone(),
                failure_type: finding.category.clone(),
                text: finding.evidence.clone(),
            }
        } else {
            JUnitTestResult::Passed
        };

        builder = builder.add_test_case(
            &report.target,
            &finding.title,
            &finding.category,
            0.0,
            result,
        );
    }

    let junit_report = builder.build();
    junit_report
        .to_xml()
        .map_err(|e| format!("Failed to generate JUnit XML: {}", e))
}

pub fn convert_to_sarif(report: &ScanReportData) -> Result<String, String> {
    use super::sarif::SarifBuilder;

    let mut builder = SarifBuilder::new();

    for finding in &report.findings {
        let level = match parse_severity(&finding.severity) {
            crate::types::Severity::Critical | crate::types::Severity::High => "error",
            crate::types::Severity::Medium => "warning",
            _ => "note",
        };

        builder = builder.add_result(
            &format!(
                "SLAPPER-{}",
                finding.category.to_uppercase().replace(" ", "-")
            ),
            level,
            &finding.description,
            &finding.location,
        );
    }

    let sarif_report = builder.build();
    sarif_report
        .to_json()
        .map_err(|e| format!("Failed to generate SARIF: {}", e))
}

pub fn convert_to_html(report: &ScanReportData) -> String {
    use super::html::generate_html_report;
    use super::markdown::ScanSummary;

    let summary = ScanSummary::from(report);
    let findings: Vec<super::markdown::Finding> =
        report.findings.iter().map(|f| f.into()).collect();

    let mut html = generate_html_report(summary, findings);

    if !report.wireless_networks.is_empty() {
        html.push_str("<section class=\"wireless-networks\">\n");
        html.push_str("<h2>Wireless Networks</h2>\n");
        html.push_str("<table border=\"1\" cellpadding=\"5\" cellspacing=\"0\">\n");
        html.push_str("<tr><th>SSID</th><th>BSSID</th><th>Channel</th><th>Security</th><th>Signal</th><th>Last Seen</th></tr>\n");
        for network in &report.wireless_networks {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{} dBm</td><td>{}</td></tr>\n",
                super::escape::escape_html(&network.ssid),
                super::escape::escape_html(&network.bssid),
                network.channel,
                super::escape::escape_html(&network.security_type),
                network.signal_strength,
                super::escape::escape_html(&network.last_seen)
            ));
        }
        html.push_str("</table>\n");
        html.push_str("</section>\n\n");
    }

    html
}

pub fn convert_to_markdown(report: &ScanReportData) -> Result<String, std::fmt::Error> {
    use super::markdown::{generate_markdown_report, ScanSummary};
    use std::fmt::Write;

    let summary = ScanSummary::from(report);
    let findings: Vec<super::markdown::Finding> =
        report.findings.iter().map(|f| f.into()).collect();

    let mut md = generate_markdown_report(summary, findings)?;

    if !report.wireless_networks.is_empty() {
        writeln!(md, "## Wireless Networks\n")?;
        writeln!(
            md,
            "| SSID | BSSID | Channel | Security | Signal | Last Seen |"
        )?;
        writeln!(
            md,
            "|------|-------|---------|----------|--------|-----------|"
        )?;
        for network in &report.wireless_networks {
            let escape_pipe = |s: &str| s.replace('|', "\\|");
            writeln!(
                md,
                "| {} | {} | {} | {} | {} dBm | {} |",
                escape_pipe(&network.ssid),
                escape_pipe(&network.bssid),
                network.channel,
                escape_pipe(&network.security_type),
                network.signal_strength,
                escape_pipe(&network.last_seen)
            )?;
        }
        writeln!(md)?;
    }

    Ok(md)
}

pub fn convert_to_csv(report: &ScanReportData) -> String {
    let mut csv = String::new();
    csv.push_str("severity,category,title,location,description,cves\n");

    for finding in &report.findings {
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            super::escape::escape_csv(&finding.severity),
            super::escape::escape_csv(&finding.category),
            super::escape::escape_csv(&finding.title),
            super::escape::escape_csv(&finding.location),
            super::escape::escape_csv(&finding.description),
            super::escape::escape_csv(&finding.cwe_ids.join(";"))
        ));
    }

    csv
}

pub fn convert_to_json(report: &ScanReportData) -> Result<String, String> {
    serde_json::to_string_pretty(report).map_err(|e| format!("Failed to serialize to JSON: {}", e))
}

impl From<&ScanReportData> for super::markdown::ScanSummary {
    fn from(report: &ScanReportData) -> Self {
        let findings_by_severity = |sev: crate::types::Severity| {
            report
                .findings
                .iter()
                .filter(|f| parse_severity(&f.severity) == sev)
                .count() as u32
        };

        super::markdown::ScanSummary {
            target: report.target.clone(),
            scan_type: report.scan_type.clone(),
            timestamp: report.timestamp.clone(),
            duration_seconds: report.duration_ms / 1000,
            total_requests: 0,
            findings_count: report.findings.len() as u32,
            critical_count: findings_by_severity(crate::types::Severity::Critical),
            high_count: findings_by_severity(crate::types::Severity::High),
            medium_count: findings_by_severity(crate::types::Severity::Medium),
            low_count: findings_by_severity(crate::types::Severity::Low),
            info_count: findings_by_severity(crate::types::Severity::Info),
        }
    }
}

impl From<&FindingData> for super::markdown::Finding {
    fn from(f: &FindingData) -> Self {
        super::markdown::Finding {
            title: f.title.clone(),
            severity: f.severity.clone(),
            category: f.category.clone(),
            description: f.description.clone(),
            location: f.location.clone(),
            evidence: f.evidence.clone(),
            remediation: f.remediation.clone(),
            references: Vec::new(),
            cwe_ids: f.cwe_ids.clone(),
        }
    }
}

impl From<&FindingData> for super::AgentFinding {
    fn from(f: &FindingData) -> Self {
        let severity = parse_severity(&f.severity);
        super::AgentFinding::new(
            f.category.clone(),
            severity,
            f.title.clone(),
            f.location.clone(),
            f.location.clone(),
        )
        .with_description(f.description.clone())
        .with_evidence(super::Evidence::new().with_request(f.evidence.clone().unwrap_or_default()))
        .with_remediation(super::Remediation::new(
            f.remediation.clone().unwrap_or_default(),
        ))
    }
}

impl From<&super::AgentFinding> for FindingData {
    fn from(f: &super::AgentFinding) -> Self {
        Self {
            title: f.title.clone(),
            severity: f.severity.as_str().to_string(),
            category: f.vulnerability_type.clone(),
            description: f.description.clone(),
            location: f.endpoint.clone(),
            evidence: f.evidence.request.clone(),
            remediation: Some(f.remediation.summary.clone()),
            cwe_ids: f.cwe_ids.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report_with_severity(severity: &str) -> ScanReportData {
        ScanReportData {
            target: "example.com".to_string(),
            scan_type: "full".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            findings: vec![FindingData {
                title: "Test finding".to_string(),
                severity: severity.to_string(),
                category: "xss".to_string(),
                description: "desc".to_string(),
                location: "/".to_string(),
                evidence: None,
                remediation: None,
                cwe_ids: vec![],
            }],
            open_ports: vec![],
            services: vec![],
            duration_ms: 1000,
            wireless_networks: vec![],
        }
    }

    #[test]
    fn junit_treats_mixed_case_high_as_failed() {
        let report = sample_report_with_severity("High");
        let xml = convert_to_junit(&report).expect("JUnit conversion should succeed");
        assert!(xml.contains("<failure"));
    }

    #[test]
    fn sarif_treats_mixed_case_medium_as_warning() {
        let report = sample_report_with_severity("MeDiuM");
        let json = convert_to_sarif(&report).expect("SARIF conversion should succeed");
        let sarif: serde_json::Value =
            serde_json::from_str(&json).expect("SARIF output should be valid JSON");
        let level = sarif["runs"][0]["results"][0]["level"]
            .as_str()
            .expect("result level should be a string");
        assert_eq!(level, "warning");
    }

    #[test]
    fn summary_counts_mixed_case_critical() {
        let report = sample_report_with_severity("CRITICAL");
        let summary = crate::output::markdown::ScanSummary::from(&report);
        assert_eq!(summary.critical_count, 1);
    }
}
