use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

pub mod auth;
#[cfg(feature = "headless-browser")]
pub mod browser;
pub mod ci;
pub mod cluster;
pub mod fuzz;
pub mod http;
pub mod misc;
pub mod packet;
pub mod plan;
pub mod scan;
pub mod storage;
pub mod stress;
#[cfg(feature = "wireless")]
pub mod wireless;
pub(crate) mod timeout;
pub mod vuln;

pub use ci::*;
pub use cluster::*;
pub use fuzz::*;
pub use http::*;
pub use misc::*;
pub use packet::*;
pub use plan::*;
pub use scan::*;
pub use storage::*;
pub use vuln::*;

#[cfg(feature = "stress-testing")]
pub use stress::*;

#[cfg(feature = "wireless")]
pub use wireless::*;

#[cfg(feature = "headless-browser")]
pub use browser::*;

#[cfg(feature = "ai-integration")]
pub mod ai_analyze;
#[cfg(feature = "ai-integration")]
pub use ai_analyze::*;
pub use auth::*;

#[cfg(feature = "rest-api")]
pub mod agent;
#[cfg(feature = "rest-api")]
pub use agent::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP_AFTER: &str = r#"
EXAMPLES:
  Scan ports:        slapper scan-ports 192.168.1.1 -p 1-1000
  Fuzz endpoints:    slapper fuzz https://example.com/api -t sqli
  Detect WAF:        slapper waf https://example.com
  Auth testing:      slapper auth-test https://example.com/login --all
  MCP for coding:    slapper codegg-mcp

See https://dbowm91.dev/docs for full documentation.
"#;

#[derive(Parser)]
#[command(name = "slapper")]
#[command(about = "High-performance security testing toolkit")]
#[command(version = VERSION, long_about = None)]
#[command(after_help = HELP_AFTER)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, global = true, help = "Output in JSON format")]
    pub json: bool,

    #[arg(long, global = true, help = "Configuration file path")]
    pub config: Option<String>,

    #[arg(long, global = true, help = "Scope file path")]
    pub scope: Option<String>,

    #[arg(long, help = "Generate default configuration file to stdout")]
    pub generate_config: bool,

    #[arg(long, help = "Generate shell completion scripts", value_enum)]
    pub generate_shell_completion: Option<Shell>,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    // --- Scan operations ---
    #[command(about = "Scan ports on target host", long_about = SCAN_PORTS_ABOUT)]
    ScanPorts(PortScanArgs),
    #[command(about = "Discover sensitive HTTP endpoints", long_about = SCAN_ENDPOINTS_ABOUT)]
    ScanEndpoints(EndpointScanArgs),
    #[command(about = "Fingerprint services (AMAP-style)", long_about = FINGERPRINT_ABOUT)]
    Fingerprint(FingerprintArgs),
    #[command(about = "Run chained security assessment pipeline", long_about = SCAN_ABOUT)]
    Scan(ScanArgs),
    #[command(about = "Resume a previous scan from session file", long_about = RESUME_ABOUT)]
    Resume(ResumeArgs),

    // --- Assessment operations ---
    #[command(about = "Fuzz target with security payloads", long_about = FUZZ_ABOUT)]
    Fuzz(FuzzArgs),
    #[command(about = "Evaluate WAF detection and evasion resistance", long_about = WAF_ABOUT)]
    Waf(WafArgs),
    #[command(about = "Comprehensive WAF stress testing", long_about = WAF_STRESS_ABOUT)]
    WafStress(WafStressArgs),
    #[command(about = "Validate GraphQL endpoint security controls", long_about = GRAPHQL_ABOUT)]
    Graphql(GraphQlArgs),
    #[command(about = "Validate OAuth/OIDC endpoint security controls", long_about = OAUTH_ABOUT)]
    OAuth(OAuthArgs),
    #[command(about = "Validate authentication controls in authorized environments", long_about = AUTH_TEST_ABOUT)]
    AuthTest(AuthTestArgs),

    // --- Recon operations ---
    #[command(about = "Gather reconnaissance information", long_about = RECON_ABOUT)]
    Recon(ReconArgs),

    // --- Planning & CI ---
    #[command(about = "Preview execution plan without running it")]
    Plan(PlanArgs),
    #[command(about = "Run security checks in CI/CD mode")]
    Ci(CiArgs),
    #[command(about = "Validate configuration files", long_about = CONFIG_ABOUT)]
    Config(ConfigArgs),
    #[command(about = "Check system and runtime dependencies", long_about = DOCTOR_ABOUT)]
    Doctor,
    #[cfg(feature = "sbom")]
    #[command(about = "Generate SBOM and check supply chain security")]
    Sbom(SbomArgs),

    // --- Load testing ---
    #[command(about = "Run HTTP load test against target URL", long_about = LOAD_ABOUT)]
    Load(LoadArgs),

    // --- Tool operations ---
    #[cfg(feature = "packet-inspection")]
    #[command(about = "Packet inspection and analysis tools", long_about = PACKET_ABOUT)]
    Packet(PacketArgs),
    #[cfg(feature = "nse")]
    #[command(about = "Run Nmap NSE-compatible scripts through Slapper's optional Lua/NSE compatibility runtime", long_about = NSE_ABOUT)]
    Nse(NseArgs),
    #[command(about = "Convert and generate security scan reports")]
    Report(ReportArgs),
    #[command(about = "Vulnerability management tools (CVSS scoring, triage, remediation)", long_about = VULN_ABOUT)]
    Vuln(VulnArgs),
    #[command(about = "Database storage and query operations", long_about = STORAGE_ABOUT)]
    Storage(StorageArgs),

    // --- Stress testing operations ---
    #[cfg(feature = "stress-testing")]
    #[command(about = "Run stress/load testing against target", long_about = STRESS_ABOUT)]
    Stress(StressArgs),
    #[cfg(feature = "stress-testing")]
    #[command(about = "Manage proxy pool and rotation", long_about = PROXY_ABOUT)]
    Proxy(ProxyArgs),
    #[cfg(feature = "stress-testing")]
    #[command(about = "Send ICMP echo probes to target host", long_about = ICMP_ABOUT)]
    Icmp(IcmpArgs),
    #[cfg(feature = "stress-testing")]
    #[command(about = "Trace network path to target host", long_about = TRACEROUTE_ABOUT)]
    Traceroute(TracerouteArgs),

    // --- Infrastructure operations ---
    #[command(about = "Manage distributed scanning cluster", long_about = CLUSTER_ABOUT)]
    Cluster(ClusterArgs),
    #[command(about = "Test and manage notifications", long_about = NOTIFY_ABOUT)]
    Notify(NotifyArgs),
    #[command(about = "Start remote listener for distributed commands")]
    Remote(RemoteArgs),
    #[command(about = "Execute commands on remote systems")]
    Exec(ExecArgs),
    #[cfg(feature = "rest-api")]
    #[command(about = "Start REST API server for external tool integration")]
    Serve(ServeArgs),
    #[cfg(feature = "rest-api")]
    #[command(
        about = "Start MCP server for AI assistant integration",
        alias = "mcp-serve"
    )]
    McpServe(McpServeArgs),
    #[cfg(feature = "rest-api")]
    #[command(
        about = "Start MCP server for coding agent integration (stdio + coding-agent profile)",
        alias = "mcp-codegg"
    )]
    CodeggMcp(CodeggMcpArgs),

    // --- Agent orchestration ---
    #[cfg(feature = "rest-api")]
    #[command(
        about = "Run security agent for scheduled assessments",
        alias = "agent"
    )]
    Agent(AgentArgs),

    // --- AI operations ---
    #[cfg(feature = "ai-integration")]
    #[command(about = "Post-scan AI analysis of findings")]
    AiAnalyze(AiAnalyzeArgs),

    // --- Wireless operations ---
    #[cfg(feature = "wireless")]
    #[command(about = "Scan wireless networks for security issues", long_about = WIRELESS_ABOUT)]
    Wireless(WirelessArgs),

    // --- Browser operations ---
    #[cfg(feature = "headless-browser")]
    #[command(about = "Run headless browser security testing", long_about = BROWSER_ABOUT)]
    Browser(BrowserArgs),

    // --- gRPC server ---
    #[cfg(feature = "grpc-api")]
    #[command(about = "Start gRPC server for external tool integration")]
    Grpc(GrpcServerArgs),
}

#[derive(clap::Args, Clone)]
pub struct CommonHttpArgs {
    #[arg(long, help = "Skip TLS certificate verification")]
    pub insecure: bool,
    #[arg(long, help = "HTTP proxy URL (e.g., http://127.0.0.1:8080)")]
    pub proxy: Option<String>,
    #[arg(long, help = "Proxy authentication (user:pass)")]
    pub proxy_auth: Option<String>,
    #[arg(long, help = "Basic authentication (user:pass)")]
    pub auth: Option<String>,
    #[arg(long, help = "Bearer token")]
    pub bearer: Option<String>,
    #[arg(long, help = "Cookie header value")]
    pub cookie: Option<String>,
    #[arg(
        long,
        help = "API key header (format: name:value or just value for X-API-Key)"
    )]
    pub api_key: Option<String>,
    #[arg(long, help = "Custom User-Agent header")]
    pub user_agent: Option<String>,
    #[arg(long, help = "Simulate realistic user behavior with randomized timing/headers for regression testing")]
    pub stealth: bool,
    #[arg(long, help = "Rate limit (requests per second)")]
    pub rate_limit: Option<u32>,
    #[arg(long, help = "Random delay between requests (ms range, e.g., 100-500)")]
    pub jitter: Option<String>,
    #[arg(
        long,
        help = "Path to auth context YAML file (multi-user/multi-role testing)"
    )]
    pub auth_context: Option<String>,
    #[arg(
        long,
        help = "Auth role name from the auth context file (required when --auth-context is set)"
    )]
    pub auth_role: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FuzzMode {
    Sequential,
    Burst,
    Adaptive,
}

impl std::fmt::Display for FuzzMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FuzzMode::Sequential => write!(f, "sequential"),
            FuzzMode::Burst => write!(f, "burst"),
            FuzzMode::Adaptive => write!(f, "adaptive"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScanProfile {
    Quick,
    Endpoint,
    Web,
    Waf,
    Full,
    Api,
    Recon,
    Stealth,
    Deep,
    Vuln,
    Auth,
    DefenseLab,
    SynvoidLocal,
    WafRegression,
    ProtocolEdge,
    NseSafe,
}

impl std::fmt::Display for ScanProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanProfile::Quick => write!(f, "quick"),
            ScanProfile::Endpoint => write!(f, "endpoint"),
            ScanProfile::Web => write!(f, "web"),
            ScanProfile::Waf => write!(f, "waf"),
            ScanProfile::Full => write!(f, "full"),
            ScanProfile::Api => write!(f, "api"),
            ScanProfile::Recon => write!(f, "recon"),
            ScanProfile::Stealth => write!(f, "stealth"),
            ScanProfile::Deep => write!(f, "deep"),
            ScanProfile::Vuln => write!(f, "vuln"),
            ScanProfile::Auth => write!(f, "auth"),
            ScanProfile::DefenseLab => write!(f, "defense-lab"),
            ScanProfile::SynvoidLocal => write!(f, "synvoid-local"),
            ScanProfile::WafRegression => write!(f, "waf-regression"),
            ScanProfile::ProtocolEdge => write!(f, "protocol-edge"),
            ScanProfile::NseSafe => write!(f, "nse-safe"),
        }
    }
}

impl ScanProfile {
    /// Returns `true` if this profile is a defense-lab variant that requires
    /// local/private-scope targets only.
    pub fn requires_private_scope(&self) -> bool {
        matches!(
            self,
            ScanProfile::DefenseLab
                | ScanProfile::SynvoidLocal
                | ScanProfile::WafRegression
                | ScanProfile::ProtocolEdge
                | ScanProfile::NseSafe
        )
    }

    /// Returns `true` if this profile requires the `packet-inspection` feature.
    pub fn requires_packet_inspection(&self) -> bool {
        matches!(self, ScanProfile::ProtocolEdge)
    }

    /// Returns `true` if this profile requires the `nse` feature.
    pub fn requires_nse(&self) -> bool {
        matches!(self, ScanProfile::NseSafe)
    }

    /// Returns the maximum `ProbeRisk` level allowed for this profile.
    ///
    /// Stages whose risk exceeds this budget are skipped during pipeline
    /// execution, providing a guardrail against unintended intrusive testing.
    pub fn max_risk_budget(&self) -> crate::probe::ProbeRisk {
        match self {
            ScanProfile::Quick | ScanProfile::ProtocolEdge | ScanProfile::NseSafe => {
                crate::probe::ProbeRisk::SafeActive
            }
            ScanProfile::Stealth => crate::probe::ProbeRisk::Passive,
            ScanProfile::DefenseLab | ScanProfile::SynvoidLocal | ScanProfile::WafRegression => {
                crate::probe::ProbeRisk::Intrusive
            }
            ScanProfile::Endpoint
            | ScanProfile::Web
            | ScanProfile::Waf
            | ScanProfile::Recon
            | ScanProfile::Vuln
            | ScanProfile::Auth => crate::probe::ProbeRisk::Intrusive,
            ScanProfile::Full | ScanProfile::Api | ScanProfile::Deep => {
                crate::probe::ProbeRisk::Stress
            }
        }
    }
}

pub use crate::types::OutputFormat;

#[cfg(feature = "grpc-api")]
#[derive(clap::Args, Clone)]
pub struct GrpcServerArgs {
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, default_value = "50051")]
    pub port: u16,
    #[arg(long, help = "API key for authentication")]
    pub api_key: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::ProbeRisk;

    #[test]
    fn quick_profile_allows_safe_active() {
        assert_eq!(
            ScanProfile::Quick.max_risk_budget(),
            ProbeRisk::SafeActive
        );
    }

    #[test]
    fn stealth_profile_allows_passive_only() {
        assert_eq!(
            ScanProfile::Stealth.max_risk_budget(),
            ProbeRisk::Passive
        );
    }

    #[test]
    fn full_profile_allows_stress() {
        assert_eq!(ScanProfile::Full.max_risk_budget(), ProbeRisk::Stress);
    }

    #[test]
    fn defense_lab_allows_intrusive() {
        assert_eq!(
            ScanProfile::DefenseLab.max_risk_budget(),
            ProbeRisk::Intrusive
        );
    }

    #[test]
    fn risk_budget_ordering() {
        assert!(
            ProbeRisk::Passive.risk_level()
                < ScanProfile::Quick.max_risk_budget().risk_level()
        );
        assert!(
            ScanProfile::Stealth.max_risk_budget().risk_level()
                < ScanProfile::Quick.max_risk_budget().risk_level()
        );
    }
}
