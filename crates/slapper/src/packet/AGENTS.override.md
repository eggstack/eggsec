# Packet Module Override

Specialized guidance for the packet capture/crafting module.

## Key Files

| File | Purpose |
|------|---------|
| `crates/slapper/src/packet/mod.rs` | Module entry point |
| `crates/slapper/src/packet/capture.rs` | Packet capture using pcap |
| `crates/slapper/src/packet/craft.rs` | Packet crafting for raw sockets |
| `crates/slapper/src/packet/parse_impl.rs` | Packet parsing orchestration |
| `crates/slapper/src/packet/types.rs` | Packet type definitions |
| `crates/slapper/src/packet/traceroute.rs` | Traceroute implementation |
| `crates/slapper/src/packet/validation.rs` | Packet validation |
| `crates/slapper/src/packet/hexdump.rs` | Hexdump utility |

## Module Characteristics

- Uses `pnet` and `pnet_packet` for raw sockets
- Requires `stress-testing` feature for raw sockets and IP spoofing
- Requires `packet-inspection` feature for packet capture

## Known Issues

**None currently pending.**

## Testing

```bash
cargo test --lib -p slapper packet::
cargo clippy --lib -p slapper --features stress-testing
```