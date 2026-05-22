use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[cfg(feature = "ruby-plugins")]
use magnus::prelude::*;
#[cfg(feature = "ruby-plugins")]
use magnus::Ruby;

use super::{RubyPlugin, RubyPluginResult};
use slapper_plugin::security::validate_ruby_plugin;

const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Internal bridge that owns the Ruby VM. NOT Send/Sync — lives on one thread only.
pub struct RubyBridge {
    #[cfg(feature = "ruby-plugins")]
    ruby: Ruby,
    loaded: bool,
    block_suspicious_plugins: bool,
}

/// Messages sent to the Ruby VM thread.
enum RubyRequest {
    LoadPlugin {
        path: PathBuf,
        resp: mpsc::Sender<Result<RubyPlugin>>,
    },
    RunPlugin {
        plugin: RubyPlugin,
        target: String,
        resp: mpsc::Sender<Result<RubyPluginResult>>,
    },
}

/// Thread-safe client for the Ruby VM. Send + Sync via message-passing.
pub struct RubyPluginClient {
    tx: mpsc::Sender<RubyRequest>,
    _thread: thread::JoinHandle<()>,
}

impl RubyPluginClient {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let _thread = thread::Builder::new()
            .name("ruby-vm".into())
            .spawn(move || {
                let bridge = match RubyBridge::new() {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::error!("Failed to initialize Ruby VM: {}", e);
                        return;
                    }
                };

                for msg in rx {
                    match msg {
                        RubyRequest::LoadPlugin { path, resp } => {
                            if resp.send(bridge.load_plugin(&path)).is_err() {
                                tracing::warn!("Ruby VM response channel dropped for load_plugin");
                            }
                        }
                        RubyRequest::RunPlugin {
                            plugin,
                            target,
                            resp,
                        } => {
                            if resp.send(bridge.run_plugin(&plugin, &target)).is_err() {
                                tracing::warn!("Ruby VM response channel dropped for run_plugin");
                            }
                        }
                    }
                }
            })
            .map_err(|e| anyhow!("Failed to spawn Ruby VM thread: {}", e))?;

        Ok(Self { tx, _thread })
    }

    pub fn load_plugin(&self, path: &Path) -> Result<RubyPlugin> {
        self.load_plugin_with_timeout(path, DEFAULT_TIMEOUT_SECS)
    }

    pub fn load_plugin_with_timeout(&self, path: &Path, timeout_secs: u64) -> Result<RubyPlugin> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(RubyRequest::LoadPlugin {
                path: path.to_path_buf(),
                resp: tx,
            })
            .map_err(|_| anyhow!("Ruby VM thread has shut down"))?;
        match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
            Ok(result) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                anyhow::bail!("Plugin loading timed out after {} seconds", timeout_secs)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                anyhow::bail!("Ruby VM thread has shut down")
            }
        }
    }

    pub fn run_plugin(
        &self,
        plugin: &RubyPlugin,
        target: &str,
        timeout_secs: u64,
    ) -> Result<RubyPluginResult> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(RubyRequest::RunPlugin {
                plugin: plugin.clone(),
                target: target.to_string(),
                resp: tx,
            })
            .map_err(|_| anyhow!("Ruby VM thread has shut down"))?;
        match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
            Ok(result) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                anyhow::bail!("Plugin execution timed out after {} seconds", timeout_secs)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                anyhow::bail!("Ruby VM thread has shut down")
            }
        }
    }

    pub fn close(&self) {
        drop(&self.tx);
    }

    pub fn get_timeout(&self) -> u64 {
        DEFAULT_TIMEOUT_SECS
    }
}

impl RubyBridge {
    #[cfg(feature = "ruby-plugins")]
    fn new() -> Result<Self> {
        let init_result = magnus::Ruby::init(|ruby| {
            super::api::register_api(ruby)?;
            Ok(())
        });

        match init_result {
            Ok(()) => {
                let ruby = unsafe { magnus::Ruby::get_unchecked() };
                Ok(Self {
                    ruby,
                    loaded: true,
                    block_suspicious_plugins: true,
                })
            }
            Err(e) => Err(anyhow!("Failed to initialize Ruby: {}", e)),
        }
    }

    #[cfg(not(feature = "ruby-plugins"))]
    fn new() -> Result<Self> {
        Ok(Self {
            loaded: false,
            block_suspicious_plugins: true,
        })
    }

    #[cfg(feature = "ruby-plugins")]
    fn with_block_suspicious_plugins(block: bool) -> Result<Self> {
        let init_result = magnus::Ruby::init(|ruby| {
            super::api::register_api(ruby)?;
            Ok(())
        });

        match init_result {
            Ok(()) => {
                let ruby = unsafe { magnus::Ruby::get_unchecked() };
                Ok(Self {
                    ruby,
                    loaded: true,
                    block_suspicious_plugins: block,
                })
            }
            Err(e) => Err(anyhow!("Failed to initialize Ruby: {}", e)),
        }
    }

    #[cfg(feature = "ruby-plugins")]
    fn load_plugin(&self, path: &Path) -> Result<RubyPlugin> {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid plugin path"))?;

        let plugin_content = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read plugin file: {}", e))?;

        validate_ruby_plugin(&plugin_content, self.block_suspicious_plugins)?;

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

        let author: Option<String> = plugin_class
            .const_get::<_, magnus::Value>("AUTHOR")
            .ok()
            .and_then(|v| String::try_convert(v).ok());

        let description: Option<String> = plugin_class
            .const_get::<_, magnus::Value>("DESCRIPTION")
            .ok()
            .and_then(|v| String::try_convert(v).ok());

        Ok(RubyPlugin::new_with_meta(
            name,
            version,
            path.to_path_buf(),
            author,
            description,
        ))
    }

    #[cfg(not(feature = "ruby-plugins"))]
    fn load_plugin(&self, _path: &Path) -> Result<RubyPlugin> {
        anyhow::bail!("Ruby plugins require 'ruby-plugins' feature");
    }

    #[cfg(feature = "ruby-plugins")]
    fn load_plugin_with_timeout(&self, path: &Path, timeout_secs: u64) -> Result<RubyPlugin> {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid plugin path"))?;

        let plugin_content = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read plugin file: {}", e))?;

        validate_ruby_plugin(&plugin_content, self.block_suspicious_plugins)?;

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

        let author: Option<String> = plugin_class
            .const_get::<_, magnus::Value>("AUTHOR")
            .ok()
            .and_then(|v| String::try_convert(v).ok());

        let description: Option<String> = plugin_class
            .const_get::<_, magnus::Value>("DESCRIPTION")
            .ok()
            .and_then(|v| String::try_convert(v).ok());

        let _ = timeout_secs;
        Ok(RubyPlugin::new_with_meta(
            name,
            version,
            path.to_path_buf(),
            author,
            description,
        ))
    }

    #[cfg(feature = "ruby-plugins")]
    fn run_plugin(&self, _plugin: &RubyPlugin, target: &str) -> Result<RubyPluginResult> {
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

        let findings: Vec<super::RubyPluginFinding> = hash
            .lookup::<_, magnus::Value>("findings")
            .ok()
            .and_then(|v| {
                magnus::RArray::try_convert(v)
                    .ok()
                    .map(extract_findings_from_array)
            })
            .or_else(|| {
                hash.lookup::<_, magnus::Value>("results")
                    .ok()
                    .and_then(|v| {
                        magnus::RArray::try_convert(v)
                            .ok()
                            .map(extract_findings_from_array)
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
    fn run_plugin(&self, _plugin: &RubyPlugin, _target: &str) -> Result<RubyPluginResult> {
        anyhow::bail!("Ruby plugins require 'ruby-plugins' feature");
    }

    #[allow(dead_code)]
    pub fn is_available(&self) -> bool {
        self.loaded
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
