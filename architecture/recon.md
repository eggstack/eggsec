# Reconnaissance Module

Deep-dive architecture for `crates/eggsec/src/recon/`.

## Role & Responsibilities

The reconnaissance module performs **passive and active information gathering** about a target before more intrusive testing. It aggregates results from 17+ specialized sub-modules into a single `FullReconResult` struct. Recon modules are **policy-free executors**: they accept plain config structs and never reference `EnforcementContext`. All authorization and scope enforcement happens upstream in the dispatch layer ([dispatch.md](dispatch.md)).

## Location & Feature Gating

- **Path**: `crates/eggsec/src/recon/` (35 `.rs` files: 30 top-level + 5 in `cloud/`)
- **Declared modules** (`mod.rs:78-102`): 19 unconditional `pub mod` + 1 conditional (`cloud` behind `cfg(feature = "cloud")`) = 20 total
- **Feature-gated modules**: `cloud` (feature `cloud`), `git_secrets` (feature `git-secrets`)
- **Detached utilities** (7 files exist on disk but are NOT declared as `pub mod` — `mod.rs:497-505`): `asn`, `cve_lookup`, `dns_enhanced`, `ftp_auth`, `smtp_auth`, `ssh_auth`, `ssl_audit`

## Module Inventory

| File / Directory | Lines | Purpose | Pipeline? | Notes |
|------------------|-------|---------|:---------:|-------|
| `mod.rs` | 520 | Module declarations, `FullReconResult`, `ReconRequest`, entry points, `FULL_RECON_PIPELINE_MODULES` constant | — | 17-module list at `mod.rs:435-453` |
| `runner.rs` | 1107 | Pipeline orchestrator (`run_full_recon_from_request`), all `run_*` wrappers, result aggregation, human-readable formatter | — | `runner.rs:536` is the primary entry |
| `techdetect.rs` | 538 | Technology stack detection via HTTP headers + body signatures (servers, frameworks, CMS, CDNs, JS libs, languages) | Yes | 8 detection categories |
| `subdomain.rs` | 454 | Subdomain enumeration via crt.sh certificate transparency, Threatminer API, DNS brute-force | Yes | Uses `hickory_resolver` with configurable concurrency |
| `ssl.rs` | 339 | SSL/TLS certificate analysis: chain inspection, protocol versions, cipher suites, expiry checks | Yes | Extracts `CertificateDer` from reqwest extensions |
| `cve.rs` | 498 | CVE mapping: built-in database (7 product families) + NVD API v2.0 fallback | Yes | Global `OnceLock` cache (`CVE_CACHE`); optional NVD API key |
| `secrets.rs` | 492 | Secret detection in HTTP responses via 25 regex patterns (31 `SecretType` enum variants, 25 with active patterns) | Yes | LazyLock patterns; entropy filter for AWS secrets |
| `content.rs` | 423 | Content/directory discovery: scans ~80 sensitive paths concurrently | Yes | Semaphore-bounded concurrency |
| `cors.rs` | 281 | CORS misconfiguration testing: sends 9 test origins, checks `Access-Control-*` headers | Yes | Tests `null`, `*`, localhost, evil origins |
| `dns_records.rs` | 185 | DNS record enumeration: A, AAAA, MX, TXT, NS, SOA, CAA via `hickory_resolver` | Yes | No external API dependency |
| `reverse_dns.rs` | 187 | Reverse DNS lookup (PTR) + ASN extraction from hostname | Yes | Uses `hickory_resolver` with 10s timeout |
| `geolocation.rs` | 681 | IP geolocation: MaxMind local DB + online fallback (ip-api.com, ipapi.co, ipwho.is, ip2c) | Yes | 5 online providers; local/private IP handling |
| `whois.rs` | 265 | WHOIS lookup via raw TCP to WHOIS servers with retry logic | Yes | Protocol-level, no HTTP API |
| `threatintel.rs` | 647 | Threat intelligence: VirusTotal (IP/domain), AlienVault OTX, Shodan | Yes | Requires API keys; graceful degradation |
| `wayback.rs` | 289 | Wayback Machine historical URL retrieval via CDX API | Yes | Optional API key for higher rate limits |
| `email.rs` | 295 | Email/contact discovery: extracts emails, phone numbers, social media profiles, physical addresses via regex | Yes | 9 social platform patterns |
| `email_security.rs` | 913 | Email security analysis: SPF, DKIM, DMARC, MX, STARTTLS, BIMI records | No | Standalone; not in `run_full_recon` pipeline |
| `takeover.rs` | 560 | Subdomain takeover detection via dangling CNAME/NS fingerprinting | Yes | 10+ service fingerprints; runs after subdomain enum |
| `js.rs` | 375 | JavaScript file analysis: endpoint extraction, API key/secret detection in JS sources | Yes | Uses `scraper` crate for HTML parsing |
| `api_schema.rs` | 299 | API schema discovery: probes 19 common OpenAPI/Swagger/GraphQL paths | No | Standalone; not in pipeline |
| `containers.rs` | 300 | Container security: Kubernetes pod scanning, Docker config analysis | No | Feature-gated: `container` |
| `cloud/mod.rs` | 453 | Cloud asset discovery: S3, Azure Blob, GCP Storage, Firebase, Heroku, GitHub repos | Yes* | Feature-gated: `cloud`; runs separately from main parallel block |
| `cloud/iam.rs` | 219 | IAM privilege escalation pattern analysis (12 known patterns) | — | Sub-module of cloud |
| `cloud/metadata.rs` | 157 | IMDSv1/v2 metadata endpoint testing for AWS/GCP/Azure | — | Sub-module of cloud; 3s per-endpoint timeout |
| `cloud/services.rs` | 104 | Cloud service enumeration: Lambda, API Gateway, CloudFront, Azure Functions, GCP Functions | — | Sub-module of cloud |
| `cloud/storage_test.rs` | 225 | S3 bucket security testing: public read, listing, ACL checks | — | Sub-module of cloud |
| `spinner.rs` | 34 | Terminal progress spinner (Braille characters) for CLI output | No | CLI-only; manages display lifecycle |
| **Detached (not wired):** | | | | |
| `asn.rs` | ~280 | ASN lookup via ARIN RDAP | No | `intentionally_detached` |
| `cve_lookup.rs` | ~420 | Dedicated CVE engine with caching, ExploitDB integration | No | `intentionally_detached` |
| `dns_enhanced.rs` | ~400 | Enhanced DNS enumeration with wordlist-based brute force | No | `intentionally_detached` |
| `ftp_auth.rs` | ~180 | FTP banner grabbing and authentication testing | No | `intentionally_detached` |
| `smtp_auth.rs` | ~200 | SMTP banner grabbing and auth testing (LOGIN/PLAIN) | No | `intentionally_detached` |
| `ssh_auth.rs` | ~200 | SSH banner grabbing and limited auth probing | No | `intentionally_detached` |
| `ssl_audit.rs` | ~230 | TestSSL-like TLS security auditing | No | `intentionally_detached` |
| `git_secrets.rs` | 473 | Git repository secret scanning (feature: `git-secrets`) | No | Feature-gated; standalone |

## Pipeline Flow

`run_full_recon_from_request()` (`runner.rs:536`) is the primary engine entry point.

### 1. Target Resolution (`runner.rs:42-93`)

`resolve_target()` strips protocol prefixes, extracts the domain, and performs DNS resolution if the target is not already an IP. Returns `(url, domain, resolved_ip, port)`.

### 2. Parallel Execution Block (`runner.rs:589-628`)

13 modules execute concurrently via `tokio::join!`:

```
reverse_dns, geolocation, threat_intel, ssl, whois, subdomain_enum,
dns_records, tech_detection, js_analysis, wayback_check,
content_analysis, cors_check, email_discovery
```

Cloud detection runs separately after the parallel block (feature-gated `#[cfg(feature = "cloud")]`).

### 3. Sequential Dependencies (post-parallel)

| Module | Depends On | Runner Function |
|--------|-----------|-----------------|
| `takeover` | `subdomain_enum` results | `run_takeover_check()` (`runner.rs:398-429`) |
| `cve` | `tech_detection` results | `run_cve_check()` (`runner.rs:434-451`) |
| `secrets` | `content_analysis` results | `run_secrets_check()` (`runner.rs:456-507`) |

### 4. Result Aggregation (`runner.rs:656-753`)

Each `ReconStep` result is checked: `.is_failed()` populates error strings on `FullReconResult`; `.into_option()` extracts successful values. Non-critical failures are tracked but do not halt the pipeline.

### Pipeline Module List

`FULL_RECON_PIPELINE_MODULES` (`mod.rs:435-453`) = 17 canonical modules:

```
reverse_dns, geolocation, threatintel, ssl, whois, subdomain,
dns_records, techdetect, js, wayback, cloud, content, cors,
email, takeover, cve, secrets
```

## Key Capabilities (per-submodule)

### techdetect — Technology Stack Detection (`techdetect.rs`)

- **Data source**: HTTP response headers (`Server`, `X-Powered-By`, `X-Framework`, CDN-specific headers like `CF-Ray`) + body content analysis
- **Detection categories**: Servers (Nginx, Apache, IIS, LiteSpeed, OpenResty, Caddy), Frameworks (Express, Django, Rails, Laravel, Spring, ASP.NET, Next.js, etc.), CMS (WordPress, Drupal, Joomla, Magento, Shopify), CDNs (Cloudflare, Akamai, CloudFront, Fastly, BunnyCDN, KeyCDN, jsDelivr), Databases (MySQL, PostgreSQL, MongoDB, Redis, Elasticsearch, Memcached), JavaScript libs (React, Vue.js, Angular, jQuery, Svelte, Backbone), Languages (PHP, Ruby, Python, Node.js, Java, C#, Go, Rust)
- **Output**: `TechDetectionResult` with `TechStack` (8 vectors) + status code + headers

### cve — CVE Mapping (`cve.rs`)

- **Data sources**: Built-in CVE database (Apache, nginx, WordPress, Node.js/Express, MySQL, PostgreSQL, Redis, MongoDB) + NVD API v2.0 (`services.nvd.nist.gov`)
- **Caching**: Global `OnceLock<Arc<Mutex<FxHashMap>>>` cache (`cve.rs:11`) — survives across multiple `CveMapper` instances within a process
- **NVD API**: Requires optional API key (`config.recon.apis.nvd.api_key`); 10 results per product; sorted by CVSS score descending
- **Output**: `CveMapping` with vulnerability list + severity counts (critical/high/medium)

### secrets — Secret Detection (`secrets.rs`)

- **Pattern count**: 25 regex patterns covering 31 `SecretType` enum variants. Six variants (`AzureKey`, `GcpServiceAccount`, `HerokuKey`, `NetlifyToken`, `DockerhubToken`, `KubernetesSecret`) are defined in the enum but have no active pattern in `build_patterns()`.
- **High-confidence types**: AWS keys (3 variants), GitHub tokens (PAT + OAuth), GitLab PAT, Slack tokens, OpenAI keys, Stripe keys, GCP API keys, private keys, JWT tokens, Discord tokens, Twilio/SendGrid/Mailchimp keys, database connection strings (MongoDB/PostgreSQL/MySQL URIs), password-in-URL, GitHub credentials in URL
- **Entropy filter**: AWS secret key candidates with Shannon entropy < 3.5 are discarded (`secrets.rs:334`)
- **Output**: `Vec<SecretFinding>` with type, value preview (truncated to 20 chars), confidence, severity

### cloud — Cloud Asset Discovery (`cloud/mod.rs`)

- **Sub-modules**: `iam` (12 privilege escalation patterns), `metadata` (IMDSv1/v2 testing), `services` (Lambda/API Gateway/CloudFront/Azure Functions/GCP Functions), `storage_test` (S3 bucket security)
- **Enumerated providers**: S3 buckets, Azure Blob Storage, GCP Cloud Storage, Firebase Realtime Database, Heroku apps, GitHub repos
- **Name generation**: `generate_cloud_names()` (`cloud/mod.rs:416-447`) creates 25 name variants per domain (e.g., `{domain}-prod`, `{domain}-staging`, `{base}-static`)
- **Execution**: Semaphore-bounded concurrency; 30s timeout per HTTP probe; runs in separate `tokio::spawn` tasks

### takeover — Subdomain Takeover Detection (`takeover.rs`)

- **Fingerprint database**: 10+ services (AWS S3, GitHub Pages, Heroku, Azure Web Apps, GCP Storage, Shopify, Fastly, Pantheon, Zendesk, Squarespace, Campaign Monitor, Helpjuice, Helpscout, Statuspage, Tilda, Intercom, Mashery) — each with CNAME patterns, NXDOMAIN indicators, and HTTP body signatures
- **Detection method**: Dangling CNAME/NS records + HTTP response body fingerprinting
- **Output**: `Vec<TakeoverResult>` with status (`Vulnerable`/`Safe`/`Unknown`), service identification, evidence string, severity

### email_security — Email Security Analysis (`email_security.rs`)

- **Checks**: SPF record validation, DKIM selector enumeration, DMARC policy analysis, MX record security, STARTTLS support testing, BIMI record discovery
- **Notable**: 913 lines; the most complex standalone module. Tests TLS negotiation via direct TCP + `native-tls` connector.
- **Output**: `EmailSecurityReport` with per-check results + overall score + findings list

### subdomain — Subdomain Enumeration (`subdomain.rs`)

- **Sources**: crt.sh (Certificate Transparency logs), Threatminer API, DNS brute-force with hickory_resolver
- **Concurrency**: Configurable via `ReconRequest.concurrency` (defaults to `config.recon.dns_concurrency`); semaphore-bounded
- **Resolver**: `hickory_resolver::TokioResolver` with 10s timeout, 2 attempts

### wayback — Wayback Machine (`wayback.rs`)

- **API**: Wayback Machine CDX API (`web.archive.org/cdx/search/cdx`)
- **Rate limiting**: Optional API key for higher throughput
- **Default limit**: 100 snapshots per domain

## Entry Points & Public API

| Entry Point | Signature | File:Line | Feature Gate |
|-------------|-----------|-----------|--------------|
| `run_full_recon_from_request()` | `pub async fn run_full_recon_from_request(request: &ReconRequest, config: &EggsecConfig, stage: Arc<Mutex<String>>, verbose: bool) -> Result<FullReconResult>` | `runner.rs:536` | — |
| `run_full_recon()` | `pub async fn run_full_recon(args: &ReconArgs, config: &EggsecConfig, stage: Arc<Mutex<String>>, verbose: bool) -> Result<FullReconResult>` | `runner.rs:520` | `cli` |
| `run_cli()` | `pub async fn run_cli(args: ReconArgs, config: &EggsecConfig) -> Result<()>` | `mod.rs:417` | `cli` |
| `run_with_callback()` | `pub async fn run_with_callback(request: &ReconRequest, config: &EggsecConfig, callback: F) -> Result<FullReconResult>` | `mod.rs:324` | `tool-api` |
| `run_cli_with_callback()` | `pub async fn run_cli_with_callback(args: ReconArgs, config: &EggsecConfig, callback: F) -> Result<()>` | `mod.rs:301` | `tool-api` + `cli` |
| `ReconRequest` | `pub struct ReconRequest` (target + 17 `no_*` toggles + concurrency) | `mod.rs:238` | — |
| `FullReconResult` | `pub struct FullReconResult` (18 `Option` result fields + errors) | `mod.rs:183` | — |
| `print_recon_results_string()` | `pub fn print_recon_results_string(recon: &FullReconResult) -> String` | `runner.rs:767` | — |
| `FULL_RECON_PIPELINE_MODULES` | `pub const FULL_RECON_PIPELINE_MODULES: &[&str]` (17 entries) | `mod.rs:435` | — |

### ReconRequest (`mod.rs:238-258`)

Plain struct (no Clap derives — CLI parsing converts `ReconArgs` to this via `From`):

```rust
pub struct ReconRequest {
    pub target: String,
    pub concurrency: Option<usize>,
    pub no_tech: bool,      pub no_dns: bool,
    pub no_geo: bool,       pub no_whois: bool,
    pub no_subdomains: bool, pub no_ssl: bool,
    pub no_dns_records: bool, pub no_js: bool,
    pub no_content: bool,   pub no_cloud: bool,
    pub no_wayback: bool,   pub no_cors: bool,
    pub no_threat: bool,    pub no_cve: bool,
    pub no_email: bool,     pub no_takeover: bool,
}
```

### FullReconResult (`mod.rs:183-223`)

18 result fields + 14 error fields:

```
target, domain, ip_address, tech_stack, reverse_dns, geolocation, whois,
subdomains, ssl_analysis, dns_records, js_analysis, wayback,
cloud (cfg), content, cors, email_discovery, threat_intel, cve_mapping,
takeover, secrets
```

Each has a corresponding `*_error: Option<String>` field (except `secrets` and `target`). Cloud fields are behind `#[cfg(feature = "cloud")]`.

## Integration Points

- **CLI handler**: `handle_recon()` in `crates/eggsec/src/commands/handlers/` calls `run_cli()`
- **Dispatch**: `eggsec::dispatch` routes `TaskKind::Recon` to `run_full_recon_from_request()` via the recon worker
- **Tool registry**: `run_with_callback()` (`mod.rs:324`) is the tool-API entry, emitting `Finding` objects for CVE, technology, and takeover results
- **Python bindings**: `eggsec-python` exposes `run_full_recon_from_request()` through the stable-core API
- **Pipeline**: Recon is stage in the scan pipeline; `ReconRequest` is constructed from pipeline context

## External Service Dependencies

| Service | Used By | API Key Required | Rate Limit Notes |
|---------|---------|:----------------:|------------------|
| crt.sh | `subdomain.rs` | No | Public CT logs; no key needed |
| Threatminer | `subdomain.rs` | No | Free API; may throttle |
| NVD (NIST) | `cve.rs` | Optional (improves throughput) | API key required for >5 req/30s; 10 results per query |
| ip-api.com | `geolocation.rs` | No | 45 req/min free tier |
| ipapi.co | `geolocation.rs` | No | Free tier available |
| ipwho.is | `geolocation.rs` | No | Free API |
| ip2c | `geolocation.rs` | No | Free API |
| MaxMind GeoIP2 | `geolocation.rs` | Yes (account + license) | Local DB lookup; optional auto-update |
| VirusTotal | `threatintel.rs` | Yes | 4 req/min free tier |
| AlienVault OTX | `threatintel.rs` | Yes | Free API |
| Shodan | `threatintel.rs` | Yes | 1 req/sec free tier |
| Wayback Machine CDX | `wayback.rs` | Optional (higher limits) | Unauthenticated: ~10-20 req/min |
| ARIN RDAP | `asn.rs` (detached) | No | Public; rate-limited |
| WHOIS servers | `whois.rs` | No | Protocol-level TCP; retry with backoff |

**Caching implemented**: CVE results (`cve.rs` — global `OnceLock` cache). No other module-level caching.

## Testing

- **Unit tests**: Present in every submodule (`#[cfg(test)] mod tests`). Test serialization round-trips, scanner creation, pattern matching, and result construction.
- **Module registration test**: `mod.rs:456-519` (`recon_modules_match_filesystem`) asserts that `pub mod` declarations match the filesystem, accounting for the `intentionally_detached` set.
- **Runner tests**: `runner.rs:981-1107` cover target resolution (HTTP/HTTPS/IP/IPv6/port), `FullReconResult` construction, serialization, and NVD API key extraction.

## Invariants & Gotchas

1. **Policy-free modules**: Recon functions never reference `EnforcementContext`. Scope enforcement is upstream in dispatch.
2. **Detached files exist for a reason**: The 7 `intentionally_detached` modules (`mod.rs:497-505`) are standalone utilities that exist on disk but are NOT wired into the module tree. They are available for direct internal use but are excluded from the public API and pipeline. The `recon_modules_match_filesystem` test enforces this separation.
3. **Cloud runs separately**: The `cloud` module executes AFTER the main `tokio::join!` parallel block (`runner.rs:644-648`) because it is feature-gated and cannot participate in the uniform join tuple.
4. **Takeover depends on subdomains**: `run_takeover_check()` (`runner.rs:398-429`) receives the subdomain result by reference and only proceeds if subdomains were found. This is a sequential dependency.
5. **CVE depends on tech detection**: `run_cve_check()` (`runner.rs:434-451`) receives the tech detection result by reference. CVE mapping cannot run without a detected tech stack.
6. **Secrets depend on content**: `run_secrets_check()` (`runner.rs:456-507`) fetches each sensitive file URL discovered by content scanning and applies regex patterns to the response body. This is sequential after content discovery.
7. **ReconStep<T> is internal**: The `ReconStep` enum (`runner.rs:19-36`) is not public; it is an internal tracking mechanism for pipeline step outcomes.
8. **SpinnerGuard is CLI-only**: The `SpinnerGuard` struct and all spinner logic are behind `#[cfg(feature = "cli")]`.

## Confirmed Bugs

No critical bugs found. Minor observations:

- **`runner.rs:1008`**: `let _ = resolved_ip;` — test-only, not a production concern (suppresses unused variable warning in `test_resolve_target_no_prefix`).
- **No `unwrap_or_default()` on async operations**: Clean.
- **No `std::collections::HashMap` in hot paths**: All collections use `rustc_hash::FxHashMap`/`FxHashSet`.
- **All outbound HTTP calls have timeouts**: Either via `tokio::time::timeout` (cloud, content, cors, js, takeover, runner) or via reqwest client timeout configuration (10-30s). Cloud metadata uses 3s per-endpoint timeout.

## Document References

- [overview.md](overview.md) — system architecture, module index
- [scanner.md](scanner.md) — port scanning and service fingerprinting
- [dispatch.md](dispatch.md) — task dispatch and enforcement flow
- [config.md](config.md) — configuration, scope, enforcement model

*Last verified against source: 2026-08-25*
