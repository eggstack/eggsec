# Stress Module Override

Specialized guidance for the stress testing module.

## raw_udp Integration

The `raw_udp` module in `stress/udp.rs:20-117` is integrated:
- `run_udp_flood()` calls `run_udp_flood_spoofed()`
- Uses `raw_udp::build_udp_packet` when IP spoofing is enabled on Unix
- Feature-gated behind `stress-testing`

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