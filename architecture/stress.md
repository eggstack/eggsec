# Stress Testing Module

## Overview

The stress testing module provides denial-of-service simulation capabilities for authorized defense-lab and resilience testing. It generates high volumes of network traffic (SYN, UDP, HTTP, ICMP floods) to test system resilience under attack conditions. The module is designed as a **defense-lab tool** — it requires explicit scope authorization, rate/duration caps, and (optionally) interactive confirmation before any flood is dispatched.

**Feature gate:** `stress-testing` (in `Cargo.toml`). Authorization, metrics, and warning modules compile unconditionally so config validation and scope checks work even when flood engines are off.

**Role:** Defense-lab DoS simulation with hard authorization gates. For HTTP performance benchmarking (no spoofing, no raw sockets), see [loadtest.md](loadtest.md).

## Module Structure

| File | Lines | Feature-gated | Purpose |
|------|-------|---------------|---------|
| `mod.rs` | 216 | no | Orchestrator: `StressTest`, `StressType`, `StressConfig`, `StressResult`, `StressConfigSummary` |
| `syn.rs` | 284 | `stress-testing` + unix | SYN flood via raw Ethernet frames (IPv4 + IPv6) |
| `udp.rs` | 427 | `stress-testing` | UDP flood: standard socket + raw socket spoofed mode |
| `http.rs` | 204 | `stress-testing` | HTTP GET flood with proxy pool support |
| `icmp.rs` | 247 | `stress-testing` + unix | ICMP echo request flood (IPv4 + IPv6) via raw Ethernet |
| `metrics.rs` | 222 | always compiled | Thread-safe atomic counters: `StressMetrics`, `StressStats` |
| `authorization.rs` | 272 | always compiled | Scope enforcement, rate/duration caps, TOML config |
| `warning.rs` | 89 | always compiled | Legal warning banner, interactive confirmation prompt |
| `utils.rs` | 207 | `stress-testing` | DNS resolution, interface detection, channel creation, spoofed IPs, payload generation |

**Total:** 10 files, 2168 lines.

### Compilation model

- `metrics`, `authorization`, `warning` compile unconditionally (needed for config/validation even when flood types are off).
- `syn`, `icmp` require both `stress-testing` and unix (raw Ethernet via pnet).
- `udp`, `http`, `utils` require `stress-testing` only.
- When `stress-testing` is off, `run_inner()` returns an error message directing the user to enable the feature (`mod.rs:157-164`).

## Key Types

### `StressType` enum

Five variants, serializable via serde (`mod.rs:23-35`):

```rust
pub enum StressType {
    Syn,   // "SYN flood"
    Udp,   // "UDP flood"
    Http,  // "HTTP flood"
    Tcp,   // "TCP flood" — NOT IMPLEMENTED
    Icmp,  // "ICMP flood"
}
```

**⚠ TCP flood is declared but NOT IMPLEMENTED.** Selecting `StressType::Tcp` returns `Err(EggsecError::Runtime(...))` at `mod.rs:139-144` with the message: *"TCP flood is not yet implemented. Use HTTP flood for application-layer testing."* This is not a compile-time error — it surfaces at runtime only when the operator explicitly selects TCP.

`Display` impl (`mod.rs:37-47`) produces human-readable names for all five variants.

### `StressConfig`

Runtime configuration for a test run (`mod.rs:49-63`):

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target` | `String` | `""` | Target hostname or IP |
| `port` | `u16` | `80` | Target port (TCP/UDP) |
| `stress_type` | `StressType` | `Http` | Flood type |
| `rate_pps` | `u64` | `1000` | Packets per second |
| `duration_secs` | `u64` | `60` | Test duration |
| `concurrency` | `usize` | `10` | Parallel workers |
| `spoof_source` | `bool` | `false` | Enable IP source spoofing |
| `spoof_range` | `Option<String>` | `None` | CIDR or dash-range for spoofed IPs |
| `random_source_port` | `bool` | `true` | Randomize source port each packet |
| `payload_size` | `usize` | `64` | Payload bytes (HTTP: path length; ICMP: min 56) |
| `use_proxies` | `bool` | `false` | Route HTTP flood through proxy pool |
| `proxy_pool` | `Option<String>` | `None` | Path to proxy list file |

### `StressTest` (orchestrator)

Main entry point (`mod.rs:84-89`). Created via `StressTest::new(config)` (`mod.rs:92-105`) which:

1. Loads `StressAuthorization::from_scope()` (TOML config + scope file).
2. Verifies target is in scope (`verify_target`).
3. Verifies rate is within limits (`verify_rate`).
4. Verifies duration is within limits (`verify_duration`).
5. Initializes `StressMetrics` (when `stress-testing` feature enabled).

Two run methods:

- **`run()`** (`mod.rs:107-114`) — Interactive: displays warning banner, prompts for `"yes"` confirmation (if `require_confirmation` is set in scope), then dispatches to the flood implementation.
- **`run_non_interactive()`** (`mod.rs:116-119`) — Bypasses stdin confirmation (used by non-interactive surfaces like REST/agent).

Both call `run_inner()` which dispatches based on `StressType` (`mod.rs:121-165`):

```rust
Syn  → syn::run_syn_flood()
Udp  → udp::run_udp_flood()
Icmp → icmp::run_icmp_flood()
Http → http::run_http_flood()
Tcp  → returns error ("not yet implemented")
```

### `StressResult` and `StressConfigSummary`

Serializable output types for report generation (`mod.rs:168-183`):

```rust
pub struct StressResult {
    pub target: String,
    pub stress_type: StressType,
    pub stats: StressStats,
    pub config_used: StressConfigSummary,
    pub warnings: Vec<String>,
}

pub struct StressConfigSummary {
    pub rate_pps: u64,
    pub duration_secs: u64,
    pub spoof_source: bool,
    pub used_proxies: bool,
}
```

## Attack Types

### SYN Flood (`syn.rs`)

Raw Ethernet-level SYN packet construction via `pnet`. Builds complete Ethernet + IPv4/IPv6 + TCP(SYN) frames.

- **IPv4**: Ethernet (14) + IPv4 (20) + TCP (20) = 54-byte frame (`syn.rs:121`).
- **IPv6**: Ethernet (14) + IPv6 (40) + TCP (20) = 74-byte frame (`syn.rs:168`).
- Sequence numbers increment from 1000, wrapping via `wrapping_add(1)` (`syn.rs:40,98`).
- Source port randomized in `[40000, 60000)` or incremented (`syn.rs:44-48`).
- Manual TCP checksum computation over pseudo-header (`syn.rs:201-257`).
- Source IP: local interface IP (normal) or random from spoof range (spoofed) (`syn.rs:50-83`).
- Destination MAC set to zeros (layer-2 forwarding) (`syn.rs:32`).
- Rate-controlled via `tokio::time::sleep(interval)` where `interval = 1s / rate_pps` (`syn.rs:38,100-102`).
- **Unix only** — uses `pnet::datalink` for raw Ethernet channel.

### UDP Flood (`udp.rs`)

Two modes:

1. **Standard mode** (`run_udp_flood_standard`, `udp.rs:342-404`): Tokio `UdpSocket` with concurrency semaphore (`Semaphore::new(config.concurrency)`). Each worker acquires a permit, creates a new socket per send when `random_source_port` is true, sends one datagram, and drops the permit. Broadcast enabled (`udp.rs:414`).

2. **Spoofed mode** (`run_udp_flood_spoofed`, `udp.rs:158-291`, unix only): Raw socket via `libc::socket(PF_INET, SOCK_RAW, IPPROTO_RAW)` with `IP_HDRINCL`. Manually constructs IPv4 + UDP headers (`raw_udp::build_udp_packet`). Random source IP from spoof range. **IPv4 only** — returns error for IPv6 targets (`udp.rs:166-173`). Uses `Arc<Mutex<RawFd>>` for thread-safe raw socket access with poisoned-mutex recovery (`udp.rs:248-251`).

Spoofed IP range formats:
- CIDR: `"192.168.1.0/24"` — random within host bits (`udp.rs:296-314`).
- Dash-range: `"100000000-100001000"` — integer range (`udp.rs:317-325`).

### HTTP Flood (`http.rs`)

Application-layer GET flood using `reqwest` (`http.rs:13-121`):

- Target URL auto-detects scheme (port 443 → HTTPS, else HTTP) (`http.rs:14`).
- Optional random path appended (`payload_size` controls length) (`http.rs:15-25`).
- Randomized headers: `User-Agent` (3 Chrome variants), `X-Forwarded-For`, `X-Real-IP`, `Cache-Control: no-cache` (`http.rs:84-90`).
- **Proxy pool support**: Loads proxies from file via `ProxyManager`, creates one `reqwest::Client` per healthy proxy (SOCKS4/5, HTTP, HTTPS, Tor). TLS verification disabled for proxy health checks (`http.rs:27-37,123-154`).
- Progress bar via `indicatif` (`http.rs:43-51`).
- Worker count = `min(concurrency, total_requests)` (`http.rs:57-60`). Each worker loop increments an atomic counter and breaks when `current >= total_requests` (`http.rs:74-77`).

### ICMP Flood (`icmp.rs`)

Raw Ethernet ICMP Echo Request flood via `pnet` (`icmp.rs:31-100`):

- **IPv4**: Ethernet (14) + IPv4 (20) + ICMP Echo Request (8 + payload) (`icmp.rs:148-192`).
- **IPv6**: Ethernet (14) + IPv6 (40) + ICMPv6 Echo Request (8 + payload) (`icmp.rs:194-237`).
- ICMP payload filled with random bytes (minimum 56 bytes) (`icmp.rs:28,44`).
- Identifier field randomized or incremented per packet (`icmp.rs:53-60`).
- Separate checksum implementations for ICMP (IPv4, `icmp.rs:103-117`) and ICMPv6 (with pseudo-header, `icmp.rs:120-145`).
- **Unix only** — requires raw Ethernet access.

## Metrics (`metrics.rs`)

### `StressMetrics`

Thread-safe counters using `AtomicU64` (`metrics.rs:5-75`):

| Method | Description |
|--------|-------------|
| `new()` | Default with all counters at 0 (`metrics.rs:14-16`) |
| `start()` | Records start time via `OnceLock`, warns if called twice (`metrics.rs:18-21`) |
| `record_packet(size)` | Increments packet count and byte count (`metrics.rs:24-27`) |
| `record_error()` | Increments error count (`metrics.rs:29-31`) |
| `packets_sent()` | Returns current packet count (`metrics.rs:33-35`) |
| `bytes_sent()` | Returns current byte count (`metrics.rs:37-39`) |
| `errors()` | Returns current error count (`metrics.rs:41-43`) |
| `elapsed()` | Returns duration since `start()` (`metrics.rs:45-50`) |
| `to_stats()` | Converts to `StressStats` snapshot (`metrics.rs:52-59`) |

Implements `Clone` — copies current atomic values into new instances (`metrics.rs:62-75`).

### `StressStats`

Serializable result snapshot (`metrics.rs:77-108`):

| Method | Description |
|--------|-------------|
| `avg_rate_pps()` | `packets_sent * 1000 / duration_ms` (0 if duration is 0) (`metrics.rs:86-91`) |
| `avg_bandwidth_mbps()` | `bytes_sent * 8 / seconds / 1_000_000` (0.0 if duration is 0) (`metrics.rs:93-100`) |
| `merge(other)` | Combines two stats (max duration, sum packets/bytes/errors) (`metrics.rs:102-107`) |

## Authorization Model

### `StressScope` (TOML config)

Loaded from `{config_dir}/stress.toml` (falls back to `stress.toml` in CWD) (`authorization.rs:58-83`). Fields (`authorization.rs:7-26`):

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `allow_stress_test` | `bool` | `false` | Master enable gate |
| `max_rate_pps` | `Option<u64>` | `Some(100_000)` | Rate cap (`None` = unlimited) |
| `max_duration_secs` | `Option<u64>` | `Some(300)` | Duration cap (`None` = unlimited) |
| `allowed_stress_types` | `Option<Vec<String>>` | `None` | Type allowlist (`None` = all) |
| `require_confirmation` | `bool` | `true` | Require interactive `"yes"` |
| `warning_message` | `Option<String>` | `None` | Custom warning text |

### `StressAuthorization`

Constructed via `from_scope()` (`authorization.rs:48-56`) which loads both the main scope file and `stress.toml`.

Verification chain (called in `StressTest::new()`, `mod.rs:95-97`):

1. **`verify_target(target)`** (`authorization.rs:85-111`) — Checks `scope.is_target_allowed(target)` AND `stress_scope.allow_stress_test`.
2. **`verify_rate(rate_pps)`** (`authorization.rs:113-123`) — Enforces `max_rate_pps` if set.
3. **`verify_duration(duration_secs)`** (`authorization.rs:125-135`) — Enforces `max_duration_secs` if set.

### `create_example_stress_config()`

Generates a sample TOML string for documentation/reference (`authorization.rs:256-272`).

## Utilities (`utils.rs`)

| Function | Signature | Description |
|----------|-----------|-------------|
| `resolve_target` | `async (target: &str) -> Result<IpAddr>` | Parses IP literal or does DNS lookup via `tokio::net::lookup_host` (`utils.rs:14-25`) |
| `get_network_interface` | `() -> Result<NetworkInterface>` | Finds first up, non-loopback interface with IPs (via `pnet::datalink`) (`utils.rs:28-35`) |
| `create_channel` | `(interface, label) -> Result<(tx, rx)>` | Opens raw Ethernet channel; checks privilege first (`utils.rs:38-56`) |
| `get_local_ip` | `(interface) -> Result<Ipv4Addr>` | First IPv4 address on interface (`utils.rs:59-68`) |
| `get_local_ip_v6` | `(interface) -> Result<Ipv6Addr>` | First IPv6 address on interface (`utils.rs:71-80`) |
| `get_spoofed_source` | `(range: &Option<String>) -> Result<Ipv4Addr>` | Random IPv4 from CIDR/dash-range, or fully random (`utils.rs:83-125`) |
| `get_spoofed_source_v6` | `(range: &Option<String>) -> Result<Ipv6Addr>` | Random IPv6 from CIDR/dash-range, or `fe80::/64` link-local (`utils.rs:128-199`) |
| `generate_payload` | `(size: usize) -> Vec<u8>` | Random byte buffer (`utils.rs:202-207`) |

### IPv6 support

Full IPv6 support in SYN and ICMP flood paths:
- `get_local_ip_v6()` for source address selection (`utils.rs:71-80`).
- `get_spoofed_source_v6()` for spoofed IPv6 (CIDR and dash-range parsing with per-segment randomization) (`utils.rs:128-199`).
- `build_syn_packet_v6()` / `build_icmp_packet_v6()` construct proper IPv6 headers with ICMPv6 checksums.
- UDP spoofed mode is IPv4-only (raw socket `PF_INET`) (`udp.rs:166-173`).

## Warning System (`warning.rs`)

### `display_warning(config)`

Prints to stderr (`warning.rs:7-63`):
1. Legal warning banner (CFAA, Computer Misuse Act references).
2. Test configuration summary (target, type, rate, duration, concurrency, spoof/proxy status).
3. Extra warning if IP spoofing is enabled (`warning.rs:56-60`).

### `require_confirmation()`

Prompts `"Type 'yes' to proceed"` on stdin. Returns `Ok(false)` (test cancelled) if input is not `"yes"` (`warning.rs:65-79`).

### `display_completion(stats)`

Prints final stats to stderr: duration, packets, bytes, average rate, errors (`warning.rs:81-89`).

## Safety & Authorization

Multiple layers prevent accidental or unauthorized use:

1. **Compile-time:** Flood engines gated behind `stress-testing` feature. Without it, `run_inner()` returns an explicit error (`mod.rs:157-164`).
2. **Scope file:** `allow_stress_test` defaults to `false`. Without it, `verify_target()` rejects all targets (`authorization.rs:97-103`).
3. **Rate cap:** `max_rate_pps` defaults to 100,000 pps. Requests exceeding this are rejected (`authorization.rs:113-123`).
4. **Duration cap:** `max_duration_secs` defaults to 300s. Requests exceeding this are rejected (`authorization.rs:125-135`).
5. **Interactive confirmation:** `require_confirmation` defaults to `true`. The `run()` method prompts for `"yes"` on stdin (`mod.rs:110-112`).
6. **Warning banner:** Legal warning with CFAA/Computer Misuse Act references displayed before every run (`warning.rs:8-24`).
7. **Privilege check:** `crate::utils::privilege::check_privileged()` called before raw socket creation (`utils.rs:45`, `udp.rs:181`).
8. **Platform gates:** SYN/ICMP require unix; UDP spoofing requires unix.

## Probe Risk & Defense-Lab Integration

The stress module is tagged with `ProbeRisk::Stress` (risk level 4) and `ProbeIntent::Stress` in the shared probe classification system (`crates/eggsec/src/probe.rs`). These are used by:

- **Defense-lab profiles** to include/exclude stress probes based on risk budgets (`architecture/defense_lab.md`).
- **Pipeline stage scheduling** to enforce feature gates and scope requirements.
- **Policy evaluator** via `ProbeRisk::to_operation_risk()` → `OperationRisk::StressTest` → `allow_stress_testing` gate (`architecture/probe.md:40-47`).

## Integration Points

- **CLI/TUI**: `StressTest::run()` for interactive use with confirmation.
- **REST/Agent/MCP**: `StressTest::run_non_interactive()` bypasses stdin.
- **Proxy pool**: HTTP flood integrates with `crate::proxy::ProxyManager` for distributed traffic.
- **Privilege checks**: `crate::utils::privilege::check_privileged()` before raw socket operations.
- **Scope enforcement**: Uses `crate::config::{load_scope, Scope}` for target allowlisting.
- **Dispatch**: `StressTest` task kind dispatched via `runtime_bridge` from daemon/runtime surfaces (`architecture/runtime_bridge.md`).

## Testing

Tests are gated behind `#[cfg(all(test, feature = "stress-testing"))]` (`mod.rs:185-216`). Unit tests verify:
- `run_non_interactive()` bypasses confirmation (`mod.rs:189-215`).
- Authorization rate/duration caps (`authorization.rs:174-253`).
- Metrics counters and stats computation (`metrics.rs:110-221`).

## Invariants & Gotchas

1. **⚠ TCP flood stub:** `StressType::Tcp` is declared but returns `Err` at runtime (`mod.rs:139-144`). This is not a compile-time error.
2. **IPv4-only spoofed UDP:** Spoofed mode uses `PF_INET` raw socket — IPv6 targets return an error (`udp.rs:166-173`).
3. **Root required:** SYN, ICMP, and spoofed UDP require root/CAP_NET_RAW. `check_privileged()` is called before raw socket creation.
4. **No timeouts on spawned tasks:** Flood worker tasks in `udp.rs:228,372` and `http.rs:72` are spawned without `tokio::time::timeout` wrappers. They are bounded by the outer duration loop and sleep interval, but a hung task (e.g., blocked on socket) would not be killed. This violates the AGENTS.md invariant that all spawned tokio tasks need timeout wrappers.
5. **Clone semantics:** `StressMetrics::clone()` copies current atomic values into new instances — cloned metrics are independent snapshots, not shared references (`metrics.rs:62-75`).

## See Also

- [overview.md](overview.md) — system-wide module index
- [probe.md](probe.md) — shared probe intent/risk vocabulary
- [defense_lab.md](defense_lab.md) — defense-lab profiles and risk budgets
- [networking.md](networking.md) — packet capture, crafting, and low-level network access

*Last verified against source: 2026-08-25*
