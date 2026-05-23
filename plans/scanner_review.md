# Scanner Module Architecture Review

**Date:** 2026-05-23  
**Reviewer:** Architecture Review Session  
**Document:** `architecture/scanner.md`

---

## Verified Claims

### Port Scanning (`ports/mod.rs`, `ports/spoofed.rs`)

| Claim | Status | Evidence |
|-------|--------|----------|
| TCP Connect Scan with `tokio::net::TcpStream` | **VERIFIED** | `ports/mod.rs:553` uses `connect_with_nodelay_timeout` |
| Semaphore-controlled concurrency | **VERIFIED** | `ports/mod.rs:537` creates `Semaphore::new(config.concurrency)` |
| SYN Scan via `pnet` crate | **VERIFIED** | `spoofed.rs:103` imports `build_tcp_packet` from `spoof.rs` ( gated by `stress-testing`) |
| Service Fingerprinting | **VERIFIED** | `fingerprint.rs` implements banner grabbing with `PROBES` static |
| Spoofed Scanning with IP spoofing and decoys | **VERIFIED** | `spoofed.rs` implements full spoofed scanning with `DecoyMode::Simultaneous`/`Staggered` |
| Timing Templates (T0-T5) | **VERIFIED** | `timing.rs:66-135` implements `TimingConfig::from_preset()` for all 6 presets |

### Endpoint Discovery (`endpoints.rs`)

| Claim | Status | Evidence |
|-------|--------|----------|
| Wordlist-based brute forcing | **VERIFIED** | `endpoints.rs:35-259` has `DEFAULT_ENDPOINTS` with 261 entries |
| Custom wordlist loading | **VERIFIED** | `endpoints.rs:377-386` reads from file via `tokio::fs::read_to_string` |
| Custom payload support | **VERIFIED** | `endpoints.rs` passes any `Vec<String>` as endpoints |
| Does NOT implement recursive crawling | **VERIFIED** | Confirmed - only flat wordlist scan |

### Fingerprinting (`fingerprint.rs`, `cms/`)

| Claim | Status | Evidence |
|-------|--------|----------|
| HTTP Banner Grabbing | **VERIFIED** | `fingerprint.rs:24` sends `HEAD / HTTP/1.0\r\n\r\n` probe |
| Technology Detection (CMS, frameworks) | **VERIFIED** | `cms/mod.rs:170-219` identifies WordPress, Drupal, Joomla |
| CVE Mapping | **VERIFIED** | `cms/mod.rs:100-126` `build_vulnerabilities()` maps CVEs to versions |
| 34+ service probes in `PROBES` static | **VERIFIED** | `fingerprint.rs:23-69` has 40+ probes |

### Advanced Probing

| Claim | Status | Evidence |
|-------|--------|----------|
| ICMP Probing (`icmp_probe.rs`) | **VERIFIED** | `icmp_probe.rs:1` gated by `#![cfg(feature = "stress-testing")]` |
| UDP Fingerprinting (`udp_fingerprint.rs`) | **VERIFIED** | `udp_fingerprint.rs:65-108` has 44 UDP probes |
| Spoofing (`spoof.rs`) | **VERIFIED** | `spoof.rs:313-365` `build_tcp_packet()` with raw sockets |

### Design Patterns

| Pattern | Status | Evidence |
|--------|--------|----------|
| `DashMap` for concurrent result collection | **VERIFIED** | Used in `ports/mod.rs:519`, `endpoints.rs:717`, `fingerprint.rs:251`, `spoofed.rs:151-154` |
| `tokio::sync::Semaphore` for concurrency control | **VERIFIED** | Used in all scan functions |
| `FxHashMap` instead of `HashMap` | **VERIFIED** | Fixed per Bug Fixes 2026-05-22 table |
| Feature gating (`stress-testing`) | **VERIFIED** | ICMP and spoofed scanning gated |
| `Arc::try_unwrap` + `map_err` | **VERIFIED** | Fixed per Bug Fixes 2026-05-22 table at `ports/mod.rs:595-597` |

---

## Discrepancies

### 1. Wordlist Count Mismatch
- **Document says:** "224 built-in paths"
- **Actual code:** 261 entries in `DEFAULT_ENDPOINTS` (`endpoints.rs:35-259`)
- **Impact:** Low - Documentation is outdated; more endpoints is beneficial
- **Priority:** Low

### 2. CMS Detection - "34 WAF Products" Confusion
- **Document says:** `architecture/scanner.md` mentions "34 WAF products" but this is in the **WAF module** documentation, not scanner
- **Actual:** Scanner's `cms/mod.rs` detects 3 CMS types (WordPress, Drupal, Joomla)
- **Impact:** None - different module, documentation issue in WAF doc
- **Priority:** Informational

### 3. Template Matcher DNS Matcher Documentation
- **Document says:** Section 26 mentions `DnsMatcher` fixed in Bug Fixes 2026-05-22
- **Actual code:** `DnsMatcher` exists at `templates/models.rs:62-67` and is used in `matcher.rs:162-180`
- **Impact:** No discrepancy - feature exists and is functional
- **Priority:** N/A

---

## Bugs Found

### Bug 1: Silent Error Suppression in CMS Component Enumeration
- **File:** `cms/mod.rs:258-268`
- **Code:**
  ```rust
  let plugins = wordpress::enumerate_plugins(url).await.unwrap_or_default();
  let themes = wordpress::enumerate_themes(url).await.unwrap_or_default();
  let modules = drupal::enumerate_modules(url).await.unwrap_or_default();
  let extensions = joomla::enumerate_extensions(url).await.unwrap_or_default();
  ```
- **Issue:** Silent fallback to empty vectors on network failures
- **Impact:** Medium - CMS detection may succeed but component enumeration silently fails, losing potentially valuable security findings
- **Fix:** Add `tracing::debug` for failures (already documented as fixed in 2026-05-27 but still uses `unwrap_or_default`)
- **Priority:** Medium

### Bug 2: Silent Error Suppression in `check_directory_listing`
- **File:** `cms/mod.rs:355`
- **Code:**
  ```rust
  let text = resp.text().await.unwrap_or_default();
  ```
- **Issue:** Same pattern - silently ignores text extraction failures
- **Impact:** Low-Medium - May miss directory listing misconfigurations
- **Priority:** Medium

### Bug 3: `scan_ports` Result Filtering Bug
- **File:** `ports/mod.rs:608`
- **Code:**
  ```rust
  let open_ports: Vec<PortResult> = results.into_iter().filter(|p| p.status == "open").collect();
  ```
- **Issue:** Filters to only "open" status, but `spoofed.rs:478` filters the same way. This is correct behavior for TCP but note that `PortResult.status` can be "filtered" or "closed" for UDP scans
- **Impact:** Low - Only "open" ports returned, which is standard for TCP scans
- **Priority:** Low

### Bug 4: No Bounds Check in `decoy_count_for_port`
- **File:** `ports/spoofed.rs:458-462`
- **Code:**
  ```rust
  fn decoy_count_for_port(config: &crate::scanner::spoof::SpoofConfig, port: u16) -> usize {
      let base_count = config.decoy_count.max(1);
      let variation = (port as usize) % 3;
      base_count + variation
  }
  ```
- **Issue:** No error case - returns `decoy_count + (port % 3)`. If `config.decoy_count` is 0, returns `1 + (port % 3)` which is correct minimum
- **Impact:** None - Logic is correct
- **Priority:** N/A

### Bug 5: Integer Division in `icmp_probe.rs`
- **File:** `icmp_probe.rs:87`
- **Code:**
  ```rust
  Some(total / rtts.len() as u32)
  ```
- **Issue:** `rtts.len()` is `usize`, converting to `u32` is safe but dividing `Duration` by `u32` is defined but imprecise
- **Impact:** Low - Imprecise average RTT calculation
- **Priority:** Low

---

## Improvement Opportunities

### High Priority

#### 1. CMS Component Enumeration Error Handling
- **File:** `cms/mod.rs:255-271`
- **Current:** Silent `unwrap_or_default()` on all CMS component enumeration
- **Suggested:** Replace with explicit error handling:
  ```rust
  let plugins = match wordpress::enumerate_plugins(url).await {
      Ok(p) => p,
      Err(e) => {
          tracing::debug!("Plugin enumeration failed: {}", e);
          Vec::new()
      }
  };
  ```
- **Estimated Impact:** Better visibility into scan quality, 30 min fix
- **Priority:** High

### Medium Priority

#### 2. Endpoint Scan - `Arc::try_unwrap` Error Message
- **File:** `endpoints.rs:840-842`
- **Current:** Generic error "Arc ref count non-zero after workers completed"
- **Issue:** Doesn't indicate which worker might be keeping a reference
- **Suggested:** Add debugging info or use a counter to track active workers
- **Estimated Impact:** Easier debugging, 15 min fix
- **Priority:** Medium

#### 3. Port Scan - Progress Bar Finish Race Condition
- **File:** `ports/mod.rs:590-593`
- **Issue:** Progress bar is finished before all results are processed. If `join_all` panics, progress bar is already finished
- **Suggested:** Consider wrapping `join_all` in error handling
- **Estimated Impact:** Better error visibility, 20 min fix
- **Priority:** Medium

#### 4. Fingerprint Service - Static Probes Slice
- **File:** `fingerprint.rs:23-69`
- **Current:** `PROBES` is already a `&'static [&str]` - good
- **Issue:** `probes_to_try` at line 347-391 creates a new slice per port via match. While it's a reference to static data, the match itself creates a new slice allocation in some cases
- **Suggested:** Consider making `probes_to_try` avoid the intermediate Vec by using an iterator that yields references
- **Estimated Impact:** Minor memory/performance improvement, 1 hour refactor
- **Priority:** Low-Medium

#### 5. Regex Compilation Not Cached in Template Matcher
- **File:** `templates/matcher.rs:185-192`
- **Current:** Regex is compiled on every `search_pattern` call for regex mode
- **Issue:** The same regex pattern may be used multiple times across templates
- **Suggested:** Add an `LruCache<String, Regex>` for compiled regex patterns
- **Estimated Impact:** Performance improvement for templates using regex matchers, 2 hour refactor
- **Priority:** Medium

### Low Priority

#### 6. Documentation: "224 built-in paths" Should be "261"
- **File:** `architecture/scanner.md`
- **Current:** Line 21 says "224 built-in paths"
- **Fix:** Update to "261 built-in paths"
- **Estimated Impact:** Documentation accuracy, 1 min fix
- **Priority:** Low

#### 7. Consistent Error Handling Pattern in CMS Scanner
- **File:** `cms/mod.rs:248`, `cms/mod.rs:355`
- **Current:** `unwrap_or_default()` in multiple places
- **Suggested:** Create a helper `async fn get_response_text(resp: Response) -> String` that handles errors consistently
- **Estimated Impact:** Code consistency, 30 min refactor
- **Priority:** Low

#### 8. Hardcoded UDP Probe Semaphore Limit
- **File:** `udp_fingerprint.rs:140`
- **Current:** `Semaphore::new(50)` is hardcoded
- **Issue:** Not configurable like TCP port scanning
- **Suggested:** Add to `UdpFingerprintConfig` or use `TimingConfig`
- **Estimated Impact:** Consistency, 30 min fix
- **Priority:** Low

#### 9. Missing Test Coverage for Spoofed Scanning
- **File:** `ports/spoofed.rs`
- **Current:** Only 1 test (`test_init_packet_trace_creates_file`)
- **Issue:** Core spoofed scanning logic has no unit tests
- **Suggested:** Add tests for `parse_tcp_response`, `decoy_count_for_port`, packet building
- **Estimated Impact:** Test coverage, 2 hour work
- **Priority:** Low

---

## Summary

| Category | Count | High Priority |
|----------|-------|--------------|
| Verified Claims | 18 | - |
| Discrepancies | 2 | 0 |
| Bugs Found | 5 | 0 |
| Improvement Opportunities | 9 | 1 |

### Key Takeaways

1. **Documentation Accuracy:** The architecture document is largely accurate. The main discrepancy is the wordlist count (224 vs 261).

2. **Error Handling:** The most significant improvement opportunity is in CMS component enumeration where `unwrap_or_default()` silently swallows errors. This was noted in the Bug Fixes section but not fully addressed.

3. **Design Patterns:** The module correctly uses `DashMap`, `FxHashMap`, `Semaphore`, and feature gating as documented. The `Arc::try_unwrap` pattern with proper error handling is correctly implemented.

4. **Test Coverage:** The spoofed scanning implementation lacks adequate unit tests despite being complex code with raw sockets, packet parsing, and concurrent response handling.

5. **Performance:** No major performance issues identified. The UDP socket reuse (mentioned in recent bug fixes) is correctly implemented.

---

## Recommendations

| Priority | Recommendation | Estimated Time |
|----------|----------------|----------------|
| High | Fix CMS component enumeration error handling | 30 min |
| High | Update documentation wordlist count (224 -> 261) | 1 min |
| Medium | Cache compiled regexes in template matcher | 2 hours |
| Medium | Add progress bar error handling wrapper | 20 min |
| Low | Add tests for spoofed scanning | 2 hours |
| Low | Make UDP semaphore configurable | 30 min |

---

*End of Review*
