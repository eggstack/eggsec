# Scanner Module Override

## Key Files

| File | Purpose |
|------|---------|
| `scanner/ports/mod.rs` | Main TCP port scanning with semaphore concurrency |
| `scanner/ports/spoofed.rs` | Raw socket scanning (requires `stress-testing` + Unix) |
| `scanner/endpoints.rs` | HTTP endpoint discovery with wordlist brute forcing |
| `scanner/fingerprint.rs` | TCP service fingerprinting via banner grabbing |
| `scanner/udp_fingerprint.rs` | UDP service fingerprinting |
| `scanner/spoof.rs` | IP spoofing config, decoy modes, packet crafting |
| `scanner/timing.rs` | Nmap-style timing presets (T0-T5) |
| `scanner/templates/` | Nuclei-style vulnerability template engine |
| `scanner/cms/` | WordPress, Drupal, Joomla detection + CVE mapping |

## Critical Patterns

### Arc::try_unwrap Error Handling
When collecting results from parallel workers, the pattern is:
```rust
let results_map = Arc::try_unwrap(results).map_err(|_| {
    crate::error::SlapperError::Runtime("Arc ref count non-zero after workers completed".into())
})?;
let results = results_map.into_iter().map(|(_, v)| v).collect();
```

### Hash Collections
Use `FxHashMap`/`FxHashSet` from `rustc_hash` instead of `std::collections::HashMap`:
```rust
use rustc_hash::FxHashMap;
let map: FxHashMap<String, String> = FxHashMap::default();
```

### Packet Trace Initialization
`init_packet_trace(path, include_header)` takes a boolean to control header writing:
- `true` = write CSV header (for new files/tests)
- `false` = append without header (for CLI runs)

## Bug Fixes Applied (2026-05-22)

| Issue | Fix |
|-------|-----|
| `Arc::try_unwrap().expect()` panic in 4 files | Proper error handling via `map_err` |
| `init_packet_trace` opened file twice with contradictory options | Added `include_header` parameter |
| Duplicate `HttpMatcher` definition | Removed duplicate, `DnsMatcher` now defined before `Matcher` enum |
| HashMap in templates/matcher, templates/models, cms/mod | Changed to `FxHashMap` |
| Unused `std::collections::HashMap` import in spoofed.rs | Removed unused import |

## Bug Fixes Applied (2026-05-28)

| Issue | Fix |
|-------|-----|
| `fingerprint.rs:347-391` - Vec allocation in hot path | Changed to `&'static [&str]` slice |
| `spoofed.rs:285,303` - silent errors from build_tcp_packet and send_to | Added `tracing::debug` for failed packet builds |

## Bug Fixes Applied (2026-06-07)

| Issue | Fix |
|-------|-----|
| Fragmented packets never populated `sent_packets` - all responses silently dropped | Added `sent_packets.insert()` after sending fragments |
| Off-by-one in spoofed progress reporting | Changed to `fetch_add(1, ...) + 1` |
| Early-return error paths skipped progress update | Added progress increment before returns |
| `TokenBucket` race condition in refill | Refactored to `compare_exchange` loop |
| `template_id` path traversal in marketplace | Added `/`, `\`, `..` validation |
| Tag parameter not URL-encoded | Used `urlencoding::encode()` |
| Server header parsing lost port info | Changed to `split_once(':')` |
| CMS enumerate functions created new clients | Accept `&Client` parameter |
| `endpoints.rs` used async Mutex for counter | Replaced with `AtomicU64` |

## Bug Fixes Applied (2026-06-07, round 3)

| Issue | Fix |
|-------|-----|
| `max_rate=0` caused division by zero panic in rate limiting | Added validation in `from_args` |
| Simultaneous decoy mode logged "staggered" message | Fixed log message to match actual mode |
| Staggered decoy mode logged generic message | Fixed log message to "staggered decoy packet" |
| `TemplateMarketplace::default()` panicked on client failure | Falls back to `reqwest::Client::new()` |

## Bug Fixes Applied (2026-06-07, round 4)

| Issue | Fix |
|-------|-----|
| `check_xml_rpc` sent JSON body to XML-RPC endpoint | Sends proper XML-RPC format with `text/xml` Content-Type |
| Error-path progress sends used silent `let _ =` | Logs warning on failure to match success-path behavior |
| `CmsScanner::default()` fallback used `expect()` | Changed to `unwrap_or_else` with `reqwest::Client::new()` |



(End file - 72 lines)