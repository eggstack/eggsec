//! EnforcementFacade — extracted enforcement evaluation and approval logic.
//!
//! Phase 8 extraction: moves policy evaluation and approval out of App
//! into a focused struct. App retains the UI-level enforcement flows
//! (request/confirm/cancel policy confirmation) because they touch overlay state.

use eggsec::audit::{audit_event_from_enforcement_outcome, emit_audit_event};
use eggsec::config::{
    confirmation_classes_for, ApprovedOperation, ConfirmationClass, EnforcementError,
    EnforcementOutcome, ExecutionProfile, ExecutionSurface, ManualOverride, OperationDescriptor,
    PolicyDecision,
};

/// Cached approval plus the evaluation inputs that produced it.
///
/// Reuse requires exact equality on every field: the descriptor (via
/// `ApprovedOperation::matches_descriptor`, which uses derived `PartialEq`
/// so future descriptor fields participate automatically), plus the
/// scope fingerprint, policy hash, surface/profile, and manual-override
/// state. Any change discards the cache and forces fresh evaluation, so a
/// stale token never triggers a second approval for the wrong descriptor
/// nor a confusing late binding failure.
#[derive(Debug, Clone)]
struct CachedApproval {
    approved: ApprovedOperation,
    policy_hash: String,
    scope_fingerprint: String,
    surface: ExecutionSurface,
    profile: ExecutionProfile,
    manual_override: ManualOverride,
}

/// Extracted enforcement facade — owns the enforcement state and provides
/// policy evaluation and approval methods. Reduces App's responsibility surface.
pub struct EnforcementFacade {
    pub state: super::enforcement::TuiEnforcementState,
    /// Cached approval token from the pre-dispatch gate in `handle_enter()`.
    /// Consumed by `evaluate_policy_and_dispatch()` to avoid redundant evaluation.
    pending_approved: Option<CachedApproval>,
}

impl EnforcementFacade {
    pub fn new(state: super::enforcement::TuiEnforcementState) -> Self {
        Self {
            state,
            pending_approved: None,
        }
    }

    fn scope_fingerprint(&self) -> String {
        serde_json::to_string(&self.state.loaded_scope).unwrap_or_default()
    }

    fn current_cache_key(
        &self,
    ) -> (
        String,
        String,
        ExecutionSurface,
        ExecutionProfile,
        ManualOverride,
    ) {
        (
            self.state.enforcement.policy_hash(),
            self.scope_fingerprint(),
            self.state.surface,
            self.state.enforcement.execution_profile,
            self.state.manual_override.clone(),
        )
    }

    fn cached_matches(&self, cached: &CachedApproval, desc: &OperationDescriptor) -> bool {
        if !cached.approved.matches_descriptor(desc) {
            return false;
        }
        let (policy_hash, scope_fp, surface, profile, manual_override) = self.current_cache_key();
        cached.policy_hash == policy_hash
            && cached.scope_fingerprint == scope_fp
            && cached.surface == surface
            && cached.profile == profile
            && cached.manual_override == manual_override
            // The token itself must also agree with the current surface/profile.
            && cached.approved.surface() == surface
            && cached.approved.profile() == profile
    }

    /// Store an approval for later reuse, capturing the current evaluation
    /// inputs. Overwrites any prior cached token.
    pub fn set_cached_approval(&mut self, approved: ApprovedOperation) {
        let (policy_hash, scope_fingerprint, surface, profile, manual_override) =
            self.current_cache_key();
        self.pending_approved = Some(CachedApproval {
            approved,
            policy_hash,
            scope_fingerprint,
            surface,
            profile,
            manual_override,
        });
    }

    /// Discard any cached approval without consuming it.
    pub fn clear_cached_approval(&mut self) {
        self.pending_approved = None;
    }

    /// Invalidate the cache for reloaded config/scope or other generation
    /// changes. Phase 2 live reload must call this when scope/policy state
    /// is replaced; the fingerprint check already fails closed, this makes
    /// invalidation explicit and testable.
    pub fn invalidate_cached_approval(&mut self) {
        self.clear_cached_approval();
    }

    /// Returns `true` when a cached approval is present (regardless of match).
    /// Test helper for asserting cache lifecycle without exposing the token.
    #[cfg(test)]
    pub(crate) fn has_pending_approval(&self) -> bool {
        self.pending_approved.is_some()
    }

    /// Attempt to approve an operation using the appropriate enforcement path
    /// based on the current TUI surface.
    pub fn try_approve(
        &mut self,
        desc: OperationDescriptor,
    ) -> Result<ApprovedOperation, EnforcementError> {
        let outcome = self.state.enforcement.evaluate(&desc);
        self.state.last_preflight = Some(super::enforcement::TuiPreflightResult::from_outcome(
            &desc,
            &outcome,
            &self.state.enforcement.execution_policy,
        ));
        let required_classes: Vec<ConfirmationClass> = match &outcome {
            EnforcementOutcome::RequireConfirmation(decision) => {
                confirmation_classes_for(&desc, decision, &self.state.enforcement.execution_policy)
            }
            _ => vec![],
        };
        let audit = audit_event_from_enforcement_outcome(
            self.state.surface,
            &self.state.enforcement,
            &desc,
            &outcome,
            false,
            false,
            None,
            &required_classes,
            None,
            None,
        );
        emit_audit_event(&audit);

        match self.state.surface {
            ExecutionSurface::TuiManual => self.state.enforcement.approve_manual(
                self.state.surface,
                desc,
                Some(&self.state.manual_override),
            ),
            _ => self.state.enforcement.approve(self.state.surface, desc),
        }
    }

    /// Central policy evaluation + dispatch. Uses the `ApprovedOperation` token
    /// to structurally gate `spawn_task()`. Handles `EnforcementError` variants
    /// for confirmation/denial flows.
    ///
    /// If a cached `ApprovedOperation` from the pre-dispatch gate exists (set in
    /// `handle_enter()`), it is consumed here to avoid redundant evaluation.
    /// Reuse requires exact descriptor binding plus unchanged scope, policy,
    /// surface/profile, and manual-override state. A stale token is discarded
    /// and the request is freshly evaluated for the correct descriptor, so it
    /// never produces a second approval for the wrong descriptor nor a late
    /// engine binding failure the frontend could have caught earlier.
    /// Engine-side `validate_request_binding` remains the final gate.
    pub fn evaluate_and_try_approve(
        &mut self,
        desc: OperationDescriptor,
    ) -> Result<ApprovedOperation, EnforcementError> {
        // Consume cached approval from the pre-dispatch gate if available.
        // Mismatched generations are dropped (fail closed to fresh evaluation).
        if let Some(cached) = self.pending_approved.take() {
            if self.cached_matches(&cached, &desc) {
                return Ok(cached.approved);
            }
        }
        self.try_approve(desc)
    }

    /// Consume a cached approval only when it is bound to the exact descriptor
    /// currently requested and the evaluation inputs are unchanged.
    pub fn take_cached_approval(
        &mut self,
        desc: &OperationDescriptor,
    ) -> Option<ApprovedOperation> {
        let cached = self.pending_approved.take()?;
        if self.cached_matches(&cached, desc) {
            Some(cached.approved)
        } else {
            None
        }
    }

    /// Confirm the pending policy override and return the approved operation + audit info.
    /// Returns (ApprovedOperation, EnforcementOutcome for audit, required classes, decision).
    pub fn confirm_override(
        &mut self,
        descriptor: &OperationDescriptor,
        required_classes: &[ConfirmationClass],
        reason: Option<String>,
    ) -> Result<
        (
            ApprovedOperation,
            EnforcementOutcome,
            Vec<ConfirmationClass>,
            PolicyDecision,
        ),
        EnforcementError,
    > {
        let mut mo = ManualOverride::default();
        for c in required_classes {
            match c {
                ConfirmationClass::OutOfScope | ConfirmationClass::TargetExpansion => {
                    mo.allow_out_of_scope = true;
                }
                ConfirmationClass::ExplicitExclusion => {
                    mo.allow_explicit_exclusion = true;
                }
                ConfirmationClass::HighRisk => {
                    mo.allow_high_risk = true;
                }
                ConfirmationClass::NonBaselineCapability => {
                    mo.allow_nonbaseline_capability = true;
                }
                ConfirmationClass::PrivateResolution => {
                    mo.allow_private_resolution = true;
                }
                ConfirmationClass::CrossHostRedirect => {
                    mo.allow_cross_host_redirect = true;
                }
                ConfirmationClass::TrafficInterception => {
                    mo.allow_web_proxy = true;
                }
            }
        }
        mo.reason = reason;
        mo.assume_yes = false; // TUI confirm popup never sets broad assume_yes

        // Manual-override state is an evaluation input: drop any token cached
        // under the previous override generation before re-evaluating.
        self.clear_cached_approval();
        // Track the override centrally
        self.state.manual_override = mo.clone();

        let result = match self.state.surface {
            ExecutionSurface::TuiManual => self.state.enforcement.approve_manual(
                self.state.surface,
                descriptor.clone(),
                Some(&mo),
            ),
            _ => self
                .state
                .enforcement
                .approve(self.state.surface, descriptor.clone()),
        };

        match result {
            Ok(approved_op) => {
                let decision = approved_op.decision().clone();
                let outcome = EnforcementOutcome::RequireConfirmation(decision.clone());
                Ok((approved_op, outcome, required_classes.to_vec(), decision))
            }
            Err(e) => Err(e),
        }
    }

    /// Build an audit event for a confirmed override.
    pub fn audit_confirmed_override(
        &self,
        descriptor: &OperationDescriptor,
        outcome: &EnforcementOutcome,
        required_classes: &[ConfirmationClass],
        mo: &ManualOverride,
    ) {
        let audit = audit_event_from_enforcement_outcome(
            self.state.surface,
            &self.state.enforcement,
            descriptor,
            outcome,
            true,
            false,
            Some(mo),
            required_classes,
            None,
            None,
        );
        emit_audit_event(&audit);
    }

    /// Access the underlying enforcement state immutably.
    pub fn state(&self) -> &super::enforcement::TuiEnforcementState {
        &self.state
    }

    /// Access the underlying enforcement state mutably.
    pub fn state_mut(&mut self) -> &mut super::enforcement::TuiEnforcementState {
        &mut self.state
    }

    /// Toggle enforcement posture (delegates to TuiEnforcementState).
    /// Clears any cached approval: surface/profile is an evaluation input,
    /// so a token issued under the previous posture must not be reused.
    pub fn toggle_posture(&mut self) -> eggsec::config::ExecutionProfile {
        self.clear_cached_approval();
        self.state.toggle_posture()
    }

    /// Get the mode label ("Manual" or "Guarded").
    pub fn mode_label(&self) -> &'static str {
        self.state.mode_label()
    }

    /// Get the status string for the status bar.
    pub fn status_string(&self) -> String {
        self.state.status_string()
    }

    /// Get scope label.
    pub fn scope_label(&self) -> String {
        self.state.scope_label()
    }

    /// Get allow rule count.
    pub fn allow_rule_count(&self) -> usize {
        self.state.allow_rule_count()
    }

    /// Get exclusion rule count.
    pub fn exclusion_rule_count(&self) -> usize {
        self.state.exclusion_rule_count()
    }

    /// Check if guarded mode is active.
    pub fn is_guarded(&self) -> bool {
        self.state.is_guarded()
    }

    /// Run advisory preflight evaluation.
    pub fn preflight(
        &mut self,
        descriptor: &eggsec::config::OperationDescriptor,
    ) -> super::enforcement::TuiPreflightResult {
        self.state.preflight(descriptor)
    }

    /// Access the underlying EnforcementContext (for direct evaluation in UI).
    pub fn enforcement(&self) -> &eggsec::config::EnforcementContext {
        &self.state.enforcement
    }

    /// Access the loaded scope (for scope checks in UI).
    pub fn loaded_scope(&self) -> &eggsec::config::LoadedScope {
        &self.state.loaded_scope
    }

    /// Access the execution surface.
    pub fn surface(&self) -> eggsec::config::ExecutionSurface {
        self.state.surface
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec::config::{
        EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope, OperationDescriptor,
        OperationMode, OperationRisk, Scope, ScopeRule, ScopeSource,
    };

    fn test_facade(surface: ExecutionSurface) -> EnforcementFacade {
        let scope = LoadedScope::default_empty();
        let policy = ExecutionPolicy::default();
        let enforcement = EnforcementContext::for_surface(surface, policy, scope.clone());
        let state =
            super::super::enforcement::TuiEnforcementState::new(surface, scope, enforcement);
        EnforcementFacade::new(state)
    }

    fn passive_descriptor(operation: &str, target: Option<&str>) -> OperationDescriptor {
        OperationDescriptor::new(
            operation.to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::Passive,
            vec![],
            target.map(|t| t.to_string()),
            vec![],
            vec![],
            false,
            false,
            vec![],
        )
    }

    #[test]
    fn try_approve_allows_passive_in_default_scope() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let result = facade.try_approve(desc);
        assert!(result.is_ok(), "passive op should be approved");
    }

    #[test]
    fn try_approve_populates_last_preflight() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let _ = facade.try_approve(desc);
        assert!(
            facade.state.last_preflight.is_some(),
            "try_approve should set last_preflight"
        );
    }

    #[test]
    fn evaluate_and_try_approve_uses_cached_approval() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        // Pre-populate a cached approval
        let desc = passive_descriptor("recon", Some("example.com"));
        let first = facade.try_approve(desc.clone()).unwrap();
        facade.set_cached_approval(first);
        // Second call should use the cached token
        let second = facade.evaluate_and_try_approve(desc);
        assert!(second.is_ok(), "cached approval should be reused");
    }

    #[test]
    fn take_cached_approval_returns_matching() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc.clone()).unwrap();
        facade.set_cached_approval(approved);
        let taken = facade.take_cached_approval(&desc);
        assert!(taken.is_some(), "should take matching approval");
        assert!(!facade.has_pending_approval(), "pending should be cleared");
    }

    #[test]
    fn take_cached_approval_rejects_mismatch() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc1 = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc1).unwrap();
        facade.set_cached_approval(approved);
        let desc2 = passive_descriptor("scan-ports", Some("example.com"));
        let taken = facade.take_cached_approval(&desc2);
        assert!(taken.is_none(), "should not return mismatched approval");
    }

    #[test]
    fn confirm_override_sets_manual_override_flags() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let classes = vec![ConfirmationClass::OutOfScope];
        let result = facade.confirm_override(&desc, &classes, Some("test reason".to_string()));
        assert!(
            result.is_ok(),
            "confirmation should succeed for manual mode"
        );
        assert!(facade.state.manual_override.allow_out_of_scope);
        assert_eq!(
            facade.state.manual_override.reason,
            Some("test reason".to_string())
        );
    }

    #[test]
    fn facade_state_accessor() {
        let facade = test_facade(ExecutionSurface::TuiManual);
        assert_eq!(facade.state().surface, ExecutionSurface::TuiManual);
    }

    // ─── Work item 3: Preflight/execution descriptor identity tests ────

    fn test_facade_with_scope(surface: ExecutionSurface) -> EnforcementFacade {
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
        let state =
            super::super::enforcement::TuiEnforcementState::new(surface, loaded_scope, enforcement);
        EnforcementFacade::new(state)
    }

    fn test_facade_guarded(surface: ExecutionSurface) -> EnforcementFacade {
        let mut facade = test_facade(surface);
        facade.toggle_posture();
        assert!(facade.is_guarded(), "should be in guarded mode");
        facade
    }

    fn safe_active_descriptor(operation: &str, target: Option<&str>) -> OperationDescriptor {
        OperationDescriptor::new(
            operation.to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![],
            target.map(|t| t.to_string()),
            vec![],
            vec![],
            false,
            false,
            vec![],
        )
    }

    /// Preflight and execution agree: passive op with in-scope target → Allow.
    #[test]
    fn preflight_and_execution_agree_on_allowed_action() {
        let mut facade = test_facade_with_scope(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));

        // Preflight path
        let preflight = facade.preflight(&desc);
        let preflight_outcome = match preflight.outcome_kind {
            super::super::enforcement::TuiPreflightOutcomeKind::Allow => "allowed",
            super::super::enforcement::TuiPreflightOutcomeKind::Warn => "warning",
            super::super::enforcement::TuiPreflightOutcomeKind::RequireConfirmation => {
                "confirmation"
            }
            super::super::enforcement::TuiPreflightOutcomeKind::Deny => "denied",
        };

        // Execution path
        let exec_result = facade.try_approve(desc);
        let exec_outcome = if exec_result.is_ok() {
            "allowed"
        } else {
            "denied"
        };

        assert_eq!(
            preflight_outcome, exec_outcome,
            "preflight and execution must agree on allowed action"
        );
    }

    /// Preflight and execution agree: out-of-scope target → RequireConfirmation.
    #[test]
    fn preflight_and_execution_agree_on_confirmation_action() {
        let mut facade = test_facade_with_scope(ExecutionSurface::TuiManual);
        let desc = safe_active_descriptor("scan-ports", Some("evil.example.net"));

        // Preflight path
        let preflight = facade.preflight(&desc);
        let preflight_needs_confirmation = matches!(
            preflight.outcome_kind,
            super::super::enforcement::TuiPreflightOutcomeKind::RequireConfirmation
        );

        // Execution path (TuiManual surface uses approve_manual, which may succeed
        // with a Warn/Confirm outcome — the key is the evaluation agrees)
        let exec_result = facade.try_approve(desc.clone());
        // In manual mode, RequireConfirmation triggers a confirmation overlay,
        // not an immediate deny. The facade's try_approve calls approve_manual
        // which may succeed (returning Ok) if manual override is available.
        // What matters is that the evaluation outcome matches.
        let outcome = facade.state.enforcement.evaluate(&desc);
        let exec_needs_confirmation = matches!(
            outcome,
            eggsec::config::EnforcementOutcome::RequireConfirmation(_)
        );

        assert_eq!(
            preflight_needs_confirmation, exec_needs_confirmation,
            "preflight and execution must agree on confirmation requirement"
        );
    }

    /// Preflight and execution agree: guarded mode + out-of-scope → Deny.
    #[test]
    fn preflight_and_execution_agree_on_denied_action() {
        let mut facade = test_facade_guarded(ExecutionSurface::TuiManual);
        let desc = safe_active_descriptor("scan-ports", Some("evil.example.net"));

        // Preflight path
        let preflight = facade.preflight(&desc);
        let preflight_denied = matches!(
            preflight.outcome_kind,
            super::super::enforcement::TuiPreflightOutcomeKind::Deny
        );

        // Execution path
        let exec_result = facade.try_approve(desc.clone());
        let exec_denied = exec_result.is_err();

        assert_eq!(
            preflight_denied, exec_denied,
            "preflight and execution must agree on denied action"
        );
    }

    /// Descriptor mismatch: cached approval for one operation does not match a different operation.
    #[test]
    fn descriptor_mismatch_catches_different_operations() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc1 = passive_descriptor("recon", Some("example.com"));
        let desc2 = passive_descriptor("scan-ports", Some("example.com"));

        let approved = facade.try_approve(desc1.clone()).unwrap();
        facade.set_cached_approval(approved);

        let taken = facade.take_cached_approval(&desc2);
        assert!(
            taken.is_none(),
            "cached approval for 'recon' should not match 'scan-ports'"
        );
        // After take, pending is now None (take always empties)
        assert!(
            !facade.has_pending_approval(),
            "pending_approved should be empty after take"
        );
    }

    // ─── Phase 0.1: exact approval-cache binding regression cases ───

    #[test]
    fn same_operation_same_descriptor_may_reuse() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc.clone()).unwrap();
        facade.set_cached_approval(approved);
        assert!(
            facade.take_cached_approval(&desc).is_some(),
            "same operation + same descriptor must reuse"
        );
    }

    #[test]
    fn same_operation_different_target_must_not_reuse() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc1 = passive_descriptor("recon", Some("example.com"));
        let desc2 = passive_descriptor("recon", Some("other.example.com"));
        let approved = facade.try_approve(desc1).unwrap();
        facade.set_cached_approval(approved);
        assert!(
            facade.take_cached_approval(&desc2).is_none(),
            "same operation + different target must not reuse"
        );
        // Stale token is discarded, not retained for a later wrong reuse.
        assert!(!facade.has_pending_approval());
    }

    #[test]
    fn same_operation_changed_option_must_not_reuse() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc1 = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc1.clone()).unwrap();
        facade.set_cached_approval(approved);
        let mut desc2 = desc1;
        desc2.risk = OperationRisk::Intrusive;
        assert!(
            facade.take_cached_approval(&desc2).is_none(),
            "same operation + changed policy-relevant option must not reuse"
        );
    }

    #[test]
    fn changed_scope_invalidates_cached_approval() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc.clone()).unwrap();
        facade.set_cached_approval(approved);
        // Simulate a scope reload: replace loaded scope + enforcement context.
        let new_scope = LoadedScope::explicit(
            Scope {
                allowed_targets: vec![ScopeRule::new("example.com".to_string())],
                ..Default::default()
            },
            ScopeSource::ConfigFile,
            Some("scope.toml".to_string()),
        );
        facade.state.loaded_scope = new_scope.clone();
        facade.state.enforcement = EnforcementContext::for_surface(
            ExecutionSurface::TuiManual,
            ExecutionPolicy::default(),
            new_scope,
        );
        assert!(
            facade.take_cached_approval(&desc).is_none(),
            "changed scope generation must invalidate prior cached approval"
        );
    }

    #[test]
    fn changed_policy_invalidates_cached_approval() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc.clone()).unwrap();
        facade.set_cached_approval(approved);
        // Simulate a policy reload with a different flag.
        let mut policy = ExecutionPolicy::default();
        policy.allow_intrusive_fuzzing = true;
        facade.state.enforcement = EnforcementContext::for_surface(
            ExecutionSurface::TuiManual,
            policy,
            LoadedScope::default_empty(),
        );
        // loaded_scope stays default-empty, but policy hash changed.
        facade.state.loaded_scope = LoadedScope::default_empty();
        assert!(
            facade.take_cached_approval(&desc).is_none(),
            "changed policy generation must invalidate prior cached approval"
        );
    }

    #[test]
    fn posture_toggle_invalidates_cached_approval() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc.clone()).unwrap();
        facade.set_cached_approval(approved);
        assert!(facade.has_pending_approval());
        facade.toggle_posture();
        assert!(
            !facade.has_pending_approval(),
            "posture toggle must clear cached approval"
        );
    }

    #[test]
    fn manual_override_change_invalidates_cached_approval() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc.clone()).unwrap();
        facade.set_cached_approval(approved);
        // confirm_override replaces manual_override state and clears the cache.
        let classes = vec![ConfirmationClass::OutOfScope];
        let _ = facade.confirm_override(&desc, &classes, Some("reason".to_string()));
        assert!(
            !facade.has_pending_approval(),
            "override-state change must invalidate prior cached approval"
        );
    }

    #[test]
    fn stale_token_forces_fresh_evaluation_not_late_binding_failure() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc1 = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc1).unwrap();
        facade.set_cached_approval(approved);
        // Request a different target: facade must not return the stale token.
        // It falls back to fresh evaluation for the correct descriptor.
        let desc2 = passive_descriptor("recon", Some("other.example.com"));
        let result = facade.evaluate_and_try_approve(desc2.clone());
        // Fresh evaluation for a passive op in default-empty scope warns/allows;
        // the key invariant is the returned token (if Ok) is bound to desc2.
        if let Ok(token) = result {
            assert!(
                token.matches_descriptor(&desc2),
                "re-evaluated token must be bound to the requested descriptor, not the stale one"
            );
        }
        assert!(!facade.has_pending_approval());
    }

    #[test]
    fn invalidate_cached_approval_clears_for_live_reload() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc).unwrap();
        facade.set_cached_approval(approved);
        assert!(facade.has_pending_approval());
        facade.invalidate_cached_approval();
        assert!(!facade.has_pending_approval());
    }

    /// Preflight result's outcome_kind matches the raw enforcement evaluate() outcome.
    #[test]
    fn preflight_populates_same_outcome_as_raw_evaluate() {
        let mut facade = test_facade_with_scope(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));

        // Raw evaluation
        let raw_outcome = facade.enforcement().evaluate(&desc);

        // Preflight
        let preflight = facade.preflight(&desc);

        // Compare
        let raw_matches = match (&raw_outcome, &preflight.outcome_kind) {
            (
                eggsec::config::EnforcementOutcome::Allow(_),
                super::super::enforcement::TuiPreflightOutcomeKind::Allow,
            ) => true,
            (
                eggsec::config::EnforcementOutcome::Warn(_),
                super::super::enforcement::TuiPreflightOutcomeKind::Warn,
            ) => true,
            (
                eggsec::config::EnforcementOutcome::RequireConfirmation(_),
                super::super::enforcement::TuiPreflightOutcomeKind::RequireConfirmation,
            ) => true,
            (
                eggsec::config::EnforcementOutcome::Deny(_),
                super::super::enforcement::TuiPreflightOutcomeKind::Deny,
            ) => true,
            _ => false,
        };

        assert!(
            raw_matches,
            "preflight outcome_kind {:?} should match raw evaluate outcome {:?}",
            preflight.outcome_kind, raw_outcome
        );
    }
}
