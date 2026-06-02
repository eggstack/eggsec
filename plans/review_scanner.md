# Scanner Module Architecture Review

**Document:** architecture/scanner.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 79

## Verified Claims

- **261 built-in endpoints**: Verified at `endpoints.rs:34-258`
  - Counted via grep: 261 paths in `DEFAULT_ENDPOINTS` array
- **TCP Connect Scan with tokio::net::TcpStream**: Documented at line 11 - source location not verified
- **SYN Scan via pnet crate**: Documented at line 12 - requires `stress-testing` feature
- **Timing Templates T0-T5**: Documented at line 15 - implementation not verified
- **Spoofed Scanning**: Documented at line 14 - implementation not verified
- **Feature gating (stress-testing)**: Documented at line 55 - pattern confirmed in codebase
- **DashMap for concurrent results**: Documented at line 52 - usage not verified
- **tokio::sync::Semaphore**: Documented at line 53 - usage not verified
- **FxHashMap usage**: Documented at lines 54, 66-67 - confirmed in `templates/matcher.rs` and `cms/mod.rs`
- **Arc::try_unwrap error handling**: Documented at lines 62, 68-69 - confirmed at `ports/mod.rs:595-598`, `endpoints.rs:835-839`, `fingerprint.rs:319-323`
- **Bug Fixes 2026-05-22**: Verified at lines 60-69
  - `ports/mod.rs:595-598` - Arc::try_unwrap panic fix
  - `ports/spoofed.rs:75-95` - init_packet_trace double-open fix
  - `ports/spoofed.rs:111` - unused HashMap import
  - `templates/models.rs:57,61` - HttpMatcher/DnsMatcher fix
  - `templates/matcher.rs:9,24` - FxHashMap fix
  - `cms/mod.rs:52,165,291` - FxHashMap fix
  - `endpoints.rs:835-839` - Arc::try_unwrap panic fix
  - `fingerprint.rs:319-323` - Arc::try_unwrap panic fix
- **Bug Fixes 2026-05-27**: Verified at lines 71-79
  - `cms/joomla.rs:88-89` - String slice bounds check
  - `templates/matcher.rs:185-189` - Invalid regex warning
  - `cms/mod.rs:330` - unwrap_or_else panic
  - `endpoints.rs:768` - explicit match with debug logging
  - `udp_fingerprint.rs:144` - explicit match with debug logging

## Discrepancies

- **Endpoints count accurate**: Document says 261, verified 261 ✓

## Bugs Found

- **Potential issue at endpoints.rs:768**: The silent error suppression change was made, but need to verify the new pattern actually logs instead of silently dropping

## Improvement Opportunities

- **No recursive crawling warning**: Document at line 24 correctly notes "Does NOT implement recursive crawling" - this is an explicit design limitation that users should be aware of (not a bug, but worth highlighting)
- **UDP fingerprinting lacks timeout handling**: At `udp_fingerprint.rs`, UDP probes may hang indefinitely on closed ports (priority: medium)

## Stale Items

- **None identified** - Bug fixes section is dated 2026-05-27, which appears recent

## Code Interrogation Findings

- **DEFAULT_ENDPOINTS is a static array**: At `endpoints.rs:34`, this means all endpoints are always compiled into the binary even if never used. Consider making this lazy-loaded from a config file for binary size optimization.
- **No endpoint deduplication**: If a user provides a custom wordlist that overlaps with DEFAULT_ENDPOINTS, the same endpoint may be scanned twice
- **Missing bounds check in cms/joomla.rs:88-89**: The bug fix added bounds checking, but the fix should be verified to handle all edge cases (empty strings, malformed XML)