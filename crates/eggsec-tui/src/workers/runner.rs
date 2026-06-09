use crate::tabs::recon::ReconOptions;
use eggsec::cli::ScanProfile;
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TaskConfig {
    LoadTest {
        target: String,
        requests: u64,
        concurrency: usize,
        timeout: Duration,
    },
    StressTest {
        target: String,
        stress_type: String,
        rate: u64,
        duration: u64,
        concurrency: usize,
    },
    PortScan {
        target: String,
        ports: String,
        concurrency: usize,
        timeout: Duration,
    },
    EndpointScan {
        target: String,
        concurrency: usize,
        timeout: Duration,
        wordlist: Option<String>,
    },
    Fingerprint {
        target: String,
        ports: String,
        timeout: Duration,
    },
    Fuzz {
        target: String,
        payload_type: String,
        mode: String,
        mutations: bool,
        mutation_count: usize,
        method: String,
        param: Option<String>,
        concurrency: usize,
        timeout: u64,
        graphql_introspection: bool,
        graphql_depth_bypass: bool,
        graphql_alias_overload: bool,
        oauth_redirect_test: bool,
        oauth_scope_test: bool,
        oauth_state_test: bool,
        oauth_grant_test: bool,
    },
    Waf {
        target: String,
        bypass_mode: bool,
        techniques: Vec<String>,
    },
    WafStress {
        target: String,
        concurrency: usize,
        timeout: u64,
    },
    Pipeline {
        target: String,
        profile: ScanProfile,
        output_file: String,
        output_format: String,
    },
    Recon {
        target: String,
        concurrency: usize,
        options: ReconOptions,
    },
    PacketCapture {
        interface: String,
        filter: String,
        max_packets: usize,
        output_file: Option<String>,
    },
    PacketTraceroute {
        target: String,
        max_hops: u8,
    },
    PacketSend {
        target: String,
        port: u16,
        count: u32,
        packet_size: usize,
    },
    GraphQl {
        url: String,
        introspection: bool,
        inject: bool,
        depth_bypass: bool,
        alias_overload: bool,
        concurrency: usize,
        timeout: u64,
    },
    OAuth {
        url: String,
        client_id: Option<String>,
        redirect_uri: Option<String>,
        redirect_test: bool,
        scope_test: bool,
        state_test: bool,
        grant_test: bool,
        concurrency: usize,
        timeout: u64,
    },
    #[cfg(feature = "nse")]
    Nse {
        target: String,
        script: String,
        script_args: Option<String>,
        custom_script: Option<String>,
    },
    #[cfg(feature = "advanced-hunting")]
    Hunt {
        target: String,
        config: eggsec::hunt::HuntConfig,
    },
    #[cfg(feature = "headless-browser")]
    Browser {
        target: String,
        config: eggsec::browser::BrowserConfig,
    },
    #[cfg(feature = "compliance")]
    Compliance {
        target: String,
        framework: eggsec::compliance::ComplianceFramework,
    },
    #[cfg(feature = "database")]
    Storage {
        config: eggsec::storage::StorageConfig,
        mode: String,
        scan_id: Option<String>,
        cve_id: Option<String>,
        severity_filter: Option<String>,
    },
    #[cfg(feature = "external-integrations")]
    Integrations {
        config: eggsec::integrations::IntegrationConfig,
        mode: String,
        title: Option<String>,
        description: Option<String>,
        labels: Vec<String>,
        assignees: Vec<String>,
        search_query: Option<String>,
    },
    #[cfg(feature = "finding-workflow")]
    Workflow {
        mode: String,
        target: Option<String>,
        finding_ids: Vec<String>,
    },
    #[cfg(feature = "vuln-management")]
    Vuln {
        mode: String,
        target: Option<String>,
        cve_id: Option<String>,
        title: Option<String>,
        description: Option<String>,
        cvss_vector: Option<String>,
        asset_type: Option<String>,
        severity: Option<String>,
    },
    #[cfg(feature = "wireless")]
    Wireless {
        interface: String,
    },
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum TaskResult {
    LoadTest(eggsec::loadtest::metrics::LoadTestResults),
    #[cfg(feature = "stress-testing")]
    StressTest {
        target: String,
        stats: eggsec::stress::StressStats,
    },
    PortScan(eggsec::scanner::PortScanResults),
    EndpointScan(eggsec::scanner::EndpointScanResults),
    Fingerprint(eggsec::scanner::FingerprintResults),
    WafDetection(eggsec::waf::WafDetectionResult),
    WafBypass {
        detection: eggsec::waf::WafDetectionResult,
        bypasses: Vec<eggsec::waf::BypassResult>,
    },
    WafStress(Vec<eggsec::waf::BypassResult>),
    Pipeline(eggsec::pipeline::PipelineReport),
    Fuzz(eggsec::fuzzer::engine::FuzzSession),
    Recon(eggsec::recon::FullReconResult),
    PacketCapture {
        packets_captured: usize,
        output_file: Option<String>,
    },
    PacketTraceroute {
        hops: Vec<TracerouteHopResult>,
    },
    PacketSend {
        packets_sent: u32,
        bytes_sent: u64,
    },
    GraphQl(crate::tabs::graphql::GraphQlResults),
    OAuth(crate::tabs::oauth::OAuthResults),
    #[cfg(feature = "nse")]
    Nse(crate::tabs::nse::NseResults),
    #[cfg(feature = "advanced-hunting")]
    Hunt(eggsec::hunt::HuntReport),
    #[cfg(feature = "headless-browser")]
    Browser(eggsec::browser::BrowserReport),
    #[cfg(feature = "compliance")]
    Compliance(eggsec::compliance::ComplianceReport),
    #[cfg(feature = "database")]
    Storage,
    #[cfg(feature = "database")]
    StorageListScans {
        scans: Vec<eggsec::storage::models::StoredScan>,
    },
    #[cfg(feature = "database")]
    StorageListFindings {
        findings: Vec<eggsec::findings::lifecycle::StoredFinding>,
    },
    #[cfg(feature = "external-integrations")]
    Integrations,
    #[cfg(feature = "external-integrations")]
    IntegrationsCreateIssue {
        issue: eggsec::integrations::Issue,
    },
    #[cfg(feature = "external-integrations")]
    IntegrationsSearchIssues {
        issues: Vec<eggsec::integrations::Issue>,
    },
    #[cfg(feature = "finding-workflow")]
    Workflow(eggsec::workflow::WorkflowReport),
    #[cfg(feature = "vuln-management")]
    Vuln(eggsec::vuln::VulnAssessment),
    #[cfg(feature = "wireless")]
    Wireless(eggsec::wireless::WirelessScanResult),
    Error(String),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TracerouteHopResult {
    pub hop: u8,
    pub address: Option<String>,
    pub rtt_ms: Option<f64>,
}

pub struct TaskRunner {
    pub config: TaskConfig,
    pub progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    pub result_tx: tokio::sync::mpsc::Sender<TaskResult>,
}

impl TaskRunner {
    pub fn new(
        config: TaskConfig,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
        result_tx: tokio::sync::mpsc::Sender<TaskResult>,
    ) -> Self {
        Self {
            config,
            progress_tx,
            result_tx,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let (progress_tx, result_tx) = (self.progress_tx, self.result_tx);

        let result = match self.config {
            TaskConfig::LoadTest {
                target,
                requests,
                concurrency,
                timeout,
            } => {
                super::network::run_load_test(
                    target,
                    requests,
                    concurrency,
                    timeout,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::StressTest {
                target,
                stress_type,
                rate,
                duration,
                concurrency,
            } => {
                super::network::run_stress_test(
                    target,
                    stress_type,
                    rate,
                    duration,
                    concurrency,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::PortScan {
                target,
                ports,
                concurrency,
                timeout,
            } => {
                super::scanner::run_port_scan(
                    target,
                    ports,
                    concurrency,
                    timeout,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::EndpointScan {
                target,
                concurrency,
                timeout,
                wordlist,
            } => {
                super::scanner::run_endpoint_scan(
                    target,
                    concurrency,
                    timeout,
                    wordlist,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::Fingerprint {
                target,
                ports,
                timeout,
            } => {
                super::scanner::run_fingerprint(target, ports, timeout, progress_tx, result_tx)
                    .await
            }
            TaskConfig::Fuzz {
                target,
                payload_type,
                mode,
                mutations,
                mutation_count,
                method,
                param,
                concurrency,
                timeout,
                graphql_introspection,
                graphql_depth_bypass,
                graphql_alias_overload,
                oauth_redirect_test,
                oauth_scope_test,
                oauth_state_test,
                oauth_grant_test,
            } => {
                super::fuzzer::run_fuzz(
                    target,
                    payload_type,
                    mode,
                    mutations,
                    mutation_count,
                    method,
                    param,
                    concurrency,
                    timeout,
                    graphql_introspection,
                    graphql_depth_bypass,
                    graphql_alias_overload,
                    oauth_redirect_test,
                    oauth_scope_test,
                    oauth_state_test,
                    oauth_grant_test,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::Waf {
                target,
                bypass_mode,
                techniques,
            } => {
                super::fuzzer::run_waf(target, bypass_mode, techniques, progress_tx, result_tx)
                    .await
            }
            TaskConfig::WafStress {
                target,
                concurrency,
                timeout,
            } => {
                super::fuzzer::run_waf_stress(target, concurrency, timeout, progress_tx, result_tx)
                    .await
            }
            TaskConfig::Pipeline {
                target,
                profile,
                output_file,
                output_format,
            } => {
                super::recon::run_pipeline(
                    target,
                    profile,
                    output_file,
                    output_format,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::Recon {
                target,
                concurrency,
                options,
            } => {
                super::recon::run_recon(target, concurrency, options, progress_tx, result_tx).await
            }
            TaskConfig::PacketCapture {
                interface,
                filter,
                max_packets,
                output_file,
            } => {
                super::network::run_packet_capture(
                    interface,
                    filter,
                    max_packets,
                    output_file,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::PacketTraceroute { target, max_hops } => {
                super::network::run_packet_traceroute(target, max_hops, progress_tx, result_tx)
                    .await
            }
            TaskConfig::PacketSend {
                target,
                port,
                count,
                packet_size,
            } => {
                super::network::run_packet_send(
                    target,
                    port,
                    count,
                    packet_size,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::GraphQl {
                url,
                introspection,
                inject,
                depth_bypass,
                alias_overload,
                concurrency,
                timeout,
            } => {
                super::api::run_graphql(
                    url,
                    introspection,
                    inject,
                    depth_bypass,
                    alias_overload,
                    concurrency,
                    timeout,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            TaskConfig::OAuth {
                url,
                client_id,
                redirect_uri,
                redirect_test,
                scope_test,
                state_test,
                grant_test,
                concurrency,
                timeout,
            } => {
                super::api::run_oauth(
                    url,
                    client_id,
                    redirect_uri,
                    redirect_test,
                    scope_test,
                    state_test,
                    grant_test,
                    concurrency,
                    timeout,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            #[cfg(feature = "nse")]
            TaskConfig::Nse {
                target,
                script,
                script_args,
                custom_script,
            } => {
                super::api::run_nse(
                    target,
                    script,
                    script_args,
                    custom_script,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            #[cfg(feature = "advanced-hunting")]
            TaskConfig::Hunt { target, config } => {
                super::security::run_hunt_task(target, config, progress_tx, result_tx).await
            }
            #[cfg(feature = "headless-browser")]
            TaskConfig::Browser { target, config } => {
                super::security::run_browser_task(target, config, progress_tx, result_tx).await
            }
            #[cfg(feature = "compliance")]
            TaskConfig::Compliance { target, framework } => {
                super::security::run_compliance_task(target, framework, progress_tx, result_tx)
                    .await
            }
            #[cfg(feature = "database")]
            TaskConfig::Storage {
                config,
                mode,
                scan_id,
                cve_id,
                severity_filter,
            } => {
                super::security::run_storage_task(
                    config,
                    mode,
                    scan_id,
                    cve_id,
                    severity_filter,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            #[cfg(feature = "external-integrations")]
            TaskConfig::Integrations {
                config,
                mode,
                title,
                description,
                labels,
                assignees,
                search_query,
            } => {
                super::security::run_integrations_task(
                    config,
                    mode,
                    title,
                    description,
                    labels,
                    assignees,
                    search_query,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            #[cfg(feature = "finding-workflow")]
            TaskConfig::Workflow {
                mode,
                target,
                finding_ids,
            } => {
                super::security::run_workflow_task(
                    mode,
                    target,
                    finding_ids,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            #[cfg(feature = "vuln-management")]
            TaskConfig::Vuln {
                mode,
                target,
                cve_id,
                title,
                description,
                cvss_vector,
                asset_type,
                severity,
            } => {
                super::security::run_vuln_task(
                    mode,
                    target,
                    cve_id,
                    title,
                    description,
                    cvss_vector,
                    asset_type,
                    severity,
                    progress_tx,
                    result_tx,
                )
                .await
            }
            #[cfg(feature = "wireless")]
            TaskConfig::Wireless { interface } => {
                super::security::run_wireless_task(interface, progress_tx, result_tx).await
            }
        };
        result
    }
}
