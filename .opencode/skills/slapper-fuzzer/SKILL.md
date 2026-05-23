# Slapper Fuzzer Skill

Fuzzing engine module workflows and patterns for security testing.

## Key Types and Patterns

### Core Types
- `FuzzEngine` - Main fuzzing engine (returns `Result`)
- `FuzzResult` - Fuzzing result in `fuzzer/engine/types.rs` with `response_body: Option<String>` for regex matching
- `PayloadType` - Enum of 30 payload categories

### payload_vec! Macro
`fuzzer/payloads/macros.rs` defines `payload_vec!` for building payload vectors from inline data, reducing repetitive `for` loops.

### Filters
`fuzzer/filters.rs` provides response filtering with compiled `Regex` support, using `FuzzResult.response_body`.

### ChainExecutor
`fuzzer/chain.rs` has `ChainExecutor` with LRU regex cache using `lru = "0.18"` (cache size 100, `NonZeroUsizer`).

### Timing Analysis
`fuzzer/detection/` has `TimingAnalyzer` with lock-free atomics, using IQR (Interquartile Range) for baseline calculation.

### API Schema Fuzzing
`fuzzer/api_schema/mod.rs` provides OpenAPI 3.0 (JSON/YAML) parsing with type-aware fuzzing:
- `ApiSchemaFuzzer` - Generates fuzz targets from OpenAPI specs
- Type-aware payloads based on parameter types (string, integer, boolean, array, object)
- Auth bypass via headers (X-Original-URL, X-Override-URL, X-Rewrite-URL)
- Required parameter omission testing
- Oversized payload generation (1KB, 10KB, 100KB, 1MB)

### Advanced Fuzzers (`fuzzer/advanced.rs`)
- `GraphQLFuzzer` - Introspection, depth bypass, alias overload, batch queries
- `JwtFuzzer` - None algorithm attack, key injection, token validation
- `OAuthFuzzer` - Redirect URI, scope escalation, state parameter, grant mixing
- `IdorFuzzer` - Horizontal/vertical escalation testing
- `SstiFuzzer` - Template engine detection (Jinja2, ERB, etc.)
- `WebSocketFuzzer` - Message injection (ensure `PayloadType::Websocket` is used, not `PayloadType::Grpc`)
- `GrpcFuzzer` - Method injection

### ReDoS Detection (`fuzzer/redos_detect.rs`)
- `RegexExecutor` - Timeout-based detection (default 1000ms, max 100k iterations)
- Known vulnerable patterns: `(.+)+`, `(.*)*`, `(a+)+`, etc.
- Uses `FxHashMap` for vulnerable payload tracking

### WAF Fingerprinting (`fuzzer/waf_fingerprint.rs`)
- Supports 18 WAF products (Cloudflare, Akamai, AWS WAF, Imperva, etc.)
- Header-based signatures and body pattern matching
- Confidence scoring with 0.2 threshold

## Code Conventions

### Hash Collections
Use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of `std::collections::HashMap/HashSet` for better performance.

### Magic Numbers
Extract magic numbers to named constants at module level:
```rust
const DEFAULT_SPIKE_THRESHOLD: f64 = 3.0;
const DEFAULT_REDOS_THRESHOLD_MS: u64 = 5000;
const BODY_LENGTH_ANOMALY_THRESHOLD: isize = 1000;
const TIMING_ANOMALY_THRESHOLD_MS: i64 = 1000;
const OVERSIZED_PAYLOAD_SIZES: [usize; 4] = [1_000, 10_000, 100_000, 1_000_000];
```

### Error Handling
Prefer explicit error handling over `unwrap_or_default()`:
```rust
// Instead of:
let body = response.text().await.unwrap_or_default();

// Use:
let body = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```

### NaN Handling in Timing Analysis
When using `partial_cmp` with f64 values, handle NaN explicitly:
```rust
s.sort_by(|a, b| a.partial_cmp(b).unwrap_or_else(|| {
    if a.is_nan() && b.is_nan() {
        std::cmp::Ordering::Equal
    } else if a.is_nan() {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Less
    }
}));
```

### Notable Bug Fixes

#### 2026-05-28
- **detection/analyzer.rs:188-190** - IQR calculation could divide by zero if `iqr_samples` vec is empty after slice. Added `if iqr_samples.is_empty() { return; }` check.

## Testing

### Running Fuzzer Tests
```bash
cargo test --lib -p slapper fuzzer::
```

### Writing Tests
Follow existing test patterns in `fuzzer/` modules, using `FuzzEngine` and `FuzzResult` types.

## Common Tasks

### Adding a New Payload Category
1. Add variant to `PayloadType` enum
2. Implement payload generation in `payloads/`
3. Use `payload_vec!` macro for inline payload data
4. Add tests for new payload type

### Adding Response Filters
1. Implement filter logic in `filters.rs`
2. Use compiled `Regex` for performance
3. Test with `FuzzResult` samples

### OpenAPI Schema Fuzzing Workflow
1. Parse spec: `ApiSchemaFuzzer::parse_openapi(content)`
2. Generate type-aware payloads: `generate_type_aware_payloads(&param)`
3. Generate auth bypass: `generate_auth_bypass_payloads(endpoints, base_url)`
4. Generate oversized: `generate_oversized_payloads(endpoints)`
5. Fuzz endpoint: `fuzz_endpoint(&target).await`

## Resources
- `crates/slapper/src/fuzzer/AGENTS.override.md` - Detailed fuzzer patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
- `architecture/fuzzer.md` - Fuzzer module architecture details
