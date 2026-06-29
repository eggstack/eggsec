# Wireless Module

## Purpose

Standalone-complete passive WiFi network reconnaissance and basic security posture assessment (defense validation / lab use). Linux `iwlist`-based scanning, security type parsing (Open/WEP/WPA/WPA2/WPA3/Enterprise/Unknown), WPS/hidden/transition detection, vulnerability analysis (weak/legacy/rogue heuristics), recommendations, and structured output. Feature-gated behind `wireless`. Exposed as the standalone CLI (`eggsec wireless <iface>`), TUI tab, and report integration. Passive scanning is the base surface; active deauth/disassoc is available under `wireless-advanced`, and handshake capture remains future work.

## CLI Behavior

- Build with `--features wireless` (or `--features full`).
- Real scans require Linux `iwlist` from `wireless-tools`, root or `CAP_NET_ADMIN`, and a wireless interface in managed mode and up.
- Default human output summarizes rogue/suspicious candidates by count and hint; use `--detect-suspicious` to print the full findings list and recommendations.
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
| `cli/wireless.rs` | WirelessArgs + WIRELESS_ABOUT (repeat, detect-suspicious, warnings) |
| `commands/handlers/wireless.rs` | handle_wireless with EnforcementContext (SafeActive + wireless feature) |
| `active/mod.rs` | ActiveWirelessAttackResult, ActiveWirelessFinding, ActiveAttackConfig, re-exports attacks |
| `active/attacks/mod.rs` | attacks submodule |
| `active/attacks/deauth.rs` | Deauth/disassoc frame builders (build_deauth_frame, build_disassoc_frame), inject_frames, run_deauth, run_disassoc |
| `active/mod.rs` (+bridge) | `to_active_scan_report_data` bridge: converts `ActiveWirelessAttackResult` → `ScanReportData` for SARIF/JUnit/HTML/Markdown/etc. |
| `eggsec-tui/.../tabs/wireless.rs` | WirelessTab (inputs, results view, task integration) |
| `eggsec-tui/.../workers/security.rs` | run_wireless_task (TUI worker) |
| `eggsec-output/.../convert.rs` | WirelessNetworkReportData + ScanReportData integration (HTML/MD/JSON) |

## Status

**Phase 0 (passive) complete (2026-06-11)**. **Phase 1 (active attacks, wireless-advanced) implemented; active reporting bridge complete (CLI + TUI integration)**. This doc reflects passive + Phase 1 active state: passive uses summarized rogue output by default, `--detect-suspicious` for full details, `--known-good` for lab baselines; active adds `eggsec wireless <iface> deauth` for deauth/disassoc frame injection (lab-only, requires `--allow-active-wireless`) and the TUI active mode with the same task/confirmation flow. Active results are auto-bridged for SARIF/JUnit/HTML/Markdown reporting via `report convert`. Active attacks under `wireless-advanced` feature follow the same standalone defense-lab rule: MCP/agent tool exposure intentionally absent (per plan recommendation). See `plans/wireless-active-attacks-loadout-design-plan.md` (Phase 1 loadout). (Resolution cross-ref: `plans/wireless-active-tui-final-wiring-and-polish-plan.md` closed 2026-06-12 after doc verification + cleanup pass.)

## MCP / Agentic / Tool Integration Status (post wireless-tui-mcp-agentic-handoff-plan 2026-06-11; see plan resolution note)

Wireless is a **standalone defense-lab surface** (CLI primary + optional TUI tab under the `wireless` feature). It is **not registered** as a `SecurityTool` in the tool registry (`tool/mod.rs`, `tool/registry.rs`) and is therefore **not listed or callable** via the MCP `tools/list` / `tools/call` surface (or agentic dispatch).

- Policy enforcement for the CLI command (`commands/handlers/wireless.rs`) uses the central `CommandContext::evaluate_and_enforce_operation` with `OperationRisk::SafeActive` + `required_features: ["wireless"]` (no `requires_explicit_scope` because the "target" is a local interface name, not a network host).
- The TUI tab participates in the same enforcement model via `TabSpec` (risk_group SafeActive, feature="wireless", direct_launch=true, operation="wireless") + `App::build_current_operation_descriptor` (which now propagates `spec.feature` into `required_features` for parity with CLI descriptors) + pre-dispatch policy eval in handle_enter + shared `EnforcementContext` / `PendingPolicyConfirmation` / preflight.
- **TUI integration (Phase 0 + Phase 1 complete)**: The Wireless tab in the TUI supports both passive scanning (`wireless` feature) and active attacks — deauth/disassoc (`wireless-advanced` feature). Active mode toggles via `a` key; input fields for BSSID, Client MAC, Frame Count, and Rate Limit; dry-run toggled via `d` (on by default). Pressing Enter in `ActiveConfig` focus with a valid `active_attack_config()` launches the attack via the shared task system (`TaskConfig::WirelessActive` → `run_wireless_active_task` worker → `TaskResult::WirelessActive` → `set_active_results`). Active attacks (live, non-dry-run) trigger the policy confirmation overlay (`OperationRisk::Intrusive` + `OperationMode::DefenseLab` + explicit operator confirmation required); dry-run attacks (default) execute without a confirmation prompt. Results display findings, evidence, and recommendations for both scan and attack results.
- In strict profiles (`McpStrict`, `AgentStrict`, `CiStrict`) the feature gate + explicit LoadedScope provenance rules apply if ever invoked; currently the only supported invocation path is the CLI handler (or direct library use under the same `EnforcementContext`).
- This mirrors the consolidated "standalone defense-lab surfaces" pattern (wireless + mobile + auth-test). Wireless and mobile emit local types directly with an optional `to_scan_report_data` bridge (and CLI auto-bridge in `report convert`); auth-test is local-only with no bridge. None participate in `ScanProfile` pipelines or dedicated profiles/stages in this round. See `architecture/defense_lab.md`, `architecture/cli_commands.md` (Special Cases), `architecture/output.md`, `docs/USAGE.md` (Output Models block), AGENTS.md (standalone defense-lab surfaces note), and the handoff plan resolution note.
- If future work adds a `WirelessTool` impl + registry entry, it would also need updates in `tool/protocol/mcp/policy.rs` (classify_tool_risk, required_capabilities_for_tool_call, infer_tool_category, CodingAgent allowlist consideration) + special target handling for interface names, plus MCP handler tests. No such registration is planned in the current round (design decision: keep wireless as a focused passive defense-lab CLI/TUI capability; MCP/agent tool exposure intentionally absent).
- Active attacks (implemented under `wireless-advanced`) follow the same standalone defense-lab rule: **MCP/agent tool exposure intentionally absent** (per plan recommendation; no `SecurityTool` registration). See `plans/wireless-active-attacks-loadout-design-plan.md` (non-goals + open questions).

The optional `to_scan_report_data` bridge (and CLI `report convert` auto-bridge) works for any consumer that obtains a native JSON `WirelessScanResult` (or repeat-wrapped form), regardless of invocation surface.

See `docs/WIRELESS.md` for usage/safety/examples/best-practices.

## Integration with Reporting Pipeline

Produces local `WirelessScanResult` + findings directly (human/JSON via CLI + TUI). Optional `to_scan_report_data()` bridge (`wireless/mod.rs`, wired via `eggsec-output/convert.rs` for `WirelessNetworkReportData` + `ScanReportData`) converts to canonical `ScanReportData` (findings + full `wireless_networks` list) for SARIF/JUnit/HTML/etc. consumers.

The CLI `report convert` handler includes an auto-bridge: native `--json` output (direct `WirelessScanResult` or `--repeat` wrapped `{last_scan, ...}`) is accepted directly and converted on the fly when the `wireless` feature is enabled. This makes documented flows like `eggsec wireless wlan0 --json -o w.json ; eggsec report convert w.json -f sarif` work without manual pre-processing. TUI-generated results follow the same bridge path when exported to JSON.

Active attack results (`ActiveWirelessAttackResult` from `eggsec wireless <iface> deauth --json`) are also auto-bridged by `eggsec report convert` when the `wireless-advanced` feature is enabled. The bridge produces `wireless-active-*` categories (e.g. `wireless-active-deauth`) for findings, enabling SARIF/JUnit/HTML/Markdown/CSV reporting for active results. Both CLI and TUI active attack output use this same reporting path.

Bridged findings use `wireless-*` categories (e.g. `wireless-rogue`, `wireless-security`, `wireless-wps`, `wireless-hidden`, `wireless-signal`, `wireless-transition`, `wireless-other`); evidence is populated as `network=<ssid> bssid=<bssid> ch=... sig=...dBm sec=...` (richer than bare id while keeping prefix for compatibility); remediation from recs. Bridge is per-result (on last_scan for repeat-wrapped native JSON). Always analyzes with None for known_good (suppression is native UX only). Timestamp in bridged `ScanReportData` is report generation time; per-net `last_seen` carries scan time.

**Design decision (standalone completion 2026-06-11)**: Wireless is intentionally a standalone defense-lab capability (CLI primary + TUI tab). Passive scanning lives under `wireless`, active deauth/disassoc lives under `wireless-advanced`, and neither participates in `ScanProfile` pipeline stages or dedicated wireless profiles (aspirational only; see `architecture/defense_lab.md` Future Integration and `cli_commands.md` Special Cases). Report integration is an optional lightweight bridge, not mandatory participation in chained pipelines. The bridge always runs rogue analysis (known-good suppression is UX-only for human/repeat output).

See docs/WIRELESS.md (Integration section), CAPABILITIES.md (Lab Defense row), and `crates/eggsec/src/commands/handlers/report.rs`.

Post passive standalone + integration (2026-06-11): CLI help polished (MODE prefix, more practical examples, --detect-suspicious canonical flag form), TUI descriptor now carries feature for policy parity, worker failure path made explicit. Active deauth/disassoc is available in both CLI and TUI active mode; MCP exposure remains intentionally absent (standalone defense-lab). All changes preserve the standalone defense-lab identity and policy model. Active phases are implemented under `wireless-advanced` (MCP exposure remains absent).
