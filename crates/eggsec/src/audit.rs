use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{
    ConfirmationClass, EnforcementContext, EnforcementOutcome, ExecutionProfile, ExecutionSurface,
    LoadedScope, ManualOverride, OperationDescriptor, ScopeSource,
};

/// Normalized audit event for enforcement decisions across all execution surfaces.
///
/// Every meaningful policy decision produces a consistent audit record that
/// identifies the execution surface, profile, scope provenance, operation
/// metadata, decision outcome, confirmation classes, and whether any manual
/// override was accepted or ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementAuditEvent {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub surface: ExecutionSurface,
    pub profile: ExecutionProfile,
    pub operation_id: String,
    pub target: Option<String>,
    pub outcome: AuditOutcome,
    pub decision: crate::config::PolicyDecision,
    pub confirmation_classes: Vec<ConfirmationClass>,
    pub manual_override: Option<ManualOverrideAudit>,
    pub manual_override_ignored: bool,
    pub scope: ScopeAudit,
    pub policy_hash: Option<String>,
    pub metadata_id: Option<String>,
    pub correlation_id: Option<String>,
}

/// Simplified outcome for audit events (maps from `EnforcementOutcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuditOutcome {
    Allow,
    Warn,
    Confirmed,
    Deny,
    ConfirmationRequired,
}

impl AuditOutcome {
    /// Derive the audit outcome from an `EnforcementOutcome` and whether
    /// a manual override was applied.
    pub fn from_outcome(outcome: &EnforcementOutcome, confirmed: bool) -> Self {
        match outcome {
            EnforcementOutcome::Allow(_) => AuditOutcome::Allow,
            EnforcementOutcome::Warn(_) => AuditOutcome::Warn,
            EnforcementOutcome::RequireConfirmation(_) => {
                if confirmed {
                    AuditOutcome::Confirmed
                } else {
                    AuditOutcome::ConfirmationRequired
                }
            }
            EnforcementOutcome::Deny(_) => AuditOutcome::Deny,
        }
    }
}

/// Audit record for manual override details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualOverrideAudit {
    pub reason: Option<String>,
    pub classes: Vec<String>,
}

impl ManualOverrideAudit {
    /// Build from a `ManualOverride` and the confirmation classes that were required.
    pub fn from_override(mo: &ManualOverride, required_classes: &[ConfirmationClass]) -> Self {
        Self {
            reason: mo.reason.clone(),
            classes: required_classes
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
        }
    }
}

/// Scope provenance summary for audit events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeAudit {
    pub source: ScopeSource,
    pub path: Option<String>,
    pub allow_rule_count: usize,
    pub exclusion_rule_count: usize,
    pub explicit_manifest: bool,
}

impl ScopeAudit {
    /// Build from a `LoadedScope`.
    pub fn from_loaded_scope(loaded: &LoadedScope) -> Self {
        Self {
            source: loaded.source.clone(),
            path: loaded.path.clone(),
            allow_rule_count: loaded.scope.allowed_targets.len(),
            exclusion_rule_count: loaded.scope.excluded_targets.len(),
            explicit_manifest: loaded.is_explicit_manifest(),
        }
    }
}

/// Build an audit event from an enforcement outcome (dispatch or preflight).
///
/// This is the primary builder for recording enforcement decisions at the
/// point of evaluation, whether from preflight, CLI, TUI, REST, MCP, or agent.
pub fn audit_event_from_enforcement_outcome(
    surface: ExecutionSurface,
    enforcement: &EnforcementContext,
    descriptor: &OperationDescriptor,
    outcome: &EnforcementOutcome,
    confirmed: bool,
    override_ignored: bool,
    manual_override: Option<&ManualOverride>,
    required_classes: &[ConfirmationClass],
    correlation_id: Option<&str>,
    metadata_id: Option<&str>,
) -> EnforcementAuditEvent {
    let decision = outcome.decision().clone();
    let outcome_kind = AuditOutcome::from_outcome(outcome, confirmed);

    let manual_override_audit = if confirmed {
        manual_override.map(|mo| ManualOverrideAudit::from_override(mo, required_classes))
    } else {
        None
    };

    EnforcementAuditEvent {
        event_id: Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        surface,
        profile: enforcement.execution_profile,
        operation_id: descriptor.operation.clone(),
        target: descriptor.target.clone(),
        outcome: outcome_kind,
        decision,
        confirmation_classes: required_classes.to_vec(),
        manual_override: manual_override_audit,
        manual_override_ignored: override_ignored,
        scope: ScopeAudit::from_loaded_scope(&enforcement.loaded_scope),
        policy_hash: Some(enforcement.policy_hash()),
        metadata_id: metadata_id.map(String::from),
        correlation_id: correlation_id.map(String::from),
    }
}

/// Build an audit event from a preflight result.
///
/// Use this for preflight advisory evaluations that do not result in dispatch.
pub fn audit_event_from_preflight(
    surface: ExecutionSurface,
    enforcement: &EnforcementContext,
    descriptor: &OperationDescriptor,
    outcome: &EnforcementOutcome,
    manual_override: Option<&ManualOverride>,
    required_classes: &[ConfirmationClass],
    correlation_id: Option<&str>,
) -> EnforcementAuditEvent {
    audit_event_from_enforcement_outcome(
        surface,
        enforcement,
        descriptor,
        outcome,
        false, // preflight never confirms
        false, // no override ignored at preflight
        manual_override,
        required_classes,
        correlation_id,
        None,
    )
}

/// Emit the audit event at the appropriate tracing level.
///
/// - `Allow` / `Warn` / `Confirmed`: info level
/// - `ConfirmationRequired` / `Deny`: warn level
pub fn emit_audit_event(event: &EnforcementAuditEvent) {
    let outcome_str = serde_json::to_string(&event.outcome).unwrap_or_default();
    let decision_id = &event.decision.decision_id;
    let operation = &event.operation_id;
    let surface = event.surface;
    let profile = event.profile;

    match event.outcome {
        AuditOutcome::Allow | AuditOutcome::Warn | AuditOutcome::Confirmed => {
            tracing::info!(
                event_id = %event.event_id,
                decision_id = %decision_id,
                outcome = %outcome_str,
                operation = %operation,
                surface = %surface,
                profile = ?profile,
                target = ?event.target,
                scope_source = ?event.scope.source,
                manual_override_ignored = event.manual_override_ignored,
                "enforcement audit"
            );
        }
        AuditOutcome::ConfirmationRequired | AuditOutcome::Deny => {
            tracing::warn!(
                event_id = %event.event_id,
                decision_id = %decision_id,
                outcome = %outcome_str,
                operation = %operation,
                surface = %surface,
                profile = ?profile,
                target = ?event.target,
                scope_source = ?event.scope.source,
                manual_override_ignored = event.manual_override_ignored,
                "enforcement audit"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ConfirmationClass, EnforcementContext, EnforcementOutcome, ExecutionPolicy,
        ExecutionSurface, IntendedUse, LoadedScope, ManualOverride, OperationDescriptor,
        OperationMode, OperationRisk, Scope,
    };

    fn test_enforcement() -> EnforcementContext {
        let scope = LoadedScope::default_empty();
        EnforcementContext::manual_permissive(ExecutionPolicy::default(), scope)
    }

    fn test_descriptor(target: Option<&str>) -> OperationDescriptor {
        OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: target.map(String::from),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        }
    }

    #[test]
    fn scope_audit_counts_rules() {
        let scope = Scope {
            allowed_targets: vec![
                crate::config::ScopeRule::new("10.0.0.0/8".to_string()),
                crate::config::ScopeRule::new("192.168.0.0/16".to_string()),
            ],
            excluded_targets: vec![crate::config::ScopeRule::new("10.0.0.5".to_string())],
            ..Default::default()
        };
        let loaded = LoadedScope::explicit(
            scope,
            crate::config::ScopeSource::ConfigFile,
            Some("/path/to/scope.toml".to_string()),
        );
        let audit = ScopeAudit::from_loaded_scope(&loaded);
        assert_eq!(audit.allow_rule_count, 2);
        assert_eq!(audit.exclusion_rule_count, 1);
        assert!(audit.explicit_manifest);
        assert_eq!(audit.source, ScopeSource::ConfigFile);
        assert_eq!(audit.path.as_deref(), Some("/path/to/scope.toml"));
    }

    #[test]
    fn scope_audit_default_empty() {
        let loaded = LoadedScope::default_empty();
        let audit = ScopeAudit::from_loaded_scope(&loaded);
        assert_eq!(audit.allow_rule_count, 0);
        assert_eq!(audit.exclusion_rule_count, 0);
        assert!(!audit.explicit_manifest);
        assert_eq!(audit.source, ScopeSource::DefaultEmpty);
    }

    #[test]
    fn audit_event_from_allow_outcome() {
        let enforcement = test_enforcement();
        let desc = test_descriptor(Some("127.0.0.1"));
        let outcome = EnforcementOutcome::Allow(crate::config::PolicyDecision::allowed(
            "scan-ports",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
        ));
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::CliManual,
            &enforcement,
            &desc,
            &outcome,
            false,
            false,
            None,
            &[],
            None,
            None,
        );
        assert_eq!(event.outcome, AuditOutcome::Allow);
        assert_eq!(event.surface, ExecutionSurface::CliManual);
        assert_eq!(event.operation_id, "scan-ports");
        assert_eq!(event.target.as_deref(), Some("127.0.0.1"));
        assert!(!event.manual_override_ignored);
        assert!(event.manual_override.is_none());
        assert!(!event.scope.explicit_manifest); // default_empty is not explicit
    }

    #[test]
    fn audit_event_from_deny_outcome() {
        let enforcement = test_enforcement();
        let desc = test_descriptor(Some("93.184.216.34"));
        let decision = crate::config::PolicyDecision::denied(
            "scan-ports",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
            "target not in scope",
        );
        let outcome = EnforcementOutcome::Deny(decision);
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::McpServer,
            &enforcement,
            &desc,
            &outcome,
            false,
            false,
            None,
            &[],
            Some("req-123"),
            None,
        );
        assert_eq!(event.outcome, AuditOutcome::Deny);
        assert_eq!(event.surface, ExecutionSurface::McpServer);
        assert_eq!(event.correlation_id.as_deref(), Some("req-123"));
    }

    #[test]
    fn audit_event_with_confirmed_override() {
        let enforcement = test_enforcement();
        let desc = test_descriptor(Some("93.184.216.34"));
        let outcome =
            EnforcementOutcome::RequireConfirmation(crate::config::PolicyDecision::allowed(
                "scan-ports",
                OperationMode::StandardAssessment,
                OperationRisk::SafeActive,
                vec![IntendedUse::WebAssessment],
            ));
        let mo = ManualOverride {
            allow_out_of_scope: true,
            reason: Some("testing".to_string()),
            ..Default::default()
        };
        let classes = vec![ConfirmationClass::OutOfScope];
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::CliManual,
            &enforcement,
            &desc,
            &outcome,
            true,  // confirmed
            false, // not ignored
            Some(&mo),
            &classes,
            None,
            None,
        );
        assert_eq!(event.outcome, AuditOutcome::Confirmed);
        assert!(event.manual_override.is_some());
        let mo_audit = event.manual_override.unwrap();
        assert_eq!(mo_audit.reason.as_deref(), Some("testing"));
        assert_eq!(mo_audit.classes, vec!["out-of-scope"]);
    }

    #[test]
    fn audit_event_with_ignored_override() {
        let enforcement = test_enforcement();
        let desc = test_descriptor(Some("93.184.216.34"));
        let decision = crate::config::PolicyDecision::denied(
            "scan-ports",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
            "target not in scope",
        );
        let outcome = EnforcementOutcome::Deny(decision);
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::McpServer,
            &enforcement,
            &desc,
            &outcome,
            false,
            true, // override ignored (automated surface)
            None,
            &[],
            None,
            None,
        );
        assert!(event.manual_override_ignored);
        assert!(event.manual_override.is_none());
    }

    #[test]
    fn audit_event_serializes_roundtrip() {
        let enforcement = test_enforcement();
        let desc = test_descriptor(Some("127.0.0.1"));
        let outcome = EnforcementOutcome::Allow(crate::config::PolicyDecision::allowed(
            "scan-ports",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
        ));
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::CliManual,
            &enforcement,
            &desc,
            &outcome,
            false,
            false,
            None,
            &[],
            None,
            None,
        );
        let json = serde_json::to_string(&event).unwrap();
        let parsed: EnforcementAuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_id, event.event_id);
        assert_eq!(parsed.outcome, AuditOutcome::Allow);
        assert_eq!(parsed.surface, ExecutionSurface::CliManual);
    }

    #[test]
    fn audit_outcome_from_outcome_allow() {
        let outcome = EnforcementOutcome::Allow(crate::config::PolicyDecision::allowed(
            "test",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
        ));
        assert_eq!(
            AuditOutcome::from_outcome(&outcome, false),
            AuditOutcome::Allow
        );
    }

    #[test]
    fn audit_outcome_from_outcome_warn() {
        let outcome = EnforcementOutcome::Warn(crate::config::PolicyDecision::allowed(
            "test",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
        ));
        assert_eq!(
            AuditOutcome::from_outcome(&outcome, false),
            AuditOutcome::Warn
        );
    }

    #[test]
    fn audit_outcome_from_outcome_require_confirmation_not_confirmed() {
        let outcome =
            EnforcementOutcome::RequireConfirmation(crate::config::PolicyDecision::allowed(
                "test",
                OperationMode::StandardAssessment,
                OperationRisk::SafeActive,
                vec![IntendedUse::WebAssessment],
            ));
        assert_eq!(
            AuditOutcome::from_outcome(&outcome, false),
            AuditOutcome::ConfirmationRequired
        );
    }

    #[test]
    fn audit_outcome_from_outcome_require_confirmation_confirmed() {
        let outcome =
            EnforcementOutcome::RequireConfirmation(crate::config::PolicyDecision::allowed(
                "test",
                OperationMode::StandardAssessment,
                OperationRisk::SafeActive,
                vec![IntendedUse::WebAssessment],
            ));
        assert_eq!(
            AuditOutcome::from_outcome(&outcome, true),
            AuditOutcome::Confirmed
        );
    }

    #[test]
    fn audit_outcome_from_outcome_deny() {
        let outcome = EnforcementOutcome::Deny(crate::config::PolicyDecision::denied(
            "test",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
            "denied",
        ));
        assert_eq!(
            AuditOutcome::from_outcome(&outcome, false),
            AuditOutcome::Deny
        );
    }

    #[test]
    fn manual_override_audit_from_override() {
        let mo = ManualOverride {
            allow_out_of_scope: true,
            allow_high_risk: true,
            reason: Some("reason".to_string()),
            ..Default::default()
        };
        let classes = vec![ConfirmationClass::OutOfScope, ConfirmationClass::HighRisk];
        let audit = ManualOverrideAudit::from_override(&mo, &classes);
        assert_eq!(audit.reason.as_deref(), Some("reason"));
        assert_eq!(audit.classes, vec!["out-of-scope", "high-risk"]);
    }

    #[test]
    fn policy_hash_is_stable() {
        let enforcement = test_enforcement();
        let h1 = enforcement.policy_hash();
        let h2 = enforcement.policy_hash();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn policy_hash_differs_for_different_policies() {
        let scope = LoadedScope::default_empty();
        let e1 = EnforcementContext::manual_permissive(ExecutionPolicy::default(), scope.clone());
        let mut policy2 = ExecutionPolicy::default();
        policy2.allow_intrusive_fuzzing = true;
        let e2 = EnforcementContext::manual_permissive(policy2, scope);
        assert_ne!(e1.policy_hash(), e2.policy_hash());
    }

    #[test]
    fn rest_deny_outcome_produces_audit_event() {
        let enforcement = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let desc = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("10.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let decision = crate::config::PolicyDecision::denied(
            "scan-ports",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
            "target not in scope",
        );
        let outcome = EnforcementOutcome::Deny(decision);
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::RestApi,
            &enforcement,
            &desc,
            &outcome,
            false,
            false,
            None,
            &[],
            Some("req-abc-123"),
            None,
        );
        assert_eq!(event.outcome, AuditOutcome::Deny);
        assert_eq!(event.surface, ExecutionSurface::RestApi);
        assert_eq!(event.correlation_id.as_deref(), Some("req-abc-123"));
        assert!(!event.manual_override_ignored);
        assert!(event.policy_hash.is_some());
        assert_eq!(event.policy_hash.as_ref().unwrap().len(), 64);
    }

    #[test]
    fn tui_confirm_outcome_produces_audit_event() {
        let enforcement = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let desc = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("https://example.com".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let outcome =
            EnforcementOutcome::RequireConfirmation(crate::config::PolicyDecision::allowed(
                "fuzz",
                OperationMode::StandardAssessment,
                OperationRisk::Intrusive,
                vec![IntendedUse::WebAssessment],
            ));
        let mo = ManualOverride {
            allow_out_of_scope: true,
            reason: Some("authorized testing".to_string()),
            ..Default::default()
        };
        let classes = vec![ConfirmationClass::HighRisk];
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::TuiManual,
            &enforcement,
            &desc,
            &outcome,
            true,
            false,
            Some(&mo),
            &classes,
            None,
            None,
        );
        assert_eq!(event.outcome, AuditOutcome::Confirmed);
        assert_eq!(event.surface, ExecutionSurface::TuiManual);
        assert!(event.manual_override.is_some());
        let mo_audit = event.manual_override.unwrap();
        assert_eq!(mo_audit.reason.as_deref(), Some("authorized testing"));
        assert_eq!(mo_audit.classes, vec!["high-risk"]);
        assert!(!event.manual_override_ignored);
    }

    #[test]
    fn agent_denied_scan_produces_audit_event() {
        let enforcement = EnforcementContext::agent_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let desc = OperationDescriptor {
            operation: "stress-test".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::StressTest,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("https://target.com".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let decision = crate::config::PolicyDecision::denied(
            "stress-test",
            OperationMode::StandardAssessment,
            OperationRisk::StressTest,
            vec![IntendedUse::WebAssessment],
            "stress testing not allowed in agent mode",
        );
        let outcome = EnforcementOutcome::Deny(decision);
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::SecurityAgent,
            &enforcement,
            &desc,
            &outcome,
            false,
            false,
            None,
            &[],
            None,
            None,
        );
        assert_eq!(event.outcome, AuditOutcome::Deny);
        assert_eq!(event.surface, ExecutionSurface::SecurityAgent);
        assert_eq!(event.profile, ExecutionProfile::AgentStrict);
        assert!(event.manual_override.is_none());
        assert!(!event.manual_override_ignored);
        assert!(event.policy_hash.is_some());
    }

    #[test]
    fn preflight_event_never_marks_confirmed() {
        let enforcement = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let desc = test_descriptor(Some("93.184.216.34"));
        let outcome =
            EnforcementOutcome::RequireConfirmation(crate::config::PolicyDecision::allowed(
                "scan-ports",
                OperationMode::StandardAssessment,
                OperationRisk::SafeActive,
                vec![IntendedUse::WebAssessment],
            ));
        let event = audit_event_from_preflight(
            ExecutionSurface::CliManual,
            &enforcement,
            &desc,
            &outcome,
            None,
            &[ConfirmationClass::OutOfScope],
            None,
        );
        assert_eq!(event.outcome, AuditOutcome::ConfirmationRequired);
        assert!(event.manual_override.is_none());
    }

    #[test]
    fn emit_audit_event_does_not_panic() {
        let enforcement = test_enforcement();
        let desc = test_descriptor(Some("127.0.0.1"));
        let outcome = EnforcementOutcome::Allow(crate::config::PolicyDecision::allowed(
            "scan-ports",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
        ));
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::RestApi,
            &enforcement,
            &desc,
            &outcome,
            false,
            false,
            None,
            &[],
            None,
            None,
        );
        emit_audit_event(&event);
    }

    #[test]
    fn emit_deny_event_does_not_panic() {
        let enforcement = test_enforcement();
        let desc = test_descriptor(Some("93.184.216.34"));
        let decision = crate::config::PolicyDecision::denied(
            "scan-ports",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
            "denied",
        );
        let outcome = EnforcementOutcome::Deny(decision);
        let event = audit_event_from_enforcement_outcome(
            ExecutionSurface::McpServer,
            &enforcement,
            &desc,
            &outcome,
            false,
            false,
            None,
            &[],
            None,
            None,
        );
        emit_audit_event(&event);
    }
}
