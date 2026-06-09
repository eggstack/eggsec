use super::timeout::*;
use super::{CommonHttpArgs, FuzzMode, OutputFormat};

pub(crate) const FUZZ_ABOUT: &str = "Fuzz target to validate input handling with security payloads

Validates application resilience using various payload types.
Supports SQL injection, XSS, path traversal, SSRF, open redirects, ReDoS, and more.

Examples:
  eggsec fuzz https://example.com/api?id=1 -t sqli
  eggsec fuzz https://example.com/search?q=test -t xss
  eggsec fuzz https://example.com -t all
  eggsec fuzz https://example.com -t sqli,xss,graphql -c 20
  eggsec fuzz https://example.com -t ssrf --param url
  eggsec fuzz https://example.com -t xss --mutate -m 5
  eggsec fuzz https://example.com -t xss --target nginx
  eggsec fuzz https://example.com/graphql -t graphql  # GraphQL testing
  eggsec fuzz https://api.example.com -t jwt  # JWT testing
  eggsec fuzz https://oauth.example.com -t oauth  # OAuth/OIDC testing";

pub(crate) const WAF_STRESS_ABOUT: &str = "MODE: Defense Lab | REQUIRED: --scope (private/localhost), explicit scope

WAF resilience validation

Applies all payload types to evaluate WAF detection and resilience.
For WAF regression and resilience validation on authorized targets.
Useful for WAF evaluation and tuning.

Examples:
  eggsec waf-stress http://127.0.0.1:8080
  eggsec waf-stress http://127.0.0.1:8080 -c 50
  eggsec waf-stress http://127.0.0.1:8080 --json";

pub(crate) const WAF_ABOUT: &str = "Evaluate WAF detection and evasion resistance

Detects WAF presence and evaluates evasion resistance using various techniques.
Supports 30+ WAF products with header manipulation, HTTP smuggling, and evasion resistance analysis.

Examples:
  eggsec waf https://example.com
  eggsec waf https://example.com --detect-only
  eggsec waf https://example.com --bypass
  eggsec waf https://example.com --header-bypass --smuggling
  eggsec waf https://example.com --bypass -c 20
  eggsec waf https://example.com --profile cloudflare  # WAF-specific bypass
  eggsec waf https://example.com --evasion  # Advanced evasion techniques";

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
    #[arg(long, help = "Enable request chaining for automated validation")]
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
    #[arg(long, default_value_t = FUZZ_TIMEOUT, help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, short = 'o', help = "Output file path")]
    pub output: Option<String>,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
    #[arg(long, help = "Output format: json, html, csv, markdown")]
    pub format: Option<OutputFormat>,
    #[arg(
        long,
        help = "Target type for specific payloads: nginx, apache, php, generic (adds target-specific validation payloads)"
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
    #[arg(long, help = "OpenAPI/Swagger schema URL for schema-aware fuzzing")]
    pub schema: Option<String>,
    #[arg(long, help = "Only discover API schema, do not fuzz")]
    pub discover_only: bool,
    #[arg(long, help = "Auto-discover API schema from common paths")]
    pub auto_discover_schema: bool,
    #[arg(
        long,
        help = "Enable auto-calibration before fuzzing (samples baseline responses)"
    )]
    pub calibrate: bool,
    #[arg(
        short = 'C',
        long,
        value_name = "CODES",
        help = "Filter out responses by status code (comma-separated, e.g., 404,500)"
    )]
    pub fc: Option<String>,
    #[arg(
        short = 'S',
        long,
        value_name = "SIZES",
        help = "Filter out responses by size (comma-separated, e.g., 123,456)"
    )]
    pub fs: Option<String>,
    #[arg(
        short = 'W',
        long,
        value_name = "WORDS",
        help = "Filter out responses by word count (comma-separated)"
    )]
    pub fw: Option<String>,
    #[arg(
        short = 'L',
        long,
        value_name = "LINES",
        help = "Filter out responses by line count (comma-separated)"
    )]
    pub fl: Option<String>,
    #[arg(
        short = 'T',
        long,
        value_name = "MS",
        help = "Filter out responses by time in ms (e.g., 5000 for >5s)"
    )]
    pub ft: Option<u64>,
    #[arg(
        short = 'R',
        long,
        value_name = "REGEX",
        help = "Filter out responses matching regex pattern"
    )]
    pub fr: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
}

#[derive(clap::Args, Clone)]
pub struct WafStressArgs {
    #[arg(help = "Target URL")]
    pub url: String,
    #[arg(short = 'c', long, default_value = "20", help = "Concurrent requests")]
    pub concurrency: usize,
    #[arg(long, default_value_t = FUZZ_TIMEOUT, help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
}

impl From<WafStressArgs> for FuzzArgs {
    fn from(args: WafStressArgs) -> Self {
        FuzzArgs {
            url: args.url,
            payload_type: "all".to_string(),
            mode: FuzzMode::Sequential,
            mutate: false,
            mutation_count: 0,
            grammar_fuzz: false,
            grammar_type: None,
            adaptive_rate: false,
            session: false,
            diffing: false,
            capture_baseline: false,
            enhanced_redos: false,
            waf_fingerprint: false,
            chaining: false,
            chain_file: None,
            method: "GET".to_string(),
            param: None,
            concurrency: args.concurrency,
            timeout: args.timeout,
            json: args.json,
            output: args.output,
            verbose: args.verbose,
            quiet: args.quiet,
            format: None,
            target: None,
            jwt_token: None,
            oauth_issuer: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            graphql_introspection: false,
            graphql_depth_bypass: false,
            graphql_alias_overload: false,
            oauth_redirect: false,
            oauth_scope: false,
            oauth_state: false,
            oauth_grant: false,
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
            calibrate: false,
            fc: None,
            fs: None,
            fw: None,
            fl: None,
            ft: None,
            fr: None,
            common: args.common,
        }
    }
}

#[derive(clap::Args, Clone)]
pub struct WafArgs {
    #[arg(help = "Target URL")]
    pub url: String,
    #[arg(short = 'd', long, help = "Detect WAF only (no bypass attempts)")]
    pub detect_only: bool,
    #[arg(short = 'b', long, help = "Test WAF detection and evasion resistance")]
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
    #[arg(long, default_value_t = WAF_TIMEOUT, help = "Request timeout in seconds")]
    pub timeout: u64,
    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
    #[arg(long, help = "Verbose output")]
    pub verbose: bool,
    #[arg(long, short = 'q', help = "Suppress non-essential output")]
    pub quiet: bool,
    #[arg(long, short = 'o', help = "Output to file")]
    pub output: Option<String>,
    #[command(flatten)]
    pub common: CommonHttpArgs,
}
