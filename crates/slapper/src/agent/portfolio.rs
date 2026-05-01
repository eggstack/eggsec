//! Target portfolio management for the security agent.
//!
//! Manages a collection of targets to monitor, including their configurations,
//! schedules, and scan history.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use anyhow::Result;
use chrono::{DateTime, Utc, Timelike};
use chrono_tz::Tz;
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScanDepth {
    Shallow,
    Deep,
}

impl ScanDepth {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanDepth::Shallow => "shallow",
            ScanDepth::Deep => "deep",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ScanDepth::Shallow => "Quick scan with essential checks only",
            ScanDepth::Deep => "Comprehensive scan with all payload types",
        }
    }
}

impl Default for ScanDepth {
    fn default() -> Self {
        ScanDepth::Shallow
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OffPeakWindow {
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: String,
}

impl OffPeakWindow {
    pub fn is_in_window(&self, time: &DateTime<Utc>) -> bool {
        let tz: Tz = self.timezone.parse().unwrap_or(chrono_tz::UTC);
        let local = time.with_timezone(&tz);
        let current_hour = local.hour() as i32;
        let start = self.start_hour as i32;
        let end = self.end_hour as i32;

        if start <= end {
            current_hour >= start && current_hour < end
        } else {
            current_hour >= start || current_hour < end
        }
    }
}

impl Default for OffPeakWindow {
    fn default() -> Self {
        Self {
            start_hour: 0,
            end_hour: 6,
            timezone: "UTC".to_string(),
        }
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
    pub scan_depth: ScanDepth,
    pub off_peak_window: Option<OffPeakWindow>,
    #[serde(default)]
    pub scope: Option<crate::config::Scope>,
}

impl TargetConfig {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            target_type: "url".to_string(),
            priority: Priority::Normal,
            schedule: None,
            alert_channels: Vec::new(),
            last_scan: None,
            scan_history: Vec::new(),
            baseline_findings: Vec::new(),
            enabled: true,
            scan_depth: ScanDepth::Shallow,
            off_peak_window: None,
            scope: None,
        }
    }

    /// Convert target_type string to TargetType enum
    pub fn get_target_type(&self) -> crate::tool::request::TargetType {
        match self.target_type.to_lowercase().as_str() {
            "url" => crate::tool::request::TargetType::Url,
            "domain" => crate::tool::request::TargetType::Domain,
            "ip" => crate::tool::request::TargetType::Ip,
            "cidr" => crate::tool::request::TargetType::Cidr,
            "file" => crate::tool::request::TargetType::File,
            _ => {
                tracing::warn!("Unknown target type: {}, defaulting to Url", self.target_type);
                crate::tool::request::TargetType::Url
            }
        }
    }
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
            scan_depth: ScanDepth::default(),
            off_peak_window: None,
            scope: None,
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
    data: Arc<RwLock<PortfolioData>>,
    file_path: Option<PathBuf>,
}

impl TargetPortfolio {
    pub fn file_path(&self) -> Option<&PathBuf> {
        self.file_path.as_ref()
    }

    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(PortfolioData::default())),
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
                data: Arc::new(RwLock::new(data)),
                file_path: Some(path.clone()),
            })
        } else {
            // Retain file_path even if file doesn't exist (Phase 9 fix)
            Ok(Self {
                data: Arc::new(RwLock::new(PortfolioData::default())),
                file_path: Some(path.clone()),
            })
        }
    }

    /// Load from file for testing - bypasses path validation
    #[cfg(test)]
    pub fn load_for_testing(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let data: PortfolioData = serde_json::from_str(&content)?;
            Ok(Self {
                data: Arc::new(RwLock::new(data)),
                file_path: Some(path.clone()),
            })
        } else {
            Ok(Self {
                data: Arc::new(RwLock::new(PortfolioData::default())),
                file_path: Some(path.clone()),
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(ref path) = self.file_path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            let data = self.data.read();
            let content = serde_json::to_string(&*data)?;

            // Atomic write: temp file + rename to avoid partial writes
            let temp_path = path.with_extension("tmp");
            fs::write(&temp_path, &content)?;
            fs::rename(&temp_path, path)?;
        }
        Ok(())
    }

    pub fn add_target(&self, id: String, config: TargetConfig) {
        self.data.write().targets.insert(id, config);
    }

    pub fn remove_target(&self, id: &str) -> bool {
        self.data.write().targets.remove(id).is_some()
    }

    pub fn get_target(&self, id: &str) -> Option<TargetConfig> {
        self.data.read().targets.get(id).cloned()
    }

    pub fn update_target(&self, id: &str, updater: impl FnOnce(&mut TargetConfig)) -> bool {
        if let Some(target) = self.data.write().targets.get_mut(id) {
            updater(target);
            true
        } else {
            false
        }
    }

    pub fn get_all_targets(&self) -> Vec<(String, TargetConfig)> {
        self.data
            .read()
            .targets
            .iter()
            .filter(|(_, c)| c.enabled)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn update_last_scan(&self, id: &str, timestamp: &DateTime<Utc>) {
        if let Some(target) = self.data.write().targets.get_mut(id) {
            target.last_scan = Some(*timestamp);
        }
    }

    pub fn add_scan_record(&self, id: &str, record: ScanRecord) {
        if let Some(target) = self.data.write().targets.get_mut(id) {
            target.scan_history.push(record);
        }
    }

    pub fn set_baseline(&self, id: &str, finding_ids: Vec<String>) {
        if let Some(target) = self.data.write().targets.get_mut(id) {
            target.baseline_findings = finding_ids;
        }
    }

    pub fn targets_count(&self) -> usize {
        self.data.read().targets.len()
    }

    pub fn enabled_count(&self) -> usize {
        self.data.read().targets.values().filter(|t| t.enabled).count()
    }

    /// Reload portfolio data from the file path (if set).
    /// Returns error if file doesn't exist or is invalid.
    /// On success, replaces the live data with the loaded data.
    pub fn reload_from_file(&self) -> Result<()> {
        if let Some(ref path) = self.file_path {
            if !path.exists() {
                return Err(anyhow::anyhow!("Portfolio file does not exist: {:?}", path));
            }
            let content = fs::read_to_string(path)?;
            let new_data: PortfolioData = serde_json::from_str(&content)?;
            *self.data.write() = new_data;
            tracing::info!("Portfolio reloaded successfully from {:?}", path);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No file path set for portfolio"))
        }
    }

    /// Create a TargetPortfolio with a file path without path validation.
    /// For testing purposes only.
    #[cfg(test)]
    pub fn new_for_testing(file_path: PathBuf) -> Self {
        Self {
            data: Arc::new(RwLock::new(PortfolioData::default())),
            file_path: Some(file_path),
        }
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

    #[test]
    fn test_target_config_default() {
        let config = TargetConfig::default();
        assert!(config.target.is_empty());
        assert_eq!(config.target_type, "url");
        assert_eq!(config.priority, Priority::Normal);
        assert!(config.schedule.is_none());
        assert!(config.alert_channels.is_empty());
        assert!(config.last_scan.is_none());
        assert!(config.scan_history.is_empty());
        assert!(config.baseline_findings.is_empty());
        assert!(config.enabled);
    }

    #[test]
    fn test_target_config_with_values() {
        let config = TargetConfig {
            target: "https://example.com".to_string(),
            target_type: "url".to_string(),
            priority: Priority::High,
            schedule: Some("0 0 * * *".to_string()),
            alert_channels: vec!["webhook".to_string()],
            last_scan: Some(chrono::Utc::now()),
            scan_history: vec![],
            baseline_findings: vec!["finding-1".to_string()],
            enabled: true,
            scan_depth: ScanDepth::default(),
            off_peak_window: None,
            scope: None,
        };
        assert_eq!(config.target, "https://example.com");
        assert_eq!(config.priority, Priority::High);
        assert!(config.schedule.is_some());
        assert_eq!(config.alert_channels.len(), 1);
        assert_eq!(config.baseline_findings.len(), 1);
    }

    #[test]
    fn test_portfolio_data_default() {
        let data = PortfolioData::default();
        assert_eq!(data.version, "1.0");
        assert!(data.targets.is_empty());
    }

    #[test]
    fn test_target_portfolio_new() {
        let portfolio = TargetPortfolio::new();
        assert_eq!(portfolio.targets_count(), 0);
        assert_eq!(portfolio.enabled_count(), 0);
    }

    #[test]
    fn test_target_portfolio_add_target() {
        let portfolio = TargetPortfolio::new();
        let config = TargetConfig {
            target: "https://example.com".to_string(),
            priority: Priority::High,
            ..Default::default()
        };
        portfolio.add_target("example.com".to_string(), config);
        assert_eq!(portfolio.targets_count(), 1);
        assert_eq!(portfolio.enabled_count(), 1);
    }

    #[test]
    fn test_target_portfolio_remove_target() {
        let portfolio = TargetPortfolio::new();
        let config = TargetConfig {
            target: "https://example.com".to_string(),
            ..Default::default()
        };
        portfolio.add_target("example.com".to_string(), config);
        assert_eq!(portfolio.targets_count(), 1);

        let removed = portfolio.remove_target("example.com");
        assert!(removed);
        assert_eq!(portfolio.targets_count(), 0);
    }

    #[test]
    fn test_target_portfolio_remove_nonexistent() {
        let portfolio = TargetPortfolio::new();
        let removed = portfolio.remove_target("nonexistent.com");
        assert!(!removed);
    }

    #[test]
    fn test_target_portfolio_get_target() {
        let portfolio = TargetPortfolio::new();
        let config = TargetConfig {
            target: "https://example.com".to_string(),
            priority: Priority::Critical,
            ..Default::default()
        };
        portfolio.add_target("example.com".to_string(), config);

        let retrieved = portfolio.get_target("example.com");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().priority, Priority::Critical);
    }

    #[test]
    fn test_target_portfolio_get_target_nonexistent() {
        let portfolio = TargetPortfolio::new();
        let retrieved = portfolio.get_target("nonexistent.com");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_target_portfolio_get_all_targets() {
        let portfolio = TargetPortfolio::new();

        portfolio.add_target("example1.com".to_string(), TargetConfig {
            target: "https://example1.com".to_string(),
            enabled: true,
            ..Default::default()
        });
        portfolio.add_target("example2.com".to_string(), TargetConfig {
            target: "https://example2.com".to_string(),
            enabled: false,
            ..Default::default()
        });
        portfolio.add_target("example3.com".to_string(), TargetConfig {
            target: "https://example3.com".to_string(),
            enabled: true,
            ..Default::default()
        });

        let targets = portfolio.get_all_targets();
        assert_eq!(targets.len(), 2);
    }

    #[test]
    fn test_target_portfolio_update_last_scan() {
        let portfolio = TargetPortfolio::new();
        portfolio.add_target("example.com".to_string(), TargetConfig::default());

        let now = chrono::Utc::now();
        portfolio.update_last_scan("example.com", &now);

        let target = portfolio.get_target("example.com").unwrap();
        assert!(target.last_scan.is_some());
    }

    #[test]
    fn test_target_portfolio_add_scan_record() {
        let portfolio = TargetPortfolio::new();
        portfolio.add_target("example.com".to_string(), TargetConfig::default());

        let record = ScanRecord {
            scan_id: "scan-123".to_string(),
            scan_type: "recon".to_string(),
            timestamp: chrono::Utc::now(),
            findings_count: 5,
            severity_counts: std::collections::HashMap::new(),
        };
        portfolio.add_scan_record("example.com", record);

        let target = portfolio.get_target("example.com").unwrap();
        assert_eq!(target.scan_history.len(), 1);
    }

    #[test]
    fn test_target_portfolio_set_baseline() {
        let portfolio = TargetPortfolio::new();
        portfolio.add_target("example.com".to_string(), TargetConfig::default());

        let finding_ids = vec!["finding-1".to_string(), "finding-2".to_string()];
        portfolio.set_baseline("example.com", finding_ids.clone());

        let target = portfolio.get_target("example.com").unwrap();
        assert_eq!(target.baseline_findings, finding_ids);
    }

    #[test]
    fn test_scan_record_creation() {
        let record = ScanRecord {
            scan_id: "scan-456".to_string(),
            scan_type: "fuzzer".to_string(),
            timestamp: chrono::Utc::now(),
            findings_count: 10,
            severity_counts: {
                let mut counts = std::collections::HashMap::new();
                counts.insert("Critical".to_string(), 2);
                counts.insert("High".to_string(), 5);
                counts.insert("Medium".to_string(), 3);
                counts
            },
        };
        assert_eq!(record.scan_id, "scan-456");
        assert_eq!(record.findings_count, 10);
        assert_eq!(record.severity_counts.get("Critical"), Some(&2));
    }

    #[test]
    fn test_priority_as_int_values() {
        assert_eq!(Priority::Low.as_int(), 0);
        assert_eq!(Priority::Normal.as_int(), 1);
        assert_eq!(Priority::High.as_int(), 2);
        assert_eq!(Priority::Critical.as_int(), 3);
    }

    #[test]
    fn test_priority_default() {
        assert_eq!(Priority::default(), Priority::Normal);
    }

    // Phase 9: Portfolio Persistence Semantics tests

    #[test]
    fn test_missing_file_load_add_target_save_creates_file() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let portfolio_path = temp_dir.path().join("portfolio.json");

        // Load non-existent file - should retain path (Phase 9 fix)
        let portfolio = TargetPortfolio::load_for_testing(&portfolio_path).unwrap();
        assert!(portfolio.file_path().is_some());

        // Add target and save - should create file
        portfolio.add_target("example.com".to_string(), TargetConfig::new("https://example.com"));
        portfolio.save().unwrap();
        assert!(portfolio_path.exists());
    }

    #[test]
    fn test_existing_file_load_mutate_save_preserves_path() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let portfolio_path = temp_dir.path().join("portfolio.json");

        // Use load_for_testing to set the path, then add target and save
        let portfolio = TargetPortfolio::load_for_testing(&portfolio_path).unwrap();
        portfolio.add_target("example.com".to_string(), TargetConfig::new("https://example.com"));
        portfolio.save().unwrap();

        // Load existing file
        let portfolio = TargetPortfolio::load_for_testing(&portfolio_path).unwrap();
        assert_eq!(portfolio.file_path().unwrap(), &portfolio_path);

        // Save preserves path
        portfolio.save().unwrap();
        assert!(portfolio_path.exists());
    }

    #[test]
    fn test_atomic_write_creates_valid_json() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let portfolio_path = temp_dir.path().join("portfolio.json");

        // Use load_for_testing to set the path, then add target and save
        let portfolio = TargetPortfolio::load_for_testing(&portfolio_path).unwrap();
        portfolio.add_target("example.com".to_string(), TargetConfig::new("https://example.com"));
        portfolio.save().unwrap();

        // Verify file contains valid JSON
        let content = fs::read_to_string(&portfolio_path).unwrap();
        let _parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify no temp file left behind
        let tmp_path = portfolio_path.with_extension("tmp");
        assert!(!tmp_path.exists());
    }

    #[test]
    fn test_invalid_path_validation_rejects_outside_base() {
        let base = std::path::PathBuf::from("/home/sugarwookie/.config/slapper");
        let invalid_path = std::path::Path::new("/etc/passwd");
        let result = crate::utils::validation::validate_path(&base, invalid_path);
        assert!(result.is_err());
    }
}
