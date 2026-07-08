# Mobile Module

## Purpose

Standalone static security analysis of Android APKs and iOS IPAs (Phase 1 static complete) + dynamic Android runtime testing via ADB + logcat (Phase 1 dynamic complete 2026-06-12) + Phase 2 proxy foundation + runtime permission testing + correlation + close-out (closed 2026-06-12) for authorized lab / defense-validation use. Pure-Rust static (ZIP + bounded AXML / plist). Dynamic: pure-Rust ADB TCP (emulator primary) + high-signal log parser; Level-1 proxy (device global http_proxy via --proxy + user-managed mitmproxy + --traffic-capture summary parser in new traffic.rs); runtime permission grant/revoke/list via adb helpers; no Frida/instrumentation in Phase 1/2 (closed). Produces local `MobileScanReport` / `MobileFinding` (static) and `DynamicMobileReport` / `DynamicMobileFinding` (dynamic; extended with optional traffic_summary/permission_state), both with optional bridges to `ScanReportData` (extra info findings for summary/state under mobile-dynamic-* categories). Standalone defense-lab surface (MCP/agent absent; same pattern as wireless-active + static-mobile + auth-test). Phase 1 polish (smoke test script `scripts/test-mobile-dynamic.sh`, `--list-devices` convenience, troubleshooting, docs) complete 2026-06-12 per `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed). Phase 2 complete/closed 2026-06-12 per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (combined; Level 1 proxy integration + permissions + correlation + hygiene; no new sub-feature, all under mobile-dynamic per M1 decision). Final polish + close-out executed 2026-06-12 per prior polish plans. Phase 3 kickoff (Frida) vision documented. See plans for 2b+/Frida/etc.

## Architecture: Adapter + Domain Crate

All domain logic lives in the `eggsec-mobile-lab` crate
(`crates/eggsec-mobile-lab/src/`). The main `eggsec` crate has a thin adapter
layer at `crates/eggsec/src/mobile/mod.rs` that re-exports types and delegates
CLI entry points to the domain crate for backward compatibility.

### Adapter Layer (`crates/eggsec/src/mobile/mod.rs`)

Thin bridge providing:

- **Re-exports** of all domain types (`MobilePlatform`, `MobileFinding`,
  `MobileScanReport`, `DynamicMobileReport`, `DynamicMobileFinding`, etc.)
- **Re-exports** of domain submodules (`apk`, `ipa`, `adb`, `dynamic`,
  `frida`, `traffic`, `runtime`) for backward compatibility
- **`run_cli(args, config)`**: Resolves effective static CLI args and delegates
  to `eggsec_mobile_lab::run_static_cli()`
- **`run_dynamic_cli(args, config)`**: Delegates to
  `eggsec_mobile_lab::run_dynamic_cli()`

No business logic lives in the adapter. It exists solely to preserve the
`crates/eggsec/src/mobile::` path for existing imports.

### Domain Crate (`crates/eggsec-mobile-lab/src/`)

All implementation, types, and tests:

| File | Description |
|------|-------------|
| `lib.rs` | Core types (`MobilePlatform`, `MobileFinding`, `MobileScanReport`), `run_static_cli`, `format_mobile_report`, `build_general_recommendations`, `to_scan_report_data`, `to_report_envelope` |
| `apk.rs` | APK analysis (ZIP open, manifest parsing (text + binary AXML), permissions, components, network-security-config, secret scanning, cert checks) |
| `ipa.rs` | IPA analysis (ZIP open, Info.plist, embedded.mobileprovision, code signature markers, transport/entitlements) |
| `dynamic.rs` | Dynamic types (`DynamicMobileReport`/`Finding`, `LabManifest`, `DynamicMobileArgs` internal skeleton), `run_dynamic_cli`, format/bridge (`to_scan_report_data_dynamic`; Phase 2 closed extensions + extra info findings + correlate_findings; Phase 3a frida_instrumentation + frida-* findings + bridge); `correlate_findings` (public re-export) + `CorrelatedFinding`. Phase 4a (delivered 2026-06-12): + `CorrelationEngine`, `correlate_reports`, `CorrelationResult` (correlations + timeline + summary), `CorrelationSummary`, `CorrelationType`; scoring inside correlate_findings + engine; `build_timeline`; all under single mobile-dynamic. |
| `adb.rs` | Pure-Rust ADB TCP framing + `AdbClient`/`AdbConnection` (list_devices, connect, shell, install, launch, uninstall, capture_logcat, set_global_proxy/clear_global_proxy, grant/revoke/list_permissions); external `adb` only for discovery convenience; Phase 2 permission/proxy helpers |
| `traffic.rs` | Phase 2 (closed): `TrafficSummary` + `parse_traffic_capture` (summary-only parser for mitmproxy logs/HAR-like; domains, counts, cleartext/suspicious hints; produces high-signal findings); public under mobile-dynamic |
| `frida.rs` | Phase 3a+3b+3c: Frida instrumentation (`FridaSession`, `FridaInstrumentation`, `FridaScriptResult`), builtin scripts (basic_method_trace, crypto_keystore, bypass_validation, api_trace), `resolve_frida_script_spec`, `run_frida_spec`, `run_builtin`, `generate_*`, `redact_frida_evidence`, embedded `FRIDA_LIB_COMMON_HOOKS` |
| `runtime.rs` | Runtime helpers for dynamic analysis |

## CLI Behavior

- Build with `--features mobile` (static) or `--features mobile-dynamic` (dynamic + static; implies mobile).
- `eggsec mobile <path-to-.apk-or-.ipa>` (legacy direct static) or `eggsec mobile static <path>` (explicit static subcommand); supports `--json`, `-o/--output`, `-q/--quiet`.
- `eggsec mobile dynamic <target.apk> --device <serial|host:port> [--install] [--launch <activity>] [--capture-logs --duration N] [--uninstall-after] [--dry-run] [--allow-dynamic-mobile] [--lab-manifest FILE] [--proxy <host:port>] [--reset-proxy] [--traffic-capture <file>] [--grant-permission P] [--revoke-permission P] [--list-permissions] [--json] [-o OUT]`. (Phase 2 proxy/permission flags + correlation; closed 2026-06-12; still DefenseLab/SafeActive + allow gate).
- Static: pure offline on user-supplied lab binaries. Size guard (200 MiB). Lab framing note unless quiet.
- Dynamic (P1): controlled ADB + logcat on lab devices/emulators you control. Dry-run always valid (no device/net touch, full report produced). Real runs require explicit `--allow-dynamic-mobile` (audited) + best-effort cleanup. Actions audited in report.
- Direct human/JSON from native report types; optional `to_scan_report_data*` bridges for unified consumers. `eggsec report convert` auto-bridges native JSON when respective feature enabled (mirrors wireless).

## Key Types

All types are defined in `crates/eggsec-mobile-lab/src/` and re-exported through
the adapter at `crates/eggsec/src/mobile/`.

| Type | Defined In | Description |
|------|-----------|-------------|
| `MobilePlatform` | `lib.rs` | Enum: Android, Ios |
| `MobileFinding` | `lib.rs` | Severity-rated finding (category, title, description, recommendation, optional evidence) |
| `MobileScanReport` | `lib.rs` | Full report (target, scan_type="mobile-static", platform, app_id, version, findings, recommendations, duration) |
| `DynamicMobileReport` | `dynamic.rs` | Full dynamic report (target, scan_type="mobile-dynamic", platform=Android, device_serial, app_id, findings, actions_performed, dry_run, duration; Phase 2: + optional traffic_summary, permission_state; Phase 3c: + regression_notes; Phase 4a: + correlation_result) |
| `DynamicMobileFinding` | `dynamic.rs` | Runtime finding (category e.g. runtime-permission/crash-log/cleartext-observed/log-secret-leak/traffic-summary/permission-state/frida-*, severity, title, description, recommendation, evidence, static_correlation); `static_correlation` populated by `correlate_findings` |
| `CorrelatedFinding` | `dynamic.rs` | Lightweight result from `correlate_findings(dynamic_findings, static_findings)` linking high-value static signals to dynamic observations (Phase 4a: + optional score, correlation_type, enrichment) |
| `LabManifest` | `dynamic.rs` | Optional advisory TOML allowlist (allowed_device_serials, allowed_packages); advisory in P1 |
| `DynamicMobileArgs` | `dynamic.rs` | Internal dispatcher args (target, device, install/launch/capture_logs/duration/uninstall_after/dry_run, allow_dynamic_mobile, lab_manifest, frida_script, allow_frida) |
| `FridaInstrumentation` | `frida.rs` | Frida execution result (script_results, structured_results, correlation_notes, start_time, regression_notes) |
| `FridaScriptResult` | `frida.rs` | Individual script execution result (script_name, output, timing, structured output) |
| `MobileBaseline` | `dynamic.rs` | Captured baseline for regression comparison |
| `CorrelationEngine` | `dynamic.rs` | Correlation engine for linking static, dynamic, and Frida findings |
| `TrafficSummary` | `traffic.rs` | Parsed traffic capture summary (domains, counts, cleartext/suspicious hints) |

## CLI Types (in main crate)

| Type | Location | Description |
|------|----------|-------------|
| `MobileArgs` | `crates/eggsec-cli/src/cli/mobile.rs` | CLI args (path, json, output, quiet, command: Option<MobileSubcommand>) + `MOBILE_ABOUT` |
| `MobileStaticArgs` | `crates/eggsec-cli/src/cli/mobile.rs` | Static subcommand args |
| `DynamicMobileArgs` (CLI) | `crates/eggsec-cli/src/cli/mobile.rs` | Dynamic subcommand args (CLI-facing) |

## Handler

| File | Description |
|------|-------------|
| `crates/eggsec/src/commands/handlers/mobile.rs` | `handle_mobile` (subcommand dispatch; static uses StandardAssessment/SafeActive + "mobile"; dynamic uses DefenseLab/SafeActive (Intrusive for real Frida) + "mobile-dynamic" + explicit `--allow-dynamic-mobile` + (for Frida) `--allow-frida` gate; policy + notify + map to internal DynamicMobileArgs; Phase 2 proxy/permission flags + correlation passed through; Phase 3a/3b Frida flags + policy) |

## Status

Phase 1 static complete (pure-Rust, SafeActive, standalone CLI + optional report bridge; closed 2026-06-11). Phase 1 dynamic (Android ADB core + high-signal runtime logcat analysis) complete 2026-06-12 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed) + parent `plans/dynamic-mobile-testing-loadout-design-plan.md`. Phase 1 polish (smoke test script, `--list-devices` convenience, troubleshooting section, updated success criteria) complete 2026-06-12 per `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed). Phase 2 (Level 1 proxy integration + runtime permission testing + correlation + hygiene) closed 2026-06-12 per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (combined close-out + kickoff; executed): new `traffic.rs`, adb proxy/permission helpers, `DynamicMobileReport` + `to_scan_report_data_dynamic` extensions (traffic_summary/permission_state + bridge info findings + static_correlation), CLI/handler mapping; kept under single mobile-dynamic feature (M1 decision: no sub-feature split); standalone defense-lab (MCP/agent absent). Final polish + close-out executed 2026-06-12 per prior plans. Phase 3a (Frida foundation + basic_method_trace under single mobile-dynamic per phase3-frida-expansion-plan.md Key Decision; runtime --allow-frida + Intrusive policy for real; dry-run safe; frida.rs real impl + CLI/handler/report/bridge/smoke) delivered 2026-06-12 (no mobile-frida sub-feature). Phase 3b (additional builtins crypto-keystore/bypass-validation/api-trace + Frida+traffic/static correlation in correlate_findings + richer FridaInstrumentation with structured_results/correlation_notes/start_time + redaction + structured JSON output preference + expanded tests + smoke) delivered 2026-06-12 (executed; same Key Decision; frida.rs run_builtin + generate_* + redact + structured_output; dynamic.rs richer carrier + real/dry paths + extended correlate + format/bridge/recommendations + tests; CLI/ABOUTs updated). Phase 3c (user script library + multi-script sessions + advanced static<->dynamic<->Frida correlation + behavioral baselining/regression + optional evidence bundles) delivered 2026-06-12 (executed per plan; library via embedded FRIDA_LIB_COMMON_HOOKS + resolve_frida_script_spec/run_frida_spec; repeatable --frida-script; FridaInstrumentation +regression_notes; MobileBaseline + capture_baseline/compare_to_baseline + --baseline; export_evidence_bundle; correlate_findings + 3c cross rules; CLI --baseline/--evidence-bundle; 85 mobile tests total). Phase 4a (Core Correlation Engine: CorrelationEngine + correlate_reports + enriched CorrelatedFinding + CorrelationResult with timeline/summary + conservative scoring) delivered 2026-06-12 under single mobile-dynamic (non-breaking; 6 new tests). Phase 4b TUI reviewed/deferred 2026-06-12 per standalone policy; reporting polish delivered in format_dynamic_report / build_dynamic_recommendations. Phase 4c explored + partial (supply-chain native-load builtin + correlation, regression category delta, bundle_manifest, run_baseline_compare_workflow helper) 2026-06-12 (additive, same constraints).

Standalone defense-lab surface (MCP/agent absent, same pattern as wireless active). Local native types + optional bridge; auto-bridge in `report convert`. No TUI tab or pipeline profile integration in this round (`mobile-static`/`mobile-dynamic`/`mobile-regression` aspirational). iOS dynamic + full advanced regression future.

See `docs/MOBILE.md`.

## Integration with Reporting Pipeline

`eggsec mobile` is intentionally a **standalone defense-lab CLI** (not a `ScanProfile` pipeline stage). It emits local `MobileScanReport` / `MobileFinding` types directly for human and `--json` use.

An optional `to_scan_report_data()` bridge (in `crates/eggsec-mobile-lab/src/lib.rs`; mirrors wireless pattern) converts to `ScanReportData` for unified consumers (SARIF, JUnit, HTML, Markdown, CSV, JSON, trend, etc.).

The `report convert` handler auto-bridges native `MobileScanReport` JSON when the `mobile` feature is enabled, and native `DynamicMobileReport` JSON when `mobile-dynamic` is enabled, so `eggsec mobile ... --json -o m.json ; eggsec report convert m.json -f ...` works directly for both.

Categories in bridged output are `mobile-{android,ios}-<native-category>` (static) or `mobile-dynamic-android-<category>` (dynamic) (e.g. `mobile-android-manifest`, `mobile-dynamic-android-runtime-permission`, `mobile-dynamic-android-crash-log`; Phase 2 closed: `mobile-dynamic-android-traffic-summary`, `mobile-dynamic-android-permission-state`, etc. via extra info findings). Evidence carries through; empty findings are valid (0 findings in bridge). Dynamic bridge mirrors static + active wireless pattern.

**Design decision (Phase 1 static close 2026-06-11; dynamic P1 2026-06-12; Phase 2 closed 2026-06-12)**: Standalone CLI-only (no TUI, no pipeline stages/profiles); optional bridge provides reporting unification without forcing `ScanProfile` integration (`mobile-static`/`mobile-dynamic`/`mobile-regression` remain aspirational per `architecture/defense_lab.md` Future). Use native types for lab-specific flows; use bridge (or `report convert` on native JSON) for unified report consumers. Integration is lightweight and opt-in. Dynamic P1 complete (ADB + logcat); Phase 2 (proxy Level-1 + permissions + correlation) closed under single `mobile-dynamic` (no sub-feature split per M1); future phases (Frida) per the design plan.

Integration points:
- Enforcement: static uses StandardAssessment/SafeActive + "mobile"; dynamic uses DefenseLab/SafeActive + "mobile-dynamic" + explicit `--allow-dynamic-mobile` gate (audited) + feature check (see handler).
- Reporting: local types emitted directly; `to_scan_report_data` / `to_scan_report_data_dynamic` available for OutputFormat consumers; auto-bridge in `report convert` (extended for `mobile-dynamic` native JSON; categories `mobile-dynamic-android-*`; Phase 2 closed: extra info findings for traffic_summary/permission_state under mobile-dynamic-android-*).
- Feature gate + policy: `mobile` / `mobile-dynamic=["mobile"]` in Cargo.toml; listed in `full`; policy in `config/policy_decision.rs` + handler descriptor.
- Handler dispatch: `commands/handlers/mod.rs` and `cli/mod.rs`.

Safety model: Lab/defense use only. Static: user-provided binaries, offline, bounded. Dynamic (P1): controlled ADB on lab devices you authorize; dry-run always valid; real runs require explicit allow + best-effort cleanup; all actions audited in report. Both under central `EnforcementContext`.

Policy / descriptor (dynamic): `OperationDescriptor { operation: "mobile-dynamic", mode: DefenseLab, risk: SafeActive, required_features: ["mobile-dynamic"], ... }` in handler; extra runtime gate for `!dry_run && !allow_dynamic_mobile`. Static uses StandardAssessment/SafeActive + "mobile". Dry-run is always accepted (no device actions). (See `crates/eggsec/src/commands/handlers/mobile.rs:26-51`.)

See `crates/eggsec-mobile-lab/src/` (domain), `crates/eggsec/src/mobile/mod.rs` (adapter), `crates/eggsec/src/commands/handlers/mobile.rs`, `crates/eggsec-mobile-lab/src/traffic.rs` (Phase 2), and `crates/eggsec/Cargo.toml` (feature flags).

## Future

Phase 1 dynamic complete (ADB core + logcat; 2026-06-12). Phase 2 (Level 1 proxy integration + permission testing + correlation + hygiene) closed 2026-06-12 per combined closeout+kickoff plan. Phase 3a+3b+3c (Frida under mobile-dynamic, runtime --allow-frida gate, foundation + multiple high-signal builtins + library + multi-script + regression + bundles, safety via EnforcementContext + explicit allow + policy (Intrusive for real), reporting bridge (frida_instrumentation richer + regression_notes + new frida-* + behavioral-regression + mobile-dynamic-android-* + correlation), Frida+dynamic/static+traffic correlation + baselining, Android-first, standalone defense-lab) delivered. Phase 4a (Core Correlation Engine) delivered 2026-06-12. Phase 4b TUI reviewed/deferred 2026-06-12 per standalone policy; reporting polish delivered. Phase 4c explored + partial 2026-06-12. iOS dynamic + full advanced regression future.
