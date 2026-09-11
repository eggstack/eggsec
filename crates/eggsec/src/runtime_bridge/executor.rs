use std::future::Future;
use std::pin::Pin;

use eggsec_runtime::event::{LogLevel, TaskOutcome};
use eggsec_runtime::request::RunRequest;
use eggsec_runtime::{RuntimeError, RuntimeEventSink, RuntimeTaskExecutor, TaskId};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::config::{ExecutionPolicy, LoadedScope};
use crate::dispatch::TaskResult;

use super::bundle::approve_run_request_bundle;

/// Executor adapter that bridges `eggsec_runtime::RuntimeTaskExecutor` to the
/// real Eggsec engine dispatch pipeline.
///
/// This replaces the daemon's `NoopExecutorStub` with actual task execution:
///
/// 1. Uses the **actual** session surface and scope from the runtime context
///    (never hardcoded permissive defaults).
/// 2. Runs enforcement via `approve_run_request()`.
/// 3. Dispatches through the approved bundle
///    (`dispatch_approved_runtime_request` → `execute_approved`, the single
///    canonical execution boundary shared with embedded TUI execution).
/// 4. Converts `TaskResult` → `TaskOutcome` via the single engine-owned
///    envelope mapping for the runtime lifecycle.
///
/// # Trust boundary
///
/// The executor receives an `eggsec_runtime::session::RuntimeExecutionContext` carrying the session's
/// creation surface and scope metadata. For strict surfaces (MCP, REST, gRPC,
/// Agent, CI), the executor **fails closed** if scope cannot be resolved from
/// the stored path. Manual surfaces (CLI, TUI) fall back to permissive defaults
/// when no explicit scope is available.
pub struct EggsecRuntimeExecutor {
    policy: ExecutionPolicy,
}

impl Default for EggsecRuntimeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl EggsecRuntimeExecutor {
    /// Create a new executor with the default permissive policy.
    pub fn new() -> Self {
        Self {
            policy: ExecutionPolicy::default(),
        }
    }

    /// Create a new executor with a custom execution policy.
    pub fn with_policy(policy: ExecutionPolicy) -> Self {
        Self { policy }
    }

    /// Resolve a `LoadedScope` from session scope metadata.
    ///
    /// For strict surfaces, if the scope has a path, it attempts to load from
    /// disk. If no path is available, it returns `None` (caller must fail
    /// closed). For permissive manual surfaces, it falls back to a default
    /// empty scope.
    fn resolve_loaded_scope(
        scope: &Option<eggsec_runtime::SessionScope>,
        surface: eggsec_runtime::RuntimeSurface,
    ) -> Option<LoadedScope> {
        // Determine if this surface honors manual overrides.
        let is_permissive = matches!(
            surface,
            eggsec_runtime::RuntimeSurface::CliManual | eggsec_runtime::RuntimeSurface::TuiManual
        );

        match scope {
            Some(s) if s.is_explicit => {
                if let Some(ref path) = s.path {
                    // Attempt to load scope from the stored path.
                    match crate::config::load_scope(Some(path.as_str())) {
                        Ok(raw_scope) => Some(LoadedScope::explicit(
                            raw_scope,
                            crate::config::ScopeSource::ConfigFile,
                            Some(path.clone()),
                        )),
                        Err(e) => {
                            tracing::warn!(
                                path = %path,
                                error = %e,
                                "Failed to load scope from stored path; failing closed for strict surface"
                            );
                            None
                        }
                    }
                } else {
                    // Explicit scope but no path — cannot resolve.
                    tracing::warn!(
                        "Session has explicit scope metadata but no path; cannot resolve LoadedScope"
                    );
                    None
                }
            }
            Some(_) | None => {
                // Non-explicit or no scope. Permissive surfaces get default_empty.
                if is_permissive {
                    Some(LoadedScope::default_empty())
                } else {
                    // Strict surface without explicit scope — fail closed.
                    tracing::warn!(
                        surface = ?surface,
                        "Strict surface with no explicit scope; failing closed"
                    );
                    None
                }
            }
        }
    }

    /// Convert a `TaskResult` into a `TaskOutcome` for the runtime lifecycle.
    ///
    /// Phase 3 closure: thin delegation to the single engine-owned
    /// [`crate::dispatch::task_result_envelope`] mapping. No parallel
    /// `TaskResult` match here.
    fn task_result_to_outcome(result: &TaskResult) -> TaskOutcome {
        TaskOutcome::Result(crate::dispatch::task_result_envelope(result))
    }
}

impl RuntimeTaskExecutor for EggsecRuntimeExecutor {
    fn execute(
        &self,
        _task_id: TaskId,
        request: RunRequest,
        context: eggsec_runtime::RuntimeExecutionContext,
        sink: RuntimeEventSink,
        cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>> {
        let policy = self.policy.clone();
        let surface = context.surface.clone();

        Box::pin(async move {
            // Check cancellation before starting.
            if cancel.is_cancelled() {
                return Err(RuntimeError::DispatchFailed("task cancelled".into()));
            }

            // Validate surface is not Unknown.
            if surface == eggsec_runtime::RuntimeSurface::Unknown {
                return Err(RuntimeError::DispatchFailed(
                    "session surface is Unknown; cannot dispatch".into(),
                ));
            }

            // Resolve LoadedScope from session context.
            let loaded_scope = Self::resolve_loaded_scope(&context.scope, surface.clone())
                .ok_or_else(|| {
                    RuntimeError::DispatchFailed(format!(
                        "strict surface {:?} requires explicit scope but none was resolved",
                        surface
                    ))
                })?;

            // Run enforcement — get ApprovedRunRequest or fail.
            let bundle = approve_run_request_bundle(surface, policy, loaded_scope, request, None)
                .map_err(|e| match e {
                super::surface::RuntimeBridgeError::UnsupportedTaskKind { .. } => {
                    RuntimeError::UnsupportedTaskKind
                }
                super::surface::RuntimeBridgeError::ManualOverrideRejected { .. } => {
                    RuntimeError::DispatchFailed(format!("manual override rejected: {e}"))
                }
                _ => RuntimeError::DispatchFailed(format!("enforcement denied: {e}")),
            })?;

            // Log the approved operation for audit.
            sink.log(
                LogLevel::Info,
                format!(
                    "dispatching {} (target: {:?})",
                    bundle.approved().descriptor().operation,
                    bundle.approved().descriptor().target,
                ),
            );

            // Dispatch through the engine, racing against cancellation via
            // the shared primitive (same as the embedded TUI adapter, so
            // terminal/cancel semantics match by construction). The progress
            // forwarder drains on success and is aborted when cancellation
            // wins, so no detached task survives.
            let (progress_tx, mut progress_rx) = mpsc::channel(16);

            // Spawn a task to forward dispatch progress to runtime events.
            let sink_clone = sink.clone();
            let progress_forwarder = tokio::spawn(async move {
                while let Some((completed, total)) = progress_rx.recv().await {
                    sink_clone.progress(completed, Some(total), None);
                }
            });

            let dispatch_fut =
                super::bundle::dispatch_approved_runtime_request(bundle, progress_tx);
            let dispatch_mapped = async move {
                dispatch_fut.await.map_err(|e| {
                    RuntimeError::DispatchFailed(format!("task execution failed: {e}"))
                })
            };
            let task_result = match eggsec_runtime::race_with_cancel(dispatch_mapped, cancel).await
            {
                Ok(task_result) => task_result,
                Err(cancelled) => {
                    progress_forwarder.abort();
                    return Err(cancelled);
                }
            };

            // Wait for the forwarder to drain remaining progress messages.
            if let Err(error) = progress_forwarder.await {
                tracing::warn!(?error, "runtime progress forwarder failed");
            }

            // Convert TaskResult → TaskOutcome.
            let outcome = Self::task_result_to_outcome(&task_result);

            Ok(outcome)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_result_port_scan_to_outcome() {
        let result = TaskResult::PortScan(crate::scanner::PortScanResults {
            host: "10.0.0.1".into(),
            ports_scanned: 1000,
            open_ports: vec![],
            total_open_ports: 0,
            results_truncated: false,
            duration_ms: 500,
            spoof_stats: None,
        });
        let outcome = EggsecRuntimeExecutor::task_result_to_outcome(&result);
        match outcome {
            TaskOutcome::Result(envelope) => {
                assert_eq!(envelope.kind, "port-scan");
                assert!(envelope.summary.unwrap().contains("1000"));
            }
            other => panic!("expected TaskOutcome::Result, got {:?}", other),
        }
    }

    #[test]
    fn task_result_error_to_outcome() {
        let result = TaskResult::Error("something went wrong".into());
        let outcome = EggsecRuntimeExecutor::task_result_to_outcome(&result);
        match outcome {
            TaskOutcome::Result(envelope) => {
                assert_eq!(envelope.kind, "error");
                assert_eq!(envelope.summary.as_deref(), Some("something went wrong"));
            }
            other => panic!("expected TaskOutcome::Result, got {:?}", other),
        }
    }

    #[test]
    fn task_result_load_test_to_outcome() {
        let result = TaskResult::LoadTest(crate::loadtest::metrics::LoadTestResults {
            target_url: "http://example.com".into(),
            total_requests: 5000,
            successful_requests: 4900,
            failed_requests: 100,
            total_duration_ms: 30000,
            requests_per_second: 150.0,
            latency_min_ms: 10.0,
            latency_max_ms: 500.0,
            latency_mean_ms: 45.2,
            latency_p50_ms: 38.0,
            latency_p90_ms: 90.0,
            latency_p95_ms: 120.0,
            latency_p99_ms: 250.0,
            status_codes: Default::default(),
            errors: vec![],
        });
        let outcome = EggsecRuntimeExecutor::task_result_to_outcome(&result);
        match outcome {
            TaskOutcome::Result(envelope) => {
                assert_eq!(envelope.kind, "load-test");
                assert!(envelope.summary.unwrap().contains("5000"));
            }
            other => panic!("expected TaskOutcome::Result, got {:?}", other),
        }
    }
}
