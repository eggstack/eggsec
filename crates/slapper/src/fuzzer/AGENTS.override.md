# Fuzzer Module Override

Specialized guidance for the fuzzing engine module.

## Key Types

- `FuzzEngine` - Main fuzzing engine (returns `Result`)
- `FuzzResult` - Fuzzing result in `fuzzer/engine/types.rs`:
  - `response_body: Option<String>` - captured response body for regex matching
  - Used by `filters::FilterChain` for regex-based filtering
- `PayloadType` - Enum of 30 payload categories

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