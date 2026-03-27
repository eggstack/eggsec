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
    pub service: Option<String>,
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
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str("\n");
    xml.push_str(
        r#"<testsuites name="Slapper Security Scan" tests="" failures="" errors="" time="">"#,
    );
    xml.push_str("\n");
    xml.push_str(&format!(
        r#"  <testsuite name="{}" tests="{}" failures="{}" errors="{}" time="{}">"#,
        report.target,
        report.findings.len(),
        report
            .findings
            .iter()
            .filter(|f| f.severity == "high" || f.severity == "critical")
            .count(),
        0,
        report.duration_ms as f64 / 1000.0
    ));
    xml.push_str("\n");

    for finding in &report.findings {
        let status = if finding.severity == "high" || finding.severity == "critical" {
            "failure"
        } else {
            "skipped"
        };

        xml.push_str(&format!(
            r#"    <testcase name="{}" classname="{}">"#,
            escape_xml(&finding.title),
            escape_xml(&finding.category)
        ));
        xml.push_str("\n");

        if status == "failure" {
            xml.push_str(&format!(
                r#"      <failure message="{}" type="security">{}</failure>"#,
                escape_xml(&finding.description),
                escape_xml(finding.evidence.as_deref().unwrap_or(""))
            ));
            xml.push_str("\n");
        }

        xml.push_str("    </testcase>\n");
    }

    xml.push_str("  </testsuite>\n");
    xml.push_str("</testsuites>\n");

    xml
}

pub fn convert_to_sarif(report: &ScanReportData) -> String {
    let mut sarif = serde_json::json!({
        "version": "2.1.0",
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "Slapper",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/slapper-tool/slapper"
                }
            },
            "results": []
        }]
    });

    let mut results = Vec::new();

    for finding in &report.findings {
        let level = match finding.severity.as_str() {
            "critical" | "high" => "error",
            "medium" => "warning",
            _ => "note",
        };

        results.push(serde_json::json!({
            "ruleId": format!("SLAPPER-{}", finding.category.to_uppercase().replace(" ", "-")),
            "level": level,
            "message": {
                "text": finding.description
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": finding.location
                    }
                }
            }]
        }));
    }

    if let Some(runs) = sarif.get_mut("runs").and_then(|r| r.as_array_mut()) {
        if let Some(first_run) = runs.first_mut() {
            first_run["results"] = serde_json::json!(results);
        }
    }

    serde_json::to_string_pretty(&sarif).unwrap_or_default()
}

pub fn convert_to_html(report: &ScanReportData) -> String {
    let findings_by_severity =
        |sev: &str| report.findings.iter().filter(|f| f.severity == sev).count();

    let critical = findings_by_severity("critical");
    let high = findings_by_severity("high");
    let medium = findings_by_severity("medium");
    let low = findings_by_severity("low");

    let mut html = String::new();

    html.push_str(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Security Scan Report - </title>"#,
    );
    html.push_str(&report.target);
    html.push_str(r#"</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #1a1a2e; color: #eee; }
        .header { background: #16213e; padding: 20px; border-radius: 8px; margin-bottom: 20px; }
        .summary { display: flex; gap: 20px; margin: 20px 0; }
        .stat { background: #0f3460; padding: 15px 25px; border-radius: 8px; text-align: center; }
        .stat .count { font-size: 2em; font-weight: bold; }
        .stat.critical { border-left: 4px solid #ff4444; }
        .stat.high { border-left: 4px solid #ff8800; }
        .stat.medium { border-left: 4px solid #ffcc00; }
        .stat.low { border-left: 4px solid #4488ff; }
        .findings { margin-top: 20px; }
        .finding { background: #0f3460; padding: 15px; margin: 10px 0; border-radius: 8px; }
        .finding.critical { border-left: 4px solid #ff4444; }
        .finding.high { border-left: 4px solid #ff8800; }
        .finding.medium { border-left: 4px solid #ffcc00; }
        .finding.low { border-left: 4px solid #4488ff; }
        .severity { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 0.8em; text-transform: uppercase; }
        .severity.critical { background: #ff4444; }
        .severity.high { background: #ff8800; }
        .severity.medium { background: #ffcc00; color: #000; }
        .severity.low { background: #4488ff; }
        .location { color: #888; font-family: monospace; font-size: 0.9em; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Security Scan Report</h1>
        <p><strong>Target:</strong> "#);
    html.push_str(&report.target);
    html.push_str(
        r#"</p>
        <p><strong>Scan Type:</strong> "#,
    );
    html.push_str(&report.scan_type);
    html.push_str(
        r#"</p>
        <p><strong>Timestamp:</strong> "#,
    );
    html.push_str(&report.timestamp);
    html.push_str(
        r#"</p>
    </div>

    <div class="summary">
        <div class="stat critical"><div class="count">"#,
    );
    html.push_str(&critical.to_string());
    html.push_str(
        r#"</div>Critical</div>
        <div class="stat high"><div class="count">"#,
    );
    html.push_str(&high.to_string());
    html.push_str(
        r#"</div>High</div>
        <div class="stat medium"><div class="count">"#,
    );
    html.push_str(&medium.to_string());
    html.push_str(
        r#"</div>Medium</div>
        <div class="stat low"><div class="count">"#,
    );
    html.push_str(&low.to_string());
    html.push_str(
        r#"</div>Low</div>
    </div>

    <div class="findings">
        <h2>Findings</h2>"#,
    );

    for finding in &report.findings {
        html.push_str(&format!(
            r#"
        <div class="finding {}">
            <span class="severity {}">{}</span>
            <h3>{}</h3>
            <p class="location">{}</p>
            <p>{}</p>
        </div>"#,
            finding.severity.clone(),
            finding.severity.clone(),
            finding.severity.to_uppercase(),
            escape_html(&finding.title),
            escape_html(&finding.location),
            escape_html(&finding.description)
        ));
    }

    html.push_str(
        r#"
    </div>
</body>
</html>"#,
    );

    html
}

pub fn convert_to_markdown(report: &ScanReportData) -> String {
    let mut md = String::new();

    md.push_str("# Security Scan Report\n\n");
    md.push_str("## Summary\n\n");
    md.push_str("| Field | Value |\n");
    md.push_str("|-------|-------|\n");
    md.push_str(&format!("| Target | {} |\n", report.target));
    md.push_str(&format!("| Scan Type | {} |\n", report.scan_type));
    md.push_str(&format!("| Timestamp | {} |\n", report.timestamp));
    md.push_str(&format!("| Duration | {} ms |\n", report.duration_ms));
    md.push_str(&format!("| Total Findings | {} |\n", report.findings.len()));
    md.push_str("\n## Findings by Severity\n\n");
    md.push_str("| Severity | Count |\n");
    md.push_str("|----------|-------|\n");

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for f in &report.findings {
        *counts.entry(f.severity.clone()).or_insert(0) += 1;
    }

    for sev in &["critical", "high", "medium", "low", "info"] {
        md.push_str(&format!(
            "| {} | {} |\n",
            sev,
            counts.get(&sev.to_string()).unwrap_or(&0)
        ));
    }

    md.push_str("\n## Detailed Findings\n\n");

    for finding in &report.findings {
        md.push_str(&format!(
            "### [{}] {}\n\n",
            finding.severity.to_uppercase(),
            finding.title
        ));
        md.push_str(&format!("**Location:** `{}`\n\n", finding.location));
        md.push_str(&format!("**Category:** {}\n\n", finding.category));
        md.push_str(&format!("{}\n\n", finding.description));

        if let Some(ref remediation) = finding.remediation {
            md.push_str(&format!("**Remediation:** {}\n\n", remediation));
        }

        if !finding.cve_ids.is_empty() {
            md.push_str(&format!("**CVEs:** {}\n\n", finding.cve_ids.join(", ")));
        }

        md.push_str("---\n\n");
    }

    md
}

pub fn convert_to_csv(report: &ScanReportData) -> String {
    let mut csv = String::new();
    csv.push_str("severity,category,title,location,description,cves\n");

    for finding in &report.findings {
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            finding.severity,
            escape_csv(&finding.category),
            escape_csv(&finding.title),
            escape_csv(&finding.location),
            escape_csv(&finding.description),
            finding.cve_ids.join(";")
        ));
    }

    csv
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
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
