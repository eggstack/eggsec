# Wireless Module

## Purpose

Passive WiFi network reconnaissance and basic security posture assessment (defense validation / lab use). iwlist-based scanning (Linux), security type parsing (Open/WEP/WPA/WPA2/WPA3/Enterprise/Unknown), WPS/hidden/transition detection, vulnerability analysis (weak/legacy/rogue heuristics), recommendations, and structured output. Feature-gated behind `wireless`. Standalone CLI (`eggsec wireless <iface>`) + TUI tab + report integration. No active attacks or handshake capture in first-handoff phase.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WirelessScanner` | `wireless/mod.rs` | Main wireless scanning engine (scan, parse, analyze) |
| `WirelessNetwork` | `wireless/mod.rs` | Discovered network (SSID, BSSID, channel, security, signal, wps_enabled, is_hidden, transition_mode) |
| `SecurityType` | `wireless/mod.rs` | Enum: Open, WEP, WPA, WPA2, WPA3, Enterprise, Unknown |
| `WirelessScanResult` | `wireless/mod.rs` | Interface + networks + duration + recommendations |
| `WirelessVulnerability` | `wireless/mod.rs` | Finding from analyze_networks (type, severity, desc, rec) |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Core: scanner, models, parse_scan_output (iwlist), analyze_networks (incl. rogue heuristic), generate_recommendations, run_cli, to_scan_report_data |
| `cli/wireless.rs` | WirelessArgs + WIRELESS_ABOUT (repeat, detect_suspicious, warnings) |
| `commands/handlers/wireless.rs` | handle_wireless with EnforcementContext (SafeActive + wireless feature) |
| `eggsec-tui/.../tabs/wireless.rs` | WirelessTab (inputs, results view, task integration) |
| `eggsec-tui/.../workers/security.rs` | run_wireless_task (TUI worker) |
| `eggsec-output/.../convert.rs` | WirelessNetworkReportData + ScanReportData integration (HTML/MD/JSON) |

## Implementation Status (Post First-Handoff)

Usable standalone state achieved per plans/wireless-first-handoff-plan.md:

- Enhanced passive parsing: WPS, hidden SSIDs (normalized to "<hidden>"), WPA2/WPA3 transition/mixed mode.
- Enhanced analysis: weak signal, duplicate-SSID basic rogue/Evil-Twin candidate (passive heuristic, labeled), WPS/transition/hidden findings + original legacy/open/unknown/enterprise.
- Recommendations expanded; always-on "run repeated scans for rogue observation".
- CLI polish: --repeat, upfront root/iwlist/perms warning (unless quiet), per-iteration progress, "Findings / Vulnerabilities:" section in human output, JSON fidelity.
- Rogue/suspicious detection: integrated passive heuristic in analyze_networks (same SSID + differing BSSID/security).
- Output: extended WirelessNetworkReportData (3 new fields, serde defaults); findings feed unified reports.
- Docs: docs/WIRELESS.md (full), architecture/wireless.md, README/CAPABILITIES/SAFETY/AGENTS/skill updates.
- Tests: 12 unit tests (parse + analysis + rogue/weak/etc.) under --features wireless; no hardware required.
- Safety: Policy-enforced (SafeActive); explicit lab/authorized use messaging; passive only.

Not implemented (deferred): active attacks (deauth etc.), handshake/PMKID/WPS PIN, deep WPS, BT/BLE, non-Linux, full pipeline integration (standalone defense-lab command for now; callable under policy from agent/MCP).

See docs/WIRELESS.md for usage, safety, examples; plans/wireless-first-handoff-plan.md for phase goals.
