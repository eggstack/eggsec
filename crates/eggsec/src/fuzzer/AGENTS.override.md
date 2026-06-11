# Fuzzer Module Override

Specialized guidance for the fuzzing engine module.

## Key Types

- `FuzzEngine` - Main fuzzing engine (returns `Result`)
- `FuzzResult` - Fuzzing result in `fuzzer/engine/types.rs`:
  - `response_body: Option<String>` - captured response body for regex matching
  - Used by `filters::FilterChain` for regex-based filtering
- `PayloadType` - Enum of 40 payload categories

## Module Structure (Verified 2026-06-04)

The following modules exist in `fuzzer/`:
- `calibration.rs` - Calibration engine for baseline timing
- `chain.rs` - `ChainExecutor` with LRU regex cache
- `detection/` - Timing analysis and detection logic
- `engine/` - Core fuzzing engine
- `payloads/` - Payload definitions by category (sqli, xss, etc.)
- `targets/` - Target handling
- `grammar.rs` - Grammar-based fuzzing

## payload_vec! Macro

`fuzzer/payloads/macros.rs` defines `payload_vec!` for building payload vectors from inline data. Reduces repetitive `for` loops (e.g., sqli.rs went from 8 loops to 1 macro call).

## Filters

`fuzzer/filters.rs` provides response filtering with regex support:
- Stores compiled `Regex` internally
- Used with `FuzzResult.response_body`

## Chain Executor

`fuzzer/chain.rs` has `ChainExecutor` with LRU regex cache:
- Use `lru = "0.18"` with cache size 100 (`NonZeroUsizer`)

## Timing Analysis

`fuzzer/detection/` has `TimingAnalyzer` (lock-free with atomics)

## Division by Zero Guard

In `fuzzer/detection/analyzer.rs:188-190`, the IQR calculation uses `if start >= end` check but this is insufficient if `sorted_samples.len() < 4`. Always add explicit empty check:

```rust
let iqr_samples: Vec<f64> = sorted_samples[start..end].to_vec();
if iqr_samples.is_empty() {
    return;
}
self.baseline_ms = Some(sum / iqr_samples.len() as f64);
```