# Slapper Consolidated Remediation Plan

Merged from plan.md, plan2.md, plan3.md, plan4.md, and plan6.md.
Execution order: security/critical bugs first, then high bugs, then medium fixes, then low/cleanup, then deferred architectural items.

---

## Pre-Work: Known Compilation Issues

Before any work begins, these compilation blockers must be resolved:

| Issue | Feature Flag | Details | Status |
|-------|-------------|---------|--------|
| PyO3 incompatible with Python 3.14 | `python-plugins` | PyO3 0.24.2 max is 3.13; needs `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` or PyO3 upgrade | Open |
| rb-sys stable API missing | `ruby-plugins` | Needs `stable-api-compiled-fallback` feature or rb-sys update | Open |
| `dyn StdError` not Send/Sync | `nse` | `executor.rs:59-82` — `run_script_with_timeout` channel type issue | **DONE** |

**Verification:** `cargo check --lib -p slapper --features full`

---

## Critical Priority

### 1. REST API Timing Attack (Security)

**Source:** plan2.md #1
**File:** `crates/slapper/src/tool/protocol/rest.rs:181`
**Problem:** `require_auth()` compares API keys with `==`. Both `mcp.rs:81` and `grpc.rs:27` use `subtle::ConstantTimeEq::ct_eq()`, but REST does not. Timing side-channel vulnerability.

**Fix:**
- Add `use subtle::ConstantTimeEq;` to rest.rs
- Change line 181 from `Some(v) if v == key =>` to constant-time comparison using `ct_eq()`

**Status:** DONE
**Verification:** Existing REST API tests must pass.

---

## High Priority

### 2. Spoofed Scanning: TCP Checksum Not Computed (Bug)

**Source:** plan2.md #2
**Files:** `crates/slapper/src/scanner/spoof.rs:358,407`
**Problem:** `build_tcp_packet()` and `build_fragmented_packets()` set `tcp_packet.set_checksum(0)` and never compute a real checksum. Target hosts silently drop packets with invalid checksums — spoofed scans always report zero open ports.

**Fix:**
- After setting all TCP fields, compute checksum via `pnet::packet::tcp::ipv4_checksum()`
- Apply to both `build_tcp_packet()` (line 358) and `build_fragmented_packets()` (line 407)
- Add `use pnet::packet::tcp::ipv4_checksum;`

**Status:** DONE

### 3. Spoofed Scanning: Last Fragment Flag Error (Bug)

**Source:** plan2.md #3
**File:** `crates/slapper/src/scanner/spoof.rs:428-432`
**Problem:** Both branches of the if/else set `MoreFragments`. The last fragment must NOT have this flag, or target IP reassembly fails.

**Fix:**
- Track total chunks before loop: `let total_chunks = tcp_data.chunks(fragment_size).len();`
- Only set `MoreFragments` when `i < total_chunks - 1`

**Status:** DONE

### 4. Burst Mode Ignores Payloads (Bug)

**Source:** plan2.md #4
**File:** `crates/slapper/src/fuzzer/engine/execution.rs:161-176`
**Problem:** `run_burst_with_session()` iterates payloads with `_p` (underscore = dropped). Sends plain GET to base URL ignoring the payload entirely. Contrast with `run_sequential_with_session()` which correctly calls `self.send_fuzz_request(&p)`.

**Fix:**
- Remove underscore from `_p` to bind as `p`
- Build actual fuzz request using payload (method, body, headers, parameters) instead of bare `client.get(&url)`

**Status:** DONE

### 5. `expect()` Calls in Hot Paths (Robustness)

**Source:** plan3.md #2, plan2.md #10
**Status:** DONE. Replaced `ProgressStyle::template().unwrap()` with `.unwrap_or_else(|_| ProgressStyle::default_bar())` in 6 locations: `scanner/ports/mod.rs`, `scanner/endpoints.rs`, `scanner/fingerprint.rs`, `stress/http.rs`, `pipeline/executor.rs`, `fuzzer/engine/execution.rs` (x2). Replaced all `duration_since(UNIX_EPOCH).unwrap()` with `.unwrap_or_default()` in NSE libraries.

### 6. Inconsistent Error Handling in proxy/mod.rs (Robustness)

**Source:** plan3.md #2c
**File:** `crates/slapper/src/proxy/mod.rs:41`
**Problem:** `HealthChecker::new` returns `Result` but uses `.expect()`. Should use `?`.

**Status:** DONE

---

## Medium Priority

### 7. Invalid XML Port Scan Output (Bug)

**Source:** plan2.md #5
**File:** `crates/slapper/src/scanner/ports/mod.rs:228-234`
**Problem:** `<port>`, `<state>`, `<service>` are siblings, not proper nested XML. Nmap-style XML expects ports with attributes or nested elements.

**Fix:** Restructure to nmap-compatible format:
```xml
<port number="80" protocol="tcp">
  <state state="open"/>
  <service name="http"/>
</port>
```

**Status:** DONE

### 8. Constant Mismatch: DEFAULT_MAX_REDIRECTS (Correctness)

**Source:** plan2.md #6
**Files:** `constants.rs:31` (`= 5`) vs `config/settings.rs:487` (`= 10`)
**Problem:** Config default returns 10, constant says 5. Constant is never referenced by actual code. Types also differ (u32 vs usize).

**Fix:** Update `constants::http::DEFAULT_MAX_REDIRECTS` from `5` to `10`.

**Status:** DONE

### 9. Hardcoded BLOCKED_STATUS_CODES Arrays (Maintainability)

**Source:** plan2.md #7
**Files:**
- `waf/bypass/evasion.rs:265`
- `waf/bypass/headers.rs:201`
- `waf/bypass/smuggling.rs:285`
- `waf/waf_patterns.rs:517-519`

**Problem:** `[403, 406, 429, 503]` duplicated in 4 places. Canonical constant exists at `constants::waf::BLOCKED_STATUS_CODES` but is only used by `detector.rs`.

**Fix:** Replace all inline arrays with `crate::constants::waf::BLOCKED_STATUS_CODES`. Change `get_blocked_status_codes()` to return `BLOCKED_STATUS_CODES.to_vec()`.

**Status:** DONE

### 10. Silent Error Swallowing in Recon (Observability)

**Source:** plan2.md #11
**File:** `crates/slapper/src/recon/mod.rs:305-393`
**Problem:** All 14 recon module invocations use `.ok()`, silently discarding errors with no logging.

**Fix:** Replace with `match` + `tracing::warn!()`:
```rust
match reverse_dns::reverse_dns_lookup(ip).await {
    Ok(v) => Some(v),
    Err(e) => { tracing::warn!("reverse DNS lookup failed: {e}"); None }
}
```

**Status:** DONE

### 11. Blocking HTTP Clients in Async Context (Performance)

**Source:** plan2.md #12
**Files:**
- `recon/cve_lookup.rs:43,175` — `CveMapper::lookup_cve()`, `match_technology_cves()`
- `recon/asn.rs:32,123,167` — `AsnLookup::lookup()`, `lookup_by_ip()`, `get_prefixes()`

**Problem:** Synchronous `reqwest::blocking::Client` called from `recon/mod.rs` inside `tokio::join!`. Can starve the async executor.

**Fix:** Wrap calls in `tokio::task::spawn_blocking()` in `recon/mod.rs` (lower risk than full async conversion).

**Status:** NOT NEEDED (blocking clients are not used in async recon path; only used in NSE tests).

### 12. WAF Evasion: 3xx Redirects Treated as Success (Logic)

**Source:** plan2.md #13
**File:** `crates/slapper/src/waf/bypass/evasion.rs:265-266`
**Problem:** `is_bypass_successful()` checks `status < 400` which includes 3xx redirects. A redirect could be a WAF block page.

**Fix:** Tighten to `!blocked_codes.contains(&status) && status >= 200 && status < 300`.

**Status:** DONE

### 13. Logging Investigation (Architecture)

**Source:** plan4.md #1.2
**Finding:** The codebase already uses `tracing` extensively (95+ calls). `println!/eprintln!` exist in user-facing CLI output (expected for a CLI tool). This is NOT a logging problem — `tracing` is properly integrated.

**Action:** Audit `println!` calls to confirm they are user-facing output, not diagnostic logging. Convert any diagnostic `println!` to `tracing::info!`/`debug!`.

**Status:** DONE (tracing integrated; audit complete — all `println!`/`eprintln!` in library code are in CLI-facing `run_cli()` methods or output utilities, which is appropriate for a CLI tool).

### 14. Plugin Directory Defaults Unification (Incomplete from plan.md)

**Source:** plan6.md #1, plan.md Phase 2
**Problem:** Three different default directory lists:
1. `slapper-plugin::PluginManager::default_plugin_dirs()` — canonical, accepts optional config_dir
2. `slapper-ruby::PluginLoader::new()` — accepts dirs as parameter (already updated)
3. `commands/handlers/plugin.rs:8-19` — local `default_plugin_dirs()` duplicates logic

**Fix:**
- `commands/handlers/plugin.rs:25` should call `crate::plugin::PluginManager::default_plugin_dirs(None)` instead of local function
- Delete local `default_plugin_dirs()` at plugin.rs:8-19
- `slapper-ruby::PluginLoader::new()` already accepts `Vec<PathBuf>` — no change needed

**Verification:** `cargo check --lib -p slapper --features python-plugins,ruby-plugins`
**Status:** DONE

### 15. NSE Timeout Thread Safety — DONE

**Source:** plan6.md #3, plan.md Phase 5.3
**Status:** Completed. `run_script_with_timeout()` now converts `mlua::Error` to `String` before sending through channel, then converts back to `mlua::Error::RuntimeError` on receive side.

---

## Low Priority

### 16. Dead Code: scanner/scan_helper.rs (Cleanup)

**Source:** plan2.md #8
**File:** `crates/slapper/src/scanner/scan_helper.rs` (88 lines)
**Problem:** File exists but is never declared as a module (`mod scan_helper;` not in `scanner/mod.rs`). Orphan file, never compiled.

**Fix:** Delete the file.

**Status:** DONE

### 17. Dead Code: constants::severity Module (Cleanup)

**Source:** plan2.md #9
**File:** `crates/slapper/src/constants.rs:21-27`
**Problem:** Defines `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, `INFO` string constants. No code references `constants::severity::`. The `Severity::as_str()` method provides the same values.

**Fix:** Remove the `pub mod severity { ... }` block.

**Status:** DONE

### 18. Heavy Arc<Mutex> Usage Review (Architecture)

**Source:** plan4.md #1.3
**Finding:** 31 instances of `Arc<Mutex<T>>`/`Arc<RwLock<T>>`. Potential lock contention in async context.

**Fix:** Audit for potential deadlocks and performance bottlenecks. Consider tokio async mutexes, channels, or lock-free structures where appropriate. This is a large-scale refactor — scope per-file.

### 19. Stub Encoder Implementations (Correctness)

**Source:** plan.md Phase 3.6 (marked complete with error messages)
**File:** `crates/slapper-ruby/src/api.rs:934-949`
**Problem:** `encoder_encode()` and `encoder_compatible_payloads()` return `Err("not yet implemented")`. They are callable from Ruby plugins but will always fail.

**Fix:** Delegate to MSF RPC via `msf_execute_module("encoder", ...)` or leave as-is if encoder integration is not a priority.

---

## Deferred (Architectural — Lower Priority)

These items require more design work or are lower impact:

| Item | Source | Description | Status |
|------|--------|-------------|--------|
| Unified Plugin trait | plan.md #8.1 | Define `Plugin` trait in `slapper-plugin` for all backends | **DONE** |
| Python class-based plugins | plan.md #8.2 | Support `class MyScanner(Plugin)` pattern | **DONE** |
| Plugin documentation | plan.md #8.3 | Create `docs/plugins/` with developer guides | **DONE** |
| Plugin sandboxing | plan.md #8.4 | NSE: disable dangerous Lua libs; Python/Ruby: process isolation | **DONE** |
| Output consolidation | plan2.md #14 | Merge `output/convert.rs` with dedicated builder modules | **DONE** |
| Split Commands enum | plan4.md #3.2 | CLI `Commands` enum has 26 variants, could split into subcommands | **DONE** |
| Review unwrap() count | plan3.md #3 | ~423 `.unwrap()`/`.unwrap_or()` calls across codebase — audit for edge cases | **PARTIAL** (hot paths + NSE done) |

---

## Execution Order

```
Pre-Work: Fix compilation issues (Python 3.14, Ruby stable API, NSE thread safety)

Security & Critical Bugs:
  1. REST API timing attack (security)

High Bugs:
  2. Spoofed scanning TCP checksum (bug)
  3. Spoofed scanning fragment flags (bug)
  4. Burst mode payload drop (bug)
  5. expect() in hot paths (robustness)
  6. proxy/mod.rs error handling (robustness)

Medium Fixes:
  7. XML port scan output (bug)
  8. DEFAULT_MAX_REDIRECTS constant (correctness)
  9. BLOCKED_STATUS_CODES consolidation (maintainability)
  10. Silent error swallowing in recon (observability)
  11. Blocking HTTP clients in async (performance)
  12. WAF evasion 3xx logic (logic)
  13. Logging audit (architecture)
  14. Plugin directory defaults (incomplete from plan.md)
  15. NSE timeout thread safety (incomplete from plan.md)

Low Cleanup:
  16. Delete scan_helper.rs dead code
  17. Delete constants::severity module
  18. Arc<Mutex> usage review
  19. Stub encoder implementations

Deferred:
  Unified Plugin trait, class-based plugins, docs, sandboxing,
  output consolidation, Commands enum split, unwrap() audit
```

---

## Verification Commands

After each phase:

```bash
# Check base compilation (no features)
cargo check --lib -p slapper

# Check individual feature flags (after fixing compilation issues)
cargo check --lib -p slapper --features python-plugins
cargo check --lib -p slapper --features ruby-plugins
cargo check --lib -p slapper --features nse

# Check full feature set
cargo check --lib -p slapper --features full

# Run tests
cargo test --lib -p slapper --features full

# Lint
cargo clippy --lib -p slapper --features full -- -D warnings
```

---

## Success Criteria

| Criterion | Target |
|-----------|--------|
| REST API auth | Constant-time comparison (was `==`) |
| Spoofed TCP checksum | Computed (was 0) |
| Fragment flags | Last fragment no `MoreFragments` |
| Burst mode payloads | Used (was dropped) |
| XML output | Valid nested XML (was malformed) |
| DEFAULT_MAX_REDIRECTS | 10 (was 5) |
| BLOCKED_STATUS_CODES | Single source of truth (was 4 copies) |
| Recon error logging | `tracing::warn!` (was `.ok()`) |
| Plugin directory resolution | Single canonical function |
| NSE timeout | Compiles with `nse` feature |
| All features | Compile with `--features full` |
| Existing tests | All passing (328+) |
| Clippy warnings | 0 |

---

## Remaining Issues (Post-Remediation)

### A. Ruby Plugin Compilation Issues

#### A1. Magnus API Compatibility (Critical) — FIXED
**Files:** `crates/slapper-plugin/src/ruby.rs`
**Status:** DONE. `slapper-plugin` now uses magnus 0.8 and the Ruby code was rewritten for the 0.8 API:
- `eval::<()>` replaced with `let _: Value = eval(...)`
- `funcall` uses explicit `Value` return types
- Hash field extraction uses `RHash::lookup` + `String::try_convert` instead of `funcall("get", ...)` + `to_s()`
- `ruby-plugins` feature now includes `dep:magnus` in `slapper-plugin/Cargo.toml`
- `slapper/Cargo.toml` `ruby-plugins` feature now includes `dep:slapper-plugin`

#### A2. Thread Safety for RubyPluginAdapter (Critical)
**Files:** `crates/slapper-ruby/src/loader.rs:133`
**Problem:** `RubyPluginAdapter` cannot implement `Plugin` trait because:
- `Plugin` trait requires `Send + Sync`
- `Ruby` contains `PhantomData<*mut ()>` which is not `Send`/`Sync`
- `Arc<Mutex<RubyBridge>>` approach doesn't fully solve the issue

**Root Cause:** Magnus `Ruby` type is not thread-safe by design due to Ruby's GIL.

**Solution Options:**
1. **Make Plugin trait not require Send+Sync** (Breaking change)
   - Change `pub trait Plugin: Send + Sync` to `pub trait Plugin`
   - Update `PluginRegistry` to use `Arc<Mutex<dyn Plugin>>` instead of `Arc<dyn Plugin>`
   - Impact: All plugin implementations need updating

2. **Use thread-local Ruby instance** (Complex)
   - Store `Ruby` instance in thread-local storage
   - Create adapter that accesses thread-local instance
   - More complex but maintains thread safety

3. **Use unsafe Send+Sync implementation** (Risky)
   - `unsafe impl Send for RubyBridge` and `unsafe impl Sync for RubyBridge`
   - Only safe if Ruby GIL is properly held during access
   - Requires careful review of magnus internals

**Recommended:** Option 1 - Change Plugin trait to not require Send+Sync since plugins are inherently not thread-safe due to their runtime dependencies.

#### A3. Function Macro Trait Bounds (Medium)
**Files:** `crates/slapper-ruby/src/api.rs:56-59, 519, 549`
**Problem:** `magnus::function!` macro fails with trait bound errors.

**Root Cause:** Function signatures don't match expected `RubyFunction` trait bounds.

**Solution:**
- Update function signatures to include `&Ruby` parameter as first argument
- Or use `magnus::method!` macro if appropriate
- Check magnus 0.8.x documentation for correct usage

### B. Python Plugin TUI Integration Issues

#### B1. Missing Await for Async Method (Medium) — FIXED
**Files:** `crates/slapper/src/commands/handlers/plugin.rs:81`
**Status:** DONE. The `.await?` is already present on line 81.

#### B2. TUI App Structure Missing Plugin Field (Medium)
**Files:** `crates/slapper/src/tui/ui.rs:442, 599`
**Problem:** `app.plugin` field doesn't exist in `App` struct.

**Root Cause:** TUI plugin tab was partially implemented.

**Solution:**
- Add `plugin` field to `App` struct in `crates/slapper/src/tui/app.rs`
- Initialize plugin state in `App::new()`
- Or remove plugin tab references if not needed

#### B3. Lifetime Issue in Plugin Results (Low)
**Files:** `crates/slapper/src/tui/tabs/plugin.rs:111`
**Problem:** `results.findings` borrowed but doesn't live long enough.

**Solution:**
- Clone findings data instead of borrowing
- Or use owned `String` types instead of `&str` references

### C. Implementation Order

1. **Phase 1: Magnus Upgrade** (High Priority)
   - Upgrade magnus to 0.8.2 in `slapper-ruby/Cargo.toml`
   - Update API calls in `api.rs` to match 0.8.x
   - Fix function macro usage
   - Test compilation

2. **Phase 2: Plugin Trait Refactor** (High Priority)
   - Remove `Send + Sync` requirement from `Plugin` trait
   - Update `PluginRegistry` to use `Arc<Mutex<dyn Plugin>>`
   - Update all plugin implementations
   - Test compilation

3. **Phase 3: Python Plugin Fixes** (Medium Priority)
   - Add `await` to async call in plugin handler
   - Add plugin field to TUI App struct
   - Fix lifetime issues in plugin results tab
   - Test compilation with `--features python-plugins`

4. **Phase 4: Integration Testing**
   - Test Ruby plugins with `--features ruby-plugins`
   - Test Python plugins with `--features python-plugins`
   - Test full feature set with `--features full`
   - Run existing test suite

### D. Verification Commands

After each phase:

```bash
# Check Ruby plugin compilation
cargo check --lib -p slapper-ruby --features ruby-plugins
cargo check --lib -p slapper --features ruby-plugins

# Check Python plugin compilation
cargo check --lib -p slapper-plugin --features python-plugins
cargo check --lib -p slapper --features python-plugins

# Check full compilation
cargo check --lib -p slapper --features full

# Run tests
cargo test --lib -p slapper --features full

# Lint
cargo clippy --lib -p slapper --features full -- -D warnings
```

### E. Success Criteria

| Criterion | Current Status | Target |
|-----------|----------------|--------|
| Ruby plugins compile | ✅ Compiles with `--features ruby-plugins` | ✅ Compiles with `--features ruby-plugins` |
| Python plugins compile | ✅ Compiles with `--features python-plugins` | ✅ Compiles with `--features python-plugins` |
| Plugin trait thread safety | ⚠️ Requires Send+Sync (deferred) | ✅ No requirement for non-thread-safe runtimes |
| All features compile | ⚠️ `python-plugins` + `ruby-plugins` compile separately; `full` needs NSE timeout | ✅ Compiles with `--features full` |
| Existing tests pass | ✅ 328 | ✅ All passing |
| Clippy warnings | ✅ 0 | ✅ 0 |
