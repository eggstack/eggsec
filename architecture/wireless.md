# Wireless Module

## Role & Responsibilities

Standalone-complete passive WiFi network reconnaissance and active defense-validation attack primitives (lab-only). The module provides:

- **Passive scanning** (`wireless` feature): Linux `iwlist`-based scanning, security type parsing (7 types), WPS/hidden/transition detection, rogue-AP/evil-twin heuristics, known-good suppression, temporal scan diffing, and structured output.
- **Active attacks** (`wireless-advanced` feature): 802.11 deauthentication and disassociation frame crafting and injection via raw sockets. Broadcast and targeted modes. Dry-run safe.

**Safety posture**: Passive scanning is the base surface. Active attacks are opt-in (`wireless-advanced`), require root or `CAP_NET_ADMIN`, monitor-mode interface, and explicit `--allow-active-wireless` flag (or policy confirmation for non-dry-run). MCP/agent tool exposure is intentionally absent — wireless is a standalone defense-lab surface only.

## Location & Feature Gating

| Component | Feature gate | `cfg` line |
|-----------|-------------|------------|
| Passive scan (`WirelessScanner::scan`) | `wireless` | `wireless/mod.rs:86` |
| Active attacks module | `wireless-advanced` | `wireless/mod.rs:9` (`#[cfg(feature = "wireless-advanced")] pub mod active;`) |
| CLI handler scan path | `cli` | `commands/handlers/wireless.rs:11` |
| CLI handler deauth path | `wireless-advanced` | `commands/handlers/wireless.rs:7` (`#[cfg(feature = "wireless-advanced")]`) |
| TUI integration | `wireless` (passive) + `wireless-advanced` (active) | `eggsec-tui/src/tabs/wireless.rs` |

## Architecture

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WirelessScanner` | `wireless/mod.rs:72` | Main scanning engine. `new()`, `with_interface()`, `scan()` (async, iwlist), `parse_scan_output()`, `analyze_networks()`, `generate_recommendations()` |
| `WirelessNetwork` | `wireless/mod.rs:17` | Discovered network: `ssid`, `bssid`, `channel`, `security_type`, `signal_strength`, `last_seen`, `wps_enabled`, `is_hidden`, `transition_mode` (9 fields) |
| `SecurityType` | `wireless/mod.rs:30` | Enum: `Open`, `WEP`, `WPA`, `WPA2`, `WPA3`, `Enterprise`, `Unknown` — **7 variants** |
| `WirelessScanResult` | `wireless/mod.rs:54` | Scan output: `interface`, `networks`, `scan_duration_secs`, `recommendations` |
| `WirelessVulnerability` | `wireless/mod.rs:62` | Finding from `analyze_networks`: `ssid`, `bssid`, `vulnerability_type`, `severity`, `description`, `recommendation` |
| `ActiveAttackConfig` | `wireless/active/mod.rs:68` | Attack configuration: `interface`, `bssid` (`Option<[u8; 6]>`), `client`, `reason_code`, `max_frames`, `frames_per_second`, `dry_run` |
| `ActiveWirelessAttackResult` | `wireless/active/mod.rs:29` | Attack result: `interface`, `attack_type`, `target_bssid`, `target_client`, `frames_sent`, `duration_secs`, `dry_run`, `findings`, `raw_output`, `recommendations` |
| `ActiveWirelessFinding` | `wireless/active/mod.rs:53` | Finding from active attack: `attack_type`, `severity`, `description`, `evidence`, `remediation` |

### Files

| File | Description |
|------|-------------|
| `wireless/mod.rs` | Core: `WirelessScanner`, models, `parse_scan_output` (iwlist parser), `analyze_networks` (incl. rogue heuristic), `generate_recommendations`, `run_cli`, `to_scan_report_data`, `compute_changes_since`, `build_temporal_summary`, `load_known_good`, `is_known_good`, `wireless_category_for` |
| `wireless/active/mod.rs` | Active types: `ActiveAttackConfig`, `ActiveWirelessAttackResult`, `ActiveWirelessFinding`, `to_active_scan_report_data` bridge; MAC parse/format helpers |
| `wireless/active/attacks/mod.rs` | Attacks submodule re-exports |
| `wireless/active/attacks/deauth.rs` | 802.11 deauth/disassoc frame crafting (`build_deauth_frame`, `build_disassoc_frame`), raw socket injection (`inject_frames`), attack runners (`run_deauth`, `run_disassoc`), `reason_codes` module (9 constants), `RawSocketFd` RAII wrapper |
| `cli/wireless.rs` | `WirelessArgs`, `WirelessScanArgs`, `DeauthArgs`, `WirelessSubcommand` enum, `WIRELESS_ABOUT` help text |
| `commands/handlers/wireless.rs` | `handle_wireless`, `handle_scan`, `handle_deauth` with `EnforcementContext` |
| `eggsec-tui/.../tabs/wireless.rs` | `WirelessTab` (passive + active mode, input fields, results view) |
| `eggsec-tui/.../workers/security.rs` | `run_wireless_task`, `run_wireless_active_task` (TUI workers) |
| `eggsec-output/.../convert.rs` | `WirelessNetworkReportData` + `ScanReportData` integration (HTML/MD/JSON/SARIF/JUnit) |

## Behavior / Flow

### Passive Scan Lifecycle

1. **Invoke**: `eggsec wireless <iface>` → `handle_wireless()` → `handle_scan()`
2. **Policy gate**: `EnforcementContext::evaluate()` with `OperationRisk::SafeActive`, `required_features: ["wireless"]`; no `requires_explicit_scope` (target is local interface name)
3. **Scan**: `WirelessScanner::scan()` spawns `iwlist <iface> scan` via `tokio::process::Command` (`mod.rs:87`)
4. **Parse**: `parse_scan_output()` (`mod.rs:152`) iterates iwlist line-by-line. State resets per `Cell` line. Handles:
   - `Address:` → BSSID
   - `ESSID:"..."` → SSID (empty/`<hidden>`/`""` normalized to `<hidden>`)
   - `Channel:` → channel
   - `Signal level=` or `Signal level:` → signal strength (dBm)
   - `WPS` / `Wi-Fi Protected Setup` → WPS enabled
   - `WPA2/WPA3` / `transition` → transition mode
   - `Encryption key:off` → Open; `WPA3`/`WPA2`/`WPA`/`WEP` → security type
   - `Authentication Suites: 802.1X` → Enterprise
   - State also tracks `saw_wpa2`/`saw_wpa3` for transition detection (both present → `transition_mode = true`)
   - Incomplete cells (missing SSID or BSSID) are skipped with a warning
5. **Analyze**: `analyze_networks()` (`mod.rs:324`) generates `WirelessVulnerability` findings:
   - Signal ≤ -80 dBm → "Weak Signal Strength" (Low if ≤ -90, else Medium)
   - WPS enabled → "WPS Enabled" (Medium)
   - Hidden SSID → "Hidden SSID" (Low)
   - Transition mode → "WPA2/WPA3 Transition Mode" (Low)
   - Security type findings: Open (Medium), WEP (High), WPA (Medium), Enterprise (Low), Unknown (Medium)
   - **Rogue AP / Evil Twin heuristic** (`mod.rs:456-511`): Groups networks by SSID; if ≥2 distinct BSSIDs or ≥2 distinct security configs, emits a rogue candidate. Severity: Medium if security diff, Low otherwise. Description includes BSSID list and explicit "passive heuristic" caveat. Suppressed if any network in the group matches `known_good` (by SSID, BSSID, or `"SSID,BSSID"` format)
6. **Recommendations**: `generate_recommendations()` (`mod.rs:516`) deduplicates by security type and per-BSSID for WPS/transition/hidden/weak, using `FxHashSet`
7. **Output**: Human (default: rogue candidates summarized by count; `--detect-suspicious` for full list) or JSON

### Temporal Analysis (Repeat Scans)

`compute_changes_since()` (`mod.rs:916-998`) and `build_temporal_summary()` (`mod.rs:1001-1066`) implement repeated-scan diffing:

- **New networks**: SSID+BSSID not in previous scan
- **Security changes**: Same BSSID, different `SecurityType`
- **Signal drifts**: Same BSSID, signal delta > 5 dBm
- **New rogue candidates**: Re-analyze both scans with known-good suppression; compare rogue SSID sets

`build_temporal_summary()` tracks across all scans: unique SSIDs, scans with new networks, total security changes, total signal drifts, total rogue candidates.

### Active Attack Frame Construction (Deauth/Disassoc)

`deauth.rs` constructs raw 802.11 management frames:

**Frame byte layout** (34 bytes total):
```
[0..8]    Radiotap header (8 bytes minimal: version=0x00, length=0x08, padding, no present flags)
[8..10]   Frame Control (2 bytes LE): 0xC000 (deauth, subtype 12) or 0xA000 (disassoc, subtype 10)
[10..12]  Duration (2 bytes LE, always 0)
[12..18]  Address 1 (6 bytes): destination (client MAC or broadcast FF:FF:FF:FF:FF:FF)
[18..24]  Address 2 (6 bytes): source/BSSID
[24..30]  Address 3 (6 bytes): BSSID
[30..32]  Sequence Control (2 bytes LE, always 0)
[32..34]  Reason Code (2 bytes LE)
```

**9 IEEE 802.11 Reason Codes** (`deauth.rs:26-45`):
1. `UNSPECIFIED` (1)
2. `AUTH_INVALID` (2)
3. `STA_LEAVING` (3)
4. `INACTIVITY` (4)
5. `AP_BUSY` (5)
6. `CLASS2_FROM_UNAUTH` (6)
7. `CLASS3_FROM_UNASSOC` (7)
8. `BSS_LEAVING` (8)
9. `STA_NOT_AUTH` (9)

**Injection**: `inject_frames()` (`deauth.rs:162-264`) opens `AF_PACKET/SOCK_RAW/ETH_P_ALL` raw socket via FFI, constructs `sockaddr_ll` with interface index, and sends frames via `sendto()`. Rate-limited by tokio interval. Linux-only; other platforms bail with error. Socket is RAII-closed via `RawSocketFd::drop()`.

**Attack runners**: `run_deauth()` (`deauth.rs:270`) and `run_disassoc()` (`deauth.rs:381`) build N frames (from `max_frames`), optionally inject them, and return `ActiveWirelessAttackResult` with findings and remediation advice (WIDS/WIPS verification, 802.11w PMF recommendation).

**Safety constraints** (enforced in `commands/handlers/wireless.rs:90-96`):
- Non-dry-run requires `--allow-active-wireless` flag
- `max_frames` hard-capped to 1000, `frames_per_second` capped to 100
- Policy gate: `OperationRisk::Intrusive` + `OperationMode::DefenseLab`
- Dry-run is default; no frames transmitted

## Public API

### Library (`wireless/mod.rs`)

- `WirelessScanner::new() -> Self`
- `WirelessScanner::with_interface(self, String) -> Self`
- `WirelessScanner::scan(&self, duration_secs: u64) -> Result<WirelessScanResult>` (feature `wireless`)
- `WirelessScanner::analyze_networks(networks, known_good) -> Vec<WirelessVulnerability>` (public, static)
- `to_scan_report_data(result: &WirelessScanResult) -> ScanReportData` (report bridge)
- `SecurityType::as_str(&self) -> &str`

### Active (`wireless/active/mod.rs`)

- `ActiveAttackConfig::parse_mac(&str) -> Option<[u8; 6]>`
- `ActiveAttackConfig::format_mac(&[u8; 6]) -> String`
- `to_active_scan_report_data(result) -> ScanReportData` (active report bridge)

### Active Attacks (`wireless/active/attacks/deauth.rs`)

- `build_deauth_frame(bssid, client, reason_code) -> Vec<u8>`
- `build_disassoc_frame(bssid, client, reason_code) -> Vec<u8>`
- `inject_frames(interface, frames, frames_per_second) -> Result<u64>`
- `run_deauth(config, broadcast) -> Result<ActiveWirelessAttackResult>`
- `run_disassoc(config, broadcast) -> Result<ActiveWirelessAttackResult>`

## Integration Points

### CLI

- `eggsec wireless <iface>` → scan (default)
- `eggsec wireless <iface> scan` → explicit scan
- `eggsec wireless <iface> deauth --bssid <MAC>` → active deauth (requires `wireless-advanced`)
- Scan args: `--json`, `--repeat N`, `--duration SECS`, `--detect-suspicious`, `--known-good FILE`, `--dry-run`, `--output FILE`, `--quiet`
- Deauth args: `--bssid`, `--client`, `--count`, `--reason-code`, `--broadcast`, `--max-frames`, `--fps`, `--dry-run`, `--allow-active-wireless`, `--json`, `--output`

### TUI

- WirelessTab supports passive scan and active deauth/disassoc via `a` key toggle
- Active mode: input fields for BSSID, Client MAC, Frame Count, Rate Limit; dry-run via `d` (default on)
- Policy confirmation overlay for live (non-dry-run) active attacks
- Task system integration: `TaskConfig::WirelessActive` → `run_wireless_active_task` → `TaskResult::WirelessActive`

### Enforcement Gating

| Path | Risk | Features | Overrides |
|------|------|----------|-----------|
| CLI scan | `SafeActive` | `["wireless"]` | ManualPermissive allows |
| CLI deauth | `Intrusive` | `["wireless-advanced"]` | ManualPermissive, but non-dry-run requires `--allow-active-wireless` |
| TUI passive | `SafeActive` | `["wireless"]` | Via `TabSpec` |
| TUI active | `Intrusive` | `["wireless-advanced"]` | Via `TabSpec` + `PendingPolicyConfirmation` |

### Reporting Pipeline

- Passive: `to_scan_report_data()` converts `WirelessScanResult` → `ScanReportData` (findings + `wireless_networks`). Bridge categories: `wireless-rogue`, `wireless-security`, `wireless-wps`, `wireless-hidden`, `wireless-signal`, `wireless-transition`, `wireless-other`
- Active: `to_active_scan_report_data()` converts `ActiveWirelessAttackResult` → `ScanReportData` with `wireless-active-*` categories
- CLI `report convert` auto-bridges native JSON output for both passive and active
- Bridge always runs rogue analysis with `known_good=None` (suppression is UX-only for human/repeat output)
- Evidence format: `network=<ssid> bssid=<bssid> ch=<n> sig=<n>dBm sec=<type>`

### ProbeIntent / ProbeRisk

Wireless does not use `ProbeIntent`/`ProbeRisk` — it is a standalone defense-lab surface outside the `ScanProfile` pipeline. Risk classification is handled directly via `OperationRisk` in the `OperationDescriptor`.

## Platform Requirements

| Requirement | Passive | Active |
|-------------|---------|--------|
| OS | Linux (iwlist) | Linux (raw socket AF_PACKET) |
| Privileges | root or `CAP_NET_ADMIN` | root or `CAP_NET_ADMIN` |
| External tool | `iwlist` from `wireless-tools` | None (pure-Rust frame construction) |
| Interface mode | Managed mode, up | Monitor mode |
| Build flag | `--features wireless` | `--features wireless-advanced` |

## Testing

### Unit Tests (`wireless/mod.rs:1068-1568`)

14 tests covering:
- `SecurityType::as_str()` round-trip
- `WirelessScanner` creation
- `analyze_networks` with open/WPA3 networks
- `parse_scan_output` per-cell state reset, open/enterprise/WPS/hidden/transition/mixed-WPA2-WPA3
- Weak signal detection
- Rogue candidate detection (multi-BSSID + security diff)
- Known-good suppression (by SSID, BSSID, and "SSID,BSSID" format)
- Empty known-good set (rogue not suppressed)
- `to_scan_report_data` bridge validation (evidence format, categories, serde roundtrip)

### Active Tests (`wireless/active/mod.rs:168-297`)

7 tests: MAC parse/format, serde roundtrips for `ActiveWirelessAttackResult` and `ActiveWirelessFinding`, `to_active_scan_report_data` bridge with and without BSSID.

### Deauth Tests (`wireless/active/attacks/deauth.rs:475-555`)

11 tests: frame length (34 bytes), broadcast vs targeted addresses, frame control fields (0xC000 deauth, 0xA000 disassoc), radiotap header, reason code encoding, multi-frame batch building.

## Invariants & Gotchas

1. **Standalone defense-lab**: Wireless is intentionally not a `SecurityTool` — no MCP/agent registration, no `ScanProfile` pipeline participation.
2. **Known-good suppression is UX-only**: The report bridge always analyzes with `None` for `known_good` — suppression only affects human output and repeat-scan summaries.
3. **Hidden SSID normalization**: Empty ESSID, `"<hidden>"`, and `'""'` all normalize to `"<hidden>"` with `is_hidden=true`.
4. **Transition mode dual detection**: Set if iwlist line contains `"WPA2/WPA3"` or `"transition"`, OR if both `saw_wpa2` and `saw_wpa3` are true within a single Cell block.
5. **Malformed cell skipping**: If a `Cell` line appears while previous SSID or BSSID is present but the pair is incomplete, the entry is skipped with a warning.
6. **Frame injection is Linux-only**: Non-Linux platforms get `anyhow::bail!` from `inject_frames()`.
7. **Budget caps**: CLI handler enforces `max_frames.min(1000)` and `fps.min(100)` regardless of user input.
8. **Dry-run default**: Active attacks default to dry-run; `--allow-active-wireless` only required for live execution.
9. **Signal drift threshold**: Temporal diffing uses 5 dBm absolute delta — configurable in logic but not exposed as CLI flag.
10. **`FxHashSet` in recommendations**: Deduplication uses `rustc_hash::FxHashSet` for performance.

---

See also: [overview.md](overview.md), [probe.md](probe.md), [stress.md](stress.md), [defense_lab.md](defense_lab.md)

*Last verified against source: 2026-08-25*
