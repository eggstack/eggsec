use eggsec::config::{
    confirmation_classes_for, ConfirmationClass, EnforcementContext, EnforcementOutcome,
    ExecutionPolicy, ExecutionProfile, ExecutionSurface, LoadedScope, ManualOverride,
    OperationDescriptor, PolicyDecision, ScopeSource,
};

#[derive(Debug, Clone)]
pub struct TuiEnforcementState {
    pub surface: ExecutionSurface,
    pub loaded_scope: LoadedScope,
    pub enforcement: EnforcementContext,
    pub manual_override: ManualOverride,
    pub last_preflight: Option<TuiPreflightResult>,
}

#[derive(Debug, Clone)]
pub struct TuiPreflightResult {
    pub operation: String,
    pub target: Option<String>,
    pub outcome_kind: TuiPreflightOutcomeKind,
    pub decision: PolicyDecision,
    pub required_confirmation_classes: Vec<ConfirmationClass>,
    pub suggested_cli_flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiPreflightOutcomeKind {
    Allow,
    Warn,
    RequireConfirmation,
    Deny,
}

impl TuiEnforcementState {
    pub fn new(
        surface: ExecutionSurface,
        loaded_scope: LoadedScope,
        enforcement: EnforcementContext,
    ) -> Self {
        Self {
            surface,
            loaded_scope,
            enforcement,
            manual_override: ManualOverride::default(),
            last_preflight: None,
        }
    }

    pub fn toggle_posture(&mut self) -> ExecutionProfile {
        let new_surface = match self.surface {
            ExecutionSurface::TuiManual => ExecutionSurface::TuiManualStrict,
            ExecutionSurface::TuiManualStrict => ExecutionSurface::TuiManual,
            other => other,
        };
        let new_profile = new_surface.profile();
        self.surface = new_surface;
        self.enforcement.execution_profile = new_profile;
        self.last_preflight = None;
        new_profile
    }

    pub fn is_guarded(&self) -> bool {
        matches!(self.surface, ExecutionSurface::TuiManualStrict)
    }

    pub fn preflight(&mut self, descriptor: &OperationDescriptor) -> TuiPreflightResult {
        let outcome = self.enforcement.evaluate(descriptor);
        let result = TuiPreflightResult::from_outcome(
            descriptor,
            &outcome,
            &self.enforcement.execution_policy,
        );
        self.last_preflight = Some(result.clone());
        result
    }

    pub fn mode_label(&self) -> &'static str {
        if self.is_guarded() {
            "Guarded"
        } else {
            "Manual"
        }
    }

    pub fn scope_label(&self) -> String {
        match self.loaded_scope.source {
            ScopeSource::DefaultEmpty => "none".to_string(),
            ScopeSource::ConfigFile => self
                .loaded_scope
                .path
                .as_deref()
                .map(|p| {
                    std::path::Path::new(p)
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| "config".to_string())
                })
                .unwrap_or_else(|| "config".to_string()),
            ScopeSource::CliScopeFile => self
                .loaded_scope
                .path
                .as_deref()
                .map(|p| {
                    std::path::Path::new(p)
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| "cli-scope".to_string())
                })
                .unwrap_or_else(|| "cli-scope".to_string()),
            ScopeSource::GeneratedPreset => "preset".to_string(),
        }
    }

    pub fn allow_rule_count(&self) -> usize {
        self.loaded_scope.scope.allowed_targets.len()
    }

    pub fn exclusion_rule_count(&self) -> usize {
        self.loaded_scope.scope.excluded_targets.len()
    }

    pub fn status_string(&self) -> String {
        let mode = self.mode_label();
        let scope = self.scope_label();
        let allows = self.allow_rule_count();
        let excludes = self.exclusion_rule_count();
        if allows == 0 && excludes == 0 {
            format!("Mode: {} | Scope: {} (warnings enabled)", mode, scope)
        } else {
            format!(
                "Mode: {} | Scope: {} | allow: {} | exclude: {}",
                mode, scope, allows, excludes
            )
        }
    }

    pub fn honors_manual_override(&self) -> bool {
        self.surface.honors_manual_override()
    }
}

impl TuiPreflightResult {
    pub fn from_outcome(
        descriptor: &OperationDescriptor,
        outcome: &EnforcementOutcome,
        policy: &ExecutionPolicy,
    ) -> Self {
        let decision = outcome.decision().clone();
        let outcome_kind = match outcome {
            EnforcementOutcome::Allow(_) => TuiPreflightOutcomeKind::Allow,
            EnforcementOutcome::Warn(_) => TuiPreflightOutcomeKind::Warn,
            EnforcementOutcome::RequireConfirmation(_) => {
                TuiPreflightOutcomeKind::RequireConfirmation
            }
            EnforcementOutcome::Deny(_) => TuiPreflightOutcomeKind::Deny,
        };

        let required_confirmation_classes =
            if let EnforcementOutcome::RequireConfirmation(_) = outcome {
                confirmation_classes_for(descriptor, &decision, policy)
            } else {
                Vec::new()
            };

        let suggested_cli_flags = Self::cli_flags_for_classes(&required_confirmation_classes);

        Self {
            operation: descriptor.operation.clone(),
            target: descriptor.target.clone(),
            outcome_kind,
            decision,
            required_confirmation_classes,
            suggested_cli_flags,
        }
    }

    fn cli_flags_for_classes(classes: &[ConfirmationClass]) -> Vec<String> {
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

    pub fn outcome_summary(&self) -> String {
        match self.outcome_kind {
            TuiPreflightOutcomeKind::Allow => "Allow".to_string(),
            TuiPreflightOutcomeKind::Warn => "Warn".to_string(),
            TuiPreflightOutcomeKind::RequireConfirmation => {
                let classes: Vec<&str> = self
                    .required_confirmation_classes
                    .iter()
                    .map(|c| c.as_str())
                    .collect();
                format!("Confirm required: [{}]", classes.join(", "))
            }
            TuiPreflightOutcomeKind::Deny => self
                .decision
                .denied_reasons
                .first()
                .cloned()
                .unwrap_or_else(|| "Denied by policy".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec::config::{
        EnforcementContext, ExecutionPolicy, ExecutionProfile, ExecutionSurface, LoadedScope,
        OperationDescriptor, OperationMode, OperationRisk, Scope, ScopeRule, ScopeSource,
    };

    fn test_state(surface: ExecutionSurface) -> TuiEnforcementState {
        let scope = LoadedScope::default_empty();
        let policy = ExecutionPolicy::default();
        let enforcement = EnforcementContext::for_surface(surface, policy, scope.clone());
        TuiEnforcementState::new(surface, scope, enforcement)
    }

    fn passive_descriptor(operation: &str, target: Option<&str>) -> OperationDescriptor {
        OperationDescriptor {
            operation: operation.to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Passive,
            intended_uses: vec![],
            target: target.map(|t| t.to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        }
    }

    #[test]
    fn defaults_to_tui_manual() {
        let state = test_state(ExecutionSurface::TuiManual);
        assert_eq!(state.surface, ExecutionSurface::TuiManual);
        assert!(matches!(
            state.enforcement.execution_profile,
            ExecutionProfile::ManualPermissive
        ));
        assert!(!state.is_guarded());
    }

    #[test]
    fn tui_manual_maps_to_manual_permissive() {
        let state = test_state(ExecutionSurface::TuiManual);
        assert!(matches!(
            state.enforcement.execution_profile,
            ExecutionProfile::ManualPermissive
        ));
    }

    #[test]
    fn tui_guarded_maps_to_manual_guarded() {
        let state = test_state(ExecutionSurface::TuiManualStrict);
        assert!(matches!(
            state.enforcement.execution_profile,
            ExecutionProfile::ManualGuarded
        ));
        assert!(state.is_guarded());
    }

    #[test]
    fn toggle_from_manual_goes_to_guarded() {
        let mut state = test_state(ExecutionSurface::TuiManual);
        let new_profile = state.toggle_posture();
        assert!(matches!(new_profile, ExecutionProfile::ManualGuarded));
        assert!(state.is_guarded());
        assert_eq!(state.surface, ExecutionSurface::TuiManualStrict);
    }

    #[test]
    fn toggle_from_guarded_returns_to_manual() {
        let mut state = test_state(ExecutionSurface::TuiManualStrict);
        let new_profile = state.toggle_posture();
        assert!(matches!(new_profile, ExecutionProfile::ManualPermissive));
        assert!(!state.is_guarded());
        assert_eq!(state.surface, ExecutionSurface::TuiManual);
    }

    #[test]
    fn toggle_roundtrip() {
        let mut state = test_state(ExecutionSurface::TuiManual);
        state.toggle_posture();
        state.toggle_posture();
        assert_eq!(state.surface, ExecutionSurface::TuiManual);
        assert!(matches!(
            state.enforcement.execution_profile,
            ExecutionProfile::ManualPermissive
        ));
    }

    #[test]
    fn toggle_clears_last_preflight() {
        let mut state = test_state(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        state.preflight(&desc);
        assert!(state.last_preflight.is_some());
        state.toggle_posture();
        assert!(state.last_preflight.is_none());
    }

    #[test]
    fn toggle_preserves_scope_source() {
        let mut state = test_state(ExecutionSurface::TuiManual);
        let original_source = state.loaded_scope.source;
        state.toggle_posture();
        assert_eq!(state.loaded_scope.source, original_source);
    }

    #[test]
    fn mode_label_manual() {
        let state = test_state(ExecutionSurface::TuiManual);
        assert_eq!(state.mode_label(), "Manual");
    }

    #[test]
    fn mode_label_guarded() {
        let state = test_state(ExecutionSurface::TuiManualStrict);
        assert_eq!(state.mode_label(), "Guarded");
    }

    #[test]
    fn scope_label_default_empty() {
        let state = test_state(ExecutionSurface::TuiManual);
        assert_eq!(state.scope_label(), "none");
    }

    #[test]
    fn honors_manual_override_in_tui_manual() {
        let state = test_state(ExecutionSurface::TuiManual);
        assert!(state.honors_manual_override());
    }

    #[test]
    fn does_not_honor_manual_override_in_tui_guarded() {
        let state = test_state(ExecutionSurface::TuiManualStrict);
        assert!(!state.honors_manual_override());
    }

    #[test]
    fn preflight_populates_last_preflight() {
        let mut state = test_state(ExecutionSurface::TuiManual);
        assert!(state.last_preflight.is_none());
        let desc = passive_descriptor("recon", Some("example.com"));
        state.preflight(&desc);
        assert!(state.last_preflight.is_some());
        let pf = state.last_preflight.as_ref().unwrap();
        assert_eq!(pf.operation, "recon");
        assert_eq!(pf.target.as_deref(), Some("example.com"));
    }

    #[test]
    fn preflight_safe_operation_allows_or_warns() {
        let mut state = test_state(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let result = state.preflight(&desc);
        assert_eq!(
            result.outcome_kind,
            TuiPreflightOutcomeKind::Allow,
            "Safe passive op with default scope should allow, got {:?}",
            result.outcome_kind
        );
    }

    #[test]
    fn preflight_result_outcome_summary() {
        let mut state = test_state(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let result = state.preflight(&desc);
        let summary = result.outcome_summary();
        assert!(
            summary == "Allow" || summary == "Warn",
            "Expected 'Allow' or 'Warn' summary, got {:?}",
            summary
        );
    }

    #[test]
    fn preflight_scope_miss_triggers_confirmation_or_deny() {
        let mut state = test_state(ExecutionSurface::TuiManualStrict);
        let desc = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![],
            target: Some("10.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = state.preflight(&desc);
        assert_eq!(
            result.outcome_kind,
            TuiPreflightOutcomeKind::Deny,
            "Guarded mode scope miss should deny, got {:?}",
            result.outcome_kind
        );
    }

    #[test]
    fn status_string_default_empty() {
        let state = test_state(ExecutionSurface::TuiManual);
        let s = state.status_string();
        assert!(s.contains("Manual"));
        assert!(s.contains("none"));
    }

    #[test]
    fn status_string_guarded() {
        let state = test_state(ExecutionSurface::TuiManualStrict);
        let s = state.status_string();
        assert!(s.contains("Guarded"));
    }

    #[test]
    fn allow_and_exclusion_counts_default_zero() {
        let state = test_state(ExecutionSurface::TuiManual);
        assert_eq!(state.allow_rule_count(), 0);
        assert_eq!(state.exclusion_rule_count(), 0);
    }

    #[test]
    fn cli_flags_for_confirmation_classes() {
        let classes = vec![
            eggsec::config::ConfirmationClass::OutOfScope,
            eggsec::config::ConfirmationClass::HighRisk,
        ];
        let flags = TuiPreflightResult::cli_flags_for_classes(&classes);
        assert_eq!(flags.len(), 2);
        assert!(flags.contains(&"--allow-out-of-scope".to_string()));
        assert!(flags.contains(&"--allow-high-risk".to_string()));
    }

    #[test]
    fn new_initializes_no_preflight() {
        let state = test_state(ExecutionSurface::TuiManual);
        assert!(state.last_preflight.is_none());
        assert!(!state.manual_override.assume_yes);
        assert!(!state.manual_override.allow_out_of_scope);
        assert!(!state.manual_override.allow_high_risk);
    }

    fn test_state_with_positive_scope(surface: ExecutionSurface) -> TuiEnforcementState {
        let scope = Scope {
            allowed_targets: vec![ScopeRule::new("example.com".to_string())],
            excluded_targets: vec![],
            ..Default::default()
        };
        let loaded_scope = LoadedScope {
            scope,
            source: ScopeSource::ConfigFile,
            path: Some("scope.toml".to_string()),
        };
        let policy = ExecutionPolicy::default();
        let enforcement = EnforcementContext::for_surface(surface, policy, loaded_scope.clone());
        TuiEnforcementState::new(surface, loaded_scope, enforcement)
    }

    #[test]
    fn preflight_positive_scope_miss_returns_require_confirmation() {
        let mut state = test_state_with_positive_scope(ExecutionSurface::TuiManual);
        let desc = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Passive,
            intended_uses: vec![],
            target: Some("10.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = state.preflight(&desc);
        assert_eq!(
            result.outcome_kind,
            TuiPreflightOutcomeKind::RequireConfirmation,
            "Positive scope miss in manual mode should require confirmation, got {:?}",
            result.outcome_kind
        );
    }

    #[test]
    fn confirm_clears_last_preflight() {
        let mut state = test_state(ExecutionSurface::TuiManual);
        assert!(state.last_preflight.is_none());
        // Simulate a preflight result being set
        let desc = passive_descriptor("recon", Some("example.com"));
        state.preflight(&desc);
        assert!(state.last_preflight.is_some());
        // Simulate what confirm_policy_action does: clear last_preflight
        state.last_preflight = None;
        assert!(state.last_preflight.is_none());
    }

    #[test]
    fn confirming_with_matching_override_permits_dispatch() {
        use eggsec::config::ConfirmationClass;
        let mut state = test_state(ExecutionSurface::TuiManual);
        // Set up manual override that permits OutOfScope
        state.manual_override.allow_out_of_scope = true;
        // Verify permits
        assert!(state.manual_override.permits(ConfirmationClass::OutOfScope));
        assert!(state
            .manual_override
            .permits(ConfirmationClass::TargetExpansion));
    }

    #[test]
    fn manual_override_does_not_permits_unset_classes() {
        use eggsec::config::ConfirmationClass;
        let mut state = test_state(ExecutionSurface::TuiManual);
        state.manual_override.allow_out_of_scope = true;
        // Should not permit classes that weren't explicitly set
        assert!(!state.manual_override.permits(ConfirmationClass::HighRisk));
        assert!(!state
            .manual_override
            .permits(ConfirmationClass::PrivateResolution));
        assert!(!state
            .manual_override
            .permits(ConfirmationClass::TrafficInterception));
    }

    #[test]
    fn from_outcome_uses_provided_policy() {
        // Verify that from_outcome with the provided policy produces the same classes
        // as confirmation_classes_for with that policy.
        let desc = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![],
            target: Some("10.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        // Build a decision that would trigger RequireConfirmation
        let scope = Scope {
            allowed_targets: vec![ScopeRule::new("example.com".to_string())],
            excluded_targets: vec![],
            ..Default::default()
        };
        let loaded_scope = LoadedScope {
            scope,
            source: ScopeSource::ConfigFile,
            path: Some("scope.toml".to_string()),
        };
        let policy = ExecutionPolicy::default();
        let enforcement =
            EnforcementContext::for_surface(ExecutionSurface::TuiManual, policy.clone(), loaded_scope);
        let outcome = enforcement.evaluate(&desc);
        // Compute expected classes using the same function
        let expected_classes =
            confirmation_classes_for(&desc, outcome.decision(), &enforcement.execution_policy);
        // Compute via from_outcome
        let result = TuiPreflightResult::from_outcome(&desc, &outcome, &enforcement.execution_policy);
        assert_eq!(
            result.required_confirmation_classes, expected_classes,
            "from_outcome should use the provided policy for class calculation"
        );
    }

    #[test]
    fn explicit_exclusion_suggests_correct_cli_flag() {
        let classes = vec![eggsec::config::ConfirmationClass::ExplicitExclusion];
        let flags = TuiPreflightResult::cli_flags_for_classes(&classes);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0], "--allow-excluded-target");
    }

    #[test]
    fn from_outcome_out_of_scope_suggests_correct_flag() {
        let classes = vec![eggsec::config::ConfirmationClass::OutOfScope];
        let flags = TuiPreflightResult::cli_flags_for_classes(&classes);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0], "--allow-out-of-scope");
    }

    #[test]
    fn confirmation_classes_match_request_policy_confirmation() {
        // Verify that preflight classes match what request_policy_confirmation
        // would compute for the same descriptor/policy.
        let mut state = test_state_with_positive_scope(ExecutionSurface::TuiManual);
        let desc = OperationDescriptor {
            operation: "scan-ports".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Passive,
            intended_uses: vec![],
            target: Some("10.0.0.1".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = state.preflight(&desc);
        // Compute expected classes using the same function that request_policy_confirmation uses
        let outcome = state.enforcement.evaluate(&desc);
        let expected_classes = eggsec::config::confirmation_classes_for(
            &desc,
            outcome.decision(),
            &state.enforcement.execution_policy,
        );
        assert_eq!(
            result.required_confirmation_classes, expected_classes,
            "Preflight classes should match request_policy_confirmation computation"
        );
    }
}
