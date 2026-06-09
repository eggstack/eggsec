use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ExecutionPolicy, IntendedUse, OperationDescriptor, OperationMode, OperationRisk, Scope};

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
        lines.push(format!("Policy Decision [{}]: {}", status, self.decision_id));
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
            lines.push(format!("  Resolved: {}", self.resolved_addresses.join(", ")));
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
                    decision
                        .warnings
                        .push(format!("scope check error: {}", e));
                }
            }
        } else if descriptor.requires_explicit_scope || descriptor.requires_private_or_local_target {
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
        decision
            .denied_reasons
            .push(format!(
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
                decision
                    .required_policy_flags
                    .push(flag.clone());
            }
            _ => {
                decision
                    .required_policy_flags
                    .push(flag.clone());
            }
        }
    }

    decision
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
        };
        let mut policy = ExecutionPolicy::default();
        policy.allow_exploit_adjacent = true;
        let decision = evaluate_operation_policy(&descriptor, &policy, None);
        assert!(decision.allowed);
        assert!(decision
            .required_features
            .iter()
            .any(|f| f == "packet-inspection"));
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
        };
        let policy = ExecutionPolicy::default();
        let decision = evaluate_operation_policy(&descriptor, &policy, Some(&scope));
        assert!(!decision.allowed);
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
}
