use anyhow::Result;
use notify::RecommendedWatcher;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::agent::portfolio::TargetPortfolio;
use crate::config::SlapperConfig;

pub trait ConfigReloader: Send + Sync {
    fn reload(&self, path: &Path) -> Result<()>;
}

pub struct ConfigWatcher {
    watcher: Debouncer<RecommendedWatcher>,
}

impl ConfigWatcher {
    pub fn new<P: AsRef<Path>>(
        config_paths: Vec<P>,
        reloader: Arc<dyn ConfigReloader>,
    ) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel(100);

        let watcher = new_debouncer(Duration::from_secs(1), move |res: DebounceEventResult| {
            if let Err(e) = tx.blocking_send(res) {
                tracing::error!("Failed to send debounced event: {}", e);
            }
        })?;

        let mut watcher = watcher;

        for path in &config_paths {
            let path = path.as_ref();
            if path.exists() {
                watcher.watcher().watch(path, RecursiveMode::NonRecursive)?;
                tracing::debug!("Watching config file: {:?}", path);
            } else if let Some(parent) = path.parent() {
                // Watch parent directory for files that don't exist yet
                if parent.exists() {
                    watcher.watcher().watch(parent, RecursiveMode::NonRecursive)?;
                    tracing::debug!("Watching parent directory for future file: {:?}", path);
                }
            }
        }

        let reloader_clone = reloader;
        tokio::spawn(async move {
            while let Some(result) = rx.recv().await {
                match result {
                    Ok(events) => {
                        for event in events {
                            if matches!(event.kind, notify_debouncer_mini::DebouncedEventKind::Any) {
                                tracing::info!("Config file changed: {:?}", event.path);
                                if let Err(e) = reloader_clone.reload(&event.path) {
                                    tracing::error!("Failed to reload config: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Watch error: {:?}", e);
                    }
                }
            }
        });

        Ok(Self { watcher })
    }

    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.watcher
            .watcher()
            .watch(path.as_ref(), RecursiveMode::NonRecursive)?;
        tracing::debug!("Now watching: {:?}", path.as_ref());
        Ok(())
    }
}

pub struct SlapperConfigReloader {
    portfolio: Option<TargetPortfolio>,
    portfolio_path: Option<PathBuf>,
    config_path: Option<PathBuf>,
}

impl SlapperConfigReloader {
    pub fn new(
        portfolio: Option<TargetPortfolio>,
        portfolio_path: Option<PathBuf>,
        config_path: Option<PathBuf>,
    ) -> Self {
        Self {
            portfolio,
            portfolio_path,
            config_path,
        }
    }
}

impl ConfigReloader for SlapperConfigReloader {
    fn reload(&self, path: &Path) -> Result<()> {
        if let Some(ref portfolio_path) = self.portfolio_path {
            if path == portfolio_path {
                tracing::info!("Portfolio config changed, reloading...");
                return self.reload_portfolio();
            }
        }
        if let Some(ref config_path) = self.config_path {
            if path == config_path {
                tracing::info!("Main config changed, reloading...");
                return self.reload_main_config();
            }
        }
        Ok(())
    }
}

impl SlapperConfigReloader {
    fn reload_portfolio(&self) -> Result<()> {
        if let Some(ref portfolio) = self.portfolio {
            // Use the new reload_from_file method which handles validation
            match portfolio.reload_from_file() {
                Ok(()) => {
                    tracing::info!("Portfolio reloaded successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to reload portfolio: {}", e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn reload_main_config(&self) -> Result<()> {
        if let Some(ref config_path) = self.config_path {
            if !config_path.exists() {
                tracing::warn!("Config file no longer exists: {:?}", config_path);
                return Ok(());
            }
            // Attempt to reload and validate the config
            match SlapperConfig::load(config_path) {
                Ok(_new_config) => {
                    tracing::info!("Main config reloaded successfully from {:?}", config_path);
                    // Note: Agent would need to apply relevant config changes
                    // For now, we just validate that the config is parseable
                }
                Err(e) => {
                    tracing::error!("Failed to reload config from {:?}: {}", config_path, e);
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::portfolio::TargetPortfolio;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_slapper_config_reloader_new_signature() {
        let portfolio = TargetPortfolio::new();
        let portfolio_path = PathBuf::from("/tmp/portfolio.json");
        let config_path = PathBuf::from("/tmp/slapper.toml");

        let reloader = SlapperConfigReloader::new(
            Some(portfolio),
            Some(portfolio_path.clone()),
            Some(config_path.clone()),
        );

        // Test that reload doesn't error on non-matching paths
        assert!(reloader.reload(&PathBuf::from("/tmp/other.toml")).is_ok());
    }

    #[test]
    fn test_portfolio_file_change_reloads_targets() {
        let temp_dir = TempDir::new().unwrap();
        let portfolio_path = temp_dir.path().join("portfolio.json");

        // Create initial portfolio with one target
        let portfolio_data = serde_json::json!({
            "version": "1.0",
            "targets": {
                "example.com": {
                    "target": "https://example.com",
                    "target_type": "url",
                    "priority": "normal",
                    "schedule": null,
                    "alert_channels": [],
                    "last_scan": null,
                    "scan_history": [],
                    "baseline_findings": [],
                    "enabled": true,
                    "scan_depth": "shallow",
                    "off_peak_window": null,
                    "scope": null
                }
            }
        });
        fs::write(&portfolio_path, serde_json::to_string_pretty(&portfolio_data).unwrap()).unwrap();

        // Create portfolio with testing helper (bypasses path validation)
        let portfolio = TargetPortfolio::new_for_testing(portfolio_path.clone());
        // Load initial data
        portfolio.reload_from_file().unwrap();
        assert_eq!(portfolio.targets_count(), 1);

        // Create reloader with this portfolio
        let reloader = SlapperConfigReloader::new(
            Some(portfolio.clone()),
            Some(portfolio_path.clone()),
            None,
        );

        // Add a new target to the portfolio file
        let mut data: serde_json::Value = serde_json::from_str(&fs::read_to_string(&portfolio_path).unwrap()).unwrap();
        data["targets"]["test.com"] = serde_json::json!({
            "target": "https://test.com",
            "target_type": "url",
            "priority": "high",
            "schedule": null,
            "alert_channels": [],
            "last_scan": null,
            "scan_history": [],
            "baseline_findings": [],
            "enabled": true,
            "scan_depth": "shallow",
            "off_peak_window": null,
            "scope": null
        });
        fs::write(&portfolio_path, serde_json::to_string_pretty(&data).unwrap()).unwrap();

        // Reload should pick up the new target
        let result = reloader.reload(&portfolio_path);
        assert!(result.is_ok());
        assert_eq!(portfolio.targets_count(), 2);
    }

    #[test]
    fn test_invalid_portfolio_json_leaves_previous_state() {
        let temp_dir = TempDir::new().unwrap();
        let portfolio_path = temp_dir.path().join("portfolio.json");

        // Create valid portfolio
        let portfolio_data = serde_json::json!({
            "version": "1.0",
            "targets": {
                "example.com": {
                    "target": "https://example.com",
                    "target_type": "url",
                    "priority": "normal",
                    "schedule": null,
                    "alert_channels": [],
                    "last_scan": null,
                    "scan_history": [],
                    "baseline_findings": [],
                    "enabled": true,
                    "scan_depth": "shallow",
                    "off_peak_window": null,
                    "scope": null
                }
            }
        });
        fs::write(&portfolio_path, serde_json::to_string_pretty(&portfolio_data).unwrap()).unwrap();

        // Create portfolio with testing helper
        let portfolio = TargetPortfolio::new_for_testing(portfolio_path.clone());
        portfolio.reload_from_file().unwrap();
        assert_eq!(portfolio.targets_count(), 1);

        // Create reloader
        let reloader = SlapperConfigReloader::new(
            Some(portfolio.clone()),
            Some(portfolio_path.clone()),
            None,
        );

        // Write invalid JSON to the portfolio file
        fs::write(&portfolio_path, "invalid json{").unwrap();

        // Reload should fail
        let result = reloader.reload(&portfolio_path);
        assert!(result.is_err());

        // But the live portfolio should still have the original data
        assert_eq!(portfolio.targets_count(), 1);
    }

    #[test]
    fn test_missing_file_at_startup_can_be_loaded_later() {
        let temp_dir = TempDir::new().unwrap();
        let portfolio_path = temp_dir.path().join("portfolio.json");

        // Don't create the file yet - simulate missing file at startup
        let portfolio = TargetPortfolio::new_for_testing(portfolio_path.clone());
        
        // Initially, reload should fail because file doesn't exist
        let result = portfolio.reload_from_file();
        assert!(result.is_err());

        // Create reloader
        let reloader = SlapperConfigReloader::new(
            Some(portfolio.clone()),
            Some(portfolio_path.clone()),
            None,
        );

        // Now create the portfolio file
        let portfolio_data = serde_json::json!({
            "version": "1.0",
            "targets": {
                "example.com": {
                    "target": "https://example.com",
                    "target_type": "url",
                    "priority": "normal",
                    "schedule": null,
                    "alert_channels": [],
                    "last_scan": null,
                    "scan_history": [],
                    "baseline_findings": [],
                    "enabled": true,
                    "scan_depth": "shallow",
                    "off_peak_window": null,
                    "scope": null
                }
            }
        });
        fs::write(&portfolio_path, serde_json::to_string_pretty(&portfolio_data).unwrap()).unwrap();

        // Now reload should succeed
        let result = reloader.reload(&portfolio_path);
        assert!(result.is_ok());
        assert_eq!(portfolio.targets_count(), 1);
    }
}
