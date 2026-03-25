use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::bridge::RubyBridge;
use super::{RubyPlugin, RubyPluginResult};

pub struct PluginLoader {
    bridge: RubyBridge,
    plugin_dirs: Vec<PathBuf>,
    loaded_plugins: Vec<RubyPlugin>,
}

impl PluginLoader {
    pub fn new() -> Result<Self> {
        let bridge = RubyBridge::new()?;

        let mut plugin_dirs = Vec::new();

        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "slapper", "slapper") {
            plugin_dirs.push(proj_dirs.config_dir().join("plugins"));
        }

        plugin_dirs.push(PathBuf::from("./plugins"));

        Ok(Self {
            bridge,
            plugin_dirs,
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
                    if let Ok(plugin) = self.bridge.load_plugin(&path) {
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
        let plugin = self.bridge.load_plugin(path)?;
        self.loaded_plugins.push(plugin.clone());
        Ok(plugin)
    }

    pub fn run_plugin(&self, name: &str, target: &str) -> Result<RubyPluginResult> {
        let plugin = self
            .loaded_plugins
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", name))?;

        self.bridge.run_plugin(plugin, target)
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
        Self::new().expect("Failed to create plugin loader")
    }
}
