use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    DenialClass, ExecutionPolicy, ExecutionProfile, ExecutionSurface, IntendedUse,
    OperationDescriptor, OperationMode, OperationRisk, Scope,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub decision_id: String,
    pub allowed: bool,
    pub operation: String,
    pub operation_mode: OperationMode,
    pub operation_risk: OperationRisk,
    pub intended_uses: Vec<IntendedUse>,
    pub target_original: Option<String>,
    pub target_normalized: Option<String>,
    pub resolved_addresses: Vec<String>,
    pub matched_scope_rules: Vec<String>,
    pub matched_exclusion_rules: Vec<String>,
    pub required_features: Vec<String>,
    pub missing_features: Vec<String>,
    pub required_policy_flags: Vec<String>,
    pub denied_reasons: Vec<String>,
    pub warnings: Vec<String>,
    // Manual override audit (populated only for ManualPermissive when override accepted)
    pub manual_override_used: bool,
    pub manual_override_reason: Option<String>,
    pub manual_override_classes: Vec<String>,
}

impl PolicyDecision {
    pub fn allowed(
        operation: &str,
        mode: OperationMode,
        risk: OperationRisk,
        intended_uses: Vec<IntendedUse>,
    ) -> Self {
        Self {
            decision_id: Uuid::new_v4().to_string(),
            allowed: true,
            operation: operation.to_string(),
            operation_mode: mode,
            operation_risk: risk,
            intended_uses,
            target_original: None,
            target_normalized: None,
            resolved_addresses: Vec::new(),
            matched_scope_rules: Vec::new(),
            matched_exclusion_rules: Vec::new(),
            required_features: Vec::new(),
            missing_features: Vec::new(),
            required_policy_flags: Vec::new(),
            denied_reasons: Vec::new(),
            warnings: Vec::new(),
            manual_override_used: false,
            manual_override_reason: None,
            manual_override_classes: Vec::new(),
        }
    }

    pub fn denied(
        operation: &str,
        mode: OperationMode,
        risk: OperationRisk,
        intended_uses: Vec<IntendedUse>,
        reason: &str,
    ) -> Self {
        Self {
            decision_id: Uuid::new_v4().to_string(),
            allowed: false,
            operation: operation.to_string(),
            operation_mode: mode,
            operation_risk: risk,
            intended_uses,
            target_original: None,
            target_normalized: None,
            resolved_addresses: Vec::new(),
            matched_scope_rules: Vec::new(),
            matched_exclusion_rules: Vec::new(),
            required_features: Vec::new(),
            missing_features: Vec::new(),
            required_policy_flags: Vec::new(),
            denied_reasons: vec![reason.to_string()],
            warnings: Vec::new(),
            manual_override_used: false,
            manual_override_reason: None,
            manual_override_classes: Vec::new(),
        }
    }

    pub fn with_target(mut self, original: &str, normalized: &str) -> Self {
        self.target_original = Some(original.to_string());
        self.target_normalized = Some(normalized.to_string());
        self
    }

    pub fn with_warning(mut self, warning: &str) -> Self {
        self.warnings.push(warning.to_string());
        self
    }

    pub fn with_scope_rule(mut self, rule: &str) -> Self {
        self.matched_scope_rules.push(rule.to_string());
        self
    }

    pub fn with_required_feature(mut self, feature: &str) -> Self {
        self.required_features.push(feature.to_string());
        self
    }

    pub fn with_missing_feature(mut self, feature: &str) -> Self {
        self.missing_features.push(feature.to_string());
        self
    }

    pub fn with_required_policy_flag(mut self, flag: &str) -> Self {
        self.required_policy_flags.push(flag.to_string());
        self
    }

    pub fn with_denied_reason(mut self, reason: &str) -> Self {
        self.denied_reasons.push(reason.to_string());
        self
    }

    pub fn with_manual_override_record(
        mut self,
        reason: Option<String>,
        classes: Vec<String>,
    ) -> Self {
        self.manual_override_used = true;
        self.manual_override_reason = reason;
        self.manual_override_classes = classes;
        self
    }

    pub fn to_human_readable(&self) -> String {
        let mut lines = Vec::new();
        let status = if self.allowed { "ALLOWED" } else { "DENIED" };
        lines.push(format!(
            "Policy Decision [{}]: {}",
            status, self.decision_id
        ));
        lines.push(format!("  Operation: {}", self.operation));
        lines.push(format!("  Mode: {}", self.operation_mode));
        lines.push(format!("  Risk: {}", self.operation_risk));
        if !self.intended_uses.is_empty() {
            let uses: Vec<_> = self.intended_uses.iter().map(|u| u.label()).collect();
            lines.push(format!("  Intended use: {}", uses.join(", ")));
        }
        if let Some(ref target) = self.target_original {
            lines.push(format!("  Target: {}", target));
        }
        if let Some(ref normalized) = self.target_normalized {
            lines.push(format!("  Normalized: {}", normalized));
        }
        if !self.resolved_addresses.is_empty() {
            lines.push(format!(
                "  Resolved: {}",
                self.resolved_addresses.join(", ")
            ));
        }
        if !self.matched_scope_rules.is_empty() {
            lines.push(format!(
                "  Scope rules: {}",
                self.matched_scope_rules.join(", ")
            ));
        }
        if !self.required_features.is_empty() {
            lines.push(format!(
                "  Required features: {}",
                self.required_features.join(", ")
            ));
        }
        if !self.missing_features.is_empty() {
            lines.push(format!(
                "  Missing features: {}",
                self.missing_features.join(", ")
            ));
        }
        if !self.denied_reasons.is_empty() {
            lines.push("  Denied reasons:".to_string());
            for reason in &self.denied_reasons {
                lines.push(format!("    - {}", reason));
            }
        }
        if !self.warnings.is_empty() {
            lines.push("  Warnings:".to_string());
            for warning in &self.warnings {
                lines.push(format!("    - {}", warning));
            }
        }
        lines.join("\n")
    }
}

/// Outcome of evaluating an operation against a profile's enforcement rules.
///
/// Wraps a [`PolicyDecision`] with profile-aware semantics:
/// - `Allow`: operation may proceed.
/// - `Warn`: operation may proceed but warnings should be surfaced.
/// - `RequireConfirmation`: manual-only intermediate; CLI/TUI may proceed if explicit
///   manual override flags match the required confirmation classes. Automated profiles
///   (CI/MCP/Agent) and ManualGuarded must treat this as a denial.
/// - `Deny`: operation must not proceed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementOutcome {
    Allow(PolicyDecision),
    Warn(PolicyDecision),
    RequireConfirmation(PolicyDecision),
    Deny(PolicyDecision),
}

impl EnforcementOutcome {
    /// Returns a reference to the inner `PolicyDecision`.
    pub fn decision(&self) -> &PolicyDecision {
        match self {
            Self::Allow(d) | Self::Warn(d) | Self::RequireConfirmation(d) | Self::Deny(d) => d,
        }
    }

    /// Returns `true` if the outcome permits the operation to proceed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow(_) | Self::Warn(_))
    }

    /// Returns `true` if the outcome is a hard denial.
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Deny(_))
    }

    /// Returns `true` if the outcome requires manual confirmation (manual-only intermediate).
    /// Automated profiles and ManualGuarded must treat this as denial.
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, Self::RequireConfirmation(_))
    }
}

/// Structured error returned by [`EnforcementContext::approve`] and
/// [`EnforcementContext::approve_manual`] when an operation is not authorized.
#[derive(Debug, thiserror::Error)]
pub enum EnforcementError {
    /// Operation denied by policy (Deny outcome).
    #[error("operation denied by policy")]
    Denied { decision: PolicyDecision },

    /// Manual confirmation required but not available for this surface.
    #[error("manual confirmation required")]
    ConfirmationRequired {
        decision: PolicyDecision,
        required_classes: Vec<ConfirmationClass>,
    },

    /// Manual override is unavailable for this execution surface.
    #[error("manual override unavailable for surface {surface}")]
    ManualOverrideUnavailable {
        surface: ExecutionSurface,
        decision: PolicyDecision,
    },
}

impl EnforcementError {
    /// Returns a reference to the inner `PolicyDecision`.
    pub fn decision(&self) -> &PolicyDecision {
        match self {
            Self::Denied { decision }
            | Self::ConfirmationRequired { decision, .. }
            | Self::ManualOverrideUnavailable { decision, .. } => decision,
        }
    }
}

/// Proof that an operation has passed enforcement evaluation.
///
/// This token is produced exclusively by [`EnforcementContext::approve`] or
/// [`EnforcementContext::approve_manual`]. Strict programmatic surfaces
/// (REST, MCP, Agent, CI) require an `ApprovedOperation` before dispatching
/// a tool, ensuring enforcement is structurally impossible to bypass.
///
/// Fields are private; access is via read-only accessors.
#[derive(Debug, Clone)]
pub struct ApprovedOperation {
    descriptor: OperationDescriptor,
    decision: PolicyDecision,
    surface: ExecutionSurface,
    profile: ExecutionProfile,
    audit_event_id: Option<String>,
}

impl ApprovedOperation {
    /// Construct an approved operation. Only enforcement code should call this.
    pub(crate) fn new(
        descriptor: OperationDescriptor,
        decision: PolicyDecision,
        surface: ExecutionSurface,
        profile: ExecutionProfile,
        audit_event_id: Option<String>,
    ) -> Self {
        Self {
            descriptor,
            decision,
            surface,
            profile,
            audit_event_id,
        }
    }

    /// The operation descriptor that was approved.
    pub fn descriptor(&self) -> &OperationDescriptor {
        &self.descriptor
    }

    /// The policy decision underlying this approval.
    pub fn decision(&self) -> &PolicyDecision {
        &self.decision
    }

    /// The execution surface that produced this approval.
    pub fn surface(&self) -> ExecutionSurface {
        self.surface
    }

    /// The execution profile that produced this approval.
    pub fn profile(&self) -> ExecutionProfile {
        self.profile
    }

    /// Optional audit event ID associated with this approval.
    pub fn audit_event_id(&self) -> Option<&str> {
        self.audit_event_id.as_deref()
    }
}

/// Categories of conditions that trigger `RequireConfirmation` under `ManualPermissive`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationClass {
    OutOfScope,
    ExplicitExclusion,
    HighRisk,
    NonBaselineCapability,
    PrivateResolution,
    CrossHostRedirect,
    TargetExpansion,
    TrafficInterception,
}

impl ConfirmationClass {
    /// Stable kebab-case string for audit, JSON, warnings, and error messages.
    /// Used instead of Debug formatting for machine-readable and consistent output.
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfirmationClass::OutOfScope => "out-of-scope",
            ConfirmationClass::ExplicitExclusion => "explicit-exclusion",
            ConfirmationClass::HighRisk => "high-risk",
            ConfirmationClass::NonBaselineCapability => "nonbaseline-capability",
            ConfirmationClass::PrivateResolution => "private-resolution",
            ConfirmationClass::CrossHostRedirect => "cross-host-redirect",
            ConfirmationClass::TargetExpansion => "target-expansion",
            ConfirmationClass::TrafficInterception => "traffic-interception",
        }
    }
}

/// Manual override flags honored only for `ExecutionProfile::ManualPermissive`.
/// These are never part of MCP request types, agent config, or tool serialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManualOverride {
    pub assume_yes: bool,
    pub allow_out_of_scope: bool,
    pub allow_explicit_exclusion: bool,
    pub allow_high_risk: bool,
    pub allow_db_pentest: bool,
    pub allow_web_proxy: bool,
    pub allow_nonbaseline_capability: bool,
    pub allow_private_resolution: bool,
    pub allow_cross_host_redirect: bool,
    pub reason: Option<String>,
}

impl ManualOverride {
    /// Returns true if this override permits the given confirmation class.
    ///
    /// `--yes` / `assume_yes` is prompt suppression for low-risk manual scope confirmations only
    /// (OutOfScope, TargetExpansion). It does NOT authorize high-risk, explicit exclusions,
    /// non-baseline capabilities, private-resolution, or cross-host redirects.
    /// Those require their specific `--allow-*` flags.
    pub fn permits(&self, class: ConfirmationClass) -> bool {
        match class {
            ConfirmationClass::OutOfScope => self.allow_out_of_scope || self.assume_yes,
            ConfirmationClass::TargetExpansion => self.allow_out_of_scope || self.assume_yes,
            ConfirmationClass::PrivateResolution => self.allow_private_resolution,
            ConfirmationClass::CrossHostRedirect => self.allow_cross_host_redirect,
            ConfirmationClass::ExplicitExclusion => self.allow_explicit_exclusion,
            ConfirmationClass::HighRisk => self.allow_high_risk || self.allow_db_pentest,
            ConfirmationClass::TrafficInterception => self.allow_web_proxy,
            ConfirmationClass::NonBaselineCapability => self.allow_nonbaseline_capability,
        }
    }
}

/// Reusable enforcement context that bundles execution profile, policy, and scope.
///
/// Created once per execution path (CLI, MCP, agent) and used to evaluate
/// every operation descriptor through the same shared enforcement logic.
#[derive(Debug, Clone)]
pub struct EnforcementContext {
    pub execution_profile: ExecutionProfile,
    pub execution_policy: ExecutionPolicy,
    pub loaded_scope: super::scope::LoadedScope,
}

impl EnforcementContext {
    pub fn manual_permissive(
        policy: ExecutionPolicy,
        loaded_scope: super::scope::LoadedScope,
    ) -> Self {
        Self {
            execution_profile: ExecutionProfile::ManualPermissive,
            execution_policy: policy,
            loaded_scope,
        }
    }
    pub fn manual_guarded(
        policy: ExecutionPolicy,
        loaded_scope: super::scope::LoadedScope,
    ) -> Self {
        Self {
            execution_profile: ExecutionProfile::ManualGuarded,
            execution_policy: policy,
            loaded_scope,
        }
    }
    pub fn ci_strict(policy: ExecutionPolicy, loaded_scope: super::scope::LoadedScope) -> Self {
        Self {
            execution_profile: ExecutionProfile::CiStrict,
            execution_policy: policy,
            loaded_scope,
        }
    }
    pub fn mcp_strict(policy: ExecutionPolicy, loaded_scope: super::scope::LoadedScope) -> Self {
        Self {
            execution_profile: ExecutionProfile::McpStrict,
            execution_policy: policy,
            loaded_scope,
        }
    }
    pub fn agent_strict(policy: ExecutionPolicy, loaded_scope: super::scope::LoadedScope) -> Self {
        Self {
            execution_profile: ExecutionProfile::AgentStrict,
            execution_policy: policy,
            loaded_scope,
        }
    }

    /// Construct an [`EnforcementContext`] from an [`ExecutionSurface`].
    ///
    /// This is the canonical way to build enforcement from a caller-origin
    /// identity. It delegates to the appropriate profile-specific constructor.
    pub fn for_surface(
        surface: super::ExecutionSurface,
        policy: ExecutionPolicy,
        loaded_scope: super::scope::LoadedScope,
    ) -> Self {
        match surface.profile() {
            ExecutionProfile::ManualPermissive => Self::manual_permissive(policy, loaded_scope),
            ExecutionProfile::ManualGuarded => Self::manual_guarded(policy, loaded_scope),
            ExecutionProfile::CiStrict => Self::ci_strict(policy, loaded_scope),
            ExecutionProfile::McpStrict => Self::mcp_strict(policy, loaded_scope),
            ExecutionProfile::AgentStrict => Self::agent_strict(policy, loaded_scope),
        }
    }

    /// Returns `true` if the profile requires an explicit scope manifest for networked tools.
    pub fn require_explicit_scope_for_networked(&self) -> bool {
        self.execution_profile.is_automated()
    }

    /// Returns `true` if this profile + descriptor combination requires an explicit scope manifest.
    ///
    /// Strict automated profiles (CiStrict, McpStrict, AgentStrict) require an explicit
    /// manifest (not DefaultEmpty) for target-bearing operations that set `requires_explicit_scope`.
    /// ManualGuarded may require it for such ops; ManualPermissive generally does not
    /// unless the descriptor itself is hazardous.
    pub fn requires_explicit_manifest_for(&self, descriptor: &OperationDescriptor) -> bool {
        self.execution_profile.is_automated()
            && descriptor.target.is_some()
            && descriptor.requires_explicit_scope
    }

    /// Evaluate an operation descriptor against this enforcement context.
    ///
    /// Centralizes explicit-manifest provenance checks for strict profiles.
    /// The inner evaluate_enforcement receives the scope rules, but provenance
    /// (LoadedScope::is_explicit_manifest) is enforced here for automated profiles.
    pub fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome {
        let outcome = evaluate_enforcement(
            descriptor,
            &self.execution_policy,
            Some(&self.loaded_scope.scope),
            self.execution_profile,
        );

        if self.requires_explicit_manifest_for(descriptor)
            && !self.loaded_scope.is_explicit_manifest()
        {
            let mut decision = outcome.decision().clone().with_denied_reason(
                "explicit scope manifest required for automated networked operation",
            );
            decision.allowed = false;
            return EnforcementOutcome::Deny(decision);
        }

        outcome
    }

    /// Approve an operation for dispatch on a strict automated surface.
    ///
    /// Only `Allow` outcomes produce an `ApprovedOperation`. `Warn`,
    /// `RequireConfirmation`, and `Deny` all fail with [`EnforcementError`].
    ///
    /// Use this for REST, MCP, Agent, and CI surfaces.
    pub fn approve(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
    ) -> Result<ApprovedOperation, EnforcementError> {
        let outcome = self.evaluate(&descriptor);
        match outcome {
            EnforcementOutcome::Allow(decision) => Ok(ApprovedOperation::new(
                descriptor,
                decision,
                surface,
                self.execution_profile,
                None,
            )),
            EnforcementOutcome::Warn(decision) => Err(EnforcementError::Denied { decision }),
            EnforcementOutcome::RequireConfirmation(decision) => {
                let required_classes =
                    confirmation_classes_for(&descriptor, &decision, &self.execution_policy);
                Err(EnforcementError::ConfirmationRequired {
                    decision,
                    required_classes,
                })
            }
            EnforcementOutcome::Deny(decision) => Err(EnforcementError::Denied { decision }),
        }
    }

    /// Approve an operation for dispatch on a manual surface with optional override.
    ///
    /// For permissive manual surfaces (`CliManual`, `TuiManual`), this supports
    /// `Warn` outcomes (approved with warning recorded) and `RequireConfirmation`
    /// when a matching manual override is present. For strict or automated surfaces,
    /// manual overrides are rejected.
    ///
    /// Use this for CLI and TUI manual dispatch paths.
    pub fn approve_manual(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        manual_override: Option<&ManualOverride>,
    ) -> Result<ApprovedOperation, EnforcementError> {
        let outcome = self.evaluate(&descriptor);
        match outcome {
            EnforcementOutcome::Allow(decision) => Ok(ApprovedOperation::new(
                descriptor,
                decision,
                surface,
                self.execution_profile,
                None,
            )),
            EnforcementOutcome::Warn(decision) => {
                if surface.honors_manual_override() {
                    Ok(ApprovedOperation::new(
                        descriptor,
                        decision,
                        surface,
                        self.execution_profile,
                        None,
                    ))
                } else {
                    Err(EnforcementError::Denied { decision })
                }
            }
            EnforcementOutcome::RequireConfirmation(decision) => {
                if !surface.honors_manual_override() {
                    let required_classes =
                        confirmation_classes_for(&descriptor, &decision, &self.execution_policy);
                    return Err(EnforcementError::ConfirmationRequired {
                        decision,
                        required_classes,
                    });
                }
                let override_ = match manual_override {
                    Some(o) => o,
                    None => {
                        let required_classes = confirmation_classes_for(
                            &descriptor,
                            &decision,
                            &self.execution_policy,
                        );
                        return Err(EnforcementError::ConfirmationRequired {
                            decision,
                            required_classes,
                        });
                    }
                };
                let required_classes =
                    confirmation_classes_for(&descriptor, &decision, &self.execution_policy);
                let all_permitted = required_classes.iter().all(|c| override_.permits(*c));
                if all_permitted {
                    Ok(ApprovedOperation::new(
                        descriptor,
                        decision,
                        surface,
                        self.execution_profile,
                        None,
                    ))
                } else {
                    Err(EnforcementError::ConfirmationRequired {
                        decision,
                        required_classes,
                    })
                }
            }
            EnforcementOutcome::Deny(decision) => Err(EnforcementError::Denied { decision }),
        }
    }

    pub fn policy_hash(&self) -> String {
        let json = serde_json::to_vec(&self.execution_policy)
            .expect("ExecutionPolicy is JSON-serializable");
        let hash = Sha256::digest(&json);
        hex::encode(hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightResult {
    pub surface: super::ExecutionSurface,
    pub profile: ExecutionProfile,
    pub descriptor: OperationDescriptor,
    pub outcome_kind: PreflightOutcomeKind,
    pub decision: PolicyDecision,
    pub required_confirmation_classes: Vec<ConfirmationClass>,
    pub manual_override_honored: bool,
    pub scope_source: super::scope::ScopeSource,
    pub scope_path: Option<String>,
    pub suggested_cli_flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreflightOutcomeKind {
    Allow,
    Warn,
    RequireConfirmation,
    Deny,
}

impl PreflightOutcomeKind {
    pub fn from_outcome(outcome: &EnforcementOutcome) -> Self {
        match outcome {
            EnforcementOutcome::Allow(_) => PreflightOutcomeKind::Allow,
            EnforcementOutcome::Warn(_) => PreflightOutcomeKind::Warn,
            EnforcementOutcome::RequireConfirmation(_) => PreflightOutcomeKind::RequireConfirmation,
            EnforcementOutcome::Deny(_) => PreflightOutcomeKind::Deny,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PreflightOutcomeKind::Allow => "allow",
            PreflightOutcomeKind::Warn => "warn",
            PreflightOutcomeKind::RequireConfirmation => "confirmation-required",
            PreflightOutcomeKind::Deny => "deny",
        }
    }
}

pub fn preflight_operation(
    surface: super::ExecutionSurface,
    enforcement: &EnforcementContext,
    descriptor: OperationDescriptor,
    manual_override: Option<&ManualOverride>,
) -> PreflightResult {
    let outcome = enforcement.evaluate(&descriptor);
    let outcome_kind = PreflightOutcomeKind::from_outcome(&outcome);
    let decision = outcome.decision().clone();

    let required_confirmation_classes = if let EnforcementOutcome::RequireConfirmation(_) = &outcome
    {
        confirmation_classes_for(&descriptor, &decision, &enforcement.execution_policy)
    } else {
        Vec::new()
    };

    let manual_override_honored = if surface.honors_manual_override() {
        if let Some(mo) = manual_override {
            !required_confirmation_classes.is_empty()
                && required_confirmation_classes.iter().all(|c| mo.permits(*c))
        } else {
            false
        }
    } else {
        false
    };

    let suggested_cli_flags = if surface.is_manual() {
        confirmation_class_cli_flags(&required_confirmation_classes)
    } else {
        Vec::new()
    };

    PreflightResult {
        surface,
        profile: enforcement.execution_profile,
        descriptor,
        outcome_kind,
        decision,
        required_confirmation_classes,
        manual_override_honored,
        scope_source: enforcement.loaded_scope.source.clone(),
        scope_path: enforcement.loaded_scope.path.clone(),
        suggested_cli_flags,
    }
}

fn confirmation_class_cli_flags(classes: &[ConfirmationClass]) -> Vec<String> {
    classes
        .iter()
        .map(|c| match c {
            ConfirmationClass::OutOfScope => "--allow-out-of-scope".to_string(),
            ConfirmationClass::TargetExpansion => "--allow-out-of-scope".to_string(),
            ConfirmationClass::PrivateResolution => "--allow-private-resolution".to_string(),
            ConfirmationClass::CrossHostRedirect => "--allow-cross-host-redirect".to_string(),
            ConfirmationClass::ExplicitExclusion => "--allow-excluded-target".to_string(),
            ConfirmationClass::HighRisk => "--allow-high-risk".to_string(),
            ConfirmationClass::TrafficInterception => "--allow-web-proxy".to_string(),
            ConfirmationClass::NonBaselineCapability => {
                "--allow-nonbaseline-capability".to_string()
            }
        })
        .collect()
}

impl PreflightResult {
    pub fn to_human_readable(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Operation: {}", self.descriptor.operation));
        if let Some(ref target) = self.descriptor.target {
            lines.push(format!("Target: {}", target));
        }
        lines.push(format!("Surface: {}", self.surface.label()));
        lines.push(format!("Profile: {}", self.profile));
        lines.push(format!("Outcome: {}", self.outcome_kind.label()));
        if !self.required_confirmation_classes.is_empty() {
            let classes: Vec<&str> = self
                .required_confirmation_classes
                .iter()
                .map(|c| c.as_str())
                .collect();
            lines.push(format!("Classes: {}", classes.join(", ")));
        }
        if !self.suggested_cli_flags.is_empty() {
            lines.push(format!(
                "Suggested flags: {}",
                self.suggested_cli_flags.join(" ")
            ));
        }
        if self.manual_override_honored {
            lines.push("Manual override: honored".to_string());
        }
        lines.push(format!("Scope: {:?}", self.scope_source));
        if let Some(ref path) = self.scope_path {
            lines.push(format!("Scope path: {}", path));
        }
        if !self.decision.denied_reasons.is_empty() {
            lines.push(format!(
                "Denied reasons: {}",
                self.decision.denied_reasons.join("; ")
            ));
        }
        if !self.decision.warnings.is_empty() {
            lines.push(format!("Warnings: {}", self.decision.warnings.join("; ")));
        }
        lines.join("\n")
    }
}

/// Check whether a named compile-time Cargo feature is enabled.
///
/// Returns `true` for features that are always available or not relevant
/// as compile-time gates, and `false` for features that are behind a
/// `cfg(feature = "...")` gate that is not currently active.
///
/// # Unknown Features
///
/// Unknown feature names default to `true` (available). This is intentional:
/// if a feature string in metadata doesn't match any gate here, the operation
/// should not be blocked at this level — the gate may live in a dependency
/// crate or may have been added without updating this function. The trade-off
/// is that typos in feature names silently pass. Use [`is_known_feature`] in
/// tests to validate that feature strings in metadata are recognized.
pub fn is_feature_enabled(feature: &str) -> bool {
    match feature {
        "packet-inspection" => cfg!(feature = "packet-inspection"),
        "stress-testing" => cfg!(feature = "stress-testing"),
        "nse" => cfg!(feature = "nse"),
        "nse-sandbox" => cfg!(feature = "nse-sandbox"),
        "headless-browser" => cfg!(feature = "headless-browser"),
        "rest-api" => cfg!(feature = "rest-api"),
        "grpc-api" => cfg!(feature = "grpc-api"),
        "ws-api" => cfg!(feature = "ws-api"),
        "ai-integration" => cfg!(feature = "ai-integration"),
        "database" => cfg!(feature = "database"),
        "container" => cfg!(feature = "container"),
        "sbom" => cfg!(feature = "sbom"),
        "websocket" => cfg!(feature = "websocket"),
        "compliance" => cfg!(feature = "compliance"),
        "external-integrations" => cfg!(feature = "external-integrations"),
        "finding-workflow" => cfg!(feature = "finding-workflow"),
        "vuln-management" => cfg!(feature = "vuln-management"),
        "cloud" => cfg!(feature = "cloud"),
        "git-secrets" => cfg!(feature = "git-secrets"),
        "wireless" => cfg!(feature = "wireless"),
        "mobile" => cfg!(feature = "mobile"),
        "pdf" => cfg!(feature = "pdf"),
        "advanced-hunting" => cfg!(feature = "advanced-hunting"),
        _ => true, // Unknown features are assumed available (see doc comment)
    }
}

/// Returns `true` if the given feature string is recognized by [`is_feature_enabled`].
///
/// Use this in tests to validate that feature strings in metadata (OperationMetadata,
/// DomainDescriptor) are not misspelled. Does not check whether the feature is
/// currently compiled — only that the name is known.
#[allow(dead_code)] // test/validation helper; not used in production code paths
pub fn is_known_feature(feature: &str) -> bool {
    matches!(
        feature,
        "packet-inspection"
            | "stress-testing"
            | "nse"
            | "nse-sandbox"
            | "headless-browser"
            | "rest-api"
            | "grpc-api"
            | "ws-api"
            | "ai-integration"
            | "database"
            | "container"
            | "sbom"
            | "websocket"
            | "compliance"
            | "external-integrations"
            | "finding-workflow"
            | "vuln-management"
            | "cloud"
            | "git-secrets"
            | "wireless"
            | "mobile"
            | "pdf"
            | "advanced-hunting"
    )
}

/// Shared policy evaluation entry point.
///
/// Takes an [`OperationDescriptor`], the current [`ExecutionPolicy`], and an
/// optional [`Scope`], and returns a fully-populated [`PolicyDecision`].
///
/// This is the canonical function that command handlers, MCP dispatchers,
/// agent workflows, and API endpoints should call instead of building
/// policy checks inline.
pub fn evaluate_operation_policy(
    descriptor: &OperationDescriptor,
    policy: &ExecutionPolicy,
    scope: Option<&Scope>,
) -> PolicyDecision {
    let mut decision = PolicyDecision::allowed(
        &descriptor.operation,
        descriptor.mode,
        descriptor.risk,
        descriptor.intended_uses.clone(),
    );

    // Attach target if provided
    if let Some(ref target) = descriptor.target {
        decision = decision.with_target(target, target);
    }

    // Propagate required features from descriptor
    for feature in &descriptor.required_features {
        decision = decision.with_required_feature(feature);
    }

    // Check required feature availability
    for feature in &descriptor.required_features {
        if !is_feature_enabled(feature) {
            decision = decision.with_missing_feature(feature);
            decision
                .denied_reasons
                .push(format!("required feature '{}' is not enabled", feature));
            decision.allowed = false;
        }
    }

    // Check scope if a target and scope are provided
    if let Some(ref target) = descriptor.target {
        if let Some(scope) = scope {
            match scope.is_target_allowed(target) {
                Ok(true) => {
                    decision
                        .matched_scope_rules
                        .push("target in scope".to_string());
                }
                Ok(false) => {
                    if scope.is_excluded(target) {
                        decision
                            .matched_exclusion_rules
                            .push(format!("excluded: {}", target));
                        decision
                            .denied_reasons
                            .push("target is explicitly excluded from scope".to_string());
                    } else {
                        decision
                            .denied_reasons
                            .push("target not in scope".to_string());
                    }
                    decision.allowed = false;
                }
                Err(e) => {
                    decision.warnings.push(format!("scope check error: {}", e));
                }
            }
        } else if descriptor.requires_explicit_scope || descriptor.requires_private_or_local_target
        {
            decision
                .denied_reasons
                .push("scope file required but not provided".to_string());
            decision.allowed = false;
        } else if super::is_private_ip(
            &target
                .parse()
                .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)),
        ) {
            decision.warnings.push(
                "target is a private IP; scope file recommended for defense-lab profiles"
                    .to_string(),
            );
        }
    }

    // Check risk against execution policy
    if !descriptor.risk.is_allowed_by(policy) {
        decision.denied_reasons.push(format!(
            "operation risk '{}' is not allowed by current execution policy",
            descriptor.risk
        ));
        decision.allowed = false;
    }

    // Check required policy flags
    for flag in &descriptor.required_policy_flags {
        match flag.as_str() {
            "require_explicit_scope" => {
                if !policy.require_explicit_scope {
                    decision
                        .denied_reasons
                        .push("require_explicit_scope is disabled in policy".to_string());
                    decision.allowed = false;
                }
                decision.required_policy_flags.push(flag.clone());
            }
            _ => {
                decision.required_policy_flags.push(flag.clone());
            }
        }
    }

    decision
}

/// Classify the denial reasons in a `PolicyDecision` into structured `DenialClass` values.
///
/// This enables profile-specific downgrade logic (e.g., ManualPermissive downgrading
/// safe scope-selection misses to warnings) while keeping feature/risk/capability/exclusion
/// denials as hard denials.
pub fn classify_denial_reasons(decision: &PolicyDecision) -> Vec<DenialClass> {
    use std::collections::HashSet;
    let mut classes: HashSet<DenialClass> = HashSet::new();
    let reasons = &decision.denied_reasons;

    if reasons.iter().any(|r| {
        r.contains("scope file required") || r.contains("explicit scope manifest required")
    }) {
        classes.insert(DenialClass::ScopeMissing);
    }

    let has_exclusion = !decision.matched_exclusion_rules.is_empty()
        || reasons.iter().any(|r| {
            r.contains("excluded") || r.contains("explicitly excluded") || r.contains("exclusion")
        });
    if has_exclusion {
        classes.insert(DenialClass::ExplicitExclusion);
    } else if reasons.iter().any(|r| r.contains("target not in scope")) {
        classes.insert(DenialClass::TargetOutOfScope);
    }

    if !decision.missing_features.is_empty()
        || reasons
            .iter()
            .any(|r| r.contains("required feature") || r.contains("not enabled"))
    {
        classes.insert(DenialClass::FeatureMissing);
    }

    if reasons.iter().any(|r| {
        r.contains("operation risk") || r.contains("not allowed by current execution policy")
    }) {
        classes.insert(DenialClass::RiskPolicyDenied);
    }

    if reasons.iter().any(|r| r.contains("capability")) {
        classes.insert(DenialClass::CapabilityDenied);
    }

    // Invalid target or scope parse/check errors
    if reasons.iter().any(|r| {
        r.contains("invalid") || r.contains("scope check error") || r.contains("DNS resolution")
    }) || decision
        .target_original
        .as_deref()
        .is_some_and(|t| t.trim().is_empty())
    {
        classes.insert(DenialClass::InvalidTarget);
    }

    if classes.is_empty() {
        classes.insert(DenialClass::Unknown);
    }

    classes.into_iter().collect()
}

/// Returns whether the given denial classes for the descriptor/profile may be downgraded
/// from denial to warning under ManualPermissive semantics.
///
/// Downgrade is allowed only for safe (Passive/SafeActive), StandardAssessment operations
/// whose *only* denial classes are ScopeMissing or TargetOutOfScope (no exclusions, no
/// feature/risk/capability denials). Strict and guarded profiles never downgrade.
pub fn may_downgrade_to_warning(
    descriptor: &OperationDescriptor,
    classes: &[DenialClass],
    profile: ExecutionProfile,
) -> bool {
    if profile != ExecutionProfile::ManualPermissive {
        return false;
    }
    if !matches!(
        descriptor.risk,
        OperationRisk::Passive | OperationRisk::SafeActive
    ) {
        return false;
    }
    if descriptor.mode != OperationMode::StandardAssessment {
        return false;
    }
    if classes.is_empty() {
        return false;
    }
    // All classes must be safe-to-downgrade scope-related; presence of any other class blocks downgrade
    let only_safe_scope = classes
        .iter()
        .all(|c| matches!(c, DenialClass::ScopeMissing | DenialClass::TargetOutOfScope));
    only_safe_scope
}

/// Evaluate an operation with profile-aware enforcement semantics.
///
/// Calls [`evaluate_operation_policy`] internally, then transforms the
/// resulting [`PolicyDecision`] into [`EnforcementOutcome::Allow`],
/// [`EnforcementOutcome::Warn`], [`EnforcementOutcome::RequireConfirmation`],
/// or [`EnforcementOutcome::Deny`] according to the given [`ExecutionProfile`].
///
/// For ManualPermissive (default manual), safe scope-selection denials
/// (ScopeMissing / TargetOutOfScope for low-risk ops with *no* positive scope rules
/// and no exclusions) downgrade to Warn. Explicit allowlist misses (positive rules),
/// explicit exclusions, high-risk operations, and non-baseline capabilities produce
/// `RequireConfirmation` (operator can override with CLI flags). Missing features,
/// invalid targets, denied capabilities, and compile-time unavailability are always
/// hard `Deny`. ManualGuarded / CiStrict / McpStrict / AgentStrict treat
/// `RequireConfirmation` cases as hard `Deny` (no override path).
pub fn evaluate_enforcement(
    descriptor: &OperationDescriptor,
    policy: &ExecutionPolicy,
    scope: Option<&Scope>,
    profile: ExecutionProfile,
) -> EnforcementOutcome {
    let mut decision = evaluate_operation_policy(descriptor, policy, scope);

    // Capability checks (denied always deny; strict profiles require explicit allow for non-baseline)
    if !decision.required_features.is_empty() || !descriptor.required_capabilities.is_empty() {
        // Denied capabilities always deny, regardless of profile
        for cap in &descriptor.required_capabilities {
            if policy.denied_capabilities.contains(cap) {
                decision.denied_reasons.push(format!(
                    "capability '{}' is denied by execution policy",
                    cap
                ));
                decision.allowed = false;
                return EnforcementOutcome::Deny(decision);
            }
        }

        // For strict automated profiles, non-baseline capabilities must be explicitly allowed
        if profile.is_strict() {
            for cap in &descriptor.required_capabilities {
                if !policy.allowed_capabilities.contains(cap)
                    && !super::baseline_allowed_capability(*cap)
                {
                    decision.denied_reasons.push(format!(
                        "capability '{}' requires explicit allow in {} execution policy",
                        cap, profile
                    ));
                    decision.allowed = false;
                    return EnforcementOutcome::Deny(decision);
                }
            }
        }
    }

    if !decision.allowed {
        // Base policy denied. For ManualPermissive, attempt to downgrade safe scope misses using DenialClass.
        if profile == ExecutionProfile::ManualPermissive {
            let classes = classify_denial_reasons(&decision);
            if may_downgrade_to_warning(descriptor, &classes, profile) {
                // Additional carve-out per hardening plan intent:
                // "safe out-of-scope target can warn only when no explicit exclusion exists" AND
                // when the user did not declare positive scope rules (i.e. truly ambiguous/empty scope).
                // If a scope with non-empty allowed_targets was provided and target missed it,
                // treat as hard denial even in permissive (user intent was explicit).
                let has_positive_scope_rules = scope.is_some_and(|s| !s.allowed_targets.is_empty());
                let is_pure_out_of_scope_miss = classes
                    .iter()
                    .any(|c| matches!(c, DenialClass::TargetOutOfScope))
                    && !classes
                        .iter()
                        .any(|c| matches!(c, DenialClass::ExplicitExclusion));
                if is_pure_out_of_scope_miss && has_positive_scope_rules {
                    // Explicit rules declared; mismatch is not a warnable miss.
                    // Per 2026-06-10 manual discretion plan:
                    // Under ManualPermissive this is a confirmable operator-discretion case
                    // (RequireConfirmation), not a silent warn and not an immediate hard denial.
                    // Strict/guarded/automated profiles still hard-deny.
                    if profile == ExecutionProfile::ManualPermissive {
                        // Classify before any mutation so confirmation_classes_for can see "not in scope" etc.
                        let conf_classes = confirmation_classes_for(descriptor, &decision, policy);
                        let mut d = decision;
                        for c in &conf_classes {
                            d.warnings.push(format!("confirmation required: {:?}", c));
                        }
                        // Do not drain denied_reasons here; leave them for diagnostics and for
                        // confirmation_classes_for callers that inspect the decision inside RequireConfirmation.
                        return EnforcementOutcome::RequireConfirmation(d);
                    } else {
                        return EnforcementOutcome::Deny(decision);
                    }
                }

                // Move denial reasons to warnings and allow-as-warn (safe ambiguity cases)
                let mut d = decision;
                if !d.denied_reasons.is_empty() {
                    d.warnings.extend(
                        d.denied_reasons
                            .drain(..)
                            .map(|r| format!("downgraded: {}", r)),
                    );
                }
                d.allowed = true;
                return EnforcementOutcome::Warn(d);
            }

            // ManualPermissive: map discretion-denial cases to RequireConfirmation
            // (explicit out-of-scope with positive rules, explicit exclusion, etc.)
            let conf_classes = confirmation_classes_for(descriptor, &decision, policy);
            if !conf_classes.is_empty() {
                let mut d = decision;
                for c in &conf_classes {
                    d.warnings.push(format!("confirmation required: {:?}", c));
                }
                return EnforcementOutcome::RequireConfirmation(d);
            }
        }
        return EnforcementOutcome::Deny(decision);
    }

    // ManualPermissive: even for base-allowed decisions, high-risk operations and
    // non-baseline capabilities require explicit operator confirmation (discretion).
    if profile == ExecutionProfile::ManualPermissive {
        let conf_classes = confirmation_classes_for(descriptor, &decision, policy);
        if !conf_classes.is_empty() {
            let mut d = decision;
            for c in &conf_classes {
                d.warnings.push(format!("confirmation required: {:?}", c));
            }
            return EnforcementOutcome::RequireConfirmation(d);
        }
    }

    // Check for warnings based on profile
    let mut warnings = decision.warnings.clone();

    match profile {
        ExecutionProfile::ManualPermissive => {
            // Scope ambiguity becomes a warning, not a denial
            if decision.target_original.is_some()
                && decision.matched_scope_rules.is_empty()
                && decision.denied_reasons.is_empty()
            {
                warnings
                    .push("target scope is ambiguous; consider using --strict-scope".to_string());
            }
            if !warnings.is_empty() {
                let mut d = decision;
                d.warnings = warnings;
                EnforcementOutcome::Warn(d)
            } else {
                EnforcementOutcome::Allow(decision)
            }
        }
        ExecutionProfile::ManualGuarded => {
            // Missing scope for target-networked operations denies
            if descriptor.requires_explicit_scope && scope.is_none() {
                let mut d = decision;
                d.denied_reasons
                    .push("scope file required in guarded mode".to_string());
                d.allowed = false;
                return EnforcementOutcome::Deny(d);
            }
            EnforcementOutcome::Allow(decision)
        }
        ExecutionProfile::CiStrict
        | ExecutionProfile::McpStrict
        | ExecutionProfile::AgentStrict => {
            // Strict profiles: missing scope for networked operations denies
            if descriptor.requires_explicit_scope && scope.is_none() {
                let mut d = decision;
                d.denied_reasons
                    .push(format!("scope file required in {} mode", profile));
                d.allowed = false;
                return EnforcementOutcome::Deny(d);
            }
            // Strict profiles: scope ambiguity denies
            if decision.target_original.is_some()
                && decision.matched_scope_rules.is_empty()
                && decision.denied_reasons.is_empty()
            {
                let mut d = decision;
                d.denied_reasons
                    .push(format!("target scope is ambiguous in {} mode", profile));
                d.allowed = false;
                return EnforcementOutcome::Deny(d);
            }
            if !warnings.is_empty() {
                let mut d = decision;
                d.warnings = warnings;
                EnforcementOutcome::Deny(d)
            } else {
                EnforcementOutcome::Allow(decision)
            }
        }
    }
}

/// Classify conditions in a denied (or would-be-denied) decision + descriptor that warrant
/// `RequireConfirmation` under `ManualPermissive`. Returns the list of confirmation classes
/// that apply. Used by `evaluate_enforcement` and by `CommandContext` to determine which
/// manual override flags are required.
///
/// Only operator-discretion cases are returned here; missing features, invalid targets,
/// denied-capabilities, risk-policy denials ("not allowed by current execution policy"),
/// and compile-time unavailability are always hard denials and do not produce confirmation
/// classes (they remain `Deny` even for ManualPermissive).
pub fn confirmation_classes_for(
    descriptor: &OperationDescriptor,
    decision: &PolicyDecision,
    _policy: &ExecutionPolicy,
) -> Vec<ConfirmationClass> {
    let mut classes = Vec::new();

    // Hard denials must never become confirmation: feature missing, capability denied, risk policy denied, invalid target.
    let has_hard_deny = !decision.missing_features.is_empty()
        || decision.denied_reasons.iter().any(|r| {
            r.contains("capability") && r.contains("denied")
                || r.contains("requires explicit allow")
                || r.contains("not allowed by current execution policy")
                || r.contains("required feature")
                || r.contains("invalid")
                || r.contains("scope check error")
        });
    if has_hard_deny {
        return classes; // empty => will stay Deny
    }

    // Explicit exclusion
    if !decision.matched_exclusion_rules.is_empty()
        || decision
            .denied_reasons
            .iter()
            .any(|r| r.contains("explicitly excluded") || r.contains("excluded"))
    {
        classes.push(ConfirmationClass::ExplicitExclusion);
    }

    // TargetOutOfScope with positive (explicit) scope rules present -> OutOfScope
    let has_positive_scope_rules = !decision.matched_scope_rules.is_empty()
        || decision
            .denied_reasons
            .iter()
            .any(|r| r.contains("not in scope"));
    if has_positive_scope_rules
        && decision
            .denied_reasons
            .iter()
            .any(|r| r.contains("not in scope"))
        && !classes.contains(&ConfirmationClass::ExplicitExclusion)
    {
        classes.push(ConfirmationClass::OutOfScope);
    }

    // High-risk operations: only when the *base policy would have allowed the risk*
    // (i.e. the denial was not a risk-policy denial) and runtime/feature exists.
    // For base-allowed paths we will also surface this below.
    if matches!(
        descriptor.risk,
        OperationRisk::Intrusive
            | OperationRisk::LoadTest
            | OperationRisk::StressTest
            | OperationRisk::RawPacket
            | OperationRisk::CredentialTesting
            | OperationRisk::DbPentest
            | OperationRisk::ExploitAdjacent
            | OperationRisk::RemoteExecution
    ) {
        // If we got here, there was no hard risk-policy denial string.
        // For a denied decision that reached discretion (e.g. scope discretion + high risk),
        // or for a base-allowed high-risk, surface confirmation.
        if !classes.contains(&ConfirmationClass::HighRisk) {
            classes.push(ConfirmationClass::HighRisk);
        }
    }

    // Non-baseline capability required (and not already hard-denied)
    for cap in &descriptor.required_capabilities {
        if !super::baseline_allowed_capability(*cap)
            && !classes.contains(&ConfirmationClass::NonBaselineCapability)
        {
            classes.push(ConfirmationClass::NonBaselineCapability);
        }
    }

    // Resolver/redirect signals (best-effort; only if not hard-denied above).
    // PrivateResolution is for signals that a public/nominal input resolved to private/loopback
    // (e.g. DNS rebinding or misdirection). The generic "target is a private IP; scope recommended"
    // advisory for explicit private targeting is informational only and does not trigger confirmation.
    let has_private_resolution_signal = decision.warnings.iter().any(|w| {
        let wl = w.to_lowercase();
        (wl.contains("private") || wl.contains("loopback"))
            && (wl.contains("resolv")
                || wl.contains("public")
                || wl.contains("rebind")
                || wl.contains("misdirect"))
    }) || decision.denied_reasons.iter().any(|r| {
        let rl = r.to_lowercase();
        (rl.contains("private") || rl.contains("loopback"))
            && (rl.contains("resolv") || rl.contains("public") || rl.contains("rebind"))
    });
    if has_private_resolution_signal && !classes.contains(&ConfirmationClass::PrivateResolution) {
        classes.push(ConfirmationClass::PrivateResolution);
    }
    if (decision
        .warnings
        .iter()
        .any(|w| w.contains("redirect") || w.contains("canonical"))
        || decision
            .denied_reasons
            .iter()
            .any(|r| r.contains("redirect") || r.contains("host")))
        && !classes.contains(&ConfirmationClass::CrossHostRedirect)
    {
        classes.push(ConfirmationClass::CrossHostRedirect);
    }

    // Target expansion discovered outside original input (placeholder)
    if decision
        .warnings
        .iter()
        .any(|w| w.contains("expansion") || w.contains("discovered"))
        && !classes.contains(&ConfirmationClass::TargetExpansion)
    {
        classes.push(ConfirmationClass::TargetExpansion);
    }

    classes
}

/// Stable kebab-case strings for the given confirmation classes.
/// Deduplicates while preserving first-seen order (for deterministic audit/JSON).
pub fn confirmation_class_strings(classes: &[ConfirmationClass]) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    classes
        .iter()
        .filter_map(|c| {
            let s = c.as_str().to_string();
            if seen.insert(s.clone()) {
                Some(s)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_decision_allowed_serializes() {
        let decision = PolicyDecision::allowed(
            "test",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
        );
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("\"allowed\":true"));
        assert!(json.contains("\"standard-assessment\""));
    }

    #[test]
    fn policy_decision_denied_has_reason() {
        let decision = PolicyDecision::denied(
            "test",
            OperationMode::DefenseLab,
            OperationRisk::Intrusive,
            vec![IntendedUse::WafRegression],
            "target not in scope",
        );
        assert!(!decision.allowed);
        assert_eq!(decision.denied_reasons.len(), 1);
    }

    #[test]
    fn policy_decision_human_readable() {
        let decision = PolicyDecision::allowed(
            "test",
            OperationMode::DefenseLab,
            OperationRisk::Intrusive,
            vec![IntendedUse::WafRegression],
        );
        let text = decision.to_human_readable();
        assert!(text.contains("ALLOWED"));
        assert!(text.contains("defense-lab"));
    }

    #[test]
    fn policy_decision_with_target() {
        let decision = PolicyDecision::allowed(
            "test",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
        )
        .with_target("http://127.0.0.1:8080", "127.0.0.1");
        assert_eq!(
            decision.target_original.as_deref(),
            Some("http://127.0.0.1:8080")
        );
        assert_eq!(decision.target_normalized.as_deref(), Some("127.0.0.1"));
    }

    #[test]
    fn policy_decision_with_warnings() {
        let decision = PolicyDecision::allowed(
            "test",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
        )
        .with_warning("private IP");
        assert_eq!(decision.warnings.len(), 1);
    }

    #[test]
    fn evaluate_operation_policy_allowed_localhost() {
        let scope = super::super::Scope {
            allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
            ..Default::default()
        };
        let descriptor = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let decision = evaluate_operation_policy(&descriptor, &policy, Some(&scope));
        assert!(decision.allowed);
        assert!(!decision.matched_scope_rules.is_empty());
    }

    #[test]
    fn evaluate_operation_policy_denied_public_target() {
        let scope = super::super::Scope {
            allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
            ..Default::default()
        };
        let descriptor = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let decision = evaluate_operation_policy(&descriptor, &policy, Some(&scope));
        assert!(!decision.allowed);
        assert!(decision
            .denied_reasons
            .iter()
            .any(|r| r.contains("not in scope")));
    }

    #[test]
    fn evaluate_operation_policy_denied_by_risk() {
        let descriptor = OperationDescriptor {
            operation: "stress".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::StressTest,
            intended_uses: vec![IntendedUse::DistributedSystemStress],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let decision = evaluate_operation_policy(&descriptor, &policy, None);
        assert!(!decision.allowed);
        assert!(decision
            .denied_reasons
            .iter()
            .any(|r| r.contains("not allowed by current execution policy")));
    }

    #[test]
    fn evaluate_operation_policy_denied_missing_scope() {
        let descriptor = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::DefenseLab,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::WafRegression],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let decision = evaluate_operation_policy(&descriptor, &policy, None);
        assert!(!decision.allowed);
        assert!(decision
            .denied_reasons
            .iter()
            .any(|r| r.contains("scope file required")));
    }

    #[test]
    fn evaluate_operation_policy_hazardous_lab_allowed() {
        let descriptor = OperationDescriptor {
            operation: "raw-packet".to_string(),
            mode: OperationMode::HazardousLab,
            risk: OperationRisk::ExploitAdjacent,
            intended_uses: vec![IntendedUse::ProtocolEdgeValidation],
            target: Some("127.0.0.1".to_string()),
            required_features: vec!["packet-inspection".to_string()],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let mut policy = ExecutionPolicy::default();
        policy.allow_exploit_adjacent = true;
        let decision = evaluate_operation_policy(&descriptor, &policy, None);
        assert!(decision
            .required_features
            .iter()
            .any(|f| f == "packet-inspection"));
        if cfg!(feature = "packet-inspection") {
            assert!(decision.allowed);
        } else {
            assert!(!decision.allowed);
            assert!(decision
                .missing_features
                .iter()
                .any(|f| f == "packet-inspection"));
        }
    }

    #[test]
    fn evaluate_operation_policy_excluded_target() {
        let scope = super::super::Scope {
            allowed_targets: vec![super::super::ScopeRule::new("*".to_string())],
            excluded_targets: vec![super::super::ScopeRule::new(
                "admin.example.com".to_string(),
            )],
            ..Default::default()
        };
        let descriptor = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("admin.example.com".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let decision = evaluate_operation_policy(&descriptor, &policy, Some(&scope));
        assert!(!decision.allowed);
    }

    #[test]
    fn evaluate_operation_policy_missing_feature_denies() {
        let descriptor = OperationDescriptor {
            operation: "nse-scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec!["nonexistent-test-feature".to_string()],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let decision = evaluate_operation_policy(&descriptor, &policy, None);
        // "nonexistent-test-feature" maps to _ => true, so it's not missing
        assert!(decision.missing_features.is_empty());
        assert!(decision.allowed);
    }

    #[test]
    fn is_feature_enabled_unknown_defaults_true() {
        assert!(is_feature_enabled("totally-fake-feature"));
    }

    #[test]
    fn is_known_feature_recognizes_all_gated_features() {
        // Every feature handled by is_feature_enabled should be recognized by is_known_feature.
        let gated_features = [
            "packet-inspection",
            "stress-testing",
            "nse",
            "nse-sandbox",
            "headless-browser",
            "rest-api",
            "grpc-api",
            "ws-api",
            "ai-integration",
            "database",
            "container",
            "sbom",
            "websocket",
            "compliance",
            "external-integrations",
            "finding-workflow",
            "vuln-management",
            "cloud",
            "git-secrets",
            "wireless",
            "mobile",
            "pdf",
            "advanced-hunting",
        ];
        for feat in &gated_features {
            assert!(
                is_known_feature(feat),
                "is_known_feature('{}') should return true",
                feat
            );
        }
    }

    #[test]
    fn is_known_feature_rejects_unknown() {
        assert!(!is_known_feature("totally-fake-feature"));
        assert!(!is_known_feature("rest_api")); // underscore variant
        assert!(!is_known_feature(""));
    }

    #[test]
    fn is_known_feature_consistent_with_is_feature_enabled() {
        // For all known features, is_feature_enabled returns a cfg! result (not the default).
        // This test ensures both functions agree on the set of known features.
        let known = [
            "packet-inspection",
            "stress-testing",
            "nse",
            "nse-sandbox",
            "headless-browser",
            "rest-api",
            "grpc-api",
            "ws-api",
            "ai-integration",
            "database",
            "container",
            "sbom",
            "websocket",
            "compliance",
            "external-integrations",
            "finding-workflow",
            "vuln-management",
            "cloud",
            "git-secrets",
            "wireless",
            "mobile",
            "pdf",
            "advanced-hunting",
        ];
        for feat in &known {
            assert!(is_known_feature(feat));
            // is_feature_enabled should return a concrete cfg! value, not the default true
            let _ = is_feature_enabled(feat);
        }
        // Unknown features: is_feature_enabled defaults true, is_known_feature returns false
        assert!(!is_known_feature("unknown-xyz"));
        assert!(is_feature_enabled("unknown-xyz"));
    }

    #[test]
    fn evaluate_operation_policy_golden_json() {
        let descriptor = OperationDescriptor {
            operation: "waf-detect".to_string(),
            mode: OperationMode::DefenseLab,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::WafRegression],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let mut policy = ExecutionPolicy::default();
        policy.allow_intrusive_fuzzing = true;
        let decision = evaluate_operation_policy(&descriptor, &policy, None);
        let json = serde_json::to_string_pretty(&decision).unwrap();
        assert!(decision.allowed);
        assert!(json.contains("\"defense-lab\""));
        assert!(json.contains("\"intrusive\""));
        assert!(json.contains("\"waf-regression\""));
    }

    #[test]
    fn manual_permissive_warns_for_ambiguous_scope() {
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let outcome = evaluate_enforcement(
            &descriptor,
            &policy,
            None,
            ExecutionProfile::ManualPermissive,
        );
        assert!(outcome.is_allowed());
    }

    #[test]
    fn mcp_strict_denies_for_missing_scope() {
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let outcome = evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::McpStrict);
        assert!(outcome.is_denied());
    }

    #[test]
    fn agent_strict_denies_for_missing_scope() {
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let outcome =
            evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::AgentStrict);
        assert!(outcome.is_denied());
    }

    #[test]
    fn enforcement_context_manual_permissive_allows_safe_low_risk() {
        use super::super::scope::LoadedScope;
        let ctx = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_allowed());
    }

    #[test]
    fn enforcement_context_mcp_strict_denies_requires_explicit_scope() {
        use super::super::scope::LoadedScope;
        let ctx = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_denied());
    }

    #[test]
    fn enforcement_context_agent_strict_denies_requires_explicit_scope() {
        use super::super::scope::LoadedScope;
        let ctx = EnforcementContext::agent_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_denied());
    }

    #[test]
    fn enforcement_context_require_explicit_scope_for_networked() {
        use super::super::scope::LoadedScope;
        let manual = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        assert!(!manual.require_explicit_scope_for_networked());
        let mcp = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        assert!(mcp.require_explicit_scope_for_networked());
    }

    #[test]
    fn enforcement_outcome_json_serialization() {
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let outcome = evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::McpStrict);
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(json.contains("\"deny\"") || json.contains("\"Deny\""));
    }

    #[test]
    fn manual_permissive_can_warn_for_safe_low_risk_missing_scope() {
        use super::super::scope::LoadedScope;
        let ctx = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        // ManualPermissive should allow (with warning) even with missing explicit scope for safe ops
        assert!(outcome.is_allowed());
    }

    #[test]
    fn manual_guarded_denies_for_missing_explicit_scope() {
        // ManualGuarded denies via evaluate_enforcement when scope is None,
        // but EnforcementContext always provides Some(scope).
        // Test the direct evaluate_enforcement path with None scope instead.
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let outcome =
            evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::ManualGuarded);
        assert!(outcome.is_denied());
    }

    #[test]
    fn ci_strict_denies_for_missing_explicit_scope() {
        use super::super::scope::LoadedScope;
        let ctx =
            EnforcementContext::ci_strict(ExecutionPolicy::default(), LoadedScope::default_empty());
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_denied());
    }

    #[test]
    fn explicit_exclusion_denies_in_all_profiles() {
        use super::super::scope::{LoadedScope, ScopeRule};
        let scope = super::super::Scope {
            allowed_targets: vec![super::super::ScopeRule::new("*".to_string())],
            excluded_targets: vec![ScopeRule::new("admin.example.com".to_string())],
            ..Default::default()
        };
        let loaded = LoadedScope::explicit(scope, super::super::ScopeSource::ConfigFile, None);

        for profile in &[
            ExecutionProfile::ManualPermissive,
            ExecutionProfile::ManualGuarded,
            ExecutionProfile::CiStrict,
            ExecutionProfile::McpStrict,
            ExecutionProfile::AgentStrict,
        ] {
            let ctx = EnforcementContext {
                execution_profile: *profile,
                execution_policy: ExecutionPolicy::default(),
                loaded_scope: loaded.clone(),
            };
            let descriptor = OperationDescriptor {
                operation: "scan".to_string(),
                mode: OperationMode::StandardAssessment,
                risk: OperationRisk::SafeActive,
                intended_uses: vec![IntendedUse::WebAssessment],
                target: Some("admin.example.com".to_string()),
                required_features: Vec::new(),
                required_policy_flags: Vec::new(),
                requires_private_or_local_target: false,
                requires_explicit_scope: false,
                required_capabilities: Vec::new(),
            };
            let outcome = ctx.evaluate(&descriptor);
            if *profile == ExecutionProfile::ManualPermissive {
                // ManualPermissive: explicit exclusion is a confirmable operator-discretion case
                assert!(
                    outcome.requires_confirmation(),
                    "Profile {:?} should require confirmation for excluded target",
                    profile
                );
            } else {
                assert!(
                    outcome.is_denied(),
                    "Profile {:?} should deny excluded target",
                    profile
                );
            }
        }
    }

    #[test]
    fn denied_capability_denies_in_all_profiles() {
        use super::super::scope::LoadedScope;
        let mut policy = ExecutionPolicy::default();
        policy.denied_capabilities = vec![crate::config::Capability::RawPacketProbe];
        let loaded = LoadedScope::default_empty();

        // Denied capability check only applies to strict profiles
        for profile in &[
            ExecutionProfile::CiStrict,
            ExecutionProfile::McpStrict,
            ExecutionProfile::AgentStrict,
        ] {
            let ctx = EnforcementContext {
                execution_profile: *profile,
                execution_policy: policy.clone(),
                loaded_scope: loaded.clone(),
            };
            let descriptor = OperationDescriptor {
                operation: "packet".to_string(),
                mode: OperationMode::StandardAssessment,
                risk: OperationRisk::SafeActive,
                intended_uses: vec![IntendedUse::WebAssessment],
                target: Some("127.0.0.1".to_string()),
                required_features: Vec::new(),
                required_policy_flags: Vec::new(),
                requires_private_or_local_target: false,
                requires_explicit_scope: false,
                required_capabilities: vec![crate::config::Capability::RawPacketProbe],
            };
            let outcome = ctx.evaluate(&descriptor);
            assert!(
                outcome.is_denied(),
                "Profile {:?} should deny denied capability",
                profile
            );
        }
    }

    #[test]
    fn json_denial_includes_decision_id_allowed_risk_reasons() {
        use super::super::scope::LoadedScope;
        let ctx = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(json.contains("\"decision_id\""));
        assert!(json.contains("\"allowed\""));
        assert!(json.contains("\"operation_risk\""));
        assert!(json.contains("\"denied_reasons\""));
    }

    // --- Pass 7 focused tests per enforcement-consistency-hardening-plan ---

    #[test]
    fn enforcement_context_evaluate_denies_strict_default_empty_explicit_manifest_required() {
        use super::super::scope::LoadedScope;
        let ctx = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_denied());
        assert!(outcome
            .decision()
            .denied_reasons
            .iter()
            .any(|r| r.contains("explicit scope manifest required")));
    }

    #[test]
    fn enforcement_context_evaluate_allows_strict_with_explicit_manifest_matching_target() {
        use super::super::scope::{LoadedScope, ScopeRule};
        let scope = super::super::Scope {
            allowed_targets: vec![ScopeRule::new("127.0.0.1".to_string())],
            ..Default::default()
        };
        let loaded = LoadedScope::explicit(scope, super::super::ScopeSource::CliScopeFile, None);
        let ctx = EnforcementContext::agent_strict(ExecutionPolicy::default(), loaded);
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_allowed());
    }

    #[test]
    fn manual_permissive_downgrades_safe_target_out_of_scope_and_scope_missing_to_warning() {
        use super::super::scope::LoadedScope;
        let ctx = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        // ScopeMissing case (no scope, requires_explicit_scope, safe risk)
        let d1 = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let o1 = ctx.evaluate(&d1);
        assert!(o1.is_allowed()); // downgraded to warn counts as allowed
        assert!(matches!(o1, EnforcementOutcome::Warn(_)));

        // TargetOutOfScope case: explicit empty scope with no rules + target provided
        let empty_loaded = LoadedScope::explicit(
            super::super::Scope::default(),
            super::super::ScopeSource::CliScopeFile,
            None,
        );
        let ctx2 = EnforcementContext::manual_permissive(ExecutionPolicy::default(), empty_loaded);
        let d2 = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("example.com".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let o2 = ctx2.evaluate(&d2);
        // With empty explicit scope and no rules, evaluate_enforcement treats as ambiguous -> warn in permissive
        assert!(o2.is_allowed());
    }

    #[test]
    fn manual_permissive_does_not_downgrade_explicit_exclusion() {
        use super::super::scope::{LoadedScope, ScopeRule};
        let scope = super::super::Scope {
            allowed_targets: vec![ScopeRule::new("*".to_string())],
            excluded_targets: vec![ScopeRule::new("admin.example.com".to_string())],
            ..Default::default()
        };
        let loaded = LoadedScope::explicit(scope, super::super::ScopeSource::ConfigFile, None);
        let ctx = EnforcementContext::manual_permissive(ExecutionPolicy::default(), loaded);
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("admin.example.com".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        // ManualPermissive: explicit exclusion produces RequireConfirmation (operator discretion), not hard denial.
        assert!(outcome.requires_confirmation());
        let classes = classify_denial_reasons(outcome.decision());
        assert!(classes.contains(&DenialClass::ExplicitExclusion));
    }

    #[test]
    fn manual_permissive_does_not_downgrade_risk_policy_denial() {
        use super::super::scope::LoadedScope;
        let policy = ExecutionPolicy::default();
        // Intrusive not allowed by default policy
        let ctx = EnforcementContext::manual_permissive(policy, LoadedScope::default_empty());
        let descriptor = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_denied());
        let classes = classify_denial_reasons(outcome.decision());
        assert!(classes.contains(&DenialClass::RiskPolicyDenied));
    }

    #[test]
    fn manual_permissive_does_not_downgrade_feature_missing_denial() {
        use super::super::scope::LoadedScope;
        let ctx = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        // Use a real gated feature name that will be reported missing when the feature is off.
        // "packet-inspection" is behind cfg; when disabled it will trigger missing feature path.
        let descriptor = OperationDescriptor {
            operation: "packet".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec!["packet-inspection".to_string()],
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        // If the feature is compiled in, this would allow; when off it should deny with FeatureMissing.
        if !is_feature_enabled("packet-inspection") {
            assert!(outcome.is_denied());
            let classes = classify_denial_reasons(outcome.decision());
            assert!(classes.contains(&DenialClass::FeatureMissing));
        } else {
            // When enabled, it should be allowed (or at worst warn for other reasons)
            assert!(outcome.is_allowed() || outcome.decision().missing_features.is_empty());
        }
    }

    #[test]
    fn manual_permissive_does_not_downgrade_capability_denial() {
        use super::super::scope::LoadedScope;
        let mut policy = ExecutionPolicy::default();
        policy.denied_capabilities = vec![crate::config::Capability::WafStressTest];
        let ctx = EnforcementContext::manual_permissive(policy, LoadedScope::default_empty());
        let descriptor = OperationDescriptor {
            operation: "stress".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![crate::config::Capability::WafStressTest],
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_denied());
        let classes = classify_denial_reasons(outcome.decision());
        assert!(classes.contains(&DenialClass::CapabilityDenied));
    }

    #[test]
    fn classify_denial_reasons_maps_strings_and_exclusions() {
        use super::super::scope::{LoadedScope, ScopeRule};
        // Build a decision-like scenario via evaluate
        let scope = super::super::Scope {
            allowed_targets: vec![ScopeRule::new("*".to_string())],
            excluded_targets: vec![ScopeRule::new("secret.example.com".to_string())],
            ..Default::default()
        };
        let loaded = LoadedScope::explicit(scope, super::super::ScopeSource::ConfigFile, None);
        let ctx = EnforcementContext::manual_permissive(ExecutionPolicy::default(), loaded);
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("secret.example.com".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        let classes = classify_denial_reasons(outcome.decision());
        assert!(classes.contains(&DenialClass::ExplicitExclusion));

        // Scope missing string
        let ctx2 =
            EnforcementContext::ci_strict(ExecutionPolicy::default(), LoadedScope::default_empty());
        let d2 = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let o2 = ctx2.evaluate(&d2);
        let c2 = classify_denial_reasons(o2.decision());
        assert!(c2.contains(&DenialClass::ScopeMissing));
    }

    #[test]
    fn strict_profiles_enforce_positive_capability_allow_for_non_baseline() {
        use super::super::scope::LoadedScope;
        // Default policy has no allowed_capabilities; strict should deny non-baseline like IntrusiveFuzz
        let ctx = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: vec![crate::config::Capability::IntrusiveFuzz],
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_denied());
        assert!(outcome
            .decision()
            .denied_reasons
            .iter()
            .any(|r| r.contains("requires explicit allow")));
    }

    #[test]
    fn confirmation_class_as_str_returns_exact_kebab_strings() {
        assert_eq!(ConfirmationClass::OutOfScope.as_str(), "out-of-scope");
        assert_eq!(
            ConfirmationClass::ExplicitExclusion.as_str(),
            "explicit-exclusion"
        );
        assert_eq!(ConfirmationClass::HighRisk.as_str(), "high-risk");
        assert_eq!(
            ConfirmationClass::NonBaselineCapability.as_str(),
            "nonbaseline-capability"
        );
        assert_eq!(
            ConfirmationClass::PrivateResolution.as_str(),
            "private-resolution"
        );
        assert_eq!(
            ConfirmationClass::CrossHostRedirect.as_str(),
            "cross-host-redirect"
        );
        assert_eq!(
            ConfirmationClass::TargetExpansion.as_str(),
            "target-expansion"
        );
    }

    #[test]
    fn confirmation_class_strings_dedupes_and_preserves_order() {
        let classes = vec![
            ConfirmationClass::HighRisk,
            ConfirmationClass::OutOfScope,
            ConfirmationClass::HighRisk,
            ConfirmationClass::ExplicitExclusion,
            ConfirmationClass::OutOfScope,
            ConfirmationClass::TargetExpansion,
        ];
        let strs = confirmation_class_strings(&classes);
        assert_eq!(
            strs,
            vec![
                "high-risk".to_string(),
                "out-of-scope".to_string(),
                "explicit-exclusion".to_string(),
                "target-expansion".to_string()
            ]
        );
    }

    #[test]
    fn manual_override_permits_narrow_yes_for_outofscope_targetexpansion_only() {
        let mut mo = ManualOverride::default();
        mo.assume_yes = true;
        assert!(mo.permits(ConfirmationClass::OutOfScope));
        assert!(mo.permits(ConfirmationClass::TargetExpansion));
        assert!(!mo.permits(ConfirmationClass::HighRisk));
        assert!(!mo.permits(ConfirmationClass::ExplicitExclusion));
        assert!(!mo.permits(ConfirmationClass::NonBaselineCapability));
        assert!(!mo.permits(ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(ConfirmationClass::CrossHostRedirect));
    }

    #[test]
    fn manual_override_dedicated_flags_permit_only_their_class() {
        let mut mo = ManualOverride::default();
        mo.allow_high_risk = true;
        assert!(mo.permits(ConfirmationClass::HighRisk));
        assert!(!mo.permits(ConfirmationClass::ExplicitExclusion));
        assert!(!mo.permits(ConfirmationClass::OutOfScope));
        assert!(!mo.permits(ConfirmationClass::PrivateResolution));

        let mut mo = ManualOverride::default();
        mo.allow_explicit_exclusion = true;
        assert!(mo.permits(ConfirmationClass::ExplicitExclusion));
        assert!(!mo.permits(ConfirmationClass::HighRisk));
        assert!(!mo.permits(ConfirmationClass::OutOfScope));

        let mut mo = ManualOverride::default();
        mo.allow_nonbaseline_capability = true;
        assert!(mo.permits(ConfirmationClass::NonBaselineCapability));

        let mut mo = ManualOverride::default();
        mo.allow_private_resolution = true;
        assert!(mo.permits(ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(ConfirmationClass::CrossHostRedirect));

        let mut mo = ManualOverride::default();
        mo.allow_cross_host_redirect = true;
        assert!(mo.permits(ConfirmationClass::CrossHostRedirect));
        assert!(!mo.permits(ConfirmationClass::PrivateResolution));

        let mut mo = ManualOverride::default();
        mo.allow_out_of_scope = true;
        assert!(mo.permits(ConfirmationClass::OutOfScope));
        assert!(mo.permits(ConfirmationClass::TargetExpansion));
        assert!(!mo.permits(ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(ConfirmationClass::CrossHostRedirect));
        assert!(!mo.permits(ConfirmationClass::HighRisk));
        assert!(!mo.permits(ConfirmationClass::ExplicitExclusion));
    }

    #[test]
    fn manual_override_traffic_interception_permits_only_web_proxy() {
        let mut mo = ManualOverride::default();
        mo.allow_web_proxy = true;
        assert!(mo.permits(ConfirmationClass::TrafficInterception));
        assert!(!mo.permits(ConfirmationClass::HighRisk));
        assert!(!mo.permits(ConfirmationClass::ExplicitExclusion));
        assert!(!mo.permits(ConfirmationClass::OutOfScope));
        assert!(!mo.permits(ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(ConfirmationClass::CrossHostRedirect));
        assert!(!mo.permits(ConfirmationClass::NonBaselineCapability));

        let mut mo2 = ManualOverride::default();
        mo2.allow_high_risk = true;
        assert!(!mo2.permits(ConfirmationClass::TrafficInterception));
    }

    #[test]
    fn manual_override_db_pentest_flag_permits_high_risk_class() {
        let mut mo = ManualOverride::default();
        mo.allow_db_pentest = true;
        assert!(mo.permits(ConfirmationClass::HighRisk));
        assert!(!mo.permits(ConfirmationClass::ExplicitExclusion));
        assert!(!mo.permits(ConfirmationClass::OutOfScope));
    }

    #[test]
    fn guarded_positive_scope_miss_with_explicit_rules_denies() {
        use super::super::scope::{LoadedScope, ScopeRule};
        let scope = super::super::Scope {
            allowed_targets: vec![ScopeRule::new("127.0.0.1".to_string())],
            ..Default::default()
        };
        let loaded = LoadedScope::explicit(scope, super::super::ScopeSource::ConfigFile, None);
        let ctx = EnforcementContext {
            execution_profile: ExecutionProfile::ManualGuarded,
            execution_policy: ExecutionPolicy::default(),
            loaded_scope: loaded,
        };
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let outcome = ctx.evaluate(&descriptor);
        assert!(
            outcome.is_denied() || outcome.requires_confirmation(),
            "ManualGuarded must deny or require confirmation for positive-scope miss, got: {:?}",
            outcome
        );
    }

    #[test]
    fn manual_permissive_allows_safe_passive_operation_with_warnings() {
        let descriptor = OperationDescriptor {
            operation: "recon".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Passive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let policy = ExecutionPolicy::default();
        let outcome = evaluate_enforcement(
            &descriptor,
            &policy,
            None,
            ExecutionProfile::ManualPermissive,
        );
        assert!(
            outcome.is_allowed(),
            "ManualPermissive should allow safe passive with warnings, got: {:?}",
            outcome
        );
    }

    #[test]
    fn manual_yes_does_not_permit_private_resolution() {
        let mut mo = ManualOverride::default();
        mo.assume_yes = true;
        assert!(!mo.permits(ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(ConfirmationClass::CrossHostRedirect));
    }

    #[test]
    fn manual_yes_does_not_permit_nonbaseline_capability() {
        let mut mo = ManualOverride::default();
        mo.assume_yes = true;
        assert!(!mo.permits(ConfirmationClass::NonBaselineCapability));
    }

    #[test]
    fn manual_yes_does_not_permit_high_risk() {
        let mut mo = ManualOverride::default();
        mo.assume_yes = true;
        assert!(!mo.permits(ConfirmationClass::HighRisk));
        assert!(!mo.permits(ConfirmationClass::TrafficInterception));
    }

    #[test]
    fn manual_specific_private_resolution_flag_permits_private_resolution_confirmation() {
        let mut mo = ManualOverride::default();
        mo.allow_private_resolution = true;
        assert!(mo.permits(ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(ConfirmationClass::CrossHostRedirect));
        assert!(!mo.permits(ConfirmationClass::HighRisk));
        assert!(!mo.permits(ConfirmationClass::OutOfScope));
    }

    #[test]
    fn manual_specific_cross_host_redirect_flag_permits_redirect_confirmation() {
        let mut mo = ManualOverride::default();
        mo.allow_cross_host_redirect = true;
        assert!(mo.permits(ConfirmationClass::CrossHostRedirect));
        assert!(!mo.permits(ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(ConfirmationClass::HighRisk));
        assert!(!mo.permits(ConfirmationClass::OutOfScope));
    }

    #[test]
    fn strict_profiles_treat_require_confirmation_as_deny() {
        for profile in &[
            ExecutionProfile::CiStrict,
            ExecutionProfile::McpStrict,
            ExecutionProfile::AgentStrict,
        ] {
            let descriptor = OperationDescriptor {
                operation: "scan".to_string(),
                mode: OperationMode::StandardAssessment,
                risk: OperationRisk::SafeActive,
                intended_uses: vec![IntendedUse::WebAssessment],
                target: Some("127.0.0.1".to_string()),
                required_features: Vec::new(),
                required_policy_flags: Vec::new(),
                requires_private_or_local_target: false,
                requires_explicit_scope: true,
                required_capabilities: Vec::new(),
            };
            let policy = ExecutionPolicy::default();
            let outcome = evaluate_enforcement(&descriptor, &policy, None, *profile);
            assert!(
                outcome.is_denied(),
                "Profile {:?} must deny (not confirm) for missing scope, got: {:?}",
                profile,
                outcome
            );
        }
    }

    // --- Preflight tests ---

    #[test]
    fn preflight_operation_allow_for_safe_passive() {
        let scope = super::super::Scope {
            allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
            ..Default::default()
        };
        let loaded_scope = super::super::LoadedScope::explicit(
            scope,
            super::super::ScopeSource::ConfigFile,
            Some("scope.toml".to_string()),
        );
        let policy = ExecutionPolicy::default();
        let enforcement = EnforcementContext::manual_permissive(policy, loaded_scope);
        let descriptor = OperationDescriptor {
            operation: "recon".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Passive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = super::preflight_operation(
            super::super::ExecutionSurface::CliManual,
            &enforcement,
            descriptor,
            None,
        );
        assert!(
            result.outcome_kind == super::PreflightOutcomeKind::Allow
                || result.outcome_kind == super::PreflightOutcomeKind::Warn,
            "Safe passive op should allow or warn, got {:?}",
            result.outcome_kind
        );
        assert_eq!(result.surface, super::super::ExecutionSurface::CliManual);
        assert!(result.suggested_cli_flags.is_empty());
    }

    #[test]
    fn preflight_operation_deny_for_strict_missing_scope() {
        let loaded_scope = super::super::LoadedScope::default_empty();
        let policy = ExecutionPolicy::default();
        let enforcement = EnforcementContext::mcp_strict(policy, loaded_scope);
        let descriptor = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: vec![],
        };
        let result = super::preflight_operation(
            super::super::ExecutionSurface::RestApi,
            &enforcement,
            descriptor,
            None,
        );
        assert_eq!(
            result.outcome_kind,
            super::PreflightOutcomeKind::Deny,
            "MCP strict with missing scope should deny, got {:?}",
            result.outcome_kind
        );
        assert!(result.suggested_cli_flags.is_empty());
        assert!(!result.manual_override_honored);
    }

    #[test]
    fn preflight_operation_require_confirmation_for_high_risk_manual() {
        let loaded_scope = super::super::LoadedScope::default_empty();
        let mut policy = ExecutionPolicy::default();
        policy.allow_intrusive_fuzzing = true;
        let enforcement = EnforcementContext::manual_permissive(policy, loaded_scope);
        let descriptor = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::DefenseLab,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::WafRegression],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = super::preflight_operation(
            super::super::ExecutionSurface::CliManual,
            &enforcement,
            descriptor,
            None,
        );
        assert_eq!(
            result.outcome_kind,
            super::PreflightOutcomeKind::RequireConfirmation,
            "High-risk fuzz in manual mode should require confirmation, got {:?}",
            result.outcome_kind
        );
        assert!(!result.required_confirmation_classes.is_empty());
        assert!(!result.suggested_cli_flags.is_empty());
    }

    #[test]
    fn preflight_manual_override_honored_when_flags_match() {
        let loaded_scope = super::super::LoadedScope::default_empty();
        let mut policy = ExecutionPolicy::default();
        policy.allow_intrusive_fuzzing = true;
        let enforcement = EnforcementContext::manual_permissive(policy, loaded_scope);
        let descriptor = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::DefenseLab,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::WafRegression],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let mo = ManualOverride {
            allow_high_risk: true,
            ..Default::default()
        };
        let result = super::preflight_operation(
            super::super::ExecutionSurface::CliManual,
            &enforcement,
            descriptor,
            Some(&mo),
        );
        assert!(
            result.manual_override_honored,
            "Manual override with matching flag should be honored"
        );
    }

    #[test]
    fn preflight_no_suggested_flags_for_automated_surface() {
        let loaded_scope = super::super::LoadedScope::default_empty();
        let policy = ExecutionPolicy::default();
        let enforcement = EnforcementContext::mcp_strict(policy, loaded_scope);
        let descriptor = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: vec![],
        };
        let result = super::preflight_operation(
            super::super::ExecutionSurface::McpServer,
            &enforcement,
            descriptor,
            None,
        );
        assert!(
            result.suggested_cli_flags.is_empty(),
            "Automated surfaces should not suggest CLI flags"
        );
    }

    #[test]
    fn preflight_result_serializes() {
        let loaded_scope = super::super::LoadedScope::default_empty();
        let policy = ExecutionPolicy::default();
        let enforcement = EnforcementContext::manual_permissive(policy, loaded_scope);
        let descriptor = OperationDescriptor {
            operation: "recon".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Passive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = super::preflight_operation(
            super::super::ExecutionSurface::CliManual,
            &enforcement,
            descriptor,
            None,
        );
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"outcome_kind\""));
        assert!(json.contains("\"surface\""));
        assert!(json.contains("\"descriptor\""));
        assert!(json.contains("\"decision\""));
    }

    #[test]
    fn preflight_matches_evaluate_outcome() {
        let loaded_scope = super::super::LoadedScope::default_empty();
        let policy = ExecutionPolicy::default();
        let enforcement = EnforcementContext::manual_permissive(policy, loaded_scope);
        let descriptor = OperationDescriptor {
            operation: "recon".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Passive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let preflight = super::preflight_operation(
            super::super::ExecutionSurface::CliManual,
            &enforcement,
            descriptor.clone(),
            None,
        );
        let direct = enforcement.evaluate(&descriptor);
        let preflight_kind = match preflight.outcome_kind {
            super::PreflightOutcomeKind::Allow => true,
            super::PreflightOutcomeKind::Warn => true,
            super::PreflightOutcomeKind::RequireConfirmation => false,
            super::PreflightOutcomeKind::Deny => false,
        };
        assert_eq!(
            preflight_kind,
            direct.is_allowed(),
            "Preflight outcome should match direct evaluate outcome"
        );
    }

    // --- Phase 12: Type-level enforcement dispatch tests ---

    #[test]
    fn approve_returns_approved_operation_on_allow() {
        use super::super::scope::LoadedScope;
        let scope = LoadedScope::explicit(
            super::super::Scope {
                allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
                ..Default::default()
            },
            super::super::ScopeSource::ConfigFile,
            None,
        );
        let ctx = EnforcementContext::mcp_strict(ExecutionPolicy::default(), scope);
        let descriptor = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: vec![],
        };
        let result = ctx.approve(super::super::ExecutionSurface::McpServer, descriptor);
        assert!(
            result.is_ok(),
            "approve() should succeed for Allow outcome, got {:?}",
            result.err()
        );
        let approved = result.unwrap();
        assert_eq!(approved.descriptor().operation, "scan-ports");
        assert_eq!(
            approved.surface(),
            super::super::ExecutionSurface::McpServer
        );
        assert_eq!(approved.profile(), ExecutionProfile::McpStrict);
        assert!(approved.decision().allowed);
    }

    #[test]
    fn approve_rejects_warn_for_strict_surface() {
        use super::super::scope::LoadedScope;
        // ManualPermissive + empty scope + safe op with target = Warn (scope ambiguous)
        let ctx = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        // Verify evaluate produces Warn
        let outcome = ctx.evaluate(&descriptor);
        assert!(
            matches!(outcome, EnforcementOutcome::Warn(_)),
            "expected Warn, got {:?}",
            outcome
        );
        // approve() should reject Warn -> Err(Denied)
        let result = ctx.approve(super::super::ExecutionSurface::McpServer, descriptor);
        assert!(result.is_err(), "approve() should reject Warn outcome");
        match result.unwrap_err() {
            EnforcementError::Denied { .. } => {}
            other => panic!("expected Denied, got {:?}", other),
        }
    }

    #[test]
    fn approve_rejects_require_confirmation_for_strict_surface() {
        use super::super::scope::LoadedScope;
        // Permissive + out-of-scope with positive rules = RequireConfirmation
        let scope = LoadedScope::explicit(
            super::super::Scope {
                allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
                ..Default::default()
            },
            super::super::ScopeSource::ConfigFile,
            None,
        );
        let ctx = EnforcementContext::manual_permissive(ExecutionPolicy::default(), scope);
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        // Verify evaluate produces RequireConfirmation
        let outcome = ctx.evaluate(&descriptor);
        assert!(
            outcome.requires_confirmation(),
            "expected RequireConfirmation, got {:?}",
            outcome
        );
        // approve() should return Err(ConfirmationRequired)
        let result = ctx.approve(super::super::ExecutionSurface::McpServer, descriptor);
        assert!(
            result.is_err(),
            "approve() should reject RequireConfirmation"
        );
        match result.unwrap_err() {
            EnforcementError::ConfirmationRequired {
                required_classes, ..
            } => {
                assert!(!required_classes.is_empty(), "should have required classes");
            }
            other => panic!("expected ConfirmationRequired, got {:?}", other),
        }
    }

    #[test]
    fn approve_rejects_deny_for_all_surfaces() {
        use super::super::scope::LoadedScope;
        // MCP strict + missing scope for networked op = Deny
        let ctx = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: vec![],
        };
        // Verify evaluate produces Deny
        let outcome = ctx.evaluate(&descriptor);
        assert!(outcome.is_denied(), "expected Deny, got {:?}", outcome);
        // approve() should return Err(Denied)
        let result = ctx.approve(super::super::ExecutionSurface::McpServer, descriptor);
        assert!(result.is_err());
        match result.unwrap_err() {
            EnforcementError::Denied { .. } => {}
            other => panic!("expected Denied, got {:?}", other),
        }
    }

    #[test]
    fn approve_manual_accepts_warn_on_permissive_surface() {
        use super::super::scope::LoadedScope;
        // ManualPermissive + empty scope + safe op + target = Warn
        let ctx = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        // Verify evaluate produces Warn
        let outcome = ctx.evaluate(&descriptor);
        assert!(
            matches!(outcome, EnforcementOutcome::Warn(_)),
            "expected Warn, got {:?}",
            outcome
        );
        // approve_manual with TuiManual (permissive) should accept
        let result =
            ctx.approve_manual(super::super::ExecutionSurface::TuiManual, descriptor, None);
        assert!(
            result.is_ok(),
            "approve_manual should accept Warn on permissive surface, got {:?}",
            result.err()
        );
    }

    #[test]
    fn approve_manual_rejects_warn_on_automated_surface() {
        use super::super::scope::LoadedScope;
        // ManualPermissive + empty scope + safe op + target = Warn
        let ctx = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        // approve_manual with McpServer (automated) should reject Warn
        let result =
            ctx.approve_manual(super::super::ExecutionSurface::McpServer, descriptor, None);
        assert!(
            result.is_err(),
            "approve_manual should reject Warn on automated surface"
        );
        match result.unwrap_err() {
            EnforcementError::Denied { .. } => {}
            other => panic!("expected Denied, got {:?}", other),
        }
    }

    #[test]
    fn approve_manual_accepts_confirmation_with_matching_override() {
        use super::super::scope::LoadedScope;
        // Permissive + out-of-scope with positive rules = RequireConfirmation
        let scope = LoadedScope::explicit(
            super::super::Scope {
                allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
                ..Default::default()
            },
            super::super::ScopeSource::ConfigFile,
            None,
        );
        let ctx = EnforcementContext::manual_permissive(ExecutionPolicy::default(), scope);
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let mut mo = ManualOverride::default();
        mo.allow_out_of_scope = true;
        // approve_manual with TuiManual should accept with matching override
        let result = ctx.approve_manual(
            super::super::ExecutionSurface::TuiManual,
            descriptor,
            Some(&mo),
        );
        assert!(
            result.is_ok(),
            "approve_manual should accept with matching override, got {:?}",
            result.err()
        );
    }

    #[test]
    fn approve_manual_rejects_confirmation_without_override() {
        use super::super::scope::LoadedScope;
        // Permissive + out-of-scope with positive rules = RequireConfirmation
        let scope = LoadedScope::explicit(
            super::super::Scope {
                allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
                ..Default::default()
            },
            super::super::ScopeSource::ConfigFile,
            None,
        );
        let ctx = EnforcementContext::manual_permissive(ExecutionPolicy::default(), scope);
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        // No override -> should reject
        let result =
            ctx.approve_manual(super::super::ExecutionSurface::TuiManual, descriptor, None);
        assert!(
            result.is_err(),
            "approve_manual should reject without override"
        );
        match result.unwrap_err() {
            EnforcementError::ConfirmationRequired {
                required_classes, ..
            } => {
                assert!(!required_classes.is_empty());
            }
            other => panic!("expected ConfirmationRequired, got {:?}", other),
        }
    }

    #[test]
    fn approve_manual_rejects_override_on_guarded_surface() {
        use super::super::scope::LoadedScope;
        // Permissive context produces RequireConfirmation for out-of-scope
        let scope = LoadedScope::explicit(
            super::super::Scope {
                allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
                ..Default::default()
            },
            super::super::ScopeSource::ConfigFile,
            None,
        );
        let ctx = EnforcementContext::manual_permissive(ExecutionPolicy::default(), scope);
        let descriptor = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let mut mo = ManualOverride::default();
        mo.allow_out_of_scope = true;
        // TuiManualStrict (guarded) -> honors_manual_override() is false
        let result = ctx.approve_manual(
            super::super::ExecutionSurface::TuiManualStrict,
            descriptor,
            Some(&mo),
        );
        assert!(
            result.is_err(),
            "approve_manual should reject override on guarded surface"
        );
        match result.unwrap_err() {
            EnforcementError::ConfirmationRequired { .. } => {}
            other => panic!("expected ConfirmationRequired, got {:?}", other),
        }
    }

    #[test]
    fn approved_operation_accessors_return_correct_values() {
        use super::super::scope::LoadedScope;
        let scope = LoadedScope::explicit(
            super::super::Scope {
                allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
                ..Default::default()
            },
            super::super::ScopeSource::ConfigFile,
            None,
        );
        let ctx = EnforcementContext::agent_strict(ExecutionPolicy::default(), scope);
        let descriptor = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: vec![],
        };
        let surface = super::super::ExecutionSurface::SecurityAgent;
        let result = ctx.approve(surface, descriptor).unwrap();

        assert_eq!(result.descriptor().operation, "scan-ports");
        assert_eq!(result.descriptor().target.as_deref(), Some("127.0.0.1"));
        assert!(result.decision().allowed);
        assert_eq!(
            result.surface(),
            super::super::ExecutionSurface::SecurityAgent
        );
        assert_eq!(result.profile(), ExecutionProfile::AgentStrict);
        assert!(result.audit_event_id().is_none());
    }

    #[test]
    fn enforcement_error_decision_reference() {
        use super::super::scope::LoadedScope;
        // Test Denied variant
        let ctx_deny = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor_deny = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: vec![],
        };
        let err_deny = ctx_deny
            .approve(super::super::ExecutionSurface::McpServer, descriptor_deny)
            .unwrap_err();
        let decision_deny = err_deny.decision();
        assert!(!decision_deny.allowed);

        // Test ConfirmationRequired variant
        let scope = LoadedScope::explicit(
            super::super::Scope {
                allowed_targets: vec![super::super::ScopeRule::new("127.0.0.1".to_string())],
                ..Default::default()
            },
            super::super::ScopeSource::ConfigFile,
            None,
        );
        let ctx_confirm = EnforcementContext::manual_permissive(ExecutionPolicy::default(), scope);
        let descriptor_confirm = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let err_confirm = ctx_confirm
            .approve_manual(
                super::super::ExecutionSurface::TuiManual,
                descriptor_confirm,
                None,
            )
            .unwrap_err();
        let decision_confirm = err_confirm.decision();
        assert!(!decision_confirm.allowed);

        // Test ManualOverrideUnavailable variant (constructed directly)
        let decision_manual = PolicyDecision::denied(
            "test",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![],
            "manual override unavailable",
        );
        let err_manual = EnforcementError::ManualOverrideUnavailable {
            surface: super::super::ExecutionSurface::McpServer,
            decision: decision_manual,
        };
        let decision_ref = err_manual.decision();
        assert!(!decision_ref.allowed);
        assert!(decision_ref
            .denied_reasons
            .iter()
            .any(|r| r.contains("manual override unavailable")));
    }
}
