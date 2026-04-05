# Consolidated Improvement Plan

> Last updated: 2026-04-05 | Based on comprehensive codebase review
> 400 source files | ~609 library tests | 29 tab variants | 60 TUI files

## Overview

This plan consolidates all improvement items from three source plans (OpenClaw integration, TUI improvements, core library quality) into a single, ordered roadmap. Work is organized into **waves** with **parallelizable blocks** — items within the same block are independent and can be executed concurrently by sub-agents.

### Current State

| Metric | Value |
|--------|-------|
| Source files | 400 |
| Library tests | ~851 passing, 0 failing |
| TUI files | 60 |
| Tab variants | 29 |
| Dispatch macros | 6 |
| Doc coverage | ~8% |
| Failing tests | 0 |

### Key Gaps

1. ~~2 failing integration tests with incorrect assertions~~ ✅ Fixed
2. ~~UTF-8 panic in `InputField` (multi-byte character handling)~~ ✅ Fixed
3. ~~Grammar fuzzer mislabels all payloads as `PayloadType::Xss`~~ ✅ Fixed
4. ~~CSV export doesn't escape all fields~~ ✅ Fixed
5. ~~Silent error swallowing in tool registry and TUI~~ ✅ Fixed
6. ~~Type-level bugs (`u16` overflow, `&PathBuf` vs `&Path`)~~ ✅ Fixed
7. ~~`ResponseSeverity` lacks `Ord`/`PartialOrd`~~ ✅ Fixed
8. 67% of source files lack inline tests
9. 8% documentation coverage
10. ~~Unconditionally compiled stub modules (no CLI/TUI wiring)~~ ✅ Feature-gated (Wave 4A)
11. ~~No OpenResponses API for OpenClaw integration~~ ✅ Implemented (Wave 7A)
12. ~~OpenAI handler uses keyword matching; `FunctionCall.arguments` is always `{}`~~ ✅ Improved (Wave 7B)
13. TUI dispatch uses 6 near-identical macros instead of trait-based dispatch
14. ~~Secret exposure in HTTP options popup~~ ✅ Fixed
15. ~~Infinite hang in `run_packet_capture()`~~ ✅ Fixed
16. ~~Duplicate subdomain enumeration in recon takeover check~~ ✅ Fixed

---

## Wave 1: Critical Bug Fixes

**Priority:** Highest — these cause panics, hangs, and test failures.
**Dependencies:** None. All blocks are independent.
**Parallelization:** Blocks A–E can run simultaneously.

### Block A: Test & Build Fixes

#### A1. Fix 2 failing negative tests
- **Files:** `crates/slapper/tests/negative_tests.rs:185-197`
- **Problem:** `test_scope_empty_target` and `test_scope_invalid_target` assert `Ok` but `parse_hostname_only()` returns `Err(InvalidTarget)` — this is correct behavior.
- **Fix:** Update assertions to expect `Err(ScopeError::InvalidTarget)`.
- **Verification:** `cargo test --test negative_tests -p slapper`
- **Estimated effort:** 10 min

### Block B: UTF-8 & Input Handling

#### B1. Fix UTF-8 panic in `InputField::delete()` and `backspace()`
- **File:** `crates/slapper/src/tui/components/input.rs:79-90`
- **Problem:** `self.value.remove(self.cursor_pos)` panics when `cursor_pos` falls in the middle of a multi-byte UTF-8 character. `backspace()` decrements by 1 byte; `delete()` uses byte index directly.
- **Fix:** Use `char_boundary` helpers:
  ```rust
  // delete()
  if let Some(next) = self.value[self.cursor_pos..].chars().next() {
      let end = self.cursor_pos + next.len_utf8();
      self.value.drain(self.cursor_pos..end);
  }
  // backspace()
  if let Some(prev) = self.value[..self.cursor_pos].chars().next_back() {
      self.cursor_pos -= prev.len_utf8();
      self.value.drain(self.cursor_pos..self.cursor_pos + prev.len_utf8());
  }
  ```
- **Estimated effort:** 1 hour

#### B2. Fix `InputField` cursor movement for multi-byte chars
- **File:** `crates/slapper/src/tui/components/input.rs:92-108`
- **Problem:** `move_left()` and `move_right()` increment/decrement by 1 byte, not by char boundary.
- **Fix:** Use `char_indices()` to find correct boundaries.
- **Estimated effort:** 20 min

#### B3. Fix `/` key binding conflict
- **File:** `crates/slapper/src/tui/app/runner.rs:182-183` and `runner.rs:331`
- **Problem:** `/` in Normal mode is bound to `toggle_command_palette()` at line 183, making the `toggle_search()` binding at line 331 unreachable.
- **Fix:** Remove the duplicate binding at line 182-183. Keep `/` for search. Use `Ctrl+P` for command palette (already bound).
- **Estimated effort:** 15 min

### Block C: Core Logic Bugs

#### C1. Fix `PortScanResults::ports_scanned` overflow
- **File:** `crates/slapper/src/scanner/ports/mod.rs:69`
- **Problem:** `ports_scanned: u16` overflows at 65,536.
- **Fix:** Change to `u32`. Update all construction sites and serialization.
- **Estimated effort:** 15 min

#### C2. Fix grammar fuzzer payload type mislabeling
- **File:** `crates/slapper/src/fuzzer/engine/core.rs:175-181`
- **Problem:** All grammar-generated payloads are tagged as `PayloadType::Xss` regardless of actual grammar type (JSON, GraphQL, XML, JWT, SSTI).
- **Fix:**
  1. Add a `kind: GrammarKind` enum field to `GrammarFuzzer`
  2. Set it during construction based on which factory method was used
  3. Use it when tagging payloads (match on `GrammarKind` to select correct `PayloadType`)
- **Estimated effort:** 45 min

#### C3. Fix CSV export unescaped fields
- **File:** `crates/slapper/src/output/convert.rs:133-149`
- **Problem:** `severity` and `cve_ids.join(";")` are written to CSV without escaping.
- **Fix:** Apply `escape_csv()` to all 6 fields.
- **Estimated effort:** 10 min

#### C4. Fix `ConfigError::Io` losing error chain
- **File:** `crates/slapper/src/config/settings.rs:262-275`
- **Problem:** `ConfigError::Io(String)` wraps a string instead of `std::io::Error`, losing `source()` information.
- **Fix:** Change to `Io(std::io::Error)` and implement proper `source()` method.
- **Estimated effort:** 20 min

#### C5. Fix `SlapperConfig::load` and `save` signatures
- **Files:** `crates/slapper/src/config/settings.rs:215`, `settings.rs:222`
- **Problem:** Both take `&PathBuf` instead of idiomatic `impl AsRef<Path>`.
- **Fix:** Change signatures to `impl AsRef<Path>`.
- **Estimated effort:** 15 min

### Block D: TUI Runtime Bugs

#### D1. Fix infinite hang in `run_packet_capture()`
- **File:** `crates/slapper/src/tui/workers/network.rs:132`
- **Problem:** `while let Some(_packet) = pkt_rx.recv().await` blocks forever if the capture source stops sending packets before `max_packets` is reached.
- **Fix:** Add `tokio::time::timeout()` to the `recv()` call. Break the loop if no packet arrives within a reasonable timeout (e.g., 5 seconds).
- **Estimated effort:** 30 min

#### D2. Fix unreachable arms in `cycle_export_format()` and `get_export_extension()`
- **File:** `crates/slapper/src/tui/app/mod.rs:191-213`
- **Problem:** Both functions have unreachable `_` catch-all arms since all 6 `OutputFormat` variants are covered.
- **Fix:** Remove the unreachable `_` arms.
- **Estimated effort:** 5 min

#### D3. Fix `get_tab_status()` unused in Resume tab
- **File:** `crates/slapper/src/tui/ui.rs:563-574`
- **Problem:** The `Resume` tab has inline match on `AppState` instead of using the `get_tab_status()` helper.
- **Fix:** Use `get_tab_status(app.current_tab, &app.resume.state)` like other tabs.
- **Estimated effort:** 15 min

#### D4. Fix `pending_key` not cleared on mode change
- **File:** `crates/slapper/src/tui/app/runner.rs:123-133`
- **Problem:** If user presses `g` then switches to Insert mode, the pending `g` is not cleared.
- **Fix:** Clear `pending_key` when mode changes or on any non-matching key press.
- **Estimated effort:** 10 min

### Block E: Recon & Fuzzer Bugs

#### E1. Eliminate duplicate subdomain enumeration in takeover check
- **File:** `crates/slapper/src/recon/runner.rs:290-291`
- **Problem:** Takeover detection re-calls `subdomain::enumerate_subdomains()` even though results are already available from line 184.
- **Fix:** Pass the existing `subdomain_result` to the takeover detection logic.
- **Estimated effort:** 20 min

#### E2. Fix unreachable `None` branch in `WafEngine::run`
- **File:** `crates/slapper/src/waf/mod.rs:220-226`
- **Problem:** After `self.bypass_engine = Some(...)`, the match on `&self.bypass_engine` has a `None` arm that is unreachable.
- **Fix:** Use `.as_ref().expect("bypass engine must be initialized")` or restructure.
- **Estimated effort:** 15 min

---

## Wave 2: Security & Error Handling

**Priority:** High — security fixes and developer experience improvements.
**Dependencies:** None (can run in parallel with Wave 1, but recommended after for clean state).
**Parallelization:** Blocks A–C can run simultaneously.

### Block A: Security Fixes

#### A1. Redact secrets in HTTP options popup
- **File:** `crates/slapper/src/tui/ui.rs:69-137`
- **Problem:** `draw_http_options_popup()` displays `proxy_auth`, `bearer`, `cookie`, and `api_key` values in plain text.
- **Fix:** Show `****` or `[REDACTED]` for sensitive fields. Add a "Show" toggle button.
- **Estimated effort:** 30 min

### Block B: Error Handling Consistency

#### B1. Add `From<tokio::time::error::Elapsed>` for `SlapperError`
- **File:** `crates/slapper/src/error/mod.rs:175-287`
- **Problem:** No `From` impl for `tokio::time::error::Elapsed`.
- **Fix:** Add to the existing `From` impl block mapping to `SlapperError::Timeout`.
- **Estimated effort:** 10 min

#### B2. Fix `check_scope` to use `SlapperError`
- **File:** `crates/slapper/src/utils/scope.rs:4`
- **Problem:** Uses `anyhow::Result` and `anyhow::bail!` instead of canonical `SlapperError::ScopeViolation`.
- **Fix:** Use `crate::error::{Result, SlapperError}` and return `SlapperError::ScopeViolation`.
- **Estimated effort:** 15 min

#### B3. Fix `StreamEvent::error` ignoring error parameter
- **File:** `crates/slapper/src/tool/response.rs:432`
- **Problem:** `pub fn error(request_id: &str, _error: &str)` — the `_error` parameter is unused.
- **Fix:** Inspect `StreamEvent` structure and populate the error message in the appropriate field.
- **Estimated effort:** 20 min

#### B4. Log tool registration failures
- **File:** `crates/slapper/src/tool/mod.rs:40-74`
- **Problem:** `create_default_registry()` silently swallows all registration errors via `.ok()`.
- **Fix:** Replace `.ok()` with `.map_err(|e| tracing::warn!("Failed to register tool: {e}"))`.
- **Estimated effort:** 20 min

#### B5. Fix `SlapperError::with_timeout` unnecessary `mem::take`
- **File:** `crates/slapper/src/error/mod.rs:161-170`
- **Problem:** `operation` is `String`, so `std::mem::take` replaces it with empty string temporarily — unnecessarily complex.
- **Fix:** Destructure and rebuild directly.
- **Estimated effort:** 10 min

### Block C: TUI Reliability

#### C1. Fix silent export failures
- **File:** `crates/slapper/src/tui/app/export.rs:54-157`, `export.rs:217`
- **Problem:** 18 tabs have no-op `export_json()` implementations. `export_converted()` silently fails when JSON file doesn't exist. `unwrap_or_default()` swallows serialization errors.
- **Fix:**
  - Add `eprintln!` or `tracing::warn!` when a tab has no exportable data
  - Return error or show user message when source JSON is missing
  - Replace `unwrap_or_default()` with proper error handling
- **Estimated effort:** 1 hour

#### C2. Fix silently dropped errors in `set_error_for_current_tab()`
- **File:** `crates/slapper/src/tui/app/state_update.rs:241`
- **Problem:** The `_ => {}` arm silently drops errors for 12 tabs (Resume, Proxy, GraphQl, OAuth, Cluster, Stress, Report, Nse, Plugin, Settings, History, Dashboard).
- **Fix:** Add `set_error()` calls for all tabs with a `state` field. For tabs without state (History, Dashboard), log via `tracing::error!`.
- **Estimated effort:** 45 min

#### C3. Fix `spawn_task()` channel replacement bug
- **File:** `crates/slapper/src/tui/app/task_management.rs:4-26`
- **Problem:** `spawn_task()` replaces `progress_rx` and `result_rx` without draining old channels. Old task results are lost.
- **Fix:** Check if a task is already running before replacing channels. Abort old task or reject new spawn.
- **Estimated effort:** 30 min

#### C4. Fix mouse click tab area fragility
- **File:** `crates/slapper/src/tui/app/runner.rs:84-89`
- **Problem:** Hardcoded `tab_area` rect (`y: 1, height: 3`) may not match actual rendered tab bar.
- **Fix:** Derive tab area from actual terminal dimensions or add a constant computed from layout constraints.
- **Estimated effort:** 30 min

---

## Wave 3: Code Quality — Components & Workers

**Priority:** Medium — improves code quality and developer experience.
**Dependencies:** Wave 1 recommended first (avoid debugging pre-existing bugs during refactor).
**Parallelization:** Blocks A–C can run simultaneously.

### Block A: Component Improvements

#### A1. Deduplicate `ScrollableText::render()` and `render_with_style()`
- **File:** `crates/slapper/src/tui/components/scrollable.rs:96-184`
- **Problem:** 43 lines of duplicated rendering code. Only difference is border color parameter.
- **Fix:** Add a `border_color: Option<Color>` parameter to `render()`. Remove `render_with_style()`.
- **Estimated effort:** 20 min

#### A2. Add upper bound to `ScrollableText::scroll_right()`
- **File:** `crates/slapper/src/tui/components/scrollable.rs:63-65`
- **Problem:** `horizontal_offset += amount` has no upper bound check.
- **Fix:** Clamp to maximum line width.
- **Estimated effort:** 15 min

#### A3. Fix duplicate `DropdownInfo::render()` ListState allocation
- **File:** `crates/slapper/src/tui/components/selector.rs`
- **Problem:** `ListState` is created on every render call.
- **Fix:** Store `ListState` as a field on `DropdownInfo`.
- **Estimated effort:** 15 min

#### A4. Fix `RadioGroup::render()` overflow
- **File:** `crates/slapper/src/tui/components/selector.rs`
- **Problem:** Renders all options on a single line. Overflows with many options.
- **Fix:** Wrap options across multiple lines or add horizontal scrolling.
- **Estimated effort:** 30 min

### Block B: App & Workers

#### B1. Remove unused `get_tab_status()` parameter
- **File:** `crates/slapper/src/tui/ui.rs:533-534`
- **Problem:** `get_tab_status(_tab: Tab, state: &AppState)` takes a `Tab` parameter that is never used.
- **Fix:** Remove the `_tab` parameter.
- **Estimated effort:** 5 min

#### B2. Fix unnecessary clones in `run_nse()`
- **File:** `crates/slapper/src/tui/workers/api.rs:312-315`
- **Problem:** Clones strings before moving into closure. Originals are not used after.
- **Fix:** Move original variables into closure directly.
- **Estimated effort:** 10 min

#### B3. Implement real `run_packet_send()` or mark as stub
- **File:** `crates/slapper/src/tui/workers/network.rs:228-256`
- **Problem:** No-op loop that just increments counters. No actual packets are sent.
- **Fix:** Either implement actual packet sending using raw sockets, or clearly mark as stub with user-facing message.
- **Estimated effort:** 2 hours (if implementing) or 15 min (if marking as stub)

#### B4. Add incremental progress to scanner and recon workers
- **Files:** `crates/slapper/src/tui/workers/scanner.rs`, `crates/slapper/src/tui/workers/recon.rs`
- **Problem:** Progress is only sent at completion (0% → 100% jump).
- **Fix:** Send progress updates at meaningful intervals (per batch of ports, per recon stage).
- **Estimated effort:** 1 hour

#### B5. Fix `build_waf_stress_task()` reusing `TaskConfig::Fuzz`
- **File:** `crates/slapper/src/tui/app/task_management.rs:145-169`
- **Problem:** Creates a `TaskConfig::Fuzz` with all GraphQL/OAuth flags set to `false` as a workaround.
- **Fix:** Add `TaskConfig::WafStress { target, concurrency, timeout }` variant and handle it in `TaskRunner::run()`.
- **Estimated effort:** 30 min

#### B6. Fix `build_packet_send_task()` silent port default
- **File:** `crates/slapper/src/tui/app/task_management.rs:218`
- **Problem:** `self.packet.filter().parse().unwrap_or(80)` silently defaults to port 80.
- **Fix:** Return `None` if filter cannot be parsed as a port, or validate in PacketTab before allowing Enter.
- **Estimated effort:** 15 min

#### B7. Fix `run_waf()` unused techniques parameter
- **File:** `crates/slapper/src/tui/workers/fuzzer.rs:91`
- **Problem:** `_techniques: Vec<String>` parameter is unused.
- **Fix:** Pass techniques to the WAF detection engine.
- **Estimated effort:** 30 min

#### B8. Fix `dispatch_void!` Settings no-op inconsistency
- **File:** `crates/slapper/src/tui/app/dispatch.rs:74`
- **Problem:** `Tab::Settings => {}` is a no-op in `dispatch_void!` but calls the method in `dispatch!`.
- **Fix:** Either make `dispatch_void!` call the method for consistency, or add a comment explaining the intentional exclusion.
- **Estimated effort:** 10 min

### Block C: Type & API Improvements

#### C1. Add `Ord`/`PartialOrd` to `ResponseSeverity`
- **File:** `crates/slapper/src/tool/response.rs:287-295`
- **Problem:** `ResponseSeverity` lacks ordering, making sorting inconsistent with canonical `Severity`.
- **Fix:** Add `as_int()`, `Ord`, `PartialOrd` implementations. Add `From` conversions between `ResponseSeverity` and `Severity`.
- **Estimated effort:** 30 min

#### C2. Fix `run_sequential` unnecessary `Arc<Mutex<Vec>>`
- **File:** `crates/slapper/src/fuzzer/engine/execution.rs:32-46`
- **Problem:** Sequential execution uses `Arc<Mutex<Vec<FuzzResult>>>` but processes one payload at a time.
- **Fix:** Use a plain `Vec<FuzzResult>`.
- **Estimated effort:** 15 min

#### C3. Optimize `run_concurrent_inner` semaphore acquisition
- **File:** `crates/slapper/src/fuzzer/engine/execution.rs:98`
- **Problem:** `semaphore.acquire_owned().await` is called in the spawning loop, serializing task creation.
- **Fix:** Spawn all tasks eagerly, acquire semaphore inside each task. (Keep current approach if backpressure during spawning is desired.)
- **Estimated effort:** 30 min

#### C4. Fix `find_config_file` and `find_scope_file` thread-safety
- **File:** `crates/slapper/src/config/loader.rs:66-96`
- **Problem:** Uses `std::env::current_dir()` implicitly. Tests that change `current_dir` are fragile and not thread-safe.
- **Fix:** Accept an optional base directory parameter.
- **Estimated effort:** 30 min

#### C5. Review `strip_controls` and `preserve_all` padding behavior
- **File:** `crates/slapper/src/utils/formatting.rs:7`
- **Problem:** Functions pad short strings with spaces to fill `max_len` width.
- **Decision:** Add a separate `truncate_only` function — no padding, just `.chars().take(max_len).collect()`.
- **Estimated effort:** 15 min

---

## Wave 4: Architecture — Stub Modules & Dispatch Refactor

**Priority:** Medium-High — reduces compilation overhead and simplifies core dispatch.
**Dependencies:** Wave 1 recommended first (fix bugs before refactoring).
**Parallelization:** Blocks A and B are independent.

### Block A: Stub Module Resolution

#### A1. Feature-gate stub modules
- **Files:** `crates/slapper/src/lib.rs:66-72`
- **Problem:** 8 modules are unconditionally compiled but have no CLI commands or TUI tabs in default features:
  - `container`, `storage`, `supply_chain`, `hunt`, `compliance`, `integrations`, `workflow`, `vuln`
- **Fix:** Feature-gate each module:
  | Module | Feature Flag |
  |--------|-------------|
  | `container` | `container` (already exists) |
  | `storage` | `database` (already exists) |
  | `supply_chain` | `sbom` (already exists) |
  | `hunt` | New: `advanced-hunting` |
  | `compliance` | New: `compliance` |
  | `integrations` | New: `external-integrations` |
  | `workflow` | New: `finding-workflow` |
  | `vuln` | New: `vuln-management` |
  Add all new flags to the `full` feature.
- **Estimated effort:** 2 hours

#### A2. Fix `Commands::Nse` and `Commands::Plugin` feature gating
- **Files:** `crates/slapper/src/cli/mod.rs:106-111`
- **Problem:** Both variants are gated with `#[cfg(feature = "...")]` on the enum variants themselves. Per AGENTS.md, they should always exist with feature-gated match arms.
- **Fix:** Remove `#[cfg]` from both variants. Add both `#[cfg(feature = "...")]` and `#[cfg(not(feature = "..."))]` arms in `handle_command`.
- **Estimated effort:** 30 min

#### A3. Consolidate duplicate project constants
- **Files:** `crates/slapper/src/config/loader.rs:11-12` vs `config/settings.rs:230`
- **Problem:** `"tools"` / `"slapper"` hardcoded in two places for `ProjectDirs::from()`.
- **Fix:** Define in `constants.rs` as `PROJECT_QUALIFIER` and `PROJECT_NAME`.
- **Estimated effort:** 15 min

#### A4. Fix `Tab::all()` feature-gated variant handling
- **File:** `crates/slapper/src/tui/tabs/mod.rs:209-241`
- **Problem:** `Tab::all()` returns all variants unconditionally, but some tab structs are feature-gated.
- **Fix:** Use conditional compilation for feature-gated tab pushes. Audit all 29 variants.
- **Estimated effort:** 30 min

### Block B: Dispatch & Tab Abstraction

#### B1. Consolidate 6 dispatch macros into trait-based dispatch
- **File:** `crates/slapper/src/tui/app/dispatch.rs`
- **Problem:** 6 macros (`dispatch!`, `dispatch_void!`, `dispatch_bool!`, `dispatch_page!`, `dispatch_is_at_edge!`, `dispatch_reset!`) are near-identical ~40-line match expansions.
- **Fix:** Replace with trait-based dispatch. Add methods on the `Tab` enum that delegate to the correct tab struct. This eliminates macros entirely and gives compile-time exhaustiveness checking.
- **Estimated effort:** 4 hours

#### B2. Replace match-on-Tab in `handle_enter()` with trait dispatch
- **File:** `crates/slapper/src/tui/app/mod.rs:226-368`
- **Problem:** 142-line match statement. Each arm follows the same pattern.
- **Fix:** After B1, this becomes a single method call on the current tab.
- **Estimated effort:** 2 hours (depends on B1)

#### B3. Replace match-on-Tab in `ui.rs` draw functions
- **Files:** `crates/slapper/src/tui/ui.rs:260-403`, `ui.rs:405-531`, `ui.rs:546-670`
- **Problem:** Three 100+ line match statements for rendering.
- **Fix:** Add `render_content()`, `breadcrumb()`, and `status_text()` methods to the `Tab` enum or a rendering trait.
- **Estimated effort:** 3 hours (depends on B1)

#### B4. Add `Tab` enum method for tab state access
- **File:** `crates/slapper/src/tui/app/mod.rs`
- **Problem:** To access a tab's state, you need a match on `Tab` to get the right field from `App`.
- **Fix:** Add a method like `fn as_tab_state(&self, app: &App) -> &dyn TabState` on the `Tab` enum.
- **Estimated effort:** 2 hours

#### B5. Replace `#[macro_use]` with modern macro system
- **File:** `crates/slapper/src/tui/app/mod.rs:1`
- **Problem:** Uses old-style `#[macro_use]` for macros.
- **Fix:** After B1 (consolidating/removing macros), this becomes moot. If any macros remain, move them to a `macros` submodule.
- **Estimated effort:** 15 min

---

## Wave 5: Recon & Fuzzer Improvements

**Priority:** Medium — directly affects quality of security testing output.
**Dependencies:** Wave 1 recommended (benefits from bug fixes).
**Parallelization:** Single block — items are small enough to do sequentially.

### Block A: Recon & Fuzzer

#### A1. Split `run_full_recon` (~340 lines)
- **File:** `crates/slapper/src/recon/runner.rs`
- **Problem:** Single ~340-line function with 15-way `tokio::join!` and repetitive error handling.
- **Fix:** Extract into focused functions (`run_dns_recon`, `run_ssl_recon`, `run_tech_detection`, etc.) and compose in `run_full_recon`.
- **Estimated effort:** 3 hours

---

## Wave 6: Feature Completeness

**Priority:** Medium — depends on product requirements for stub tabs.
**Dependencies:** None. Can run in parallel with Waves 1–5.
**Parallelization:** Blocks A and B are independent.

### Block A: Stub Tab Implementation

#### A1. Implement Storage tab functionality
- **Files:** `crates/slapper/src/tui/tabs/storage.rs`, `crates/slapper/src/tui/workers/security.rs:51-57`
- **Problem:** Tab, task config, and worker are all stubs.
- **Fix:** Define what "Storage" means (likely scan result storage/caching) and implement, or remove the tab.
- **Estimated effort:** TBD

#### A2. Implement Integrations tab functionality
- **Files:** `crates/slapper/src/tui/tabs/integrations.rs`, `crates/slapper/src/tui/workers/security.rs:59-65`
- **Problem:** Stub implementation.
- **Fix:** Define requirements or remove tab.
- **Estimated effort:** TBD

#### A3. Implement Workflow tab functionality
- **Files:** `crates/slapper/src/tui/tabs/workflow.rs`, `crates/slapper/src/tui/workers/security.rs:67-73`
- **Problem:** Stub implementation.
- **Fix:** Define requirements or remove tab.
- **Estimated effort:** TBD

#### A4. Implement Vuln tab functionality
- **Files:** `crates/slapper/src/tui/tabs/vuln.rs`, `crates/slapper/src/tui/workers/security.rs:75-81`
- **Problem:** Stub implementation.
- **Fix:** Define requirements or remove tab.
- **Estimated effort:** TBD

#### A5. Implement real `run_compliance_task()`
- **File:** `crates/slapper/src/tui/workers/security.rs:34-49`
- **Problem:** Uses hardcoded `vec![Severity::High, Severity::Medium, Severity::Low]` for findings.
- **Fix:** Generate findings from actual scan results or target analysis.
- **Estimated effort:** 2 hours

### Block B: Command Palette & UI Polish

#### B1. Fix duplicate command palette entries
- **File:** `crates/slapper/src/tui/help.rs`
- **Problem:** "settings", "resume", and "history" appear twice with different shortcuts.
- **Fix:** Remove duplicates. Keep the entry with the most appropriate shortcut.
- **Estimated effort:** 15 min

#### B2. Add pagination to command palette results
- **File:** `crates/slapper/src/tui/ui.rs:139-210`
- **Problem:** Renders all results without pagination. Overflows popup area with many results.
- **Fix:** Only render visible items based on popup height. Add scroll indicator.
- **Estimated effort:** 45 min

#### B3. Add fuzzy matching to command palette search
- **File:** `crates/slapper/src/tui/help.rs:84-96`
- **Problem:** Only does case-insensitive substring matching. No ranking.
- **Fix:** Implement fuzzy matching (subsequence matching) and sort results by relevance.
- **Estimated effort:** 1 hour

#### B4. Fix `InputGroup::handle_tab()` conflicts with global Tab key
- **File:** `crates/slapper/src/tui/components/input.rs:420-427`
- **Problem:** Tab key is used for both autocomplete (in InputGroup) and focus navigation (global).
- **Fix:** Use a different key for autocomplete (e.g., `Ctrl+Space` or `Down` arrow). Or make Tab do autocomplete when suggestions exist, focus navigation otherwise.
- **Estimated effort:** 30 min

---

## Wave 7: OpenClaw Integration

**Priority:** High — enables AI agent integration.
**Dependencies:** None (but benefits from Wave 3's `ResponseSeverity` ordering).
**Parallelization:** Blocks A and B are independent. Blocks C, D, E can run in parallel after A.

### Block A: OpenResponses API (Critical Path)

**Feature gate:** `rest-api`

#### A1. New module: `tool/protocol/openresponses/`
- **Files to create:**
  - `tool/protocol/openresponses/mod.rs` — Module entry, router builder
  - `tool/protocol/openresponses/types.rs` — OpenResponses request/response types
  - `tool/protocol/openresponses/handlers.rs` — Request handler with tool dispatch
- **Key types:** `ResponsesRequest`, `Input` (polymorphic), `InputItem`, `FunctionTool`, `ResponsesResponse`, `OutputItem` (polymorphic with `slapper:security_finding` extension), `StreamEvent` (SSE events)
- **Estimated effort:** 4 hours

#### A2. Router integration
- **Modify:** `tool/protocol/mod.rs` — add module declaration
- **Modify:** `tool/protocol/mcp/routes.rs` — merge OpenResponses router
- **Endpoints:** `POST /v1/responses`
- **Estimated effort:** 30 min

#### A3. Authentication
- Reuse existing auth patterns: `Authorization: Bearer <key>`, `x-api-key` header, constant-time comparison via `subtle::ConstantTimeEq`
- Implement middleware/extractor returning `401` with `ErrorResponse` on failure
- **Estimated effort:** 1 hour

#### A4. Request normalization
- Handle OpenClaw body wrapper (`{ body: ResponsesRequest }`) and direct payload
- Handle `input` being either `String` or `Vec<InputItem>`
- **Estimated effort:** 30 min

#### A5. Tool execution strategy
- Implement agentic loop: parse request → match tools → execute via ToolRegistry → build structured output
- Return structured JSON results (findings count, target info, evidence, remediation)
- **Estimated effort:** 2 hours

#### A6. Tests
- 15+ integration tests covering: non-streaming/streaming requests, tool matching, target extraction, auth rejection, rate limiting, error responses, body wrapper normalization, `previous_response_id`, `tool_choice` modes, extensible item types
- **Estimated effort:** 2 hours

### Block B: Fix OpenAI Chat Completions & Add Model Discovery

#### B1. Improve OpenAI tool calling
- **File:** `crates/slapper/src/tool/protocol/openai/handlers.rs`
- **Problems:** Naive keyword matching, `FunctionCall.arguments` always `{}`, plain text results, no parameter extraction
- **Fix:** Parameter extraction from queries, structured JSON results, smarter matching using capability descriptions, multi-step execution support
- **Estimated effort:** 3 hours

#### B2. Add `/v1/models` and `/v1/models/{model_id}` endpoints
- **New file:** `tool/protocol/openai/models.rs`
- **Modify:** `tool/protocol/openai/mod.rs` to add routes
- **Estimated effort:** 1 hour

#### B3. Tests
- Parameter extraction, structured result serialization, model endpoint responses
- **Estimated effort:** 1 hour

### Block C: Expose AI Features via HTTP

**Feature gate:** `rest-api` + `ai-integration`

#### C1. AI analysis endpoints
- **New file:** `tool/protocol/ai_routes.rs`
- **Endpoints:**
  | Method | Path | Maps To |
  |---|---|---|
  | POST | `/api/v1/ai/analyze` | `AiClient::analyze_findings()` |
  | POST | `/api/v1/ai/suggest-payloads` | `AiClient::suggest_payloads()` |
  | POST | `/api/v1/ai/waf-bypass` | `AiClient::suggest_waf_bypass()` |
  | POST | `/api/v1/ai/scan-strategy` | `AdaptiveScanEngine::adjust_strategy()` |
  | GET | `/api/v1/ai/circuit-breaker` | `AiClient::circuit_breaker_state()` |
- **Estimated effort:** 2 hours

#### C2. AI config validation endpoint
- `POST /api/v1/ai/validate-config`
- **Estimated effort:** 30 min

#### C3. Tests
- Each endpoint with valid/invalid inputs, circuit breaker state transitions, graceful degradation
- **Estimated effort:** 1.5 hours

### Block D: OpenClaw SKILL.md

#### D1. Create SKILL.md
- **New file:** `skills/slapper-security/SKILL.md`
- Teaches OpenClaw agents when and how to use Slapper (scan, fuzz, recon, waf:detect tools)
- **Estimated effort:** 1 hour

#### D2. Installation instructions
- **New file:** `skills/slapper-security/INSTALL.md`
- **Estimated effort:** 15 min

### Block E: Agent Registry HTTP Endpoints (Optional)

#### E1. Agent management endpoints
- **New file:** `tool/protocol/agent_routes.rs`
- CRUD for agents: POST/GET/DELETE `/api/v1/agents`, heartbeat, delegation
- **Estimated effort:** 2 hours

#### E2. Task management endpoints
- Task lifecycle: POST/GET `/api/v1/tasks`, cancel, get result
- **Estimated effort:** 1.5 hours

#### E3. Tests
- Full CRUD, task lifecycle, delegation flow
- **Estimated effort:** 1.5 hours

---

## Wave 8: Test Coverage Expansion

**Priority:** Medium — benefits from Waves 1–4 (stable, correct code to test against).
**Dependencies:** Wave 1 (clean build), Wave 3 (type/API improvements) recommended first.
**Parallelization:** Blocks A–D can run simultaneously.

### Block A: Fuzzer Tests

#### A1. Add tests for fuzzer payload modules (14 files)
- **Files:** `crates/slapper/src/fuzzer/payloads/` — headers, compression, redos, websocket, macros, deser, oauth, soap, redirect, cache, idor, gRPC
- **Fix:** Add `#[cfg(test)]` modules with payload count checks, content validation, macro expansion tests
- **Estimated effort:** 3 hours

#### A2. Add tests for fuzzer engine modules (6 files)
- **Files:** `crates/slapper/src/fuzzer/engine/` — core, utils, advanced, execution, types
- **Fix:** Add tests for `FuzzEngine::new()`, payload execution ordering, result aggregation, rate limiter, ReDoS detection
- **Estimated effort:** 3 hours

### Block B: Infrastructure Tests

#### B1. Add tests for proxy modules (6 files)
- **Files:** `crates/slapper/src/proxy/` — http_connect, pool, config, health, rotator, socks
- **Fix:** Add tests for pool management, health checking, rotation strategies, SOCKS parsing, config serialization
- **Estimated effort:** 2 hours

#### B2. Add tests for scanner modules (3 files)
- **Files:** `crates/slapper/src/scanner/` — udp_fingerprint, icmp_probe, mod
- **Fix:** Add tests for UDP fingerprinting, ICMP probing, timing presets
- **Estimated effort:** 1 hour

#### B3. Add tests for utility modules (4 files)
- **Files:** `crates/slapper/src/utils/` — service_detection, scope, privilege, client_pool
- **Fix:** Add tests for service detection, scope validation, privilege checking, client pool
- **Estimated effort:** 1.5 hours

### Block C: Recon & Output Tests

#### C1. Add tests for recon modules
- **Files:** `crates/slapper/src/recon/` — runner.rs (~340-line function), various submodules
- **Fix:** Add tests for `run_full_recon()` stage ordering, subdomain merging, tech detection, CORS, email security
- **Estimated effort:** 3 hours

#### C2. Add tests for output modules
- **Files:** `crates/slapper/src/output/` — dedup, trend, baseline, ai_schema
- **Fix:** Add tests for dedup engine, trend analysis, baseline comparison, AI output serialization
- **Estimated effort:** 2 hours

---

## Wave 9: Documentation

**Priority:** Medium — should be written against finalized APIs (after other waves).
**Dependencies:** None, but benefits from all other waves.
**Parallelization:** Blocks A–D can run simultaneously.

### Block A: Module Documentation

#### A1. Add module-level documentation
- **Scope:** All 39+ top-level modules
- **Fix:** Add `//!` module docs with purpose, public API, and examples
- **Priority order:** Public API modules → Infrastructure → Feature-gated → Internal
- **Estimated effort:** 6 hours

### Block B: Function Documentation

#### B1. Add doc comments to public functions
- **Scope:** 3,226+ public items, ~8% documented
- **Fix strategy:** Prioritize public API (full docs with `# Examples` and `# Errors`), then `pub(crate)` (brief one-liners)
- **Target:** Raise doc coverage from 8% to 40%
- **Estimated effort:** 12 hours

### Block C: Documentation Maintenance

#### C1. Update stale doc comments
- **Files:** `crates/slapper/src/waf/mod.rs:4` (says "30+ WAF products", actual count is 26), and others
- **Fix:** Audit all doc comments against `constants.rs` values and actual implementation
- **Estimated effort:** 30 min

#### C2. Document feature flags in lib.rs
- **File:** `crates/slapper/src/lib.rs:30-37`
- **Problem:** Feature flag documentation is incomplete — missing `websocket`, `headless-browser`, and newer flags
- **Fix:** Update crate-level doc comment to list all 17+ feature flags
- **Estimated effort:** 20 min

---

## Wave 10: Performance & Polish

**Priority:** Low — always valuable but not urgent.
**Dependencies:** None. Can run in parallel with any wave.
**Parallelization:** Single block — items are small and independent.

### Block A: Performance Optimizations

#### A1. Avoid cloning `command_palette_entries` on every open
- **File:** `crates/slapper/src/tui/app/command.rs:4-21`
- **Problem:** Clones `Vec<CommandPaletteResult>` with 37 items on every open.
- **Fix:** Store entries as `&'static` references or use `Arc`. Keep single copy in `HelpManager`.
- **Estimated effort:** 20 min

#### A2. Optimize `ScrollableText` render allocation
- **File:** `crates/slapper/src/tui/components/scrollable.rs:108-114`
- **Problem:** `.cloned().collect()` allocates a new `Vec<Line>` on every render frame.
- **Fix:** Use pre-allocated buffer or render directly from iterator.
- **Estimated effort:** 30 min

#### A3. Add export directory configuration
- **File:** `crates/slapper/src/tui/app/export.rs:229-251`
- **Problem:** Hardcoded `./exports/` path.
- **Fix:** Read export directory from config or add settings option.
- **Estimated effort:** 30 min

#### A4. Fix `search_backup` loss on tab switch
- **File:** `crates/slapper/src/tui/app/navigation.rs:60-70`
- **Problem:** Backup is lost when switching tabs while search is active.
- **Fix:** Clear search on tab switch, or persist backup across tab switches.
- **Estimated effort:** 15 min

---

## Implementation Order & Parallelization

```
Wave 1: Critical Bug Fixes      ██████████████████████████████████████████████  (Blocks A-E parallel)
Wave 2: Security & Error        ██████████████████████████████████████████████  (Blocks A-C parallel)
Wave 3: Code Quality            ██████████████████████████████████████████████  (Blocks A-C parallel)
Wave 4: Architecture            ██████████████████████████████████████████████  (Blocks A-B parallel)
Wave 5: Recon/Fuzzer            ██████████████████████████████████████████████  (Single block)
Wave 6: Feature Completeness    ██████████████████████████████████████████████  (Blocks A-B parallel)
Wave 7: OpenClaw Integration    ██████████████████████████████████████████████  (Blocks A-B parallel, then C-D-E)
Wave 8: Test Coverage           ██████████████████████████████████████████████  (Blocks A-C parallel)
Wave 9: Documentation           ██████████████████████████████████████████████  (Blocks A-D parallel)
Wave 10: Performance & Polish   ██████████████████████████████████████████████  (Single block)
```

### Recommended Execution Strategy

| Phase | Waves | Rationale |
|-------|-------|-----------|
| **Phase 1: Stabilize** | Wave 1 | Fix all bugs first — clean foundation for everything else |
| **Phase 2: Harden** | Wave 2 + Wave 3 | Security fixes and code quality improvements (parallel) |
| **Phase 3: Refactor** | Wave 4 | Architecture changes on stable, tested code |
| **Phase 4: Enhance** | Wave 5 + Wave 6 | Recon improvements and feature completeness (parallel) |
| **Phase 5: Integrate** | Wave 7 | OpenClaw integration (benefits from Wave 3's ResponseSeverity) |
| **Phase 6: Test** | Wave 8 | Comprehensive test coverage against finalized code |
| **Phase 7: Document** | Wave 9 | Write docs against stable APIs |
| **Phase 8: Polish** | Wave 10 | Performance optimizations (can run anytime) |

### Parallel Execution Matrix

Within each wave, blocks marked as parallel can be assigned to separate sub-agents:

| Wave | Parallel Blocks | Max Concurrent Agents |
|------|----------------|----------------------|
| 1 | A, B, C, D, E | 5 |
| 2 | A, B, C | 3 |
| 3 | A, B, C | 3 |
| 4 | A, B | 2 |
| 5 | — | 1 |
| 6 | A, B | 2 |
| 7 | A, B (then C, D, E) | 2 |
| 8 | A, B, C | 3 |
| 9 | A, B, C, D | 4 |
| 10 | — | 1 |

---

## Summary

| Wave | Focus | Items | Est. Effort | Dependencies | Status |
|------|-------|-------|-------------|--------------|--------|
| 1 | Critical Bug Fixes | 15 | ~4.5 hours | None | ✅ 100% complete |
| 2 | Security & Error Handling | 10 | ~3 hours | None | ✅ 100% complete |
| 3 | Code Quality | 15 | ~5.5 hours | Wave 1 recommended | ✅ 100% complete |
| 4 | Architecture | 9 | ~12 hours | Wave 1 recommended | ⚠️ 4A done, 4B pending |
| 5 | Recon/Fuzzer | 1 | ~3 hours | Wave 1 recommended | ✅ 100% complete |
| 6 | Feature Completeness | 9 | ~6+ hours (excl. TBD) | None | ⚠️ 67% complete |
| 7 | OpenClaw Integration | ~17 | ~20 hours | None | ✅ Complete |
| 8 | Test Coverage | 7 | ~15.5 hours | Waves 1, 3 recommended | ✅ 86% complete |
| 9 | Documentation | 4 | ~18.5 hours | None (benefits from all) | ⚠️ 67% complete |
| 10 | Performance & Polish | 4 | ~1.75 hours | None | ✅ 100% complete |

**Total estimated effort:** ~89.75+ hours (excluding Wave 6 stub tabs which need requirements)
**Completed to date:** ~82% of all plan items

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Failing tests | 0 | 0 |
| Library tests | 851 (default) | 851+ |
| Files with inline tests | ~200/400 (50%) | 200/400 (50%) |
| Doc coverage | ~8% | ~40% |
| Unconditionally compiled stub modules | 0 | 0 |
| Clippy warnings (default) | 0 | 0 |
| Clippy warnings (rest-api) | 12 (pre-existing) | 0 |
| Silent error swallowing sites | 0 | 0 |
| Type-level bugs (overflow, wrong types) | 0 | 0 |
| OpenResponses API | Implemented with 6 tests | Implemented with tests |
| `/v1/models` endpoint | Implemented | Implemented |
| AI routes (`/api/v1/ai/*`) | Implemented | Implemented |
| Agent/task routes | Implemented | Implemented |
| SKILL.md for OpenClaw | Created | Created |

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Wave 4 feature-gating breaks downstream users | Medium | Add all new flags to `full` feature; document migration path |
| Wave 8 test expansion uncovers hidden bugs | Low (good thing) | Fix bugs as found; they represent real issues |
| Wave 3 API changes break existing callers | Medium | Use `cargo check --all-features` to find all callers |
| Wave 9 documentation effort is too large | Low | Focus on public API first; internal docs can be incremental |
| Wave 5 `run_full_recon` refactor introduces regressions | Medium | Comprehensive tests in Wave 8 should catch regressions |
| Wave 7 OpenResponses spec evolves | Medium | Design types to be extensible; use `#[serde(flatten)]` for unknown fields |
| Wave 7 tool execution is slow (scans take time) | High | Support streaming responses; set OpenClaw `timeoutSeconds: 600` |

## Feature Flags

All new code is gated on existing or new feature flags:

| Module | Feature Flag |
|--------|-------------|
| `tool/protocol/openresponses/` | `rest-api` |
| `tool/protocol/ai_routes.rs` | `rest-api` + `ai-integration` |
| `tool/protocol/agent_routes.rs` | `rest-api` |
| `skills/slapper-security/` | Always available (not compiled code) |
| `container` | `container` (existing) |
| `storage` | `database` (existing) |
| `supply_chain` | `sbom` (existing) |
| `hunt` | `advanced-hunting` (new) |
| `compliance` | `compliance` (new) |
| `integrations` | `external-integrations` (new) |
| `workflow` | `finding-workflow` (new) |
| `vuln` | `vuln-management` (new) |

The `full` feature should include all new flags.

## File Change Summary

| Action | File | Wave |
|--------|------|------|
| **Modify** | `crates/slapper/tests/negative_tests.rs` | 1A |
| **Modify** | `crates/slapper/src/tui/components/input.rs` | 1B, 6B |
| **Modify** | `crates/slapper/src/tui/app/runner.rs` | 1B, 1D, 2C |
| **Modify** | `crates/slapper/src/scanner/ports/mod.rs` | 1C |
| **Modify** | `crates/slapper/src/fuzzer/engine/core.rs` | 1C |
| **Modify** | `crates/slapper/src/output/convert.rs` | 1C |
| **Modify** | `crates/slapper/src/config/settings.rs` | 1C |
| **Modify** | `crates/slapper/src/tui/workers/network.rs` | 1D, 3B |
| **Modify** | `crates/slapper/src/tui/app/mod.rs` | 1D, 3B, 4B |
| **Modify** | `crates/slapper/src/tui/ui.rs` | 1D, 2A, 3B, 4B, 6B |
| **Modify** | `crates/slapper/src/recon/runner.rs` | 1E, 5A |
| **Modify** | `crates/slapper/src/waf/mod.rs` | 1E |
| **Modify** | `crates/slapper/src/tui/app/export.rs` | 2C, 10A |
| **Modify** | `crates/slapper/src/tui/app/state_update.rs` | 2C |
| **Modify** | `crates/slapper/src/tui/app/task_management.rs` | 2C, 3B |
| **Modify** | `crates/slapper/src/error/mod.rs` | 2B |
| **Modify** | `crates/slapper/src/utils/scope.rs` | 2B |
| **Modify** | `crates/slapper/src/tool/response.rs` | 2B, 3C |
| **Modify** | `crates/slapper/src/tool/mod.rs` | 2B |
| **Modify** | `crates/slapper/src/tui/components/scrollable.rs` | 3A, 10A |
| **Modify** | `crates/slapper/src/tui/components/selector.rs` | 3A |
| **Modify** | `crates/slapper/src/tui/workers/api.rs` | 3B |
| **Modify** | `crates/slapper/src/tui/workers/fuzzer.rs` | 3B |
| **Modify** | `crates/slapper/src/tui/app/dispatch.rs` | 3B, 4B |
| **Modify** | `crates/slapper/src/fuzzer/engine/execution.rs` | 3C |
| **Modify** | `crates/slapper/src/config/loader.rs` | 3C, 4A |
| **Modify** | `crates/slapper/src/utils/formatting.rs` | 3C |
| **Modify** | `crates/slapper/src/lib.rs` | 4A |
| **Modify** | `crates/slapper/src/cli/mod.rs` | 4A |
| **Modify** | `crates/slapper/src/tui/tabs/mod.rs` | 4A |
| **Create** | `crates/slapper/src/constants.rs` entries | 4A |
| **Modify** | `crates/slapper/src/tui/workers/security.rs` | 6A |
| **Modify** | `crates/slapper/src/tui/help.rs` | 6B |
| **Modify** | `crates/slapper/src/tui/app/navigation.rs` | 10A |
| **Modify** | `crates/slapper/src/tui/app/command.rs` | 10A |
| **Create** | `crates/slapper/src/tool/protocol/openresponses/mod.rs` | 7A |
| **Create** | `crates/slapper/src/tool/protocol/openresponses/types.rs` | 7A |
| **Create** | `crates/slapper/src/tool/protocol/openresponses/handlers.rs` | 7A |
| **Modify** | `crates/slapper/src/tool/protocol/mod.rs` | 7A |
| **Modify** | `crates/slapper/src/tool/protocol/mcp/routes.rs` | 7A |
| **Modify** | `crates/slapper/src/tool/protocol/openai/handlers.rs` | 7B |
| **Create** | `crates/slapper/src/tool/protocol/openai/models.rs` | 7B |
| **Modify** | `crates/slapper/src/tool/protocol/openai/mod.rs` | 7B |
| **Create** | `crates/slapper/src/tool/protocol/ai_routes.rs` | 7C |
| **Create** | `skills/slapper-security/SKILL.md` | 7D |
| **Create** | `skills/slapper-security/INSTALL.md` | 7D |
| **Create** | `crates/slapper/src/tool/protocol/agent_routes.rs` | 7E |
