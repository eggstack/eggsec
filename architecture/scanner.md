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
| `cms/mod.rs:330` | Default impl could panic on init failure | Changed `unwrap()` to `unwrap_or_else` with panic |
| `endpoints.rs:768` | Silent error suppression on network failures | Changed to explicit `match` with debug logging |
| `udp_fingerprint.rs:144` | Silent task join failures | Changed to explicit `match` with debug logging |