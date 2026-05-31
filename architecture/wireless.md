# Wireless Module

## Purpose

Wireless network security testing including WiFi reconnaissance, WPA/WPA2 handshake capture analysis, and weak point detection. Feature-gated behind the `wireless` flag.

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

Implemented behind `wireless` feature flag. Core types and scanner logic are in place with network enumeration, handshake analysis, and vulnerability detection.
