//! Enforcement facade over the pure policy kernel.
//!
//! Canonical owner of authorization semantics is `eggsec-policy` (Phase C).
//! This module re-exports the pure decision/approval types unchanged and
//! provides the engine-owned I/O-enabled [`EnforcementContext`] wrapper:
//!
//! - feature availability is snapshotted from the engine `feature_registry`
//!   at construction ([`current_enabled_features`](crate::policy_bridge::features::current_enabled_features));
//! - destination facts are acquired per evaluation via the engine resolver
//!   bridge, then decided by the pure kernel;
//! - unresolvable targets and CIDR-without-IP cases preserve the legacy
//!   `InvalidTarget` hard-denial behavior.
//!
//! New code that already owns resolution facts and feature sets should use
//! `eggsec_policy` directly. Existing `crate::config::EnforcementContext`
//! call sites keep working unchanged.

use std::ops::{Deref, DerefMut};

pub use eggsec_policy::decision::{
    classify_denial_reasons, confirmation_class_strings, confirmation_classes_for,
    may_downgrade_to_warning, ConfirmationClass, EnforcementError, EnforcementOutcome,
    ManualOverride, PolicyDecision, PreflightOutcomeKind, PreflightResult,
};
pub use eggsec_policy::{
    ApprovedOperation, DenialClass, ExecutionPolicy, ExecutionProfile, ExecutionSurface,
    OperationDescriptor,
};

use crate::policy_bridge::features::current_enabled_features;
use crate::policy_bridge::resolver::{resolve_hostname_facts, resolve_target_facts};
use eggsec_policy::{LoadedScope, Scope, TargetScope};

/// I/O-enabled enforcement context (engine facade).
///
/// Wraps the pure [`eggsec_policy::EnforcementContext`] with engine-owned
/// inputs: the feature snapshot comes from the compile-time registry, and
/// each evaluation resolves destination facts through system DNS before
/// delegating to the deterministic kernel. Field accesses dereference to the
/// inner pure context.
#[derive(Debug, Clone)]
pub struct EnforcementContext {
    inner: eggsec_policy::EnforcementContext,
}

impl Deref for EnforcementContext {
    type Target = eggsec_policy::EnforcementContext;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for EnforcementContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Resolve caller-visible facts for a descriptor target, mirroring the
/// legacy parse choice (full resolution when CIDR rules exist, hostname-only
/// otherwise).
fn resolve_facts_for_scope(
    scope: &Scope,
    target: &str,
) -> Result<TargetScope, eggsec_policy::ScopeError> {
    if scope.has_ip_based_rules() {
        resolve_target_facts(target)
    } else {
        resolve_hostname_facts(target)
    }
}

impl EnforcementContext {
    pub fn manual_permissive(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self {
        Self {
            inner: eggsec_policy::EnforcementContext::manual_permissive(
                policy,
                loaded_scope,
                current_enabled_features(),
            ),
        }
    }
    pub fn manual_guarded(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self {
        Self {
            inner: eggsec_policy::EnforcementContext::manual_guarded(
                policy,
                loaded_scope,
                current_enabled_features(),
            ),
        }
    }
    pub fn ci_strict(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self {
        Self {
            inner: eggsec_policy::EnforcementContext::ci_strict(
                policy,
                loaded_scope,
                current_enabled_features(),
            ),
        }
    }
    pub fn mcp_strict(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self {
        Self {
            inner: eggsec_policy::EnforcementContext::mcp_strict(
                policy,
                loaded_scope,
                current_enabled_features(),
            ),
        }
    }
    pub fn agent_strict(policy: ExecutionPolicy, loaded_scope: LoadedScope) -> Self {
        Self {
            inner: eggsec_policy::EnforcementContext::agent_strict(
                policy,
                loaded_scope,
                current_enabled_features(),
            ),
        }
    }

    /// Construct an [`EnforcementContext`] from an [`ExecutionSurface`].
    pub fn for_surface(
        surface: ExecutionSurface,
        policy: ExecutionPolicy,
        loaded_scope: LoadedScope,
    ) -> Self {
        Self {
            inner: eggsec_policy::EnforcementContext::for_surface(
                surface,
                policy,
                loaded_scope,
                current_enabled_features(),
            ),
        }
    }

    /// Evaluate an operation descriptor against this enforcement context.
    ///
    /// Acquires destination facts via system DNS (legacy behavior), then
    /// delegates to the pure kernel. Unresolvable targets and CIDR-rules
    /// without resolved IPs preserve the legacy `InvalidTarget` hard denial.
    pub fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome {
        match descriptor.target.as_deref() {
            Some(target) => {
                match resolve_facts_for_scope(&self.inner.loaded_scope.scope, target) {
                    Ok(facts) => {
                        // Legacy `DnsResolution` error: CIDR rules configured
                        // but nothing resolved.
                        if self.inner.loaded_scope.scope.has_ip_based_rules() && facts.ip.is_none()
                        {
                            let err = eggsec_policy::ScopeError::DnsResolution(
                                target.to_string(),
                                "DNS resolution failed with CIDR rules configured".to_string(),
                            );
                            return self.invalid_target_denial(
                                descriptor,
                                target,
                                &format!("scope check error: {}", err),
                            );
                        }
                        self.inner.evaluate(descriptor, Some(&facts))
                    }
                    Err(e) => self.invalid_target_denial(
                        descriptor,
                        target,
                        &format!("scope check error: {}", e),
                    ),
                }
            }
            None => self.inner.evaluate(descriptor, None),
        }
    }

    fn invalid_target_denial(
        &self,
        descriptor: &OperationDescriptor,
        target: &str,
        reason: &str,
    ) -> EnforcementOutcome {
        // Mirror the legacy `Err(e)` arm: run the pure evaluation over
        // pattern-only fallback facts so feature/risk/flag context is still
        // recorded, then force the `InvalidTarget` hard denial. The "invalid"
        // marker keeps `confirmation_classes_for` hard (no confirmation path).
        let fallback = TargetScope::for_host_without_addresses(target.to_string());
        let mut decision = eggsec_policy::evaluate_operation_policy(
            descriptor,
            &self.inner.execution_policy,
            Some(&self.inner.loaded_scope.scope),
            &self.inner.enabled_features,
            Some(&fallback),
        );
        decision.push_denial_class(DenialClass::InvalidTarget, reason);
        decision.allowed = false;
        EnforcementOutcome::Deny(decision)
    }

    /// Approve an operation for dispatch on a strict automated surface.
    ///
    /// Only `Allow` outcomes produce an [`ApprovedOperation`]. Token issuance
    /// itself happens inside the pure kernel; this facade only acquires facts.
    #[allow(clippy::result_large_err)]
    pub fn approve(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
    ) -> Result<ApprovedOperation, EnforcementError> {
        let facts = self.facts_for_approval(&descriptor)?;
        self.inner.approve(surface, descriptor, facts.as_ref())
    }

    /// Approve an operation for dispatch on a manual surface with optional override.
    ///
    /// Token issuance itself happens inside the pure kernel; this facade only
    /// acquires facts.
    #[allow(clippy::result_large_err)]
    pub fn approve_manual(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        manual_override: Option<&ManualOverride>,
    ) -> Result<ApprovedOperation, EnforcementError> {
        let facts = self.facts_for_approval(&descriptor)?;
        self.inner
            .approve_manual(surface, descriptor, facts.as_ref(), manual_override)
    }

    /// Resolve facts for an approval path, preserving the legacy
    /// `InvalidTarget` hard denial for unresolvable targets.
    #[allow(clippy::result_large_err)]
    fn facts_for_approval(
        &self,
        descriptor: &OperationDescriptor,
    ) -> Result<Option<TargetScope>, EnforcementError> {
        match descriptor.target.as_deref() {
            Some(target) => match resolve_facts_for_scope(&self.inner.loaded_scope.scope, target) {
                Ok(facts) => {
                    if self.inner.loaded_scope.scope.has_ip_based_rules() && facts.ip.is_none() {
                        let err = eggsec_policy::ScopeError::DnsResolution(
                            target.to_string(),
                            "DNS resolution failed with CIDR rules configured".to_string(),
                        );
                        return Err(EnforcementError::Denied {
                            decision: self.invalid_target_decision(
                                descriptor,
                                target,
                                &format!("scope check error: {}", err),
                            ),
                        });
                    }
                    Ok(Some(facts))
                }
                Err(e) => Err(EnforcementError::Denied {
                    decision: self.invalid_target_decision(
                        descriptor,
                        target,
                        &format!("scope check error: {}", e),
                    ),
                }),
            },
            None => Ok(None),
        }
    }

    fn invalid_target_decision(
        &self,
        descriptor: &OperationDescriptor,
        target: &str,
        reason: &str,
    ) -> PolicyDecision {
        let fallback = TargetScope::for_host_without_addresses(target.to_string());
        let mut decision = eggsec_policy::evaluate_operation_policy(
            descriptor,
            &self.inner.execution_policy,
            Some(&self.inner.loaded_scope.scope),
            &self.inner.enabled_features,
            Some(&fallback),
        );
        decision.push_denial_class(DenialClass::InvalidTarget, reason);
        decision.allowed = false;
        decision
    }
}

/// Check if a named Cargo feature is currently compiled.
///
/// Delegates to the authoritative feature registry.
/// **Fail-closed**: unknown feature names return `false`.
pub fn is_feature_enabled(feature: &str) -> bool {
    super::is_feature_enabled_registry(feature)
}

/// Returns `true` if the given feature string is in the authoritative registry.
#[allow(dead_code)]
pub fn is_known_feature(feature: &str) -> bool {
    super::is_known_feature_registry(feature)
}

/// Shared policy evaluation entry point (engine compatibility).
///
/// Snapshots current features and resolves destination facts, then delegates
/// to the pure [`eggsec_policy::evaluate_operation_policy`].
pub fn evaluate_operation_policy(
    descriptor: &OperationDescriptor,
    policy: &ExecutionPolicy,
    scope: Option<&Scope>,
) -> PolicyDecision {
    let features = current_enabled_features();
    let facts = descriptor
        .target
        .as_deref()
        .and_then(|target| scope.and_then(|s| resolve_facts_for_scope(s, target).ok()));
    eggsec_policy::evaluate_operation_policy(descriptor, policy, scope, &features, facts.as_ref())
}

/// Profile-aware enforcement evaluation (engine compatibility).
pub fn evaluate_enforcement(
    descriptor: &OperationDescriptor,
    policy: &ExecutionPolicy,
    scope: Option<&Scope>,
    profile: ExecutionProfile,
) -> EnforcementOutcome {
    let features = current_enabled_features();
    let facts = descriptor
        .target
        .as_deref()
        .and_then(|target| scope.and_then(|s| resolve_facts_for_scope(s, target).ok()));
    eggsec_policy::evaluate_enforcement(
        descriptor,
        policy,
        scope,
        profile,
        &features,
        facts.as_ref(),
    )
}

/// Advisory preflight evaluation (engine compatibility).
pub fn preflight_operation(
    surface: ExecutionSurface,
    enforcement: &EnforcementContext,
    descriptor: OperationDescriptor,
    manual_override: Option<&ManualOverride>,
) -> PreflightResult {
    let facts = descriptor.target.as_deref().and_then(|target| {
        resolve_facts_for_scope(&enforcement.inner.loaded_scope.scope, target).ok()
    });
    eggsec_policy::preflight_operation(
        surface,
        &enforcement.inner,
        descriptor,
        facts.as_ref(),
        manual_override,
    )
}
