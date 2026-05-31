# Scanner Architecture Review

**Document:** architecture/scanner.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims
- TCP Connect Scan using `tokio::net::TcpStream` with semaphore-controlled concurrency verified at `crates/slapper/src/scanner/ports/mod.rs`
- SYN Scan via `pnet` crate (requires `stress-testing` feature + Unix) verified at `crates/slapper/src/scanner/ports/spoofed.rs:6,15`
- Service Fingerprinting exists at `crates/slapper/src/scanner/fingerprint.rs`
- Spoofed Scanning with IP spoofing and decoy support verified at `crates/slapper/src/scanner/ports/spoofed.rs`
- Timing Templates (T0-T5) exist at `crates/slapper/src/scanner/timing.rs`
- Endpoint Discovery (`endpoints.rs`) exists at `crates/slapper/src/scanner/endpoints.rs`
- Wordlist-based brute forcing with built-in paths verified - actual count is 261 endpoints (doc says 224) at `crates/slapper/src/scanner/endpoints.rs:34-250+`
- Custom Wordlist Loading supported
- Does NOT implement recursive crawling (flat wordlist scan only) - verified
- Fingerprinting (`fingerprint.rs`, `cms/`) exists at `crates/slapper/src/scanner/fingerprint.rs` and `crates/slapper/src/scanner/cms/`
- ICMP Probing (`icmp_probe.rs`) feature-gated behind `stress-testing` verified at `crates/slapper/src/scanner/mod.rs:91-92`
- UDP Fingerprinting (`udp_fingerprint.rs`) exists at `crates/slapper/src/scanner/udp_fingerprint.rs`
- Spoofing (`spoof.rs`) exists at `crates/slapper/src/scanner/spoof.rs`
- DashMap for concurrent result collection verified at `crates/slapper/src/scanner/endpoints.rs:5`
- tokio::sync::Semaphore for concurrency control verified
- FxHashMap usage in templates/matcher.rs verified at `crates/slapper/src/scanner/templates/matcher.rs:12-15`
- Feature gating (`stress-testing`) for ICMP and raw socket features verified at `crates/slapper/src/scanner/mod.rs:91-92`
- Arc::try_unwrap pattern with map_err verified at `crates/slapper/src/scanner/ports/mod.rs:600-602`

## Discrepancies
- [Endpoint count]: Documented as "224 built-in paths", but actual count is 261 endpoints in `DEFAULT_ENDPOINTS` (`crates/slapper/src/scanner/endpoints.rs:34-250+`)

## Bugs Found
- None identified

## Improvement Opportunities
- [Endpoint count update]: Update documentation to reflect the actual 261 built-in endpoint paths. (priority: low)

## Stale Items
- None identified
