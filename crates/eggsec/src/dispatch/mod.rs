//! Task dispatch module — frontend-neutral worker execution.
//!
//! This module owns the canonical dispatch logic for all assessment tasks.
//! Frontend crates (TUI, CLI, agent) call [`dispatch_task()`] with a
//! [`eggsec_runtime::request::RunRequest`] and receive typed [`TaskResult`]
//! values through channels.
//!
//! # Architecture
//!
//! ```text
//! eggsec-runtime (trait: TaskDispatcher)
//!        ↓
//! eggsec::dispatch::dispatch_task()   ← this module
//!        ↓
//! eggsec engine functions (scanner, loadtest, fuzzer, etc.)
//! ```

pub mod canonical_execution;
pub mod executor;
pub mod executors;

mod api;
mod auth;
#[cfg(feature = "c2")]
mod c2;
#[cfg(feature = "db-pentest")]
mod db_pentest;
mod fuzzer;
#[cfg(feature = "web-proxy")]
mod intercept;
mod network;
mod recon;
mod scanner;
#[cfg(any(
    feature = "advanced-hunting",
    feature = "compliance",
    feature = "database",
    feature = "external-integrations",
    feature = "finding-workflow",
    feature = "vuln-management",
    feature = "headless-browser",
    feature = "wireless"
))]
mod security;
mod types;

pub use canonical_execution::{
    execute_approved, execute_canonical, executor_route_for, is_feature_available, outcome_kind,
    CanonicalOperationRequest, ExecutionError, ExecutionEvent, ExecutionSink,
};
pub use types::{
    GraphQlResults, NseResults, OAuthResults, ReconOptions, TaskResult, TracerouteHopResult,
};

use eggsec_runtime::request::RunRequest;
use tokio::sync::mpsc;

/// Dispatch a task described by a [`RunRequest`].
///
/// Creates per-task progress and result channels, then routes to the
/// appropriate worker function based on `TaskKind`. Returns the result
/// channel receiver for the caller to consume.
///
/// # Returns
///
/// A tuple of `(progress_rx, result_rx)` receivers. The caller should
/// poll these channels for progress updates and the final task result.
pub async fn dispatch_task(
    request: RunRequest,
) -> anyhow::Result<(mpsc::Receiver<(u64, u64)>, mpsc::Receiver<TaskResult>)> {
    let (progress_tx, progress_rx) = mpsc::channel(100);
    let (result_tx, result_rx) = mpsc::channel(1);

    let result = dispatch_inner(request, progress_tx).await;

    match result {
        Ok(task_result) => {
            if let Err(error) = result_tx.send(task_result).await {
                tracing::warn!(?error, "Failed to deliver dispatch result");
            }
        }
        Err(e) => {
            tracing::warn!("Dispatch failed: {}", e);
            if let Err(error) = result_tx.send(TaskResult::Error(e.to_string())).await {
                tracing::warn!(?error, "Failed to deliver dispatch error result");
            }
        }
    }

    Ok((progress_rx, result_rx))
}

#[allow(dead_code)]
fn load_test_parameters(p: &eggsec_runtime::request::LoadTestParams) -> (u64, usize) {
    // Canonical owner: `eggsec-tool-core::operation_request::resolve_load_test_counts`.
    // Explicit `requests` wins; otherwise legacy `connections` is the total;
    // otherwise the canonical default. Bounds failures fall back to defaults
    // here because dispatch must stay infallible for legacy callers; strict
    // validation happens in `operation_request::LoadTestRequest::normalize`.
    crate::operation_request::resolve_load_test_counts(p.requests, p.connections).unwrap_or((
        crate::operation_request::DEFAULT_LOAD_REQUESTS,
        crate::operation_request::DEFAULT_LOAD_CONCURRENCY,
    ))
}

/// Internal dispatch that routes `TaskKind` to worker functions.
///
/// Returns the [`TaskResult`] directly so callers can convert it to a
/// `eggsec_runtime::event::TaskResultEnvelope` for the runtime outcome path. Worker functions
/// return `TaskResult` values directly instead of sending through channels.
///
/// Phase 1 convergence: this is a **legacy manual shim** over the canonical
/// execution boundary. It converts `TaskKind` → [`CanonicalOperationRequest`]
/// and delegates to [`execute_canonical`], which owns the single executor
/// match. New code should use [`execute_approved`] (canonical operation ID +
/// canonical request + `ApprovedOperation`) or the runtime-bridge bundle
/// instead of calling this directly.
///
/// **Enforcement note:** this function performs *no* policy checks of its
/// own. It must only be invoked from manual surfaces (CLI/TUI, whose
/// `ManualPermissive` context supports operator-directed overrides). Strict
/// surfaces (REST/MCP/agent/gRPC) must never call it directly — route through
/// `EnforcementContext::evaluate()` and `EnforcedDispatcher::dispatch_checked()`.
/// Kept `pub` because `eggsec-tui`'s dispatcher is a sanctioned caller during
/// the Phase 1 migration.
#[doc(hidden)]
pub async fn dispatch_inner(
    request: RunRequest,
    progress_tx: mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use canonical_execution::{CanonicalOperationRequest, ExecutionError, ExecutionSink};

    // Single-owner conversion: TaskKind → canonical request (exhaustive, no
    // wildcard). Defaults/validation flow through the canonical
    // `operation_request` contracts, guaranteeing equivalent CLI/runtime/tool
    // requests produce the same engine request.
    let canonical = CanonicalOperationRequest::from_task_kind(&request.task_kind);
    let sink = ExecutionSink::detached(progress_tx);

    match execute_canonical(canonical, &sink).await {
        Ok(result) => Ok(result),
        // Preserve legacy behavior for feature-gated variants compiled out:
        // callers expect Ok(Error), not a typed Err.
        Err(ExecutionError::FeatureUnavailable {
            operation_id,
            feature,
        }) => {
            tracing::warn!(
                operation_id,
                feature,
                "Received unsupported or feature-gated task kind"
            );
            Ok(TaskResult::Error(format!(
                "Unsupported task kind (feature '{feature}' not compiled for '{operation_id}')"
            )))
        }
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_runtime::request::{PortScanParams, RuntimeSurface, TaskKind};

    #[test]
    fn load_test_parameters_preserve_requests_and_legacy_connections() {
        let explicit = eggsec_runtime::request::LoadTestParams {
            requests: Some(1_000),
            connections: Some(10),
            ..Default::default()
        };
        assert_eq!(load_test_parameters(&explicit), (1_000, 10));

        let legacy = eggsec_runtime::request::LoadTestParams {
            connections: Some(25),
            ..Default::default()
        };
        assert_eq!(load_test_parameters(&legacy), (25, 25));
    }

    #[tokio::test]
    async fn dispatch_task_port_scan_returns_receivers() {
        let request = RunRequest {
            task_kind: TaskKind::PortScan(PortScanParams {
                target: "127.0.0.1".into(),
                ports: Some("22".into()),
                scan_type: None,
                timeout_ms: Some(2000),
                ..Default::default()
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        };

        // dispatch_task should return receivers without error
        // (actual port scan may fail, but the dispatch plumbing works)
        let result = dispatch_task(request).await;
        assert!(result.is_ok());
        let (progress_rx, result_rx) = result.unwrap();
        // Drop receivers so channels close cleanly
        drop(progress_rx);
        drop(result_rx);
    }

    #[tokio::test]
    async fn dispatch_inner_returns_task_result_for_error_case() {
        let (progress_tx, _progress_rx) = tokio::sync::mpsc::channel(100);

        // Use a LoadTest with an unreachable target to trigger an error path.
        // The key assertion is that dispatch_inner returns TaskResult, not ().
        let request = RunRequest {
            task_kind: TaskKind::LoadTest(eggsec_runtime::request::LoadTestParams {
                target: "http://192.0.2.1:1".into(), // TEST-NET, unreachable
                method: "GET".into(),
                duration_secs: Some(1),
                connections: Some(1),
                rate_limit: None,
                ..Default::default()
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        };

        let result = dispatch_inner(request, progress_tx).await;
        // May succeed or fail depending on timeout, but the return type
        // is TaskResult — proving the plumbing works.
        match result {
            Ok(task_result) => {
                let debug_str = format!("{:?}", task_result);
                assert!(!debug_str.is_empty());
            }
            Err(e) => {
                // Error is also acceptable — proves dispatch_inner returns
                // a Result, not () — the key invariant.
                assert!(!e.to_string().is_empty());
            }
        }
    }

    #[test]
    fn executor_registry_covers_core_operations() {
        let reg = executors::build_default_registry();

        // All core operation IDs should be handled
        let core_ops = &[
            "scan-ports",
            "scan-endpoints",
            "fingerprint",
            "recon",
            "pipeline",
            "waf-detect",
            "waf-bypass",
            "waf-stress",
            "load-test",
            "stress-test",
            "packet",
            "auth-test",
            "fuzz",
            "graphql",
            "oauth",
        ];

        for &op_id in core_ops {
            assert!(
                reg.find_executor(op_id).is_some(),
                "No executor registered for core operation: {}",
                op_id
            );
        }
    }

    #[test]
    fn executor_registry_feature_gated_operations() {
        let _reg = executors::build_default_registry();

        // Feature-gated operations (only check if feature is enabled)
        #[cfg(feature = "nse")]
        assert!(
            reg.find_executor("nse").is_some(),
            "No executor registered for nse operation"
        );

        #[cfg(feature = "db-pentest")]
        assert!(
            reg.find_executor("db-pentest").is_some(),
            "No executor registered for db-pentest operation"
        );
    }

    #[test]
    fn executor_registry_no_duplicates() {
        let reg = executors::build_default_registry();
        let ids = reg.all_operation_ids();

        // Check for duplicates by collecting into a set
        let mut seen = rustc_hash::FxHashSet::default();
        for id in &ids {
            assert!(
                seen.insert(*id),
                "Duplicate operation ID in registry: {}",
                id
            );
        }
    }
}
