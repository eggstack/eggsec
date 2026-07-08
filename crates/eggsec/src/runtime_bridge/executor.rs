use std::future::Future;
use std::pin::Pin;

use eggsec_runtime::event::{LogLevel, TaskOutcome, TaskResultEnvelope};
use eggsec_runtime::request::RunRequest;
use eggsec_runtime::{RuntimeError, RuntimeEventSink, RuntimeTaskExecutor, TaskId};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::config::{ApprovedOperation, ExecutionPolicy, LoadedScope};
use crate::dispatch::TaskResult;

use super::manual::approve_run_request;

/// Executor adapter that bridges `eggsec_runtime::RuntimeTaskExecutor` to the
/// real Eggsec engine dispatch pipeline.
///
/// This replaces the daemon's `NoopExecutorStub` with actual task execution:
///
/// 1. Uses permissive enforcement defaults (CliManual surface, empty scope)
///    appropriate for a locally-operated daemon.
/// 2. Runs enforcement via `approve_run_request()`.
/// 3. Dispatches through `eggsec::dispatch::dispatch_inner()`.
/// 4. Converts `TaskResult` → `TaskOutcome` for the runtime lifecycle.
///
/// # Design Note
///
/// This executor does **not** hold a reference to the `Runtime` to avoid
/// circular dependency issues (the Runtime owns the executor). Session
/// context (surface, scope) uses permissive defaults. A future iteration
/// can pass session context through a shared state object if strict surface
/// support is needed for daemon-backed sessions.
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

    /// Convert a `TaskResult` into a `TaskOutcome` for the runtime lifecycle.
    ///
    /// Extracts a kind discriminator and summary from each variant. The full
    /// typed result is not serialized — clients receive structured envelope
    /// metadata (kind + summary) rather than the raw domain payload.
    fn task_result_to_outcome(result: &TaskResult) -> TaskOutcome {
        let (kind, summary) = match result {
            TaskResult::LoadTest(r) => (
                "load-test".into(),
                Some(format!("{} requests completed", r.total_requests)),
            ),
            TaskResult::PortScan(r) => (
                "port-scan".into(),
                Some(format!("{} ports scanned", r.ports_scanned)),
            ),
            TaskResult::EndpointScan(r) => (
                "endpoint-scan".into(),
                Some(format!("{} endpoints found", r.endpoints_found)),
            ),
            TaskResult::Fingerprint(r) => (
                "fingerprint".into(),
                Some(format!("{} services identified", r.services_identified)),
            ),
            TaskResult::WafDetection(r) => (
                "waf".into(),
                Some(format!(
                    "WAF: {}",
                    r.waf_name.as_deref().unwrap_or("unknown")
                )),
            ),
            TaskResult::Recon(r) => ("recon".into(), Some(format!("target: {}", r.target))),
            TaskResult::Fuzz(r) => ("fuzz".into(), Some(format!("{} findings", r.findings))),
            TaskResult::GraphQl(r) => (
                "graphql".into(),
                Some(format!("{} findings", r.injection_findings.len())),
            ),
            TaskResult::OAuth(r) => (
                "oauth".into(),
                Some(format!(
                    "redirect: {}, scope: {}, state: {}",
                    r.redirect_vulnerabilities.len(),
                    r.scope_vulnerabilities.len(),
                    r.state_vulnerabilities.len()
                )),
            ),
            TaskResult::Auth(r) => (
                "auth-test".into(),
                Some(format!("{} findings", r.findings.len())),
            ),
            TaskResult::Pipeline(r) => (
                "pipeline".into(),
                Some(format!("{} stages", r.stage_results.len())),
            ),
            TaskResult::PacketTraceroute { hops } => {
                ("traceroute".into(), Some(format!("{} hops", hops.len())))
            }
            TaskResult::PacketCapture {
                packets_captured, ..
            } => (
                "packet-capture".into(),
                Some(format!("{packets_captured} packets captured")),
            ),
            TaskResult::PacketSend {
                packets_sent,
                bytes_sent,
            } => (
                "packet-send".into(),
                Some(format!("{packets_sent} packets, {bytes_sent} bytes")),
            ),
            TaskResult::WafBypass { bypasses, .. } => (
                "waf-bypass".into(),
                Some(format!("{} bypasses found", bypasses.len())),
            ),
            TaskResult::WafStress(bypasses) => (
                "waf-stress".into(),
                Some(format!("{} bypasses found", bypasses.len())),
            ),
            TaskResult::Error(msg) => ("error".into(), Some(msg.clone())),
            // Feature-gated variants
            #[cfg(feature = "stress-testing")]
            TaskResult::StressTest { target, .. } => {
                ("stress-test".into(), Some(format!("stress-test: {target}")))
            }
            #[cfg(feature = "nse")]
            TaskResult::Nse(r) => (
                "nse".into(),
                Some(format!(
                    "NSE {}: {}",
                    r.script,
                    if r.success { "ok" } else { "failed" }
                )),
            ),
            #[cfg(feature = "advanced-hunting")]
            TaskResult::Hunt(r) => (
                "hunt".into(),
                Some(format!("{} findings", r.findings.len())),
            ),
            #[cfg(feature = "headless-browser")]
            TaskResult::Browser(r) => (
                "browser".into(),
                Some(format!("{} findings", r.findings.len())),
            ),
            #[cfg(feature = "compliance")]
            TaskResult::Compliance(r) => ("compliance".into(), Some(format!("{}", r.framework))),
            #[cfg(feature = "database")]
            TaskResult::Storage => ("storage".into(), Some("storage operation".into())),
            #[cfg(feature = "database")]
            TaskResult::StorageListScans { scans } => (
                "storage".into(),
                Some(format!("{} stored scans", scans.len())),
            ),
            #[cfg(feature = "database")]
            TaskResult::StorageListFindings { findings } => (
                "storage".into(),
                Some(format!("{} stored findings", findings.len())),
            ),
            #[cfg(feature = "external-integrations")]
            TaskResult::Integrations => {
                ("integration".into(), Some("integration operation".into()))
            }
            #[cfg(feature = "external-integrations")]
            TaskResult::IntegrationsCreateIssue { .. } => {
                ("integration".into(), Some("issue created".into()))
            }
            #[cfg(feature = "external-integrations")]
            TaskResult::IntegrationsSearchIssues { issues } => (
                "integration".into(),
                Some(format!("{} issues found", issues.len())),
            ),
            #[cfg(feature = "finding-workflow")]
            TaskResult::Workflow(r) => ("workflow".into(), Some(format!("{}", r.name))),
            #[cfg(feature = "vuln-management")]
            TaskResult::Vuln(r) => (
                "vuln".into(),
                Some(format!("{} findings", r.findings.len())),
            ),
            #[cfg(feature = "wireless")]
            TaskResult::Wireless(r) => (
                "wireless".into(),
                Some(format!("{} networks", r.networks.len())),
            ),
            #[cfg(feature = "wireless-advanced")]
            TaskResult::WirelessActive(r) => (
                "wireless-active".into(),
                Some(format!("{} frames sent", r.frames_sent)),
            ),
            #[cfg(feature = "db-pentest")]
            TaskResult::DbPentest(r) => ("db-pentest".into(), Some(format!("{}", r.db_type))),
            #[cfg(feature = "web-proxy")]
            TaskResult::Intercept(r) => {
                ("intercept".into(), Some(format!("{} flows", r.flows.len())))
            }
            #[cfg(feature = "c2")]
            TaskResult::C2(r) => ("c2".into(), Some(format!("{}", r.profile))),
        };

        TaskOutcome::Result(TaskResultEnvelope {
            kind,
            summary,
            payload: serde_json::json!({}),
            artifacts: vec![],
        })
    }
}

impl RuntimeTaskExecutor for EggsecRuntimeExecutor {
    fn execute(
        &self,
        _task_id: TaskId,
        request: RunRequest,
        sink: RuntimeEventSink,
        _cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>> {
        let policy = self.policy.clone();

        Box::pin(async move {
            // Use permissive defaults for a locally-operated daemon:
            // - CliManual surface: operator-directed, honors overrides
            // - Empty scope: all targets allowed
            let surface = eggsec_runtime::request::RuntimeSurface::CliManual;
            let loaded_scope = LoadedScope::default_empty();

            // Run enforcement — get ApprovedOperation or fail.
            let approved: ApprovedOperation =
                approve_run_request(surface, policy, loaded_scope, &request, None).map_err(
                    |e| match e {
                        super::surface::RuntimeBridgeError::UnsupportedTaskKind { .. } => {
                            RuntimeError::UnsupportedTaskKind
                        }
                        super::surface::RuntimeBridgeError::ManualOverrideRejected { .. } => {
                            RuntimeError::DispatchFailed(format!("manual override rejected: {e}"))
                        }
                        _ => RuntimeError::DispatchFailed(format!("enforcement denied: {e}")),
                    },
                )?;

            // Log the approved operation for audit.
            sink.log(
                LogLevel::Info,
                format!(
                    "dispatching {} (target: {:?})",
                    approved.descriptor().operation,
                    approved.descriptor().target,
                ),
            );

            // Dispatch through the engine.
            let (progress_tx, _progress_rx) = mpsc::channel(16);

            let task_result = crate::dispatch::dispatch_inner(request, progress_tx)
                .await
                .map_err(|e| RuntimeError::DispatchFailed(format!("task execution failed: {e}")))?;

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
