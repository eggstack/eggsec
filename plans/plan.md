# Slapper Implementation Plan

**Date**: 2026-05-28 (Consolidated from Architecture Reviews)
**Last Updated**: 2026-05-28
**Status**: REMAINING ITEMS - Not Yet Implemented

## Overview

This plan consolidates remaining action items from architecture reviews across all Slapper modules. Items are organized into waves based on parallelization potential - items within a wave can be implemented in parallel by different agents.

**Previous Completion**: All 11 items from the initial plan (waves 1-3, 2026-05-28) were completed. These are NEW items identified in the subsequent comprehensive review.

**Total Items**: 27 remaining items across 3 waves

| Wave | Items | Priority | Parallelization |
|------|-------|----------|------------------|
| Wave 1 | 4 | Critical | Fully parallel |
| Wave 2 | 10 | High | Module-level parallel |
| Wave 3 | 13 | Medium/Low | Fine-grained parallel |

---

## Wave 1: Critical Path (Fully Parallel)

These items have no dependencies and can be implemented in parallel by separate agents.

### 1.1 Scanner: UDP Socket Reuse with Arc\<UdpSocket\>

**Location**: `crates/slapper/src/scanner/udp_fingerprint.rs:169`

**Issue**: Each call to `fingerprint_udp_port()` creates a new UDP socket:
```rust
let socket = match UdpSocket::bind("0.0.0.0:0").await {
    Ok(s) => s,
    Err(_) => return None,
};
```
This is inefficient when scanning multiple ports on the same host.

**Fix**: Pass `Arc<UdpSocket>` across port scans to the same target. Modify function signature to accept optional socket parameter.

**Files to modify**:
- `scanner/udp_fingerprint.rs:169` - socket creation
- `scanner/mod.rs` - caller that iterates ports

---

### 1.2 WAF: HashMap-Based Profile Lookup

**Location**: `crates/slapper/src/waf/mod.rs:151-163`

**Issue**: The `select_profile()` function does O(p×s) nested linear scan through all profiles and signatures:
```rust
for profile in bypass::get_waf_profiles() {
    for sig in &profile.detection_signatures {
        // O(profiles * signatures) comparison
    }
}
```

**Fix**: Build a `FxHashMap<String, &WafProfile>` static at module load time for O(1) lookup by signature name.

**Files to modify**:
- `waf/mod.rs:151-163` - select_profile function
- `waf/bypass/profiles.rs` - profile storage structure

---

### 1.3 CLI: CIDR Scope Validation Order

**Location**: `crates/slapper/src/config/scope.rs:209-221`

**Issue**: Private IP check occurs BEFORE scope rule evaluation. Target `10.255.255.255` with scope rule `allow 10.0.0.0/8` is rejected because it matches `is_private_ip()` first.

**Fix**: Move private IP validation to after scope rule checking, allowing explicit scope overrides. Or pass scope context into `parse()`.

**Files to modify**:
- `config/scope.rs:209-221` - parse function private IP check
- `config/scope.rs` - scope rule matching logic

---

### 1.4 Recon: CveMapper Cache Persistence

**Location**: `crates/slapper/src/recon/cve.rs:31-32, 348-350`

**Issue**: `CveMapper::new()` creates a fresh instance each call, so the cache `FxHashMap<String, Vec<VulnerabilityInfo>>` never persists across invocations:
```rust
pub async fn map_cves(tech_stack: &TechStack, nvd_api_key: Option<String>) -> Result<CveMapping> {
    let mut mapper = CveMapper::new(nvd_api_key)?;  // Fresh instance, no cache
    mapper.map_cves(tech_stack).await
}
```

**Fix**: Add `persist()` / `load()` methods or use a module-level `Arc<Mutex<FxHashMap>>` cache that survives across instances.

**Files to modify**:
- `recon/cve.rs:31` - CveMapper struct definition
- `recon/cve.rs:348-350` - public API entry point

---

## Wave 2: Module-Specific Fixes (Module-Level Parallelization)

Items grouped by module - different agents can work on different modules in parallel.

### 2.1 Distributed Module (4 items)

**Location**: `crates/slapper/src/distributed/`

#### 2.1.1 Rate Limit Race Condition
**File**: `remote.rs:127-148`
**Issue**: `check_rate_limit()` holds lock across await points:
```rust
let timestamps = limits.entry(ip.to_string()).or_insert_with(Vec::new);
timestamps.retain(...);  // await point inside lock scope
```
**Fix**: Restructure to minimize lock duration. Use atomic counter or separate cleanup phase.

#### 2.1.2 Connection Cleanup on Drop
**File**: `remote.rs:377-389`
**Issue**: `RemoteClient` lacks `Drop` impl - connections not explicitly closed on panic.
**Fix**: Add `impl Drop for RemoteClient` or use a `Guard` wrapper pattern.

#### 2.1.3 Heartbeat Connection Churn
**File**: `worker.rs:132-161`
**Issue**: Every heartbeat creates a new TCP connection via `RemoteClient::new_plaintext()`.
**Fix**: Parse URL once at startup, cache host/port, reuse client instance.

#### 2.1.4 DNS Lookup Per Call
**File**: `remote.rs:575-594`
**Issue**: `resolve_host()` called every `connect()` call.
**Fix**: Cache resolved `SocketAddr` in `RemoteClient`.

---

### 2.2 Pipeline Module (2 items)

**Location**: `crates/slapper/src/pipeline/`

#### 2.2.1 Hardcoded Ports Duplicated
**File**: `executor.rs:276-283` and `executor.rs:534-535`
**Issue**: Port lists duplicated in `get_extended_ports()` and inline.
**Fix**: Extract to `crate::constants` module or `PipelineConfig` struct.

#### 2.2.2 Profile Mapping Duplication
**File**: `stage.rs:31-92` vs `tool/implementations/pipeline.rs:64-77`
**Issue**: `Stage::from_profile()` maps enum; tool impl maps string→enum separately.
**Fix**: Create single `ProfileStages` mapping used by both.

---

### 2.3 Loadtest Module (2 items)

**Location**: `crates/slapper/src/loadtest/`

#### 2.3.1 Streaming Body Consumption
**File**: `runner.rs:336-349`
**Issue**: `response.bytes().await` called inside lock - body consumed while holding metrics lock.
**Fix**: Move body consumption outside lock, then update metrics.

#### 2.3.2 Missing Test Coverage
**File**: `tests/loadtest_tests.rs`
**Issue**: Only 4 tests + 2 zero-tests. No TLS, streaming, chunked, redirect tests.
**Fix**: Add tests for: TLS connections, 4xx/5xx body consumption, redirect handling, metrics accuracy, rate limiting, auth, proxy, timeouts.

---

### 2.4 Output Module (1 item)

**Location**: `crates/slapper/src/output/`

#### 2.4.1 LazyLock for Compliance Templates
**File**: `template.rs:453-545`
**Issue**: Functions like `pcidss_template()`, `soc2_template()` recreate template structs every call.
**Fix**: Use `std::sync::LazyLock` static:

```rust
static PCIDSS_TEMPLATE: LazyLock<ComplianceTemplate> = LazyLock::new(|| {
    ComplianceTemplate { ... }
});
```

---

### 2.5 Config Module (1 item)

**Location**: `crates/slapper/src/config/`

#### 2.5.1 DNS Failure with CIDR Rules
**File**: `scope.rs:251-259`
**Issue**: DNS failure silently returns `ip: None`; CIDR allowlist then bypasses security (no IP to match against).
**Fix**: When DNS fails and `has_ip_based_rules()` is true, return error instead of silently allowing.

---

## Wave 3: Improvement Opportunities (Fine-Grained Parallelization)

Lower priority items that can be parallelized at feature level.

### 3.1 Scanner (1 item)

#### 3.1.1 Batch UDP Scanning with Worker Pools
**File**: `scanner/udp_fingerprint.rs:128-139`
**Issue**: Sequential port scanning could be parallelized with `tokio::sync::mpsc` worker pools.
**Fix**: Implement batch UDP scanning using Semaphore for concurrency control.

---

### 3.2 Pipeline (1 item)

#### 3.2.1 Concurrent Stage Execution
**File**: `pipeline/executor.rs:182-200`
**Issue**: Sequential `for stage in &self.stages` loop.
**Fix**: Add optional concurrent stage execution mode using `futures::future::join_all` or stage-level parallelism for independent stages.

---

### 3.3 Fuzzer (3 items)

#### 3.3.1 Arc::try_unwrap Error Handling
**File**: `fuzzer/engine/execution.rs:207-216`
**Issue**: `Arc::try_unwrap(results)` could use `map_err` with descriptive message instead of unwrap.
**Fix**: Already partially addressed - ensure consistent pattern.

#### 3.3.2 GrammarFuzzer Seed Parameter
**File**: `fuzzer/grammar.rs:212-220`
**Issue**: `GrammarFuzzer::with_seed()` exists but documentation doesn't mention it.
**Fix**: Ensure seed parameter is documented for reproducible fuzzing.

#### 3.3.3 LazyLock for Vulnerable Patterns
**File**: `fuzzer/redos_detect.rs:229-247`
**Issue**: `default_vulnerable_patterns()` creates Vec on every call.
**Fix**: Use `LazyLock` static:

```rust
static KNOWN_VULNERABLE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    // ... compile patterns once
});
```

---

### 3.4 Networking (2 items)

#### 3.4.1 IPv6 Raw Socket Support
**File**: `stress/udp.rs:157-163`
**Issue**: IPv6 returns error "IPv6 not supported for spoofed UDP".
**Fix**: Implement IPv6 raw socket support or document limitation clearly.

#### 3.4.2 PacketBuilder::validate()
**File**: `packet/craft.rs:76-203`
**Issue**: `PacketBuilder` has no `validate()` method to check packet sanity before build.
**Fix**: Add validation method for length checks, protocol constraints.

---

### 3.5 WAF (2 items)

#### 3.5.1 Redundant Payload Generation
**File**: `waf/bypass/evasion.rs:101-157`
**Issue**: `get_sqli_payloads()` called 7 times in loops - payloads regenerated each iteration.
**Fix**: Call once before loops, store in local variable.

#### 3.5.2 Missing error Field in BypassResult
**File**: `waf/bypass/mod.rs:62-70`
**Issue**: `BypassResult` lacks `error: Option<String>` field for network error details.
**Fix**: Add field and populate on network failures.

---

### 3.6 TUI (1 item)

#### 3.6.1 Remove or Wire App.tabs Dead Code
**File**: `tui/app/mod.rs:52,183`
**Issue**: `tabs: FxHashMap` field exists but is never populated/used.
**Fix**: Either remove dead code or implement the tab registry functionality.

---

### 3.7 Recon (3 items)

#### 3.7.1 Add secrets to FULL_RECON_PIPELINE_MODULES
**File**: `recon/mod.rs:346-363`
**Issue**: `secrets` module not in `FULL_RECON_PIPELINE_MODULES`.
**Fix**: Consider adding `secrets` to enable secret detection in full pipeline.

#### 3.7.2 CveMapper Cache Persistence
**File**: `recon/cve.rs:31-32` (also in Wave 1)
**Fix**: See Wave 1 item 1.4

#### 3.7.3 Cloud extract_target_from_url Warning
**File**: `recon/cloud/mod.rs:55`
**Issue**: Silently falls back to input on URL extraction failure.
**Fix**: Add `tracing::warn` when fallback occurs.

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

**Wave 1** (4 items): Each item is in a different module, fully parallelizable. Assign to 4 agents.

**Wave 2** (10 items): Group by module:
- 4 distributed items → 1 agent
- 2 pipeline items → 1 agent
- 2 loadtest items → 1 agent
- 1 output item → 1 agent
- 1 config item → 1 agent

**Wave 3** (13 items): Can parallelize within modules for larger features.

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
2. **Distributed module** has 4 issues total: worker capabilities (fixed), env handling (fixed), lock contention (documented here), queue.rs (fixed)
3. **Recon module** has many instances of `unwrap_or_default()` - use grep to find all: `rg "unwrap_or_default\(\)" crates/slapper/src/recon/`
4. **FxHashMap imports** should use `use rustc_hash::{FxHashMap, FxHashSet}` at the top of files
5. **Test code** can use `.unwrap()` and `.expect()` - the architecture guidelines about these apply only to production code
6. **Networking DNS parsing** is in `packet/parse_impl.rs` (packet module), not `networking/` module
7. **AlertChannelsConfig validation** now enforced in `SlapperConfig::validate()` - validates URL format, required fields, etc.

(End of file - total 395 lines)