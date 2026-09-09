//! Minimal engine service interfaces for protocol and agent adapters.
//!
//! Phase D (Workstream 1): protocol servers and the autonomous agent must not
//! construct or own engine internals directly. They depend on these narrow
//! capability traits instead of concrete [`crate::tool::ToolRegistry`],
//! [`crate::tool::ToolDispatcher`], or ad hoc policy helpers.
//!
//! # Dependency direction
//!
//! ```text
//! protocol adapter / agent  -->  service traits (this module)  -->  engine impls
//! ```
//!
//! The traits expose only canonical request/result DTOs (Phase C contracts):
//! [`crate::tool::ToolRequest`]/[`crate::tool::ToolResponse`],
//! [`crate::config::OperationDescriptor`]/[`crate::config::ApprovedOperation`],
//! and [`crate::config::OperationMetadata`]. Authorization remains inside the
//! engine service path:
//!
//! - [`CheckedExecutor`] exposes *only* checked execution. There is no raw
//!   `dispatch()` method, so adapters cannot invoke unchecked dispatch.
//! - [`PreflightService`] delegates to [`crate::config::EnforcementContext`];
//!   adapters never duplicate scope/policy evaluation.
//! - [`OperationCatalog`] is read-only introspection over canonical metadata.
//!
//! Protocol exposure metadata (which tools are visible on which surface)
//! remains separate from authorization; see `tool::registration` and
//! `tool::protocol::mcp::policy`.
//!
//! Trait objects (`Arc<dyn ...>`) or generics are both acceptable. Prefer the
//! simplest API that preserves testability; do not create one enormous
//! `EngineService` trait.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::config::{
    preflight_operation, ApprovedOperation, EnforcementContext, EnforcementError,
    EnforcementOutcome, ExecutionSurface, ManualOverride, OperationDescriptor, OperationMetadata,
    PolicyDecision, PreflightResult,
};
use crate::error::EggsecError;
use crate::tool::{EnforcedDispatcher, ToolRequest, ToolResponse};

// ---------------------------------------------------------------------------
// OperationCatalog / OperationIntrospection
// ---------------------------------------------------------------------------

/// Read-only catalog over canonical operation metadata.
///
/// Implementations must derive from [`crate::config::OperationMetadata`] (or
/// its direct successor). Adapters use this for `list`/`describe` paths and
/// for resolving tool aliases to canonical operation IDs. It never authorizes.
pub trait OperationCatalog: Send + Sync {
    /// Canonical metadata for an operation or tool alias ID.
    fn metadata_for_id(&self, id: &str) -> Option<&'static OperationMetadata>;

    /// All canonical operation metadata.
    fn all_metadata(&self) -> &'static [OperationMetadata];

    /// Returns true when `tool_id` resolves to `operation_id`
    /// (alias-aware, same semantics as `operation_matches_tool_id`).
    fn matches_tool_id(&self, tool_id: &str, operation_id: &str) -> bool;
}

/// Static catalog backed by the canonical `ALL_OPERATION_METADATA` registry.
///
/// This is the production implementation. Tests may substitute a fake
/// catalog without building a [`crate::tool::ToolRegistry`].
#[derive(Debug, Default, Clone, Copy)]
pub struct StaticOperationCatalog;

impl OperationCatalog for StaticOperationCatalog {
    fn metadata_for_id(&self, id: &str) -> Option<&'static OperationMetadata> {
        crate::config::metadata_for_tool_id(id)
    }

    fn all_metadata(&self) -> &'static [OperationMetadata] {
        crate::config::all_operation_metadata()
    }

    fn matches_tool_id(&self, tool_id: &str, operation_id: &str) -> bool {
        crate::config::operation_matches_tool_id(tool_id, operation_id)
    }
}

// ---------------------------------------------------------------------------
// CheckedExecutor (OperationExecutor)
// ---------------------------------------------------------------------------

/// Checked-only execution service.
///
/// This is intentionally narrow: the *only* execution method requires an
/// [`ApprovedOperation`] token and verifies the tool+target binding before
/// dispatch (via `validate_request_binding` inside
/// [`EnforcedDispatcher::dispatch_checked`]). Adapters hold this trait object
/// instead of a raw dispatcher, so unchecked execution is unrepresentable.
pub trait CheckedExecutor: Send + Sync {
    /// Dispatch `request` under `approved`, failing closed on any binding
    /// mismatch (operation, normalized target, surface).
    fn dispatch_checked<'a>(
        &'a self,
        approved: &'a ApprovedOperation,
        request: ToolRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResponse, EggsecError>> + Send + 'a>>;
}

impl CheckedExecutor for EnforcedDispatcher {
    fn dispatch_checked<'a>(
        &'a self,
        approved: &'a ApprovedOperation,
        request: ToolRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResponse, EggsecError>> + Send + 'a>> {
        Box::pin(self.dispatch_checked(approved, request))
    }
}

// ---------------------------------------------------------------------------
// PreflightService
// ---------------------------------------------------------------------------

/// Policy evaluation and approval service.
///
/// All evaluation delegates to [`EnforcementContext`]; implementations must
/// not reimplement scope/CIDR, feature, risk, or manifest-provenance checks.
/// Strict surfaces use [`PreflightService::approve`]; manual surfaces use
/// [`PreflightService::approve_manual`].
pub trait PreflightService: Send + Sync {
    /// Evaluate `descriptor` without executing (shared enforcement path).
    fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome;

    /// Approve for a strict automated surface (`RestApi`, `McpServer`,
    /// `GrpcApi`, `SecurityAgent`, `Ci`). Never honors manual overrides.
    fn approve(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
    ) -> Result<ApprovedOperation, EnforcementError>;

    /// Approve for a manual surface with an optional override.
    fn approve_manual(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        manual_override: Option<&ManualOverride>,
    ) -> Result<ApprovedOperation, EnforcementError>;

    /// Policy preview without execution.
    fn preflight(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        manual_override: Option<&ManualOverride>,
    ) -> PreflightResult;

    /// Borrow the underlying enforcement context (profile/policy/scope).
    fn enforcement(&self) -> &EnforcementContext;
}

impl PreflightService for EnforcementContext {
    fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome {
        EnforcementContext::evaluate(self, descriptor)
    }

    fn approve(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
    ) -> Result<ApprovedOperation, EnforcementError> {
        EnforcementContext::approve(self, surface, descriptor)
    }

    fn approve_manual(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        manual_override: Option<&ManualOverride>,
    ) -> Result<ApprovedOperation, EnforcementError> {
        EnforcementContext::approve_manual(self, surface, descriptor, manual_override)
    }

    fn preflight(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        manual_override: Option<&ManualOverride>,
    ) -> PreflightResult {
        preflight_operation(surface, self, descriptor, manual_override)
    }

    fn enforcement(&self) -> &EnforcementContext {
        self
    }
}

// ---------------------------------------------------------------------------
// EngineServices bundle
// ---------------------------------------------------------------------------

/// Injected bundle consumed by protocol adapters.
///
/// Composition roots (CLI server setup, daemon host, tests) construct this
/// from a concrete registry/dispatcher plus an [`EnforcementContext`].
/// Adapters store the trait objects and never see the concrete types.
///
/// Prefer [`EngineServices::with_executor`] when a fake executor is needed in
/// tests; prefer [`EngineServices::new`] in production composition roots.
#[derive(Clone)]
pub struct EngineServices {
    catalog: Arc<dyn OperationCatalog>,
    executor: Arc<dyn CheckedExecutor>,
    enforcement: EnforcementContext,
}

impl EngineServices {
    /// Production constructor: builds a checked executor from `registry`.
    ///
    /// This is the *only* place where adapter-facing code is allowed to build
    /// an executor from a concrete registry. Adapters themselves receive the
    /// resulting trait object.
    pub fn new(registry: crate::tool::ToolRegistry, enforcement: EnforcementContext) -> Self {
        let dispatcher = EnforcedDispatcher::new(crate::tool::ToolDispatcher::new(registry));
        Self {
            catalog: Arc::new(StaticOperationCatalog),
            executor: Arc::new(dispatcher),
            enforcement,
        }
    }

    /// Injection constructor for tests and custom composition roots.
    pub fn with_executor(
        executor: Arc<dyn CheckedExecutor>,
        enforcement: EnforcementContext,
    ) -> Self {
        Self {
            catalog: Arc::new(StaticOperationCatalog),
            executor,
            enforcement,
        }
    }

    /// Full injection constructor (custom catalog + executor).
    pub fn with_parts(
        catalog: Arc<dyn OperationCatalog>,
        executor: Arc<dyn CheckedExecutor>,
        enforcement: EnforcementContext,
    ) -> Self {
        Self {
            catalog,
            executor,
            enforcement,
        }
    }

    /// Read-only operation catalog.
    pub fn catalog(&self) -> &Arc<dyn OperationCatalog> {
        &self.catalog
    }

    /// Checked-only executor. No raw dispatch is exposed.
    pub fn executor(&self) -> &Arc<dyn CheckedExecutor> {
        &self.executor
    }

    /// Enforcement context (profile/policy/scope) for evaluate/approve.
    pub fn enforcement(&self) -> &EnforcementContext {
        &self.enforcement
    }

    /// Evaluate `descriptor` through the shared enforcement path.
    pub fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome {
        self.enforcement.evaluate(descriptor)
    }

    /// Strict approval (automated surfaces). Fails closed on Warn/Confirm/Deny.
    pub fn approve(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
    ) -> Result<ApprovedOperation, EnforcementError> {
        self.enforcement.approve(surface, descriptor)
    }

    /// Checked dispatch under a previously issued approval token.
    pub async fn dispatch_checked(
        &self,
        approved: &ApprovedOperation,
        request: ToolRequest,
    ) -> Result<ToolResponse, EggsecError> {
        self.executor.dispatch_checked(approved, request).await
    }

    /// Policy preview without execution.
    pub fn preflight(
        &self,
        surface: ExecutionSurface,
        descriptor: OperationDescriptor,
        manual_override: Option<&ManualOverride>,
    ) -> PreflightResult {
        preflight_operation(surface, &self.enforcement, descriptor, manual_override)
    }

    /// Canonical metadata lookup (alias-aware).
    pub fn metadata_for_id(&self, id: &str) -> Option<&'static OperationMetadata> {
        self.catalog.metadata_for_id(id)
    }

    /// Underlying policy decision for observability (no authorization here).
    pub fn decision_for(&self, descriptor: &OperationDescriptor) -> PolicyDecision {
        self.evaluate(descriptor).decision().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutionPolicy, LoadedScope, OperationMode, OperationRisk};

    #[test]
    fn static_catalog_resolves_aliases_like_engine() {
        let catalog = StaticOperationCatalog;
        // `scan` is a canonical alias for `scan-ports` (Phase C compat).
        assert!(catalog.matches_tool_id("scan", "scan-ports"));
        assert!(catalog.metadata_for_id("scan-ports").is_some());
        assert!(!catalog.all_metadata().is_empty());
    }

    #[test]
    fn engine_services_evaluate_delegates_to_enforcement() {
        let enforcement = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let registry = crate::tool::ToolRegistry::new();
        let services = EngineServices::new(registry, enforcement.clone());
        let descriptor = OperationDescriptor::new(
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
        let via_services = services.evaluate(&descriptor);
        let via_context = enforcement.evaluate(&descriptor);
        assert_eq!(
            via_services.decision().allowed,
            via_context.decision().allowed
        );
    }
}
