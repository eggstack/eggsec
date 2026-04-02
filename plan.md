# Consolidated Improvement Plan

Consolidated from plans `plan2`–`plan7`. Verified against codebase 2026-04-02.

## Current State

| Metric | Value |
|--------|-------|
| Tests | ~363 passing |
| Build | Clean (default features) |
| Clippy | 0 warnings |
| Feature-gated build | **FAILS** with `--features stress-testing` |
| `tui/app/mod.rs` | 1387 lines (reduced from 2087, still needs work) |
| `recon/mod.rs` | 625 lines |
| `config/settings.rs` | 581 lines |

---

## Wave 1: Critical Fixes (30 min, must be first)

These break compilation or test runs. Fix before everything else.

### 1.1 Fix missing imports in `scanner/ports/spoofed.rs`

**File:** `crates/slapper/src/scanner/ports/spoofed.rs:130-132,169`

**Status:** CONFIRMED — `cargo check --lib -p slapper --features stress-testing` fails. `HashMap`, `AtomicBool`, `Ordering` used without imports inside `scan_ports_spoofed()`.

**Fix:** Add to the function's import block:
```rust
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
```

**Verify:** `cargo check --lib -p slapper --features stress-testing`

### 1.2 Fix stale doctest in `utils/mod.rs`

**File:** `crates/slapper/src/utils/mod.rs:18,25`

**Status:** CONFIRMED — doc example references removed `truncate` function. `cargo test --doc` will fail.

**Fix:** Replace `truncate` references with `strip_controls`:
```rust
//! use slapper::utils::{check_scope, create_http_client, strip_controls};
//! let cleaned = strip_controls("Some text with \x00 control chars");
```

**Verify:** `cargo test --doc -p slapper`

### 1.3 Gate `stress` module declaration behind feature flag

**File:** `crates/slapper/src/lib.rs:65`

**Status:** CONFIRMED — `pub mod stress;` is unconditional. Submodules are gated internally but the module itself is always compiled.

**Fix:**
```rust
#[cfg(feature = "stress-testing")]
pub mod stress;
```

**Verify:** `cargo check --lib -p slapper && cargo check --lib -p slapper --features stress-testing`

---

## Wave 2: Code Quality & Correctness (4-6 hours)

### Sub-wave 2A: Security & Correctness (parallelizable per item)

#### 2A.1 Defer DNS resolution in scope checks

**File:** `crates/slapper/src/config/scope.rs:203,218`

**Status:** CONFIRMED — `TargetScope::parse()` calls `resolve_host()` during construction. `Scope::is_target_allowed()` calls `TargetScope::parse()` on every invocation, causing DNS lookups per request.

**Fix:** Split scope checking: fast path for hostname string matching (no DNS), slow path for DNS + CIDR only when IP-based rules exist.

**Verify:** `cargo test --lib -p slapper -- scope`

#### 2A.2 Preserve timeout value in `SlapperError::Timeout`

**File:** `crates/slapper/src/error/mod.rs:147`

**Status:** CONFIRMED — timeout errors map to `timeout_ms: 0` because reqwest doesn't expose configured timeout. Callers lose timeout context.

**Fix:** Add `with_timeout` helper to `SlapperError`. Call sites that know their timeout use `.map_err(|e| SlapperError::from(e).with_timeout(configured_ms))`.

**Verify:** `cargo test --lib -p slapper -- error`

#### 2A.3 Stop cloning API keys from `SensitiveString` to plain `String`

**File:** `crates/slapper/src/recon/mod.rs:229,233,243-246`

**Status:** CONFIRMED — 6 API keys extracted via `s.expose_secret().to_string()`, producing plain `String` that persists after zeroization.

**Fix:** Pass `&SensitiveString` references to recon modules, or wrap clones in new `SensitiveString`.

**Verify:** `cargo test --lib -p slapper -- recon`

#### 2A.4 Fix non-JSON WAF file output writing empty string

**File:** `crates/slapper/src/waf/mod.rs:255-259`

**Status:** CONFIRMED — when `!self.args.json`, `output` is `String::new()`. File writes empty content.

**Fix:** Generate text output before the file-write block (move format generation into the else branch).

**Verify:** `cargo test --lib -p slapper -- waf`

#### 2A.5 Fix CircuitBreaker TOCTOU race

**File:** `crates/slapper/src/utils/circuit_breaker.rs:67-99`

**Status:** CONFIRMED — `fetch_add` then `load` under lock creates race window. Use `fetch_add` return values instead of loading.

**Verify:** `cargo test --lib -p slapper -- circuit_breaker`

#### 2A.6 Fix `is_vulnerable()` semantics

**File:** `crates/slapper/src/fuzzer/engine/types.rs:24-29`

**Status:** CONFIRMED — returns `true` when `is_waf_blocked` is `true`. WAF block = protection, not vulnerability.

**Fix:** Add `is_true_positive()` method:
```rust
pub fn is_true_positive(&self) -> bool {
    self.is_vulnerable && !self.is_waf_blocked
}
```

**Verify:** `cargo test --lib -p slapper -- fuzzer`

#### 2A.7 Fix overly broad `select_profile()` WAF matching

**File:** `crates/slapper/src/waf/mod.rs:138-143`

**Status:** CONFIRMED — last condition `waf_lower.contains(&sig_lower.to_string())` matches arbitrary substrings (e.g., signature `"a"` matches `"cloudflare"`).

**Fix:** Remove bare `contains()` fallback, use word-boundary matching only.

**Verify:** `cargo test --lib -p slapper -- waf`

#### 2A.8 Fix wrong error variants in `SlapperConfig::validate()`

**File:** `crates/slapper/src/config/settings.rs:517,536`

**Status:** CONFIRMED — `max_retries > 10` returns `InvalidTimeout`; proxy weight returns `InvalidConcurrency`. Both semantically wrong.

**Fix:** Use `ConfigValidationError::Validation` with descriptive messages.

**Verify:** `cargo test --lib -p slapper -- config`

#### 2A.9 Fix `create_dir()` to `create_dir_all()` in TUI export

**File:** `crates/slapper/src/tui/app/mod.rs:835`

**Status:** CONFIRMED — `create_dir()` fails if parent dirs don't exist.

**Fix:** Replace with `create_dir_all()`.

**Verify:** `cargo test --lib -p slapper`

#### 2A.10 Fix `danger_accept_invalid_certs(true)` hardcoded

**File:** `crates/slapper/src/scanner/endpoints.rs:582`

**Status:** CONFIRMED — always ignores TLS cert validation during endpoint scanning.

**Fix:** Make configurable via args, default to `false`.

**Verify:** `cargo test --lib -p slapper -- scanner`

### Sub-wave 2B: Dead Code Removal & Deduplication (parallelizable per item)

#### 2B.1 Remove dead `constants::errors` module

**File:** `crates/slapper/src/constants.rs:64-80`

**Status:** CONFIRMED — 15 constants defined, none used.

**Fix:** Remove the module.

#### 2B.2 Remove duplicate `centered_rect()` from `tui/ui.rs`

**File:** `crates/slapper/src/tui/ui.rs:241` (duplicate of `tui/components/popup.rs:166`)

**Status:** CONFIRMED — identical private function.

**Fix:** Remove from `ui.rs`, import from `popup.rs`.

#### 2B.3 Remove dead TUI code

**Status:** CONFIRMED — all items verified.

| Location | Item | Lines |
|----------|------|-------|
| `tui/components/scrollable.rs:187-323` | `ScrollableTable` struct + impl | ~136 lines |
| `tui/components/progress.rs:85-135` | `StatusBar` struct + impl | ~50 lines |
| `tui/workers/runner.rs:413-461` | `is_retryable_error()` + `run_with_retry()` | ~49 lines |
| `tui/components/popup.rs:186-324` | `help_popup()` function | ~138 lines |

#### 2B.4 Remove `_mode_style` dead variable

**File:** `crates/slapper/src/tui/ui.rs:541`

**Status:** CONFIRMED — computed but never used.

#### 2B.5 Consolidate escape functions

**Files:** `output/convert.rs:164,171`, `output/csv.rs:110`, `output/html.rs:314`

**Status:** CONFIRMED — `escape_csv` duplicated in convert.rs and csv.rs; `escape_html` duplicated in convert.rs and html.rs; `escape_xml` in convert.rs is dead.

**Fix:** Create `output/escape.rs` with canonical implementations. Remove duplicates.

#### 2B.6 Deduplicate fuzzer execution logic

**File:** `crates/slapper/src/fuzzer/engine/execution.rs:57-128 vs 162-234`

**Status:** CONFIRMED — `run_concurrent` and `run_burst_with_session` are nearly identical. `run_sequential` and `run_sequential_with_session` also duplicated.

**Fix:** Extract shared internal method with optional session callback.

#### 2B.7 Remove dead `ScopeError::OutOfScope` variant

**File:** `crates/slapper/src/config/scope.rs`

**Status:** CONFIRMED — never constructed.

#### 2B.8 Fix `urlencoding::decode()` error type

**File:** `crates/slapper/src/utils/urlencoding.rs:18`

**Status:** CONFIRMED — returns `Result<String, String>` instead of `crate::error::Result<String>`.

**Fix:** Use `SlapperError::Parse`.

### Sub-wave 2C: Minor Fixes & Documentation (parallelizable per item)

#### 2C.1 Add `is_empty()` to `ClientPool`

**File:** `crates/slapper/src/utils/client_pool.rs`

**Status:** CONFIRMED — has `len()` but no `is_empty()`.

#### 2C.2 Remove module-level `#![allow(dead_code)]`

**Files:** `utils/rate_limiter.rs:2`, `recon/ssl.rs:2`

**Status:** CONFIRMED — hides unused code.

**Fix:** Replace with targeted `#[allow(dead_code)]` or gate module declaration.

#### 2C.3 Rename `TestType::from_string` to `parse`

**File:** `crates/slapper/src/waf/bypass/mod.rs`

**Status:** CONFIRMED — triggers clippy `should_implement_trait` lint.

#### 2C.4 Replace glob re-exports with explicit exports

**Files:** `commands/handlers/mod.rs`, `cli/mod.rs`

**Status:** CONFIRMED — `pub use module::*` for 8-12 modules causes namespace pollution.

#### 2C.5 Align `utils/` error types with crate conventions

**Files:** `utils/http.rs`, `utils/scope.rs`, `utils/validation.rs`, `utils/parsing.rs`

**Status:** CONFIRMED — these use `anyhow::Result` while core should use `SlapperError`.

#### 2C.6 Fix no-op test assertion

**File:** Test files with `assert!(!config.http.verify_tls || config.http.verify_tls)` — always `true`.

#### 2C.7 Fix `From<anyhow::Error>` to preserve error chain

**File:** `crates/slapper/src/error/mod.rs`

**Status:** CONFIRMED — uses `e.to_string()`, losing chain. Fix: use `format!("{:#}", e)`.

#### 2C.8 Extract magic number to constant

**File:** `crates/slapper/src/fuzzer/engine/utils.rs:130` — hardcoded `100` body length diff threshold.

#### 2C.9 Document `SensitiveString` Hash omission

**File:** `crates/slapper/src/types.rs`

**Fix:** Add doc comment explaining `Hash` is intentionally not implemented.

#### 2C.10 Plan deprecated `Finding` type migration

**File:** `output/` module (21 occurrences of `#[allow(deprecated)]`)

**Fix:** Document migration path (deprecated → `AgentFinding`). Multi-PR effort.

---

## Wave 3: TUI Quick Wins (low effort, medium impact)

These are self-contained TUI improvements that can be done in parallel.

### 3.1 Use `SensitiveString` for credential fields

**File:** `crates/slapper/src/tui/app/options.rs:5-9`

**Status:** CONFIRMED — `bearer`, `cookie`, `api_key`, `proxy_auth`, `auth` all use `Option<String>`.

**Fix:** Change to `Option<SensitiveString>`. Update read sites to use `expose_secret()`.

### 3.2 Implement GraphQL checkbox toggle

**File:** `crates/slapper/src/tui/tabs/graphql.rs:350-352`

**Status:** CONFIRMED — `handle_enter` for Options has empty body with comment `// Toggle focused checkbox`.

**Fix:** Track focused checkbox index, toggle corresponding boolean field on enter.

### 3.3 Implement OAuth checkbox toggle

**File:** `crates/slapper/src/tui/tabs/oauth.rs:387-389`

**Status:** CONFIRMED — identical no-op as GraphQL.

### 3.4 Add `set_error` overrides to missing tabs

**Status:** CONFIRMED — Resume, Report, Proxy tabs silently discard errors via trait default no-op.

**Fix:** Implement `set_error` following existing tab patterns.

### 3.5 Implement WafStress `get_results()`

**File:** `crates/slapper/src/tui/tabs/waf_stress.rs:31-33`

**Status:** CONFIRMED — always returns `None`. Export never works.

### 3.6 Add navigation methods to minimal tabs

**Status:** CONFIRMED — Resume, Nse, Plugin tabs lack `page_up`/`page_down`/`handle_top`/`handle_bottom`.

### 3.7 Remove empty `render_overlays` stubs

**Files:** `tui/tabs/proxy.rs`, `tui/tabs/packet.rs`

**Status:** CONFIRMED — empty override bodies.

### 3.8 Make history limit configurable

**File:** `crates/slapper/src/tui/tabs/history.rs:74`

**Status:** CONFIRMED — hardcoded limit of 100 entries.

### 3.9 Fix phantom keybindings in help docs

**File:** `crates/slapper/src/tui/help.rs:456-501`

**Status:** CONFIRMED — Ctrl+Q, Ctrl+S, Ctrl+R, Ctrl+F, Ctrl+G documented but handlers missing.

**Fix:** Either wire up handlers (recommended) or remove from docs.

### 3.10 Wire up digit keys for direct tab jumping

**File:** `crates/slapper/src/tui/app/runner.rs`

**Status:** CONFIRMED — tab titles show `[1] Recon` etc. but pressing digits does nothing.

### 3.11 Add mouse scroll wheel support

**File:** `crates/slapper/src/tui/app/runner.rs:50-82`

**Status:** CONFIRMED — only `MouseButton::Left` clicks handled. `WheelUp`/`WheelDown` ignored.

### 3.12 Add spinner animation for indeterminate progress

**File:** `crates/slapper/src/tui/components/progress.rs`

**Problem:** Long-running ops with unknown totals show no activity indicator.

---

## Wave 4: TUI Functionality & Architecture (medium-high effort)

### 4.1 Inline input validation feedback

**File:** `crates/slapper/src/tui/components/input.rs`

**Problem:** 5 validators exist but almost never used. No real-time validation during rendering.

**Fix:** Add `validation_error` field, `validate_on_change` flag, render errors below input border.

### 4.2 Fuzzy matching in command palette

**File:** `crates/slapper/src/tui/help.rs`

**Problem:** Uses simple case-insensitive substring match. No fuzzy matching.

### 4.3 Add breadcrumbs to all tabs

**Status:** CONFIRMED — only Proxy and Packet tabs implement `breadcrumb()`. Other 20 tabs show just tab name.

### 4.4 Implement tab-agnostic search

**File:** `crates/slapper/src/tui/app/mod.rs` — `perform_search()`

**Status:** CONFIRMED — search only works on History tab. `TabInput::handle_search` exists but unused.

### 4.5 Complete export support for all tabs

**File:** `crates/slapper/src/tui/app/mod.rs:749-766`

**Status:** CONFIRMED — 12 of 22 tabs have empty `export_json` arms. Plus WafStress returns `None` from `get_results()`.

### 4.6 Fix progress display for tabs showing 0%

**Status:** CONFIRMED — Cluster, Report, Resume, Proxy, Packet, Plugin, Nse tabs always show 0%.

### 4.7 Fix Cluster view result rendering

**File:** `crates/slapper/src/tui/tabs/cluster.rs`

**Status:** CONFIRMED — only Status view has results. Worker/Coordinator views have no result methods.

### 4.8 Create shared Checkbox/Input component

**Files:** `tui/tabs/graphql.rs`, `oauth.rs`, `stress.rs`, `plugin.rs`

**Status:** CONFIRMED — graphql.rs and oauth.rs are heavily duplicated (~400+ lines shared pattern).

### 4.9 Add global loading indicator

**File:** `crates/slapper/src/tui/ui.rs`

**Problem:** No visible indicator when switching tabs while a task runs on another tab.

### 4.10 Tab grouping / collapsing for narrow terminals

**Files:** `tui/tabs/mod.rs`, `tui/ui.rs`, `tui/app/runner.rs`

**Problem:** 22 tabs exceed 200 chars on terminals < 120 cols.

**Fix:** Add `TabGroup` enum, grouped display mode for narrow terminals, `Shift+J`/`Shift+K` for group cycling.

### 4.11 Enum-dispatch trait pattern (replace match blocks)

**Files:** New `tui/app/tab_dispatch.rs`, `tui/app/mod.rs`, `tui/app/dispatch.rs`, `tui/tabs/mod.rs`

**Problem:** 19+ separate 22-arm match statements. Adding a tab requires 15+ updates.

**Fix — phased approach:**
- Phase A: Move `title()`, `cli_command()`, `description()` into per-tab trait impls
- Phase B: Consolidate export dispatch via `base_export_name()` and `get_json_results()` trait methods
- Phase C: Evaluate trait dispatch for `draw_content()`, `draw_breadcrumb()`, `draw_status_bar()`
- Phase D: Merge 8 dispatch macros into 2

### 4.12 Extract `app/mod.rs` into submodules

**File:** `crates/slapper/src/tui/app/mod.rs` (1387 lines)

**Fix:** Extract export, task-building, command-palette logic into separate files. Target: `mod.rs` < 600 lines.

### 4.13 Unify dispatch macros

**File:** `crates/slapper/src/tui/app/dispatch.rs` (295 lines, 8 macros)

**Fix:** Merge into 2 macros (`dispatch_void!`, `dispatch_check!`). Target: ~80 lines.

### 4.14 Responsive layout for small terminals

**Files:** `tui/tabs/fuzz.rs`, `scan.rs`, `settings.rs`, `ui.rs`

**Problem:** Fixed layout proportions break on small terminals. Fuzz tab hardcodes 27 rows for config.

### 4.15 Enhanced mouse support (click-to-focus, click-to-select)

**File:** `crates/slapper/src/tui/app/runner.rs:50-82`

**Problem:** Only tab bar click works. No click-to-focus-input, no click-to-select in lists.

### 4.16 Configurable color theme

**Files:** New `tui/theme.rs`, `config/mod.rs`, `tui/ui.rs`, `tui/tabs/*.rs`

**Problem:** All colors hardcoded. No user configuration, no dark/light mode.

### 4.17 Dynamic help system

**File:** `crates/slapper/src/tui/help.rs:99-735`

**Problem:** Help content hardcoded, doesn't reflect compiled features or runtime state.

### 4.18 Configurable export path with user feedback

**File:** `crates/slapper/src/tui/app/mod.rs:829-851`

**Status:** CONFIRMED — `save_export()` hardcodes `"./exports/"` and ignores existing `OutputConfig.results_dir`.

### 4.19 Replace ScrollableText with Table widget where appropriate

**Problem:** ScanPorts, ScanEndpoints, Proxy, Fingerprint tabs use plain `ScrollableText` where `Table` with columns would be better.

### 4.20 Implement Packet tab operations

**File:** `crates/slapper/src/tui/tabs/packet.rs`

**Problem:** Traceroute, ICMP, capture, send operations are stubs saying "Use CLI for this".

---

## Wave 5: Large File Refactoring (non-TUI)

### 5.1 Decompose `recon/mod.rs` (625 lines)

**Fix:** Extract parallel execution to `recon/runner.rs`, output formatting to `recon/output.rs`. Target: `mod.rs` < 150 lines.

### 5.2 Split `config/settings.rs` (581 lines)

**Fix:** Split into `config/settings/` directory with `http.rs`, `scan.rs`, `output.rs`, `recon.rs`, `paths.rs`, `mod.rs`. No file > 200 lines.

### 5.3 Split `waf/detector.rs` (595 lines)

**File:** `crates/slapper/src/waf/detector.rs`

**Problem:** Contains WafDetector struct, 4 async methods, ResponseDiff, and 20+ tests in one file.

**Fix:**
```
waf/detector/
  mod.rs         # WafDetector struct, new(), re-exports
  detect.rs      # detect(), normalize_url()
  block_check.rs # check_waf_block()
  compare.rs     # compare_responses(), ResponseDiff
  types.rs       # WafDetectionResult, WafSignatureLower
```
Target: no file > 200 lines.

### 5.4 Unify error handling: anyhow → SlapperError

**Problem:** Core library modules (waf, scanner, proxy, recon, fuzzer, loadtest, stress, pipeline, distributed, output) use `anyhow::Result` instead of `crate::error::Result`. ~55 usages across ~25 core files.

**Fix — phased migration (leaf → root):**

1. Add new `SlapperError` variants: `Proxy(String)`, `Fingerprint(String)`, `Recon(String)`, `LoadTest(String)`
2. Add `From` impls: `ResolveError → Network`, `InvalidHeaderValue → Http`
3. Migrate modules in order: waf → scanner → proxy → recon → fuzzer → loadtest → stress → pipeline → distributed → output
4. Update doc examples from `anyhow::Result` to `slapper::error::Result`
5. Document acceptable `anyhow` usage: `main.rs`, `commands/handlers/`, `tui/`, test code

**Not migrating (acceptable anyhow):** `main.rs`, command handlers, TUI code, test code.

**Verify:** `rg "use anyhow" crates/slapper/src/{waf,scanner,proxy,recon,fuzzer,loadtest,stress,pipeline,distributed,output}/ | wc -l` — target < 10

### 5.5 Extract magic numbers to constants

**File:** Various — extract hardcoded values (e.g., `100` body length threshold in fuzzer) to `constants.rs`.

---

## Wave 6: Multi-Provider AI Integration

Consolidated from plan5. Supports 41 LLM providers.

### 6.1 Add core chat types

**File:** `crates/slapper/src/ai/types.rs`

Add `ChatMessage`, `ChatRequest`, `ChatResponse`, `TokenUsage`, `FinishReason`, `ChatChunk`, `AiTool`. Keep existing types unchanged. Gate behind `ai-integration`.

### 6.2 Define Provider trait

**File:** `crates/slapper/src/ai/providers/mod.rs` (new)

```rust
#[async_trait]
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;
    fn supports_streaming(&self) -> bool;
    async fn chat(&self, request: ChatRequest) -> AiResult<ChatResponse>;
    async fn chat_stream(&self, _request: ChatRequest) -> AiResult<ChatStream>;
}
```

### 6.3 Implement OpenAI-compatible provider

**File:** `crates/slapper/src/ai/providers/openai.rs` (new)

Covers 35+ providers. Differentiated by `base_url`, `default_model`, auth style. Supports Azure deployment URLs.

### 6.4 Implement Anthropic provider

**File:** `crates/slapper/src/ai/providers/anthropic.rs` (new)

Uses `x-api-key` header + `anthropic-version`. `system` is top-level field. Different request/response format.

### 6.5 Implement Gemini provider

**File:** `crates/slapper/src/ai/providers/gemini.rs` (new)

Auto-detects Vertex AI via `GOOGLE_CLOUD_PROJECT` env. API key as query param.

### 6.6 Implement Bedrock provider

**File:** `crates/slapper/src/ai/providers/bedrock.rs` (new)

Bearer token auth (SigV4 deferred). Supports Claude models initially.

### 6.7 Provider registry & presets

**File:** `crates/slapper/src/ai/providers/registry.rs` (new)

Static map of 41 providers with `AuthStyle` enum (Bearer, AnthropicKey, ApiKeyParam, None).

### 6.8 Refactor AiClient

**File:** `crates/slapper/src/ai/client.rs`

Replace monolithic client with provider delegation. Public API unchanged for backward compat.

### 6.9 Config overhaul

**File:** `crates/slapper/src/config/settings.rs`

Add `providers: HashMap<String, AiProviderConfig>` while preserving legacy flat fields.

### 6.10 Wire MCP sampling to provider system

**File:** `crates/slapper/src/tool/protocol/mcp/sampling.rs`

Currently unused. Route through AI provider.

### 6.11 Enhance OpenAI protocol endpoint

**File:** `crates/slapper/src/tool/protocol/openai/handlers.rs`

Current stub returns static response. Wire to `AiClient.chat()` with tool-call loop.

### 6.12 Tests

New integration test file `tests/ai_provider_tests.rs`. Unit tests per provider with mocked HTTP. Config backward compat tests.

### 6.13 Cargo.toml changes

```toml
# Add async-stream to ai-integration deps:
ai-integration = ["tool-api", "eventsource-stream", "async-stream"]
```

No new crates needed.

---

## Wave 7: CI/CD & Tooling

### 7.1 Tighten CI security checks

**File:** `.github/workflows/test.yml`

Remove `continue-on-error: true` from security audit for `main` branch pushes. Add weekly dependency review job.

### 7.2 Pin Rust toolchain version

**File:** `rust-toolchain.toml`

Pin to `1.80` to match MSRV. Currently uses `stable` without pinning.

### 7.3 Migrate to Criterion benchmarks

**File:** `crates/slapper/benches/bench.rs` (203 lines)

Replace custom `Instant`-based benchmarks with Criterion for statistical analysis and regression detection.

### 7.4 Expand proptest regression corpus

**File:** `proptest-regressions/`

Only `utils/formatting.txt` exists. Run all property tests to generate corpus files.

### 7.5 Use strum `EnumIter` for `PayloadType`

**File:** `crates/slapper/src/fuzzer/payloads/mod.rs`

Currently manually lists all 22 variants. Use `#[derive(EnumIter)]` to auto-generate.

---

## Wave 8: Testing & Documentation

### 8.1 Add tests for untested high-value modules

Priority targets: command handlers (14 files), recon modules (`ssl.rs`, `subdomain.rs`, `cors.rs`), fuzzer engine (`core.rs`, `advanced.rs`), output modules (`html.rs`, `csv.rs`, `dedup.rs`).

### 8.2 Fix weak test assertions

**File:** Various test files — replace `assert!(x == y)` with `assert_eq!`, add diagnostic messages.

### 8.3 Audit all doc examples

Run `cargo test --doc -p slapper`. Fix any failures from stale references.

### 8.4 Remove plan-specific items from AGENTS.md

Clean up AGENTS.md references to individual plan files. Keep codebase knowledge.

---

## Parallelization Summary

| Wave | Items | Can parallelize? | Depends on |
|------|-------|-----------------|------------|
| **1** Critical fixes | 3 items | No (sequential, each is a blocker) | — |
| **2A** Security/correctness | 10 items | Yes (different files) | Wave 1 |
| **2B** Dead code/dedup | 8 items | Yes (different files) | Wave 1 |
| **2C** Minor fixes | 10 items | Yes (different files) | Wave 1 |
| **3** TUI quick wins | 12 items | Yes (different tabs/components) | Wave 1 |
| **4** TUI architecture | 20 items | Partially (4.12/4.13 before 4.11) | Wave 3 |
| **5** Large file refactoring | 5 items | Partially (5.1-5.3 parallel, 5.4 sequential) | Wave 2B |
| **6** AI multi-provider | 13 items | Mostly sequential (6.1→6.2→6.3-6.6→6.7-6.9→6.10-6.13) | Wave 1 |
| **7** CI/CD & tooling | 5 items | Yes (independent) | Wave 1 |
| **8** Testing & docs | 4 items | Yes (independent) | All waves |

**Parallel execution blocks:**
- **Block A (after Wave 1):** 2A + 2B + 2C + 3 — 40 items, fully parallel across sub-agents
- **Block B (after Block A):** 4 + 5 + 6 + 7 — ~41 items, partially parallel
- **Block C (after Block B):** 8 — 4 items, final cleanup

---

## Verification Commands

After each wave:

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
cargo test --doc -p slapper
```

Feature combinations:

```bash
cargo check --lib -p slapper --features stress-testing
cargo check --lib -p slapper --features nse
cargo check --lib -p slapper --features rest-api
cargo check --lib -p slapper --features python-plugins
cargo check --lib -p slapper --features full
```

After all waves:

```bash
cargo test -p slapper
cargo clippy --lib -p slapper -- -D warnings
cargo build --release -p slapper
```

---

## Success Criteria

| Criterion | Target |
|-----------|--------|
| `stress-testing` feature | Compiles and tests pass |
| Doc tests | All pass |
| Clippy warnings | 0 |
| Existing tests | All passing |
| WAF text file output | Non-empty |
| Scope DNS calls | Eliminated for hostname-only rules |
| `SensitiveString` API keys | No plain String clones in recon |
| Escape functions | Single canonical location |
| Dead code | Removed (`constants::errors`, `ScopeError::OutOfScope`, TUI dead items) |
| `tui/app/mod.rs` | < 600 lines |
| `recon/mod.rs` | < 150 lines |
| TUI tab exports | All 22 tabs export results |
| AI providers | 4+ providers working (OpenAI, Anthropic, Gemini, Bedrock) |

---

## Rollback Plan

- **Waves 1-3:** Individual commit reverts (each fix is independent)
- **Wave 4:** Phased — can revert individual items without affecting others
- **Wave 6:** AI provider changes are additive; legacy config path preserved
- **All waves:** No public API changes except `open_ports` rename (includes serde alias)

No plan files remain after consolidation. This is the single source of truth.
