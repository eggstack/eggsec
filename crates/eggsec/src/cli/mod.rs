use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};

pub mod auth;
#[cfg(feature = "headless-browser")]
pub mod browser;
#[cfg(feature = "c2")]
pub mod c2;
pub mod ci;
pub mod cluster;
#[cfg(feature = "db-pentest")]
pub mod db_pentest;
#[cfg(feature = "evasion")]
pub mod evasion;
pub mod explain;
pub mod fuzz;
pub mod http;
#[cfg(feature = "advanced-hunting")]
pub mod hunt;
pub mod misc;
#[cfg(feature = "mobile")]
pub mod mobile;
pub mod packet;
pub mod plan;
#[cfg(feature = "postex")]
pub mod postex;
pub mod preflight;
pub mod scan;
pub mod storage;
pub mod stress;
pub(crate) mod timeout;
pub mod vuln;
#[cfg(feature = "web-proxy")]
pub mod web_proxy;
#[cfg(feature = "wireless")]
pub mod wireless;

pub use ci::*;
pub use cluster::*;
pub use explain::*;
pub use fuzz::*;
pub use http::*;
#[cfg(feature = "advanced-hunting")]
pub use hunt::*;
pub use misc::*;
pub use packet::*;
pub use plan::*;
pub use preflight::*;
pub use scan::*;
pub use storage::*;
pub use vuln::*;

#[cfg(feature = "stress-testing")]
pub use stress::*;

#[cfg(feature = "wireless")]
pub use wireless::*;

#[cfg(feature = "headless-browser")]
pub use browser::*;

#[cfg(feature = "mobile")]
pub use mobile::*;

#[cfg(feature = "db-pentest")]
pub use db_pentest::*;

#[cfg(feature = "evasion")]
pub use evasion::*;

#[cfg(feature = "c2")]
pub use c2::*;
#[cfg(feature = "postex")]
pub use postex::*;
#[cfg(feature = "web-proxy")]
pub use web_proxy::*;

#[cfg(feature = "ai-integration")]
pub mod ai_analyze;
#[cfg(feature = "ai-integration")]
pub use ai_analyze::*;
pub use auth::*;

#[cfg(feature = "rest-api")]
pub mod agent;
#[cfg(feature = "rest-api")]
pub use agent::*;

const POLICY_EXPLAIN_ABOUT: &str = r#"Explain policy decisions for a target and profile

Evaluates what would happen if you ran a given profile against a target,
without sending any network traffic. Shows operation mode, risk level,
intended use, scope matching, required features, and any policy blocks.

Examples:
  eggsec policy-explain --target http://127.0.0.1:8080 --profile waf-regression
  eggsec policy-explain --target http://127.0.0.1:8080 --profile defense-lab --json
"#;

const SCOPE_EXPLAIN_ABOUT: &str = r#"Explain scope matching for a target

Evaluates whether a target falls within the configured scope, without
sending any network traffic. Shows rule matches, exclusions, and
private-IP detection.

Examples:
  eggsec scope-explain --target 10.0.0.5
  eggsec scope-explain --target example.com --json
"#;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP_AFTER: &str = r#"
OPERATING MODES:
  Standard Assessment  - Scoped recon, scanning, fuzzing, API testing, WAF detection
  Defense Lab          - Local/private-scope WAF and distributed-system validation
  Hazardous Lab        - Raw packets, flood stress, proxy rotation, protocol edge cases

COMMANDS:
  Scan ports:        eggsec scan-ports 192.168.1.1 -p 1-1000
  Fuzz endpoints:    eggsec fuzz https://example.com/api -t sqli
  Detect WAF:        eggsec waf https://example.com
  Auth testing:      eggsec auth-test https://example.com/login --all
  MCP for coding:    eggsec codegg-mcp
  Policy explain:    eggsec policy-explain --target http://127.0.0.1:8080 --profile defense-lab
  Scope explain:     eggsec scope-explain --target 192.168.1.1

See https://github.com/eggstack/eggsec#readme for full documentation.
"#;

#[derive(Parser)]
#[command(name = "eggsec")]
#[command(about = "Security testing and defense-validation toolkit")]
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

    #[arg(
        long,
        global = true,
        help = "Enforce strict scope rules (deny out-of-scope targets instead of warning)"
    )]
    pub strict_scope: bool,

    // --- Manual discretion overrides (honored only for ManualPermissive / default CLI/TUI) ---
    // --yes is narrow (low-risk scope prompts only). High-risk/exclusions/private/redirect/nonbaseline require specific --allow-* flags.
    // These are ignored or rejected under --strict-scope, CI, MCP, and agent paths.
    #[arg(
        long,
        global = true,
        help = "Assume yes to low-risk manual confirmation prompts (out-of-scope, target-expansion only). Does not authorize high-risk, explicit exclusions, non-baseline capabilities, private-resolution, or cross-host redirects. Use specific --allow-* flags for those classes. Manual-only."
    )]
    pub yes: bool,

    #[arg(
        long,
        global = true,
        help = "Allow operations on targets outside configured scope (manual-only)"
    )]
    pub allow_out_of_scope: bool,

    #[arg(
        long,
        global = true,
        help = "Allow operations on explicitly excluded targets (manual-only)"
    )]
    pub allow_excluded_target: bool,

    #[arg(
        long,
        global = true,
        help = "Allow high-risk operations (intrusive, stress, load, raw packet, credential, exploit-adjacent, remote, db-pentest) (manual-only)"
    )]
    pub allow_high_risk: bool,

    #[arg(
        long,
        global = true,
        help = "Allow direct database pentesting (lab/defense use only). Required for non-dry-run db pentest operations. (manual-only)"
    )]
    pub allow_db_pentest: bool,

    #[arg(
        long,
        global = true,
        help = "Allow traffic interception / MITM proxy operations (narrow override, audited)"
    )]
    pub allow_web_proxy: bool,

    #[arg(
        long,
        global = true,
        help = "Allow non-baseline capabilities (manual-only)"
    )]
    pub allow_nonbaseline_capability: bool,

    #[arg(
        long,
        global = true,
        help = "Allow target resolution to private/loopback addresses when detected (manual-only)"
    )]
    pub allow_private_resolution: bool,

    #[arg(
        long,
        global = true,
        help = "Allow cross-host redirect/canonicalization boundary changes (manual-only)"
    )]
    pub allow_cross_host_redirect: bool,

    #[arg(
        long,
        global = true,
        help = "Reason for manual override (recorded for audit)"
    )]
    pub manual_override_reason: Option<String>,

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

    // --- Hunt operations ---
    #[cfg(feature = "advanced-hunting")]
    #[command(about = "Run advanced vulnerability hunting", long_about = HUNT_ABOUT)]
    Hunt(HuntArgs),

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
    #[command(about = "Preview enforcement decision without executing")]
    Preflight(PreflightArgs),
    #[command(about = "Run security checks in CI/CD mode")]
    Ci(CiArgs),
    #[command(about = "Validate configuration files", long_about = CONFIG_ABOUT)]
    Config(ConfigArgs),
    #[command(about = "Check system and runtime dependencies", long_about = DOCTOR_ABOUT)]
    Doctor,
    #[command(
        about = "Explain policy decisions for a target and profile",
        long_about = POLICY_EXPLAIN_ABOUT
    )]
    PolicyExplain(PolicyExplainArgs),
    #[command(
        about = "Explain scope matching for a target",
        long_about = SCOPE_EXPLAIN_ABOUT
    )]
    ScopeExplain(ScopeExplainArgs),
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
    #[command(about = "Run Nmap NSE-compatible scripts through Eggsec's optional Lua/NSE compatibility runtime", long_about = NSE_ABOUT)]
    Nse(NseArgs),
    #[command(about = "Convert and generate security scan reports")]
    Report(ReportArgs),
    #[command(about = "Vulnerability management tools (CVSS scoring, triage, remediation)", long_about = VULN_ABOUT)]
    Vuln(VulnArgs),
    #[command(about = "Database storage and query operations", long_about = STORAGE_ABOUT)]
    Storage(StorageArgs),

    // --- Web proxy operations (standalone defense-lab) ---
    #[cfg(feature = "web-proxy")]
    #[command(
        about = "Interactive web proxy / traffic interception (defense-lab only)",
        long_about = "Start an interactive MITM proxy for capturing and inspecting HTTP/HTTPS traffic.\n\n\
                      WARNING: This tool is for authorized lab/defense environments only.\n\
                      Traffic interception may be illegal without proper authorization."
    )]
    ProxyIntercept(ProxyInterceptArgs),

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
    #[command(about = "Start remote listener for distributed commands", long_about = REMOTE_ABOUT)]
    Remote(RemoteArgs),
    #[command(about = "Execute commands on remote systems", long_about = EXEC_ABOUT)]
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
        long_about = AGENT_ABOUT,
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

    // --- Mobile operations ---
    #[cfg(feature = "mobile")]
    #[command(about = "Static security analysis of Android APKs and iOS IPAs (lab/defense use only)", long_about = MOBILE_ABOUT)]
    Mobile(MobileArgs),

    // --- Evasion testing operations ---
    #[cfg(feature = "evasion")]
    #[command(about = "Validate detection of common evasion techniques (defense-lab only)", long_about = EVASION_ABOUT)]
    Evasion(EvasionArgs),

    // --- Post-exploitation simulation operations ---
    #[cfg(feature = "postex")]
    #[command(about = "Simulate post-exploitation techniques for defense validation (defense-lab only)", long_about = POSTEX_ABOUT)]
    Postex(PostexArgs),

    // --- C2 simulation operations ---
    #[cfg(feature = "c2")]
    #[command(about = "Simulate C2 operations for defense validation (defense-lab only)", long_about = C2_ABOUT)]
    C2(C2Args),

    // --- Database pentesting operations (standalone defense-lab) ---
    #[cfg(feature = "db-pentest")]
    #[command(subcommand, about = "Database pentesting (direct checks for authorized lab/defense instances only)", long_about = DB_PENTEST_ABOUT)]
    Db(DbCommand),

    // --- gRPC server ---
    #[cfg(feature = "grpc-api")]
    #[command(about = "Start gRPC server for external tool integration")]
    Grpc(GrpcServerArgs),
}

impl Commands {
    /// Return the stable command ID string for this variant.
    ///
    /// Used by the command registry dispatch bridge to resolve metadata
    /// and by diagnostics for unknown-command suggestions.
    pub fn command_id(&self) -> &'static str {
        match self {
            Self::ScanPorts(_) => "scan-ports",
            Self::ScanEndpoints(_) => "scan-endpoints",
            Self::Fingerprint(_) => "fingerprint",
            Self::Scan(_) => "scan",
            Self::Resume(_) => "resume",
            Self::Recon(_) => "recon",
            Self::Plan(_) => "plan",
            Self::Preflight(_) => "preflight",
            Self::Ci(_) => "ci",
            Self::Config(_) => "config",
            Self::Doctor => "doctor",
            Self::PolicyExplain(_) => "policy-explain",
            Self::ScopeExplain(_) => "scope-explain",
            Self::Load(_) => "load",
            Self::Fuzz(_) => "fuzz",
            Self::Waf(_) => "waf",
            Self::WafStress(_) => "waf-stress",
            Self::Graphql(_) => "graphql",
            Self::OAuth(_) => "oauth",
            Self::AuthTest(_) => "auth-test",
            Self::Report(_) => "report",
            Self::Vuln(_) => "vuln",
            Self::Storage(_) => "storage",
            Self::Cluster(_) => "cluster",
            Self::Notify(_) => "notify",
            Self::Remote(_) => "remote",
            Self::Exec(_) => "exec",
            #[cfg(feature = "grpc-api")]
            Self::Grpc(_) => "grpc",
            #[cfg(feature = "rest-api")]
            Self::Serve(_) => "serve",
            #[cfg(feature = "rest-api")]
            Self::McpServe(_) => "mcp-serve",
            #[cfg(feature = "rest-api")]
            Self::CodeggMcp(_) => "mcp-serve",
            #[cfg(feature = "rest-api")]
            Self::Agent(_) => "agent",
            #[cfg(feature = "ai-integration")]
            Self::AiAnalyze(_) => "ai-analyze",
            #[cfg(feature = "wireless")]
            Self::Wireless(_) => "wireless",
            #[cfg(feature = "headless-browser")]
            Self::Browser(_) => "browser",
            #[cfg(feature = "mobile")]
            Self::Mobile(_) => "mobile",
            #[cfg(feature = "evasion")]
            Self::Evasion(_) => "evasion",
            #[cfg(feature = "postex")]
            Self::Postex(_) => "postex",
            #[cfg(feature = "c2")]
            Self::C2(_) => "c2",
            #[cfg(feature = "db-pentest")]
            #[cfg(feature = "db-pentest")]
            Self::Db(_) => "db",
            #[cfg(feature = "nse")]
            Self::Nse(_) => "nse",
            #[cfg(feature = "advanced-hunting")]
            Self::Hunt(_) => "hunt",
            #[cfg(feature = "packet-inspection")]
            Self::Packet(_) => "packet",
            #[cfg(feature = "stress-testing")]
            Self::Icmp(_) => "icmp",
            #[cfg(feature = "stress-testing")]
            Self::Traceroute(_) => "traceroute",
            #[cfg(feature = "stress-testing")]
            Self::Stress(_) => "stress",
            #[cfg(feature = "stress-testing")]
            Self::Proxy(_) => "proxy",
            #[cfg(feature = "web-proxy")]
            Self::ProxyIntercept(_) => "proxy-intercept",
            #[cfg(feature = "sbom")]
            Self::Sbom(_) => "sbom",
        }
    }
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
    #[arg(
        long,
        help = "Simulate realistic user behavior with randomized timing/headers for regression testing"
    )]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
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
    DbRegression,
    WebProxy,
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
            ScanProfile::DbRegression => write!(f, "db-regression"),
            ScanProfile::WebProxy => write!(f, "web-proxy"),
        }
    }
}

impl std::str::FromStr for ScanProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "quick" => Ok(ScanProfile::Quick),
            "endpoint" => Ok(ScanProfile::Endpoint),
            "web" => Ok(ScanProfile::Web),
            "waf" => Ok(ScanProfile::Waf),
            "full" => Ok(ScanProfile::Full),
            "api" => Ok(ScanProfile::Api),
            "recon" => Ok(ScanProfile::Recon),
            "stealth" => Ok(ScanProfile::Stealth),
            "deep" => Ok(ScanProfile::Deep),
            "vuln" => Ok(ScanProfile::Vuln),
            "auth" => Ok(ScanProfile::Auth),
            "defense-lab" => Ok(ScanProfile::DefenseLab),
            "synvoid-local" => Ok(ScanProfile::SynvoidLocal),
            "waf-regression" => Ok(ScanProfile::WafRegression),
            "protocol-edge" => Ok(ScanProfile::ProtocolEdge),
            "nse-safe" => Ok(ScanProfile::NseSafe),
            "db-regression" | "db_regression" | "dbregression" => Ok(ScanProfile::DbRegression),
            "web-proxy" | "webproxy" | "proxy" => Ok(ScanProfile::WebProxy),
            _ => Err(format!("Unknown scan profile: {}", s)),
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
                | ScanProfile::DbRegression
                | ScanProfile::WafRegression
                | ScanProfile::ProtocolEdge
                | ScanProfile::NseSafe
                | ScanProfile::WebProxy
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
            ScanProfile::DefenseLab
            | ScanProfile::SynvoidLocal
            | ScanProfile::WafRegression
            | ScanProfile::DbRegression
            | ScanProfile::WebProxy => crate::probe::ProbeRisk::Intrusive,
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

    /// Returns the operating mode for this profile.
    pub fn operation_mode(&self) -> crate::config::OperationMode {
        match self {
            ScanProfile::Quick
            | ScanProfile::Endpoint
            | ScanProfile::Web
            | ScanProfile::Waf
            | ScanProfile::Full
            | ScanProfile::Api
            | ScanProfile::Recon
            | ScanProfile::Stealth
            | ScanProfile::Deep
            | ScanProfile::Vuln
            | ScanProfile::Auth => crate::config::OperationMode::StandardAssessment,
            ScanProfile::DefenseLab
            | ScanProfile::SynvoidLocal
            | ScanProfile::WafRegression
            | ScanProfile::ProtocolEdge
            | ScanProfile::NseSafe
            | ScanProfile::DbRegression
            | ScanProfile::WebProxy => crate::config::OperationMode::DefenseLab,
        }
    }

    /// Returns the intended use cases for this profile.
    pub fn intended_uses(&self) -> Vec<crate::config::IntendedUse> {
        match self {
            ScanProfile::Quick | ScanProfile::Endpoint | ScanProfile::Web => {
                vec![crate::config::IntendedUse::WebAssessment]
            }
            ScanProfile::Api => vec![crate::config::IntendedUse::ApiAssessment],
            ScanProfile::Waf | ScanProfile::WafRegression => {
                vec![crate::config::IntendedUse::WafRegression]
            }
            ScanProfile::Full | ScanProfile::Deep => vec![
                crate::config::IntendedUse::WebAssessment,
                crate::config::IntendedUse::ApiAssessment,
            ],
            ScanProfile::Recon => vec![crate::config::IntendedUse::WebAssessment],
            ScanProfile::Stealth => vec![crate::config::IntendedUse::WebAssessment],
            ScanProfile::Vuln | ScanProfile::Auth => {
                vec![crate::config::IntendedUse::WebAssessment]
            }
            ScanProfile::DefenseLab => vec![
                crate::config::IntendedUse::WafRegression,
                crate::config::IntendedUse::SynvoidRegression,
            ],
            ScanProfile::SynvoidLocal => {
                vec![crate::config::IntendedUse::SynvoidRegression]
            }
            ScanProfile::ProtocolEdge => {
                vec![crate::config::IntendedUse::ProtocolEdgeValidation]
            }
            ScanProfile::NseSafe => vec![crate::config::IntendedUse::CodingAgentVerification],
            ScanProfile::DbRegression => vec![
                crate::config::IntendedUse::WafRegression,
                crate::config::IntendedUse::SynvoidRegression,
            ],
            ScanProfile::WebProxy => vec![
                crate::config::IntendedUse::WafRegression,
                crate::config::IntendedUse::SynvoidRegression,
            ],
        }
    }

    /// Returns a human-readable description of this profile's mode and risk.
    pub fn mode_description(&self) -> String {
        let mode = self.operation_mode();
        let risk = self.max_risk_budget();
        format!(
            "{} mode (max risk: {})",
            mode.label(),
            format!("{:?}", risk).to_lowercase()
        )
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
        assert_eq!(ScanProfile::Quick.max_risk_budget(), ProbeRisk::SafeActive);
    }

    #[test]
    fn stealth_profile_allows_passive_only() {
        assert_eq!(ScanProfile::Stealth.max_risk_budget(), ProbeRisk::Passive);
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
            ProbeRisk::Passive.risk_level() < ScanProfile::Quick.max_risk_budget().risk_level()
        );
        assert!(
            ScanProfile::Stealth.max_risk_budget().risk_level()
                < ScanProfile::Quick.max_risk_budget().risk_level()
        );
    }

    #[test]
    fn defense_lab_profile_operation_mode() {
        assert_eq!(
            ScanProfile::DefenseLab.operation_mode(),
            crate::config::OperationMode::DefenseLab
        );
    }

    #[test]
    fn standard_profile_operation_mode() {
        assert_eq!(
            ScanProfile::Quick.operation_mode(),
            crate::config::OperationMode::StandardAssessment
        );
    }

    #[test]
    fn defense_lab_intended_uses() {
        let uses = ScanProfile::DefenseLab.intended_uses();
        assert!(uses.contains(&crate::config::IntendedUse::WafRegression));
        assert!(uses.contains(&crate::config::IntendedUse::SynvoidRegression));
    }
}
