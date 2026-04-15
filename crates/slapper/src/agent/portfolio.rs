//! Target portfolio management for the security agent.
//!
//! Manages a collection of targets to monitor, including their configurations,
//! schedules, and scan history.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Priority {
    pub fn as_int(&self) -> i32 {
        match self {
            Priority::Low => 0,
            Priority::Normal => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        }
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanRecord {
    pub scan_id: String,
    pub scan_type: String,
    pub timestamp: DateTime<Utc>,
    pub findings_count: usize,
    pub severity_counts: HashMap<String, usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetConfig {
    pub target: String,
    pub target_type: String,
    pub priority: Priority,
    pub schedule: Option<String>,
    pub alert_channels: Vec<String>,
    pub last_scan: Option<DateTime<Utc>>,
    pub scan_history: Vec<ScanRecord>,
    pub baseline_findings: Vec<String>,
    pub enabled: bool,
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self {
            target: String::new(),
            target_type: "url".to_string(),
            priority: Priority::Normal,
            schedule: None,
            alert_channels: Vec::new(),
            last_scan: None,
            scan_history: Vec::new(),
            baseline_findings: Vec::new(),
            enabled: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioData {
    pub version: String,
    pub targets: HashMap<String, TargetConfig>,
}

impl Default for PortfolioData {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            targets: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct TargetPortfolio {
    data: PortfolioData,
    file_path: Option<PathBuf>,
}

impl TargetPortfolio {
    pub fn new() -> Self {
        Self {
            data: PortfolioData::default(),
            file_path: None,
        }
    }

    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let base_dir = directories::ProjectDirs::from("com", "slapper", "slapper")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("~/.config/slapper"));

        crate::utils::validation::validate_path(&base_dir, path)?;

        if path.exists() {
            let content = fs::read_to_string(path)?;
            let data: PortfolioData = serde_json::from_str(&content)?;
            Ok(Self {
                data,
                file_path: Some(path.clone()),
            })
        } else {
            Ok(Self::new())
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(ref path) = self.file_path {
            let base_dir = directories::ProjectDirs::from("com", "slapper", "slapper")
                .map(|d| d.config_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from("~/.config/slapper"));

            crate::utils::validation::validate_path(&base_dir, path)?;

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = serde_json::to_string(&self.data)?;
            fs::write(path, content)?;
        }
        Ok(())
    }

    pub fn add_target(&mut self, id: String, config: TargetConfig) {
        self.data.targets.insert(id, config);
    }

    pub fn remove_target(&mut self, id: &str) -> bool {
        self.data.targets.remove(id).is_some()
    }

    pub fn get_target(&self, id: &str) -> Option<&TargetConfig> {
        self.data.targets.get(id)
    }

    pub fn get_mut_target(&mut self, id: &str) -> Option<&mut TargetConfig> {
        self.data.targets.get_mut(id)
    }

    pub fn get_all_targets(&self) -> Vec<(String, TargetConfig)> {
        self.data
            .targets
            .iter()
            .filter(|(_, c)| c.enabled)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn update_last_scan(&mut self, id: &str, timestamp: &DateTime<Utc>) {
        if let Some(target) = self.data.targets.get_mut(id) {
            target.last_scan = Some(*timestamp);
        }
    }

    pub fn add_scan_record(&mut self, id: &str, record: ScanRecord) {
        if let Some(target) = self.data.targets.get_mut(id) {
            target.scan_history.push(record);
        }
    }

    pub fn set_baseline(&mut self, id: &str, finding_ids: Vec<String>) {
        if let Some(target) = self.data.targets.get_mut(id) {
            target.baseline_findings = finding_ids;
        }
    }

    pub fn targets_count(&self) -> usize {
        self.data.targets.len()
    }

    pub fn enabled_count(&self) -> usize {
        self.data.targets.values().filter(|t| t.enabled).count()
    }
}

impl Default for TargetPortfolio {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_crud() {
        let mut portfolio = TargetPortfolio::new();

        let config = TargetConfig {
            target: "https://example.com".to_string(),
            schedule: Some("0 0 * * *".to_string()),
            ..Default::default()
        };

        portfolio.add_target("example.com".to_string(), config);

        assert_eq!(portfolio.targets_count(), 1);
        assert!(portfolio.get_target("example.com").is_some());

        let targets = portfolio.get_all_targets();
        assert_eq!(targets.len(), 1);

        portfolio.remove_target("example.com");
        assert_eq!(portfolio.targets_count(), 0);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical.as_int() > Priority::High.as_int());
        assert!(Priority::High.as_int() > Priority::Normal.as_int());
        assert!(Priority::Normal.as_int() > Priority::Low.as_int());
    }
}
