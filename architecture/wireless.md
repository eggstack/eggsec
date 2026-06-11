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

## Implementation Status

Standalone completion achieved (2026-06-11) per plans/wireless-standalone-completion-plan.md. Final polish (per plans/wireless-remaining-work-plan.md 2026-06-11): comprehensive docs updates (WIRELESS.md Best Practices + expanded examples/rogue UX clarification; README/CAPABILITIES/SAFETY/AGENTS/architecture/skill updates); rogue UX refinement (summarized by default in human output with count/hint + short explanation; --detect_suspicious for full details; analysis always runs; known-good suppression); robustness (repeat error continuity already solid; added repeat metadata to JSON --repeat path for summary convenience; --help/docs verified complete). Still passive-only, Linux/iwlist, SafeActive + feature-gated, no active attacks or full pipeline integration.

See docs/WIRELESS.md for usage/safety/examples/best-practices; plans/wireless-remaining-work-plan.md (final close-out), plans/wireless-standalone-completion-plan.md, historical plans/wireless-first-handoff-plan.md.
