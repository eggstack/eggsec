use anyhow::{anyhow, Result};
use std::path::Path;

use super::{RubyPlugin, RubyPluginResult};

#[cfg(feature = "ruby-plugins")]
use magnus::{prelude::*, Ruby};

pub struct RubyBridge {
    #[cfg(feature = "ruby-plugins")]
    ruby: Ruby,
    loaded: bool,
}

// Safety: Ruby runtime is thread-safe when GIL is held.
// The Ruby type contains PhantomData<*mut ()> which prevents auto-Send+Sync,
// but magnus ensures proper GIL handling for thread safety.
#[cfg(feature = "ruby-plugins")]
unsafe impl Send for RubyBridge {}
#[cfg(feature = "ruby-plugins")]
unsafe impl Sync for RubyBridge {}

impl RubyBridge {
    #[cfg(feature = "ruby-plugins")]
    pub fn new() -> Result<Self> {
        // Initialize Ruby runtime
        let init_result = magnus::Ruby::init(|ruby| {
            super::api::register_api(ruby)?;
            Ok(())
        });

        match init_result {
            Ok(()) => {
                // Get a Ruby handle after initialization
                // Ruby is a ZST marker type, we can get it via get_with on any value
                let ruby = unsafe { magnus::Ruby::get_unchecked() };
                Ok(Self { ruby, loaded: true })
            }
            Err(e) => Err(anyhow!("Failed to initialize Ruby: {}", e)),
        }
    }

    #[cfg(not(feature = "ruby-plugins"))]
    pub fn new() -> Result<Self> {
        Ok(Self { loaded: false })
    }

    #[cfg(feature = "ruby-plugins")]
    pub fn load_plugin(&self, path: &Path) -> Result<RubyPlugin> {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid plugin path"))?;

        self.ruby
            .require(path_str)
            .map_err(|e| anyhow!("Failed to load plugin: {}", e))?;

        let plugin_class = self
            .ruby
            .class_object()
            .const_get::<_, magnus::RModule>("Slapper")
            .map_err(|e| anyhow!("Slapper module not found: {}", e))?
            .const_get::<_, magnus::RClass>("Plugin")
            .map_err(|e| anyhow!("Plugin class not found: {}", e))?;

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
    pub fn run_plugin(&self, _plugin: &RubyPlugin, target: &str) -> Result<RubyPluginResult> {
        let plugin_class = self
            .ruby
            .class_object()
            .const_get::<_, magnus::RModule>("Slapper")
            .map_err(|e| anyhow!("Slapper module not found: {}", e))?
            .const_get::<_, magnus::RClass>("Plugin")
            .map_err(|e| anyhow!("Plugin class not found: {}", e))?;

        let instance = plugin_class
            .new_instance(())
            .map_err(|e| anyhow!("Failed to instantiate plugin: {}", e))?;

        let result: magnus::Value = instance
            .funcall("run", (target,))
            .map_err(|e| anyhow!("Failed to run plugin: {}", e))?;

        let hash: magnus::RHash = magnus::TryConvert::try_convert(result)
            .map_err(|e| anyhow!("Plugin did not return a hash: {}", e))?;

        let success: bool = bool::try_convert(
            hash.lookup::<_, magnus::Value>("success")
                .map_err(|e| anyhow!("Missing success field: {}", e))?,
        )
        .map_err(|e: magnus::Error| anyhow!("Invalid success value: {}", e))?;

        // Accept both {success, message} and {success, target, results} formats
        let message: Option<String> = hash
            .lookup::<_, magnus::Value>("message")
            .ok()
            .and_then(|v| String::try_convert(v).ok())
            .or_else(|| {
                hash.lookup::<_, magnus::Value>("target")
                    .ok()
                    .and_then(|v| String::try_convert(v).ok())
            });
        let message = message.unwrap_or_default();

        // Extract findings from either "findings" or "results" key
        let findings: Vec<super::RubyPluginFinding> = hash
            .lookup::<_, magnus::Value>("findings")
            .ok()
            .and_then(|v| {
                magnus::RArray::try_convert(v)
                    .ok()
                    .map(|arr| extract_findings_from_array(arr))
            })
            .or_else(|| {
                hash.lookup::<_, magnus::Value>("results")
                    .ok()
                    .and_then(|v| {
                        magnus::RArray::try_convert(v)
                            .ok()
                            .map(|arr| extract_findings_from_array(arr))
                    })
            })
            .unwrap_or_default();

        let error: Option<String> = hash
            .lookup::<_, magnus::Value>("error")
            .ok()
            .and_then(|v| String::try_convert(v).ok());

        Ok(RubyPluginResult {
            success,
            message,
            findings,
            error,
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

#[cfg(feature = "ruby-plugins")]
fn extract_findings_from_array(arr: magnus::RArray) -> Vec<super::RubyPluginFinding> {
    use magnus::prelude::*;
    arr.into_iter()
        .filter_map(|item| {
            let item_hash: magnus::RHash = magnus::TryConvert::try_convert(item).ok()?;
            let severity: String = item_hash
                .lookup::<_, magnus::Value>("severity")
                .ok()
                .and_then(|v| String::try_convert(v).ok())
                .unwrap_or_default();
            let finding_type: String = item_hash
                .lookup::<_, magnus::Value>("type")
                .ok()
                .and_then(|v| String::try_convert(v).ok())
                .or_else(|| {
                    item_hash
                        .lookup::<_, magnus::Value>("finding_type")
                        .ok()
                        .and_then(|v| String::try_convert(v).ok())
                })
                .unwrap_or_default();
            let description: String = item_hash
                .lookup::<_, magnus::Value>("description")
                .ok()
                .and_then(|v| String::try_convert(v).ok())
                .unwrap_or_default();
            let location: String = item_hash
                .lookup::<_, magnus::Value>("location")
                .ok()
                .and_then(|v| String::try_convert(v).ok())
                .unwrap_or_default();
            let evidence: Option<String> = item_hash
                .lookup::<_, magnus::Value>("evidence")
                .ok()
                .and_then(|v| String::try_convert(v).ok());
            Some(super::RubyPluginFinding {
                severity,
                finding_type,
                description,
                location,
                evidence,
                references: vec![],
            })
        })
        .collect()
}
