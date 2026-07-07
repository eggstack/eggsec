use tokio::sync::mpsc;

/// Send a progress update, logging on channel failure instead of propagating.
pub(crate) async fn send_progress(tx: &mpsc::Sender<(u64, u64)>, done: u64, total: u64) {
    if let Err(e) = tx.send((done, total)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
}

/// Results from GraphQL security testing.
#[derive(Clone, Debug)]
pub struct GraphQlResults {
    pub target: String,
    pub introspection_enabled: bool,
    pub depth_limit_bypassed: bool,
    pub alias_overload_vulnerable: bool,
    pub injection_findings: Vec<String>,
    pub total_requests: usize,
    pub errors: usize,
    pub duration_ms: u64,
}

/// Results from OAuth security testing.
#[derive(Clone, Debug)]
pub struct OAuthResults {
    pub target: String,
    pub redirect_vulnerabilities: Vec<String>,
    pub scope_vulnerabilities: Vec<String>,
    pub state_vulnerabilities: Vec<String>,
    pub grant_vulnerabilities: Vec<String>,
    pub total_requests: usize,
    pub errors: usize,
    pub duration_ms: u64,
}

/// Results from NSE script execution.
#[derive(Clone, Debug)]
pub struct NseResults {
    pub target: String,
    pub script: String,
    pub output: String,
    pub errors: String,
    pub success: bool,
    #[cfg(feature = "nse")]
    pub report: Option<eggsec_nse::NseRunReport>,
}

/// A single hop in a traceroute result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TracerouteHopResult {
    pub hop: u8,
    pub address: Option<String>,
    pub rtt_ms: Option<f64>,
}

/// Options for reconnaissance scans.
#[derive(Debug, Clone, Default)]
pub struct ReconOptions {
    pub no_tech: bool,
    pub no_dns: bool,
    pub no_geo: bool,
    pub no_whois: bool,
    pub no_subdomains: bool,
    pub no_ssl: bool,
    pub no_dns_records: bool,
    pub no_js: bool,
    pub no_content: bool,
    pub no_cloud: bool,
    pub no_wayback: bool,
    pub no_cors: bool,
    pub no_threat: bool,
    pub no_cve: bool,
    pub no_email: bool,
    pub no_takeover: bool,
}

/// Result of a dispatched task.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum TaskResult {
    LoadTest(crate::loadtest::metrics::LoadTestResults),
    #[cfg(feature = "stress-testing")]
    StressTest {
        target: String,
        stats: crate::stress::StressStats,
    },
    PortScan(crate::scanner::PortScanResults),
    EndpointScan(crate::scanner::EndpointScanResults),
    Fingerprint(crate::scanner::FingerprintResults),
    WafDetection(crate::waf::WafDetectionResult),
    WafBypass {
        detection: crate::waf::WafDetectionResult,
        bypasses: Vec<crate::waf::BypassResult>,
    },
    WafStress(Vec<crate::waf::BypassResult>),
    Pipeline(crate::pipeline::PipelineReport),
    Fuzz(crate::fuzzer::engine::FuzzSession),
    Recon(crate::recon::FullReconResult),
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
    GraphQl(GraphQlResults),
    OAuth(OAuthResults),
    #[cfg(feature = "nse")]
    Nse(NseResults),
    #[cfg(feature = "advanced-hunting")]
    Hunt(crate::hunt::HuntReport),
    #[cfg(feature = "headless-browser")]
    Browser(crate::browser::BrowserReport),
    #[cfg(feature = "compliance")]
    Compliance(crate::compliance::ComplianceReport),
    #[cfg(feature = "database")]
    Storage,
    #[cfg(feature = "database")]
    StorageListScans {
        scans: Vec<crate::storage::models::StoredScan>,
    },
    #[cfg(feature = "database")]
    StorageListFindings {
        findings: Vec<crate::findings::lifecycle::StoredFinding>,
    },
    #[cfg(feature = "external-integrations")]
    Integrations,
    #[cfg(feature = "external-integrations")]
    IntegrationsCreateIssue {
        issue: crate::integrations::Issue,
    },
    #[cfg(feature = "external-integrations")]
    IntegrationsSearchIssues {
        issues: Vec<crate::integrations::Issue>,
    },
    #[cfg(feature = "finding-workflow")]
    Workflow(crate::workflow::WorkflowReport),
    #[cfg(feature = "vuln-management")]
    Vuln(crate::vuln::VulnAssessment),
    #[cfg(feature = "wireless")]
    Wireless(crate::wireless::WirelessScanResult),
    #[cfg(feature = "wireless-advanced")]
    WirelessActive(crate::wireless::active::ActiveWirelessAttackResult),
    Auth(crate::auth::AuthTestReport),
    #[cfg(feature = "db-pentest")]
    DbPentest(crate::db_pentest::DbPentestReport),
    #[cfg(feature = "web-proxy")]
    Intercept(crate::proxy::intercept::types::InterceptSession),
    #[cfg(feature = "c2")]
    C2(crate::c2::C2Report),
    Error(String),
}
