# Networking & Packets Module Architecture Review

## Summary

The networking module (`crates/slapper/src/packet/` and `crates/slapper/src/stress/`) largely matches the documented architecture in `architecture/networking.md`. All core capabilities are implemented as documented, and recent bug fixes are correctly applied. However, there are some minor discrepancies and one potential issue.

## Verified Correct

| Claim | Implementation | Status |
|-------|----------------|--------|
| `EthernetFrame::parse()` | `parse_impl.rs:8-38` | ✅ |
| `IpPacket::parse()` dispatching to IPv4/IPv6 | `parse_impl.rs:40-212` | ✅ |
| `TcpHeader::parse()` with options parsing | `parse_impl.rs:214-327` | ✅ |
| `UdpHeader::parse()` | `parse_impl.rs:329-350` | ✅ |
| `IcmpHeader::parse()` | `parse_impl.rs:352-386` | ✅ |
| `DnsRecord::parse()` with compression | `parse_impl.rs:494-590` | ✅ |
| `TlsHandshake::parse()` | `parse_impl.rs:592-634` | ✅ |
| `HttpRequest::parse()` / `HttpResponse::parse()` | `parse_impl.rs:388-492` | ✅ |
| `ParsedPacket::parse()` orchestrating full chain | `parse_impl.rs:636-691` | ✅ |
| Packet capture with `pnet` library | `capture.rs` (feature-gated) | ✅ |
| BPF-style filtering | `capture.rs:276-307` | ✅ |
| Hexdump | `hexdump.rs` | ✅ |
| TCP/UDP/ICMP crafting | `craft.rs` | ✅ |
| Validation | `validation.rs` | ✅ |
| Traceroute (UDP mode default, ICMP disabled) | `traceroute.rs:119-123` | ✅ |
| SYN flooding | `syn.rs` (stress-testing feature) | ✅ |
| UDP flooding with IP spoofing | `udp.rs` (stress-testing feature) | ✅ |
| HTTP stressing | `http.rs` (stress-testing feature) | ✅ |
| Recent bug fix: IPv4 fragmentation flags byte | `craft.rs:187` - `bytes[7] = 0` | ✅ |
| Recent bug fix: PcapWriter timestamp error handling | `capture.rs:47-53` - propagates error | ✅ |
| Recent bug fix: IPv4 flags in ICMP packet builder | `icmp.rs:120` - `set_flags(0x40)` | ✅ |
| Recent bug fix: Mutex poisoning graceful handling | `udp.rs:246` - `*poisoned.into_inner()` | ✅ |
| Recent bug fix: IPv4 spoof range notation | `syn.rs:237-270` | ✅ |
| Recent bug fix: IPv6 spoof range notation | `syn.rs:272-326` | ✅ |
| Recent bug fix: ICMP IPv4/IPv6 spoof ranges | `icmp.rs:244-333` | ✅ |

## Bugs Found

| Priority | Issue | Location |
|----------|-------|----------|
| P2 | Unwrap on `parse_dns_name` could panic | `parse_impl.rs:531` - `new_offset` could exceed `data.len()` |

**Details**: In `DnsRecord::parse()`, the code calls `super::validation::parse_dns_name(data, offset)` and then checks `new_offset + 4 > data.len()`. However, `parse_dns_name` could return an offset that is already beyond `data.len()`, causing the `+ 4` to overflow or the subsequent `u16::from_be_bytes` calls to panic if `new_offset + 4` wraps.

```rust
// parse_impl.rs:531-546
if let Some((name, new_offset)) = super::validation::parse_dns_name(data, offset) {
    if new_offset + 4 > data.len() {  // Potential panic if new_offset > data.len()
        break;
    }
    let qtype = u16::from_be_bytes([data[new_offset], data[new_offset + 1]]);
```

**Recommended fix**: Add bounds check before accessing `data[new_offset]`:
```rust
if new_offset >= data.len() || new_offset + 4 > data.len() {
    break;
}
```

## Discrepancies

| Item | Documented | Actual |
|------|-----------|--------|
| DNS parsing location | Listed under "Diagnostics & Tools" as "Implemented in `parse_impl.rs`" | Correct - `DnsRecord::parse()` in `parse_impl.rs` |
| TLS parsing location | Listed under "Diagnostics & Tools" as "Implemented in `parse_impl.rs`" | Correct - `TlsHandshake::parse()` in `parse_impl.rs` |

## Performance Notes

- The module uses standard `HashMap` in some internal operations but this is acceptable for the packet parsing path as it operates on single packets
- No `FxHashMap` requirements were documented for this module

## Security Notes

- Raw socket operations correctly require `stress-testing` feature and Unix platform
- ICMP traceroute is correctly disabled due to TTL control issues (documented and enforced in `traceroute.rs:119-123`)
- Privileged operations use `crate::utils::privilege::check_privileged()` before raw socket creation

## Recommendations

1. **P2**: Add bounds check in `DnsRecord::parse()` before accessing byte slices to prevent potential panics on malformed DNS responses

2. **Low Priority**: Consider adding more test coverage for packet parsing edge cases (truncated packets, malformed headers)