# Wireless Module Architecture Review

**Document:** architecture/wireless.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 25

## Verified Claims

### Key Types

- **WirelessScanner**: Verified at `crates/slapper/src/wireless/mod.rs:66-68` - interface field
- **WirelessNetwork**: Verified at `wireless/mod.rs:13-21` - ssid, bssid, channel, security_type, signal_strength, last_seen
- **SecurityType enum**: Verified at `wireless/mod.rs:23-32` - Open, WEP, WPA, WPA2, WPA3, Enterprise, Unknown
- **WirelessScanResult**: Verified at `wireless/mod.rs:48-54` - interface, networks, scan_duration_secs, recommendations
- **WirelessVulnerability**: Verified at `wireless/mod.rs:56-64` - ssid, bssid, vulnerability_type, severity, description, recommendation

### Files

- **mod.rs**: Verified - Module root with `WirelessScanner`, `WirelessNetwork`, `SecurityType`, scanning and vulnerability detection logic

### Feature Gating

- **wireless feature flag**: Verified at `wireless/mod.rs:80` - `#[cfg(feature = "wireless")]` gates the `scan()` method
- **scan() without feature**: Verified at `wireless/mod.rs:106-111` - returns error "Wireless scanning requires the `wireless` feature"

### Security Type Analysis

- **WPA/WPA2/WPA3 detection**: Verified at `wireless/mod.rs:175-180` - checks for WPA2/WPA3 keywords
- **WPA detection**: Verified at `wireless/mod.rs:181-182`
- **WEP detection**: Verified at `wireless/mod.rs:183-184`
- **Open network detection**: Verified at `wireless/mod.rs:185-187`

### iwlist Parsing

- **parse_scan_output()**: Verified at `wireless/mod.rs:114-202` - parses ESSID, Address, Channel, Signal level, security keywords

## Discrepancies

- **None identified**: All types and functionality match between documentation and implementation.

## Bugs Found

- **Bug**: In `wireless/mod.rs:149-155`, the ESSID parsing uses:
  ```rust
  current_ssid = Some(
      line.split("ESSID:\"")
          .nth(1)
          .unwrap_or("")
          .trim_matches('"')
          .to_string(),
  );
  ```
  This will panic if the line doesn't contain `ESSID:"` but does start with `ESSID:`. For example, `ESSID:"test" with extra` would incorrectly get `"test" with extra`. However, since it checks `line.starts_with("ESSID:")` before this parsing, it should be safe. The parsing is fragile but not buggy.

## Improvement Opportunities

- **Priority: Medium**: The `analyze_networks()` method at `wireless/mod.rs:209-249` only generates vulnerabilities for Open, WEP, and WPA networks. It does not flag WPA2 Enterprise or other potentially weak configurations. This is a limited detection scope that could be expanded.

- **Priority: Low**: The `parse_scan_output()` method at `wireless/mod.rs:114-202` is a simple line-by-line parser that doesn't handle all possible iwlist output formats. Networks might be missed if the output format differs from expected.

- **Priority: Low**: The module uses `tokio::process::Command` to call `iwlist` which is a Linux-specific tool. The module is not explicitly documented as Linux-only, but `iwlist` is not available on macOS or Windows.

- **Priority: Low**: Signal strength parsing at `wireless/mod.rs:168-173` uses `unwrap_or(-100)` which means invalid signal levels default to -100 dBm (very weak). This is a reasonable fallback but could hide parsing errors.

## Stale Items

- **WPA/WPA2 handshake capture**: Document correctly notes this is "aspirational (not yet implemented)" - confirmed at `wireless/mod.rs:5` in docstring.

## Code Interrogation Findings

- **Finding**: The `WirelessScanner::scan()` method at `wireless/mod.rs:81-104` runs `iwlist` as an external command and parses its output. This is inherently platform-dependent (Linux wireless tools) and the output format may vary between system versions.

- **Finding**: The vulnerability analysis at `wireless/mod.rs:209-249` correctly identifies Open networks as Medium severity, WEP as High, and WPA as Medium. This is appropriate severity mapping.

- **Finding**: The `SecurityType::as_str()` method at `wireless/mod.rs:34-46` provides string representations matching the enum variants.

- **Finding**: The `#[cfg(not(feature = "wireless"))]` paths at `wireless/mod.rs:106-111,204-207` return appropriate error messages indicating the feature is required.

## Summary

The wireless module architecture documentation is accurate and concise. All types are correctly documented, and the feature-gated implementation is properly described. The document correctly notes WPA/WPA2 handshake capture is not yet implemented. No critical bugs found - the module is straightforward and well-implemented for its current scope (iwlist parsing and security type analysis).