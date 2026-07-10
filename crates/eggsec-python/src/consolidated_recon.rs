use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::runtime_sync;

/// Configuration for consolidated reconnaissance.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ConsolidatedReconConfigPy {
    #[pyo3(get)]
    pub run_dns: bool,
    #[pyo3(get)]
    pub run_ssl: bool,
    #[pyo3(get)]
    pub run_tech_detect: bool,
    #[pyo3(get)]
    pub run_subdomain: bool,
    #[pyo3(get)]
    pub run_whois: bool,
    #[pyo3(get)]
    pub run_cors: bool,
    #[pyo3(get)]
    pub run_wayback: bool,
    #[pyo3(get)]
    pub run_js_analysis: bool,
    #[pyo3(get)]
    pub run_content: bool,
    #[pyo3(get)]
    pub run_email: bool,
    #[pyo3(get)]
    pub timeout_secs: u64,
    #[pyo3(get)]
    pub concurrency: usize,
}

#[pymethods]
impl ConsolidatedReconConfigPy {
    /// Create a new consolidated recon configuration.
    ///
    /// Args:
    ///     run_dns: Run DNS enumeration (default: true).
    ///     run_ssl: Run SSL/TLS analysis (default: true).
    ///     run_tech_detect: Run technology detection (default: true).
    ///     run_subdomain: Run subdomain enumeration (default: true).
    ///     run_whois: Run WHOIS lookup (default: true).
    ///     run_cors: Run CORS analysis (default: true).
    ///     run_wayback: Run Wayback Machine lookup (default: true).
    ///     run_js_analysis: Run JavaScript analysis (default: true).
    ///     run_content: Run content discovery (default: true).
    ///     run_email: Run email discovery (default: true).
    ///     timeout_secs: Timeout per module in seconds (default: 30).
    ///     concurrency: Concurrency for subdomain/content scans (default: 10).
    #[new]
    #[pyo3(signature = (*, run_dns=true, run_ssl=true, run_tech_detect=true, run_subdomain=true, run_whois=true, run_cors=true, run_wayback=true, run_js_analysis=true, run_content=true, run_email=true, timeout_secs=30, concurrency=10))]
    fn new(
        run_dns: bool,
        run_ssl: bool,
        run_tech_detect: bool,
        run_subdomain: bool,
        run_whois: bool,
        run_cors: bool,
        run_wayback: bool,
        run_js_analysis: bool,
        run_content: bool,
        run_email: bool,
        timeout_secs: u64,
        concurrency: usize,
    ) -> PyResult<Self> {
        Ok(Self {
            run_dns,
            run_ssl,
            run_tech_detect,
            run_subdomain,
            run_whois,
            run_cors,
            run_wayback,
            run_js_analysis,
            run_content,
            run_email,
            timeout_secs,
            concurrency,
        })
    }

    /// Get the list of modules that will be run.
    fn enabled_modules(&self) -> Vec<String> {
        let mut modules = Vec::new();
        if self.run_dns {
            modules.push("dns_records".to_string());
        }
        if self.run_ssl {
            modules.push("ssl".to_string());
        }
        if self.run_tech_detect {
            modules.push("techdetect".to_string());
        }
        if self.run_subdomain {
            modules.push("subdomain".to_string());
        }
        if self.run_whois {
            modules.push("whois".to_string());
        }
        if self.run_cors {
            modules.push("cors".to_string());
        }
        if self.run_wayback {
            modules.push("wayback".to_string());
        }
        if self.run_js_analysis {
            modules.push("js".to_string());
        }
        if self.run_content {
            modules.push("content".to_string());
        }
        if self.run_email {
            modules.push("email".to_string());
        }
        modules
    }

    fn __repr__(&self) -> String {
        format!(
            "ConsolidatedReconConfig(modules={}, timeout_secs={})",
            self.enabled_modules().len(),
            self.timeout_secs
        )
    }
}

/// A single recon module result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconModuleResultPy {
    #[pyo3(get)]
    pub module: String,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub data: Option<String>,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl ReconModuleResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("module", &self.module)?;
        dict.set_item("success", self.success)?;
        dict.set_item("data", &self.data)?;
        dict.set_item("error", &self.error)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "ReconModuleResult(module={}, success={})",
            self.module, self.success
        )
    }
}

/// Consolidated reconnaissance report.
///
/// Contains results from multiple recon modules run against a single target.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedReconReportPy {
    #[pyo3(get)]
    pub target: String,
    modules: Vec<ReconModuleResultPy>,
    #[pyo3(get)]
    pub modules_run: usize,
    #[pyo3(get)]
    pub modules_succeeded: usize,
    #[pyo3(get)]
    pub modules_failed: usize,
}

#[pymethods]
impl ConsolidatedReconReportPy {
    #[getter]
    fn modules(&self) -> Vec<ReconModuleResultPy> {
        self.modules.clone()
    }

    fn get_module(&self, module_name: &str) -> Option<ReconModuleResultPy> {
        self.modules
            .iter()
            .find(|m| m.module == module_name)
            .cloned()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("modules_run", self.modules_run)?;
        dict.set_item("modules_succeeded", self.modules_succeeded)?;
        dict.set_item("modules_failed", self.modules_failed)?;

        let modules_list = PyList::empty_bound(py);
        for m in &self.modules {
            modules_list.append(m.to_dict(py)?)?;
        }
        dict.set_item("modules", modules_list)?;

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ConsolidatedReconReport(target={}, modules_run={}, succeeded={}, failed={})",
            self.target, self.modules_run, self.modules_succeeded, self.modules_failed
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Recon report for {}: {}/{} modules succeeded",
            self.target, self.modules_succeeded, self.modules_run
        )
    }
}

/// Run consolidated reconnaissance against a target.
///
/// Executes selected recon modules (DNS, SSL, tech detection, subdomain enum,
/// WHOIS, CORS, Wayback, JS analysis, content discovery, email discovery)
/// and aggregates results into a single report.
///
/// Modules that require API keys or complex parameters (threat intel, CVE,
/// secrets, takeover) are not included — use the standalone functions for those.
///
/// Args:
///     target: Target domain or URL.
///     config: Recon configuration specifying which modules to run.
///
/// Returns:
///     ConsolidatedReconReportPy: Aggregated report from all modules.
///
/// Raises:
///     NetworkError: If the target is unreachable.
///     ConfigError: If the configuration is invalid.
#[pyfunction]
pub fn run_consolidated_recon(
    target: &str,
    config: ConsolidatedReconConfigPy,
) -> PyResult<ConsolidatedReconReportPy> {
    let target_owned = target.to_string();

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let mut modules = Vec::new();

            // DNS enumeration
            if config.run_dns {
                let module_result =
                    match eggsec::recon::dns_records::enumerate_dns_records(&target_owned).await {
                        Ok(_) => ReconModuleResultPy {
                            module: "dns_records".to_string(),
                            success: true,
                            data: Some("DNS records enumerated successfully".to_string()),
                            error: None,
                        },
                        Err(e) => ReconModuleResultPy {
                            module: "dns_records".to_string(),
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    };
                modules.push(module_result);
            }

            // SSL/TLS analysis
            if config.run_ssl {
                let module_result = match eggsec::recon::ssl::analyze_ssl(&target_owned, 443).await
                {
                    Ok(_) => ReconModuleResultPy {
                        module: "ssl".to_string(),
                        success: true,
                        data: Some("SSL/TLS analysis completed".to_string()),
                        error: None,
                    },
                    Err(e) => ReconModuleResultPy {
                        module: "ssl".to_string(),
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                };
                modules.push(module_result);
            }

            // Technology detection
            if config.run_tech_detect {
                let module_result =
                    match eggsec::recon::techdetect::detect_tech_stack(&target_owned).await {
                        Ok(_) => ReconModuleResultPy {
                            module: "techdetect".to_string(),
                            success: true,
                            data: Some("Technology detection completed".to_string()),
                            error: None,
                        },
                        Err(e) => ReconModuleResultPy {
                            module: "techdetect".to_string(),
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    };
                modules.push(module_result);
            }

            // Subdomain enumeration
            if config.run_subdomain {
                let module_result = match eggsec::recon::subdomain::enumerate_subdomains(
                    &target_owned,
                    config.concurrency,
                )
                .await
                {
                    Ok(_) => ReconModuleResultPy {
                        module: "subdomain".to_string(),
                        success: true,
                        data: Some("Subdomain enumeration completed".to_string()),
                        error: None,
                    },
                    Err(e) => ReconModuleResultPy {
                        module: "subdomain".to_string(),
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                };
                modules.push(module_result);
            }

            // WHOIS lookup
            if config.run_whois {
                let module_result = match eggsec::recon::whois::whois_lookup(&target_owned).await {
                    Ok(_) => ReconModuleResultPy {
                        module: "whois".to_string(),
                        success: true,
                        data: Some("WHOIS lookup completed".to_string()),
                        error: None,
                    },
                    Err(e) => ReconModuleResultPy {
                        module: "whois".to_string(),
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                };
                modules.push(module_result);
            }

            // CORS analysis
            if config.run_cors {
                let module_result = match eggsec::recon::cors::analyze_cors(&target_owned).await {
                    Ok(_) => ReconModuleResultPy {
                        module: "cors".to_string(),
                        success: true,
                        data: Some("CORS analysis completed".to_string()),
                        error: None,
                    },
                    Err(e) => ReconModuleResultPy {
                        module: "cors".to_string(),
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                };
                modules.push(module_result);
            }

            // Wayback Machine
            if config.run_wayback {
                let module_result =
                    match eggsec::recon::wayback::get_wayback_snapshots(&target_owned, None, 100)
                        .await
                    {
                        Ok(_) => ReconModuleResultPy {
                            module: "wayback".to_string(),
                            success: true,
                            data: Some("Wayback Machine query completed".to_string()),
                            error: None,
                        },
                        Err(e) => ReconModuleResultPy {
                            module: "wayback".to_string(),
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    };
                modules.push(module_result);
            }

            // JavaScript analysis
            if config.run_js_analysis {
                let module_result = match eggsec::recon::js::analyze_js(&target_owned).await {
                    Ok(_) => ReconModuleResultPy {
                        module: "js".to_string(),
                        success: true,
                        data: Some("JavaScript analysis completed".to_string()),
                        error: None,
                    },
                    Err(e) => ReconModuleResultPy {
                        module: "js".to_string(),
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                };
                modules.push(module_result);
            }

            // Content discovery
            if config.run_content {
                let module_result =
                    match eggsec::recon::content::scan_content(&target_owned, config.concurrency)
                        .await
                    {
                        Ok(_) => ReconModuleResultPy {
                            module: "content".to_string(),
                            success: true,
                            data: Some("Content discovery completed".to_string()),
                            error: None,
                        },
                        Err(e) => ReconModuleResultPy {
                            module: "content".to_string(),
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    };
                modules.push(module_result);
            }

            // Email discovery
            if config.run_email {
                let module_result =
                    match eggsec::recon::email::discover_contacts(&target_owned).await {
                        Ok(_) => ReconModuleResultPy {
                            module: "email".to_string(),
                            success: true,
                            data: Some("Email discovery completed".to_string()),
                            error: None,
                        },
                        Err(e) => ReconModuleResultPy {
                            module: "email".to_string(),
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        },
                    };
                modules.push(module_result);
            }

            let modules_run = modules.len();
            let modules_succeeded = modules.iter().filter(|m| m.success).count();
            let modules_failed = modules_run - modules_succeeded;

            Ok::<_, PyErr>(ConsolidatedReconReportPy {
                target: target_owned,
                modules,
                modules_run,
                modules_succeeded,
                modules_failed,
            })
        })?;

        Ok(result)
    })
}

/// Run consolidated reconnaissance (async).
///
/// Returns a PyFuture that resolves to a ConsolidatedReconReportPy.
#[pyfunction]
pub fn async_run_consolidated_recon(
    target: &str,
    config: ConsolidatedReconConfigPy,
) -> PyResult<crate::runtime_async::PyFuture> {
    let target_owned = target.to_string();

    crate::runtime_async::spawn_async(async move {
        let mut modules = Vec::new();

        if config.run_dns {
            let module_result =
                match eggsec::recon::dns_records::enumerate_dns_records(&target_owned).await {
                    Ok(_) => ReconModuleResultPy {
                        module: "dns_records".to_string(),
                        success: true,
                        data: Some("DNS records enumerated successfully".to_string()),
                        error: None,
                    },
                    Err(e) => ReconModuleResultPy {
                        module: "dns_records".to_string(),
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                };
            modules.push(module_result);
        }

        if config.run_ssl {
            let module_result = match eggsec::recon::ssl::analyze_ssl(&target_owned, 443).await {
                Ok(_) => ReconModuleResultPy {
                    module: "ssl".to_string(),
                    success: true,
                    data: Some("SSL/TLS analysis completed".to_string()),
                    error: None,
                },
                Err(e) => ReconModuleResultPy {
                    module: "ssl".to_string(),
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                },
            };
            modules.push(module_result);
        }

        if config.run_tech_detect {
            let module_result =
                match eggsec::recon::techdetect::detect_tech_stack(&target_owned).await {
                    Ok(_) => ReconModuleResultPy {
                        module: "techdetect".to_string(),
                        success: true,
                        data: Some("Technology detection completed".to_string()),
                        error: None,
                    },
                    Err(e) => ReconModuleResultPy {
                        module: "techdetect".to_string(),
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                };
            modules.push(module_result);
        }

        let modules_run = modules.len();
        let modules_succeeded = modules.iter().filter(|m| m.success).count();
        let modules_failed = modules_run - modules_succeeded;

        Ok::<ConsolidatedReconReportPy, PyErr>(ConsolidatedReconReportPy {
            target: target_owned,
            modules,
            modules_run,
            modules_succeeded,
            modules_failed,
        })
    })
}
