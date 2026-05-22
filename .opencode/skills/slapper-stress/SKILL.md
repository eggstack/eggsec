# Slapper Stress Skill

Network stress testing and DoS simulation module workflows and patterns.

## Stress vs Load Testing

Note: The `stress` module is for network-level stress testing (UDP floods, IP spoofing),
while the `loadtest` module handles HTTP load/performance testing. See the `slapper-agent/http_load_testing.md` skill for HTTP load testing guidance.

## Key Types and Patterns

### raw_udp Integration
The `raw_udp` module in `stress/udp.rs:20-117` is integrated:
- `run_udp_flood()` calls `run_udp_flood_spoofed()`
- Uses `raw_udp::build_udp_packet` when IP spoofing is enabled on Unix
- Feature-gated behind `stress-testing`

### Stress Types
- `StressType` enum: `Syn`, `Udp`, `Http`, `Tcp`, `Icmp`
- `StressConfig` struct: target, port, rate_pps, duration_secs, concurrency, spoof_source
- `StressMetrics`: atomic counters for packets_sent, bytes_sent, errors
- `StressStats`: aggregated results from stress test runs

### Authorization
- `StressAuthorization::verify_target()` - checks scope for target allowance
- `StressAuthorization::verify_rate()` - enforces max_rate_pps
- `StressAuthorization::verify_duration()` - enforces max_duration_secs
- `StressScope` config loaded from `stress.toml` in config dir

### Bug Fixes (2026-05-22)

| Issue | Fix |
|-------|-----|
| `icmp.rs:119` | IPv4 flags field properly set to `0x40` (Don't Fragment) in `build_icmp_packet_v4()` |
| `udp.rs:244` | Mutex poisoning handled in `run_udp_flood_spoofed()` with `into_inner()` instead of `unwrap()` |
| `syn.rs:237-260` | IPv4 spoof range now supports both CIDR notation (`10.0.0.0/24`) and range notation (`10.0.0.1-10.0.0.254`) |
| `syn.rs:263-306` | IPv6 spoof range now supports both CIDR notation and range notation |
| `icmp.rs:244-267` | IPv4 spoof range now supports both CIDR and range notation (consistent with syn.rs) |
| `icmp.rs:270-313` | IPv6 spoof range now supports both CIDR and range notation (consistent with syn.rs) |

## Feature Requirements
- All stress tests require `stress-testing` feature flag
- Raw socket operations (SYN, UDP spoofed, ICMP) require Unix platform
- IP spoofing requires `CAP_NET_RAW` or root privileges

## Testing

### Running Stress Tests
```bash
cargo test --lib -p slapper stress::
```

### Writing Tests
Follow existing test patterns in `stress/` modules, testing flood logic and raw socket features (gated behind `stress-testing`).

## Common Tasks

### Adding a New Stress Test Type
1. Add new variant to `StressType` enum in `stress/mod.rs`
2. Implement test logic in new file under `stress/` (e.g., `syn.rs`, `udp.rs`)
3. Gate raw socket/IP spoofing features behind `stress-testing` feature flag
4. Add case to `run_inner()` match statement
5. Use `raw_udp` modules for packet building when applicable
6. Add tests for new stress test type

## Resources
- `crates/slapper/src/stress/AGENTS.override.md` - Detailed stress patterns
- `AGENTS.md` - General project guidelines
- `architecture/networking.md` - Networking module design
