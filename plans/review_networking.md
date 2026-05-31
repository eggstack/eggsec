# Networking Architecture Review
**Document:** architecture/networking.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 71

## Verified Claims
- EthernetFrame::parse(): Verified at `parse_impl.rs:7-38`
- IpPacket::parse_ipv4(): Verified at `parse_impl.rs:40-100`
- ParsedPacket::parse() orchestration: Referenced in `capture.rs:263` (uses `ParsedPacket::parse(data)`)
- Packet capture with pnet: Verified at `capture.rs:12` (`use pnet::datalink`)
- Hexdump: Verified at `packet/mod.rs:11` (`pub use hexdump::{hexdump, hexdump_with_offset}`)
- Packet crafting (TCP/UDP/ICMP): Verified at `craft.rs` (PacketBuilder struct with ethernet, ipv4, ipv6, tcp, udp, icmp builders)
- Validation: Verified at `packet/mod.rs:7` (`pub mod validation`)
- Traceroute: Verified at `traceroute.rs:12-42` (TracerouteConfig with UDP default, ICMP disabled)
- Stress module: Verified at `stress/` directory (syn.rs, udp.rs, icmp.rs, http.rs)
- Stress-testing feature flag: Verified at `traceroute.rs:9` (`#[cfg(all(feature = "stress-testing", unix))]`)
- DNS parsing in parse_impl.rs: Verified at `parse_impl.rs` (DnsRecord type in types.rs)
- TLS parsing in parse_impl.rs: Verified at `parse_impl.rs` (TlsHandshake type in types.rs)
- HTTP parsing in parse_impl.rs: Verified at `parse_impl.rs` (HttpRequest/HttpResponse types in types.rs)

## Discrepancies
- [File location wrong]: Document says "Packet Parsing (`parse_impl.rs`)" at line 7, but types are defined in `types.rs` and `parse_impl.rs` contains the `impl` blocks. The doc conflates type definitions with implementations. Should clarify that types live in `types.rs` and parsing logic in `parse_impl.rs`.
- [Missing detail]: Document says "BPF-style filters" for capture filtering (`capture.rs:26`), but actual filtering at `capture.rs:276-306` is a custom implementation (TCP/UDP/ICMP/port matching), not true BPF. The filter syntax is limited to `tcp`, `udp`, `icmp`, `ip`, and `port N`.
- [Missing detail]: Document doesn't mention `CaptureBuilder` pattern at `capture.rs:455-510`.
- [Missing detail]: Document doesn't mention `PcapWriter` at `capture.rs:14-74` for writing pcap files.
- [Missing detail]: Document doesn't mention `PacketInfo` struct at `mod.rs:26-34` or `PacketInfo::summary()` at `mod.rs:37-91`.
- [Missing detail]: Document doesn't mention `cli.rs` (feature-gated behind `packet-inspection`) at `packet/mod.rs:21`.
- [Bug fixes section]: The bug fixes listed are accurate and match the code. The `parse_impl.rs:702-717` fix for reading TCP ports from header instead of payload is referenced but the actual line numbers may have shifted.
- [Missing feature gate]: Document doesn't mention that `PacketCapture::start()` requires both `packet-inspection` AND `unix` features (`capture.rs:147`).

## Bugs Found
- [No bugs found]: The networking module appears well-structured.

## Improvement Opportunities
- [Documentation gap]: Clarify that types are in `types.rs` and parsing impls are in `parse_impl.rs`. (priority: medium)
- [Documentation gap]: Correct "BPF-style filters" to describe the actual custom filter implementation. (priority: medium)
- [Documentation gap]: Add CaptureBuilder, PcapWriter, and PacketInfo to the key components. (priority: medium)
- [Documentation gap]: Mention the `packet-inspection` + `unix` feature gate requirements. (priority: medium)
- [Documentation gap]: Add mention of the `cli.rs` module for packet-inspection CLI. (priority: low)

## Stale Items
- [Bug fixes]: The "Recent Bug Fixes (2026-05-28)" section references specific line numbers that may have shifted due to subsequent edits. The fixes themselves are verified as present in the code.
- [None other]: No stale architectural information found.
