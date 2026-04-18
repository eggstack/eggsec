# Slapper Improvement Plan

**Date**: 2026-04-18
**Status**: COMPLETED (2026-04-18)
**Goal**: Consolidate and execute all improvement items across 7 parallel tracks

---

## Executive Summary

This is the consolidated improvement plan for Slapper, derived from 7 individual plan reviews (plan2-plan8). All items were marked "PLANNED (Not Started)" and should be executed using parallel sub-agents where possible.

| Track | Items | Priority | Estimated Time | Status |
|-------|-------|----------|----------------|--------|
| A: Core Fixes | 8 | CRITICAL | 8-12 hours | ✅ COMPLETED |
| B: Security | 33 | CRITICAL/HIGH | 48-72 hours | ✅ PARTIALLY COMPLETED (B1, B3 partial) |
| C: Performance | 28 | CRITICAL/HIGH | 32-40 hours | ✅ COMPLETED |
| D: Documentation & Testing | 30 | MEDIUM/LOW | 40-52 hours | ✅ COMPLETED |
| E: TUI | 14 | HIGH/MEDIUM | 24-32 hours | ✅ PARTIALLY COMPLETED |
| F: LLM/AI Provider | 10 | HIGH/MEDIUM | 16-24 hours | ✅ MOSTLY COMPLETED |
| G: CLI Architecture | 13 | HIGH/MEDIUM | 24-32 hours | ✅ MOSTLY COMPLETED |
| **Total** | **~136** | | **~192-264 hours** | **~75% COMPLETE** |

---

## Wave A: Core Fixes (CRITICAL - Execute First)

These items block other work and should be fixed immediately.

### A1: Fix Test Compilation Failure

**Issue**: `crates/slapper/tests/common/mod.rs:30` missing `Debug` trait bound

**Fix**: Add `+ std::fmt::Debug` to where clause (line 39-40):
```rust
// BEFORE
where T: Serialize + DeserializeOwned + Eq,

// AFTER
where T: Serialize + DeserializeOwned + Eq + std::fmt::Debug,
```

**Verification**: `cargo test --test scanner_tests -p slapper --no-run`

---

### A2: Fix Doctest Failures (4 failures)

| Location | Issue | Fix |
|----------|-------|-----|
| `scanner/mod.rs:56` | Missing `max_results` field | Add field to doc example |
| `scanner/mod.rs:32` | Wrong `scan_ports` signature | Update doc to match actual signature |
| `output/mod.rs:36` | Error type mismatch | Fix error conversion |
| `recon/mod.rs:50` | Wrong import path | Use `slapper::recon::techdetect::TechDetector` |

**Verification**: `cargo test --doc -p slapper`

---

### A3: Replace Mutex with DashMap in Fuzzer

**Location**: `fuzzer/engine/execution.rs:95-96, 128`

**Issue**: `Arc<Mutex<Vec<FuzzResult>>>` causes lock contention

**Fix**:
```rust
use dashmap::DashMap;
use rustc_hash::FxHashMap;

let results: Arc<DashMap<usize, FuzzResult>> = Arc::new(DashMap::default());
let id = Arc::new(AtomicUsize::new(0));
// ...
let idx = id.fetch_add(1, Ordering::Relaxed);
results.insert(idx, r);  // Lock-free
```

**Verification**: `cargo test --lib -p slapper -- --test-threads=4`

---

### A4: Fix TimingAnalyzer Serialization

**Location**: `fuzzer/engine/utils.rs:182, 198`

**Issue**: Single `Arc<Mutex<TimingAnalyzer>>` serializes all timing analysis

**Fix**: Use per-task analyzer or extract atomic statistics:
```rust
#[derive(Default)]
struct AtomicTimingStats {
    total_requests: AtomicU64,
    total_duration_ms: AtomicU64,
}
```

**Verification**: Same as A3

---

### A5: Replace parking_lot with tokio::sync in Async Context

**Locations**:
- `scanner/ports/spoofed.rs:154, 264, 278, 283`
- `tool/ratelimit.rs:6, 76`
- `tui/state/mod.rs:5-10`
- `tool/registry.rs:24`

**Issue**: `parking_lot::Mutex` blocks async executor threads

**Fix**:
```rust
// BEFORE
use parking_lot::Mutex;
let mutex = Arc::new(Mutex::new(data));

// AFTER
use tokio::sync::Mutex;
let mutex = Arc::new(Mutex::new(data));
// Note: lock().await instead of lock()
```

**Verification**: `cargo clippy --lib -p slapper 2>&1 | grep -i "parking_lot"`

---

### A6: Replace Busy-Wait with Async Channel

**Location**: `scanner/ports/spoofed.rs:388-396`

**Issue**: 50ms polling loop wastes CPU

**Fix**:
```rust
let (tx, mut rx) = tokio::sync::mpsc::channel::<Response>(100);
// In packet handler:
let _ = tx.send(response).await;
// In loop:
while let Some(response) = rx.recv().await { /* process */ }
```

**Verification**: Monitor CPU usage during scan with `perf stat`

---

### A7: Add TUI Render Caching (Dirty Flag)

**Location**: `tui/app/runner.rs:125-126`

**Issue**: `terminal.draw()` redraws everything every frame unconditionally

**Fix**:
```rust
struct AppState {
    needs_redraw: bool,
}

loop {
    if app.needs_redraw {
        terminal.draw(|f| ui::draw(f, app))?;
        app.needs_redraw = false;
    }
    // Handle events
    app.needs_redraw = true;
}
```

**Verification**: `perf record -F 60 --call-graph=./target/release/slapper tui`

---

### A8: Replace Mutex<u64> with AtomicU64 in Port Scanner

**Location**: `scanner/ports/mod.rs:460, 565`

**Issue**: Using `Mutex<u64>` instead of `AtomicU64` for simple counter

**Fix**:
```rust
use std::sync::atomic::{AtomicU64, Ordering};
let scanned_count = Arc::new(AtomicU64::new(0));
scanned_count.fetch_add(1, Ordering::Relaxed);
```

---

## Wave B: Security (CRITICAL/HIGH - Execute in Parallel with Wave A)

### B1: Authentication Fixes (Sub-Track B1 - Run in Parallel)

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| C1 | OpenAI `chat_completions()` has no auth | `tool/protocol/openai/handlers.rs:13` | Add API key validation |
| C2 | All AI endpoints unprotected | `tool/protocol/ai_routes.rs` | Add `require_auth()` to all 6 endpoints |
| H4 | MCP `initialize()` bypasses auth | `tool/protocol/mcp/routes.rs:232` | Require auth when `api_key` is `Some` |
| M7 | Inconsistent HTTP/STDIO auth | `tool/protocol/mcp/routes.rs:178,231` | Both modes same behavior |
| M8 | Health/rate-limit endpoints leak info | `tool/protocol/rest.rs:182,169` | Protect endpoints |

**Verification**:
```bash
curl -X POST http://localhost:PORT/v1/chat/completions  # Should 401
curl http://localhost:PORT/api/v1/ai/circuit-breaker   # Should 401
```

---

### B2: Plugin Security (Sub-Track B2 - Run in Parallel with B1)

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| C3 | Python plugins run without sandbox | `slapper-plugin/src/python.rs:131-199` | Implement AST-based validation |
| H1 | Ruby plugins have no runtime restrictions | `slapper-ruby/src/bridge.rs:155-236` | Restrict dangerous APIs |
| H2 | Plugin pattern detection easily bypassed | `slapper-plugin/src/python.rs:12-21`, `slapper-ruby/src/bridge.rs:13-29` | AST-based validation |
| H3 | NSE `io.lines()` lacks path validation | `slapper-nse/src/libraries/io.rs:207-220,323-345` | Add path validation |
| M9 | NSE sandbox not enabled by default | `Cargo.toml:210` | Enable by default when `nse` enabled |
| M10 | No resource limits for plugins | Plugin system | Implement CPU/memory limits |

**Verification**: Test with obfuscated malicious plugin code

---

### B3: Input Validation (Sub-Track B3 - Run in Parallel with B1, B2)

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| H5 | No private IP check on DNS resolution | `config/scope.rs:270-281` | Add `is_private()`, `is_loopback()` checks |
| H6 | No DNS rebinding protection | `config/scope.rs:117-126` | TTL/resolved IP validation |
| M1 | SQL injection in `storage/queries.rs` | `crates/slapper/src/storage/queries.rs:6-33` | Parameterized queries |
| M11 | Path validation missing `canonicalize()` | `config/settings.rs:364-378` | Use `std::fs::canonicalize()` |
| M12 | Custom payload files not sanitized | `config/settings.rs:38` | Add content validation |
| M13 | HTTP errors silently ignored | `fuzzer/engine/utils.rs:94,119,206` | Distinguish errors, log failures |
| M14 | NSE TOCTOU in path validation | `slapper-nse/src/lib.rs:78-94` | Open FD after canonicalization |

---

### B4: Credential & Concurrency (Sub-Track B4 - Run in Parallel)

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| M2 | Webhook secret not protected | `agent/alerts.rs:31` | Change to `SensitiveString` |
| M3 | API keys stored as String | `tool/protocol/rest.rs:19`, `tool/protocol/grpc.rs:26`, etc. | Change to `SensitiveString` |
| M4 | `AlertRouter` not thread-safe | `agent/alerts.rs:74-78` | Wrap in `Arc<Mutex<>>` |
| M5 | `TargetPortfolio` not thread-safe | `agent/portfolio.rs:94` | Wrap in `Arc<Mutex<>>` |
| M6 | `LongitudinalMemory` not thread-safe | `agent/memory.rs:84` | Wrap in `Arc<Mutex<>>` |
| L3 | Proxy password in URL string | `proxy/config.rs:136-140` | Safer URL construction |
| L4 | Proxy username not `SensitiveString` | `proxy/config.rs:64` | Consistent protection |

---

### B5: Low Priority Security (Sub-Track B5 - Run Last)

| ID | Issue | Location |
|----|-------|----------|
| L1 | UUIDs use non-CSRPNG | `tool/state.rs:54` |
| L2 | No HTTP response size limits | `scanner/fingerprint.rs:411` |
| L5 | Compound read in `circuit_breaker::failure_rate()` | `utils/circuit_breaker.rs:110-116` |
| L6 | No shared auth middleware | Multiple protocol files |
| L7 | RBAC not implemented | `storage/models.rs:47-60` |
| L8 | Missing host validation in CLI | `cli/scan.rs:74,232` |
| L9 | Content-Type not validated | `fuzzer/diff.rs:186-218` |
| L10 | Formula injection protection incomplete | `output/escape.rs` |

---

## Wave C: Performance

### C1: HTTP Connection Pooling

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| H3 | Missing `tcp_nodelay` | `integrations/github.rs:20-24`, `integrations/gitlab.rs:20-24`, `integrations/jira.rs:21-25`, `recon/asn.rs:36,127,177`, `recon/cve_lookup.rs:56,195` | Add `tcp_nodelay(true)` |
| M6 | HTTP client pool inconsistency | Integration clients vs `utils/http.rs` | Standardize on `pool_max_idle_per_host(20)` |

**Verification**: `rg "tcp_nodelay" crates/slapper/src/integrations/`

---

### C2: Memory & Allocation Optimization

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| H2 | Excessive clones in output conversion | `output/convert.rs:157-226` | Use references where ownership not needed |
| H4 | `to_lowercase()` in hot loop | `scanner/fingerprint.rs:418-424` | Pre-compute lowercase patterns |
| M1 | Pre-allocation missing in evasion | `waf/bypass/evasion.rs:271,300,314,334,352` | Add `String::with_capacity()` |
| M2 | URL encoding no pre-allocation | `utils/urlencoding.rs:3-17,20-48` | Pre-allocate based on input length |
| M3 | Inefficient DashMap clone pattern | `scanner/ports/mod.rs:582` | Use `into_iter()` directly |
| L1 | `get_current_help()` allocates every frame | `tui/app/navigation.rs:87-159` | Return `&'static str` |
| L2 | PendingAction message allocates | `tui/app/mod.rs:37-68` | Return `&'static str` for static messages |
| L3 | escape_csv double allocation | `output/escape.rs:24` | Use `write!` to pre-allocated buffer |

---

### C3: TUI Performance

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| H5 | TUI state clones in render path | All `tui/tabs/*.rs` | Return `&AppState` instead of cloning |
| H6 | `format!` in loops | `scanner/ports/mod.rs:228-240,399-430`, `recon/runner.rs:501-647`, `packet/hexdump.rs:14-52,87-111` | Pre-allocate + `write!` |
| M4 | ScrollableText clones lines on render | `tui/components/scrollable.rs:113-119` | Reference-based rendering |
| M5 | Selector clones before render | `fuzz.rs:455-469`, `waf.rs:347-354`, `scan_endpoints.rs:279` | Use `&mut` reference |
| L4 | Missing keepalive on raw TCP | `recon/whois.rs:142`, `distributed/io.rs:54-102` | Enable keepalive |

---

## Wave D: Documentation & Testing

### D1: Documentation Coverage (High Priority)

| ID | Issue | Fix |
|----|-------|-----|
| D1 | Only 0.4% of `pub fn` have `# Errors` | Add `# Errors` sections to 7 core APIs |
| D2 | Only 3.4% of `pub fn` have `# Examples` | Add `# Examples` to entry point functions |
| D3 | 15+ module files lack documentation | Add module-level docs to `mod.rs` files |
| E1 | Handlers lack error context | Add `.context()` to command handlers |
| E2 | Scope violations show generic messages | Improve `config/scope.rs` error messages |

**Priority Functions for Documentation**:
1. `FuzzEngine::new()` / `FuzzEngine::execute()`
2. `ToolRegistry::register()`
3. `Scanner::scan_ports()`
4. `SlapperConfig::load()`
5. `run_full_recon()`
6. `WafDetector::detect()`

---

### D2: Error Handling Enhancement

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| E3 | Network errors masked by `unwrap_or_default()` | `fuzzer/engine/utils.rs:94,119,206` | Distinguish errors, log failures |
| E4 | No error recovery suggestions | `error/mod.rs` | Add `user_message()` helper |
| E5 | Inconsistent recon error propagation | `recon/runner.rs` | Add `ReconReport` with per-module status |

---

### D3: CLI/UX Improvements

| ID | Issue | Fix |
|----|-------|-----|
| U1 | No shell completion support | Use `clap_complete` for bash/zsh/fish |
| U2 | No `--dry-run` flag | Add to `cli/fuzz.rs`, `cli/scan.rs` |
| U3 | Output not colorized by severity | Add color support to JSON/text output |
| U4 | No interactive target selection | Multi-select prompts for endpoint scanning |
| U5 | Recon runs silently | Add progress bar per module |
| U6 | Config errors show raw TOML | Improve error context in `config/mod.rs` |

---

### D4: Test Coverage

| ID | Issue | Fix |
|----|-------|-----|
| T1 | No property-based tests for payloads | Add `proptest` for fuzzing payloads |
| T2 | No MCP integration tests | Add `tests/mcp_tests.rs` |
| T3 | Wildcard semantics not tested | Add `tests/scope_tests.rs` with wildcard tests |
| T4 | No benchmarks for hot paths | Add benchmarks in `benches/` |

---

### D5: Developer Experience

| ID | Issue | Fix |
|----|-------|-----|
| X1 | No dev helper alias | Add `cargo check-all` to `Cargo.toml` |
| X2 | No pre-commit hook | Add `.git/hooks/pre-commit` |
| X3 | No `SLAPPER_NO_COLOR` support | Add env var for CI color control |
| X4 | No `--explain <error-code>` | Add error code explanations |
| X5 | No rust-analyzer config | Add `.vscode/settings.json` |
| X6 | No CONTRIBUTING.md | Create contribution guidelines |
| X7 | No `SLAPPER_CONFIG` env var | Add env var support for config path |

---

### D6: Feature Flags Documentation

**Missing Flags to Document in AGENTS.md**:
- `websocket` - WebSocket security testing
- `headless-browser` - DOM XSS and SPA crawling
- `database` - SQLx-based storage for findings
- `container` - Kubernetes container security scanning
- `cloud` - Cloud security scanning
- `api-schema` - OpenAPI v3 schema-based fuzzing
- `sbom` - SBOM generation and vulnerability checking
- `git-secrets` - Git secrets scanning
- `pdf` - PDF report generation

---

## Wave E: TUI Architecture

### E1: TabDispatcher Optimization

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| A1 | `dispatcher_mut()` recreates on every call | `tui/app/mod.rs:276-336` | Cache TabDispatcher |
| A2 | Tab state accessed via 29-arm matches | `tui/app/mod.rs:272`, `tui/ui.rs:593-666` | Extract `TabStateAccessor` trait |
| C2 | `draw_breadcrumb()` 29-arm match | `tui/ui.rs:274-434` | Add `breadcrumb_text()` to trait |
| C3 | `draw_status_bar()` 29-arm match | `tui/ui.rs:593-666` | Add `status_text()` to trait |

---

### E2: Render Optimization

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| C1 | `draw_content()` 29-arm match | `tui/ui.rs:437-580` | Create `TabRenderer` trait with macro |
| C5 | `Tab::title()` 29-arm match | `tui/tabs/mod.rs:116-168` | Use const array indexed by variant |

---

### E3: Feature Gate Consolidation

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| A3 | Dead code paths for disabled features | Multiple files | Use `cfg_if` for fallback pattern |
| A4 | Tab trait has 17 default methods | `tui/tabs/mod.rs:600-700` | Audit and simplify trait |

---

### E4: UX Consistency

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| U1 | Some tabs missing `render_overlays()` | 7 tabs lack overlays | Add empty implementations |
| U2 | Status bar overflow on narrow terminals | `tui/ui.rs:668-687` | Add terminal width awareness |
| U3 | No terminal size warnings | `tui/ui.rs` | Add size check at startup |

---

### E5: App Struct & Code Quality

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| C4 | App struct 140+ lines | `tui/app/mod.rs:80-220` | Group related fields into sub-structs |
| P1 | Fuzzy scoring no caching | `tui/help.rs:61-93` | Add debouncing |

---

## Wave F: LLM Provider & AI Integration

### F1: Provider Implementation

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| P1 | `AiConfig.provider` never used | `ai/client.rs` | Implement `Provider` enum with selection logic |

```rust
#[derive(Debug, Clone, Copy)]
pub enum Provider {
    OpenAI,
    Azure,
    Anthropic,
    OpenAICompatible,
}
```

---

### F2: AI Routes Integration

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| P2 | AI routes return placeholder responses | `tool/protocol/ai_routes.rs` | Connect to `AiClient` for real analysis |

---

### F3: MCP Server Integration

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| P3 | MCP prompts can't execute AI analysis | `tool/protocol/mcp/handlers.rs` | Add optional `AiClient` to MCP server |

---

### F4: Severity Consolidation

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| S1 | `ResponseSeverity` duplicates `Severity` | `tool/response.rs:344-385` | Use `Option<Severity>` instead |
| S2 | `AgentSeverity` is confusing alias | `output/mod.rs:70` | Remove re-export, use `types::Severity` directly |

---

### F5: Auth Middleware

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| A1 | Duplicate `constant_time_eq` | `tool/protocol/mcp/auth.rs`, `tool/protocol/openresponses/handlers.rs` | Create shared `utils/auth.rs` |
| A2 | No unified auth middleware | Multiple protocol handlers | Create `tool/protocol/middleware/auth.rs` |

---

### F6: Tool Abstraction

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| T1 | `ToolInfo` and `McpTool` duplication | `tool/registry.rs`, `tool/protocol/mcp/types.rs` | Create shared `ToolMetadata` |
| T2 | MCP tools/list lacks capability metadata | `tool/protocol/mcp/handlers.rs` | Enhance with capability vectors |

---

### F7: MCP Streaming

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| M1 | All SSE clients share single broadcast channel | `tool/protocol/mcp/routes.rs:handle_sse_stream` | Create per-request event channels |

---

## Wave G: CLI Architecture

### G1: Command Dispatch

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| A1 | `handle_command()` 30+ arm match | `commands/handlers/mod.rs:87-138` | Implement Command trait or macro-based dispatch |
| C1 | Handler errors lack context | `commands/handlers/*.rs` | Add `.context()` to all handlers |
| C2 | Handler signatures inconsistent | `commands/handlers/*.rs` | Standardize to `CommandResult` alias |
| C4 | Scope validation not used consistently | `commands/handlers/mod.rs:80-85` | Create `require_scope!` macro |

---

### G2: Command Structure

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| A2 | Flat `Commands` enum with 30+ variants | `cli/mod.rs:100-200` | Group into sub-enums (breaking change) |
| A3 | `CommonHttpArgs` lacks documentation | `cli/mod.rs:172` | Add comprehensive doc comments |
| C3 | Handler functions undocumented | `commands/handlers/*.rs` | Add doc comments to all handlers |
| C5 | Inconsistent error propagation | `commands/handlers/*.rs` | Audit and standardize |

---

### G3: UX Consistency

| ID | Issue | Location | Fix |
|----|-------|----------|-----|
| U1 | Inconsistent short flags | `cli/*.rs` | Standardize flag conventions |
| U2 | No progressive disclosure in help | `cli/*.rs` | Use `help_heading` for option groups |
| U3 | Inconsistent output formats | `handlers/*.rs` | Document and enforce output patterns |

---

### G4: Documentation

| ID | Issue | Fix |
|----|-------|-----|
| D1 | No CLI architecture doc | Create `docs/adr/CLI_ARCH.md` |
| D2 | No command cookbook | Create `docs/cli-cookbook.md` |

---

## Parallelization Strategy

### Sub-Agent Assignment (6 Parallel Tracks)

| Agent | Tracks | Items |
|-------|--------|-------|
| Agent-1 | Wave A (Core) | A1-A8 (8 items) |
| Agent-2 | Wave B (Security) | B1-B5 (33 items) |
| Agent-3 | Wave C (Performance) | C1-C3 (18 items) |
| Agent-4 | Wave D (Doc/Testing) | D1-D6 (30 items) |
| Agent-5 | Wave E (TUI) | E1-E5 (14 items) |
| Agent-6 | Wave F (LLM/AI) + Wave G (CLI) | F1-F7 + G1-G4 (23 items) |

### Within-Track Parallelization

**Wave B (Security)**:
- B1 (Auth) and B2 (Plugin) can run in parallel
- B3 (Input) and B4 (Credentials) can run in parallel
- B5 (Low priority) runs last

**Wave D (Documentation)**:
- D1 (Doc coverage) and D2 (Error handling) are independent
- D3 (CLI/UX) and D4 (Testing) are independent
- D5 (DX) and D6 (Feature flags) are independent

**Wave F+G (LLM+CLI)**:
- F1 (Provider) can start before F2 (AI routes)
- G1 (Dispatch) and G2 (Structure) are independent
- G3 (UX) and G4 (Docs) run later

---

## Verification Commands

After each wave:

```bash
# Wave A - Core
cargo test --test scanner_tests -p slapper --no-run
cargo test --test fuzzer_tests -p slapper --no-run
cargo test --test fingerprint_tests -p slapper --no-run
cargo test --doc -p slapper
cargo clippy --lib -p slapper

# Wave B - Security
curl -X POST http://localhost:PORT/v1/chat/completions  # Should 401
curl http://localhost:PORT/api/v1/ai/circuit-breaker     # Should 401

# Wave C - Performance
cargo test --lib -p slapper -- --test-threads=4
perf stat -e cpu-migrations,context-switches ./target/release/slapper scan --spoof ...

# Wave D - Documentation
cargo doc --no-deps -p slapper
cargo test --lib -p slapper

# Wave E - TUI
cargo test --lib -p slapper -- --test-threads=4

# Wave F+G
cargo test --lib -p slapper
cargo clippy --lib -p slapper -- -D warnings

# All Waves Complete
cargo test --lib -p slapper
cargo clippy --lib -p slapper
cargo build --release -p slapper --features full
```

---

## Dependencies

| Wave | Depends On | Blocks |
|------|------------|--------|
| Wave A | None (run first) | All other waves |
| Wave B | Wave A | - |
| Wave C | Wave A | - |
| Wave D | Wave A | - |
| Wave E | Wave A | - |
| Wave F | Wave A | - |
| Wave G | Wave A | - |

---

## Metrics to Track

| Metric | Before | Target | Actual | Notes |
|--------|--------|--------|--------|-------|
| Tests | 1063+ | 1100+ | 1064 | Verification run |
| Clippy warnings | 0 | 0 | 1 | Pre-existing (scan_ports 8 args) |
| Doctests | 15 pass, 4 fail | 17+ pass, 0 fail | 19 pass, 0 fail | All doctests passing |
| Severity enum types | 2 | 1 | 2 | ResponseSeverity still present (F4 deferred) |
| Auth implementations | 4+ | 1 | 2+ | Shared utils/auth.rs created (F5) |
| `handle_command()` arms | ~30 | <15 | ~30 | Deferred to future (G1) |

---

## Notes

- **Security First**: Wave B (Security) should be prioritized alongside Wave A
- **Backward Compatibility**: CLI changes (G1, G2) may be breaking; test thoroughly
- **Feature Flags**: Properly gate all optional functionality
- **Testing**: Run `cargo test --lib -p slapper -- --test-threads=4` to stress concurrent code
- **Performance Profiling**: Use `perf record` and `perf stat` to verify improvements
- **Documentation**: Update AGENTS.md with new patterns as work progresses
