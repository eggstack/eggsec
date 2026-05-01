---
name: fuzz_core
description: "Fuzzer core types and patterns including FuzzResult, PayloadType, and FilterChain"
triggers:
  - FuzzResult
  - fuzzer types
  - filter chain
  - payload types
metadata:
  category: fuzzer
  tools: []
  scope: fuzzer
---

## Overview

This skill documents the core types and patterns used in the Slapper fuzzer module.

## FuzzResult Struct

Location: `crates/slapper/src/fuzzer/engine/types.rs`

The `FuzzResult` struct represents the result of a single fuzz test:

```rust
pub struct FuzzResult {
    pub payload: Payload,
    pub status_code: u16,
    pub response_time_ms: u64,
    pub response_length: Option<u64>,
    pub response_body: Option<String>,  // Captured response body for regex matching
    pub is_waf_blocked: bool,
    pub is_anomaly: bool,
    pub is_redos_suspected: bool,
    pub leaks_found: Vec<String>,
    pub error: Option<String>,
    pub owasp_category: Option<String>,
    pub detected_severity: Severity,
}
```

### Key Fields

- `response_body: Option<String>` - The captured response body. This is used by `FilterChain` for regex-based filtering. When creating `FuzzResult`, populate this field with the response body when available.
- `payload: Payload` - The payload that was sent
- `status_code: u16` - HTTP status code (0 for errors)
- `leaks_found: Vec<String>` - Detected information leaks

### Creating FuzzResult

When creating `FuzzResult` in fuzzer implementations:

```rust
FuzzResult {
    payload: payload.clone(),
    status_code: status,
    response_time_ms: timing_result.response_time_ms,
    response_length: content_length,
    response_body: Some(body),  // Include response body for regex filtering
    is_waf_blocked: false,
    is_anomaly: timing_result.is_anomaly,
    is_redos_suspected: timing_result.is_redos_suspected,
    leaks_found: leaks.iter().map(|l| format!("{}: {}", l.category, l.pattern)).collect(),
    error: None,
    owasp_category: Some(owasp_str),
    detected_severity,
}
```

## FilterChain and Regex Filtering

Location: `crates/slapper/src/fuzzer/filters.rs`

The `FilterChain` provides flexible response filtering similar to ffuf:

```rust
pub struct FilterChain {
    filters: Vec<PayloadFilter>,
}
```

### PayloadFilter Enum

```rust
pub enum PayloadFilter {
    StatusCode(Vec<u16>),
    ResponseSize(Vec<u64>),
    ResponseSizeRange { min: u64, max: u64 },
    WordCount(Vec<u64>),
    WordCountRange { min: u64, max: u64 },
    LineCount(Vec<u64>),
    LineCountRange { min: u64, max: u64 },
    ResponseTime(u64),
    ResponseTimeRange { min: u64, max: u64 },
    Regex(Regex),  // Stores compiled Regex directly
    SizeGreaterThan(u64),
    SizeLessThan(u64),
}
```

### Regex Filter

The `Regex(Regex)` variant stores the compiled regex directly. When matching:

```rust
PayloadFilter::Regex(regex) => {
    if let Some(ref body) = result.response_body {
        regex.is_match(body)
    } else {
        false
    }
}
```

**Important:** For regex filters to work, the `response_body` field in `FuzzResult` must be populated.

## PayloadType Enum

Location: `crates/slapper/src/fuzzer/payloads/mod.rs`

The `PayloadType` enum defines 30 payload categories. When adding new payload types:

1. Add variant to `PayloadType` enum
2. Implement payload generation in the appropriate module
3. Use the `payload_vec!` macro for concise payload vector creation

## Adding New Payload Types

1. Define the payload type in `fuzzer/payloads/mod.rs`
2. Create payload generation in appropriate sub-module (e.g., `fuzzer/payloads/sqli.rs`)
3. Use `payload_vec!` macro:
   ```rust
   payload_vec![
       ("payload1", "description1", Severity::High),
       ("payload2", "description2", Severity::Medium),
   ]
   ```

## Key Files

| File | Purpose |
|------|---------|
| `fuzzer/engine/types.rs` | `FuzzResult`, `FuzzSession`, `BaselineResponse` |
| `fuzzer/filters.rs` | `FilterChain`, `PayloadFilter` with regex support |
| `fuzzer/chain.rs` | `ChainExecutor` with LRU regex cache |
| `fuzzer/payloads/mod.rs` | `PayloadType` enum, `Payload` struct |
| `fuzzer/engine/execution.rs` | `send_fuzz_request`, creates `FuzzResult` with body |
| `fuzzer/engine/utils.rs` | `send_payload_async`, captures response body |

## Triggers

Keywords that activate this skill:
- "FuzzResult"
- "fuzzer types"
- "filter chain"
- "payload types"
- "add payload type"
