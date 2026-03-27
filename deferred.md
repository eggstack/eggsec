# Deferred Items — Detailed Implementation Plan

**Status:** All 7 items below have been implemented. This file is preserved for historical reference.

Items from `fullplan.md` Deferred section, ordered by dependency and impact.

---

## 1. Unified Plugin Trait — DONE

**Completed:** Added `Plugin` trait (`async-trait`, `Send + Sync`) to `slapper-plugin/src/lib.rs`.
Implemented for `PythonPluginManager` and `RubyPluginAdapter`. Created `PluginRegistry`.

---

## 2. Python Class-Based Plugin Support — DONE

**Completed:** `python.rs` now scans for `PLUGINS = [MyPlugin]` lists, instantiates classes,
calls `run(target, config)`, and extracts findings via proper Python-to-JSON conversion.
Falls back to function-based approach if `PLUGINS` is not found.

---

## 3. Plugin Documentation — DONE

**Completed:** Updated `docs/PLUGINS.md` with both interfaces. Created `docs/NSE_SCRIPTS.md`
(NSE developer guide) and `docs/PLUGIN_DEVELOPMENT.md` (unified overview).

---

## 4. Plugin Sandboxing — DONE

**Completed:** Added `sandbox` feature to `slapper-nse/Cargo.toml`. Created `SandboxConfig`
struct in `lib.rs`. Restricts `io.popen` (blocked by default), `io.open` path traversal,
`os.getenv` (returns empty), `os.setenv`/`os.unsetenv` (blocked), `os.remove`/`os.rename`/`os.chdir`
(restricted to sandbox dir). Config threaded through `ExecutorCore` and `Executor`.

---

## 5. Output Consolidation — DONE

**Completed:** Added `From<&ScanReportData> for ScanSummary` and `From<&FindingData> for Finding`
impls in `convert.rs`. `report.rs` now uses `HtmlReport` builder for HTML output.

---

## 6. Split Commands Enum — DONE

**Completed:** Commands reorganized into logical groups (scan, attack, recon, tool, infrastructure)
with backward-compatible aliases (`alias = "scan-ports"`, `alias = "waf-stress"`, `alias = "mcp-serve"`).

---

## 7. Unwrap/Expect Audit — PARTIAL

**Completed (this pass):**
- Tier 1: 4 `ProgressStyle::template().unwrap()` replaced with `.unwrap_or_else(|_| ProgressStyle::default_bar())`
- Tier 2: All `duration_since(UNIX_EPOCH).unwrap()` replaced with `.unwrap_or_default()` in NSE libraries
- Tier 3: `.or(Some("url")).unwrap()` replaced with `.unwrap_or("url")` in rest.rs
- Fixed `run_script_with_timeout` Send/Sync issue

**Remaining:** ~100+ other `.unwrap()` calls in non-test code remain unreviewed. The most dangerous
hot-path patterns (progress bars, time operations) are fixed. NSE Lua library time calls are fixed.
The remaining unwraps are in output.rs (writes to Vec, can't fail), test code, and infrequently
reached paths. A full audit with `// SAFETY:` comments is recommended for a future pass.

---

## Remaining Deferred Items (from fullplan.md)

These items were NOT part of the deferred.md scope and remain open:

| Item | Source | Description |
|------|--------|-------------|
| REST API timing attack | fullplan.md #1 | `rest.rs:181` — need `subtle::ConstantTimeEq` |
| Spoofed TCP checksum | fullplan.md #2 | Packets sent with checksum=0 |
| Spoofed fragment flags | fullplan.md #3 | Last fragment has MoreFragments set |
| Burst mode payload drop | fullplan.md #4 | `_p` (underscore) ignores payloads |
| proxy/mod.rs error handling | fullplan.md #6 | `.expect()` in `HealthChecker::new` |
| XML port scan output | fullplan.md #7 | Invalid nested XML structure |
| DEFAULT_MAX_REDIRECTS | fullplan.md #8 | Constant 5 vs default 10 |
| BLOCKED_STATUS_CODES consolidation | fullplan.md #9 | 4 duplicate copies |
| Silent error swallowing in recon | fullplan.md #10 | 14 `.ok()` discarding errors |
| Blocking HTTP in async | fullplan.md #11 | `reqwest::blocking` in tokio context |
| WAF 3xx redirect logic | fullplan.md #12 | 3xx treated as success |
| Dead code cleanup | fullplan.md #16-17 | `scan_helper.rs`, `constants::severity` |
| Arc<Mutex> audit | fullplan.md #18 | 31 instances to review |
| Stub encoder implementations | fullplan.md #19 | Always returns "not yet implemented" |
