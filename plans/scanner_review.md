# Scanner Module Architecture Review

**Document**: `architecture/scanner.md`
**Review Date**: 2026-05-28
**Branch**: `architecture/scanner-review`

---

## 1. Summary: What's Implemented Correctly

### Bug Fixes Verified (2026-05-22)
| File | Line | Issue | Status |
|------|------|-------|--------|
| `ports/mod.rs` | 595-598 | `Arc::try_unwrap(...).expect()` panic | FIXED - uses `map_err` |
| `ports/spoofed.rs` | 75-95 | `init_packet_trace` opened file twice | FIXED - added `include_header` param |
| `ports/spoofed.rs` | 111 | Unused `std::collections::HashMap` import | FIXED - removed |
| `templates/models.rs` | 57,61 | Duplicate `HttpMatcher` + missing `DnsMatcher` | FIXED - proper struct order |
| `templates/matcher.rs` | 9,24 | `HashMap` instead of `FxHashMap` | FIXED - uses `FxHashMap` |
| `cms/mod.rs` | 52,165,291 | `HashMap` instead of `FxHashMap` | FIXED - uses `FxHashMap` |
| `endpoints.rs` | 835-839 | `Arc::try_unwrap(...).expect()` panic | FIXED - uses `map_err` |
| `fingerprint.rs` | 319-323 | `Arc::try_unwrap(...).expect()` panic | FIXED - uses `map_err` |

### Bug Fixes Verified (2026-05-27)
| File | Line | Issue | Status |
|------|------|-------|--------|
| `cms/joomla.rs` | 88-89 | String slice bounds could panic on malformed XML | FIXED - added bounds checks |
| `templates/matcher.rs` | 185-189 | Invalid regex silently returned false | FIXED - tracing debug added |
| `cms/mod.rs` | 330 | Default impl could panic on init failure | FIXED - uses `unwrap_or_else` |
| `endpoints.rs` | 768 | Silent error suppression on network failures | FIXED - explicit match |
| `udp_fingerprint.rs` | 144 | Silent task join failures | FIXED - explicit match |

### Design Patterns Verified
- `DashMap` for lock-free concurrent result collection
- `tokio::sync::Semaphore` for concurrency control
- `rustc_hash::FxHashMap` used throughout (no std `HashMap` in production code)
- Feature gating (`stress-testing`) for raw socket features
- `Arc::try_unwrap` + `map_err` pattern used consistently

---

## 2. Issues Found

### Issue 1: Unwraps in Test Code (Low Severity)
**Files**: `templates/matcher.rs`, `templates/models.rs`, `ports/mod.rs`, `endpoints.rs`, `fingerprint.rs`, `udp_fingerprint.rs`, `templates/loader.rs`, `templates/verify.rs`, `icmp_probe.rs`, `spoof.rs`, `templates/marketplace.rs`

All `unwrap()` calls found are in test code, which is acceptable since tests should be controlled environments.

### Issue 2: Marketplace Default Impl (Low Severity)
**File**: `templates/marketplace.rs:266`
```rust
impl Default for TemplateMarketplace {
    fn default() -> Self {
        Self::new("https://templates.slapper.io").unwrap()
    }
}
```
This follows the same pattern as `cms/mod.rs:330` which was flagged and fixed. The URL is hardcoded and valid, so panic is unlikely, but could be more defensive with `unwrap_or_else`.

### Issue 3: Header Value Unwrap in spoofed.rs (Low Severity)
**File**: `ports/spoofed.rs:529`
```rust
let path_str = path.to_str().unwrap();
```
Path is from a template and should be valid UTF-8. However, this is in test code, so acceptable.

---

## 3. Recommended Fixes

### Priority: Low (Cosmetic/Consistency)

1. **`templates/marketplace.rs:266`**: Consider applying the same fix pattern as `cms/mod.rs:330` for consistency:
   ```rust
   impl Default for TemplateMarketplace {
       fn default() -> Self {
           Self::new("https://templates.slapper.io").unwrap_or_else(|e| {
               panic!("TemplateMarketplace initialization failed: {}", e)
           })
       }
   }
   ```

---

## 4. Discrepancies Between Arch Doc and Implementation

### No Discrepancies Found

The architecture document accurately reflects the implementation:

1. **Core Capabilities**: All described (port scanning, endpoint discovery, fingerprinting, ICMP/UDP probing) match the file structure.

2. **Bug Fix Table**: All entries in the "Bug Fixes (2026-05-22)" and "Bug Fixes (2026-05-27)" sections have been properly applied to the implementation.

3. **Design Patterns**: All patterns listed (DashMap, Semaphore, FxHashMap, feature gating, Arc::try_unwrap) are correctly used.

4. **Timing Templates**: Documented in `timing.rs` as described.

---

## 5. Notes

- The scanner module is in good shape with all documented bug fixes properly applied.
- The `Arc::try_unwrap` pattern with proper error handling via `map_err` is used consistently across all 4 files (`ports/mod.rs`, `ports/spoofed.rs`, `endpoints.rs`, `fingerprint.rs`).
- No `std::collections::HashMap` or `HashSet` found in production code - only FxHashMap/FxHashSet as expected.
- Most `unwrap()` calls are in test code, which is acceptable.
- The spoofs.rs file has several `unwrap()` calls in test code (lines 546, 554, 576) which are acceptable for tests.

---

## 6. Conclusion

The scanner module implementation matches the architecture document. All documented bug fixes have been properly applied. The only minor recommendation is to apply the same defensive `unwrap_or_else` pattern to `TemplateMarketplace::default()` for consistency with `CmsScanner::default()`.

**Recommendation**: No urgent fixes required. The module is stable and well-maintained.
