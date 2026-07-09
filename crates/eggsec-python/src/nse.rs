use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::ScanError;
use crate::runtime_async;
use crate::runtime_sync;

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

fn build_nse_config(
    target: &str,
    script: &str,
    script_args: Option<&str>,
    verbose: bool,
) -> eggsec::nse::NseConfig {
    eggsec::nse::NseConfig::new(target, script, script_args, None, false, verbose)
}

fn run_nse_sync(config: eggsec::nse::NseConfig) -> PyResult<NseReportPy> {
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            run_nse_inner(config)
                .await
                .map_err(|e| ScanError::new_err(format!("NSE execution failed: {}", e)))
        })?;
        Ok(NseReportPy::from_engine(result))
    })
}

fn run_nse_async(config: eggsec::nse::NseConfig) -> PyResult<runtime_async::PyFuture> {
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
