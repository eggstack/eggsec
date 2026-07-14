use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::ScanError;
use crate::runtime_async;
use crate::runtime_sync;

// ═══════════════════════════════════════════════════════════════════
// Release 3: NSE Library Descriptor
// ═══════════════════════════════════════════════════════════════════

/// Full metadata descriptor for a registered NSE library module.
///
/// Exposes the library registry entry including category, sandbox side
/// effects, fallback behavior, and compatibility notes.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseLibraryDescriptorPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub category: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub sandbox_side_effects: Vec<String>,
    #[pyo3(get)]
    pub fallback_behavior: String,
    #[pyo3(get)]
    pub notes: String,
    #[pyo3(get)]
    pub optional_deps: Vec<String>,
    #[pyo3(get)]
    pub enforcement_status: String,
}

impl NseLibraryDescriptorPy {
    pub fn from_engine(desc: &eggsec::nse::NseLibraryDescriptor) -> Self {
        let side_effects = desc
            .sandbox_side_effects
            .iter()
            .map(|se| se.to_string())
            .collect();
        let optional_deps = desc.optional_deps.iter().map(|d| d.to_string()).collect();
        Self {
            name: desc.name.to_string(),
            category: desc.category.to_string(),
            description: String::new(),
            sandbox_side_effects: side_effects,
            fallback_behavior: desc.fallback_behavior.to_string(),
            notes: desc.notes.to_string(),
            optional_deps,
            enforcement_status: desc.enforcement_status.to_string(),
        }
    }
}

#[pymethods]
impl NseLibraryDescriptorPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("category", &self.category)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("sandbox_side_effects", &self.sandbox_side_effects)?;
        dict.set_item("fallback_behavior", &self.fallback_behavior)?;
        dict.set_item("notes", &self.notes)?;
        dict.set_item("optional_deps", &self.optional_deps)?;
        dict.set_item("enforcement_status", &self.enforcement_status)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseLibraryDescriptor(name={}, category={}, enforcement={})",
            self.name, self.category, self.enforcement_status
        )
    }

    fn __str__(&self) -> String {
        format!(
            "NSE library '{}' [{}]: {}",
            self.name, self.category, self.notes
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3: NSE Argument
// ═══════════════════════════════════════════════════════════════════

/// A structured argument for NSE script execution.
///
/// Represents a single key=value argument passed to an NSE script,
/// with type information for validation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseArgumentPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub value: String,
    #[pyo3(get)]
    pub arg_type: String,
}

#[pymethods]
impl NseArgumentPy {
    #[new]
    #[pyo3(signature = (name, value, *, arg_type="string"))]
    fn new(name: &str, value: &str, arg_type: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
            arg_type: arg_type.to_string(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("value", &self.value)?;
        dict.set_item("arg_type", &self.arg_type)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseArgument(name={}, value={}, type={})",
            self.name, self.value, self.arg_type
        )
    }

    fn __str__(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3: NSE Library Registry
// ═══════════════════════════════════════════════════════════════════

/// Query interface for the NSE library registry.
///
/// Wraps access to `eggsec::nse::all_libraries()`, `find_library()`,
/// and `libraries_by_category()`.
#[pyclass]
pub struct NseLibraryRegistryPy;

#[pymethods]
impl NseLibraryRegistryPy {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Return all registered library descriptors.
    fn list(&self) -> Vec<NseLibraryDescriptorPy> {
        eggsec::nse::all_libraries()
            .iter()
            .map(NseLibraryDescriptorPy::from_engine)
            .collect()
    }

    /// Look up a library by name. Returns None if not found.
    fn get(&self, name: &str) -> Option<NseLibraryDescriptorPy> {
        eggsec::nse::find_library(name).map(NseLibraryDescriptorPy::from_engine)
    }

    /// Return all libraries in the given category.
    fn by_category(&self, category: &str) -> Vec<NseLibraryDescriptorPy> {
        let cat = match category {
            "Core" => eggsec::nse::NseLibraryCategory::Core,
            "Protocol" => eggsec::nse::NseLibraryCategory::Protocol,
            "Utility" => eggsec::nse::NseLibraryCategory::Utility,
            "Exploit" => eggsec::nse::NseLibraryCategory::Exploit,
            "Auth" => eggsec::nse::NseLibraryCategory::Auth,
            _ => return Vec::new(),
        };
        eggsec::nse::libraries_by_category(cat)
            .iter()
            .map(|d| NseLibraryDescriptorPy::from_engine(d))
            .collect()
    }

    /// Return the total number of registered libraries.
    fn count(&self) -> usize {
        eggsec::nse::registry_count()
    }
}

// ═══════════════════════════════════════════════════════════════════
// DTOs
// ═══════════════════════════════════════════════════════════════════

/// Configuration for running NSE (Nmap Scripting Engine) scripts.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseConfigPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub script: String,
    #[pyo3(get)]
    pub script_args: Option<String>,
    #[pyo3(get)]
    pub script_file: Option<String>,
    #[pyo3(get)]
    pub json: bool,
    #[pyo3(get)]
    pub verbose: bool,
}

#[pymethods]
impl NseConfigPy {
    #[new]
    #[pyo3(signature = (target, script, *, script_args=None, script_file=None, json=false, verbose=false))]
    fn new(
        target: &str,
        script: &str,
        script_args: Option<&str>,
        script_file: Option<&str>,
        json: bool,
        verbose: bool,
    ) -> Self {
        Self {
            target: target.to_string(),
            script: script.to_string(),
            script_args: script_args.map(|s| s.to_string()),
            script_file: script_file.map(|s| s.to_string()),
            json,
            verbose,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("script", &self.script)?;
        dict.set_item("script_args", &self.script_args)?;
        dict.set_item("script_file", &self.script_file)?;
        dict.set_item("json", self.json)?;
        dict.set_item("verbose", self.verbose)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseConfigPy(target={}, script={}, verbose={})",
            self.target, self.script, self.verbose
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// A single library use report from an NSE execution.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseLibraryUsePy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub category: String,
    #[pyo3(get)]
    pub loaded: bool,
    #[pyo3(get)]
    pub side_effects: Vec<String>,
    #[pyo3(get)]
    pub fallback_behavior: String,
    #[pyo3(get)]
    pub notes: String,
    #[pyo3(get)]
    pub warnings: Vec<String>,
}

impl NseLibraryUsePy {
    pub fn from_engine(report: eggsec::nse::NseLibraryUseReport) -> Self {
        Self {
            name: report.name,
            category: report.category,
            loaded: report.loaded,
            side_effects: report.side_effects,
            fallback_behavior: report.fallback_behavior,
            notes: report.notes,
            warnings: report.warnings,
        }
    }
}

#[pymethods]
impl NseLibraryUsePy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("category", &self.category)?;
        dict.set_item("loaded", self.loaded)?;
        dict.set_item("side_effects", &self.side_effects)?;
        dict.set_item("fallback_behavior", &self.fallback_behavior)?;
        dict.set_item("notes", &self.notes)?;
        dict.set_item("warnings", &self.warnings)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseLibraryUsePy(name={}, category={}, loaded={})",
            self.name, self.category, self.loaded
        )
    }

    fn __str__(&self) -> String {
        format!(
            "NSE library '{}' [{}] loaded={}",
            self.name, self.category, self.loaded
        )
    }
}

/// A single rule evaluation result from an NSE execution.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseRuleEvaluationPy {
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub evaluated: bool,
    #[pyo3(get)]
    pub matched: bool,
    #[pyo3(get)]
    pub exactness: String,
    #[pyo3(get)]
    pub error: Option<String>,
    #[pyo3(get)]
    pub summary: String,
    #[pyo3(get)]
    pub unsupported: Option<String>,
}

impl NseRuleEvaluationPy {
    pub fn from_engine(report: eggsec::nse::NseRuleEvaluationReport) -> Self {
        Self {
            kind: report.kind,
            evaluated: report.evaluated,
            matched: report.matched,
            exactness: report.exactness,
            error: report.error,
            summary: report.summary,
            unsupported: report.unsupported,
        }
    }
}

#[pymethods]
impl NseRuleEvaluationPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("kind", &self.kind)?;
        dict.set_item("evaluated", self.evaluated)?;
        dict.set_item("matched", self.matched)?;
        dict.set_item("exactness", &self.exactness)?;
        dict.set_item("error", &self.error)?;
        dict.set_item("summary", &self.summary)?;
        dict.set_item("unsupported", &self.unsupported)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseRuleEvaluationPy(kind={}, evaluated={}, matched={}, exactness={})",
            self.kind, self.evaluated, self.matched, self.exactness
        )
    }

    fn __str__(&self) -> String {
        format!(
            "NSE rule '{}' evaluated={} matched={} ({})",
            self.kind, self.evaluated, self.matched, self.summary
        )
    }
}

/// Type alias for the NSE run report used by the operation registry.
pub type NseRunReportPy = NseReportPy;

/// A single structured evidence item from NSE execution.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseEvidenceItemPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub summary: String,
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub port: Option<u16>,
    #[pyo3(get)]
    pub service: Option<String>,
    #[pyo3(get)]
    pub confidence: String,
    #[pyo3(get)]
    pub source: String,
    #[pyo3(get)]
    pub raw_excerpt: Option<String>,
    #[pyo3(get)]
    pub references: Vec<String>,
    #[pyo3(get)]
    pub tags: Vec<String>,
}

#[pymethods]
impl NseEvidenceItemPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("kind", &self.kind)?;
        dict.set_item("title", &self.title)?;
        dict.set_item("summary", &self.summary)?;
        dict.set_item("target", &self.target)?;
        dict.set_item("port", self.port)?;
        dict.set_item("service", &self.service)?;
        dict.set_item("confidence", &self.confidence)?;
        dict.set_item("source", &self.source)?;
        dict.set_item("raw_excerpt", &self.raw_excerpt)?;
        dict.set_item("references", &self.references)?;
        dict.set_item("tags", &self.tags)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseEvidenceItem(id={}, kind={}, title={})",
            self.id, self.kind, self.title
        )
    }

    fn __str__(&self) -> String {
        format!("[{}] {}", self.kind, self.title)
    }
}

/// Simplified result from an NSE script execution.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseReportPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub script_name: String,
    #[pyo3(get)]
    pub output: String,
    #[pyo3(get)]
    pub output_lines: usize,
    #[pyo3(get)]
    pub has_output: bool,
    #[pyo3(get)]
    pub warnings: Vec<String>,
    #[pyo3(get)]
    pub errors: Vec<String>,
    #[pyo3(get)]
    pub library_count: usize,
    #[pyo3(get)]
    pub compatibility_status: String,
    #[pyo3(get)]
    pub fidelity: String,
    #[pyo3(get)]
    pub elapsed_secs: f64,
    libraries: Vec<NseLibraryUsePy>,
    rules: Vec<NseRuleEvaluationPy>,
    evidence: Vec<NseEvidenceItemPy>,
}

impl NseReportPy {
    pub fn from_engine(report: eggsec::nse::NseRunReport) -> Self {
        let libraries: Vec<NseLibraryUsePy> = report
            .libraries
            .into_iter()
            .map(NseLibraryUsePy::from_engine)
            .collect();
        let rules: Vec<NseRuleEvaluationPy> = report
            .rules
            .into_iter()
            .map(NseRuleEvaluationPy::from_engine)
            .collect();
        let evidence: Vec<NseEvidenceItemPy> = report
            .evidence
            .into_iter()
            .map(|e| NseEvidenceItemPy {
                id: e.id,
                kind: e.kind.to_string(),
                title: e.title,
                summary: e.summary,
                target: e.target,
                port: e.port,
                service: e.service,
                confidence: e.confidence,
                source: e.source,
                raw_excerpt: e.raw_excerpt,
                references: e.references,
                tags: e.tags,
            })
            .collect();
        Self {
            target: report.target,
            script_name: report.script_name,
            output: report.output.content,
            output_lines: report.output.line_count,
            has_output: report.output.has_output,
            warnings: report.warnings,
            errors: report.errors,
            library_count: libraries.len(),
            compatibility_status: report.compatibility.status.to_string(),
            fidelity: report.compatibility.fidelity.to_string(),
            elapsed_secs: report.stats.elapsed_secs,
            libraries,
            rules,
            evidence,
        }
    }
}

#[pymethods]
impl NseReportPy {
    #[getter]
    fn libraries(&self) -> Vec<NseLibraryUsePy> {
        self.libraries.clone()
    }

    #[getter]
    fn rules(&self) -> Vec<NseRuleEvaluationPy> {
        self.rules.clone()
    }

    #[getter]
    fn evidence(&self) -> Vec<NseEvidenceItemPy> {
        self.evidence.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("script_name", &self.script_name)?;
        dict.set_item("output", &self.output)?;
        dict.set_item("output_lines", self.output_lines)?;
        dict.set_item("has_output", self.has_output)?;
        dict.set_item("warnings", &self.warnings)?;
        dict.set_item("errors", &self.errors)?;
        dict.set_item("library_count", self.library_count)?;
        dict.set_item("compatibility_status", &self.compatibility_status)?;
        dict.set_item("fidelity", &self.fidelity)?;
        dict.set_item("elapsed_secs", self.elapsed_secs)?;
        let libs_list = PyList::empty_bound(py);
        for lib in &self.libraries {
            libs_list.append(lib.to_dict(py)?)?;
        }
        dict.set_item("libraries", libs_list)?;
        let rules_list = PyList::empty_bound(py);
        for rule in &self.rules {
            rules_list.append(rule.to_dict(py)?)?;
        }
        dict.set_item("rules", rules_list)?;
        let evidence_list = PyList::empty_bound(py);
        for ev in &self.evidence {
            evidence_list.append(ev.to_dict(py)?)?;
        }
        dict.set_item("evidence", evidence_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseReportPy(target={}, script={}, libs={}, status={})",
            self.target, self.script_name, self.library_count, self.compatibility_status
        )
    }

    fn __str__(&self) -> String {
        format!(
            "NSE report for '{}' on '{}': {} libraries, {} warnings, {} errors, compatibility={}",
            self.script_name,
            self.target,
            self.library_count,
            self.warnings.len(),
            self.errors.len(),
            self.compatibility_status,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Internal helpers
// ═══════════════════════════════════════════════════════════════════

pub(crate) fn build_nse_config(
    target: &str,
    script: &str,
    script_args: Option<&str>,
    verbose: bool,
) -> eggsec::nse::NseConfig {
    eggsec::nse::NseConfig::new(target, script, script_args, None, false, verbose)
}

pub(crate) fn run_nse_sync(config: eggsec::nse::NseConfig) -> PyResult<NseReportPy> {
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            run_nse_inner(config)
                .await
                .map_err(|e| ScanError::new_err(format!("NSE execution failed: {}", e)))
        })?;
        Ok(NseReportPy::from_engine(result))
    })
}

pub(crate) fn run_nse_async(config: eggsec::nse::NseConfig) -> PyResult<runtime_async::PyFuture> {
    runtime_async::spawn_async(async move {
        let result = run_nse_inner(config)
            .await
            .map_err(|e| ScanError::new_err(format!("NSE execution failed: {}", e)))?;
        Ok(NseReportPy::from_engine(result))
    })
}

async fn run_nse_inner(
    config: eggsec::nse::NseConfig,
) -> anyhow::Result<eggsec::nse::NseRunReport> {
    use eggsec::nse::NseRunReport;

    let target = config.target.clone();
    let script = config.script.clone();

    // Build the execution profile (AgentSafe for automated Python surface)
    let profile = eggsec::nse::ResolvedNseExecutionProfile::agent_safe(&target, &[]);

    let report_profile = profile.clone();
    let execution_profile = profile.clone();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<(
        String,
        eggsec::nse::NseScriptSource,
        Vec<eggsec::nse::NseLoadDiagnostic>,
        Vec<eggsec::nse::NseRuleEvaluationReport>,
        Vec<eggsec::nse::NseLibraryUseReport>,
        Vec<eggsec::nse::NseCapabilityEvent>,
    )> {
        let mut executor = eggsec::nse::NseExecutor::with_profile(&execution_profile)
            .map_err(|e| anyhow::anyhow!("Failed to create NSE executor: {}", e))?;
        executor
            .set_target(&target)
            .map_err(|e| anyhow::anyhow!("Failed to set target: {}", e))?;
        if let Some(ref args) = config.script_args {
            executor
                .set_script_args(args)
                .map_err(|e| anyhow::anyhow!("Invalid script args: {}", e))?;
        }

        let mut resolver = eggsec::nse::ScriptResolver::new(
            execution_profile.script_policy.clone(),
            execution_profile.module_policy.clone(),
            execution_profile.limits.clone(),
        );

        let (script_content, script_source) = if let Some(ref script_file) = config.script_file {
            let source = eggsec::nse::NseScriptSource::File {
                path: std::path::PathBuf::from(script_file),
            };
            let src = source.clone();
            match resolver.resolve_script(source) {
                Ok(resolved) => (resolved.content, src),
                Err(e) => {
                    anyhow::bail!("Script file resolution failed: {}", e);
                }
            }
        } else {
            let content = eggsec::nse::get_builtin_script(&script);
            let source = eggsec::nse::NseScriptSource::InlineManual {
                label: script.clone(),
                content: content.clone(),
            };
            let src = source.clone();
            match resolver.resolve_script(source) {
                Ok(_) => (content, src),
                Err(e) => {
                    anyhow::bail!("Built-in script resolution failed: {}", e);
                }
            }
        };

        let diagnostics = resolver.take_diagnostics();

        let (output, _raw_outputs, rule_reports) = executor
            .run_script_with_rules(&script_content)
            .map_err(|e| anyhow::anyhow!("Script execution failed: {}", e))?;

        let mut library_reports = executor.library_reports();
        if library_reports.is_empty() {
            // Fallback: extract static requires from script content and build reports
            let static_requires = extract_static_requires(&script_content);
            if !static_requires.is_empty() {
                library_reports = static_requires
                    .iter()
                    .map(|name| {
                        let mut warnings = Vec::new();
                        warnings.push(
                            "detected statically; runtime require tracking did not complete"
                                .to_string(),
                        );
                        if let Some(desc) = eggsec::nse::find_library(name) {
                            let side_effects = desc
                                .sandbox_side_effects
                                .iter()
                                .map(|se| se.to_string())
                                .collect();
                            eggsec::nse::NseLibraryUseReport {
                                name: desc.name.to_string(),
                                category: desc.category.to_string(),
                                registered: true,
                                side_effects,
                                fallback_behavior: desc.fallback_behavior.to_string(),
                                notes: desc.notes.to_string(),
                                loaded: false,
                                warnings,
                            }
                        } else {
                            eggsec::nse::NseLibraryUseReport {
                                name: name.clone(),
                                category: "Unknown".to_string(),
                                registered: false,
                                side_effects: Vec::new(),
                                fallback_behavior: "Unknown".to_string(),
                                notes: "not present in NSE library registry".to_string(),
                                loaded: false,
                                warnings,
                            }
                        }
                    })
                    .collect();
            }
        }

        let capability_events = executor.capability_events();

        Ok((
            output,
            script_source,
            diagnostics,
            rule_reports,
            library_reports,
            capability_events,
        ))
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))??;

    let (output, script_source, diagnostics, rule_reports, library_reports, capability_events) =
        result;

    let report = NseRunReport::new(&config.target, &config.script)
        .with_profile(&report_profile)
        .with_script_source(&script_source)
        .with_resolver_diagnostics(&diagnostics)
        .with_libraries(library_reports)
        .with_rules(rule_reports)
        .with_capability_events(capability_events)
        .with_output(&output)
        .compute_compatibility();

    Ok(report)
}

// Minimal static require extraction for fallback when no dynamic reports exist.
fn extract_static_requires(script_content: &str) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut names = Vec::new();
    // Simple regex-like matching for require("name") patterns
    for line in script_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--") {
            continue;
        }
        if let Some(start) = trimmed.find("require") {
            let rest = &trimmed[start + 7..];
            // Match require "name" or require("name")
            let rest = rest.trim_start();
            if rest.starts_with('(') {
                let rest = rest[1..].trim_start();
                if let Some(end) = rest.find(')') {
                    let inner = rest[..end].trim().trim_matches(|c| c == '"' || c == '\'');
                    if !inner.is_empty() && seen.insert(inner.to_string()) {
                        names.push(inner.to_string());
                    }
                }
            } else if rest.starts_with('"') || rest.starts_with('\'') {
                let quote = rest.as_bytes()[0] as char;
                if let Some(end) = rest[1..].find(quote) {
                    let inner = &rest[1..1 + end];
                    if !inner.is_empty() && seen.insert(inner.to_string()) {
                        names.push(inner.to_string());
                    }
                }
            }
        }
    }
    names
}

// ═══════════════════════════════════════════════════════════════════
// Functions
// ═══════════════════════════════════════════════════════════════════

/// Run an NSE script against a target (synchronous).
///
/// Executes a Lua script through the NSE engine and returns a structured report
/// with output, library usage, rule evaluations, and compatibility information.
///
/// Args:
///     target: Target host or IP address.
///     script: Script name (built-in) or Lua content.
///     script_args: Optional comma-separated script arguments.
///     verbose: Enable verbose output (default: False).
///
/// Returns:
///     NseReportPy: Execution report with output and diagnostics.
///
/// Raises:
///     ScanError: If the NSE execution fails.
#[pyfunction]
#[pyo3(signature = (target, script, *, script_args=None, verbose=false))]
pub fn nse_run(
    target: &str,
    script: &str,
    script_args: Option<&str>,
    verbose: bool,
) -> PyResult<NseReportPy> {
    let config = build_nse_config(target, script, script_args, verbose);
    run_nse_sync(config)
}

/// Run an NSE script against a target (asynchronous).
///
/// Returns a PyFuture that can be awaited in Python.
#[pyfunction]
#[pyo3(signature = (target, script, *, script_args=None, verbose=false))]
pub fn async_nse_run(
    target: &str,
    script: &str,
    script_args: Option<&str>,
    verbose: bool,
) -> PyResult<runtime_async::PyFuture> {
    let config = build_nse_config(target, script, script_args, verbose);
    run_nse_async(config)
}

/// List all available built-in NSE library names.
///
/// Returns the names of all registered NSE library modules (e.g. "stdnse",
/// "http", "dns"). These can be used in script require() statements.
///
/// Returns:
///     list[str]: Sorted list of library names.
#[pyfunction]
pub fn nse_list_libraries() -> Vec<String> {
    let mut names: Vec<String> = eggsec::nse::all_libraries()
        .iter()
        .map(|lib| lib.name.to_string())
        .collect();
    names.sort();
    names
}

/// Return all registered library descriptors with full metadata.
///
/// Provides detailed information about each NSE library module including
/// category, sandbox side effects, fallback behavior, and enforcement status.
///
/// Returns:
///     list[NseLibraryDescriptorPy]: Full library descriptors.
#[pyfunction]
pub fn nse_list_libraries_detailed() -> Vec<NseLibraryDescriptorPy> {
    eggsec::nse::all_libraries()
        .iter()
        .map(NseLibraryDescriptorPy::from_engine)
        .collect()
}

/// Look up a library by name from the registry.
///
/// Args:
///     name: Library name (e.g. "stdnse", "http", "dns").
///
/// Returns:
///     NseLibraryDescriptorPy | None: Library descriptor, or None if not found.
#[pyfunction]
pub fn nse_get_library_descriptor(name: &str) -> Option<NseLibraryDescriptorPy> {
    eggsec::nse::find_library(name).map(NseLibraryDescriptorPy::from_engine)
}

// ═══════════════════════════════════════════════════════════════════
// D1: NSE Runtime completion types
// ═══════════════════════════════════════════════════════════════════

/// Metadata about an NSE script.
///
/// Describes a script's name, category, description, and dependencies
/// without loading or executing it.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseScriptMetadataPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub category: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub author: Option<String>,
    #[pyo3(get)]
    pub license: Option<String>,
    #[pyo3(get)]
    pub dependencies: Vec<String>,
    #[pyo3(get)]
    pub targets: Option<String>,
    #[pyo3(get)]
    pub categories: Vec<String>,
    #[pyo3(get)]
    pub is_builtin: bool,
}

#[pymethods]
impl NseScriptMetadataPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("category", &self.category)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("author", &self.author)?;
        dict.set_item("license", &self.license)?;
        dict.set_item("dependencies", &self.dependencies)?;
        dict.set_item("targets", &self.targets)?;
        dict.set_item("categories", &self.categories)?;
        dict.set_item("is_builtin", self.is_builtin)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseScriptMetadata(name={}, category={}, builtin={})",
            self.name, self.category, self.is_builtin
        )
    }

    fn __str__(&self) -> String {
        format!(
            "NSE script '{}' [{}]: {}",
            self.name, self.category, self.description
        )
    }
}

/// Built-in script metadata derived from the resolver registry and
/// `get_builtin_script()` source.
fn builtin_script_metadata(name: &str) -> Option<NseScriptMetadataPy> {
    if !eggsec::nse::is_builtin_script(name) {
        return None;
    }
    let (category, description, dependencies) = match name {
        "default" => (
            "discovery",
            "Default set of discovery scripts",
            vec!["stdnse".to_string()],
        ),
        "discovery" => (
            "discovery",
            "Discovery and enumeration scripts",
            vec!["stdnse".to_string()],
        ),
        "banner" => (
            "discovery",
            "Grab service banners from open ports",
            vec![
                "stdnse".to_string(),
                "comm".to_string(),
                "socket".to_string(),
            ],
        ),
        "http-headers" => (
            "discovery",
            "Display HTTP response headers from web servers",
            vec!["stdnse".to_string(), "http".to_string()],
        ),
        "dns-check" => (
            "discovery",
            "DNS resolution and validation checks",
            vec!["stdnse".to_string(), "dns".to_string()],
        ),
        "ssl-cert" => (
            "discovery",
            "Display SSL/TLS certificate information from targets",
            vec![
                "stdnse".to_string(),
                "sslcert".to_string(),
                "tls".to_string(),
            ],
        ),
        _ => return None,
    };
    Some(NseScriptMetadataPy {
        name: name.to_string(),
        category: category.to_string(),
        description: description.to_string(),
        author: None,
        license: None,
        dependencies,
        targets: None,
        categories: vec![category.to_string()],
        is_builtin: true,
    })
}

/// List available built-in NSE scripts.
///
/// Returns a list of NseScriptMetadataPy objects describing each available
/// built-in script from the resolver registry.
///
/// Args:
///     category: Optional category filter. Matches against the script's
///         category field.
///
/// Returns:
///     list[NseScriptMetadataPy]: Script metadata entries.
#[pyfunction]
#[pyo3(signature = (category=None))]
pub fn nse_list_scripts(category: Option<&str>) -> Vec<NseScriptMetadataPy> {
    let all_names = &[
        "default",
        "discovery",
        "banner",
        "http-headers",
        "dns-check",
        "ssl-cert",
    ];
    let mut scripts: Vec<NseScriptMetadataPy> = all_names
        .iter()
        .filter_map(|name| builtin_script_metadata(name))
        .collect();
    if let Some(cat) = category {
        scripts.retain(|s| s.category == cat || s.categories.contains(&cat.to_string()));
    }
    scripts
}

/// Get metadata for a specific NSE script by name.
///
/// Args:
///     script_name: Name of the script (e.g. "http-headers", "ssl-cert").
///
/// Returns:
///     NseScriptMetadataPy: Script metadata, or None if not found.
#[pyfunction]
pub fn nse_get_script_metadata(script_name: &str) -> PyResult<Option<NseScriptMetadataPy>> {
    Ok(builtin_script_metadata(script_name))
}

/// Run an NSE script using a full NseConfigPy configuration.
///
/// This provides access to all configuration options including script_file,
/// json output, and verbose mode.
///
/// Args:
///     config: NseConfigPy with all execution parameters.
///
/// Returns:
///     NseReportPy: Execution report with output and diagnostics.
///
/// Raises:
///     ScanError: If the NSE execution fails.
#[pyfunction]
pub fn nse_run_with_config(config: &NseConfigPy) -> PyResult<NseReportPy> {
    let eggsec_config = eggsec::nse::NseConfig::new(
        &config.target,
        &config.script,
        config.script_args.as_deref(),
        config.script_file.as_deref(),
        config.json,
        config.verbose,
    );
    run_nse_sync(eggsec_config)
}

/// Validate NSE script syntax without executing it.
///
/// Checks that the script is a recognized built-in name or a non-empty
/// inline Lua source string. Does not run the script or perform network
/// operations. Full Lua syntax validation is deferred to the execution phase.
///
/// Args:
///     script: Lua script source code or built-in script name.
///
/// Returns:
///     dict: Validation result with keys:
///         - "valid" (bool): Whether the script is valid
///         - "error" (str | None): Error message if invalid
///         - "script_name" (str): The script name or "<inline>"
#[pyfunction]
pub fn nse_validate_script(script: &str, py: Python) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);

    if script.is_empty() {
        dict.set_item("valid", false)?;
        dict.set_item("error", "script is empty")?;
        dict.set_item("script_name", "<inline>")?;
        return Ok(dict.into());
    }

    if eggsec::nse::is_builtin_script(script) {
        dict.set_item("valid", true)?;
        dict.set_item("error", Option::<String>::None)?;
        dict.set_item("script_name", script)?;
        return Ok(dict.into());
    }

    // For inline scripts, check basic Lua shebang / structure
    let trimmed = script.trim();
    if trimmed.starts_with("--")
        || trimmed.starts_with("local ")
        || trimmed.starts_with("function ")
        || trimmed.contains("require")
        || trimmed.contains("return")
    {
        dict.set_item("valid", true)?;
        dict.set_item("error", Option::<String>::None)?;
        dict.set_item("script_name", "<inline>")?;
    } else {
        dict.set_item("valid", false)?;
        dict.set_item(
            "error",
            "unrecognized script: not a built-in name and does not look like Lua source",
        )?;
        dict.set_item("script_name", "<inline>")?;
    }
    Ok(dict.into())
}

/// Sandbox policy for NSE script execution.
///
/// Controls filesystem access, network restrictions, and resource limits
/// for scripts running in the NSE sandbox.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseSandboxPolicyPy {
    #[pyo3(get)]
    pub allow_filesystem: bool,
    #[pyo3(get)]
    pub allowed_dirs: Vec<String>,
    #[pyo3(get)]
    pub allow_network: bool,
    #[pyo3(get)]
    pub allowed_cidrs: Vec<String>,
    #[pyo3(get)]
    pub max_lua_instructions: u64,
    #[pyo3(get)]
    pub max_output_bytes: usize,
    #[pyo3(get)]
    pub max_network_ops: usize,
    #[pyo3(get)]
    pub max_memory_bytes: usize,
}

impl NseSandboxPolicyPy {
    pub fn from_engine(config: &eggsec::nse::SandboxConfig) -> Self {
        Self {
            allow_filesystem: config.allowed_dir.is_some(),
            allowed_dirs: config
                .allowed_dir
                .as_ref()
                .map(|p| vec![p.to_string_lossy().to_string()])
                .unwrap_or_default(),
            allow_network: config.allowed_networks.is_empty() || !config.enabled,
            allowed_cidrs: config
                .allowed_networks
                .iter()
                .map(|n| n.to_string())
                .collect(),
            max_lua_instructions: 1_000_000,
            max_output_bytes: 1_048_576,
            max_network_ops: 100,
            max_memory_bytes: 67_108_864,
        }
    }
}

#[pymethods]
impl NseSandboxPolicyPy {
    #[new]
    #[pyo3(signature = (allow_filesystem=false, allowed_dirs=None, allow_network=true, allowed_cidrs=None, max_lua_instructions=1000000, max_output_bytes=1048576, max_network_ops=100, max_memory_bytes=67108864))]
    fn new(
        allow_filesystem: bool,
        allowed_dirs: Option<Vec<String>>,
        allow_network: bool,
        allowed_cidrs: Option<Vec<String>>,
        max_lua_instructions: u64,
        max_output_bytes: usize,
        max_network_ops: usize,
        max_memory_bytes: usize,
    ) -> Self {
        Self {
            allow_filesystem,
            allowed_dirs: allowed_dirs.unwrap_or_default(),
            allow_network,
            allowed_cidrs: allowed_cidrs.unwrap_or_default(),
            max_lua_instructions,
            max_output_bytes,
            max_network_ops,
            max_memory_bytes,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("allow_filesystem", self.allow_filesystem)?;
        dict.set_item("allowed_dirs", &self.allowed_dirs)?;
        dict.set_item("allow_network", self.allow_network)?;
        dict.set_item("allowed_cidrs", &self.allowed_cidrs)?;
        dict.set_item("max_lua_instructions", self.max_lua_instructions)?;
        dict.set_item("max_output_bytes", self.max_output_bytes)?;
        dict.set_item("max_network_ops", self.max_network_ops)?;
        dict.set_item("max_memory_bytes", self.max_memory_bytes)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseSandboxPolicy(fs={}, net={}, max_instr={})",
            self.allow_filesystem, self.allow_network, self.max_lua_instructions
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Target context for NSE script execution.
///
/// Provides host, port, and service information that scripts can use
/// to tailor their behavior to the target.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NseTargetContextPy {
    #[pyo3(get)]
    pub host_ip: String,
    #[pyo3(get)]
    pub hostname: Option<String>,
    #[pyo3(get)]
    pub port: Option<u16>,
    #[pyo3(get)]
    pub protocol: Option<String>,
    #[pyo3(get)]
    pub service_name: Option<String>,
    #[pyo3(get)]
    pub service_product: Option<String>,
    #[pyo3(get)]
    pub service_version: Option<String>,
    #[pyo3(get)]
    pub os_detection: Option<String>,
}

#[pymethods]
impl NseTargetContextPy {
    #[new]
    #[pyo3(signature = (host_ip, hostname=None, port=None, protocol=None, service_name=None, service_product=None, service_version=None, os_detection=None))]
    fn new(
        host_ip: &str,
        hostname: Option<&str>,
        port: Option<u16>,
        protocol: Option<&str>,
        service_name: Option<&str>,
        service_product: Option<&str>,
        service_version: Option<&str>,
        os_detection: Option<&str>,
    ) -> Self {
        Self {
            host_ip: host_ip.to_string(),
            hostname: hostname.map(|s| s.to_string()),
            port,
            protocol: protocol.map(|s| s.to_string()),
            service_name: service_name.map(|s| s.to_string()),
            service_product: service_product.map(|s| s.to_string()),
            service_version: service_version.map(|s| s.to_string()),
            os_detection: os_detection.map(|s| s.to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("host_ip", &self.host_ip)?;
        dict.set_item("hostname", &self.hostname)?;
        dict.set_item("port", self.port)?;
        dict.set_item("protocol", &self.protocol)?;
        dict.set_item("service_name", &self.service_name)?;
        dict.set_item("service_product", &self.service_product)?;
        dict.set_item("service_version", &self.service_version)?;
        dict.set_item("os_detection", &self.os_detection)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NseTargetContext(host={}, port={:?}, service={:?})",
            self.host_ip, self.port, self.service_name
        )
    }

    fn __str__(&self) -> String {
        let service = self.service_name.as_deref().unwrap_or("unknown");
        format!("{} ({})", self.host_ip, service)
    }
}
