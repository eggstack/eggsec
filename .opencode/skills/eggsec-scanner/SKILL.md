---
name: eggsec-scanner
description: "Port scanning and endpoint discovery - use when working with port scans, service fingerprinting, endpoint discovery, CMS detection, or template-based vulnerability matching."
---

# Eggsec Scanner Skill

Port scanning and endpoint discovery module workflows and patterns.

## Key Files and Types

### Port Scanning (`scanner/ports/`)
- `mod.rs` - `scan_ports()` entry point, `PortScanConfig`, `PortResult`, `PortScanResults`
- `spoofed.rs` - Raw socket scanning, `init_packet_trace(path, include_header)` for CSV tracing
- `PortStatus` - a per-port verdict: `open`/`closed` are *proofs*; `filtered` and
  `open|filtered` are the *absence* of one. Never render an `open|filtered` as
  `open`, and never count list membership as "open" -- a UDP run puts every
  probed port in `open_ports`. Use `PortScanResults::proved_open_ports()`.
- `PortProtocol` - `Tcp` (default) or `Udp`; `#[serde(default)]` so payloads
  written before the field existed still load.
- UDP transport lives in the separate `eggsec-udp-scan` crate behind the
  `udp-scan` feature; the engine projects it into `PortStatus`/`PortProtocol`.
  See `architecture/capability_segregation.md`.

### Endpoint Discovery (`scanner/endpoints.rs`, `scanner/wordlist.rs`)
- `EndpointScanConfig`, `EndpointResult`, `EndpointScanResults`
- `Wordlist` - validated wordlist parsing with normalization and error checking
- 347 built-in endpoint paths (`DEFAULT_ENDPOINTS`)

### Fingerprinting (`scanner/fingerprint.rs`, `scanner/udp_fingerprint.rs`)
- `ServiceFingerprint`, `fingerprint_services()`, `fingerprint_udp_services()`

### Templates (`scanner/templates/`)
- `VulnerabilityTemplate`, `Matcher`, `HttpMatcher`, `DnsMatcher`, `TemplateInfo`
- `TemplateExecutor`, `TemplateMatcher`
- Uses `FxHashMap` for headers (not `std::collections::HashMap`)

### CMS (`scanner/cms/`)
- WordPress, Drupal, Joomla detection
- `CmsScanResult`, `CmsVulnerability`

## CLI Commands

| Command | Handler | Key Args |
|---------|---------|----------|
| `scan-ports <host>` | `handle_scan_ports()` | `--ports`, `--timeout`, `--source-ip`, `--decoy`, `--udp` |
| `scan-endpoints <url>` | `handle_scan_endpoints()` | `--wordlist`, `--source-ip`, `--concurrency` |
| `fingerprint <host>` | `handle_fingerprint()` | `--ports`, `--timeout` |

`--udp` is a *sibling* of `--scan-type`, not one of its values: `scan_type` is a
TCP technique with a strict parser. `--udp` needs the `udp-scan` feature and
fails closed without it rather than silently scanning TCP. Spoof/decoy/
fragment/packet-trace/max-rate/ttl options are TCP-SYN concepts and are
ignored (with a warning) under `--udp`.

## Critical Patterns

### Arc::try_unwrap Error Handling
```rust
// CORRECT - proper error handling
let results_map = Arc::try_unwrap(results).map_err(|_| {
    EggsecError::Runtime("Arc ref count non-zero after workers completed".into())
})?;
let results = results_map.into_iter().map(|(_, v)| v).collect();

// WRONG - could panic
let results = Arc::try_unwrap(results).expect("all workers completed").into_iter()...
```

### HashMap Usage
Use `FxHashMap` from `rustc_hash` for performance:
```rust
use rustc_hash::FxHashMap;
let headers: FxHashMap<String, String> = FxHashMap::default();
```

### init_packet_trace
```rust
// For new files (tests) - write header
init_packet_trace(path, true);

// For CLI runs - append without header
init_packet_trace(path, false);
```

## Testing

```bash
cargo test --lib -p eggsec -- scanner::
cargo test --test scanner_tests -p eggsec
```

## Adding New Features

### New Port Scan Type
1. Add to `scanner/ports/mod.rs` or `spoofed.rs`
2. Gate raw socket features behind `#[cfg(feature = "stress-testing")]`
3. Use `OnceLock<Mutex<File>>` for thread-safe packet tracing
4. Return proper `Result<PortScanResults>` with error handling

### New Endpoint Discovery Pattern
1. Add to `DEFAULT_ENDPOINTS` in `endpoints.rs` (for built-in paths)
2. Or users can supply custom endpoints via `--wordlist` / `Wordlist::from_file()`
3. Update `is_interesting()` for new sensitivity patterns
4. Add tests

## Bug Fixes (2026-05-22)

- `Arc::try_unwrap().expect()` replaced with `map_err` + proper error handling in 4 files
- `init_packet_trace` fixed with `include_header` boolean parameter
- Duplicate `HttpMatcher` removed, `DnsMatcher` properly ordered before `Matcher` enum
- HashMap → FxHashMap in templates/matcher.rs, templates/models.rs, cms/mod.rs

## Bug Fixes (2026-05-30)

| File | Issue | Fix |
|------|-------|-----|
| `scanner/ports/mod.rs:582` | Silent error suppression on progress send | Changed to explicit `is_err()` check with debug logging |
| `scanner/ports/spoofed.rs:450` | Silent error suppression on progress send | Same fix |
| `scanner/fingerprint.rs:306` | Silent error suppression on progress send | Same fix |
| `scanner/endpoints.rs:827` | Silent error suppression on progress send | Same fix |

## Bug Fixes (2026-05-27)

| File | Issue | Fix |
|------|-------|-----|
| `cms/joomla.rs:88-89` | String slice bounds could panic on malformed XML | Added bounds check before slicing |
| `templates/matcher.rs:185-189` | Invalid regex silently returned false | Added `tracing::debug` warning on invalid regex |
| `cms/mod.rs:330` | Default impl could panic on init failure | Changed `unwrap()` to `unwrap_or_else` with panic |
| `endpoints.rs:768` | Silent error suppression on network failures | Changed to explicit `match` with debug logging |
| `udp_fingerprint.rs:144` | Silent task join failures | Changed to explicit `match` with debug logging |