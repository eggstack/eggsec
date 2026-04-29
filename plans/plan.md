# Slapper Improvement Plan - Master Consolidated

**Date**: 2026-04-29
**Status**: IN PROGRESS
**Priority**: High

---

## Executive Summary

This document is the single source of truth for all planned improvements to Slapper. It consolidates multiple research phases, security reviews, and TUI deep-dives into a wave-based execution model designed for parallelization.

**Current State**:
- **1,115** passing tests (base library)
- **1,364** passing tests (full features)
- **7** pre-existing AI test failures (ai::planner, ai::waf_bypass, ai::client)
- **~21** clippy warnings (mostly TUI-specific acceptable)
- **503** source files, **30** payload types, **29** TUI tabs.

---

## Parallel Execution Strategy (Waves)

The plan is organized into "Waves" and "Sub-waves" to allow multiple agents to work in parallel on independent domains.

| Wave | Domain | Parallel Agents | Key Items |
|------|--------|-----------------|-----------|
| **1** | **Critical & Security** | 3-4 | Security vulnerabilities, compilation blockers, stress module hardening. |
| **2** | **TUI UX & Features** | 2 | Global search, clipboard, pause/resume, render optimizations. |
| **3** | **Core Quality & Refactor** | 2 | Dead code, error handling standardization, large file splitting. |
| **4** | **Performance & Hardening** | 2 | FxHashMap migration, regex cache limits, fuzzer clone reduction. |
| **5** | **Feature Enhancements** | 4-5 | Agent intelligence, plugin API expansion, WAF/Fuzzer gaps. |
| **6** | **Long-term Capabilities**| 1-2 | Exploitation framework, cloud/mobile security, GUI. |
| **7** | **Documentation** | 1 | README, CAPABILITIES, ARCHITECTURE updates. |

---

## WAVE 1: Critical Fixes & Security (High Priority)

### Wave 1A: Security Vulnerabilities
*Goal: Address critical security gaps in authentication and data handling.*

#### 1.1.1 Plaintext Passwords in Auth Results
- **Issue**: Authentication structs store passwords as `String` instead of `SensitiveString`.
- **Files**: `recon/ssh_auth.rs:30-35`, `recon/ftp_auth.rs:19`, `recon/smtp_auth.rs:27-28`, `auth/credential_stuffing.rs:17-22`.
- **Fix**: Replace `password: String` with `password: SensitiveString`. Implement custom serialization if needed.
- **Verification**: `rg "password: String" crates/slapper/src/recon/` should return no matches for credential fields.

#### 1.1.2 WebSocket Authentication Bypass
- **Issue**: `/ws` endpoint in REST API doesn't enforce authentication.
- **File**: `tool/protocol/rest.rs:224-250`.
- **Fix**: Update `ws_handler` to accept `State` and `HeaderMap`, and call `require_auth()`.
- **Verification**: `curl -i -N -H "Upgrade: websocket" http://localhost:8080/ws` should return 401 when API key is configured.

#### 1.1.3 Template Marketplace Verification Silent Failure
- **Issue**: Signature verification errors only log warnings and continue.
- **File**: `scanner/templates/marketplace.rs:133-167`.
- **Fix**: Treat verification errors as fatal; return `Err` instead of logging and continuing.
- **Verification**: Attempt to load a template with an invalid signature; it should be rejected.

---

### Wave 1B: Compilation & Blockers
*Goal: Fix immediate build issues and large enum performance.*

#### 1.1.4 Commands Enum Box Optimization
- **Issue**: `Commands` enum has a very large `FuzzArgs` variant (~776 bytes), causing stack pressure.
- **File**: `crates/slapper/src/cli/mod.rs:95`.
- **Fix**: Change `Fuzz(FuzzArgs)` to `Fuzz(Box<FuzzArgs>)`.
- **Verification**: `cargo clippy --lib -p slapper` should no longer warn about `large_enum_variant`.

#### 1.1.5 Spoofed Scanner Missing Import
- **Issue**: `get_service_name` is called but not imported in `spoofed.rs`.
- **File**: `crates/slapper/src/scanner/ports/spoofed.rs:419`.
- **Fix**: Add `use crate::scanner::ports::get_service_name;`.
- **Verification**: `cargo check --lib -p slapper --features stress-testing` should pass.

---

### Wave 1C: Stress Module Hardening
*Goal: Prevent abuse and improve stability of stress-testing modules.*

#### 1.1.6 Spoof Range Private IP Validation
- **Issue**: Spoof range validation doesn't check for private/reserved IPs.
- **Files**: `stress/syn.rs:234-303`, `stress/udp.rs`, `stress/icmp.rs`.
- **Fix**: Add private IP validation to `parse_spoof_range()`.
- **Verification**: Test with `10.0.0.1-10.0.0.255` range; it should be blocked unless explicitly allowed.

#### 1.1.7 UDP Broadcast Restriction
- **Issue**: UDP flood enables `set_broadcast(true)` unconditionally.
- **File**: `stress/udp.rs:401`.
- **Fix**: Remove `set_broadcast(true)` or restrict it to specific interfaces.

---

## WAVE 2: TUI UX & Features (High Priority)

### Wave 2A: High-Impact TUI Features
*Goal: Implement essential UX features identified in the TUI Deep Dive.*

#### 2.1.1 Global Search (`Ctrl+F`)
- **Goal**: Search across all tabs and results.
- **Implementation**:
  1. Add `global_search` field to `App` struct.
  2. Implement `draw_global_search_overlay()` in `tui/ui.rs`.
  3. Add `searchable` method to `TabState` trait.
- **Key Binding**: `Ctrl+F` to open, `Esc` to close, `Enter` to navigate.

#### 2.1.2 Clipboard Support
- **Goal**: Copy results and findings to system clipboard.
- **Dependency**: Add `arboard` crate.
- **Implementation**: Create `ClipboardManager` in `tui/utils/clipboard.rs`. Add `y` (yank) keybinding.

#### 2.1.3 Progress Pause/Resume
- **Goal**: Allow pausing long-running scans without aborting.
- **Implementation**: Use `PauseToken` (AtomicBool + Notify) in workers (`scanner.rs`, `fuzzer.rs`).
- **Key Binding**: `Ctrl+Z` to pause, `Ctrl+Y` to resume.

#### 2.1.4 Dispatcher Match Arm Refactoring
- **Issue**: 1,500+ lines of repetitive match statements for 39 tabs.
- **Solution**: Use `enum_dispatch` crate to automate trait delegation.
- **Files**: `tui/tabs/mod.rs`, `tui/app/dispatch.rs`.

---

### Wave 2B: TUI Render Optimizations
*Goal: Improve TUI responsiveness and reduce CPU usage.*

#### 2.2.1 Progress Update Throttling
- **Issue**: High-frequency progress updates trigger redundant full redraws.
- **Fix**: Implement 100ms debounce in `tui/app/state_update.rs`.

#### 2.2.2 Breadcrumb & Render Caching
- **Fix**: Cache breadcrumbs on tab change. Use "dirty flag" pattern to skip redraws when state hasn't changed.

---

## WAVE 3: Core Quality & Refactoring

### Wave 3A: Error Handling & Standardization
*Goal: Eliminate crashes from `.unwrap()` and unify error types.*

#### 3.1.1 Refactor Agent Initialization
- **Issue**: Multiple `.unwrap()` calls in `agent/mod.rs` can crash the CLI.
- **Fix**: Replace with `?` and map to `SlapperError::AgentError`.

#### 3.1.2 Standardize Result Types
- **Issue**: Mix of `Result<T, String>` and `Result<T, SlapperError>`.
- **Fix**: Standardize on `SlapperError` for all internal logic. Use `String` only at the outermost TUI display layer.

---

### Wave 3B: Architecture Refactoring (Large Files)
*Goal: Improve maintainability by splitting files exceeding 1000 lines.*

#### 3.2.1 Split `tool/session.rs`
- **Current**: 1418 lines.
- **Target**: Split into `session/mod.rs`, `auth.rs`, `csrf.rs`, `forms.rs`, `state.rs`.

#### 3.2.2 Split `mcp/handlers/server.rs`
- **Current**: 898 lines.
- **Target**: Split into `handlers/mod.rs`, `tools.rs`, `session.rs`, `resources.rs`, `prompts.rs`.

---

## WAVE 4: Performance & Hardening

### Wave 4A: Performance Quick Wins
*Goal: Reduce allocations and lock contention.*

#### 4.1.1 Eliminate `to_string()` in Fuzzer Hot Path
- **File**: `fuzzer/engine/utils.rs:211,237`.
- **Issue**: `payload_type.to_string()` called twice per request.
- **Fix**: Use `as_str()` and avoid redundant allocations.

#### 4.1.2 Migrate to `parking_lot`
- **Fix**: Replace remaining `std::sync::Mutex` with `parking_lot::Mutex` in performance-sensitive areas (`stress/udp.rs`, `tui/state/`).

---

### Wave 4B: Map & Cache Optimization
*Goal: Optimized hashing and bounded memory usage.*

#### 4.2.1 `FxHashMap` Migration
- **Target**: Migrate hot-path HashMaps to `rustc_hash::FxHashMap`. Priority: `redos_detect.rs`, `session.rs`, `alerts/routing.rs`.

#### 4.2.2 Bounded Regex Caches (ReDoS Protection)
- **Issue**: Regex caches in `fuzzer/chain.rs` and `scanner/templates/matcher.rs` are unbounded.
- **Fix**: Replace `FxHashMap` with `LruCache` (e.g., limit to 1000 entries).

---

## WAVE 5: Feature Enhancements

### 5.1 Agent & Plugin Enhancement
- **Agent**: Implement adaptive scan strategy, false positive learning, and threat intel integration.
- **Plugin**: Expand Ruby API (Http, Scanner, Fuzzer access). Implement plugin signing and network restrictions for sandboxed plugins.

### 5.2 Fuzzer & WAF Gaps
- **Fuzzer**: Add HTTP Request Smuggling payloads (CL.TE, TE.CL). Add business logic payloads (integer overflows).
- **WAF**: Add detection profiles for OpenResty, HAProxy WAF, and Reblaze.

---

## WAVE 6: Long-term Capabilities
- **Exploitation Framework**: Native shellcode generation, session management, and pivoting.
- **Cloud/Mobile**: S3 bucket enumeration, APK analysis, Frida instrumentation.
- **Enhanced GUI**: React-based web dashboard.

---

## WAVE 7: Documentation
- **README.md**: Fix detection counts (WAF: 34, Payloads: 30). Add feature matrix.
- **CAPABILITIES.md**: Document all 29 TUI tabs and 31 recon modules.
- **ARCHITECTURE.md**: Remove fictional composite features; fix MCP references.

---

## Pre-existing Test Failures (7)
*Note: These should be fixed as a prerequisite for any changes touching these modules.*

1. `ai::client::tests::test_extract_content_valid_response`
2. `ai::planner::tests::test_parse_modifications_from_text_add_stage`
3. `ai::planner::tests::test_parse_modifications_from_text_reduce_duration`
4. `ai::planner::tests::test_parse_modifications_multiple_types`
5. `ai::planner::tests::test_planner_cache_clear`
6. `ai::planner::tests::test_record_outcome_updates_success_rate`
7. `ai::waf_bypass::tests::test_record_success_adds_to_knowledge_base`

---

## False Positives & Already Fixed (Do Not Address)
- **k8s-openapi**: Claims of compilation failure are false (already fixed via version 0.22).
- **Intercept Proxy TLS**: Claim of missing validation is a false positive (it's an intercepting proxy by design).
- **HistoryTab Search**: Claim of missing feature is false (exists in `tui/tabs/history.rs`).
- **Auto-Calibration**: Claim of missing feature is false (exists in `fuzzer/calibration.rs`).

*Last updated: 2026-04-29*
