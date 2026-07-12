use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::authorization::{ExecutionPolicyPy, ManualOverridePy};
use crate::error::EnforcementError as PyEnforcementError;
use crate::scope_eval::LoadedScopePy;

// ---------------------------------------------------------------------------
// ExecutionSurfacePy
// ---------------------------------------------------------------------------

/// Python wrapper for Eggsec execution surface.
///
/// Describes *where* an operation originates (CLI, TUI, MCP, agent, CI, etc.).
#[pyclass(frozen, name = "ExecutionSurfacePy")]
#[derive(Clone)]
pub struct ExecutionSurfacePy {
    pub(crate) inner: eggsec::config::ExecutionSurface,
}

#[pymethods]
impl ExecutionSurfacePy {
    #[staticmethod]
    fn cli_manual() -> Self {
        Self {
            inner: eggsec::config::ExecutionSurface::CliManual,
        }
    }

    #[staticmethod]
    fn tui_manual() -> Self {
        Self {
            inner: eggsec::config::ExecutionSurface::TuiManual,
        }
    }

    #[staticmethod]
    fn cli_manual_strict() -> Self {
        Self {
            inner: eggsec::config::ExecutionSurface::CliManualStrict,
        }
    }

    #[staticmethod]
    fn tui_manual_strict() -> Self {
        Self {
            inner: eggsec::config::ExecutionSurface::TuiManualStrict,
        }
    }

    #[staticmethod]
    fn mcp_server() -> Self {
        Self {
            inner: eggsec::config::ExecutionSurface::McpServer,
        }
    }

    #[staticmethod]
    fn security_agent() -> Self {
        Self {
            inner: eggsec::config::ExecutionSurface::SecurityAgent,
        }
    }

    #[staticmethod]
    fn ci() -> Self {
        Self {
            inner: eggsec::config::ExecutionSurface::Ci,
        }
    }

    #[staticmethod]
    fn rest_api() -> Self {
        Self {
            inner: eggsec::config::ExecutionSurface::RestApi,
        }
    }

    #[staticmethod]
    fn grpc_api() -> Self {
        Self {
            inner: eggsec::config::ExecutionSurface::GrpcApi,
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    #[getter]
    fn label(&self) -> &'static str {
        self.inner.label()
    }

    #[getter]
    fn is_manual(&self) -> bool {
        self.inner.is_manual()
    }

    #[getter]
    fn is_agent_controlled(&self) -> bool {
        self.inner.is_agent_controlled()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ExecutionSurface({})", self.inner)
    }
}

// ---------------------------------------------------------------------------
// ExecutionProfilePy
// ---------------------------------------------------------------------------

/// Python wrapper for Eggsec execution profile.
///
/// Caller trust boundary for scope enforcement.
#[pyclass(frozen, name = "ExecutionProfilePy")]
#[derive(Clone)]
pub struct ExecutionProfilePy {
    pub(crate) inner: eggsec::config::ExecutionProfile,
}

#[pymethods]
impl ExecutionProfilePy {
    #[staticmethod]
    fn manual_permissive() -> Self {
        Self {
            inner: eggsec::config::ExecutionProfile::ManualPermissive,
        }
    }

    #[staticmethod]
    fn manual_guarded() -> Self {
        Self {
            inner: eggsec::config::ExecutionProfile::ManualGuarded,
        }
    }

    #[staticmethod]
    fn ci_strict() -> Self {
        Self {
            inner: eggsec::config::ExecutionProfile::CiStrict,
        }
    }

    #[staticmethod]
    fn mcp_strict() -> Self {
        Self {
            inner: eggsec::config::ExecutionProfile::McpStrict,
        }
    }

    #[staticmethod]
    fn agent_strict() -> Self {
        Self {
            inner: eggsec::config::ExecutionProfile::AgentStrict,
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    #[getter]
    fn is_strict(&self) -> bool {
        self.inner.is_strict()
    }

    #[getter]
    fn is_automated(&self) -> bool {
        self.inner.is_automated()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ExecutionProfile({})", self.inner)
    }
}

// ---------------------------------------------------------------------------
// PolicyDecisionPy
// ---------------------------------------------------------------------------

/// Python wrapper for Eggsec policy decision.
///
/// Result of evaluating an operation against policy and scope rules.
#[pyclass(frozen, name = "PolicyDecisionPy")]
#[derive(Clone)]
pub struct PolicyDecisionPy {
    pub(crate) inner: eggsec::config::PolicyDecision,
}

#[pymethods]
impl PolicyDecisionPy {
    #[getter]
    fn operation_id(&self) -> &str {
        &self.inner.decision_id
    }

    #[getter]
    fn allowed(&self) -> bool {
        self.inner.allowed
    }

    #[getter]
    fn denied(&self) -> Vec<String> {
        self.inner.denied_reasons.clone()
    }

    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }

    #[getter]
    fn requires_confirmation(&self) -> bool {
        // Manual override used indicates confirmation was required
        self.inner.manual_override_used
    }

    #[getter]
    fn confirmation_classes(&self) -> Vec<String> {
        self.inner.manual_override_classes.clone()
    }

    #[getter]
    fn scope_match(&self) -> Vec<String> {
        self.inner.matched_scope_rules.clone()
    }

    #[getter]
    fn risk_assessment(&self) -> String {
        self.inner.operation_risk.to_string()
    }

    #[getter]
    fn operation(&self) -> String {
        self.inner.operation.clone()
    }

    #[getter]
    fn operation_mode(&self) -> String {
        self.inner.operation_mode.to_string()
    }

    #[getter]
    fn target_original(&self) -> Option<String> {
        self.inner.target_original.clone()
    }

    #[getter]
    fn target_normalized(&self) -> Option<String> {
        self.inner.target_normalized.clone()
    }

    #[getter]
    fn required_features(&self) -> Vec<String> {
        self.inner.required_features.clone()
    }

    #[getter]
    fn missing_features(&self) -> Vec<String> {
        self.inner.missing_features.clone()
    }

    #[getter]
    fn matched_exclusion_rules(&self) -> Vec<String> {
        self.inner.matched_exclusion_rules.clone()
    }

    fn to_human_readable(&self) -> String {
        self.inner.to_human_readable()
    }

    fn __repr__(&self) -> String {
        format!(
            "PolicyDecision(allowed={}, operation={:?}, risk={})",
            self.inner.allowed, self.inner.operation, self.inner.operation_risk
        )
    }

    fn __str__(&self) -> String {
        self.to_human_readable()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.decision_id.hash(&mut hasher);
        self.inner.allowed.hash(&mut hasher);
        hasher.finish()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner.decision_id == other.inner.decision_id
            && self.inner.allowed == other.inner.allowed
    }
}

// ---------------------------------------------------------------------------
// EnforcementOutcomePy
// ---------------------------------------------------------------------------

/// Python wrapper for Eggsec enforcement outcome.
///
/// Wraps a PolicyDecision with profile-aware semantics: Allow, Warn,
/// RequireConfirmation, or Deny.
#[pyclass(frozen, name = "EnforcementOutcomePy")]
#[derive(Clone)]
pub struct EnforcementOutcomePy {
    inner: eggsec::config::EnforcementOutcome,
}

#[pymethods]
impl EnforcementOutcomePy {
    /// The outcome type: "allow", "warn", "require_confirmation", or "deny".
    #[getter]
    fn outcome_type(&self) -> String {
        match &self.inner {
            eggsec::config::EnforcementOutcome::Allow(_) => "allow".to_string(),
            eggsec::config::EnforcementOutcome::Warn(_) => "warn".to_string(),
            eggsec::config::EnforcementOutcome::RequireConfirmation(_) => {
                "require_confirmation".to_string()
            }
            eggsec::config::EnforcementOutcome::Deny(_) => "deny".to_string(),
        }
    }

    /// The underlying policy decision.
    #[getter]
    fn decision(&self) -> PolicyDecisionPy {
        PolicyDecisionPy {
            inner: self.inner.decision().clone(),
        }
    }

    /// Confirmation classes that apply (only meaningful for RequireConfirmation).
    #[getter]
    fn confirmation_classes(&self) -> Vec<String> {
        match &self.inner {
            eggsec::config::EnforcementOutcome::RequireConfirmation(d) => d
                .warnings
                .iter()
                .filter_map(|w| {
                    w.strip_prefix("confirmation required: ")
                        .map(|s| s.to_string())
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Warnings from the decision.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.decision().warnings.clone()
    }

    /// Whether the outcome permits the operation to proceed.
    #[getter]
    fn is_allowed(&self) -> bool {
        self.inner.is_allowed()
    }

    /// Whether the outcome is a hard denial.
    #[getter]
    fn is_denied(&self) -> bool {
        self.inner.is_denied()
    }

    /// Whether the outcome requires manual confirmation.
    #[getter]
    fn requires_confirmation(&self) -> bool {
        self.inner.requires_confirmation()
    }

    fn __repr__(&self) -> String {
        format!("EnforcementOutcome({})", self.outcome_type())
    }

    fn __str__(&self) -> String {
        self.outcome_type()
    }
}

// ---------------------------------------------------------------------------
// OperationDescriptorPy
// ---------------------------------------------------------------------------

/// Python wrapper for Eggsec operation descriptor.
///
/// Bundles the metadata needed for policy evaluation and enforcement.
#[pyclass(frozen, name = "OperationDescriptorPy")]
#[derive(Clone)]
pub struct OperationDescriptorPy {
    pub(crate) inner: eggsec::config::OperationDescriptor,
}

#[pymethods]
impl OperationDescriptorPy {
    /// Create an operation descriptor.
    #[new]
    #[pyo3(signature = (operation, mode, risk, intended_uses, target=None, required_features=None, required_policy_flags=None, requires_private_or_local_target=false, requires_explicit_scope=false, required_capabilities=None))]
    fn new(
        operation: String,
        mode: &str,
        risk: &str,
        intended_uses: Vec<String>,
        target: Option<String>,
        required_features: Option<Vec<String>>,
        required_policy_flags: Option<Vec<String>>,
        requires_private_or_local_target: bool,
        requires_explicit_scope: bool,
        required_capabilities: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let mode = match mode {
            "standard-assessment" => eggsec::config::OperationMode::StandardAssessment,
            "defense-lab" => eggsec::config::OperationMode::DefenseLab,
            "hazardous-lab" => eggsec::config::OperationMode::HazardousLab,
            _ => {
                return Err(PyValueError::new_err(format!(
                    "Invalid mode: {}. Expected 'standard-assessment', 'defense-lab', or 'hazardous-lab'",
                    mode
                )))
            }
        };

        let risk = parse_operation_risk(risk)?;

        let intended_uses: Vec<eggsec::config::IntendedUse> = intended_uses
            .iter()
            .map(|u| parse_intended_use(u))
            .collect::<PyResult<_>>()?;

        let required_capabilities: Vec<eggsec::config::Capability> =
            if let Some(caps) = required_capabilities {
                caps.iter()
                    .map(|c| parse_capability(c))
                    .collect::<PyResult<_>>()?
            } else {
                Vec::new()
            };

        Ok(Self {
            inner: eggsec::config::OperationDescriptor {
                operation,
                mode,
                risk,
                intended_uses,
                target,
                required_features: required_features.unwrap_or_default(),
                required_policy_flags: required_policy_flags.unwrap_or_default(),
                requires_private_or_local_target,
                requires_explicit_scope,
                required_capabilities,
            },
        })
    }

    #[getter]
    fn operation(&self) -> String {
        self.inner.operation.clone()
    }

    #[getter]
    fn target(&self) -> Option<String> {
        self.inner.target.clone()
    }

    #[getter]
    fn risk(&self) -> String {
        self.inner.risk.to_string()
    }

    #[getter]
    fn mode(&self) -> String {
        self.inner.mode.to_string()
    }

    #[getter]
    fn intended_uses(&self) -> Vec<String> {
        self.inner
            .intended_uses
            .iter()
            .map(|u| u.to_string())
            .collect()
    }

    #[getter]
    fn required_features(&self) -> Vec<String> {
        self.inner.required_features.clone()
    }

    #[getter]
    fn required_policy_flags(&self) -> Vec<String> {
        self.inner.required_policy_flags.clone()
    }

    #[getter]
    fn requires_explicit_scope(&self) -> bool {
        self.inner.requires_explicit_scope
    }

    #[getter]
    fn requires_private_or_local_target(&self) -> bool {
        self.inner.requires_private_or_local_target
    }

    #[getter]
    fn required_capabilities(&self) -> Vec<String> {
        self.inner
            .required_capabilities
            .iter()
            .map(|c| c.to_string())
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "OperationDescriptor(operation={}, target={:?}, risk={})",
            self.inner.operation, self.inner.target, self.inner.risk
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} (mode={}, risk={})",
            self.inner.operation, self.inner.mode, self.inner.risk
        )
    }
}

// ---------------------------------------------------------------------------
// ApprovedOperationPy
// ---------------------------------------------------------------------------

/// Python wrapper for Eggsec approved operation token.
///
/// Proof that an operation has passed enforcement evaluation.
/// Can only be produced by EnforcementContext.approve() or
/// EnforcementContext.approve_manual(). No public constructors.
#[pyclass(frozen, name = "ApprovedOperationPy")]
#[derive(Clone)]
pub struct ApprovedOperationPy {
    inner: eggsec::config::ApprovedOperation,
    policy_hash: String,
}

#[pymethods]
impl ApprovedOperationPy {
    #[getter]
    fn operation_id(&self) -> String {
        self.inner.descriptor().operation.clone()
    }

    #[getter]
    fn target(&self) -> Option<String> {
        self.inner.descriptor().target.clone()
    }

    #[getter]
    fn risk(&self) -> String {
        self.inner.descriptor().risk.to_string()
    }

    #[getter]
    fn mode(&self) -> String {
        self.inner.descriptor().mode.to_string()
    }

    #[getter]
    fn surface(&self) -> ExecutionSurfacePy {
        ExecutionSurfacePy {
            inner: self.inner.surface(),
        }
    }

    #[getter]
    fn policy_hash(&self) -> String {
        self.policy_hash.clone()
    }

    #[getter]
    fn audit_event_id(&self) -> Option<String> {
        self.inner.audit_event_id().map(|s| s.to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "ApprovedOperation(operation={}, target={:?})",
            self.inner.descriptor().operation,
            self.inner.descriptor().target
        )
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.audit_event_id().map(|s| s.hash(&mut hasher));
        self.inner.descriptor().operation.hash(&mut hasher);
        hasher.finish()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner.audit_event_id() == other.inner.audit_event_id()
            && self.inner.descriptor().operation == other.inner.descriptor().operation
            && self.inner.descriptor().target == other.inner.descriptor().target
    }
}

// ---------------------------------------------------------------------------
// EnforcementContextPy
// ---------------------------------------------------------------------------

/// Python wrapper for Eggsec enforcement context.
///
/// Core type that bundles execution profile, policy, and scope.
/// Created once per execution path and used to evaluate every operation.
#[pyclass(name = "EnforcementContext")]
pub struct EnforcementContextPy {
    inner: eggsec::config::EnforcementContext,
}

#[pymethods]
impl EnforcementContextPy {
    /// Create a ManualPermissive enforcement context (default CLI/TUI).
    #[staticmethod]
    fn manual_permissive(policy: &ExecutionPolicyPy, scope: &LoadedScopePy) -> PyResult<Self> {
        Ok(Self {
            inner: eggsec::config::EnforcementContext::manual_permissive(
                policy.inner.clone(),
                scope.as_inner().clone(),
            ),
        })
    }

    /// Create a ManualGuarded enforcement context (CLI/TUI with --strict-scope).
    #[staticmethod]
    fn manual_guarded(policy: &ExecutionPolicyPy, scope: &LoadedScopePy) -> PyResult<Self> {
        Ok(Self {
            inner: eggsec::config::EnforcementContext::manual_guarded(
                policy.inner.clone(),
                scope.as_inner().clone(),
            ),
        })
    }

    /// Create a CiStrict enforcement context (non-interactive CI).
    #[staticmethod]
    fn ci_strict(policy: &ExecutionPolicyPy, scope: &LoadedScopePy) -> PyResult<Self> {
        Ok(Self {
            inner: eggsec::config::EnforcementContext::ci_strict(
                policy.inner.clone(),
                scope.as_inner().clone(),
            ),
        })
    }

    /// Create an McpStrict enforcement context (MCP server).
    #[staticmethod]
    fn mcp_strict(policy: &ExecutionPolicyPy, scope: &LoadedScopePy) -> PyResult<Self> {
        Ok(Self {
            inner: eggsec::config::EnforcementContext::mcp_strict(
                policy.inner.clone(),
                scope.as_inner().clone(),
            ),
        })
    }

    /// Create an AgentStrict enforcement context (autonomous agent).
    #[staticmethod]
    fn agent_strict(policy: &ExecutionPolicyPy, scope: &LoadedScopePy) -> PyResult<Self> {
        Ok(Self {
            inner: eggsec::config::EnforcementContext::agent_strict(
                policy.inner.clone(),
                scope.as_inner().clone(),
            ),
        })
    }

    /// Create an enforcement context from an execution surface.
    #[staticmethod]
    fn for_surface(
        surface: &ExecutionSurfacePy,
        policy: &ExecutionPolicyPy,
        scope: &LoadedScopePy,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: eggsec::config::EnforcementContext::for_surface(
                surface.inner,
                policy.inner.clone(),
                scope.as_inner().clone(),
            ),
        })
    }

    /// Evaluate an operation descriptor against this enforcement context.
    fn evaluate(&self, descriptor: &OperationDescriptorPy) -> EnforcementOutcomePy {
        EnforcementOutcomePy {
            inner: self.inner.evaluate(&descriptor.inner),
        }
    }

    /// Approve an operation for dispatch on a strict automated surface.
    ///
    /// Only Allow outcomes produce an ApprovedOperation token. Warn,
    /// RequireConfirmation, and Deny all fail with an error.
    fn approve(
        &self,
        surface: &ExecutionSurfacePy,
        descriptor: &OperationDescriptorPy,
    ) -> PyResult<ApprovedOperationPy> {
        match self.inner.approve(surface.inner, descriptor.inner.clone()) {
            Ok(approved) => Ok(ApprovedOperationPy {
                policy_hash: self.inner.policy_hash(),
                inner: approved,
            }),
            Err(e) => Err(PyEnforcementError::new_err(format!("{}", e))),
        }
    }

    /// Approve an operation for dispatch on a manual surface with optional override.
    ///
    /// For permissive manual surfaces, this supports Warn outcomes and
    /// RequireConfirmation when a matching manual override is present.
    fn approve_manual(
        &self,
        surface: &ExecutionSurfacePy,
        descriptor: &OperationDescriptorPy,
        override_: Option<&ManualOverridePy>,
    ) -> PyResult<ApprovedOperationPy> {
        let mo = override_.map(|o| &o.inner);
        match self
            .inner
            .approve_manual(surface.inner, descriptor.inner.clone(), mo)
        {
            Ok(approved) => Ok(ApprovedOperationPy {
                policy_hash: self.inner.policy_hash(),
                inner: approved,
            }),
            Err(e) => Err(PyEnforcementError::new_err(format!("{}", e))),
        }
    }

    /// SHA-256 hash of the serialized execution policy.
    fn policy_hash(&self) -> String {
        self.inner.policy_hash()
    }

    #[getter]
    fn profile(&self) -> ExecutionProfilePy {
        ExecutionProfilePy {
            inner: self.inner.execution_profile,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "EnforcementContext(profile={}, policy_hash={})",
            self.inner.execution_profile,
            &self.inner.policy_hash()[..12],
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_operation_risk(s: &str) -> PyResult<eggsec::config::OperationRisk> {
    match s {
        "passive" => Ok(eggsec::config::OperationRisk::Passive),
        "safe-active" => Ok(eggsec::config::OperationRisk::SafeActive),
        "intrusive" => Ok(eggsec::config::OperationRisk::Intrusive),
        "load-test" => Ok(eggsec::config::OperationRisk::LoadTest),
        "stress-test" => Ok(eggsec::config::OperationRisk::StressTest),
        "raw-packet" => Ok(eggsec::config::OperationRisk::RawPacket),
        "credential-testing" => Ok(eggsec::config::OperationRisk::CredentialTesting),
        "db-pentest" => Ok(eggsec::config::OperationRisk::DbPentest),
        "traffic-interception" => Ok(eggsec::config::OperationRisk::TrafficInterception),
        "exploit-adjacent" => Ok(eggsec::config::OperationRisk::ExploitAdjacent),
        "evasion-testing" => Ok(eggsec::config::OperationRisk::EvasionTesting),
        "post-exploitation" => Ok(eggsec::config::OperationRisk::PostExploitation),
        "c2-operation" => Ok(eggsec::config::OperationRisk::C2Operation),
        "remote-execution" => Ok(eggsec::config::OperationRisk::RemoteExecution),
        "agent-autonomous" => Ok(eggsec::config::OperationRisk::AgentAutonomous),
        _ => Err(PyValueError::new_err(format!(
            "Invalid risk: {}. See OperationRisk variants.",
            s
        ))),
    }
}

fn parse_intended_use(s: &str) -> PyResult<eggsec::config::IntendedUse> {
    match s {
        "web-assessment" => Ok(eggsec::config::IntendedUse::WebAssessment),
        "api-assessment" => Ok(eggsec::config::IntendedUse::ApiAssessment),
        "waf-regression" => Ok(eggsec::config::IntendedUse::WafRegression),
        "synvoid-regression" => Ok(eggsec::config::IntendedUse::SynvoidRegression),
        "distributed-system-stress" => Ok(eggsec::config::IntendedUse::DistributedSystemStress),
        "protocol-edge-validation" => Ok(eggsec::config::IntendedUse::ProtocolEdgeValidation),
        "ci-regression" => Ok(eggsec::config::IntendedUse::CiRegression),
        "coding-agent-verification" => Ok(eggsec::config::IntendedUse::CodingAgentVerification),
        _ => Err(PyValueError::new_err(format!(
            "Invalid intended use: {}. See IntendedUse variants.",
            s
        ))),
    }
}

fn parse_capability(s: &str) -> PyResult<eggsec::config::Capability> {
    match s {
        "passive-fingerprint" => Ok(eggsec::config::Capability::PassiveFingerprint),
        "active-probe" => Ok(eggsec::config::Capability::ActiveProbe),
        "crawl" => Ok(eggsec::config::Capability::Crawl),
        "http-fuzz-low-impact" => Ok(eggsec::config::Capability::HttpFuzzLowImpact),
        "intrusive-fuzz" => Ok(eggsec::config::Capability::IntrusiveFuzz),
        "waf-detect" => Ok(eggsec::config::Capability::WafDetect),
        "waf-bypass-simulation" => Ok(eggsec::config::Capability::WafBypassSimulation),
        "waf-stress-test" => Ok(eggsec::config::Capability::WafStressTest),
        "load-test" => Ok(eggsec::config::Capability::LoadTest),
        "raw-packet-probe" => Ok(eggsec::config::Capability::RawPacketProbe),
        "credential-testing" => Ok(eggsec::config::Capability::CredentialTesting),
        "remote-execution" => Ok(eggsec::config::Capability::RemoteExecution),
        "nse-safe" => Ok(eggsec::config::Capability::NseSafe),
        "nse-intrusive" => Ok(eggsec::config::Capability::NseIntrusive),
        "traffic-interception" => Ok(eggsec::config::Capability::TrafficInterception),
        "evasion-testing" => Ok(eggsec::config::Capability::EvasionTesting),
        "database-assessment" => Ok(eggsec::config::Capability::DatabaseAssessment),
        "c2-simulation" => Ok(eggsec::config::Capability::C2Simulation),
        "mobile-dynamic-analysis" => Ok(eggsec::config::Capability::MobileDynamicAnalysis),
        _ => Err(PyValueError::new_err(format!(
            "Invalid capability: {}. See Capability variants.",
            s
        ))),
    }
}
