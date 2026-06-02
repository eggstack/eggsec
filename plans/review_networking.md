# Networking & Packets Module Architecture Review

**Document:** architecture/networking.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 115

## Verified Claims

- **PacketInfo struct**: Verified at `crates/slapper/src/packet/mod.rs:25-34` - timestamp, ethernet, ip, transport, app, raw_size, hex_dump fields
- **CaptureConfig struct**: Verified at `crates/slapper/src/packet/capture.rs:76-100` - interface, filter, promiscuous, snapshot_len, timeout, max_packets, save_to_file, validate_checksums
- **CaptureStats struct**: Verified at `capture.rs:103-109` - packets_captured, bytes_captured, packets_dropped, runtime_ms
- **PcapWriter struct**: Verified at `capture.rs:14-74` - writes PCAP format with magic 0xa1b2c3d4, version 2.4, network type 1/Ethernet
- **CaptureBuilder struct**: Verified at `capture.rs:455-510` - fluent builder with interface(), filter(), promiscuous(), snapshot_len(), timeout(), max_packets(), save_to_file(), build() methods
- **PacketCapture struct**: Verified at `capture.rs:111` - main capture engine
- **PcapWriter timestamp error handling**: Verified at `capture.rs:46-53` - propagates error with warning log when clock errors occur
- **summary() method**: Verified at `packet/mod.rs:37-91` - produces human-readable one-liner

### Parsing Functions (parse_impl.rs)

- **EthernetFrame::parse()**: Verified
- **IpPacket::parse()**: Verified - dispatches to IPv4/IPv6
- **TcpHeader::parse()**: Verified (in types.rs)
- **UdpHeader::parse()**: Verified (in types.rs)
- **IcmpHeader::parse()**: Verified (in types.rs)
- **DnsRecord::parse()**: Verified (in types.rs)
- **TlsHandshake::parse()**: Verified (in types.rs)
- **HttpRequest::parse() / HttpResponse::parse()**: Verified (in types.rs)

### Bug Fixes Verified

- **parse_impl.rs:644-651 redundant IP extraction**: Fixed - code no longer has this issue
- **craft.rs:186-187 IPv4 fragmentation flags**: Fixed - `bytes[7] = 0` added
- **capture.rs:47-49 PcapWriter timestamp**: Fixed - now propagates error with warning
- **icmp.rs:119 IPv4 flags**: Fixed - `set_flags(0x40)` added for Don't Fragment
- **udp.rs:244 Mutex poisoning**: Fixed - uses proper handling
- **parse_impl.rs:702-717 TCP ports from header**: Fixed - uses TcpHeader::src_port/dst_port
- **syn.rs:237-260 IPv4 spoof range notation**: Fixed - supports both CIDR and range notation
- **syn.rs:263-306 IPv6 spoof range notation**: Fixed
- **icmp.rs:244-267 IPv4 spoof range notation**: Fixed
- **icmp.rs:270-313 IPv6 spoof range notation**: Fixed
- **parse_impl.rs:531,551 DNS bounds check**: Fixed - added `new_offset >= data.len()` check

## Discrepancies

- **Document says `CaptureBuilder` at `capture.rs:455-510`**: Actual is `455-510` - CORRECT

## Bugs Found

- **Bug**: In `capture.rs:209`, the PcapWriter `write_packet` result is silently dropped:
  ```rust
  if let Some(ref mut writer) = pcap_writer {
      let _ = writer.write_packet(&packet);
  }
  ```
  While the PcapWriter itself now properly handles errors (as documented), the caller still ignores the result. This could hide write failures. (capture.rs:209)

## Improvement Opportunities

- **Priority: Medium**: The `parse_impl.rs` file is large (866 lines) and contains both packet parsing implementations and types. The document references `parse_impl.rs` for DNS/TLS/HTTP parsing, but these implementations are actually in `types.rs`. The document could clarify that `parse_impl.rs` contains the `ParsedPacket::parse()` orchestration method while the individual protocol parsers are in `types.rs`.

- **Priority: Low**: The `CaptureBuilder::build()` method at `capture.rs:501` clones the config into `PacketCapture::new(self.config)`. This is fine but the builder pattern could be more ergonomic with `#[must_use]`.

## Stale Items

- **None identified**: All bug fixes are current and the bug fix table is accurate.

## Code Interrogation Findings

- **Finding**: The `packet_inspection` feature flag guards packet capture functionality at `capture.rs:11-12` with `#[cfg(all(feature = "packet-inspection", unix))]`. The architecture correctly notes this is Unix-only.
- **Finding**: `stress-testing` feature flag gates SYN flooding, UDP flooding, and HTTP stressing. The architecture correctly notes these require elevated privileges.
- **Finding**: The `PacketInfo::summary()` method at `packet/mod.rs:37-91` generates human-readable output but the example format in the doc (`"AA:BB:CC:DD:EE:FF → 11:22:33:44:55:66 | 10.0.0.1 → 10.0.0.2 | TCP 443 → 54321 | SYN"`) is representative but actual output depends on available protocol layers.
- **Finding**: The `PcapWriter::write_packet()` at `capture.rs:46-69` truncates data to `snapshot_len` before writing, which matches the PCAP format spec and the documented behavior.

## Summary

The networking & packets module architecture documentation is highly accurate. All packet parsing types, capture functionality, and documented bug fixes are verified. The module structure is correctly described with proper feature gating noted. Minor issue with silent error suppression in capture code.