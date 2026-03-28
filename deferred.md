# Deferred Items

Items from `fullplan.md` that were skipped, deferred, or require future work.

---

## Previously Completed (Historical Reference)

These items from `fullplan.md` Deferred section were completed in prior sessions:

| Item | Source | Status |
|------|--------|--------|
| Unified Plugin trait | plan.md #8.1 | DONE |
| Python class-based plugins | plan.md #8.2 | DONE |
| Plugin documentation | plan.md #8.3 | DONE |
| Plugin sandboxing | plan.md #8.4 | DONE |
| Output consolidation | plan2.md #14 | DONE |
| Split Commands enum | plan4.md #3.2 | DONE |
| Review unwrap() count | plan3.md #3 | PARTIAL (hot paths + NSE done) |
| REST API timing attack | fullplan.md #1 | DONE |
| Spoofed TCP checksum | fullplan.md #2 | DONE |
| Spoofed fragment flags | fullplan.md #3 | DONE |
| Burst mode payload drop | fullplan.md #4 | DONE |
| expect() in hot paths | fullplan.md #5 | DONE |
| proxy/mod.rs error handling | fullplan.md #6 | DONE |
| XML port scan output | fullplan.md #7 | DONE |
| DEFAULT_MAX_REDIRECTS | fullplan.md #8 | DONE |
| BLOCKED_STATUS_CODES consolidation | fullplan.md #9 | DONE |
| Silent error swallowing in recon | fullplan.md #10 | DONE |
| WAF 3xx redirect logic | fullplan.md #12 | DONE |
| Logging audit | fullplan.md #13 | DONE |
| Plugin directory defaults | fullplan.md #14 | DONE |
| NSE timeout thread safety | fullplan.md #15 | DONE |
| Dead code cleanup | fullplan.md #16-17 | DONE |
| Magnus API compatibility | fullplan.md A1 | DONE |
| Python await fix | fullplan.md B1 | DONE |

---

## Pre-Work: Compilation Blockers (Not Resolved)

| Issue | Feature Flag | Details |
|-------|-------------|---------|
| PyO3 incompatible with Python 3.14 | `python-plugins` | PyO3 0.24.2 max is 3.13; needs `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` env var or PyO3 upgrade. `python-plugins` compiles on Python 3.12 and below. |
| rb-sys stable API missing | `ruby-plugins` | Needs `stable-api-compiled-fallback` feature or rb-sys update. `ruby-plugins` compiles because slapper-ruby uses magnus (not rb-sys directly). |

---

## Remaining Issues: Ruby Plugin Thread Safety

### A2. Thread Safety for RubyPluginAdapter (Critical)

**Files:** `crates/slapper-ruby/src/loader.rs:133`
**Problem:** `RubyPluginAdapter` cannot implement `Plugin` trait because:
- `Plugin` trait requires `Send + Sync`
- Magnus `Ruby` type contains `PhantomData<*mut ()>` which is not `Send`/`Sync`
- Ruby's GIL makes thread safety inherently complex

**Options:**
1. Remove `Send + Sync` requirement from `Plugin` trait (breaking change for all plugin implementations)
2. Use thread-local Ruby instance (complex)
3. Use `unsafe impl Send/Sync` for `RubyBridge` (risky, requires GIL guarantees)

### A3. Function Macro Trait Bounds (Medium)

**Files:** `crates/slapper-ruby/src/api.rs:56-59, 519, 549`
**Problem:** `magnus::function!` macro fails with trait bound errors in magnus 0.8.
**Fix:** Update function signatures to include `&Ruby` as first parameter, or use `magnus::method!`.

---

## Remaining Issues: Python Plugin TUI Integration

### B2. TUI App Structure Missing Plugin Field (Medium)

**Files:** `crates/slapper/src/tui/ui.rs:442, 599`
**Problem:** `app.plugin` field doesn't exist in `App` struct.
**Fix:** Add `plugin` field to `App` struct in `tui/app.rs`, or remove plugin tab references if not needed.

### B3. Lifetime Issue in Plugin Results (Low)

**Files:** `crates/slapper/src/tui/tabs/plugin.rs:111`
**Problem:** `results.findings` borrowed but doesn't live long enough.
**Fix:** Clone findings data or use owned `String` types.

---

## Low Priority Items

### 18. Heavy Arc<Mutex> Usage Review (Architecture)

**Source:** plan4.md #1.3
**Scope:** 16+ instances of `Arc<Mutex<T>>` across codebase (scanner, fuzzer, recon, pipeline, utils, tui, tool/protocol).
**Action:** Audit for deadlocks and lock contention. Consider tokio async mutexes, channels, or lock-free structures per-file. This is a large-scale refactor.

### 19. Stub Encoder Implementations (Correctness)

**Source:** plan.md Phase 3.6
**Files:** `crates/slapper-ruby/src/api.rs` (encoder functions)
**Note:** grep for "not yet implemented" returned no results — may have been removed or reworked already. Verify before tackling.

### Not NEEDED: Blocking HTTP Clients in Async

**Source:** fullplan.md #11
**Status:** NOT NEEDED — blocking clients are not used in async recon path; only used in NSE tests.
