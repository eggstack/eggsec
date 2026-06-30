//! EnforcementFacade — extracted enforcement evaluation and approval logic.
//!
//! Phase 8 extraction: moves policy evaluation and approval out of App
//! into a focused struct. App retains the UI-level enforcement flows
//! (request/confirm/cancel policy confirmation) because they touch overlay state.

use eggsec::audit::{audit_event_from_enforcement_outcome, emit_audit_event};
use eggsec::config::{
    confirmation_classes_for, ApprovedOperation, ConfirmationClass, EnforcementError,
    EnforcementOutcome, ExecutionSurface, ManualOverride, OperationDescriptor, PolicyDecision,
};

/// Extracted enforcement facade — owns the enforcement state and provides
/// policy evaluation and approval methods. Reduces App's responsibility surface.
pub struct EnforcementFacade {
    pub state: super::enforcement::TuiEnforcementState,
    /// Cached approval token from the pre-dispatch gate in `handle_enter()`.
    /// Consumed by `evaluate_policy_and_dispatch()` to avoid redundant evaluation.
    pub(crate) pending_approved: Option<ApprovedOperation>,
}

impl EnforcementFacade {
    pub fn new(state: super::enforcement::TuiEnforcementState) -> Self {
        Self {
            state,
            pending_approved: None,
        }
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
    pub fn evaluate_and_try_approve(
        &mut self,
        desc: OperationDescriptor,
    ) -> Result<ApprovedOperation, EnforcementError> {
        // Consume cached approval from the pre-dispatch gate if available
        if let Some(cached) = self.pending_approved.take() {
            if cached.descriptor().operation == desc.operation {
                return Ok(cached);
            }
        }
        self.try_approve(desc)
    }

    /// Consume a cached approval if it matches the given descriptor.
    pub fn take_cached_approval(
        &mut self,
        desc: &OperationDescriptor,
    ) -> Option<ApprovedOperation> {
        self.pending_approved
            .take()
            .filter(|a| a.descriptor().operation == desc.operation)
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
    pub fn toggle_posture(&mut self) -> eggsec::config::ExecutionProfile {
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
        EnforcementContext, ExecutionPolicy, ExecutionProfile, ExecutionSurface, LoadedScope,
        OperationDescriptor, OperationMode, OperationRisk, Scope, ScopeRule, ScopeSource,
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
        facade.pending_approved = Some(first);
        // Second call should use the cached token
        let second = facade.evaluate_and_try_approve(desc);
        assert!(second.is_ok(), "cached approval should be reused");
    }

    #[test]
    fn take_cached_approval_returns_matching() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc.clone()).unwrap();
        facade.pending_approved = Some(approved);
        let taken = facade.take_cached_approval(&desc);
        assert!(taken.is_some(), "should take matching approval");
        assert!(
            facade.pending_approved.is_none(),
            "pending should be cleared"
        );
    }

    #[test]
    fn take_cached_approval_rejects_mismatch() {
        let mut facade = test_facade(ExecutionSurface::TuiManual);
        let desc1 = passive_descriptor("recon", Some("example.com"));
        let approved = facade.try_approve(desc1).unwrap();
        facade.pending_approved = Some(approved);
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
}
