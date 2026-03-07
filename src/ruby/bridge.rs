use anyhow::{anyhow, Context, Result};
use std::path::Path;

use super::{RubyPlugin, RubyPluginResult};

#[cfg(feature = "ruby-plugins")]
use magnus::{prelude::*, Error, Ruby};

pub struct RubyBridge {
    #[cfg(feature = "ruby-plugins")]
    ruby: Ruby,
    loaded: bool,
}

impl RubyBridge {
    #[cfg(feature = "ruby-plugins")]
    pub fn new() -> Result<Self> {
        let ruby = Ruby::init(|ruby| {
            super::api::register_api(ruby)?;
            Ok(())
        })
        .map_err(|e| anyhow!("Failed to initialize Ruby: {}", e))?;

        Ok(Self { ruby, loaded: true })
    }

    #[cfg(not(feature = "ruby-plugins"))]
    pub fn new() -> Result<Self> {
        Ok(Self { loaded: false })
    }

    #[cfg(feature = "ruby-plugins")]
    pub fn load_plugin(&self, path: &Path) -> Result<RubyPlugin> {
        use magnus::value::ReprValue;

        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid plugin path"))?;

        self.ruby
            .require(path_str)
            .map_err(|e| anyhow!("Failed to load plugin: {}", e))?;

        let plugin_class = self
            .ruby
            .module()
            .const_get::<_, magnus::RModule>("Slapper")
            .map_err(|e| anyhow!("Slapper module not found: {}", e))?
            .const_get::<_, magnus::RModule>("Plugin")
            .map_err(|e| anyhow!("Plugin module not found: {}", e))?;

        let name: String = plugin_class
            .const_get("NAME")
            .map_err(|e| anyhow!("Plugin NAME not found: {}", e))?;

        let version: String = plugin_class
            .const_get("VERSION")
            .map_err(|e| anyhow!("Plugin VERSION not found: {}", e))?;

        Ok(RubyPlugin::new(name, version, path.to_path_buf()))
    }

    #[cfg(not(feature = "ruby-plugins"))]
    pub fn load_plugin(&self, _path: &Path) -> Result<RubyPlugin> {
        anyhow::bail!("Ruby plugins require 'ruby-plugins' feature");
    }

    #[cfg(feature = "ruby-plugins")]
    pub fn run_plugin(&self, plugin: &RubyPlugin, target: &str) -> Result<RubyPluginResult> {
        use magnus::value::ReprValue;

        let plugin_class = self
            .ruby
            .module()
            .const_get::<_, magnus::RModule>("Slapper")
            .map_err(|e| anyhow!("Slapper module not found: {}", e))?
            .const_get::<_, magnus::RModule>("Plugin")
            .map_err(|e| anyhow!("Plugin module not found: {}", e))?;

        let instance = plugin_class
            .new_instance(())
            .map_err(|e| anyhow!("Failed to instantiate plugin: {}", e))?;

        let result: magnus::Value = instance
            .funcall("run", (target,))
            .map_err(|e| anyhow!("Failed to run plugin: {}", e))?;

        let hash = result
            .to_r_hash()
            .map_err(|e| anyhow!("Plugin did not return a hash: {}", e))?;

        let success: bool = hash
            .lookup("success")
            .map_err(|e| anyhow!("Missing success field: {}", e))?
            .ok_or_else(|| anyhow!("success field is nil"))?
            .try_convert()
            .map_err(|e| anyhow!("Invalid success value: {}", e))?;

        let message: String = hash
            .lookup("message")
            .map_err(|e| anyhow!("Missing message field: {}", e))?
            .ok_or_else(|| anyhow!("message field is nil"))?
            .try_convert()
            .map_err(|e| anyhow!("Invalid message value: {}", e))?;

        Ok(RubyPluginResult {
            success,
            message,
            findings: vec![],
            error: None,
        })
    }

    #[cfg(not(feature = "ruby-plugins"))]
    pub fn run_plugin(&self, _plugin: &RubyPlugin, _target: &str) -> Result<RubyPluginResult> {
        anyhow::bail!("Ruby plugins require 'ruby-plugins' feature");
    }

    pub fn is_available(&self) -> bool {
        self.loaded
    }
}

impl Default for RubyBridge {
    fn default() -> Self {
        Self::new().expect("Failed to create Ruby bridge")
    }
}
