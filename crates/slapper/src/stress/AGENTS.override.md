# Stress Module Override

Specialized guidance for the stress testing module.

## raw_udp Integration

The `raw_udp` module in `stress/udp.rs:20-117` is integrated:
- `run_udp_flood()` calls `run_udp_flood_spoofed()`
- Uses `raw_udp::build_udp_packet` when IP spoofing is enabled on Unix
- Feature-gated behind `stress-testing`

## UDP Spoof Range

`get_random_spoofed_ip()` uses O(1) random selection from CIDR or dash-range notation:
- CIDR: randomly selects an IP within the network range
- Dash-range: randomly selects within start-end bounds
- Falls back to `generate_random_ip()` if range parsing fails

This approach matches `syn.rs::get_spoofed_source()` and avoids loading all IPs into memory.

## UDP Checksum Pseudo-Header

When implementing UDP checksum calculation, the pseudo-header format (RFC 768) is:
```
Bytes 0-3:   Source IP (4 bytes)
Bytes 4-7:   Destination IP (4 bytes)
Byte 8:      Zero (1 byte)
Byte 9:      Protocol (1 byte) - 17 for UDP
Bytes 10-11: UDP length (2 bytes)
Bytes 12-13: Source port (2 bytes)
Bytes 14-15: Destination port (2 bytes)
```

In `calculate_udp_checksum()` at `stress/udp.rs:82-113`:
- Protocol byte (17) is at offset 9
- Length is at offsets 10-11
- Ports are at offsets 12-15