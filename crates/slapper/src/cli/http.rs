use super::CommonHttpArgs;

pub(crate) const LOAD_ABOUT: &str = "Run HTTP load test against target URL

Sends concurrent HTTP requests to measure server performance and gather metrics.
Useful for identifying bottlenecks and testing server resilience.

Examples:
  slapper load https://example.com -n 1000 -c 50
  slapper load https://api.example.com/endpoint -n 500 -c 20 -m POST -d '{\"query\": \"test\"}'
  slapper load https://example.com -n 100 --json
  slapper load https://example.com -n 200 -c 10 --proxy http://127.0.0.1:8080";

pub(crate) const RECON_ABOUT: &str = "Gather reconnaissance information

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

pub(crate) const GRAPHQL_ABOUT: &str = "Test GraphQL endpoints for security issues

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

pub(crate) const OAUTH_ABOUT: &str = "Test OAuth/OIDC endpoints for vulnerabilities

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
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
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
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
}
