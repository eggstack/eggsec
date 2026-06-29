//! Baseline capture and regression comparison for db-pentest (Phase 6).
//!
//! Captures a snapshot of a `DbPentestReport` as a baseline, then compares
//! subsequent reports against it to identify regressions (new findings,
//! severity increases) and improvements (resolved findings).

use crate::types::{DbFinding, DbPentestReport};
use eggsec_core::types::Severity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A captured baseline snapshot for regression comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbBaseline {
    /// Timestamp when the baseline was captured.
    pub captured_at: String,
    /// The db_type this baseline applies to.
    pub db_type: String,
    /// The scan profile / checks that produced the baseline.
    pub checks: String,
    /// Summary of findings at baseline time.
    pub finding_categories: Vec<String>,
    /// Severity counts at baseline time.
    pub severity_counts: HashMap<String, usize>,
    /// Total findings count.
    pub total_findings: usize,
    /// The full report data for detailed comparison.
    pub report: DbPentestReport,
    /// Optional human-readable label for this baseline.
    pub label: Option<String>,
}

/// Result of comparing a new report against a baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbRegressionResult {
    /// Whether the baseline was provided.
    pub has_baseline: bool,
    /// New findings not present in the baseline.
    pub new_findings: Vec<DbFinding>,
    /// Findings that were in the baseline but are now resolved/absent.
    pub resolved_findings: Vec<DbFinding>,
    /// Findings where severity increased.
    pub severity_increases: Vec<SeverityChange>,
    /// Findings where severity decreased (improvements).
    pub severity_decreases: Vec<SeverityChange>,
    /// Human-readable summary.
    pub summary: String,
    /// Whether this is a regression (any new findings or severity increases).
    pub is_regression: bool,
    /// Whether this shows improvement (resolved findings or severity decreases).
    pub is_improvement: bool,
}

/// A severity change between baseline and current scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityChange {
    pub category: String,
    pub from: Severity,
    pub to: Severity,
}

/// Capture a baseline from a `DbPentestReport`.
pub fn capture_baseline(report: &DbPentestReport, label: Option<&str>) -> DbBaseline {
    let mut severity_counts: HashMap<String, usize> = HashMap::new();
    for finding in &report.findings {
        *severity_counts
            .entry(format!("{:?}", finding.severity))
            .or_insert(0) += 1;
    }

    DbBaseline {
        captured_at: report.timestamp.clone(),
        db_type: report.db_type.clone(),
        checks: report.scan_type.clone(),
        finding_categories: report.findings.iter().map(|f| f.category.clone()).collect(),
        severity_counts,
        total_findings: report.findings.len(),
        report: report.clone(),
        label: label.map(|s| s.to_string()),
    }
}

/// Compare a new report against a baseline to detect regressions and improvements.
pub fn compare_to_baseline(baseline: &DbBaseline, current: &DbPentestReport) -> DbRegressionResult {
    let baseline_cats: std::collections::HashSet<&str> = baseline
        .finding_categories
        .iter()
        .map(|s| s.as_str())
        .collect();
    let current_cats: std::collections::HashSet<&str> = current
        .findings
        .iter()
        .map(|f| f.category.as_str())
        .collect();

    // New findings: categories in current but not in baseline
    let new_findings: Vec<DbFinding> = current
        .findings
        .iter()
        .filter(|f| !baseline_cats.contains(f.category.as_str()))
        .cloned()
        .collect();

    // Resolved findings: categories in baseline but not in current
    let resolved_findings: Vec<DbFinding> = baseline
        .report
        .findings
        .iter()
        .filter(|f| !current_cats.contains(f.category.as_str()))
        .cloned()
        .collect();

    // Severity changes: same category, different severity
    let mut severity_increases = Vec::new();
    let mut severity_decreases = Vec::new();

    let baseline_map: HashMap<&str, &Severity> = baseline
        .report
        .findings
        .iter()
        .map(|f| (f.category.as_str(), &f.severity))
        .collect();

    for finding in &current.findings {
        if let Some(&baseline_sev) = baseline_map.get(finding.category.as_str()) {
            let current_sev = &finding.severity;
            if severity_rank(current_sev) > severity_rank(baseline_sev) {
                severity_increases.push(SeverityChange {
                    category: finding.category.clone(),
                    from: *baseline_sev,
                    to: *current_sev,
                });
            } else if severity_rank(current_sev) < severity_rank(baseline_sev) {
                severity_decreases.push(SeverityChange {
                    category: finding.category.clone(),
                    from: *baseline_sev,
                    to: *current_sev,
                });
            }
        }
    }

    let is_regression = !new_findings.is_empty() || !severity_increases.is_empty();
    let is_improvement = !resolved_findings.is_empty() || !severity_decreases.is_empty();

    let summary = if is_regression && is_improvement {
        format!(
            "Mixed: {} new finding(s), {} resolved, {} severity increase(s), {} severity decrease(s)",
            new_findings.len(),
            resolved_findings.len(),
            severity_increases.len(),
            severity_decreases.len()
        )
    } else if is_regression {
        format!(
            "Regression: {} new finding(s), {} severity increase(s)",
            new_findings.len(),
            severity_increases.len()
        )
    } else if is_improvement {
        format!(
            "Improvement: {} resolved finding(s), {} severity decrease(s)",
            resolved_findings.len(),
            severity_decreases.len()
        )
    } else {
        "No regression detected — findings match baseline".to_string()
    };

    DbRegressionResult {
        has_baseline: true,
        new_findings,
        resolved_findings,
        severity_increases,
        severity_decreases,
        summary,
        is_regression,
        is_improvement,
    }
}

fn severity_rank(sev: &Severity) -> u8 {
    match sev {
        Severity::Info => 0,
        Severity::Low => 1,
        Severity::Medium => 2,
        Severity::High => 3,
        Severity::Critical => 4,
    }
}

/// Format a regression result for human-readable output.
pub fn format_regression_report(
    result: &DbRegressionResult,
    baseline_label: Option<&str>,
) -> String {
    let mut out = String::new();
    let label = baseline_label.unwrap_or("baseline");
    out.push_str(&format!("=== Regression Report (vs. {}) ===\n", label));
    out.push_str(&format!("{}\n", result.summary));

    if !result.new_findings.is_empty() {
        out.push_str("\nNew Findings (not in baseline):\n");
        for f in &result.new_findings {
            out.push_str(&format!(
                "  [{}] {} ({})\n",
                f.severity, f.title, f.category
            ));
        }
    }
    if !result.resolved_findings.is_empty() {
        out.push_str("\nResolved Findings (were in baseline):\n");
        for f in &result.resolved_findings {
            out.push_str(&format!(
                "  [{}] {} ({})\n",
                f.severity, f.title, f.category
            ));
        }
    }
    if !result.severity_increases.is_empty() {
        out.push_str("\nSeverity Increases:\n");
        for c in &result.severity_increases {
            out.push_str(&format!("  {} : {:?} -> {:?}\n", c.category, c.from, c.to));
        }
    }
    if !result.severity_decreases.is_empty() {
        out.push_str("\nSeverity Decreases (improvements):\n");
        for c in &result.severity_decreases {
            out.push_str(&format!("  {} : {:?} -> {:?}\n", c.category, c.from, c.to));
        }
    }
    out.push_str("\nNOTE: This is a lab-only defensive validation tool. Use only on systems you own and are authorized to test.\n");
    out
}

/// Export a baseline as JSON for storage/loading.
pub fn export_baseline_json(baseline: &DbBaseline) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(baseline)
}

/// Import a baseline from JSON.
pub fn import_baseline_json(json: &str) -> Result<DbBaseline, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_finding(category: &str, severity: Severity) -> DbFinding {
        DbFinding {
            category: category.to_string(),
            severity,
            title: format!("Finding: {}", category),
            description: "test".to_string(),
            recommendation: "test".to_string(),
            evidence: None,
            db_type: "postgres".to_string(),
            target_host: "h".to_string(),
        }
    }

    fn make_report(categories: Vec<(&str, Severity)>) -> DbPentestReport {
        let mut report = DbPentestReport::new("test", "postgres");
        for (cat, sev) in categories {
            report.findings.push(make_finding(cat, sev));
        }
        report
    }

    #[test]
    fn capture_baseline_snapshot() {
        let report = make_report(vec![
            ("db-postgres-priv-excessive", Severity::Medium),
            ("db-postgres-misconfig-logging", Severity::Low),
        ]);
        let baseline = capture_baseline(&report, Some("v1"));
        assert_eq!(baseline.total_findings, 2);
        assert_eq!(baseline.db_type, "postgres");
        assert_eq!(baseline.label.as_deref(), Some("v1"));
        assert!(baseline
            .finding_categories
            .contains(&"db-postgres-priv-excessive".to_string()));
    }

    #[test]
    fn compare_no_change() {
        let report = make_report(vec![("db-postgres-priv-excessive", Severity::Medium)]);
        let baseline = capture_baseline(&report, None);
        let result = compare_to_baseline(&baseline, &report);
        assert!(!result.is_regression);
        assert!(!result.is_improvement);
        assert!(result.new_findings.is_empty());
        assert!(result.resolved_findings.is_empty());
    }

    #[test]
    fn compare_detects_new_finding() {
        let baseline_report = make_report(vec![("db-postgres-priv-excessive", Severity::Medium)]);
        let current_report = make_report(vec![
            ("db-postgres-priv-excessive", Severity::Medium),
            ("db-postgres-misconfig-dangerous-extension", Severity::High),
        ]);
        let baseline = capture_baseline(&baseline_report, None);
        let result = compare_to_baseline(&baseline, &current_report);
        assert!(result.is_regression);
        assert_eq!(result.new_findings.len(), 1);
        assert_eq!(
            result.new_findings[0].category,
            "db-postgres-misconfig-dangerous-extension"
        );
    }

    #[test]
    fn compare_detects_resolved_finding() {
        let baseline_report = make_report(vec![
            ("db-postgres-priv-excessive", Severity::Medium),
            ("db-postgres-misconfig-logging", Severity::Low),
        ]);
        let current_report = make_report(vec![("db-postgres-priv-excessive", Severity::Medium)]);
        let baseline = capture_baseline(&baseline_report, None);
        let result = compare_to_baseline(&baseline, &current_report);
        assert!(!result.is_regression);
        assert!(result.is_improvement);
        assert_eq!(result.resolved_findings.len(), 1);
        assert_eq!(
            result.resolved_findings[0].category,
            "db-postgres-misconfig-logging"
        );
    }

    #[test]
    fn compare_detects_severity_increase() {
        let baseline_report = make_report(vec![("db-postgres-priv-excessive", Severity::Low)]);
        let current_report = make_report(vec![("db-postgres-priv-excessive", Severity::High)]);
        let baseline = capture_baseline(&baseline_report, None);
        let result = compare_to_baseline(&baseline, &current_report);
        assert!(result.is_regression);
        assert_eq!(result.severity_increases.len(), 1);
        assert_eq!(result.severity_increases[0].from, Severity::Low);
        assert_eq!(result.severity_increases[0].to, Severity::High);
    }

    #[test]
    fn compare_detects_severity_decrease() {
        let baseline_report = make_report(vec![("db-postgres-priv-excessive", Severity::High)]);
        let current_report = make_report(vec![("db-postgres-priv-excessive", Severity::Low)]);
        let baseline = capture_baseline(&baseline_report, None);
        let result = compare_to_baseline(&baseline, &current_report);
        assert!(!result.is_regression);
        assert!(result.is_improvement);
        assert_eq!(result.severity_decreases.len(), 1);
    }

    #[test]
    fn compare_mixed_results() {
        let baseline_report = make_report(vec![
            ("db-postgres-priv-excessive", Severity::Medium),
            ("db-postgres-misconfig-logging", Severity::Low),
        ]);
        let current_report = make_report(vec![
            ("db-postgres-priv-excessive", Severity::High), // severity increase
            ("db-oracle-version", Severity::Info),          // new finding
        ]);
        let baseline = capture_baseline(&baseline_report, None);
        let result = compare_to_baseline(&baseline, &current_report);
        assert!(result.is_regression);
        assert!(result.is_improvement); // logging resolved
        assert_eq!(result.new_findings.len(), 1);
        assert_eq!(result.resolved_findings.len(), 1);
        assert_eq!(result.severity_increases.len(), 1);
    }

    #[test]
    fn baseline_serialization_roundtrip() {
        let report = make_report(vec![("db-postgres-priv-excessive", Severity::Medium)]);
        let baseline = capture_baseline(&report, Some("test-label"));
        let json = export_baseline_json(&baseline).unwrap();
        let imported = import_baseline_json(&json).unwrap();
        assert_eq!(imported.total_findings, baseline.total_findings);
        assert_eq!(imported.label, baseline.label);
        assert_eq!(imported.db_type, baseline.db_type);
    }

    #[test]
    fn format_regression_report_output() {
        let baseline_report = make_report(vec![("db-postgres-priv-excessive", Severity::Medium)]);
        let current_report = make_report(vec![("db-postgres-priv-excessive", Severity::High)]);
        let baseline = capture_baseline(&baseline_report, Some("v1"));
        let result = compare_to_baseline(&baseline, &current_report);
        let output = format_regression_report(&result, Some("v1"));
        assert!(output.contains("Regression Report"));
        assert!(output.contains("v1"));
        assert!(output.contains("severity increase"));
    }

    #[test]
    fn compare_empty_baseline_and_report() {
        let baseline_report = DbPentestReport::new("test", "postgres");
        let current_report = DbPentestReport::new("test", "postgres");
        let baseline = capture_baseline(&baseline_report, None);
        let result = compare_to_baseline(&baseline, &current_report);
        assert!(!result.is_regression);
        assert!(!result.is_improvement);
    }
}
