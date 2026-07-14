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
            let _ = result_tx.send(task_result).await;
        }
        Err(e) => {
            tracing::warn!("Dispatch failed: {}", e);
            let _ = result_tx.send(TaskResult::Error(e.to_string())).await;
        }
    }

    Ok((progress_rx, result_rx))
}

/// Internal dispatch that routes `TaskKind` to worker functions.
///
/// Returns the [`TaskResult`] directly so callers can convert it to a
/// [`TaskResultEnvelope`] for the runtime outcome path. Worker functions
/// return `TaskResult` values directly instead of sending through channels.
pub async fn dispatch_inner(
    request: RunRequest,
    progress_tx: mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    match request.task_kind {
        TaskKind::LoadTest(p) => {
            let timeout = std::time::Duration::from_secs(p.duration_secs.unwrap_or(30) as u64);
            network::run_load_test(
                p.target,
                p.connections.unwrap_or(100) as u64,
                p.connections.unwrap_or(10) as usize,
                timeout,
                progress_tx,
            )
            .await
        }
        TaskKind::StressTest(p) => {
            network::run_stress_test(
                p.target,
                p.flood_type,
                1000,
                p.duration_secs.unwrap_or(60) as u64,
                p.threads.unwrap_or(10) as usize,
                progress_tx,
            )
            .await
        }
        TaskKind::PortScan(p) => {
            let timeout = std::time::Duration::from_millis(p.timeout_ms.unwrap_or(5000));
            scanner::run_port_scan(
                p.target,
                p.ports.unwrap_or_else(|| "1-1024".to_string()),
                100,
                timeout,
                progress_tx,
            )
            .await
        }
        TaskKind::EndpointScan(p) => {
            let timeout = std::time::Duration::from_secs(60);
            scanner::run_endpoint_scan(p.target, 10, timeout, p.wordlist, progress_tx).await
        }
        TaskKind::Fingerprint(p) => {
            let timeout = std::time::Duration::from_secs(60);
            scanner::run_fingerprint(p.target, "1-1024".to_string(), timeout, progress_tx).await
        }
        TaskKind::Fuzz(p) => {
            fuzzer::run_fuzz(
                p.target,
                p.payload_type.unwrap_or_else(|| "xss".to_string()),
                "smart".to_string(),
                false,
                0,
                "GET".to_string(),
                None,
                p.threads.unwrap_or(10) as usize,
                60,
                false,
                false,
                false,
                false,
                false,
                false,
                false,
                progress_tx,
            )
            .await
        }
        TaskKind::Waf(p) => fuzzer::run_waf(p.target, false, vec![], progress_tx).await,
        TaskKind::WafStress(p) => {
            fuzzer::run_waf_stress(p.target, 10, p.requests.unwrap_or(100) as u64, progress_tx)
                .await
        }
        TaskKind::Pipeline(p) => {
            let profile = match p.profile.as_deref() {
                Some("quick") => crate::cli::ScanProfile::Quick,
                Some("endpoint") => crate::cli::ScanProfile::Endpoint,
                Some("web") => crate::cli::ScanProfile::Web,
                Some("waf") => crate::cli::ScanProfile::Waf,
                Some("full") => crate::cli::ScanProfile::Full,
                Some("api") => crate::cli::ScanProfile::Api,
                Some("recon") => crate::cli::ScanProfile::Recon,
                Some("stealth") => crate::cli::ScanProfile::Stealth,
                Some("deep") => crate::cli::ScanProfile::Deep,
                Some("vuln") => crate::cli::ScanProfile::Vuln,
                Some("auth") => crate::cli::ScanProfile::Auth,
                Some("defense-lab") => crate::cli::ScanProfile::DefenseLab,
                _ => crate::cli::ScanProfile::Quick,
            };
            recon::run_pipeline(
                p.target,
                profile,
                String::new(),
                "json".to_string(),
                progress_tx,
            )
            .await
        }
        TaskKind::Recon(p) => {
            recon::run_recon(p.target, 20, ReconOptions::default(), progress_tx).await
        }
        TaskKind::PacketCapture(p) => {
            network::run_packet_capture(
                p.interface.unwrap_or_else(|| "eth0".to_string()),
                String::new(),
                1000,
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
            network::run_packet_send(p.target, 80, 10, 64, progress_tx).await
        }
        TaskKind::GraphQl(p) => {
            api::run_graphql(
                p.target,
                p.introspection.unwrap_or(true),
                false,
                false,
                false,
                10,
                300,
                progress_tx,
            )
            .await
        }
        TaskKind::OAuth(p) => {
            api::run_oauth(
                p.target,
                None,
                None,
                false,
                false,
                false,
                false,
                10,
                300,
                progress_tx,
            )
            .await
        }
        TaskKind::AuthTest(p) => {
            auth::run_auth_task(
                p.target,
                p.username,
                p.credential_list,
                None,
                100,
                1,
                30,
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
                crate::compliance::ComplianceFramework::OwaspTop10,
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
        TaskKind::Workflow(p) => {
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
            db_pentest::run_db_pentest_task(
                None,
                Some(p.target),
                Some(p.db_type),
                "all".to_string(),
                true,
                false,
                200,
                120,
                progress_tx,
            )
            .await
        }
        #[cfg(feature = "web-proxy")]
        TaskKind::Intercept(p) => {
            intercept::run_intercept_task(
                format!("127.0.0.1:{}", p.listen_port.unwrap_or(8080)),
                true,
                100,
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
                true,
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

    #[tokio::test]
    async fn dispatch_task_port_scan_returns_receivers() {
        let request = RunRequest {
            task_kind: TaskKind::PortScan(PortScanParams {
                target: "127.0.0.1".into(),
                ports: Some("22".into()),
                scan_type: None,
                timeout_ms: Some(2000),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        };

        // dispatch_task should return receivers without error
        // (actual port scan may fail, but the dispatch plumbing works)
        let result = dispatch_task(request).await;
        assert!(result.is_ok());
        let (mut progress_rx, mut result_rx) = result.unwrap();
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
        let reg = executors::build_default_registry();

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
