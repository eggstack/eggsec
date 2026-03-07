use serde::{Deserialize, Serialize};

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

    pub fn generate(&self) -> String {
        let mut md = String::new();

        md.push_str(&format!("# Security Scan Report\n\n"));
        md.push_str(&format!("## Summary\n\n"));
        md.push_str(&format!("| Field | Value |\n"));
        md.push_str(&format!("|-------|-------|\n"));
        md.push_str(&format!("| Target | {} |\n", self.summary.target));
        md.push_str(&format!("| Scan Type | {} |\n", self.summary.scan_type));
        md.push_str(&format!("| Timestamp | {} |\n", self.summary.timestamp));
        md.push_str(&format!(
            "| Duration | {} seconds |\n",
            self.summary.duration_seconds
        ));
        md.push_str(&format!(
            "| Total Requests | {} |\n",
            self.summary.total_requests
        ));
        md.push_str(&format!("| Critical | {} |\n", self.summary.critical_count));
        md.push_str(&format!("| High | {} |\n", self.summary.high_count));
        md.push_str(&format!("| Medium | {} |\n", self.summary.medium_count));
        md.push_str(&format!("| Low | {} |\n", self.summary.low_count));
        md.push_str(&format!("| Info | {} |\n", self.summary.info_count));
        md.push_str("\n");

        if !self.findings.is_empty() {
            md.push_str("## Findings\n\n");

            for (i, finding) in self.findings.iter().enumerate() {
                let severity_icon = match finding.severity.to_lowercase().as_str() {
                    "critical" => "🔴",
                    "high" => "🟠",
                    "medium" => "🟡",
                    "low" => "🔵",
                    _ => "⚪",
                };

                md.push_str(&format!(
                    "### {}. {} {}\n\n",
                    i + 1,
                    severity_icon,
                    finding.title
                ));
                md.push_str(&format!("**Severity:** {}  \n", finding.severity));
                md.push_str(&format!("**Category:** {}  \n", finding.category));
                md.push_str(&format!("**Location:** {}  \n\n", finding.location));

                md.push_str(&format!("{}\n\n", finding.description));

                if let Some(evidence) = &finding.evidence {
                    md.push_str(&format!("**Evidence:**\n```\n{}\n```\n\n", evidence));
                }

                if let Some(remediation) = &finding.remediation {
                    md.push_str(&format!("**Remediation:** {}\n\n", remediation));
                }

                if !finding.cve_ids.is_empty() {
                    md.push_str(&format!("**CVE IDs:** {}\n\n", finding.cve_ids.join(", ")));
                }

                if !finding.references.is_empty() {
                    md.push_str("**References:**\n");
                    for reference in &finding.references {
                        md.push_str(&format!("- {}\n", reference));
                    }
                    md.push_str("\n");
                }

                md.push_str("---\n\n");
            }
        } else {
            md.push_str("## Findings\n\n");
            md.push_str("No vulnerabilities were found in this scan.\n\n");
        }

        md
    }
}

pub fn generate_markdown_report(summary: ScanSummary, findings: Vec<Finding>) -> String {
    let report = MarkdownReport::new(summary, findings);
    report.generate()
}
