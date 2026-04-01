# Consolidated Improvement Plan

Generated: 2026-04-01
Consolidated from: plan2.md, plan3.md, plan4.md, plan5.md, plan6.md, plan7.md

## Overview

This plan consolidates all improvement items from 6 separate plan files into a
single prioritized roadmap. Items are organized into **10 waves** by priority
and dependency order. Waves with no inter-dependencies can be executed in
parallel by separate sub-agents.

**Current State:** 350+ tests passing, 0 clippy warnings, clean compilation.
The codebase is production-quality but has accumulated bugs in specific areas:
TUI event handling, fuzzer baseline capture, MCP auth, UTF-8 slicing, scope
enforcement, and distributed/proxy subsystems.

| Metric | Current | Target |
|--------|---------|--------|
| Critical Bugs | ~10 | 0 |
| High Bugs | ~8 | 0 |
| Medium Issues | ~15 | 0 |
| Known Panics (UTF-8) | 4 | 0 |
| TUI Unit Tests | 0 | 50+ |

---

## Wave 1: Critical Bugs (P0)

**Estimated effort:** ~2.5 hours
**Parallelization:** All items independent — each can be a separate sub-agent

### 1.1 Remove Duplicate Key Handlers

**Location:** `crates/slapper/src/tui/app/runner.rs:287-335`

Lines 287-335 duplicate the key handler block from lines 229-280 verbatim
(`h/j/k/l/H/L/G/w/b/n/N`). The first block always matches, making the second
unreachable dead code. This is a maintenance trap.

**Fix:** Delete lines 287-335 entirely.

### 1.2 Fix `g` Key Breaking Insert Mode

**Location:** `crates/slapper/src/tui/app/runner.rs:281-283`

The `g` key handler at line 281 has no `InputMode` guard. In Insert mode, the
guarded arm at line 262 is skipped, and this arm catches `g` before the
character-input catch-all. Pressing `g` in Insert mode jumps to bottom instead
of inserting the character.

**Fix:** Add `if app.mode == InputMode::Normal` guard to line 281.

### 1.3 Fix Mouse Tab Selection for All 22 Tabs

**Location:** `crates/slapper/src/tui/app/runner.rs:76-78`

Hardcoded `15` for tab width and index check. There are 22 tabs; tabs 16-22
(Stress, Report, NSE, Plugin, Settings, History, Dashboard) are unreachable
via mouse click.

**Fix:**
```rust
let tab_count = Tab::all().len();
let tab_width = tab_area.width / tab_count as u16;
let tab_index = (mouse_event.column.saturating_sub(1) / tab_width) as usize;
if tab_index < tab_count {
    app.select_tab(tab_index);
}
```

### 1.4 Fix Concurrency Override in Fuzzer

**Location:** `crates/slapper/src/fuzzer/engine/core.rs:87`

`args.concurrency.max(100)` forces minimum concurrency to 100, ignoring user
values. A user setting `--concurrency 10` gets 100 concurrent requests.

**Fix:** `let concurrency = args.concurrency.clamp(1, 500);`

### 1.5 Fix `default_value = "None"` on Option Fields

**Location:** `crates/slapper/src/cli/fuzz.rs:116,121`

`#[arg(default_value = "None")]` on `Option<String>` produces `Some("None")`
instead of `None`. For `Option<OutputFormat>`, clap fails parsing `"None"`.

**Fix:** Remove both `#[arg(default_value = "None")]` lines. `Option<T>`
defaults to `None` automatically.

### 1.6 Fix `verbose` Silently Dropped in WafStressArgs

**Location:** `crates/slapper/src/cli/fuzz.rs:240`

`From<WafStressArgs> for FuzzArgs` hardcodes `verbose: false` instead of
forwarding `args.verbose`.

**Fix:** Change `verbose: false` to `verbose: args.verbose`.

### 1.7 Fix Fuzzer Baseline Header Capture

**Location:** `crates/slapper/src/fuzzer/engine/utils.rs:94,119`

Baseline response capture creates a new empty `HeaderMap` instead of using
actual response headers. All diffing comparisons show spurious differences.

**Fix:** Pass actual response headers: `resp.headers().clone()`.

### 1.8 Fix MCP Auth Header Bearer Stripping

**Location:** `crates/slapper/src/tool/protocol/mcp/auth.rs:23-27`

The code compares the full `Authorization` header value (including `Bearer `
prefix) against the API key. Properly formatted headers always fail auth.

**Fix:** Strip `Bearer ` prefix before comparison:
```rust
.and_then(|v| v.strip_prefix("Bearer ").or(Some(v)))
```

### 1.9 Fix Scope Bypass on Malformed URLs

**Location:** `crates/slapper/src/utils/scope.rs:12-16`

`check_scope_from_url` returns `Ok(())` when `extract_target_from_url`
returns `None`, silently bypassing scope enforcement.

**Fix:** Return an error when URL cannot be parsed.

### 1.10 Fix IPv6 Address Parsing in Cluster Handler

**Location:** `crates/slapper/src/commands/handlers/cluster.rs:274-281`

`extract_host_and_port` uses `split(':')` which breaks on IPv6 addresses
like `[::1]:9000`. Multiple colons produce many parts; `parts[1]` is empty
(port falls back to 22); `parts[0]` is `[` (broken host).

**Fix:** Use `rsplit_once(':')` or `std::net::SocketAddr::parse`.

### 1.11 Fix XSS Vulnerability in Pipeline Reports

**Location:** `crates/slapper/src/pipeline/report.rs:81-171` (HTML), `173-220` (CSV)

`generate_html()` interpolates user-controlled data without escaping.
`generate_csv()` uses raw `format!` with no CSV escaping.

**Fix:** Use existing `escape_html()` and `escape_csv()` from `convert.rs`.

---

## Wave 2: Panic-Prone Code (P0)

**Estimated effort:** ~1.5 hours
**Parallelization:** All items independent

### 2.1 Fix UTF-8 Byte Slicing in Formatting Functions

**Location:** `crates/slapper/src/utils/formatting.rs:7,15`

Both `strip_controls` and `preserve_all` use `&s[..max_len]` which panics
when byte boundary falls mid-character on multi-byte UTF-8.

**Fix:** Use character-based truncation:
```rust
let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
format!("{}...", truncated)
```

**Tests:** Add cases for Chinese, Japanese, emoji, mixed ASCII+multi-byte.

### 2.2 Fix UTF-8 Byte Slicing in Fuzzer Mutator

**Location:** `crates/slapper/src/fuzzer/mutator.rs:133,157,169`

`truncate`, `add_comment`, `add_whitespace` use `payload.len()` (byte length)
with byte slicing. Multi-byte payloads will panic.

**Fix:** Use `payload.chars().count()` and `payload.chars().take(n).collect()`.

### 2.3 Fix UTF-8 Byte Slicing in Secret Preview

**Location:** `crates/slapper/src/recon/secrets.rs:328-330`

`&value[..20]` panics on multi-byte characters.

**Fix:** `value.chars().take(20).collect::<String>()`

### 2.4 Fix Division by Zero in Client Pool

**Location:** `crates/slapper/src/utils/client_pool.rs:61`

If all builders fail, `clients` is empty and `% self.clients.len()` panics.

**Fix:** Guard: `if self.clients.is_empty() { return None; }`

### 2.5 Fix Panics in Stealth Utilities

**Location:** `crates/slapper/src/utils/stealth.rs:211-214,222`

`random_user_agent` panics when list is empty (`gen_range(0..0)`).
`random_delay` panics when `jitter_min_ms > jitter_max_ms`.

**Fix:** Return default string if list empty; swap or clamp min/max.

---

## Wave 3: Non-Functional Subsystems (P0)

**Estimated effort:** ~3 hours
**Parallelization:** All items independent

### 3.1 Fix or Remove Distributed Worker Module

**Location:** `crates/slapper/src/distributed/worker.rs`

Four independent bugs make this completely non-functional:
1. Task channel sender dropped immediately (line 63)
2. HTTP REST endpoints don't exist (lines 83-122)
3. Heartbeat sends hardcoded zeros (lines 101-122)
4. No task result reporting (lines 126-141)

**Fix (recommended):** Remove the `Worker` module. The `RemoteListener`/
`RemoteClient` in `remote.rs` provides a working TCP protocol.

### 3.2 Fix LineWriter Buffered Data Loss

**Location:** `crates/slapper/src/distributed/io.rs:247-255`

Every `read_line()` call creates a fresh `BufReader`. Buffered data from
previous reads is discarded, making the line-based protocol unreliable.

**Fix:** Store `BufReader` as a field on `LineWriter`.

### 3.3 Fix Proxy Chaining — Connections Not Actually Chained

**Location:** `crates/slapper/src/proxy/socks.rs:382-421`, `proxy/mod.rs:146-194`

Both `chain_connect()` and `create_chained_connection()` discard intermediate
connections. Each proxy connects directly from the local machine.

**Fix:** Use the existing tunneled stream instead of creating fresh connections.

### 3.4 Fix Spoofed Scanner Response Matching

**Location:** `crates/slapper/src/scanner/ports/spoofed.rs:184-193`

When a TCP response is received, ALL entries in `sent_packets` are marked with
the same status. One SYN-ACK marks every port as "open".

**Fix:** Parse response port and match against `sent_packets` individually.

### 3.5 Fix or Remove WAF HTTP Smuggling

**Location:** `crates/slapper/src/waf/bypass/smuggling.rs`

Uses `reqwest`'s high-level API which normalizes requests. Cannot send
conflicting headers, double Content-Length, or malformed chunks.

**Fix (recommended):** Remove smuggling module. Mark as unsupported with
reqwest. Real smuggling requires raw socket control.

---

## Wave 4: Security & Memory Safety (P1)

**Estimated effort:** ~2 hours
**Parallelization:** All items independent

### 4.1 Add Cleanup for MCP Completed Results Map

**Location:** `crates/slapper/src/tool/protocol/mcp/handlers.rs:23`

`completed_results` HashMap grows unboundedly. Results only removed when
explicitly fetched. Long-running servers leak memory.

**Fix:** Add TTL-based cleanup (5 min expiry) or max_results limit.

### 4.2 Add Cleanup for Rate Limiter Token Buckets

**Location:** `crates/slapper/src/tool/ratelimit.rs:76`

Token buckets created per-client but never removed. HashMap grows unboundedly.

**Fix:** Add `last_used` timestamp; remove buckets unused for 10+ minutes.

### 4.3 Fix SSE Stream Heartbeat Logic

**Location:** `crates/slapper/src/tool/protocol/mcp/routes.rs:105-147`

Heartbeat tick and event receive are sequential. Put heartbeat in same
`tokio::select!` as the receive.

### 4.4 Remove API Key from Request Params

**Location:** `crates/slapper/src/tool/protocol/mcp/handlers.rs:178`

`validate_auth_params` accepts API keys in JSON body, which are logged in
access logs and visible in dev tools.

**Fix:** Remove or deprecate. Only accept keys via headers.

### 4.5 Fix Client TLS — MITM Vulnerable

**Location:** `crates/slapper/src/distributed/io.rs:175-185`

`TlsClient` uses `NoVerifier` accepting ANY certificate. Active MITM can
intercept PSK during distributed communication.

**Fix:** Add certificate pinning or prominent runtime warning.

### 4.6 Fix `ProxyEntry::enabled` Default

**Location:** `crates/slapper/src/proxy/config.rs:84`

Proxies from config default to disabled (`bool` defaults to `false`).

**Fix:** `#[serde(default = "default_true")]` with `fn default_true() -> bool { true }`.

### 4.7 Fix Blocking DNS in Async Contexts

**Location:** `proxy/mod.rs:245-266`, `config/scope.rs`

Uses `std::net::ToSocketAddrs` (blocking) in async contexts, can block
Tokio runtime threads.

**Fix:** Use `tokio::net::lookup_host` instead.

---

## Wave 5: TUI Fixes (P1)

**Estimated effort:** ~2 hours
**Parallelization:** Most items independent (5.7 depends on Wave 6.9)

### 5.1 Fix Mouse Event Double-Read

**Location:** `crates/slapper/src/tui/app/runner.rs:93,389`

`event::read()` called twice — once for key events, again for mouse events.
The second `read()` blocks until another event arrives. Mouse clicks require
two keypresses to register.

**Fix:** Read event once, match on both `Key` and `Mouse` variants.

### 5.2 Fix Export Falling Through to JSON

**Location:** `crates/slapper/src/tui/app/mod.rs:1208-1216`

Html, Markdown, Sarif, Junit formats all call `self.export_json()` instead
of proper handlers.

**Fix:** Either implement proper export or log warning and remove from format
cycle.

### 5.3 Remove `println!`/`eprintln!` from TUI Code

**Location:** `crates/slapper/src/tui/app/mod.rs:1349,1355,1357`

Direct stdout/stderr writes corrupt TUI display (raw mode + alternate screen).

**Fix:** Replace with `tracing::info!`/`tracing::error!` or App notification.

### 5.4 Fix Search Destructively Replacing History

**Location:** `crates/slapper/src/tui/app/mod.rs:198-219`

`perform_search` clears history entries and replaces with search results.
Original ordering is lost permanently.

**Fix:** Store search results separately. Display filtered view without
modifying underlying data.

### 5.5 Fix Silent Mutex Lock Failures

**Location:** `crates/slapper/src/tui/app/mod.rs` (16 instances)

`if let Ok(mut h) = self.history.lock()` silently drops poisoned lock errors.

**Fix:** Log lock poisoning or use `.expect()` to fail fast.

### 5.6 Use `_mode_style` for Mode Indicator

**Location:** `crates/slapper/src/tui/ui.rs:616-623`

`_mode_style` is computed but never rendered. Use it to display NOR/INS
indicator in the status bar.

### 5.7 Fix Export `save_export` Using `println!`

**Location:** `crates/slapper/src/tui/app/mod.rs`

`save_export` calls `println!`/`eprintln!` which corrupts TUI alternate screen.

**Fix:** Display messages through TUI (status bar, toast, or log panel).

### 5.8 Fix Default TabInput Trait Anti-Patterns

**Location:** `crates/slapper/src/tui/tabs/mod.rs:255-284`

Default implementations use `for _ in 0..100 { self.handle_up(); }` which
is fragile and slow.

**Fix:** Change defaults to no-ops (`{}`). Tabs needing scrolling should
override with direct scroll calls.

### 5.9 Add `page_up`/`page_down` to Missing Tabs

**Location:** `crates/slapper/src/tui/tabs/{graphql,oauth,cluster,stress,report,nse,plugin}.rs`

These tabs have no `page_up`/`page_down` support; default loops through 100
individual moves.

**Fix:** Override with proper scroll-to-position logic.

---

## Wave 6: Code Quality & Consistency (P2)

**Estimated effort:** ~3 hours
**Parallelization:** Most items independent

### 6.1 Implement `FromStr` Trait for Severity

**Location:** `crates/slapper/src/types.rs:25`

Custom `from_str` inherent method shadows `FromStr` trait.
`"critical".parse::<Severity>()` does not work.

**Fix:** Replace inherent method with `impl std::str::FromStr for Severity`.

### 6.2 Fix `CircuitBreakerRegistry::get_state` Stub

**Location:** `crates/slapper/src/utils/circuit_breaker.rs:157`

`get_state` always returns `None`. Look up breaker by name and return actual
state.

### 6.3 Fix Race Condition in Circuit Breaker

**Location:** `crates/slapper/src/utils/circuit_breaker.rs:41-59`

Three separate locks acquired sequentially in `is_available`. State
transition from `Open` to `HalfOpen` is not atomic.

**Fix:** Use a single `Mutex<CircuitBreakerState>` for all mutable state.

### 6.4 Remove Duplicate `ToolDispatcher` in Registry

**Location:** `crates/slapper/src/tool/registry.rs:135-157`

Simpler `ToolDispatcher` defined here but the real one in `tool/dispatcher.rs`
is re-exported. Registry version is dead code.

**Fix:** Delete lines 135-157.

### 6.5 Remove Duplicate `ToolResult` Type Alias

**Location:** `crates/slapper/src/tool/traits.rs:7` and `tool/mod.rs:38`

Defined in both places. `traits.rs` definition is shadowed.

**Fix:** Keep one, remove the other.

### 6.6 Fix JUnit Empty XML Attributes

**Location:** `crates/slapper/src/output/convert.rs:54-56`

`tests=""`, `failures=""` etc. produce invalid JUnit XML.

**Fix:** Populate with actual values or remove optional attributes.

### 6.7 Remove TCP Protocols from UDP Probe List

**Location:** `crates/slapper/src/scanner/udp_fingerprint.rs:89-91`

Kafka (9092) and Redis (6379) are TCP-only protocols listed as UDP probes.

**Fix:** Remove these entries from `UDP_PROBES`.

### 6.8 Change `proxy_type` from String to Enum

**Location:** `crates/slapper/src/config/settings.rs:90`

`proxy_type: String` accepts invalid values; only fails at runtime.

**Fix:** Change to `enum ProxyType { Http, Socks5, Tor }`.

### 6.9 Fix Duplicate `PortData` Structs

**Location:** `output/convert.rs:31` and `tool/response.rs:328`

Two different `PortData` types with different fields. Causes confusion and
data loss during conversion.

**Fix:** Use the richer definition everywhere, or merge fields.

### 6.10 Fix `update_session_from_results` — No-Op

**Location:** `crates/slapper/src/fuzzer/engine/utils.rs:160-166`

Loop body is empty: `if result.status_code == 200 || result.status_code == 302 {}`

**Fix:** Implement session update logic or remove the function.

### 6.11 Fix `fingerprint_services` Ignoring Concurrency

**Location:** `crates/slapper/src/scanner/fingerprint.rs:190`

Accepts `concurrency` parameter but hardcodes `Semaphore::new(20)`.

**Fix:** `Semaphore::new(concurrency)`

### 6.12 Fix Bypass Success Criteria Inconsistency

**Location:** `crates/slapper/src/waf/bypass/`

Three different `is_bypass_successful()` implementations with conflicting
logic across `headers.rs`, `smuggling.rs`, `evasion.rs`.

**Fix:** Define single function in `bypass/mod.rs`.

### 6.13 Extract Shared `SpoofArgs` Struct

**Location:** `crates/slapper/src/cli/scan.rs`

`PortScanArgs`, `EndpointScanArgs`, and `ScanArgs` all independently define
~14 near-identical spoofing fields.

**Fix:** Extract into shared `SpoofArgs` struct with `#[command(flatten)]`.

### 6.14 Remove `SUPPORTED_WAF_COUNT` Stale Constant

**Location:** `crates/slapper/src/constants.rs:19`

Manually maintained count becomes wrong when WAF signatures change.

**Fix:** Compute from actual data or add assertion test.

---

## Wave 7: Architectural Debt (P2)

**Estimated effort:** ~8 hours
**Parallelization:** 7.1-7.4 independent; 7.5 depends on 7.1/7.3

### 7.1 Consolidate Finding Types

**Location:** `crates/slapper/src/output/`

Four different `Finding` structs with overlapping fields:
`FindingData`, `markdown::Finding`, `AgentFinding`, `trend::Finding`.

**Fix:** Designate `AgentFinding` as canonical. Add `From` impls for others.
Deprecate duplicates.

### 7.2 Unify Duplicate Report Generators

**Location:** `crates/slapper/src/output/`

HTML, CSV, SARIF, JUnit each have two implementations (quick in `convert.rs`
and proper in dedicated modules). Report quality depends on entry point.

**Fix:** Make `convert.rs` functions delegate to proper implementations.

### 7.3 Refactor TUI Tab Dispatch

**Location:** `crates/slapper/src/tui/app/mod.rs` (~1917 lines)

~30 methods with 22-arm match statements = ~700 lines of boilerplate.

**Fix (macro approach — recommended):**
```rust
macro_rules! dispatch_tab {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self.current_tab {
            Tab::Recon => $self.recon.$method($($arg),*),
            Tab::Load => $self.load.$method($($arg),*),
            // ...
        }
    };
}
```

Then split `app/mod.rs` into:
- `app/mod.rs` (~200 lines) — App struct, new(), tab navigation
- `app/dispatch.rs` (~400 lines) — Input handler delegation via macro
- `app/export.rs` (~300 lines) — Export logic
- `app/tasks.rs` (~400 lines) — Task spawning and building
- `app/confirm.rs` (~100 lines) — PendingAction and confirmation

### 7.4 Remove or Fix Dead Code Modules

**Location:** Multiple files

Blanket `#![allow(dead_code)]` in 15+ files masks real issues. Specific
dead code: `fuzzer/redos_detect.rs`, `waf/bypass/evasion.rs` (`HomoglyphMap`),
`waf/bypass/smuggling.rs`, `distributed/command.rs:157-161`.

**Fix:** Remove genuinely unused code. Replace blanket allows with targeted
`#[allow(dead_code)]` on specific items.

### 7.5 Create Shared Task Execution Layer

**Location:** New module `crates/slapper/src/tasks/`

TUI and CLI execute same operations with duplicate logic. TUI builds
`TaskConfig` enums, CLI builds `FuzzArgs` etc., both call separate paths.

**Fix:** Create unified task runner. CLI calls `tasks::run_fuzz(args)`, TUI
calls `tasks::run_fuzz_from_config(config)`. Both return `TaskResult`.

### 7.6 Unify Progress Reporting

**Location:** `crates/slapper/src/tasks/progress.rs` (new)

CLI uses `indicatif` progress bars, TUI uses `mpsc` channels. Each task
handles both separately.

**Fix:** Create `ProgressReporter` trait. Implement for CLI and TUI.
Pass `Box<dyn ProgressReporter>` to task runners.

---

## Wave 8: CLI & TUI UX (P3)

**Estimated effort:** ~4 hours
**Parallelization:** All items independent

### 8.1 Group FuzzArgs Fields

**Location:** `crates/slapper/src/cli/fuzz.rs:44-195`

60+ fields in flat structure. Use clap's `#[command(flatten)]` to organize
into logical sub-structs (Basic, Advanced, Session, Target, Output).

### 8.2 Unify Output Format Enums

**Location:** `cli/mod.rs`, `cli/misc.rs`, `output/csv.rs`

Three separate enums: `OutputFormat`, `ReportFormat`, `ExportFormat` with
overlapping but inconsistent variants.

**Fix:** Consolidate into two enums max. Remove `ReportFormat`, use
`OutputFormat` everywhere.

### 8.3 Make Hidden `--json` Flags Discoverable

**Location:** Multiple CLI arg structs with `#[arg(hide = true)]`

Per-command `--json` flags are hidden. Users won't see them in `--help`.

**Fix:** Remove `#[arg(hide = true)]` or remove per-command flags and rely
on global `--json`.

### 8.4 Add `--dry-run` to More Commands

**Location:** Only `PortScanArgs` has it

Add to `FuzzArgs`, `WafArgs`, `StressArgs`, `LoadArgs`.

### 8.5 Add Config File Validation

**Location:** `crates/slapper/src/config/mod.rs`

Invalid config values silently ignored or cause runtime errors.

**Fix:** Add `SlapperConfig::validate()` checking log level, proxy URL
format, positive timeouts, etc.

### 8.6 Add Tab-Specific Help Content

**Location:** `crates/slapper/src/tui/help.rs`

`HelpManager.sections` HashMap is never populated. `get_help_for_tab`
always returns `None`.

**Fix:** Populate with per-tab help sections (keybindings, options).

### 8.7 Add Mode Indicator to Status Bar

**Location:** `crates/slapper/src/tui/ui.rs:616-623`

Related to Wave 5.6. Use `_mode_style` to render NOR/INS in status bar.

### 8.8 Fix Command Palette Tab Navigation

**Location:** `crates/slapper/src/tui/help.rs:212-217`

Dashboard mapped to shortcut "0" but Recon is "1". Inconsistent with
tab bar numbering.

**Fix:** Make shortcuts match tab bar or remove shortcut numbers.

### 8.9 Reduce Status Bar Duplication

**Location:** `crates/slapper/src/tui/ui.rs:470-614`

22-arm match with near-identical patterns for status display.

**Fix:** Extract helper function that takes tab name + state.

### 8.10 Add Shell Completion for Subcommands

**Location:** `crates/slapper/src/cli/mod.rs`

Global `--generate-shell-completion` works but per-subcommand completion
doesn't.

**Fix:** Enable per-subcommand completions.

---

## Wave 9: Test Coverage (P3)

**Estimated effort:** ~6 hours
**Parallelization:** All items independent (run after Waves 1-5 fix the bugs)

### 9.1 Add TUI State Management Tests

**Location:** New `crates/slapper/tests/tui_tests.rs` or `tui/app/tests.rs`

17,000+ TUI lines with zero test coverage.

**Tests:** Tab navigation, input mode transitions, confirmation flow,
export format cycling, search functionality, history operations.

### 9.2 Add Fuzzer Engine Tests

**Location:** New tests in `crates/slapper/tests/fuzzer_tests.rs`

**Tests:** Baseline capture with actual headers, concurrency respects user
values, mutator UTF-8 safety, session detection.

### 9.3 Add MCP Auth Tests

**Location:** Tests in `auth.rs` or `tests/`

**Tests:** `Bearer <key>`, `Basic <key>`, `X-API-Key: <key>` authenticate
correctly. Invalid/missing key rejected.

### 9.4 Add Scope Enforcement Tests

**Location:** `crates/slapper/tests/scope_tests.rs`

**Tests:** Malformed URL returns error, wildcard matching behavior, empty
target, CIDR edge cases.

### 9.5 Add WAF Bypass Tests

**Location:** Tests in `waf/bypass/`

Zero test coverage for `BypassEngine`, header bypass, evasion, profile
selection, encoding transformations.

### 9.6 Improve Negative Test Assertions

**Location:** `crates/slapper/tests/negative_tests.rs`

Several tests assert `is_ok()` without verifying actual result.

**Fix:** Use `assert_eq!(result.unwrap(), expected_value)`.

### 9.7 Fix No-Op Scanner Test

**Location:** `crates/slapper/tests/scanner_tests.rs:174-180`

`test_port_scan_timeout` only tests `std::time::Duration`, not actual
port scanning behavior.

### 9.8 Add CLI Argument Parsing Tests

**Location:** New tests in `crates/slapper/src/cli/` or `tests/`

**Tests:** Multiple payload types, invalid payload type, profile values,
argument group parsing.

---

## Wave 10: Cleanup & Documentation (P3)

**Estimated effort:** ~3 hours
**Parallelization:** All items independent

### 10.1 Remove Global Clippy Suppressions

**Location:** `crates/slapper/src/lib.rs:50-55`

Six blanket `#[allow(clippy::...)]` hide real issues. Move to specific
items or fix underlying issues.

### 10.2 Remove `#![allow(dead_code)]` in TUI

**Location:** `tui/mod.rs`, `tui/tabs/mod.rs`, `tui/components/*.rs`,
`tui/workers/runner.rs`

Module-level dead code allowances mask unused code.

**Fix:** Remove allowances, fix resulting warnings.

### 10.3 Document Output Patterns

**Location:** `utils/output.rs` or AGENTS.md

Mixed use of `eprintln!` vs `tracing::warn!`. Document the pattern:
- `eprintln!` for progress messages
- `tracing::warn!` for recoverable logged issues
- `tracing::error!` for unrecoverable errors
- `println!` only for final output

### 10.4 Extract Spinner from recon/mod.rs

**Location:** `crates/slapper/src/recon/mod.rs:71-101`

30-line `Spinner` struct in orchestrator module.

**Fix:** Move to `recon/spinner.rs`.

### 10.5 Extract Print Functions from waf/mod.rs

**Location:** `crates/slapper/src/waf/mod.rs:275-336`

`print_detection()` and `print_results()` are pure output functions.

**Fix:** Move to `waf/output.rs`.

### 10.6 Fix AGENTS.md Documentation Errors

**Issues to fix:**
1. Wildcard matching **includes** apex domain (`*.example.com` matches
   `example.com`), AGENTS.md says "correctly excludes"
2. `TargetScope` has NO `pinned_ip` field — AGENTS.md claims it does
3. Verify `mcp-server` feature removal claim

---

## Parallelization Guide

Waves can be parallelized as follows:

```
Sub-agent A: Wave 1 (Critical Bugs)         ──┐
Sub-agent B: Wave 2 (Panic-Prone Code)       ──┤
Sub-agent C: Wave 3 (Non-Functional Systems) ──┼── All independent
Sub-agent D: Wave 4 (Security & Memory)      ──┤
Sub-agent E: Wave 5 (TUI Fixes)              ──┘

                        ↓ After Waves 1-5 complete ↓

Sub-agent F: Wave 6 (Code Quality)           ──┐
Sub-agent G: Wave 7 (Architectural Debt)     ──┼── Mostly independent
Sub-agent H: Wave 8 (CLI/TUI UX)            ──┘

                        ↓ After Waves 1-5 complete ↓

Sub-agent I: Wave 9 (Test Coverage)          ── Tests for fixed bugs
Sub-agent J: Wave 10 (Cleanup & Docs)        ── Independent
```

**Recommended execution order for maximum parallelism:**

1. Launch 5 sub-agents for Waves 1-5 simultaneously
2. After completion, launch 3 sub-agents for Waves 6-8
3. Launch 2 sub-agents for Waves 9-10

**Total estimated effort:** ~35-45 hours (wall time with parallelization: ~12-15 hours)

---

## Dependencies

- Waves 1-5 are all independent of each other
- Wave 5.7 depends on Wave 6.9 (PortData unification for clean export)
- Wave 6.13 (SpoofArgs) is independent
- Wave 7.3 (TUI dispatch refactor) makes Wave 9.1 (TUI tests) easier
- Wave 7.5 (shared tasks) makes Wave 7.6 (progress reporting) cleaner
- Wave 9 tests should be written after the bugs they test are fixed (Waves 1-5)
- Wave 10 can be done anytime

---

## Verification Commands

After each wave:
```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

After all waves:
```bash
cargo test -p slapper --features full
cargo clippy --lib -p slapper -- -D warnings
cargo test --test scanner_tests -p slapper
cargo test --test negative_tests -p slapper
```

Feature-gated builds:
```bash
cargo check --lib -p slapper --no-default-features
cargo check --lib -p slapper --features stress-testing
cargo check --lib -p slapper --features nse
cargo check --lib -p slapper --features python-plugins
cargo check --lib -p slapper --features ruby-plugins
```

---

## Success Criteria

1. [ ] All critical bugs (Waves 1-3) fixed and verified
2. [ ] Zero panics on multi-byte UTF-8 input (all 4 locations)
3. [ ] MCP auth works with `Authorization: Bearer <token>` headers
4. [ ] Scope enforcement rejects malformed URLs
5. [ ] Mouse clicks work on all 22 tabs
6. [ ] Export formats either implemented or clearly marked unavailable
7. [ ] TUI has 50+ state management tests
8. [ ] No `println!`/`eprintln!` in TUI code
9. [ ] Circuit breaker `get_state` returns actual state
10. [ ] `app/mod.rs` reduced from 1917 to < 500 lines
11. [ ] Single canonical Finding type with conversion paths
12. [ ] All existing 350+ tests still pass
13. [ ] Zero clippy warnings maintained
14. [ ] No new blanket `#![allow(...)]` suppressions

---

## Notes

- This codebase is production-quality; changes should be conservative
- Maintain backward compatibility for public APIs and config files
- Follow existing code patterns and conventions (see AGENTS.md)
- After completing each wave, run verification commands
- Update AGENTS.md if new patterns are established
- The UTF-8 panics, MCP auth bugs, and scope bypass are real issues that
  could cause failures in production
