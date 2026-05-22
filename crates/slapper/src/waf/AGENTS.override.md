# WAF Module Override

Specialized guidance for the WAF detection and bypass module.

## Constants

`constants::waf` module has scoring and detection constants. Use these instead of magic numbers in WAF-related code.

## Performance Note

This module uses `FxHashMap` and `FxHashSet` from `rustc_hash` for performance. Do NOT use `std::collections::HashMap` or `std::collections::HashSet` in WAF code.

Key types:
- `ResponseDiff.normal_headers` / `malicious_headers` - `FxHashMap<String, String>`
- `WafDetector.signatures` - `FxHashMap<String, WafSignature>`
- `WafProfile` generation uses `FxHashSet<String>` for existing names

## Bypass Detection Pattern

When implementing WAF bypass detection, use `is_bypass_successful()` from `waf/bypass/mod.rs`:

```rust
pub fn is_bypass_successful(
    status: u16,
    detection: &WafDetectionResult,
    payload: &str,
    response_body: &str,
) -> bool
```

The function verifies:
1. Status is not in BLOCKED_STATUS_CODES
2. Status differs from baseline detection
3. Status is 2xx (200-299)
4. Payload is reflected in response body (urlencoded or raw)

## Certificate Info Extraction (Recon SSL)

When extracting certificate info in `recon/ssl.rs`, use the `pem` crate:

```rust
if let Ok(pem_data) = pem::parse(der_bytes) {
    let pem_str = String::from_utf8_lossy(pem_data.contents());
    // Parse fields: Subject, Issuer, Not Before, Not After, Serial Number, SAN
}
```