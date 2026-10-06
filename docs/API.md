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
    pub notifications: NotificationConfig,
    pub profiles: FxHashMap<String, ScanProfile>,
    pub paths: PathsConfig,               // serde(flatten)
    pub recon: ReconConfig,
    pub schedule: Vec<ScheduledScan>,
    pub remote: RemoteConfig,
    pub proxies: Vec<ProxyConfigEntry>,
    pub ai: Option<AiConfig>,
    pub search: Option<SearchConfig>,
    pub alert_channels: AlertChannelsConfig,
    pub execution_policy: ExecutionPolicy,
    // + auto-save interval; `integrations` behind the `external-integrations` feature
}

pub struct HttpConfig {
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub verify_tls: bool,
    pub follow_redirects: bool,
    pub max_redirects: usize,
    pub default_headers: FxHashMap<String, String>,
    pub default_user_agent: Option<String>,
    pub proxy: Option<String>,
    pub proxy_auth: Option<SensitiveString>,
}
```

Full definitions: `crates/eggsec/src/config/settings.rs` and `crates/eggsec/src/config/http.rs`.

### Geolocation Configuration

Eggsec supports multiple geolocation providers with automatic fallback:

```rust
pub struct ApiConfig {
    pub virustotal: ApiKeyConfig,
    pub alienvault: ApiKeyConfig,
    pub shodan: ApiKeyConfig,
    pub ipapi: IpApiConfig,
    pub maxmind: MaxMindConfig,
    pub wayback_machine: WaybackConfig,
    pub nvd: NvdConfig,
}

pub struct ApiKeyConfig {
    pub enabled: bool,
    pub api_key: Option<SensitiveString>,
}

pub struct IpApiConfig {
    pub enabled: bool,
    pub api_key: Option<SensitiveString>,   // Get from https://ipapi.co/
}

pub struct MaxMindConfig {
    pub enabled: bool,
    pub account_id: Option<u32>,             // Get from https://www.maxmind.com/
    pub license_key: Option<SensitiveString>, // Get from https://www.maxmind.com/
    pub edition_ids: Vec<String>,            // e.g., ["GeoLite2-City", "GeoLite2-Country"]
    pub auto_update: bool,                   // Auto-download database on startup
    pub data_dir: PathBuf,                   // Where to store the .mmdb file
}
```

### Fallback Order

`GeoIpResolver::lookup` (`crates/eggsec/src/recon/geolocation.rs:245`) checks a
built-in private-CIDR table first, then -- only when online lookups are enabled --
walks this chain, returning the first hit:

0. **Local CIDR table** - private/reserved ranges, no network I/O
1. **MaxMind DB** (local `.mmdb`, if configured) - offline-first, unlimited lookups
2. **geoip.vuiz.net** - commercial OK, full data
3. **ipapi.co** - 1000/day (free) or unlimited with API key
4. **ip-api.com** - 45 requests/min, non-commercial only
5. **ipwho.is** - 1 request/sec, full data
6. **ip2c.org** - unlimited, country only (last resort)

If every step fails the resolver returns an "Unknown" / `XX` `GeoLocation` rather
than an error.

### Configuration Example:

```toml
[recon.apis.ipapi]
enabled = true
api_key = "your-ipapi-co-api-key"  # Get free key at https://ipapi.co/

[recon.apis.maxmind]
enabled = true
account_id = 12345
license_key = "your-maxmind-license-key"
edition_ids = ["GeoLite2-City", "GeoLite2-Country"]
auto_update = true
data_dir = "~/.eggsec/geoip"
```

To get a MaxMind license key:
1. Create free account at https://www.maxmind.com/
2. Go to "License Keys" and create a new key
3. Download GeoLite2 databases (free) or GeoIP2 (paid)

```rust
pub struct ScanConfig {
    pub default_concurrency: usize,
    pub rate_limit_per_second: Option<u32>,
    pub jitter_ms: Option<(u64, u64)>,
    pub stealth_mode: bool,
    pub exclude_ports: Vec<u16>,
    pub exclude_hosts: Vec<String>,
    pub port_timeout_secs: u64,
    pub save_session: bool,
    pub session_dir: Option<PathBuf>,
}
```

`Scope` is re-exported from `eggsec-policy` (`crates/eggsec-policy/src/scope.rs:88`):

```rust
pub struct Scope {
    pub require_explicit_scope: bool,
    pub allowed_targets: Vec<ScopeRule>,
    pub excluded_targets: Vec<ScopeRule>,
    pub allowed_ports: Option<Vec<u16>>,
    pub excluded_ports: Vec<u16>,
    pub max_requests_per_second: Option<u32>,
    pub scope_file: Option<String>,
}
```

`load_scope` returns a `Scope`; `load_scope_with_source` returns a `LoadedScope`
carrying the `ScopeSource`, which is what strict surfaces require for
authorization.

## Load Testing

### Running Load Tests

```rust
use eggsec::cli::{CommonHttpArgsCli, LoadArgs};
use eggsec::config::EggsecConfig;

let args = LoadArgs {
    url: "https://example.com".to_string(),
    requests: 1000,
    concurrency: 50,
    method: "GET".to_string(),
    // ... remaining fields
    common: CommonHttpArgsCli::default(),
};

eggsec::loadtest::run_cli(args, &config).await?;
```

`LoadArgs` lives in `eggsec::cli` (`crates/eggsec/src/cli/http.rs:64`); the Clap
argument struct is flattened into a runtime `CommonHttpArgs` before execution.
The args type is `cli::CommonHttpArgsCli`, while the runtime type
`eggsec::types::CommonHttpArgs` carries the extra `auth_context` / `auth_role`
fields.

Callers that have already enforced operation policy should pass the authorized
scope explicitly so per-request authorization matches the pre-dispatch verdict:

```rust
eggsec::loadtest::run_cli_with_scope(args, &config, scope).await?;
```

### Load Test Results

`eggsec::loadtest::LoadTestResults` (`crates/eggsec/src/loadtest/metrics.rs:118`)
reports latency as flat `f64` percentile fields rather than a nested struct:

```rust
pub struct LoadTestResults {
    pub target_url: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_duration_ms: u64,
    pub requests_per_second: f64,
    pub latency_min_ms: f64,
    pub latency_max_ms: f64,
    pub latency_mean_ms: f64,
    pub latency_p50_ms: f64,
    pub latency_p90_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    pub status_codes: FxHashMap<u16, u64>,
    pub errors: Vec<String>,
    pub error_kinds: FxHashMap<String, u64>,
}
```

## Port Scanning

### Scanning Ports

```rust
use eggsec::cli::PortScanArgs;
use eggsec::scanner::ports;

let args = PortScanArgs {
    host: "example.com".to_string(),
    ports: "1-1024".to_string(),
    concurrency: 100,
    timeout: 2,
    json: false,
    // ... spoof/decoy/dry-run fields
};

ports::run_cli(args, &config).await?;
```

`PortScanArgs` is a Clap args struct defined in `eggsec::cli`
(`crates/eggsec/src/cli/scan.rs:82`); `scanner::ports` imports it privately.
`run_cli` returns `Result<()>` and renders output itself -- to obtain structured
results in-process, call `scanner::ports::scan_ports` or
`scanner::ports::run_cli_with_callback` instead.

### Port Scan Results

```rust
pub struct PortScanResults {
    pub host: String,
    pub ports_scanned: u32,
    pub open_ports: Vec<PortResult>,
    pub total_open_ports: usize,
    pub results_truncated: bool,
    pub duration_ms: u64,
    pub spoof_stats: Option<SpoofStats>,   // skipped when None
    // + `udp_state` under the `udp-scan` feature
}

pub struct PortResult {
    pub port: u16,
    pub status: PortStatus,
    pub protocol: PortProtocol,
    pub service: String,
}
```

## Endpoint Discovery

### Discovering Endpoints

```rust
use eggsec::cli::{CommonHttpArgsCli, EndpointScanArgs};

let args = EndpointScanArgs {
    url: "https://example.com".to_string(),
    wordlist: Some("/path/to/wordlist.txt".to_string()),
    concurrency: 20,
    timeout: 10,
    include_404: false,
    json: false,
    // ... spoof/decoy/verbose/quiet/output fields
    common: CommonHttpArgsCli::default(),
};

eggsec::scanner::endpoints::run_cli(args, &config).await?;
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
use eggsec::cli::FingerprintArgs;

let args = FingerprintArgs {
    host: "example.com".to_string(),
    ports: "80,443,22,21,25,3306,5432".to_string(),
    timeout: 5,
    json: false,
    // ... udp/verbose/quiet/output/concurrency fields
};

eggsec::scanner::fingerprint::run_cli(args, &config).await?;
```

### Fingerprint Results

```rust
pub struct FingerprintResults {
    pub host: String,
    pub ports_scanned: usize,
    pub services_identified: usize,
    pub total_services_identified: usize,
    pub duration_ms: u64,
    pub results: Vec<ServiceFingerprint>,
}

pub struct ServiceFingerprint {
    pub port: u16,
    pub service: String,
    pub banner: Option<String>,
    pub version: Option<String>,
    pub product: Option<String>,
    pub extra: Option<String>,
    pub confidence: u8,
}
```

Deeper per-service evidence lives in `scanner::fingerprint_types`
(`EnhancedFingerprint`, `FingerprintEvidence`, `EvidenceType`,
`FingerprintConfidence`, `ServiceIdentity`).

## Fuzzing

### Running Fuzz Tests

```rust
use eggsec::cli::{CommonHttpArgsCli, FuzzArgs};

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
    // ... remaining fields
    common: CommonHttpArgsCli::default(),
};

eggsec::fuzzer::run_cli(args.into()).await?;
```

`FuzzArgs` (`crates/eggsec/src/cli/fuzz.rs:50`) is the Clap args struct in
`eggsec::cli`; it converts into the engine-owned `fuzzer::config::FuzzConfig`,
which is what `fuzzer::run_cli` takes (`crates/eggsec/src/fuzzer/mod.rs:144`).
Note that `run_cli` takes **no** `&EggsecConfig`.

### Payload Types

`eggsec-payloads` defines **40** `PayloadType` variants. The `sqli`/`xss`/
`traversal`/`ssrf`/`redirect`/`redos`/`headers`/`compression` set below is the
core web set; the remaining variants cover API, protocol and injection families.
The CLI additionally accepts the selector `all`.

| Group | Types |
|-------|-------|
| Web injection | `sqli`, `xss`, `traversal`, `ssrf`, `redirect`, `redos`, `headers`, `compression`, `ssti`, `xxe`, `ldap`, `nosql`, `xpath`, `expression`, `prototype`, `cmd`, `deser` |
| API / auth | `graphql`, `oauth`, `jwt`, `idor`, `mass_assign`, `saml` |
| Protocol | `grpc`, `soap`, `websocket`, `host`, `cache` |
| Web-specific | `html_inject`, `css_inject`, `ssi`, `dom_clobber`, `xslt`, `viewstate`, `dep_confusion`, `xs_leak`, `latex`, `csv` |
| Out-of-band / concurrency | `oast`, `race` |
| Selector | `all` (CLI only, not a `PayloadType` variant) |

The engine facade re-exports `PayloadType` at `eggsec::fuzzer::payloads`.

### Fuzz Results

`fuzzer::engine::types::FuzzResult` is a **per-payload** observation, not an
aggregate; there is no `FuzzFinding` type.

```rust
pub struct FuzzResult {
    pub payload: Payload,
    pub status_code: u16,
    pub response_time_ms: u64,
    pub response_length: Option<u64>,
    pub response_body: Option<String>,
    pub is_waf_blocked: bool,
    pub is_anomaly: bool,
    pub is_redos_suspected: bool,
    pub leaks_found: Vec<String>,
    pub error: Option<String>,
    pub owasp_category: Option<String>,
    pub detected_severity: Severity,
}
```

`FuzzResult::is_vulnerable()` and `is_true_positive()` are the convenience
predicates; `OwaspSummary::from_results` aggregates a slice into A01-A10 counts.

## WAF Detection

### Detecting WAFs

```rust
use eggsec::cli::WafArgs;

let args = WafArgs {
    url: "https://example.com".to_string(),
    detect_only: true,
    bypass: false,
    header_bypass: false,
    smuggling: false,
    evasion: false,
    // ... profile/test_type/concurrency/timeout/json/verbose/quiet/output/common
};

eggsec::waf::run_cli(args).await?;
```

`WafArgs` is defined in `eggsec::cli` (`crates/eggsec/src/cli/fuzz.rs:341`) and
`waf::run_cli` takes **only** the args -- no `&EggsecConfig`
(`crates/eggsec/src/waf/mod.rs:110`).

### WAF Bypass

```rust
let args = WafArgs {
    url: "https://example.com".to_string(),
    detect_only: false,
    bypass: true,          // Enable all bypass techniques
    header_bypass: true,  // Header manipulation
    smuggling: true,       // HTTP smuggling
    evasion: true,         // ML-based evasion techniques
    profile: "cloudflare".to_string(),  // or auto / akamai / aws-waf / ...
    // ... other fields
};

eggsec::waf::run_cli(args).await?;
```

### WAF Results

There is no `WafResult` type. The result type is `waf::types::ScanResults`
(`crates/eggsec/src/waf/types.rs:184`):

```rust
pub struct ScanResults {
    pub target: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub waf_detection: Option<WafDetectionResult>,
    pub findings: Vec<Finding>,
    pub summary: ScanSummary,
}
```

`waf::bypass::BypassTechnique` enumerates more than the four techniques the old
table listed -- `HeaderManipulation`, `UserAgentRotation`,
`XForwardedForSpoof`, `ContentTypeBypass`, `EncodingBypass`, `Homoglyph`,
`ZeroWidthInjection`, `CaseRotation`, `UnicodeEncoding`,
`CommentObfuscation`, `WhitespaceVariation`, `ChunkedEncoding`, and others
(`crates/eggsec/src/waf/bypass/mod.rs:44`).

## Reconnaissance

### Running Reconnaissance

```rust
use eggsec::cli::ReconArgs;

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
    no_takeover: false,
    concurrency: Some(20),
    json: false,
    // ... quiet/verbose/output fields
};

eggsec::recon::run_cli(args, &config).await?;
```

`ReconArgs` is defined in `eggsec::cli` (`crates/eggsec/src/cli/http.rs:102`).
Note the `--no-takeover` flag, which the old example omitted.

### Recon Results

There is no `ReconResult` type. `recon::run_cli` returns `Result<()>` and renders
output itself; in-process callers use `recon::runner::run_full_recon`, which
returns a `recon::FullReconResult` (`crates/eggsec/src/recon/mod.rs:214`):

```rust
pub struct FullReconResult {
    pub target: String,
    pub domain: Option<String>,
    pub ip_address: Option<String>,
    pub tech_stack: Option<TechStack>,
    pub geolocation: Option<GeoLocation>,
    pub whois: Option<WhoisResult>,
    pub subdomains: Option<SubdomainResult>,
    pub ssl_analysis: Option<SslAnalysis>,
    pub dns_records: Option<DnsRecords>,
    pub js_analysis: Option<JsAnalysis>,
    pub wayback: Option<WaybackResult>,
    pub cloud: Option<CloudDiscovery>,       // `cloud` feature
    // ... plus per-stage `*_error: Option<String>` fields
}
```

## Pipeline

### Scan Profiles

Eggsec provides **18** scan profiles (`eggsec::types::ScanProfile`,
`crates/eggsec/src/types.rs:123`):

```rust
pub enum ScanProfile {
    Quick,          // Port scan + fingerprint
    Endpoint,       // Quick + endpoint discovery
    Web,            // Endpoint + web fuzzing
    Waf,            // Endpoint + WAF detection and bypass
    Full,           // All stages including load testing
    Api,            // GraphQL/JWT/OAuth focused
    Recon,          // Intelligence-led with tech detection + CVE mapping
    Stealth,        // Web scan with evasion techniques
    Deep,           // Web scan with mutation fuzzing
    Vuln,           // CVE-prioritized based on detected tech
    Auth,           // JWT/OAuth/IDOR focused
    DefenseLab,     // Baseline diff and defense validation
    SynvoidLocal,   // Localhost SYN scan testing
    WafRegression,  // WAF detection regression testing
    ProtocolEdge,   // Protocol edge case testing
    NseSafe,        // Safe NSE script execution
    DbRegression,   // Database pentest regression (Stage::DbPentest)
    WebProxy,       // Web proxy interception (Stage::WebProxy)
}
```

### Running a Scan Pipeline

```rust
use eggsec::cli::{CommonHttpArgsCli, ScanArgs};

let args = ScanArgs {
    target: "example.com".to_string(),
    profile: ScanProfile::Full,
    stages: None,  // Automatically determined by profile
    // ... remaining fields
    common: CommonHttpArgsCli::default(),
};

eggsec::pipeline::run_cli(args, &config).await?;
```

`ScanArgs` is defined in `eggsec::cli` (`crates/eggsec/src/cli/scan.rs:319`);
`concurrency` is `Option<usize>`, `concurrent_stages` runs stages in parallel,
and `format` is `Option<OutputFormat>`.

### Custom Stages

You can also specify custom stages:

```rust
let args = ScanArgs {
    target: "example.com".to_string(),
    profile: ScanProfile::Quick,  // Ignored when stages specified
    stages: Some("port,fingerprint,endpoint,fuzz,load,waf,recon,graphql,oauth,jwt".to_string()),
    // ... remaining fields
};
```

### Pipeline Results

`run_cli` returns `Result<()>`; the structured report is
`pipeline::report::PipelineReport` (`crates/eggsec/src/pipeline/report.rs:19`):

```rust
pub struct PipelineReport {
    pub target: String,
    pub total_duration_ms: u64,
    pub stage_results: Vec<StageResult>,
    pub open_ports: Vec<PortResult>,
    pub services: Vec<ServiceFingerprint>,
    pub endpoints: Vec<EndpointResult>,
    pub manifest: Option<RunManifest>,
    pub vuln_assessment: Option<VulnAssessment>,
    pub load_test_results: Option<LoadTestResults>,
    // + `web_proxy_report` under the `web-proxy` feature
}

pub struct StageResult {
    pub stage: Stage,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}
```

## Output

### Output Formats

```rust
use eggsec::pipeline::report::{generate_csv, generate_html, PipelineReport};
use serde_json;

let report = PipelineReport { /* ... */ };

// Generate HTML report
let html = generate_html(&report)?;

// Generate CSV export
let csv = generate_csv(&report)?;

// Generate JSON output
let json = serde_json::to_string_pretty(&report)?;
```

Both generators take a `&PipelineReport` and return `eggsec::error::Result<String>`
(`crates/eggsec/src/pipeline/report.rs:194` and `:335`).

### SARIF Output (via eggsec-output crate)

`SarifBuilder` has no `with_report`; it accumulates rules and results, then
builds. `SarifReport::to_json` serialises it (`crates/eggsec-output/src/sarif.rs`):

```rust
use eggsec_output::SarifBuilder;

let sarif = SarifBuilder::new()
    .with_tool("eggsec".to_string(), env!("CARGO_PKG_VERSION").to_string())
    .add_rule("RULE-ID", "Rule name", "warning", "description")
    .add_result(/* rule_id, uri, message */)
    .build();

let json = sarif.to_json()?;
```

### JUnit XML Output (via eggsec-output crate)

```rust
use eggsec_output::JUnitBuilder;

let junit = JUnitBuilder::new("eggsec")
    .add_finding(/* ... */)
    .build();
let xml = junit.to_xml()?;
```

`to_xml` takes `&self` and returns `Result<String, quick_xml::Error>`
(`crates/eggsec-output/src/junit.rs:260`).

## Error Handling

Eggsec uses `anyhow` for application-level error handling and `thiserror` for library error types. The canonical error type is `EggsecError` in `error/mod.rs` with domain-specific variants.

```rust
use anyhow::Result;

async fn run_scan() -> Result<()> {
    let config = load_config(None)?;
    let scope = load_scope(None)?;
    
    // Run scan
    let results = scanner::ports::run_cli(args, &config).await?;
    
    Ok(())
}
```

### Policy Enforcement

All target-bearing operations must pass policy enforcement before execution:

```rust
use anyhow::Result;

async fn run_scan() -> Result<()> {
    let config = load_config(None)?;
    let scope = load_scope(None)?;
    
    // Run scan
    scanner::ports::run_cli(args, &config).await?;
    
    Ok(())
}
```

### Policy Enforcement

`EnforcementContext::evaluate()` (`crates/eggsec/src/config/policy_decision.rs:141`)
is the mandatory pre-dispatch gate for **all** surfaces. Strict surfaces
(REST/MCP/agent/CI) must dispatch through `EnforcedDispatcher::dispatch_execution()`
with an `ApprovedExecution` bundle from `approve_execution()` /
`approve_manual_execution()`; only scope-insensitive manual tools may use the
raw `dispatch_checked()` path. See
[`docs/ENFORCEMENT_MODES.md`](ENFORCEMENT_MODES.md) and
[`architecture/dispatch.md`](../architecture/dispatch.md).

`CommandContext` wraps the enforcement context for CLI handlers
(`crates/eggsec/src/commands/handlers/mod.rs:107`):

```rust
use eggsec::commands::handlers::CommandContext;

let ctx = CommandContext::new(config)?;

let decision = ctx.evaluate_and_enforce_operation(OperationDescriptor {
    operation: "scan-ports".to_string(),
    mode: OperationMode::StandardAssessment,
    risk: OperationRisk::SafeActive,
    intended_uses: vec![/* IntendedUse */],
    target: Some(target.to_string()),
    normalized_target: eggsec_policy::target::normalize_target(target, None),
    // required_features, required_policy_flags, capability requirements
})?;
```

`OperationDescriptor` (`crates/eggsec-policy/src/policy.rs:284`) carries
`intended_uses`, `normalized_target`, `required_features`,
`required_policy_flags`, `requires_private_or_local_target`,
`requires_explicit_scope` and `required_capabilities` in addition to the fields
shown. Prefer `OperationDescriptor::new` -- or, on strict surfaces,
`OperationMetadata::try_descriptor_for_target` -- over a bare struct literal.
`evaluate_and_enforce_operation` returns `Result<PolicyDecision>`.

## Creating Custom Scans

You can implement custom scanning logic using the existing modules:

```rust
use eggsec::fuzzer::engine::FuzzEngine;
use eggsec::fuzzer::config::FuzzConfig;

let mut engine = FuzzEngine::new(FuzzConfig {
    url: "https://example.com/api".to_string(),
    payload_types: vec![/* PayloadType::Sqli, PayloadType::Xss */],
    // ...
})?;

engine.run().await?;
```

`FuzzEngine::new` takes a `FuzzConfig` (not an `EggsecConfig`)
(`crates/eggsec/src/fuzzer/engine/core.rs:128`). There are no `payloads()` or
`analyze()` methods -- the engine exposes `run()`, `run_all_types()`,
`run_return_session()` and the AI-generator hooks. For per-payload inspection,
iterate `FuzzResult` values as described under [Fuzzing](#fuzzing).
