#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn as_int(&self) -> i32 {
        match self {
            Severity::Critical => 4,
            Severity::High => 3,
            Severity::Medium => 2,
            Severity::Low => 1,
            Severity::Info => 0,
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" | "moderate" => Severity::Medium,
            "low" => Severity::Low,
            _ => Severity::Info,
        }
    }
}

pub struct ResultComparator;

impl ResultComparator {
    pub fn compare(old: &ScanResult, new: &ScanResult) -> ComparisonResult {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut unchanged = Vec::new();

        let old_findings: HashMap<_, _> = old
            .details
            .iter()
            .map(|f| (f.title.clone(), f.clone()))
            .collect();
        let new_findings: HashMap<_, _> = new
            .details
            .iter()
            .map(|f| (f.title.clone(), f.clone()))
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
    results: Vec<ScanResult>,
}

impl TrendAnalyzer {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: ScanResult) {
        self.results.push(result);
        self.results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
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

        let critical_trend: Vec<i32> = self
            .results
            .windows(2)
            .map(|w| w[1].summary.critical as i32 - w[0].summary.critical as i32)
            .collect();

        let high_trend: Vec<i32> = self
            .results
            .windows(2)
            .map(|w| w[1].summary.high as i32 - w[0].summary.high as i32)
            .collect();

        let medium_trend: Vec<i32> = self
            .results
            .windows(2)
            .map(|w| w[1].summary.medium as i32 - w[0].summary.medium as i32)
            .collect();

        let total_duration: u64 = self
            .results
            .iter()
            .map(|r| r.summary.scan_duration_ms)
            .sum();
        let average_scan_time_ms = total_duration / self.results.len() as u64;

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

    pub fn get_findings_by_category(&self) -> HashMap<String, usize> {
        let mut categories: HashMap<String, usize> = HashMap::new();
        for result in &self.results {
            for finding in &result.details {
                *categories.entry(finding.category.clone()).or_insert(0) += 1;
            }
        }
        categories
    }

    pub fn get_most_common_findings(&self, limit: usize) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for result in &self.results {
            for finding in &result.details {
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
