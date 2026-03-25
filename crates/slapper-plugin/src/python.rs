#![cfg(feature = "python-plugins")]

use anyhow::Result;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::Path;

pub struct PythonPluginManager {
    plugins: Vec<LoadedPlugin>,
}

struct LoadedPlugin {
    name: String,
    module: Py<PyAny>,
}

#[derive(Debug, Clone)]
pub struct PluginCheck {
    pub name: String,
    pub check_type: String,
    pub target: Option<String>,
    pub description: Option<String>,
}

impl PythonPluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn load_plugins(&mut self, plugin_dir: &Path) -> Result<()> {
        Python::with_gil(|py| {
            if !plugin_dir.exists() {
                return Ok(());
            }

            // Add plugin directory to sys.path so Python can import modules from it
            let sys = py.import_bound("sys")?;
            let path = sys.getattr("path")?;
            let dir_str = plugin_dir
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Plugin directory path is not valid UTF-8"))?;
            if !path.contains(dir_str)? {
                path.call_method1("insert", (0, dir_str))?;
            }

            for entry in std::fs::read_dir(plugin_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().map(|e| e == "py").unwrap_or(false) {
                    if let Some(stem) = path.file_stem() {
                        if let Some(module_name) = stem.to_str() {
                            if let Ok(module) = PyModule::import_bound(py, module_name) {
                                let module = module.into();
                                self.plugins.push(LoadedPlugin {
                                    name: module_name.to_string(),
                                    module,
                                });
                            }
                        }
                    }
                }
            }

            Ok(())
        })
    }

    pub fn get_checks(&self) -> Vec<PluginCheck> {
        Python::with_gil(|py| {
            let mut checks = Vec::new();

            for plugin in &self.plugins {
                if let Ok(module) = plugin.module.as_ref(py).downcast::<PyModule>() {
                    if let Ok(register_func) = module.getattr("register_checks") {
                        if let Ok(result) = register_func.call0() {
                            if let Ok(list) = result.downcast::<PyList>() {
                                for item in list.iter() {
                                    if let Ok(dict) = item.downcast::<PyDict>() {
                                        let check = PluginCheck {
                                            name: dict
                                                .get_item("name")
                                                .and_then(|v| v.extract().ok())
                                                .unwrap_or_default(),
                                            check_type: dict
                                                .get_item("type")
                                                .and_then(|v| v.extract().ok())
                                                .unwrap_or_default(),
                                            target: dict
                                                .get_item("target")
                                                .and_then(|v| v.extract().ok()),
                                            description: dict
                                                .get_item("description")
                                                .and_then(|v| v.extract().ok()),
                                        };
                                        checks.push(check);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            checks
        })
    }

    pub fn run_check(&self, check_name: &str, target: &str) -> Result<Vec<serde_json::Value>> {
        Python::with_gil(|py| {
            for plugin in &self.plugins {
                if let Ok(module) = plugin.module.as_ref(py).downcast::<PyModule>() {
                    if let Ok(run_func) = module.getattr("run_check") {
                        let args = (check_name, target);
                        if let Ok(result) = run_func.call1(args) {
                            if let Ok(list) = result.downcast::<PyList>() {
                                let mut results = Vec::new();
                                for item in list.iter() {
                                    if let Ok(json_str) = item.extract::<String>() {
                                        if let Ok(value) = serde_json::from_str(&json_str) {
                                            results.push(value);
                                        }
                                    }
                                }
                                return Ok(results);
                            }
                        }
                    }
                }
            }

            Ok(Vec::new())
        })
    }
}

impl Default for PythonPluginManager {
    fn default() -> Self {
        Self::new()
    }
}
