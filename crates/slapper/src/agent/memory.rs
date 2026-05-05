//! Longitudinal memory system for the security agent.
//!
//! Provides persistent storage of scan results, findings, and pattern detection
//! across multiple scans of the same targets.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::tool::response::Finding;

const ALERTED_FINDINGS_FILE: &str = "alerted_findings.json";
const SNAPSHOT_FILE: &str = "portfolio_snapshot.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanMemory {
    pub scan_id: String,
    pub target: String,
    pub scan_type: String,
    pub timestamp: DateTime<Utc>,
    pub findings: Vec<Finding>,
    pub summary: ScanSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_findings: usize,
    pub by_severity: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
}

impl ScanSummary {
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut by_severity: HashMap<String, usize> = HashMap::new();
        let mut by_type: HashMap<String, usize> = HashMap::new();

        for finding in findings {
            *by_severity
                .entry(finding.severity.as_str().to_string())
                .or_insert(0) += 1;
            *by_type
                .entry(format!("{:?}", finding.finding_type))
                .or_insert(0) += 1;
        }

        Self {
            total_findings: findings.len(),
            by_severity,
            by_type,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternEntry {
    pub pattern_type: String,
    pub description: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub occurrence_count: usize,
    pub related_findings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetMemory {
    pub target: String,
    pub scans: Vec<ScanMemory>,
    pub patterns: Vec<PatternEntry>,
    pub baselines: Vec<String>,
}

impl Default for TargetMemory {
    fn default() -> Self {
        Self {
            target: String::new(),
            scans: Vec::new(),
            patterns: Vec::new(),
            baselines: Vec::new(),
        }
    }
}

pub struct LongitudinalMemory {
    storage_dir: PathBuf,
    max_scans_per_target: Option<usize>,
    // Locks for concurrent access
    target_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    alerted_lock: Mutex<()>,
    snapshot_lock: Mutex<()>,
    patterns_lock: Mutex<()>,
}

impl LongitudinalMemory {
    pub async fn new(storage_dir: PathBuf) -> Result<Self> {
        Self::with_max_scans(storage_dir, None).await
    }

    pub async fn with_max_scans(storage_dir: PathBuf, max_scans: Option<usize>) -> Result<Self> {
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir).await?;
        }

        let targets_dir = storage_dir.join("targets");
        if !targets_dir.exists() {
            fs::create_dir_all(&targets_dir).await?;
        }

        let patterns_dir = storage_dir.join("patterns");
        if !patterns_dir.exists() {
            fs::create_dir_all(&patterns_dir).await?;
        }

        Ok(Self {
            storage_dir,
            max_scans_per_target: max_scans,
            target_locks: Mutex::new(HashMap::new()),
            alerted_lock: Mutex::new(()),
            snapshot_lock: Mutex::new(()),
            patterns_lock: Mutex::new(()),
        })
    }

    pub fn storage_dir(&self) -> &PathBuf {
        &self.storage_dir
    }

    async fn get_target_lock(&self, target: &str) -> Arc<Mutex<()>> {
        let target_path = self.get_target_path(target);
        let target_hash = target_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let mut locks = self.target_locks.lock().await;
        locks
            .entry(target_hash)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn atomic_write(&self, path: &PathBuf, content: &str) -> Result<()> {
        let tmp_path = path.with_file_name(format!(
            "{}.tmp",
            path.file_name().unwrap().to_string_lossy()
        ));
        let mut file = fs::File::create(&tmp_path).await?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
        fs::rename(&tmp_path, path).await?;
        Ok(())
    }

    async fn load_target_memory(&self, target: &str) -> Result<TargetMemory> {
        let target_path = self.get_target_path(target);
        if !target_path.exists() {
            return Ok(TargetMemory {
                target: target.to_string(),
                ..Default::default()
            });
        }
        let content = fs::read_to_string(&target_path).await?;
        let memory: TargetMemory = serde_json::from_str(&content).map_err(|e| {
            anyhow::anyhow!(
                "Corrupt target memory file {}: {}",
                target_path.display(),
                e
            )
        })?;
        Ok(memory)
    }

    fn get_target_path(&self, target: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        target.hash(&mut hasher);
        let hash = format!("{:016x}", hasher.finish());

        self.storage_dir
            .join("targets")
            .join(format!("{}.json", hash))
    }

    fn get_patterns_path(&self) -> PathBuf {
        self.storage_dir.join("patterns").join("detected.json")
    }

    fn get_alerted_findings_path(&self) -> PathBuf {
        self.storage_dir.join(ALERTED_FINDINGS_FILE)
    }

    fn get_snapshot_path(&self) -> PathBuf {
        self.storage_dir.join(SNAPSHOT_FILE)
    }

    async fn load_alerted_findings(&self) -> Result<HashSet<String>> {
        let path = self.get_alerted_findings_path();
        if !path.exists() {
            return Ok(HashSet::new());
        }
        let content = fs::read_to_string(&path).await?;
        match serde_json::from_str::<HashSet<String>>(&content) {
            Ok(findings) => Ok(findings),
            Err(e) => {
                tracing::warn!("Corrupt alerted findings file, starting fresh: {}", e);
                Ok(HashSet::new())
            }
        }
    }

    async fn save_alerted_findings(&self, findings: &HashSet<String>) -> Result<()> {
        let path = self.get_alerted_findings_path();
        let content = serde_json::to_string_pretty(findings)?;
        self.atomic_write(&path, &content).await
    }

    pub async fn warm_cache(&self) -> Result<()> {
        self.load_alerted_findings().await?;
        Ok(())
    }

    pub async fn deduplicate_findings(
        &self,
        findings: Vec<Finding>,
    ) -> Result<(Vec<Finding>, Vec<Finding>)> {
        let _lock = self.alerted_lock.lock().await;
        let alerted = self.load_alerted_findings().await?;
        let mut new_alerted = alerted.clone();

        let mut deduplicated = Vec::new();
        let mut filtered = Vec::new();

        for finding in findings {
            if alerted.contains(&finding.id) {
                filtered.push(finding);
            } else {
                new_alerted.insert(finding.id.clone());
                deduplicated.push(finding);
            }
        }

        if new_alerted != alerted {
            self.save_alerted_findings(&new_alerted).await?;
        }

        Ok((deduplicated, filtered))
    }

    pub async fn write_portfolio_snapshot(&self) -> Result<()> {
        let _lock = self.snapshot_lock.lock().await;
        let targets_dir = self.storage_dir.join("targets");

        let mut unique_targets: HashSet<String> = HashSet::new();
        let mut total_scans = 0;
        let mut findings_by_severity: HashMap<String, usize> = HashMap::new();
        let mut today_scans = 0;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        if targets_dir.exists() {
            let mut entries = fs::read_dir(&targets_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(memory) = serde_json::from_str::<TargetMemory>(&content) {
                            unique_targets.insert(memory.target.clone());

                            for scan in &memory.scans {
                                total_scans += 1;

                                let scan_day = scan.timestamp.format("%Y-%m-%d").to_string();
                                if scan_day == today {
                                    today_scans += 1;
                                }

                                for finding in &scan.findings {
                                    *findings_by_severity
                                        .entry(finding.severity.as_str().to_lowercase())
                                        .or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        let critical_findings = *findings_by_severity.get("critical").unwrap_or(&0);
        let high_findings = *findings_by_severity.get("high").unwrap_or(&0);
        let medium_findings = *findings_by_severity.get("medium").unwrap_or(&0);
        let low_findings = *findings_by_severity.get("low").unwrap_or(&0);
        let total_findings: usize = findings_by_severity.values().sum();

        let health_score = if total_findings > 0 {
            let penalty = (critical_findings as f64 * 0.4)
                + (high_findings as f64 * 0.25)
                + (medium_findings as f64 * 0.15)
                + (low_findings as f64 * 0.1);
            (1.0 - (penalty / total_findings as f64)).max(0.0).min(1.0)
        } else {
            1.0
        };

        let mut findings_trend: Vec<(String, usize)> = Vec::new();
        let mut monthly_counts: HashMap<String, usize> = HashMap::new();

        if targets_dir.exists() {
            let mut entries = fs::read_dir(&targets_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(memory) = serde_json::from_str::<TargetMemory>(&content) {
                            for scan in &memory.scans {
                                let month = scan.timestamp.format("%Y-%m").to_string();
                                *monthly_counts.entry(month).or_insert(0) += scan.findings.len();
                            }
                        }
                    }
                }
            }
        }

        let mut sorted_months: Vec<String> = monthly_counts.keys().cloned().collect();
        sorted_months.sort();
        for month in sorted_months.into_iter().rev().take(6) {
            if let Some(&count) = monthly_counts.get(&month) {
                findings_trend.push((month, count));
            }
        }
        findings_trend.reverse();

        let snapshot = PortfolioSnapshot {
            unique_targets: unique_targets.len(),
            total_scans,
            scans_today: today_scans,
            findings_by_severity,
            findings_trend,
            critical_findings,
            health_score,
            last_updated: chrono::Utc::now(),
        };

        let path = self.get_snapshot_path();
        let content = serde_json::to_string_pretty(&snapshot)?;
        self.atomic_write(&path, &content).await
    }

    pub fn read_portfolio_snapshot(&self) -> Option<PortfolioSnapshot> {
        let path = self.get_snapshot_path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<PortfolioSnapshot>(&content).ok()
    }

    pub async fn store_scan_results(
        &self,
        target: &str,
        response: &crate::tool::ToolResponse,
    ) -> Result<()> {
        let target_lock = self.get_target_lock(target).await;
        let _lock = target_lock.lock().await;

        let scan_memory = ScanMemory {
            scan_id: response.request_id.clone(),
            target: target.to_string(),
            scan_type: response.tool_id.clone(),
            timestamp: response.metadata.completed_at,
            findings: response.findings.clone(),
            summary: ScanSummary::from_findings(&response.findings),
        };

        let target_path = self.get_target_path(target);
        let mut memory = self.load_target_memory(target).await?;

        memory.scans.push(scan_memory);

        if let Some(max) = self.max_scans_per_target {
            while memory.scans.len() > max {
                memory.scans.remove(0);
            }
        }

        let content = serde_json::to_string(&memory)?;
        self.atomic_write(&target_path, &content).await?;

        self.detect_and_record_patterns(target, &memory).await?;

        if let Err(e) = self.write_portfolio_snapshot().await {
            tracing::warn!("Failed to write portfolio snapshot: {}", e);
        }

        Ok(())
    }

    pub async fn get_target_history(&self, target: &str) -> Result<Vec<ScanMemory>> {
        let target_path = self.get_target_path(target);

        if !target_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&target_path).await?;
        let memory: TargetMemory = serde_json::from_str(&content)?;

        Ok(memory.scans)
    }

    pub async fn get_patterns(&self, target: &str) -> Result<Vec<PatternEntry>> {
        let target_path = self.get_target_path(target);

        if !target_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&target_path).await?;
        let memory: TargetMemory = serde_json::from_str(&content)?;

        Ok(memory.patterns)
    }

    pub async fn set_baseline(&self, target: &str, finding_ids: Vec<String>) -> Result<()> {
        let target_lock = self.get_target_lock(target).await;
        let _lock = target_lock.lock().await;

        let mut memory = self.load_target_memory(target).await?;

        memory.baselines = finding_ids;

        let content = serde_json::to_string(&memory)?;
        let target_path = self.get_target_path(target);
        self.atomic_write(&target_path, &content).await
    }

    async fn detect_and_record_patterns(&self, _target: &str, memory: &TargetMemory) -> Result<()> {
        let _lock = self.patterns_lock.lock().await;
        let mut patterns: HashMap<String, PatternEntry> = HashMap::new();

        for scan in &memory.scans {
            for finding in &scan.findings {
                let pattern_key =
                    format!("{:?}:{}", finding.finding_type, finding.severity.as_str());

                let entry = patterns
                    .entry(pattern_key.clone())
                    .or_insert_with(|| PatternEntry {
                        pattern_type: format!("{:?}", finding.finding_type),
                        description: format!(
                            "Finding type '{:?}' with severity '{}'",
                            finding.finding_type,
                            finding.severity.as_str()
                        ),
                        first_seen: scan.timestamp,
                        last_seen: scan.timestamp,
                        occurrence_count: 0,
                        related_findings: Vec::new(),
                    });

                entry.last_seen = scan.timestamp;
                entry.occurrence_count += 1;
                entry.related_findings.push(finding.id.clone());
            }
        }

        if !patterns.is_empty() {
            let patterns_path = self.get_patterns_path();
            let content = serde_json::to_string(&patterns.values().collect::<Vec<_>>())?;
            self.atomic_write(&patterns_path, &content).await?;
        }

        Ok(())
    }

    pub async fn compare_with_baseline(
        &self,
        target: &str,
        findings: &[Finding],
    ) -> Result<BaselineComparison> {
        let target_path = self.get_target_path(target);

        let (baseline_ids, all_historical_findings) = if target_path.exists() {
            let content = fs::read_to_string(&target_path).await?;
            let memory: TargetMemory = serde_json::from_str(&content)?;
            let all_findings: Vec<Finding> = memory
                .scans
                .iter()
                .flat_map(|scan| scan.findings.iter().cloned())
                .collect();
            (memory.baselines, all_findings)
        } else {
            (Vec::new(), Vec::new())
        };

        let current_ids: HashSet<&str> = findings.iter().map(|f| f.id.as_str()).collect();

        let new_findings: Vec<Finding> = findings
            .iter()
            .filter(|f| !baseline_ids.contains(&f.id))
            .cloned()
            .collect();

        let baseline_ids_set: HashSet<&str> = baseline_ids.iter().map(|s| s.as_str()).collect();
        let resolved_ids: HashSet<&str> =
            baseline_ids_set.difference(&current_ids).cloned().collect();

        let resolved_findings: Vec<Finding> = all_historical_findings
            .into_iter()
            .filter(|f| resolved_ids.contains(f.id.as_str()))
            .collect();

        Ok(BaselineComparison {
            new_findings,
            resolved_findings,
            unchanged_count: findings.len(),
        })
    }

    pub async fn detect_cross_target_patterns(&self) -> Result<Vec<CrossTargetPattern>> {
        let mut patterns: HashMap<String, CrossTargetPatternBuilder> = HashMap::new();
        let targets_dir = self.storage_dir.join("targets");

        if !targets_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&targets_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(memory) = serde_json::from_str::<TargetMemory>(&content) {
                        for scan in &memory.scans {
                            for finding in &scan.findings {
                                let pattern_key = format!(
                                    "{}:{}",
                                    finding.finding_type,
                                    finding.severity.as_str()
                                );

                                let builder =
                                    patterns.entry(pattern_key.clone()).or_insert_with(|| {
                                        CrossTargetPatternBuilder {
                                            pattern_type: format!("{:?}", finding.finding_type),
                                            description: format!(
                                                "{} with severity {}",
                                                finding.finding_type,
                                                finding.severity.as_str()
                                            ),
                                            affected_targets: Vec::new(),
                                            first_seen: scan.timestamp,
                                            last_seen: scan.timestamp,
                                            total_occurrences: 0,
                                            severity: finding.severity.to_agent_severity(),
                                        }
                                    });

                                if !builder.affected_targets.contains(&memory.target) {
                                    builder.affected_targets.push(memory.target.clone());
                                }
                                builder.last_seen = scan.timestamp;
                                builder.total_occurrences += 1;
                            }
                        }
                    }
                }
            }
        }

        let cross_patterns: Vec<CrossTargetPattern> = patterns
            .into_values()
            .filter(|p| p.affected_targets.len() > 1)
            .map(|p| p.into())
            .collect();

        Ok(cross_patterns)
    }

    pub async fn analyze_temporal_patterns(&self, target: &str) -> Result<TemporalAnalysis> {
        let history = self.get_target_history(target).await?;

        let mut findings_by_day: HashMap<String, Vec<&Finding>> = HashMap::new();
        let mut severity_trend: Vec<(String, HashMap<String, usize>)> = Vec::new();

        for scan in &history {
            let day = scan.timestamp.format("%Y-%m-%d").to_string();
            let day_findings = findings_by_day.entry(day.clone()).or_insert_with(Vec::new);

            for finding in &scan.findings {
                day_findings.push(finding);
            }
        }

        let mut current_day = String::new();
        let mut current_counts: HashMap<String, usize> = HashMap::new();

        let mut sorted_days: Vec<String> = findings_by_day.keys().cloned().collect();
        sorted_days.sort();

        for day in sorted_days {
            if current_day.is_empty() {
                current_day = day.clone();
            }

            let day_severities: HashMap<String, usize> =
                findings_by_day[&day]
                    .iter()
                    .fold(HashMap::new(), |mut acc, f| {
                        *acc.entry(f.severity.as_str().to_string()).or_insert(0) += 1;
                        acc
                    });

            if day != current_day {
                severity_trend.push((current_day.clone(), current_counts.clone()));
                current_day = day.clone();
            }
            current_counts = day_severities;
        }

        if !current_counts.is_empty() {
            severity_trend.push((current_day, current_counts));
        }

        Ok(TemporalAnalysis {
            target: target.to_string(),
            findings_by_day: severity_trend,
            total_scans: history.len(),
        })
    }

    pub async fn cleanup_old_patterns(&self, ttl_days: u64) -> Result<usize> {
        let _lock = self.patterns_lock.lock().await;
        let patterns_path = self.get_patterns_path();

        if !patterns_path.exists() {
            return Ok(0);
        }

        let content = fs::read_to_string(&patterns_path).await?;
        let patterns: Vec<PatternEntry> = match serde_json::from_str(&content) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Corrupt patterns file, starting fresh: {}", e);
                return Ok(0);
            }
        };

        let cutoff = chrono::Utc::now() - chrono::Duration::days(ttl_days as i64);
        let original_count = patterns.len();

        let filtered: Vec<PatternEntry> = patterns
            .into_iter()
            .filter(|p| p.last_seen > cutoff)
            .collect();

        let removed_count = original_count - filtered.len();

        if removed_count > 0 {
            let content = serde_json::to_string(&filtered)?;
            self.atomic_write(&patterns_path, &content).await?;
        }

        Ok(removed_count)
    }
}

struct CrossTargetPatternBuilder {
    pattern_type: String,
    description: String,
    affected_targets: Vec<String>,
    first_seen: chrono::DateTime<Utc>,
    last_seen: chrono::DateTime<Utc>,
    total_occurrences: usize,
    severity: crate::types::Severity,
}

#[derive(Clone, Debug)]
pub struct CrossTargetPattern {
    pub pattern_type: String,
    pub description: String,
    pub affected_targets: Vec<String>,
    pub target_count: usize,
    pub first_seen: chrono::DateTime<Utc>,
    pub last_seen: chrono::DateTime<Utc>,
    pub total_occurrences: usize,
    pub severity: crate::types::Severity,
}

impl From<CrossTargetPatternBuilder> for CrossTargetPattern {
    fn from(builder: CrossTargetPatternBuilder) -> Self {
        Self {
            pattern_type: builder.pattern_type,
            description: builder.description,
            affected_targets: builder.affected_targets.clone(),
            target_count: builder.affected_targets.len(),
            first_seen: builder.first_seen,
            last_seen: builder.last_seen,
            total_occurrences: builder.total_occurrences,
            severity: builder.severity,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TemporalAnalysis {
    pub target: String,
    pub findings_by_day: Vec<(String, HashMap<String, usize>)>,
    pub total_scans: usize,
}

#[derive(Clone, Debug)]
pub struct BaselineComparison {
    pub new_findings: Vec<Finding>,
    pub resolved_findings: Vec<Finding>,
    pub unchanged_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub unique_targets: usize,
    pub total_scans: usize,
    pub scans_today: usize,
    pub findings_by_severity: HashMap<String, usize>,
    pub findings_trend: Vec<(String, usize)>,
    pub critical_findings: usize,
    pub health_score: f64,
    pub last_updated: DateTime<Utc>,
}

impl Default for LongitudinalMemory {
    fn default() -> Self {
        Self {
            storage_dir: PathBuf::from("~/.config/slapper/memory"),
            max_scans_per_target: None,
            target_locks: Mutex::new(HashMap::new()),
            alerted_lock: Mutex::new(()),
            snapshot_lock: Mutex::new(()),
            patterns_lock: Mutex::new(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scan_summary() {
        let findings = vec![];

        let summary = ScanSummary::from_findings(&findings);
        assert_eq!(summary.total_findings, 0);
    }

    #[test]
    fn test_scan_summary_with_findings() {
        let finding = Finding {
            id: "test-1".to_string(),
            finding_type: crate::tool::response::FindingType::Vulnerability,
            severity: crate::tool::response::ResponseSeverity::Critical,
            title: "SQL Injection".to_string(),
            description: "SQL injection detected".to_string(),
            location: "https://example.com/login".to_string(),
            evidence: None,
            cve_ids: vec![],
            remediation: Some("Use parameterized queries".to_string()),
            references: vec![],
            metadata: Default::default(),
        };
        let findings = vec![finding];

        let summary = ScanSummary::from_findings(&findings);
        assert_eq!(summary.total_findings, 1);
        assert_eq!(*summary.by_severity.get("critical").unwrap_or(&0), 1);
    }

    #[tokio::test]
    async fn test_longitudinal_memory_new_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir.clone()).await.unwrap();

        assert!(storage_dir.exists());
        assert!(storage_dir.join("targets").exists());
        assert!(storage_dir.join("patterns").exists());
    }

    #[tokio::test]
    async fn test_longitudinal_memory_store_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir.clone()).await.unwrap();

        let tool_response = crate::tool::ToolResponse {
            request_id: "scan-123".to_string(),
            tool_id: "recon".to_string(),
            status: crate::tool::response::ResponseStatus::Success,
            results: serde_json::json!({}),
            metadata: crate::tool::response::ResponseMetadata {
                started_at: chrono::Utc::now(),
                completed_at: chrono::Utc::now(),
                duration_ms: 100,
                targets_scanned: 1,
                findings_count: 0,
            },
            errors: vec![],
            findings: vec![],
        };

        let result = memory
            .store_scan_results("https://example.com", &tool_response)
            .await;
        assert!(result.is_ok());

        let history = memory
            .get_target_history("https://example.com")
            .await
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].scan_id, "scan-123");
    }

    #[tokio::test]
    async fn test_longitudinal_memory_get_target_history_empty() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).await.unwrap();

        let history = memory
            .get_target_history("https://nonexistent.com")
            .await
            .unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_longitudinal_memory_multiple_scans() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).await.unwrap();

        for i in 0..3 {
            let tool_response = crate::tool::ToolResponse {
                request_id: format!("scan-{}", i),
                tool_id: "recon".to_string(),
                status: crate::tool::response::ResponseStatus::Success,
                results: serde_json::json!({}),
                metadata: crate::tool::response::ResponseMetadata {
                    started_at: chrono::Utc::now(),
                    completed_at: chrono::Utc::now(),
                    duration_ms: 100,
                    targets_scanned: 1,
                    findings_count: 0,
                },
                errors: vec![],
                findings: vec![],
            };
            memory
                .store_scan_results("https://example.com", &tool_response)
                .await
                .unwrap();
        }

        let history = memory
            .get_target_history("https://example.com")
            .await
            .unwrap();
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn test_longitudinal_memory_set_and_get_baseline() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).await.unwrap();

        let finding_ids = vec!["finding-1".to_string(), "finding-2".to_string()];
        let result = memory
            .set_baseline("https://example.com", finding_ids.clone())
            .await;
        assert!(result.is_ok());

        let tool_response = crate::tool::ToolResponse {
            request_id: "scan-1".to_string(),
            tool_id: "recon".to_string(),
            status: crate::tool::response::ResponseStatus::Success,
            results: serde_json::json!({}),
            metadata: crate::tool::response::ResponseMetadata {
                started_at: chrono::Utc::now(),
                completed_at: chrono::Utc::now(),
                duration_ms: 100,
                targets_scanned: 1,
                findings_count: 0,
            },
            errors: vec![],
            findings: vec![],
        };
        memory
            .store_scan_results("https://example.com", &tool_response)
            .await
            .unwrap();

        let history = memory
            .get_target_history("https://example.com")
            .await
            .unwrap();
        assert!(!history.is_empty());
    }

    #[tokio::test]
    async fn test_longitudinal_memory_compare_with_baseline_new_findings() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).await.unwrap();

        memory
            .set_baseline("https://example.com", vec![])
            .await
            .unwrap();

        let new_finding = Finding {
            id: "new-finding-1".to_string(),
            finding_type: crate::tool::response::FindingType::Vulnerability,
            severity: crate::tool::response::ResponseSeverity::Critical,
            title: "SQL Injection".to_string(),
            description: "SQL injection detected".to_string(),
            location: "https://example.com/login".to_string(),
            evidence: None,
            cve_ids: vec![],
            remediation: Some("Use parameterized queries".to_string()),
            references: vec![],
            metadata: Default::default(),
        };

        let comparison = memory
            .compare_with_baseline("https://example.com", &[new_finding])
            .await
            .unwrap();
        assert_eq!(comparison.new_findings.len(), 1);
        assert_eq!(comparison.new_findings[0].id, "new-finding-1");
    }

    #[tokio::test]
    async fn test_longitudinal_memory_get_patterns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).await.unwrap();

        let patterns = memory.get_patterns("https://example.com").await.unwrap();
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_target_memory_default() {
        let memory = TargetMemory::default();
        assert!(memory.target.is_empty());
        assert!(memory.scans.is_empty());
        assert!(memory.patterns.is_empty());
        assert!(memory.baselines.is_empty());
    }

    #[test]
    fn test_pattern_entry_creation() {
        let entry = PatternEntry {
            pattern_type: "SQLInjection".to_string(),
            description: "SQL injection pattern".to_string(),
            first_seen: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            occurrence_count: 5,
            related_findings: vec!["finding-1".to_string()],
        };
        assert_eq!(entry.occurrence_count, 5);
        assert_eq!(entry.pattern_type, "SQLInjection");
    }
}
