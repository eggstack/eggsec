use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use super::finding_schema::VersionedFindingPy;

/// Base reporter for generating finding reports in various formats.
#[pyclass(name = "FindingReporter")]
pub struct FindingReporterPy {
    format: String,
    redaction_policy: String,
    include_artifacts: bool,
}

#[pymethods]
impl FindingReporterPy {
    #[new]
    #[pyo3(signature = (format, *, redaction_policy=None, include_artifacts=None))]
    fn new(
        format: String,
        redaction_policy: Option<String>,
        include_artifacts: Option<bool>,
    ) -> Self {
        Self {
            format,
            redaction_policy: redaction_policy.unwrap_or_else(|| "redact_sensitive".to_string()),
            include_artifacts: include_artifacts.unwrap_or(false),
        }
    }

    /// Generate a report in the configured format.
    #[pyo3(signature = (findings, title=None))]
    fn generate(
        &self,
        findings: Vec<VersionedFindingPy>,
        title: Option<String>,
    ) -> PyResult<String> {
        match self.format.as_str() {
            "json" => self.generate_json(findings, title),
            "jsonl" => self.generate_jsonl(findings),
            "markdown" => self.generate_markdown(findings, title),
            "csv" => self.generate_csv(findings),
            "html" => self.generate_html(findings, title),
            "sarif" => self.generate_sarif(findings, title),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unsupported format: '{}'. Supported: json, jsonl, markdown, csv, html, sarif",
                self.format
            ))),
        }
    }

    /// Generate a JSON report.
    #[pyo3(signature = (findings, title=None))]
    fn generate_json(
        &self,
        findings: Vec<VersionedFindingPy>,
        title: Option<String>,
    ) -> PyResult<String> {
        let report_title = title.unwrap_or_else(|| "Eggsec Findings Report".to_string());
        let envelope = InternalReportEnvelope {
            title: report_title,
            generated_at: chrono::Utc::now().to_rfc3339(),
            finding_count: findings.len() as u32,
            severity_summary: SeveritySummary::from_findings(&findings),
            findings,
        };
        serde_json::to_string_pretty(&envelope)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Generate a JSONL report (one JSON object per line).
    fn generate_jsonl(&self, findings: Vec<VersionedFindingPy>) -> PyResult<String> {
        let mut lines = Vec::with_capacity(findings.len());
        for finding in &findings {
            let line = serde_json::to_string(finding)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            lines.push(line);
        }
        Ok(lines.join("\n"))
    }

    /// Generate a Markdown report.
    #[pyo3(signature = (findings, title=None))]
    fn generate_markdown(
        &self,
        findings: Vec<VersionedFindingPy>,
        title: Option<String>,
    ) -> PyResult<String> {
        let report_title = title.unwrap_or_else(|| "Eggsec Findings Report".to_string());
        let summary = SeveritySummary::from_findings(&findings);
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", report_title));
        md.push_str(&format!(
            "*Generated: {}*\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Summary table
        md.push_str("## Summary\n\n");
        md.push_str("| Severity | Count |\n");
        md.push_str("|----------|-------|\n");
        md.push_str(&format!("| Critical | {} |\n", summary.critical));
        md.push_str(&format!("| High | {} |\n", summary.high));
        md.push_str(&format!("| Medium | {} |\n", summary.medium));
        md.push_str(&format!("| Low | {} |\n", summary.low));
        md.push_str(&format!("| Info | {} |\n", summary.info));
        md.push_str(&format!("| **Total** | **{}** |\n\n", summary.total));
        md.push_str(&format!("**Risk Score:** {:.1}\n\n", summary.risk_score));

        // Findings table
        if !findings.is_empty() {
            md.push_str("## Findings\n\n");
            md.push_str("| # | ID | Severity | Title | Asset | Type |\n");
            md.push_str("|---|-----|----------|-------|-------|------|\n");
            for (i, f) in findings.iter().enumerate() {
                md.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} |\n",
                    i + 1,
                    f.id,
                    f.severity,
                    f.title,
                    f.affected_asset.identifier,
                    f.finding_type.as_str(),
                ));
            }
            md.push('\n');

            // Detailed findings
            md.push_str("## Detailed Findings\n\n");
            for f in &findings {
                md.push_str(&format!("### {} [{}]\n\n", f.title, f.severity));
                md.push_str(&format!("- **ID:** {}\n", f.id));
                md.push_str(&format!(
                    "- **Asset:** {} ({})\n",
                    f.affected_asset.identifier, f.affected_asset.asset_type
                ));
                if let Some(ref host) = f.affected_asset.host {
                    md.push_str(&format!("- **Host:** {}\n", host));
                }
                md.push_str(&format!("- **Type:** {}\n", f.finding_type.as_str()));
                md.push_str(&format!("- **Confidence:** {}\n", f.confidence.as_str()));
                if let Some(ref cve) = f.cve {
                    if !cve.is_empty() {
                        md.push_str(&format!("- **CVE:** {}\n", cve));
                    }
                }
                if let Some(ref cwe) = f.cwe {
                    if !cwe.is_empty() {
                        md.push_str(&format!("- **CWE:** {}\n", cwe));
                    }
                }
                if !f.tags.is_empty() {
                    md.push_str(&format!("- **Tags:** {}\n", f.tags.join(", ")));
                }
                md.push_str(&format!("- **Tool:** {}\n", f.source_tool));
                md.push_str(&format!("- **Discovered:** {}\n", f.discovered_at));
                md.push_str(&format!("\n{}\n\n", f.description));
                if let Some(ref rec) = f.remediation {
                    if !rec.is_empty() {
                        md.push_str(&format!("**Remediation:** {}\n\n", rec));
                    }
                }
            }
        }

        Ok(md)
    }

    /// Generate a CSV report.
    fn generate_csv(&self, findings: Vec<VersionedFindingPy>) -> PyResult<String> {
        let mut csv = String::new();
        csv.push_str("id,title,severity,confidence,finding_type,asset,host,cve,cwe,source_tool,discovered_at\n");

        for f in &findings {
            let cve = f.cve.as_deref().unwrap_or("");
            let cwe = f.cwe.as_deref().unwrap_or("");
            let host = f.affected_asset.host.as_deref().unwrap_or("");
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{}\n",
                csv_escape(&f.id),
                csv_escape(&f.title),
                csv_escape(&f.severity),
                csv_escape(f.confidence.as_str()),
                csv_escape(f.finding_type.as_str()),
                csv_escape(&f.affected_asset.identifier),
                csv_escape(host),
                csv_escape(cve),
                csv_escape(cwe),
                csv_escape(&f.source_tool),
                csv_escape(&f.discovered_at),
            ));
        }

        Ok(csv)
    }

    /// Generate an HTML report.
    #[pyo3(signature = (findings, title=None))]
    fn generate_html(
        &self,
        findings: Vec<VersionedFindingPy>,
        title: Option<String>,
    ) -> PyResult<String> {
        let report_title = title.unwrap_or_else(|| "Eggsec Findings Report".to_string());
        let summary = SeveritySummary::from_findings(&findings);
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str(&format!("<title>{}</title>\n", report_title));
        html.push_str("<style>\n");
        html.push_str("body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 2em; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; margin: 1em 0; }\n");
        html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
        html.push_str("th { background-color: #f5f5f5; }\n");
        html.push_str(".severity-critical { color: #dc2626; font-weight: bold; }\n");
        html.push_str(".severity-high { color: #ea580c; font-weight: bold; }\n");
        html.push_str(".severity-medium { color: #ca8a04; }\n");
        html.push_str(".severity-low { color: #2563eb; }\n");
        html.push_str(".severity-info { color: #6b7280; }\n");
        html.push_str("h1 { border-bottom: 2px solid #e5e7eb; padding-bottom: 0.5em; }\n");
        html.push_str("h2 { margin-top: 2em; }\n");
        html.push_str(".finding { margin: 1.5em 0; padding: 1em; border: 1px solid #e5e7eb; border-radius: 8px; }\n");
        html.push_str(".meta { color: #6b7280; font-size: 0.9em; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        html.push_str(&format!("<h1>{}</h1>\n", report_title));
        html.push_str(&format!(
            "<p class=\"meta\">Generated: {}</p>\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Summary
        html.push_str("<h2>Summary</h2>\n");
        html.push_str("<table>\n<tr><th>Severity</th><th>Count</th></tr>\n");
        html.push_str(&format!(
            "<tr><td class=\"severity-critical\">Critical</td><td>{}</td></tr>\n",
            summary.critical
        ));
        html.push_str(&format!(
            "<tr><td class=\"severity-high\">High</td><td>{}</td></tr>\n",
            summary.high
        ));
        html.push_str(&format!(
            "<tr><td class=\"severity-medium\">Medium</td><td>{}</td></tr>\n",
            summary.medium
        ));
        html.push_str(&format!(
            "<tr><td class=\"severity-low\">Low</td><td>{}</td></tr>\n",
            summary.low
        ));
        html.push_str(&format!(
            "<tr><td class=\"severity-info\">Info</td><td>{}</td></tr>\n",
            summary.info
        ));
        html.push_str(&format!(
            "<tr><td><strong>Total</strong></td><td><strong>{}</strong></td></tr>\n",
            summary.total
        ));
        html.push_str("</table>\n");
        html.push_str(&format!(
            "<p><strong>Risk Score:</strong> {:.1}</p>\n",
            summary.risk_score
        ));

        // Findings
        if !findings.is_empty() {
            html.push_str("<h2>Findings</h2>\n");
            for f in &findings {
                let severity_class = format!("severity-{}", f.severity.to_lowercase());
                html.push_str("<div class=\"finding\">\n");
                html.push_str(&format!(
                    "<h3><span class=\"{}\">[{}]</span> {}</h3>\n",
                    severity_class, f.severity, f.title
                ));
                html.push_str("<table>\n");
                html.push_str(&format!(
                    "<tr><td><strong>ID</strong></td><td>{}</td></tr>\n",
                    html_escape(&f.id)
                ));
                html.push_str(&format!(
                    "<tr><td><strong>Asset</strong></td><td>{} ({})</td></tr>\n",
                    html_escape(&f.affected_asset.identifier),
                    html_escape(&f.affected_asset.asset_type)
                ));
                if let Some(ref host) = f.affected_asset.host {
                    html.push_str(&format!(
                        "<tr><td><strong>Host</strong></td><td>{}</td></tr>\n",
                        html_escape(host)
                    ));
                }
                html.push_str(&format!(
                    "<tr><td><strong>Type</strong></td><td>{}</td></tr>\n",
                    html_escape(f.finding_type.as_str())
                ));
                html.push_str(&format!(
                    "<tr><td><strong>Confidence</strong></td><td>{}</td></tr>\n",
                    html_escape(f.confidence.as_str())
                ));
                if let Some(ref cve) = f.cve {
                    if !cve.is_empty() {
                        html.push_str(&format!(
                            "<tr><td><strong>CVE</strong></td><td>{}</td></tr>\n",
                            html_escape(cve)
                        ));
                    }
                }
                if let Some(ref cwe) = f.cwe {
                    if !cwe.is_empty() {
                        html.push_str(&format!(
                            "<tr><td><strong>CWE</strong></td><td>{}</td></tr>\n",
                            html_escape(cwe)
                        ));
                    }
                }
                if !f.tags.is_empty() {
                    html.push_str(&format!(
                        "<tr><td><strong>Tags</strong></td><td>{}</td></tr>\n",
                        html_escape(&f.tags.join(", "))
                    ));
                }
                html.push_str(&format!(
                    "<tr><td><strong>Tool</strong></td><td>{}</td></tr>\n",
                    html_escape(&f.source_tool)
                ));
                html.push_str(&format!(
                    "<tr><td><strong>Discovered</strong></td><td>{}</td></tr>\n",
                    html_escape(&f.discovered_at)
                ));
                html.push_str("</table>\n");
                html.push_str(&format!("<p>{}</p>\n", html_escape(&f.description)));
                if let Some(ref rec) = f.remediation {
                    if !rec.is_empty() {
                        html.push_str(&format!(
                            "<p><strong>Remediation:</strong> {}</p>\n",
                            html_escape(rec)
                        ));
                    }
                }
                html.push_str("</div>\n");
            }
        }

        html.push_str("</body>\n</html>");
        Ok(html)
    }

    /// Generate a SARIF 2.1.0 report.
    #[pyo3(signature = (findings, title=None))]
    fn generate_sarif(
        &self,
        findings: Vec<VersionedFindingPy>,
        title: Option<String>,
    ) -> PyResult<String> {
        let _report_title = title.unwrap_or_else(|| "Eggsec Findings Report".to_string());

        let mut rules: Vec<serde_json::Value> = Vec::new();
        let mut results: Vec<serde_json::Value> = Vec::new();
        let mut rule_ids_used: std::collections::HashSet<String> = std::collections::HashSet::new();

        for f in &findings {
            let rule_id = if f.cve.is_some() && !f.cve.as_ref().map_or(true, |c| c.is_empty()) {
                f.cve.clone().unwrap_or_default()
            } else {
                format!(
                    "EGGSEC-{}",
                    f.finding_type.as_str().to_uppercase().replace(' ', "-")
                )
            };

            if rule_ids_used.insert(rule_id.clone()) {
                rules.push(serde_json::json!({
                    "id": rule_id,
                    "shortDescription": {
                        "text": f.title
                    },
                    "fullDescription": {
                        "text": f.description
                    },
                    "defaultConfiguration": {
                        "level": severity_to_sarif_level(&f.severity)
                    },
                    "properties": {
                        "tags": [
                            "security",
                            f.finding_type.as_str()
                        ]
                    }
                }));
            }

            let level = severity_to_sarif_level(&f.severity);
            let mut result = serde_json::json!({
                "ruleId": rule_id,
                "ruleIndex": rules.iter().position(|r| r["id"] == rule_id).unwrap_or(0),
                "message": {
                    "text": f.description
                },
                "level": level,
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": f.affected_asset.identifier
                        },
                        "region": {
                            "startLine": 1
                        }
                    }
                }],
                "properties": {
                    "id": f.id,
                    "severity": f.severity,
                    "confidence": f.confidence.as_str(),
                    "findingType": f.finding_type.as_str(),
                    "sourceTool": f.source_tool,
                    "discoveredAt": f.discovered_at
                }
            });

            if let Some(ref cwe) = f.cwe {
                if !cwe.is_empty() {
                    if let Some(ref mut props) = result["properties"].as_object_mut() {
                        props.insert("cwe".to_string(), serde_json::Value::String(cwe.clone()));
                    }
                }
            }

            if let Some(ref mut props) = result["properties"].as_object_mut() {
                if !f.tags.is_empty() {
                    props.insert(
                        "tags".to_string(),
                        serde_json::to_value(&f.tags).unwrap_or(serde_json::Value::Null),
                    );
                }
            }

            results.push(result);
        }

        let sarif = serde_json::json!({
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "eggsec",
                        "version": env!("CARGO_PKG_VERSION"),
                        "informationUri": "https://github.com/eggsec/eggsec",
                        "rules": rules
                    }
                },
                "results": results
            }]
        });

        serde_json::to_string_pretty(&sarif)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Write the report to a file. Returns the number of bytes written.
    #[pyo3(signature = (findings, path, title=None))]
    fn write(
        &self,
        findings: Vec<VersionedFindingPy>,
        path: &str,
        title: Option<String>,
    ) -> PyResult<u64> {
        let content = self.generate(findings, title)?;
        let bytes = content.len() as u64;
        std::fs::write(path, &content)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(bytes)
    }

    /// Return the configured format name.
    fn format_name(&self) -> &str {
        &self.format
    }

    /// Return the configured redaction policy name.
    fn redaction_policy_name(&self) -> &str {
        &self.redaction_policy
    }

    /// Return whether artifacts are included in the report.
    fn includes_artifacts(&self) -> bool {
        self.include_artifacts
    }
}

/// Severity summary for reports.
#[pyclass(frozen, name = "SeveritySummary")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeveritySummaryPy {
    #[pyo3(get)]
    pub critical: u32,
    #[pyo3(get)]
    pub high: u32,
    #[pyo3(get)]
    pub medium: u32,
    #[pyo3(get)]
    pub low: u32,
    #[pyo3(get)]
    pub info: u32,
    #[pyo3(get)]
    pub total: u32,
    #[pyo3(get)]
    pub risk_score: f64,
}

#[pymethods]
impl SeveritySummaryPy {
    /// Create a summary from a list of findings.
    #[staticmethod]
    fn from_findings(findings: Vec<VersionedFindingPy>) -> Self {
        let s = SeveritySummary::from_findings(&findings);
        Self {
            critical: s.critical,
            high: s.high,
            medium: s.medium,
            low: s.low,
            info: s.info,
            total: s.total,
            risk_score: s.risk_score,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("critical", self.critical)?;
        dict.set_item("high", self.high)?;
        dict.set_item("medium", self.medium)?;
        dict.set_item("low", self.low)?;
        dict.set_item("info", self.info)?;
        dict.set_item("total", self.total)?;
        dict.set_item("risk_score", self.risk_score)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Report envelope for normalized output.
#[pyclass(frozen, name = "ReportEnvelope")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEnvelopePy {
    #[pyo3(get)]
    pub report_id: String,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub generated_at: String,
    #[pyo3(get)]
    pub schema_version: String,
    #[pyo3(get)]
    pub finding_count: u32,
    #[pyo3(get)]
    pub severity_summary: SeveritySummaryPy,
    #[pyo3(get)]
    pub findings: Vec<VersionedFindingPy>,
    #[pyo3(get)]
    pub tool_name: String,
    #[pyo3(get)]
    pub tool_version: String,
}

#[pymethods]
impl ReportEnvelopePy {
    #[new]
    #[pyo3(signature = (title, findings, *, report_id=None, tool_name=None, tool_version=None))]
    fn new(
        title: String,
        findings: Vec<VersionedFindingPy>,
        report_id: Option<String>,
        tool_name: Option<String>,
        tool_version: Option<String>,
    ) -> Self {
        let finding_count = findings.len() as u32;
        let s = SeveritySummary::from_findings(&findings);
        let severity_summary = SeveritySummaryPy {
            critical: s.critical,
            high: s.high,
            medium: s.medium,
            low: s.low,
            info: s.info,
            total: s.total,
            risk_score: s.risk_score,
        };
        Self {
            report_id: report_id
                .unwrap_or_else(|| format!("rpt-{}", chrono::Utc::now().timestamp_millis())),
            title,
            generated_at: chrono::Utc::now().to_rfc3339(),
            schema_version: "1.0.0".to_string(),
            finding_count,
            severity_summary,
            findings,
            tool_name: tool_name.unwrap_or_else(|| "eggsec".to_string()),
            tool_version: tool_version.unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("report_id", &self.report_id)?;
        dict.set_item("title", &self.title)?;
        dict.set_item("generated_at", &self.generated_at)?;
        dict.set_item("schema_version", &self.schema_version)?;
        dict.set_item("finding_count", self.finding_count)?;

        // Build severity_summary as a dict
        let ss_dict = PyDict::new_bound(py);
        ss_dict.set_item("critical", self.severity_summary.critical)?;
        ss_dict.set_item("high", self.severity_summary.high)?;
        ss_dict.set_item("medium", self.severity_summary.medium)?;
        ss_dict.set_item("low", self.severity_summary.low)?;
        ss_dict.set_item("info", self.severity_summary.info)?;
        ss_dict.set_item("total", self.severity_summary.total)?;
        ss_dict.set_item("risk_score", self.severity_summary.risk_score)?;
        dict.set_item("severity_summary", ss_dict)?;

        // Build findings as a list of dicts
        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            let item_dict = PyDict::new_bound(py);
            item_dict.set_item("id", &f.id)?;
            item_dict.set_item("title", &f.title)?;
            item_dict.set_item("severity", &f.severity)?;
            item_dict.set_item("description", &f.description)?;
            item_dict.set_item("fingerprint", &f.fingerprint)?;
            item_dict.set_item("confidence", f.confidence.as_str())?;
            item_dict.set_item("finding_type", f.finding_type.as_str())?;
            item_dict.set_item("cve", &f.cve)?;
            item_dict.set_item("cwe", &f.cwe)?;
            item_dict.set_item("tags", &f.tags)?;
            item_dict.set_item("source_tool", &f.source_tool)?;
            item_dict.set_item("source_module", &f.source_module)?;
            item_dict.set_item("discovered_at", &f.discovered_at)?;
            findings_list.append(item_dict)?;
        }
        dict.set_item("findings", findings_list)?;
        dict.set_item("tool_name", &self.tool_name)?;
        dict.set_item("tool_version", &self.tool_version)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Generate a report in the specified format using this envelope.
    fn generate_report(&self, format: &str) -> PyResult<String> {
        let reporter = FindingReporterPy::new(format.to_string(), None, None);
        reporter.generate(self.findings.clone(), Some(self.title.clone()))
    }
}

// ── Internal types (not exposed to Python) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeveritySummary {
    critical: u32,
    high: u32,
    medium: u32,
    low: u32,
    info: u32,
    total: u32,
    risk_score: f64,
}

impl SeveritySummary {
    fn from_findings(findings: &[VersionedFindingPy]) -> Self {
        let mut critical = 0u32;
        let mut high = 0u32;
        let mut medium = 0u32;
        let mut low = 0u32;
        let mut info = 0u32;

        for f in findings {
            match f.severity.to_lowercase().as_str() {
                "critical" => critical += 1,
                "high" => high += 1,
                "medium" => medium += 1,
                "low" => low += 1,
                "info" | "informational" => info += 1,
                _ => {}
            }
        }

        let total = critical + high + medium + low + info;
        let risk_score = if total > 0 {
            let weighted = critical as f64 * 10.0
                + high as f64 * 7.5
                + medium as f64 * 5.0
                + low as f64 * 2.5
                + info as f64 * 0.5;
            (weighted / total as f64).min(10.0)
        } else {
            0.0
        };

        Self {
            critical,
            high,
            medium,
            low,
            info,
            total,
            risk_score,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InternalReportEnvelope {
    title: String,
    generated_at: String,
    finding_count: u32,
    severity_summary: SeveritySummary,
    findings: Vec<VersionedFindingPy>,
}

// ── Utility functions ──

fn severity_to_sarif_level(severity: &str) -> &'static str {
    match severity.to_lowercase().as_str() {
        "critical" | "high" => "error",
        "medium" => "warning",
        "low" => "note",
        "info" | "informational" => "none",
        _ => "warning",
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
