---
name: eggsec-wireless
description: "WiFi security analysis - use when working with wireless network discovery, passive scanning, network security analysis, or active wireless attack primitives (lab-only)."
---

# Eggsec Wireless Skill

Wireless security analysis: passive WiFi reconnaissance plus lab-only active attack primitives.

## Module Location

`crates/eggsec/src/wireless/` (single `mod.rs`, ~60KB; `active/` submodule for advanced primitives)

## Feature Gates

- `wireless` - Passive scanning and security analysis; system dep `wireless-tools`; real scans want root
- `wireless-advanced` - Active attacks (deauth/disassoc); requires `wireless`; policy-gated Intrusive, lab-only

## Key Types

- `WirelessNetwork` - Discovered network entry
- `SecurityType` - Encryption classification (`as_str()` for display)
- `WirelessScanResult` - Aggregated scan output
- `WirelessVulnerability` - Detected weakness
- `WirelessScanner` - Builder-style scanner: `new()`, `with_interface()`, `scan()`, `analyze_networks()`
- `to_scan_report_data()` - Bridge to unified reporting

## CLI Integration

- Command: `eggsec wireless <args>` (feature-gated)
- Handler: `commands/handlers/wireless.rs`
- CLI args: `cli/wireless.rs`

## Policy Notes

- Passive scans are SafeActive at most; `wireless-advanced` operations are Intrusive and require explicit authorization + lab environment
- Never run active primitives outside owned test labs

## Testing

```bash
cargo check -p eggsec --features wireless
cargo check -p eggsec --features wireless-advanced
cargo test --lib -p eggsec wireless::
```

Hardware-dependent tests must be feature-gated and skipped without an interface.

## Resources

- `crates/eggsec/src/wireless/AGENTS.override.md` - Module guidance
- `docs/WIRELESS.md` - Full usage documentation
- `architecture/wireless.md` - Architecture deep dive
