pub mod python;
pub mod ruby;

pub use python::PythonPluginManager;
#[cfg(feature = "ruby-plugins")]
pub use ruby::RubyPluginManager;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

pub struct PluginManager {
    plugin_dirs: Vec<PathBuf>,
    plugins: HashMap<String, PluginInfo>,
    configs: HashMap<String, PluginConfig>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugin_dirs: Self::default_plugin_dirs(),
            plugins: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    pub fn plugin_dirs(&self) -> &[PathBuf] {
        &self.plugin_dirs
    }

    fn default_plugin_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        if let Some(config_dir) = dirs::config_dir() {
            dirs.push(config_dir.join("slapper").join("plugins"));
        }

        if let Ok(home) = std::env::var("HOME") {
            dirs.push(
                PathBuf::from(home)
                    .join(".config")
                    .join("slapper")
                    .join("plugins"),
            );
            dirs.push(PathBuf::from(home).join(".slapper").join("plugins"));
        }

        dirs.push(PathBuf::from("plugins"));

        dirs
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
                    } else if path.extension().map(|e| e == "rb").unwrap_or(false) {
                        if let Some(info) = self.load_ruby_plugin(&path) {
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
            let line = line.trim();
            if line.starts_with("NAME") || line.starts_with("name") {
                name = line
                    .split('=')
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').to_string());
            } else if line.starts_with("VERSION") || line.starts_with("version") {
                version = line
                    .split('=')
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').to_string());
            } else if line.starts_with("DESCRIPTION") || line.starts_with("description") {
                description = line
                    .split('=')
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').to_string());
            } else if line.starts_with("AUTHOR") || line.starts_with("author") {
                author = line
                    .split('=')
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').to_string());
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

    fn load_ruby_plugin(&self, path: &PathBuf) -> Option<PluginInfo> {
        let content = std::fs::read_to_string(path).ok()?;

        let mut name = None;
        let mut version = None;

        for line in content.lines() {
            if line.contains("NAME") || line.contains("NAME") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        name = Some(line[start + 1..start + 1 + end].to_string());
                    }
                }
            }
            if line.contains("VERSION") || line.contains("VERSION") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        version = Some(line[start + 1..start + 1 + end].to_string());
                    }
                }
            }
        }

        Some(PluginInfo {
            name: name.or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))?,
            version: version.unwrap_or_else(|| "1.0.0".to_string()),
            description: String::new(),
            author: String::new(),
            tags: vec![],
            language: PluginLanguage::Ruby,
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
