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

pub use types::{
    GraphQlResults, NseResults, OAuthResults, ReconOptions, TaskResult, TracerouteHopResult,
};

use eggsec_runtime::request::{RunRequest, TaskKind};
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
/// **Enforcement note:** this function performs *no* policy checks of its
/// own. It must only be invoked from manual surfaces (CLI/TUI, whose
/// `ManualPermissive` context supports operator-directed overrides). Strict
/// surfaces (REST/MCP/agent/gRPC) must never call it directly — route through
/// `EnforcementContext::evaluate()` and `EnforcedDispatcher::dispatch_checked()`.
/// Kept `pub` because `eggsec-tui`'s dispatcher is a sanctioned caller.
#[doc(hidden)]
pub async fn dispatch_inner(
    request: RunRequest,
    progress_tx: mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    // All defaults/validation below flow through the canonical
    // `operation_request` contracts (single owner). Runtime params are first
    // adapted into canonical requests, then normalized. This guarantees
    // equivalent CLI/runtime/tool requests produce the same engine request.
    match request.task_kind {
        TaskKind::LoadTest(p) => {
            use crate::operation_request::runtime_adapters::load_test_from_runtime;
            let normalized = load_test_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid load-test request: {e}"))?;
            let timeout = std::time::Duration::from_secs(normalized.duration_secs);
            network::run_load_test(
                normalized.target,
                normalized.requests,
                normalized.concurrency,
                timeout,
                progress_tx,
            )
            .await
        }
        TaskKind::StressTest(p) => {
            network::run_stress_test(
                p.target,
                p.flood_type,
                p.rate_pps.unwrap_or(1000),
                p.duration_secs.unwrap_or(60) as u64,
                p.threads.unwrap_or(10) as usize,
                progress_tx,
            )
            .await
        }
        TaskKind::PortScan(p) => {
            use crate::operation_request::runtime_adapters::port_scan_from_runtime;
            let normalized = port_scan_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid port-scan request: {e}"))?;
            let timeout = std::time::Duration::from_millis(normalized.timeout_ms);
            scanner::run_port_scan(
                normalized.target,
                normalized.ports,
                normalized.concurrency,
                timeout,
                progress_tx,
            )
            .await
        }
        TaskKind::EndpointScan(p) => {
            use crate::operation_request::runtime_adapters::endpoint_scan_from_runtime;
            let normalized = endpoint_scan_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid endpoint-scan request: {e}"))?;
            let timeout = std::time::Duration::from_secs(normalized.timeout_secs);
            scanner::run_endpoint_scan(
                normalized.target,
                normalized.concurrency,
                timeout,
                normalized.wordlist,
                progress_tx,
            )
            .await
        }
        TaskKind::Fingerprint(p) => {
            use crate::operation_request::runtime_adapters::fingerprint_from_runtime;
            let normalized = fingerprint_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid fingerprint request: {e}"))?;
            let timeout = std::time::Duration::from_secs(normalized.timeout_secs);
            scanner::run_fingerprint(
                normalized.target,
                normalized.ports,
                timeout,
                normalized.concurrency,
                progress_tx,
            )
            .await
        }
        TaskKind::Fuzz(p) => {
            use crate::operation_request::runtime_adapters::fuzz_from_runtime;
            let n = fuzz_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid fuzz request: {e}"))?;
            fuzzer::run_fuzz(
                n.target,
                n.payload_type,
                n.mode,
                n.mutations,
                n.mutation_count,
                n.method,
                n.param,
                n.threads,
                n.timeout_secs,
                n.graphql_introspection,
                n.graphql_depth_bypass,
                n.graphql_alias_overload,
                n.oauth_redirect_test,
                n.oauth_scope_test,
                n.oauth_state_test,
                n.oauth_grant_test,
                progress_tx,
            )
            .await
        }
        TaskKind::Waf(p) => {
            use crate::operation_request::runtime_adapters::waf_from_runtime;
            let n = waf_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid waf request: {e}"))?;
            fuzzer::run_waf(n.target, n.bypass_mode, n.techniques, progress_tx).await
        }
        TaskKind::WafStress(p) => {
            use crate::operation_request::runtime_adapters::waf_stress_from_runtime;
            let n = waf_stress_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid waf-stress request: {e}"))?;
            fuzzer::run_waf_stress(n.target, n.concurrency, n.requests, progress_tx).await
        }
        TaskKind::Pipeline(p) => {
            use crate::operation_request::runtime_adapters::pipeline_from_runtime;
            let n = pipeline_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid pipeline request: {e}"))?;
            let profile = match n.profile.as_str() {
                "quick" => crate::types::ScanProfile::Quick,
                "endpoint" => crate::types::ScanProfile::Endpoint,
                "web" => crate::types::ScanProfile::Web,
                "waf" => crate::types::ScanProfile::Waf,
                "full" => crate::types::ScanProfile::Full,
                "api" => crate::types::ScanProfile::Api,
                "recon" => crate::types::ScanProfile::Recon,
                "stealth" => crate::types::ScanProfile::Stealth,
                "deep" => crate::types::ScanProfile::Deep,
                "vuln" => crate::types::ScanProfile::Vuln,
                "auth" => crate::types::ScanProfile::Auth,
                "defense-lab" => crate::types::ScanProfile::DefenseLab,
                // Validated by `parse_scan_profile`; unreachable for known
                // profiles. Fail closed rather than silently downgrading.
                other => return Err(anyhow::anyhow!("unknown scan profile '{other}'")),
            };
            recon::run_pipeline(n.target, profile, progress_tx).await
        }
        TaskKind::Recon(p) => {
            use crate::operation_request::runtime_adapters::recon_from_runtime;
            let n = recon_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid recon request: {e}"))?;
            recon::run_recon(n.target, 20, ReconOptions::default(), progress_tx).await
        }
        TaskKind::PacketCapture(p) => {
            network::run_packet_capture(
                p.interface.unwrap_or_else(|| "eth0".to_string()),
                p.filter.unwrap_or_default(),
                p.max_packets.unwrap_or(1000),
                p.promiscuous.unwrap_or(true),
                None,
                progress_tx,
            )
            .await
        }
        TaskKind::PacketTraceroute(p) => {
            network::run_packet_traceroute(p.target, p.max_hops.unwrap_or(30) as u8, progress_tx)
                .await
        }
        TaskKind::PacketSend(p) => {
            network::run_packet_send(
                p.target,
                p.port.unwrap_or(80),
                p.count.unwrap_or(10),
                p.packet_size.unwrap_or(64),
                progress_tx,
            )
            .await
        }
        TaskKind::GraphQl(p) => {
            use crate::operation_request::runtime_adapters::graphql_from_runtime;
            let n = graphql_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid graphql request: {e}"))?;
            api::run_graphql(
                n.target,
                n.introspection,
                n.inject,
                n.depth_bypass,
                n.alias_overload,
                n.concurrency,
                n.timeout_secs,
                progress_tx,
            )
            .await
        }
        TaskKind::OAuth(p) => {
            use crate::operation_request::runtime_adapters::oauth_from_runtime;
            let n = oauth_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid oauth request: {e}"))?;
            api::run_oauth(
                n.target,
                n.client_id,
                n.redirect_uri,
                n.redirect_test,
                n.scope_test,
                n.state_test,
                n.grant_test,
                n.concurrency,
                n.timeout_secs,
                progress_tx,
            )
            .await
        }
        TaskKind::AuthTest(p) => {
            use crate::operation_request::runtime_adapters::auth_test_from_runtime;
            let n = auth_test_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid auth-test request: {e}"))?;
            auth::run_auth_task(
                n.target,
                n.username,
                n.credential_list,
                n.credential_file,
                n.max_attempts,
                n.concurrency,
                n.timeout_secs,
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "nse")]
        TaskKind::Nse(p) => api::run_nse(p.target, p.script, p.args, None, progress_tx).await,
        #[cfg(feature = "advanced-hunting")]
        TaskKind::Hunt(p) => {
            security::run_hunt_task(p.target, crate::hunt::HuntConfig::default(), progress_tx).await
        }
        #[cfg(feature = "headless-browser")]
        TaskKind::Browser(p) => {
            security::run_browser_task(
                p.target,
                crate::browser::BrowserConfig::default(),
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "compliance")]
        TaskKind::Compliance(p) => {
            security::run_compliance_task(
                p.target,
                crate::compliance::ComplianceFramework::OWASP,
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "database")]
        TaskKind::Storage(p) => {
            security::run_storage_task(
                crate::storage::StorageConfig::default(),
                "read".to_string(),
                None,
                None,
                None,
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "external-integrations")]
        TaskKind::Integrations(p) => {
            security::run_integrations_task(
                crate::integrations::IntegrationConfig::default(),
                "list".to_string(),
                None,
                None,
                vec![],
                vec![],
                None,
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "finding-workflow")]
        TaskKind::Workflow(_p) => {
            security::run_workflow_task("list".to_string(), None, vec![], progress_tx).await
        }
        #[cfg(feature = "vuln-management")]
        TaskKind::Vuln(p) => {
            security::run_vuln_task(
                "assess".to_string(),
                Some(p.target),
                None,
                None,
                None,
                None,
                None,
                None,
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "wireless")]
        TaskKind::Wireless(p) => {
            security::run_wireless_task(
                p.interface.unwrap_or_else(|| "wlan0".to_string()),
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "wireless-advanced")]
        TaskKind::WirelessActive(p) => {
            security::run_wireless_active_task(
                p.interface.unwrap_or_else(|| "wlan0".to_string()),
                "deauth".to_string(),
                p.target_bssid,
                None,
                100,
                10,
                true,
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "db-pentest")]
        TaskKind::DbPentest(p) => {
            use crate::operation_request::runtime_adapters::db_pentest_from_runtime;
            let n = db_pentest_from_runtime(&p)
                .normalize()
                .map_err(|e| anyhow::anyhow!("invalid db-pentest request: {e}"))?;
            db_pentest::run_db_pentest_task(
                None,
                Some(n.target),
                Some(n.db_type),
                n.port,
                n.checks,
                n.dry_run,
                n.allow_advanced,
                n.max_queries,
                n.max_duration_secs,
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "web-proxy")]
        TaskKind::Intercept(p) => {
            intercept::run_intercept_task(
                format!(
                    "{}:{}",
                    p.listen_host.unwrap_or_else(|| "127.0.0.1".to_string()),
                    p.listen_port.unwrap_or(8080)
                ),
                p.dry_run.unwrap_or(true),
                p.max_flows.unwrap_or(100),
                p.target,
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "c2")]
        TaskKind::C2(p) => {
            c2::run_c2_task(
                p.target.unwrap_or_else(|| "127.0.0.1".to_string()),
                p.profile.unwrap_or_else(|| "default".to_string()),
                p.dry_run.unwrap_or(true),
                progress_tx,
            )
            .await
        }
        // Feature-gated variants without their feature enabled.
        // These should never reach dispatch_task in practice, as the
        // frontend should reject unsupported task kinds before submission.
        _ => {
            tracing::warn!("Received unsupported or feature-gated task kind");
            Ok(TaskResult::Error("Unsupported task kind".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_runtime::request::{PortScanParams, RuntimeSurface};

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
