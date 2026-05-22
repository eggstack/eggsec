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
`fuzzer/detection/` has `TimingAnalyzer` with lock-free atomics.

## Code Conventions

### Hash Collections
Use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of `std::collections::HashMap/HashSet` for better performance.

### Magic Numbers
Extract magic numbers to named constants at module level:
```rust
const DEFAULT_SPIKE_THRESHOLD: f64 = 3.0;
const DEFAULT_REDOS_THRESHOLD_MS: u64 = 5000;
const BODY_LENGTH_ANOMALY_THRESHOLD: isize = 1000;
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

## Resources
- `crates/slapper/src/fuzzer/AGENTS.override.md` - Detailed fuzzer patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
