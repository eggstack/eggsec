# Mobile Module Override

## Key Files

| File | Purpose |
|------|---------|
| `crates/eggsec/src/mobile/mod.rs` | Module entry, public types (MobilePlatform, MobileFinding, MobileScanReport), run_cli dispatcher, format_mobile_report, to_scan_report_data bridge to unified reports; cfg-gated reexports + run_dynamic_cli for dynamic |
| `crates/eggsec/src/mobile/apk.rs` | Android APK static analysis (ZIP + AndroidManifest.xml binary/text AXML parsing, permissions, components, network-security-config, secrets, debug certs) |
| `crates/eggsec/src/mobile/ipa.rs` | iOS IPA static analysis (ZIP + Info.plist + embedded.mobileprovision + _CodeSignature, get-task-allow, provisioning profile risks) |
| `crates/eggsec/src/mobile/dynamic.rs` | Dynamic types (DynamicMobileReport/Finding, LabManifest, DynamicMobileArgs), run_dynamic_cli dispatcher, format_dynamic_report, to_scan_report_data_dynamic bridge |
| `crates/eggsec/src/mobile/adb.rs` | Pure-Rust ADB TCP framing (CNXN/OPEN/WRTE etc.) + AdbClient/AdbConnection (list, connect, shell, sync_push, install, launch, uninstall, capture_logcat); external `adb` only for discovery |
| `crates/eggsec/src/mobile/runtime.rs` | High-signal logcat parser (parse_logcat_findings): runtime-permission, crash-log, cleartext-observed, log-secret-leak (basic redaction) |

## Implementation Notes

- **Pure-Rust only**: Uses `zip` crate (under `mobile` feature). No external binaries, no shelling out, no Frida/dynamic, no decompilation. Bounded extraction + size guards (200 MiB).
- **Phase 1 scope**: Static-only manifest/config surface (high-signal findings) + Phase 1 dynamic (Android ADB core + high-signal runtime logcat analysis) complete 2026-06-12. Lab/defense validation framing only. Explicit "authorized lab/defensive validation use only" note in CLI for both paths.
- **Types**: `MobilePlatform` (Android/Ios), `MobileFinding` (category, Severity, title, description, recommendation, optional evidence), `MobileScanReport` (target, platform, app_id/version, findings, recommendations, duration). All serializable.
- **Bridge**: `to_scan_report_data()` converts to `crate::output::convert::ScanReportData` for JSON/SARIF/etc. consumers (pattern matches wireless module).
- **CLI surface**: Standalone `eggsec mobile <path.{apk,ipa}>` (legacy direct) or `eggsec mobile static ...` (static); `eggsec mobile dynamic ...` under `mobile-dynamic` feature. Handler uses `evaluate_and_enforce_operation` with `SafeActive` risk + feature gate (`mobile` for static, `mobile-dynamic` for dynamic). Dynamic adds DefenseLab mode + explicit `--allow-dynamic-mobile` gate for non-dry runs (audited). Not part of TUI tabs or pipeline profiles.
- **Enforcement**: `SafeActive` (low risk tier), no scope requirement, local file target only. Policy gate + descriptor in `commands/handlers/mobile.rs`. Dynamic descriptor: `DefenseLab + SafeActive + required_features: ["mobile-dynamic"]`. Dry-run always valid; real runs require explicit allow + best-effort cleanup (actions audited in report).

- **Dynamic (P1)**: Pure-Rust ADB TCP framing (emulator-XXXX primary; host:port supported); external `adb` binary only for `list_devices` convenience. High-signal-only log parser (no full log processing). `DynamicMobileReport` / `DynamicMobileFinding` + `to_scan_report_data_dynamic` bridge (categories `mobile-dynamic-android-*`). Policy enforced in handler (DefenseLab mode + explicit `--allow-dynamic-mobile` for non-dry runs; audited). Dry-run always valid (produces complete serializable report, zero device/net side effects). All actions recorded in `actions_performed`; best-effort cleanup (uninstall) always attempted on real install paths. Standalone defense-lab (MCP/agent absent). See `dynamic.rs:127`, `adb.rs`, `runtime.rs`, `commands/handlers/mobile.rs:26`.

## Testing Guidance

- Unit tests use synthetic ZIP archives created in-memory (see `#[test]` blocks in `apk.rs` and `ipa.rs` using `zip::ZipWriter` + `Cursor`).
- Prefer small, deterministic synthetic fixtures for regression (manifest XML, binary AXML chunks, plists, provision profiles).
- No real APKs/IPAs or device tooling required for core parser tests.
- Run with feature:

```bash
cargo check -p eggsec --features mobile
cargo test --lib -p eggsec mobile::
cargo clippy --lib -p eggsec --features mobile

# Dynamic (P1)
cargo check -p eggsec --features mobile-dynamic
cargo test --lib -p eggsec --features mobile-dynamic
cargo clippy --lib -p eggsec --features mobile-dynamic
```

- Dynamic-specific tests: adb framing (duplex mocks in `adb.rs` tests), runtime parser on synthetic fixtures (`runtime.rs`), `Dynamic*` serde/bridge/dry-run (`dynamic.rs` tests + `to_scan_report_data_dynamic` categories), handler policy paths (static vs dynamic descriptors + allow gate). No real devices required for unit tests.

## Related

See `architecture/defense_lab.md`, wireless module (similar standalone defense-lab + bridge pattern; wireless active as direct precedent for dynamic mobile), `commands/handlers/mobile.rs`, `cli/mobile.rs`, and the mobile section in `AGENTS.md` (Key Types, Feature Flags, Security Notes). Dynamic loadout design in root `plans/dynamic-mobile-testing-loadout-design-plan.md`; Phase 1 implementation handoff in `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed 2026-06-12; mirrors wireless active precedent).

Phase 1 static closed 2026-06-11. Phase 1 dynamic (ADB + logcat) complete 2026-06-12 per handoff plan (CLI subcommands, types + dispatcher + bridge, pure-Rust ADB + parser, policy + handler, tests, report auto-bridge, docs). Module stable for P1 scope (static + dynamic). Future phases per design plan.
