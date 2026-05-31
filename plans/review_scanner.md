# Scanner Architecture Review

**Document:** architecture/scanner.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 79

## Verified Claims

| Claim | Status | Evidence |
|-------|--------|----------|
| 261 built-in paths for endpoint discovery | ✅ Verified | `crates/slapper/src/scanner/endpoints.rs:34-258` - `DEFAULT_ENDPOINTS` array, 261 entries |
| TCP Connect Scan using `tokio::net::TcpStream` | ✅ Verified | `crates/slapper/src/scanner/ports/mod.rs:15` - `use dashmap::DashMap`, tokio-based |
| Semaphore-controlled concurrency | ✅ Verified | `crates/slapper/src/scanner/ports/mod.rs:537` - `Arc::new(tokio::sync::Semaphore::new(config.concurrency))` |
| SYN Scan via `pnet` (requires `stress-testing`) | ✅ Verified | `crates/slapper/src/scanner/ports/spoofed.rs` - pnet imports with `#[cfg(all(feature = "stress-testing", unix))]` |
| IP spoofing with decoy support (Simultaneous/Staggered) | ✅ Verified | `crates/slapper/src/scanner/spoof.rs:14-17` - `DecoyMode::Simultaneous`, `DecoyMode::Staggered` |
| Timing Templates T0-T5 | ✅ Verified | `crates/slapper/src/scanner/timing.rs:4-11` - `TimingPreset` with Paranoid/Sneaky/Polite/Normal/Aggressive/Insane |
| TimingConfig controls parallelism, timeouts, rate limits | ✅ Verified | `crates/slapper/src/scanner/timing.rs:52-63` - `TimingConfig` struct with `min_parallelism`, `max_parallelism`, `timeout_ms`, `max_rate`, etc. |
| `DashMap` for concurrent result collection | ✅ Verified | `crates/slapper/src/scanner/ports/mod.rs:519` - `Arc<DashMap<u16, PortResult>>` |
| `FxHashMap` usage | ✅ Verified | `crates/slapper/src/scanner/templates/matcher.rs:12` - `use rustc_hash::FxHashMap` |
| `cms/` directory with WordPress, Drupal, Joomla | ✅ Verified | `crates/slapper/src/scanner/cms/` - `wordpress.rs`, `drupal.rs`, `joomla.rs` |
| CVE mapping in fingerprinting | ✅ Verified | `crates/slapper/src/scanner/fingerprint_types.rs:69` - `possible_cves: Vec<String>` |
| ICMP probing (requires `stress-testing`) | ✅ Verified | `crates/slapper/src/scanner/icmp_probe.rs` exists |
| UDP fingerprinting | ✅ Verified | `crates/slapper/src/scanner/udp_fingerprint.rs` exists |
| Spoofing in `spoof.rs` | ✅ Verified | `crates/slapper/src/scanner/spoof.rs` - `SpoofConfig` struct with `source_ip`, `decoy_ips`, etc. |
| Feature gating (`stress-testing`) | ✅ Verified | `crates/slapper/src/scanner/spoof.rs:6` - `#[cfg(all(feature = "stress-testing", unix))]` |
| `Arc::try_unwrap` + `map_err` pattern | ✅ Verified | `crates/slapper/src/scanner/ports/mod.rs:600-602` - proper error handling |
| No recursive crawling - flat wordlist only | ✅ Verified | `crates/slapper/src/scanner/endpoints.rs` - `DEFAULT_ENDPOINTS` is a flat array, no recursion logic |

## Discrepancies

### 1. Bug Fix References: Line Numbers May Be Stale

**Severity:** Informational

The document's "Bug Fixes (2026-05-22)" table references specific line numbers:
- `ports/mod.rs:595-598` - actual `Arc::try_unwrap` is at line 600
- `endpoints.rs:835-839` - actual `Arc::try_unwrap` is at line 842
- `fingerprint.rs:319-323` - actual `Arc::try_unwrap` is at line 320
- `templates/matcher.rs:9,24` - `FxHashMap` usage is at line 12-15
- `cms/mod.rs:52,165,291` - `FxHashMap` usage is at line 14, 52, 165

These are minor line number drifts from code changes since the bug fixes were documented. The fixes themselves are verified as present.

**Evidence:**
- `crates/slapper/src/scanner/ports/mod.rs:600` - `Arc::try_unwrap`
- `crates/slapper/src/scanner/endpoints.rs:842` - `Arc::try_unwrap`
- `crates/slapper/src/scanner/fingerprint.rs:320` - `Arc::try_unwrap`

### 2. "Bug Fixes (2026-05-27)" Line Numbers Also Drifted

**Severity:** Informational

- `cms/joomla.rs:88-89` - bounds check location may have shifted
- `templates/matcher.rs:185-189` - invalid regex warning location may have shifted
- `endpoints.rs:768` - silent error suppression fix location may have shifted
- `udp_fingerprint.rs:144` - silent task join fix location may have shifted

These line number references should be verified against current source if the bug fixes need to be referenced in code review.

## Bugs

No bugs found in the document. All structural claims are accurate.

## Improvements

### 1. Document Endpoint Wordlist Sourcing

The document mentions "261 built-in paths" but doesn't specify the sources or categories covered. The wordlist includes admin panels, API endpoints, config files, cloud metadata, CMS paths, database admin tools, DevOps tools, environment files, and more. A brief categorization would help users understand coverage.

### 2. Add CMS Vulnerability Count

The `cms/` module has CVE databases for WordPress, Drupal, and Joomla. The document mentions "CVE Mapping" but doesn't specify how many CVEs are tracked. Adding this count would help users assess the module's detection capability.

## Stale Items

### 1. Bug Fix Line Numbers

The line numbers in the bug fix tables (2026-05-22 and 2026-05-27 sections) have drifted from current source due to subsequent code changes. The fixes themselves are verified as present, but line references should be updated if the tables are used for code navigation.

**Evidence:**
- `crates/slapper/src/scanner/ports/mod.rs:600` vs doc's `595-598`
- `crates/slapper/src/scanner/endpoints.rs:842` vs doc's `835-839`
- `crates/slapper/src/scanner/fingerprint.rs:320` vs doc's `319-323`
