use anyhow::Result;
use async_trait::async_trait;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;
use std::path::Path;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use regex::Regex;

use super::validation::validate_plugin_path;
use super::{Plugin, PluginCheck, PluginConfig, PluginInfo, PluginLanguage, PluginResult, HealthStatus};

#[cfg(feature = "python-plugins")]
use tokio::time::timeout;

const MAX_PLUGIN_SIZE_BYTES: usize = 1_000_000;
const MAX_JSON_SIZE_BYTES: usize = 100_000;

static SUSPICIOUS_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"os\.system").unwrap(),
        Regex::new(r"subprocess").unwrap(),
        Regex::new(r"socket").unwrap(),
        Regex::new(r"eval\(").unwrap(),
        Regex::new(r"\bexec\b").unwrap(),
        Regex::new(r"\bfork\b").unwrap(),
        Regex::new(r"__import__").unwrap(),
        Regex::new(r"\bopen\(").unwrap(),
        Regex::new(r"pty\.spawn").unwrap(),
        Regex::new(r"os\.popen").unwrap(),
        Regex::new(r"multiprocessing\.Process").unwrap(),
        Regex::new(r"ctypes").unwrap(),
        Regex::new(r"importlib").unwrap(),
        Regex::new(r"getattr\(").unwrap(),
        Regex::new(r"chr\(").unwrap(),
        Regex::new(r"\\x[0-9a-fA-F]{2}").unwrap(),
        Regex::new(r"\\u[0-9a-fA-F]{4}").unwrap(),
        Regex::new(r"\\[0-7]{3,}").unwrap(),
    ]
});

fn validate_python_plugin(content: &str, block_suspicious_plugins: bool) -> Result<()> {
    if content.len() > MAX_PLUGIN_SIZE_BYTES {
        anyhow::bail!("Plugin exceeds maximum size of {} bytes", MAX_PLUGIN_SIZE_BYTES);
    }

    let mut suspicious_found = Vec::new();
    for pattern in SUSPICIOUS_PATTERNS.iter() {
        if pattern.is_match(content) {
            suspicious_found.push(pattern.as_str());
        }
    }

    if !suspicious_found.is_empty() {
        if block_suspicious_plugins {
            anyhow::bail!(
                "Plugin contains suspicious patterns and blocking is enabled: {}",
                suspicious_found.join(", ")
            );
        } else {
            tracing::warn!(
                "Plugin contains suspicious patterns (allowing due to config): {}",
                suspicious_found.join(", ")
            );
        }
    }

    Ok(())
}

pub struct PythonPluginManager {
    plugins: Vec<LoadedPlugin>,
    info: PluginInfo,
    block_suspicious_plugins: bool,
    checks_cache: std::sync::OnceLock<Vec<PluginCheck>>,
}

struct LoadedPlugin {
    name: String,
    module: Py<PyAny>,
    /// Class-based plugins extracted from PLUGINS list
    class_plugins: Vec<ClassPlugin>,
}

struct ClassPlugin {
    name: String,
    class: Py<PyAny>,
}

/// Convert a Python value to a JSON value.
fn py_value_to_json(_py: Python<'_>, val: &pyo3::Bound<'_, pyo3::PyAny>) -> serde_json::Value {
    if let Ok(s) = val.extract::<String>() {
        serde_json::Value::String(s)
    } else if let Ok(b) = val.extract::<bool>() {
        serde_json::Value::Bool(b)
    } else if let Ok(i) = val.extract::<i64>() {
        serde_json::Value::Number(serde_json::Number::from(i))
    } else if let Ok(f) = val.extract::<f64>() {
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(list) = val.downcast::<PyList>() {
        let items: Vec<serde_json::Value> =
            list.iter().map(|item| py_value_to_json(_py, &item)).collect();
        serde_json::Value::Array(items)
    } else if let Ok(dict) = val.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            if let Ok(key) = k.extract::<String>() {
                map.insert(key, py_value_to_json(_py, &v));
            }
        }
        serde_json::Value::Object(map)
    } else {
        serde_json::Value::Null
    }
}

impl PythonPluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            info: PluginInfo {
                name: "python-plugin-manager".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: "Python plugin backend".to_string(),
                author: "Slapper".to_string(),
                tags: vec!["python".to_string()],
                language: PluginLanguage::Python,
            },
            block_suspicious_plugins: true,
            checks_cache: std::sync::OnceLock::new(),
        }
    }

    pub fn from_config(config: &PluginConfig) -> Self {
        Self {
            plugins: Vec::new(),
            info: PluginInfo {
                name: "python-plugin-manager".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: "Python plugin backend".to_string(),
                author: "Slapper".to_string(),
                tags: vec!["python".to_string()],
                language: PluginLanguage::Python,
            },
            block_suspicious_plugins: config.block_suspicious_plugins,
            checks_cache: std::sync::OnceLock::new(),
        }
    }

    pub fn with_block_suspicious_plugins(block: bool) -> Self {
        Self {
            plugins: Vec::new(),
            info: PluginInfo {
                name: "python-plugin-manager".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: "Python plugin backend".to_string(),
                author: "Slapper".to_string(),
                tags: vec!["python".to_string()],
                language: PluginLanguage::Python,
            },
            block_suspicious_plugins: block,
            checks_cache: std::sync::OnceLock::new(),
        }
    }

    pub fn load_plugins(&mut self, plugin_dir: &Path) -> Result<()> {
        Python::attach(|py| {
            if !plugin_dir.exists() {
                return Ok(());
            }

            let sys = py.import("sys")?;
            let path = sys.getattr("path")?;
            let dir_str = plugin_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Plugin directory path is not valid UTF-8"))?;
            if !path.contains(dir_str)? {
                path.call_method1("insert", (0, dir_str))?;
            }

            for entry in std::fs::read_dir(plugin_dir)? {
                let entry = entry?;
                let file_path = entry.path();

                if file_path.extension().map(|e| e == "py").unwrap_or(false) {
                    if let Err(e) = validate_plugin_path(plugin_dir, &file_path) {
                        tracing::warn!(path = %file_path.display(), error = %e, "Path validation failed");
                        continue;
                    }

                    if let Some(stem) = file_path.file_stem() {
                        if let Some(module_name) = stem.to_str() {
                            let plugin_content = match std::fs::read_to_string(&file_path) {
                                Ok(c) => c,
                                Err(e) => {
                                    tracing::warn!(
                                        file = %file_path.display(),
                                        error = %e,
                                        "Failed to read plugin file"
                                    );
                                    continue;
                                }
                            };

                            if let Err(e) = validate_python_plugin(&plugin_content, self.block_suspicious_plugins) {
                                tracing::warn!(
                                    file = %file_path.display(),
                                    error = %e,
                                    "Plugin validation failed"
                                );
                                continue;
                            }

                            match PyModule::import(py, module_name) {
                                Ok(module) => {
                                    let class_plugins =
                                        Self::extract_class_plugins(py, &module, module_name);
                                    self.plugins.push(LoadedPlugin {
                                        name: module_name.to_string(),
                                        module: module.into(),
                                        class_plugins,
                                    });
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        module = %module_name,
                                        path = %file_path.display(),
                                        error = %e,
                                        "Failed to import Python plugin"
                                    );
                                }
                            }
                        }
                    }
                }
            }

            Ok(())
        })
    }

    /// Extract class-based plugins from a module's PLUGINS list.
    fn extract_class_plugins(
        _py: Python<'_>,
        module: &pyo3::Bound<'_, PyModule>,
        module_name: &str,
    ) -> Vec<ClassPlugin> {
        let mut class_plugins = Vec::new();

        if let Ok(plugins_attr) = module.getattr("PLUGINS") {
            if let Ok(list) = plugins_attr.downcast::<PyList>() {
                for item in list.iter() {
                    if let Ok(inst) = item.call0() {
                        let name = inst
                            .getattr("name")
                            .and_then(|n| n.extract::<String>())
                            .or_else(|_| {
                                inst.getattr("__class__")
                                    .and_then(|c| c.getattr("__name__"))
                                    .and_then(|n| n.extract::<String>())
                            })
                            .unwrap_or_else(|_| module_name.to_string());

                        class_plugins.push(ClassPlugin {
                            name,
                            class: item.into(),
                        });
                    }
                }
            }
        }

        class_plugins
    }

    /// Run a class-based plugin and return results as JSON values.
    fn run_class_plugin(
        py: Python<'_>,
        class_plugin: &ClassPlugin,
        target: &str,
        config: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>> {
        let instance = class_plugin
            .class
            .call0(py)
            .map_err(|e| anyhow::anyhow!("Failed to instantiate plugin '{}': {}", class_plugin.name, e))?;

        let config_dict = PyDict::new(py);
        if let Some(obj) = config.as_object() {
            for (k, v) in obj {
                config_dict.set_item(k, json_value_to_py(py, v)?)?;
            }
        }

        let result = instance
            .call_method1(py, "run", (target, config_dict))
            .map_err(|e| anyhow::anyhow!("Plugin '{}' run() failed: {}", class_plugin.name, e))?;

        let mut json_results = Vec::new();

        // Try to extract as a dict with "findings" key
        if let Ok(dict) = result.downcast_bound::<PyDict>(py) {
            if let Some(findings) = dict.get_item("findings").ok().flatten() {
                if let Ok(list) = findings.downcast::<PyList>() {
                    for item in list.iter() {
                        if let Ok(finding_dict) = item.downcast::<PyDict>() {
                            let mut finding = serde_json::Map::new();
                            for (key, val) in finding_dict.iter() {
                                if let Ok(k) = key.extract::<String>() {
                                    let json_val = py_value_to_json(py, &val);
                                    finding.insert(k, json_val);
                                }
                            }
                            json_results.push(serde_json::Value::Object(finding));
                        }
                    }
                }
            }
        }

        Ok(json_results)
    }

    pub fn get_checks(&self) -> Vec<PluginCheck> {
        self.checks_cache
            .get_or_init(|| {
                Python::attach(|py| {
                    let mut checks = Vec::new();

                    for plugin in &self.plugins {
                        // Collect checks from function-based plugins
                        if let Ok(module) = plugin.module.bind(py).downcast::<PyModule>() {
                            if let Ok(register_func) = module.getattr("register_checks") {
                                if let Ok(result) = register_func.call0() {
                                    if let Ok(list) = result.downcast::<PyList>() {
                                        for item in list.iter() {
                                            if let Ok(dict) = item.downcast::<PyDict>() {
                                                let name = dict
                                                    .get_item("name")
                                                    .ok()
                                                    .flatten()
                                                    .and_then(|v| v.extract::<String>().ok())
                                                    .unwrap_or_default();
                                                let check_type = dict
                                                    .get_item("type")
                                                    .ok()
                                                    .flatten()
                                                    .and_then(|v| v.extract::<String>().ok())
                                                    .unwrap_or_default();
                                                let target = dict
                                                    .get_item("target")
                                                    .ok()
                                                    .flatten()
                                                    .and_then(|v| v.extract::<String>().ok());
                                                let description = dict
                                                    .get_item("description")
                                                    .ok()
                                                    .flatten()
                                                    .and_then(|v| v.extract::<String>().ok());
                                                let check = PluginCheck {
                                                    name,
                                                    check_type,
                                                    target,
                                                    description,
                                                };
                                                checks.push(check);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Collect checks from class-based plugins
                        for class_plugin in &plugin.class_plugins {
                            checks.push(PluginCheck {
                                name: class_plugin.name.clone(),
                                check_type: "class".to_string(),
                                target: None,
                                description: Some(format!(
                                    "Class-based plugin from {}",
                                    plugin.name
                                )),
                            });
                        }
                    }

                    checks
                })
            })
            .clone()
    }

    pub fn run_check_direct(&self, check_name: &str, target: &str, config: &serde_json::Value) -> Result<Vec<serde_json::Value>> {
        Python::attach(|py| {
            let mut all_results = Vec::new();

            for plugin in &self.plugins {
                // Try function-based plugins
                if let Ok(module) = plugin.module.bind(py).downcast::<PyModule>() {
                    if let Ok(run_func) = module.getattr("run_check") {
                        let args = (check_name, target);
                        if let Ok(result) = run_func.call1(args) {
                            if let Ok(list) = result.downcast::<PyList>() {
                                for item in list.iter() {
                                        if let Ok(json_str) = item.extract::<String>() {
                                            if json_str.len() > MAX_JSON_SIZE_BYTES {
                                                tracing::warn!(
                                                    "JSON result exceeds max size, truncating"
                                                );
                                                continue;
                                            }
                                            if let Ok(value) = serde_json::from_str(&json_str) {
                                                all_results.push(value);
                                            }
                                        }
                                }
                            }
                        }
                    }
                }

                // Try class-based plugins
                for class_plugin in &plugin.class_plugins {
                    if class_plugin.name == check_name {
                        match Self::run_class_plugin(py, class_plugin, target, config) {
                            Ok(results) => all_results.extend(results),
                            Err(e) => {
                                tracing::warn!(
                                    plugin = %class_plugin.name,
                                    error = %e,
                                    "Class-based plugin check failed"
                                );
                            }
                        }
                    }
                }
            }

            Ok(all_results)
        })
    }
}

impl Default for PythonPluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonPluginManager {
    fn run_impl(&self, target: &str, config: &PluginConfig) -> Result<PluginResult> {
        let start = Instant::now();
        let checks = self.get_checks();
        let mut findings = Vec::new();
        let mut errors = Vec::new();

        let config_json = serde_json::Value::Object(
            config.config.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        );

        for check in &checks {
            match self.run_check_direct(&check.name, target, &config_json) {
                Ok(json_results) => {
                    for v in json_results {
                        if let Some(title) = v.get("title").and_then(|t| t.as_str()) {
                            findings.push(super::PluginFinding {
                                title: title.to_string(),
                                severity: v
                                    .get("severity")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("info")
                                    .to_string(),
                                description: v
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                location: v
                                    .get("location")
                                    .and_then(|l| l.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                evidence: v
                                    .get("evidence")
                                    .and_then(|e| e.as_str())
                                    .map(String::from),
                                cve_ids: v
                                    .get("cve_ids")
                                    .and_then(|c| c.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|id| id.as_str().map(String::from))
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                            });
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Check '{}' failed: {}", check.name, e));
                }
            }
        }

        Ok(PluginResult {
            plugin_name: self.info.name.clone(),
            success: errors.is_empty(),
            findings,
            errors,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn json_value_to_py(py: Python<'_>, v: &serde_json::Value) -> PyResult<Py<PyAny>> {
    let result = match v {
        serde_json::Value::String(s) => s.clone().into_bound_py_any(py)?,
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_bound_py_any(py)?
            } else if let Some(f) = n.as_f64() {
                f.into_bound_py_any(py)?
            } else {
                n.to_string().into_bound_py_any(py)?
            }
        }
        serde_json::Value::Bool(b) => (*b).into_bound_py_any(py)?,
        serde_json::Value::Null => py.None().into_bound_py_any(py)?,
        serde_json::Value::Array(arr) => {
            let items: Vec<Py<PyAny>> = arr.iter()
                .map(|item| json_value_to_py(py, item))
                .collect::<Result<Vec<_>, _>>()?;
            let list = PyList::new(py, &items)?;
            list.into_any()
        }
        serde_json::Value::Object(_) => v.to_string().into_bound_py_any(py)?,
    };
    Ok(result.unbind())
}

#[async_trait]
impl Plugin for PythonPluginManager {
    fn info(&self) -> &PluginInfo {
        &self.info
    }

    fn language(&self) -> PluginLanguage {
        PluginLanguage::Python
    }

    fn list_checks(&self) -> Vec<PluginCheck> {
        self.get_checks()
    }

    async fn run_check(&self, check_name: &str, target: &str) -> Result<PluginResult> {
        let start = Instant::now();
        let json_results = self.run_check_direct(check_name, target, &serde_json::Value::Object(serde_json::Map::new()))?;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        let findings = json_results
            .into_iter()
            .filter_map(|v| {
                let title = v.get("title").and_then(|t| t.as_str())?.to_string();
                let severity = v
                    .get("severity")
                    .and_then(|s| s.as_str())
                    .unwrap_or("info")
                    .to_string();
                let description = v
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();
                let location = v
                    .get("location")
                    .and_then(|l| l.as_str())
                    .unwrap_or("")
                    .to_string();
                let evidence = v.get("evidence").and_then(|e| e.as_str()).map(String::from);
                let cve_ids = v
                    .get("cve_ids")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|id| id.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                Some(super::PluginFinding {
                    title,
                    severity,
                    description,
                    location,
                    evidence,
                    cve_ids,
                })
            })
            .collect();

        Ok(PluginResult {
            plugin_name: self.info.name.clone(),
            success: true,
            findings,
            errors: Vec::new(),
            execution_time_ms,
        })
    }

    async fn run(&self, target: &str, config: &PluginConfig) -> Result<PluginResult> {
        let timeout_duration = Duration::from_secs(config.timeout_secs);

        let result = timeout(timeout_duration, async {
            self.run_impl(target, config)
        }).await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_) => {
                anyhow::bail!(
                    "Plugin execution timed out after {} seconds",
                    config.timeout_secs
                )
            }
        }
    }

    fn init(&self) -> Result<()> {
        tracing::info!("Initializing Python plugin manager");
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down Python plugin manager");
        Ok(())
    }

    fn health_check(&self) -> Result<HealthStatus> {
        let plugin_count = self.plugins.len();
        if plugin_count == 0 {
            Ok(HealthStatus::Degraded)
        } else {
            Ok(HealthStatus::Healthy)
        }
    }

    fn priority(&self) -> u32 {
        50
    }
}
