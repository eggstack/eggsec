# Fuzzer Architecture Review

**Document:** architecture/fuzzer.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 121

## Verified Claims

| Claim | Status | Evidence |
|-------|--------|----------|
| PayloadType enum has 30 variants | ✅ Verified | `crates/slapper/src/fuzzer/payloads/mod.rs:39-70` - 30 variants from `Sqli` to `Oast` |
| FuzzMode: Sequential, Burst (up to 500), Adaptive | ✅ Verified | `crates/slapper/src/fuzzer/engine/core.rs:134` - `concurrency.clamp(1, 500)`, `core.rs:335-337` - mode dispatch |
| `WAF_BLOCKED_STATUS_CODES: &[u16] = &[403, 406, 429]` | ✅ Verified | `crates/slapper/src/fuzzer/engine/utils.rs:18` |
| `TimingAnalyzer` uses IQR for baseline with NaN handling | ✅ Verified | `crates/slapper/src/fuzzer/detection/analyzer.rs:166-180` - IQR calculation with `partial_cmp` NaN handling |
| `DEFAULT_SPIKE_THRESHOLD: f64 = 3.0` | ✅ Verified | `crates/slapper/src/fuzzer/detection/analyzer.rs:27` |
| `DEFAULT_REDOS_THRESHOLD_MS: u64 = 5000` | ✅ Verified | `crates/slapper/src/fuzzer/detection/analyzer.rs:28` |
| `BODY_LENGTH_ANOMALY_THRESHOLD: isize = 1000` | ✅ Verified | `crates/slapper/src/fuzzer/diff.rs:229` |
| `TIMING_ANOMALY_THRESHOLD_MS: i64 = 1000` | ✅ Verified | `crates/slapper/src/fuzzer/diff.rs:294` |
| `OVERSIZED_PAYLOAD_SIZES: [usize; 4] = [1_000, 10_000, 100_000, 1_000_000]` | ✅ Verified | `crates/slapper/src/fuzzer/api_schema/mod.rs:7` |
| Advanced fuzzers: GraphQL, JWT, OAuth, IDOR, SSTI, WebSocket, gRPC | ✅ Verified | `crates/slapper/src/fuzzer/advanced.rs:4-14` - re-exports all 7 types |
| `WebSocketFuzzer` uses `PayloadType::Websocket` (not `Grpc`) | ✅ Verified | `crates/slapper/src/fuzzer/payloads/mod.rs:62` - `Websocket` variant exists |
| ReDoS: `RegexExecutor` with 1000ms timeout, 100k iterations | ✅ Verified | `crates/slapper/src/fuzzer/redos_detect.rs:58-59` - `timeout: 1000`, `max_iterations: 100000` |
| Known vulnerable patterns: `(.+)+`, `(.*)*`, `(a+)+`, etc. | ✅ Verified | `crates/slapper/src/fuzzer/redos_detect.rs:10-28` - 15 patterns including all listed |
| `FxHashMap` used for vulnerable payload tracking in ReDoS | ✅ Verified | `crates/slapper/src/fuzzer/redos_detect.rs:2` - `use rustc_hash::FxHashMap` |
| Grammar-based fuzzing: JSON, GraphQL, XML, JWT, SSTI | ✅ Verified | `crates/slapper/src/fuzzer/grammar.rs:6-12` - `GrammarKind` enum with 5 variants |
| API Schema fuzzing: OpenAPI 3.0 parsing | ✅ Verified | `crates/slapper/src/fuzzer/api_schema/mod.rs` exists |
| File tree structure matches source | ✅ Verified | Directory listing of `fuzzer/` matches document structure |
| Diffing in `diff.rs` | ✅ Verified | `crates/slapper/src/fuzzer/diff.rs` - `ResponseDiff`, `DiffResult` types |

## Discrepancies

### 1. WAF Blocked Status Codes: Fuzzer vs WAF Module Inconsistency

**Severity:** Low (not a doc error, but a code inconsistency worth noting)

The fuzzer module defines its own local `WAF_BLOCKED_STATUS_CODES` at `engine/utils.rs:18` with **3 codes** `[403, 406, 429]`, while the WAF module uses `constants::waf::BLOCKED_STATUS_CODES` at `constants.rs:77` with **4 codes** `[403, 406, 429, 503]`.

The document accurately describes the fuzzer's constant. However, this means the fuzzer does **not** treat HTTP 503 as a WAF block, while the WAF bypass logic does. This could lead to inconsistent bypass detection between the two modules.

**Evidence:**
- `crates/slapper/src/fuzzer/engine/utils.rs:18` - 3 codes
- `crates/slapper/src/constants.rs:77` - 4 codes
- `crates/slapper/src/waf/bypass/mod.rs:138` - uses `constants::waf::BLOCKED_STATUS_CODES`

## Bugs

No bugs found in the document. All code references are accurate.

## Improvements

### 1. Document Could Note the Fuzzer/WAF Blocked Code Divergence

The document accurately describes the fuzzer's constant, but could mention that the WAF module uses a different (larger) set of blocked codes. This would help developers understand why bypass detection may differ between modules.

### 2. Missing `body_looks_blocked()` in Bypass Detection Description

The document doesn't describe the `body_looks_blocked()` function in `waf/bypass/mod.rs:177-181` which checks response body for blocked patterns (e.g., "access denied", "blocked", "firewall"). This is a secondary bypass failure condition beyond status codes.

## Stale Items

No stale items found. All claims match current codebase state.
