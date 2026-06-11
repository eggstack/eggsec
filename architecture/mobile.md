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

Phase 1 static-only (pure-Rust, SafeActive, standalone CLI + optional report bridge). No dynamic capabilities, no Frida, no TUI tab, no pipeline profile integration (`mobile-static` / `mobile-regression` are aspirational). See plans/mobile-first-handoff-plan.md and docs/MOBILE.md.

Integration points:
- Enforcement: `CommandContext::evaluate_and_enforce_operation` (OperationMode::StandardAssessment, OperationRisk::SafeActive, required_features: ["mobile"]).
- Reporting: local types emitted directly; `to_scan_report_data` available for OutputFormat consumers (JSON/SARIF/etc.).
- Feature gate + policy: `mobile` in Cargo.toml (depends on optional `zip`/`plist`); listed in `full`; policy decision in `config/policy_decision.rs`.
- Handler dispatch: `commands/handlers/mod.rs` and `cli/mod.rs`.

Safety model: Lab/defense use only. Requires user-provided test builds (no internet fetch). Explicit provenance note. Bounded, offline, no side effects. All operations under central `EnforcementContext`.

See `crates/eggsec/src/mobile/`, `crates/eggsec/src/commands/handlers/mobile.rs`, and `crates/eggsec/Cargo.toml:310`.
