---
name: wireless_security_testing
description: "Wireless network security testing - WiFi reconnaissance and WPA/WPA2/WPA3 analysis"
triggers:
  - wifi
  - wireless
  - wpa
  - wpa2
  - wpa3
  - ssid
  - bssid
  - handshake
  - access point
  - access point
  - wireless reconnaissance
  - wep
  - enterprise
metadata:
  category: recon
  tools: [wireless]
  scope: interface
---

## Overview

Slapper provides wireless network security testing capabilities through the `wireless` module. This enables WiFi reconnaissance, security type detection, and identification of weak points in wireless infrastructure.

**Note**: This module is feature-gated behind the `wireless` feature flag.

## Capabilities

- **WiFi Reconnaissance**: Discover and enumerate wireless networks
- **Security Type Detection**: Identify Open, WEP, WPA, WPA2, WPA3, Enterprise
- **Signal Strength Analysis**: Evaluate coverage and detect rogue access points
- **Handshake Analysis**: (Requires external tools like aircrack-ng)
- **Vulnerability Detection**: Identify weak configurations and security issues
- **Enterprise Security**: Support for WPA-Enterprise configurations

## Key Types

```rust
// Wireless network information
pub struct WirelessNetwork {
    pub ssid: String,              // Network name
    pub bssid: String,             // MAC address of AP
    pub channel: u8,               // WiFi channel
    pub security_type: SecurityType,
    pub signal_strength: i32,       // dBm
    pub last_seen: String,
}

// Security type enum
pub enum SecurityType {
    Open,
    WEP,           // Deprecated, insecure
    WPA,
    WPA2,
    WPA3,
    Enterprise,    // WPA-Enterprise
    Unknown,
}

// Scan results
pub struct WirelessScanResult {
    pub interface: String,
    pub networks: Vec<WirelessNetwork>,
    pub scan_duration_secs: u64,
    pub recommendations: Vec<String>,
}

// Detected vulnerability
pub struct WirelessVulnerability {
    pub ssid: String,
    pub bssid: String,
    pub vulnerability_type: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
}
```

## Usage

### CLI Usage

```bash
# Scan for wireless networks on interface
slapper wireless scan wlan0

# With specific interface
slapper wireless scan --interface wlan0mon

# Scan and output JSON
slapper wireless scan wlan0 --format json
```

### API Usage

```rust
use slapper::wireless::{WirelessScanner, SecurityType};

let scanner = WirelessScanner::new()?.with_interface("wlan0".to_string());
let result = scanner.scan().await?;

for network in &result.networks {
    println!("{} ({:?}) - {} dBm", network.ssid, network.security_type, network.signal_strength);
}
```

## Security Type Reference

| Type | Security Level | Notes |
|------|---------------|-------|
| Open | None | No encryption - highly insecure |
| WEP | Very Low | Deprecated, easily cracked |
| WPA | Low | TKIP encryption, deprecated |
| WPA2 | Medium | AES-CCMP, widely used |
| WPA3 | High | Latest standard, SAE key exchange |
| Enterprise | Varies | 802.1X with RADIUS authentication |

## Common Vulnerabilities

- **Open Networks**: No encryption, all traffic visible
- **WEP Encryption**: Easily crackable with public tools
- **Default Credentials**: APs with factory default passwords
- **Hidden SSID**: Not actually hidden, just not beaconed
- **Evil Twin**: Rogue access point mimicking legitimate network
- **Weak WPA2**: Pre-shared key brute force vulnerability

## Triggers

Keywords that activate this skill: `wifi`, `wireless`, `wpa`, `wpa2`, `wpa3`, `ssid`, `bssid`, `handshake`, `access point`, `enterprise`, `wireless reconnaissance`
