# Slapper WAF Skill

WAF detection and bypass module workflows and patterns.

## Key Types and Patterns

### Constants
`constants::waf` module has scoring and detection constants. Use these instead of magic numbers in WAF-related code.

### Bypass Detection

The `is_bypass_successful()` function in `waf/bypass/mod.rs` verifies:
1. Status is not in `BLOCKED_STATUS_CODES`
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
- Call `response.text().await` to get body
- Pass payload and body to `is_bypass_successful()`
- Don't just check status codes - verify payload reflection

### Bypass Modules

- `evasion.rs` - Payload-based evasion techniques
- `headers.rs` - HTTP header manipulation
- `smuggling.rs` - HTTP request smuggling

## Testing

### Running WAF Tests
```bash
cargo test --lib -p slapper waf::
```

### Writing Tests
Follow existing test patterns in `waf/` modules, testing detection logic and bypass techniques.

## Common Tasks

### Adding a New WAF Detection Rule
1. Add scoring/detection constants to `constants::waf`
2. Implement detection logic in `waf/` modules
3. Avoid magic numbers by using defined constants
4. Add tests for new detection rule

### Implementing a New Bypass Technique
1. Add technique to `BypassTechnique` enum
2. Implement test method in appropriate module (evasion/headers/smuggling)
3. Pass payload and response body to `is_bypass_successful()`
4. Add test for the new technique

## Resources
- `crates/slapper/src/waf/AGENTS.override.md` - Detailed WAF patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
