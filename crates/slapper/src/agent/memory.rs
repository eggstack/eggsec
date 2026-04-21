//! Longitudinal memory system for the security agent.
//!
//! Provides persistent storage of scan results, findings, and pattern detection
//! across multiple scans of the same targets.

use std::collections::{HashMap, HashSet};
use tokio::fs;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tool::response::Finding;

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
}

impl LongitudinalMemory {
    pub fn new(storage_dir: PathBuf) -> Result<Self> {
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)?;
        }

        let targets_dir = storage_dir.join("targets");
        if !targets_dir.exists() {
            fs::create_dir_all(&targets_dir)?;
        }

        let patterns_dir = storage_dir.join("patterns");
        if !patterns_dir.exists() {
            fs::create_dir_all(&patterns_dir)?;
        }

        Ok(Self { storage_dir })
    }

    fn get_target_path(&self, target: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let safe_name = target
            .replace("://", "_")
            .replace("/", "_")
            .replace(":", "_");

        let mut hasher = DefaultHasher::new();
        target.hash(&mut hasher);
        let hash = format!("{:x}", hasher.finish());

        self.storage_dir
            .join("targets")
            .join(format!("{}_{}.json", safe_name, hash))
    }

    fn get_patterns_path(&self) -> PathBuf {
        self.storage_dir.join("patterns").join("detected.json")
    }

    pub fn store_scan_results(
        &self,
        target: &str,
        response: &crate::tool::ToolResponse,
    ) -> Result<()> {
        let scan_memory = ScanMemory {
            scan_id: response.request_id.clone(),
            target: target.to_string(),
            scan_type: response.tool_id.clone(),
            timestamp: response.metadata.completed_at,
            findings: response.findings.clone(),
            summary: ScanSummary::from_findings(&response.findings),
        };

        let target_path = self.get_target_path(target);

        let mut memory = if target_path.exists() {
            let content = fs::read_to_string(&target_path)?;
            serde_json::from_str::<TargetMemory>(&content)?
        } else {
            TargetMemory {
                target: target.to_string(),
                ..Default::default()
            }
        };

        memory.scans.push(scan_memory);

        let content = serde_json::to_string(&memory)?;
        fs::write(&target_path, content)?;

        self.detect_and_record_patterns(target, &memory)?;

        Ok(())
    }

    pub fn get_target_history(&self, target: &str) -> Result<Vec<ScanMemory>> {
        let target_path = self.get_target_path(target);

        if !target_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&target_path)?;
        let memory: TargetMemory = serde_json::from_str(&content)?;

        Ok(memory.scans)
    }

    pub fn get_patterns(&self, target: &str) -> Result<Vec<PatternEntry>> {
        let target_path = self.get_target_path(target);

        if !target_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&target_path)?;
        let memory: TargetMemory = serde_json::from_str(&content)?;

        Ok(memory.patterns)
    }

    pub fn set_baseline(&self, target: &str, finding_ids: Vec<String>) -> Result<()> {
        let target_path = self.get_target_path(target);

        let mut memory = if target_path.exists() {
            let content = fs::read_to_string(&target_path)?;
            serde_json::from_str::<TargetMemory>(&content)?
        } else {
            TargetMemory {
                target: target.to_string(),
                ..Default::default()
            }
        };

        memory.baselines = finding_ids;

        let content = serde_json::to_string(&memory)?;
        fs::write(&target_path, content)?;

        Ok(())
    }

    fn detect_and_record_patterns(&self, _target: &str, memory: &TargetMemory) -> Result<()> {
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
            fs::write(&patterns_path, content)?;
        }

        Ok(())
    }

    pub fn compare_with_baseline(
        &self,
        target: &str,
        findings: &[Finding],
    ) -> Result<BaselineComparison> {
        let target_path = self.get_target_path(target);

        let (baseline_ids, all_historical_findings) = if target_path.exists() {
            let content = fs::read_to_string(&target_path)?;
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
}

#[derive(Clone, Debug)]
pub struct BaselineComparison {
    pub new_findings: Vec<Finding>,
    pub resolved_findings: Vec<Finding>,
    pub unchanged_count: usize,
}

impl Default for LongitudinalMemory {
    fn default() -> Self {
        Self {
            storage_dir: PathBuf::from("~/.config/slapper/memory"),
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
            finding_type: crate::tool::response::FindingType::SqlInjection,
            severity: crate::types::Severity::Critical,
            title: "SQL Injection".to_string(),
            description: "SQL injection detected".to_string(),
            evidence: vec![],
            remediation: vec![],
            confidence: 0.9,
            metadata: Default::default(),
        };
        let findings = vec![finding];

        let summary = ScanSummary::from_findings(&findings);
        assert_eq!(summary.total_findings, 1);
        assert_eq!(*summary.by_severity.get("Critical").unwrap_or(&0), 1);
    }

    #[test]
    fn test_longitudinal_memory_new_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir.clone()).unwrap();

        assert!(storage_dir.exists());
        assert!(storage_dir.join("targets").exists());
        assert!(storage_dir.join("patterns").exists());
    }

    #[test]
    fn test_longitudinal_memory_store_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir.clone()).unwrap();

        let tool_response = crate::tool::ToolResponse {
            request_id: "scan-123".to_string(),
            tool_id: "recon".to_string(),
            findings: vec![],
            metadata: crate::tool::response::ToolResponseMetadata {
                started_at: chrono::Utc::now(),
                completed_at: chrono::Utc::now(),
                duration_ms: 100,
            },
        };

        let result = memory.store_scan_results("https://example.com", &tool_response);
        assert!(result.is_ok());

        let history = memory.get_target_history("https://example.com").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].scan_id, "scan-123");
    }

    #[test]
    fn test_longitudinal_memory_get_target_history_empty() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).unwrap();

        let history = memory.get_target_history("https://nonexistent.com").unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_longitudinal_memory_multiple_scans() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).unwrap();

        for i in 0..3 {
            let tool_response = crate::tool::ToolResponse {
                request_id: format!("scan-{}", i),
                tool_id: "recon".to_string(),
                findings: vec![],
                metadata: crate::tool::response::ToolResponseMetadata {
                    started_at: chrono::Utc::now(),
                    completed_at: chrono::Utc::now(),
                    duration_ms: 100,
                },
            };
            memory.store_scan_results("https://example.com", &tool_response).unwrap();
        }

        let history = memory.get_target_history("https://example.com").unwrap();
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_longitudinal_memory_set_and_get_baseline() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).unwrap();

        let finding_ids = vec!["finding-1".to_string(), "finding-2".to_string()];
        let result = memory.set_baseline("https://example.com", finding_ids.clone());
        assert!(result.is_ok());

        let tool_response = crate::tool::ToolResponse {
            request_id: "scan-1".to_string(),
            tool_id: "recon".to_string(),
            findings: vec![],
            metadata: crate::tool::response::ToolResponseMetadata {
                started_at: chrono::Utc::now(),
                completed_at: chrono::Utc::now(),
                duration_ms: 100,
            },
        };
        memory.store_scan_results("https://example.com", &tool_response).unwrap();

        let history = memory.get_target_history("https://example.com").unwrap();
        assert!(!history.is_empty());
    }

    #[test]
    fn test_longitudinal_memory_compare_with_baseline_new_findings() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).unwrap();

        memory.set_baseline("https://example.com", vec![]).unwrap();

        let new_finding = Finding {
            id: "new-finding-1".to_string(),
            finding_type: crate::tool::response::FindingType::SqlInjection,
            severity: crate::types::Severity::Critical,
            title: "SQL Injection".to_string(),
            description: "SQL injection detected".to_string(),
            evidence: vec![],
            remediation: vec![],
            confidence: 0.9,
            metadata: Default::default(),
        };

        let comparison = memory.compare_with_baseline("https://example.com", &[new_finding]).unwrap();
        assert_eq!(comparison.new_findings.len(), 1);
        assert_eq!(comparison.new_findings[0].id, "new-finding-1");
    }

    #[test]
    fn test_longitudinal_memory_get_patterns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().join("memory");
        let memory = LongitudinalMemory::new(storage_dir).unwrap();

        let patterns = memory.get_patterns("https://example.com").unwrap();
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
