use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use slapper_plugin::{Plugin, PluginCheck, PluginConfig, PluginInfo, PluginLanguage, PluginResult};

use super::bridge::RubyPluginClient;
use super::{RubyPlugin, RubyPluginResult};

pub struct PluginLoader {
    client: Arc<RubyPluginClient>,
    plugin_dirs: Vec<PathBuf>,
    loaded_plugins: Vec<RubyPlugin>,
}

impl PluginLoader {
    pub fn new(plugin_dirs: Vec<PathBuf>) -> Result<Self> {
        let client = Arc::new(RubyPluginClient::new()?);

        let dirs = if plugin_dirs.is_empty() {
            vec![PathBuf::from("./plugins")]
        } else {
            plugin_dirs
        };

        Ok(Self {
            client,
            plugin_dirs: dirs,
            loaded_plugins: Vec::new(),
        })
    }

    pub fn add_plugin_dir(&mut self, dir: PathBuf) {
        self.plugin_dirs.push(dir);
    }

    pub fn discover_plugins(&mut self) -> Result<Vec<RubyPlugin>> {
        let mut discovered = Vec::new();

        for dir in &self.plugin_dirs {
            if !dir.exists() {
                continue;
            }

            let entries = fs::read_dir(dir)
                .with_context(|| format!("Failed to read plugin directory: {:?}", dir))?;

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.extension().map(|e| e == "rb").unwrap_or(false) {
                    if let Ok(plugin) = self.client.load_plugin(&path) {
                        tracing::info!(
                            name = %plugin.name,
                            version = %plugin.version,
                            "Discovered Ruby plugin"
                        );
                        discovered.push(plugin);
                    }
                }
            }
        }

        self.loaded_plugins = discovered.clone();

        Ok(discovered)
    }

    pub fn load_plugin(&mut self, path: &Path) -> Result<RubyPlugin> {
        let plugin = self.client.load_plugin(path)?;
        self.loaded_plugins.push(plugin.clone());
        Ok(plugin)
    }

    pub fn run_plugin(&self, name: &str, target: &str) -> Result<RubyPluginResult> {
        let plugin = self
            .loaded_plugins
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", name))?;

        self.client.run_plugin(plugin, target)
    }

    pub fn list_plugins(&self) -> &[RubyPlugin] {
        &self.loaded_plugins
    }

    pub fn get_plugin(&self, name: &str) -> Option<&RubyPlugin> {
        self.loaded_plugins.iter().find(|p| p.name == name)
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new(vec![]).expect("Failed to create plugin loader")
    }
}

/// Adapter that wraps a Ruby plugin and implements the unified `Plugin` trait.
/// Thread-safe via the message-passing RubyPluginClient — no unsafe code needed.
pub struct RubyPluginAdapter {
    plugin: RubyPlugin,
    client: Arc<RubyPluginClient>,
    info: PluginInfo,
}

impl RubyPluginAdapter {
    pub fn new(plugin: RubyPlugin, client: Arc<RubyPluginClient>) -> Self {
        let info = PluginInfo {
            name: plugin.name.clone(),
            version: plugin.version.clone(),
            description: plugin.description.clone().unwrap_or_default(),
            author: plugin.author.clone().unwrap_or_default(),
            tags: vec!["ruby".to_string()],
            language: PluginLanguage::Ruby,
        };
        Self {
            plugin,
            client,
            info,
        }
    }
}

#[async_trait]
impl Plugin for RubyPluginAdapter {
    fn info(&self) -> &PluginInfo {
        &self.info
    }

    fn language(&self) -> PluginLanguage {
        PluginLanguage::Ruby
    }

    fn list_checks(&self) -> Vec<PluginCheck> {
        vec![PluginCheck {
            name: self.plugin.name.clone(),
            check_type: "ruby".to_string(),
            target: None,
            description: self.plugin.description.clone(),
        }]
    }

    async fn run_check(&self, check_name: &str, target: &str) -> Result<PluginResult> {
        let start = Instant::now();

        if check_name != self.plugin.name {
            anyhow::bail!("Unknown check: {}", check_name);
        }

        let ruby_result = self.client.run_plugin(&self.plugin, target)?;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        let findings = ruby_result
            .findings
            .into_iter()
            .map(|f| slapper_plugin::PluginFinding {
                title: f.description.clone(),
                severity: f.severity,
                description: f.description,
                location: f.location,
                evidence: f.evidence,
                cve_ids: Vec::new(),
            })
            .collect();

        let errors = if let Some(err) = ruby_result.error {
            vec![err]
        } else {
            Vec::new()
        };

        Ok(PluginResult {
            plugin_name: self.info.name.clone(),
            success: ruby_result.success,
            findings,
            errors,
            execution_time_ms,
        })
    }

    async fn run(&self, target: &str, _config: &PluginConfig) -> Result<PluginResult> {
        self.run_check(&self.plugin.name, target).await
    }
}
