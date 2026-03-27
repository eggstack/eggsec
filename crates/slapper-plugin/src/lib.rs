#[cfg(feature = "python-plugins")]
pub mod python;

#[cfg(feature = "python-plugins")]
pub use python::PythonPluginManager;

pub mod ruby;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub language: PluginLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginLanguage {
    Python,
    Ruby,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            config: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub plugin_name: String,
    pub success: bool,
    pub findings: Vec<PluginFinding>,
    pub errors: Vec<String>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFinding {
    pub title: String,
    pub severity: String,
    pub description: String,
    pub location: String,
    pub evidence: Option<String>,
    pub cve_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PluginCheck {
    pub name: String,
    pub check_type: String,
    pub target: Option<String>,
    pub description: Option<String>,
}

/// Unified trait for all plugin backends (Python, Ruby, etc.)
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Returns metadata about this plugin.
    fn info(&self) -> &PluginInfo;

    /// Returns the language/runtime this plugin uses.
    fn language(&self) -> PluginLanguage;

    /// Lists all checks provided by this plugin.
    fn list_checks(&self) -> Vec<PluginCheck>;

    /// Runs a specific check by name against a target.
    async fn run_check(&self, check_name: &str, target: &str) -> Result<PluginResult>;

    /// Runs the plugin with full configuration.
    async fn run(&self, target: &str, config: &PluginConfig) -> Result<PluginResult>;
}

/// Registry that holds all loaded plugin backends.
pub struct PluginRegistry {
    plugins: Vec<Arc<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin backend.
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// List all registered plugins.
    pub fn list(&self) -> Vec<&PluginInfo> {
        self.plugins.iter().map(|p| p.info()).collect()
    }

    /// Get a plugin by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Plugin>> {
        self.plugins.iter().find(|p| p.info().name == name)
    }

    /// Run a check on all plugins that have it.
    pub async fn run_check(&self, check_name: &str, target: &str) -> Result<Vec<PluginResult>> {
        let mut results = Vec::new();
        for plugin in &self.plugins {
            let checks = plugin.list_checks();
            if checks.iter().any(|c| c.name == check_name) {
                match plugin.run_check(check_name, target).await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        tracing::warn!(
                            plugin = %plugin.info().name,
                            check = %check_name,
                            error = %e,
                            "Plugin check failed"
                        );
                    }
                }
            }
        }
        Ok(results)
    }

    /// Run all plugins against a target.
    pub async fn run_all(&self, target: &str, config: &PluginConfig) -> Result<Vec<PluginResult>> {
        let mut results = Vec::new();
        for plugin in &self.plugins {
            match plugin.run(target, config).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!(
                        plugin = %plugin.info().name,
                        error = %e,
                        "Plugin execution failed"
                    );
                }
            }
        }
        Ok(results)
    }

    /// Returns the number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Returns true if no plugins are registered.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PluginManager {
    plugin_dirs: Vec<PathBuf>,
    plugins: HashMap<String, PluginInfo>,
    configs: HashMap<String, PluginConfig>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        Self::with_config_dir(None)
    }

    pub fn with_config_dir(config_dir: Option<PathBuf>) -> Self {
        Self {
            plugin_dirs: Self::default_plugin_dirs(config_dir),
            plugins: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    pub fn plugin_dirs(&self) -> &[PathBuf] {
        &self.plugin_dirs
    }

    pub fn default_plugin_dirs(config_dir: Option<PathBuf>) -> Vec<PathBuf> {
        let mut dirs_vec = Vec::new();

        if let Some(dir) = config_dir {
            dirs_vec.push(dir);
        }

        #[cfg(feature = "python-plugins")]
        if let Some(config_dir) = dirs::config_dir() {
            dirs_vec.push(config_dir.join("slapper").join("plugins"));
        }

        if let Ok(home) = std::env::var("HOME") {
            dirs_vec.push(
                PathBuf::from(&home)
                    .join(".config")
                    .join("slapper")
                    .join("plugins"),
            );
            dirs_vec.push(PathBuf::from(&home).join(".slapper").join("plugins"));
        }

        dirs_vec.push(PathBuf::from("plugins"));

        dirs_vec
    }

    pub fn add_plugin_dir(&mut self, dir: PathBuf) {
        self.plugin_dirs.push(dir);
    }

    pub fn discover_plugins(&mut self) -> Vec<PluginInfo> {
        let mut discovered = Vec::new();

        for dir in &self.plugin_dirs {
            if !dir.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    if path.extension().map(|e| e == "py").unwrap_or(false) {
                        if let Some(info) = self.load_python_plugin(&path) {
                            self.plugins.insert(info.name.clone(), info.clone());
                            discovered.push(info);
                        }
                    }
                }
            }
        }

        discovered
    }

    fn load_python_plugin(&self, path: &PathBuf) -> Option<PluginInfo> {
        let content = std::fs::read_to_string(path).ok()?;

        let mut name = None;
        let mut version = None;
        let mut description = None;
        let mut author = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                if let Some(value) = trimmed.strip_prefix("# ") {
                    if value.starts_with("Name:") {
                        name = Some(value.trim_start_matches("Name:").trim().to_string());
                    } else if value.starts_with("Version:") {
                        version = Some(value.trim_start_matches("Version:").trim().to_string());
                    } else if value.starts_with("Description:") {
                        description =
                            Some(value.trim_start_matches("Description:").trim().to_string());
                    } else if value.starts_with("Author:") {
                        author = Some(value.trim_start_matches("Author:").trim().to_string());
                    }
                }
            }
        }

        Some(PluginInfo {
            name: name.or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))?,
            version: version.unwrap_or_else(|| "1.0.0".to_string()),
            description: description.unwrap_or_default(),
            author: author.unwrap_or_default(),
            tags: vec![],
            language: PluginLanguage::Python,
        })
    }

    pub fn get_plugin(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }

    pub fn list_plugins(&self) -> Vec<&PluginInfo> {
        self.plugins.values().collect()
    }

    pub fn set_plugin_config(&mut self, name: &str, config: PluginConfig) {
        self.configs.insert(name.to_string(), config);
    }

    pub fn get_plugin_config(&self, name: &str) -> Option<&PluginConfig> {
        self.configs.get(name)
    }
}
