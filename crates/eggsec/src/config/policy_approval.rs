//! Approval token issuance (Phase D WS6).
//!
//! Cohesive module extracted from `config/policy_decision.rs`: approval
//! issuance and binding checks only. Evaluation logic lives in
//! `policy_decision.rs`; descriptor/catalog types live in `policy.rs`.
//!
//! [`ApprovedOperation`] is the only valid dispatch token. Construction is
//! private/controlled (`pub(crate) new`, plus a `for_test` shim behind
//! `test`/`test-helpers`); surfaces obtain tokens via
//! `EnforcementContext::approve` / `approve_manual`. Dispatch binding is
//! verified by `validate_request_binding` before any executor runs.
//!
//! Stable facade: `policy_decision.rs` re-exports everything here.

use super::{ExecutionProfile, ExecutionSurface, OperationDescriptor, PolicyDecision};

/// Proof that an operation has passed enforcement evaluation.
///
/// This token is produced exclusively by [`crate::config::EnforcementContext::approve`] or
/// [`crate::config::EnforcementContext::approve_manual`]. Strict programmatic surfaces
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

    /// Construct an approved operation for integration testing.
    ///
    /// This is a public convenience wrapper around the private constructor
    /// intended for integration test files that need to build approval tokens
    /// without going through the full enforcement path.
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn for_test(
        descriptor: OperationDescriptor,
        decision: PolicyDecision,
        surface: ExecutionSurface,
        profile: ExecutionProfile,
    ) -> Self {
        Self::new(descriptor, decision, surface, profile, None)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutionPolicy, LoadedScope, OperationMode, OperationRisk};

    fn sample_descriptor() -> OperationDescriptor {
        OperationDescriptor::new(
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
        )
    }

    #[test]
    fn approval_token_exposes_bound_descriptor_and_surface() {
        let enforcement = crate::config::EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let descriptor = sample_descriptor();
        let approved = enforcement
            .approve(ExecutionSurface::RestApi, descriptor.clone())
            .expect("allowlisted loopback should approve");
        assert_eq!(approved.descriptor().operation, "scan-ports");
        assert_eq!(approved.surface(), ExecutionSurface::RestApi);
        assert_eq!(
            approved.profile(),
            crate::config::ExecutionProfile::McpStrict
        );
    }

    #[test]
    fn approval_for_test_builds_token_without_enforcement() {
        let descriptor = sample_descriptor();
        let decision = PolicyDecision::allowed(
            "scan-ports",
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            Vec::new(),
        );
        let approved = ApprovedOperation::for_test(
            descriptor.clone(),
            decision,
            ExecutionSurface::RestApi,
            crate::config::ExecutionProfile::McpStrict,
        );
        assert_eq!(approved.descriptor().operation, descriptor.operation);
    }
}
