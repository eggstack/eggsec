# WAF Module

The WAF (Web Application Firewall) module is dedicated to detecting, fingerprinting, and bypassing security filters in front of web applications.

## Core Components (`src/waf/`)

```
waf/
├── mod.rs                    # WafEngine, public exports, run_cli()
├── types.rs                  # OwaspCategory, Finding, ScanResults, ScanSummary
├── output.rs                 # Text/JSON output formatting
├── waf_patterns.rs           # Pattern utilities (re-exports data)
├── AGENTS.override.md        # Module-specific guidance
├── bypass/
│   ├── mod.rs                # BypassEngine, BypassResult, BypassTechnique, TestType
│   ├── headers.rs            # Header manipulation bypass techniques
│   ├── evasion.rs            # Payload evasion/obfuscation techniques
│   ├── smuggling.rs          # HTTP desync/smuggling attack techniques
│   └── profiles.rs           # WAF-specific profiles (Cloudflare, Akamai, AWS, etc.)
├── data/
│   ├── mod.rs                # Re-exports patterns
│   └── patterns.rs           # WafSignature definitions for 34 WAF products
├── detector/
│   ├── mod.rs                # WafDetector struct
│   ├── detect.rs             # WAF detection logic via HTTP response analysis
│   ├── types.rs              # WafDetectionResult, ResponseDiff, WafSignatureLower
│   ├── compare.rs            # Response comparison for baseline/differential analysis
│   ├── block_check.rs        # check_waf_block() method
│   └── tests.rs              # Unit tests
└── payloads/
    ├── mod.rs                # Module declaration
    └── encoding.rs           # Payload sets (SQLi, XSS, SSRF, cmd injection, traversal)
```

### Key Data Structures

| Type | Location | Purpose |
|------|----------|---------|
| `WafDetector` | `detector/mod.rs` | Sends probes, scores matches against 34 WAF signatures |
| `WafDetectionResult` | `detector/types.rs` | Returns detected WAF name, confidence (0-100), matched indicators |
| `WafSignature` | `data/patterns.rs` | Header, cookie, body pattern, and IP range signatures for a WAF |
| `WafEngine` | `mod.rs` | High-level orchestrator for detection + bypass |
| `BypassEngine` | `bypass/mod.rs` | Orchestrates bypass testing across five categories |
| `BypassResult` | `bypass/mod.rs` | Reports technique, success status, payload, status code |
| `BypassTechnique` | `bypass/mod.rs` | Enum of 15 bypass techniques |
| `WafProfile` | `bypass/profiles.rs` | WAF-specific bypass configurations |
| `ResponseDiff` | `detector/types.rs` | Baseline vs malicious response comparison |

### Detection (`detector/`)

Slapper can identify **34 different WAF products** by analyzing HTTP responses for specific headers, cookies, body patterns, and IP ranges.

- **WAF Patterns (`data/patterns.rs`)**: A collection of signatures for well-known WAFs stored in a `FxHashMap<String, WafSignature>` (keys are lowercase names)
- **Detector Logic**: Orchestrates probes to trigger WAF responses and matches them against known patterns.
- **Scoring System** (uses `u16` internally to prevent overflow):
  - Header match: +25 points
  - Cookie match: +20 points
  - Body pattern match: +15 points
  - Remote IP match: +20 points (IP in known WAF IP range)
  - High confidence exit: 90 points

### Bypass (`bypass/`)

`get_waf_profiles()` and `get_profile_by_name()` in `profiles.rs` use a static `LazyLock<Vec<WafProfile>>` to cache profiles and avoid recreation on every call.

Once a WAF is identified, Slapper can apply specialized bypass techniques across five categories:

- **Encodings**: Using different character encodings (e.g., URL, Double URL, Unicode, Hex) to evade simple pattern matching.
- **Header Manipulation**: Injecting or modifying headers (e.g., `X-Forwarded-For`, `User-Agent`) that might influence WAF behavior.
- **Payload Splitting**: Dividing payloads into multiple parts to bypass length restrictions or inspection limits.
- **Protocol Obfuscation**: Using HTTP/2 or other protocol features to hide malicious intent.
- **HTTP Smuggling**: Raw TCP/TLS attacks via `smuggling.rs` (CL.TE, TE.CL, chunked malformed)

**Bypass Success Detection**: The `is_bypass_successful()` function verifies:
1. Response status NOT in blocked codes (403, 406, 429, 503)
2. Response status differs from baseline
3. Response status is 2xx (200-299)
4. Payload (or URL-encoded version) is reflected in response body

### Payloads (`payloads/`)

The WAF module includes a set of "benign" and "malicious" probes specifically designed to test WAF sensitivity without necessarily triggering a block:
- `get_sqli_payloads()` - 19 SQL injection payloads
- `get_xss_payloads()` - 17 XSS payloads
- `get_ssrf_payloads()` - 16 SSRF payloads
- `get_command_injection_payloads()` - 16 cmd injection payloads
- `get_traversal_payloads()` - 10 path traversal payloads

## Integration

The WAF module is used by both the **Scanner** (during the discovery phase) and the **Fuzzer** (to ensure payloads are delivered successfully). The **AI** module can also suggest advanced bypasses based on the detected WAF via `SmartWafBypass`.

## Supported WAFs (34 products)

Cloudflare, Akamai, AWS WAF, Azure WAF, Google Cloud Armor, Fastly, Imperva, Sucuri, CloudFront, F5 BIG-IP, Barracuda, Fortinet, Citrix NetScaler, ModSecurity, Wordfence, DataDome, PerimeterX, Nginx, Traefik, Kong, Varnish, Radware, Signal Sciences, Wallarm, Reblaze, F5 BIG-IP Advanced WAF, Palo Alto, Qrator, Imunify360, SiteGuard, StackPath WAF, Humanity, Datadog, Generic WAF Block
