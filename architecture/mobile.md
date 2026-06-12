# Mobile Module

## Purpose

Standalone static security analysis of Android APKs and iOS IPAs (Phase 1 static complete) + dynamic Android runtime testing via ADB + logcat (Phase 1 dynamic complete 2026-06-12) for authorized lab / defense-validation use. Pure-Rust static (ZIP + bounded AXML / plist). Dynamic: pure-Rust ADB TCP (emulator primary) + high-signal log parser; no Frida/instrumentation in P1. Produces local `MobileScanReport` / `MobileFinding` (static) and `DynamicMobileReport` / `DynamicMobileFinding` (dynamic), both with optional bridges to `ScanReportData`. Standalone defense-lab surface (MCP/agent absent). See plans for Phase 2+.

## CLI Behavior

- Build with `--features mobile` (static) or `--features mobile-dynamic` (dynamic + static; implies mobile).
- `eggsec mobile <path-to-.apk-or-.ipa>` (legacy direct static) or `eggsec mobile static <path>` (explicit static subcommand); supports `--json`, `-o/--output`, `-q/--quiet`.
- `eggsec mobile dynamic <target.apk> --device <serial|host:port> [--install] [--launch <activity>] [--capture-logs --duration N] [--uninstall-after] [--dry-run] [--allow-dynamic-mobile] [--lab-manifest FILE] [--json] [-o OUT]`.
- Static: pure offline on user-supplied lab binaries. Size guard (200 MiB). Lab framing note unless quiet.
- Dynamic (P1): controlled ADB + logcat on lab devices/emulators you control. Dry-run always valid (no device/net touch, full report produced). Real runs require explicit `--allow-dynamic-mobile` (audited) + best-effort cleanup. Actions audited in report.
- Direct human/JSON from native report types; optional `to_scan_report_data*` bridges for unified consumers. `eggsec report convert` auto-bridges native JSON when respective feature enabled (mirrors wireless).

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `MobilePlatform` | `mobile/mod.rs` | Enum: Android, Ios |
| `MobileFinding` | `mobile/mod.rs` | Severity-rated finding (category, title, description, recommendation, optional evidence) |
| `MobileScanReport` | `mobile/mod.rs` | Full report (target, scan_type="mobile-static", platform, app_id, version, findings, recommendations, duration) |
| `MobileArgs` | `cli/mobile.rs` | CLI args (path, json, output, quiet, command: Option<MobileSubcommand>) + `MOBILE_ABOUT` |
| `DynamicMobileReport` | `mobile/dynamic.rs` | Full dynamic report (target, scan_type="mobile-dynamic", platform=Android, device_serial, app_id, findings, actions_performed, dry_run, duration) |
| `DynamicMobileFinding` | `mobile/dynamic.rs` | Runtime finding (category e.g. runtime-permission/crash-log/cleartext-observed/log-secret-leak, severity, title, description, recommendation, evidence, static_correlation) |
| `LabManifest` | `mobile/dynamic.rs` | Optional advisory TOML allowlist (allowed_device_serials, allowed_packages); advisory in P1 |
| `DynamicMobileArgs` | `mobile/dynamic.rs` (internal) | Dispatcher args (target, device, install/launch/capture_logs/duration/uninstall_after/dry_run, json/output/quiet, allow_dynamic_mobile, lab_manifest) |
| `run_dynamic_cli` | `mobile/dynamic.rs` | Async dispatcher for dynamic path (mirrors static `run_cli`) |
| `MobileStaticArgs` / `DynamicMobileArgs` (CLI) | `cli/mobile.rs` | Subcommand arg structs under `MobileSubcommand` |

## Files

| File | Description |
|------|-------------|
| `mobile/mod.rs` | Core types, `run_cli`, `format_mobile_report`, `build_general_recommendations`, `to_scan_report_data` bridge; cfg-gated reexports for dynamic |
| `mobile/apk.rs` | APK analysis (zip open, manifest parsing (text + binary AXML), permissions, components, network-security-config, secret scanning, cert checks) |
| `mobile/ipa.rs` | IPA analysis (zip open, Info.plist, embedded.mobileprovision, code signature markers, transport/entitlements) |
| `mobile/dynamic.rs` | Dynamic types (`DynamicMobileReport`/`Finding`, `LabManifest`, `DynamicMobileArgs`), `run_dynamic_cli`, format/bridge (`to_scan_report_data_dynamic`) |
| `mobile/adb.rs` | Pure-Rust ADB TCP framing + `AdbClient`/`AdbConnection` (list_devices, connect, shell, install, launch, uninstall, capture_logcat); external `adb` only for discovery convenience |
| `mobile/runtime.rs` | High-signal logcat parser (`parse_logcat_findings`): runtime-permission, crash-log, cleartext-observed, log-secret-leak (basic redaction) |
| `cli/mobile.rs` | `MobileArgs` + `MOBILE_ABOUT`; `MobileSubcommand` (Static/Dynamic), `MobileStaticArgs`, `DynamicMobileArgs` (CLI) |
| `commands/handlers/mobile.rs` | `handle_mobile` (subcommand dispatch; static uses StandardAssessment/SafeActive + "mobile"; dynamic uses DefenseLab/SafeActive + "mobile-dynamic" + explicit `--allow-dynamic-mobile` gate; policy + notify + map to internal DynamicMobileArgs) |

## Status

Phase 1 static complete (pure-Rust, SafeActive, standalone CLI + optional report bridge; closed 2026-06-11). Phase 1 dynamic (Android ADB core + high-signal runtime logcat analysis) complete 2026-06-12 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed) + parent `plans/dynamic-mobile-testing-loadout-design-plan.md`. 

Standalone defense-lab surface (MCP/agent absent, same pattern as wireless active). Local native types + optional bridge; auto-bridge in `report convert`. No TUI tab or pipeline profile integration in this round (`mobile-static`/`mobile-dynamic`/`mobile-regression` aspirational).

See `docs/MOBILE.md`.

## Integration with Reporting Pipeline

`eggsec mobile` is intentionally a **standalone defense-lab CLI** (not a `ScanProfile` pipeline stage). It emits local `MobileScanReport` / `MobileFinding` types directly for human and `--json` use.

An optional `to_scan_report_data()` bridge (in `mobile/mod.rs`; mirrors wireless pattern) converts to `ScanReportData` for unified consumers (SARIF, JUnit, HTML, Markdown, CSV, JSON, trend, etc.).

The `report convert` handler auto-bridges native `MobileScanReport` JSON when the `mobile` feature is enabled, and native `DynamicMobileReport` JSON when `mobile-dynamic` is enabled, so `eggsec mobile ... --json -o m.json ; eggsec report convert m.json -f ...` works directly for both.

Categories in bridged output are `mobile-{android,ios}-<native-category>` (static) or `mobile-dynamic-android-<category>` (dynamic) (e.g. `mobile-android-manifest`, `mobile-dynamic-android-runtime-permission`, `mobile-dynamic-android-crash-log`). Evidence carries through; empty findings are valid (0 findings in bridge). Dynamic bridge mirrors static + active wireless pattern.

**Design decision (Phase 1 static close 2026-06-11; dynamic P1 2026-06-12)**: Standalone CLI-only (no TUI, no pipeline stages/profiles); optional bridge provides reporting unification without forcing `ScanProfile` integration (`mobile-static`/`mobile-dynamic`/`mobile-regression` remain aspirational per `architecture/defense_lab.md` Future). Use native types for lab-specific flows; use bridge (or `report convert` on native JSON) for unified report consumers. Integration is lightweight and opt-in. Dynamic P1 complete (ADB + logcat); future phases per the design plan.

Integration points:
- Enforcement: static uses StandardAssessment/SafeActive + "mobile"; dynamic uses DefenseLab/SafeActive + "mobile-dynamic" + explicit `--allow-dynamic-mobile` gate (audited) + feature check (see handler).
- Reporting: local types emitted directly; `to_scan_report_data` / `to_scan_report_data_dynamic` available for OutputFormat consumers; auto-bridge in `report convert` (extended for `mobile-dynamic` native JSON; categories `mobile-dynamic-android-*`).
- Feature gate + policy: `mobile` / `mobile-dynamic=["mobile"]` in Cargo.toml; listed in `full`; policy in `config/policy_decision.rs` + handler descriptor.
- Handler dispatch: `commands/handlers/mod.rs` and `cli/mod.rs`.

Safety model: Lab/defense use only. Static: user-provided binaries, offline, bounded. Dynamic (P1): controlled ADB on lab devices you authorize; dry-run always valid; real runs require explicit allow + best-effort cleanup; all actions audited in report. Both under central `EnforcementContext`.

Policy / descriptor (dynamic): `OperationDescriptor { operation: "mobile-dynamic", mode: DefenseLab, risk: SafeActive, required_features: ["mobile-dynamic"], ... }` in handler; extra runtime gate for `!dry_run && !allow_dynamic_mobile`. Static uses StandardAssessment/SafeActive + "mobile". Dry-run is always accepted (no device actions). (See `commands/handlers/mobile.rs:26-51`.)

See `crates/eggsec/src/mobile/`, `crates/eggsec/src/commands/handlers/mobile.rs`, and `crates/eggsec/Cargo.toml:310,318`.

## Future

Phase 1 dynamic complete (ADB core + logcat; 2026-06-12). See `plans/dynamic-mobile-testing-loadout-design-plan.md` for Phase 2+ (proxy correlation, deeper instrumentation, iOS notes, pipeline profiles, etc.). `mobile-static` / `mobile-dynamic` / `mobile-regression` profiles remain aspirational. TUI tab and MCP exposure not in scope for current round (standalone defense-lab pattern).
