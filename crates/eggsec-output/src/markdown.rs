use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::agent::AgentFinding;

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
    #[serde(alias = "cve_ids")]
    pub cwe_ids: Vec<String>,
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
            cwe_ids: f.cwe_ids.clone(),
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

/// Escape user-controlled text for Markdown table cells: a literal `|` would
/// split the row, and newlines/control characters would break the table.
fn escape_table_cell(s: &str) -> String {
    s.replace('|', "\\|")
        .replace(['\n', '\r'], " ")
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

/// Sanitize user-controlled text rendered as headings or inline fields:
/// collapse newlines and strip control characters so finding content cannot
/// inject headings, lists, or multi-line block structure.
fn sanitize_inline(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
        .chars()
        .filter(|c| !c.is_control())
        .collect()
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
        writeln!(
            md,
            "| Target | {} |",
            escape_table_cell(&self.summary.target)
        )?;
        writeln!(
            md,
            "| Scan Type | {} |",
            escape_table_cell(&self.summary.scan_type)
        )?;
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

                writeln!(
                    md,
                    "### {}. {} {}\n",
                    i + 1,
                    severity_icon,
                    sanitize_inline(&finding.title)
                )?;
                writeln!(
                    md,
                    "**Severity:** {}  \n",
                    sanitize_inline(&finding.severity)
                )?;
                writeln!(
                    md,
                    "**Category:** {}  \n",
                    sanitize_inline(&finding.category)
                )?;
                writeln!(
                    md,
                    "**Location:** {}  \n\n",
                    sanitize_inline(&finding.location)
                )?;

                writeln!(md, "{}\n\n", finding.description)?;

                if let Some(evidence) = &finding.evidence {
                    writeln!(md, "**Evidence:**\n```\n{}\n```\n\n", evidence)?;
                }

                if let Some(remediation) = &finding.remediation {
                    writeln!(md, "**Remediation:** {}\n\n", remediation)?;
                }

                if !finding.cwe_ids.is_empty() {
                    writeln!(md, "**CWE IDs:** {}\n\n", finding.cwe_ids.join(", "))?;
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

pub fn generate_markdown_report(
    summary: ScanSummary,
    findings: Vec<Finding>,
) -> Result<String, std::fmt::Error> {
    let report = MarkdownReport::new(summary, findings);
    report.generate()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_summary(target: &str) -> ScanSummary {
        ScanSummary {
            target: target.to_string(),
            scan_type: "recon".to_string(),
            timestamp: "2026-09-03".to_string(),
            duration_seconds: 1,
            total_requests: 1,
            findings_count: 1,
            critical_count: 0,
            high_count: 1,
            medium_count: 0,
            low_count: 0,
            info_count: 0,
        }
    }

    #[test]
    fn user_controlled_fields_do_not_break_table_or_headings() {
        let findings = vec![Finding {
            title: "XSS\n# injected heading".to_string(),
            severity: "High".to_string(),
            category: "xss".to_string(),
            description: "desc".to_string(),
            location: "https://example.com/?q=1|2".to_string(),
            evidence: None,
            remediation: None,
            references: vec![],
            cwe_ids: vec![],
        }];
        let md = generate_markdown_report(test_summary("example.com | injected"), findings)
            .expect("report generation is infallible for valid input");
        assert!(md.contains("example.com \\| injected"));
        // The newline in the title is collapsed: no line may start a new
        // heading from finding content.
        assert!(
            !md.lines()
                .any(|line| line.starts_with("# injected heading")),
            "finding title must not inject a heading line"
        );
        assert!(md.contains("XSS # injected heading"));
    }
}
