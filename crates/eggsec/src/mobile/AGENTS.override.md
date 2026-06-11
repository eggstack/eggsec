# Mobile Module Override

## Key Files

| File | Purpose |
|------|---------|
| `crates/eggsec/src/mobile/mod.rs` | Module entry, public types (MobilePlatform, MobileFinding, MobileScanReport), run_cli dispatcher, format_mobile_report, to_scan_report_data bridge to unified reports |
| `crates/eggsec/src/mobile/apk.rs` | Android APK static analysis (ZIP + AndroidManifest.xml binary/text AXML parsing, permissions, components, network-security-config, secrets, debug certs) |
| `crates/eggsec/src/mobile/ipa.rs` | iOS IPA static analysis (ZIP + Info.plist + embedded.mobileprovision + _CodeSignature, get-task-allow, provisioning profile risks) |

## Implementation Notes

- **Pure-Rust only**: Uses `zip` crate (under `mobile` feature). No external binaries, no shelling out, no Frida/dynamic, no decompilation. Bounded extraction + size guards (200 MiB).
- **Phase 1 scope**: Static-only manifest/config surface (high-signal findings). Lab/defense validation framing only. Explicit "authorized lab/defensive validation use only" note in CLI.
- **Types**: `MobilePlatform` (Android/Ios), `MobileFinding` (category, Severity, title, description, recommendation, optional evidence), `MobileScanReport` (target, platform, app_id/version, findings, recommendations, duration). All serializable.
- **Bridge**: `to_scan_report_data()` converts to `crate::output::convert::ScanReportData` for JSON/SARIF/etc. consumers (pattern matches wireless module).
- **CLI surface**: Standalone `eggsec mobile <path.{apk,ipa}>` (gated on feature). Handler uses `evaluate_and_enforce_operation` with `SafeActive` risk + `required_features: ["mobile"]`. Not part of TUI tabs or pipeline profiles.
- **Enforcement**: `SafeActive` (low risk tier), no scope requirement, local file target only. Policy gate in `commands/handlers/mobile.rs`.

## Testing Guidance

- Unit tests use synthetic ZIP archives created in-memory (see `#[test]` blocks in `apk.rs` and `ipa.rs` using `zip::ZipWriter` + `Cursor`).
- Prefer small, deterministic synthetic fixtures for regression (manifest XML, binary AXML chunks, plists, provision profiles).
- No real APKs/IPAs or device tooling required for core parser tests.
- Run with feature:

```bash
cargo check -p eggsec --features mobile
cargo test --lib -p eggsec mobile::
cargo clippy --lib -p eggsec --features mobile
```

## Related

See `architecture/defense_lab.md`, wireless module (similar standalone defense-lab + bridge pattern), `commands/handlers/mobile.rs`, `cli/mobile.rs`, and the mobile section in `AGENTS.md` (Key Types, Feature Flags, Security Notes).

(Stub created 2026-06-11 per AGENTS.md update task; expand with real findings once Phase 1 hardens.)
