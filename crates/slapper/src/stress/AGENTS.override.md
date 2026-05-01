# Stress Module Override

Specialized guidance for the stress testing module.

## raw_udp Integration

The `raw_udp` module in `stress/udp.rs:20-117` is integrated:
- `run_udp_flood()` calls `run_udp_flood_spoofed()`
- Uses `raw_udp::build_udp_packet` when IP spoofing is enabled on Unix
- Feature-gated behind `stress-testing`