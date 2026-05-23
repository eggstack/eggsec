# Scanner Module Architecture Review

**Date**: 2026-05-23
**Reviewer**: Architecture Review
**Module**: `scanner/`
**Files Reviewed**: 11 directories, 743 lines in fingerprint.rs alone

---

## Executive Summary

The scanner module architecture document is **accurate and well-documented**. All documented bug fixes from 2026-05-22 and 2026-05-27 have been properly applied. Implementation uses FxHashMap correctly (20 matches confirmed), DashMap for concurrent collections, and proper error handling with `Arc::try_unwrap` + `map_err`. No critical bugs found.

---

## 1. Implementation vs Documentation

### ✅ Verified Claims

| Claim | Status | Evidence |
|-------|--------|----------|
| DashMap for concurrent result collection | ✅ Accurate | `ports/mod.rs:519`, `fingerprint.rs:4`, `endpoints.rs:5` |
| tokio::sync::Semaphore for concurrency control | ✅ Accurate | `ports/mod.rs:537` |
| FxHashMap for performance | ✅ Accurate | 20 matches found across templates/, cms/, ports/ |
| Arc::try_unwrap + map_err pattern | ✅ Accurate | `ports/mod.rs:595-597` |
| Feature gating (stress-testing) | ✅ Accurate | `icmp_probe.rs` behind `#[cfg(feature)]` |

### ✅ Bug Fixes Applied (2026-05-22)

| Documented Fix | File | Implementation |
|---------------|------|----------------|
| Arc::try_unwrap panic fix | `ports/mod.rs:595-598` | Uses `map_err` with SlapperError::Runtime |
| init_packet_trace file handling | `ports/spoofed.rs:75-95` | Added `include_header` parameter |
| Unused HashMap import removed | `ports/spoofed.rs:111` | Confirmed removed |
| Duplicate HttpMatcher fixed | `templates/models.rs:57,61` | Struct order corrected |
| HashMap→FxHashMap performance fix | `templates/matcher.rs:9,24` | Uses `rustc_hash::FxHashMap` |
| HashMap→FxHashMap in CMS | `cms/mod.rs:52,165,291` | Uses `rustc_hash::FxHashMap` |
| Arc::try_unwrap panic fix | `endpoints.rs:835-839` | Uses map_err pattern |
| Arc::try_unwrap panic fix | `fingerprint.rs:319-323` | Uses map_err pattern |

### ✅ Bug Fixes Applied (2026-05-27)

| Documented Fix | File | Implementation |
|---------------|------|----------------|
| String slice bounds check | `cms/joomla.rs:88-89` | Bounds check added before slicing |
| Invalid regex warning | `templates/matcher.rs:185-189` | `tracing::debug` warning added |
| unwrap→unwrap_or_else | `cms/mod.rs:330` | Uses `unwrap_or_else` |
| Explicit match error handling | `endpoints.rs:768` | `match` with debug logging |
| Explicit match task join | `udp_fingerprint.rs:144` | `match` with debug logging |

---

## 2. Key Design Patterns Verification

### ✅ DashMap Usage

```rust
// ports/mod.rs:519
let results: Arc<DashMap<u16, PortResult>> = Arc::new(DashMap::new());

// fingerprint.rs:4
use dashmap::DashMap;

// endpoints.rs:5
use dashmap::DashMap;
```

### ✅ FxHashMap Usage (20 matches)

```rust
// templates/matcher.rs:9
use rustc_hash::FxHashMap;

// templates/models.rs:52
pub headers: FxHashMap<String, String>,

// cms/mod.rs:14
use rustc_hash::FxHashMap;

// ports/mod.rs:55-79
static COMMON_PORTS_MAP: LazyLock<FxHashMap<u16, &'static str>> = LazyLock::new(|| {
    let mut m = FxHashMap::default();
    m.insert(21, "FTP");
    // ... 22 more entries
});
```

### ✅ Proper Arc::try_unwrap Pattern

```rust
// ports/mod.rs:595-597
let results_map = Arc::try_unwrap(results).map_err(|_| {
    crate::error::SlapperError::Runtime("Arc ref count non-zero after workers completed".into())
})?;
```

---

## 3. Bug Analysis

### No Critical Bugs Found

The documented bug fixes have all been properly implemented:
- No `.expect()` panics on Arc::try_unwrap
- No silent error suppressions
- Bounds checks on string slicing
- Proper error propagation

### ⚠️ Minor Observation: write! in Display Trait

```rust
// ports/mod.rs:279,294,298,302,473,474,479,482
write!(s, "Host: {}\n", results.host).unwrap();
```

The `Display` implementation for `PortScanResults` uses `.unwrap()` on `write!` calls. While technically could panic on I/O error (which would be exceptional), using `map_err` or `?` operator would be more idiomatic. **Not a critical issue** - I/O errors in string formatting are essentially impossible.

---

## 4. Performance Observations

### ✅ LazyLock Static Maps

```rust
// ports/mod.rs:55-80
static COMMON_PORTS_MAP: LazyLock<FxHashMap<u16, &'static str>> = LazyLock::new(|| {
    // ... initialization
});
```

This is correct - Common ports are computed once at startup.

### ✅ Semaphore-Controlled Concurrency

```rust
// ports/mod.rs:537
let semaphore = Arc::new(tokio::sync::Semaphore::new(config.concurrency));

// ports/mod.rs:543
let permit = semaphore.clone().acquire_owned().await?;
```

Proper use of semaphore for bounded concurrency.

---

## 5. Feature Gating

### ✅ stress-testing Feature

```rust
// icmp_probe.rs - only compiled with stress-testing feature
#[cfg(feature = "stress-testing")]
pub mod icmp_probe;

// spoofed.rs:9-14
#[cfg(all(feature = "stress-testing", unix))]
use super::get_service_name;
```

Correct implementation of feature-gated raw socket features.

---

## 6. Discrepancies

### None Found

The architecture document accurately describes the implementation:
- Bug fixes are properly documented and verified
- Design patterns are correctly implemented
- Feature gating works as documented

---

## 7. Recommendations

### Not Issues (Informational)

1. **write! unwrap in Display** - While using `.unwrap()` on `write!` is technically correct (fmt errors are unrecoverable), using `map_err` would be more idiomatic. Not a bug.

2. **COMMON_PORTS_MAP initialization** - The static LazyLock creates a 22-entry map at startup. This is negligible overhead.

---

## 8. Conclusion

The scanner module implementation is **excellent** and fully matches the architecture document. All documented bug fixes from both 2026-05-22 and 2026-05-27 have been properly applied. The module demonstrates good practices:
- Proper use of concurrent collections (DashMap)
- High-performance hash maps (FxHashMap)
- Correct error handling patterns (Arc::try_unwrap + map_err)
- Appropriate feature gating

No critical bugs or security issues found. The module is production-ready.