# Fuzzer Architecture Review

**Document:** architecture/fuzzer.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 121

## Verified Claims

- **30 payload types**: Verified at `crates/slapper/src/fuzzer/payloads/mod.rs:39-70`
  - Enum has exactly 30 variants from `Sqli` to `Oast`
- **WAF_BLOCKED_STATUS_CODES constant**: Verified at `engine/utils.rs:18`
  - `const WAF_BLOCKED_STATUS_CODES: &[u16] = &[403, 406, 429];`
- **DEFAULT_SPIKE_THRESHOLD = 3.0**: Verified at `detection/analyzer.rs:27`
- **DEFAULT_REDOS_THRESHOLD_MS = 5000**: Verified at `detection/analyzer.rs:28`
- **OVERSIZED_PAYLOAD_SIZES**: Verified at `api_schema/mod.rs:7`
  - `[1_000, 10_000, 100_000, 1_000_000]`
- **TimingAnalyzer IQR with NaN handling**: Verified at `detection/analyzer.rs:168-177`
- **RegexExecutor default timeout 1000ms**: Verified at `redos_detect.rs:58`
- **RegexExecutor max iterations 100k**: Verified at `redos_detect.rs:59`
- **Known vulnerable patterns**: Verified at `redos_detect.rs:10-28`
  - Contains 15 patterns including `(.+)+`, `(.*)*`, `(a+)+`, etc.
- **FxHashMap for vulnerable_payloads**: Verified at `redos_detect.rs:277`
- **Advanced fuzzers exist**: Verified at `advanced.rs`
  - `GraphQLFuzzer`, `JwtFuzzer`, `OAuthFuzzer`, `IdorFuzzer`, `SstiFuzzer`, `WebSocketFuzzer`, `GrpcFuzzer`
- **Auth bypass headers**: Verified at `api_schema/mod.rs:229-233`
  - `X-Original-URL`, `X-Override-URL`, `X-Rewrite-URL`

## Discrepancies

- **None identified** - All major claims verified against source

## Bugs Found

- **Potential issue in api_schema/mod.rs:291-306**: The `fuzz_endpoint` function silently continues on request failure without proper error propagation. While it creates a `SchemaFuzzResult` with `vulnerable: false`, the error details could be lost if `tracing::debug` is not enabled.

## Improvement Opportunities

- **Error context in fuzz_endpoint**: Consider adding failed requests to a separate counter or logging at warn level instead of debug, since request failures during fuzzing may indicate interesting network conditions (priority: medium)
- **RegexExecutor iteration limit**: The `max_iterations: 100000` in `check_pattern_async` could lead to long-running tasks on complex regexes (priority: medium)

## Stale Items

- **None identified**

## Code Interrogation Findings

- **Missing PayloadType variant check**: At `payloads/mod.rs:152-185`, the `get_payloads` match expression handles all 30 variants but could benefit from a catch-all that returns an empty vec for unknown types instead of compile-time guarantee
- **No rate limiting on adaptive mode**: The fuzzer mentions "Adaptive" mode in execution but the `AdaptiveRateLimiter` is defined but not visibly integrated into the main fuzzing loop in `engine/core.rs`
- **TimingAnalyzer clone**: At `detection/analyzer.rs:31-47`, the `Clone` implementation for `TimingAnalyzer` creates a new state that may diverge from the original during parallel fuzzing - consider using `Arc<Mutex<TimingAnalyzer>>` instead