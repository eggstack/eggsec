use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "python-plugins")]
pub mod python;

#[cfg(feature = "python-plugins")]
pub use python::PythonPluginManager;

use futures::future::join_all;

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
    #[serde(default = "default_block_suspicious_plugins")]
    pub block_suspicious_plugins: bool,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_file_size_bytes")]
    pub max_file_size_bytes: usize,
}

fn default_block_suspicious_plugins() -> bool {
    true
}

fn default_timeout_secs() -> u64 {
    300
}

fn default_max_file_size_bytes() -> usize {
    1_000_000
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            config: HashMap::new(),
            block_suspicious_plugins: true,
            timeout_secs: 300,
            max_file_size_bytes: 1_000_000,
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

    /// Unregister a plugin backend by name.
    pub fn unregister(&mut self, name: &str) -> bool {
        let initial_len = self.plugins.len();
        self.plugins.retain(|p| p.info().name != name);
        self.plugins.len() < initial_len
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
        let futures: Vec<_> = self
            .plugins
            .iter()
            .filter(|p| p.list_checks().iter().any(|c| c.name == check_name))
            .map(|plugin| plugin.run_check(check_name, target))
            .collect();

        let results = join_all(futures).await;
        let mut successful = Vec::new();
        for result in results {
            match result {
                Ok(r) => successful.push(r),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Plugin check failed"
                    );
                }
            }
        }
        Ok(successful)
    }

    /// Run all plugins against a target.
    pub async fn run_all(&self, target: &str, config: &PluginConfig) -> Result<Vec<PluginResult>> {
        let futures: Vec<_> = self.plugins.iter().map(|plugin| plugin.run(target, config)).collect();
        let results = join_all(futures).await;
        let mut successful = Vec::new();
        for result in results {
            match result {
                Ok(r) => successful.push(r),
                Err(e) => {
                    tracing::warn!(error = %e, "Plugin execution failed");
                }
            }
        }
        Ok(successful)
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

#[cfg(test)]
mod tests {
    use super::*;

    struct MockPlugin {
        info: PluginInfo,
        checks: Vec<PluginCheck>,
        should_fail: bool,
    }

    impl MockPlugin {
        fn new(name: &str, checks: Vec<PluginCheck>) -> Self {
            Self {
                info: PluginInfo {
                    name: name.to_string(),
                    version: "1.0.0".to_string(),
                    description: "Mock plugin for testing".to_string(),
                    author: "Test".to_string(),
                    tags: vec!["test".to_string()],
                    language: PluginLanguage::Rust,
                },
                checks,
                should_fail: false,
            }
        }

        fn with_failure(name: &str) -> Self {
            Self {
                info: PluginInfo {
                    name: name.to_string(),
                    version: "1.0.0".to_string(),
                    description: "Failing mock plugin".to_string(),
                    author: "Test".to_string(),
                    tags: vec![],
                    language: PluginLanguage::Rust,
                },
                checks: vec![],
                should_fail: true,
            }
        }
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        fn info(&self) -> &PluginInfo {
            &self.info
        }

        fn language(&self) -> PluginLanguage {
            self.info.language
        }

        fn list_checks(&self) -> Vec<PluginCheck> {
            self.checks.clone()
        }

        async fn run_check(&self, check_name: &str, _target: &str) -> Result<PluginResult> {
            if self.should_fail {
                anyhow::bail!("Mock plugin failure")
            }
            Ok(PluginResult {
                plugin_name: self.info.name.clone(),
                success: true,
                findings: vec![PluginFinding {
                    title: format!("Finding from {}", check_name),
                    severity: "medium".to_string(),
                    description: "Test finding".to_string(),
                    location: "/test".to_string(),
                    evidence: None,
                    cve_ids: vec![],
                }],
                errors: vec![],
                execution_time_ms: 10,
            })
        }

        async fn run(&self, _target: &str, _config: &PluginConfig) -> Result<PluginResult> {
            if self.should_fail {
                anyhow::bail!("Mock plugin run failure")
            }
            Ok(PluginResult {
                plugin_name: self.info.name.clone(),
                success: true,
                findings: vec![],
                errors: vec![],
                execution_time_ms: 5,
            })
        }
    }

    #[test]
    fn test_plugin_info_serde() {
        let info = PluginInfo {
            name: "TestPlugin".to_string(),
            version: "2.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: "Tester".to_string(),
            tags: vec!["security".to_string(), "scan".to_string()],
            language: PluginLanguage::Python,
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: PluginInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, info.name);
        assert_eq!(parsed.version, info.version);
        assert_eq!(parsed.language, PluginLanguage::Python);
    }

    #[test]
    fn test_plugin_language_variants() {
        assert_eq!(PluginLanguage::Python, PluginLanguage::Python);
        assert_eq!(PluginLanguage::Ruby, PluginLanguage::Ruby);
        assert_eq!(PluginLanguage::Rust, PluginLanguage::Rust);
        assert_ne!(PluginLanguage::Python, PluginLanguage::Ruby);
    }

    #[test]
    fn test_plugin_config_default() {
        let config = PluginConfig::default();
        assert!(config.enabled);
        assert!(config.config.is_empty());
    }

    #[test]
    fn test_plugin_config_with_values() {
        let mut config = PluginConfig::default();
        config.enabled = false;
        config.config.insert("timeout".to_string(), serde_json::json!(60));

        assert!(!config.enabled);
        assert_eq!(config.config.get("timeout").unwrap().as_i64(), Some(60));
    }

    #[test]
    fn test_plugin_result_structure() {
        let result = PluginResult {
            plugin_name: "test".to_string(),
            success: true,
            findings: vec![PluginFinding {
                title: "SQL Injection".to_string(),
                severity: "high".to_string(),
                description: "Potential SQL injection".to_string(),
                location: "/api/user".to_string(),
                evidence: Some("' OR 1=1--".to_string()),
                cve_ids: vec!["CVE-2021-1234".to_string()],
            }],
            errors: vec![],
            execution_time_ms: 150,
        };

        assert_eq!(result.plugin_name, "test");
        assert!(result.success);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].cve_ids.len(), 1);
    }

    #[test]
    fn test_plugin_check_structure() {
        let check = PluginCheck {
            name: "xss".to_string(),
            check_type: "reflected".to_string(),
            target: Some("https://example.com".to_string()),
            description: Some("Check for XSS vulnerabilities".to_string()),
        };

        assert_eq!(check.name, "xss");
        assert_eq!(check.check_type, "reflected");
        assert!(check.target.is_some());
    }

    #[test]
    fn test_plugin_registry_new() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_plugin_registry_register_and_list() {
        let mut registry = PluginRegistry::new();
        assert!(registry.list().is_empty());

        let plugin = Arc::new(MockPlugin::new("TestPlugin", vec![]));
        registry.register(plugin);

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let infos = registry.list();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].name, "TestPlugin");
    }

    #[test]
    fn test_plugin_registry_get() {
        let mut registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin::new("GetMe", vec![]));
        registry.register(plugin);

        let found = registry.get("GetMe");
        assert!(found.is_some());

        let not_found = registry.get("NonExistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_plugin_registry_multiple_plugins() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(MockPlugin::new("Plugin1", vec![])));
        registry.register(Arc::new(MockPlugin::new("Plugin2", vec![])));
        registry.register(Arc::new(MockPlugin::new("Plugin3", vec![])));

        assert_eq!(registry.len(), 3);
        assert_eq!(registry.list().len(), 3);
    }

    #[test]
    fn test_plugin_registry_run_check_matching() {
        let mut registry = PluginRegistry::new();
        let checks = vec![
            PluginCheck {
                name: "sqli".to_string(),
                check_type: "injection".to_string(),
                target: None,
                description: None,
            },
            PluginCheck {
                name: "xss".to_string(),
                check_type: "reflected".to_string(),
                target: None,
                description: None,
            },
        ];
        registry.register(Arc::new(MockPlugin::new("TestPlugin", checks)));

        let results = futures::executor::block_on(registry.run_check("sqli", "http://test.com"));
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 1);
    }

    #[test]
    fn test_plugin_registry_run_check_no_matching() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(MockPlugin::new("TestPlugin", vec![])));

        let results = futures::executor::block_on(registry.run_check("nonexistent", "http://test.com"));
        assert!(results.is_ok());
        assert!(results.unwrap().is_empty());
    }

    #[test]
    fn test_plugin_registry_run_all() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(MockPlugin::new("Plugin1", vec![])));
        registry.register(Arc::new(MockPlugin::new("Plugin2", vec![])));

        let config = PluginConfig::default();
        let results = futures::executor::block_on(registry.run_all("http://test.com", &config));
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 2);
    }

    #[test]
    fn test_plugin_registry_error_handling() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(MockPlugin::with_failure("FailingPlugin")));

        let results = futures::executor::block_on(registry.run_check("sqli", "http://test.com"));
        assert!(results.is_ok());
        assert!(results.unwrap().is_empty());
    }

    #[test]
    fn test_plugin_manager_new() {
        let manager = PluginManager::new();
        assert!(manager.plugin_dirs().len() >= 1);
    }

    #[test]
    fn test_plugin_manager_with_config_dir() {
        let custom_dir = PathBuf::from("/custom/plugins");
        let manager = PluginManager::with_config_dir(Some(custom_dir.clone()));

        assert_eq!(manager.plugin_dirs()[0], custom_dir);
    }

    #[test]
    fn test_plugin_manager_default_plugin_dirs() {
        let dirs = PluginManager::default_plugin_dirs(None);
        assert!(!dirs.is_empty());
    }

    #[test]
    fn test_plugin_manager_add_plugin_dir() {
        let mut manager = PluginManager::new();
        let initial_len = manager.plugin_dirs().len();

        manager.add_plugin_dir(PathBuf::from("/new/dir"));

        assert_eq!(manager.plugin_dirs().len(), initial_len + 1);
    }

    #[test]
    fn test_plugin_manager_discover_plugins_nonexistent_dir() {
        let mut manager = PluginManager::new();
        manager.add_plugin_dir(PathBuf::from("/nonexistent/path/12345"));

        let discovered = manager.discover_plugins();
        assert!(discovered.is_empty());
    }

    #[test]
    fn test_plugin_manager_discover_plugins_empty_dir() {
        let mut manager = PluginManager::new();
        let temp_dir = std::env::temp_dir().join("slapper_test_empty_").to_string_lossy().to_string();
        std::fs::create_dir_all(&temp_dir).unwrap();
        manager.add_plugin_dir(PathBuf::from(&temp_dir));

        let discovered = manager.discover_plugins();
        assert!(discovered.is_empty());

        std::fs::remove_dir(&temp_dir).ok();
    }

    #[test]
    fn test_plugin_manager_load_python_plugin_parses_info() {
        let manager = PluginManager::new();
        let temp_file = std::env::temp_dir().join("test_plugin_").to_string_lossy().to_string();

        let content = r#"# Name: TestSqlInjection
# Version: 1.5.0
# Description: SQL injection detection plugin
# Author: Test Author

def register_checks():
    return []
"#;
        std::fs::write(&temp_file, content).unwrap();

        let info = manager.load_python_plugin(&PathBuf::from(&temp_file));
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.name, "TestSqlInjection");
        assert_eq!(info.version, "1.5.0");
        assert_eq!(info.description, "SQL injection detection plugin");
        assert_eq!(info.author, "Test Author");
        assert_eq!(info.language, PluginLanguage::Python);

        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_plugin_manager_load_python_plugin_fallback_name() {
        let manager = PluginManager::new();
        let temp_file = std::env::temp_dir().join("my_ruby_check_plugin").to_string_lossy().to_string();

        let content = r#"# No name here
# Version: 2.0.0

def register_checks():
    return []
"#;
        std::fs::write(&temp_file, content).unwrap();

        let info = manager.load_python_plugin(&PathBuf::from(&temp_file));
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.name, "my_ruby_check_plugin");
        assert_eq!(info.version, "2.0.0");

        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_plugin_manager_load_python_plugin_missing_file() {
        let manager = PluginManager::new();
        let info = manager.load_python_plugin(&PathBuf::from("/nonexistent/file.py"));
        assert!(info.is_none());
    }

    #[test]
    fn test_plugin_manager_load_python_plugin_partial_metadata() {
        let manager = PluginManager::new();
        let temp_file = std::env::temp_dir().join("partial_meta_").to_string_lossy().to_string();

        let content = r#"# Name: PartialPlugin
# Just name provided

def register_checks():
    return []
"#;
        std::fs::write(&temp_file, content).unwrap();

        let info = manager.load_python_plugin(&PathBuf::from(&temp_file));
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.name, "PartialPlugin");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.description, "");
        assert_eq!(info.author, "");

        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_plugin_manager_get_set_plugin_config() {
        let mut manager = PluginManager::new();

        manager.set_plugin_config("TestPlugin", PluginConfig::default());
        let config = manager.get_plugin_config("TestPlugin");
        assert!(config.is_some());

        let non_existent = manager.get_plugin_config("NonExistent");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_plugin_manager_list_plugins_empty() {
        let manager = PluginManager::new();
        assert!(manager.list_plugins().is_empty());
    }

    #[test]
    fn test_plugin_manager_get_plugin_none() {
        let manager = PluginManager::new();
        let plugin = manager.get_plugin("NonExistent");
        assert!(plugin.is_none());
    }

    #[test]
    fn test_plugin_info_debug() {
        let info = PluginInfo {
            name: "DebugTest".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Author".to_string(),
            tags: vec![],
            language: PluginLanguage::Python,
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("DebugTest"));
    }

    #[test]
    fn test_plugin_result_with_errors() {
        let result = PluginResult {
            plugin_name: "Test".to_string(),
            success: false,
            findings: vec![],
            errors: vec!["Check 1 failed".to_string(), "Check 2 failed".to_string()],
            execution_time_ms: 100,
        };

        assert!(!result.success);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_plugin_findings_with_cves() {
        let finding = PluginFinding {
            title: "Remote Code Execution".to_string(),
            severity: "critical".to_string(),
            description: "RCE via deserialization".to_string(),
            location: "/api/deserialize".to_string(),
            evidence: Some("pickle.loads(user_input)".to_string()),
            cve_ids: vec!["CVE-2021-1234".to_string(), "CVE-2021-5678".to_string()],
        };

        assert_eq!(finding.cve_ids.len(), 2);
        assert!(finding.evidence.is_some());
    }
}
