# Mobile Module

## Purpose

Standalone static security analysis of Android APKs and iOS IPAs for authorized lab / defense-validation use (Phase 1). Pure-Rust implementation (ZIP extraction + bounded AXML for APKs; ZIP + plist for IPAs). Detects high-signal manifest/config issues: debuggable/backup/exported components, over-privileged permissions, insecure transport (cleartext/weak TLS), hardcoded secrets, insecure storage patterns, signing indicators, WebView/JS bridge risks, and basic supply-chain hints. Produces local `MobileScanReport` / `MobileFinding` (with optional `to_scan_report_data` bridge for unified consumers). No dynamic instrumentation, Frida, network activity, or device interaction.

## CLI Behavior

- Build with `--features mobile` (or `--features full`).
- `eggsec mobile <path-to-.apk-or-.ipa>` (supports `--json`, `-o/--output`, `-q/--quiet`).
- Pure offline analysis on user-supplied lab binaries only. Size guard (200 MiB). Prints lab-only framing note unless `--quiet`.
- Direct human/JSON output from local report types; optional bridge to `ScanReportData` for JSON/SARIF/JUnit/etc. (mirrors wireless pattern).

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `MobilePlatform` | `mobile/mod.rs` | Enum: Android, Ios |
| `MobileFinding` | `mobile/mod.rs` | Severity-rated finding (category, title, description, recommendation, optional evidence) |
| `MobileScanReport` | `mobile/mod.rs` | Full report (target, scan_type="mobile-static", platform, app_id, version, findings, recommendations, duration) |
| `MobileArgs` | `cli/mobile.rs` | CLI args (path, json, output, quiet) + `MOBILE_ABOUT` |

## Files

| File | Description |
|------|-------------|
| `mobile/mod.rs` | Core types, `run_cli`, `format_mobile_report`, `build_general_recommendations`, `to_scan_report_data` bridge |
| `mobile/apk.rs` | APK analysis (zip open, manifest parsing (text + binary AXML), permissions, components, network-security-config, secret scanning, cert checks) |
| `mobile/ipa.rs` | IPA analysis (zip open, Info.plist, embedded.mobileprovision, code signature markers, transport/entitlements) |
| `cli/mobile.rs` | `MobileArgs` + `MOBILE_ABOUT` |
| `commands/handlers/mobile.rs` | `handle_mobile` (EnforcementContext via `evaluate_and_enforce_operation`, SafeActive + feature gate) |

## Status

Phase 1 static-only (pure-Rust, SafeActive, standalone CLI + optional report bridge). No dynamic capabilities, no Frida, no TUI tab, no pipeline profile integration (`mobile-static` / `mobile-regression` are aspirational). Phase 1 closed as complete standalone capability on 2026-06-11.

See `docs/MOBILE.md`.

## Integration with Reporting Pipeline

`eggsec mobile` is intentionally a **standalone defense-lab CLI** (not a `ScanProfile` pipeline stage). It emits local `MobileScanReport` / `MobileFinding` types directly for human and `--json` use.

An optional `to_scan_report_data()` bridge (in `mobile/mod.rs`; mirrors wireless pattern) converts to `ScanReportData` for unified consumers (SARIF, JUnit, HTML, Markdown, CSV, JSON, trend, etc.).

The `report convert` handler auto-bridges native `MobileScanReport` JSON when the `mobile` feature is enabled, so `eggsec mobile app.apk --json -o m.json ; eggsec report convert m.json -f ...` works directly.

Categories in bridged output are `mobile-{android,ios}-<native-category>` (e.g. `mobile-android-manifest`, `mobile-android-permission`, `mobile-ios-secret`) to preserve the original finding category signal while satisfying the platform prefix requirement. Evidence carries through (e.g. permission name like "READ_SMS", manifest key like "debuggable=true", secret pattern like "assets/config.json: ...api_key=..."); empty findings are valid (0 findings in bridge).

**Design decision (Phase 1 close 2026-06-11)**: Standalone CLI-only (no TUI, no pipeline stages/profiles); optional bridge provides reporting unification without forcing `ScanProfile` integration (`mobile-static`/`mobile-regression` remain aspirational per `architecture/defense_lab.md` Future). Use native types for lab-specific flows; use bridge (or `report convert` on native JSON) for unified report consumers. Integration is lightweight and opt-in.

Integration points:
- Enforcement: `CommandContext::evaluate_and_enforce_operation` (OperationMode::StandardAssessment, OperationRisk::SafeActive, required_features: ["mobile"]).
- Reporting: local types emitted directly; `to_scan_report_data` available for OutputFormat consumers (JSON/SARIF/etc.); auto-bridge in report handler.
- Feature gate + policy: `mobile` in Cargo.toml (depends on optional `zip`/`plist`); listed in `full`; policy decision in `config/policy_decision.rs`.
- Handler dispatch: `commands/handlers/mod.rs` and `cli/mod.rs`.

Safety model: Lab/defense use only. Requires user-provided test builds (no internet fetch). Explicit provenance note. Bounded, offline, no side effects. All operations under central `EnforcementContext`.

See `crates/eggsec/src/mobile/`, `crates/eggsec/src/commands/handlers/mobile.rs`, and `crates/eggsec/Cargo.toml:310`.
