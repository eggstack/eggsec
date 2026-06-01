# Networking & Packets Module

The Networking module provides low-level access to the network stack for tasks like packet capture, custom packet crafting, and stress testing.

## Core Capabilities (`crates/slapper/src/packet/` & `crates/slapper/src/stress/`)

### Packet Parsing (`parse_impl.rs`)

Deep packet inspection for various protocols:

- **Ethernet**: `EthernetFrame::parse()` - L2 frame parsing
- **IP**: `IpPacket::parse()` - dispatches to IPv4/IPv6 specific parsers
- **TCP**: `TcpHeader::parse()` - transport layer with options parsing
- **UDP**: `UdpHeader::parse()` - simple datagram parsing
- **ICMP**: `IcmpHeader::parse()` - control message parsing
- **DNS**: `DnsRecord::parse()` - full DNS message parsing with compression
- **TLS**: `TlsHandshake::parse()` - handshake type and version extraction
- **HTTP**: `HttpRequest::parse()` / `HttpResponse::parse()` - application layer parsing

The `ParsedPacket::parse()` method orchestrates the full parsing chain from L2 through L7.

### Packet Capture (`capture.rs`)

Live packet capture and analysis using the `pnet` library.

- **Filtering**: Custom protocol/port filter for capturing relevant traffic (matches TCP, UDP, ICMP, and specific ports via string comparison).
- **Hexdump (`hexdump.rs`)**: Pretty-printed hex views of packet data.

### Packet Crafting (`craft.rs`)

Creating custom network packets from scratch.

- **TCP/UDP/ICMP**: Support for crafting standard transport and network layer packets with custom flags and payloads.
- **Validation (`validation.rs`)**: Ensuring crafted packets are well-formed and valid.

### Diagnostics & Tools

- **Traceroute (`traceroute.rs`)**: High-performance, multi-protocol traceroute implementation (UDP mode default; ICMP mode disabled due to TTL control issues).
- **DNS Parsing**: Implemented in `parse_impl.rs` via `DnsRecord::parse()` - low-level DNS message parsing with bounds check validation for malformed responses.
- **TLS Parsing**: Implemented in `parse_impl.rs` via `TlsHandshake::parse()` - extracting information from TLS handshakes (SNI, certificates).
- **HTTP Parsing**: Implemented in `parse_impl.rs` via `HttpRequest::parse()` and `HttpResponse::parse()`.

### Stress Testing (`crates/slapper/src/stress/`)

Generating massive amounts of network traffic to test the resilience of infrastructure and security appliances.

- **SYN Flooding**: Testing WAF/IPS resilience to half-open connection attacks.
- **UDP Flooding**: Volumetric stress testing with IP spoofing support.
- **HTTP Stressing**: High-volume HTTP request generation (different from the `loadtest` module which is more focused on performance benchmarking).

All stress tests require `stress-testing` feature flag. Raw socket operations require Unix platform.

## Security & Privileges

Many features in this module require elevated privileges (e.g., `root` or `CAP_NET_RAW` on Linux) as they interact with raw sockets.

## Recent Bug Fixes (2026-05-28)

| Component | Issue | Fix |
|-----------|-------|-----|
| `parse_impl.rs:644-651` | Redundant IP payload re-extraction in `ParsedPacket::parse()` | Removed; `IpPacket::parse_ipv4()` already extracts payload correctly |
| `craft.rs:186-187` | IPv4 fragmentation flags byte not initialized in `Ipv4Builder` | Added `bytes[7] = 0` to properly set flags octet |
| `capture.rs:47-49` | PcapWriter timestamp silently defaulted on clock error | Changed to propagate error with warning log |
| `icmp.rs:119` | IPv4 flags not set in ICMP packet builder | Added `set_flags(0x40)` for Don't Fragment in `build_icmp_packet_v4()` |
| `udp.rs:244` | Mutex poisoning could cause panic in raw UDP flood | Changed `unwrap()` to `into_inner()` for graceful handling |
| `parse_impl.rs:702-717` | `parse_app_layer()` read TCP ports from payload instead of header | Now uses `TcpHeader::src_port`/`dst_port` directly |
| `syn.rs:237-260` | IPv4 spoof range now supports both CIDR and range notation | Added range notation (`10.0.0.1-10.0.0.254`) parsing alongside CIDR |
| `syn.rs:263-306` | IPv6 spoof range now supports both CIDR and range notation | Added range notation parsing for consistency |
| `icmp.rs:244-267` | IPv4 spoof range now supports both CIDR and range notation | Added range notation parsing (consistent with syn.rs) |
| `icmp.rs:270-313` | IPv6 spoof range now supports both CIDR and range notation | Added range notation parsing for consistency |
| `parse_impl.rs:531,551` | DNS parsing bounds check for malformed responses | Added `new_offset >= data.len()` check before byte access |