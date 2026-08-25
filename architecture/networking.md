# Networking & Packets Module

## Role & Responsibilities

Low-level network stack access for packet capture, custom packet crafting, protocol parsing, hex dumping, and traceroute. This module provides the foundation for packet-level inspection and diagnostic tools.

**Safety posture**: Passive by default — packet capture reads from the network without modification. Active operations (crafting, sending, raw sockets) require explicit invocation and elevated privileges. Feature-gated behind `packet-inspection`.

## Location & Feature Gating

| Component | Feature gate | `cfg` line |
|-----------|-------------|------------|
| All `packet/` submodules | Always compiled | `packet/mod.rs:1-7` |
| `packet/cli` module | `packet-inspection` + `cli` | `packet/mod.rs:20-22` (`#[cfg(all(feature = "packet-inspection", unix))]` for capture; `#[cfg(all(feature = "packet-inspection", cli))]` for CLI) |
| `PacketCapture::start()` | `packet-inspection` + unix | `capture.rs:151` |
| `list_interfaces()` (real) | `packet-inspection` + unix | `capture.rs:414` |
| `list_interfaces()` (stub) | non-packet-inspection | `capture.rs:434` |
| Raw ICMP send (`send_raw_icmp`) | `packet-inspection` + unix | `cli.rs:148` |
| ICMP traceroute (`probe_hop_icmp_parallel`) | `stress-testing` + unix | `traceroute.rs:348` |

## Architecture

### Submodules (7)

| Submodule | File | Purpose |
|-----------|------|---------|
| `capture` | `capture.rs` (557 lines) | Live packet capture via pnet, PCAP writing, interface enumeration, BPF-style filtering |
| `craft` | `craft.rs` (748 lines) | Packet construction: Ethernet, IPv4, IPv6, TCP, UDP, ICMP with checksums |
| `hexdump` | `hexdump.rs` (192 lines) | Hex dump formatting: streaming `HexDumper<W>` and convenience functions |
| `parse_impl` | `parse_impl.rs` (866 lines) | Protocol parsing: L2→L7 chain (Ethernet, IPv4/IPv6, TCP/UDP/ICMP, DNS, TLS, HTTP) |
| `traceroute` | `traceroute.rs` (677 lines) | Multi-protocol traceroute: UDP (default) and ICMP (disabled), parallel probes, reverse DNS |
| `types` | `types.rs` (265 lines) | Core data types: `ParsedPacket`, protocol structs, `AppLayer` enum |
| `validation` | `validation.rs` (149 lines) | DNS name parsing with compression, RData formatting, IPv6 formatting |

### Key Types

#### Parsing (`types.rs`)

| Type | Location | Description |
|------|----------|-------------|
| `ParsedPacket` | `types.rs:260` | Top-level parsed representation: `ethernet`, `ip`, `transport`, `app` (all `Option`) |
| `EthernetFrame` | `types.rs:4` | L2: `dst_mac`, `src_mac`, `ether_type`, `ether_type_name` |
| `IpPacket` | `types.rs:18` | L3: version, header_len, total_len, ttl, protocol, src/dst IP, payload, options, flags, checksum |
| `IpFlags` | `types.rs:34` | `reserved`, `dont_fragment`, `more_fragments` |
| `TcpHeader` | `types.rs:132` | L4 TCP: ports, seq/ack numbers, data_offset, flags, window, checksum, urgent, payload, options |
| `TcpFlags` | `types.rs:71` | 8 flags: `fin`, `syn`, `rst`, `psh`, `ack`, `urg`, `ece`, `cwr` |
| `UdpHeader` | `types.rs:154` | L4 UDP: ports, length, checksum, payload |
| `IcmpHeader` | `types.rs:163` | L4 ICMP: type, code, checksum, payload |
| `TransportProtocol` | `types.rs:171` | Enum: `Tcp(TcpHeader)`, `Udp(UdpHeader)`, `Icmp(IcmpHeader)`, `Unknown(Vec<u8>)` — **4 variants** |
| `HttpRequest` | `types.rs:180` | L7 HTTP request: method, uri, version, headers, body |
| `HttpResponse` | `types.rs:188` | L7 HTTP response: version, status_code, reason_phrase, headers, body |
| `HttpHeader` | `types.rs:198` | HTTP header: name, value |
| `DnsRecord` | `types.rs:204` | L7 DNS: transaction_id, flags, query_type, questions, answers |
| `DnsQuestion` | `types.rs:212` | DNS question: name, query_type, class |
| `DnsAnswer` | `types.rs:219` | DNS answer: name, record_type, ttl, data |
| `TlsHandshake` | `types.rs:228` | L7 TLS: handshake_type, version, client_hello, server_hello |
| `TlsClientHello` | `types.rs:236` | TLS ClientHello: session_id, cipher_suites, compression_methods, server_name, supported_versions |
| `TlsServerHello` | `types.rs:244` | TLS ServerHello: version, session_id, cipher_suite |
| `AppLayer` | `types.rs:252` | Enum: `Http(HttpRequest)`, `Dns(DnsRecord)`, `Tls(TlsHandshake)`, `Unknown` — **4 variants** |

#### Capture (`capture.rs`)

| Type | Location | Description |
|------|----------|-------------|
| `PacketCapture` | `capture.rs:115` | Main capture engine: `new()`, `is_running()`, `stop()`, `stats()`, `running()`, `start()` (async) |
| `CaptureConfig` | `capture.rs:80` | Builder input: interface, filter, promiscuous, snapshot_len, timeout, max_packets, save_to_file, validate_checksums |
| `CaptureStats` | `capture.rs:107` | Post-capture metrics: packets_captured, bytes_captured, packets_dropped, runtime_ms |
| `CaptureBuilder` | `capture.rs:466` | Fluent builder for `PacketCapture` |
| `PcapWriter` | `capture.rs:14` | PCAP file writer: 24-byte global header + per-packet headers |
| `CaptureError` | `capture.rs:448` | Error enum: `AlreadyRunning`, `NoInterface`, `InterfaceNotFound`, `RequiresRoot`, `UnsupportedChannel`, `ChannelError`, `IoError` — **7 variants** |
| `NetworkInterfaceInfo` | `capture.rs:439` | Interface metadata: name, ips, mac, is_up, is_loopback |
| `PacketInfo` | `mod.rs:26` | Parsed captured packet: timestamp, ethernet, ip, transport, app, raw_size, hex_dump |

#### Crafting (`craft.rs`)

| Type | Location | Description |
|------|----------|-------------|
| `PacketBuilder` | `craft.rs:189` | Top-level builder: `ethernet()`, `ipv4()`, `ipv6()`, `tcp()`, `udp()`, `icmp()`, `payload()`, `validate()`, `build()` |
| `EthernetBuilder` | `craft.rs:434` | 14-byte Ethernet frame builder |
| `Ipv4Builder` | `craft.rs:451` | 20-byte IPv4 header builder (with random ID) |
| `Ipv6Builder` | `craft.rs:480` | 40-byte IPv6 header builder |
| `TcpBuilder` | `craft.rs:505` | TCP segment builder with pseudo-header checksum |
| `UdpBuilder` | `craft.rs:576` | UDP datagram builder with pseudo-header checksum |
| `IcmpBuilder` | `craft.rs:595` | ICMP message builder with checksum |
| `TransportBuilder` | `craft.rs:743` | Enum: `Tcp`, `Udp`, `Icmp` |
| `PacketValidationError` | `craft.rs:156` | Validation error: `AddressFamilyMismatch`, `InvalidTtl`, `InvalidHopLimit`, `InvalidTcpOptionsLength`, `PacketTooLarge`, `PayloadTooLarge` — **6 variants** |

#### Traceroute (`traceroute.rs`)

| Type | Location | Description |
|------|----------|-------------|
| `Traceroute` | `traceroute.rs:113` | Traceroute engine: `new()`, `run()` (async) |
| `TracerouteConfig` | `traceroute.rs:13` | Config: target, max_hops(30), timeout(3s), max_retries, first_ttl(1), port(33434), use_icmp(false), packet_size(32), parallel_probes(true), resolve_names(true), max_concurrent_probes(6) |
| `TracerouteBuilder` | `traceroute.rs:574` | Fluent builder for `Traceroute` |
| `TracerouteResult` | `traceroute.rs:104` | Result: target, resolved_address, hops, total_hops, success |
| `TracerouteHop` | `traceroute.rs:46` | Hop: hop number, address, rtt, rtt_ms, name, is_final, probes |
| `HopProbe` | `traceroute.rs:57` | Individual probe: address, rtt, success |
| `TracerouteError` | `traceroute.rs:548` | Error: `ResolveError`, `ProbeError`, `RequiresRoot`, `Unsupported` |
| `ProbeError` | `traceroute.rs:560` | Probe error: `SocketError`, `SendError`, `ReceiveError`, `Timeout`, `PortUnreachable` |

#### Hexdump (`hexdump.rs`)

| Type | Location | Description |
|------|----------|-------------|
| `HexDumper<W>` | `hexdump.rs:54` | Streaming hex dump to `fmt::Write` writer with configurable bytes_per_line and offset |

### Key Functions

| Function | Location | Description |
|----------|----------|-------------|
| `ParsedPacket::parse(data)` | `parse_impl.rs:758` | Full L2→L7 parsing chain orchestrator |
| `EthernetFrame::parse(data)` | `parse_impl.rs:8` | 14-byte Ethernet frame parsing |
| `IpPacket::parse(data)` | `types.rs:58` | Dispatches to IPv4/IPv6 parser |
| `TcpHeader::parse(data)` | `parse_impl.rs:220` | TCP header with options parsing |
| `UdpHeader::parse(data)` | `parse_impl.rs:335` | UDP datagram parsing |
| `IcmpHeader::parse(data)` | `parse_impl.rs:358` | ICMP message parsing |
| `DnsRecord::parse(data)` | `parse_impl.rs:447` | Full DNS message parsing with compression |
| `TlsHandshake::parse(data)` | `parse_impl.rs:598` | TLS handshake type/version extraction |
| `HttpRequest::parse(data)` | `parse_impl.rs:500` | HTTP request parsing |
| `HttpResponse::parse(data)` | `parse_impl.rs:540` | HTTP response parsing |
| `hexdump(data)` | `hexdump.rs:5` | Convenience hex dump to string |
| `hexdump_with_offset(data, offset, bpl)` | `hexdump.rs:9` | Hex dump with custom start offset and bytes per line |
| `parse_dns_name(data, offset)` | `validation.rs:10` | DNS name parsing with compression pointer support (max 100 jumps) |
| `parse_dns_rdata(data, offset, rtype, rdlen)` | `validation.rs:70` | DNS RData formatting (A, AAAA, NS, CNAME, PTR, MX, TXT, SOA) |

## Behavior / Flow

### Packet Capture Flow

1. `CaptureBuilder::build()` creates `PacketCapture` with config
2. `PacketCapture::start(sender)` opens pnet datalink channel on the interface
3. Spawns a capture thread reading from `DataLinkReceiver` into a crossbeam bounded channel (100 messages)
4. Main loop: reads packets, applies string-based filter (`tcp`/`udp`/`icmp`/`ip`/`port N`), writes to PCAP if configured, parses via `ParsedPacket::parse()`, sends `PacketInfo` to caller
5. Stops on: `max_packets` reached, `stop()` called, or sender dropped
6. Returns `CaptureStats`

**Filter implementation** (`capture.rs:284-314`): String-based matching on IP protocol number (6=TCP, 17=UDP, 1/58=ICMP) and transport ports. Not true BPF — simple string comparison.

### Packet Parsing Chain

`ParsedPacket::parse()` (`parse_impl.rs:758`) orchestrates:

1. **Ethernet** (bytes 0-13): `EthernetFrame::parse()` extracts MACs and ether type
2. **IP** (bytes 14+): `IpPacket::parse()` dispatches to IPv4 (20+ byte header) or IPv6 (40 byte header)
3. **Transport** (after IP header): Protocol-dependent — TCP (20+ bytes with options), UDP (8 bytes), ICMP (8+ bytes)
4. **Application**: `parse_app_layer()` uses IP protocol + transport ports to detect:
   - TCP ports 80/8080 → `HttpRequest::parse()`
   - UDP → `DnsRecord::parse()`
   - TLS record type 0x16 + version 0x03xx → `TlsHandshake::parse()`

### Packet Crafting Flow

`PacketBuilder::build()` (`craft.rs:334`):

1. `validate()`: checks address family consistency, TTL/hop-limit non-zero, TCP options alignment, packet/payload size limits
2. Serializes: Ethernet → IPv4/IPv6 → Transport (with checksums) → payload
3. TCP/UDP checksums use pseudo-headers (IPv4: 12-byte, IPv6: 40-byte)
4. ICMP checksums computed over full ICMP message
5. IPv4 header checksum computed over 20-byte header

### Traceroute Flow

1. Resolve target (IP or DNS via `std::net::ToSocketAddrs`)
2. UDP mode (default): Send UDP packets to `port + ttl - 1`, increment TTL per hop
3. ICMP mode (disabled — `traceroute.rs:123-127`): Currently returns `Unsupported` error
4. Parallel mode (default): Spawns all probes for a hop concurrently with semaphore-based concurrency control (`max_concurrent_probes: 6`)
5. Sequential mode: Retries per hop with 50ms inter-hop delay
6. Detects final hop: response from target IP or ICMP port-unreachable
7. Reverse DNS via `hickory_resolver` (2s timeout, 1 attempt)
8. Port-unreachable from target treated as successful final hop

## Public API

Re-exported from `packet/mod.rs:9-18`:
- `CaptureBuilder`, `CaptureConfig`, `CaptureError`, `CaptureStats`, `PacketCapture`
- `PacketBuilder`
- `hexdump`, `hexdump_with_offset`
- `TracerouteConfig`, `TracerouteError`, `TracerouteHop`, `TracerouteResult`
- `ParsedPacket`
- All protocol types: `EthernetFrame`, `IpPacket`, `TcpHeader`, `UdpHeader`, `IcmpHeader`, `TcpFlags`, `AppLayer`, `DnsRecord`, `TlsHandshake`, `HttpRequest`, `HttpResponse`, etc.

## Integration Points

### CLI Handlers (`packet/cli.rs`)

| Command | Handler | Requirements |
|---------|---------|-------------|
| `eggsec packet capture` | `handle_packet_capture()` | `packet-inspection` + unix + root |
| `eggsec packet send` | `handle_packet_send()` | Root for raw ICMP; UDP fallback without |
| `eggsec packet dump` | `handle_packet_dump()` | None (file parsing) |
| `eggsec packet traceroute` | `handle_packet_traceroute()` | Root for ICMP mode |
| `eggsec packet interfaces` | `handle_packet_interfaces()` | `packet-inspection` + unix |

### Dispatch / Tool Registry

Packet is not registered as a `SecurityTool` — it is a utility module accessed via CLI commands. No MCP/REST/gRPC tool exposure.

### ProbeIntent / ProbeRisk

Packet module does not use `ProbeIntent`/`ProbeRisk`. It is outside the `ScanProfile` pipeline.

## Platform Requirements

| Component | Requirements |
|-----------|-------------|
| Packet capture | `packet-inspection` feature, Unix (pnet datalink), root or `CAP_NET_RAW` |
| Packet send (UDP) | None (standard UDP socket) |
| Packet send (raw ICMP) | `packet-inspection` + unix + root |
| Packet dump | None (file parsing only) |
| Traceroute (UDP) | None (standard UDP socket) |
| Traceroute (ICMP) | `stress-testing` or `packet-inspection` + unix + root (uses `surge_ping`) |
| Build | `libpcap-dev` for pnet on some platforms |

## Testing

### Unit Tests

- **`capture.rs:523-557`**: 2 tests — packet filter matching (TCP/UDP protocol, port filtering)
- **`craft.rs:399-432`**: 2 tests — UDP checksum presence for IPv4/IPv6, mixed address family rejection
- **`hexdump.rs:134-191`**: 7 tests — empty input, basic dump, offset, non-printable, 16-byte boundary, >16 bytes, zero bytes-per-line safety
- **`traceroute.rs:651-676`**: 2 tests — ICMP unsupported error, PTR name normalization
- **`validation.rs:57-68`**: 1 test — compressed DNS name offset
- **`cli.rs:725-757`**: 1 test — PCAP record parsing with `incl_len`

### Test Data

- `capture.rs` includes a 54-byte `TCP_PACKET` constant for filter tests
- `cli.rs` constructs PCAP files programmatically for record parsing tests

## Invariants & Gotchas

1. **ICMP traceroute disabled**: `Traceroute::run()` returns `Unsupported` when `use_icmp` is true (`traceroute.rs:123`). This is documented as a TTL control issue. Use UDP mode.
2. **Filter is string-based**: `packet_matches_filter()` does simple string comparison, not true BPF. Only supports `tcp`, `udp`, `icmp`, `ip`, and `port N`.
3. **PCAP timestamp saturation**: Y2038 boundary handled by saturating to `u32::MAX` instead of panicking (`capture.rs:59`).
4. **Frame capture thread**: Uses `crossbeam::channel::bounded(100)` as bridge between sync capture thread and async caller. Disconnection breaks the loop.
5. **Duplicate `TcpFlags`**: Defined in both `types.rs:71` and `craft.rs:635`. The `craft.rs` version adds `to_byte()` and convenience constructors (`syn()`, `ack()`, `syn_ack()`, `fin()`, `rst()`).
6. **DNS compression jump limit**: `parse_dns_name()` limits compression pointer chains to 100 jumps to prevent infinite loops (`validation.rs:33`).
7. **ICMP checksum uses `0xffff` for zero**: UDP checksum normalizes 0 to 0xffff per RFC 768 (`craft.rs:133-137`).
8. **PacketBuilder random ID**: `Ipv4Builder` uses `rand::random()` for IP identification field (`craft.rs:224`).
9. **PacketInfo.summary()**: Produces human-readable one-liner with `→` separator and `|` between layers (`mod.rs:38-91`).
10. **CaptureBuilder defaults**: promiscuous=true, snapshot_len=65535, timeout=1s, no max_packets, no file output, no checksum validation (`capture.rs:92-105`).

---

See also: [overview.md](overview.md), [probe.md](probe.md), [stress.md](stress.md), [defense_lab.md](defense_lab.md)

*Last verified against source: 2026-08-25*
