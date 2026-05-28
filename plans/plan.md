# Slapper Consolidated Implementation Plan

**Created:** 2026-05-28
**Status:** Ready for implementation

---

## Priority Index

| # | Severity | Module | Issue | Effort | Wave |
|---|----------|--------|-------|--------|------|
| 1 | CRITICAL | distributed | Task results never sent to coordinator | Low | 1 |
| 2 | HIGH | distributed | WorkerStats never updated | Medium | 1 |
| 3 | HIGH | distributed | Heartbeat reports static zeros | Medium | 1 |
| 4 | HIGH | cli_commands | Resume command bypasses scope validation | Low | 1 |
| 5 | MEDIUM | loadtest | Rate limiting initial burst | Low | 1 |
| 6 | MEDIUM | distributed | Worker registration flow incomplete | Medium | 1 |
| 7 | MEDIUM | distributed | No task assignment pull mechanism | High | 3 |
| 8 | MEDIUM | distributed | No graceful worker shutdown | Medium | 2 |
| 9 | MEDIUM | distributed | Connection cleanup on panic | Medium | 2 |
| 10 | MEDIUM | distributed | Rate limit cleanup unbounded | Low | 2 |
| 11 | MEDIUM | networking | IPv6 spoof range entropy | Low | 1 |
| 12 | MEDIUM | networking | Traceroute no concurrency limit | Low | 2 |
| 13 | MEDIUM | networking | HTTP stress lacks response validation | Low | 1 |
| 14 | MEDIUM | networking | TLS SNI extraction not implemented | Medium | 3 |
| 15 | MEDIUM | networking | UDP spoof range loads all IPs into memory | High | 2 |
| 16 | MEDIUM | scanner | Clone-on-every-request in endpoint scan | Low | 2 |
| 17 | MEDIUM | scanner | Packet trace file handle leak | Low | 2 |
| 18 | MEDIUM | waf | Cookie matching fallible index lookup | Low | 1 |
| 19 | MEDIUM | waf | compare_responses creates new client | Low | 1 |
| 20 | MEDIUM | waf | Missing circuit breaker on detection | Medium | 2 |
| 21 | MEDIUM | output | Template registration unwrap() panic | Low | 1 |
| 22 | MEDIUM | ai_agents | Rate limit counter never resets | Medium | 2 |
| 23 | MEDIUM | ai_agents | Knowledge base eviction bug | Medium | 2 |
| 24 | MEDIUM | ai_agents | MCP integration not implemented | High | 3 |
| 25 | MEDIUM | cli_commands | Proxy handler missing scope validation | Low | 1 |
| 26 | MEDIUM | cli_commands | Timeout defaults not standardized | Low | 2 |
| 27 | MEDIUM | tui | InputGroup bounds checking in reset | Low | 1 |
| 28 | MEDIUM | recon | ThreatStream API key hardcoded to None | Low | 1 |
| 29 | LOW | recon | FullReconResult callback uses HashMap | Low | 2 |
| 30 | LOW | recon | dependency_scan not in pipeline | Low | 3 |
| 31 | LOW | waf | HTTP/2 smuggling dead code | Low | 3 |
| 32 | LOW | waf | WAF count mismatch in docs | Low | 3 |
| 33 | LOW | scanner | Duplicate Memcached probe entry | Low | 1 |
| 34 | LOW | scanner | ICMP probe unused timeout param | Low | 2 |
| 35 | LOW | scanner | UDP fingerprinting no rate limit | Low | 2 |
| 36 | LOW | networking | DNS compression pointer loop limit too low | Low | 1 |
| 37 | LOW | output | ResultComparator finding key undocumented | Low | 3 |
| 38 | LOW | ai_agents | Test code uses std HashMap | Low | 2 |
| 39 | LOW | ai_agents | load_skills silently skips invalid | Low | 2 |
| 40 | LOW | cli_commands | StressArgs naming inconsistency | Low | 3 |
| 41 | LOW | cli_commands | Several handlers don't use CommandContext | Low | 3 |
| 42 | LOW | cli_commands | traceroute max_hops no bounds validation | Low | 2 |
| 43 | LOW | tui | Auto-save interval hardcoded despite config | Low | 3 |
| 44 | LOW | tui | SessionState bookmarks not deduplicated | Low | 3 |
| 45 | LOW | config | Scope validation docs missing | Low | 3 |
| 46 | LOW | distributed | DNS rebinding protection gap | Low | 3 |
| 47 | LOW | distributed | Worker capabilities not validated | Low | 3 |
| 48 | LOW | distributed | Documentation line number offsets | Low | 3 |
| 49 | LOW | loadtest | Rate limit lock contention | Medium | 2 |
| 50 | LOW | loadtest | Missing request cancellation on timeout | Medium | 2 |
| 51 | LOW | cli_commands | AiAnalyze handler loads its own config | Low | 2 |

---

## Wave 1: Critical & Quick Fixes (Parallel)

These items are independent and can all be worked on simultaneously. Each item includes enough context for a sub-agent to implement without additional research.

### 1.1 Distributed: Send Task Results to Coordinator (CRITICAL)

**File:** `crates/slapper/src/distributed/worker.rs:166-182`

**Problem:** `start_task_processing_loop()` spawns tasks but drops `TaskResult` - never sends back via `CommandMessage::Result`. The entire distributed result aggregation system is non-functional.

**Current code (lines 171-177):**
```rust
while let Some(task) = receiver.recv().await {
    tokio::spawn(async move {
        let result = process_task(task).await;
        if let Err(e) = result {
            tracing::error!("Task processing error: {}", e);
        }
        // TaskResult is DROPPED here
    });
}
```

**Fix:**
- Add a `tokio::sync::mpsc::Sender<CommandMessage>` field to `Worker` struct
- Pass the sender into the spawned task
- After `process_task(task).await`, send `CommandMessage::Result { id, result }` back through the channel
- The receiver side at `remote.rs:372` already handles `CommandMessage::Result` via `task_queue.complete(result).await`

**Verification:** After fix, ensure worker sends `CommandMessage::Result { id, result }` for every completed task.

### 1.2 Distributed: Update WorkerStats and Heartbeat (HIGH)

**File:** `crates/slapper/src/distributed/worker.rs:55-82, 150-156`

**Problem:** `WorkerStats` fields are initialized to 0 (lines 78-82) and never updated. Heartbeat (lines 150-156) always reports hardcoded values:
```rust
let status = serde_json::json!({
    "worker_id": worker_id,
    "status": "idle",           // hardcoded
    "current_jobs": 0,          // hardcoded
    "completed_jobs": 0,        // hardcoded
    "failed_jobs": 0,           // hardcoded
});
```

**Fix:**
- In `start_task_processing_loop()`, increment `self.stats.tasks_in_progress` when a task starts, decrement when done
- Increment `self.stats.tasks_completed` or `self.stats.tasks_failed` based on result
- In heartbeat, use actual `self.stats` values instead of hardcoded zeros
- Track actual `WorkerStatus` (idle/busy) based on in-progress task count

### 1.3 CLI: Add Scope Validation to Resume (HIGH)

**File:** `crates/slapper/src/commands/handlers/scan.rs:60-63`

**Problem:** `handle_resume` is the only handler that doesn't take `CommandContext` and doesn't call `ensure_scope()`:
```rust
pub async fn handle_resume(args: crate::cli::ResumeArgs) -> Result<()> {
    crate::pipeline::resume_cli(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
```

All other handlers (lines 4, 15, 26, 38, 52) take `ctx: &CommandContext` and call `ctx.ensure_scope()`.

**Fix:**
- Change signature to `handle_resume(args: crate::cli::ResumeArgs, ctx: &CommandContext)`
- After loading session, validate all targets against `ctx.config.scope`
- Reject session if any target is out of scope
- Update the caller in `commands/handlers/mod.rs` to pass `ctx`

### 1.4 Loadtest: Fix Rate Limiting Initial Burst (MEDIUM)

**File:** `crates/slapper/src/loadtest/runner.rs:275-281`

**Problem:** `next_allowed_at` initialized to `TokioInstant::now() - min_interval`:
```rust
Arc::new(Mutex::new(TokioInstant::now() - min_interval)),
```

This lets the first worker through immediately (since `next` is in the past), and subsequent workers are serialized by the mutex. The first burst is still uncontrolled.

**Fix:** Change line 279 from:
```rust
Arc::new(Mutex::new(TokioInstant::now() - min_interval)),
```
to:
```rust
Arc::new(Mutex::new(TokioInstant::now())),
```

This ensures even the first worker waits the full interval.

### 1.5 Distributed: Worker Registration Flow (MEDIUM)

**File:** `crates/slapper/src/distributed/worker.rs:93-104`

**Problem:** Worker `start()` method spawns heartbeat and task processing but never sends a registration message to the coordinator.

**Fix:**
- Before spawning heartbeat/task loops, send `CommandMessage::Register` with worker ID, address, and capabilities
- Wait for registration confirmation before starting heartbeat
- Handle registration failure with retry logic

### 1.6 Quick Fixes (Batch)

**waf/detector/detect.rs:118-124** - Cookie matching `unwrap_or(0)` bug:
```rust
sig_matched_cookies.push(
    signature.cookies[sig_lower
        .cookies
        .iter()
        .position(|c| c == cookie_pattern_lower)
        .unwrap_or(0)]  // BUG: indexes wrong cookie if pattern not found
    .clone(),
);
```
Fix: Use `if let Some(pos) = ...` and skip pattern if not found.

**waf/detector/compare.rs:14-19** - Creates new HTTP client on every call. The function is standalone (not a method), so `self.client` doesn't exist. Fix: Convert to a method on `WafDetector` or pass the client as a parameter.

**output/template.rs:141-152** - Four `.unwrap()` calls on template registration. Change to `.expect("template registration should never fail")`.

**scanner/fingerprint.rs:32,54** - Two Memcached entries in PROBES array (stats vs version commands). These are intentionally different probes, not duplicates. Verify if both are needed; if so, remove from plan.

**packet/validation.rs:35-37** - DNS compression pointer limit hardcoded to 10. Change to 100 for complex DNS responses.

**stress/http.rs:99-107** - HTTP stress doesn't check status codes. A 4xx/5xx is treated as success. Add status code validation.

**recon/threatintel.rs** - The `threatstream_key` parameter no longer exists in the codebase. `ThreatIntelClient` has `virustotal_key`, `alienvault_key`, `shodan_key` only. This item is stale - remove from plan.

**tui/tabs/fuzz.rs:408** - Bounds check in `reset()` already exists (`if self.inputs.fields.len() > 6`). Already fixed - remove from plan.

**cli/commands/handlers/stress.rs:12** - Scope validation already exists (`ctx.ensure_scope(&args.target)?`). Already fixed - remove from plan.

**cli/commands/handlers/stress.rs:69** - Proxy handler scope validation already exists (`ctx.ensure_scope(&proxy.address)?`). Already fixed - remove from plan.

**cli/commands/handlers/mod.rs:160** - `cli.config` is a valid `Option<String>` field. No bug exists - remove from plan.

**output/pdf.rs:71-79** - PDF truncation warning already exists. Already fixed - remove from plan.

---

## Wave 2: Medium Priority Improvements (Parallel)

These items are independent of each other but depend on Wave 1 completion.

### 2.1 Distributed Module Improvements

**worker.rs:64-104** - Add `shutdown()` method with cancellation channel:
- Worker struct has `heartbeat_handle: Option<JoinHandle<()>>` and `task_processor_handle: Option<JoinHandle<()>>` (lines 69-70)
- Add a `tokio::sync::watch` channel for shutdown signal
- Spawn a `select!` on heartbeat that also listens for shutdown
- The `RemoteListener` in `remote.rs:115-119` already has a `shutdown()` method to reference

**remote.rs:207-211** - Wrap `handle_connection()` with panic catch:
- `JoinHandle` from `tokio::spawn` is dropped (line 209), so panics are silently lost
- Either capture the handle and log panics, or use `Arc<Mutex<>>` for shared state

**remote.rs:121-140** - Add periodic cleanup of stale rate limit entries:
- `check_rate_limit()` cleans timestamps within a single IP's vector (line 132: `timestamps.retain(...)`)
- But top-level map keys (IP addresses) are never removed
- Add a periodic task that removes IPs with no recent timestamps

### 2.2 Networking Improvements

**packet/traceroute.rs:141-168** - Add semaphore to limit concurrent traceroute probes:
- `probe_hop_udp_parallel` and `probe_hop_icmp_parallel` spawn `max_retries` tasks without concurrency limits
- Add `Arc<Semaphore>` with configurable max concurrent probes

**packet/parse_impl.rs:597-638** - Extract SNI from TLS handshake:
- `TlsHandshake::parse()` only extracts `handshake_type` and `version`
- `client_hello` and `server_hello` are always `None` (lines 635-636)
- The `TlsClientHello` struct (types.rs:236-242) has a `server_name: Option<String>` field ready for SNI

**stress/syn.rs:283-292** - Fix IPv6 spoof range randomization:
- Currently limited to 16-bit randomization per segment via `.min(16)` (line 286)
- For prefixes smaller than /96, randomization is insufficient
- `offset_lo` starts at 1 instead of 0 (line 284), meaning original low segment is never chosen

**networking/loadtest** - UDP spoof range memory issue:
- `parse_spoof_range()` loads all IPs into memory for large ranges (e.g., /8 = ~16M entries)
- Should use iterator-based approach or validate range size before expanding

### 2.3 Scanner Improvements

**scanner/endpoints.rs:748** - Use `Arc<SpoofConfig>` instead of cloning per request:
- `config.spoof_config.clone()` in the loop for every endpoint iteration
- `SpoofConfig` derives `Clone` (spoof.rs:29)

**scanner/ports/spoofed.rs:55-56** - Add `shutdown_packet_trace()`:
- `PACKET_TRACE_FILE` is a global `OnceLock<parking_lot::Mutex<File>>`
- Once initialized, no mechanism to close the file handle

**scanner/icmp_probe.rs:32** - Add `tokio::time::timeout()` wrapper:
- `ping_host()` takes `_timeout` parameter (underscore prefix = unused)
- No timeout wrapper around `surge_ping::ping()` calls

**scanner/udp_fingerprint.rs:140** - Add token bucket rate limiting:
- `Semaphore::new(50)` limits concurrency but not packets per second
- All 50 probes could fire simultaneously, overwhelming target

### 2.4 WAF Improvements

**waf/detector/detect.rs:212-219** - Use `url::Url::parse()` in `normalize_url_static`:
- Current implementation uses simple string prefix checking
- `url::Url::parse()` would handle edge cases (trailing slashes, query params, etc.)

**waf/bypass/mod.rs:70** - Use `response_diff` field for confidence scoring:
- `response_diff` is defined in `BypassResult` but set to `None` in most implementations
- `is_bypass_successful()` (lines 142-157) doesn't reference `response_diff`

### 2.5 AI Agents Improvements

**agent/constraints/checker.rs:215-228** - Implement background rate limit reset:
- `evaluate_rate_limit()` increments counters (line 224) but never resets them
- Once limit reached, violation persists permanently
- Add a background task or time-based reset

**ai/waf_bypass.rs:82-94** - Fix knowledge base eviction:
- Current logic sorts by `(failed_attempts > 0, staleness)` then truncates to 50%
- ALL entries with failures are discarded first, even those with few failures
- Should keep entries with `failed_attempts < 3`

**agent/portfolio.rs:572,600** - Replace `std::collections::HashMap` with `FxHashMap` in test code.

**agent/skills.rs:172-198** - Add aggregate error reporting for skill loading:
- Errors logged individually via `tracing::warn!` but no summary returned
- Callers have no way to know how many skills failed

### 2.6 CLI Improvements

**cli/mod.rs** - Standardize timeout defaults:
- Timeout values scattered across different arg structs
- Create a `timeout` constants module

**cli/commands/handlers/network.rs:117-162** - Add bounds validation for `max_hops`:
- `max_hops: u8` has no `#[arg(value_parser)]` constraint
- Value of 0 is valid for u8 but nonsensical
- Validate 1-255 range

### 2.7 TUI Improvements

**tui/components/input.rs:568-596** - Add bounds checks to InputGroup methods:
- `focus_prev()` and `focus()` methods may have edge cases
- `InputField::insert()` (line 115) could panic if `cursor_pos > value.len()`
- Verify cursor_pos bounds are maintained by movement methods

### 2.8 Recon Improvements

**recon/mod.rs:348-366** - Add `dependency_scan` to `FULL_RECON_PIPELINE_MODULES` or update docs:
- Module-level doc comment at line 37 lists `dependency_scan` as part of pipeline
- But `FULL_RECON_PIPELINE_MODULES` constant doesn't include it

### 2.9 Loadtest Improvements

**loadtest/runner.rs:306-317** - Rate limit lock contention:
- Every worker acquires mutex per request
- Should use per-worker counters or channel-based rate limiting

**loadtest/runner.rs:322-333** - Missing request cancellation on timeout:
- In-flight requests not gracefully cancelled
- Need `JoinSet::abort()` and cancellation tokens

### 2.10 CLI Config Override

**cli/commands/handlers/ai_analyze.rs** - AiAnalyze handler loads its own config:
- Calls `load_config(None)` instead of using `ctx.config`
- Misses CLI overrides

---

## Wave 3: Low Priority & Documentation (Parallel)

These are cleanup items that can be batched.

### 3.1 Dead Code Cleanup

**waf/bypass/smuggling.rs:298-315** - HTTP/2 dead code:
- `supports_http2_probes()` (line 304) always returns `false`
- H2CUpgrade request generation (line 199) is gated by this, making lines 200-215 dead code
- `Http2Frame` variant in `SmugglingType` (line 26) is never exercised

### 3.2 Documentation Updates

**architecture/waf.md:66** - Fix "Three Sub-Engines" description:
- Text says "across three sub-engines" but lists five categories (Encodings, Header Manipulation, Payload Splitting, Protocol Obfuscation, HTTP Smuggling)

**architecture/config.md:22-41** - Document `Scope::validate()` and `ScopeRule::with_cidr()`:
- `Scope::validate()` (scope.rs:36-71) validates: allowed_targets not empty when require_explicit_scope is true, no duplicate ports, max_requests_per_second in range
- `ScopeRule::with_cidr()` (scope.rs:203-211) creates a scope rule from CIDR notation

**architecture/distributed.md** - Fix line number offsets:
- RemoteClient: doc says remote.rs:395-697, actual is remote.rs:407 (off by 12)
- generate_psk: doc says command.rs:249-254, actual is command.rs:258 (off by 9)
- Multiple other components off by 1-4 lines

**output/diff.rs:136-140** - Add doc comment for `has_regressions()`:
- No doc comment; checks if escalated findings have severity >= High

**output/trend.rs:58-64** - Document `ResultComparator::finding_key()`:
- Generates composite key of `(title, category, cve)` for deduplication
- No doc comment explaining the algorithm

### 3.3 Code Cleanup

**cli/stress.rs:129** - Rename `spoof`/`spoof_range` to descriptive names:
- `spoof: bool` and `spoof_range: Option<String>` naming is inconsistent
- Suggested names may not match field types; use `source_ip_spoof: bool` and `source_ip_range: Option<String>`

**cli/commands/handlers/mod.rs:100-155** - Document which commands bypass scope validation:
- `Resume`, `Config`, `Doctor`, `Notify`, `Remote`, `Exec`, `Storage`, `Report`, `Plugin`, `Cluster` don't call `ensure_scope()`

**tui/session.rs:45** - Allow config to override auto-save interval default:
- Hardcoded to 30 seconds in `Default for SessionConfig`

**tui/session.rs:130-156** - Consolidate bookmark deduplication loops:
- Two separate loops (stable IDs at 141-145, legacy at 147-153) could be consolidated

**config/scope.rs:36-71** - Add doc comments for `Scope::validate()`:
- Checks: allowed_targets not empty, no duplicate ports, max_requests_per_second in 1-10000

**distributed/remote.rs:447-455** - Add DNS cache validation:
- `resolve_cached()` trusts cached address unconditionally within 60-second TTL
- No validation that cached address is still reachable

**distributed/remote.rs:347-356** - Validate worker capabilities:
- Coordinator echoes worker's claimed capabilities without validation against `CAPABILITIES` (mod.rs:83-91)
- Malicious worker could advertise arbitrary capabilities

### 3.4 Future Enhancements (Deferred)

- **ai_agents MCP integration**: Implement Model Context Protocol support (High effort)
- **recon IMDSv1/v2 testing**: AWS/GCP/Azure metadata endpoint testing
- **recon dependency_scan pipeline**: Wire `dependency_scan` module into `run_full_recon` pipeline

---

## Verification Commands

```bash
# Core checks
cargo check --lib -p slapper
cargo check --lib -p slapper-plugin
cargo check --lib -p slapper-ruby
cargo check -p slapper-nse

# Tests
cargo test --lib -p slapper
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper

# Lint
cargo clippy --lib -p slapper
cargo clippy --lib -p slapper-plugin
cargo clippy --lib -p slapper-ruby
```

---

## Module Health Summary

| Module | Health | Key Issues |
|--------|--------|------------|
| config | Excellent | Documentation gaps only |
| output | Good | Template unwrap, minor inconsistencies |
| scanner | Good | Clone per request, packet trace leak |
| tui | Good | Bounds checking, auto-save config |
| recon | Fair | Dead code stubs, pipeline docs |
| waf | Fair | Cookie matching, HTTP/2 dead code |
| loadtest | Fair | Rate limiting burst, lock contention |
| networking | Fair | IPv6 entropy, traceroute concurrency |
| ai_agents | Fair | Rate limit persistence, MCP gap |
| cli_commands | Needs Work | Resume scope bypass, timeout defaults |
| distributed | Needs Work | Core result system broken, stats hardcoded |
