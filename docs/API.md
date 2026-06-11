# Eggsec API Documentation

This document provides detailed API documentation for using Eggsec as a Rust library.

## Table of Contents

- [Configuration](#configuration)
- [Load Testing](#load-testing)
- [Port Scanning](#port-scanning)
- [Endpoint Discovery](#endpoint-discovery)
- [Service Fingerprinting](#service-fingerprinting)
- [Fuzzing](#fuzzing)
- [WAF Detection](#waf-detection)
- [Reconnaissance](#reconnaissance)
- [Pipeline](#pipeline)
- [Output](#output)
- [Error Handling](#error-handling)

## Configuration

### Loading Configuration

```rust
use eggsec::{load_config, load_scope, EggsecConfig, Scope};

let config = load_config(Some("path/to/config.toml"))?;
let scope = load_scope(Some("path/to/scope.toml"))?;
```

### Configuration Structure

```rust
pub struct EggsecConfig {
    pub http: HttpConfig,
    pub scan: ScanConfig,
    pub output: OutputConfig,
    pub notifications: NotificationsConfig,
    pub profiles: HashMap<String, ScanProfile>,
}

pub struct HttpConfig {
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub verify_tls: bool,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub default_headers: HashMap<String, String>,
    pub default_user_agent: Option<String>,
    pub proxy: Option<String>,
    pub proxy_auth: Option<String>,
}

### Geolocation Configuration

Eggsec supports multiple geolocation providers with automatic fallback:

```rust
pub struct ApiConfig {
    pub ipapi: IpApiConfig,
    pub maxmind: MaxMindConfig,
}

pub struct IpApiConfig {
    pub enabled: bool,
    pub use_premium: bool,
    pub api_key: Option<String>,  // Get from https://ipapi.co/
}

pub struct MaxMindConfig {
    pub enabled: bool,
    pub account_id: Option<u32>,      // Get from https://www.maxmind.com/
    pub license_key: Option<String>,   // Get from https://www.maxmind.com/
    pub edition_ids: Vec<String>,      // e.g., ["GeoLite2-City", "GeoLite2-Country"]
    pub auto_update: bool,             // Auto-download database on startup
    pub data_dir: String,              // Where to store the .mmdb file
    pub use_geoipupdate_binary: bool,  // Use geoipupdate binary instead of direct download
}
```

#### Fallback Order (when online):

1. **MaxMind DB** (local, if configured) - Offline-first, unlimited lookups
2. **geoip.vuiz.net** - 100 requests/min, commercial OK, full data
3. **ipapi.co** - 1000/day (free) or unlimited with API key
4. **ip-api.com** - 45 requests/min, non-commercial only
5. **ipwhois.io** - 1 request/sec, full data
6. **ip2c.org** - Unlimited, country only (last resort)

#### Configuration Example:

```toml
[recon.apis.ipapi]
enabled = true
use_premium = true
api_key = "your-ipapi-co-api-key"  # Get free key at https://ipapi.co/

[recon.apis.maxmind]
enabled = true
account_id = 12345
license_key = "your-maxmind-license-key"
edition_ids = ["GeoLite2-City", "GeoLite2-Country"]
auto_update = true
data_dir = "~/.eggsec/geoip"
use_geoipupdate_binary = false  # Set true to use external geoipupdate tool
```

To get a MaxMind license key:
1. Create free account at https://www.maxmind.com/
2. Go to "License Keys" and create a new key
3. Download GeoLite2 databases (free) or GeoIP2 (paid)

pub struct ScanConfig {
    pub default_concurrency: usize,
    pub rate_limit_per_second: Option<u32>,
    pub jitter_ms: Option<(u64, u64)>,
    pub stealth_mode: bool,
    pub exclude_ports: Vec<u16>,
    pub exclude_hosts: Vec<String>,
    pub port_timeout_secs: u64,
    pub save_session: bool,
}

pub struct Scope {
    pub require_explicit_scope: bool,
    pub allowed_targets: Vec<TargetPattern>,
    pub excluded_targets: Vec<TargetPattern>,
    pub excluded_ports: Vec<u16>,
}
```

## Load Testing

### Running Load Tests

```rust
use eggsec::loadtest::{self, LoadTestArgs, CommonHttpArgs};

let args = LoadTestArgs {
    url: "https://example.com".to_string(),
    requests: 1000,
    concurrency: 50,
    method: "GET".to_string(),
    body: None,
    headers: vec![],
    timeout: 30,
    json: false,
    common: CommonHttpArgs {
        insecure: false,
        proxy: None,
        proxy_auth: None,
        auth: None,
        bearer: None,
        cookie: None,
        api_key: None,
        user_agent: None,
        stealth: false,
        rate_limit: None,
        jitter: None,
    },
};

let results = loadtest::run_cli(args, &config).await?;
```

### Load Test Results

```rust
pub struct LoadTestResults {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub requests_per_second: f64,
    pub latency: LatencyMetrics,
    pub status_codes: HashMap<u16, u64>,
}

pub struct LatencyMetrics {
    pub min: u64,
    pub max: u64,
    pub mean: u64,
    pub median: u64,
    pub p95: u64,
    pub p99: u64,
}
```

## Port Scanning

### Scanning Ports

```rust
use eggsec::scanner::ports::{self, PortScanArgs};

let args = PortScanArgs {
    host: "example.com".to_string(),
    ports: "1-1024".to_string(),
    concurrency: 100,
    timeout: 2,
    json: false,
};

let results = scanner::ports::run_cli(args, &config).await?;
```

### Port Scan Results

```rust
pub struct PortScanResult {
    pub host: String,
    pub open_ports: Vec<PortInfo>,
    pub scan_duration: Duration,
}

pub struct PortInfo {
    pub port: u16,
    pub protocol: String,
    pub state: PortState,
    pub service: Option<String>,
    pub version: Option<String>,
}
```

## Endpoint Discovery

### Discovering Endpoints

```rust
use eggsec::scanner::endpoints::{self, EndpointScanArgs};

let args = EndpointScanArgs {
    url: "https://example.com".to_string(),
    wordlist: Some("/path/to/wordlist.txt".to_string()),
    concurrency: 20,
    timeout: 10,
    json: false,
    include_404: false,
    common: /* CommonHttpArgs */,
};

let results = scanner::endpoints::run_cli(args, &config).await?;
```

### Parsing a Custom Wordlist

```rust
use eggsec::scanner::wordlist::Wordlist;

// Parse from a file (async)
let wordlist = Wordlist::from_file("/path/to/endpoints.txt").await?;
let endpoints: Vec<String> = wordlist.into_endpoints();

// Parse from a string
let wordlist = Wordlist::parse("/admin\n/api/v1\n/login\n")?;
assert_eq!(wordlist.len(), 3);
```

The `Wordlist` parser:
- Skips empty lines and `#` comments
- Normalizes paths to start with `/`
- Rejects paths with whitespace, control chars, or length > 2048
- Returns an error if the wordlist contains no valid endpoints

## Service Fingerprinting

### Fingerprinting Services

```rust
use eggsec::scanner::fingerprint::{self, FingerprintArgs};

let args = FingerprintArgs {
    host: "example.com".to_string(),
    ports: "80,443,22,21,25,3306,5432".to_string(),
    timeout: 5,
    json: false,
};

let results = scanner::fingerprint::run_cli(args, &config).await?;
```

### Fingerprint Results

```rust
pub struct FingerprintResult {
    pub host: String,
    pub services: Vec<ServiceInfo>,
}

pub struct ServiceInfo {
    pub port: u16,
    pub protocol: String,
    pub service_name: String,
    pub version: Option<String>,
    pub banner: Option<String>,
    pub fingerprints: Vec<String>,
}
```

## Fuzzing

### Running Fuzz Tests

```rust
use eggsec::fuzzer::{self, FuzzArgs, FuzzMode};

let args = FuzzArgs {
    url: "https://example.com/api".to_string(),
    payload_type: "sqli,xss".to_string(),
    mode: FuzzMode::Sequential,
    mutate: false,
    mutation_count: 3,
    method: "GET".to_string(),
    param: Some("id".to_string()),
    concurrency: 10,
    timeout: 10,
    json: false,
    target: Some("generic".to_string()),
    common: /* CommonHttpArgs */,
};

let results = fuzzer::run_cli(args, &config).await?;
```

### Payload Types

| Type | Description |
|------|-------------|
| `sqli` | SQL Injection |
| `xss` | Cross-Site Scripting |
| `traversal` | Path Traversal |
| `ssrf` | Server-Side Request Forgery |
| `redirect` | Open Redirect |
| `redos` | Regular Expression DoS |
| `headers` | HTTP Header Injection |
| `compression` | Compression Bomb |
| `all` | All payload types |

### Fuzz Results

```rust
pub struct FuzzResult {
    pub target: String,
    pub findings: Vec<FuzzFinding>,
    pub payloads_tested: u64,
    pub duration: Duration,
}

pub struct FuzzFinding {
    pub payload_type: String,
    pub payload: String,
    pub location: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub evidence: String,
}
```

## WAF Detection

### Detecting WAFs

```rust
use eggsec::waf::{self, WafArgs};

let args = WafArgs {
    url: "https://example.com".to_string(),
    detect_only: true,
    bypass: false,
    header_bypass: false,
    smuggling: false,
    evasion: false,
    concurrency: 10,
    timeout: 15,
    json: false,
    common: /* CommonHttpArgs */,
};

let results = waf::run_cli(args, &config).await?;
```

### WAF Bypass

```rust
let args = WafArgs {
    url: "https://example.com".to_string(),
    detect_only: false,
    bypass: true,          // Enable all bypass techniques
    header_bypass: true,  // Header manipulation
    smuggling: true,       // HTTP smuggling
    evasion: true,        // ML evasion techniques
    // ... other fields
};

let results = waf::run_cli(args, &config).await?;
```

### WAF Results

```rust
pub struct WafResult {
    pub detected: bool,
    pub waf_name: Option<String>,
    pub confidence: f32,
    pub bypass_successful: Vec<BypassTechnique>,
    pub findings: Vec<WafFinding>,
}

pub enum BypassTechnique {
    HeaderManipulation,
    HttpSmuggling,
    Encoding,
    Evasion,
}
```

## Reconnaissance

### Running Reconnaissance

```rust
use eggsec::recon::{self, ReconArgs};

let args = ReconArgs {
    target: "example.com".to_string(),
    no_tech: false,
    no_dns: false,
    no_geo: false,
    no_whois: false,
    no_subdomains: false,
    no_ssl: false,
    no_dns_records: false,
    no_js: false,
    no_content: false,
    no_cloud: false,
    no_wayback: false,
    no_cors: false,
    no_threat: false,
    no_cve: false,
    no_email: false,
    concurrency: Some(20),
    json: false,
};

let results = recon::run_cli(args, &config).await?;
```

### Recon Results

```rust
pub struct ReconResult {
    pub target: String,
    pub tech_stack: Vec<TechStackEntry>,
    pub dns_records: DnsRecords,
    pub geolocation: Option<GeoLocation>,
    pub whois: Option<WhoisInfo>,
    pub subdomains: Vec<String>,
    pub ssl_info: Option<SslInfo>,
    pub javascript: Vec<JsFile>,
    pub content: Vec<ContentFinding>,
    pub cloud: CloudAssets,
    pub wayback: Vec<WaybackUrl>,
    pub cors: CorsAnalysis,
    pub threat_intel: Vec<ThreatIntel>,
    pub cve_mappings: Vec<CveMapping>,
    pub emails: Vec<String>,
}
```

## Pipeline

### Scan Profiles

Eggsec provides 16 pre-configured scan profiles:

```rust
pub enum ScanProfile {
    Quick,     // Port scan + fingerprint
    Endpoint,  // Quick + endpoint discovery
    Web,       // Endpoint + web fuzzing
    Waf,       // Endpoint + WAF detection and bypass
    Full,      // All stages including load testing
    Api,       // GraphQL/JWT/OAuth focused
    Recon,     // Intelligence-led with tech detection + CVE mapping
    Stealth,   // Web scan with evasion techniques
    Deep,      // Web scan with mutation fuzzing
    Vuln,      // CVE-prioritized based on detected tech
    Auth,      // JWT/OAuth/IDOR focused
}
```

### Running a Scan Pipeline

```rust
use eggsec::pipeline::{self, ScanArgs, ScanProfile};

let args = ScanArgs {
    target: "example.com".to_string(),
    profile: ScanProfile::Full,
    stages: None,  // Automatically determined by profile
    concurrency: 10,
    json: false,
    output: Some("results.html".to_string()),
    format: Some(OutputFormat::Html),
    web_types: None,
    common: /* CommonHttpArgs */,
};

let results = pipeline::run_cli(args, &config).await?;
```

### Custom Stages

You can also specify custom stages:

```rust
let args = ScanArgs {
    target: "example.com".to_string(),
    profile: ScanProfile::Quick,  // Ignored when stages specified
    stages: Some("port,fingerprint,endpoint,waf,recon".to_string()),
    concurrency: 10,
    json: false,
    output: None,
    format: None,
    web_types: None,
    common: /* CommonHttpArgs */,
};
```

### Pipeline Results

```rust
pub struct PipelineResult {
    pub target: String,
    pub profile: String,
    pub stages: Vec<StageResult>,
    pub duration: Duration,
    pub findings: Vec<Finding>,
}

pub struct StageResult {
    pub name: String,
    pub success: bool,
    pub findings: Vec<Finding>,
    pub duration: Duration,
}
```

## Output

### Output Formats

```rust
use eggsec::pipeline::report::{generate_html, generate_csv};
use serde_json;

// Generate HTML report
let html = generate_html(&report)?;

// Generate CSV export
let csv = generate_csv(&report)?;

// Generate JSON output
let json = serde_json::to_string_pretty(&report)?;
```

### SARIF Output (Internal)

```rust
use eggsec::output::sarif::SarifBuilder;

let sarif = SarifBuilder::new()
    .with_report(&report)
    .build();
```

### JUnit XML Output (Internal)

```rust
use eggsec::output::junit::JUnitBuilder;

let junit = JUnitBuilder::new("eggsec")
    .with_report(&report)
    .build();
let xml = junit.to_xml()?;
```

## Error Handling

Eggsec uses `anyhow` for application errors and `thiserror` for library errors:

```rust
use eggsec::{EggsecError, Result};

pub enum EggsecError {
    Config(String),
    Network(String),
    Scan(String),
    Output(String),
}

impl std::error::Error for EggsecError {}

impl std::fmt::Display for EggsecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Network(msg) => write!(f, "Network error: {}", msg),
            Self::Scan(msg) => write!(f, "Scan error: {}", msg),
            Self::Output(msg) => write!(f, "Output error: {}", msg),
        }
    }
}
```

### Handling Errors

```rust
use anyhow::Result;

async fn run_scan() -> Result<()> {
    let config = load_config(None)?;
    let scope = load_scope(None)?;
    
    // Check scope before scanning
    if !scope.is_target_allowed(&target)? {
        anyhow::bail!("Target not in allowed scope");
    }
    
    // Run scan
    let results = scanner::ports::run_cli(args, &config).await?;
    
    Ok(())
}
```

## Creating Custom Scans

You can implement custom scanning logic using the existing modules:

```rust
use eggsec::fuzzer::engine::FuzzEngine;

let mut engine = FuzzEngine::new(config.clone());

for payload in engine.payloads() {
    let response = client.request(payload).await?;
    if engine.analyze(&response) {
        // Found vulnerability
    }
}
```
