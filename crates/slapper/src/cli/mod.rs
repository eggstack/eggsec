use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};

pub mod cluster;
pub mod fuzz;
pub mod http;
pub mod misc;
pub mod packet;
pub mod scan;
pub mod stress;

pub use cluster::*;
pub use fuzz::*;
pub use http::*;
pub use misc::*;
pub use packet::*;
pub use scan::*;
#[cfg(feature = "stress-testing")]
pub use stress::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "slapper")]
#[command(about = "High-performance security testing toolkit")]
#[command(version = VERSION, long_about = None)]
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
pub enum Commands {
    // --- Scan operations ---
    #[command(about = "Scan ports on target host", long_about = SCAN_PORTS_ABOUT, alias = "scan-ports")]
    ScanPorts(PortScanArgs),
    #[command(about = "Discover sensitive HTTP endpoints", long_about = SCAN_ENDPOINTS_ABOUT, alias = "scan-endpoints")]
    ScanEndpoints(EndpointScanArgs),
    #[command(about = "Fingerprint services (AMAP-style)", long_about = FINGERPRINT_ABOUT)]
    Fingerprint(FingerprintArgs),
    #[command(about = "Run chained security assessment pipeline", long_about = SCAN_ABOUT)]
    Scan(ScanArgs),
    #[command(about = "Resume a previous scan from session file", long_about = RESUME_ABOUT)]
    Resume(ResumeArgs),

    // --- Attack operations ---
    #[command(about = "Fuzz target with security payloads", long_about = FUZZ_ABOUT)]
    Fuzz(FuzzArgs),
    #[command(about = "Detect and bypass Web Application Firewalls", long_about = WAF_ABOUT)]
    Waf(WafArgs),
    #[command(about = "Comprehensive WAF stress testing", long_about = WAF_STRESS_ABOUT, alias = "waf-stress")]
    WafStress(WafStressArgs),
    #[command(about = "Test GraphQL endpoints for security issues", long_about = GRAPHQL_ABOUT)]
    Graphql(GraphQlArgs),
    #[command(about = "Test OAuth/OIDC endpoints for vulnerabilities", long_about = OAUTH_ABOUT)]
    OAuth(OAuthArgs),

    // --- Recon operations ---
    #[command(about = "Gather reconnaissance information", long_about = RECON_ABOUT)]
    Recon(ReconArgs),

    // --- Load testing ---
    #[command(about = "Run HTTP load test against target URL", long_about = LOAD_ABOUT)]
    Load(LoadArgs),

    // --- Tool operations ---
    #[command(about = "Packet inspection and analysis tools", long_about = PACKET_ABOUT)]
    Packet(PacketArgs),
    #[cfg(feature = "nse")]
    #[command(about = "Run Nmap NSE scripts for security scanning", long_about = NSE_ABOUT)]
    Nse(NseArgs),
    #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
    #[command(about = "Manage and run security scanning plugins")]
    Plugin(PluginArgs),
    #[command(about = "Convert and generate security scan reports")]
    Report(ReportArgs),

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
        help = "Enable stealth mode (randomized delays, header rotation)"
    )]
    pub stealth: bool,
    #[arg(long, help = "Rate limit (requests per second)")]
    pub rate_limit: Option<u32>,
    #[arg(long, help = "Random delay between requests (ms range, e.g., 100-500)")]
    pub jitter: Option<String>,
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Compact,
    Html,
    Csv,
    Sarif,
    Junit,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Pretty => write!(f, "pretty"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Compact => write!(f, "compact"),
            OutputFormat::Html => write!(f, "html"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Sarif => write!(f, "sarif"),
            OutputFormat::Junit => write!(f, "junit"),
        }
    }
}
