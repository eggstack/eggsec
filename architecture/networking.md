# Networking & Packets Module

The Networking module provides low-level access to the network stack for tasks like packet capture, custom packet crafting, and stress testing.

## Core Capabilities (`src/packet/` & `src/stress/`)

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

- **Filtering**: Support for BPF-style filters to capture only relevant traffic.
- **Hexdump (`hexdump.rs`)**: Pretty-printed hex views of packet data.

### Packet Crafting (`craft.rs`)

Creating custom network packets from scratch.

- **TCP/UDP/ICMP**: Support for crafting standard transport and network layer packets with custom flags and payloads.
- **Validation (`validation.rs`)**: Ensuring crafted packets are well-formed and valid.

### Diagnostics & Tools

- **Traceroute (`traceroute.rs`)**: High-performance, multi-protocol traceroute implementation.
- **DNS Parsing**: Implemented in `parse_impl.rs` via `DnsRecord::parse()` - low-level DNS message parsing.
- **TLS Parsing**: Implemented in `parse_impl.rs` via `TlsHandshake::parse()` - extracting information from TLS handshakes (SNI, certificates).
- **HTTP Parsing**: Implemented in `parse_impl.rs` via `HttpRequest::parse()` and `HttpResponse::parse()`.

### Stress Testing (`src/stress/`)

Generating massive amounts of network traffic to test the resilience of infrastructure and security appliances.

- **SYN Flooding**: Testing WAF/IPS resilience to half-open connection attacks.
- **UDP Flooding**: Volumetric stress testing.
- **HTTP Stressing**: High-volume HTTP request generation (different from the `loadtest` module which is more focused on performance benchmarking).

## Security & Privileges

Many features in this module require elevated privileges (e.g., `root` or `CAP_NET_RAW` on Linux) as they interact with raw sockets.
