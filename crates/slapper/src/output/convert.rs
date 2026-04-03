#![allow(clippy::single_char_add_str)]
#![allow(clippy::unnecessary_to_owned)]

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReportData {
    pub target: String,
    pub scan_type: String,
    pub timestamp: String,
    pub findings: Vec<FindingData>,
    pub open_ports: Vec<PortData>,
    pub services: Vec<ServiceData>,
    pub duration_ms: u64,
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
    pub cve_ids: Vec<String>,
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

pub fn load_scan_report(path: &str) -> Result<ScanReportData, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
}

pub fn convert_to_junit(report: &ScanReportData) -> String {
    use super::junit::{JUnitBuilder, JUnitTestResult};

    let mut builder = JUnitBuilder::new("Slapper Security Scan");

    for finding in &report.findings {
        let result = if finding.severity == "high" || finding.severity == "critical" {
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
        .unwrap_or_else(|_| "<error>Failed to generate JUnit XML</error>".to_string())
}

pub fn convert_to_sarif(report: &ScanReportData) -> String {
    use super::sarif::SarifBuilder;

    let mut builder = SarifBuilder::new();

    for finding in &report.findings {
        let level = match finding.severity.as_str() {
            "critical" | "high" => "error",
            "medium" => "warning",
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
    sarif_report.to_json().unwrap_or_default()
}

pub fn convert_to_html(report: &ScanReportData) -> String {
    use super::html::generate_html_report;
    use super::markdown::ScanSummary;

    let summary = ScanSummary::from(report);
    let findings: Vec<super::markdown::Finding> =
        report.findings.iter().map(|f| f.into()).collect();

    generate_html_report(summary, findings)
}

pub fn convert_to_markdown(report: &ScanReportData) -> String {
    use super::markdown::{generate_markdown_report, ScanSummary};

    let summary = ScanSummary::from(report);
    let findings: Vec<super::markdown::Finding> =
        report.findings.iter().map(|f| f.into()).collect();

    generate_markdown_report(summary, findings)
}

pub fn convert_to_csv(report: &ScanReportData) -> String {
    let mut csv = String::new();
    csv.push_str("severity,category,title,location,description,cves\n");

    for finding in &report.findings {
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            finding.severity,
            super::escape::escape_csv(&finding.category),
            super::escape::escape_csv(&finding.title),
            super::escape::escape_csv(&finding.location),
            super::escape::escape_csv(&finding.description),
            finding.cve_ids.join(";")
        ));
    }

    csv
}

impl From<&ScanReportData> for super::markdown::ScanSummary {
    fn from(report: &ScanReportData) -> Self {
        let findings_by_severity =
            |sev: &str| report.findings.iter().filter(|f| f.severity == sev).count() as u32;

        super::markdown::ScanSummary {
            target: report.target.clone(),
            scan_type: report.scan_type.clone(),
            timestamp: report.timestamp.clone(),
            duration_seconds: report.duration_ms / 1000,
            total_requests: 0,
            findings_count: report.findings.len() as u32,
            critical_count: findings_by_severity("critical"),
            high_count: findings_by_severity("high"),
            medium_count: findings_by_severity("medium"),
            low_count: findings_by_severity("low"),
            info_count: findings_by_severity("info"),
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
            cve_ids: f.cve_ids.clone(),
        }
    }
}

impl From<&FindingData> for super::AgentFinding {
    fn from(f: &FindingData) -> Self {
        use crate::types::Severity;
        let severity = match f.severity.to_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" => Severity::Medium,
            "low" => Severity::Low,
            _ => Severity::Info,
        };
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
            cve_ids: f.cwe_ids.clone(),
        }
    }
}
