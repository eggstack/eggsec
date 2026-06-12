# Wireless Module Override

Specialized guidance for the `wireless/` module (passive recon + active deauth/disassoc). This file complements `AGENTS.md` with module-specific patterns.

## Standalone Defense-Lab Surface

`wireless/` follows the **standalone defense-lab** pattern, alongside `mobile/` and `auth-test` (see `AGENTS.md` "Standalone Defense-Lab Surfaces" section). Key invariants:

- **Local types direct**: `WirelessScanResult` / `WirelessNetwork` (passive) and `ActiveWirelessAttackResult` / `ActiveWirelessFinding` (active) are emitted from the CLI/TUI as first-class types. Human-readable output, `--json`, and file writes all use these local types.
- **Optional `to_scan_report_data()` bridge**: provides a thin conversion to `ScanReportData` for unified consumers. The report handler (`commands/handlers/report.rs`) auto-bridges native JSON when the relevant feature is enabled, so `eggsec report convert <wireless.json> -f sarif` works without manual conversion. Categories are `wireless-*` (passive) and `wireless-active-*` (active, under `wireless-advanced`).
- **No `ScanProfile` participation**: wireless does not have a dedicated pipeline stage. The proposed `WirelessAnalysis` / `wireless-defense` profile is **deferred** (see `architecture/proposed-wireless-mobile-stages.md`).
- **MCP/agentic exposure absent**: wireless is **not** registered as a `SecurityTool`. `tools/list` and the `OpsAgent` / `CodingAgent` MCP profiles do not see wireless commands. This is a deliberate design decision recorded in `architecture/wireless.md` (MCP/Agentic section).
- **No persistence** (no `FindingStore` writes, no DB rows for `wireless` commands).

## Feature Gating

| Feature | Module surface | Public? |
|---------|---------------|---------|
| `wireless` | `wireless/mod.rs`, `wireless/scanner.rs`, `wireless/types.rs` | Yes |
| `wireless-advanced` | `wireless/active/` (deauth, disassoc) | Yes |
| (neither) | `mod wireless { ... }` stub with `#[allow(dead_code)]` | Internal-only |

`wireless-advanced` declares `wireless-advanced = ["wireless"]` in `Cargo.toml` and is intentionally **not** in `full` (per `README.md` build features). Build with `cargo check -p eggsec --features wireless-advanced` to compile active attacks.

## Policy Model

The active attacks follow the central `EnforcementContext::evaluate()` flow:

- **Dry-run** (default): `OperationRisk::SafeActive` under `OperationMode::DefenseLab` → `Allow` / `Warn` → no prompt.
- **Live mode**: `OperationRisk::Intrusive` under `DefenseLab` → `RequireConfirmation` under `ManualPermissive` → opens the policy overlay; the handler additionally hard-bails on `!allow_active_wireless` (CLI flag) for safety.
- **MCP / agent paths**: `AgentStrict` / `McpStrict` profiles treat `RequireConfirmation` as `Deny`, so the live-mode path is not reachable from automated callers. This reinforces the "defense-lab only" posture.

Operation descriptors:

- CLI: `OperationDescriptor { operation: "wireless-deauth", mode: DefenseLab, risk: Intrusive, target: Some(bssid), required_features: vec!["wireless-advanced"], ... }` (see `commands/handlers/wireless.rs:74-91`).
- TUI: same shape, built in `app::build_current_operation_descriptor` (special-cased for wireless active mode at `app/mod.rs:436-471`).

## Active Attack Pipeline (deauth / disassoc)

1. `ActiveAttackConfig` (`wireless/active/mod.rs:68-84`) bundles interface, optional BSSID, optional client MAC, reason code, max frames, frames-per-second, and dry-run flag. Hard budgets: `max_frames ≤ 1000`, `frames_per_second ≤ 100` (enforced in both the worker and the handler).
2. `run_deauth(config, broadcast)` / `run_disassoc(config, broadcast)` (`wireless/active/attacks/deauth.rs`) build raw 802.11 management frames (pure-Rust, no `pnet` for the frame body), wrap them in radiotap headers, and inject via `AF_PACKET` / `SOCK_RAW` on Linux. Dry-run short-circuits before socket creation.
3. The result is an `ActiveWirelessAttackResult` with `interface`, `attack_type`, `target_bssid`, `target_client`, `frames_sent`, `duration_secs`, `dry_run`, `findings: Vec<ActiveWirelessFinding>`, `raw_output`, and `recommendations`.

`broadcast` is `true` when `client` is `None` (no specific target) and `false` when targeting a single client MAC. This matches the standard 802.11 deauth frame semantics (addr1 = receiver, addr2 = sender/attacker, addr3 = BSSID).

## Reporting Bridge

`to_active_scan_report_data()` (`wireless/active/mod.rs`) bridges `ActiveWirelessAttackResult` → `ScanReportData` with:

- One `Finding` per `ActiveWirelessFinding` (severity mapped, category `wireless-active-deauth` or `wireless-active-disassoc`).
- Metadata includes `interface`, `target_bssid`, `attack_type`, `dry_run` flag, and a summary of `frames_sent` / `duration_secs`.

The auto-bridge inside `commands/handlers/report.rs` activates the conversion when the input JSON file looks like an active wireless result and the `wireless-advanced` feature is compiled in. Native `--json` from `eggsec wireless <iface> deauth` is therefore directly ingestible by `eggsec report convert -f sarif|junit|html|markdown|csv`.

## Testing

- `wireless/active/attacks/deauth.rs` has 8 unit tests covering frame construction, MAC parsing, and dry-run short-circuit (lines 459-543).
- `tabs/wireless.rs` has 12 unit tests under `#[cfg(all(test, feature = "wireless-advanced"))] mod tests` (lines 899-1082) covering `active_attack_config`, `build_task_config`, `set_active_results`, mode toggling, dry-run flipping, `start_active_attack`, and `handle_enter` flows.
- Integration tests requiring a monitor-mode interface and root/CAP_NET_ADMIN are not checked in; they are expected to be run manually in a lab environment with `cargo test --features wireless-advanced -- --ignored` after moving the relevant `#[ignore]` tests, if added.

Verification commands (from the top-level `AGENTS.md`):

```bash
cargo check -p eggsec --features wireless
cargo test --lib -p eggsec --features wireless
cargo clippy --lib -p eggsec --features wireless
cargo check -p eggsec --features wireless-advanced
cargo test --lib -p eggsec --features wireless-advanced
cargo clippy --lib -p eggsec --features wireless-advanced
cargo check -p eggsec-tui --features wireless-advanced
```
