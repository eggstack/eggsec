# Slapper Consolidated Improvement Plan

Consolidated from plan2.md, plan3.md, plan4.md, and plan5.md on 2026-03-31.

## Current Status

| Metric | Value |
|--------|-------|
| Tests | 350 passing |
| Build | Clean compilation |
| Clippy | 1 warning (MSRV `is_multiple_of`, non-blocking) |
| Doctests | 14 pass, 1 ignored, 0 fail |
| `SlapperError` variants | 23 |
| `once_cell` in slapper | 0 (replaced with `std::sync::LazyLock`) |
| MSRV | 1.80 |
| `thiserror` | 2.x |
| Largest file | `tui/workers/runner.rs` (1192 lines) |
| `mcp-server` feature | Removed |
| `native-tls` in slapper | Migrated to `rustls` |
| prost/prost-build | Both 0.13 |

## Already Complete

These items from the source plans are confirmed done:

- `waf/detector.rs` split into `waf/detector/` directory (6 files, all <200 lines)
- `SlapperError` has 23 variants (Proxy, Recon, LoadTest, Fingerprint added)
- Core library modules migrated from `anyhow::Result` to `crate::error::Result`
- `lib.rs` documents anyhow usage policy
- Severity import paths in `fuzzer/engine/` use correct re-export path
- `unreachable!` in `fuzzer/chain.rs:148` replaced with error return
- NSE `duration_since` unwraps replaced with `unwrap_or_default()`
- Ruby plugins zero warnings with `--features ruby-plugins`
- prost/prost-build both at 0.13.5
- Config reloading uses `ctx.config` directly (no `load_config()` re-reads)
- Port scanner records open/closed/filtered states (3 match arms in `scanner/ports/mod.rs:306-327`)
- `InvalidHeaderValue` `From` impl added
- Doc examples use `slapper::error::Result` (0 using `anyhow::Result`)
- Unused `_config` parameters removed from `fuzzer::run_cli`, `fuzzer::run_waf_stress`, `waf::run_cli`
- Deprecated `mcp-server` feature removed
- `thiserror` upgraded to 2.x
- `once_cell` replaced with `std::sync::LazyLock` (17 files)
- MSRV set to 1.80 in workspace root + 4 crates
- `native-tls` migrated to `rustls` (distributed/io.rs, distributed/remote.rs, recon/ssl.rs)
- `tool/protocol/mcp.rs` (1710 lines) split into `tool/protocol/mcp/` (6 files, largest 890 lines)
- `tui/app.rs` (2193 lines) split into `tui/app/` (5 files, largest 1192 lines in runner.rs)
- `docs/FEATURES.md` updated with complete feature flag documentation
- `SlapperError` doc examples expanded with helper method usage
- CI workflow updated with plugin feature checks
- Feature flag integration test added (`tests/feature_tests.rs`)
- Scope enforcement audit complete (`tests/scope_tests.rs` added)
- Circuit breaker implemented (`utils/circuit_breaker.rs`)
- Sensitive data logging audit complete (`SensitiveString` helpers added)
- Payload lazy loading implemented (`fuzzer/payloads/mod.rs` uses `LazyLock`)
- Truncation functions renamed (`strip_controls`, `preserve_all`)

---

## Wave 1: Quick Bug Fixes (Independent, Low Risk)

These tasks are independent, touch different files, and can all run in parallel.

### 1.1 Remove Duplicated Keybinding Block in TUI Runner

**Problem:** `tui/app/runner.rs:284-332` contains a verbatim duplicate of lines 226-277. Dead code from a bad merge.

**File:** `crates/slapper/src/tui/app/runner.rs`

**Fix:** Delete lines 284-332 entirely.

**Verify:** `cargo check -p slapper && cargo clippy --lib -p slapper`

**Effort:** 5 min | **Risk:** None

---

### 1.2 Fix Mouse Tab Click Calculation

**Problem:** `tui/app/runner.rs:75-77` uses hardcoded `/ 15` for tab width and `< 15` limit. There are actually 22 tabs. Tabs 15-21 are unreachable by mouse click.

**File:** `crates/slapper/src/tui/app/runner.rs:51-82`

**Fix:** Use `Tab::all().len()` dynamically:
```rust
let tab_count = crate::tui::tabs::Tab::all().len() as u16;
let tab_width = tab_area.width / tab_count;
if tab_width == 0 { return; }
let tab_index = (mouse_event.column.saturating_sub(tab_area.x) / tab_width) as usize;
if tab_index < tab_count as usize { app.select_tab(tab_index); }
```

**Verify:** `cargo check -p slapper`

**Effort:** 10 min | **Risk:** Low

---

### 1.3 Fix WebSocket/gRPC `PayloadType` Misclassification

**Problem:** `websocket.rs` and `grpc.rs` set `payload_type: PayloadType::GraphQL` in `get_payloads()`. There is no `PayloadType::WebSocket` variant.

**Files:**
- `crates/slapper/src/fuzzer/payloads/websocket.rs` — lines 267, 275, 283
- `crates/slapper/src/fuzzer/payloads/grpc.rs` — lines 320, 328
- `crates/slapper/src/fuzzer/payloads/mod.rs` — enum at lines 31-54

**Fix:**
1. Add `WebSocket` variant to `PayloadType` enum (after `Grpc`)
2. Add `Display` arm: `PayloadType::WebSocket => write!(f, "WebSocket")`
3. Add to `all_variants()` slice
4. Add match arm in `get_payloads()`: `PayloadType::WebSocket => websocket::get_payloads()`
5. Fix `websocket.rs` to use `PayloadType::WebSocket`
6. Fix `grpc.rs` to use `PayloadType::Grpc`

**Verify:** `cargo test -p slapper -- websocket grpc`

**Effort:** 15 min | **Risk:** Low

---

### 1.4 Fix Port Scanner Error Logging

**Problem:** Port scanner at `scanner/ports/mod.rs:314-327` classifies errors with zero diagnostic logging.

**File:** `crates/slapper/src/scanner/ports/mod.rs`

**Fix:** Add `tracing::debug!` to each match arm:
```rust
Ok(Err(e)) => { tracing::debug!("Port {} closed on {}: {}", port, host, e); ... }
Err(_) => { tracing::debug!("Port {} filtered (timeout) on {}", port, host); ... }
```

Also fix `ports_scanned: ports_count as u16` (line 349) — change `PortScanResults.ports_scanned` field to `u32` or `usize`.

**Verify:** `cargo check -p slapper`

**Effort:** 15 min | **Risk:** None

---

### 1.5 Fix `CircuitBreakerRegistry::get_state()` Stub

**Problem:** `utils/circuit_breaker.rs:157-159` — `get_state()` always returns `None`. Stub never implemented.

**File:** `crates/slapper/src/utils/circuit_breaker.rs`

**Fix:** Implement properly:
```rust
pub async fn get_state(&self, name: &str) -> Option<CircuitState> {
    let breakers = self.breakers.lock().await;
    breakers.get(name).map(|b| b.get_state().clone())
}
```

**Verify:** `cargo test -p slapper -- circuit_breaker`

**Effort:** 5 min | **Risk:** Low (no external callers)

---

### 1.6 Fix Conflicting `/` Key Binding

**Problem:** `runner.rs:144` toggles command palette, `runner.rs:345` toggles search. Due to match ordering, search toggle is unreachable.

**File:** `crates/slapper/src/tui/app/runner.rs` — lines 144, 345

**Fix:** Decide which behavior `/` should have, remove the unreachable arm. If both needed, assign different keys.

**Effort:** 5 min | **Risk:** Low

---

### 1.7 Fix Double `event::read()` in Event Loop

**Problem:** Two `event::read()` calls per loop iteration (lines 92 and 380). Second call can block or lose events.

**File:** `crates/slapper/src/tui/app/runner.rs` — lines 92, 380

**Fix:** Use a single `event::read()` and match on the variant:
```rust
let event = event::read()?;
match event {
    Event::Key(key) => { /* handle key */ }
    Event::Mouse(mouse_event) => { handle_mouse_event(mouse_event, app); }
    _ => {}
}
```

**Verify:** `cargo check -p slapper`

**Effort:** 15 min | **Risk:** Low

---

## Wave 2: Security Fixes

### 2.1 Fix XSS in HTML Report Converter

**Problem:** `convert_to_html` directly interpolates `report.target`, `report.scan_type`, `report.timestamp` into HTML without escaping.

**File:** `crates/slapper/src/output/convert.rs` — lines 172-213

**Fix:** Add `html_escape(s: &str) -> String` helper that escapes `&`, `<`, `>`, `"`, `'`. Apply to all interpolated fields.

**Verify:** `cargo test -p slapper -- convert`

**Effort:** 20 min | **Risk:** Low

---

### 2.2 Fix JUnit XML Attribute Escaping

**Problem:** XML attributes written without escaping. Special characters in hostnames/messages produce malformed XML.

**File:** `crates/slapper/src/output/junit.rs` — lines 313-343

**Fix:** Use `quick_xml`'s built-in escaping or add `xml_escape` helper. Verify if `quick_xml::events::BytesStart::push_attribute` already handles escaping.

**Verify:** `cargo test -p slapper -- junit`

**Effort:** 20 min | **Risk:** Low

---

### 2.3 Fix JUnit XML Empty Numeric Attributes in `convert.rs`

**Problem:** `convert_to_junit` produces `<testsuites tests="" failures="" errors="" time="">` with empty string attributes.

**File:** `crates/slapper/src/output/convert.rs` — line 55

**Fix:** Compute actual counts and use numeric values.

**Effort:** 15 min | **Risk:** Low

---

### 2.4 Fix Discord Token Regex (Actually Slack Pattern)

**Problem:** `recon/secrets.rs:277` uses `xox[baprs]-` pattern (Slack tokens) but labels it as `SecretType::DiscordToken`.

**File:** `crates/slapper/src/recon/secrets.rs` — line 277

**Fix:** Change to `SecretType::SlackToken`, add proper Discord token pattern.

**Verify:** `cargo test -p slapper -- secrets`

**Effort:** 10 min | **Risk:** None

---

### 2.5 Fix Wildcard Scope Matching Apex Domain

**Problem:** `*.example.com` matches both `sub.example.com` AND `example.com` (line 166). Most bug-bounty programs exclude apex from wildcard scope.

**File:** `crates/slapper/src/config/scope.rs` — line 166

**Fix:** Change wildcard matching to NOT match apex domain:
```rust
if self.pattern.starts_with("*.") {
    let suffix = &self.pattern[1..]; // ".example.com"
    return target.host.ends_with(suffix); // Only subdomains
}
```

**Verify:** `cargo test -p slapper -- scope`

**Effort:** 10 min | **Risk:** Low (behavior change, more correct)

---

### 2.6 Fix DNS Rebinding TOCTOU in Scope Checking

**Problem:** `TargetScope::parse()` resolves hostname to IP at scope-check time. The actual scan later may resolve to a different IP.

**File:** `crates/slapper/src/config/scope.rs` — lines 193-208

**Fix:** Add `pinned_ip: Option<IpAddr>` field to `TargetScope`. Before executing network operations, re-resolve and compare.

**Verify:** `cargo test -p slapper -- scope`

**Effort:** 45 min | **Risk:** Low

---

### 2.7 Use `SensitiveString` for Webhook URLs and TUI Credentials

**Problem:** Webhook URLs are plain `String` in config. TUI `GlobalHttpOptions` uses plain `String` for credentials.

**Files:**
- `crates/slapper/src/config/settings.rs` — lines 419-425
- `crates/slapper/src/tui/app/options.rs` — lines 5-9

**Fix:** Change to `Option<SensitiveString>`. Update TUI display to show `[REDACTED]`.

**Verify:** `cargo check -p slapper && cargo test -p slapper`

**Effort:** 30 min | **Risk:** Low

---

### 2.8 Add Scope Enforcement to TUI Task Runners

**Problem:** CLI command handlers call `ctx.ensure_scope()` before execution. TUI task runners in `workers/runner.rs` have zero scope validation.

**File:** `crates/slapper/src/tui/workers/runner.rs`

**Fix:** Add `scope: Option<Arc<Scope>>` field to `TaskRunner`. Add scope check at start of each `run_*` method.

**Verify:** `cargo check -p slapper`

**Effort:** 1 hour | **Risk:** Medium (TUI integration)

---

### 2.9 Fix ip-api.com HTTP (Not HTTPS) Fallback

**Problem:** Uses `http://` (not `https://`) for ip-api.com. Leaks reconnaissance activity.

**File:** `crates/slapper/src/recon/geolocation.rs` — line 484

**Fix:** Change to `https://ip-api.com/json/{}`. Free tier supports HTTPS.

**Effort:** 5 min | **Risk:** None

---

### 2.10 Fix Geolocation License Key Exposure

**Problem:** MaxMind license key passed as URL query parameter. May be logged by intermediaries.

**File:** `crates/slapper/src/recon/geolocation.rs` — line 214

**Fix:** Already uses HTTP Basic Auth (line 222). Verify the license_key in URL is not needed — if so, remove it from query string.

**Effort:** 15 min | **Risk:** Low

---

### 2.11 Fix `SensitiveFile.severity` Populated with Category String

**Problem:** `content.rs` line 101 sets `severity: content.category.clone()` — a category string instead of a severity level.

**File:** `crates/slapper/src/recon/content.rs` — line 101

**Fix:** Map category to actual severity using `Severity` enum.

**Effort:** 10 min | **Risk:** None

---

### 2.12 Fix Empty Match Pattern for "RedTeam C2" Fingerprint

**Problem:** `scanner/fingerprint.rs` line 286 has an empty match pattern `""` for "RedTeam C2". Empty string matches any response.

**File:** `crates/slapper/src/scanner/fingerprint.rs` — line 286

**Fix:** Add a real match pattern or remove the entry entirely.

**Effort:** 5 min | **Risk:** None

---

## Wave 3: Async Correctness

### 3.1 Migrate `recon/asn.rs` to Async HTTP

**Problem:** Uses `reqwest::blocking::Client`. Blocks tokio runtime threads.

**File:** `crates/slapper/src/recon/asn.rs`

**Fix:** Replace with `reqwest::Client`, make public methods async, update callers.

**Verify:** `cargo check -p slapper`

**Effort:** 30 min | **Risk:** Low

---

### 3.2 Migrate `recon/cve_lookup.rs` to Async HTTP

**Problem:** Uses `reqwest::blocking::Client`. Same issue as 3.1.

**File:** `crates/slapper/src/recon/cve_lookup.rs`

**Fix:** Replace with `reqwest::Client`, make public methods async, update callers.

**Verify:** `cargo check -p slapper`

**Effort:** 30 min | **Risk:** Low

---

### 3.3 Fix Blocking DNS Lookups in `recon/dns_enhanced.rs`

**Problem:** Uses `dns_lookup::lookup_host()` which is blocking.

**File:** `crates/slapper/src/recon/dns_enhanced.rs`

**Fix:** Replace with `hickory_resolver` (already a dependency) for async DNS, or wrap in `tokio::task::spawn_blocking`.

**Effort:** 30 min | **Risk:** Low

---

## Wave 4: Recon Accuracy

### 4.1 Implement Real SSL Certificate Extraction

**Problem:** `extract_certificate_info` returns placeholder text for all fields. TLS versions/ciphers are hardcoded, not negotiated.

**File:** `crates/slapper/src/recon/ssl.rs`

**Fix:** Parse `rustls_pki_types::CertificateDer` to extract real certificate data. Remove hardcoded `supported_versions`/`supported_cipher_suites` or mark as "not tested".

**Verify:** `cargo check -p slapper`

**Effort:** 2 hours | **Risk:** Medium (new dependency, certificate parsing complexity)

---

### 4.2 Remove Alexa Subdomain Query Stub

**Problem:** `query_alexa` always returns empty `HashSet`. Alexa Top Sites API was discontinued in 2022.

**File:** `crates/slapper/src/recon/subdomain.rs` — lines 123-125

**Fix:** Remove `query_alexa` method entirely and remove from `enumerate_subdomains` call chain.

**Effort:** 10 min | **Risk:** None

---

### 4.3 Implement `check_zone_transfer` or Remove It

**Problem:** `check_zone_transfer` always returns empty `Vec`. Dead code.

**File:** `crates/slapper/src/recon/dns_enhanced.rs` — lines 224-226

**Fix:** Either implement zone transfer checking or remove the method and all references.

**Effort:** 30 min (implement) or 10 min (remove) | **Risk:** Low

---

### 4.4 Fix Cloud Discovery "Access Denied" vs "Not Found"

**Problem:** Both 403 (Access Denied) and 404 (Not Found) treated as "not public". A 403 means the bucket exists but is private — a valuable finding.

**File:** `crates/slapper/src/recon/cloud.rs` — lines 79-92

**Fix:** Distinguish between 403 and 404, recording 403 as a "private" finding.

**Effort:** 15 min | **Risk:** Low

---

## Wave 5: Code Quality and Dead Code

### 5.1 Fix `preserve_all` UTF-8 Byte Slicing

**Problem:** `preserve_all` uses `&s[..max_len]` byte slicing. Panics on multi-byte UTF-8 characters if `max_len` falls mid-character.

**File:** `crates/slapper/src/utils/formatting.rs` — line 15

**Fix:** Replace with character-aware truncation using `char_indices()`.

**Verify:** `cargo test -p slapper -- formatting`

**Effort:** 15 min | **Risk:** Low

---

### 5.2 Fix `build_packet_send_task` Wrong Field for Port

**Problem:** `self.packet.filter()` returns a BPF filter string, not a port number. Parsing it as `u16` always fails, defaulting to 80.

**File:** `crates/slapper/src/tui/app/mod.rs` — line 1591

**Fix:** Add a `port` field to the packet tab UI and use that instead of parsing the filter string.

**Effort:** 20 min | **Risk:** Low

---

### 5.3 Fix Silent Export Serialization Failures

**Problem:** `.unwrap_or_default()` on `serde_json::to_string_pretty()` writes empty files on failure with no error logged.

**File:** `crates/slapper/src/tui/app/mod.rs` — lines 1120-1163

**Fix:** Replace with proper error handling and logging.

**Effort:** 20 min | **Risk:** Low

---

### 5.4 Fix Orphaned Tasks in TUI

**Problem:** Starting a new TUI task replaces the handle without aborting the old task.

**File:** `crates/slapper/src/tui/app/mod.rs` — lines 1401-1423

**Fix:** Before spawning a new task, abort the existing one and drain old channels.

**Effort:** 15 min | **Risk:** Low

---

### 5.5 Replace `eprintln!`/`println!` in Export with TUI-Safe Display

**Problem:** `save_export` uses `eprintln!` and `println!` which corrupt the raw-mode terminal.

**File:** `crates/slapper/src/tui/app/mod.rs` — lines 1234-1257

**Fix:** Return result and display via status message in the TUI status bar.

**Effort:** 20 min | **Risk:** Low

---

### 5.6 Remove `#![allow(dead_code)]` and `#![allow(unused_imports)]`

**Problem:** Three files use blanket allow attributes masking real issues:
- `crates/slapper/src/tui/mod.rs:1` — `#![allow(unused_imports)]`
- `crates/slapper/src/tui/tabs/mod.rs:2` — `#![allow(dead_code)]`
- `crates/slapper/src/tui/workers/runner.rs:1` — `#![allow(dead_code)]`

**Fix:** Remove allow attributes. Fix any warnings by removing unused imports or properly gating feature-dependent code.

**Verify:** `cargo check -p slapper --features full && cargo clippy --lib -p slapper --features full`

**Effort:** 30 min | **Risk:** Low

---

## Wave 6: Fuzzer Improvements

### 6.1 Migrate `cmd.rs` to Use `payload_vec!` Macro

**Problem:** `cmd.rs` manually constructs 38 payloads with 370 lines of boilerplate.

**File:** `crates/slapper/src/fuzzer/payloads/cmd.rs`

**Fix:** Refactor to use `payload_vec!` macro.

**Verify:** `cargo test -p slapper -- cmd`

**Effort:** 30 min | **Risk:** Low

---

### 6.2 Fix `payload_vec!` Macro Capacity

**Problem:** Fixed capacity of 64 regardless of actual count.

**File:** `crates/slapper/src/fuzzer/payloads/macros.rs` — line 26

**Fix:** Count tuples at compile time using a helper macro pattern.

**Verify:** `cargo test -p slapper -- payload`

**Effort:** 20 min | **Risk:** Low

---

### 6.3 Fix No-op `update_session_from_results`

**Problem:** Empty `if` block. Does nothing.

**File:** `crates/slapper/src/fuzzer/engine/utils.rs` — lines 160-166

**Fix:** Implement session cookie extraction logic or remove the function.

**Effort:** 20 min | **Risk:** Low

---

### 6.4 Fix Empty HeaderMap in Diffing

**Problem:** `capture_baseline_for_diffing` and `apply_diffing` create empty `HeaderMap::new()` instead of extracting actual response headers.

**File:** `crates/slapper/src/fuzzer/engine/utils.rs` — lines 94, 119

**Fix:** Replace with `response.headers().clone()`.

**Verify:** `cargo test -p slapper -- diff`

**Effort:** 10 min | **Risk:** None

---

### 6.5 Reduce `FuzzerResultConverter` Boilerplate

**Problem:** 7 nearly identical `FuzzerResultConverter` impl blocks.

**File:** `crates/slapper/src/fuzzer/advanced.rs` — lines 59-495

**Fix:** Create a macro or helper function to reduce duplication.

**Effort:** 1 hour | **Risk:** Medium (macro complexity)

---

### 6.6 Add Missing Tests for Payload Modules

**Problem:** Several payload modules have zero tests: `headers.rs`, `compression.rs`, `cache.rs`, `csv.rs`, `soap.rs`, `host.rs`.

**Fix:** Add `#[cfg(test)]` modules with `test_get_payloads_not_empty()` and `test_payload_types_correct()`.

**Effort:** 1 hour | **Risk:** None

---

## Wave 7: WAF and Config Fixes

### 7.1 Unify Bypass Success Criteria

**Problem:** Three different definitions of "bypass successful" across `headers.rs`, `evasion.rs`, `smuggling.rs`.

**Files:** `waf/bypass/headers.rs`, `waf/bypass/evasion.rs`, `waf/bypass/smuggling.rs`

**Fix:** Create a shared function `is_bypass_successful(status, original_status) -> bool`.

**Verify:** `cargo test -p slapper -- waf`

**Effort:** 20 min | **Risk:** Low

---

### 7.2 Remove Unused `HomoglyphMap` Struct

**Problem:** `HomoglyphMap` struct defined but never used.

**File:** `crates/slapper/src/waf/bypass/evasion.rs` — lines 405-424

**Fix:** Remove the struct and its `new()` method.

**Effort:** 5 min | **Risk:** None

---

### 7.3 Document HTTP Smuggling Limitation

**Problem:** `reqwest`/`hyper` normalizes headers, so malformed smuggling headers never reach the network.

**File:** `crates/slapper/src/waf/bypass/smuggling.rs`

**Fix:** Add `tracing::warn!` documenting the limitation. Mark results accordingly.

**Effort:** 10 min | **Risk:** Low

---

### 7.4 Fix `Verbosity` Enum Serialization

**Problem:** `Verbosity` enum serializes as PascalCase while `Severity` uses lowercase. Inconsistent.

**File:** `crates/slapper/src/config/settings.rs` — lines 470-477

**Fix:** Add `#[serde(rename_all = "lowercase")]` to `Verbosity` enum.

**Effort:** 5 min | **Risk:** Low

---

## Wave 8: TUI Architecture Improvements

### 8.1 Replace Match-Based Dispatch with Trait Method

**Problem:** `app/mod.rs` has ~15 delegation methods, each with a 30+ line match statement dispatching to the current tab.

**Files:**
- `crates/slapper/src/tui/tabs/mod.rs`
- `crates/slapper/src/tui/app/mod.rs`

**Fix:** Add a dispatch method to the `Tab` enum that returns a mutable reference to a trait object, or use an enum-based dispatch pattern.

**Verify:** `cargo check -p slapper --features full && cargo test -p slapper --features full`

**Effort:** 2-3 hours | **Risk:** Medium

---

### 8.2 Replace Busy-Loop Defaults in TabInput

**Problem:** Default trait implementations use busy-loops (`for _ in 0..100 { self.handle_left(); }`).

**File:** `crates/slapper/src/tui/tabs/mod.rs` — lines 255-284

**Fix:** Replace defaults with no-op or direct cursor/index manipulation.

**Effort:** 1-2 hours | **Risk:** Low

---

### 8.3 Fix Export Format Fallback

**Problem:** Most export formats fall back to JSON: Html, Markdown, Sarif, Junit all call `self.export_json()`.

**File:** `crates/slapper/src/tui/app/mod.rs` — lines 1106-1113

**Fix:** Wire up existing `output/` module reporters for each format.

**Effort:** 2-4 hours | **Risk:** Medium

---

### 8.4 Implement Real GraphQL Worker Logic

**Problem:** `run_graphql` returns hardcoded fake results.

**File:** `crates/slapper/src/tui/workers/runner.rs` — lines 1086-1117

**Fix:** Perform actual GraphQL security testing using `fuzzer::payloads::graphql` module.

**Verify:** `cargo check -p slapper --features full`

**Effort:** 2-3 hours | **Risk:** Medium

---

### 8.5 Implement Real OAuth Worker Logic

**Problem:** `run_oauth` returns hardcoded fake results.

**File:** `crates/slapper/src/tui/workers/runner.rs` — lines 1119-1157

**Fix:** Perform actual OAuth security testing using `fuzzer::payloads::oauth` module.

**Verify:** `cargo check -p slapper --features full`

**Effort:** 2-3 hours | **Risk:** Medium

---

### 8.6 Implement Real NSE Worker Logic

**Problem:** `run_nse` returns fake output string.

**File:** `crates/slapper/src/tui/workers/runner.rs` — lines 1159-1191

**Fix:** Actually execute Nmap NSE scripts via `tokio::process::Command`.

**Verify:** `cargo check -p slapper --features nse`

**Effort:** 1 hour | **Risk:** Low

---

### 8.7 Implement Tab Input Handlers for Stub Tabs

**Problem:** GraphQL, OAuth, Cluster, Stress, Report, Nse, Plugin tabs have empty `{}` bodies for all input handlers.

**File:** `crates/slapper/src/tui/app/mod.rs`

**Fix:** Implement at minimum `handle_enter()`, `handle_escape()`, `is_input_focused()`, `is_running()` for each stub tab.

**Verify:** `cargo check -p slapper --features full`

**Effort:** 2-3 hours | **Risk:** Low

---

### 8.8 Add Confirmation for Destructive Operations

**Problem:** No confirmation for history deletion, tab reset, settings save.

**Fix:** Add a `PendingAction` enum and confirmation dialog popup.

**Effort:** 1-2 hours | **Risk:** Low

---

## Wave 9: Large File Refactoring

### 9.1 Split `tui/workers/runner.rs` (1192 lines)

**Problem:** Single file handles all task types. Style guidelines say split files > 500 lines.

**File:** `crates/slapper/src/tui/workers/runner.rs`

**Proposed split — 7 files, each under 500 lines:**

```
tui/workers/
├── mod.rs         # Re-exports (update)
├── runner.rs      # Types + TaskRunner::run() dispatch (~460 lines)
├── scanner.rs     # run_port_scan, run_endpoint_scan, run_fingerprint (~130 lines)
├── fuzzer.rs      # run_fuzz, run_waf (~160 lines)
├── network.rs     # run_load_test, run_stress_test, run_packet_capture, run_packet_traceroute, run_packet_send (~220 lines)
├── api.rs         # run_graphql, run_oauth, run_nse (~130 lines)
└── recon.rs       # run_recon, run_pipeline (~150 lines)
```

**Verify:** `cargo check -p slapper --features full && cargo test -p slapper --features full`

**Effort:** 1-2 hours | **Risk:** Medium

---

### 9.2 Fix Spoofed Port Scanner — No Response Parsing

**Problem:** `spoofed.rs` sends SYN packets but never reads responses. `_rx` receiver is unused. Every port is reported as "decoy" or "spoofed" regardless of actual state.

**File:** `crates/slapper/src/scanner/ports/spoofed.rs`

**Fix:** Use the `_rx` channel to receive packets. Parse for SYN-ACK (open) vs RST (closed). Add timeout for filtered ports.

**Note:** Non-trivial raw socket implementation. If full response parsing is too large:
- Add `tracing::warn!` documenting that response parsing is not yet implemented
- Change status labels to `"sent"` instead of `"decoy"`/`"spoofed"`

**Verify:** `cargo check -p slapper --features stress-testing`

**Effort:** 2-4 hours | **Risk:** Medium

---

## Wave 10: Documentation and Testing

### 10.1 Document `native-tls` in `slapper-nse`

**Problem:** `native-tls` was removed from main `slapper` crate but `slapper-nse` still uses it (25 usages across 6 files). This is intentional for Nmap compatibility but undocumented.

**Fix:** Add note to `AGENTS.md` under TLS section and `docs/FEATURES.md` under `nse` feature.

**Effort:** 10 min | **Risk:** None

---

### 10.2 Add Missing Tests for Payload Modules

Covered in 6.6 above.

---

### 10.3 Expand Test Coverage

- Property-based tests for parsing modules (proptest)
- Expand negative tests in `tests/negative_tests.rs`
- Chaos testing: inject network failures, timeouts, malformed responses
- Increase coverage for `config/` and `utils/` to 80%

**Effort:** 3 days | **Risk:** Low

---

## Verification Commands

After each wave:
```bash
cargo check -p slapper --features full
cargo test -p slapper --features full
cargo clippy --lib -p slapper
cargo test --doc -p slapper
```

Final verification:
```bash
# Full test suite
cargo test -p slapper --features full

# Lint
cargo clippy -p slapper --features full -- -D warnings

# Doctests
cargo test --doc -p slapper
```

---

## Parallelization Strategy (Waves)

Waves are ordered by dependency and risk. Tasks **within** a wave are independent and can run in parallel with sub-agents. Waves themselves should run sequentially.

| Wave | Focus | Tasks | Can Parallelize |
|------|-------|-------|-----------------|
| 1 | Quick Bug Fixes | 1.1-1.7 | Yes — all touch different files |
| 2 | Security Fixes | 2.1-2.12 | Yes — all independent |
| 3 | Async Correctness | 3.1-3.3 | Yes — different recon modules |
| 4 | Recon Accuracy | 4.1-4.4 | Yes — different recon modules |
| 5 | Code Quality | 5.1-5.6 | Yes — different files |
| 6 | Fuzzer Improvements | 6.1-6.6 | Yes — different fuzzer files |
| 7 | WAF and Config Fixes | 7.1-7.4 | Yes — different modules |
| 8 | TUI Architecture | 8.1-8.8 | Partially — 8.4/8.5/8.6/8.7 independent; 8.1/8.2/8.3 touch shared files |
| 9 | Large File Refactoring | 9.1-9.2 | Yes — independent files |
| 10 | Documentation and Testing | 10.1-10.3 | Yes — independent |

**Sub-agent mapping for Wave 1 (Quick Bug Fixes):**
- Agent 1: 1.1 (duplicate keybindings) + 1.6 (conflicting `/` key)
- Agent 2: 1.2 (mouse tab calculation)
- Agent 3: 1.3 (WebSocket/gRPC types)
- Agent 4: 1.4 (port scanner logging) + 1.5 (circuit breaker stub)
- Agent 5: 1.7 (double event::read)

**Sub-agent mapping for Wave 2 (Security Fixes):**
- Agent 1: 2.1 (XSS) + 2.2 (JUnit escaping) + 2.3 (JUnit empty attrs)
- Agent 2: 2.4 (Discord/Slack regex) + 2.5 (wildcard scope) + 2.9 (ip-api HTTPS)
- Agent 3: 2.6 (DNS rebinding) + 2.7 (SensitiveString) + 2.10 (license key) + 2.11 (severity mapping) + 2.12 (fingerprint)
- Agent 4: 2.8 (TUI scope enforcement)

**Sub-agent mapping for Wave 5 (Code Quality):**
- Agent 1: 5.1 (UTF-8 slicing) + 5.6 (allow attributes)
- Agent 2: 5.2 (packet send port) + 5.3 (export serialization)
- Agent 3: 5.4 (orphaned tasks) + 5.5 (eprintln in export)

---

## Success Criteria

| Criterion | Status |
|-----------|--------|
| Duplicated keybindings removed | Pending |
| Mouse tab calculation dynamic | Pending |
| WebSocket/gRPC PayloadType correct | Pending |
| Port scanner error logging | Pending |
| CircuitBreakerRegistry::get_state() implemented | Pending |
| Conflicting `/` key resolved | Pending |
| Single event::read() per loop | Pending |
| XSS in HTML report fixed | Pending |
| JUnit XML escaping correct | Pending |
| Discord/Slack token patterns correct | Pending |
| Wildcard scope excludes apex | Pending |
| DNS rebinding protection | Pending |
| Webhook URLs use SensitiveString | Pending |
| TUI scope enforcement | Pending |
| ip-api.com uses HTTPS | Pending |
| recon/asn.rs async | Pending |
| recon/cve_lookup.rs async | Pending |
| Blocking DNS lookups fixed | Pending |
| SSL certificate extraction real | Pending |
| Alexa stub removed | Pending |
| preserve_all UTF-8 safe | Pending |
| Export serialization errors logged | Pending |
| Orphaned TUI tasks prevented | Pending |
| Allow attributes removed | Pending |
| cmd.rs uses payload_vec! macro | Pending |
| payload_vec! macro capacity dynamic | Pending |
| No-op session update fixed | Pending |
| Empty HeaderMap in diffing fixed | Pending |
| Verbosity serialization lowercase | Pending |
| tui/workers/runner.rs split | Pending |
| Spoofed scanner response parsing | Pending |
| native-tls in slapper-nse documented | Pending |
| All tests passing | 350+ |
| Clippy warnings | 1 (MSRV, non-blocking) |
