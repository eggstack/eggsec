use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Wrapper for `eggsec::audit::AuditOutcome`.
///
/// Simplified outcome for audit events, derived from enforcement decisions.
#[pyclass(frozen, name = "AuditOutcomePy")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditOutcomePy {
    inner: eggsec::audit::AuditOutcome,
}

#[pymethods]
impl AuditOutcomePy {
    /// The kebab-case name of the outcome variant.
    #[getter]
    fn name(&self) -> String {
        match self.inner {
            eggsec::audit::AuditOutcome::Allow => "allow".to_string(),
            eggsec::audit::AuditOutcome::Warn => "warn".to_string(),
            eggsec::audit::AuditOutcome::Confirmed => "confirmed".to_string(),
            eggsec::audit::AuditOutcome::Deny => "deny".to_string(),
            eggsec::audit::AuditOutcome::ConfirmationRequired => {
                "confirmation-required".to_string()
            }
        }
    }

    fn __str__(&self) -> String {
        self.name()
    }

    fn __repr__(&self) -> String {
        format!("AuditOutcomePy({})", self.name())
    }

    #[staticmethod]
    fn allow() -> Self {
        Self {
            inner: eggsec::audit::AuditOutcome::Allow,
        }
    }

    #[staticmethod]
    fn warn() -> Self {
        Self {
            inner: eggsec::audit::AuditOutcome::Warn,
        }
    }

    #[staticmethod]
    fn confirmed() -> Self {
        Self {
            inner: eggsec::audit::AuditOutcome::Confirmed,
        }
    }

    #[staticmethod]
    fn deny() -> Self {
        Self {
            inner: eggsec::audit::AuditOutcome::Deny,
        }
    }

    #[staticmethod]
    fn confirmation_required() -> Self {
        Self {
            inner: eggsec::audit::AuditOutcome::ConfirmationRequired,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", self.name())?;
        Ok(dict.into())
    }
}

impl AuditOutcomePy {
    pub fn from_engine(outcome: eggsec::audit::AuditOutcome) -> Self {
        Self { inner: outcome }
    }

    pub fn into_engine(self) -> eggsec::audit::AuditOutcome {
        self.inner
    }
}

/// Wrapper for `eggsec::audit::ManualOverrideAudit`.
///
/// Audit record for manual override details.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ManualOverrideAuditPy {
    inner: eggsec::audit::ManualOverrideAudit,
}

#[pymethods]
impl ManualOverrideAuditPy {
    /// The reason provided for the manual override.
    #[getter]
    fn reason(&self) -> Option<String> {
        self.inner.reason.clone()
    }

    /// The confirmation classes that were overridden.
    #[getter]
    fn classes(&self) -> Vec<String> {
        self.inner.classes.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("reason", &self.inner.reason)?;
        let classes_list = PyList::empty_bound(py);
        for c in &self.inner.classes {
            classes_list.append(c.as_str())?;
        }
        dict.set_item("classes", classes_list)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "ManualOverrideAuditPy(reason={:?}, classes={:?})",
            self.inner.reason, self.inner.classes
        )
    }
}

impl ManualOverrideAuditPy {
    pub fn from_engine(audit: eggsec::audit::ManualOverrideAudit) -> Self {
        Self { inner: audit }
    }
}

/// Wrapper for `eggsec::audit::ScopeAudit`.
///
/// Scope provenance summary for audit events.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ScopeAuditPy {
    inner: eggsec::audit::ScopeAudit,
}

#[pymethods]
impl ScopeAuditPy {
    /// The source of the scope (e.g., "config-file", "cli-scope-file", "default-empty").
    #[getter]
    fn source(&self) -> String {
        match self.inner.source {
            eggsec::config::ScopeSource::DefaultEmpty => "default-empty".to_string(),
            eggsec::config::ScopeSource::ConfigFile => "config-file".to_string(),
            eggsec::config::ScopeSource::CliScopeFile => "cli-scope-file".to_string(),
            eggsec::config::ScopeSource::GeneratedPreset => "generated-preset".to_string(),
        }
    }

    /// Optional path to the scope manifest file.
    #[getter]
    fn path(&self) -> Option<String> {
        self.inner.path.clone()
    }

    /// Number of allow rules in the scope.
    #[getter]
    fn allow_rule_count(&self) -> usize {
        self.inner.allow_rule_count
    }

    /// Number of exclusion rules in the scope.
    #[getter]
    fn exclusion_rule_count(&self) -> usize {
        self.inner.exclusion_rule_count
    }

    /// Whether the scope came from an explicit manifest.
    #[getter]
    fn explicit_manifest(&self) -> bool {
        self.inner.explicit_manifest
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("source", self.source())?;
        dict.set_item("path", &self.inner.path)?;
        dict.set_item("allow_rule_count", self.inner.allow_rule_count)?;
        dict.set_item("exclusion_rule_count", self.inner.exclusion_rule_count)?;
        dict.set_item("explicit_manifest", self.inner.explicit_manifest)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "ScopeAuditPy(source={}, allow_rules={}, exclusion_rules={})",
            self.source(),
            self.inner.allow_rule_count,
            self.inner.exclusion_rule_count
        )
    }
}

impl ScopeAuditPy {
    pub fn from_engine(audit: eggsec::audit::ScopeAudit) -> Self {
        Self { inner: audit }
    }
}

/// Wrapper for `eggsec::audit::EnforcementAuditEvent`.
///
/// Normalized audit event for enforcement decisions across all execution surfaces.
#[pyclass(frozen, name = "EnforcementAuditEventPy")]
#[derive(Debug, Clone)]
pub struct EnforcementAuditEventPy {
    inner: eggsec::audit::EnforcementAuditEvent,
}

#[pymethods]
impl EnforcementAuditEventPy {
    /// Unique event identifier.
    #[getter]
    fn event_id(&self) -> String {
        self.inner.event_id.clone()
    }

    /// ISO 8601 timestamp of the event.
    #[getter]
    fn timestamp(&self) -> String {
        self.inner.timestamp.to_rfc3339()
    }

    /// The execution surface that triggered the event.
    #[getter]
    fn surface(&self) -> String {
        format!("{}", self.inner.surface)
    }

    /// The execution profile active for this event.
    #[getter]
    fn profile(&self) -> String {
        format!("{}", self.inner.profile)
    }

    /// The operation identifier.
    #[getter]
    fn operation_id(&self) -> String {
        self.inner.operation_id.clone()
    }

    /// The target being operated on, if any.
    #[getter]
    fn target(&self) -> Option<String> {
        self.inner.target.clone()
    }

    /// The audit outcome (allow, warn, confirmed, deny, confirmation-required).
    #[getter]
    fn outcome(&self) -> AuditOutcomePy {
        AuditOutcomePy::from_engine(self.inner.outcome)
    }

    /// Summary of the policy decision.
    #[getter]
    fn decision_summary(&self) -> String {
        if self.inner.decision.allowed {
            format!("allowed (operation: {})", self.inner.operation_id)
        } else {
            format!(
                "denied ({} reasons)",
                self.inner.decision.denied_reasons.len()
            )
        }
    }

    /// The confirmation classes that were required.
    #[getter]
    fn confirmation_classes(&self) -> Vec<String> {
        self.inner
            .confirmation_classes
            .iter()
            .map(|c| c.as_str().to_string())
            .collect()
    }

    /// The manual override details, if a manual override was applied.
    #[getter]
    fn manual_override(&self) -> Option<ManualOverrideAuditPy> {
        self.inner
            .manual_override
            .as_ref()
            .map(|mo| ManualOverrideAuditPy::from_engine(mo.clone()))
    }

    /// Scope provenance summary.
    #[getter]
    fn scope(&self) -> ScopeAuditPy {
        ScopeAuditPy::from_engine(self.inner.scope.clone())
    }

    /// Hash of the policy that produced this decision.
    #[getter]
    fn policy_hash(&self) -> Option<String> {
        self.inner.policy_hash.clone()
    }

    /// Optional correlation ID for linking related events.
    #[getter]
    fn correlation_id(&self) -> Option<String> {
        self.inner.correlation_id.clone()
    }

    /// Whether a manual override was ignored (automated surface).
    #[getter]
    fn manual_override_ignored(&self) -> bool {
        self.inner.manual_override_ignored
    }

    /// The decision ID from the policy decision.
    #[getter]
    fn decision_id(&self) -> String {
        self.inner.decision.decision_id.clone()
    }

    /// Optional metadata ID.
    #[getter]
    fn metadata_id(&self) -> Option<String> {
        self.inner.metadata_id.clone()
    }

    /// Serialize the event to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("event_id", &self.inner.event_id)?;
        dict.set_item("timestamp", self.inner.timestamp.to_rfc3339())?;
        dict.set_item("surface", format!("{}", self.inner.surface))?;
        dict.set_item("profile", format!("{}", self.inner.profile))?;
        dict.set_item("operation_id", &self.inner.operation_id)?;
        dict.set_item("target", &self.inner.target)?;

        let outcome = AuditOutcomePy::from_engine(self.inner.outcome);
        dict.set_item("outcome", outcome.to_dict(py)?)?;

        dict.set_item("decision_id", &self.inner.decision.decision_id)?;
        dict.set_item("decision_allowed", self.inner.decision.allowed)?;
        dict.set_item("decision_summary", self.decision_summary())?;

        let confirmation_list = PyList::empty_bound(py);
        for c in &self.inner.confirmation_classes {
            confirmation_list.append(c.as_str())?;
        }
        dict.set_item("confirmation_classes", confirmation_list)?;

        match &self.inner.manual_override {
            Some(mo) => {
                let mo_audit = ManualOverrideAuditPy::from_engine(mo.clone());
                dict.set_item("manual_override", mo_audit.to_dict(py)?)?;
            }
            None => {
                dict.set_item("manual_override", py.None())?;
            }
        }

        dict.set_item(
            "manual_override_ignored",
            self.inner.manual_override_ignored,
        )?;

        let scope_audit = ScopeAuditPy::from_engine(self.inner.scope.clone());
        dict.set_item("scope", scope_audit.to_dict(py)?)?;

        dict.set_item("policy_hash", &self.inner.policy_hash)?;
        dict.set_item("metadata_id", &self.inner.metadata_id)?;
        dict.set_item("correlation_id", &self.inner.correlation_id)?;

        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        let outcome_name = match self.inner.outcome {
            eggsec::audit::AuditOutcome::Allow => "allow",
            eggsec::audit::AuditOutcome::Warn => "warn",
            eggsec::audit::AuditOutcome::Confirmed => "confirmed",
            eggsec::audit::AuditOutcome::Deny => "deny",
            eggsec::audit::AuditOutcome::ConfirmationRequired => "confirmation-required",
        };
        format!(
            "EnforcementAuditEventPy(event_id={}, outcome={}, operation={}, surface={})",
            self.inner.event_id,
            outcome_name,
            self.inner.operation_id,
            self.inner.surface.label(),
        )
    }
}

impl EnforcementAuditEventPy {
    pub fn from_engine(event: eggsec::audit::EnforcementAuditEvent) -> Self {
        Self { inner: event }
    }

    pub fn into_engine(self) -> eggsec::audit::EnforcementAuditEvent {
        self.inner
    }
}

/// Create an audit event from an enforcement outcome.
///
/// This is the primary builder for recording enforcement decisions at the
/// point of evaluation, whether from preflight, CLI, TUI, REST, MCP, or agent.
#[pyfunction]
#[pyo3(signature = (surface, enforcement, descriptor, outcome, confirmed, override_ignored, manual_override, required_classes, correlation_id, metadata_id))]
pub fn audit_event_from_enforcement(
    py: Python,
    surface: String,
    enforcement: PyObject,
    descriptor: PyObject,
    outcome: PyObject,
    confirmed: bool,
    override_ignored: bool,
    manual_override: Option<PyObject>,
    required_classes: Vec<String>,
    correlation_id: Option<String>,
    metadata_id: Option<String>,
) -> PyResult<EnforcementAuditEventPy> {
    // For Python bindings, we accept serialized representations and reconstruct
    // the Rust types. This is a simplified bridge that works when the Python side
    // provides the data as simple types (strings, dicts).
    //
    // The full EnforcementContext / OperationDescriptor / EnforcementOutcome
    // construction from Python dicts is complex; this function delegates to the
    // engine's builder after extracting what we need.
    //
    // For now, we build a minimal event from the string parameters. The Python
    // caller should construct the full event via the engine's Rust API when
    // possible, or use this for audit-only paths where the enriched fields
    // (decision, scope details) are not needed.
    let _ = py; // suppress unused py warning

    let surface_enum = parse_execution_surface(&surface)?;
    let classes: Vec<eggsec::config::ConfirmationClass> = required_classes
        .iter()
        .map(|s| parse_confirmation_class(s))
        .collect::<PyResult<_>>()?;

    let manual_override_audit = if confirmed {
        // When confirmed and override provided, build a simplified audit record
        manual_override.and_then(|mo| {
            let dict: PyResult<Bound<'_, PyDict>> = mo.extract(py);
            match dict {
                Ok(d) => {
                    let reason = d
                        .get_item("reason")
                        .ok()
                        .flatten()
                        .and_then(|v| v.extract::<Option<String>>().ok().flatten());
                    let classes_obj = d
                        .get_item("classes")
                        .ok()
                        .flatten()
                        .and_then(|v| v.extract::<Vec<String>>().ok());
                    Some(eggsec::audit::ManualOverrideAudit {
                        reason,
                        classes: classes_obj.unwrap_or_default(),
                    })
                }
                Err(_) => None,
            }
        })
    } else {
        None
    };

    let event = eggsec::audit::EnforcementAuditEvent {
        event_id: format!("evt-{}", chrono::Utc::now().timestamp_millis()),
        timestamp: chrono::Utc::now(),
        surface: surface_enum,
        profile: surface_enum.profile(),
        operation_id: descriptor
            .extract::<String>(py)
            .unwrap_or_else(|_| "unknown".to_string()),
        target: None,
        outcome: eggsec::audit::AuditOutcome::from_outcome(
            &build_minimal_outcome(py, &outcome)?,
            confirmed,
        ),
        decision: eggsec::config::PolicyDecision::allowed(
            "unknown",
            eggsec::config::OperationMode::StandardAssessment,
            eggsec::config::OperationRisk::SafeActive,
            vec![],
        ),
        confirmation_classes: classes,
        manual_override: manual_override_audit,
        manual_override_ignored: override_ignored,
        scope: eggsec::audit::ScopeAudit {
            source: eggsec::config::ScopeSource::DefaultEmpty,
            path: None,
            allow_rule_count: 0,
            exclusion_rule_count: 0,
            explicit_manifest: false,
        },
        policy_hash: None,
        metadata_id: metadata_id,
        correlation_id: correlation_id,
    };

    Ok(EnforcementAuditEventPy::from_engine(event))
}

/// Create an audit event from a preflight result.
///
/// Use this for preflight advisory evaluations that do not result in dispatch.
#[pyfunction]
#[pyo3(signature = (surface, enforcement, descriptor, outcome, manual_override, required_classes, correlation_id))]
pub fn audit_event_from_preflight(
    py: Python,
    surface: String,
    enforcement: PyObject,
    descriptor: PyObject,
    outcome: PyObject,
    manual_override: Option<PyObject>,
    required_classes: Vec<String>,
    correlation_id: Option<String>,
) -> PyResult<EnforcementAuditEventPy> {
    // Preflight never confirms and never ignores overrides.
    audit_event_from_enforcement(
        py,
        surface,
        enforcement,
        descriptor,
        outcome,
        false, // confirmed = false (preflight never confirms)
        false, // override_ignored = false (preflight never ignores)
        manual_override,
        required_classes,
        correlation_id,
        None,
    )
}

/// Emit the audit event at the appropriate tracing level.
///
/// Writes the event to the audit log via the engine's tracing infrastructure.
#[pyfunction]
pub fn emit_audit_event(py: Python, event: &EnforcementAuditEventPy) -> PyResult<()> {
    let _ = py;
    eggsec::audit::emit_audit_event(&event.inner);
    Ok(())
}

// --- Helper functions ---

fn parse_execution_surface(surface: &str) -> PyResult<eggsec::config::ExecutionSurface> {
    match surface {
        "cli-manual" => Ok(eggsec::config::ExecutionSurface::CliManual),
        "tui-manual" => Ok(eggsec::config::ExecutionSurface::TuiManual),
        "cli-manual-strict" => Ok(eggsec::config::ExecutionSurface::CliManualStrict),
        "tui-manual-strict" => Ok(eggsec::config::ExecutionSurface::TuiManualStrict),
        "mcp-server" => Ok(eggsec::config::ExecutionSurface::McpServer),
        "security-agent" => Ok(eggsec::config::ExecutionSurface::SecurityAgent),
        "ci" => Ok(eggsec::config::ExecutionSurface::Ci),
        "rest-api" => Ok(eggsec::config::ExecutionSurface::RestApi),
        "grpc-api" => Ok(eggsec::config::ExecutionSurface::GrpcApi),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Invalid execution surface: '{}'. Must be one of: cli-manual, tui-manual, cli-manual-strict, tui-manual-strict, mcp-server, security-agent, ci, rest-api, grpc-api",
            surface
        ))),
    }
}

fn parse_confirmation_class(class: &str) -> PyResult<eggsec::config::ConfirmationClass> {
    match class {
        "out-of-scope" => Ok(eggsec::config::ConfirmationClass::OutOfScope),
        "explicit-exclusion" => Ok(eggsec::config::ConfirmationClass::ExplicitExclusion),
        "high-risk" => Ok(eggsec::config::ConfirmationClass::HighRisk),
        "nonbaseline-capability" => Ok(eggsec::config::ConfirmationClass::NonBaselineCapability),
        "private-resolution" => Ok(eggsec::config::ConfirmationClass::PrivateResolution),
        "cross-host-redirect" => Ok(eggsec::config::ConfirmationClass::CrossHostRedirect),
        "target-expansion" => Ok(eggsec::config::ConfirmationClass::TargetExpansion),
        "traffic-interception" => Ok(eggsec::config::ConfirmationClass::TrafficInterception),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Invalid confirmation class: '{}'",
            class
        ))),
    }
}

fn build_minimal_outcome(
    py: Python,
    outcome: &PyObject,
) -> PyResult<eggsec::config::EnforcementOutcome> {
    // Attempt to extract a string label from the outcome object
    let label = outcome
        .extract::<String>(py)
        .unwrap_or_else(|_| "allow".to_string());

    let decision = eggsec::config::PolicyDecision::allowed(
        "unknown",
        eggsec::config::OperationMode::StandardAssessment,
        eggsec::config::OperationRisk::SafeActive,
        vec![],
    );

    match label.as_str() {
        "allow" | "Allow" => Ok(eggsec::config::EnforcementOutcome::Allow(decision)),
        "warn" | "Warn" => Ok(eggsec::config::EnforcementOutcome::Warn(decision)),
        "deny" | "Deny" => Ok(eggsec::config::EnforcementOutcome::Deny(decision)),
        _ => Ok(eggsec::config::EnforcementOutcome::Allow(decision)),
    }
}
