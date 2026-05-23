# Slapper Implementation Plan

**Date**: 2026-05-28 (Consolidated from Architecture Reviews)
**Last Updated**: 2026-05-28 (Review Session)
**Status**: REMAINING ITEMS - Verified against codebase

## Overview

This plan consolidates remaining action items from architecture reviews across all Slapper modules. Items are organized into waves based on parallelization potential - items within a wave can be implemented in parallel by different agents.

**Previous Completion**: All items from the initial review sessions (2026-05-23 to 2026-05-28) were completed. These are NEW items identified in the subsequent comprehensive review.

**Verification Status**: All items verified against codebase on 2026-05-28.

| Wave | Items | Priority | Parallelization |
|------|-------|----------|------------------|
| Wave 1 | 4 | Critical | Fully parallel (4 agents) |
| Wave 2 | 10 | High | Module-level parallel |
| Wave 3 | 13 | Medium/Low | Fine-grained parallel |

---

## Wave 1: Critical Path (Fully Parallel)

These items have no dependencies and can be implemented in parallel by separate agents.

### 1.1 Scanner: UDP Socket Reuse with Arc\<UdpSocket\>

**Status**: OUTSTANDING

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

**Status**: PARTIALLY FIXED (LazyLock done, algorithm not)

**Location**: `crates/slapper/src/waf/mod.rs:151-163`

**Issue**: The `select_profile()` function does O(p×s) nested linear scan through all profiles and signatures:
```rust
for profile in bypass::get_waf_profiles() {
    for sig in &profile.detection_signatures {
        // O(profiles * signatures) comparison
    }
}
```
Note: `get_waf_profiles()` now uses LazyLock (fixed), but the lookup algorithm still does nested linear scan.

**Fix**: Build a `FxHashMap<String, &WafProfile>` static at module load time for O(1) lookup by signature name.

**Files to modify**:
- `waf/mod.rs:151-163` - select_profile function (change algorithm)
- `waf/bypass/profiles.rs` - may need to add HashMap variant

---

### 1.3 CLI: CIDR Scope Validation Order

**Status**: OUTSTANDING

**Location**: `crates/slapper/src/config/scope.rs:209-221`

**Issue**: Private IP check occurs BEFORE scope rule evaluation. Target `10.255.255.255` with scope rule `allow 10.0.0.0/8` is rejected because it matches `is_private_ip()` first:
```rust
if let Ok(ip) = IpAddr::from_str(target) {
    if ip.is_loopback() { ... return error; }
    if is_private_ip(&ip) { ... return error; }  // <-- Before scope check
    return Ok(Self { host: target.to_string(), ip: Some(ip) });
}
```

**Fix**: Move private IP validation to after scope rule checking, allowing explicit scope overrides. Or pass scope context into `parse()` so scope rules can override private IP restrictions.

**Files to modify**:
- `config/scope.rs:209-221` - parse function private IP check
- `config/scope.rs` - scope rule matching logic

---

### 1.4 Recon: CveMapper Cache Persistence

**Status**: OUTSTANDING

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

### 2.1 Distributed Module (3 items, one was fixed)

**Location**: `crates/slapper/src/distributed/`

#### 2.1.1 Connection Cleanup on Drop

**Status**: OUTSTANDING

**File**: `remote.rs:377-389`

**Issue**: `RemoteClient` lacks `Drop` impl - connections not explicitly closed on panic.

**Fix**: Add `impl Drop for RemoteClient` or use a `Guard` wrapper pattern to ensure connections are closed on drop.

---

#### 2.1.2 Heartbeat Connection Churn

**Status**: OUTSTANDING

**File**: `worker.rs:154`

**Issue**: Every heartbeat creates a new TCP connection via `RemoteClient::new_plaintext()`:
```rust
let client = RemoteClient::new_plaintext(psk.clone());
if let Err(e) = client.send_heartbeat(...).await {
```

**Fix**: Parse URL once at startup, cache host/port, reuse `RemoteClient` instance or use a connection pool.

---

#### 2.1.3 DNS Lookup Per Call

**Status**: OUTSTANDING

**File**: `remote.rs:575-594`

**Issue**: `resolve_host()` called every `connect()` call - DNS lookup not cached.

**Fix**: Cache resolved `SocketAddr` in `RemoteClient` with TTL or connection-level caching.

---

### 2.2 Pipeline Module (2 items)

**Location**: `crates/slapper/src/pipeline/`

#### 2.2.1 Hardcoded Ports Duplicated

**Status**: OUTSTANDING

**File**: `executor.rs:276-283` and `executor.rs:534-535`

**Issue**: Port lists hardcoded in two places:
- `run_fingerprint()` lines 276-280
- `get_extended_ports()` line 534

**Fix**: Extract to `crate::constants` module or `PipelineConfig` struct with a single source of truth.

---

#### 2.2.2 Profile Mapping Duplication

**Status**: OUTSTANDING

**File**: `stage.rs:31-92` vs `tool/implementations/pipeline.rs:64-77`

**Issue**: `Stage::from_profile()` defines profile-to-stage mapping; tool implementation has separate string-to-enum match.

**Fix**: Create single `ProfileStages` mapping used by both. Extract to a shared function or constant.

---

### 2.3 Loadtest Module (2 items)

**Location**: `crates/slapper/src/loadtest/`

#### 2.3.1 Streaming Body Consumption

**Status**: OUTSTANDING

**File**: `runner.rs:336-349`

**Issue**: `response.bytes().await` called at line 342 INSIDE the `metrics.lock().await` block at line 336 - body consumed while holding metrics lock.

**Fix**: Move body consumption outside lock, then update metrics after body is fully read:
```rust
let body = response.bytes().await;  // outside lock
let _ = metrics_lock.await;  // then update
```

---

#### 2.3.2 Missing Test Coverage

**Status**: OUTSTANDING

**File**: `tests/loadtest_tests.rs`

**Issue**: Only 4 tests + 2 zero-tests. Missing coverage for: TLS connections, streaming, chunked transfers, redirect handling, metrics accuracy, rate limiting, auth, proxy, timeouts.

**Fix**: Add tests for:
- TLS connection establishment and certificate validation
- 4xx/5xx error body consumption
- Redirect following (30x responses)
- Metrics accuracy under load
- Rate limiting behavior
- Authentication (Basic, Bearer)
- Proxy support
- Connection timeout scenarios

---

### 2.4 Output Module (1 item)

**Location**: `crates/slapper/src/output/`

#### 2.4.1 LazyLock for Compliance Templates

**Status**: OUTSTANDING

**File**: `template.rs:453-545`

**Issue**: Functions like `pcidss_template()`, `soc2_template()`, `hipaa_template()` recreate template structs every call.

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

**Status**: OUTSTANDING

**File**: `scope.rs:251-259`

**Issue**: DNS failure silently returns `ip: None`; CIDR allowlist then bypasses security (no IP to match against):
```rust
tracing::debug!("DNS resolution failed for {target}: {e}");
// ...
return Ok(Self { host: target.to_string(), ip: None });
```

**Fix**: When DNS fails and `has_ip_based_rules()` is true, return error instead of silently allowing:
```rust
if has_ip_based_rules() {
    return Err(anyhow!("DNS failed for {target} with CIDR rules configured"));
}
```

---

## Wave 3: Improvement Opportunities (Fine-Grained Parallelization)

Lower priority items that can be parallelized at feature level.

### 3.1 Scanner (1 item)

#### 3.1.1 Batch UDP Scanning with Worker Pools

**Status**: OUTSTANDING

**File**: `scanner/udp_fingerprint.rs:128-139`

**Issue**: Sequential port scanning could be parallelized with `tokio::sync::mpsc` worker pools.

**Fix**: Implement batch UDP scanning using Semaphore for concurrency control. Consider combining with item 1.1 (socket reuse) for best performance.

---

### 3.2 Pipeline (1 item)

#### 3.2.1 Concurrent Stage Execution

**Status**: OUTSTANDING

**File**: `pipeline/executor.rs:182-200`

**Issue**: Sequential `for stage in &self.stages` loop - stages execute one at a time.

**Fix**: Add optional concurrent stage execution mode using `futures::future::join_all` or stage-level parallelism for independent stages (e.g., multiple reconnaissance modules can run concurrently).

---

### 3.3 Fuzzer (2 items)

#### 3.3.1 GrammarFuzzer Seed Parameter Documentation

**Status**: OUTSTANDING (Medium priority)

**File**: `fuzzer/grammar.rs:212-220`

**Issue**: `GrammarFuzzer::with_seed()` exists but is undocumented. Users cannot use deterministic fuzzing for reproducible results.

**Fix**: Document the seed parameter in the struct-level doc comment and add example usage.

---

#### 3.3.2 LazyLock for Vulnerable Patterns

**Status**: OUTSTANDING

**File**: `fuzzer/redos_detect.rs:229-247`

**Issue**: `default_vulnerable_patterns()` creates Vec on every call:
```rust
fn default_vulnerable_patterns() -> Vec<String> {
    vec![
        r"(.+)+".to_string(),
        r"(.*)*".to_string(),
        // ... 13 more patterns created each call
    ]
}
```

**Fix**: Use `LazyLock` static:
```rust
static KNOWN_VULNERABLE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(.+)+").unwrap(),
        // ...
    ]
});
```

---

### 3.4 Networking (2 items)

**Location**: `stress/` and `packet/` modules

#### 3.4.1 IPv6 Raw Socket Support

**Status**: OUTSTANDING

**File**: `stress/udp.rs:157-163`

**Issue**: IPv6 returns error "IPv6 not supported for spoofed UDP":
```rust
if is_ipv6 {
    return Err(anyhow!("IPv6 not supported for spoofed UDP"));
}
```

**Fix**: Implement IPv6 raw socket support or document limitation clearly with user-facing error message.

---

#### 3.4.2 PacketBuilder::validate()

**Status**: OUTSTANDING

**File**: `packet/craft.rs:76-203`

**Issue**: `PacketBuilder` has no `validate()` method to check packet sanity before build.

**Fix**: Add validation method for:
- IP header length constraints
- Protocol-specific field validation
- Payload size limits
- Checksum pre-computation validation

---

### 3.5 WAF (2 items)

**Location**: `crates/slapper/src/waf/`

#### 3.5.1 Redundant Payload Generation

**Status**: OUTSTANDING

**File**: `waf/bypass/evasion.rs:101-157`

**Issue**: `get_sqli_payloads()` called 7 times in loops at lines 102, 110, 118, 126, 134, 142, 150 - payloads regenerated each iteration.

**Fix**: Call once before loops, store in local variable:
```rust
let sqli_payloads = get_sqli_payloads();
// then use sqli_payloads in all 7 loops
```

---

#### 3.5.2 Missing error Field in BypassResult

**Status**: OUTSTANDING

**File**: `waf/bypass/mod.rs:62-70`

**Issue**: `BypassResult` struct has no `error: Option<String>` field for network error details:
```rust
pub struct BypassResult {
    pub technique: String,
    pub success: bool,
    pub description: String,
    pub payload: Option<String>,
    pub status_code: Option<u16>,
    pub response_diff: Option<f64>,
    // missing: error: Option<String>
}
```

**Fix**: Add `error: Option<String>` field and populate on network failures.

---

### 3.6 TUI (1 item)

**Location**: `crates/slapper/src/tui/`

#### 3.6.1 Remove or Wire App.tabs Dead Code

**Status**: PARTIALLY FIXED (unused field remains)

**File**: `app/mod.rs:52,183`

**Issue**: `tabs: FxHashMap<Tab, Box<dyn TabInput>>` field initialized but never populated or read (grep found zero matches for `self.tabs` usage).

**Fix**: Either remove the dead code field, or implement the tab registry functionality if that was the intent.

---

### 3.7 Recon (3 items)

**Location**: `crates/slapper/src/recon/`

#### 3.7.1 Add secrets to FULL_RECON_PIPELINE_MODULES

**Status**: OUTSTANDING

**File**: `mod.rs:346-363`

**Issue**: `secrets` module not in `FULL_RECON_PIPELINE_MODULES`. The `secrets` module is standalone but could enhance full pipeline reconnaissance.

**Fix**: Consider adding `"secrets"` to `FULL_RECON_PIPELINE_MODULES` array to enable secret detection in full pipeline runs.

---

#### 3.7.2 Cloud extract_target_from_url Warning

**Status**: OUTSTANDING

**File**: `cloud/mod.rs:55`

**Issue**: Silently falls back to input on URL extraction failure:
```rust
.unwrap_or_else(|| domain.to_string())
```

**Fix**: Add `tracing::warn` when fallback occurs to help users debug scope issues.

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

**Wave 1** (4 items): Each item is in a different module, fully parallelizable. Assign to 4 agents:

| Agent | Item | Module |
|-------|------|--------|
| 1 | 1.1 Scanner UDP Socket | scanner/ |
| 2 | 1.2 WAF HashMap Lookup | waf/ |
| 3 | 1.3 CLI CIDR Validation | config/ |
| 4 | 1.4 CveMapper Cache | recon/ |

**Wave 2** (10 items): Group by module:
- Distributed (3 items) → 1 agent
- Pipeline (2 items) → 1 agent
- Loadtest (2 items) → 1 agent
- Output (1 item) → 1 agent
- Config (1 item) → 1 agent

**Wave 3** (13 items): Can parallelize within modules:
- Scanner (1 item)
- Pipeline (1 item)
- Fuzzer (2 items) → 1 agent
- Networking (2 items) → 1 agent
- WAF (2 items) → 1 agent
- TUI (1 item)
- Recon (2 items, excluding 1.4 which is Wave 1)

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