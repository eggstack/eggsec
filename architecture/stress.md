# Stress Testing Module

## Overview

The stress testing module provides load generation and denial-of-service simulation capabilities for authorized security testing. It sends high volumes of traffic (SYN, UDP, HTTP, ICMP) to test system resilience. Requires the `stress-testing` feature flag, and raw socket privileges for SYN/ICMP/spoofed UDP.

**Feature gate:** `stress-testing` (in `Cargo.toml`)

## Module Structure

| File | Lines | Feature-gated | Purpose |
|------|-------|---------------|---------|
| `mod.rs` | 216 | no | Orchestrator: `StressTest`, `StressType`, `StressConfig`, `StressResult`, `StressConfigSummary` |
| `syn.rs` | 284 | `stress-testing` + unix | SYN flood via raw Ethernet frames (IPv4 + IPv6) |
| `udp.rs` | 427 | `stress-testing` | UDP flood: standard socket + raw socket spoofed mode |
| `http.rs` | 205 | `stress-testing` | HTTP GET flood with proxy pool support |
| `icmp.rs` | 247 | `stress-testing` + unix | ICMP echo request flood (IPv4 + IPv6) via raw Ethernet |
| `metrics.rs` | 222 | always compiled | Thread-safe atomic counters: `StressMetrics`, `StressStats` |
| `authorization.rs` | 272 | always compiled | Scope enforcement, rate/duration caps, TOML config |
| `warning.rs` | 89 | always compiled | Legal warning banner, interactive confirmation prompt |
| `utils.rs` | 207 | `stress-testing` | DNS resolution, interface detection, channel creation, spoofed IPs, payload generation |

### Compilation model

- `metrics`, `authorization`, `warning` compile unconditionally (needed for config/validation even when flood types are off).
- `syn`, `icmp` require both `stress-testing` and unix (raw Ethernet via pnet).
- `udp`, `http`, `utils` require `stress-testing` only.
- When `stress-testing` is off, `run_inner()` returns an error message directing the user to enable the feature.

## Key Types

### `StressType` enum

Five variants, serializable via serde:

```rust
pub enum StressType {
    Syn,   // "SYN flood"
    Udp,   // "UDP flood"
    Http,  // "HTTP flood"
    Tcp,   // "TCP flood" — not yet implemented
    Icmp,  // "ICMP flood"
}
```

`Display` impl produces human-readable names. `Tcp` returns a runtime error in `run_inner()`.

### `StressConfig`

Runtime configuration for a test run:

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

Main entry point. Created via `StressTest::new(config)` which:

1. Loads `StressAuthorization::from_scope()` (TOML config + scope file).
2. Verifies target is in scope (`verify_target`).
3. Verifies rate is within limits (`verify_rate`).
4. Verifies duration is within limits (`verify_duration`).
5. Initializes `StressMetrics` (when feature enabled).

Two run methods:

- **`run()`** — Interactive: displays warning banner, prompts for `"yes"` confirmation (if `require_confirmation` is set in scope), then dispatches to the flood implementation.
- **`run_non_interactive()`** — Bypasses stdin confirmation (used by non-interactive surfaces like REST/agent).

Both call `run_inner()` which dispatches based on `StressType`:

```rust
Syn  → syn::run_syn_flood()
Udp  → udp::run_udp_flood()
Icmp → icmp::run_icmp_flood()
Http → http::run_http_flood()
Tcp  → returns error ("not yet implemented")
```

### `StressResult` and `StressConfigSummary`

Serializable output types for report generation:

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

- **IPv4**: Ethernet (14) + IPv4 (20) + TCP (20) = 54-byte frame.
- **IPv6**: Ethernet (14) + IPv6 (40) + TCP (20) = 74-byte frame.
- Sequence numbers increment from 1000. Source port randomized in `[40000, 60000)` or incremented.
- Manual TCP checksum computation over pseudo-header (IPv4 and IPv6 variants).
- Source IP: local interface IP (normal) or random from spoof range (spoofed).
- Destination MAC set to zeros (layer-2 forwarding).
- Rate-controlled via `tokio::time::sleep(interval)` where `interval = 1s / rate_pps`.
- **Unix only** — uses `pnet::datalink` for raw Ethernet channel.

### UDP Flood (`udp.rs`)

Two modes:

1. **Standard mode** (`run_udp_flood_standard`): Tokio `UdpSocket` with concurrency semaphore. Each worker sends one datagram, creates a new socket per send when `random_source_port` is true. Broadcast enabled.

2. **Spoofed mode** (`run_udp_flood_spoofed`, unix only): Raw socket via `libc::socket(PF_INET, SOCK_RAW, IPPROTO_RAW)` with `IP_HDRINCL`. Manually constructs IPv4 + UDP headers. Random source IP from spoof range. **IPv4 only** — returns error for IPv6 targets.

Spoofed IP range formats:
- CIDR: `"192.168.1.0/24"` — random within host bits.
- Dash-range: `"100000000-100001000"` — integer range.

### HTTP Flood (`http.rs`)

Application-layer GET flood using `reqwest`:

- Target URL auto-detects scheme (port 443 → HTTPS, else HTTP).
- Optional random path appended (`payload_size` controls length).
- Randomized headers: `User-Agent` (3 Chrome variants), `X-Forwarded-For`, `X-Real-IP`, `Cache-Control: no-cache`.
- **Proxy pool support**: Loads proxies from file via `ProxyManager`, creates one `reqwest::Client` per healthy proxy (SOCKS4/5, HTTP, HTTPS, Tor). TLS verification disabled for proxy health checks.
- Progress bar via `indicatif`.
- Worker count = `min(concurrency, total_requests)`. Each worker loop increments an atomic counter.

### ICMP Flood (`icmp.rs`)

Raw Ethernet ICMP Echo Request flood via `pnet`:

- **IPv4**: Ethernet (14) + IPv4 (20) + ICMP Echo Request (8 + payload).
- **IPv6**: Ethernet (14) + IPv6 (40) + ICMPv6 Echo Request (8 + payload).
- ICMP payload filled with random bytes (minimum 56 bytes).
- Identifier field randomized or incremented per packet.
- Separate checksum implementations for ICMP (IPv4) and ICMPv6 (with pseudo-header).
- **Unix only** — requires raw Ethernet access.

## Metrics (`metrics.rs`)

### `StressMetrics`

Thread-safe counters using `AtomicU64`:

| Method | Description |
|--------|-------------|
| `start()` | Records start time (`OnceLock`, warns if called twice) |
| `record_packet(size)` | Increments packet count and byte count |
| `record_error()` | Increments error count |
| `to_stats()` | Converts to `StressStats` snapshot |

Implements `Clone` (copies current atomic values).

### `StressStats`

Serializable result snapshot:

| Method | Description |
|--------|-------------|
| `avg_rate_pps()` | `packets_sent * 1000 / duration_ms` |
| `avg_bandwidth_mbps()` | `bytes_sent * 8 / seconds / 1_000_000` |
| `merge(other)` | Combines two stats (max duration, sum packets/bytes/errors) |

## Authorization Model

### `StressScope` (TOML config)

Loaded from `{config_dir}/stress.toml` (falls back to `stress.toml` in CWD). Fields:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `allow_stress_test` | `bool` | `false` | Master enable gate |
| `max_rate_pps` | `Option<u64>` | `100_000` | Rate cap (`None` = unlimited) |
| `max_duration_secs` | `Option<u64>` | `300` | Duration cap (`None` = unlimited) |
| `allowed_stress_types` | `Option<Vec<String>>` | `None` | Type allowlist (`None` = all) |
| `require_confirmation` | `bool` | `true` | Require interactive `"yes"` |
| `warning_message` | `Option<String>` | `None` | Custom warning text |

### `StressAuthorization`

Constructed via `from_scope()` which loads both the main scope file and `stress.toml`.

Verification chain (called in `StressTest::new()`):

1. **`verify_target(target)`** — Checks `scope.is_target_allowed(target)` AND `stress_scope.allow_stress_test`.
2. **`verify_rate(rate_pps)`** — Enforces `max_rate_pps` if set.
3. **`verify_duration(duration_secs)`** — Enforces `max_duration_secs` if set.

### `create_example_stress_config()`

Generates a sample TOML string for documentation/reference.

## Utilities (`utils.rs`)

| Function | Signature | Description |
|----------|-----------|-------------|
| `resolve_target` | `async (target: &str) -> Result<IpAddr>` | Parses IP literal or does DNS lookup via `tokio::net::lookup_host` |
| `get_network_interface` | `() -> Result<NetworkInterface>` | Finds first up, non-loopback interface with IPs (via `pnet::datalink`) |
| `create_channel` | `(interface, label) -> Result<(tx, rx)>` | Opens raw Ethernet channel; checks privilege first |
| `get_local_ip` | `(interface) -> Result<Ipv4Addr>` | First IPv4 address on interface |
| `get_local_ip_v6` | `(interface) -> Result<Ipv6Addr>` | First IPv6 address on interface |
| `get_spoofed_source` | `(range: &Option<String>) -> Result<Ipv4Addr>` | Random IPv4 from CIDR/dash-range, or fully random |
| `get_spoofed_source_v6` | `(range: &Option<String>) -> Result<Ipv6Addr>` | Random IPv6 from CIDR/dash-range, or `fe80::/64` link-local |
| `generate_payload` | `(size: usize) -> Vec<u8>` | Random byte buffer |

### IPv6 support

Full IPv6 support in SYN and ICMP flood paths:
- `get_local_ip_v6()` for source address selection.
- `get_spoofed_source_v6()` for spoofed IPv6 (CIDR and dash-range parsing with per-segment randomization).
- `build_syn_packet_v6()` / `build_icmp_packet_v6()` construct proper IPv6 headers with ICMPv6 checksums.
- UDP spoofed mode is IPv4-only (raw socket `PF_INET`).

## Warning System (`warning.rs`)

### `display_warning(config)`

Prints to stderr:
1. Legal warning banner (CFAA, Computer Misuse Act references).
2. Test configuration summary (target, type, rate, duration, concurrency, spoof/proxy status).
3. Extra warning if IP spoofing is enabled.

### `require_confirmation()`

Prompts `"Type 'yes' to proceed"` on stdin. Returns `Ok(false)` (test cancelled) if input is not `"yes"`.

### `display_completion(stats)`

Prints final stats to stderr: duration, packets, bytes, average rate, errors.

## Error Handling

All flood functions return `Result<StressStats>`. Common error cases:
- Feature not enabled: explicit message to add `--features stress-testing`.
- Platform restriction: SYN/ICMP require unix; UDP spoofing requires unix.
- Privilege check failure: `crate::utils::privilege::check_privileged()` called before raw socket creation.
- Scope violation: `EggsecError::ScopeViolation` for unauthorized targets/rates/durations.
- DNS resolution failure: `EggsecError::Runtime` with descriptive message.

## Integration Points

- **CLI/TUI**: `StressTest::run()` for interactive use with confirmation.
- **REST/Agent/MCP**: `StressTest::run_non_interactive()` bypasses stdin.
- **Proxy pool**: HTTP flood integrates with `crate::proxy::ProxyManager` for distributed traffic.
- **Privilege checks**: `crate::utils::privilege::check_privileged()` before raw socket operations.
- **Scope enforcement**: Uses `crate::config::{load_scope, Scope}` for target allowlisting.
