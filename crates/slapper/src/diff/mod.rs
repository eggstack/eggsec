use crate::findings::Evidence;
use crate::findings::Finding;
use anyhow::Result;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of comparing two scans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub new: Vec<Finding>,
    pub resolved: Vec<Finding>,
    pub persisting: Vec<Finding>,
    pub changed: Vec<FindingChange>,
    pub summary: DiffSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingChange {
    pub fingerprint: String,
    pub title: String,
    pub old_severity: crate::types::Severity,
    pub new_severity: crate::types::Severity,
    pub old_confidence: crate::findings::Confidence,
    pub new_confidence: crate::findings::Confidence,
    pub evidence_changed: bool,
    pub old_evidence: Option<Vec<Evidence>>,
    pub new_evidence: Option<Vec<Evidence>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub new_count: usize,
    pub resolved_count: usize,
    pub persisting_count: usize,
    pub changed_count: usize,
    pub old_total: usize,
    pub new_total: usize,
}

/// Compare two sets of findings and produce a diff result
pub fn diff_findings(old_findings: &[Finding], new_findings: &[Finding]) -> DiffResult {
    let old_map: FxHashMap<&str, &Finding> = old_findings
        .iter()
        .map(|f| (f.fingerprint.as_str(), f))
        .collect();

    let new_map: FxHashMap<&str, &Finding> = new_findings
        .iter()
        .map(|f| (f.fingerprint.as_str(), f))
        .collect();

    let mut new_findings_result = Vec::new();
    let mut resolved = Vec::new();
    let mut persisting = Vec::new();
    let mut changed = Vec::new();

    for (fp, new_f) in &new_map {
        if let Some(old_f) = old_map.get(fp) {
            if old_f.severity != new_f.severity
                || old_f.confidence != new_f.confidence
                || old_f.evidence.len() != new_f.evidence.len()
            {
                changed.push(FindingChange {
                    fingerprint: fp.to_string(),
                    title: new_f.title.clone(),
                    old_severity: old_f.severity,
                    new_severity: new_f.severity,
                    old_confidence: old_f.confidence,
                    new_confidence: new_f.confidence,
                    evidence_changed: old_f.evidence.len() != new_f.evidence.len(),
                    old_evidence: Some(old_f.evidence.clone()),
                    new_evidence: Some(new_f.evidence.clone()),
                });
            } else {
                persisting.push((*new_f).clone());
            }
        } else {
            new_findings_result.push((*new_f).clone());
        }
    }

    for (fp, old_f) in &old_map {
        if !new_map.contains_key(fp) {
            resolved.push((*old_f).clone());
        }
    }

    let summary = DiffSummary {
        new_count: new_findings_result.len(),
        resolved_count: resolved.len(),
        persisting_count: persisting.len(),
        changed_count: changed.len(),
        old_total: old_findings.len(),
        new_total: new_findings.len(),
    };

    DiffResult {
        new: new_findings_result,
        resolved,
        persisting,
        changed,
        summary,
    }
}

/// Load findings from a JSON file
pub fn load_findings_from_file(path: &Path) -> Result<Vec<Finding>> {
    let content = std::fs::read_to_string(path)?;
    let findings: Vec<Finding> = serde_json::from_str(&content)?;
    Ok(findings)
}

/// Format diff as human-readable text
pub fn format_diff_text(diff: &DiffResult) -> String {
    let mut out = String::new();

    out.push_str("=== Scan Diff Report ===\n\n");
    out.push_str(&format!("Old scan: {} findings\n", diff.summary.old_total));
    out.push_str(&format!(
        "New scan: {} findings\n\n",
        diff.summary.new_total
    ));

    if !diff.new.is_empty() {
        out.push_str(&format!("--- New Findings ({}) ---\n", diff.new.len()));
        for f in &diff.new {
            out.push_str(&format!(
                "  [{}] {} ({})\n",
                f.severity, f.title, f.confidence
            ));
        }
        out.push('\n');
    }

    if !diff.resolved.is_empty() {
        out.push_str(&format!(
            "--- Resolved Findings ({}) ---\n",
            diff.resolved.len()
        ));
        for f in &diff.resolved {
            out.push_str(&format!(
                "  [{}] {} ({})\n",
                f.severity, f.title, f.confidence
            ));
        }
        out.push('\n');
    }

    if !diff.changed.is_empty() {
        out.push_str(&format!(
            "--- Changed Findings ({}) ---\n",
            diff.changed.len()
        ));
        for c in &diff.changed {
            out.push_str(&format!(
                "  {} ({}): {} -> {}\n",
                c.title, c.fingerprint, c.old_severity, c.new_severity
            ));
        }
        out.push('\n');
    }

    if !diff.persisting.is_empty() {
        out.push_str(&format!(
            "--- Persisting Findings ({}) ---\n",
            diff.persisting.len()
        ));
        for f in &diff.persisting {
            out.push_str(&format!(
                "  [{}] {} ({})\n",
                f.severity, f.title, f.confidence
            ));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::*;
    use chrono::Utc;

    fn make_finding(fingerprint: &str, severity: crate::types::Severity) -> Finding {
        Finding {
            id: format!("test-{fingerprint}"),
            fingerprint: fingerprint.to_string(),
            title: format!("Finding {fingerprint}"),
            description: "Test".to_string(),
            severity,
            confidence: Confidence::High,
            finding_type: FindingType::Vulnerability,
            cwe: None,
            owasp: None,
            cve: None,
            affected_asset: AffectedAsset {
                asset_type: "web_application".to_string(),
                identifier: "https://example.com".to_string(),
                host: Some("example.com".to_string()),
                port: Some(443),
                protocol: Some("https".to_string()),
            },
            location: FindingLocation {
                url: None,
                path: None,
                parameter: None,
                header: None,
                method: None,
                line: None,
                file: None,
            },
            evidence: vec![],
            reproduction: None,
            remediation: None,
            discovered_at: Utc::now(),
            source: FindingSource {
                tool: "test".to_string(),
                module: "test".to_string(),
                run_id: None,
            },
            tags: vec![],
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn diff_finds_new_findings() {
        let old = vec![make_finding("fp1", crate::types::Severity::Low)];
        let new = vec![
            make_finding("fp1", crate::types::Severity::Low),
            make_finding("fp2", crate::types::Severity::High),
        ];
        let diff = diff_findings(&old, &new);
        assert_eq!(diff.summary.new_count, 1);
        assert_eq!(diff.summary.persisting_count, 1);
        assert_eq!(diff.new[0].fingerprint, "fp2");
    }

    #[test]
    fn diff_finds_resolved_findings() {
        let old = vec![
            make_finding("fp1", crate::types::Severity::Low),
            make_finding("fp2", crate::types::Severity::High),
        ];
        let new = vec![make_finding("fp1", crate::types::Severity::Low)];
        let diff = diff_findings(&old, &new);
        assert_eq!(diff.summary.resolved_count, 1);
        assert_eq!(diff.resolved[0].fingerprint, "fp2");
    }

    #[test]
    fn diff_finds_severity_changes() {
        let old = vec![make_finding("fp1", crate::types::Severity::Low)];
        let new = vec![make_finding("fp1", crate::types::Severity::Critical)];
        let diff = diff_findings(&old, &new);
        assert_eq!(diff.summary.changed_count, 1);
        assert_eq!(diff.changed[0].old_severity, crate::types::Severity::Low);
        assert_eq!(
            diff.changed[0].new_severity,
            crate::types::Severity::Critical
        );
    }

    #[test]
    fn diff_deterministic() {
        let old = vec![
            make_finding("fp1", crate::types::Severity::Low),
            make_finding("fp2", crate::types::Severity::High),
        ];
        let new = vec![
            make_finding("fp1", crate::types::Severity::Medium),
            make_finding("fp3", crate::types::Severity::Critical),
        ];
        let diff1 = diff_findings(&old, &new);
        let diff2 = diff_findings(&old, &new);
        assert_eq!(diff1.summary.new_count, diff2.summary.new_count);
        assert_eq!(diff1.summary.resolved_count, diff2.summary.resolved_count);
        assert_eq!(diff1.summary.changed_count, diff2.summary.changed_count);
    }

    #[test]
    fn format_diff_text_works() {
        let old = vec![make_finding("fp1", crate::types::Severity::Low)];
        let new = vec![
            make_finding("fp1", crate::types::Severity::Low),
            make_finding("fp2", crate::types::Severity::High),
        ];
        let diff = diff_findings(&old, &new);
        let text = format_diff_text(&diff);
        assert!(text.contains("New Findings"));
        assert!(text.contains("fp2"));
    }

    #[test]
    fn diff_empty_old() {
        let old: Vec<Finding> = vec![];
        let new = vec![make_finding("fp1", crate::types::Severity::High)];
        let diff = diff_findings(&old, &new);
        assert_eq!(diff.summary.new_count, 1);
        assert_eq!(diff.summary.resolved_count, 0);
        assert_eq!(diff.summary.old_total, 0);
    }

    #[test]
    fn diff_empty_new() {
        let old = vec![make_finding("fp1", crate::types::Severity::High)];
        let new: Vec<Finding> = vec![];
        let diff = diff_findings(&old, &new);
        assert_eq!(diff.summary.new_count, 0);
        assert_eq!(diff.summary.resolved_count, 1);
    }

    #[test]
    fn diff_both_empty() {
        let old: Vec<Finding> = vec![];
        let new: Vec<Finding> = vec![];
        let diff = diff_findings(&old, &new);
        assert_eq!(diff.summary.new_count, 0);
        assert_eq!(diff.summary.resolved_count, 0);
        assert_eq!(diff.summary.persisting_count, 0);
        assert_eq!(diff.summary.changed_count, 0);
    }

    #[test]
    fn diff_evidence_change_detected() {
        let mut old = make_finding("fp1", crate::types::Severity::Low);
        old.evidence.push(Evidence::new(
            EvidenceKind::HttpResponse,
            "200 OK",
            serde_json::json!({"status": 200}),
        ));
        let mut new = make_finding("fp1", crate::types::Severity::Low);
        new.evidence.push(Evidence::new(
            EvidenceKind::HttpResponse,
            "200 OK",
            serde_json::json!({"status": 200}),
        ));
        new.evidence.push(Evidence::new(
            EvidenceKind::Header,
            "Server header",
            serde_json::json!({"header": "Server: nginx"}),
        ));
        let diff = diff_findings(&[old], &[new]);
        assert_eq!(diff.summary.changed_count, 1);
        assert!(diff.changed[0].evidence_changed);
    }

    #[test]
    fn format_diff_resolved_section() {
        let old = vec![
            make_finding("fp1", crate::types::Severity::Low),
            make_finding("fp2", crate::types::Severity::High),
        ];
        let new = vec![make_finding("fp1", crate::types::Severity::Low)];
        let diff = diff_findings(&old, &new);
        let text = format_diff_text(&diff);
        assert!(text.contains("Resolved Findings"));
        assert!(text.contains("fp2"));
    }

    #[test]
    fn format_diff_changed_section() {
        let old = vec![make_finding("fp1", crate::types::Severity::Low)];
        let new = vec![make_finding("fp1", crate::types::Severity::Critical)];
        let diff = diff_findings(&old, &new);
        let text = format_diff_text(&diff);
        assert!(text.contains("Changed Findings"));
        assert!(text.contains("LOW -> CRITICAL"));
    }
}
