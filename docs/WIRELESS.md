# Wireless Security Testing

Eggsec provides standalone-complete passive WiFi network reconnaissance and basic security posture assessment via the `wireless` feature. This enables defense validation, lab-based assessment of wireless infrastructure, and identification of obviously weak or misconfigured networks.

**This is passive reconnaissance only.** No packet injection, deauthentication, handshake capture, or active attacks are implemented in this standalone module.

## Feature Gate

Build with `--features wireless` (or `--features full`).

```bash
cargo build --release -p eggsec-cli --features wireless
```

Runtime requirements (Linux):
- Root (or `CAP_NET_ADMIN`) for `iwlist scan`
- `wireless-tools` package providing `iwlist` (e.g. `sudo apt-get install wireless-tools`)
- Wireless interface in managed mode and up (e.g. `wlan0`)

macOS/Windows: Not supported in this phase (iwlist is Linux-specific).

## Safety & Scope

- Use **only on networks you own or are explicitly authorized to assess** (lab, authorized defensive validation).
- This is **passive listening** — it does not transmit or disrupt.
- Root is required for raw interface access; do not run untrusted code as root.
- Legal/regulatory restrictions on spectrum monitoring may apply in some jurisdictions — know your local rules.
- Production impact: minimal (passive), but repeated scans or suspicious activity can still be noticed by monitoring systems.
- The CLI and TUI both surface prominent warnings about privileges and authorized use.

See also: [docs/SAFETY.md](SAFETY.md), architecture/wireless.md, and the `EnforcementContext` policy gate (handler uses `SafeActive` risk + `wireless` feature requirement).

## CLI Usage

```bash
# Basic scan (default ~10s)
eggsec wireless wlan0

# With JSON output
eggsec wireless wlan0 --json

# Write to file
eggsec wireless wlan0 -o results.json

# Longer scan window
eggsec wireless wlan0 --duration 30

# Repeated scans (useful for observing changes / basic rogue heuristics over time)
eggsec wireless wlan0 --repeat 5 --duration 10

# Quiet (minimal stderr)
eggsec wireless wlan0 -q --json

# Dry-run (plan/CI validation; no privileges or iwlist required; emits valid JSON + notes)
eggsec wireless wlan0 --dry-run --json

# Repeated scans with known-good allowlist (suppresses rogue heuristic for your lab APs)
eggsec wireless wlan0 --repeat 3 --known-good ./lab-aps.txt

# Show full rogue/suspicious details (analysis always runs; default shows count + hint only)
eggsec wireless wlan0 --detect_suspicious

# Help
eggsec wireless --help
```

**Important**: The command prints a clear root/CAP_NET_ADMIN/iwlist permissions warning (unless `--quiet`). Use only in lab/defense-validation contexts.

**Rogue / Suspicious Detection UX**: Analysis for rogue/Evil-Twin candidates (same SSID + differing BSSID or security type) **always runs**. In default human output, only a compact summary line is shown ("Rogue/suspicious candidates: N (use --detect_suspicious to show full details)"). Use `--detect_suspicious` for the full Findings list (with descriptions/recommendations). Use `--known-good` to suppress known-authorized APs from triggering the heuristic (recommended for lab baselines). A short explanatory note is included in output when candidates are present. Severity is Low (BSSID diff) or Medium (security config differences, possible downgrade). Heuristic only — always verify physically or via inventory.

## What It Detects (Passive)

- SSID (network name), BSSID (AP MAC), channel, signal strength (dBm)
- Security type: Open, WEP, WPA, WPA2, WPA3, Enterprise (802.1X), Unknown
- WPS enabled (via beacon/IE indicators in iwlist output)
- Hidden SSIDs (ESSID empty or explicitly hidden)
- WPA2/WPA3 transition mode (mixed mode networks)
- Basic vulnerability findings via `analyze_networks`:
  - Open networks (Medium)
  - WEP (High)
  - WPA (legacy TKIP, Medium)
  - Unknown security (Medium)
  - Enterprise (informational/low; verify RADIUS/EAP/cert)
  - Weak signal (<= -80 dBm)
  - WPS enabled (advisory)
  - Hidden SSID (advisory)
  - Transition mode (advisory)
  - Possible Rogue AP / Evil Twin candidate (passive heuristic: same SSID with differing BSSID or security type across cells; Low severity, labeled as such)
- Recommendations generated for weak/legacy configurations + "Run repeated scans to observe changes over time for rogue detection."

Findings and networks are also exposed via `to_scan_report_data()` for unified reporting (JSON, HTML, Markdown, etc.).

## TUI

The Wireless tab (if built with the feature) provides interactive interface entry + results view with the same data model. Navigation, export, and task management follow standard TUI tab patterns (see architecture/tui.md for tab architecture).

## Output & Integration

- Human-readable text (default) includes networks table + Recommendations + Findings/Vulnerabilities sections.
- `--json` produces the full `WirelessScanResult` (networks + recommendations + metadata). For repeated scans (`--repeat > 1`) this is wrapped as `{ "last_scan": <WirelessScanResult>, "repeat_count": N, "summary": "..." }`. The native shape (direct result or wrapped) is accepted directly by `eggsec report convert` (auto-bridged to `ScanReportData` when the `wireless` feature is enabled).
- File output (`-o`) supported for both modes.
- Structured findings feed into `ScanReportData` (via `to_scan_report_data`) for SARIF/JUnit/HTML/Markdown/etc. pipelines. The bridge is optional and opt-in.
- New fields on `WirelessNetwork` / report data: `wps_enabled`, `is_hidden`, `transition_mode` (serde defaulted for forward compat on old reports).
- Note: `to_scan_report_data()` (used for SARIF/JUnit/HTML/Markdown/etc.) always calls `analyze_networks(..., None)`; rogue/Evil-Twin candidates are therefore always present in structured report findings regardless of `--known-good`. `--known-good` suppression applies to CLI human-readable rogue findings plus repeat-scan diffs/summary, not to the raw report data.

## Data Model (Key Types)

```rust
pub struct WirelessNetwork {
    pub ssid: String,
    pub bssid: String,
    pub channel: u8,
    pub security_type: SecurityType,
    pub signal_strength: i32,
    pub last_seen: String,
    pub wps_enabled: bool,
    pub is_hidden: bool,
    pub transition_mode: bool,
}

pub enum SecurityType { Open, WEP, WPA, WPA2, WPA3, Enterprise, Unknown }

pub struct WirelessScanResult {
    pub interface: String,
    pub networks: Vec<WirelessNetwork>,
    pub scan_duration_secs: u64,
    pub recommendations: Vec<String>,
}

pub struct WirelessVulnerability { /* ... from analyze_networks ... */ }
```

See `crates/eggsec/src/wireless/mod.rs` for full definitions and `WirelessScanner::analyze_networks`.

## Example Output (Human)

```
WARNING: Requires root (or CAP_NET_ADMIN) and 'iwlist' (wireless-tools). ...
Scanning on wlan0 for ~10s...
Wireless Scan Results - Interface: wlan0
Networks found: 3

  1. CorpNet
     BSSID:    00:11:22:33:44:55
     Channel:  6
     Security: WPA2
     Signal:   -62 dBm
     Last seen: 2026-...

  ...

  Findings / Vulnerabilities:
  - Open Network (Medium): ...
  - WPS Enabled (Medium): ...
  Rogue/suspicious candidates: 1 (use --detect_suspicious to show full details)
  Note: Rogue/Evil-Twin detection is a passive heuristic (multiple BSSIDs or security differences for same SSID). Use --known-good for lab baselines; verify with physical survey or asset inventory.

  Recommendations:
  - ...
  - Run repeated scans to observe changes over time for rogue detection.
```

(JSON mode includes full structured data with the three extra booleans per network.)

## Recommended Workflows

- **Lab / defense validation**: Repeated scans (`--repeat`) against known-good APs to baseline "normal" BSSIDs/channels/security; flag deviations.
- **CI / regression**: JSON output + `to_scan_report_data` (or rely on CLI auto-bridge) into SARIF/JUnit for wireless posture checks (e.g. "no open/WEP/WPA in this environment").
- **Rogue hunting (passive)**: Use `--repeat` and review the summarized rogue count in default output, or add `--detect_suspicious` for the full findings list. Cross-check against asset inventory. This is a heuristic only — follow up with authorized physical/radio validation.
- **Reporting**: Pipe native `--json` to `eggsec report convert` (auto-bridged) or consume `ScanReportData` directly via the bridge.

## Best Practices (Lab / Defensive Use)

- **Always run as root (or with CAP_NET_ADMIN)** for real scans; use `--dry-run --json` in CI or unprivileged planning to validate flags/JSON shape without privileges.
- **Use `--known-good`** for your lab environment. Create a file with authorized SSID, BSSID, or "SSID,BSSID" entries (one per line; `#` comments supported). This suppresses false-positive rogue/Evil-Twin candidates for your known APs while still detecting new or changed ones.
- **Use `--repeat`** (e.g. 3–10) with a short `--duration` (5–15s) for monitoring or change detection. Review per-scan "Changes since previous" diffs (new nets, sec changes, signal drift, new rogue candidates) and the final "Scan summary over time".
- **Default rogue output is summarized**: Rogue/Evil-Twin candidates are always analyzed. Human output shows a count + hint by default. Add `--detect_suspicious` when you need the full details + recommendations for triage.
- **JSON for automation**: `--json` (with or without `--repeat`) produces machine-readable `WirelessScanResult` (last successful scan). With `--repeat >1`, a `"summary"` envelope field is included (alongside `last_scan` and `repeat_count`). Pipe to `eggsec report` or your own post-processing.
- **Baseline before hunting**: Run repeated scans in a clean lab state, save `--known-good` + JSON baselines. Re-run later to observe drift or new BSSIDs.
- **Interpret findings conservatively**: Open/WEP/WPA are high-confidence issues. Rogue is a passive heuristic only — same SSID from multiple BSSIDs or security downgrade signals can be legitimate roaming or guest nets; always cross-check MAC inventory or perform physical survey.
- **TUI**: The Wireless tab provides interactive entry + table view. Use for quick visual scans; exports and session features follow standard TUI patterns.
- **Prefer lab environments**: Wireless is a defense-lab / regression tool. Do not use for unauthorized spectrum monitoring. Know your local regulations.

Example practical flows:

```bash
# Single authoritative scan + JSON record
sudo eggsec wireless wlan0 --json -o baseline.json

# Dry-run to validate a CI command shape (no root needed)
eggsec wireless wlan0 --dry-run --repeat 3 --json

# Lab monitoring with known-good baseline (repeat 5x, 10s each)
sudo eggsec wireless wlan0 --repeat 5 --duration 10 --known-good ./authorized-aps.txt

# Full rogue details on demand
sudo eggsec wireless wlan0 --detect_suspicious --repeat 3
```

## Not In Scope (This Phase)

- Active attacks (deauth, disassociation, Evil Twin AP creation)
- Handshake capture / PMKID / WPS PIN attacks / KRACK-style testing
- Deep WPS enumeration beyond beacon flags
- Bluetooth/BLE
- Windows/macOS native scanning (iwlist Linux-only)
- Full pipeline integration (wireless is a standalone-complete defense-lab command; can be called from agent/MCP under policy). Optional reporting bridge only.

Future phases may add a `wireless-advanced` sub-feature for gated active/lab-only capabilities.

## Troubleshooting

- "iwlist: command not found" or permission denied: install wireless-tools; run as root or grant CAP_NET_ADMIN; ensure interface exists and is up (`ip link show`).
- No networks seen: wrong interface, interface down, regulatory domain restrictions, or very short duration.
- "No wireless interface specified": pass the interface name (e.g. `wlan0`, `wlp3s0`).
- TUI wireless tab not visible: rebuild TUI/CLI with `--features wireless`.
- Tests: Unit tests cover parsing and analysis (no hardware required). Run with `--features wireless`.

## References

- Source: `crates/eggsec/src/wireless/mod.rs`
- CLI: `crates/eggsec/src/cli/wireless.rs`
- Handler/policy: `crates/eggsec/src/commands/handlers/wireless.rs`
- TUI tab: `crates/eggsec-tui/src/tabs/wireless.rs`
- Output conversion: `crates/eggsec-output/src/convert.rs`
- Architecture: `architecture/wireless.md`
- Agent skill: `.opencode/skills/eggsec-agent/wireless_security_testing.md`
- Plan: `plans/wireless-micro-closeout-checklist.md` (closeout record); `plans/wireless-standalone-completion-plan.md` (standalone completion); historical: `plans/wireless-first-handoff-plan.md` (first handoff); `plans/integration-work-plan.md`

Always ensure explicit authorization. Prefer lab environments for development and regression.

## Integration with Reporting Pipeline

`eggsec wireless` is a **standalone-complete** defense-lab surface (CLI + optional TUI tab under the `wireless` feature). It emits local `WirelessScanResult` (with embedded `WirelessNetwork` list and `recommendations`) directly for human-readable output and `--json`.

An optional `to_scan_report_data()` bridge (in `wireless/mod.rs`) converts findings (from `analyze_networks`) + the full networks list into canonical `ScanReportData` (with `wireless_networks: Vec<WirelessNetworkReportData>` and `findings`). This enables SARIF, JUnit, HTML (with networks table), Markdown, CSV, JSON, trend, etc. via `eggsec-output`.

- Native `--json` (or the `--repeat` wrapped form with `last_scan`) is accepted directly by `eggsec report convert` (auto-bridged in the report handler when `wireless` feature is present). This fulfills the documented "CI / regression" and "Reporting" flows.
- Categories in the bridged findings use consistent `wireless-*` naming based on vulnerability type:
  - `wireless-rogue` for "Possible Rogue AP / Evil Twin (passive heuristic)"
  - `wireless-security` for Open Network / WEP Encryption / WPA Encryption
  - `wireless-wps` for "WPS Enabled"
  - `wireless-hidden` for "Hidden SSID"
  - `wireless-signal` for "Weak Signal Strength"
  - `wireless-transition` for "WPA2/WPA3 Transition Mode"
  - `wireless-other` for Enterprise / Unknown Security / other
- Evidence is populated as a compact network identifier: e.g. `network=<ssid> bssid=<bssid>`.
- Remediation is mapped from the vulnerability recommendation.
- The bridge always runs rogue analysis with `analyze_networks(..., None)` (known-good suppression is for human/repeat UX + diffs only; bridged findings always include rogue candidates).
- Native `--json` (direct `WirelessScanResult` or the `--repeat > 1` wrapped form `{ "last_scan": <WirelessScanResult>, "repeat_count": N, "summary": "..." }`) is accepted directly by `eggsec report convert` (auto-bridged in the report handler when `wireless` feature is present; the bridge is invoked on the inner `last_scan` result for the "last" scan case). The bridge is per-result.
- Design decision (standalone completion 2026-06-11): wireless remains intentionally outside the main `ScanProfile` pipeline and has no dedicated wireless profiles/stages (aspirational only; see `architecture/defense_lab.md`, `architecture/cli_commands.md` Special Cases, and `architecture/wireless.md`).

Use the native types (`WirelessScanResult` / direct `--json`) for lab-specific wireless workflows, repeated-scan temporal summaries, and `--known-good` UX. Use the bridge (or `report convert` on native `--json`) when you need unified reporting consumers (SARIF/JUnit/HTML/Markdown/CSV/trend/etc.). The integration is lightweight and opt-in. "standalone defense-lab" language is deliberate: wireless is a complete standalone CLI (with optional TUI tab) for passive lab/defense use; the bridge is only for optional unification of output formats.
