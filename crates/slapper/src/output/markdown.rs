use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::output::agent::AgentFinding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub title: String,
    pub severity: String,
    pub category: String,
    pub description: String,
    pub location: String,
    pub evidence: Option<String>,
    pub remediation: Option<String>,
    pub references: Vec<String>,
    pub cve_ids: Vec<String>,
}

impl From<&AgentFinding> for Finding {
    fn from(f: &AgentFinding) -> Self {
        Self {
            title: f.title.clone(),
            severity: f.severity.as_str().to_string(),
            category: f.vulnerability_type.clone(),
            description: f.description.clone(),
            location: f.endpoint.clone(),
            evidence: f.evidence.request.clone(),
            remediation: Some(f.remediation.summary.clone()),
            references: f.remediation.references.clone(),
            cve_ids: f.cwe_ids.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub target: String,
    pub scan_type: String,
    pub timestamp: String,
    pub duration_seconds: u64,
    pub total_requests: u64,
    pub findings_count: u32,
    pub critical_count: u32,
    pub high_count: u32,
    pub medium_count: u32,
    pub low_count: u32,
    pub info_count: u32,
}

pub struct MarkdownReport {
    findings: Vec<Finding>,
    summary: ScanSummary,
}

impl MarkdownReport {
    pub fn new(summary: ScanSummary, findings: Vec<Finding>) -> Self {
        Self { summary, findings }
    }

    pub fn generate(&self) -> Result<String, std::fmt::Error> {
        let mut md = String::new();

        writeln!(md, "# Security Scan Report\n")?;
        writeln!(md, "## Summary\n")?;
        writeln!(md, "| Field | Value |")?;
        writeln!(md, "|-------|-------|")?;
        writeln!(md, "| Target | {} |", self.summary.target)?;
        writeln!(md, "| Scan Type | {} |", self.summary.scan_type)?;
        writeln!(md, "| Timestamp | {} |", self.summary.timestamp)?;
        writeln!(
            md,
            "| Duration | {} seconds |",
            self.summary.duration_seconds
        )?;
        writeln!(md, "| Total Requests | {} |", self.summary.total_requests)?;
        writeln!(md, "| Critical | {} |", self.summary.critical_count)?;
        writeln!(md, "| High | {} |", self.summary.high_count)?;
        writeln!(md, "| Medium | {} |", self.summary.medium_count)?;
        writeln!(md, "| Low | {} |", self.summary.low_count)?;
        writeln!(md, "| Info | {} |", self.summary.info_count)?;
        writeln!(md)?;

        if !self.findings.is_empty() {
            writeln!(md, "## Findings\n")?;

            for (i, finding) in self.findings.iter().enumerate() {
                let severity_lower = finding.severity.to_lowercase();
                let severity_icon = match severity_lower.as_str() {
                    "critical" => "🔴",
                    "high" => "🟠",
                    "medium" => "🟡",
                    "low" => "🔵",
                    _ => "⚪",
                };

                writeln!(md, "### {}. {} {}\n", i + 1, severity_icon, finding.title)?;
                writeln!(md, "**Severity:** {}  \n", finding.severity)?;
                writeln!(md, "**Category:** {}  \n", finding.category)?;
                writeln!(md, "**Location:** {}  \n\n", finding.location)?;

                writeln!(md, "{}\n\n", finding.description)?;

                if let Some(evidence) = &finding.evidence {
                    writeln!(md, "**Evidence:**\n```\n{}\n```\n\n", evidence)?;
                }

                if let Some(remediation) = &finding.remediation {
                    writeln!(md, "**Remediation:** {}\n\n", remediation)?;
                }

                if !finding.cve_ids.is_empty() {
                    writeln!(md, "**CVE IDs:** {}\n\n", finding.cve_ids.join(", "))?;
                }

                if !finding.references.is_empty() {
                    writeln!(md, "**References:**\n")?;
                    for reference in &finding.references {
                        writeln!(md, "- {}\n", reference)?;
                    }
                    writeln!(md)?;
                }

                writeln!(md, "---\n\n")?;
            }
        } else {
            writeln!(md, "## Findings\n\n")?;
            writeln!(md, "No vulnerabilities were found in this scan.\n\n")?;
        }

        Ok(md)
    }
}

pub fn generate_markdown_report(summary: ScanSummary, findings: Vec<Finding>) -> Result<String, std::fmt::Error> {
    let report = MarkdownReport::new(summary, findings);
    report.generate()
}
