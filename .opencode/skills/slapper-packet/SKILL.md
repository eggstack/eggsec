# Slapper Packet Skill

Packet capture, crafting, and parsing module workflows and patterns.

## Key Types and Patterns

### Core Types (`packet/types.rs`)
- `EthernetFrame` - L2 frame with src/dst MAC, ether_type
- `IpPacket` - L3 packet with IPv4/IPv6 support, options, payload
- `TcpHeader`, `UdpHeader`, `IcmpHeader` - L4 headers
- `TransportProtocol` - enum variant for TCP/UDP/ICMP/Unknown
- `AppLayer` - enum for HTTP/DNS/TLS parsed application data
- `ParsedPacket` - full packet parsed chain (ethernet → ip → transport → app)

### Packet Parsing (`packet/parse_impl.rs`)
- `EthernetFrame::parse()` - Ethernet frame parsing
- `IpPacket::parse()` - dispatches to `parse_ipv4()` or `parse_ipv6()`
- `IpPacket::parse_ipv4()` / `parse_ipv6()` - IP layer parsing
- `TcpHeader::parse()` / `UdpHeader::parse()` / `IcmpHeader::parse()` - transport layer
- `HttpRequest::parse()` / `HttpResponse::parse()` - HTTP parsing
- `DnsRecord::parse()` - DNS message parsing
- `TlsHandshake::parse()` - TLS handshake parsing
- `ParsedPacket::parse()` - orchestrates full packet chain

### Packet Capture (`packet/capture.rs`)
- `PacketCapture` - live packet capture using `pnet` library
- `CaptureConfig` - interface, filter, promiscuous, snapshot_len, timeout
- `CaptureBuilder` - builder pattern for capture configuration
- BPF-style filtering via `packet_matches_filter()`
- pcap file writing via `PcapWriter`

### Packet Crafting (`packet/craft.rs`)
- `PacketBuilder` - fluent builder for custom packets
- `EthernetBuilder`, `Ipv4Builder`, `Ipv6Builder` - L2/L3 building
- `TcpBuilder`, `UdpBuilder`, `IcmpBuilder` - L4 building
- `TcpFlags` - TCP flag utilities (SYN, ACK, FIN, RST, etc.)
- IPv4 checksum calculation in `calculate_ipv4_checksum()`

### Traceroute (`packet/traceroute.rs`)
- `Traceroute` / `TracerouteConfig` - multi-protocol traceroute
- UDP mode (default) and ICMP mode (disabled due to bug)
- Parallel probes support
- Reverse DNS resolution via `hickory_resolver`

### Hexdump (`packet/hexdump.rs`)
- `hexdump()` / `hexdump_with_offset()` - pretty hex output
- `HexDumper<W>` - struct for writing hex to any writer

### Validation (`packet/validation.rs`)
- `format_ipv6()` - format raw IPv6 bytes as string
- `parse_dns_name()` - DNS name parsing with compression pointer support
- `parse_dns_rdata()` - DNS resource data parsing by type
- `dns_type_to_string()` - convert DNS type codes to strings

## Feature Flags
- `packet-inspection` - enables packet capture CLI (`packet/cli.rs`)

## Testing

### Running Packet Tests
```bash
cargo test --lib -p slapper packet::
```

## Common Tasks

### Adding a New Packet Parser
1. Implement `parse()` method on the appropriate type in `packet/parse_impl.rs`
2. Add integration in `ParsedPacket::parse()` if it should be auto-detected
3. Add tests for the new parser

### Capturing Packets
```rust
use crate::packet::{CaptureBuilder, PacketInfo};

let mut capture = CaptureBuilder::new()
    .interface("eth0")
    .filter("tcp")
    .max_packets(100)
    .build();

let (tx, mut rx) = tokio::sync::mpsc::channel(100);
let stats = capture.start(tx).await?;
```

### Crafting a Custom Packet
```rust
use crate::packet::craft::PacketBuilder;
use crate::packet::types::TcpFlags;

let packet = PacketBuilder::new()
    .ethernet([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff], [0x11, 0x22, 0x33, 0x44, 0x55, 0x66], 0x0800)
    .ipv4("192.168.1.1".parse()?, "192.168.1.2".parse()?, 6, 64)
    .tcp(12345, 80, 1000, 0, TcpFlags::syn(), 65535)
    .payload(b"Hello".to_vec())
    .build();
```

## Resources
- `architecture/networking.md` - Networking module design
- `crates/slapper/src/packet/mod.rs` - Public API exports