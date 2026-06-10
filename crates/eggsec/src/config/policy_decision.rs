use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    ExecutionPolicy, ExecutionProfile, IntendedUse, OperationDescriptor, OperationMode,
    OperationRisk, Scope,
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
                    decision
                        .denied_reasons
                        .push("target not in scope".to_string());
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

/// Evaluate an operation with profile-aware enforcement semantics.
///
/// Calls [`evaluate_operation_policy`] internally, then transforms the
/// resulting [`PolicyDecision`] into [`EnforcementOutcome::Allow`],
/// [`EnforcementOutcome::Warn`], or [`EnforcementOutcome::Deny`] according
/// to the given [`ExecutionProfile`].
pub fn evaluate_enforcement(
    descriptor: &OperationDescriptor,
    policy: &ExecutionPolicy,
    scope: Option<&Scope>,
    profile: ExecutionProfile,
) -> EnforcementOutcome {
    let decision = evaluate_operation_policy(descriptor, policy, scope);

    if !decision.allowed {
        return EnforcementOutcome::Deny(decision);
    }

    // Check capability requirements against profile
    if !decision.required_features.is_empty() || !descriptor.required_capabilities.is_empty() {
        // For strict profiles, missing capabilities deny
        if profile.is_strict() {
            // Check denied capabilities
            for cap in &descriptor.required_capabilities {
                if policy.denied_capabilities.contains(cap) {
                    let mut d = decision.clone();
                    d.denied_reasons.push(format!(
                        "capability '{}' is denied by execution policy",
                        cap
                    ));
                    d.allowed = false;
                    return EnforcementOutcome::Deny(d);
                }
            }
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
                warnings.push("target scope is ambiguous; consider using --strict-scope".to_string());
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
        ExecutionProfile::CiStrict | ExecutionProfile::McpStrict | ExecutionProfile::AgentStrict => {
            // Strict profiles: missing scope for networked operations denies
            if descriptor.requires_explicit_scope && scope.is_none() {
                let mut d = decision;
                d.denied_reasons.push(format!(
                    "scope file required in {} mode",
                    profile
                ));
                d.allowed = false;
                return EnforcementOutcome::Deny(d);
            }
            // Strict profiles: scope ambiguity denies
            if decision.target_original.is_some()
                && decision.matched_scope_rules.is_empty()
                && decision.denied_reasons.is_empty()
            {
                let mut d = decision;
                d.denied_reasons.push(format!(
                    "target scope is ambiguous in {} mode",
                    profile
                ));
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
        let outcome =
            evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::ManualPermissive);
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
        let outcome =
            evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::McpStrict);
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
        let outcome =
            evaluate_enforcement(&descriptor, &policy, None, ExecutionProfile::McpStrict);
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(json.contains("\"deny\"") || json.contains("\"Deny\""));
    }
}
