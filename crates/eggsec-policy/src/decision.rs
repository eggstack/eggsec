//! Deterministic policy evaluation over explicit inputs (no I/O).
//!
//! All evaluation functions take an explicit [`EnabledFeatures`] set and,
//! when a target is involved, an optional pre-resolved [`TargetScope`] fact
//! record supplied by the caller. This crate never queries compile-time
//! feature state and never performs DNS resolution itself:
//!
//! - feature availability is constructed by the engine from its
//!   `feature_registry` and passed as [`EnabledFeatures`];
//! - destination facts are acquired by the engine resolver bridge
//!   (`eggsec::policy_bridge::resolver`) or transport authority and passed
//!   as [`TargetScope`].
//!
//! The same operation can therefore be evaluated against different supplied
//! feature sets and resolution facts without recompiling this crate.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    baseline_allowed_capability, DenialClass, EnabledFeatures, ExecutionPolicy, ExecutionProfile,
    ExecutionSurface, IntendedUse, LoadedScope, OperationDescriptor, OperationMode, OperationRisk,
    Scope, ScopeSource, TargetScope,
};

// Approval token issuance lives in a cohesive module; re-exported here so
// `eggsec_policy::decision::ApprovedOperation` and `eggsec_policy::ApprovedOperation`
// remain stable paths.
pub use crate::approval::ApprovedOperation;

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
    /// Typed denial classes populated during evaluation.
    /// Replaces string inspection in `classify_denial_reasons()`.
    pub denial_classes: Vec<DenialClass>,
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
            denial_classes: Vec::new(),
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
            denial_classes: Vec::new(),
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

    pub fn with_resolved_addresses(mut self, addresses: Vec<String>) -> Self {
        self.resolved_addresses = addresses;
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

    /// Push a typed denial class. Also appends a human-readable reason to `denied_reasons`
    /// for backward compatibility.
    pub fn push_denial_class(&mut self, class: DenialClass, reason: &str) {
        self.denial_classes.push(class);
        self.denied_reasons.push(reason.to_string());
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

    /// Builder method to add a typed denial class with a human-readable reason.
    pub fn with_denial_class(mut self, class: DenialClass, reason: &str) -> Self {
        self.denial_classes.push(class);
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
#[allow(clippy::result_large_err)]
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

    /// Caller-provided surface does not match the enforcement context's profile.
    ///
    /// This is a configuration/programming error: the surface passed to
    /// `approve()` or `approve_manual()` must derive the same profile as the
    /// context was constructed with.
    #[error(
        "surface/profile mismatch: surface '{surface}' derives profile '{surface_profile}' \
         but context has profile '{context_profile}'"
    )]
    SurfaceProfileMismatch {
        surface: ExecutionSurface,
        surface_profile: ExecutionProfile,
        context_profile: ExecutionProfile,
    },
}

impl EnforcementError {
    /// Returns a reference to the inner `PolicyDecision`, if available.
    ///
    /// Returns `None` for [`SurfaceProfileMismatch`](Self::SurfaceProfileMismatch)
    /// which is a configuration error without an associated policy decision.
    pub fn decision(&self) -> Option<&PolicyDecision> {
        match self {
            Self::Denied { decision }
            | Self::ConfirmationRequired { decision, .. }
            | Self::ManualOverrideUnavailable { decision, .. } => Some(decision),
            Self::SurfaceProfileMismatch { .. } => None,
        }
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
///
/// Derives `PartialEq`/`Eq` so frontend approval caches can invalidate on
/// override-state changes without hand-maintaining a field subset.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// Reusable enforcement context that bundles execution profile, policy, scope,
/// and the explicit feature-availability input.
///
/// Created once per execution path (CLI, MCP, agent) and used to evaluate
/// every operation descriptor through the same shared enforcement logic.
/// Feature availability is caller-supplied [`EnabledFeatures`]: this crate
/// never queries engine `cfg!` state. Destination facts are supplied per
/// evaluation via [`TargetScope`]: this crate never performs DNS itself.
#[derive(Debug, Clone)]
pub struct EnforcementContext {
    pub execution_profile: ExecutionProfile,
    pub execution_policy: ExecutionPolicy,
    pub loaded_scope: LoadedScope,
    pub enabled_features: EnabledFeatures,
}

impl EnforcementContext {
    pub fn manual_permissive(
        policy: ExecutionPolicy,
        loaded_scope: LoadedScope,
        enabled_features: EnabledFeatures,
    ) -> Self {
        Self {
            execution_profile: ExecutionProfile::ManualPermissive,
            execution_policy: policy,
            loaded_scope,
            enabled_features,
        }
    }
    pub fn manual_guarded(
        policy: ExecutionPolicy,
        loaded_scope: LoadedScope,
        enabled_features: EnabledFeatures,
    ) -> Self {
        Self {
            execution_profile: ExecutionProfile::ManualGuarded,
            execution_policy: policy,
            loaded_scope,
            enabled_features,
        }
    }
    pub fn ci_strict(
        policy: ExecutionPolicy,
        loaded_scope: LoadedScope,
        enabled_features: EnabledFeatures,
    ) -> Self {
        Self {
            execution_profile: ExecutionProfile::CiStrict,
            execution_policy: policy,
            loaded_scope,
            enabled_features,
        }
    }
    pub fn mcp_strict(
        policy: ExecutionPolicy,
        loaded_scope: LoadedScope,
        enabled_features: EnabledFeatures,
    ) -> Self {
        Self {
            execution_profile: ExecutionProfile::McpStrict,
            execution_policy: policy,
            loaded_scope,
            enabled_features,
        }
    }
    pub fn agent_strict(
        policy: ExecutionPolicy,
        loaded_scope: LoadedScope,
        enabled_features: EnabledFeatures,
    ) -> Self {
        Self {
            execution_profile: ExecutionProfile::AgentStrict,
            execution_policy: policy,
            loaded_scope,
            enabled_features,
        }
    }

    /// Construct an [`EnforcementContext`] from an [`ExecutionSurface`].
    ///
    /// This is the canonical way to build enforcement from a caller-origin
    /// identity. It delegates to the appropriate profile-specific constructor.
    pub fn for_surface(
        surface: ExecutionSurface,
        policy: ExecutionPolicy,
        loaded_scope: LoadedScope,
        enabled_features: EnabledFeatures,
    ) -> Self {
        match surface.profile() {
            ExecutionProfile::ManualPermissive => {
                Self::manual_permissive(policy, loaded_scope, enabled_features)
            }
            ExecutionProfile::ManualGuarded => {
                Self::manual_guarded(policy, loaded_scope, enabled_features)
            }
            ExecutionProfile::CiStrict => Self::ci_strict(policy, loaded_scope, enabled_features),
            ExecutionProfile::McpStrict => Self::mcp_strict(policy, loaded_scope, enabled_features),
            ExecutionProfile::AgentStrict => {
                Self::agent_strict(policy, loaded_scope, enabled_features)
            }
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
    /// `target_facts` carries the caller-resolved [`TargetScope`] for
    /// `descriptor.target` (or `None` when the operation has no target).
    /// Centralizes explicit-manifest provenance checks for strict profiles.
    /// The inner evaluate_enforcement receives the scope rules, but provenance
    /// (LoadedScope::is_explicit_manifest) is enforced here for automated profiles.
    pub fn evaluate(
        &self,
        descriptor: &OperationDescriptor,
        target_facts: Option<&TargetScope>,
    ) -> EnforcementOutcome {
        let outcome = evaluate_enforcement(
            descriptor,
            &self.execution_policy,
            Some(&self.loaded_scope.scope),
            self.execution_profile,
            &self.enabled_features,
            target_facts,
        );

        if self.requires_explicit_manifest_for(descriptor)
            && !self.loaded_scope.is_explicit_manifest()
        {
            let mut decision = outcome.decision().clone().with_denial_class(
                DenialClass::ScopeMissing,
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
    /// The caller-provided `surface` must derive the same profile as this
    /// context was constructed with. Mismatches return
    /// [`EnforcementError::SurfaceProfileMismatch`].
    ///
    /// Use this for REST, MCP, Agent, and CI surfaces.
    #[allow(clippy::result_large_err)]
    pub fn approve(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        target_facts: Option<&TargetScope>,
    ) -> Result<ApprovedOperation, EnforcementError> {
        let surface_profile = surface.profile();
        if surface_profile != self.execution_profile {
            return Err(EnforcementError::SurfaceProfileMismatch {
                surface,
                surface_profile,
                context_profile: self.execution_profile,
            });
        }

        let outcome = self.evaluate(&descriptor, target_facts);
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
    /// The caller-provided `surface` must derive the same profile as this
    /// context was constructed with. Mismatches return
    /// [`EnforcementError::SurfaceProfileMismatch`].
    ///
    /// Use this for CLI and TUI manual dispatch paths.
    #[allow(clippy::result_large_err)]
    pub fn approve_manual(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        target_facts: Option<&TargetScope>,
        manual_override: Option<&ManualOverride>,
    ) -> Result<ApprovedOperation, EnforcementError> {
        let surface_profile = surface.profile();
        if surface_profile != self.execution_profile {
            return Err(EnforcementError::SurfaceProfileMismatch {
                surface,
                surface_profile,
                context_profile: self.execution_profile,
            });
        }

        let outcome = self.evaluate(&descriptor, target_facts);
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
        let json = match serde_json::to_vec(&self.execution_policy) {
            Ok(json) => json,
            // Never panic on the enforcement path: fall back to a static
            // sentinel hash so callers can still correlate decisions.
            // (No logging in the I/O-free kernel; engine bridges may log.)
            Err(_) => {
                return hex::encode(Sha256::digest(b"eggsec-policy-serialization-failed"));
            }
        };
        let hash = Sha256::digest(&json);
        hex::encode(hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightResult {
    pub surface: ExecutionSurface,
    pub profile: ExecutionProfile,
    pub descriptor: OperationDescriptor,
    pub outcome_kind: PreflightOutcomeKind,
    pub decision: PolicyDecision,
    pub required_confirmation_classes: Vec<ConfirmationClass>,
    pub manual_override_honored: bool,
    pub scope_source: ScopeSource,
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
    surface: ExecutionSurface,
    enforcement: &EnforcementContext,
    descriptor: OperationDescriptor,
    target_facts: Option<&TargetScope>,
    manual_override: Option<&ManualOverride>,
) -> PreflightResult {
    let outcome = enforcement.evaluate(&descriptor, target_facts);
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
        scope_source: enforcement.loaded_scope.source,
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

/// Shared policy evaluation entry point.
///
/// Takes an [`OperationDescriptor`], the current [`ExecutionPolicy`], an
/// optional [`Scope`], the explicit [`EnabledFeatures`] input, and the
/// caller-resolved [`TargetScope`] facts for `descriptor.target` (or `None`
/// when the operation has no target), and returns a fully-populated
/// [`PolicyDecision`].
///
/// This is the canonical function that command handlers, MCP dispatchers,
/// agent workflows, and API endpoints should call instead of building
/// policy checks inline. Callers acquire destination facts first (engine
/// resolver bridge or transport authority); this function only decides.
pub fn evaluate_operation_policy(
    descriptor: &OperationDescriptor,
    policy: &ExecutionPolicy,
    scope: Option<&Scope>,
    features: &EnabledFeatures,
    target_facts: Option<&TargetScope>,
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

    // Check required feature availability against the explicit input set.
    // Fail-closed: any required feature absent from `features` denies.
    for feature in &descriptor.required_features {
        if !features.contains(feature) {
            decision = decision.with_missing_feature(feature);
            decision.push_denial_class(
                DenialClass::FeatureMissing,
                &format!("required feature '{}' is not enabled", feature),
            );
            decision.allowed = false;
        }
    }

    // Check scope if a target and scope are provided. `target_facts` carries
    // the caller-resolved addresses; when absent for a target-bearing
    // descriptor, hostname-pattern matching over an empty fact record decides
    // (equivalent to the historical unresolvable-host path).
    if let Some(ref target) = descriptor.target {
        if let Some(scope) = scope {
            // Populate resolved addresses in the decision record
            if let Some(ts) = target_facts {
                if !ts.resolved_addresses.is_empty() {
                    decision.resolved_addresses = ts
                        .resolved_addresses
                        .iter()
                        .map(|a| a.to_string())
                        .collect();
                }
            }

            // Pure evaluation over supplied facts (no DNS here).
            let facts = match target_facts {
                Some(ts) => ts.clone(),
                None => TargetScope::for_host_without_addresses(target.clone()),
            };
            match scope.evaluate_facts(&facts) {
                Ok(true) => {
                    decision
                        .matched_scope_rules
                        .push("target in scope".to_string());
                }
                Ok(false) => {
                    if scope.is_explicitly_excluded(&facts) {
                        decision
                            .matched_exclusion_rules
                            .push(format!("excluded: {}", target));
                        decision.push_denial_class(
                            DenialClass::ExplicitExclusion,
                            "target is explicitly excluded from scope",
                        );
                    } else {
                        decision.push_denial_class(
                            DenialClass::TargetOutOfScope,
                            "target not in scope",
                        );
                    }
                    decision.allowed = false;
                }
                Err(e) => {
                    decision.push_denial_class(
                        DenialClass::InvalidTarget,
                        &format!("scope check error: {}", e),
                    );
                    decision.allowed = false;
                }
            }
        } else if descriptor.requires_explicit_scope || descriptor.requires_private_or_local_target
        {
            decision.push_denial_class(
                DenialClass::ScopeMissing,
                "scope file required but not provided",
            );
            decision.allowed = false;
        } else if let Ok(ip) = target.parse::<std::net::IpAddr>() {
            let class = crate::classify_address(&ip);
            if class.is_non_public() && class != crate::AddressClass::Loopback {
                decision.warnings.push(format!(
                    "target is a {} IP; scope file recommended for defense-lab profiles",
                    class
                ));
            }
        }
    }

    // Check risk against execution policy
    if !descriptor.risk.is_allowed_by(policy) {
        decision.push_denial_class(
            DenialClass::RiskPolicyDenied,
            &format!(
                "operation risk '{}' is not allowed by current execution policy",
                descriptor.risk
            ),
        );
        decision.allowed = false;
    }

    // Check required policy flags
    for flag in &descriptor.required_policy_flags {
        match flag.as_str() {
            "require_explicit_scope" => {
                if !policy.require_explicit_scope {
                    decision.push_denial_class(
                        DenialClass::ScopeMissing,
                        "require_explicit_scope is disabled in policy",
                    );
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
///
/// When `denial_classes` is populated (new code path), returns those directly.
/// Falls back to string inspection of `denied_reasons` for legacy compatibility.
pub fn classify_denial_reasons(decision: &PolicyDecision) -> Vec<DenialClass> {
    // Prefer typed denial classes when available (new code path)
    if !decision.denial_classes.is_empty() {
        let set: rustc_hash::FxHashSet<DenialClass> =
            decision.denial_classes.iter().copied().collect();
        return set.into_iter().collect();
    }

    // Legacy fallback: string inspection of denied_reasons
    let mut classes: rustc_hash::FxHashSet<DenialClass> = rustc_hash::FxHashSet::default();
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
/// Calls [`evaluate_operation_policy`] internally over the explicit
/// [`EnabledFeatures`] input and caller-supplied [`TargetScope`] facts, then
/// transforms the resulting [`PolicyDecision`] into [`EnforcementOutcome::Allow`],
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
    features: &EnabledFeatures,
    target_facts: Option<&TargetScope>,
) -> EnforcementOutcome {
    let mut decision = evaluate_operation_policy(descriptor, policy, scope, features, target_facts);

    // Capability checks (denied always deny; strict profiles require explicit allow for non-baseline)
    if !decision.required_features.is_empty() || !descriptor.required_capabilities.is_empty() {
        // Denied capabilities always deny, regardless of profile
        for cap in &descriptor.required_capabilities {
            if policy.denied_capabilities.contains(cap) {
                decision.push_denial_class(
                    DenialClass::CapabilityDenied,
                    &format!("capability '{}' is denied by execution policy", cap),
                );
                decision.allowed = false;
                return EnforcementOutcome::Deny(decision);
            }
        }

        // For strict automated profiles, non-baseline capabilities must be explicitly allowed
        if profile.is_strict() {
            for cap in &descriptor.required_capabilities {
                if !policy.allowed_capabilities.contains(cap) && !baseline_allowed_capability(*cap)
                {
                    decision.push_denial_class(
                        DenialClass::CapabilityDenied,
                        &format!(
                            "capability '{}' requires explicit allow in {} execution policy",
                            cap, profile
                        ),
                    );
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
        if !baseline_allowed_capability(*cap)
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
    let mut seen = rustc_hash::FxHashSet::default();
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
mod pure_evaluation_tests {
    use super::*;
    use crate::ScopeRule;

    fn permissive_ctx(features: EnabledFeatures) -> EnforcementContext {
        EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
            features,
        )
    }

    fn strict_ctx(features: EnabledFeatures) -> EnforcementContext {
        EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
            features,
        )
    }

    fn nse_descriptor(target: Option<&str>) -> OperationDescriptor {
        OperationDescriptor::new(
            "nse-scan".to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            Vec::new(),
            target.map(str::to_string),
            vec!["nse".to_string()],
            Vec::new(),
            false,
            false,
            Vec::new(),
        )
    }

    #[test]
    fn same_operation_evaluates_against_different_feature_sets() {
        // Phase C WS1 invariant: no recompilation needed to vary availability.
        let descriptor = nse_descriptor(None);
        let with_nse = EnabledFeatures::from_names(["nse"]);
        let without = EnabledFeatures::empty();

        let allowed = evaluate_operation_policy(
            &descriptor,
            &ExecutionPolicy::default(),
            None,
            &with_nse,
            None,
        );
        assert!(allowed.allowed, "nse-gated op allows when nse is supplied");

        let denied = evaluate_operation_policy(
            &descriptor,
            &ExecutionPolicy::default(),
            None,
            &without,
            None,
        );
        assert!(
            !denied.allowed,
            "nse-gated op denies when features are empty"
        );
        assert!(denied.denial_classes.contains(&DenialClass::FeatureMissing));
    }

    #[test]
    fn unknown_feature_names_fail_closed() {
        let features = EnabledFeatures::from_names(["nse"]);
        assert!(!features.contains("totally-fake-feature"));
        let mut descriptor = nse_descriptor(None);
        descriptor.required_features = vec!["totally-fake-feature".to_string()];
        let decision = evaluate_operation_policy(
            &descriptor,
            &ExecutionPolicy::default(),
            None,
            &features,
            None,
        );
        assert!(!decision.allowed);
    }

    #[test]
    fn pure_scope_evaluation_consumes_supplied_facts() {
        // Allowlist for example.com; no DNS is performed here.
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));
        let ctx = EnforcementContext::manual_guarded(
            ExecutionPolicy::default(),
            LoadedScope::explicit(scope, ScopeSource::ConfigFile, None),
            EnabledFeatures::empty(),
        );
        let descriptor = OperationDescriptor::new(
            "scan-ports".to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            Vec::new(),
            Some("example.com".to_string()),
            Vec::new(),
            Vec::new(),
            false,
            false,
            Vec::new(),
        );
        let allowed_facts = TargetScope::for_host_without_addresses("example.com");
        let denied_facts = TargetScope::for_host_without_addresses("other.com");
        assert!(ctx.evaluate(&descriptor, Some(&allowed_facts)).is_allowed());
        assert!(!ctx.evaluate(&descriptor, Some(&denied_facts)).is_allowed());
    }

    #[test]
    fn mixed_resolution_facts_deny_without_filtering() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::with_cidr("93.184.216.0/24".to_string()).unwrap());
        let mixed = TargetScope::for_host_with_addresses(
            "mixed.example",
            vec![
                "93.184.216.34".parse().unwrap(),
                "203.0.113.99".parse().unwrap(),
            ],
        );
        // Transport binding invariant holds at the pure layer: mixed answers
        // deny rather than silently subsetting.
        assert_eq!(scope.evaluate_facts(&mixed).unwrap(), false);
    }

    #[test]
    fn approval_binds_target_and_operation() {
        let ctx = strict_ctx(EnabledFeatures::empty());
        let base = OperationDescriptor::new(
            "scan-ports".to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            Vec::new(),
            Some("127.0.0.1".to_string()),
            Vec::new(),
            Vec::new(),
            false,
            false,
            Vec::new(),
        );
        let facts = TargetScope::for_ip("127.0.0.1".parse().unwrap());
        let approved = ctx
            .approve(ExecutionSurface::RestApi, base.clone(), Some(&facts))
            .expect("loopback in default scope allows");
        assert!(approved.matches_descriptor(&base));
        let mut other = base.clone();
        other.risk = OperationRisk::Intrusive;
        assert!(!approved.matches_descriptor(&other));
    }

    #[test]
    fn strict_surface_requires_explicit_manifest() {
        let ctx = strict_ctx(EnabledFeatures::empty());
        let descriptor = OperationDescriptor::new(
            "scan-ports".to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            Vec::new(),
            Some("example.com".to_string()),
            Vec::new(),
            Vec::new(),
            false,
            true,
            Vec::new(),
        );
        let facts = TargetScope::for_host_without_addresses("example.com");
        // Default-empty scope denies for automated explicit-scope operations.
        let outcome = ctx.evaluate(&descriptor, Some(&facts));
        assert!(outcome.is_denied());
    }

    #[test]
    fn permissive_ctx_downgrades_safe_ambiguity_to_warn() {
        let ctx = permissive_ctx(EnabledFeatures::empty());
        // Private IP with no scope rules: default policy blocks, but
        // ManualPermissive downgrades the safe scope miss to Warn.
        let descriptor = OperationDescriptor::new(
            "scan-ports".to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            Vec::new(),
            Some("10.0.0.1".to_string()),
            Vec::new(),
            Vec::new(),
            false,
            false,
            Vec::new(),
        );
        let facts = TargetScope::for_ip("10.0.0.1".parse().unwrap());
        let outcome = ctx.evaluate(&descriptor, Some(&facts));
        assert!(matches!(outcome, EnforcementOutcome::Warn(_)));
    }
}
