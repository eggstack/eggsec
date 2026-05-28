# Slapper Consolidated Implementation Plan

**Created:** 2026-05-28 (from 13 review files)
**Status:** Ready for implementation

---

## Priority Index

| # | Severity | Module | Issue | Effort | Wave |
|---|----------|--------|-------|--------|------|
| 1 | CRITICAL | distributed | Task results never sent to coordinator (BUG 3) | Low | 1 |
| 2 | HIGH | distributed | WorkerStats never updated (BUG 1) | Medium | 1 |
| 3 | HIGH | distributed | Heartbeat reports static zeros (BUG 2, 4) | Medium | 1 |
| 4 | HIGH | cli_commands | Resume command bypasses scope validation | Low | 1 |
| 5 | HIGH | cli_commands | load_passwords path traversal risk | Low | 1 |
| 6 | HIGH | loadtest | Rate limiting initial burst | Low | 1 |
| 7 | MEDIUM | distributed | Worker registration flow incomplete | Medium | 2 |
| 8 | MEDIUM | distributed | No task assignment pull mechanism | High | 3 |
| 9 | MEDIUM | distributed | No graceful worker shutdown | Medium | 2 |
| 10 | MEDIUM | distributed | Connection cleanup on panic | Medium | 2 |
| 11 | MEDIUM | distributed | Rate limit cleanup unbounded | Low | 2 |
| 12 | MEDIUM | networking | RateLimiter spin loop at low pps | Low | 1 |
| 13 | MEDIUM | networking | IPv6 spoof range entropy | Low | 1 |
| 14 | MEDIUM | networking | Traceroute no concurrency limit | Low | 2 |
| 15 | MEDIUM | networking | HTTP stress lacks response validation | Low | 1 |
| 16 | MEDIUM | networking | TLS SNI extraction not implemented | Medium | 3 |
| 17 | MEDIUM | scanner | Clone-on-every-request in endpoint scan | Low | 2 |
| 19 | MEDIUM | scanner | Packet trace file handle leak | Low | 2 |
| 20 | MEDIUM | waf | Cookie matching fallible index lookup | Low | 1 |
| 21 | MEDIUM | waf | compare_responses creates new client | Low | 1 |
| 22 | MEDIUM | waf | Missing circuit breaker on detection | Medium | 2 |
| 23 | MEDIUM | output | PDF truncates findings to 30 silently | Low | 1 |
| 24 | MEDIUM | output | Template registration unwrap() panic | Low | 1 |
| 25 | MEDIUM | ai_agents | Rate limit counter never resets | Medium | 2 |
| 26 | MEDIUM | ai_agents | Knowledge base eviction bug | Medium | 2 |
| 27 | MEDIUM | ai_agents | MCP integration not implemented | High | 3 |
| 28 | MEDIUM | cli_commands | Proxy handler missing scope validation | Low | 1 |
| 29 | MEDIUM | cli_commands | Timeout defaults not standardized | Low | 2 |
| 30 | MEDIUM | cli_commands | handle_no_command references non-existent field | Low | 1 |
| 31 | MEDIUM | tui | Duplicate key binding 'b' | Low | 2 |
| 32 | MEDIUM | tui | InputGroup bounds checking in reset | Low | 1 |
| 33 | MEDIUM | recon | ThreatStream API key hardcoded to None | Low | 1 |
| 34 | LOW | recon | FullReconResult callback uses HashMap | Low | 2 |
| 35 | LOW | recon | dependency_scan not in pipeline | Low | 3 |
| 39 | LOW | waf | HTTP/2 smuggling dead code | Low | 3 |
| 40 | LOW | waf | WAF count mismatch in docs | Low | 3 |
| 41 | LOW | scanner | Duplicate Memcached probe entry | Low | 1 |
| 42 | LOW | scanner | ICMP probe unused timeout param | Low | 2 |
| 43 | LOW | scanner | UDP fingerprinting no rate limit | Low | 2 |
| 44 | LOW | networking | DNS compression pointer loop limit too low | Low | 1 |
| 45 | LOW | output | convert_to_csv returns String not Result | Low | 3 |
| 46 | LOW | output | ResultComparator finding key undocumented | Low | 3 |
| 47 | LOW | ai_agents | Test code uses std HashMap | Low | 2 |
| 48 | LOW | ai_agents | load_skills silently skips invalid | Low | 2 |
| 49 | LOW | cli_commands | StressArgs naming inconsistency | Low | 3 |
| 50 | LOW | cli_commands | Several handlers don't use CommandContext | Low | 3 |
| 51 | LOW | cli_commands | traceroute max_hops no bounds validation | Low | 2 |
| 52 | LOW | tui | Auto-save interval hardcoded despite config | Low | 3 |
| 53 | LOW | tui | SessionState bookmarks not deduplicated | Low | 3 |
| 54 | LOW | config | Scope validation docs missing | Low | 3 |
| 55 | LOW | distributed | DNS rebinding protection gap | Low | 3 |
| 56 | LOW | distributed | Worker capabilities not validated | Low | 3 |
| 57 | LOW | distributed | Documentation line number offsets | Low | 3 |

---

## Wave 1: Critical & Quick High-Impact Fixes (Parallel)

These items are independent and can all be worked on simultaneously.

### 1.1 Distributed: Send Task Results to Coordinator (CRITICAL)

**File:** `crates/slapper/src/distributed/worker.rs:166-182`

**Problem:** `start_task_processing_loop()` spawns tasks but drops `TaskResult` - never sends back via `CommandMessage::Result`. The entire distributed result aggregation system is non-functional.

**Fix:**
- In the spawned task at line 172-177, after `process_task(task).await`, send the result back through a channel or via `RemoteClient::report_result()`
- Need to pass a `tokio::sync::mpsc::Sender<CommandMessage>` or similar into the task processor
- The receiver side at `remote.rs:372` already handles `CommandMessage::Result`

**Verification:** After fix, ensure worker sends `CommandMessage::Result { id, result }` for every completed task.

### 1.2 Distributed: Update WorkerStats and Heartbeat (HIGH)

**File:** `crates/slapper/src/distributed/worker.rs:56-82, 151-157`

**Problem:** `WorkerStats` fields are initialized to 0 and never updated. Heartbeat always reports `"status": "idle"`, `"current_jobs": 0`.

**Fix:**
- Update `stats.tasks_completed` / `stats.tasks_failed` / `stats.tasks_in_progress` in `start_task_processing_loop()` and `process_task()`
- In heartbeat (line 151-157), use actual `self.stats` values instead of hardcoded zeros
- Track actual `WorkerStatus` (idle/busy) based on in-progress task count

### 1.3 CLI: Add Scope Validation to Resume (HIGH)

**File:** `crates/slapper/src/commands/handlers/scan.rs:60-63`

**Problem:** `handle_resume(args: ResumeArgs)` takes only args without `CommandContext`, so session files can contain targets outside configured scope.

**Fix:**
- Change signature to `handle_resume(args: ResumeArgs, ctx: CommandContext)`
- After loading session, validate all targets against `ctx.config.scope`
- Reject session if any target is out of scope

### 1.4 CLI: Add Path Validation to load_passwords (HIGH)

**File:** `crates/slapper/src/commands/handlers/auth_test.rs:274-296`

**Problem:** `load_passwords` reads files without path validation. Relative paths like `../../etc/passwd` could be exploited.

**Fix:**
- Use `validate_path_string()` or similar path safety check before reading
- Reject paths that escape the working directory

### 1.5 Loadtest: Fix Rate Limiting Initial Burst (HIGH)

**File:** `crates/slapper/src/loadtest/runner.rs:275-281`

**Problem:** `next_allowed_at` initialized to `TokioInstant::now() - min_interval` lets all initial workers through simultaneously.

**Fix:** Change line 279 from:
```rust
Arc::new(Mutex::new(TokioInstant::now() - min_interval)),
```
to:
```rust
Arc::new(Mutex::new(TokioInstant::now())),
```

### 1.6 Networking: Fix RateLimiter Spin Loop (MEDIUM)

**File:** `crates/slapper/src/stress/metrics.rs:107-162`

**Problem:** Atomic spin loop at `target_pps=1` causes 100% CPU usage.

**Fix:** Replace spin loop with `tokio::sync::Semaphore` or add minimum sleep time.

### 1.7 Quick Fixes (MEDIUM/LOW, batch together)

- **waf/detector/detect.rs:105-110**: Fix cookie matching `unwrap_or(0)` - return early if pattern not found
- **waf/detector/compare.rs:14-18**: Reuse `self.client` instead of creating new client
- **output/pdf.rs:80**: Add warning when `findings.len() > 30`
- **output/template.rs:141-152**: Change `.unwrap()` to `.expect("template registration should never fail")`
- **cli/handlers/stress.rs:9**: Add scope validation to stress handler
- **cli/handlers/stress.rs:58**: Add scope validation to proxy handler
- **cli/handlers/mod.rs:160**: Fix `handle_no_command` to not reference `cli.config`
- **scanner/fingerprint.rs:29,54**: Remove duplicate Memcached entry from PROBES array
- **packet/validation.rs:36-38**: Change DNS compression pointer limit from 10 to 100
- **stress/http.rs:99-107**: Add status code checking to HTTP stress
- **recon/threatintel.rs:65**: Wire up `threatstream_key` from config or remove dead code
- **tui/tabs/fuzz.rs:404-413**: Add bounds check in reset() method

---

## Wave 2: Medium Priority Improvements (Parallel)

These items are independent of each other but depend on Wave 1 completion.

### 2.1 Distributed Module Improvements

**Files:** `worker.rs`, `remote.rs`

- **IMPROVEMENT 3** (worker.rs:64-104): Add `shutdown()` method with cancellation channel
- **IMPROVEMENT 4** (remote.rs:207-211): Wrap `handle_connection()` with panic catch or use `Arc<Mutex<>>`
- **IMPROVEMENT 5** (remote.rs:119-138): Add periodic cleanup of stale rate limit entries

### 2.2 Networking Improvements

- **packet/traceroute.rs:141-168**: Add semaphore to limit concurrent traceroute probes
- **packet/parse_impl.rs:592-634**: Extract SNI and certificates from TLS handshake
- **stress/syn.rs:284-289**: Fix IPv6 spoof range host_bits calculation

### 2.3 Scanner Improvements

- **scanner/endpoints.rs:742-753**: Use `Arc<SpoofConfig>` instead of cloning per request
- **scanner/ports/spoofed.rs:55-56**: Add `shutdown_packet_trace()` or bounded cleanup
- **scanner/icmp_probe.rs:32**: Add `tokio::time::timeout()` wrapper around ping loop
- **scanner/udp_fingerprint.rs:140**: Add token bucket rate limiting for UDP probes

### 2.4 WAF Improvements

- **waf/detector/detect.rs**: Add circuit breaker pattern from `utils/circuit_breaker.rs`
- **waf/detector/detect.rs:196-203**: Use `url::Url::parse()` in `normalize_url_static`
- **waf/bypass/mod.rs:70**: Use `response_diff` field for confidence scoring or remove

### 2.5 AI Agents Improvements

- **agent/constraints/checker.rs:215-228**: Implement background task or time-based rate limit reset
- **ai/waf_bypass.rs:80-88**: Fix knowledge base eviction to keep entries with `failed_attempts < 3`
- **agent/portfolio.rs:572**: Replace `std::collections::HashMap` with `FxHashMap`
- **agent/skills.rs:172-198**: Add aggregate error reporting for skill loading

### 2.6 CLI Improvements

- **cli/mod.rs**: Standardize timeout defaults - create constants module
- **cli/handlers/network.rs:117-162**: Add bounds validation for `max_hops` (1-255)

### 2.7 TUI Improvements

- **tui/app/key_handler.rs:114,124**: Resolve duplicate key binding 'b'
- **tui/components/input.rs:532**: Add `is_empty()` guard to `focus_next()`
- **tui/components/input.rs:568-596**: Add bounds checks to `insert()`, `backspace()`, `delete()`

### 2.8 Recon Improvements

- **recon/mod.rs:221,253**: Replace `std::collections::HashMap` with `FxHashMap` in `FullReconResult`

---

## Wave 3: Low Priority & Documentation (Parallel)

These are cleanup items that can be batched.

### 3.1 Dead Code Cleanup

- **waf/smuggling.rs:298-315**: Implement HTTP/2 or remove dead code paths (`supports_http2_probes()` always returns false)
- **recon/mod.rs:347-365**: Add `dependency_scan` to `FULL_RECON_PIPELINE_MODULES` or update docs
- **NOTE:** ExploitDB, Alexa, zone transfer dead code items from reviews appear to have been already removed from the codebase. Verify before implementing.

### 3.2 Documentation Updates

- **architecture/waf.md**: Fix WAF count (25 → 34), fix "Three Sub-Engines" description
- **architecture/config.md**: Document `Scope::validate()` and `ScopeRule::with_cidr()`
- **architecture/distributed.md**: Fix line number offsets
- **output/diff.rs:136-140**: Fix `has_regressions()` documentation
- **output/trend.rs:55-61**: Document `ResultComparator` finding key algorithm

### 3.3 Code Cleanup

- **recon/mod.rs:221**: Convert `FullReconResult` callback metadata to `FxHashMap`
- **agent/portfolio.rs:572**: Replace test HashMap with FxHashMap
- **cli/stress.rs:129**: Rename `spoof`/`spoof_range` to `source_ip`/`source_port`
- **cli/handlers/mod.rs**: Document which commands don't need scope validation
- **tui/session.rs:45**: Allow config to override auto-save interval default
- **tui/session.rs:124-145**: Clean up bookmark deduplication logic
- **config/scope.rs:36-71**: Add doc comments for `Scope::validate()` optimization
- **distributed/remote.rs:445-453**: Add DNS cache validation before use
- **distributed/remote.rs:345-354**: Validate worker capabilities against `CAPABILITIES`

### 3.4 Future Enhancements (Deferred)

- **ai_agents MCP integration**: Implement Model Context Protocol support (High effort)
- **recon IMDSv1/v2 testing**: AWS/GCP/Azure metadata endpoint testing
- **recon dependency_scan pipeline**: Add to standard recon workflow

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
| output | Good | PDF truncation, minor inconsistencies |
| scanner | Good | Regex caching, minor cleanup |
| tui | Good | Bounds checking, key binding |
| recon | Fair | Dead code stubs, pipeline docs |
| waf | Fair | Cookie matching, HTTP/2 dead code |
| loadtest | Fair | Rate limiting burst |
| networking | Fair | Spin loop, IPv6 entropy |
| ai_agents | Fair | Rate limit persistence, MCP gap |
| cli_commands | Needs Work | Scope validation gaps, path traversal |
| distributed | Needs Work | Core result system broken |
