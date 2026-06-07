# Scanner Module

The Scanner module is responsible for the "discovery" phase of a security assessment. It includes port scanning, service identification, and endpoint discovery.

## Core Capabilities (`src/scanner/`)

### Port Scanning (`ports/`)

High-performance TCP and UDP port scanning.

- **TCP Connect Scan**: Standard TCP connection using `tokio::net::TcpStream` with semaphore-controlled concurrency
- **SYN Scan**: Raw socket scanning via `pnet` crate (requires `stress-testing` feature + Unix + root privileges)
- **Service Fingerprinting**: Once a port is found open, Slapper attempts to identify the running service and version
- **Spoofed Scanning**: IP spoofing with decoy support (Simultaneous or Staggered modes)
- **Timing Templates**: Nmap-style T0-T5 presets controlling parallelism, timeouts, and rate limits

### Endpoint Discovery (`endpoints.rs`)

Finding hidden files and directories on web servers.

- **Wordlist-based Brute Forcing**: Uses extensive wordlists (261 built-in paths) to find common endpoints
- **Custom Wordlist Loading**: Load endpoints from file
- **Custom Payload Support**: Allows for targeted discovery based on specific technologies
- **Note**: Does NOT implement recursive crawling - flat wordlist scan only

### Fingerprinting (`fingerprint.rs`, `cms/`)

Identifying the technology stack of a target.

- **HTTP Banner Grabbing**: Extracting information from server headers
- **Technology Detection**: Identifying frameworks (e.g., React, Django), databases, and CMS (e.g., WordPress, Drupal)
- **CVE Mapping**: Automatically mapping discovered versions to known vulnerabilities

### Advanced Probing (`icmp_probe.rs`, `udp_fingerprint.rs`)

- **ICMP Probing**: Host discovery using echo requests (requires `stress-testing`)
- **UDP Fingerprinting**: Identifying services on UDP ports through specific probe payloads
- **Spoofing (`spoof.rs`)**: Techniques for source IP spoofing and decoys (where supported)

## Timing and Performance (`timing.rs`)

The scanner uses "Timing Templates" (similar to Nmap's -T0 through -T5) to control the speed and aggressiveness of scans, ensuring they stay within the limits of the target network and the user's requirements.

## Integration

Discovered information is often fed into the **Fuzzer** or **Vulnerability Management** modules for further analysis.

## Key Design Patterns

| Pattern | Usage |
|---------|-------|
| `DashMap` | Lock-free concurrent result collection |
| `tokio::sync::Semaphore` | Concurrency control for parallel operations |
| `rustc_hash::FxHashMap` | High-performance hash map (instead of std `HashMap`) |
| Feature gating (`stress-testing`) | ICMP and raw socket features gated behind feature flag |
| `Arc::try_unwrap` + `map_err` | Safe error handling when collecting parallel results |

## Bug Fixes (2026-05-22)

| File | Issue | Fix |
|------|-------|-----|
| `ports/mod.rs:595-598` | `Arc::try_unwrap(...).expect()` panic | Proper error handling |
| `ports/spoofed.rs:75-95` | `init_packet_trace` opened file twice | Added `include_header` parameter |
| `ports/spoofed.rs:111` | Unused `std::collections::HashMap` import | Removed unused import |
| `templates/models.rs:57,61` | Duplicate `HttpMatcher` + missing `DnsMatcher` | Fixed struct order |
| `templates/matcher.rs:9,24` | `HashMap` instead of `FxHashMap` | Performance fix |
| `cms/mod.rs:52,165,291` | `HashMap` instead of `FxHashMap` | Performance fix |
| `endpoints.rs:835-839` | `Arc::try_unwrap(...).expect()` panic | Proper error handling |
| `fingerprint.rs:319-323` | `Arc::try_unwrap(...).expect()` panic | Proper error handling |

## Bug Fixes (2026-05-27)

| File | Issue | Fix |
|------|-------|-----|
| `cms/joomla.rs:88-89` | String slice bounds could panic on malformed XML | Added bounds check before slicing |
| `templates/matcher.rs:185-189` | Invalid regex silently returned false | Added `tracing::debug` warning on invalid regex |
| `cms/mod.rs:330` | Default impl could panic on init failure | Changed `unwrap()` to `unwrap_or_else` with fallback client |

## Bug Fixes (2026-06-07)

| File | Issue | Fix |
|------|-------|-----|
| `ports/spoofed.rs:288-295` | Fragmented packets never populated `sent_packets` map, causing all responses to be silently dropped | Added `sent_packets.insert()` after sending fragments |
| `ports/spoofed.rs:472-473` | Off-by-one in spoofed progress (pre-increment vs post-increment) | Changed to `fetch_add(1, ...) + 1` to match non-spoofed scan |
| `ports/spoofed.rs:298-301,320-326` | Early-return error paths skipped progress bar and progress_tx notification | Added progress updates before early returns |
| `ports/spoofed.rs:298` | Wrong error message said "UDP packets" for TCP fragments | Changed to "fragmented TCP packets" |
| `spoof.rs:197` | `header_value()` confusing modulo logic with off-by-one edge case | Simplified to `rand % len` with direct return |
| `udp_fingerprint.rs:301-320` | `TokenBucket` race condition in refill (non-atomic read-modify-write) | Refactored to use `compare_exchange` loop in `refill()` |
| `templates/marketplace.rs:176` | `template_id` path traversal via unsanitized IDs | Added validation rejecting `/`, `\`, `..` in template IDs |
| `templates/marketplace.rs:87` | Tag parameter not URL-encoded, allowing query injection | Used `urlencoding::encode()` for tag values |
| `fingerprint.rs:510` | Server header parsing lost port info with `split(':')` | Changed to `split_once(':')` to preserve `host:port` |
| `fingerprint.rs:437-438` | Unnecessary `SmallVec` + `resize` for buffer allocation | Replaced with `vec![0u8; 4096]` |
| `templates/matcher.rs:105` | Dead `let _ = &matcher.method` binding | Removed unused binding |
| `cms/mod.rs:358-363` | `Default` impl panicked on init failure | Uses fallback HTTP client instead of panicking |
| `cms/wordpress.rs:36-39,65-68` | `enumerate_plugins/themes` created new `Client` per call | Accept `&Client` parameter, reuse caller's client |
| `cms/drupal.rs:31`, `cms/joomla.rs:31` | Enumerate functions ignored TLS verification setting | Accept `&Client` parameter from caller |
| `endpoints.rs:717` | Used `tokio::sync::Mutex` for simple counter | Replaced with `AtomicU64` for zero-overhead atomic increments |
| `endpoints.rs:768` | Silent error suppression on network failures | Changed to explicit `match` with debug logging |
| `udp_fingerprint.rs:144` | Silent task join failures | Changed to explicit `match` with debug logging |

## Bug Fixes (2026-06-07, round 3)

| File | Issue | Fix |
|------|-------|-----|
| `spoof.rs:126` | `max_rate=0` caused division by zero panic in spoofed scan rate limiting | Added validation: `max_rate` must be > 0 |
| `ports/spoofed.rs:384` | Simultaneous decoy mode logged "staggered decoy packet" | Fixed to "simultaneous decoy packet" |
| `ports/spoofed.rs:425` | Staggered decoy mode logged generic "decoy packet" | Fixed to "staggered decoy packet" |
| `templates/marketplace.rs:279` | `Default::default()` panicked if reqwest client construction failed | Falls back to `reqwest::Client::new()` instead of panicking |

## Bug Fixes (2026-06-07, round 4)

| File | Issue | Fix |
|------|-------|-----|
| `cms/wordpress.rs:122-133` | `check_xml_rpc` sent JSON body to XML-RPC endpoint | Sends proper XML-RPC format, validates response contains XML-RPC indicators |
| `ports/spoofed.rs:306,338` | Error-path progress sends used silent `let _ =` pattern | Logs warning on failure to match success-path behavior |
| `cms/mod.rs:348` | `CmsScanner` Default fallback still used `expect()` | Changed to `unwrap_or_else` with `reqwest::Client::new()` fallback |

## Bug Fixes (2026-06-07, round 5)

| File | Issue | Fix |
|------|-------|-----|
| `spoof.rs:432` | `build_fragmented_packets` over-allocated buffer (always 28 bytes) causing trailing zeros on wire for last fragment | Changed to `vec![0u8; 20 + chunk.len()]` for exact per-fragment sizing |