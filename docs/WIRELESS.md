# Wireless Security Testing

Eggsec provides standalone-complete passive WiFi network reconnaissance and basic security posture assessment via the `wireless` feature. This enables defense validation, lab-based assessment of wireless infrastructure, and identification of obviously weak or misconfigured networks.

Passive reconnaissance is the default. Active deauth/disassoc is available under `wireless-advanced` (CLI `deauth` and TUI active mode). Handshake capture remains future work.

**(Phase 0 — complete 2026-06-11; Phase 1 — complete 2026-06-12)**: Standalone-complete passive WiFi recon + rogue heuristic + reporting bridge + TUI tab + policy integration. Active deauth/disassoc is available under `wireless-advanced` in both CLI and TUI, with dry-run default and live confirmation. See the active loadout details (completed).

## Feature Gate

Build with `--features wireless` (or `--features full`) for passive scanning.

Build with `--features wireless-advanced` (or `--features full`) to also enable active deauth/disassoc attacks (Phase 1), exposed through the CLI `deauth` subcommand and the TUI active mode:

```bash
# Passive only
cargo build --release -p eggsec-cli --features wireless

# With active attacks (Phase 1 deauth)
cargo build --release -p eggsec-cli --features wireless-advanced
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
eggsec wireless wlan0 --detect-suspicious

# Help
eggsec wireless --help
```

**Important**: The command prints a clear root/CAP_NET_ADMIN/iwlist permissions warning (unless `--quiet`). Use only in lab/defense-validation contexts.

**Rogue / Suspicious Detection UX**: Analysis for rogue/Evil-Twin candidates (same SSID + differing BSSID or security type) **always runs**. In default human output, only a compact summary line is shown ("Rogue/suspicious candidates: N (use --detect-suspicious to show full details)"). Use `--detect-suspicious` for the full Findings list (with descriptions/recommendations). Use `--known-good` to suppress known-authorized APs from triggering the heuristic (recommended for lab baselines). A short explanatory note is included in output when candidates are present. Severity is Low (BSSID diff) or Medium (security config differences, possible downgrade). Heuristic only — always verify physically or via inventory.

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

## TUI Integration

The Wireless tab in the TUI provides both passive scanning and active attack capabilities.

**Passive Scanning (wireless feature):**
- Enter your wireless interface name and press Enter
- Results display detected networks with SSID, BSSID, channel, security, and signal strength
- Rogue and suspicious network heuristics are applied automatically

**Active Attacks (wireless-advanced feature):**
- Press `a` to toggle between passive and active attack mode
- Active mode shows input fields for: BSSID, Client MAC, Frame Count, Rate Limit
- Press `d` to toggle Dry Run mode (on by default for safety)
- Press Enter to execute — the policy confirmation overlay will appear for live (non-dry-run) attacks; dry-run attacks (default) launch without a prompt
- Active attacks require `OperationRisk::Intrusive` clearance and explicit operator confirmation when live
- Results display findings, evidence, and recommendations
- The active attack runs through the same `TaskConfig::WirelessActive` worker as the CLI path; result updates `set_active_results()` and renders in the same Results view

**Key Bindings:**
- `a` — Toggle active/passive mode
- `d` — Toggle dry-run
- `Enter` — Start scan or attack
- `Escape` — Stop running operation

Navigation, export, and task management follow standard TUI tab patterns (see architecture/tui.md for tab architecture).

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
  Rogue/suspicious candidates: 1 (use --detect-suspicious to show full details)
  Note: Rogue/Evil-Twin detection is a passive heuristic (multiple BSSIDs or security differences for same SSID). Use --known-good for lab baselines; verify with physical survey or asset inventory.

  Recommendations:
  - ...
  - Run repeated scans to observe changes over time for rogue detection.
```

(JSON mode includes full structured data with the three extra booleans per network.)

## Recommended Workflows

- **Lab / defense validation**: Repeated scans (`--repeat`) against known-good APs to baseline "normal" BSSIDs/channels/security; flag deviations.
- **CI / regression**: JSON output + `to_scan_report_data` (or rely on CLI auto-bridge) into SARIF/JUnit for wireless posture checks (e.g. "no open/WEP/WPA in this environment").
- **Rogue hunting (passive)**: Use `--repeat` and review the summarized rogue count in default output, or add `--detect-suspicious` for the full findings list. Cross-check against asset inventory. This is a heuristic only — follow up with authorized physical/radio validation.
- **Reporting**: Pipe native `--json` to `eggsec report convert` (auto-bridged) or consume `ScanReportData` directly via the bridge.

## Best Practices (Lab / Defensive Use)

- **Always run as root (or with CAP_NET_ADMIN)** for real scans; use `--dry-run --json` in CI or unprivileged planning to validate flags/JSON shape without privileges.
- **Use `--known-good`** for your lab environment. Create a file with authorized SSID, BSSID, or "SSID,BSSID" entries (one per line; `#` comments supported). This suppresses false-positive rogue/Evil-Twin candidates for your known APs while still detecting new or changed ones.
- **Use `--repeat`** (e.g. 3–10) with a short `--duration` (5–15s) for monitoring or change detection. Review per-scan "Changes since previous" diffs (new nets, sec changes, signal drift, new rogue candidates) and the final "Scan summary over time".
- **Default rogue output is summarized**: Rogue/Evil-Twin candidates are always analyzed. Human output shows a count + hint by default. Add `--detect-suspicious` when you need the full details + recommendations for triage.
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
sudo eggsec wireless wlan0 --detect-suspicious --repeat 3
```

## Not In Scope (This Phase)

- Active attacks (deauth, disassociation, Evil Twin AP creation, handshake capture, etc.) — Phase 1 deauth/disassoc is now implemented (see "Active Attacks (Phase 1)" below); Phase 2+ handshake capture and Phase 3+ flood/rogue-sim remain future work. Phase 1 design (completed): deauth gated behind `wireless-advanced` feature flag = ["wireless"]; heavily policy-gated with `--allow-active-wireless`, packet budgets, and `Intrusive` risk tier; **MCP/agent tool exposure remains intentionally absent** for the entire wireless surface, including advanced — standalone defense-lab design decision, not registered as `SecurityTool`.
- Handshake capture / PMKID / WPS PIN attacks / KRACK-style testing
- Deep WPS enumeration beyond beacon flags
- Bluetooth/BLE
- Windows/macOS native scanning (iwlist Linux-only)
- Full pipeline integration (wireless is a standalone-complete defense-lab surface; MCP and agentic tool exposure is intentionally absent per design decision — wireless is not registered as a SecurityTool and does not appear in tools/list or agent dispatch). Optional reporting bridge only. See architecture/wireless.md (MCP / Agentic section).

## Active Attacks (Phase 1)

**Build with `--features wireless-advanced`** (or `--features full`).

Phase 1 implements targeted and broadcast deauthentication/disassociation frame injection for defense validation, WIPS/WIDS testing, and AP/client resilience regression.

### Prerequisites

- Root or `CAP_NET_ADMIN` privileges
- Monitor-mode capable wireless interface (e.g., `wlan0mon` created via `airmon-ng start wlan0`)
- Explicit `--allow-active-wireless` flag for non-dry-run execution
- **Only on networks you own or have explicit written authorization to test**

### CLI Usage

```bash
# Dry-run deauth (no frames sent; valid JSON output; no privileges required)
sudo eggsec wireless wlan0 deauth --bssid AA:BB:CC:DD:EE:FF --dry-run --json

# Targeted client deauth (lab only)
sudo eggsec wireless wlan0 deauth \
  --bssid 00:11:22:33:44:55 \
  --client aa:bb:cc:dd:ee:ff \
  --count 30 \
  --reason-code 7 \
  --allow-active-wireless \
  --manual-override-reason "Authorized WIPS regression test on lab AP"

# Broadcast deauth (all clients on AP - higher impact)
sudo eggsec wireless wlan0 deauth --bssid 00:11:22:33:44:55 --broadcast --count 100

# Output to file
eggsec wireless wlan0 deauth --bssid AA:BB:CC:DD:EE:FF --dry-run -o deauth-plan.json
```

### Deauth Arguments

| Argument | Description |
|----------|-------------|
| `--bssid` | Target AP BSSID (required, e.g., `AA:BB:CC:DD:EE:FF`) |
| `--client` | Target client MAC (omit for broadcast deauth to all clients) |
| `--broadcast` | Send to broadcast address (all clients on AP) |
| `--count` | Number of frames to send (default: 50) |
| `--reason-code` | 802.11 reason code (default: 7 = class 3 from nonassociated STA) |
| `--max-frames` | Hard budget cap (default: 100, max enforced) |
| `--fps` | Frame injection rate in frames/sec (default: 10, max enforced) |
| `--dry-run` | Plan mode: show what would be sent without transmitting |
| `--allow-active-wireless` | Required for non-dry-run execution |
| `--json` | JSON output |
| `--output FILE` | Write output to file |
| `--monitor-iface IFACE` | Specify monitor-mode interface |

### What It Does

The deauth subcommand crafts 802.11 deauthentication frames (with radiotap headers) and optionally injects them via a raw socket on a monitor-mode interface. Each frame targets a specific (BSSID, client) pair or broadcasts to all clients of an AP.

### Findings

Produces `wireless-active-deauth` findings with severity High, evidence (frames sent, target, reason code), and remediation (enable 802.11w/PMF, check WIPS detection latency, verify client reconnection).

### Safety & Policy

- **Lab-only**: Active frame injection can disrupt legitimate wireless clients
- **Defense validation**: Intended for WIPS/WIDS regression testing and AP resilience validation
- **Feature gated**: Requires `wireless-advanced` feature (not included in default build)
- **Policy gated**: `OperationRisk::Intrusive` + `required_features: ["wireless-advanced"]`
- **Manual confirmation**: `--allow-active-wireless` required for non-dry-run (audited)
- **Budget enforcement**: Hard caps on max frames (1000) and rate (100 fps)
- **Dry-run safe**: `--dry-run` produces valid JSON without any transmission

See `docs/SAFETY.md`.

### Reporting Bridge

Active attack results can be converted to unified reports (SARIF, JUnit, HTML, Markdown, CSV) via the reporting bridge. The auto-bridge works for both CLI and TUI output:

```bash
# Dry-run deauth → JSON → SARIF report
sudo eggsec wireless wlan0 deauth --bssid AA:BB:CC:DD:EE:FF --dry-run --json -o deauth.json
eggsec report convert deauth.json -f sarif

# Real deauth → HTML report
sudo eggsec wireless wlan0 deauth --bssid AA:BB:CC:DD:EE:FF --count 10 --allow-active-wireless --json -o deauth.json
eggsec report convert deauth.json -f html
```

The bridge produces `wireless-active-*` categories (e.g. `wireless-active-deauth`) for findings. Native `--json` output from `eggsec wireless <iface> deauth` is auto-bridged by `eggsec report convert` when the `wireless-advanced` feature is enabled. TUI-generated results follow the same bridge path when exported to JSON.

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
- Plans: All wireless implementation plans completed — standalone completion, active attacks loadout design, TUI/MCP/agentic integration, CLI integration, and closeout checklist.

## Active Attacks (Future, Phase 2+)

Phase 1 deauth/disassoc is now implemented (see "Active Attacks (Phase 1)" above). Full roadmap:

- **Phase 2**: Handshake capture (PMKID, WPA handshake), WPS PIN enumeration
- **Phase 3+**: Flood attacks, rogue AP simulation, KRACK-style testing

All active paths require `--features wireless-advanced`, runtime confirmations/overrides (`--allow-active-wireless` + reason), packet budgets, and lab-only framing. Reporting bridge extends with `wireless-active-*` categories. MCP/agent tool exposure remains intentionally absent for the entire wireless surface.

Always ensure explicit authorization. Prefer lab environments for development and regression.

## Integration with Reporting Pipeline

`eggsec wireless` is a standalone defense-lab surface: passive scanning lives under `wireless`, and active deauth/disassoc lives under `wireless-advanced` (CLI and TUI active mode). It emits local `WirelessScanResult` (with embedded `WirelessNetwork` list and `recommendations`) directly for human-readable output and `--json`.

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
- Evidence is populated with network identifier plus channel/signal/security for context (e.g. `network=<ssid> bssid=<bssid> ch=6 sig=-60dBm sec=Open`). The compact id prefix is preserved for compatibility.
- Remediation is mapped from the vulnerability recommendation.
- The bridge always runs rogue analysis with `analyze_networks(..., None)` (known-good suppression is for human/repeat UX + diffs only; bridged findings always include rogue candidates).
- Native `--json` (direct `WirelessScanResult` or the `--repeat > 1` wrapped form `{ "last_scan": <WirelessScanResult>, "repeat_count": N, "summary": "..." }`) is accepted directly by `eggsec report convert` (auto-bridged in the report handler when `wireless` feature is present; the bridge is invoked on the inner `last_scan` result for the "last" scan case). The bridge is per-result.
- Design decision (standalone completion 2026-06-11): wireless remains intentionally outside the main `ScanProfile` pipeline and has no dedicated wireless profiles/stages (aspirational only; see `architecture/defense_lab.md`, `architecture/cli_commands.md` Special Cases, and `architecture/wireless.md`).

Use the native types (`WirelessScanResult` / direct `--json`) for lab-specific wireless workflows, repeated-scan temporal summaries, and `--known-good` UX. Use the bridge (or `report convert` on native `--json`) when you need unified reporting consumers (SARIF/JUnit/HTML/Markdown/CSV/trend/etc.). The integration is lightweight and opt-in. "standalone defense-lab" language is deliberate: wireless is a complete standalone CLI/TUI surface for lab/defense use; passive scanning lives under `wireless`, active deauth/disassoc under `wireless-advanced`, and the bridge is only for optional unification of output formats.

This is one of the consolidated "standalone defense-lab surfaces" (wireless + mobile + auth-test). Wireless and mobile provide local types directly + optional `to_scan_report_data` bridge (auto-bridged by `eggsec report convert` when the feature is present); auth-test is local-only (no bridge, no conversion). None participate in `ScanProfile` pipelines or dedicated profiles/stages in this round (aspirational only). See the short shared "Output Models" explanation in `docs/USAGE.md` (Report Management → Convert Reports), `architecture/{wireless,mobile,auth,cli_commands,defense_lab,output}.md`, AGENTS.md (standalone defense-lab surfaces note), and CAPABILITIES.md (Lab Defense table). MCP/agent tool exposure is intentionally absent for wireless (see architecture/wireless.md MCP/Agentic section and the handoff plan resolution).

Active wireless results extend the bridge with `wireless-active-*` findings while preserving the standalone defense-lab + optional bridge model and MCP-absent design.
