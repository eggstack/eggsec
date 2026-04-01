# Consolidated Improvement Plan

## Overview

This plan consolidates items from plan2–plan9 into a single prioritized roadmap. Items are organized into **waves** that can be executed in parallel by sub-agents when they touch independent file sets.

Items already resolved (eprintln! migration, println! migration, utils SlapperError migration, blocking DNS in scope, config validation, config template, deprecated truncation aliases, etc.) have been removed. Only verified, still-present issues remain.

**Current State (verified 2026-04-01):**

| Metric | Value |
|--------|-------|
| Tests | 363 passing |
| Build | Clean compilation |
| Clippy | 0 warnings (114 → 0, waves 1-2) |
| `eprintln!` in library code | 0 (already migrated) |
| `println!` in library modules | 0 (already migrated) |
| `pub fn anyhow::Result` in utils | 0 (already migrated) |
| Largest file | `tui/app/mod.rs` (2,087 lines) |
| TUI empty dispatch arms | 0 (wave 3 completed) |
| `.bak` files | 0 (removed) |
| Source files | 210 |
| `SlapperError` variants | 23 |
| Tab variants | 22 |

**Completed:**
- Wave 1: Critical bug fixes (fuzzer error swallowing, burst concurrency)
- Wave 2: Code quality & Clippy (114 → 0 warnings)
- Wave 3: TUI wiring fixes (all 22 tabs functional)
- `.bak` files removed (4 files)

**Total Estimated Effort:** 55–85 hours remaining across 7 waves
**Estimated Calendar Time (with parallelization):** 4–5 weeks remaining

---

## Parallelization Strategy

Waves are grouped into **execution blocks**. Within each block, waves touch independent file sets and can run concurrently via sub-agents.

```
Block A (Weeks 1–2):  Wave 1 ─┐
                      Wave 2 ─┤─ All parallel (different files)
                      Wave 3 ─┘

Block B (Weeks 2–3):  Wave 4 (TUI refactor, depends on Wave 3)
                      Wave 5 (Code hygiene, independent)
                      Wave 6 (Foundation features, independent)
                      Wave 7 (Tests, independent)

Block C (Weeks 3–5):  Wave 8 ─┐
                      Wave 9 ─┤─ All parallel (independent modules)
                      Wave 10 ─┘
```

---

## Wave 1: Critical Bug Fixes ✅ COMPLETED

**Risk:** Low | **Effort:** 2–4 hours | **Files:** 2–3

**Status:** Completed 2026-04-01

### Task 1.1: Fix Silent Error Swallowing in Fuzzer

**File:** `crates/slapper/src/fuzzer/engine/execution.rs:108–112`

**Problem:** In `run_concurrent()`, `if let Ok(r) = result` consumes `result`, then `result.err()` always returns `None`. Failed requests are silently dropped.

```rust
// Current (broken):
if let Ok(r) = result {
    results.lock().await.push(r);
} else {
    tracing::debug!("Fuzz request failed: {:?}", result.err()); // always None
}

// Fix:
match result {
    Ok(r) => { results.lock().await.push(r); }
    Err(e) => { tracing::debug!("Fuzz request failed: {:?}", e); }
}
```

### Task 1.2: Fix Unbounded Burst Concurrency (Session Mode)

**File:** `crates/slapper/src/fuzzer/engine/execution.rs:163–177`

**Problem:** `run_burst_with_session()` collects all futures and calls `join_all` with no concurrency limit. With 1000 payloads, all 1000 requests fly simultaneously.

```rust
// Current (unbounded):
let mut futures = Vec::new();
for payload in &payloads {
    futures.push(self.send_fuzz_request(payload, Method::GET));
}
let results: Vec<Result<FuzzResult>> = join_all(futures).await;
```

**Fix:** Use semaphore-based concurrency matching the pattern in `run_concurrent()` (line 79).

**Verification:**
```bash
cargo test --lib -p slapper -- fuzzer
cargo clippy --lib -p slapper
```

---

## Wave 2: Code Quality & Clippy ✅ COMPLETED

**Risk:** Low | **Effort:** 3–5 hours | **Files:** ~15

**Status:** Completed 2026-04-01. 114 → 0 warnings.

### Task 2.1: Auto-fix Clippy Warnings

Run `cargo clippy --fix --lib -p slapper` for the 25 auto-fixable warnings. Then manually address remaining ~89 warnings.

**Warning breakdown:**

| Warning | Count | Fix |
|---------|-------|-----|
| Deprecated `Finding` struct | ~39 | Task 2.2 |
| Unused imports | ~10 | Auto-fix |
| Derivable `impl Default` | 5 | Task 2.3 |
| Too many arguments | 2 | Task 2.4 |
| `from_str` confusion | 2 | Task 2.5 |
| Never-constructed variants | ~9 | Task 2.6 |
| Useless `format!` | 1 | Auto-fix |

### Task 2.2: Remove Deprecated Finding Types

**Files:** `output/markdown.rs`, `output/trend.rs`

Audit usage. If no external consumers, remove deprecated types and migrate references to `AgentFinding`. Otherwise suppress with `#[allow(deprecated)]`.

### Task 2.3: Add `Default` Impls

Add `impl Default` for any types where `Clippy` warns about derivable defaults.

### Task 2.4: Reduce Function Arguments

Use builder pattern, config struct parameter, or `#[allow(clippy::too_many_arguments)]` if internal-only.

### Task 2.5: Rename Conflicting `from_str` Methods

Methods named `from_str` that aren't `FromStr` trait implementations should be renamed (e.g., `parse_from_str`).

### Task 2.6: Audit Dead Code

Review items flagged by Clippy as never-constructed or never-used. Either remove or add `#[allow(dead_code)]` with justification.

**Verification:**
```bash
cargo clippy --lib -p slapper -- -D warnings
# Target: 0 warnings
```

---

## Wave 3: TUI Wiring Fixes ✅ COMPLETED

**Risk:** Medium | **Effort:** 6–10 hours | **Files:** 5–6

**Status:** Completed 2026-04-01. All 22 tabs functional. Added `page_up`/`page_down` to 5 new tab structs.

The TUI has 22 tabs with a central `App` struct dispatching input, tasks, and results. 7 newer tabs (GraphQl, OAuth, Cluster, Stress, Report, Nse, Plugin) render correctly but are **non-functional** because `app/mod.rs` has empty `{}` arms instead of forwarding to tab structs.

### Task 3.1: Fix TabInput Delegation (18+ Methods)

**File:** `crates/slapper/src/tui/app/mod.rs`

Replace empty `{}` arms with proper delegation for all 7 tabs across these methods:

`handle_char`, `handle_backspace`, `handle_tab`, `handle_up`, `handle_down`, `handle_left`, `handle_right`, `handle_focus_next`, `handle_focus_prev`, `is_at_left_edge`, `is_at_right_edge`, `reset_current_tab`, `page_up`, `page_down`, `handle_word_forward`, `handle_word_backward`, `handle_home`, `handle_end`, `handle_top`, `handle_bottom`

**Pattern:**
```rust
// Before:
Tab::GraphQl => {}
// After:
Tab::GraphQl => self.graphql.handle_char(c),
```

Feature-gate Nse/Plugin arms with `#[cfg(feature = "nse")]` / `#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]`.

### Task 3.2: Add `build_*_task` Methods

**File:** `crates/slapper/src/tui/app/mod.rs`

Add missing task builders:
- `build_graphql_task()` — reads endpoint, concurrency, timeout, checkboxes
- `build_oauth_task()` — reads endpoint, client_id, redirect_uri, checkboxes
- `build_nse_task()` — reads target, script, script_args (feature-gated)

**Prerequisites:** Add checkbox accessor methods to `GraphQlTab` and `OAuthTab` if they don't exist.

### Task 3.3: Wire `handle_enter` for Task-Spawning Tabs

For GraphQl, OAuth, Nse — update `handle_enter` arms to call `handle_enter()`, check `is_running()`, then `spawn_task()`.

### Task 3.4: Fix Result Handling

**File:** `crates/slapper/src/tui/app/mod.rs`

- Expand `TaskResult::Error` routing to cover all 22 tabs (currently only handles ~9)
- Add history entries for GraphQl/OAuth results
- Fix WAF history target if hardcoded as `"<target>"`

### Task 3.5: Fix Export Naming

**File:** `crates/slapper/src/tui/app/mod.rs`

Replace `"unknown"` export base names with descriptive names:
```rust
Tab::GraphQl => "graphql_results",
Tab::OAuth => "oauth_results",
Tab::Cluster => "cluster_status",
Tab::Stress => "stress_results",
Tab::Report => "report_results",
Tab::Nse => "nse_results",
Tab::Plugin => "plugin_results",
```

### Task 3.6: Fix History Tab Input Handling

Forward `handle_escape`, `handle_char`, `handle_backspace` for `Tab::History`.

### Task 3.7: Fix Redundant Feature Gates in `ui.rs`

If `draw_breadcrumb()` has identical Plugin arms for both `#[cfg(...)]` and `#[cfg(not(...))]`, collapse to single arm.

**Verification:**
```bash
cargo check --lib -p slapper
cargo check --lib -p slapper --features full
cargo test --lib -p slapper
```

---

## Wave 4: TUI Macro Dispatch Refactor

**Risk:** Medium | **Effort:** 8–12 hours | **Files:** 4–6

**Depends on:** Wave 3 (wiring fixes must be complete first)

### Task 4.1: Introduce `dispatch_tab!` Macro

**File:** New `crates/slapper/src/tui/app/dispatch.rs`

Create a macro that generates 22-arm match dispatch code. Each of the ~30 methods in `app/mod.rs` becomes a one-liner.

```rust
macro_rules! dispatch_tab {
    ($self:expr, $method:ident $(, $args:expr)*) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.$method($($args),*),
            Tab::Fuzz => $self.fuzz.$method($($args),*),
            // ... all 22 tabs
        }
    };
}
```

**Expected reduction:** `app/mod.rs` from ~1,963 lines to ~400–500 lines.

### Task 4.2: Consolidate Tab Metadata

**File:** `crates/slapper/src/tui/tabs/mod.rs`

Replace 22-arm match statements (`title()`, `cli_command()`, `description()`) with a const array of structs indexed by discriminant.

### Task 4.3: Remove `.bak` Files

Delete:
- `crates/slapper/src/tui/tabs/mod.rs.bak`
- `crates/slapper/src/tui/ui.rs.bak`
- `crates/slapper/src/tui/app/runner.rs.bak`
- `crates/slapper/src/tui/app/mod.rs.bak`

### Task 4.4: Remove Duplicated Utilities

If `centered_rect` or other functions are duplicated across files, consolidate to one location. Remove duplicate `"resume"` command palette entry if present.

**Verification:**
```bash
cargo check --lib -p slapper
cargo check --lib -p slapper --features full
cargo clippy --lib -p slapper
```

---

## Wave 5: Code Hygiene

**Risk:** Low | **Effort:** 3–5 hours | **Files:** 5–8

### Task 5.1: Fix Severity `Ord` Footgun

**File:** `crates/slapper/src/types.rs`

`Severity` derives `Ord` by declaration order (Critical < High), which is semantically inverted. Remove derives and provide custom `Ord` implementation using `as_int()`.

**First:** Search for all `Severity` comparisons to find any code relying on the inverted order:
```bash
rg "sort.*[Ss]everity|\.cmp.*[Ss]everity" crates/slapper/src/
```

### Task 5.2: Update Stale Documentation

**File:** `crates/slapper/src/lib.rs`

Update payload type and WAF product counts to match actual values.

### Task 5.3: Remove Dead Code in WAF Evasion Module

If `HomoglyphMap` has empty fields or `EvasionTechnique` variants are never matched, complete or remove them.

### Task 5.4: Fix `strip_controls` Name/Behavior Mismatch

**File:** `crates/slapper/src/utils/formatting.rs`

If `strip_controls` strips all non-ASCII rather than just control characters, either rename to `ascii_only` or change the filter to `!c.is_control()`.

### Task 5.5: Remove Deprecated Truncation Aliases

**File:** `crates/slapper/src/utils/formatting.rs`

If `truncate` and `truncate_simple` are still present and deprecated, verify no callers exist and remove them.

**Verification:**
```bash
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

---

## Wave 6: Foundation Features

**Risk:** Low | **Effort:** 8–12 hours | **Files:** New files + minor modifications

### Task 6.1: `plan` CLI Command

Let users/AI preview execution plans without running them.

**New files:** `cli/plan.rs`, `commands/handlers/plan.rs`
**Modify:** `cli/mod.rs`, `commands/handlers/mod.rs`

Reuse `ChainPlanner`, `ToolRegistry::with_defaults()`. Output as JSON or formatted table.

### Task 6.2: Finding Deduplication Engine

**New file:** `crates/slapper/src/output/dedup.rs`

```rust
pub enum DedupStrategy { Strict, Fuzzy, Disabled }

pub struct DedupEngine {
    strategy: DedupStrategy,
    seen: HashMap<String, Uuid>,
}

impl DedupEngine {
    pub fn deduplicate(&mut self, findings: &[AgentFinding]) -> Vec<AgentFinding>;
}
```

Add `dedup_strategy` field to `SlapperConfig`.

### Task 6.3: AI Feature Flag and Config

**Modify:** `crates/slapper/Cargo.toml`, `crates/slapper/src/config/settings.rs`

```toml
[features]
ai-integration = ["tool-api", "eventsource-stream"]
```

Add `AiConfig` struct (api_url, api_key, model, max_tokens, temperature) to `SlapperConfig`.

### Task 6.4: Structured AI Output Schema

**New file:** `crates/slapper/src/output/ai_schema.rs`

Define `AiOutput`, `AiFinding`, `AiEvidence`, `AiRemediation`, `AiSummary` structs for machine-readable output. Add `AiJson` variant to `OutputFormat`.

**Verification:**
```bash
cargo test --test plan_tests -p slapper
cargo test --lib -p slapper -- output::dedup
cargo check --lib -p slapper --features ai-integration
```

---

## Wave 7: Test Coverage

**Risk:** Low | **Effort:** 6–10 hours | **Files:** 4–6

### Task 7.1: Fix Circuit Breaker Test Flakiness

**File:** `crates/slapper/src/utils/circuit_breaker.rs` (tests)

Replace `tokio::time::sleep` with `tokio::time::pause()` + `tokio::time::advance()` for deterministic time control.

### Task 7.2: Add Circuit Breaker Concurrency Test

Spawn N tasks calling `record_failure()`/`record_success()` concurrently, verify breaker state.

### Task 7.3: Improve Scope Bypass Test Assertions

**File:** `crates/slapper/tests/scope_tests.rs`

Replace weak `assert!(result.is_ok())` with specific assertions checking the out-of-scope target was rejected.

### Task 7.4: Add HTTP Client Behavior Tests

**File:** `crates/slapper/src/utils/http.rs` (tests)

Test timeout enforcement, insecure TLS, proxy configuration.

### Task 7.5: Property-Based Tests

**New file:** `crates/slapper/tests/prop_tests.rs`

- Scope wildcard matching invariants (wildcard matches apex, subdomain matches, rejects unrelated)
- URL parsing invariants
- Severity round-trip and monotonicity invariants

**Verification:**
```bash
cargo test --lib -p slapper -- circuit_breaker
cargo test --test scope_tests -p slapper
cargo test --test prop_tests -p slapper
```

---

## Wave 8: CI/CD Pipeline

**Risk:** Low | **Effort:** 6–10 hours | **Files:** New files + minor modifications

### Task 8.1: `ci` Command

**New files:** `cli/ci.rs`, `commands/handlers/ci.rs`, `output/baseline.rs`

Purpose-built for CI/CD with:
- Exit codes: 0 (pass), 1 (fail), 2 (error), 3 (scope violation)
- `--fail-on <severity>` and `--max-findings <n>` thresholds
- `--baseline <sarif>` for regression-only mode
- SARIF, JUnit, JSON output
- `--quiet` mode for CI-friendly output

### Task 8.2: Baseline Comparison Module

**New file:** `crates/slapper/src/output/baseline.rs`

```rust
pub struct BaselineComparison {
    pub new_findings: Vec<AgentFinding>,
    pub resolved_findings: Vec<AgentFinding>,
    pub unchanged_findings: Vec<AgentFinding>,
}
```

**Verification:**
```bash
cargo test --test ci_tests -p slapper
```

---

## Wave 9: OpenAI-Compatible API & MCP Enhancements

**Risk:** Low-Medium | **Effort:** 8–12 hours | **Files:** New files under `tool/protocol/`

### Task 9.1: OpenAI Function-Calling Endpoint

**New files:** `tool/protocol/openai/mod.rs`, `types.rs`, `handlers.rs`

Expose slapper tools via OpenAI chat completions format at `/v1/chat/completions`. Auto-generate tool definitions from `ToolRegistry`. Feature-gated behind `rest-api`.

### Task 9.2: MCP Prompts

**New file:** `tool/protocol/mcp/prompts.rs`

Built-in prompt templates: `vulnerability-analysis`, `attack-chain`, `remediation`, `scope-check`, `report-summary`, `payload-suggestion`, `waf-bypass`. New MCP methods: `prompts/list`, `prompts/get`.

### Task 9.3: MCP Sampling

**New file:** `tool/protocol/mcp/sampling.rs`

Allow MCP clients to request AI completions through the server. Feature-gated behind `ai-integration`.

### Task 9.4: MCP Tool Output Schema

Add `output_schema() -> Option<serde_json::Value>` to `SecurityTool` trait (default `None`).

**Verification:**
```bash
cargo test --test openai_tests -p slapper --features rest-api
cargo test --lib -p slapper -- tool::protocol::mcp --features rest-api
```

---

## Wave 10: Multi-Agent Orchestration & AI Features

**Risk:** Medium | **Effort:** 16–24 hours | **Files:** New `ai/` module + integrations

### Task 10.1: Agent Registry

**New files:** `tool/agents/mod.rs`, `registry.rs`, `delegation.rs`

Agent registration, heartbeat, task delegation via HTTP callbacks. Feature-gated behind `rest-api`.

### Task 10.2: Agent MCP Methods

New MCP methods: `agents/register`, `agents/unregister`, `agents/list`, `agents/delegate`, `agents/status`, `agents/result`.

### Task 10.3: AI Client Module

**New files:** `ai/mod.rs`, `client.rs`, `types.rs`

Reusable HTTP client for OpenAI-compatible APIs. Supports completions, streaming, finding analysis, payload suggestions, WAF bypass suggestions.

### Task 10.4: `ai-analyze` Command

**New files:** `cli/ai_analyze.rs`, `commands/handlers/ai_analyze.rs`

Post-scan AI analysis: severity reassessment, exploitability analysis, attack chain identification, prioritized remediation.

### Task 10.5: AI Payload Generation

**New file:** `ai/payloads.rs`

Context-aware payload generation based on detected technology. Cache results. Max 50 AI payloads per type. `--ai-payloads` CLI flag.

### Task 10.6: Smart WAF Bypass

**New file:** `ai/waf_bypass.rs`

Iterative AI-driven bypass (max 10 iterations). Builds local knowledge base. Persists to `~/.config/slapper/waf_bypasses.json`.

### Task 10.7: Adaptive Scan Engine

**New file:** `ai/adaptive.rs`

AI-guided pipeline that adjusts strategy based on intermediate findings. Falls back to standard pipeline if AI fails.

**Verification:**
```bash
cargo test --lib -p slapper -- tool::agents --features rest-api
cargo test --lib -p slapper -- ai --features ai-integration
cargo test --test ai_analyze_tests -p slapper --features ai-integration
```

---

## Implementation Order

| Wave | Theme | Risk | Est. Hours | Parallelizable With |
|------|-------|------|------------|---------------------|
| 1 | Critical bug fixes | Low | 2–4 | Waves 2, 3 |
| 2 | Code quality + Clippy | Low | 3–5 | Waves 1, 3 |
| 3 | TUI wiring fixes | Medium | 6–10 | Waves 1, 2 |
| 4 | TUI macro refactor | Medium | 8–12 | Waves 5, 6, 7 |
| 5 | Code hygiene | Low | 3–5 | Waves 4, 6, 7 |
| 6 | Foundation features | Low | 8–12 | Waves 4, 5, 7 |
| 7 | Test coverage | Low | 6–10 | Waves 4, 5, 6 |
| 8 | CI/CD pipeline | Low | 6–10 | Waves 9, 10 |
| 9 | OpenAI API + MCP | Low-Med | 8–12 | Waves 8, 10 |
| 10 | Multi-agent + AI features | Medium | 16–24 | Waves 8, 9 |
| **Total** | | | **66–104** | |

---

## Verification Commands

After each wave:

```bash
cargo check --lib -p slapper
cargo check --lib -p slapper --features full
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

After all waves:

```bash
cargo build --release --features full
cargo test -p slapper --features full
cargo clippy -p slapper --features full -- -D warnings
cargo test --doc -p slapper --features full
```

---

## Feature Flag Summary

| Feature | Gates |
|---------|-------|
| `stress-testing` | Spoofed scanner |
| `rest-api` | Waves 9.1, 9.2, 9.4, 10.1, 10.2 |
| `grpc-api` | Future gRPC service |
| `ai-integration` (new) | Waves 6.3, 9.3, 10.3–10.7 |
| `nse` | Wave 3 (Nse tab wiring) |
| `python-plugins` / `ruby-plugins` | Wave 3 (Plugin tab wiring) |

---

## New Dependencies

| Dependency | Version | Feature Gate | Used In |
|-----------|---------|-------------|---------|
| `eventsource-stream` | 1 | `ai-integration` | AI streaming responses |

All other features reuse existing dependencies (`reqwest`, `axum`, `serde_json`, `tokio`, `async-trait`, `parking_lot`, `dashmap`, `futures`, `chrono`, `thiserror`).

---

## Rollback Plan

- **Waves 1–3:** Individual file changes, easily revertible
- **Wave 4:** Macro introduction + match removal — revert by restoring original match statements from git
- **Waves 5–7:** Low-risk refactors, easily revertible
- **Waves 8–10:** All feature-gated; no impact on core functionality without flags

---

## Success Criteria

| Criterion | Before | After | Status |
|-----------|--------|-------|--------|
| Clippy warnings | 114 | 0 | ✅ Done |
| Fuzzer errors silently dropped | Yes | No (Wave 1.1) | ✅ Done |
| Burst concurrency unbounded | Yes | Semaphore-limited (Wave 1.2) | ✅ Done |
| TUI tabs functional | 15 of 22 | All 22 (Wave 3) | ✅ Done |
| `app/mod.rs` line count | 1,963 | < 600 (Wave 4) | Pending |
| Severity Ord ordering | Inverted | Semantic (Wave 5.1) | Pending |
| .bak files | 4 | 0 | ✅ Done |
| New CLI commands | 0 | plan, ci, ai-analyze (Waves 6, 8, 10) | Pending |
| AI features | 0 | Analysis, payloads, WAF bypass, adaptive scanning (Wave 10) | Pending |
| All tests passing | 363 | 363+ (no regressions) | ✅ Done |
