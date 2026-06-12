# Mobile Module

## Purpose

Standalone static security analysis of Android APKs and iOS IPAs (Phase 1 static complete) + dynamic Android runtime testing via ADB + logcat (Phase 1 dynamic complete 2026-06-12) + Phase 2 proxy foundation + runtime permission testing + correlation + close-out (closed 2026-06-12) for authorized lab / defense-validation use. Pure-Rust static (ZIP + bounded AXML / plist). Dynamic: pure-Rust ADB TCP (emulator primary) + high-signal log parser; Level-1 proxy (device global http_proxy via --proxy + user-managed mitmproxy + --traffic-capture summary parser in new traffic.rs); runtime permission grant/revoke/list via adb helpers; no Frida/instrumentation in Phase 1/2 (closed). Produces local `MobileScanReport` / `MobileFinding` (static) and `DynamicMobileReport` / `DynamicMobileFinding` (dynamic; extended with optional traffic_summary/permission_state), both with optional bridges to `ScanReportData` (extra info findings for summary/state under mobile-dynamic-* categories). Standalone defense-lab surface (MCP/agent absent; same pattern as wireless-active + static-mobile + auth-test). Phase 1 polish (smoke test script `scripts/test-mobile-dynamic.sh`, `--list-devices` convenience, troubleshooting, docs) complete 2026-06-12 per `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed). Phase 2 complete/closed 2026-06-12 per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (combined; Level 1 proxy integration + permissions + correlation + hygiene; no new sub-feature, all under mobile-dynamic per M1 decision). Final polish + close-out executed 2026-06-12 per prior polish plans. Phase 3 kickoff (Frida) vision documented. See plans for 2b+/Frida/etc.

## CLI Behavior

- Build with `--features mobile` (static) or `--features mobile-dynamic` (dynamic + static; implies mobile).
- `eggsec mobile <path-to-.apk-or-.ipa>` (legacy direct static) or `eggsec mobile static <path>` (explicit static subcommand); supports `--json`, `-o/--output`, `-q/--quiet`.
- `eggsec mobile dynamic <target.apk> --device <serial|host:port> [--install] [--launch <activity>] [--capture-logs --duration N] [--uninstall-after] [--dry-run] [--allow-dynamic-mobile] [--lab-manifest FILE] [--proxy <host:port>] [--reset-proxy] [--traffic-capture <file>] [--grant-permission P] [--revoke-permission P] [--list-permissions] [--json] [-o OUT]`. (Phase 2 proxy/permission flags + correlation; closed 2026-06-12; still DefenseLab/SafeActive + allow gate).
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
| `DynamicMobileReport` | `mobile/dynamic.rs` | Full dynamic report (target, scan_type="mobile-dynamic", platform=Android, device_serial, app_id, findings, actions_performed, dry_run, duration; Phase 2 closed 2026-06-12: + optional traffic_summary: Option<TrafficSummary>, permission_state: Option<PermissionState>) |
| `DynamicMobileFinding` | `mobile/dynamic.rs` | Runtime finding (category e.g. runtime-permission/crash-log/cleartext-observed/log-secret-leak/traffic-summary/permission-state, severity, title, description, recommendation, evidence, static_correlation); `static_correlation` populated by `correlate_findings` (cleartext ↔ usesCleartextTraffic/network config; runtime perms ↔ static declared dangerous perms) |
| `CorrelatedFinding` | `mobile/dynamic.rs` | Lightweight result from `correlate_findings(dynamic_findings, static_findings)` linking high-value static signals to dynamic observations (initial rules: cleartext, dangerous permissions) |
| `LabManifest` | `mobile/dynamic.rs` | Optional advisory TOML allowlist (allowed_device_serials, allowed_packages); advisory in P1 |
| `DynamicMobileArgs` | `cli/mobile.rs` (primary CLI definition for subcommand); internal skeleton in `mobile/dynamic.rs` | Dispatcher args (target, device, install/launch/capture_logs/duration/uninstall_after/dry_run, json/output/quiet, allow_dynamic_mobile, lab_manifest). Location cleanup per Phase 1 polish. |
| `run_dynamic_cli` | `mobile/dynamic.rs` | Async dispatcher for dynamic path (mirrors static `run_cli`) |
| `MobileStaticArgs` / `DynamicMobileArgs` (CLI) | `cli/mobile.rs` | Subcommand arg structs under `MobileSubcommand` |

## Files

| File | Description |
|------|-------------|
| `mobile/mod.rs` | Core types, `run_cli`, `format_mobile_report`, `build_general_recommendations`, `to_scan_report_data` bridge; cfg-gated reexports for dynamic |
| `mobile/apk.rs` | APK analysis (zip open, manifest parsing (text + binary AXML), permissions, components, network-security-config, secret scanning, cert checks) |
| `mobile/ipa.rs` | IPA analysis (zip open, Info.plist, embedded.mobileprovision, code signature markers, transport/entitlements) |
| `mobile/dynamic.rs` | Dynamic types (`DynamicMobileReport`/`Finding`, `LabManifest`, `DynamicMobileArgs` internal skeleton), `run_dynamic_cli`, format/bridge (`to_scan_report_data_dynamic`; Phase 2 closed extensions + extra info findings + correlate_findings); `correlate_findings` (public re-export) + `CorrelatedFinding` live here |
| `mobile/adb.rs` | Pure-Rust ADB TCP framing + `AdbClient`/`AdbConnection` (list_devices, connect, shell, install, launch, uninstall, capture_logcat, set_global_proxy/clear_global_proxy, grant/revoke/list_permissions); external `adb` only for discovery convenience; Phase 2 permission/proxy helpers |
| `mobile/traffic.rs` | Phase 2 (closed): `TrafficSummary` + `parse_traffic_capture` (summary-only parser for mitmproxy logs/HAR-like; domains, counts, cleartext/suspicious hints; produces high-signal findings); public under mobile-dynamic |
| `cli/mobile.rs` | `MobileArgs` + `MOBILE_ABOUT`; `MobileSubcommand` (Static/Dynamic), `MobileStaticArgs`, `DynamicMobileArgs` (CLI) |
| `commands/handlers/mobile.rs` | `handle_mobile` (subcommand dispatch; static uses StandardAssessment/SafeActive + "mobile"; dynamic uses DefenseLab/SafeActive + "mobile-dynamic" + explicit `--allow-dynamic-mobile` gate; policy + notify + map to internal DynamicMobileArgs; Phase 2 proxy/permission flags + correlation passed through) |

## Status

Phase 1 static complete (pure-Rust, SafeActive, standalone CLI + optional report bridge; closed 2026-06-11). Phase 1 dynamic (Android ADB core + high-signal runtime logcat analysis) complete 2026-06-12 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed) + parent `plans/dynamic-mobile-testing-loadout-design-plan.md`. Phase 1 polish (smoke test script, `--list-devices` convenience, troubleshooting section, updated success criteria) complete 2026-06-12 per `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed). Phase 2 (Level 1 proxy integration + runtime permission testing + correlation + hygiene) closed 2026-06-12 per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (combined close-out + kickoff; executed): new `traffic.rs`, adb proxy/permission helpers, `DynamicMobileReport` + `to_scan_report_data_dynamic` extensions (traffic_summary/permission_state + bridge info findings + static_correlation), CLI/handler mapping; kept under single mobile-dynamic feature (M1 decision: no sub-feature split); standalone defense-lab (MCP/agent absent). Final polish + close-out executed 2026-06-12 per prior plans. Phase 3 kickoff vision (gated mobile-frida) added to docs.

Standalone defense-lab surface (MCP/agent absent, same pattern as wireless active). Local native types + optional bridge; auto-bridge in `report convert`. No TUI tab or pipeline profile integration in this round (`mobile-static`/`mobile-dynamic`/`mobile-regression` aspirational).

See `docs/MOBILE.md`.

## Integration with Reporting Pipeline

`eggsec mobile` is intentionally a **standalone defense-lab CLI** (not a `ScanProfile` pipeline stage). It emits local `MobileScanReport` / `MobileFinding` types directly for human and `--json` use.

An optional `to_scan_report_data()` bridge (in `mobile/mod.rs`; mirrors wireless pattern) converts to `ScanReportData` for unified consumers (SARIF, JUnit, HTML, Markdown, CSV, JSON, trend, etc.).

The `report convert` handler auto-bridges native `MobileScanReport` JSON when the `mobile` feature is enabled, and native `DynamicMobileReport` JSON when `mobile-dynamic` is enabled, so `eggsec mobile ... --json -o m.json ; eggsec report convert m.json -f ...` works directly for both.

Categories in bridged output are `mobile-{android,ios}-<native-category>` (static) or `mobile-dynamic-android-<category>` (dynamic) (e.g. `mobile-android-manifest`, `mobile-dynamic-android-runtime-permission`, `mobile-dynamic-android-crash-log`; Phase 2 closed: `mobile-dynamic-android-traffic-summary`, `mobile-dynamic-android-permission-state`, etc. via extra info findings). Evidence carries through; empty findings are valid (0 findings in bridge). Dynamic bridge mirrors static + active wireless pattern.

**Design decision (Phase 1 static close 2026-06-11; dynamic P1 2026-06-12; Phase 2 closed 2026-06-12)**: Standalone CLI-only (no TUI, no pipeline stages/profiles); optional bridge provides reporting unification without forcing `ScanProfile` integration (`mobile-static`/`mobile-dynamic`/`mobile-regression` remain aspirational per `architecture/defense_lab.md` Future). Use native types for lab-specific flows; use bridge (or `report convert` on native JSON) for unified report consumers. Integration is lightweight and opt-in. Dynamic P1 complete (ADB + logcat); Phase 2 (proxy Level-1 + permissions + correlation) closed under single `mobile-dynamic` (no sub-feature split per M1); future phases (Frida) per the design plan.

Integration points:
- Enforcement: static uses StandardAssessment/SafeActive + "mobile"; dynamic uses DefenseLab/SafeActive + "mobile-dynamic" + explicit `--allow-dynamic-mobile` gate (audited) + feature check (see handler).
- Reporting: local types emitted directly; `to_scan_report_data` / `to_scan_report_data_dynamic` available for OutputFormat consumers; auto-bridge in `report convert` (extended for `mobile-dynamic` native JSON; categories `mobile-dynamic-android-*`; Phase 2a: extra info findings for traffic_summary/permission_state under mobile-dynamic-android-*).
- Feature gate + policy: `mobile` / `mobile-dynamic=["mobile"]` in Cargo.toml; listed in `full`; policy in `config/policy_decision.rs` + handler descriptor.
- Handler dispatch: `commands/handlers/mod.rs` and `cli/mod.rs`.

Safety model: Lab/defense use only. Static: user-provided binaries, offline, bounded. Dynamic (P1): controlled ADB on lab devices you authorize; dry-run always valid; real runs require explicit allow + best-effort cleanup; all actions audited in report. Both under central `EnforcementContext`.

Policy / descriptor (dynamic): `OperationDescriptor { operation: "mobile-dynamic", mode: DefenseLab, risk: SafeActive, required_features: ["mobile-dynamic"], ... }` in handler; extra runtime gate for `!dry_run && !allow_dynamic_mobile`. Static uses StandardAssessment/SafeActive + "mobile". Dry-run is always accepted (no device actions). (See `commands/handlers/mobile.rs:26-51`.)

See `crates/eggsec/src/mobile/`, `crates/eggsec/src/commands/handlers/mobile.rs`, `crates/eggsec/src/mobile/traffic.rs` (Phase 2), and `crates/eggsec/Cargo.toml:310,318`.

## Future

Phase 1 dynamic complete (ADB core + logcat; 2026-06-12). Phase 2 (Level 1 proxy integration + permission testing + correlation + hygiene) closed 2026-06-12 per combined closeout+kickoff plan (new traffic.rs + adb helpers; report fields + bridge + static_correlation; CLI/handler; tests/smoke; under single mobile-dynamic per M1 decision: no sub-feature split). Close-out complete. Phase 3 kickoff: first-pass vision for gated `mobile-frida` feature (implies mobile-dynamic). Frida primitives + basic hooking (method tracing/arg logging); safety via EnforcementContext + explicit allow + rooted/emulator constraints; reporting bridge updates (new categories); Android-first; standalone defense-lab (MCP/agent absent, same pattern). Design/scaffolding begins (no impl yet); see parent `plans/dynamic-mobile-testing-loadout-design-plan.md`. `mobile-static` / `mobile-dynamic` / `mobile-regression` profiles remain aspirational. TUI tab and MCP exposure not in scope (standalone defense-lab). Keep future notes for deeper/Frida/iOS/etc.

Phase 3 vision (gated mobile-frida, Frida primitives, basic hooking, safety via EnforcementContext + explicit allow, reporting bridge, Android-first, standalone defense-lab) documented consistently with docs/MOBILE.md.
