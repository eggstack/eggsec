---
name: eggsec-mobile
description: "Mobile app security analysis - use when working with APK/IPA static analysis, manifest checks, mobile dynamic testing (ADB/Frida), or the eggsec-mobile-lab domain crate."
---

# Eggsec Mobile Skill

Mobile application security analysis module (static APK/IPA checks plus optional dynamic Android testing).

## Module Locations

- Engine surface: `crates/eggsec/src/mobile/` (`mod.rs` CLI glue, re-exports)
- Domain crate: `crates/eggsec-mobile-lab/src/` (parsers, findings, report types)

## Features

- `mobile` - APK/IPA static analysis; pure-Rust parsers (zip, plist); no system deps
- `mobile-dynamic` - Android runtime testing via ADB (+ Frida instrumentation); requires `mobile`

## Key Types (eggsec-mobile-lab)

- `MobileError` - Error enum for parsing/analysis failures
- `MobilePlatform` - `Android` / `Ios`
- `MobileFinding` - Single finding with severity and evidence
- `MobileScanReport` - Aggregated scan result
- `format_mobile_report()` - Human-readable rendering
- `to_scan_report_data()` / `to_report_envelope()` - Bridges to unified reporting

## CLI Integration

- Command: `eggsec mobile <args>` (feature-gated: `mobile`)
- Dynamic mode: `mobile-dynamic` feature adds ADB-backed commands
- Handler: `commands/handlers/mobile.rs`
- CLI args: `cli/mobile.rs`

## Patterns

### Static Analysis Workflow

1. Build with `cargo check -p eggsec --features mobile`
2. Run against an APK/IPA in a lab: `eggsec mobile --help` for current flags
3. Findings convert to `ScanReportData` / `ReportEnvelope` for SARIF/JUnit/HTML output

### Adding a New Static Check

1. Implement the check in `crates/eggsec-mobile-lab/src/` (parser modules live there)
2. Emit a `MobileFinding` with severity and evidence
3. Add tests next to the parser under test

## Testing

```bash
cargo check -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
cargo test -p eggsec-mobile-lab
```

Dynamic tests require a connected device/emulator; see `scripts/test-mobile-dynamic.sh`.

## Resources

- `crates/eggsec/src/mobile/AGENTS.override.md` - Module guidance
- `docs/MOBILE.md` - Full usage documentation
- `architecture/mobile.md` - Architecture deep dive
