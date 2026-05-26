# Slapper Scanner Skill

Port scanning and endpoint discovery module workflows and patterns.

## Key Files and Types

### Port Scanning (`scanner/ports/`)
- `mod.rs` - `scan_ports()` entry point, `PortScanConfig`, `PortResult`, `PortScanResults`
- `spoofed.rs` - Raw socket scanning, `init_packet_trace(path, include_header)` for CSV tracing

### Endpoint Discovery (`scanner/endpoints.rs`)
- `EndpointScanConfig`, `EndpointResult`, `EndpointScanResults`
- 224 built-in endpoint paths

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
| `scan-ports <host>` | `handle_scan_ports()` | `--ports`, `--timeout`, `--spoof-ip`, `--decoy` |
| `scan-endpoints <url>` | `handle_scan_endpoints()` | `--wordlist`, `--spoof-ip`, `--concurrency` |
| `fingerprint <host>` | `handle_fingerprint()` | `--ports`, `--timeout` |

## Critical Patterns

### Arc::try_unwrap Error Handling
```rust
// CORRECT - proper error handling
let results_map = Arc::try_unwrap(results).map_err(|_| {
    SlapperError::Runtime("Arc ref count non-zero after workers completed".into())
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
cargo test --lib -p slapper -- scanner::
cargo test --test scanner_tests -p slapper
```

## Adding New Features

### New Port Scan Type
1. Add to `scanner/ports/mod.rs` or `spoofed.rs`
2. Gate raw socket features behind `#[cfg(feature = "stress-testing")]`
3. Use `OnceLock<Mutex<File>>` for thread-safe packet tracing
4. Return proper `Result<PortScanResults>` with error handling

### New Endpoint Discovery Pattern
1. Add to `DEFAULT_ENDPOINTS` in `endpoints.rs`
2. Update `is_interesting()` for new sensitivity patterns
3. Add tests

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