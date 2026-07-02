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
            let task_result = eggsec::dispatch::dispatch_inner(request, progress_tx)
                .await
                .map_err(|e| {
                    RuntimeError::DispatchFailed(format!("task execution failed: {}", e))
                })?;

            // Convert to envelope before sending, since TaskResult is not Clone.
            let envelope = task_result_to_envelope(&task_result);

            // Send typed result through the channel for TUI rendering.
            let _ = result_tx.send(task_result).await;

            // Return a structured envelope so non-TUI frontends
            // (daemon, REST, MCP) also receive useful completion data.
            Ok(TaskOutcome::Result(envelope))
        })
    }
}

/// Convert an `eggsec::dispatch::TaskResult` into a `TaskResultEnvelope`.
///
/// Extracts a kind discriminator and summary from each variant. Domain-specific
/// payloads are returned as empty JSON — the TUI uses typed `TaskResult`
/// channels for rich rendering. Non-TUI frontends get the kind + summary.
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
    use eggsec::dispatch::TaskResult;
    use eggsec_runtime::event::TaskOutcome;

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

    #[test]
    fn envelope_error_has_kind_and_message() {
        let result = TaskResult::Error("connection refused".into());
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "error");
        assert_eq!(envelope.summary.as_deref(), Some("connection refused"));
    }

    #[test]
    fn envelope_task_outcome_is_result_variant() {
        let result = TaskResult::Error("test".into());
        let envelope = task_result_to_envelope(&result);
        let outcome = TaskOutcome::Result(envelope);
        assert!(matches!(outcome, TaskOutcome::Result(_)));
        if let TaskOutcome::Result(env) = outcome {
            assert_eq!(env.kind, "error");
            assert!(env.summary.is_some());
        }
    }

    #[test]
    fn envelope_packet_capture_has_count() {
        let result = TaskResult::PacketCapture {
            packets_captured: 42,
            output_file: Some("/tmp/capture.pcap".into()),
        };
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "packet-capture");
        assert!(envelope.summary.is_some());
        assert!(envelope.summary.unwrap().contains("42"));
    }

    #[test]
    fn envelope_packet_traceroute_has_hop_count() {
        let result = TaskResult::PacketTraceroute { hops: vec![] };
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "traceroute");
        assert!(envelope.summary.is_some());
        assert!(envelope.summary.unwrap().contains("0 hops"));
    }

    #[test]
    fn envelope_packet_send_has_counts() {
        let result = TaskResult::PacketSend {
            packets_sent: 10,
            bytes_sent: 640,
        };
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "packet-send");
        assert!(envelope.summary.is_some());
        let summary = envelope.summary.unwrap();
        assert!(summary.contains("10 packets"));
        assert!(summary.contains("640 bytes"));
    }

    #[test]
    fn envelope_waf_stress_has_count() {
        let result = TaskResult::WafStress(vec![]);
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "waf-stress");
        assert!(envelope.summary.is_some());
        assert!(envelope.summary.unwrap().contains("0 bypasses"));
    }

    #[test]
    fn envelope_artifacts_are_empty_by_default() {
        let result = TaskResult::Error("test".into());
        let envelope = task_result_to_envelope(&result);
        assert!(envelope.artifacts.is_empty());
    }

    #[test]
    fn envelope_payload_is_empty_json() {
        let result = TaskResult::Error("test".into());
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.payload, serde_json::json!({}));
    }

    #[test]
    fn envelope_port_scan_has_port_count() {
        let result = TaskResult::PortScan(eggsec::scanner::PortScanResults {
            host: "10.0.0.1".into(),
            ports_scanned: 1000,
            open_ports: vec![],
            duration_ms: 500,
            spoof_stats: None,
        });
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "port-scan");
        assert!(envelope.summary.unwrap().contains("1000"));
    }

    #[test]
    fn envelope_recon_has_target() {
        let result = TaskResult::Recon(eggsec::recon::FullReconResult {
            target: "example.com".into(),
            ..Default::default()
        });
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "recon");
        assert!(envelope.summary.unwrap().contains("example.com"));
    }

    #[test]
    fn envelope_load_test_has_request_count() {
        use rustc_hash::FxHashMap;
        let result = TaskResult::LoadTest(eggsec::loadtest::metrics::LoadTestResults {
            target_url: "http://target.local".into(),
            total_requests: 500,
            successful_requests: 490,
            failed_requests: 10,
            total_duration_ms: 10000,
            requests_per_second: 50.0,
            latency_min_ms: 1.0,
            latency_max_ms: 200.0,
            latency_mean_ms: 10.0,
            latency_p50_ms: 5.0,
            latency_p90_ms: 30.0,
            latency_p95_ms: 50.0,
            latency_p99_ms: 150.0,
            status_codes: FxHashMap::default(),
            errors: vec![],
        });
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "load-test");
        assert!(envelope.summary.unwrap().contains("500"));
    }

    #[test]
    fn envelope_graphql_has_finding_count() {
        let result = TaskResult::GraphQl(eggsec::dispatch::GraphQlResults {
            target: "http://gql.local".into(),
            introspection_enabled: true,
            depth_limit_bypassed: false,
            alias_overload_vulnerable: false,
            injection_findings: vec!["sql-injection".into()],
            total_requests: 20,
            errors: 0,
            duration_ms: 1000,
        });
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "graphql");
        assert!(envelope.summary.unwrap().contains("1"));
    }

    #[test]
    fn envelope_fingerprint_has_service_count() {
        let result = TaskResult::Fingerprint(eggsec::scanner::FingerprintResults {
            host: "10.0.0.1".into(),
            ports_scanned: 100,
            services_identified: 5,
            duration_ms: 300,
            results: vec![],
        });
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "fingerprint");
        assert!(envelope.summary.unwrap().contains("5"));
    }

    #[test]
    fn envelope_endpoint_scan_has_endpoint_count() {
        let result = TaskResult::EndpointScan(eggsec::scanner::EndpointScanResults {
            base_url: "http://api.local".into(),
            endpoints_scanned: 50,
            endpoints_found: 12,
            interesting_findings: 3,
            duration_ms: 2000,
            results: vec![],
        });
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "endpoint-scan");
        assert!(envelope.summary.unwrap().contains("12"));
    }

    #[test]
    fn envelope_waf_detection_has_name() {
        let result = TaskResult::WafDetection(eggsec::waf::WafDetectionResult {
            waf_name: Some("Cloudflare".into()),
            confidence: 95,
            request_error: None,
            matched_headers: vec![],
            matched_cookies: vec![],
            matched_patterns: vec![],
            server_header: None,
            status_code: 403,
        });
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "waf");
        assert!(envelope.summary.unwrap().contains("Cloudflare"));
    }

    #[test]
    fn envelope_waf_detection_unknown_name() {
        let result = TaskResult::WafDetection(eggsec::waf::WafDetectionResult {
            waf_name: None,
            confidence: 0,
            request_error: None,
            matched_headers: vec![],
            matched_cookies: vec![],
            matched_patterns: vec![],
            server_header: None,
            status_code: 200,
        });
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "waf");
        assert!(envelope.summary.unwrap().contains("unknown"));
    }

    #[test]
    fn envelope_auth_has_finding_count() {
        let result = TaskResult::Auth(eggsec::auth::AuthTestReport {
            target: "http://auth.local".into(),
            tests_run: vec![],
            brute_force: None,
            credential_stuffing: None,
            lockout_detection: None,
            rate_limit: None,
            mfa: None,
            session: None,
            timing: None,
            password_policy: None,
            total_attempts: 0,
            findings: vec![],
        });
        let envelope = task_result_to_envelope(&result);
        assert_eq!(envelope.kind, "auth-test");
        assert!(envelope.summary.unwrap().contains("0"));
    }
}
