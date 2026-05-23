# Slapper Implementation Plan

**Date**: 2026-05-23
**Consolidated From**: 13 architecture review documents

## Overview

This plan consolidates action items from architecture reviews of all Slapper modules. Items are organized by priority and grouped into implementation waves for parallel execution.

---

## Wave 1: Production Safety (High Priority)

Items that prevent potential panics, data corruption, or security issues.

### 1.1 NSE - Replace std::HashMap with FxHashMap (HIGH)

**File**: `crates/slapper-nse/src/public_api/api.rs`

**Issue**: Uses `std::collections::HashMap` at 4 locations instead of `FxHashMap`.

**Locations**:
- Lines 107-108: `get_cve_database()` returns `std::collections::HashMap`
- Line 381: `NseHttpResponse.headers` struct field
- Line 486: `NseHttpRequest.headers` struct field
- Lines 413, 463, 532: Local variables in functions

**Fix**: Replace all `std::collections::HashMap` with `FxHashMap` imported from `rustc_hash`.

### 1.2 Networking - DNS Parsing Bounds Check (P2)

**File**: `crates/slapper/src/networking/parse_impl.rs:531`

**Issue**: `DnsRecord::parse()` could panic on malformed DNS responses. The bounds check `new_offset + 4 > data.len()` doesn't guard against `new_offset` already exceeding `data.len()`.

**Fix**: Add explicit bounds check before byte access:
```rust
if new_offset >= data.len() || new_offset + 4 > data.len() {
    break;
}
```

### 1.3 Distributed - Worker Capabilities Mismatch (MEDIUM)

**File**: `crates/slapper/src/distributed/worker.rs:115-123`

**Issue**: Worker advertises string capabilities ("PortScan", "ServiceFingerprint") that don't match `TaskType` enum variants (`TaskType::PortScan`, `TaskType::ServiceFingerprint`).

**Fix**: Derive string capabilities from `TaskType` enum to ensure consistency:
```rust
fn get_worker_capabilities() -> Vec<String> {
    vec![
        TaskType::PortScan.to_string(),
        TaskType::ServiceFingerprint.to_string(),
        // ...
    ]
}
```

---

## Wave 2: Performance & Correctness

### 2.1 NSE - Additional HashMap Replacements (MEDIUM)

**Files & Locations**:
- `libraries/http.rs:143` - `parse_options()` function
- `libraries/datafiles.rs:31-33` - `get_services()` function
- `libraries/creds.rs:102,123` - Local `seen` variables

**Fix**: Replace `std::collections::HashMap`/`HashSet` with `FxHashMap`/`FxHashSet`.

### 2.2 Distributed - CommandMessage env Field Handling (MEDIUM)

**File**: `crates/slapper/src/distributed/command.rs:146-149`

**Issue**: `env` field is accepted in protocol but rejected at execution time. This wastes bandwidth and is confusing.

**Fix**: Either remove `env` from `CommandMessage::Execute` or document that it's a security measure and will be rejected.

### 2.3 Distributed - Rate Limit Lock Contention (MEDIUM)

**File**: `crates/slapper/src/distributed/remote.rs:127-146`

**Issue**: `check_rate_limit()` holds write lock for entire operation under high load.

**Fix**: Consider using atomic operations or a more efficient rate limiting algorithm (token bucket) if lock contention becomes an issue. For now, add a comment documenting this behavior.

### 2.4 Recon - Replace unwrap_or_default() (MEDIUM)

**File**: Multiple files in `crates/slapper/src/recon/`

**Issue**: 18 instances of `unwrap_or_default()` in production code that silently suppress errors.

**Files affected**:
- `cve_lookup.rs:140`
- `containers.rs:124-125`
- `email.rs:145`
- `js.rs:256`
- `cors.rs:107,114,121`
- `dependency_scan/mod.rs:160,172,187`
- `reverse_dns.rs:40`
- `ssl_audit.rs:275`
- `cloud/storage_test.rs:141,152`
- `asn.rs:105`
- `techdetect.rs:66`
- `threatintel.rs:277`

**Fix**: Replace with explicit match and tracing:
```rust
let pod_name = pod.metadata.name.clone().unwrap_or_else(|| {
    tracing::debug!("pod missing name field");
    String::new()
});
```

### 2.5 Fuzzer - Division by Zero Guard (LOW)

**File**: `crates/slapper/src/fuzzer/detection/analyzer.rs:190`

**Issue**: IQR calculation could produce empty slice even with `start >= end` check.

**Fix**: Add defensive empty check:
```rust
let iqr_samples: Vec<f64> = sorted_samples[start..end].to_vec();
if iqr_samples.is_empty() {
    return;
}
self.baseline_ms = Some(sum / iqr_samples.len() as f64);
```

### 2.6 Loadtest - Panic Message Imprecision (LOW)

**File**: `crates/slapper/src/loadtest/metrics.rs:76`

**Issue**: Panic message "3 significant figures is invalid" is incorrect - 3 is a valid value.

**Fix**: Use clearer message:
```rust
histogram: Histogram::new(3).expect("Failed to create hdrhistogram"),
```

---

## Wave 3: Documentation & Polish

### 3.1 AI - unwrap_or_default() in waf_bypass.rs (LOW)

**File**: `crates/slapper/src/ai/waf_bypass.rs:44`

**Issue**: Knowledge base load silently suppresses deserialization errors.

**Fix**: Use `unwrap_or_else` with logging:
```rust
.unwrap_or_else(|e| {
    tracing::warn!("Failed to load WAF bypass knowledge base: {}", e);
    Vec::new()
})
```

### 3.2 Architecture Documentation Updates (INFO)

The following discrepancies were found between architecture docs and implementation:

| Module | Issue | Action |
|--------|-------|--------|
| WAF | Doc says 15 bypass techniques, implementation has 15 | No action needed |
| TUI | Doc says 30 payload types, AGENTS.md says 31 | Update AGENTS.md to reflect `PayloadType` enum count |
| Recon | Doc mentions `secrets` module but it's standalone | Clarify in architecture doc that `secrets` is not part of `FULL_RECON_PIPELINE_MODULES` |
| Recon | Doc says 13 FxHashMap locations, 55 found | Update count in architecture doc |
| Output | Error types documentation imprecise | Update `architecture/output.md` with specific error types |

### 3.3 Architecture Document Clarifications

| Module | File | Issue |
|--------|------|-------|
| NSE | `plugins_nse.md` | Update HashMap count after fixes |
| Distributed | `distributed.md` | Document worker capabilities derivation |
| Networking | `networking.md` | Add note about DNS parsing bounds check |

---

## Items With No Action Required

The following modules were reviewed and require no code changes:

| Module | Status | Notes |
|--------|--------|-------|
| WAF | ✅ Clean | No bugs found |
| TUI | ✅ Clean | No bugs found |
| Scanner | ✅ Clean | All bug fixes properly applied |
| Pipeline | ✅ Clean | All components match architecture |
| Config | ✅ Clean | FxHashMap, private IP blocking correct |
| CLI | ✅ Clean | Implementation matches architecture |
| Loadtest | ✅ Clean | Rate limiting, metrics correct |
| Fuzzer | ✅ Clean | HashMap usage correct throughout |

---

## Implementation Order

### Wave 1 (Can be done in parallel - 3 items)
- NSE: public_api/api.rs HashMap replacement
- Networking: DNS parsing bounds check
- Distributed: Worker capabilities fix

### Wave 2 (Can be done in parallel - 5 items)
- NSE: Additional HashMap replacements (http.rs, datafiles.rs, creds.rs)
- Distributed: env field handling
- Distributed: Rate limit comment
- Recon: unwrap_or_default() replacements
- Fuzzer: Division by zero guard

### Wave 3 (Can be done in parallel - 2 items)
- AI: waf_bypass.rs logging
- Architecture doc updates

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
```

---

## Notes for Future Agents

1. **NSE module** (`slapper-nse/`) is a separate crate with its own `Cargo.toml`
2. **Distributed module** has 4 issues total: 1 worker capabilities, 1 env handling, 1 lock contention, 1 (already fixed queue.rs)
3. **Recon module** has the most instances of `unwrap_or_default()` - search for the pattern across the module
4. **FxHashMap imports** should use `use rustc_hash::{FxHashMap, FxHashSet}` at the top of files
5. **Test files** can use `.unwrap()` and `.expect()` - the architecture guidelines about these apply only to production code