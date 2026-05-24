use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use lru::LruCache;

const DEFAULT_MAX_HISTORY: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub id: String,
    pub target: String,
    pub scan_type: String,
    pub timestamp: String,
    pub summary: ResultSummary,
    pub details: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultSummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub scan_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub severity: Severity,
    pub category: String,
    pub title: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub remediation: Option<String>,
    pub cve: Option<String>,
}

impl From<&crate::output::AgentFinding> for Finding {
    fn from(f: &crate::output::AgentFinding) -> Self {
        Self {
            severity: f.severity,
            category: f.vulnerability_type.clone(),
            title: f.title.clone(),
            description: f.description.clone(),
            evidence: f.evidence.request.iter().cloned().collect(),
            remediation: Some(f.remediation.summary.clone()),
            cve: f.cwe_ids.first().cloned(),
        }
    }
}

pub use crate::types::Severity;

pub struct ResultComparator;

impl ResultComparator {
    fn finding_key(finding: &Finding) -> (String, String, String) {
        (
            finding.title.clone(),
            finding.category.clone(),
            finding.cve.clone().unwrap_or_default(),
        )
    }

    pub fn compare(old: &ScanResult, new: &ScanResult) -> ComparisonResult {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut unchanged = Vec::new();

        let old_findings: FxHashMap<_, _> = old
            .details
            .iter()
            .map(|f| (Self::finding_key(f), f.clone()))
            .collect();
        let new_findings: FxHashMap<_, _> = new
            .details
            .iter()
            .map(|f| (Self::finding_key(f), f.clone()))
            .collect();

        for (title, finding) in &new_findings {
            if !old_findings.contains_key(title) {
                added.push(finding.clone());
            } else {
                unchanged.push(finding.clone());
            }
        }

        for (title, finding) in &old_findings {
            if !new_findings.contains_key(title) {
                removed.push(finding.clone());
            }
        }

        let severity_change = (
            new.summary.critical as i32 - old.summary.critical as i32,
            new.summary.high as i32 - old.summary.high as i32,
            new.summary.medium as i32 - old.summary.medium as i32,
        );

        ComparisonResult {
            added,
            removed,
            unchanged,
            severity_change,
            old_timestamp: old.timestamp.clone(),
            new_timestamp: new.timestamp.clone(),
        }
    }

    pub fn risk_trend(changes: &[i32]) -> TrendDirection {
        if changes.is_empty() {
            return TrendDirection::Stable;
        }

        let sum: i32 = changes.iter().sum();
        if sum > 0 {
            TrendDirection::Worsening
        } else if sum < 0 {
            TrendDirection::Improving
        } else {
            TrendDirection::Stable
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub added: Vec<Finding>,
    pub removed: Vec<Finding>,
    pub unchanged: Vec<Finding>,
    pub severity_change: (i32, i32, i32),
    pub old_timestamp: String,
    pub new_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Worsening,
}

pub struct TrendAnalyzer {
    results: LruCache<String, ScanResult>,
}

impl TrendAnalyzer {
    pub fn new() -> Self {
        Self {
            results: LruCache::new(std::num::NonZeroUsize::new(DEFAULT_MAX_HISTORY).unwrap()),
        }
    }

    pub fn add_result(&mut self, result: ScanResult) {
        self.results.put(result.id.clone(), result);
    }

    pub fn get_trend(&self) -> TrendAnalysis {
        if self.results.len() < 2 {
            return TrendAnalysis {
                direction: TrendDirection::Stable,
                critical_trend: Vec::new(),
                high_trend: Vec::new(),
                medium_trend: Vec::new(),
                average_scan_time_ms: 0,
            };
        }

        let mut sorted_results: Vec<_> = self.results.iter().collect();
        sorted_results.sort_by(|a, b| a.1.timestamp.cmp(&b.1.timestamp));

        let critical_trend: Vec<i32> = sorted_results
            .windows(2)
            .map(|w| w[1].1.summary.critical as i32 - w[0].1.summary.critical as i32)
            .collect();

        let high_trend: Vec<i32> = sorted_results
            .windows(2)
            .map(|w| w[1].1.summary.high as i32 - w[0].1.summary.high as i32)
            .collect();

        let medium_trend: Vec<i32> = sorted_results
            .windows(2)
            .map(|w| w[1].1.summary.medium as i32 - w[0].1.summary.medium as i32)
            .collect();

        let total_duration: u64 = sorted_results
            .iter()
            .map(|r| r.1.summary.scan_duration_ms)
            .sum();
        let average_scan_time_ms = total_duration / sorted_results.len() as u64;

        let direction = if critical_trend.iter().any(|&x| x > 0) {
            TrendDirection::Worsening
        } else if critical_trend.iter().all(|&x| x <= 0) && critical_trend.iter().any(|&x| x < 0) {
            TrendDirection::Improving
        } else {
            TrendDirection::Stable
        };

        TrendAnalysis {
            direction,
            critical_trend,
            high_trend,
            medium_trend,
            average_scan_time_ms,
        }
    }

    pub fn get_findings_by_category(&self) -> FxHashMap<String, usize> {
        let mut categories: FxHashMap<String, usize> = FxHashMap::default();
        for result in self.results.iter() {
            for finding in &result.1.details {
                *categories.entry(finding.category.clone()).or_insert(0) += 1;
            }
        }
        categories
    }

    pub fn get_most_common_findings(&self, limit: usize) -> Vec<(String, usize)> {
        let mut counts: FxHashMap<String, usize> = FxHashMap::default();
        for result in self.results.iter() {
            for finding in &result.1.details {
                *counts.entry(finding.title.clone()).or_insert(0) += 1;
            }
        }

        let mut findings: Vec<_> = counts.into_iter().collect();
        findings.sort_by(|a, b| b.1.cmp(&a.1));
        findings.into_iter().take(limit).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub direction: TrendDirection,
    pub critical_trend: Vec<i32>,
    pub high_trend: Vec<i32>,
    pub medium_trend: Vec<i32>,
    pub average_scan_time_ms: u64,
}

impl Default for TrendAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scan_result(
        id: &str,
        timestamp: &str,
        critical: usize,
        high: usize,
        medium: usize,
    ) -> ScanResult {
        ScanResult {
            id: id.to_string(),
            target: "example.com".to_string(),
            scan_type: "full".to_string(),
            timestamp: timestamp.to_string(),
            summary: ResultSummary {
                total_findings: critical + high + medium,
                critical,
                high,
                medium,
                low: 0,
                info: 0,
                scan_duration_ms: 1000,
            },
            details: vec![],
        }
    }

    #[test]
    fn test_result_comparator_no_changes() {
        let old = make_scan_result("1", "2024-01-01", 1, 2, 3);
        let new = make_scan_result("2", "2024-01-02", 1, 2, 3);
        let result = ResultComparator::compare(&old, &new);
        assert!(result.added.is_empty());
        assert!(result.removed.is_empty());
    }

    #[test]
    fn test_result_comparator_added_finding() {
        let mut old = make_scan_result("1", "2024-01-01", 1, 2, 3);
        old.details.push(Finding {
            severity: Severity::High,
            category: "XSS".to_string(),
            title: "Old Finding".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });

        let mut new = make_scan_result("2", "2024-01-02", 1, 2, 3);
        new.details.push(Finding {
            severity: Severity::High,
            category: "XSS".to_string(),
            title: "Old Finding".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });
        new.details.push(Finding {
            severity: Severity::Critical,
            category: "SQLi".to_string(),
            title: "New Finding".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });

        let result = ResultComparator::compare(&old, &new);
        assert_eq!(result.added.len(), 1);
        assert_eq!(result.added[0].title, "New Finding");
        assert_eq!(result.unchanged.len(), 1);
    }

    #[test]
    fn test_result_comparator_removed_finding() {
        let mut old = make_scan_result("1", "2024-01-01", 1, 2, 3);
        old.details.push(Finding {
            severity: Severity::High,
            category: "XSS".to_string(),
            title: "Gone Finding".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });

        let new = make_scan_result("2", "2024-01-02", 1, 2, 3);

        let result = ResultComparator::compare(&old, &new);
        assert_eq!(result.removed.len(), 1);
        assert_eq!(result.removed[0].title, "Gone Finding");
    }

    #[test]
    fn test_result_comparator_distinguishes_same_title_by_category() {
        let mut old = make_scan_result("1", "2024-01-01", 1, 2, 3);
        old.details.push(Finding {
            severity: Severity::High,
            category: "XSS".to_string(),
            title: "Duplicate Title".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: Some("CVE-2024-1111".to_string()),
        });

        let mut new = make_scan_result("2", "2024-01-02", 1, 2, 3);
        new.details.push(Finding {
            severity: Severity::High,
            category: "SQLi".to_string(),
            title: "Duplicate Title".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: Some("CVE-2024-1111".to_string()),
        });

        let result = ResultComparator::compare(&old, &new);
        assert_eq!(result.added.len(), 1);
        assert_eq!(result.removed.len(), 1);
        assert!(result.unchanged.is_empty());
    }

    #[test]
    fn test_risk_trend_worsening() {
        let changes = vec![1, 2, -1, 3];
        assert!(matches!(
            ResultComparator::risk_trend(&changes),
            TrendDirection::Worsening
        ));
    }

    #[test]
    fn test_risk_trend_improving() {
        let changes = vec![-1, -2, 1, -3];
        assert!(matches!(
            ResultComparator::risk_trend(&changes),
            TrendDirection::Improving
        ));
    }

    #[test]
    fn test_risk_trend_stable() {
        let changes = vec![1, -1, 2, -2];
        assert!(matches!(
            ResultComparator::risk_trend(&changes),
            TrendDirection::Stable
        ));
    }

    #[test]
    fn test_risk_trend_empty() {
        assert!(matches!(
            ResultComparator::risk_trend(&[]),
            TrendDirection::Stable
        ));
    }

    #[test]
    fn test_trend_analyzer_single_result() {
        let mut analyzer = TrendAnalyzer::new();
        analyzer.add_result(make_scan_result("1", "2024-01-01", 1, 2, 3));
        let trend = analyzer.get_trend();
        assert!(matches!(trend.direction, TrendDirection::Stable));
        assert!(trend.critical_trend.is_empty());
    }

    #[test]
    fn test_trend_analyzer_worsening() {
        let mut analyzer = TrendAnalyzer::new();
        analyzer.add_result(make_scan_result("1", "2024-01-01", 1, 0, 0));
        analyzer.add_result(make_scan_result("2", "2024-01-02", 3, 0, 0));
        let trend = analyzer.get_trend();
        assert!(matches!(trend.direction, TrendDirection::Worsening));
        assert_eq!(trend.critical_trend, vec![2]);
    }

    #[test]
    fn test_trend_analyzer_improving() {
        let mut analyzer = TrendAnalyzer::new();
        analyzer.add_result(make_scan_result("1", "2024-01-01", 5, 0, 0));
        analyzer.add_result(make_scan_result("2", "2024-01-02", 2, 0, 0));
        let trend = analyzer.get_trend();
        assert!(matches!(trend.direction, TrendDirection::Improving));
        assert_eq!(trend.critical_trend, vec![-3]);
    }

    #[test]
    fn test_trend_analyzer_average_scan_time() {
        let mut analyzer = TrendAnalyzer::new();
        let mut r1 = make_scan_result("1", "2024-01-01", 0, 0, 0);
        r1.summary.scan_duration_ms = 1000;
        let mut r2 = make_scan_result("2", "2024-01-02", 0, 0, 0);
        r2.summary.scan_duration_ms = 2000;
        analyzer.add_result(r1);
        analyzer.add_result(r2);
        let trend = analyzer.get_trend();
        assert_eq!(trend.average_scan_time_ms, 1500);
    }

    #[test]
    fn test_trend_analyzer_category_counts() {
        let mut analyzer = TrendAnalyzer::new();
        let mut r = make_scan_result("1", "2024-01-01", 0, 0, 0);
        r.details.push(Finding {
            severity: Severity::High,
            category: "XSS".to_string(),
            title: "A".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });
        r.details.push(Finding {
            severity: Severity::Medium,
            category: "XSS".to_string(),
            title: "B".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });
        r.details.push(Finding {
            severity: Severity::Low,
            category: "CSRF".to_string(),
            title: "C".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });
        analyzer.add_result(r);
        let cats = analyzer.get_findings_by_category();
        assert_eq!(cats["XSS"], 2);
        assert_eq!(cats["CSRF"], 1);
    }

    #[test]
    fn test_trend_analyzer_most_common() {
        let mut analyzer = TrendAnalyzer::new();
        let mut r = make_scan_result("1", "2024-01-01", 0, 0, 0);
        r.details.push(Finding {
            severity: Severity::High,
            category: "XSS".to_string(),
            title: "Common".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });
        r.details.push(Finding {
            severity: Severity::High,
            category: "XSS".to_string(),
            title: "Common".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });
        r.details.push(Finding {
            severity: Severity::Medium,
            category: "CSRF".to_string(),
            title: "Rare".to_string(),
            description: String::new(),
            evidence: vec![],
            remediation: None,
            cve: None,
        });
        analyzer.add_result(r);
        let top = analyzer.get_most_common_findings(1);
        assert_eq!(top[0].0, "Common");
        assert_eq!(top[0].1, 2);
    }

    #[test]
    fn test_trend_analyzer_default() {
        let analyzer = TrendAnalyzer::default();
        assert!(analyzer.results.is_empty());
    }
}
