use eggsec_runtime::dispatcher::TaskDispatcher;
use eggsec_runtime::event::TaskOutcome;
use eggsec_runtime::request::RunRequest;
use eggsec_runtime::RuntimeError;
use tokio::sync::mpsc;

use crate::workers::TaskResult;

/// TUI-side task dispatcher that maps `RunRequest` to engine functions.
///
/// The dispatcher holds channel senders for progress and result delivery
/// to the TUI's consumption loop. It converts `RunRequest` → engine calls
/// and sends typed `TaskResult` through the result channel.
pub(crate) struct TuiTaskDispatcher {
    progress_tx: mpsc::Sender<(u64, u64)>,
    result_tx: mpsc::Sender<TaskResult>,
}

impl TuiTaskDispatcher {
    pub fn new(
        progress_tx: mpsc::Sender<(u64, u64)>,
        result_tx: mpsc::Sender<TaskResult>,
    ) -> Self {
        Self {
            progress_tx,
            result_tx,
        }
    }

    /// Convert a `RunRequest` into a `TaskConfig` for delegation to the
    /// existing worker module. This preserves the worker execution path
    /// while the dispatcher owns the dispatch decision.
    fn request_to_config(request: &RunRequest) -> Option<crate::workers::TaskConfig> {
        use eggsec_runtime::request::TaskKind;

        match &request.task_kind {
            TaskKind::LoadTest(p) => Some(crate::workers::TaskConfig::LoadTest {
                target: p.target.clone(),
                requests: p.connections.unwrap_or(100) as u64,
                concurrency: p.connections.unwrap_or(10) as usize,
                timeout: std::time::Duration::from_secs(p.duration_secs.unwrap_or(30) as u64),
            }),
            TaskKind::StressTest(p) => Some(crate::workers::TaskConfig::StressTest {
                target: p.target.clone(),
                stress_type: p.flood_type.clone(),
                rate: 1000,
                duration: p.duration_secs.unwrap_or(60) as u64,
                concurrency: p.threads.unwrap_or(10) as usize,
            }),
            TaskKind::PortScan(p) => Some(crate::workers::TaskConfig::PortScan {
                target: p.target.clone(),
                ports: p.ports.clone().unwrap_or_else(|| "1-1024".to_string()),
                concurrency: 100,
                timeout: std::time::Duration::from_millis(p.timeout_ms.unwrap_or(5000)),
            }),
            TaskKind::EndpointScan(p) => Some(crate::workers::TaskConfig::EndpointScan {
                target: p.target.clone(),
                concurrency: 10,
                timeout: std::time::Duration::from_secs(60),
                wordlist: p.wordlist.clone(),
            }),
            TaskKind::Fingerprint(p) => Some(crate::workers::TaskConfig::Fingerprint {
                target: p.target.clone(),
                ports: "1-1024".to_string(),
                timeout: std::time::Duration::from_secs(60),
            }),
            TaskKind::Fuzz(p) => Some(crate::workers::TaskConfig::Fuzz {
                target: p.target.clone(),
                payload_type: p
                    .payload_type
                    .clone()
                    .unwrap_or_else(|| "xss".to_string()),
                mode: "smart".to_string(),
                mutations: false,
                mutation_count: 0,
                method: "GET".to_string(),
                param: None,
                concurrency: p.threads.unwrap_or(10) as usize,
                timeout: 60,
                graphql_introspection: false,
                graphql_depth_bypass: false,
                graphql_alias_overload: false,
                oauth_redirect_test: false,
                oauth_scope_test: false,
                oauth_state_test: false,
                oauth_grant_test: false,
            }),
            TaskKind::Waf(p) => Some(crate::workers::TaskConfig::Waf {
                target: p.target.clone(),
                bypass_mode: false,
                techniques: vec![],
            }),
            TaskKind::WafStress(p) => Some(crate::workers::TaskConfig::WafStress {
                target: p.target.clone(),
                concurrency: 10,
                timeout: p.requests.unwrap_or(100) as u64,
            }),
            TaskKind::Pipeline(p) => {
                let profile = match p.profile.as_deref() {
                    Some("quick") => eggsec::cli::ScanProfile::Quick,
                    Some("endpoint") => eggsec::cli::ScanProfile::Endpoint,
                    Some("web") => eggsec::cli::ScanProfile::Web,
                    Some("waf") => eggsec::cli::ScanProfile::Waf,
                    Some("full") => eggsec::cli::ScanProfile::Full,
                    Some("api") => eggsec::cli::ScanProfile::Api,
                    Some("recon") => eggsec::cli::ScanProfile::Recon,
                    Some("stealth") => eggsec::cli::ScanProfile::Stealth,
                    Some("deep") => eggsec::cli::ScanProfile::Deep,
                    Some("vuln") => eggsec::cli::ScanProfile::Vuln,
                    Some("auth") => eggsec::cli::ScanProfile::Auth,
                    Some("defense-lab") => eggsec::cli::ScanProfile::DefenseLab,
                    _ => eggsec::cli::ScanProfile::Quick,
                };
                Some(crate::workers::TaskConfig::Pipeline {
                    target: p.target.clone(),
                    profile,
                    output_file: String::new(),
                    output_format: "json".to_string(),
                })
            }
            TaskKind::Recon(p) => Some(crate::workers::TaskConfig::Recon {
                target: p.target.clone(),
                concurrency: 20,
                options: crate::tabs::recon::ReconOptions::default(),
            }),
            TaskKind::PacketCapture(p) => Some(crate::workers::TaskConfig::PacketCapture {
                interface: p
                    .interface
                    .clone()
                    .unwrap_or_else(|| "eth0".to_string()),
                filter: String::new(),
                max_packets: 1000,
                output_file: None,
            }),
            TaskKind::PacketTraceroute(p) => Some(crate::workers::TaskConfig::PacketTraceroute {
                target: p.target.clone(),
                max_hops: p.max_hops.unwrap_or(30) as u8,
            }),
            TaskKind::PacketSend(p) => Some(crate::workers::TaskConfig::PacketSend {
                target: p.target.clone(),
                port: 80,
                count: 10,
                packet_size: 64,
            }),
            TaskKind::GraphQl(p) => Some(crate::workers::TaskConfig::GraphQl {
                url: p.target.clone(),
                introspection: p.introspection.unwrap_or(true),
                inject: false,
                depth_bypass: false,
                alias_overload: false,
                concurrency: 10,
                timeout: 300,
            }),
            TaskKind::OAuth(p) => Some(crate::workers::TaskConfig::OAuth {
                url: p.target.clone(),
                client_id: None,
                redirect_uri: None,
                redirect_test: false,
                scope_test: false,
                state_test: false,
                grant_test: false,
                concurrency: 10,
                timeout: 300,
            }),
            TaskKind::AuthTest(p) => Some(crate::workers::TaskConfig::Auth {
                target: p.target.clone(),
                username: p.username.clone(),
                password_list: p.credential_list.clone(),
                credential_file: None,
                max_attempts: 100,
                concurrency: 1,
                timeout: 30,
            }),
            #[cfg(feature = "nse")]
            TaskKind::Nse(p) => Some(crate::workers::TaskConfig::Nse {
                target: p.target.clone(),
                script: p.script.clone(),
                script_args: p.args.clone(),
                custom_script: None,
            }),
            #[cfg(feature = "advanced-hunting")]
            TaskKind::Hunt(p) => Some(crate::workers::TaskConfig::Hunt {
                target: p.target.clone(),
                config: eggsec::hunt::HuntConfig::default(),
            }),
            #[cfg(feature = "headless-browser")]
            TaskKind::Browser(p) => Some(crate::workers::TaskConfig::Browser {
                target: p.target.clone(),
                config: eggsec::browser::BrowserConfig::default(),
            }),
            #[cfg(feature = "compliance")]
            TaskKind::Compliance(p) => Some(crate::workers::TaskConfig::Compliance {
                target: p.target.clone(),
                framework: eggsec::compliance::ComplianceFramework::OwaspTop10,
            }),
            #[cfg(feature = "database")]
            TaskKind::Storage(p) => Some(crate::workers::TaskConfig::Storage {
                config: eggsec::storage::StorageConfig::default(),
                mode: "read".to_string(),
                scan_id: None,
                cve_id: None,
                severity_filter: None,
            }),
            #[cfg(feature = "external-integrations")]
            TaskKind::Integrations(p) => Some(crate::workers::TaskConfig::Integrations {
                config: eggsec::integrations::IntegrationConfig::default(),
                mode: "list".to_string(),
                title: None,
                description: None,
                labels: vec![],
                assignees: vec![],
                search_query: None,
            }),
            #[cfg(feature = "finding-workflow")]
            TaskKind::Workflow(p) => Some(crate::workers::TaskConfig::Workflow {
                mode: "list".to_string(),
                target: None,
                finding_ids: vec![],
            }),
            #[cfg(feature = "vuln-management")]
            TaskKind::Vuln(p) => Some(crate::workers::TaskConfig::Vuln {
                mode: "assess".to_string(),
                target: Some(p.target.clone()),
                cve_id: None,
                title: None,
                description: None,
                cvss_vector: None,
                asset_type: None,
                severity: None,
            }),
            #[cfg(feature = "wireless")]
            TaskKind::Wireless(p) => Some(crate::workers::TaskConfig::Wireless {
                interface: p
                    .interface
                    .clone()
                    .unwrap_or_else(|| "wlan0".to_string()),
            }),
            #[cfg(feature = "wireless-advanced")]
            TaskKind::WirelessActive(p) => Some(crate::workers::TaskConfig::WirelessActive {
                interface: p
                    .interface
                    .clone()
                    .unwrap_or_else(|| "wlan0".to_string()),
                attack_type: "deauth".to_string(),
                bssid: p.target_bssid.clone(),
                client: None,
                frame_count: 100,
                rate_limit: 10,
                dry_run: true,
            }),
            #[cfg(feature = "db-pentest")]
            TaskKind::DbPentest(p) => Some(crate::workers::TaskConfig::DbPentest {
                manifest: None,
                target: Some(p.target.clone()),
                db_type: Some(p.db_type.clone()),
                checks: "all".to_string(),
                dry_run: true,
                allow_advanced: false,
                max_queries: 200,
                max_duration: 120,
            }),
            #[cfg(feature = "web-proxy")]
            TaskKind::Intercept(p) => Some(crate::workers::TaskConfig::Intercept {
                listen_addr: format!("127.0.0.1:{}", p.listen_port.unwrap_or(8080)),
                dry_run: true,
                max_flows: 100,
                target: p.target.clone(),
            }),
            #[cfg(feature = "c2")]
            TaskKind::C2(p) => Some(crate::workers::TaskConfig::C2 {
                target: p
                    .target
                    .clone()
                    .unwrap_or_else(|| "127.0.0.1".to_string()),
                campaign: p
                    .profile
                    .clone()
                    .unwrap_or_else(|| "default".to_string()),
                dry_run: true,
            }),
            // Feature-gated variants without cfg attributes above return None
            _ => None,
        }
    }
}

impl TaskDispatcher for TuiTaskDispatcher {
    fn dispatch(
        &self,
        request: RunRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send>,
    > {
        let progress_tx = self.progress_tx.clone();
        let result_tx = self.result_tx.clone();

        Box::pin(async move {
            let config = Self::request_to_config(&request).ok_or_else(|| {
                RuntimeError::DispatchFailed(format!(
                    "unsupported or feature-gated task kind: {:?}",
                    request.task_kind
                ))
            })?;

            let runner = crate::workers::TaskRunner::new(config, progress_tx, result_tx);
            runner.run().await.map_err(|e| {
                RuntimeError::DispatchFailed(format!("task execution failed: {}", e))
            })?;

            // Return empty outcome — typed results were sent through the
            // result channel for TUI consumption.
            Ok(TaskOutcome::Empty)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_runtime::request::{PortScanParams, RuntimeSurface, TaskKind};

    #[test]
    fn request_to_config_port_scan() {
        let request = RunRequest {
            task_kind: TaskKind::PortScan(PortScanParams {
                target: "10.0.0.1".into(),
                ports: Some("80,443".into()),
                scan_type: None,
                timeout_ms: Some(3000),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        };

        let config = TuiTaskDispatcher::request_to_config(&request);
        assert!(config.is_some());
        match config.unwrap() {
            crate::workers::TaskConfig::PortScan { target, ports, .. } => {
                assert_eq!(target, "10.0.0.1");
                assert_eq!(ports, "80,443");
            }
            other => panic!("expected PortScan, got {:?}", other),
        }
    }

    #[test]
    fn request_to_config_unsupported_returns_none() {
        // Storage without the database feature should return None
        let request = RunRequest {
            task_kind: TaskKind::Storage(eggsec_runtime::request::StorageParams {
                storage_type: "sqlite".into(),
                path: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        };

        // Without the "database" feature, this should be None
        #[cfg(not(feature = "database"))]
        assert!(TuiTaskDispatcher::request_to_config(&request).is_none());
    }
}
