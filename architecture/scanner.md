# Scanner Module

The Scanner module is responsible for the "discovery" phase of a security assessment. It includes port scanning, service identification, endpoint discovery, vulnerability template matching, and CMS-specific security scanning. All scanner operations are **policy-free executors** — they receive plain config structs and never touch `EnforcementContext`. Authorization happens upstream in dispatch. See [dispatch.md](dispatch.md) and [overview.md](overview.md).

## Role & Responsibilities

- **TCP port scanning** with concurrent connections, spoofed raw-socket scans (SYN/Null/FIN/Xmas), and Nmap-style timing templates (T0–T5)
- **Endpoint discovery** via wordlist-based brute forcing (347 built-in paths) with custom wordlist support
- **Service fingerprinting** through banner grabbing and protocol-specific probes (45 probes, CPE/CVE output)
- **UDP fingerprinting** for DNS, SNMP, NTP, game servers, ICS/SCADA, and 40+ other services
- **ICMP host discovery** (feature-gated behind `stress-testing`)
- **CMS scanning** for WordPress, Drupal, and Joomla (detection, component enumeration, CVE version compare, misconfiguration checks)
- **Nuclei-style template engine** with YAML/JSON templates, Ed25519 signing/verification, marketplace integration, and Interactsh callback support

## Location & Feature Gating

| Component | Path | Feature Gate |
|-----------|------|-------------|
| Port scanning | `scanner/ports/mod.rs` | Always |
| Spoofed port scanning | `scanner/ports/spoofed.rs` | `stress-testing` + Unix |
| Endpoint discovery | `scanner/endpoints.rs` | Always |
| TCP fingerprinting | `scanner/fingerprint.rs` | Always |
| Fingerprint types | `scanner/fingerprint_types.rs` | Always |
| UDP fingerprinting | `scanner/udp_fingerprint.rs` | Always |
| ICMP probing | `scanner/icmp_probe.rs` | `stress-testing` |
| Timing templates | `scanner/timing.rs` | Always |
| Spoof config | `scanner/spoof.rs` | Always (raw sockets gated) |
| Wordlist parsing | `scanner/wordlist.rs` | Always |
| Template engine | `scanner/templates/` | Always |
| CMS scanning | `scanner/cms/` | Always |

## Architecture

### Module Inventory

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 112 | Module declarations, re-exports, doc examples |
| `ports/mod.rs` | 781 | TCP connect scan, `scan_ports()`, CLI/tool-api paths |
| `ports/spoofed.rs` | 647 | Raw-socket spoofed scanning, packet trace, response parsing |
| `endpoints.rs` | 1168 | HTTP endpoint discovery, `scan_endpoints()`, wordlist integration |
| `fingerprint.rs` | 837 | TCP service fingerprinting, `fingerprint_services()`, `fingerprint_port()` |
| `fingerprint_types.rs` | 188 | `FingerprintConfidence`, `ServiceIdentity`, `EnhancedFingerprint` |
| `udp_fingerprint.rs` | 533 | UDP service fingerprinting, `fingerprint_udp_services()` |
| `icmp_probe.rs` | 293 | ICMP echo, `ping_host()` |
| `timing.rs` | 275 | `TimingPreset` (6 presets T0–T5), `TimingConfig`, `PortPriority`, `RetryConfig` |
| `spoof.rs` | 599 | `SpoofConfig`, `ScanType`, `DecoyMode`, packet builders, CIDR utilities |
| `wordlist.rs` | 232 | `Wordlist` parser with validation, path normalization |
| `templates/mod.rs` | 100 | Template engine module root, `load_builtin_templates()` |
| `templates/executor.rs` | 359 | `TemplateEngine`, `TemplateExecutor`, request dispatch, Interactsh |
| `templates/loader.rs` | 368 | `TemplateLoader`, YAML/JSON parsing, validation, tag/ID lookup |
| `templates/matcher.rs` | 476 | `TemplateMatcher`, word/regex/binary match, regex cache (`REGEX_CACHE`) |
| `templates/models.rs` | 176 | `VulnerabilityTemplate`, `Matcher`, `HttpMatcher`, `DnsMatcher`, `SearchPattern` |
| `templates/marketplace.rs` | 376 | `TemplateMarketplace`, download/cache/verify, tag filtering |
| `templates/verify.rs` | 376 | `TemplateSigner`, `TemplateVerifier`, Ed25519 signing |
| `templates/standard/` | 1 file | Built-in template: `log4j-rce.yaml` |
| `cms/mod.rs` | 419 | `CmsScanner`, CMS detection, version compare (`version_lt`), shared helpers |
| `cms/wordpress.rs` | 212 | WP plugin/theme enum, XML-RPC, debug mode, user enum, CVE check |
| `cms/drupal.rs` | 116 | Drupal module enum, version detection from CHANGELOG.txt |
| `cms/joomla.rs` | 121 | Joomla extension enum, version detection from manifest XML |

### Key Types — Port Scanning

| Type | Location | Fields/Variants | Notes |
|------|----------|-----------------|-------|
| `PortScanConfig` | `ports/mod.rs:36` | `ports`, `concurrency`, `timeout_duration`, `tui_mode`, `spoof_config`, `progress_tx`, `max_results` | Default: concurrency=100, timeout=3s |
| `PortScanRequest` | `ports/mod.rs:74` | `host`, `ports`, `concurrency`, `timeout`, `spoof_config`, `dry_run` | Engine-facing contract (no Clap) |
| `PortResult` | `ports/mod.rs:131` | `port`, `status`, `service` | Per-port result |
| `PortScanResults` | `ports/mod.rs:137` | `host`, `ports_scanned`, `open_ports`, `total_open_ports`, `duration_ms`, `spoof_stats` | Aggregate results |
| `MAX_SCAN_RESULTS` | `ports/mod.rs:29` | `10000` | Hard cap to bound memory |
| `ScanType` | `spoof.rs:21` | `Syn`, `Null`, `Fin`, `Xmas` | Default: `Syn` |

### Key Types — Endpoint Discovery

| Type | Location | Fields | Notes |
|------|----------|--------|-------|
| `EndpointScanConfig` | `endpoints.rs:23` | `base_url`, `endpoints`, `concurrency`, `timeout_duration`, `include_404`, `tui_mode`, `spoof_config`, `verify_tls`, `progress_tx`, `max_results` | |
| `EndpointScanRequest` | `endpoints.rs:42` | `url`, `wordlist`, `concurrency`, `timeout`, `include_404`, `spoof_config` | Engine-facing contract |
| `EndpointResult` | `endpoints.rs:446` | `path`, `status_code`, `status_text`, `content_length`, `response_time_ms`, `redirect`, `interesting` | `interesting` set by `is_interesting()` |
| `EndpointScanResults` | `endpoints.rs:457` | `base_url`, `endpoints_scanned`, `endpoints_found`, `total_endpoints_matched`, `interesting_findings`, `duration_ms`, `results` | |

### Key Types — Fingerprinting

| Type | Location | Notes |
|------|----------|-------|
| `FingerprintRequest` | `fingerprint.rs:30` | `host`, `ports`, `timeout`, `udp`, `concurrency` |
| `ServiceFingerprint` | `fingerprint.rs:117` | `port`, `service`, `banner`, `version`, `product`, `extra`, `confidence` (u8) |
| `FingerprintResults` | `fingerprint.rs:128` | `host`, `ports_scanned`, `services_identified`, `total_services_identified`, `duration_ms`, `results` |
| `FingerprintConfidence` | `fingerprint_types.rs:9` | `Unknown` < `Low` < `Medium` < `High` < `Confirmed` (5 levels, derives `Ord`) |
| `EvidenceType` | `fingerprint_types.rs:45` | `Banner`, `TlsCertificate`, `TlsAlpn`, `HttpHeader`, `HttpResponse`, `ProtocolNegotiation`, `DnsRecord`, `PortState` (8 variants) |
| `FingerprintEvidence` | `fingerprint_types.rs:36` | `kind`, `raw_value`, `redacted_value`, `confidence_contribution` |
| `ServiceIdentity` | `fingerprint_types.rs:58` | `service_name`, `version`, `product`, `vendor`, `protocol`, `transport`, `port`, `confidence`, `evidence`, `cpe`, `possible_cves` |
| `EnhancedFingerprint` | `fingerprint_types.rs:95` | `identity`, `all_alternatives`, `raw_banner`, `scan_timestamp` |

### Key Types — Timing

| Type | Location | Notes |
|------|----------|-------|
| `TimingPreset` | `timing.rs:4` | 6 variants: `Paranoid`(T0), `Sneaky`(T1), `Polite`(T2), `Normal`(T3), `Aggressive`(T4), `Insane`(T5) |
| `TimingConfig` | `timing.rs:53` | `preset`, `min_parallelism`, `max_parallelism`, `timeout_ms`, `retry_count`, `retry_delay_ms`, `max_rate`, `port_batch_size`, `scan_delay_ms` |
| `PortPriority` | `timing.rs:160` | `CRITICAL_PORTS` (20), `HIGH_PORTS` (100), `categorize()`, `is_common()` |
| `RetryConfig` | `timing.rs:208` | Exponential backoff with `backoff_multiplier` (2.0) |

### Key Types — Spoofing

| Type | Location | Notes |
|------|----------|-------|
| `SpoofConfig` | `spoof.rs:30` | `enabled`, `source_ip`, `ip_range`, `use_raw_sockets`, `decoy_ips`, `decoy_count`, `decoy_mode`, `include_real_ip`, `source_port`, `random_source_port`, `fragment`, `scan_type`, `packet_trace`, `max_rate`, `ttl` |
| `DecoyMode` | `spoof.rs:14` | `Simultaneous` (default), `Staggered` |
| `SpoofStats` | `spoof.rs:221` | `packets_sent`, `packets_dropped`, `spoofed_ips_used`, `decoys_used`, `unique_decoy_ips`, `decoy_mode` |

### Key Types — Templates

| Type | Location | Notes |
|------|----------|-------|
| `VulnerabilityTemplate` | `models.rs:11` | `id`, `info`, `matchers`, `requests`. `severity()` maps to `Severity` |
| `TemplateInfo` | `models.rs:21` | `name`, `author`, `severity`, `description`, `tags`, `references`, `remediation` |
| `Matcher` | `models.rs:71` | Tagged enum: `Http(HttpMatcher)`, `Dns(DnsMatcher)`, `Other` |
| `HttpMatcher` | `models.rs:48` | `path`, `method`, `headers`, `body`, `search`, `status_codes`, `interactsh` |
| `DnsMatcher` | `models.rs:63` | `query_type`, `search` |
| `SearchPattern` | `models.rs:79` | `pattern`, `mode` (Word/Regex/Binary), `encoding` |
| `TemplateRequest` | `models.rs:104` | `method`, `path`, `headers`, `body`, `raw`. Default: GET / |
| `TemplateEngine` | `executor.rs:279` | Wraps `TemplateExecutor` in `Arc`. Methods: `scan()`, `scan_with_callback()` |
| `TemplateExecutor` | `executor.rs:17` | `client`, `loader`, `matcher`, `timeout`. Methods: `execute_on_target()`, `execute_template()`, `execute_dns_template()` |
| `TemplateLoader` | `loader.rs:11` | `template_dirs`. Methods: `load_all()`, `load_by_id()`, `load_by_tag()`, `parse_template()`, `validate_template()` |
| `TemplateMatcher` | `matcher.rs:34` | `interactsh_urls`. Methods: `match_template()`, `match_http()`, `match_dns()`, `search_pattern()`. Global `REGEX_CACHE` (LazyLock<DashMap>) |
| `TemplateMarketplace` | `marketplace.rs:34` | `base_url`, `http_client`, `local_cache`, `verifier`, `verify_downloaded`. Methods: `list_templates()`, `download_template()`, `sync_templates()` |
| `TemplateSigner` | `verify.rs:33` | Ed25519 signing. Methods: `sign()`, `new()`, `from_keypair()`, `save_private_key()` |
| `TemplateVerifier` | `verify.rs:114` | Ed25519 verification. Methods: `verify()`, `verify_raw()`, `with_public_key()` |
| `SignedTemplate` | `verify.rs:17` | `template`, `signature` (base64), `public_key` (base64), `signer_info` |

### Key Types — CMS

| Type | Location | Notes |
|------|----------|-------|
| `CmsType` | `cms/mod.rs:39` | `WordPress`, `Drupal`, `Joomla`, `Unknown` (4 variants) |
| `CmsTarget` | `cms/mod.rs:30` | `url`, `detected_cms`, `version`, `plugins`, `themes` |
| `CmsScanResult` | `cms/mod.rs:58` | `target`, `cms_type`, `version`, `vulnerabilities`, `misconfigurations`, `security_headers`, `overall_severity` |
| `CmsVulnerability` | `cms/mod.rs:69` | `id`, `title`, `severity`, `description`, `cve_ids`, `fixed_in_version` |
| `CmsMisconfiguration` | `cms/mod.rs:79` | `id`, `title`, `severity`, `description`, `recommendation` |
| `CmsScanner` | `cms/mod.rs:87` | `http_client`. Methods: `detect_cms()`, `scan()`, `build_vulnerabilities()`, `build_scan_result()` |

## Behavior / Flow

### Port Scan Lifecycle

```
scan_ports(host, config)                     [ports/mod.rs:531]
  ├─ concurrency == 0? → error
  ├─ spoof_config.enabled && use_raw_sockets?
  │   └─ YES → scan_ports_spoofed()          [ports/spoofed.rs:106]
  │       ├─ check_privileged("IP spoof")
  │       ├─ open pnet datalink channel
  │       ├─ spawn packet-capture thread (parse_tcp_response)
  │       ├─ for each port (semaphore-controlled):
  │       │   ├─ build_tcp_packet() or build_fragmented_packets()
  │       │   ├─ [optional] send decoy packets (Simultaneous or Staggered)
  │       │   ├─ insert into sent_packets map
  │       │   └─ poll responses with exponential backoff
  │       ├─ collect open ports from responses
  │       └─ return PortScanResults with SpoofStats
  └─ NO → TCP connect scan
      ├─ resolve_host()
      ├─ for each port (semaphore-controlled):
      │   ├─ tokio::spawn with 300s timeout wrapper
      │   ├─ connect_with_nodelay_timeout()
      │   ├─ on success: insert into DashMap, track max_results
      │   └─ update progress (indicatif bar or TUI channel)
      ├─ join_all handles
      ├─ Arc::try_unwrap(results) → sort by port
      └─ truncate to MAX_SCAN_RESULTS (10,000)
```

**Result bounds**: `MAX_SCAN_RESULTS = 10,000` (`ports/mod.rs:29`) caps returned results. The `max_results` field on config provides a caller-requested lower cap. Both enforce memory safety under high-port-count scans.

### Endpoint Discovery Flow

```
scan_endpoints(config)                       [endpoints.rs:992]
  ├─ install_tls_provider()
  ├─ concurrency == 0? → error
  ├─ build reqwest::Client (timeout, TLS verification, redirect policy: max 5)
  ├─ for each endpoint (semaphore-controlled):
  │   ├─ join_endpoint_url(base, endpoint) — rejects path traversal (../)
  │   ├─ build GET request with optional spoof headers (X-Forwarded-For, X-Real-IP, X-Originating-IP)
  │   ├─ send request
  │   ├─ if status != 404 or include_404:
  │   │   ├─ extract content_length, redirect location
  │   │   ├─ is_interesting() — checks against 88 sensitive patterns on status 200/403/401
  │   │   └─ insert into DashMap
  │   └─ update progress
  ├─ join_all handles
  ├─ sort: interesting first, then by status, then path
  └─ truncate to MAX_SCAN_RESULTS (100,000)
```

**Wordlist integration**: Custom wordlists are loaded via `Wordlist::from_file()` (`wordlist.rs:19`) which normalizes paths (ensures leading `/`), skips `#` comments and blank lines, rejects paths > 2048 chars, whitespace, control characters, and traversal components (`.`/`..`). If no wordlist is provided, `DEFAULT_ENDPOINTS` (347 paths, `endpoints.rs:95`) is used.

**Interesting path detection**: `is_interesting()` (`endpoints.rs:497`) matches against 88 sensitive patterns (`.env`, `.git`, `credentials`, `admin`, `wp-config`, `actuator/heapdump`, `swagger`, `jenkins`, etc.) only on HTTP 200, 403, or 401 status codes. Matching is case-insensitive with segment-based partial matching.

### Fingerprint Scoring Flow

```
fingerprint_services(host, ports, ...)       [fingerprint.rs:316]
  ├─ concurrency == 0? → error
  ├─ resolve_host()
  ├─ for each port (semaphore-controlled):
  │   ├─ tokio::spawn with 300s timeout wrapper
  │   ├─ fingerprint_port(resolved_ip, port, timeout)  [fingerprint.rs:431]
  │   │   ├─ select probes_to_try:
  │   │   │   ├─ Known port → port-specific probe (e.g., 22→SSH, 3306→MySQL)
  │   │   │   └─ Unknown port → fall back to static PROBES list (45 entries)
  │   │   ├─ for each candidate probe:
  │   │   │   ├─ connect via TcpStream
  │   │   │   ├─ send probe payload (if non-empty)
  │   │   │   ├─ read response (up to 4096 bytes, with timeout)
  │   │   │   ├─ match pattern:
  │   │   │   │   ├─ "\\x" prefix → hex_match() (byte-level comparison)
  │   │   │   │   └─ plain → case-insensitive contains()
  │   │   │   └─ on match: extract banner (first 3 lines, ≤200 chars)
  │   │   │       + extract_product_version() for HTTP/SSH
  │   │   │       → return ServiceFingerprint { confidence: 90 }
  │   │   └─ no match → return None
  │   └─ insert result into DashMap
  └─ aggregate results, sort by port
```

**PROBES static** (`fingerprint.rs:68-114`): **45** protocol probes covering HTTP, SSH, SMTP, FTP, MySQL, Redis, MongoDB, PostgreSQL, Memcached, RDP, VNC, Telnet, XMPP, LDAP, SMB, Elasticsearch, Kafka, Zookeeper, RabbitMQ, Cassandra, CouchDB, Docker, Kubernetes, Etcd, Consul, Nats, InfluxDB, MSSQL, Oracle, Rsyncd, Couchbase, OpenVPN, WinRM, Jenkins, ActiveMQ, WebSocket, gRPC, Caddy, Harbor, GitLab, MinIO, Nginx, Apache, and IIS.

**Port-specific overrides**: `fingerprint_port()` (`fingerprint.rs:438`) uses targeted single-probe arrays for ~35 known ports (e.g., 22→SSH, 53→DNS, 3306→MySQL, 6379→Redis, 27017→MongoDB, 502→Modbus/ICS, 47808→BACnet) before falling back to the full PROBES list for unknown ports.

### CMS Scanning Flow

```
CmsScanner::detect_cms(url)                 [cms/mod.rs:183]
  ├─ GET url → parse HTML
  ├─ identify_cms():
  │   ├─ HTML contains "wp-content"/"wp-includes" → WordPress
  │   │   └─ extract_wordpress_version() via WP_VERSION_PATTERNS (2 regexes)
  │   ├─ HTML contains "drupal"/"sites/default" → Drupal
  │   ├─ HTML contains "joomla"/"com_content" → Joomla
  │   ├─ check_xml_rpc() → POST /xmlrpc.php for XML-RPC/blogging signature
  │   └─ none match → Unknown
  ├─ enumerate_components():
  │   ├─ WordPress: enumerate_plugins() (WP REST API /wp-json/wp/v2/plugins)
  │   │            + enumerate_themes() (WP REST API /wp-json/wp/v2/themes)
  │   ├─ Drupal: enumerate_modules() (directory listing at /web/modules)
  │   └─ Joomla: enumerate_extensions() (directory listing at /administrator/components)
  └─ return CmsTarget with detected_cms, version, plugins, themes

CmsScanner::scan(target)                    [cms/mod.rs:291]
  ├─ WordPress → scan_wordpress():
  │   ├─ build_vulnerabilities() against WORDPRESS_VULNERABILITIES (3 CVEs)
  │   ├─ check_plugin_vulnerabilities() for known-vulnerable plugins (wordfence, akismet)
  │   ├─ check_xml_rpc(), check_wp_debug(), check_user_enumeration()
  │   └─ build_scan_result()
  ├─ Drupal → scan_drupal():
  │   ├─ build_vulnerabilities() against DRUPAL_VULNERABILITIES (2 CVEs)
  │   ├─ detect_drupal_version() from /CHANGELOG.txt
  │   ├─ check admin login page (/user/login)
  │   └─ build_scan_result()
  └─ Joomla → scan_joomla():
      ├─ build_vulnerabilities() against JOOMLA_VULNERABILITIES (2 CVEs)
      ├─ detect_joomla_version() from /administrator/manifests/files/joomla.xml
      ├─ check admin panel (/administrator)
      └─ build_scan_result()
```

**Version comparison**: `version_lt()` (`cms/mod.rs:315`) performs component-wise numeric comparison after stripping non-digit suffixes. Used by `build_vulnerabilities()` to check if a detected version is below the `fixed_in_version` threshold for each CVE.

### Template Engine Execution Flow

```
TemplateEngine::scan(target)                [executor.rs:290]
  └─ TemplateExecutor::execute_on_target()  [executor.rs:57]
      ├─ TemplateLoader::load_all()          [loader.rs:131]
      │   └─ for each template_dir:
      │       └─ load_from_directory() (recursive, YAML/JSON, validates each)
      └─ for each template:
          └─ execute_template()              [executor.rs:69]
              ├─ for each template.requests:
              │   └─ send_request()          [executor.rs:118]
              │       ├─ construct URL (target + path)
              │       ├─ build HTTP request (GET/POST/PUT/DELETE/PATCH/HEAD/OPTIONS)
              │       ├─ apply headers + process_interactsh_variables()
              │       ├─ set body if present
              │       └─ send with timeout → HttpResponseData
              └─ for each response:
                  └─ TemplateMatcher::match_template()  [matcher.rs:53]
                      ├─ Matcher::Http → match_http():
                      │   ├─ check path match (or wildcard "*")
                      │   ├─ check headers
                      │   ├─ check status_codes
                      │   ├─ check search patterns (word/regex/binary)
                      │   └─ check interactsh callback URLs
                      └─ Matcher::Dns → match_dns():
                          ├─ check query_type
                          └─ check search patterns
```

**Regex caching**: `REGEX_CACHE` (`matcher.rs:15`) is a `LazyLock<DashMap<String, Arc<Regex>>>` with a `RegexBuilder` size limit of 100,000 bytes. Cache hits avoid recompilation; misses compile outside the lock and insert.

**Interactsh support**: Templates can use `{{interactsh-url}}` placeholders in headers/body. `process_interactsh_variables()` (`executor.rs:186`) substitutes the first configured Interactsh URL. Callback detection is handled by `TemplateMatcher` checking if any Interactsh URL appears in the response body.

**Template signing**: `TemplateSigner` (`verify.rs:33`) generates Ed25519 keypairs and signs serialized YAML templates. `TemplateVerifier` (`verify.rs:114`) verifies `SignedTemplate` envelopes. The marketplace (`marketplace.rs:34`) optionally verifies signatures on download and validates template IDs against path traversal (`/`, `\`, `..`, `\0`).

## Public API

### Core Scan Functions

```rust
// Port scanning
pub async fn scan_ports(host: &str, config: PortScanConfig) -> Result<PortScanResults>;
// ports/mod.rs:531

// Endpoint discovery
pub async fn scan_endpoints(config: EndpointScanConfig) -> Result<EndpointScanResults>;
// endpoints.rs:992

// Service fingerprinting (TCP)
pub async fn fingerprint_services(
    host: &str, ports: Vec<u16>, timeout_duration: Duration,
    tui_mode: bool, concurrency: usize,
    progress_tx: Option<Sender<(u64, u64)>>, max_results: Option<usize>,
) -> Result<FingerprintResults>;
// fingerprint.rs:316

// ICMP host discovery (feature-gated: stress-testing)
pub async fn ping_host(target: &str, count: u32, timeout: Duration, interval: Duration)
    -> Result<(Vec<PingResult>, PingStats)>;
// icmp_probe.rs:29

// UDP fingerprinting
pub async fn fingerprint_udp_services(
    host: &str, ports: Vec<u16>, timeout_duration: Duration,
) -> Result<UdpFingerprintResults>;
// udp_fingerprint.rs:111
```

### CLI Wrappers (feature-gated: cli)

```rust
pub async fn run_cli(args: PortScanArgs, config: &EggsecConfig) -> Result<()>;           // ports/mod.rs:169
pub async fn run_cli(args: EndpointScanArgs, config: &EggsecConfig) -> Result<()>;       // endpoints.rs:744
pub async fn run_cli(args: FingerprintArgs, config: &EggsecConfig) -> Result<()>;        // fingerprint.rs:182
```

### Tool-API Callbacks (feature-gated: tool-api)

```rust
pub async fn run_with_callback<F>(request: &PortScanRequest, config: &EggsecConfig, cb: F) -> Result<PortScanResults>;
pub async fn run_with_callback<F>(request: &EndpointScanRequest, config: &EggsecConfig, cb: F) -> Result<EndpointScanResults>;
pub async fn run_with_callback<F>(request: &FingerprintRequest, config: &EggsecConfig, cb: F) -> Result<FingerprintResults>;
```

### CMS Scanning

```rust
impl CmsScanner {
    pub fn new() -> Result<Self>;                                    // cms/mod.rs:92
    pub fn new_insecure() -> Result<Self>;                           // cms/mod.rs:96
    pub async fn detect_cms(&self, url: &str) -> Result<CmsTarget>; // cms/mod.rs:183
    pub async fn scan(&self, target: &CmsTarget) -> Result<CmsScanResult>; // cms/mod.rs:291
}
```

### Template Engine

```rust
impl TemplateEngine {
    pub fn new(executor: TemplateExecutor) -> Self;                  // executor.rs:284
    pub async fn scan(&self, target: &str) -> Result<Vec<TemplateExecutionResult>>;  // executor.rs:290
    pub async fn scan_with_callback<F>(&self, target: &str, cb: F) -> Result<()>;   // executor.rs:294
}
```

## Integration Points

- **Dispatch**: Scanner operations are invoked by `ScannerExecutor` in `dispatch/executors/`, which calls `scan_ports()`, `scan_endpoints()`, `fingerprint_services()` directly.
- **CLI handlers**: `commands/handlers/port_scan.rs`, `endpoint_scan.rs`, `fingerprint.rs` convert CLI args → engine request types.
- **Pipeline stages**: `pipeline/` orchestrates scanner stages as part of `ScanProfile` execution (e.g., Quick scan runs port scan → fingerprint).
- **Python bindings**: `eggsec-python` exposes port scan, endpoint scan, and fingerprint as stable-core operations.
- **NSE integration**: NSE scripts can trigger scanner operations via the tool registry.
- **TUI**: Progress updates flow via `progress_tx` channel to the TUI progress bar.

## Configuration Types

| Config | Default | Key Parameters |
|--------|---------|----------------|
| `PortScanConfig` | concurrency=100, timeout=3s | `ports`, `concurrency`, `timeout_duration`, `spoof_config`, `max_results` |
| `EndpointScanConfig` | concurrency=20, timeout=10s | `base_url`, `endpoints`, `concurrency`, `verify_tls`, `include_404` |
| `TimingConfig` (Normal) | parallelism=30–100, timeout=15s, rate=200pps | `min_parallelism`, `max_parallelism`, `timeout_ms`, `max_rate`, `port_batch_size` |
| `SpoofConfig` | disabled | `source_ip`, `decoy_ips`, `decoy_mode`, `scan_type`, `fragment`, `ttl`, `max_rate` |
| `RetryConfig` | max_retries=3, initial_backoff=100ms | `backoff_multiplier` (2.0), `max_backoff_ms` (5000) |

### Timing Presets (T0–T5)

| Preset | Parallelism | Timeout | Rate (pps) | Batch | Scan Delay |
|--------|-------------|---------|------------|-------|------------|
| T0 Paranoid | 1–5 | 300s | 1 | 1 | 1000ms |
| T1 Sneaky | 5–15 | 30s | 10 | 5 | 200ms |
| T2 Polite | 10–30 | 15s | 50 | 10 | 100ms |
| T3 Normal | 30–100 | 15s | 200 | 25 | 50ms |
| T4 Aggressive | 100–300 | 8s | 1000 | 50 | 10ms |
| T5 Insane | 300–1000 | 3s | ∞ | 100 | 0ms |

`TimingPreset::parse()` (`timing.rs:14`) accepts `"t0"`–`"t5"`, `"paranoid"`–`"insane"`, or `"0"`–`"5"`. Unknown strings default to `Normal`.

## Testing

- **Unit tests**: Every scanner file includes `#[cfg(test)] mod tests` with serialization round-trips, display formatting, edge cases (empty results, unknown ports, empty patterns), and property-based tests (`proptest` in `spoof.rs` for CIDR IP generation).
- **Endpoint tests**: Verify `is_interesting()` against sensitive/insensitive paths, status code gating (200/403/401), path traversal rejection, serialization.
- **Fingerprint tests**: Verify `hex_match()` (exact/offset/not-found/empty), `extract_banner()` (truncation, multiline), `extract_product_version()` (HTTP/SSH/unknown), PROBES non-empty.
- **Wordlist tests**: Verify parsing (comments, empty lines, normalization), validation (spaces, control chars, traversal, length), `into_endpoints()`.
- **Template tests**: Loader validation (empty ID, traversal ID, invalid severity, empty search patterns), matcher behavior (word/regex/binary, interactsh, status-only), signing/verify round-trip, marketplace ID validation.
- **CMS tests**: Version comparison (`version_lt`), CMS type string, scanner creation.
- **Feature-gated tests**: ICMP tests run only with `stress-testing`; spoofed scan tests only with `stress-testing` + Unix.

## Key Design Patterns

| Pattern | Usage |
|---------|-------|
| `DashMap` | Lock-free concurrent result collection in port scan, endpoint scan, fingerprint |
| `tokio::sync::Semaphore` | Concurrency control for parallel port/endpoint/fingerprint workers |
| `tokio::time::timeout(300s)` | Every spawned task carries a 300s timeout wrapper (project invariant) |
| `Arc::try_unwrap` + `map_err` | Safe error handling when collecting parallel results |
| `LazyLock` | Static initialization for WP_VERSION_PATTERNS, REGEX_CACHE |
| `FxHashMap` | High-performance hash maps in template matcher headers |
| `rustc_hash` | FxHashMap/FxHashSet in performance paths |
| Feature gating | ICMP and raw socket features gated behind `#[cfg(all(feature = "stress-testing", unix))]` |
| Blocking pool offload | Spoofed scan uses `tokio::task::spawn_blocking` for pnet datalink sends |

## Notable Bug Fixes

| File | Issue | Fix |
|------|-------|-----|
| `ports/spoofed.rs:288-295` | Fragmented packets never populated `sent_packets` map, causing all responses to be silently dropped | Added `sent_packets.insert()` after sending fragments |
| `spoof.rs:126` | `max_rate=0` caused division by zero panic in spoofed scan rate limiting | Added validation: `max_rate` must be > 0 |
| `templates/marketplace.rs:176` | `template_id` path traversal via unsanitized IDs | Added validation rejecting `/`, `\`, `..` in template IDs |
| `udp_fingerprint.rs:301-320` | `TokenBucket` race condition in refill (non-atomic read-modify-write) | Refactored to use `compare_exchange` loop in `refill()` |
| `spoof.rs:432` | `build_fragmented_packets` over-allocated buffer causing trailing zeros on wire for last fragment | Changed to `vec![0u8; 20 + chunk.len()]` for exact per-fragment sizing |

## Invariants & Gotchas

1. **Policy-free executors**: Scanner modules never touch `EnforcementContext`. All authorization is upstream in dispatch.
2. **Timeout wrapper invariant**: Every `tokio::spawn` in scanner code carries a `tokio::time::timeout(300s)` wrapper. Stuck sends/receives cannot leak forever.
3. **MAX_SCAN_RESULTS differs**: Port scanning caps at 10,000 (`ports/mod.rs:29`); endpoint scanning caps at 100,000 (`endpoints.rs:21`). Both truncate silently.
4. **Concurrency = 0 rejected**: `scan_ports()`, `scan_endpoints()`, and `fingerprint_services()` all return an error for `concurrency == 0`.
5. **ICMP/raw sockets require both `stress-testing` feature AND Unix**: `icmp_probe.rs` is `#![cfg(feature = "stress-testing")]`; spoofed scanning is `#[cfg(all(feature = "stress-testing", unix))]`.
6. **Endpoint path traversal blocked**: `join_endpoint_url()` (`endpoints.rs:621`) rejects `.` and `..` path components. `Wordlist` parser also rejects traversal.
7. **Template ID traversal blocked**: Both `TemplateLoader::validate_template()` (`loader.rs:48`) and `validate_template_id()` (`marketplace.rs:42`) reject `/`, `\`, `..`, and `\0` in template IDs.
8. **CMS version compare is numeric-only**: `version_lt()` (`cms/mod.rs:315`) strips non-digit suffixes and compares component-wise. Pre-release suffixes are ignored.
9. **Regex cache unbounded**: `REGEX_CACHE` (`matcher.rs:15`) is a global `DashMap` with no eviction. Templates with many unique regex patterns will grow memory monotonically.
10. **Spoofed scan is IPv4-only**: `scan_ports_spoofed()` rejects IPv6 targets (`ports/spoofed.rs:134`).
11. **Endpoint scan bypasses spoof config for HTTP**: Endpoint spoofing uses `X-Forwarded-For`/`X-Real-IP`/`X-Originating-IP` headers only, not raw socket spoofing.
12. **ICMP uses surge_ping (not raw sockets)**: `icmp_probe.rs` uses the `surge_ping` crate which handles ICMP socket creation; no raw socket feature needed beyond the `stress-testing` gate.

## Cross-Links

- [overview.md](overview.md) — system architecture, module index
- [probe.md](probe.md) — shared probe intent/risk vocabulary used by scanner stages
- [recon.md](recon.md) — reconnaissance module (upstream of scanner in pipeline)
- [stress.md](stress.md) — stress testing (shares `stress-testing` feature gate)
- [dispatch.md](dispatch.md) — task dispatch and executor layer
- [config.md](config.md) — enforcement model, scope evaluation

---

*Last verified against source: 2026-08-25*
