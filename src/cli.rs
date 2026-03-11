use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const LOAD_ABOUT: &str = "Run HTTP load test against target URL

Sends concurrent HTTP requests to measure server performance and gather metrics.
Useful for identifying bottlenecks and testing server resilience.

Examples:
  slapper load https://example.com -n 1000 -c 50
  slapper load https://api.example.com/endpoint -n 500 -c 20 -m POST -d '{\"query\": \"test\"}'
  slapper load https://example.com -n 100 --json
  slapper load https://example.com -n 200 -c 10 --proxy http://127.0.0.1:8080";

const SCAN_PORTS_ABOUT: &str = "Scan ports on target host

Performs TCP port scanning to identify open services.
Uses async connections for high-speed scanning.

Examples:
  slapper scan-ports example.com -p 1-1000
  slapper scan-ports 192.168.1.1 -p 22,80,443,8080
  slapper scan-ports example.com -p 1-1024 -c 50
  slapper scan-ports example.com -p 80,443 --json";

const SCAN_ENDPOINTS_ABOUT: &str = "Discover sensitive HTTP endpoints

Scans for hidden or sensitive endpoints using wordlists.
Finds admin panels, config files, backup files, and other sensitive paths.

Examples:
  slapper scan-endpoints https://example.com
  slapper scan-endpoints https://example.com -w wordlist.txt -c 20
  slapper scan-endpoints https://example.com --include-404
  slapper scan-endpoints https://example.com -c 50 --json";

const NSE_ABOUT: &str = "Run Nmap NSE (Scripting Engine) scripts

Executes Lua-based NSE scripts for security scanning.
Built-in scripts: default, discovery, banner, http-headers

Examples:
  slapper nse example.com -s default
  slapper nse example.com -s banner
  slapper nse https://example.com -s http-headers
  slapper nse example.com -s custom -f script.nse
  slapper nse example.com -s default --script-args userdb=users.txt";

const FINGERPRINT_ABOUT: &str = "Fingerprint services (AMAP-style)

Identifies services running on open ports by analyzing responses.
Supports 20+ protocols including HTTP, SSH, FTP, SMTP, MySQL, PostgreSQL, Redis, MongoDB.

Examples:
  slapper fingerprint example.com
  slapper fingerprint 192.168.1.1 -p 22,80,443,3306
  slapper fingerprint example.com --json
  slapper fingerprint example.com --udp  # Requires root/sudo";

const FUZZ_ABOUT: &str = "Fuzz target with security payloads

Tests applications for vulnerabilities using various payload types.
Supports SQL injection, XSS, path traversal, SSRF, open redirects, ReDoS, and more.

Examples:
  slapper fuzz https://example.com/api?id=1 -t sqli
  slapper fuzz https://example.com/search?q=test -t xss
  slapper fuzz https://example.com -t all
  slapper fuzz https://example.com -t sqli,xss,graphql -c 20
  slapper fuzz https://example.com -t ssrf --param url
  slapper fuzz https://example.com -t xss --mutate -m 5
  slapper fuzz https://example.com -t xss --target nginx
  slapper fuzz https://example.com/graphql -t graphql  # GraphQL testing
  slapper fuzz https://api.example.com -t jwt  # JWT testing
  slapper fuzz https://oauth.example.com -t oauth  # OAuth/OIDC testing";

const WAF_STRESS_ABOUT: &str = "Comprehensive WAF stress testing

Applies all payload types to test WAF detection and bypass capabilities.
Useful for WAF evaluation and tuning.

Examples:
  slapper waf-stress https://example.com
  slapper waf-stress https://example.com -c 50
  slapper waf-stress https://example.com --json";

const WAF_ABOUT: &str = "Detect and bypass Web Application Firewalls

Detects WAF presence and attempts various bypass techniques.
Can detect 30+ WAF products and attempt header manipulation, HTTP smuggling, and evasion.

Examples:
  slapper waf https://example.com
  slapper waf https://example.com --detect-only
  slapper waf https://example.com --bypass
  slapper waf https://example.com --header-bypass --smuggling
  slapper waf https://example.com --bypass -c 20
  slapper waf https://example.com --profile cloudflare  # WAF-specific bypass
  slapper waf https://example.com --evasion  # Advanced evasion techniques";

const SCAN_ABOUT: &str = "Run chained security assessment pipeline

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

const GRAPHQL_ABOUT: &str = "Test GraphQL endpoints for security issues

Performs GraphQL-specific security testing including:
- Introspection enumeration
- Query injection
- Depth limit bypass
- Alias overload DoS
- Batch query testing

Examples:
  slapper graphql https://api.example.com/graphql
  slapper graphql https://api.example.com/graphql --introspection
  slapper graphql https://api.example.com/graphql --inject
  slapper graphql https://api.example.com/graphql --json";

const OAUTH_ABOUT: &str = "Test OAuth/OIDC endpoints for vulnerabilities

Tests OAuth/OIDC implementations for common issues:
- Redirect URI validation bypass
- State parameter bypass
- Scope escalation
- PKCE bypass
- Grant type mixing

Examples:
  slapper oauth https://oauth.example.com/authorize
  slapper oauth https://oauth.example.com --redirect-test
  slapper oauth https://oauth.example.com --scope-test
  slapper oauth https://oauth.example.com --json";

const RECON_ABOUT: &str = "Gather reconnaissance information

Collects comprehensive intelligence about a target including:
- Technology stack detection
- DNS records and subdomain enumeration
- Geolocation and WHOIS information
- SSL/TLS analysis
- Cloud asset discovery
- CORS configuration
- Threat intelligence
- CVE mapping

Examples:
  slapper recon example.com
  slapper recon example.com --no-tech --no-whois
  slapper recon example.com --concurrency 20
  slapper recon example.com --json";

const RESUME_ABOUT: &str = "Resume a previous scan from session file

Restores a scan session that was interrupted or saved.
Session files are created automatically when scans are paused.

Examples:
  slapper resume session.json
  slapper resume /path/to/session.json";

const PACKET_ABOUT: &str = "Packet inspection and analysis tools

Provides tools for live packet capture, packet crafting, hexdump view,
header inspection, and traceroute functionality.
NOTE: Live packet capture requires building with --features packet-inspection
Requires root/sudo for live packet capture.

Examples:
  slapper packet capture -i eth0
  slapper packet capture -i eth0 --filter tcp --max 100
  slapper packet send --tcp --dst example.com:80 --flags SYN
  slapper packet dump capture.pcap
  slapper packet traceroute example.com
  slapper packet interfaces";

#[cfg(feature = "stress-testing")]
const ICMP_ABOUT: &str = "Send ICMP echo probes to target host

Performs ICMP ping to measure reachability and round-trip time.
Requires root privileges for raw ICMP sockets.
NOTE: Requires building with --features stress-testing

Examples:
  slapper icmp 8.8.8.8
  slapper icmp example.com -c 10
  slapper icmp 192.168.1.1 --timeout 5 --json";

#[cfg(feature = "stress-testing")]
const TRACEROUTE_ABOUT: &str = "Trace network path to target host

Performs traceroute to discover the path packets take to reach a destination.
Supports both UDP and ICMP modes.
NOTE: Requires building with --features stress-testing

Examples:
  slapper traceroute 8.8.8.8
  slapper traceroute example.com --icmp
  slapper traceroute 192.168.1.1 --max-hops 30";

#[cfg(feature = "stress-testing")]
const STRESS_ABOUT: &str = "Run stress/load testing against target

Performs various stress testing techniques including SYN, UDP, HTTP, TCP, and ICMP floods.
WARNING: Only use on systems you own or have explicit permission to test.
NOTE: Requires building with --features stress-testing

Examples:
  slapper stress example.com --type http -r 1000 -d 60
  slapper stress example.com --type syn -r 5000 -d 30
  slapper stress 192.168.1.1:80 --type udp -r 10000 -d 120";

#[cfg(feature = "stress-testing")]
const PROXY_ABOUT: &str = "Manage proxy pool and rotation

Manages proxy lists for scan distribution and stealth.
Supports SOCKS4, SOCKS5, HTTP, HTTPS, and Tor proxies.
NOTE: Requires building with --features stress-testing

Examples:
  slapper proxy add --file proxies.txt
  slapper proxy list --healthy
  slapper proxy health-check
  slapper proxy rotate";

const CLUSTER_ABOUT: &str = "Manage distributed scanning cluster

Starts worker or coordinator nodes for distributed scanning.
Workers execute tasks, coordinators manage job distribution.

Examples:
  slapper cluster worker --workers 4
  slapper cluster coordinator --port 9000
  slapper cluster status";

const NOTIFY_ABOUT: &str = "Test and manage notifications

Tests webhook integrations and sends test notifications.
Supports Slack, Discord, Teams, and custom webhooks.

Examples:
  slapper notify test --slack
  slapper notify test --discord
  slapper notify test --webhook https://example.com/hook
  slapper notify send --finding 'SQL Injection found'";

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
    #[command(about = "Run HTTP load test against target URL", long_about = LOAD_ABOUT)]
    Load(LoadArgs),
    #[command(about = "Scan ports on target host", long_about = SCAN_PORTS_ABOUT)]
    ScanPorts(PortScanArgs),
    #[command(about = "Discover sensitive HTTP endpoints", long_about = SCAN_ENDPOINTS_ABOUT)]
    ScanEndpoints(EndpointScanArgs),
    #[command(about = "Fingerprint services (AMAP-style)", long_about = FINGERPRINT_ABOUT)]
    Fingerprint(FingerprintArgs),
    #[cfg(feature = "nse")]
    #[command(about = "Run Nmap NSE scripts for security scanning")]
    Nse(NseArgs),
    #[command(about = "Fuzz target with security payloads", long_about = FUZZ_ABOUT)]
    Fuzz(FuzzArgs),
    #[command(about = "Comprehensive WAF stress testing", long_about = WAF_STRESS_ABOUT)]
    WafStress(WafStressArgs),
    #[command(about = "Detect and bypass Web Application Firewalls", long_about = WAF_ABOUT)]
    Waf(WafArgs),
    #[command(about = "Run chained security assessment pipeline", long_about = SCAN_ABOUT)]
    Scan(ScanArgs),
    #[command(about = "Gather reconnaissance information", long_about = RECON_ABOUT)]
    Recon(ReconArgs),
    #[command(about = "Test GraphQL endpoints for security issues", long_about = GRAPHQL_ABOUT)]
    Graphql(GraphQlArgs),
    #[command(about = "Test OAuth/OIDC endpoints for vulnerabilities", long_about = OAUTH_ABOUT)]
    OAuth(OAuthArgs),
    #[command(about = "Resume a previous scan from session file", long_about = RESUME_ABOUT)]
    Resume(ResumeArgs),
    #[command(about = "Packet inspection and analysis tools", long_about = PACKET_ABOUT)]
    Packet(PacketArgs),
    #[cfg(feature = "stress-testing")]
    #[command(about = "Send ICMP echo probes to target host", long_about = ICMP_ABOUT)]
    Icmp(IcmpArgs),
    #[cfg(feature = "stress-testing")]
    #[command(about = "Trace network path to target host", long_about = TRACEROUTE_ABOUT)]
    Traceroute(TracerouteArgs),
    #[cfg(feature = "python-plugins")]
    #[command(about = "Manage and run security scanning plugins")]
    Plugin(PluginArgs),
    #[command(about = "Convert and generate security scan reports")]
    Report(ReportArgs),
    #[cfg(feature = "stress-testing")]
    #[command(about = "Run stress/load testing against target", long_about = STRESS_ABOUT)]
    Stress(StressArgs),
    #[cfg(feature = "stress-testing")]
    #[command(about = "Manage proxy pool and rotation", long_about = PROXY_ABOUT)]
    Proxy(ProxyArgs),
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
    #[cfg(feature = "mcp-server")]
    #[command(about = "Start MCP server for AI assistant integration")]
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

#[derive(clap::Args, Clone)]
pub struct LoadArgs {
    #[arg(help = "Target URL")]
    pub url: String,
    #[arg(
        short = 'n',
        long,
        default_value = "100",
        help = "Total number of requests"
    )]
    pub requests: u64,
    #[arg(
        short = 'c',
        long,
        default_value = "10",
        help = "Concurrent connections"
    )]
    pub concurrency: usize,
    #[arg(short = 'm', long, default_value = "GET", help = "HTTP method")]
    pub method: String,
    #[arg(short = 'd', long, help = "Request body")]
    pub body: Option<String>,
    #[arg(long, help = "Request headers (format: Key:Value)")]
    pub headers: Vec<String>,
    #[arg(long, default_value = "30", help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(
        long,
        help = "Output results as JSON (use --global-json for same effect)"
    )]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
}

#[derive(clap::Args)]
pub struct EndpointScanArgs {
    #[arg(help = "Target base URL")]
    pub url: String,
    #[arg(short = 'w', long, help = "Custom wordlist file path")]
    pub wordlist: Option<String>,
    #[arg(short = 'c', long, default_value = "20", help = "Concurrent requests")]
    pub concurrency: usize,
    #[arg(long, default_value = "10", help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Include 404 responses in output")]
    pub include_404: bool,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
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
    #[arg(long, help = "Output to file")]
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
    #[arg(long, default_value = "5", help = "Connection timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Enable UDP service fingerprinting (requires root/sudo)")]
    pub udp: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}

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
    #[arg(long, default_value = "2", help = "Connection timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
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
        help = "Split TCP packets into 8-byte fragments to bypass WAFs"
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
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
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
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}

#[derive(clap::Args, Clone)]
pub struct FuzzArgs {
    #[arg(help = "Target URL with parameter(s)")]
    pub url: String,
    #[arg(
        short = 't',
        long,
        default_value = "all",
        help = "Payload types (comma-separated): sqli, xss, traversal, ssrf, redirect, redos, headers, compression, graphql, oauth, jwt, idor, ssti, xxe, ldap, cmd, deser, host, cache, csv, soap, all\n\
                Aliases: sql (sqli), lfi/traversal (path), open-redirect (redirect), regex (redos), gzip/compression (compression)\n\
                Advanced fuzzing: graphql, oauth, jwt, idor, ssti, websocket, grpc (uses specialized fuzzers with deeper testing)\n\
                New: xxe (XML XXE), ldap (LDAP injection), cmd (Command injection), deser (Deserialization), host (Host header), cache (Cache poisoning), csv (CSV injection), soap (SOAP/XML)"
    )]
    pub payload_type: String,
    #[arg(
        short = 'M',
        long,
        default_value = "sequential",
        help = "Fuzzing mode: sequential (one-by-one), burst (concurrent), adaptive (auto-adjusts rate)"
    )]
    pub mode: FuzzMode,
    #[arg(long, help = "Enable mutation-based fuzzing")]
    pub mutate: bool,
    #[arg(long, default_value = "3", help = "Number of mutations per payload")]
    pub mutation_count: usize,
    #[arg(long, help = "Enable grammar-based fuzzing (generative)")]
    pub grammar_fuzz: bool,
    #[arg(long, help = "Grammar type: json, graphql, xml, jwt, ssti")]
    pub grammar_type: Option<String>,
    #[arg(
        long,
        help = "Enable adaptive rate limiting (auto-adjusts to server responses)"
    )]
    pub adaptive_rate: bool,
    #[arg(long, help = "Enable HTTP session/cookie handling")]
    pub session: bool,
    #[arg(long, help = "Enable response diffing (compare with baseline)")]
    pub diffing: bool,
    #[arg(long, help = "Capture baseline response before fuzzing")]
    pub capture_baseline: bool,
    #[arg(long, help = "Enable enhanced ReDoS detection (execute regexes)")]
    pub enhanced_redos: bool,
    #[arg(
        long,
        help = "Enable WAF fingerprinting (detect specific WAF products)"
    )]
    pub waf_fingerprint: bool,
    #[arg(long, help = "Enable request chaining for auto-exploitation")]
    pub chaining: bool,
    #[arg(long, help = "Chain file path (YAML/JSON with action chain)")]
    pub chain_file: Option<String>,
    #[arg(short = 'm', long, default_value = "GET", help = "HTTP method")]
    pub method: String,
    #[arg(
        short = 'p',
        long,
        help = "Parameter name to inject payloads into (default: auto-detect)"
    )]
    pub param: Option<String>,
    #[arg(
        short = 'c',
        long,
        default_value = "10",
        help = "Concurrent requests (used in burst mode)"
    )]
    pub concurrency: usize,
    #[arg(long, default_value = "10", help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, short = 'o', help = "Output file path")]
    #[arg(default_value = "None")]
    pub output: Option<String>,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output format: json, html, csv, markdown")]
    #[arg(default_value = "None")]
    pub format: Option<OutputFormat>,
    #[arg(
        long,
        help = "Target type for specific payloads: nginx, apache, php, generic (adds target-specific attack payloads)"
    )]
    pub target: Option<String>,
    #[arg(long, help = "JWT token to test (enables advanced JWT fuzzing)")]
    pub jwt_token: Option<String>,
    #[arg(
        long,
        help = "OAuth issuer URL (enables advanced OAuth fuzzing, e.g., https://auth.example.com)"
    )]
    pub oauth_issuer: Option<String>,
    #[arg(long, help = "OAuth client ID for testing")]
    pub oauth_client_id: Option<String>,
    #[arg(long, help = "OAuth client secret for testing")]
    pub oauth_client_secret: Option<String>,
    #[arg(
        long,
        help = "IDOR base user ID for testing (enables advanced IDOR fuzzing, e.g., 1)"
    )]
    pub idor_base_id: Option<String>,
    #[arg(
        long,
        help = "Comma-separated user IDs for IDOR testing (e.g., 1,2,3,admin)"
    )]
    pub idor_user_ids: Option<String>,
    #[arg(long, help = "Parameter name for SSTI fuzzing (default: name)")]
    pub ssti_param: Option<String>,
    #[arg(
        long,
        default_value = "true",
        help = "Enable GraphQL introspection: queries schema structure and field suggestions"
    )]
    pub graphql_introspection: bool,
    #[arg(
        long,
        default_value = "true",
        help = "Enable GraphQL depth bypass: tests deeply nested queries for DoS vulnerabilities"
    )]
    pub graphql_depth_bypass: bool,
    #[arg(
        long,
        default_value = "true",
        help = "Enable GraphQL alias overload: tests multiple aliases to bypass rate limits"
    )]
    pub graphql_alias_overload: bool,
    #[arg(
        long,
        default_value = "true",
        help = "Enable OAuth redirect URI testing: checks for open redirect vulnerabilities"
    )]
    pub oauth_redirect: bool,
    #[arg(
        long,
        default_value = "true",
        help = "Enable OAuth scope escalation: tests for dangerous/privileged scope requests"
    )]
    pub oauth_scope: bool,
    #[arg(
        long,
        default_value = "true",
        help = "Enable OAuth state parameter testing: checks for CSRF via missing state param"
    )]
    pub oauth_state: bool,
    #[arg(
        long,
        default_value = "true",
        help = "Enable OAuth grant type testing: tests for insecure grant type mixing"
    )]
    pub oauth_grant: bool,
    #[command(flatten)]
    pub common: CommonHttpArgs,
}

#[derive(clap::Args, Clone)]
pub struct WafStressArgs {
    #[arg(help = "Target URL")]
    pub url: String,
    #[arg(short = 'c', long, default_value = "20", help = "Concurrent requests")]
    pub concurrency: usize,
    #[arg(long, default_value = "10", help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
}

#[derive(clap::Args, Clone)]
pub struct WafArgs {
    #[arg(help = "Target URL")]
    pub url: String,
    #[arg(short = 'd', long, help = "Detect WAF only (no bypass attempts)")]
    pub detect_only: bool,
    #[arg(short = 'b', long, help = "Attempt all bypass techniques")]
    pub bypass: bool,
    #[arg(long, help = "Enable header manipulation bypass techniques")]
    pub header_bypass: bool,
    #[arg(long, help = "Enable HTTP request smuggling bypass")]
    pub smuggling: bool,
    #[arg(long, help = "Enable ML-based evasion techniques")]
    pub evasion: bool,
    #[arg(
        long,
        default_value = "auto",
        help = "WAF-specific bypass profile: cloudflare, akamai, aws-waf, azure-waf, imperva, f5-asm, cloudfront, sucuri, auto"
    )]
    pub profile: String,
    #[arg(
        long,
        help = "Test specific payload types: sqli, xss, ssrf, cmd, traversal, all"
    )]
    pub test_type: Option<String>,
    #[arg(short = 'c', long, default_value = "10", help = "Concurrent requests")]
    pub concurrency: usize,
    #[arg(long, default_value = "15", help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
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
                - waf: web + WAF detection and bypass testing\n\
                - full: all stages including load testing\n\
                - api: GraphQL/JWT/OAuth focused assessment\n\
                - recon: intelligence-led with tech detection and CVE mapping\n\
                - stealth: web scan with evasion techniques\n\
                - deep: web scan with mutation fuzzing\n\
                - vuln: CVE-prioritized fuzzing based on detected tech\n\
                - auth: JWT/OAuth/IDOR security testing"
    )]
    pub profile: ScanProfile,
    #[arg(
        long,
        help = "Custom stages (comma-separated): port, fingerprint, endpoint, fuzz, load, waf, recon, graphql, oauth, jwt"
    )]
    pub stages: Option<String>,
    #[arg(short = 'c', long, default_value = "10", help = "Concurrent requests")]
    pub concurrency: usize,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, short = 'o', help = "Output file path")]
    pub output: Option<String>,
    #[arg(long, help = "Output format: json, html, csv, sarif, junit")]
    pub format: Option<OutputFormat>,
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
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct ReconArgs {
    #[arg(help = "Target domain or URL")]
    pub target: String,
    #[arg(long, help = "Skip technology stack detection")]
    pub no_tech: bool,
    #[arg(long, help = "Skip reverse DNS lookup")]
    pub no_dns: bool,
    #[arg(long, help = "Skip geolocation lookup")]
    pub no_geo: bool,
    #[arg(long, help = "Skip WHOIS lookup")]
    pub no_whois: bool,
    #[arg(long, help = "Skip subdomain enumeration")]
    pub no_subdomains: bool,
    #[arg(long, help = "Skip SSL/TLS analysis")]
    pub no_ssl: bool,
    #[arg(long, help = "Skip DNS record enumeration (A, AAAA, MX, TXT, etc.)")]
    pub no_dns_records: bool,
    #[arg(long, help = "Skip JavaScript file analysis")]
    pub no_js: bool,
    #[arg(long, help = "Skip content/sensitive file discovery")]
    pub no_content: bool,
    #[arg(long, help = "Skip cloud asset enumeration (AWS, Azure, GCP)")]
    pub no_cloud: bool,
    #[arg(long, help = "Skip Wayback Machine integration")]
    pub no_wayback: bool,
    #[arg(long, help = "Skip CORS configuration analysis")]
    pub no_cors: bool,
    #[arg(long, help = "Skip threat intelligence lookup")]
    pub no_threat: bool,
    #[arg(long, help = "Skip CVE mapping based on detected technologies")]
    pub no_cve: bool,
    #[arg(long, help = "Skip email/contact discovery")]
    pub no_email: bool,
    #[arg(long, help = "Concurrency for parallel scans (default: 10)")]
    pub concurrency: Option<usize>,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Quiet mode (no spinner)")]
    pub quiet: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}

#[derive(clap::Args, Clone)]
pub struct GraphQlArgs {
    #[arg(help = "GraphQL endpoint URL")]
    pub url: String,
    #[arg(long, help = "Run introspection tests")]
    pub introspection: bool,
    #[arg(long, help = "Run query injection tests")]
    pub inject: bool,
    #[arg(long, help = "Run depth limit bypass tests")]
    pub depth_bypass: bool,
    #[arg(long, help = "Run alias overload tests")]
    pub alias_overload: bool,
    #[arg(short = 'c', long, default_value = "10", help = "Concurrent requests")]
    pub concurrency: usize,
    #[arg(long, default_value = "15", help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
}

#[derive(clap::Args, Clone)]
pub struct OAuthArgs {
    #[arg(help = "OAuth/OIDC authorization endpoint URL")]
    pub url: String,
    #[arg(long, help = "Client ID for testing")]
    pub client_id: Option<String>,
    #[arg(long, help = "Redirect URI for testing")]
    pub redirect_uri: Option<String>,
    #[arg(long, help = "Run redirect URI validation tests")]
    pub redirect_test: bool,
    #[arg(long, help = "Run scope escalation tests")]
    pub scope_test: bool,
    #[arg(long, help = "Run state parameter tests")]
    pub state_test: bool,
    #[arg(long, help = "Run grant type tests")]
    pub grant_test: bool,
    #[arg(short = 'c', long, default_value = "10", help = "Concurrent requests")]
    pub concurrency: usize,
    #[arg(long, default_value = "15", help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Json,
    Html,
    Csv,
    Sarif,
    Junit,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Html => write!(f, "html"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Sarif => write!(f, "sarif"),
            OutputFormat::Junit => write!(f, "junit"),
        }
    }
}

#[derive(clap::Args)]
pub struct PacketArgs {
    #[command(subcommand)]
    pub command: PacketSubcommand,
}

#[derive(clap::Subcommand)]
pub enum PacketSubcommand {
    #[command(about = "Capture packets from network interface")]
    Capture(PacketCaptureArgs),
    #[command(about = "Craft and send custom packets")]
    Send(PacketSendArgs),
    #[command(about = "Hexdump a pcap file or packet data")]
    Dump(PacketDumpArgs),
    #[command(about = "Trace network route to target")]
    Traceroute(PacketTracerouteArgs),
    #[command(about = "List available network interfaces")]
    Interfaces,
}

#[derive(clap::Args)]
pub struct PacketCaptureArgs {
    #[arg(short = 'i', long, help = "Network interface name")]
    pub interface: Option<String>,
    #[arg(long, help = "BPF filter expression (e.g., 'tcp port 80')")]
    pub filter: Option<String>,
    #[arg(long, default_value = "100", help = "Maximum packets to capture")]
    pub max: Option<usize>,
    #[arg(long, help = "Output file for pcap")]
    pub output: Option<String>,
    #[arg(long, help = "Promiscuous mode")]
    pub promiscuous: bool,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
}

#[derive(clap::Args)]
pub struct PacketSendArgs {
    #[arg(help = "Target host")]
    pub target: String,
    #[arg(long, help = "Source IP address")]
    pub src_ip: Option<String>,
    #[arg(long, help = "Source port")]
    pub src_port: Option<u16>,
    #[arg(long, help = "Destination port")]
    pub dst_port: Option<u16>,
    #[arg(long, help = "TCP flags (syn,ack,rst,fin,psh,urg)")]
    pub flags: Option<String>,
    #[arg(long, help = "Use ICMP instead of TCP/UDP")]
    pub icmp: bool,
    #[arg(long, help = "UDP mode")]
    pub udp: bool,
    #[arg(long, help = "Packet payload (hex string)")]
    pub payload: Option<String>,
    #[arg(long, help = "TTL/Hop limit")]
    pub ttl: Option<u8>,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct PacketDumpArgs {
    #[arg(help = "File to dump (pcap or raw packet data)")]
    pub file: String,
    #[arg(long, help = "Number of packets to show")]
    pub count: Option<usize>,
    #[arg(long, help = "Show only packet at index")]
    pub index: Option<usize>,
    #[arg(long, help = "Show hexdump only")]
    pub hex_only: bool,
    #[arg(long, help = "Show parsed headers only")]
    pub headers_only: bool,
    #[arg(long, help = "Bytes per line")]
    pub bytes_per_line: Option<usize>,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct PacketTracerouteArgs {
    #[arg(help = "Target host")]
    pub target: String,
    #[arg(long, default_value = "30", help = "Maximum hops")]
    pub max_hops: u8,
    #[arg(long, default_value = "3", help = "Number of probes per hop")]
    pub probes: u8,
    #[arg(long, help = "Use ICMP Echo Request (requires root/sudo)")]
    pub icmp: bool,
    #[arg(long, help = "Use UDP probes (default, no root required)")]
    pub udp: bool,
    #[arg(long, help = "Timeout in seconds")]
    pub timeout: Option<u64>,
    #[arg(long, help = "First TTL")]
    pub first_ttl: Option<u8>,
    #[arg(long, help = "Run probes in parallel")]
    pub parallel: bool,
    #[arg(long, help = "Disable reverse DNS lookup")]
    pub no_resolve: bool,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct IcmpArgs {
    #[arg(help = "Target host or IP address")]
    pub target: String,
    #[arg(
        short = 'c',
        long,
        default_value = "4",
        help = "Number of ping requests"
    )]
    pub count: u32,
    #[arg(short = 'W', long, default_value = "2", help = "Timeout in seconds")]
    pub timeout: u64,
    #[arg(
        short = 'i',
        long,
        default_value = "1",
        help = "Interval between probes in seconds"
    )]
    pub interval: f64,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct TracerouteArgs {
    #[arg(help = "Target host or IP address")]
    pub target: String,
    #[arg(long, default_value = "30", help = "Maximum number of hops")]
    pub max_hops: u8,
    #[arg(long, default_value = "3", help = "Timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Use ICMP probes (requires root/sudo)")]
    pub icmp: bool,
    #[arg(long, help = "Use UDP probes (default, no root required)")]
    pub udp: bool,
    #[arg(long, help = "Run probes in parallel")]
    pub parallel: bool,
    #[arg(long, help = "Disable reverse DNS lookup")]
    pub no_resolve: bool,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
}

#[cfg(feature = "python-plugins")]
#[derive(clap::Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub command: PluginCommand,
}

#[cfg(feature = "python-plugins")]
#[derive(clap::Subcommand)]
pub enum PluginCommand {
    #[command(about = "List available plugins")]
    List(PluginListArgs),
    #[command(about = "Run a plugin against a target")]
    Run(PluginRunArgs),
}

#[cfg(feature = "python-plugins")]
#[derive(clap::Args)]
pub struct PluginListArgs {
    #[arg(short = 'v', long, help = "Show verbose plugin information")]
    pub verbose: bool,
}

#[cfg(feature = "python-plugins")]
#[derive(clap::Args)]
pub struct PluginRunArgs {
    #[arg(help = "Plugin name to run")]
    pub name: String,
    #[arg(help = "Target URL or host")]
    pub target: String,
    #[arg(short = 'o', long, help = "Output results to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct ReportArgs {
    #[command(subcommand)]
    pub command: ReportCommand,
}

#[derive(clap::Subcommand)]
pub enum ReportCommand {
    #[command(about = "Convert scan results between formats")]
    Convert(ReportConvertArgs),
    #[command(about = "Analyze trends between multiple scan results")]
    Trend(ReportTrendArgs),
    #[command(about = "Manage scheduled scans")]
    Schedule(ScheduleArgs),
}

#[derive(clap::Args)]
pub struct ReportConvertArgs {
    #[arg(help = "Input scan results file (JSON)")]
    pub input: String,
    #[arg(short = 'f', long, help = "Output format", value_enum)]
    pub format: ReportFormat,
    #[arg(short = 'o', long, help = "Output file (stdout if not specified)")]
    pub output: Option<String>,
}

#[derive(clap::ValueEnum, Clone)]
pub enum ReportFormat {
    Json,
    Csv,
    Junit,
    Sarif,
    Html,
    Markdown,
}

#[derive(clap::Args)]
pub struct ReportTrendArgs {
    #[arg(help = "Previous scan results file")]
    pub before: String,
    #[arg(help = "Recent scan results file")]
    pub after: String,
    #[arg(short = 'o', long, help = "Output file (stdout if not specified)")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct ScheduleArgs {
    #[command(subcommand)]
    pub command: ScheduleCommand,
}

#[derive(clap::Subcommand)]
pub enum ScheduleCommand {
    #[command(about = "List scheduled scans")]
    List,
    #[command(about = "Add a new scheduled scan")]
    Add(ScheduleAddArgs),
    #[command(about = "Remove a scheduled scan")]
    Remove(ScheduleRemoveArgs),
    #[command(about = "Generate crontab entry for scheduled scan")]
    Cron(ScheduleCronArgs),
}

#[derive(clap::Args)]
pub struct ScheduleAddArgs {
    #[arg(help = "Cron schedule expression (e.g., '0 */6 * * *')")]
    pub schedule: String,
    #[arg(help = "Target host or URL")]
    pub target: String,
    #[arg(long, help = "Scan type", default_value = "scan")]
    pub scan_type: String,
    #[arg(short = 'o', long, help = "Output results to file")]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct ScheduleRemoveArgs {
    #[arg(help = "Schedule ID to remove")]
    pub id: String,
}

#[derive(clap::Args)]
pub struct ScheduleCronArgs {
    #[arg(
        help = "Schedule ID to generate crontab entry for (optional, generates all if not specified)"
    )]
    pub id: Option<String>,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct StressArgs {
    #[arg(help = "Target host or IP:port")]
    pub target: String,
    #[arg(
        long,
        default_value = "http",
        help = "Stress type: syn, udp, http, tcp, icmp"
    )]
    pub stress_type: StressTypeArg,
    #[arg(
        short = 'r',
        long,
        default_value = "1000",
        help = "Rate in packets/requests per second"
    )]
    pub rate: u64,
    #[arg(short = 'd', long, default_value = "60", help = "Duration in seconds")]
    pub duration: u64,
    #[arg(
        short = 'c',
        long,
        default_value = "10",
        help = "Concurrency (number of concurrent connections)"
    )]
    pub concurrency: usize,
    #[arg(long, help = "Source port")]
    pub src_port: Option<u16>,
    #[arg(long, help = "Spoof source IP address")]
    pub spoof: bool,
    #[arg(long, help = "Spoof source IP from CIDR range")]
    pub spoof_range: Option<String>,
    #[arg(long, help = "Random source port for each request")]
    pub random_port: bool,
    #[arg(long, help = "Payload size in bytes (for UDP)")]
    pub payload_size: Option<usize>,
    #[arg(long, help = "Use proxy pool")]
    pub use_proxies: bool,
    #[arg(long, help = "Proxy pool file")]
    pub proxy_file: Option<String>,
    #[arg(long, help = "Output results as JSON")]
    #[arg(hide = true)]
    pub json: bool,
}

#[cfg(feature = "stress-testing")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum StressTypeArg {
    Syn,
    Udp,
    Http,
    Tcp,
    Icmp,
}

#[cfg(feature = "stress-testing")]
impl std::fmt::Display for StressTypeArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StressTypeArg::Syn => write!(f, "syn"),
            StressTypeArg::Udp => write!(f, "udp"),
            StressTypeArg::Http => write!(f, "http"),
            StressTypeArg::Tcp => write!(f, "tcp"),
            StressTypeArg::Icmp => write!(f, "icmp"),
        }
    }
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyArgs {
    #[command(subcommand)]
    pub command: ProxyCommand,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Subcommand)]
pub enum ProxyCommand {
    #[command(about = "Add proxies from file")]
    Add(ProxyAddArgs),
    #[command(about = "List available proxies")]
    List(ProxyListArgs),
    #[command(about = "Check health of all proxies")]
    HealthCheck(ProxyHealthArgs),
    #[command(about = "Test a single proxy")]
    Test(ProxyTestArgs),
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyAddArgs {
    #[arg(
        help = "Path to proxy file (one proxy per line, format: type://host:port or type://user:pass@host:port)"
    )]
    pub file: String,
    #[arg(long, help = "Proxy type (if not specified in file)")]
    pub proxy_type: Option<ProxyTypeArg>,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyListArgs {
    #[arg(long, help = "Show only healthy proxies")]
    pub healthy: bool,
    #[arg(long, help = "Show proxy details")]
    pub verbose: bool,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyHealthArgs {
    #[arg(
        long,
        default_value = "https://google.com",
        help = "URL to check proxy health"
    )]
    pub test_url: String,
    #[arg(long, default_value = "10", help = "Timeout in seconds")]
    pub timeout: u64,
}

#[cfg(feature = "stress-testing")]
#[derive(clap::Args)]
pub struct ProxyTestArgs {
    #[arg(help = "Proxy to test (format: type://host:port)")]
    pub proxy: String,
    #[arg(long, default_value = "https://google.com", help = "URL to test proxy")]
    pub test_url: String,
}

#[cfg(feature = "stress-testing")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProxyTypeArg {
    Socks4,
    Socks5,
    Http,
    Https,
    Tor,
}

#[derive(clap::Args)]
pub struct ClusterArgs {
    #[command(subcommand)]
    pub command: ClusterCommand,
}

#[derive(clap::Subcommand)]
pub enum ClusterCommand {
    #[command(about = "Start a worker node")]
    Worker(ClusterWorkerArgs),
    #[command(about = "Start a coordinator node")]
    Coordinator(ClusterCoordinatorArgs),
    #[command(about = "Show cluster status")]
    Status(ClusterStatusArgs),
}

#[derive(clap::Args)]
pub struct ClusterWorkerArgs {
    #[arg(long, default_value = "localhost:9000", help = "Coordinator address")]
    pub coordinator: String,
    #[arg(long, default_value = "4", help = "Number of worker threads")]
    pub workers: usize,
    #[arg(long, help = "Worker ID (auto-generated if not set)")]
    pub worker_id: Option<String>,
    #[arg(long, help = "Pre-shared key for authentication")]
    pub psk: Option<String>,
}

#[derive(clap::Args)]
pub struct ClusterCoordinatorArgs {
    #[arg(long, default_value = "9000", help = "Port to listen on")]
    pub port: u16,
    #[arg(long, help = "Bind address (default: 0.0.0.0)")]
    pub bind: Option<String>,
    #[arg(long, help = "Maximum workers")]
    pub max_workers: Option<usize>,
    #[arg(long, help = "Pre-shared key for worker authentication")]
    pub psk: Option<String>,
}

#[derive(clap::Args)]
pub struct ClusterStatusArgs {
    #[arg(long, help = "Coordinator address (for remote status)")]
    pub coordinator: Option<String>,
}

#[derive(clap::Args)]
pub struct NotifyArgs {
    #[command(subcommand)]
    pub command: NotifyCommand,
}

#[derive(clap::Subcommand)]
pub enum NotifyCommand {
    #[command(about = "Send a test notification")]
    Test(NotifyTestArgs),
    #[command(about = "Send a notification")]
    Send(NotifySendArgs),
}

#[derive(clap::Args)]
pub struct NotifyTestArgs {
    #[arg(long, help = "Test Slack webhook")]
    pub slack: Option<String>,
    #[arg(long, help = "Test Discord webhook")]
    pub discord: Option<String>,
    #[arg(long, help = "Test Teams webhook")]
    pub teams: Option<String>,
    #[arg(long, help = "Test custom webhook")]
    pub webhook: Option<String>,
    #[arg(long, help = "Webhook secret for custom webhook")]
    pub secret: Option<String>,
}

#[derive(clap::Args)]
pub struct NotifySendArgs {
    #[arg(help = "Message to send")]
    pub message: String,
    #[arg(long, help = "Send to Slack")]
    pub slack: Option<String>,
    #[arg(long, help = "Send to Discord")]
    pub discord: Option<String>,
    #[arg(long, help = "Send to Teams")]
    pub teams: Option<String>,
    #[arg(long, help = "Send to custom webhook")]
    pub webhook: Option<String>,
    #[arg(long, help = "Finding severity (critical/high/medium/low)")]
    pub severity: Option<String>,
    #[arg(long, help = "Target that was scanned")]
    pub target: Option<String>,
}

#[derive(clap::Args)]
pub struct RemoteArgs {
    #[command(subcommand)]
    pub command: RemoteCommand,
}

#[derive(clap::Subcommand)]
pub enum RemoteCommand {
    #[command(about = "Generate a new PSK")]
    GenerateKey,
    #[command(about = "Generate TLS certificate for distributed communication")]
    Cert(CertArgs),
    #[command(about = "Start remote listener")]
    Start(RemoteStartArgs),
    #[command(about = "Stop remote listener")]
    Stop,
}

#[derive(clap::Args)]
pub struct CertArgs {
    #[arg(long, help = "Print instructions for creating TLS cert with openssl")]
    pub openssl: bool,
}

#[derive(clap::Args)]
pub struct RemoteStartArgs {
    #[arg(long, default_value = "7890", help = "Port to listen on")]
    pub port: u16,
    #[arg(long, help = "Pre-shared key for authentication")]
    pub auth: Option<String>,
    #[arg(long, help = "TLS certificate file (PKCS12 format)")]
    pub tls_cert: Option<String>,
    #[arg(long, help = "Password for TLS certificate")]
    pub tls_password: Option<String>,
}

#[cfg(feature = "rest-api")]
#[derive(clap::Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "8080", help = "Port to listen on")]
    pub port: u16,
    #[arg(long, default_value = "127.0.0.1", help = "Address to bind to")]
    pub bind: String,
    #[arg(long, help = "API key for authentication")]
    pub api_key: Option<String>,
}

#[cfg(feature = "mcp-server")]
#[derive(clap::Args)]
pub struct McpServeArgs {
    #[arg(long, default_value = "8081", help = "Port to listen on")]
    pub port: u16,
    #[arg(long, default_value = "127.0.0.1", help = "Address to bind to")]
    pub bind: String,
    #[arg(long, help = "API key for authentication")]
    pub api_key: Option<String>,
    #[arg(long, help = "Enable stdio mode for AI assistant integration")]
    pub stdio: bool,
}

#[derive(clap::Args)]
pub struct ExecArgs {
    #[arg(long, help = "Single target (host:port)")]
    pub target: Option<String>,
    #[arg(long, help = "File containing list of targets (one per line)")]
    pub targets: Option<String>,
    #[arg(long, help = "Pre-shared key for authentication")]
    pub auth: Option<String>,
    #[arg(long, default_value = "60", help = "Timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "TLS certificate file (PKCS12 format)")]
    pub tls_cert: Option<String>,
    #[arg(
        long,
        default_value = "localhost",
        help = "TLS domain for certificate verification"
    )]
    pub tls_domain: Option<String>,
    #[arg(long, help = "Password for TLS certificate")]
    pub tls_password: Option<String>,
    #[arg(help = "The command to execute")]
    pub command: Vec<String>,
}
