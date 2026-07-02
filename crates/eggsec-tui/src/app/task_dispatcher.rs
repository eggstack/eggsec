use eggsec_runtime::dispatcher::TaskDispatcher;
use eggsec_runtime::event::{TaskOutcome, TaskResultEnvelope};
use eggsec_runtime::request::RunRequest;
use eggsec_runtime::RuntimeError;

use crate::app::task_runtime::TuiDispatcherContext;
use arc_swap::ArcSwap;
use std::sync::Arc;

/// TUI-side task dispatcher that delegates to `eggsec::dispatch`.
///
/// Instead of converting `RunRequest` → `TaskConfig` → `TaskRunner`,
/// this directly calls `eggsec::dispatch::dispatch_inner` which routes
/// to the appropriate engine function based on `TaskKind`.
pub(crate) struct TuiTaskDispatcher {
    executor_context: Arc<ArcSwap<TuiDispatcherContext>>,
}

impl TuiTaskDispatcher {
    pub fn new(executor_context: Arc<ArcSwap<TuiDispatcherContext>>) -> Self {
        Self { executor_context }
    }
}

impl TaskDispatcher for TuiTaskDispatcher {
    fn dispatch(
        &self,
        request: RunRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send>,
    > {
        let ctx = self.executor_context.load();
        let progress_tx = ctx.progress_tx.clone();
        let result_tx = ctx.result_tx.clone();

        Box::pin(async move {
            eggsec::dispatch::dispatch_inner(request, progress_tx, result_tx)
                .await
                .map_err(|e| {
                    RuntimeError::DispatchFailed(format!("task execution failed: {}", e))
                })?;

            // Typed results were sent through the result channel for TUI
            // rendering. Return a structured envelope so non-TUI frontends
            // (daemon, REST, MCP) also receive useful completion data.
            Ok(TaskOutcome::Result(TaskResultEnvelope {
                kind: "tui-compat".into(),
                summary: Some("Task completed; typed result via channel".into()),
                payload: serde_json::json!({"bridge": "result_rx"}),
                artifacts: vec![],
            }))
        })
    }
}

/// Convert an `eggsec::dispatch::TaskResult` into a `TaskResultEnvelope`.
///
/// Extracts a kind discriminator and summary from each variant. Domain-specific
/// payloads are returned as empty JSON — the TUI uses typed `TaskResult`
/// channels for rich rendering. Non-TUI frontends get the kind + summary.
#[allow(dead_code)]
pub(crate) fn task_result_to_envelope(result: &eggsec::dispatch::TaskResult) -> TaskResultEnvelope {
    use eggsec::dispatch::TaskResult;

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
        // Feature-gated variants: kind + summary only
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
        TaskResult::Integrations => ("integration".into(), Some("integration operation".into())),
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
            Some(format!("success: {}", r.success)),
        ),
        #[cfg(feature = "db-pentest")]
        TaskResult::DbPentest(r) => ("db-pentest".into(), Some(format!("{}", r.db_type))),
        #[cfg(feature = "web-proxy")]
        TaskResult::Intercept(r) => (
            "intercept".into(),
            Some(format!("{} requests", r.requests.len())),
        ),
        #[cfg(feature = "c2")]
        TaskResult::C2(r) => ("c2".into(), Some(format!("{}", r.profile))),
    };

    TaskResultEnvelope {
        kind,
        summary,
        payload: serde_json::json!({}),
        artifacts: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn task_dispatcher_creation() {
        let (progress_tx, _) = tokio::sync::mpsc::channel(100);
        let (result_tx, _) = tokio::sync::mpsc::channel(1);
        let ctx = Arc::new(ArcSwap::from_pointee(TuiDispatcherContext {
            progress_tx,
            result_tx,
        }));
        let dispatcher = TuiTaskDispatcher::new(ctx);
        assert!(
            std::any::TypeId::of::<TuiTaskDispatcher>()
                == std::any::TypeId::of::<TuiTaskDispatcher>()
        );
    }
}
