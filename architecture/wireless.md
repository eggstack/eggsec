# Wireless Module

## Purpose

Standalone-complete passive WiFi network reconnaissance and basic security posture assessment (defense validation / lab use). Linux `iwlist`-based scanning, security type parsing (Open/WEP/WPA/WPA2/WPA3/Enterprise/Unknown), WPS/hidden/transition detection, vulnerability analysis (weak/legacy/rogue heuristics), recommendations, and structured output. Feature-gated behind `wireless`. Exposed as the standalone CLI (`eggsec wireless <iface>`), TUI tab, and report integration. No active attacks or handshake capture.

## CLI Behavior

- Build with `--features wireless` (or `--features full`).
- Real scans require Linux `iwlist` from `wireless-tools`, root or `CAP_NET_ADMIN`, and a wireless interface in managed mode and up.
- Default human output summarizes rogue/suspicious candidates by count and hint; use `--detect_suspicious` to print the full findings list and recommendations.
- `--known-good` suppresses rogue candidates that match the allowlist in human output and repeat-scan summaries.
- `--repeat` adds per-scan diffs plus a temporal summary; `--dry-run` emits planning output without calling `iwlist`.

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

## Status

Standalone completion achieved (2026-06-11). This doc reflects the current passive-only state: summarized rogue output by default, `--detect_suspicious` for full details, `--known-good` for lab baselines, and no active attacks or handshake capture.

See docs/WIRELESS.md for usage/safety/examples/best-practices; plans/wireless-micro-closeout-checklist.md (closeout record), plans/wireless-standalone-completion-plan.md, historical plans/wireless-first-handoff-plan.md, plans/integration-work-plan.md.

## Integration with Reporting Pipeline

Produces local `WirelessScanResult` + findings directly (human/JSON via CLI + TUI). Optional `to_scan_report_data()` bridge (`wireless/mod.rs`, wired via `eggsec-output/convert.rs` for `WirelessNetworkReportData` + `ScanReportData`) converts to canonical `ScanReportData` (findings + full `wireless_networks` list) for SARIF/JUnit/HTML/etc. consumers.

The CLI `report convert` handler includes an auto-bridge: native `--json` output (direct `WirelessScanResult` or `--repeat` wrapped `{last_scan, ...}`) is accepted directly and converted on the fly when the `wireless` feature is enabled. This makes documented flows like `eggsec wireless wlan0 --json -o w.json ; eggsec report convert w.json -f sarif` work without manual pre-processing.

Bridged findings use `wireless-*` categories (e.g. `wireless-rogue`, `wireless-security`, `wireless-wps`, `wireless-hidden`, `wireless-signal`, `wireless-transition`, `wireless-other`); evidence is populated as compact "network=<ssid> bssid=<bssid>"; remediation from recs. Bridge is per-result (on last_scan for repeat-wrapped native JSON). Always analyzes with None for known_good (suppression is native UX only).

**Design decision (standalone completion 2026-06-11)**: Wireless is intentionally a standalone-complete passive defense-lab capability (CLI primary + TUI tab). No `ScanProfile` pipeline stages or dedicated wireless profiles (aspirational only; see `architecture/defense_lab.md` Future Integration and `cli_commands.md` Special Cases). Report integration is an optional lightweight bridge, not mandatory participation in chained pipelines. The bridge always runs rogue analysis (known-good suppression is UX-only for human/repeat output).

See docs/WIRELESS.md (Integration section), CAPABILITIES.md (Lab Defense row), and `crates/eggsec/src/commands/handlers/report.rs`.
