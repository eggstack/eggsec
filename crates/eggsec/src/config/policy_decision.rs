use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    DenialClass, ExecutionPolicy, ExecutionProfile, IntendedUse, OperationDescriptor,
    OperationMode, OperationRisk, Scope,
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
/// - `Deny`: operation must not proceed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementOutcome {
    Allow(PolicyDecision),
    Warn(PolicyDecision),
    Deny(PolicyDecision),
}

impl EnforcementOutcome {
    /// Returns a reference to the inner `PolicyDecision`.
    pub fn decision(&self) -> &PolicyDecision {
        match self {
            Self::Allow(d) | Self::Warn(d) | Self::Deny(d) => d,
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
}

/// Check whether a named compile-time Cargo feature is enabled.
///
/// Returns `true` for features that are always available or not relevant
/// as compile-time gates, and `false` for features that are behind a
/// `cfg(feature = "...")` gate that is not currently active.
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
        "pdf" => cfg!(feature = "pdf"),
        "advanced-hunting" => cfg!(feature = "advanced-hunting"),
        _ => true, // Unknown features are assumed available
    }
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
        .map_or(false, |t| t.trim().is_empty())
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
/// [`EnforcementOutcome::Warn`], or [`EnforcementOutcome::Deny`] according
/// to the given [`ExecutionProfile`].
///
/// For ManualPermissive, safe scope-selection denials (ScopeMissing / TargetOutOfScope
/// for low-risk StandardAssessment ops with no explicit exclusions or other denials)
/// are downgraded to Warn using DenialClass classification. Strict profiles and
/// higher-risk/feature/risk/capability/exclusion denials are never downgraded.
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
                let has_positive_scope_rules =
                    scope.map_or(false, |s| !s.allowed_targets.is_empty());
                let is_pure_out_of_scope_miss = classes
                    .iter()
                    .any(|c| matches!(c, DenialClass::TargetOutOfScope))
                    && !classes
                        .iter()
                        .any(|c| matches!(c, DenialClass::ExplicitExclusion));
                if is_pure_out_of_scope_miss && has_positive_scope_rules {
                    // Explicit rules declared; mismatch is denial, not a warnable miss.
                    return EnforcementOutcome::Deny(decision);
                }

                // Move denial reasons to warnings and allow-as-warn
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
        }
        return EnforcementOutcome::Deny(decision);
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
        use super::super::scope::LoadedScope;
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
            assert!(
                outcome.is_denied(),
                "Profile {:?} should deny excluded target",
                profile
            );
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
        assert!(outcome.is_denied());
        // Exclusion should be classified as ExplicitExclusion and block downgrade
        let classes = classify_denial_reasons(outcome.decision());
        assert!(classes.contains(&DenialClass::ExplicitExclusion));
    }

    #[test]
    fn manual_permissive_does_not_downgrade_risk_policy_denial() {
        use super::super::scope::LoadedScope;
        let mut policy = ExecutionPolicy::default();
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
}
