# Fuzzer Architecture Review

**Document:** architecture/fuzzer.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims
- FuzzEngine, PayloadType, FuzzResult exist in `crates/slapper/src/fuzzer/engine/mod.rs:11` and `crates/slapper/src/fuzzer/payloads/mod.rs:38`
- PayloadType has 30 variants (Sqli through Oast) at `crates/slapper/src/fuzzer/payloads/mod.rs:39-70`
- State Management (`state.rs`), Mutator (`mutator.rs`), Rate Limiting (`rate_limit.rs`) all exist in `crates/slapper/src/fuzzer/`
- Execution Modes: Sequential, Burst, Adaptive referenced in FuzzEngine
- Grammar-based Fuzzing (`grammar.rs`) exists at `crates/slapper/src/fuzzer/grammar.rs`
- Detection module with analyzer.rs exists at `crates/slapper/src/fuzzer/detection/analyzer.rs`
- Diffing (`diff.rs`) exists at `crates/slapper/src/fuzzer/diff.rs`
- WAF Fingerprinting (`waf_fingerprint.rs`) exists at `crates/slapper/src/waf_fingerprint.rs`
- API Schema Fuzzing (`api_schema/`) exists at `crates/slapper/src/fuzzer/api_schema/mod.rs`
- Advanced fuzzers (GraphQLFuzzer, JwtFuzzer, OAuthFuzzer, IdorFuzzer, SstiFuzzer, WebSocketFuzzer, GrpcFuzzer) all verified at `crates/slapper/src/fuzzer/advanced.rs:4-14`
- ReDoS Detection (`redos_detect.rs`) exists at `crates/slapper/src/fuzzer/redos_detect.rs`
- Magic numbers `DEFAULT_SPIKE_THRESHOLD`, `DEFAULT_REDOS_THRESHOLD_MS` verified at `crates/slapper/src/fuzzer/detection/analyzer.rs:27-28`
- `BODY_LENGTH_ANOMALY_THRESHOLD` and `TIMING_ANOMALY_THRESHOLD_MS` referenced but not found as named constants in analyzer.rs (they may be in constants.rs)
- `OVERSIZED_PAYLOAD_SIZES` verified at `crates/slapper/src/fuzzer/api_schema/mod.rs:7`
- `WAF_BLOCKED_STATUS_CODES` verified at `crates/slapper/src/fuzzer/engine/utils.rs:18`
- TimingAnalyzer IQR with NaN handling verified at `crates/slapper/src/fuzzer/detection/analyzer.rs:168-178`
- RegexExecutor timeout-based detection (default 1000ms, max 100k iterations) verified at `crates/slapper/src/fuzzer/redos_detect.rs:57-59`
- Known vulnerable patterns verified at `crates/slapper/src/fuzzer/redos_detect.rs:10-28`
- FxHashMap usage in redos_detect.rs verified at line 2
- WebSocketFuzzer uses `PayloadType::Websocket` (not Grpc) verified at `crates/slapper/src/fuzzer/advanced.rs:432`
- ApiSchemaFuzzer with OpenAPI 3.0 parsing verified at `crates/slapper/src/fuzzer/api_schema/mod.rs:64-74`
- Auth bypass headers (X-Original-URL, X-Override-URL, X-Rewrite-URL) verified at `crates/slapper/src/fuzzer/api_schema/mod.rs:229-233`
- Type-aware payloads (string, integer, boolean, array, object) verified at `crates/slapper/src/fuzzer/api_schema/mod.rs:161-186`
- Required parameter omission testing verified at `crates/slapper/src/fuzzer/api_schema/mod.rs:258-281`

## Discrepancies
- [Body length anomaly threshold]: Documented as `BODY_LENGTH_ANOMALY_THRESHOLD: isize = 1000`, but actual code uses `LENGTH_DIFF_THRESHOLD` from constants module (`crates/slapper/src/fuzzer/engine/utils.rs:160`)
- [Timing anomaly threshold]: Documented as `TIMING_ANOMALY_THRESHOLD_MS: i64 = 1000`, but the anomaly detection uses `spike_threshold` (default 3.0 multiplier) rather than a fixed ms threshold (`crates/slapper/src/fuzzer/detection/analyzer.rs:202`)

## Bugs Found
- None identified

## Improvement Opportunities
- [Doc accuracy for thresholds]: The magic numbers section shows constants that don't match actual variable names. Update to reflect `LENGTH_DIFF_THRESHOLD` from constants module. (priority: low)

## Stale Items
- None identified
