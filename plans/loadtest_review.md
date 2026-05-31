# Load Testing Architecture Review

**Document:** architecture/loadtest.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium

## Verified Claims

- [LoadTestRunner struct]: Fields match actual implementation at `crates/slapper/src/loadtest/runner.rs:20-34`
- [new() constructor]: Basic constructor verified at `runner.rs:37-44`
- [new_with_tui_mode()]: Constructor with TUI flag verified at `runner.rs:46-79`
- [from_args()]: CLI args constructor verified at `runner.rs:81-83`
- [from_args_with_tui_mode()]: CLI args with TUI mode verified at `runner.rs:85-108`
- [from_args_with_config()]: Pipeline integration constructor verified at `runner.rs:110-134`
- [LoadTestResults struct]: Fields match actual implementation at `crates/slapper/src/loadtest/metrics.rs:8-24`
- [FxHashMap for status codes]: Uses `rustc_hash::FxHashMap` verified at `metrics.rs:3`
- [hdrhistogram]: Uses `hdrhistogram::Histogram<u64>` with 3 significant figures verified at `metrics.rs:76`
- [Rate limiting]: Semaphore-based token bucket with background permit replenishment verified at `runner.rs:271-282`
- [Connection pool body handling]: Non-success response bodies consumed before pool return verified at `runner.rs:338-339`
- [Worker model]: Spawns `min(concurrency, total_requests)` workers via JoinSet verified at `runner.rs:286-287`
- [apply_auth_headers()]: Basic/Bearer/Cookie/API Key auth verified at `runner.rs:172-204`
- [Metrics collection]: Latency percentiles (p50/p90/p95/p99), RPS, error rates verified at `metrics.rs:114-139`
- [Override file reference]: `crates/slapper/src/loadtest/AGENTS.override.md` exists

## Discrepancies

- [run_cli() signature]: Documented as `pub fn run_cli(args: LoadArgs) -> Result<(), TabError>` (line 67) but actual signature is `pub async fn run_cli(args: LoadArgs, config: &SlapperConfig) -> Result<()>` (`mod.rs:66`). Three differences: (1) function is `async`, (2) takes additional `&SlapperConfig` parameter, (3) returns `Result<()>` not `Result<(), TabError>`.
- [run_cli() description]: Document says "Parses CLI arguments, instantiates a runner, executes the load test, and outputs results. Used by the main CLI dispatcher." The actual implementation also handles verbose/quiet flags, JSON output, and file output (`mod.rs:66-100`).
- [LoadTestResults.successful_requests]: Documented at line 84 as field name, actual field is `successful_requests` (matches). However, the `Metrics::record_http_response()` method considers 200-399 as successful (`metrics.rs:97`), which differs from the doc's claim of "non-2xx/3xx" being errors.

## Bugs Found

- None found in the documented architecture.

## Improvement Opportunities

- [run_cli() docs]: The documented signature should be corrected to match the actual async signature with config parameter. (priority: high)
- [Success criteria]: The doc implies 2xx/3xx are success and 4xx/5xx are errors, which matches the implementation (`metrics.rs:97` uses `200..400` range). This is accurate but could be more explicit. (priority: low)

## Stale Items

- [run_cli() signature]: The documented signature `pub fn run_cli(args: LoadArgs) -> Result<(), TabError>` appears to be from an older version before the config parameter was added. Recommended action: Update to `pub async fn run_cli(args: LoadArgs, config: &SlapperConfig) -> Result<()>`.
