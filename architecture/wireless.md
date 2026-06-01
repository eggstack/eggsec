# Wireless Module

## Purpose

Wireless network security testing including iwlist scan parsing and wireless security type analysis (Open, WEP, WPA, WPA2, WPA3, Enterprise). Feature-gated behind the `wireless` flag.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WirelessScanner` | `wireless/mod.rs` | Main wireless scanning engine |
| `WirelessNetwork` | `wireless/mod.rs` | Discovered wireless network (SSID, BSSID, channel, security) |
| `SecurityType` | `wireless/mod.rs` | Enum: Open, WEP, WPA, WPA2, WPA3, Enterprise, Unknown |
| `WirelessScanResult` | `wireless/mod.rs` | Scan results with network list and recommendations |
| `WirelessVulnerability` | `wireless/mod.rs` | Wireless-specific vulnerability finding |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `WirelessScanner`, `WirelessNetwork`, `SecurityType`, scanning and vulnerability detection logic |

## Implementation Status

Implemented behind `wireless` feature flag. Core types and scanner logic are in place with iwlist-based network enumeration and security type analysis. WPA/WPA2 handshake capture analysis is not yet implemented (aspirational).
