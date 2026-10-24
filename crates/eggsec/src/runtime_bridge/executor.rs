use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use eggsec_runtime::event::{LogLevel, TaskOutcome};
use eggsec_runtime::request::RunRequest;
use eggsec_runtime::{
    RuntimeError, RuntimeEventSink, RuntimeReportStore, RuntimeTaskExecutor, TaskId,
};
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
/// 5. Derives the canonical report document for a completed result and keeps
///    it in the bounded [`RuntimeReportStore`], so a host can serve the report
///    without the envelope carrying a payload.
///
/// # Trust boundary
///
/// The executor receives an `eggsec_runtime::session::RuntimeExecutionContext` carrying the session's
/// creation surface and scope metadata. For strict surfaces (MCP, REST, gRPC,
/// Agent, CI), the executor **fails closed** if scope cannot be resolved from
/// the stored path. Manual surfaces (CLI, TUI, desktop GUI) fall back to
/// permissive defaults when no explicit scope is available.
pub struct EggsecRuntimeExecutor {
    policy: ExecutionPolicy,
    /// The bounded store of report documents this executor produced.
    reports: Arc<RuntimeReportStore>,
}

impl Default for EggsecRuntimeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl EggsecRuntimeExecutor {
    /// Create a new executor with the default permissive policy.
    pub fn new() -> Self {
        Self::with_policy(ExecutionPolicy::default())
    }

    /// Create a new executor with a custom execution policy.
    pub fn with_policy(policy: ExecutionPolicy) -> Self {
        Self {
            policy,
            reports: Arc::new(RuntimeReportStore::new()),
        }
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
        // Determine if this surface honors manual overrides. The desktop GUI
        // is a manual surface (ADR-0007): a peer of the CLI and the TUI, not
        // a new policy class, so it gets the same permissive default scope.
        let is_permissive = matches!(
            surface,
            eggsec_runtime::RuntimeSurface::CliManual
                | eggsec_runtime::RuntimeSurface::TuiManual
                | eggsec_runtime::RuntimeSurface::DesktopGuiManual
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
    /// The engine features compiled into this build.
    ///
    /// `Some` is correct here: this executor links the engine, so its feature
    /// set is a knowable build fact. `None` is reserved for executors that
    /// link no engine at all.
    fn enabled_features(&self) -> Option<Vec<String>> {
        let enabled = crate::policy_bridge::features::current_enabled_features();
        Some(
            crate::config::ALL_FEATURES
                .iter()
                .filter(|entry| enabled.contains(entry.name))
                .map(|entry| entry.name.to_string())
                .collect(),
        )
    }

    /// The report document derived from a completed task, if one was kept.
    fn report_for(&self, task_id: &TaskId) -> Option<serde_json::Value> {
        self.reports.get(task_id)
    }

    fn execute(
        &self,
        task_id: TaskId,
        request: RunRequest,
        context: eggsec_runtime::RuntimeExecutionContext,
        sink: RuntimeEventSink,
        cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>> {
        let policy = self.policy.clone();
        let surface = context.surface.clone();
        let reports = Arc::clone(&self.reports);

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

            // Derive the canonical report document for the completed result
            // and keep it in the bounded report store. This is a *separate*
            // path from the task envelope: the envelope's payload and
            // artifacts stay empty, and the report body is served from the
            // store instead.
            if let Some(report) = super::report::report_for_result(&task_result) {
                match serde_json::to_value(&report) {
                    Ok(document) => reports.insert(task_id, document),
                    Err(error) => tracing::warn!(
                        %error,
                        "the report document could not be serialized; no report is available for this task"
                    ),
                }
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

    /// The desktop GUI is a manual surface (ADR-0007): it must resolve the
    /// permissive default scope exactly as the CLI and the TUI do. Before the
    /// fix it fell onto the strict branch and every GUI task was denied.
    #[test]
    fn a_desktop_gui_session_receives_the_permissive_default_scope() {
        let scope = EggsecRuntimeExecutor::resolve_loaded_scope(
            &None,
            eggsec_runtime::RuntimeSurface::DesktopGuiManual,
        )
        .expect("the desktop GUI is a manual surface and must not be denied a scope");

        assert_eq!(
            scope.source,
            crate::config::ScopeSource::DefaultEmpty,
            "the permissive default scope is the one the CLI and TUI get"
        );
        assert!(
            scope.scope.allowed_targets.is_empty(),
            "the default scope carries no explicit targets"
        );
    }

    /// A GUI session that carries non-explicit scope metadata resolves the
    /// same way: the metadata is not a manifest, so the permissive default
    /// applies.
    #[test]
    fn a_desktop_gui_session_with_non_explicit_scope_metadata_is_permissive() {
        let session_scope = eggsec_runtime::SessionScope {
            is_explicit: false,
            source: "default-empty".to_string(),
            path: None,
        };

        let scope = EggsecRuntimeExecutor::resolve_loaded_scope(
            &Some(session_scope),
            eggsec_runtime::RuntimeSurface::DesktopGuiManual,
        )
        .expect("the desktop GUI is a manual surface and must not be denied a scope");

        assert_eq!(scope.source, crate::config::ScopeSource::DefaultEmpty);
    }

    /// The strict surfaces stay strict: with no explicit scope they have no
    /// manifest to load, so the executor fails closed and denies the task.
    #[test]
    fn a_strict_surface_without_an_explicit_scope_is_denied() {
        for surface in [
            eggsec_runtime::RuntimeSurface::RestApi,
            eggsec_runtime::RuntimeSurface::Ci,
            eggsec_runtime::RuntimeSurface::McpServer,
            eggsec_runtime::RuntimeSurface::SecurityAgent,
        ] {
            assert!(
                EggsecRuntimeExecutor::resolve_loaded_scope(&None, surface.clone()).is_none(),
                "{surface:?} is a strict surface and must fail closed"
            );
        }
    }

    /// The CLI and TUI manual surfaces keep the permissive scope they always
    /// had; the fix did not move them.
    #[test]
    fn the_cli_and_tui_manual_surfaces_stay_permissive() {
        for surface in [
            eggsec_runtime::RuntimeSurface::CliManual,
            eggsec_runtime::RuntimeSurface::TuiManual,
        ] {
            let scope = EggsecRuntimeExecutor::resolve_loaded_scope(&None, surface)
                .expect("a manual surface resolves the permissive default scope");
            assert_eq!(scope.source, crate::config::ScopeSource::DefaultEmpty);
        }
    }

    /// The engine executor links the engine, so its feature set is knowable
    /// and is published rather than reported as unknown.
    #[test]
    fn the_executor_publishes_its_enabled_features() {
        let executor = EggsecRuntimeExecutor::new();
        let features = executor
            .enabled_features()
            .expect("the engine executor knows its feature set");

        assert!(
            features.contains(&"cli".to_string()),
            "the build under test has the cli feature compiled in"
        );
        assert!(
            !features.contains(&"definitely-not-a-feature".to_string()),
            "only registry features are published"
        );
        for entry in crate::config::ALL_FEATURES {
            assert_eq!(
                features.contains(&entry.name.to_string()),
                entry.enabled,
                "feature '{}' must match the compile-time registry",
                entry.name
            );
        }
    }

    /// A task with no report keeps the store empty rather than storing an
    /// invented document.
    #[test]
    fn an_unknown_task_has_no_report() {
        let executor = EggsecRuntimeExecutor::new();
        assert!(executor.report_for(&TaskId::new()).is_none());
    }

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

            #[cfg(feature = "udp-scan")]
            udp_host_state: None,
            #[cfg(feature = "udp-scan")]
            udp_evidence: None,
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
            error_kinds: Default::default(),
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
