# Networking Architecture Review

**Document:** architecture/networking.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims

- [Packet Parsing]: `EthernetFrame::parse()` verified at `crates/slapper/src/packet/parse_impl.rs:8`
- [IP Parsing]: `IpPacket::parse()` dispatches to `parse_ipv4`/`parse_ipv6` verified at `crates/slapper/src/packet/types.rs:58-68`
- [TCP Parsing]: `TcpHeader::parse()` with options parsing verified at `crates/slapper/src/packet/parse_impl.rs:220`
- [UDP Parsing]: `UdpHeader::parse()` verified at `crates/slapper/src/packet/parse_impl.rs:335`
- [ICMP Parsing]: `IcmpHeader::parse()` verified at `crates/slapper/src/packet/parse_impl.rs:358`
- [DNS Parsing]: `DnsRecord::parse()` with compression support verified at `crates/slapper/src/packet/parse_impl.rs:500`
- [TLS Parsing]: `TlsHandshake::parse()` with SNI extraction verified at `crates/slapper/src/packet/parse_impl.rs:598`
- [HTTP Parsing]: `HttpRequest::parse()` and `HttpResponse::parse()` verified at `crates/slapper/src/packet/parse_impl.rs:394` and `:447`
- [ParsedPacket::parse()]: Full L2-L7 parsing chain verified at `crates/slapper/src/packet/parse_impl.rs:758`
- [Hexdump]: Pretty-printed hex views verified at `crates/slapper/src/packet/hexdump.rs:5`
- [Packet Crafting]: TCP/UDP/ICMP builders with validation verified at `crates/slapper/src/packet/craft.rs`
- [Validation]: Packet validation (`PacketValidationError`) verified at `crates/slapper/src/packet/craft.rs:68-94`
- [Traceroute]: Multi-protocol traceroute with UDP default, ICMP disabled verified at `crates/slapper/src/packet/traceroute.rs:122-127`
- [ICMP disabled]: `TracerouteError::Unsupported` for ICMP mode verified at `crates/slapper/src/packet/traceroute.rs:124-126`
- [Stress Testing]: SYN/UDP/HTTP flood modules exist in `crates/slapper/src/stress/` directory
- [Feature flags]: `stress-testing` and `packet-inspection` feature gates verified via `#[cfg(all(feature = "packet-inspection", unix))]` in capture.rs:11
- [BPF-style filters]: Simplified filter matching (tcp/udp/icmp/port) verified at `crates/slapper/src/packet/capture.rs:276-307`
- [DNS bounds check]: Malformed response protection verified at `crates/slapper/src/packet/parse_impl.rs:537` (`if new_offset >= data.len() || new_offset + 4 > data.len()`)
- [parse_app_layer ports]: Uses `tcp.src_port`/`tcp.dst_port` from header verified at `crates/slapper/src/packet/parse_impl.rs:839-844`

## Discrepancies

- [BPF-style filters]: Documented as "BPF-style filters" but implementation is a simplified protocol/port matcher, not full BPF. The filter supports `tcp`, `udp`, `icmp`, `ip`, and `port <N>` patterns only (`capture.rs:276-307`). Minor inaccuracy in terminology.
- [Bug fix line numbers]: The "Recent Bug Fixes" table references line numbers at time of fix, not current line numbers. For example, `craft.rs:186-187` (IPv4 fragmentation flags) is now at `craft.rs:328-329`. `parse_impl.rs:644-651` (IP payload re-extraction) was in a now-removed section. These are historical references, not current locations.

## Bugs Found

- None found in the documented architecture.

## Improvement Opportunities

- [Filter documentation]: The filter section could document the exact supported filter syntax (tcp/udp/icmp/ip/port N) rather than implying full BPF support. (priority: low)
- [Bug fix table]: The "Recent Bug Fixes" table should either be removed (since fixes are merged) or updated with current line numbers for reference value. (priority: low)

## Stale Items

- [Bug fix table line numbers]: Line numbers in the "Recent Bug Fixes" table are from 2026-05-28 and reference line numbers that have shifted due to subsequent code changes. The fixes themselves are still in place but line references are stale. Recommended action: Remove the table or convert to a changelog format without line numbers.
