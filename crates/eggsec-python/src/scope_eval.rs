use pyo3::prelude::*;

use crate::error::ScopeError;

/// Provenance of a loaded scope manifest.
///
/// Distinguishes between "no scope provided" and "user explicitly supplied
/// an empty scope". Strict execution profiles require an explicit manifest
/// for networked operations.
#[pyclass(frozen, name = "ScopeSource")]
#[derive(Clone)]
pub struct ScopeSourcePy {
    inner: eggsec::config::ScopeSource,
}

#[pymethods]
impl ScopeSourcePy {
    #[staticmethod]
    fn default_empty() -> Self {
        Self {
            inner: eggsec::config::ScopeSource::DefaultEmpty,
        }
    }

    #[staticmethod]
    fn config_file() -> Self {
        Self {
            inner: eggsec::config::ScopeSource::ConfigFile,
        }
    }

    #[staticmethod]
    fn cli_scope_file() -> Self {
        Self {
            inner: eggsec::config::ScopeSource::CliScopeFile,
        }
    }

    #[staticmethod]
    fn generated_preset() -> Self {
        Self {
            inner: eggsec::config::ScopeSource::GeneratedPreset,
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.inner {
            eggsec::config::ScopeSource::DefaultEmpty => "default-empty",
            eggsec::config::ScopeSource::ConfigFile => "config-file",
            eggsec::config::ScopeSource::CliScopeFile => "cli-scope-file",
            eggsec::config::ScopeSource::GeneratedPreset => "generated-preset",
        }
    }

    fn __repr__(&self) -> String {
        format!("ScopeSource({})", self.name())
    }

    fn __str__(&self) -> String {
        self.name().to_string()
    }
}

/// A scope with provenance metadata.
///
/// Wraps [`Scope`] with information about where it was loaded from, enabling
/// strict execution paths to distinguish "no scope" from "explicit empty scope".
#[pyclass(frozen, name = "LoadedScope")]
#[derive(Clone)]
pub struct LoadedScopePy {
    inner: eggsec::config::LoadedScope,
}

#[pymethods]
impl LoadedScopePy {
    #[staticmethod]
    fn default_empty() -> Self {
        Self {
            inner: eggsec::config::LoadedScope::default_empty(),
        }
    }

    /// Create a LoadedScope with explicit provenance.
    ///
    /// Args:
    ///     scope: The Scope to wrap.
    ///     source: The ScopeSource indicating where this scope came from.
    ///     path: Optional file path the scope was loaded from.
    #[staticmethod]
    #[pyo3(signature = (scope, source, path=None))]
    fn explicit(scope: &crate::scope::Scope, source: &ScopeSourcePy, path: Option<&str>) -> Self {
        Self {
            inner: eggsec::config::LoadedScope::explicit(
                scope.inner.clone(),
                source.inner,
                path.map(|p| p.to_string()),
            ),
        }
    }

    #[getter]
    fn source(&self) -> ScopeSourcePy {
        ScopeSourcePy {
            inner: self.inner.source,
        }
    }

    #[getter]
    fn path(&self) -> Option<String> {
        self.inner.path.clone()
    }

    #[getter]
    fn is_explicit(&self) -> bool {
        self.inner.is_explicit_manifest()
    }

    /// Check if a target is allowed by this scope.
    ///
    /// Args:
    ///     target: Hostname or IP to check.
    ///
    /// Returns:
    ///     bool: True if the target is allowed.
    fn is_target_allowed(&self, target: &str) -> PyResult<bool> {
        self.inner
            .scope
            .is_target_allowed(target)
            .map_err(|e| ScopeError::new_err(e.to_string()))
    }

    /// Check if a port is allowed by this scope.
    ///
    /// Args:
    ///     port: Port number to check.
    ///
    /// Returns:
    ///     bool: True if the port is allowed.
    fn is_port_allowed(&self, port: u16) -> bool {
        self.inner.scope.is_port_allowed(port)
    }

    /// Check if a target is explicitly excluded from scope.
    ///
    /// Args:
    ///     target: Hostname or IP to check.
    ///
    /// Returns:
    ///     bool: True if the target matches an explicit exclusion rule.
    fn is_excluded(&self, target: &str) -> bool {
        self.inner.scope.is_excluded(target)
    }

    /// Get allowed target rules.
    ///
    /// Returns:
    ///     list[ScopeRule]: The allowed target rules.
    #[getter]
    fn allowed_targets(&self) -> Vec<ScopeRulePy> {
        self.inner
            .scope
            .allowed_targets
            .iter()
            .map(|r| ScopeRulePy { inner: r.clone() })
            .collect()
    }

    /// Get excluded target rules.
    ///
    /// Returns:
    ///     list[ScopeRule]: The excluded target rules.
    #[getter]
    fn excluded_targets(&self) -> Vec<ScopeRulePy> {
        self.inner
            .scope
            .excluded_targets
            .iter()
            .map(|r| ScopeRulePy { inner: r.clone() })
            .collect()
    }

    /// Get allowed ports.
    ///
    /// Returns:
    ///     list[int] | None: The allowed ports, or None if unrestricted.
    #[getter]
    fn allowed_ports(&self) -> Option<Vec<u16>> {
        self.inner.scope.allowed_ports.clone()
    }

    /// Get excluded ports.
    ///
    /// Returns:
    ///     list[int]: The excluded ports.
    #[getter]
    fn excluded_ports(&self) -> Vec<u16> {
        self.inner.scope.excluded_ports.clone()
    }

    /// Get the maximum requests per second limit.
    ///
    /// Returns:
    ///     int | None: The rate limit, or None if unrestricted.
    #[getter]
    fn max_requests_per_second(&self) -> Option<u32> {
        self.inner.scope.max_requests_per_second
    }

    /// Check if explicit scope is required.
    ///
    /// Returns:
    ///     bool: True if explicit scope is required.
    #[getter]
    fn require_explicit_scope(&self) -> bool {
        self.inner.scope.require_explicit_scope
    }

    /// Get a detailed explanation of whether a target is allowed or excluded.
    ///
    /// Args:
    ///     target: Hostname or IP to explain.
    ///
    /// Returns:
    ///     ScopeExplanation: Detailed explanation of the scope decision.
    fn explain(&self, target: &str) -> PyResult<ScopeExplanationPy> {
        let scope = &self.inner.scope;

        let mut allowed = false;
        let mut reason = String::new();
        let mut matched_rules: Vec<String> = Vec::new();

        for rule in &scope.allowed_targets {
            let target_scope = match eggsec::config::TargetScope::parse_hostname_only(target) {
                Ok(ts) => ts,
                Err(e) => {
                    return Ok(ScopeExplanationPy {
                        allowed: false,
                        reason: format!("Invalid target: {}", e),
                        matched_rules: vec![],
                        excluded: false,
                        exclusion_reason: None,
                    });
                }
            };
            if rule.matches(&target_scope) {
                allowed = true;
                let desc = rule
                    .description
                    .clone()
                    .or_else(|| {
                        if rule.pattern.is_empty() {
                            rule.cidr.clone()
                        } else {
                            Some(rule.pattern.clone())
                        }
                    })
                    .unwrap_or_default();
                matched_rules.push(desc);
            }
        }

        if allowed {
            reason = "Target matches one or more allowed scope rules".to_string();
        } else if scope.allowed_targets.is_empty() && !scope.require_explicit_scope {
            allowed = true;
            reason = "No scope rules defined; all targets allowed by default".to_string();
        } else if scope.require_explicit_scope {
            reason = "Explicit scope required but no matching rule found".to_string();
        } else {
            reason = "Target does not match any allowed scope rule".to_string();
        }

        let mut excluded = false;
        let mut exclusion_reason: Option<String> = None;

        for rule in &scope.excluded_targets {
            let target_scope = match eggsec::config::TargetScope::parse_hostname_only(target) {
                Ok(ts) => ts,
                Err(_) => continue,
            };
            if rule.matches(&target_scope) {
                excluded = true;
                let desc = rule
                    .description
                    .clone()
                    .or_else(|| {
                        if rule.pattern.is_empty() {
                            rule.cidr.clone()
                        } else {
                            Some(rule.pattern.clone())
                        }
                    })
                    .unwrap_or_default();
                exclusion_reason = Some(format!("Target matches exclusion rule: {}", desc));
                break;
            }
        }

        if excluded {
            allowed = false;
            reason = exclusion_reason
                .clone()
                .unwrap_or_else(|| "Target is explicitly excluded".to_string());
        }

        Ok(ScopeExplanationPy {
            allowed,
            reason,
            matched_rules,
            excluded,
            exclusion_reason,
        })
    }

    fn __repr__(&self) -> String {
        let targets: Vec<String> = self
            .inner
            .scope
            .allowed_targets
            .iter()
            .map(|r| {
                if let Some(ref cidr) = r.cidr {
                    cidr.clone()
                } else {
                    r.pattern.clone()
                }
            })
            .collect();
        format!(
            "LoadedScope(source={}, path={:?}, targets={:?})",
            self.source().name(),
            self.inner.path,
            targets
        )
    }
}

impl LoadedScopePy {
    /// Access the inner eggsec LoadedScope for enforcement context construction.
    pub(crate) fn as_inner(&self) -> &eggsec::config::LoadedScope {
        &self.inner
    }
}

/// A single scope rule (allowed or excluded target).
#[pyclass(frozen, name = "ScopeRule")]
#[derive(Clone)]
pub struct ScopeRulePy {
    inner: eggsec::config::ScopeRule,
}

#[pymethods]
impl ScopeRulePy {
    /// The pattern string (hostname, wildcard, or CIDR notation).
    ///
    /// Returns:
    ///     str | None: The pattern, or None if this is a CIDR-only rule.
    #[getter]
    fn pattern(&self) -> Option<String> {
        let p = self.inner.pattern.clone();
        if p.is_empty() {
            None
        } else {
            Some(p)
        }
    }

    /// The CIDR notation range.
    ///
    /// Returns:
    ///     str | None: The CIDR string, or None if this is a pattern-only rule.
    #[getter]
    fn cidr(&self) -> Option<String> {
        self.inner.cidr.clone()
    }

    /// Human-readable description of this rule.
    ///
    /// Returns:
    ///     str | None: The description, or None if not set.
    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    fn __repr__(&self) -> String {
        if let Some(ref cidr) = self.inner.cidr {
            format!("ScopeRule(cidr={:?})", cidr)
        } else {
            format!("ScopeRule(pattern={:?})", self.inner.pattern)
        }
    }
}

/// Detailed explanation of a scope decision for a target.
///
/// Produced by [`LoadedScope.explain()`] to describe why a target is
/// allowed or excluded from scope.
#[pyclass(frozen, name = "ScopeExplanation")]
#[derive(Clone)]
pub struct ScopeExplanationPy {
    /// Whether the target is ultimately allowed.
    allowed: bool,
    /// Human-readable reason for the decision.
    reason: String,
    /// Descriptions of allowed scope rules that matched (if any).
    matched_rules: Vec<String>,
    /// Whether the target matches an explicit exclusion rule.
    excluded: bool,
    /// Reason for exclusion, if applicable.
    exclusion_reason: Option<String>,
}

#[pymethods]
impl ScopeExplanationPy {
    #[getter]
    fn allowed(&self) -> bool {
        self.allowed
    }

    #[getter]
    fn reason(&self) -> String {
        self.reason.clone()
    }

    #[getter]
    fn matched_rules(&self) -> Vec<String> {
        self.matched_rules.clone()
    }

    #[getter]
    fn excluded(&self) -> bool {
        self.excluded
    }

    #[getter]
    fn exclusion_reason(&self) -> Option<String> {
        self.exclusion_reason.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ScopeExplanation(allowed={}, excluded={}, reason={:?})",
            self.allowed, self.excluded, self.reason
        )
    }
}

/// Validation result for a scope configuration.
///
/// Produced by validating a scope against its internal rules (e.g.,
/// checking for empty allowed_targets when require_explicit_scope is true,
/// duplicate ports, or invalid rate limits).
#[pyclass(frozen, name = "ScopeValidation")]
#[derive(Clone)]
pub struct ScopeValidationPy {
    /// Whether the scope configuration is valid.
    valid: bool,
    /// Validation errors (empty if valid).
    errors: Vec<String>,
    /// Non-fatal warnings.
    warnings: Vec<String>,
    /// Number of allowed target rules.
    target_count: usize,
    /// Number of excluded target rules.
    exclusion_count: usize,
}

#[pymethods]
impl ScopeValidationPy {
    #[getter]
    fn valid(&self) -> bool {
        self.valid
    }

    #[getter]
    fn errors(&self) -> Vec<String> {
        self.errors.clone()
    }

    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.warnings.clone()
    }

    #[getter]
    fn target_count(&self) -> usize {
        self.target_count
    }

    #[getter]
    fn exclusion_count(&self) -> usize {
        self.exclusion_count
    }

    fn __repr__(&self) -> String {
        format!(
            "ScopeValidation(valid={}, targets={}, exclusions={}, errors={})",
            self.valid,
            self.target_count,
            self.exclusion_count,
            self.errors.len()
        )
    }
}

/// Validate a loaded scope configuration.
///
/// Args:
///     scope: The loaded scope to validate.
///
/// Returns:
///     ScopeValidation: Validation result with errors and warnings.
#[pyfunction]
pub fn validate_scope(scope: &LoadedScopePy) -> ScopeValidationPy {
    let inner = &scope.inner.scope;
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    if let Err(e) = inner.validate() {
        errors.push(e.to_string());
    }

    let target_count = inner.allowed_targets.len();
    let exclusion_count = inner.excluded_targets.len();

    if target_count == 0 && inner.require_explicit_scope {
        warnings
            .push("require_explicit_scope is true but no allowed targets are defined".to_string());
    }

    ScopeValidationPy {
        valid: errors.is_empty(),
        errors,
        warnings,
        target_count,
        exclusion_count,
    }
}
