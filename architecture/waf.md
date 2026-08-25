# WAF Module

Deep dive: Web Application Firewall detection, fingerprinting, block-page comparison, and bypass library.

Parent overview: [overview.md](overview.md). Related: [fuzzer.md](fuzzer.md), [constants.md](constants.md), [dispatch.md](dispatch.md).

## Role & Responsibilities

The WAF module detects web application firewalls in front of target applications, compares response baselines to identify blocking behavior, and attempts bypass techniques across multiple categories. It is the only always-compiled module (no feature gate) dedicated to WAF interaction.

## Location & Feature Gating

| Item | Value |
|------|-------|
| Crate | `eggsec` (engine lib) |
| Root | `crates/eggsec/src/waf/` |
| Files | 20 `.rs` files (see tree below) |
| Feature gate | **None** — always compiled |
| MSRV | 1.88 (workspace-level) |

```
waf/
├── mod.rs                        # WafEngine orchestrator, run_cli(), public re-exports
├── types.rs                      # OwaspCategory (21 variants), Finding, ScanResults, ScanSummary
├── output.rs                     # Text/JSON formatting for detection + bypass results
├── waf_patterns.rs               # Re-exports data; get_common_waf_response_patterns() (13 patterns)
├── regression_report.rs          # WafBehavior, WafRegressionCase, WafRegressionReport
├── AGENTS.override.md            # Module-specific guidance
├── bypass/
│   ├── mod.rs                    # BypassEngine, BypassResult, BypassTechnique (15 variants), is_bypass_successful()
│   ├── headers.rs                # HeaderBypass: 16 user-agents, XFF spoofing, Content-Type/Encoding, method override
│   ├── evasion.rs                # EvasionBypass: case rotation, homoglyphs, zero-width, comments, whitespace, unicode, double encoding
│   ├── smuggling.rs              # SmugglingBypass: 8 SmugglingType variants, raw TCP/TLS HTTP/1.1 probes
│   └── profiles.rs               # 8 hardcoded profiles + auto-generated profiles; WafProfile, ProfileBypass
├── data/
│   ├── mod.rs                    # Re-exports patterns
│   └── patterns.rs               # WafSignature for 34 WAF products (LazyLock<FxHashMap>)
├── detector/
│   ├── mod.rs                    # WafDetector: client, signatures, circuit breaker
│   ├── detect.rs                 # detect() — probe + score against 34 signatures
│   ├── types.rs                  # WafDetectionResult, WafSignatureLower, ResponseDiff
│   ├── compare.rs                # compare_responses() — baseline vs malicious differential
│   ├── block_check.rs            # check_waf_block() — quick block detection
│   └── tests.rs                  # ResponseDiff unit tests
└── payloads/
    ├── mod.rs                    # Module declaration
    └── encoding.rs               # Payload sets: SQLi (19), XSS (17), SSRF (16), cmd (16), traversal (10), WafPayload (7 structured)
```

## Architecture

### Signature Inventory (34 WAF products)

All signatures defined in `data/patterns.rs` inside a `LazyLock<FxHashMap<String, WafSignature>>` (keys lowercase). Count validated at compile time by `SUPPORTED_WAF_COUNT = 34` in `eggsec-core/src/constants.rs:39` and asserted in `crates/eggsec/src/constants.rs:19`.

| # | Key | Display Name | Headers | Cookies | Body Patterns | IP Ranges |
|---|-----|--------------|---------|---------|---------------|-----------|
| 1 | `cloudflare` | Cloudflare | 7 | 1 | 2 | 14 |
| 2 | `akamai` | Akamai | 6 | 0 | 1 | 11 |
| 3 | `aws_waf` | AWS WAF | 6 | 0 | 2 | 8 |
| 4 | `azure_waf` | Azure WAF | 4 | 0 | 2 | 4 |
| 5 | `google_cloud_armor` | Google Cloud Armor | 4 | 0 | 1 | 6 |
| 6 | `fastly` | Fastly | 6 | 0 | 1 | 6 |
| 7 | `imperva` | Imperva | 4 | 2 | 2 | 4 |
| 8 | `sucuri` | Sucuri | 5 | 0 | 1 | 4 |
| 9 | `cloudfront` | CloudFront | 3 | 0 | 1 | 10 |
| 10 | `f5_big_ip` | F5 BIG-IP | 4 | 1 | 2 | 0 |
| 11 | `barracuda` | Barracuda | 4 | 1 | 1 | 0 |
| 12 | `fortinet` | Fortinet | 3 | 0 | 2 | 0 |
| 13 | `citrix_netscaler` | Citrix NetScaler | 4 | 2 | 2 | 0 |
| 14 | `modsecurity` | ModSecurity | 2 | 0 | 4 | 0 |
| 15 | `wordfence` | Wordfence | 3 | 1 | 1 | 0 |
| 16 | `datadome` | DataDome | 3 | 2 | 1 | 3 |
| 17 | `perimeterx` | PerimeterX | 3 | 1 | 1 | 3 |
| 18 | `nginx` | Nginx | 3 | 0 | 2 | 0 |
| 19 | `traefik` | Traefik | 2 | 0 | 1 | 0 |
| 20 | `kong` | Kong | 3 | 0 | 1 | 0 |
| 21 | `varnish` | Varnish | 2 | 0 | 1 | 0 |
| 22 | `radware_waf` | Radware | 3 | 0 | 1 | 0 |
| 23 | `signal_sciences` | Signal Sciences | 3 | 0 | 1 | 3 |
| 24 | `wallarm_waf` | Wallarm | 3 | 0 | 1 | 3 |
| 25 | `reblaze` | Reblaze | 2 | 0 | 1 | 2 |
| 26 | `f5_bigip_asm` | F5 BIG-IP Advanced WAF | 5 | 0 | 5 | 4 |
| 27 | `palo_alto` | Palo Alto | 4 | 0 | 4 | 2 |
| 28 | `qrator` | Qrator | 2 | 1 | 2 | 3 |
| 29 | `imunify360` | Imunify360 | 2 | 0 | 5 | 0 |
| 30 | `siteguard` | SiteGuard | 2 | 0 | 2 | 0 |
| 31 | `stackpath_waf` | StackPath WAF | 4 | 0 | 2 | 3 |
| 32 | `humanity` | Humanity | 2 | 2 | 3 | 2 |
| 33 | `datadog` | Datadog | 2 | 0 | 2 | 1 |
| 34 | `denied_by_waf` | Generic WAF Block | 0 | 0 | 10 | 0 |

### Key Types

| Type | File | Line | Purpose |
|------|------|------|---------|
| `WafSignature` | `data/patterns.rs` | 4 | Header/cookie/body/ip signature for a WAF product |
| `WafSignatureLower` | `detector/types.rs` | 23 | Pre-lowered version for case-insensitive matching |
| `WafDetector` | `detector/mod.rs` | 21 | HTTP client + signatures + circuit breaker; `detect()`, `compare_responses()`, `check_waf_block()` |
| `WafDetectionResult` | `detector/types.rs` | 5 | Detection output: waf_name, confidence (0–100), matched indicators, status_code |
| `ResponseDiff` | `detector/types.rs` | 29 | Baseline vs malicious comparison: status, length, headers, body_diffs |
| `WafEngine` | `mod.rs` | 115 | Top-level orchestrator: detect → profile select → bypass → output |
| `BypassEngine` | `bypass/mod.rs` | 73 | Dispatches to HeaderBypass, EvasionBypass, SmugglingBypass |
| `BypassResult` | `bypass/mod.rs` | 62 | Per-technique result: success, payload, status_code, error |
| `BypassTechnique` | `bypass/mod.rs` | 43 | 15-variant enum of bypass technique categories |
| `TestType` | `bypass/mod.rs` | 19 | Payload family selector: All, Sql, Xss, Ssrf, Cmd, Traversal |
| `HeaderBypass` | `bypass/headers.rs` | 9 | Header-based bypass: UA rotation, XFF spoof, Content-Type, encoding, method override |
| `EvasionBypass` | `bypass/evasion.rs` | 13 | Payload obfuscation: case rotation, homoglyphs, zero-width, comments, whitespace, unicode, double encoding |
| `SmugglingBypass` | `bypass/smuggling.rs` | 22 | Raw TCP/TLS smuggling: CL.TE, TE.CL, chunked, tunneling, H2C, double CL, multipart |
| `SmugglingType` | `bypass/smuggling.rs` | 27 | 8-variant enum of HTTP smuggling techniques |
| `WafProfile` | `bypass/profiles.rs` | 8 | Named profile with detection_signatures + Vec<ProfileBypass> |
| `ProfileBypass` | `bypass/profiles.rs` | 15 | Single bypass: technique, headers, payloads, description |
| `OwaspCategory` | `types.rs` | 7 | OWASP Top 10 2021 (10 variants) + API Top 10 2023 (11 variants) = 21 total |
| `Finding` | `types.rs` | 138 | Bypass finding with OWASP category, severity, technique, payload |
| `ScanResults` | `types.rs` | 183 | Full scan output: detection + findings + summary |
| `WafBehavior` | `regression_report.rs` | 4 | Regression behavior: Blocked, Allowed, Challenged, Tarpitted, Errored, Skipped |
| `WafRegressionCase` | `regression_report.rs` | 28 | Individual regression test case with baseline comparison |
| `WafRegressionReport` | `regression_report.rs` | 104 | Full regression report: cases, summary, baseline_id, termination_reason |
| `WafPayload` | `payloads/encoding.rs` | 3 | Structured payload with name, description, and bypass_types |
| `BypassType` | `payloads/encoding.rs` | 11 | Payload classification: SqlInjection, Xss, CommandInjection, PathTraversal, Ssrf |

## Detection Flow

### `WafDetector::detect()` (`detector/detect.rs:13`)

1. **Circuit breaker check** — returns empty result if circuit is open (`detector/detect.rs:14`).
2. **URL normalization** — `normalize_url_static()` prepends `https://` if no scheme (`detector/detect.rs:204`).
3. **Single GET request** — uses `create_insecure_client_with_options()` with `SMUGGLING_TIMEOUT_SECS` (15s) timeout and same-host redirect policy (`detector/mod.rs:31`).
4. **Signature matching** — iterates all 34 signatures, scoring each:
   - Header name or value match: **+25** (`constants::waf::HEADER_MATCH_SCORE`)
   - Cookie name match: **+20** (`constants::waf::COOKIE_MATCH_SCORE`)
   - Body pattern (substring): **+15** (`constants::waf::BODY_MATCH_SCORE`)
   - Remote IP in CIDR range: **+20** (`constants::waf::IP_MATCH_SCORE`)
   - Header value capped at 256 chars (`HEADER_VALUE_MAX_LEN`, `detect.rs:10`)
5. **Early exit** — score ≥ 90 (`HIGH_CONFIDENCE_EXIT`) breaks immediately (`detect.rs:154`).
6. **Unknown WAF fallback** — if no match, scans `get_common_waf_response_patterns()` (13 patterns from `waf_patterns.rs:3`) plus `WEAK_BLOCK_INDICATOR_PATTERNS` (4 patterns from `constants`). Returns "Unknown WAF" at confidence 30 if ≥ 2 weak indicators match (`constants::waf::UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD`).
7. **Result** — `WafDetectionResult` with confidence clamped to 0–100 (`detect.rs:186`).

### `check_waf_block()` (`detector/block_check.rs:9`)

Quick block detection: appends URL-encoded payload as `?test=` param, checks status against `BLOCKED_STATUS_CODES` (403, 406, 429, 503) and body against `BLOCKED_PATTERNS` (8 patterns). Timeout: `SMUGGLING_TIMEOUT_SECS`.

### `compare_responses()` (`detector/compare.rs:8`)

Differential analysis: sends two GET requests (normal and malicious query param), builds `ResponseDiff` with status, length, headers, body_diffs. `ResponseDiff::is_waf_blocked()` (`detector/types.rs:42`) returns true if:
- Status changed AND is in `BLOCKED_STATUS_CODES`
- Length difference > `LENGTH_DIFF_THRESHOLD` (100 bytes)
- Any header diff contains "waf", "firewall", "blocked", or "attack" (case-insensitive)

## Bypass Library

### Technique Taxonomy (15 `BypassTechnique` variants)

| Technique | Category | Engine | Description |
|-----------|----------|--------|-------------|
| `HeaderManipulation` | Headers | HeaderBypass/SmugglingBypass | Generic header injection |
| `UserAgentRotation` | Headers | HeaderBypass | 16 user-agent strings including bots |
| `XForwardedForSpoof` | Headers | HeaderBypass | 18 XFF/originating IP variants (incl. hex, octal, decimal) |
| `ContentTypeBypass` | Headers | HeaderBypass | 7 Content-Type values |
| `EncodingBypass` | Evasion | EvasionBypass | Accept-Encoding variations, double encoding |
| `Homoglyph` | Evasion | EvasionBypass | Cyrillic lookalike substitution (17 chars mapped) |
| `ZeroWidthInjection` | Evasion | EvasionBypass | U+200B/200C/200D/FEFF insertion |
| `CaseRotation` | Evasion | EvasionBypass | Alternating upper/lower case |
| `UnicodeEncoding` | Evasion | EvasionBypass | `\u00xx` escape sequences |
| `CommentObfuscation` | Evasion | EvasionBypass | `/**/` inline comment insertion between keywords |
| `WhitespaceVariation` | Evasion | EvasionBypass | Unicode whitespace substitution (NBSP, OGHAM SPACE, etc.) |
| `DoubleEncoding` | Evasion | EvasionBypass | Double URL encoding (`%2527`) |
| `ChunkedEncoding` | Smuggling | SmugglingBypass | Malformed chunked transfer encoding |
| `ContentLengthConflict` | Smuggling | SmugglingBypass | CL.TE / double Content-Length |
| `TransferEncodingConflict` | Smuggling | SmugglingBypass | TE.CL / space-prefixed TE |

### Header Bypass (`bypass/headers.rs`)

`HeaderBypass` generates sets of headers to test:
- **User-Agent rotation**: 16 strings (browsers, bots, curl, wget, python-requests) (`headers.rs:260`)
- **X-Forwarded-For spoofing**: 18 IP representations (IPv4, IPv6, hex, octal, decimal, DNS) (`headers.rs:288`)
- **Content-Type bypass**: 7 values (`headers.rs:311`)
- **Accept-Encoding bypass**: 6 values (`headers.rs:323`)
- **Method override**: X-HTTP-Method-Override, X-Method-Override
- **URL spoof**: X-Original-URL, X-Rewrite-URL
- **CDN headers**: CF-Connecting-IP, True-Client-IP, X-Forwarded-Host
- **Cache bypass**: Cache-Control: no-cache, Pragma: no-cache

Default probe payload: `' OR 1=1--` (`headers.rs:13`).

### Evasion Bypass (`bypass/evasion.rs`)

Transforms payloads using 7 obfuscation techniques:
- `apply_case_rotation()` — alternating upper/lower (`evasion.rs:296`)
- `apply_homoglyphs()` — Cyrillic substitution for 17 Latin chars (`evasion.rs:310`)
- `apply_zero_width()` — random zero-width char insertion at 1/3 probability (`evasion.rs:338`)
- `apply_comment_obfuscation()` — `/**/` between keyword characters (`evasion.rs:354`)
- `apply_whitespace_variation()` — random Unicode whitespace for spaces (`evasion.rs:374`)
- `apply_unicode_encoding()` — `\u00xx` escape for ASCII alphanumeric (`evasion.rs:392`)
- `apply_double_encoding()` — double URL encoding (`evasion.rs:405`)

Each evasion is applied to payloads from `get_sqli_payloads()` (all 19), `get_xss_payloads()` (first 3), `get_ssrf_payloads()` (first 3), `get_traversal_payloads()` (first 5), `get_command_injection_payloads()` (first 5), plus all 7 `get_waf_test_payloads()`. Timeout: `SMUGGLING_TIMEOUT_SECS` per request (`evasion.rs:251`).

### Smuggling Bypass (`bypass/smuggling.rs`)

Raw TCP/TLS HTTP/1.1 probes — does not use the `reqwest` client. 8 `SmugglingType` variants, 11 generated requests:

| # | Type | Description | File:Line |
|---|------|-------------|-----------|
| 1 | `ClTe` | CL vs TE: Content-Length covers only chunk terminator | `smuggling.rs:102` |
| 2 | `TeCl` | TE: chunked encoding test | `smuggling.rs:117` |
| 3 | `ChunkedMalformed` | Small chunks (1-byte) | `smuggling.rs:126` |
| 4 | `ClTe` | CL: Incomplete body (`0\r\n\r\nG`) | `smuggling.rs:135` |
| 5 | `TeCl` | TE: Malformed `xchunked` | `smuggling.rs:144` |
| 6 | `TeCl` | TE: Space prefix in header | `smuggling.rs:153` |
| 7 | `ClTe` | Method override smuggling | `smuggling.rs:162` |
| 8 | `DoubleContentLength` | Duplicate Content-Length headers | `smuggling.rs:182` |
| 9 | `RequestTunneling` | Full HTTP request in body | `smuggling.rs:194` |
| 10 | `MultipartMixed` | Multipart method override | `smuggling.rs:207` |
| 11 | `H2CUpgrade` | HTTP/2 cleartext upgrade (**disabled**: `supports_http2_probes()` returns `false`, `smuggling.rs:321`) | `smuggling.rs:219` |

Additional chunked variants: chunked with smuggled request in body (`smuggling.rs:237`), invalid chunk size prefix (`smuggling.rs:247`), TE.CL with trailing headers (`smuggling.rs:258`).

TLS uses `rustls` with `webpki_roots` — ring-only provider (`smuggling.rs:415`). `H2CUpgrade` and `Http2Frame` variants exist in the enum but are currently dead code (`#[allow(dead_code)]` at `smuggling.rs:32-34`).

### Bypass Profiles (`bypass/profiles.rs`)

8 hardcoded WAF-specific profiles + auto-generated fallback profiles:

| Profile | Detection Signatures | Bypass Techniques | File:Line |
|---------|---------------------|-------------------|-----------|
| Cloudflare | cf-ray, cf-cache-status, cloudflare, __cfduid | HeaderManipulation, UserAgentRotation, EncodingBypass, CommentObfuscation, CaseRotation, ContentTypeBypass | `profiles.rs:66` |
| Akamai | akamai, x-akamai-transformed, akamaiedge | HeaderManipulation, EncodingBypass, Homoglyph, DoubleEncoding | `profiles.rs:131` |
| AWS WAF | awselb, x-amzn-requestid, x-amz-cf-id | HeaderManipulation, EncodingBypass, WhitespaceVariation | `profiles.rs:171` |
| Azure WAF | x-azure-ref, x-azure-origin, microsoft-azure | HeaderManipulation, EncodingBypass | `profiles.rs:211` |
| Imperva | x-cdn, x-iinfo, incapsula, incap_ses | HeaderManipulation, CommentObfuscation, UnicodeEncoding, ZeroWidthInjection | `profiles.rs:245` |
| F5 ASM | bigip, x-correlation-id, ts | HeaderManipulation, EncodingBypass, CaseRotation | `profiles.rs:290` |
| CloudFront | cloudfront, x-amz-cf-pop, x-cache | HeaderManipulation, EncodingBypass | `profiles.rs:327` |
| Sucuri | sucuri, x-sucuri, x-sucuri-id, x-sucuri-cache | HeaderManipulation, EncodingBypass | `profiles.rs:355` |

**Auto-generated profiles** (`profiles.rs:447`): For every WAF signature not covered by hardcoded profiles, a generic profile is built with XFF spoofing, user-agent rotation, double encoding, and comment obfuscation.

**Profile selection** (`mod.rs:147`): `WafEngine::select_profile()` matches by name (exact, prefix, suffix, substring) or falls back to `get_auto_profile()` (`profiles.rs:384`).

## Bypass Success Detection

`is_bypass_successful()` (`bypass/mod.rs:136`) performs a multi-point check:

1. Response body must not match `BLOCKED_PATTERNS` via `body_looks_blocked()` (`bypass/mod.rs:178`)
2. `ResponseDiff::is_waf_blocked()` must return `false` (if diff available)
3. Status must NOT be in `BLOCKED_STATUS_CODES` (403, 406, 429, 503)
4. Status must differ from baseline detection status
5. Status must be 2xx (200–299)
6. Payload (or URL-encoded form) must be reflected in response body (`payload_is_reflected()`)

For **empty payloads**: checks 1–5 only (block-to-non-block transition required, `bypass/mod.rs:160`).

## Payload Inventory

### `payloads/encoding.rs`

| Function | Count | Types |
|----------|-------|-------|
| `get_sqli_payloads()` | 19 | Union, boolean, time-based, error-based |
| `get_xss_payloads()` | 17 | Script tags, SVG, event handlers, JS URIs |
| `get_ssrf_payloads()` | 16 | Localhost, cloud metadata, file://, dict://, gopher:// |
| `get_command_injection_payloads()` | 16 | Semicolon, pipe, AND, OR, subshell, backtick |
| `get_traversal_payloads()` | 10 | Dot-dot-slash, encoded variants, null byte, semicolon |
| `get_waf_test_payloads()` | 7 | Structured `WafPayload` with `BypassType` tags |

## Regression Testing

`regression_report.rs` provides the data model for WAF regression tracking:

- **`WafBehavior`** (6 variants): Blocked, Allowed, Challenged, Tarpitted, Errored, Skipped
- **`WafRegressionCase`**: payload_family, payload_type, request_summary, status_code, behavior, response_time_ms, baseline_behavior, regression flag, confidence
- **`WafBehaviorSummary::from_cases()`** (`regression_report.rs:54`): aggregates counts and computes `regression_count` and `new_bypass_count` (Allowed when baseline was Blocked)
- **`WafRegressionReport`** (`regression_report.rs:104`): full report with target, profile, scope_file, baseline_id, budget_consumed, termination_reason
- **`to_human_readable()`** (`regression_report.rs:120`): human-readable text rendering

## Public API

### Module Re-exports (`mod.rs:89–98`)

```rust
pub use bypass::{get_auto_profile, get_profile_by_detection_sig, get_profile_by_name,
                 BypassEngine, BypassResult, TestType, WafProfile};
pub use detector::{WafDetectionResult, WafDetector};
pub use regression_report::{WafBehavior, WafBehaviorSummary, WafRegressionCase, WafRegressionReport};
pub use types::{Finding, OwaspCategory, ScanResults, Severity};
pub use waf_patterns::get_waf_signatures;
```

### `WafEngine` (`mod.rs:115`)

- `new(args: WafConfig) -> Result<Self>` — constructs detector with circuit breaker
- `run(&mut self) -> Result<()>` — full detect → profile → bypass → output pipeline
- `set_ai_bypass()` / `ai_bypass()` — AI bypass integration (behind `ai-integration` feature)

### `WafDetector`

- `new() -> Result<Self>` (`detector/mod.rs:29`) — reqwest client with `SMUGGLING_TIMEOUT_SECS` timeout, same-host redirect policy, random UA
- `detect(&self, url: &str) -> Result<WafDetectionResult>` (`detector/detect.rs:13`)
- `compare_responses(&self, url, normal_req, malicious_req) -> Result<ResponseDiff>` (`detector/compare.rs:8`)
- `check_waf_block(&self, url, test_payload) -> Result<bool>` (`detector/block_check.rs:9`)

### CLI Entry Point

`run_cli(args: WafArgs) -> Result<()>` (`mod.rs:110`) — behind `#[cfg(feature = "cli")]`.

## Integration Points

### Dispatch (`dispatch.md`)

`WafExecutor` handles three operation descriptors:

| Operation | Dispatcher | Source |
|-----------|-----------|--------|
| `waf-detect` | `fuzzer::run_waf` | `dispatch.md:69` |
| `waf-bypass` | `fuzzer::run_waf` | `dispatch.md:34` |
| `waf-stress` | `fuzzer::run_waf_stress` | `dispatch.md:70` |

### Fuzzer Interplay (bidirectional)

| Direction | Import | Location |
|-----------|--------|----------|
| Fuzzer → WAF | `waf::types::{OwaspCategory, Severity}` | `fuzzer/engine/types.rs:5`, `fuzzer/filters.rs:239`, `fuzzer/engine/execution.rs:7`, `fuzzer/engine/utils.rs:8` |
| Fuzzer → WAF constants | `crate::constants::waf::BLOCKED_STATUS_CODES`, `LENGTH_DIFF_THRESHOLD` | `fuzzer/engine/utils.rs:18`, `fuzzer/engine/utils.rs:165` |
| WAF → Fuzzer | `fuzzer::config::WafConfig` | `waf/mod.rs:86` |
| WAF → Fuzzer | `fuzzer::config::WafStressConfig` | (used by waf-stress dispatch) |

### CLI (`handle_waf` / `handle_waf_stress`)

- `handle_waf()` → `waf::run_cli()` or `fuzzer::run_waf()` depending on dispatch path
- `handle_waf_stress()` → `fuzzer::run_waf_stress()`
- CLI args: `WafArgs` (feature-gated `cli`) → converted to `WafConfig` via `From` impl (`fuzzer/config.rs:345`)

### Pipeline

- `Waf` and `WafRegression` profiles in `defense_lab.md` use the WAF module for regression validation
- Pipeline dispatch: `pipeline` depends on `waf` (`overview.md:510`)

### AI Integration (feature-gated)

`WafEngine` holds `Option<SmartWafBypass>` behind `#[cfg(feature = "ai-integration")]` (`mod.rs:120`). After standard bypasses, `run_ai_bypasses()` (`mod.rs:331`) queries the AI for suggestions on failed techniques.

## Testing

### Unit Tests

| File | Tests | Coverage |
|------|-------|----------|
| `detector/detect.rs` | 10 | URL normalization, IP CIDR matching, apply_remote_ip_match |
| `detector/tests.rs` | 12 | ResponseDiff: status blocking, length blocking, header keyword detection, case insensitivity, serialization |
| `bypass/mod.rs` | 8 | is_bypass_successful: blocked codes, status match, 2xx requirement, payload reflection, empty payload transition, body patterns, ResponseDiff |
| `bypass/smuggling.rs` | 2 | parse_status_code, extract_body |
| `output.rs` | 2 | format_detection formatting, request error display |
| `waf_patterns.rs` | 13 | Signature existence, header/cookie/body patterns, IP range format, name uniqueness, lowercase keys |

### Compile-Time Validation

`constants.rs:19`: `supported_waf_count_matches_actual()` asserts `get_waf_signatures().len() == SUPPORTED_WAF_COUNT` (34).

## Invariants & Gotchas

1. **Circuit breaker**: `WafDetector` uses `CircuitBreaker::default()` (`detector/mod.rs:56`). If too many requests fail, detection returns an empty result with `request_error: Some("Circuit breaker open")` (`detect.rs:18`).

2. **Single-request detection**: `detect()` sends exactly one GET request (`detect.rs:29`). It does not send multiple probes. The scoring runs against all 34 signatures on that single response.

3. **Smuggling uses raw sockets**: `SmugglingBypass` bypasses `reqwest` entirely, opening `TcpStream` connections directly (`smuggling.rs:340`). This means proxy settings, connection pooling, and cookie jars do not apply.

4. **HTTP/2 smuggling disabled**: `H2CUpgrade` and `Http2Frame` variants are dead code. `supports_http2_probes()` returns `false` (`smuggling.rs:321`).

5. **Profile auto-generation**: Any WAF signature not covered by the 8 hardcoded profiles gets a generic profile with XFF, UA, double encoding, and comment obfuscation (`profiles.rs:447`). This means all 34 WAFs have profiles.

6. **Header value cap**: Detection only matches header values ≤ 256 chars (`HEADER_VALUE_MAX_LEN`, `detect.rs:10`) to avoid false positives on large response bodies in header values.

7. **Smuggling timeout**: All raw TCP/TLS operations use `SMUGGLING_TIMEOUT_SECS` (15s) via `tokio::time::timeout` (`smuggling.rs:355`).

8. **`WafConfig` comes from fuzzer**: The WAF module imports `crate::fuzzer::config::WafConfig` (`mod.rs:86`), not a standalone WAF config type. This is a deliberate two-way type sharing between always-compiled modules.

9. **`is_bypass_successful()` is module-level**: The function lives in `bypass/mod.rs:136`, not on any struct. All three bypass engines delegate to it.

10. **Regression `new_bypass_count`**: Computed as cases where `behavior == Allowed && baseline_behavior == Some(Blocked)` (`regression_report.rs:84`). This specifically detects WAF rule removal/regression.

11. **Body patterns for unknown WAF**: `get_common_waf_response_patterns()` returns 13 generic patterns (`waf_patterns.rs:3`), separate from the `BLOCKED_PATTERNS` constant (8 patterns) used by block detection.

12. **`WEAK_BLOCK_INDICATOR_PATTERNS`**: 4 patterns ("security", "unauthorized", "suspicious", "rate limit") require ≥ 2 matches to trigger "Unknown WAF" detection (`constants::waf::UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD`).
