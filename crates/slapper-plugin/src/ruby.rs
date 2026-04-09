#[cfg(feature = "ruby-plugins")]
use magnus::{prelude::*, Error, Ruby, TryConvert, Value};

#[cfg(feature = "ruby-plugins")]
use crate::PluginFinding;
use crate::{PluginInfo, PluginLanguage};
#[cfg(feature = "ruby-plugins")]
use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(feature = "ruby-plugins")]
pub struct RubyPluginManager {
    loaded_modules: HashMap<String, RubyPlugin>,
}

#[cfg(feature = "ruby-plugins")]
struct RubyPlugin {
    _name: String,
    _path: PathBuf,
}

#[cfg(feature = "ruby-plugins")]
impl RubyPluginManager {
    pub fn new() -> Self {
        Self {
            loaded_modules: HashMap::new(),
        }
    }

    pub fn load_plugins(&mut self, dir: &PathBuf) -> Result<(), String> {
        if !dir.exists() {
            return Err(format!("Plugin directory does not exist: {:?}", dir));
        }

        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "rb").unwrap_or(false) {
                if let Err(e) = self.load_plugin(&path) {
                    tracing::warn!("Failed to load Ruby plugin {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    fn load_plugin(&mut self, path: &PathBuf) -> Result<(), String> {
        let ruby = Ruby::get().map_err(|e| format!("Ruby VM not initialized: {}", e))?;

        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read plugin: {}", e))?;

        let plugin_name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .ok_or_else(|| "Invalid plugin filename".to_string())?;

        let _: Value = ruby
            .eval(&content)
            .map_err(|e| format!("Failed to execute plugin: {}", e))?;

        self.loaded_modules.insert(
            plugin_name.clone(),
            RubyPlugin {
                _name: plugin_name,
                _path: path.clone(),
            },
        );

        Ok(())
    }

    pub fn get_checks(&self) -> Vec<String> {
        let ruby = match Ruby::get() {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Ruby VM not initialized: {}", e);
                return Vec::new();
            }
        };
        let mut checks = Vec::new();

        if let Ok(register_fn) =
            ruby.eval::<Value>("respond_to?(:register_checks) && register_checks rescue nil")
        {
            if !register_fn.is_nil() {
                if let Ok(result) = ruby.eval::<Vec<String>>("register_checks") {
                    checks = result;
                }
            }
        }

        checks
    }

    pub fn run_check(&self, check_name: &str, target: &str) -> Result<Vec<PluginFinding>, String> {
        let ruby = Ruby::get().map_err(|e| format!("Ruby VM not initialized: {}", e))?;

        let code = format!(
            "if respond_to?(:run_check)\n  run_check('{}', '{}')\nelse\n  nil\nend",
            check_name, target
        );

        let result: Result<Value, Error> = ruby.eval(&code);

        match result {
            Ok(value) => {
                if value.is_nil() {
                    return Ok(vec![]);
                }

                let mut findings = Vec::new();
                if let Ok(array_value) = value.funcall("to_a", ()) {
                    if let Ok(array) = magnus::RArray::try_convert(array_value) {
                        for item_value in array.into_iter() {
                            if let Ok(hash_value) = item_value.funcall("to_h", ()) {
                                if let Ok(hash) = magnus::RHash::try_convert(hash_value) {
                                    let title = hash
                                        .lookup::<_, Value>("title")
                                        .ok()
                                        .and_then(|v| String::try_convert(v).ok())
                                        .unwrap_or_default();

                                    let description = hash
                                        .lookup::<_, Value>("description")
                                        .ok()
                                        .and_then(|v| String::try_convert(v).ok())
                                        .unwrap_or_default();

                                    let severity = hash
                                        .lookup::<_, Value>("severity")
                                        .ok()
                                        .and_then(|v| String::try_convert(v).ok())
                                        .unwrap_or_default();

                                    let location = hash
                                        .lookup::<_, Value>("location")
                                        .ok()
                                        .and_then(|v| String::try_convert(v).ok())
                                        .unwrap_or_default();

                                    findings.push(PluginFinding {
                                        title,
                                        description,
                                        severity,
                                        location,
                                        evidence: None,
                                        cve_ids: vec![],
                                    });
                                }
                            }
                        }
                    }
                }

                Ok(findings)
            }
            Err(e) => Err(format!("Failed to run check: {}", e)),
        }
    }

    pub fn list_loaded(&self) -> Vec<&str> {
        self.loaded_modules.keys().map(|s| s.as_str()).collect()
    }

    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded_modules.contains_key(name)
    }
}

#[cfg(feature = "ruby-plugins")]
impl Default for RubyPluginManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_ruby_plugin_info(path: &PathBuf) -> Option<PluginInfo> {
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
                    description = Some(value.trim_start_matches("Description:").trim().to_string());
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
        language: PluginLanguage::Ruby,
    })
}
