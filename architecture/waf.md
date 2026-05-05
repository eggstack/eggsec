# WAF Module

The WAF (Web Application Firewall) module is dedicated to detecting, fingerprinting, and bypassing security filters in front of web applications.

## Core Components (`src/waf/`)

### Detection (`detector/`)

Slapper can identify over 26 different WAF products by analyzing HTTP responses for specific headers, cookies, and status codes.

- **WAF Patterns (`waf_patterns.rs`)**: A collection of signatures for well-known WAFs like Cloudflare, Akamai, AWS WAF, etc.
- **Detector Logic**: Orchestrates probes to trigger WAF responses and matches them against known patterns.

### Bypass (`bypass/`)

Once a WAF is identified, Slapper can apply specialized bypass techniques.

- **Encodings**: Using different character encodings (e.g., URL, Double URL, Unicode, Hex) to evade simple pattern matching.
- **Header Manipulation**: Injecting or modifying headers (e.g., `X-Forwarded-For`, `User-Agent`) that might influence WAF behavior.
- **Payload Splitting**: Dividing payloads into multiple parts to bypass length restrictions or inspection limits.
- **Protocol Obfuscation**: Using HTTP/2 or other protocol features to hide malicious intent.

### Payloads (`payloads/`)

The WAF module includes a set of "benign" and "malicious" probes specifically designed to test WAF sensitivity without necessarily triggering a block.

## Integration

The WAF module is used by both the **Scanner** (during the discovery phase) and the **Fuzzer** (to ensure payloads are delivered successfully). The **AI** module can also suggest advanced bypasses based on the detected WAF.
