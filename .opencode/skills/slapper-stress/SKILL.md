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

## Testing

### Running Stress Tests
```bash
cargo test --lib -p slapper stress::
```

### Writing Tests
Follow existing test patterns in `stress/` modules, testing flood logic and raw socket features (gated behind `stress-testing`).

## Common Tasks

### Adding a New Stress Test Type
1. Implement test logic in `stress/`
2. Gate raw socket/IP spoofing features behind `stress-testing` feature flag
3. Use `raw_udp` modules for packet building when applicable
4. Add tests for new stress test type

## Resources
- `crates/slapper/src/stress/AGENTS.override.md` - Detailed stress patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
