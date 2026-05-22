# WAF Module

The WAF (Web Application Firewall) module is dedicated to detecting, fingerprinting, and bypassing security filters in front of web applications.

## Core Components (`src/waf/`)

### Detection (`detector/`)

Slapper can identify **34 different WAF products** by analyzing HTTP responses for specific headers, cookies, body patterns, and IP ranges.

- **WAF Patterns (`waf_patterns.rs`)**: A collection of signatures for well-known WAFs like Cloudflare, Akamai, AWS WAF, etc.
- **Detector Logic**: Orchestrates probes to trigger WAF responses and matches them against known patterns.
- **Scoring System**:
  - Header match: +25 points
  - Cookie match: +20 points
  - Body pattern match: +15 points
  - Remote IP match: +20 points (IP in known WAF IP range)
  - High confidence exit: 90 points

### Bypass (`bypass/`)

Once a WAF is identified, Slapper can apply specialized bypass techniques.

- **Encodings**: Using different character encodings (e.g., URL, Double URL, Unicode, Hex) to evade simple pattern matching.
- **Header Manipulation**: Injecting or modifying headers (e.g., `X-Forwarded-For`, `User-Agent`) that might influence WAF behavior.
- **Payload Splitting**: Dividing payloads into multiple parts to bypass length restrictions or inspection limits.
- **Protocol Obfuscation**: Using HTTP/2 or other protocol features to hide malicious intent.
- **HTTP Smuggling**: Raw TCP/TLS attacks via `smuggling.rs` (CL.TE, TE.CL, chunked malformed)

### Payloads (`payloads/`)

The WAF module includes a set of "benign" and "malicious" probes specifically designed to test WAF sensitivity without necessarily triggering a block.

## Integration

The WAF module is used by both the **Scanner** (during the discovery phase) and the **Fuzzer** (to ensure payloads are delivered successfully). The **AI** module can also suggest advanced bypasses based on the detected WAF.

## Supported WAFs (34 products)

Cloudflare, Akamai, AWS WAF, Azure WAF, Google Cloud Armor, Fastly, Imperva, Sucuri, CloudFront, F5 BIG-IP, Barracuda, Fortinet, Citrix NetScaler, ModSecurity, Wordfence, DataDome, PerimeterX, Nginx, Traefik, Kong, Varnish, Radware, Signal Sciences, Wallarm, Reblaze, F5 BIG-IP Advanced WAF, Palo Alto, Qrator, Imunify360, SiteGuard, StackPath WAF, Humanity, Datadog, Generic WAF Block
