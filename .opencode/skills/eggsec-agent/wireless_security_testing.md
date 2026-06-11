---
name: wireless_security_testing
description: "Wireless network security testing - passive WiFi reconnaissance and basic security analysis (no handshake capture, aspirational)"
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

Eggsec provides wireless network security testing capabilities through the `wireless` module. This enables passive WiFi reconnaissance, security type detection, and identification of weak points in wireless infrastructure (no handshake capture; aspirational only).

**Note**: This module is feature-gated behind the `wireless` feature flag.

## Capabilities

- **WiFi Reconnaissance**: Discover and enumerate wireless networks (passive, iwlist)
- **Security Type Detection**: Identify Open, WEP, WPA, WPA2, WPA3, Enterprise
- **WPS / Hidden / Transition Detection**: Beacon-level indicators for WPS, hidden SSIDs, WPA2/WPA3 mixed mode
- **Signal Strength Analysis**: Evaluate coverage; flag weak signals
- **Basic Rogue / Suspicious Detection**: Passive heuristic for duplicate SSID with differing BSSID/security (labeled "Possible Rogue AP / Evil Twin (passive heuristic)")
- **Vulnerability Detection**: Identify weak/legacy configs (Open/WEP/WPA), unknown, enterprise advisory, WPS, hidden, transition, weak signal
- **Enterprise Security**: Support for WPA-Enterprise configurations
- **Recommendations**: Actionable guidance + "run repeated scans for rogue observation"
- **Handshake Analysis**: Not implemented (aspirational; external tools would be required)

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
# Passive scan for wireless networks on interface (requires --features wireless + root/iwlist)
eggsec wireless wlan0

# Repeated scans for change/rogue observation
eggsec wireless wlan0 --repeat 5 --duration 10

# JSON output (full WirelessScanResult with WPS/hidden/transition fields)
eggsec wireless wlan0 --json -o results.json

# Quiet + file
eggsec wireless wlan0 -q -o out.json
```

### API Usage

```rust
use eggsec::wireless::{WirelessScanner, SecurityType};

let scanner = WirelessScanner::new().with_interface("wlan0".to_string());
let result = scanner.scan(10).await?;  // duration_secs

for network in &result.networks {
    println!("{} ({:?}) - {} dBm  WPS:{} hidden:{} trans:{}",
        network.ssid, network.security_type, network.signal_strength,
        network.wps_enabled, network.is_hidden, network.transition_mode);
}

let vulns = WirelessScanner::analyze_networks(&result.networks);
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
| Transition (WPA2/WPA3 mixed) | Medium | Backward-compat mode; advisory |
| (WPS enabled) | Lower effective | Additional attack surface (passive flag only) |

## Common Vulnerabilities (Passive Detection)

- **Open Networks**: No encryption, all traffic visible
- **WEP Encryption**: Easily crackable with public tools
- **WPA (legacy)**: TKIP vulnerabilities
- **Default Credentials**: APs with factory default passwords (not directly detected; physical/radio follow-up)
- **Hidden SSID**: Not actually hidden, just not beaconed (advisory)
- **Evil Twin / Rogue (heuristic)**: Same SSID appearing with different BSSID or security type across scan cells (passive only)
- **Weak WPA2 / Transition**: Mixed mode or weak signal environments
- **WPS Enabled**: Known attack surface (PIN brute-force in active scenarios)

## Triggers

Keywords that activate this skill: `wifi`, `wireless`, `wpa`, `wpa2`, `wpa3`, `ssid`, `bssid`, `handshake`, `access point`, `enterprise`, `wireless reconnaissance`, `wps`, `rogue ap`, `evil twin`

## Notes (First Handoff)

- Passive-only (no injection, deauth, or handshake capture).
- Requires --features wireless + Linux iwlist + appropriate privileges.
- See docs/WIRELESS.md, architecture/wireless.md, plans/wireless-first-handoff-plan.md.
