use super::timeout::*;
use super::CommonHttpArgs;

pub(crate) const SCAN_PORTS_ABOUT: &str = "Scan ports on target host

Performs TCP port scanning to identify open services.
Uses async connections for high-speed scanning.

Examples:
  slapper scan-ports example.com -p 1-1000
  slapper scan-ports 192.168.1.1 -p 22,80,443,8080
  slapper scan-ports example.com -p 1-1024 -c 50
  slapper scan-ports example.com -p 80,443 --json";

pub(crate) const SCAN_ENDPOINTS_ABOUT: &str = "Discover sensitive HTTP endpoints

Scans for hidden or sensitive endpoints using wordlists.
Finds admin panels, config files, backup files, and other sensitive paths.

Examples:
  slapper scan-endpoints https://example.com
  slapper scan-endpoints https://example.com -w wordlist.txt -c 20
  slapper scan-endpoints https://example.com --include-404
  slapper scan-endpoints https://example.com -c 50 --json";

#[cfg(feature = "nse")]
pub(crate) const NSE_ABOUT: &str = "NSE support provides selective compatibility with Nmap Scripting Engine semantics for scriptable discovery and service checks. It is an optional compatibility layer, separate from the removed Python/Ruby plugin runtimes, and should be used for approved scripts within Slapper's scope and execution policy.

Executes Lua-based NSE scripts for security scanning.
Built-in scripts: default, discovery, banner, http-headers

Examples:
  slapper nse example.com -s default
  slapper nse example.com -s banner
  slapper nse https://example.com -s http-headers
  slapper nse example.com -s custom -f script.nse
  slapper nse example.com -s default --script-args userdb=users.txt";

pub(crate) const FINGERPRINT_ABOUT: &str = "Fingerprint services (AMAP-style)

Identifies services running on open ports by analyzing responses.
Supports 20+ protocols including HTTP, SSH, FTP, SMTP, MySQL, PostgreSQL, Redis, MongoDB.

Examples:
  slapper fingerprint example.com
  slapper fingerprint 192.168.1.1 -p 22,80,443,3306
  slapper fingerprint example.com --json
  slapper fingerprint example.com --udp  # Requires root/sudo";

pub(crate) const SCAN_ABOUT: &str = "Run chained security assessment pipeline

Executes multiple scan stages in sequence for comprehensive assessment.
Stages include port scan, fingerprinting, endpoint discovery, fuzzing, and load testing.

Examples:
  slapper scan example.com --profile quick
  slapper scan example.com --profile endpoint
  slapper scan example.com --profile web  # Web-focused assessment
  slapper scan example.com --profile waf  # WAF evaluation
  slapper scan https://example.com --stages port,fingerprint,endpoint
  slapper scan example.com -o report.html --format html
  slapper scan example.com --json -o results.json";

pub(crate) const RESUME_ABOUT: &str = "Resume a previous scan from session file

Restores a scan session that was interrupted or saved.
Session files are created automatically when scans are paused.

Examples:
  slapper resume session.json
  slapper resume /path/to/session.json";

#[derive(clap::Args)]
pub struct PortScanArgs {
    #[arg(help = "Target host (IP or hostname)")]
    pub host: String,
    #[arg(
        short = 'p',
        long,
        default_value = "1-1024",
        help = "Port range (e.g., 1-1024 or 22,80,443)"
    )]
    pub ports: String,
    #[arg(
        short = 'c',
        long,
        default_value = "100",
        help = "Concurrent connections"
    )]
    pub concurrency: usize,
    #[arg(long, default_value_t = PORT_SCAN_TIMEOUT, help = "Connection timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(
        short = 'S',
        long,
        help = "Source IP address to spoof (requires root privileges)"
    )]
    pub source_ip: Option<String>,
    #[arg(
        long,
        help = "Spoof source IP from CIDR range (random IP per request, requires root)"
    )]
    pub spoof_range: Option<String>,
    #[arg(
        long,
        help = "Show what would be sent without actually sending packets"
    )]
    pub dry_run: bool,
    #[arg(
        short = 'D',
        long = "decoy",
        help = "Comma-separated decoy IPs (e.g., 1.1.1.1,2.2.2.2 or 'RANDOM' for random IPs)"
    )]
    pub decoy: Option<String>,
    #[arg(long = "decoy-range", help = "Generate decoys from CIDR range")]
    pub decoy_range: Option<String>,
    #[arg(
        long = "decoy-count",
        help = "Number of random decoys to generate (use with RANDOM or --decoy-range)"
    )]
    pub decoy_count: Option<usize>,
    #[arg(
        long = "decoy-mode",
        help = "Decoy sending mode: 'simultaneous' (all at once) or 'staggered' (spread over time)"
    )]
    pub decoy_mode: Option<String>,
    #[arg(
        long = "include-me",
        help = "Include real IP in decoy list (like nmap -D ME)"
    )]
    pub include_me: bool,
    #[arg(
        short = 'g',
        long = "source-port",
        help = "Source port to use (commonly trusted ports: 80, 443, 53)"
    )]
    pub source_port: Option<u16>,
    #[arg(
        long = "random-source-port",
        help = "Use random source port for each packet"
    )]
    pub random_source_port: bool,
    #[arg(
        short = 'f',
        long = "fragment",
        help = "Split TCP packets into 8-byte fragments for fragmentation testing"
    )]
    pub fragment: bool,
    #[arg(
        long = "scan-type",
        help = "TCP scan type: syn (default), null, fin, xmas"
    )]
    pub scan_type: Option<String>,
    #[arg(
        long = "packet-trace",
        help = "Log all packets sent to a file for analysis"
    )]
    pub packet_trace: Option<String>,
    #[arg(long = "max-rate", help = "Maximum packets per second to send")]
    pub max_rate: Option<u32>,
    #[arg(long = "ttl", help = "Set IP time-to-live (hop limit)")]
    pub ttl: Option<u8>,
    #[arg(long = "grepable", help = "Output in grepable format (like nmap)")]
    pub grepable: bool,
    #[arg(long = "xml", help = "Output in XML format")]
    pub xml: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct EndpointScanArgs {
    #[arg(help = "Target base URL")]
    pub url: String,
    #[arg(short = 'w', long, help = "Custom wordlist file path")]
    pub wordlist: Option<String>,
    #[arg(short = 'c', long, default_value = "20", help = "Concurrent requests")]
    pub concurrency: usize,
    #[arg(long, default_value_t = ENDPOINT_SCAN_TIMEOUT, help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Include 404 responses in output")]
    pub include_404: bool,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(
        long,
        help = "Spoof source IP via HTTP headers (X-Forwarded-For, X-Real-IP)"
    )]
    pub spoof_ip: Option<String>,
    #[arg(
        long,
        help = "Spoof IP range for rotation (CIDR notation, e.g., 10.0.0.0/24)"
    )]
    pub spoof_range: Option<String>,
    #[arg(
        short = 'D',
        long = "decoy",
        help = "Comma-separated decoy IPs for HTTP header spoofing"
    )]
    pub decoy: Option<String>,
    #[arg(
        long = "decoy-range",
        help = "Generate decoys from CIDR range for HTTP header spoofing"
    )]
    pub decoy_range: Option<String>,
    #[arg(long = "decoy-count", help = "Number of random decoys to generate")]
    pub decoy_count: Option<usize>,
    #[arg(
        long = "decoy-mode",
        help = "Decoy rotation mode: 'random' or 'sequential'"
    )]
    pub decoy_mode: Option<String>,
    #[arg(long = "include-me", help = "Include real IP in decoy list")]
    pub include_me: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
}

#[derive(clap::Args)]
pub struct FingerprintArgs {
    #[arg(help = "Target host (IP or hostname)")]
    pub host: String,
    #[arg(
        short = 'p',
        long,
        default_value = "80,443,22,21,25,3306,5432,6379,27017",
        help = "Comma-separated ports to fingerprint"
    )]
    pub ports: String,
    #[arg(long, default_value_t = FINGERPRINT_TIMEOUT, help = "Connection timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, help = "Enable UDP service fingerprinting (requires root/sudo)")]
    pub udp: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
    #[arg(
        short = 'c',
        long,
        default_value = "20",
        help = "Concurrent connections"
    )]
    pub concurrency: usize,
}

#[derive(clap::Args)]
pub struct NseArgs {
    #[arg(help = "Target host or URL")]
    pub target: String,
    #[arg(
        short = 's',
        long,
        default_value = "default",
        help = "NSE script to run (e.g., default, discovery, banner, http-headers)"
    )]
    pub script: String,
    #[arg(long, help = "Script arguments in key=value format (comma-separated)")]
    pub script_args: Option<String>,
    #[arg(short = 'f', long, help = "Path to custom NSE script file")]
    pub script_file: Option<String>,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
}

#[derive(clap::Args, Clone)]
pub struct ScanArgs {
    #[arg(help = "Target host or URL")]
    pub target: String,
    #[arg(
        short = 'p',
        long,
        default_value = "quick",
        help = "Scan profile:\n\
                - quick: port scan + fingerprint\n\
                - endpoint: quick + endpoint discovery\n\
                - web: endpoint + web fuzzing (sqli, xss, ssrf, etc.)\n\
                - waf: web + WAF detection and evasion resistance evaluation\n\
                - full: all stages including load testing\n\
                - api: GraphQL/JWT/OAuth focused assessment\n\
                - recon: intelligence-led with tech detection and CVE mapping\n\
                - stealth: web scan with randomized timing/header behavior for lab realism\n\
                - deep: web scan with mutation fuzzing\n\
                - vuln: CVE-prioritized fuzzing based on detected tech\n\
                - auth: JWT/OAuth/IDOR security testing"
    )]
    pub profile: super::ScanProfile,
    #[arg(
        long,
        help = "Custom stages (comma-separated): port, fingerprint, endpoint, fuzz, load, waf, recon, graphql, oauth, jwt"
    )]
    pub stages: Option<String>,
    #[arg(short = 'c', long, default_value = "10", help = "Concurrent requests")]
    pub concurrency: usize,
    #[arg(
        long,
        help = "Run pipeline stages concurrently instead of sequentially"
    )]
    pub concurrent_stages: bool,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, short = 'o', help = "Output file path")]
    pub output: Option<String>,
    #[arg(long, help = "Output format: json, html, csv, sarif, junit")]
    pub format: Option<super::OutputFormat>,
    #[arg(long, help = "Web payload types for web scan (comma-separated)")]
    pub web_types: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
    #[arg(
        short = 'S',
        long = "source-ip",
        help = "Source IP address to spoof (requires root privileges)"
    )]
    pub source_ip: Option<String>,
    #[arg(
        long = "spoof-range",
        help = "Spoof source IP from CIDR range (random IP per request)"
    )]
    pub spoof_range: Option<String>,
    #[arg(
        short = 'D',
        long = "decoy",
        help = "Comma-separated decoy IPs (e.g., 1.1.1.1,2.2.2.2 or 'RANDOM')"
    )]
    pub decoy: Option<String>,
    #[arg(long = "decoy-range", help = "Generate decoys from CIDR range")]
    pub decoy_range: Option<String>,
    #[arg(long = "decoy-count", help = "Number of random decoys to generate")]
    pub decoy_count: Option<usize>,
    #[arg(
        long = "decoy-mode",
        help = "Decoy mode: 'simultaneous' or 'staggered'"
    )]
    pub decoy_mode: Option<String>,
    #[arg(long = "include-me", help = "Include real IP in decoy list")]
    pub include_me: bool,
    #[arg(short = 'g', long = "source-port", help = "Source port to use")]
    pub source_port: Option<u16>,
    #[arg(
        long = "random-source-port",
        help = "Use random source port for each packet"
    )]
    pub random_source_port: bool,
    #[arg(
        short = 'f',
        long = "fragment",
        help = "Split TCP packets into 8-byte fragments"
    )]
    pub fragment: bool,
    #[arg(
        long = "scan-type",
        help = "TCP scan type: syn (default), null, fin, xmas"
    )]
    pub scan_type: Option<String>,
    #[arg(long = "packet-trace", help = "Log all packets sent to a file")]
    pub packet_trace: Option<String>,
    #[arg(long = "max-rate", help = "Maximum packets per second")]
    pub max_rate: Option<u32>,
    #[arg(long = "ttl", help = "Set IP time-to-live")]
    pub ttl: Option<u8>,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
}

#[derive(clap::Args)]
pub struct ResumeArgs {
    #[arg(help = "Session file path")]
    pub session: String,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
}
