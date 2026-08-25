# Mobile Module

## Role & Responsibilities

Standalone defense-lab module for mobile app security analysis. Two surfaces:

1. **Static analysis** (always compiled): Pure-Rust APK/IPA analysis — ZIP parsing, Android binary AXML decoding, plist deserialization, manifest/permission/signing/secret scanning. No network, no device interaction.
2. **Dynamic analysis** (feature-gated `mobile-dynamic`): Android ADB runtime testing — device connect, app install/launch/uninstall, logcat capture, proxy configuration, permission management, traffic capture parsing, Frida instrumentation, behavioral baselining/regression, evidence bundles.

This is **defense validation and lab analysis**, not offensive tooling.

**Non-responsibilities**: Does not perform real exploitation. Does not interact with production devices. Does not integrate with MCP/agent/TUI/pipeline (standalone defense-lab CLI).

## Location & Feature Gating

| Item | Location | Gate |
|------|----------|------|
| Domain crate | `crates/eggsec-mobile-lab/src/` | Always (static); `mobile-dynamic` for dynamic |
| Adapter layer | `crates/eggsec/src/mobile/mod.rs` | `#[cfg(feature = "mobile")]` |
| Feature: `mobile` | `crates/eggsec/Cargo.toml` | Marker, enables static analysis |
| Feature: `mobile-dynamic` | `crates/eggsec/Cargo.toml` | `mobile-dynamic = ["mobile"]` (implies mobile) |
| CLI handler | `crates/eggsec/src/commands/handlers/mobile.rs:6` | `#[cfg(feature = "cli")]` |
| CLI args | `crates/eggsec/src/cli/mobile.rs` | `#[cfg(feature = "cli")]` |
| DomainDescriptors | `crates/eggsec/src/domain/` | `mobile-static`, `mobile-dynamic` |

## Architecture

### Crate Layout

```
crates/eggsec-mobile-lab/src/
├── lib.rs        — Core types, run_static_cli, format_mobile_report, bridges, tests
├── apk.rs        — APK analysis (ZIP + AXML + text XML + secrets + certs)
├── ipa.rs        — IPA analysis (ZIP + plist + mobileprovision + code signature)
├── adb.rs        — Pure-Rust ADB TCP framing + external adb convenience
├── dynamic.rs    — Dynamic types, run_dynamic_cli, correlation, baselines, bundles
├── frida.rs      — Frida instrumentation (sessions, scripts, builtins, library)
├── traffic.rs    — Traffic capture summary parser (text logs, minimal HAR)
└── runtime.rs    — Runtime logcat parser (permission, crash, cleartext, secrets)
```

### Files (8 domain + 1 adapter)

| File | Lines | Description |
|------|-------|-------------|
| `lib.rs` | 500 | `MobilePlatform`, `MobileFinding`, `MobileScanReport`, `run_static_cli()`, `format_mobile_report()`, `to_scan_report_data()`, `to_report_envelope()`, `build_general_recommendations()`, tests |
| `apk.rs` | 1262 | `analyze_apk()` — ZIP open (ZipSlip rejection, 50 MiB extraction budget, 128 KiB per-text scan), binary AXML decoder (string pool + linear chunk walk for START_TAG/END_TAG), text XML fallback (quick-xml), permission/manifest/application analysis, network_security_config parsing, secret scanning, debug cert detection |
| `ipa.rs` | 782 | `analyze_ipa()` — ZIP open (ZipSlip rejection, 200 MiB guard), Info.plist deserialization (plist + serde), NSAppTransportSecurity exceptions, embedded.mobileprovision markers, _CodeSignature presence, secret scanning in .app assets |
| `adb.rs` | 861 | Pure-Rust ADB TCP protocol: `AdbClient`/`AdbConnection`, CNXN/AUTH/OPEN/OKAY/WRTE/CLSE framing, `list_devices()`, `connect()`, `shell()`, `install()`, `launch()`, `uninstall()`, `capture_logcat()`, `set_global_proxy()`/`clear_global_proxy()`, `grant()`/`revoke()`/`list_permissions()` |
| `dynamic.rs` | 3264 | `DynamicMobileReport`, `DynamicMobileFinding`, `LabManifest`, `DynamicMobileArgs`, `run_dynamic_cli()`, `CorrelatedFinding`, `CorrelationEngine`, `CorrelationResult`, `capture_baseline()`, `compare_to_baseline()`, `correlate_findings()`, `correlate_reports()`, `export_evidence_bundle()`, `to_scan_report_data_dynamic()`, formatting, tests |
| `frida.rs` | 961 | `FridaSession`, `FridaScriptResult`, `FridaInstrumentation`, `connect()`, `execute_script()`, `basic_method_trace()`, builtin scripts (basic_method_trace, crypto_keystore, bypass_validation, api_trace), `resolve_frida_script_spec()`, `run_frida_spec()`, `run_builtin()`, `generate_*()`, `redact_frida_evidence()`, embedded `FRIDA_LIB_COMMON_HOOKS` |
| `traffic.rs` | 479 | `TrafficSummary`, `parse_traffic_capture()` — text log parser (mitmproxy-style), minimal HAR JSON parser, 1 MiB safety cap, domain/cleartext/suspicious analysis |
| `runtime.rs` | 248 | `parse_logcat_findings()` — high-signal logcat parser: permission grant/deny, crashes/exceptions, cleartext HTTP hints, secret-like patterns; basic redaction |
| `adapter` (`mobile/mod.rs`) | 272 | Re-exports all domain types, `run_cli()` (static dispatch), `run_dynamic_cli()` (dynamic dispatch), backward-compatible paths |

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `MobilePlatform` | `lib.rs:81` | Enum: `Android`, `Ios` |
| `MobileFinding` | `lib.rs:96` | Severity-rated finding: category, severity, title, description, recommendation, evidence |
| `MobileScanReport` | `lib.rs:107` | Static report: target, scan_type="mobile-static", platform, app_id, version, findings, recommendations, duration_ms |
| `DynamicMobileReport` | `dynamic.rs` | Dynamic report: target, scan_type="mobile-dynamic", platform, device_serial, app_id, findings, actions_performed, dry_run, traffic_summary, permission_state, correlation_result, regression_notes, frida_instrumentation |
| `DynamicMobileFinding` | `dynamic.rs` | Runtime finding: category, severity, title, description, recommendation, evidence, static_correlation |
| `DynamicMobileArgs` | `dynamic.rs:54` | Internal CLI args: target, device, install/launch/capture_logs/duration/uninstall_after/dry_run, proxy, permissions, traffic_capture, frida_script(s), allow_frida, baseline, evidence_bundle |
| `LabManifest` | `dynamic.rs` | Advisory TOML allowlist: allowed_device_serials, allowed_packages |
| `CorrelatedFinding` | `dynamic.rs` | Static↔dynamic correlation result: score, correlation_type, enrichment |
| `CorrelationEngine` | `dynamic.rs` | Core correlation: ingests static + dynamic + Frida reports, produces CorrelationResult |
| `MobileBaseline` | `dynamic.rs` | Captured baseline for regression comparison |
| `FridaSession` | `frida.rs:22` | Frida session handle: device_id, is_simulation |
| `FridaScriptResult` | `frida.rs:29` | Script result: script_source, output, findings, duration_ms, structured_output |
| `FridaInstrumentation` | `frida.rs:42` | Frida summary: sessions, script_results, enabled_builtins, structured_results, correlation_notes, regression_notes |
| `TrafficSummary` | `traffic.rs:22` | Traffic summary: total_requests, cleartext_requests, unique_domains, suspicious_endpoints, findings |

### CLI Types (main crate)

| Type | Location | Description |
|------|----------|-------------|
| `MobileArgs` | `cli/mobile.rs` | Top-level args: path, json, output, quiet, command: Option<MobileSubcommand> |
| `MobileStaticArgs` | `cli/mobile.rs` | Static subcommand args |
| `DynamicMobileArgs` (CLI) | `cli/mobile.rs` | Dynamic subcommand args (clap-facing) |

## Behavior / Flow

### Static Analysis Pipeline

```
CLI: eggsec mobile <path> or eggsec mobile static <path>
  → handle_mobile() → EnforcementContext(StandardAssessment/SafeActive, "mobile")
  → run_cli() → run_static_cli()
    → validate path (exists, is_file, .apk/.ipa, 200 MiB size guard)
    → dispatch by extension:
       ├─ .apk → apk::analyze_apk(path)
       │    → ZIP open → extract manifest, network_config, secret_candidates, certs
       │    → parse_manifest() → detect binary AXML vs text XML
       │    │    ├─ binary: extract_string_pool() → linear chunk walk (START_TAG/END_TAG)
       │    │    └─ text: quick-xml Reader → event loop
       │    → parse_network_security_config()
       │    → scan_text_for_secrets_and_storage()
       │    → check_cert_for_debug()
       │    → MobileScanReport
       └─ .ipa → ipa::analyze_ipa(path)
            → ZIP open → locate Payload/*.app/Info.plist
            → plist deserialization → ATS, file-sharing, URL schemes, extensions
            → _CodeSignature, embedded.mobileprovision checks
            → secret scanning in .app assets
            → MobileScanReport
    → build_general_recommendations()
    → format (json/human) → write to file or stdout
```

### Dynamic Analysis Pipeline

```
CLI: eggsec mobile dynamic <target.apk> --device <serial> [--install] [--launch ...] [--dry-run]
  → handle_mobile() → EnforcementContext(DefenseLab/SafeActive, "mobile-dynamic")
  → extra runtime gate: !dry_run && !allow_dynamic_mobile → bail
  → extra Frida gate: frida_script && !dry_run && !allow_frida → bail (Intrusive tier)
  → run_dynamic_cli(args)
    → connect via ADB (AdbClient::connect or external adb)
    → optional: install APK, launch activity
    → capture logcat (duration-bounded)
    → optional: set global proxy, grant/revoke permissions, list permissions
    → optional: parse traffic capture file
    → optional: Frida instrumentation (connect, resolve script spec, run)
    → optional: capture baseline, compare to prior baseline
    → optional: correlate_findings() (static↔dynamic↔Frida)
    → optional: export evidence bundle (gzipped)
    → optional: uninstall app
    → best-effort cleanup (proxy reset, etc.)
    → DynamicMobileReport → format → output
```

### Frida Session Flow (`frida.rs`)

```
resolve_frida_script_spec("builtin:basic_method_trace") → embedded script source
  → connect(device) → FridaSession { device_id, is_simulation }
  → execute_script(session, script) → FridaScriptResult
  → redact_frida_evidence(result) → sanitized output
```

- Builtins: `basic_method_trace`, `crypto_keystore`, `bypass_validation`, `api_trace`
- Library: embedded `FRIDA_LIB_COMMON_HOOKS` (reusable components via `library:` prefix)
- User scripts: file paths resolved at runtime
- Real execution requires `frida` CLI in PATH + `frida-server` on device
- Dry-run always succeeds with stub session (no side effects)

### Handler Policy (`commands/handlers/mobile.rs:6-194`)

| Path | Mode | Risk | Features |
|------|------|------|----------|
| Static | `StandardAssessment` | `SafeActive` | `["mobile"]` |
| Dynamic (dry-run) | `DefenseLab` | `SafeActive` | `["mobile-dynamic"]` |
| Dynamic (real) | `DefenseLab` | `SafeActive` | `["mobile-dynamic"]` + `--allow-dynamic-mobile` |
| Dynamic + Frida (real) | `DefenseLab` | `Intrusive` | `["mobile-dynamic"]` + `--allow-dynamic-mobile` + `--allow-frida` |

## Safety Model

- **Static**: Offline, user-supplied binaries, bounded extraction (50 MiB total, 128 KiB per text scan, 200 MiB file guard), ZipSlip rejection, no network/device interaction
- **Dynamic dry-run**: Zero device/net touch, full valid report produced
- **Dynamic real**: Requires `--allow-dynamic-mobile` flag (audited), best-effort cleanup (proxy reset, app uninstall), all actions recorded in `actions_performed`
- **Frida real**: Additional `--allow-frida` gate, `Intrusive` risk tier, requires `frida` CLI + `frida-server` on device
- **Lab manifest**: Advisory TOML allowlist (allowed_device_serials, allowed_packages) — not enforced, lab visibility only
- **ADB**: Pure-Rust TCP framing for emulator primary; external `adb` CLI only for discovery convenience (`--list-devices`)
- **No panics**: All parse operations bounded; errors returned as `MobileError`

## Public API

| Function | Location | Description |
|----------|----------|-------------|
| `analyze_apk(path)` | `apk.rs:30` | Async APK static analysis → `MobileScanReport` |
| `analyze_ipa(path)` | `ipa.rs` | Async IPA static analysis → `MobileScanReport` |
| `run_static_cli(path, json, output, quiet)` | `lib.rs:137` | CLI entry for static analysis |
| `run_dynamic_cli(args)` | `dynamic.rs` | CLI entry for dynamic analysis |
| `to_scan_report_data(report)` | `lib.rs:296` | Bridge static report → `ScanReportData` |
| `to_report_envelope(report)` | `lib.rs:332` | Bridge static report → `ReportEnvelope` |
| `to_scan_report_data_dynamic(report)` | `dynamic.rs` | Bridge dynamic report → `ScanReportData` |
| `format_mobile_report(report)` | `lib.rs:249` | Human-readable formatting |
| `format_dynamic_report(report)` | `dynamic.rs` | Dynamic human-readable formatting |
| `correlate_findings(dynamic, static)` | `dynamic.rs` | Correlate static findings with dynamic observations |
| `correlate_reports(static, dynamic)` | `dynamic.rs` | CorrelationEngine: full correlation with scoring |
| `capture_baseline(report)` | `dynamic.rs` | Capture MobileBaseline from report |
| `compare_to_baseline(baseline, report)` | `dynamic.rs` | Compare report to prior baseline |
| `export_evidence_bundle(...)` | `dynamic.rs` | Gzipped evidence bundle export |
| `connect(device)` | `frida.rs:61` | Connect to Frida server on device |
| `execute_script(session, script)` | `frida.rs` | Execute Frida script |
| `basic_method_trace()` | `frida.rs` | Built-in method tracing script |
| `parse_traffic_capture(input)` | `traffic.rs:49` | Parse traffic capture → `TrafficSummary` |
| `parse_logcat_findings(log)` | `runtime.rs:25` | Parse logcat → `DynamicMobileFinding` entries |

## Integration Points

- **CLI dispatch**: `commands/handlers/mod.rs` → `handle_mobile()` (subcommand dispatch: static or dynamic)
- **CLI args**: `cli/mobile.rs` — `MobileArgs` with `MobileSubcommand::Static` / `MobileSubcommand::Dynamic`
- **Reporting bridge**: `to_scan_report_data()` categories: `mobile-{android,ios}-<native-category>` (static); `mobile-dynamic-android-<category>` (dynamic)
- **Auto-bridge**: `report convert` handler auto-detects `mobile-static` or `mobile-dynamic` native JSON
- **Python bindings**: `analyze_apk` and `analyze_ipa` exposed as stable-core operations (22 total operations)
- **Feature gate**: `mobile` (static always compiled in domain crate), `mobile-dynamic = ["mobile"]`
- **Domain descriptors**: `mobile-static` and `mobile-dynamic` in `domain/mod.rs` with `required_feature` gates

## Testing

- `apk.rs` (7 tests): Text manifest findings, ZipSlip rejection, empty manifest, network config, insecure storage, binary AXML, invalid input
- `ipa.rs`: Synthetic IPA tests (plist, entitlements, signing, transport)
- `dynamic.rs`: Dynamic report generation, correlation, baselines, evidence bundles
- `frida.rs`: Script resolution, builtin dispatch, dry-run sessions
- `traffic.rs`: Text log parsing, HAR parsing, cleartext/suspicious detection
- `runtime.rs`: Logcat finding extraction (permission, crash, cleartext, secrets)
- `lib.rs` (6 tests): Report defaults, formatting, bridge roundtrips, iOS/Android categories
- Integration via `make test-ci`

## Invariants & Gotchas

1. **Static always compiled**: `apk.rs` and `ipa.rs` are unconditionally compiled in the domain crate — no feature gate
2. **Dynamic behind `mobile-dynamic`**: All of `adb.rs`, `dynamic.rs`, `frida.rs`, `traffic.rs`, `runtime.rs` require `#[cfg(feature = "mobile-dynamic")]`
3. **Binary AXML is minimal**: Only decodes string pool + START_TAG/END_TAG — no resource table, no full namespace resolution (`apk.rs:188-203`)
4. **200 MiB file guard**: `lib.rs:172` rejects files > 200 MiB before any analysis
5. **50 MiB extraction budget**: APK total extraction capped at 50 MiB (`apk.rs:50`)
6. **128 KiB per-text scan**: Individual text assets scanned for secrets capped at 128 KiB (`apk.rs:51`)
7. **1 MiB traffic capture cap**: Parser input truncated at 1 MiB (`traffic.rs:47`)
8. **Handler forces dry-run for real Frida**: `handlers/mobile.rs:33-38` checks `!d.frida_script.is_empty() && !d.dry_run` to set Intrusive risk
9. **Two distinct `DynamicMobileArgs` types**: CLI clap args (`cli/mobile.rs`) vs internal lib args (`dynamic.rs:54`) — handler maps between them (`handlers/mobile.rs:87-115`)
10. **Frida real requires CLI + server**: `frida.rs:11` — no heavy frida crate dependency; shells out to `frida` CLI
11. **Bridge skips empty findings**: Valid — 0 findings in bridge is expected for clean artifacts
12. **CorrelationEngine is non-breaking**: New fields are optional with serde defaults; existing `correlate_findings()` users unaffected

---

*Last verified against source: 2026-08-25*
