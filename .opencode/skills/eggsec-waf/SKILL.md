# Eggsec WAF Skill

WAF detection and bypass module workflows and patterns.

## Performance Note

This module uses `FxHashMap` and `FxHashSet` from `rustc_hash` for performance. Do NOT use `std::collections::HashMap` or `std::collections::HashSet` in WAF code. All hash-based collections in this module should use the faster variants.

## Key Types and Patterns

### Constants (`constants::waf`)

All scoring constants are `u16` to prevent overflow when accumulating scores:

| Constant | Value | Purpose |
|----------|-------|---------|
| `HEADER_MATCH_SCORE` | 25 | Header indicator match |
| `COOKIE_MATCH_SCORE` | 20 | Cookie indicator match |
| `BODY_MATCH_SCORE` | 15 | Body pattern match |
| `IP_MATCH_SCORE` | 20 | Remote IP in known WAF range |
| `HIGH_CONFIDENCE_EXIT` | 90 | Score threshold to stop detection early |
| `UNKNOWN_WAF_CONFIDENCE` | 30 | Confidence when WAF detected but unknown |
| `BLOCKED_STATUS_CODES` | [403, 406, 429, 503] | HTTP status codes indicating WAF block |
| `BLOCKED_PATTERNS` | 8 patterns | Body patterns indicating block |
| `WEAK_BLOCK_INDICATOR_PATTERNS` | 4 patterns | Patterns for "Unknown WAF" detection |
| `LENGTH_DIFF_THRESHOLD` | 100 | Response length difference for detection |

### Detection Process

WAF detection in `detector/detect.rs`:
1. Sends GET request to target URL
2. Collects headers, cookies, body, and remote IP
3. Iterates through 34 WAF signatures calculating scores (internal score uses `u16`):
   - **Header match**: +25 points (per header, value length <= 256)
   - **Cookie match**: +20 points
   - **Body pattern match**: +15 points
   - **Remote IP match**: +20 points (IP in known WAF IP range)
4. Exits early if score >= 90 (HIGH_CONFIDENCE_EXIT)
5. Falls back to "Unknown WAF" if weak indicators found (2+ weak pattern hits)

### Bypass Detection

The `is_bypass_successful()` function in `waf/bypass/mod.rs` verifies:
1. Status is NOT in `BLOCKED_STATUS_CODES`
2. Status differs from baseline detection
3. Status is 2xx (200-299)
4. **Payload is reflected in response body** (urlencoded or raw)

```rust
pub fn is_bypass_successful(
    status: u16,
    detection: &WafDetectionResult,
    payload: &str,
    response_body: &str,
) -> bool
```

When testing bypass techniques, ensure:
- Call `response.text().await` to get body, handling errors explicitly
- Pass payload and body to `is_bypass_successful()`
- Don't just check status codes - verify payload reflection

### Bypass Modules

| Module | Description |
|--------|-------------|
| `evasion.rs` | Payload-based evasion (case rotation, homoglyphs, zero-width, unicode, double encoding) |
| `headers.rs` | HTTP header manipulation (UA rotation, X-Forwarded-For spoofing, Content-Type bypass) |
| `smuggling.rs` | HTTP request smuggling via raw TCP/TLS (CL.TE, TE.CL, chunked malformed) |

### BypassTechnique Enum Variants

```rust
pub enum BypassTechnique {
    HeaderManipulation,
    UserAgentRotation,
    XForwardedForSpoof,
    ContentTypeBypass,
    EncodingBypass,
    Homoglyph,
    ZeroWidthInjection,
    CaseRotation,
    UnicodeEncoding,
    CommentObfuscation,
    WhitespaceVariation,
    ChunkedEncoding,
    ContentLengthConflict,
    TransferEncodingConflict,
    DoubleEncoding,
}
```

### Profile Caching

`get_waf_profiles()` returns a static `&'static Vec<WafProfile>` via `LazyLock` to avoid recreating profiles on every call. Always use `get_profile_by_name()` which clones from this cached data rather than reconstructing profiles.

## Testing

### Running WAF Tests
```bash
cargo test --lib -p eggsec waf::
cargo test --test waf_detector_tests -p eggsec
```

### Writing Tests
Follow existing test patterns in `waf/` modules, testing detection logic and bypass techniques.

## Common Tasks

### Adding a New WAF Detection Rule
1. Add scoring/detection constants to `constants::waf`
2. Add signature to `data/patterns.rs` (FxHashMap<String, WafSignature>)
3. Implementation note: signatures_lower in detector uses lowercase Vec<String> for matching
4. Implement detection logic in `detector/detect.rs`
5. Avoid magic numbers by using defined constants
6. Add tests for new detection rule

### Implementing a New Bypass Technique
1. Add technique to `BypassTechnique` enum in `bypass/mod.rs`
2. Implement test method in appropriate module (evasion/headers/smuggling)
3. Use explicit error handling for `response.text().await` instead of `unwrap_or_default()`
4. Pass payload and response body to `is_bypass_successful()`
5. Add test for the new technique

### Error Handling Pattern

When reading response bodies in WAF modules, use explicit match instead of `unwrap_or_default()`:

```rust
let body = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```

## Resources

- `crates/eggsec/src/waf/AGENTS.override.md` - Detailed WAF patterns
- `crates/eggsec/src/waf/data/patterns.rs` - 34 WAF signatures (FxHashMap)
- `crates/eggsec/src/waf/bypass/profiles.rs` - WAF-specific bypass profiles (cached via LazyLock)
- `AGENTS.md` - General project guidelines
- `architecture/waf.md` - Architecture documentation

## TUI Integration (WafTab)

When working with WafTab in the TUI (`crates/eggsec/src/tui/tabs/waf.rs`):

### Checkbox Bounds Check (Fixed 2026-05-25)

The `technique_checkboxes` array in WafTab requires bounds checking before access:

```rust
// WRONG - could panic if index >= len
self.technique_checkboxes[self.focused_checkbox_index].toggle();

// CORRECT - bounds check prevents panic
if self.focused_checkbox_index < self.technique_checkboxes.len() {
    self.technique_checkboxes[self.focused_checkbox_index].toggle();
}
```

This pattern is now correctly implemented at `waf.rs:519`. Compare with `recon.rs:588-590` for the same pattern.