use pyo3::prelude::*;

use crate::authorization::{ExecutionPolicyPy, ManualOverridePy};
use crate::error::EnforcementError as PyEnforcementError;
use crate::execution_context::{
    EnforcementContextPy, EnforcementOutcomePy, OperationDescriptorPy, PolicyDecisionPy,
};
use crate::scope_eval::LoadedScopePy;

// ---------------------------------------------------------------------------
// PreflightResultPy
// ---------------------------------------------------------------------------

/// Python wrapper for Eggsec preflight result.
///
/// Preview of what would happen if an operation were dispatched, without
/// actually executing it. Includes outcome, confirmation requirements,
/// and suggested CLI flags.
#[pyclass(frozen, name = "PreflightResultPy")]
#[derive(Clone)]
pub struct PreflightResultPy {
    inner: eggsec::config::PreflightResult,
}

#[pymethods]
impl PreflightResultPy {
    #[getter]
    fn operation_id(&self) -> String {
        self.inner.descriptor.operation.clone()
    }

    #[getter]
    fn target(&self) -> Option<String> {
        self.inner.descriptor.target.clone()
    }

    /// The outcome kind: "allow", "warn", "confirmation-required", or "deny".
    #[getter]
    fn outcome(&self) -> String {
        self.inner.outcome_kind.label().to_string()
    }

    #[getter]
    fn requires_confirmation(&self) -> bool {
        matches!(
            self.inner.outcome_kind,
            eggsec::config::PreflightOutcomeKind::RequireConfirmation
        )
    }

    #[getter]
    fn confirmation_classes(&self) -> Vec<String> {
        self.inner
            .required_confirmation_classes
            .iter()
            .map(|c| c.as_str().to_string())
            .collect()
    }

    #[getter]
    fn suggested_cli_flags(&self) -> Vec<String> {
        self.inner.suggested_cli_flags.clone()
    }

    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.decision.warnings.clone()
    }

    #[getter]
    fn decision(&self) -> PolicyDecisionPy {
        PolicyDecisionPy {
            inner: self.inner.decision.clone(),
        }
    }

    /// Scope status: "default-empty", "config-file", "cli-scope-file", "generated-preset".
    #[getter]
    fn scope_status(&self) -> String {
        match self.inner.scope_source {
            eggsec::config::ScopeSource::DefaultEmpty => "default-empty".to_string(),
            eggsec::config::ScopeSource::ConfigFile => "config-file".to_string(),
            eggsec::config::ScopeSource::CliScopeFile => "cli-scope-file".to_string(),
            eggsec::config::ScopeSource::GeneratedPreset => "generated-preset".to_string(),
        }
    }

    #[getter]
    fn risk_level(&self) -> String {
        self.inner.descriptor.risk.to_string()
    }

    #[getter]
    fn surface(&self) -> String {
        self.inner.surface.to_string()
    }

    #[getter]
    fn profile(&self) -> String {
        self.inner.profile.to_string()
    }

    #[getter]
    fn manual_override_honored(&self) -> bool {
        self.inner.manual_override_honored
    }

    #[getter]
    fn scope_path(&self) -> Option<String> {
        self.inner.scope_path.clone()
    }

    fn to_human_readable(&self) -> String {
        self.inner.to_human_readable()
    }

    fn __repr__(&self) -> String {
        format!(
            "PreflightResult(operation={}, outcome={}, target={:?})",
            self.inner.descriptor.operation,
            self.inner.outcome_kind.label(),
            self.inner.descriptor.target,
        )
    }

    fn __str__(&self) -> String {
        self.to_human_readable()
    }
}

// ---------------------------------------------------------------------------
// preflight_operation function
// ---------------------------------------------------------------------------

/// Preview what would happen if an operation were dispatched.
///
/// Evaluates the operation descriptor against the enforcement context
/// without actually approving or executing it. Useful for dry-run
/// previews and confirmation prompts.
///
/// Args:
///     operation_id: The operation identifier (e.g. "scan-ports", "fuzz").
///     target: Optional target hostname, IP, or URL.
///     scope: The loaded scope to evaluate against.
///     policy: The execution policy to evaluate against.
///
/// Returns:
///     PreflightResult: The preflight evaluation result.
#[pyfunction]
#[pyo3(signature = (operation_id, scope, policy, target=None))]
pub fn preflight_operation(
    py: Python<'_>,
    operation_id: &str,
    scope: &LoadedScopePy,
    policy: &ExecutionPolicyPy,
    target: Option<&str>,
) -> PyResult<PreflightResultPy> {
    // Build an OperationDescriptor from the provided parameters
    let descriptor = eggsec::config::OperationDescriptor {
        operation: operation_id.to_string(),
        mode: eggsec::config::OperationMode::StandardAssessment,
        risk: eggsec::config::OperationRisk::SafeActive,
        intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
        target: target.map(|t| t.to_string()),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    };

    // Build a default enforcement context for preview
    let enforcement = eggsec::config::EnforcementContext::manual_permissive(
        policy.inner.clone(),
        scope.as_inner().clone(),
    );

    // Run the preflight check using the engine's preflight_operation
    let result = py.allow_threads(|| {
        eggsec::config::preflight_operation(
            eggsec::config::ExecutionSurface::CliManual,
            &enforcement,
            descriptor,
            None, // no manual override for preflight preview
        )
    });

    Ok(PreflightResultPy { inner: result })
}

/// Convenience preflight that uses a provided descriptor.
///
/// Args:
///     descriptor: The operation descriptor to evaluate.
///     scope: The loaded scope to evaluate against.
///     policy: The execution policy to evaluate against.
///     surface: The execution surface for the evaluation.
///     override_reason: Optional override reason for manual surfaces.
///
/// Returns:
///     PreflightResult: The preflight evaluation result.
#[pyfunction]
#[pyo3(signature = (descriptor, scope, policy, surface, override_reason=None))]
pub fn preflight_with_descriptor(
    py: Python<'_>,
    descriptor: &OperationDescriptorPy,
    scope: &LoadedScopePy,
    policy: &ExecutionPolicyPy,
    surface: &crate::execution_context::ExecutionSurfacePy,
    override_reason: Option<&str>,
) -> PyResult<PreflightResultPy> {
    let enforcement = eggsec::config::EnforcementContext::for_surface(
        surface.inner,
        policy.inner.clone(),
        scope.as_inner().clone(),
    );

    let manual_override = override_reason.map(|reason| eggsec::config::ManualOverride {
        assume_yes: true,
        reason: Some(reason.to_string()),
        ..Default::default()
    });

    let result = py.allow_threads(|| {
        eggsec::config::preflight_operation(
            surface.inner,
            &enforcement,
            descriptor.inner.clone(),
            manual_override.as_ref(),
        )
    });

    Ok(PreflightResultPy { inner: result })
}
