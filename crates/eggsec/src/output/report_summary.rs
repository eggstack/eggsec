use crate::findings::Finding;
use crate::types::Severity;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

/// Structured report summary with aggregated statistics and risk narrative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_findings: usize,
    pub by_severity: FxHashMap<String, usize>,
    pub by_confidence: FxHashMap<String, usize>,
    pub by_type: FxHashMap<String, usize>,
    pub top_affected_assets: Vec<AssetCount>,
    pub risk_narrative: String,
    pub remediation_summary: Vec<String>,
}

/// An asset and the number of findings affecting it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetCount {
    pub asset: String,
    pub count: usize,
}

impl ReportSummary {
    /// Build a summary from a slice of findings.
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut by_severity: FxHashMap<String, usize> = FxHashMap::default();
        let mut by_confidence: FxHashMap<String, usize> = FxHashMap::default();
        let mut by_type: FxHashMap<String, usize> = FxHashMap::default();
        let mut asset_counts: FxHashMap<String, usize> = FxHashMap::default();
        let mut seen_remediations: FxHashSet<String> = FxHashSet::default();
        let mut remediations: Vec<String> = Vec::new();

        for finding in findings {
            let sev = finding.severity.as_str().to_string();
            *by_severity.entry(sev).or_insert(0) += 1;

            let conf = format!("{:?}", finding.confidence);
            *by_confidence.entry(conf).or_insert(0) += 1;

            let ftype = format!("{:?}", finding.finding_type);
            *by_type.entry(ftype).or_insert(0) += 1;

            *asset_counts
                .entry(finding.affected_asset.identifier.clone())
                .or_insert(0) += 1;

            if let Some(ref remediation) = finding.remediation {
                if seen_remediations.insert(remediation.clone()) {
                    remediations.push(remediation.clone());
                }
            }
        }

        let mut top_affected_assets: Vec<AssetCount> = asset_counts
            .into_iter()
            .map(|(asset, count)| AssetCount { asset, count })
            .collect();
        top_affected_assets.sort_by_key(|b| std::cmp::Reverse(b.count));
        top_affected_assets.truncate(10);

        let risk_narrative = generate_risk_narrative(findings);

        Self {
            total_findings: findings.len(),
            by_severity,
            by_confidence,
            by_type,
            top_affected_assets,
            risk_narrative,
            remediation_summary: remediations,
        }
    }
}

fn generate_risk_narrative(findings: &[Finding]) -> String {
    let critical = findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .count();
    let high = findings
        .iter()
        .filter(|f| f.severity == Severity::High)
        .count();
    let medium = findings
        .iter()
        .filter(|f| f.severity == Severity::Medium)
        .count();
    let low = findings
        .iter()
        .filter(|f| f.severity == Severity::Low)
        .count();

    let mut narrative = String::new();

    if critical > 0 {
        narrative.push_str(&format!(
            "CRITICAL: {} critical severity findings require immediate attention. ",
            critical
        ));
    }
    if high > 0 {
        narrative.push_str(&format!(
            "HIGH: {} high severity findings should be addressed promptly. ",
            high
        ));
    }
    if medium > 0 {
        narrative.push_str(&format!(
            "MEDIUM: {} medium severity findings represent moderate risk. ",
            medium
        ));
    }
    if low > 0 {
        narrative.push_str(&format!(
            "LOW: {} low severity findings are informational or low risk. ",
            low
        ));
    }

    if narrative.is_empty() {
        "No findings detected.".to_string()
    } else {
        narrative
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::*;
    use chrono::Utc;

    fn test_finding(severity: Severity, title: &str) -> Finding {
        Finding {
            id: format!("test-{}", title),
            fingerprint: format!("fp-{}", title),
            title: title.to_string(),
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
            remediation: Some("Fix the issue".to_string()),
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
    fn summary_counts_correctly() {
        let findings = vec![
            test_finding(Severity::Critical, "crit1"),
            test_finding(Severity::High, "high1"),
            test_finding(Severity::High, "high2"),
            test_finding(Severity::Medium, "med1"),
        ];

        let summary = ReportSummary::from_findings(&findings);
        assert_eq!(summary.total_findings, 4);
        assert_eq!(summary.by_severity.get("critical").unwrap(), &1);
        assert_eq!(summary.by_severity.get("high").unwrap(), &2);
    }

    #[test]
    fn risk_narrative_includes_critical() {
        let findings = vec![test_finding(Severity::Critical, "crit1")];
        let summary = ReportSummary::from_findings(&findings);
        assert!(summary.risk_narrative.contains("CRITICAL"));
    }

    #[test]
    fn risk_narrative_includes_high() {
        let findings = vec![
            test_finding(Severity::Critical, "crit1"),
            test_finding(Severity::High, "high1"),
        ];
        let summary = ReportSummary::from_findings(&findings);
        assert!(summary.risk_narrative.contains("HIGH"));
    }

    #[test]
    fn risk_narrative_includes_medium_and_low() {
        let findings = vec![
            test_finding(Severity::Medium, "med1"),
            test_finding(Severity::Low, "low1"),
        ];
        let summary = ReportSummary::from_findings(&findings);
        assert!(summary.risk_narrative.contains("MEDIUM"));
        assert!(summary.risk_narrative.contains("LOW"));
    }

    #[test]
    fn no_findings_returns_no_findings_message() {
        let findings: Vec<Finding> = vec![];
        let summary = ReportSummary::from_findings(&findings);
        assert_eq!(summary.risk_narrative, "No findings detected.");
    }

    #[test]
    fn confidence_counts_correctly() {
        let mut findings = vec![test_finding(Severity::Medium, "med1")];
        findings[0].confidence = Confidence::Confirmed;

        let summary = ReportSummary::from_findings(&findings);
        assert_eq!(summary.by_confidence.get("Confirmed").unwrap(), &1);
    }

    #[test]
    fn finding_type_counts_correctly() {
        let mut findings = vec![test_finding(Severity::Medium, "med1")];
        findings[0].finding_type = FindingType::Misconfiguration;

        let summary = ReportSummary::from_findings(&findings);
        assert_eq!(summary.by_type.get("Misconfiguration").unwrap(), &1);
    }

    #[test]
    fn top_affected_assets_sorted_by_count() {
        let mut findings = vec![];
        for i in 0..5 {
            let mut f = test_finding(Severity::Low, &format!("low{}", i));
            f.affected_asset.identifier = "https://a.com".to_string();
            findings.push(f);
        }
        let mut f = test_finding(Severity::Low, "single");
        f.affected_asset.identifier = "https://b.com".to_string();
        findings.push(f);

        let summary = ReportSummary::from_findings(&findings);
        assert_eq!(summary.top_affected_assets[0].asset, "https://a.com");
        assert_eq!(summary.top_affected_assets[0].count, 5);
        assert_eq!(summary.top_affected_assets[1].asset, "https://b.com");
        assert_eq!(summary.top_affected_assets[1].count, 1);
    }

    #[test]
    fn remediation_summary_deduplicates() {
        let mut findings = vec![
            test_finding(Severity::High, "h1"),
            test_finding(Severity::High, "h2"),
        ];
        findings[0].remediation = Some("Fix XSS".to_string());
        findings[1].remediation = Some("Fix XSS".to_string());

        let summary = ReportSummary::from_findings(&findings);
        assert_eq!(summary.remediation_summary.len(), 1);
        assert_eq!(summary.remediation_summary[0], "Fix XSS");
    }

    #[test]
    fn summary_serializes() {
        let findings = vec![test_finding(Severity::Critical, "crit1")];
        let summary = ReportSummary::from_findings(&findings);
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("total_findings"));
        assert!(json.contains("risk_narrative"));
    }
}
