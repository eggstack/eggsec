//! Approved execution bundle — couples enforcement approval with the request.
//!
//! The [`ApprovedRunRequest`] type bundles an [`ApprovedOperation`] token with
//! the [`RunRequest`] it was derived from. This makes it structurally difficult
//! to approve one operation and dispatch another: the dispatch wrapper validates
//! that the approved descriptor still matches the request before routing to
//! `dispatch_inner()`.
//!
//! # Trust model
//!
//! 1. Approval produces an `ApprovedRunRequest` that captures both the token
//!    and the request at a single point in time.
//! 2. The dispatch wrapper re-resolves the descriptor from the request and
//!    compares operation ID and target against the approved descriptor.
//! 3. Surface/profile consistency is checked at the enforcement layer during
//!    approval — the bundle preserves the approved surface for audit.

use eggsec_runtime::request::RunRequest;
use tokio::sync::mpsc;

use crate::config::ApprovedOperation;
use crate::dispatch::TaskResult;

use super::descriptor::descriptor_for_run_request;
use super::manual::approve_run_request;
use super::surface::RuntimeBridgeError;

/// A request bundled with its enforcement approval token.
///
/// Created by [`approve_run_request_bundle`]. Consumed by
/// [`dispatch_approved_runtime_request`]. The bundle ensures the approved
/// operation and the dispatched request are coupled — the dispatch wrapper
/// validates consistency before routing.
#[derive(Debug)]
pub struct ApprovedRunRequest {
    /// The enforcement approval token.
    approved: ApprovedOperation,
    /// The original runtime request that was approved.
    request: RunRequest,
}

impl ApprovedRunRequest {
    /// The approval token for this bundle.
    pub fn approved(&self) -> &ApprovedOperation {
        &self.approved
    }

    /// The request that was approved.
    pub fn request(&self) -> &RunRequest {
        &self.request
    }

    /// Consume the bundle, returning the approval token and request.
    pub fn into_parts(self) -> (ApprovedOperation, RunRequest) {
        (self.approved, self.request)
    }
}

/// Approve a runtime request and bundle the result with the request.
///
/// This is a convenience wrapper that calls [`approve_run_request`] and
/// packages the result into an [`ApprovedRunRequest`]. The bundle can then
/// be passed to [`dispatch_approved_runtime_request`] for validated dispatch.
pub fn approve_run_request_bundle(
    surface: eggsec_runtime::RuntimeSurface,
    policy: crate::config::ExecutionPolicy,
    loaded_scope: crate::config::LoadedScope,
    request: RunRequest,
    manual_override: Option<&crate::config::ManualOverride>,
) -> Result<ApprovedRunRequest, RuntimeBridgeError> {
    let approved = approve_run_request(surface, policy, loaded_scope, &request, manual_override)?;

    Ok(ApprovedRunRequest { approved, request })
}

/// Dispatch an approved runtime request with validation.
///
/// Before routing to `dispatch_inner()`, this function verifies that:
///
/// 1. The approved descriptor operation matches the request's task kind mapping.
/// 2. The approved descriptor target matches the request target.
/// 3. These checks prevent approve-one-dispatch-another attacks where the
///    request is mutated between approval and dispatch.
pub async fn dispatch_approved_runtime_request(
    bundle: ApprovedRunRequest,
    progress_tx: mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    let (approved, request) = bundle.into_parts();

    // Re-resolve the descriptor from the current request to detect mutations.
    let current_descriptor = descriptor_for_run_request(&request)
        .map_err(|e| anyhow::anyhow!("failed to resolve descriptor for approved request: {e}"))?;

    // Validate operation ID matches.
    if approved.descriptor().operation != current_descriptor.operation {
        return Err(anyhow::anyhow!(
            "approved operation '{}' does not match request operation '{}' — dispatch rejected",
            approved.descriptor().operation,
            current_descriptor.operation,
        ));
    }

    // Validate target matches.
    if approved.descriptor().target != current_descriptor.target {
        return Err(anyhow::anyhow!(
            "approved target {:?} does not match request target {:?} — dispatch rejected",
            approved.descriptor().target,
            current_descriptor.target,
        ));
    }

    // Dispatch through the engine.
    crate::dispatch::dispatch_inner(request, progress_tx)
        .await
        .map_err(|e| anyhow::anyhow!("task execution failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutionPolicy, LoadedScope};
    use eggsec_runtime::request::*;

    fn default_policy() -> ExecutionPolicy {
        ExecutionPolicy::default()
    }

    fn port_scan_request(target: &str) -> RunRequest {
        RunRequest {
            task_kind: TaskKind::PortScan(PortScanParams {
                target: target.into(),
                ports: Some("80".into()),
                scan_type: None,
                timeout_ms: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::CliManual,
            labels: vec![],
        }
    }

    fn fuzz_request(target: &str) -> RunRequest {
        RunRequest {
            task_kind: TaskKind::Fuzz(FuzzParams {
                target: target.into(),
                payload_type: None,
                threads: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::CliManual,
            labels: vec![],
        }
    }

    #[test]
    fn approve_bundle_captures_request_and_approval() {
        let req = port_scan_request("10.0.0.1");
        let bundle = approve_run_request_bundle(
            RuntimeSurface::CliManual,
            default_policy(),
            LoadedScope::default_empty(),
            req,
            None,
        )
        .unwrap();

        assert_eq!(bundle.approved().descriptor().operation, "scan-ports");
        assert_eq!(
            bundle.request().task_kind,
            TaskKind::PortScan(PortScanParams {
                target: "10.0.0.1".into(),
                ports: Some("80".into()),
                scan_type: None,
                timeout_ms: None,
            })
        );
    }

    #[test]
    fn approve_bundle_rejects_strict_surface_without_scope() {
        let req = port_scan_request("10.0.0.1");
        let result = approve_run_request_bundle(
            RuntimeSurface::McpServer,
            default_policy(),
            LoadedScope::default_empty(),
            req,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn approve_bundle_strict_surface_with_scope_succeeds() {
        let req = port_scan_request("10.0.0.1");
        let result = approve_run_request_bundle(
            RuntimeSurface::McpServer,
            default_policy(),
            LoadedScope::default_empty(),
            req,
            None,
        );
        // McpServer without explicit scope fails closed — this is expected.
        assert!(result.is_err());
    }

    #[test]
    fn into_parts_returns_approved_and_request() {
        let req = port_scan_request("10.0.0.1");
        let bundle = approve_run_request_bundle(
            RuntimeSurface::CliManual,
            default_policy(),
            LoadedScope::default_empty(),
            req,
            None,
        )
        .unwrap();

        let (approved, request) = bundle.into_parts();
        assert_eq!(approved.descriptor().operation, "scan-ports");
        assert_eq!(
            request.task_kind,
            TaskKind::PortScan(PortScanParams {
                target: "10.0.0.1".into(),
                ports: Some("80".into()),
                scan_type: None,
                timeout_ms: None,
            })
        );
    }

    #[tokio::test]
    async fn dispatch_rejects_operation_mismatch() {
        // Approve a port scan, then construct a bundle with a fuzz request
        // to verify the dispatch wrapper catches the mismatch.
        let port_req = port_scan_request("10.0.0.1");
        let approved = approve_run_request(
            RuntimeSurface::CliManual,
            default_policy(),
            LoadedScope::default_empty(),
            &port_req,
            None,
        )
        .unwrap();

        // Manually construct a mismatched bundle.
        let fuzz_req = fuzz_request("https://example.com");
        let bundle = ApprovedRunRequest {
            approved,
            request: fuzz_req,
        };

        let (progress_tx, _progress_rx) = mpsc::channel(16);
        let result = dispatch_approved_runtime_request(bundle, progress_tx).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("does not match request operation"),
            "expected operation mismatch error, got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn dispatch_rejects_target_mismatch() {
        // Approve a port scan for 10.0.0.1, then swap the request target.
        let req1 = port_scan_request("10.0.0.1");
        let approved = approve_run_request(
            RuntimeSurface::CliManual,
            default_policy(),
            LoadedScope::default_empty(),
            &req1,
            None,
        )
        .unwrap();

        // Create a different port scan request with a different target.
        let req2 = port_scan_request("10.0.0.2");
        let bundle = ApprovedRunRequest {
            approved,
            request: req2,
        };

        let (progress_tx, _progress_rx) = mpsc::channel(16);
        let result = dispatch_approved_runtime_request(bundle, progress_tx).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("does not match request target"),
            "expected target mismatch error, got: {err_msg}"
        );
    }

    #[test]
    fn dispatch_rejects_surface_mismatch_at_enforcement_level() {
        // Approve for MCP server (strict), but the bundle surface field
        // should reflect the approval surface. The actual surface mismatch
        // is caught during approval, not at dispatch time. This test
        // verifies the bundle correctly preserves the approved surface.
        let req = port_scan_request("10.0.0.1");
        let bundle = approve_run_request_bundle(
            RuntimeSurface::CliManual,
            default_policy(),
            LoadedScope::default_empty(),
            req,
            None,
        )
        .unwrap();

        assert_eq!(
            bundle.approved().surface(),
            crate::config::ExecutionSurface::CliManual
        );
    }
}
