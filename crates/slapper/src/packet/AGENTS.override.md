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

## HIGH Priority Issue (Pending Fix)

**PcapWriter Write Result Silently Dropped at `capture.rs:209`:**

```rust
let _ = writer.write_packet(&packet);
```

The PcapWriter `write_packet` result is silently dropped. While PcapWriter itself handles errors properly, the caller ignores the result which could hide write failures.

**Fix required**: Log a warning when write_packet fails instead of silently dropping the result:

```rust
if let Err(e) = writer.write_packet(&packet) {
    tracing::warn!("Failed to write packet to pcap: {}", e);
}
```

## Silent Error Suppression Pattern

The `let _ =` pattern is used at `capture.rs:209` for pcap write errors. This should be replaced with explicit error logging. See "Key Patterns" in AGENTS.md for proper error handling.

## Testing

```bash
cargo test --lib -p slapper packet::
cargo clippy --lib -p slapper --features stress-testing
```