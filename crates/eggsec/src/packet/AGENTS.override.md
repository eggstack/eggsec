# Packet Module Override

Specialized guidance for the packet capture/crafting module.

## Key Files

| File | Purpose |
|------|---------|
| `crates/eggsec/src/packet/mod.rs` | Module entry point |
| `crates/eggsec/src/packet/capture.rs` | Packet capture using pcap |
| `crates/eggsec/src/packet/craft.rs` | Packet crafting for raw sockets |
| `crates/eggsec/src/packet/parse_impl.rs` | Packet parsing orchestration |
| `crates/eggsec/src/packet/types.rs` | Packet type definitions |
| `crates/eggsec/src/packet/traceroute.rs` | Traceroute implementation |
| `crates/eggsec/src/packet/validation.rs` | Packet validation |
| `crates/eggsec/src/packet/hexdump.rs` | Hexdump utility |

## Module Characteristics

- Uses `pnet` and `pnet_packet` for raw sockets
- Requires `stress-testing` feature for raw sockets and IP spoofing
- Requires `packet-inspection` feature for packet capture

## Known Issues

**None currently pending.**

## Testing

```bash
cargo test --lib -p eggsec packet::
cargo clippy --lib -p eggsec --features stress-testing
```