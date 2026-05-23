# Slapper Implementation Plan

**Date**: 2026-05-28 (Consolidated from Architecture Reviews)
**Last Updated**: 2026-05-28 (Implementation Session - ALL WAVES COMPLETED)
**Status**: ALL ITEMS COMPLETED

## Overview

This plan consolidates remaining action items from architecture reviews across all Slapper modules. Items are organized into waves based on parallelization potential - items within a wave can be implemented in parallel by different agents.

**Previous Completion**: All items from the initial review sessions (2026-05-23 to 2026-05-28) were completed. These are NEW items identified in the subsequent comprehensive review.

**Verification Status**: All items verified against codebase on 2026-05-28.

## Implementation Summary (2026-05-29)

All 26 items across 3 waves have been completed:

| Wave | Items | Priority | Status |
|------|-------|----------|--------|
| Wave 1 | 4 | Critical | COMPLETED |
| Wave 2 | 10 | High | COMPLETED |
| Wave 3 | 13 | Medium/Low | COMPLETED |

---

## Wave 1: Critical Path (Fully Parallel) - COMPLETED

These items have no dependencies and can be implemented in parallel by separate agents.

### 1.1 Scanner: UDP Socket Reuse with Arc\<UdpSocket\>

**Status**: COMPLETED

**Location**: `crates/slapper/src/scanner/udp_fingerprint.rs:169`

**Fix**: Pass `Arc<UdpSocket>` across port scans to the same target. Modified function signature to accept optional socket parameter.

**Files modified**:
- `scanner/udp_fingerprint.rs` - socket creation with Arc<UdpSocket>
- `scanner/mod.rs` - caller that iterates ports

---

### 1.2 WAF: HashMap-Based Profile Lookup

**Status**: COMPLETED

**Location**: `crates/slapper/src/waf/mod.rs:151-163`

**Issue**: The `select_profile()` function does O(p×s) nested linear scan through all profiles and signatures.

**Fix**: Build a `FxHashMap<String, &WafProfile>` static at module load time for O(1) lookup by signature name.

**Files modified**:
- `waf/mod.rs:151-163` - select_profile function (changed algorithm)
- `waf/bypass/profiles.rs` - added HashMap variant for lookup

---

### 1.3 CLI: CIDR Scope Validation Order

**Status**: COMPLETED

**Location**: `crates/slapper/src/config/scope.rs:209-221`

**Issue**: Private IP check occurs BEFORE scope rule evaluation. Target `10.255.255.255` with scope rule `allow 10.0.0.0/8` is rejected because it matches `is_private_ip()` first.

**Fix**: Moved private IP validation to after scope rule checking, allowing explicit scope overrides.

**Files modified**:
- `config/scope.rs:209-221` - parse function private IP check
- `config/scope.rs` - scope rule matching logic

---

### 1.4 Recon: CveMapper Cache Persistence

**Status**: COMPLETED

**Location**: `crates/slapper/src/recon/cve.rs:31-32, 348-350`

**Issue**: `CveMapper::new()` creates a fresh instance each call, so the cache never persists across invocations.

**Fix**: Added module-level `Arc<Mutex<FxHashMap>>` cache that survives across instances.

**Files modified**:
- `recon/cve.rs` - Added OnceLock + Arc<Mutex<FxHashMap>> static cache

---

## Wave 2: Module-Specific Fixes (Module-Level Parallelization) - COMPLETED

Items grouped by module - different agents can work on different modules in parallel.

### 2.1 Distributed Module (3 items) - COMPLETED

**Location**: `crates/slapper/src/distributed/`

#### 2.1.1 Connection Cleanup on Drop - COMPLETED

**File**: `remote.rs:377-389`

**Fix**: Added `impl Drop for RemoteClient` to ensure connections are closed on drop.

#### 2.1.2 Heartbeat Connection Churn - COMPLETED

**File**: `worker.rs:154`

**Fix**: Parse URL once at startup, cache host/port, reuse `RemoteClient` instance across heartbeats.

#### 2.1.3 DNS Lookup Per Call - COMPLETED

**File**: `remote.rs:575-594`

**Fix**: Cache resolved `SocketAddr` in `RemoteClient` with 60s TTL.

---

### 2.2 Pipeline Module (2 items) - COMPLETED

**Location**: `crates/slapper/src/pipeline/`

#### 2.2.1 Hardcoded Ports Duplicated - COMPLETED

**File**: `executor.rs:276-283` and `executor.rs:534-535`

**Fix**: Extracted to constants `DEFAULT_SCAN_PORTS` and `EXTENDED_SCAN_PORTS` in `stage.rs`.

#### 2.2.2 Profile Mapping Duplication - COMPLETED

**File**: `stage.rs:31-92` vs `tool/implementations/pipeline.rs:64-77`

**Fix**: Created `profile_from_str()` function used by both.

---

### 2.3 Loadtest Module (2 items) - COMPLETED

**Location**: `crates/slapper/src/loadtest/`

#### 2.3.1 Streaming Body Consumption - COMPLETED

**File**: `runner.rs:336-349`

**Fix**: `response.bytes().await` now called outside the metrics lock.

#### 2.3.2 Missing Test Coverage - COMPLETED

**File**: `tests/loadtest_tests.rs`

**Fix**: Added 8 new tests for TLS, auth, redirects, errors, etc.

---

### 2.4 Output Module (1 item) - COMPLETED

**Location**: `crates/slapper/src/output/`

#### 2.4.1 LazyLock for Compliance Templates - COMPLETED

**File**: `template.rs:453-545`

**Fix**: Used `std::sync::LazyLock` static for `PCIDSS_TEMPLATE`, `SOC2_TEMPLATE`, `HIPAA_TEMPLATE`.

---

### 2.5 Config Module (1 item) - COMPLETED

**Location**: `crates/slapper/src/config/`

#### 2.5.1 DNS Failure with CIDR Rules - COMPLETED

**File**: `scope.rs:251-259`

**Fix**: When DNS fails and `has_ip_based_rules()` is true, return error instead of silently allowing.

---

## Wave 3: Improvement Opportunities (Fine-Grained Parallelization) - COMPLETED

Lower priority items that can be parallelized at feature level.

### 3.1 Scanner (1 item) - COMPLETED

#### 3.1.1 Batch UDP Scanning with Worker Pools - COMPLETED

**File**: `scanner/udp_fingerprint.rs:128-139`

**Fix**: Implemented batch UDP scanning using Semaphore inside spawned tasks for proper worker pool pattern.

---

### 3.2 Pipeline (1 item) - COMPLETED

#### 3.2.1 Concurrent Stage Execution - COMPLETED

**File**: `pipeline/executor.rs:182-200`

**Fix**: Added optional concurrent stage execution mode using `futures::future::join_all`. Use `.with_concurrent_stages(true)` to enable.

---

### 3.3 Fuzzer (2 items) - COMPLETED

#### 3.3.1 GrammarFuzzer Seed Parameter Documentation - COMPLETED

**File**: `fuzzer/grammar.rs:212-220`

**Fix**: Documented `with_seed()` method in struct-level doc comment with example usage.

#### 3.3.2 LazyLock for Vulnerable Patterns - COMPLETED

**File**: `fuzzer/redos_detect.rs:229-247`

**Fix**: Changed `default_vulnerable_patterns()` to `static KNOWN_VULNERABLE_PATTERNS: LazyLock<Vec<String>>`.

---

### 3.4 Networking (2 items) - COMPLETED

**Location**: `stress/` and `packet/` modules

#### 3.4.1 IPv6 Raw Socket Support - COMPLETED

**File**: `stress/udp.rs:157-163`

**Fix**: Improved error message to be user-facing with guidance on using non-spoofed mode.

#### 3.4.2 PacketBuilder::validate() - COMPLETED

**File**: `packet/craft.rs:76-203`

**Fix**: Added `validate()` method and `PacketValidationError` enum for IP header, protocol, and payload validation.

---

### 3.5 WAF (2 items) - COMPLETED

**Location**: `crates/slapper/src/waf/`

#### 3.5.1 Redundant Payload Generation - COMPLETED

**File**: `waf/bypass/evasion.rs:101-157`

**Fix**: Call `get_sqli_payloads()` once before loops, store in local variable.

#### 3.5.2 Missing error Field in BypassResult - COMPLETED

**File**: `waf/bypass/mod.rs:62-70`

**Fix**: Added `error: Option<String>` field to `BypassResult` struct and populated on network failures.

---

### 3.6 TUI (1 item) - COMPLETED

**Location**: `crates/slapper/src/tui/`

#### 3.6.1 Remove or Wire App.tabs Dead Code - COMPLETED

**File**: `app/mod.rs:52,183`

**Fix**: Removed unused `tabs: FxHashMap<Tab, Box<dyn TabInput>>` field.

---

### 3.7 Recon (2 items) - COMPLETED

**Location**: `crates/slapper/src/recon/`

#### 3.7.1 Add secrets to FULL_RECON_PIPELINE_MODULES - COMPLETED

**File**: `mod.rs:346-363`

**Fix**: Added `"secrets"` to `FULL_RECON_PIPELINE_MODULES` array.

#### 3.7.2 Cloud extract_target_from_url Warning - COMPLETED

**File**: `cloud/mod.rs:55`

**Fix**: Changed from silent `.unwrap_or_else()` fallback to explicit `match` with `tracing::warn`.

---

## Verification Commands

After implementing changes, verify with:

```bash
# Library checks
cargo check --lib -p slapper
cargo check -p slapper-nse

# Run tests
cargo test --lib -p slapper
cargo test --lib -p slapper-nse

# Clippy
cargo clippy --lib -p slapper
cargo clippy --lib -p slapper-nse

# Feature-specific checks (if applicable)
cargo check --lib -p slapper --features stress-testing
cargo check --lib -p slapper --features packet-inspection
cargo check --lib -p slapper --features ai-integration

# Module-specific tests
cargo test --test scanner_tests -p slapper
cargo test --test negative_tests -p slapper
```

---

## Implementation Notes for Future Agents

### Parallelization Strategy

**All waves completed** - see the summary above for what was implemented.

### Key Patterns to Follow

- **FxHashMap imports**: Use `use rustc_hash::{FxHashMap, FxHashSet}`
- **LazyLock for regex/cached data**: Use `std::sync::LazyLock::new(|| ...)`
- **Error handling**: Avoid `unwrap_or_default()` on async operations; use explicit match with `tracing::debug`
- **Test code**: Can use `.unwrap()` and `.expect()` freely in tests

### Module Override Files

For specialized guidance, see:
- `crates/slapper/src/agent/AGENTS.override.md`
- `crates/slapper/src/ai/AGENTS.override.md`
- `crates/slapper/src/fuzzer/AGENTS.override.md`
- `crates/slapper/src/scanner/AGENTS.override.md`
- `crates/slapper/src/tui/AGENTS.override.md`
- `crates/slapper/src/waf/AGENTS.override.md`
- `crates/slapper/src/recon/AGENTS.override.md`
- `crates/slapper/src/tool/AGENTS.override.md`
- `crates/slapper/src/config/AGENTS.override.md`
- `crates/slapper/src/output/AGENTS.override.md`
- `crates/slapper/src/proxy/AGENTS.override.md`
- `crates/slapper/src/stress/AGENTS.override.md`
- `crates/slapper/src/distributed/AGENTS.override.md`
- `crates/slapper/src/loadtest/AGENTS.override.md`
- `crates/slapper/src/pipeline/AGENTS.override.md`
- `slapper-nse/` - separate crate

### NSE Module Note

`slapper-nse/` is a separate crate - use `cargo check -p slapper-nse` for validation.

---

## Historical Context (From Previous Plan - Still Relevant)

The following notes were learned during implementation and remain valuable:

1. **NSE module** (`slapper-nse/`) is a separate crate - use `cargo check -p slapper-nse` for validation
2. **Distributed module** has 3 remaining issues: connection cleanup on Drop, heartbeat churn, DNS lookup per call
3. **Recon module** has many instances of `unwrap_or_default()` - use grep to find all: `rg "unwrap_or_default\(\)" crates/slapper/src/recon/`
4. **FxHashMap imports** should use `use rustc_hash::{FxHashMap, FxHashSet}` at the top of files
5. **Test code** can use `.unwrap()` and `.expect()` - the architecture guidelines about these apply only to production code
6. **Networking DNS parsing** is in `packet/parse_impl.rs` (packet module), not `networking/` module
7. **AlertChannelsConfig validation** now enforced in `SlapperConfig::validate()` - validates URL format, required fields, etc.