# Slapper Implementation Plan

**Date**: 2026-05-23
**Last Updated**: 2026-05-28

## Overview

This plan consolidates action items from architecture reviews of all Slapper modules. Items are organized by priority and grouped into implementation waves for parallel execution.

---

## Wave 1: Production Safety (Can execute in parallel - 4 items)

Items that prevent potential panics, data corruption, or security issues.

### 1.1 NSE - Replace std::HashMap with FxHashMap (HIGH)

**File**: `crates/slapper-nse/src/public_api/api.rs`

**Issue**: Uses `std::collections::HashMap` at 8 locations instead of `FxHashMap`. This impacts performance in hot paths.

**Current code** (lines to change):
```rust
// Line 107-108: Return type and local variable
fn get_cve_database(
) -> std::collections::HashMap<&'static str, (&'static str, &'static str, &'static str)> {
    let mut m = std::collections::HashMap::new();

// Line 381: NseHttpResponse struct field
pub headers: std::collections::HashMap<String, String>,

// Line 413, 463, 532: Local variables in functions
let mut headers = std::collections::HashMap::new();

// Line 486: NseHttpRequest struct field
pub headers: std::collections::HashMap<String, String>,

// Line 1106: NseHttpResponse construction
headers: std::collections::HashMap::new(),
```

**Fix**: Add `use rustc_hash::FxHashMap;` at top of file and replace all `std::collections::HashMap` with `FxHashMap`.

**Verification**:
```bash
cargo check -p slapper-nse
cargo test --lib -p slapper-nse
cargo clippy --lib -p slapper-nse
```

### 1.2 Networking - DNS Parsing Bounds Check (MEDIUM)

**File**: `crates/slapper/src/packet/parse_impl.rs:531`

**Issue**: `DnsRecord::parse()` could panic on malformed DNS responses. The bounds check `new_offset + 4 > data.len()` doesn't guard against `new_offset` already exceeding `data.len()`.

**Current code** (lines 530-546):
```rust
for _ in 0..questions_count {
    if let Some((name, new_offset)) = super::validation::parse_dns_name(data, offset) {
        if new_offset + 4 > data.len() {
            break;
        }
        // ... subsequent byte access assumes new_offset is valid
```

**Fix**: Add explicit bounds check before byte access:
```rust
for _ in 0..questions_count {
    if let Some((name, new_offset)) = super::validation::parse_dns_name(data, offset) {
        if new_offset >= data.len() || new_offset + 4 > data.len() {
            break;
        }
        // ... now safe to access byte slices
```

**Note**: The same pattern should be checked for the answers parsing loop (lines 549-580).

**Verification**:
```bash
cargo check --lib -p slapper --features packet-inspection
```

### 1.3 Distributed - Worker Capabilities Mismatch (MEDIUM)

**File**: `crates/slapper/src/distributed/worker.rs:115-123`

**Issue**: Worker advertises hardcoded string capabilities ("PortScan", "ServiceFingerprint") that don't match `TaskType` enum variants (`TaskType::PortScan`, `TaskType::ServiceFingerprint`). If the enum changes, capabilities become inconsistent.

**Current code**:
```rust
vec![
    "PortScan".to_string(),
    "ServiceFingerprint".to_string(),
    "EndpointDiscovery".to_string(),
    "Fuzz".to_string(),
    "WafTest".to_string(),
    "LoadTest".to_string(),
    "Recon".to_string(),
],
```

**Fix**: Derive string capabilities from `TaskType` enum:
```rust
vec![
    TaskType::PortScan.to_string(),
    TaskType::ServiceFingerprint.to_string(),
    TaskType::EndpointDiscovery.to_string(),
    TaskType::Fuzz.to_string(),
    TaskType::WafTest.to_string(),
    TaskType::LoadTest.to_string(),
    TaskType::Recon.to_string(),
]
```

Or create a helper function:
```rust
fn worker_capabilities() -> Vec<String> {
    [
        TaskType::PortScan,
        TaskType::ServiceFingerprint,
        TaskType::EndpointDiscovery,
        TaskType::Fuzz,
        TaskType::WafTest,
        TaskType::LoadTest,
        TaskType::Recon,
    ].iter().map(|t| t.to_string()).collect()
}
```

**Verification**:
```bash
cargo check --lib -p slapper --features stress-testing
cargo test --lib -p slapper distributed::
```

### 1.4 AI - Knowledge Base Load Silent Failure (LOW)

**File**: `crates/slapper/src/ai/waf_bypass.rs:40-47`

**Issue**: When loading `waf_bypasses.json`, `unwrap_or_default()` silently suppresses deserialization errors, potentially losing learned bypasses.

**Current code**:
```rust
let knowledge_base = if persist_path.exists() {
    std::fs::read_to_string(&persist_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()  // Silently ignores errors
} else {
    Vec::new()
};
```

**Fix**: Use `unwrap_or_else()` with logging:
```rust
let knowledge_base = if persist_path.exists() {
    std::fs::read_to_string(&persist_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to load WAF bypass knowledge base: {}, starting fresh", e);
            Vec::new()
        })
} else {
    Vec::new()
};
```

**Verification**:
```bash
cargo check --lib -p slapper --features ai-integration
```

---

## Wave 2: Performance & Correctness (Can execute in parallel - 5 items)

### 2.1 NSE - Additional HashMap/HashSet Replacements (MEDIUM)

**Files & Locations**:

#### `crates/slapper-nse/src/libraries/http.rs:143`
```rust
// In parse_options() function
fn parse_options(opts: Option<&Table>) -> (HashMap<String, String>, Duration) {
    let mut headers = HashMap::new();
```
**Fix**: Change to `FxHashMap<String, String>` with `FxHashMap::default()`.

#### `crates/slapper-nse/src/libraries/datafiles.rs:31-33`
```rust
fn get_services() -> &'static HashMap<&'static str, (u16, &'static str)> {
    SERVICES.get_or_init(|| {
        let mut m = HashMap::new();
```
**Fix**: Change to `FxHashMap` and use `FxHashMap::default()`.

#### `crates/slapper-nse/src/libraries/creds.rs:102,123`
```rust
// Line 102 and 123
let mut seen = std::collections::HashSet::new();
```
**Fix**: Change to `FxHashSet` with `FxHashSet::default()`.

**Verification**:
```bash
cargo check -p slapper-nse
cargo test --lib -p slapper-nse
```

### 2.2 Distributed - CommandMessage env Field Handling (LOW)

**File**: `crates/slapper/src/distributed/command.rs:146-149`

**Issue**: `env` field is accepted in protocol but rejected at execution time. This wastes bandwidth sending env that will always be rejected.

**Current code**:
```rust
// Security: Do not allow custom environment variables
if env.is_some() {
    return Err("Custom environment variables are not allowed".to_string());
}
```

**Recommendation**: Either remove the `env` field from `CommandMessage::Execute` struct, or document clearly that it's reserved for future use and currently rejected for security reasons.

**Note**: If keeping the field, at minimum add a comment explaining this is intentional for security.

**Verification**:
```bash
cargo check --lib -p slapper --features stress-testing
```

### 2.3 Recon - Replace unwrap_or_default() (MEDIUM)

**File**: Multiple files in `crates/slapper/src/recon/`

**Issue**: 20 instances of `unwrap_or_default()` in production code silently suppress errors. These should use explicit match with tracing.

**Files affected**:
- `cve_lookup.rs:140` - `references: ...unwrap_or_default()`
- `containers.rs:124-125` - `pod_name.unwrap_or_default()`, `pod_namespace.unwrap_or_default()`
- `email.rs:145` - `context: ...unwrap_or_default()`
- `js.rs:256` - `full_match...unwrap_or_default()`
- `cors.rs:107,114,121` - Multiple header extractions
- `dependency_scan/mod.rs:160,172,187` - Multiple field extractions
- `reverse_dns.rs:40` - `hostname_str.unwrap_or_default()`
- `ssl_audit.rs:275` - `check.details.clone().unwrap_or_default()`
- `cloud/storage_test.rs:141,152` - `resp.text().await.unwrap_or_default()`
- `asn.rs:105` - `hostname.unwrap_or_default()`
- `techdetect.rs:66` - `response.text().await.unwrap_or_default()`
- `threatintel.rs:277` - `category.unwrap_or_default()`

**Fix Pattern**:
```rust
// Instead of:
let pod_name = pod.metadata.name.clone().unwrap_or_default();

// Use:
let pod_name = pod.metadata.name.clone().unwrap_or_else(|| {
    tracing::debug!("pod missing name field");
    String::new()
});
```

**Important**: Do NOT use `unwrap_or_default()` in async operations - use explicit match with tracing instead.

**Verification**:
```bash
cargo check --lib -p slapper
cargo test --lib -p slapper recon::
```

### 2.4 Fuzzer - Division by Zero Guard (LOW)

**File**: `crates/slapper/src/fuzzer/detection/analyzer.rs:188-190`

**Issue**: IQR calculation could produce empty slice even with `start >= end` check, leading to division by zero.

**Current code**:
```rust
let start = len / 4;
let end = len * 3 / 4;

if start >= end {
    return;
}

let iqr_samples: Vec<f64> = sorted_samples[start..end].to_vec();
let sum: f64 = iqr_samples.iter().sum();
self.baseline_ms = Some(sum / iqr_samples.len() as f64);  // Could divide by 0 if empty
```

**Fix**: Add defensive empty check:
```rust
let iqr_samples: Vec<f64> = sorted_samples[start..end].to_vec();
if iqr_samples.is_empty() {
    return;
}
self.baseline_ms = Some(sum / iqr_samples.len() as f64);
```

**Verification**:
```bash
cargo check --lib -p slapper
cargo test --lib -p slapper fuzzer::
```

### 2.5 Loadtest - Panic Message Imprecision (LOW)

**File**: `crates/slapper/src/loadtest/metrics.rs:76`

**Issue**: Panic message "3 significant figures is invalid" is incorrect - 3 is a valid value for `Histogram::new(3)`.

**Current code**:
```rust
histogram: Histogram::new(3).expect("3 significant figures is invalid for hdrhistogram"),
```

**Fix**: Use clearer message:
```rust
histogram: Histogram::new(3).expect("Failed to create hdrhistogram"),
```

**Verification**:
```bash
cargo check --lib -p slapper
cargo test --lib -p slapper loadtest::
```

---

## Wave 3: Documentation & Polish (Can execute in parallel - 2 items)

### 3.1 Config - AlertChannelsConfig Validation (LOW)

**File**: `crates/slapper/src/config/settings.rs`

**Enhancement**: Consider adding validation for `AlertChannelsConfig` in `SlapperConfig::validate()`. Currently only profiles are validated, not alert channels.

**Note**: This is an enhancement, not a bug fix. The validation should ensure alert channels are properly configured if present.

**Verification**:
```bash
cargo check --lib -p slapper
```

### 3.2 Architecture Documentation Updates (INFO)

The following documentation updates are needed:

| Module | File | Issue |
|--------|------|-------|
| TUI | `architecture/tui.md` | Update payload type count from 30 to 31 to match `PayloadType` enum |
| Recon | `architecture/recon.md` | Clarify `secrets` module is standalone (not in `FULL_RECON_PIPELINE_MODULES`) |
| Recon | `architecture/recon.md` | Update FxHashMap count from 13 to 55 (actual usage) |
| Output | `architecture/output.md` | Clarify error types for `CsvExporter` export functions |
| CLI | `architecture/cli_commands.md` | Add `-o` short flag to `GraphQlArgs` and `OAuthArgs` for consistency |
| NSE | `architecture/plugins_nse.md` | Update HashMap count after fixes |
| Networking | `architecture/networking.md` | Add note about DNS parsing bounds check |

**Note**: These are documentation-only changes. No code modifications required.

---

## Items With No Action Required

The following modules were reviewed and require no code changes:

| Module | Status | Notes |
|--------|--------|-------|
| WAF | ✅ Clean | No bugs found |
| TUI | ✅ Clean | No bugs found |
| Scanner | ✅ Clean | All bug fixes properly applied |
| Pipeline | ✅ Clean | All components match architecture |
| Proxy | ✅ Clean | Implementation matches architecture |
| Stress | ✅ Clean | Implementation matches architecture |
| Tool | ✅ Clean | Implementation matches architecture |

---

## Implementation Order

### Wave 1 (4 items - Can run in parallel)
| # | Module | File | Priority |
|---|--------|------|----------|
| 1.1 | NSE | `public_api/api.rs` - Replace 8 std::HashMap with FxHashMap | HIGH |
| 1.2 | Networking | `packet/parse_impl.rs:531` - DNS parsing bounds check | MEDIUM |
| 1.3 | Distributed | `worker.rs:115-123` - Worker capabilities from TaskType | MEDIUM |
| 1.4 | AI | `waf_bypass.rs:44` - unwrap_or_else with logging | LOW |

### Wave 2 (5 items - Can run in parallel)
| # | Module | File | Priority |
|---|--------|------|----------|
| 2.1 | NSE | `libraries/http.rs`, `datafiles.rs`, `creds.rs` - 4 more HashMap/HashSet fixes | MEDIUM |
| 2.2 | Distributed | `command.rs:146-149` - Document or remove env field handling | LOW |
| 2.3 | Recon | Multiple files - Replace 20 unwrap_or_default() calls | MEDIUM |
| 2.4 | Fuzzer | `analyzer.rs:188-190` - Add empty IQR check | LOW |
| 2.5 | Loadtest | `metrics.rs:76` - Fix panic message | LOW |

### Wave 3 (2 items - Can run in parallel)
| # | Module | File | Priority |
|---|--------|------|----------|
| 3.1 | Config | `settings.rs` - Add AlertChannelsConfig validation | LOW |
| 3.2 | Docs | Multiple architecture files - Update counts and clarify | INFO |

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
```

---

## Notes for Future Agents

1. **NSE module** (`slapper-nse/`) is a separate crate with its own `Cargo.toml`. Always use `cargo check -p slapper-nse` for validation.

2. **Distributed module** has 4 issues total: 1 worker capabilities, 1 env handling (low priority), 1 lock contention (documented, not fixing), 1 queue.rs (already fixed).

3. **Recon module** has the most instances of `unwrap_or_default()` - use grep to find all: `rg "unwrap_or_default\(\)" crates/slapper/src/recon/`

4. **FxHashMap imports** should use `use rustc_hash::{FxHashMap, FxHashSet}` at the top of files.

5. **Test code** can use `.unwrap()` and `.expect()` - the architecture guidelines about these apply only to production code.

6. **AI Module** (`waf_bypass.rs:44`) has knowledge base load issue - fix uses `unwrap_or_else` with `tracing::warn`.

7. **Networking DNS parsing** issue is in `packet/parse_impl.rs` not `networking/` - the packet module handles raw socket parsing.

8. **CLI `-o` flag** needs to be added to `GraphQlArgs` and `OAuthArgs` in `cli_commands.md` architecture doc (not code).