# Scanner Module Architecture Review

Review of `architecture/scanner.md` against implementation in `crates/slapper/src/scanner/`

## Verified Claims

### Port Scanning (`ports/mod.rs`, `ports/spoofed.rs`)
- **TCP Connect Scan**: Uses `tokio::net::TcpStream` with semaphore-controlled concurrency (line 537)
- **SYN Scan**: Raw socket via `pnet` crate with `stress-testing` feature gate (spoofed.rs:10)
- **Service Fingerprinting**: `fingerprint_port()` in `fingerprint.rs:343-419` identifies services
- **Spoofed Scanning**: IP spoofing with decoy support in `spoofed.rs` (lines 310-396)
- **Timing Templates**: T0-T5 presets in `timing.rs:66-134` with correct values

### Endpoint Discovery (`endpoints.rs`)
- **Wordlist-based Brute Forcing**: 223 built-in endpoints in `DEFAULT_ENDPOINTS` (line 35-259)
- **Custom Wordlist Loading**: Supported via CLI arg (lines 377-386)
- **Non-recursive**: Confirmed flat scan only

### Fingerprinting (`fingerprint.rs`, `cms/`)
- **HTTP Banner Grabbing**: `extract_banner()` at line 492
- **Technology Detection**: CMS detection in `cms/mod.rs:196-218`
- **CVE Mapping**: Via `build_vulnerabilities()` method (cms/mod.rs:100-126)

### Advanced Probing
- **ICMP Probing**: `icmp_probe.rs` with `stress-testing` gate
- **UDP Fingerprinting**: `udp_fingerprint.rs` with probe payloads
- **Spoofing**: `spoof.rs` with decoy modes

### Design Patterns (Verified)
| Pattern | Implementation |
|---------|----------------|
| `DashMap` | ports/mod.rs:519, endpoints.rs:717, spoofed.rs:151-154, fingerprint.rs:251 |
| `tokio::sync::Semaphore` | All scan modules |
| `FxHashMap` | All HashMap usages now use `rustc_hash::FxHashMap` |
| Feature gating | `#[cfg(feature = "stress-testing")]` on ICMP/spoofed |
| `Arc::try_unwrap` + `map_err` | ports/mod.rs:595, endpoints.rs:840, fingerprint.rs:319 |

---

## Discrepancies

### 1. Endpoint Wordlist Count Mismatch
**Doc**: "224 built-in paths"
**Actual**: 223 endpoints (counted via `awk '/^    "\// {count++} END {print count}'`)

### 2. Bug Fix Table Format Issue
The bug fix tables reference line numbers (e.g., `ports/mod.rs:595-598`) that don't correspond to specific bugs. The actual fixes are:
- `Arc::try_unwrap(...).expect()` → `map_err` (verified, lines 595-597)
- File opened twice in `init_packet_trace` - the `include_header` parameter exists (line 81)
- Unused HashMap import removed - verified none remain in spoofed.rs

### 3. Missing `cms/` from File List
Doc says `cms/` subdirectory but doesn't list individual CMS files in core capabilities. Implementation has `wordpress.rs`, `drupal.rs`, `joomla.rs`.

---

## Bugs Found

### BUG-1: Dynamic Probe Vector Allocation in Hot Path (HIGH)
**File**: `fingerprint.rs:347-391`
**Issue**: `probes_to_try` is a `Vec` rebuilt on every port scan, even though most match specific static probes based on port number.
```rust
let probes_to_try: Vec<(&str, &[u8], &str)> = match port {
    8080 | 8090 | 8180 => vec![("Jenkins", ...)],  // allocates Vec every time
    // ...
    _ => PROBES.to_vec(),  // copies entire static array
};
```
**Fix**: Use `&'static [` slice references instead of `Vec`:
```rust
let probes_to_try: &[(&str, &[u8], &str)] = match port {
    8080 | 8090 | 8180 => &[("Jenkins", b"GET /api/json...", "\"jobs\"|Crumb")],
    // ...
};
```

### BUG-2: UDP Socket Per-Port Binding (MEDIUM)
**File**: `udp_fingerprint.rs:169`
**Issue**: Each UDP port fingerprint binds a new socket:
```rust
let socket = match UdpSocket::bind("0.0.0.0:0").await {
```
On high-port scans, this creates 1000s of socket pairs. Should reuse a single socket with `Arc<UdpSocket>`.

### BUG-3: Missing Error Context in Spoofed Scan (MEDIUM)
**File**: `spoofed.rs:285-307`
**Issue**: Errors from `build_tcp_packet` and `send_to` are silently swallowed:
```rust
Err(_) => {
    drop(permit);
    return;
}
```
No logging of why spoofed packets fail to build/send.

---

## Improvement Opportunities

### IMP-1: Batch UDP Scanning
**File**: `udp_fingerprint.rs`
Current implementation spawns individual tasks per port. For 1000 ports, this creates 1000 task futures. Consider batching with `tokio::sync::mpsc` channel-based worker pools.

### IMP-2: Lazy Evaluation of Default Endpoints
**File**: `endpoints.rs:35-259`
`DEFAULT_ENDPOINTS` is a static array that always exists in binary. Consider making it a lazy static or loading from a bundled data file to reduce binary size for embedded deployments.

### IMP-3: Consistent Error Handling Pattern
**File**: Multiple files
Some modules use explicit `match` with `tracing::debug` (endpoints.rs:814), others silently ignore (udp_fingerprint.rs:232). Standardize on:
```rust
Err(e) => {
    tracing::debug!("operation failed: {}", e);
}
```

### IMP-4: Missing `const` for Static Data
**File**: `timing.rs:163-176`
`CRITICAL_PORTS` and `HIGH_PORTS` are `const` arrays but could be `const` more explicitly:
```rust
pub const CRITICAL_PORTS: [u16; 20] = [...];
```

### IMP-5: Code Duplication in CLI Runners
**Files**: `ports/mod.rs:159-322` and `324-502`
`run_cli` and `run_cli_with_callback` share ~150 lines of duplicate code. Extract common logic into a shared helper.

---

## Priority Summary

| ID | Category | Finding | Priority |
|----|----------|---------|----------|
| BUG-1 | Performance | Dynamic Vec allocation in fingerprint hot path | HIGH |
| BUG-2 | Performance | Per-port UDP socket binding | MEDIUM |
| BUG-3 | Observability | Silent errors in spoofed scan | MEDIUM |
| IMP-1 | Performance | UDP batching | MEDIUM |
| DIS-1 | Documentation | Wordlist count mismatch (224 vs 223) | LOW |
| IMP-2 | Binary Size | Lazy default endpoints loading | LOW |
| IMP-3 | Code Quality | Standardize error handling | LOW |
| IMP-4 | Code Quality | Explicit const types | LOW |
| IMP-5 | Code Quality | DRY CLI runner code | LOW |

---

## Recommendations

1. **High Priority**: Fix BUG-1 by changing probe matching to use static slice references instead of Vec allocation
2. **High Priority**: Fix BUG-2 by reusing UDP socket across ports in a session
3. **Medium Priority**: Add tracing to silent error paths in spoofed.rs
4. **Documentation**: Update wordlist count from 224 to 223