# Mobile Static Analysis

Eggsec provides standalone static security analysis of Android APK and iOS IPA packages via the `mobile` feature. Phase 1 delivers reliable, high-signal static analysis for authorized lab and defense-validation use only.

**This is static analysis only.** The tool performs no dynamic execution, no Frida or runtime instrumentation, no device interaction, no network activity, and makes no outbound connections. All work is offline on user-supplied binaries you explicitly provide.

## Feature Gate

Build with `--features mobile` (or `--features full`).

```bash
cargo build --release -p eggsec-cli --features mobile
```

## Safety & Scope

- Use **only on applications you own or are explicitly authorized to assess** (lab, authorized defensive validation). Supply your own test builds with controlled provenance.
- **Static analysis only** — no dynamic analysis, no instrumentation, no network in the tool, no shelling out, no device flashing or app installation.
- Bounded, pure-Rust ZIP + plist + minimal AXML handling with ZipSlip rejection, size caps (200 MiB archive, ~50 MiB extraction budget), and no external dependencies or tools.
- You are responsible for the provenance, chain-of-custody, and safe destruction of test artifacts after lab use. The tool does not enforce or audit supply-chain integrity.
- Never run against production or customer-supplied binaries without explicit authorization and isolation.
- Production impact: none (offline file analysis), but always operate in a controlled lab environment.

See also: [docs/SAFETY.md](SAFETY.md), `architecture/mobile.md`, AGENTS.md, and the central `EnforcementContext` policy gate (handler uses `SafeActive` risk tier + required `"mobile"` feature).

## CLI Usage

```bash
# Basic analysis (auto-detects APK or IPA by extension)
eggsec mobile app.apk
eggsec mobile MyApp.ipa

# Structured JSON output
eggsec mobile release.apk --json

# Write results to file (human or JSON)
eggsec mobile /path/to/internal.apk -o mobile-findings.json
eggsec mobile app.ipa --json -o report.json

# Quiet mode (suppress the lab-use note on stderr)
eggsec mobile app.apk --quiet --json

# Combined with global flags
eggsec mobile app.apk --json --output findings.json
eggsec --json mobile app.ipa -o out.json

# Help
eggsec mobile --help
eggsec --help | grep -A 20 mobile
```

The command auto-selects the analyzer based on `.apk` (Android) or `.ipa` (iOS) extension. Only these two extensions are accepted. A prominent note about lab/defense use is printed unless `--quiet`.

## What It Detects (High-Signal Static)

Findings are severity-rated (Critical/High/Medium/Low/Info) with title, description, recommendation, category, and optional evidence. Common categories: `manifest`, `permission`, `transport`, `secret`, `storage`, `signing`, `build`, `url-scheme`.

**Android (APK):**
- `android:debuggable="true"` in release builds (High)
- `android:allowBackup` true or default (Medium) — data exfil via adb backup
- `android:usesCleartextTraffic="true"` or equivalent (High)
- Exported components (`activity`/`service`/`receiver`/`provider`) without `protectionLevel` or with intent-filters (Medium/High)
- Dangerous / over-privileged permissions (e.g. READ_SMS, WRITE_EXTERNAL_STORAGE patterns) (Medium)
- Insecure `network_security_config.xml` (cleartext, no pinning, weak trust anchors)
- Hardcoded secrets, tokens, passwords, API keys in strings, XML, JSON, properties, JS assets (High/Medium)
- Insecure storage hints
- v1 signing cert anomalies or debug keystores (Low)

**iOS (IPA):**
- Weak `NSAppTransportSecurity` (NSAllowsArbitraryLoads, exception domains allowing HTTP, weak min TLS) (High)
- `UIFileSharingEnabled` or `LSSupportsOpeningDocumentsInPlace` (Medium) — Files/iTunes exposure
- Custom URL schemes (Low) — potential hijacking surface
- Hardcoded secrets in bundle assets (.plist, .json, .strings, .js) (High/Medium)
- Missing `_CodeSignature/` directory (Low)
- Debug/ad-hoc/development provisioning indicators (`get-task-allow`, development `aps-environment`) (Low)
- General guidance note recommending iOS Keychain for secrets (always emitted as defensive reminder; Info)

General recommendations are always appended (prefer platform secure storage, HTTPS + pinning, destroy test builds, combine with SAST/dynamic review).

Findings and metadata are also exposed via `to_scan_report_data()` for unified reporting.

## Output & Integration

- Human-readable text (default) includes target metadata, per-finding blocks (severity/title/category/description/recommendation/evidence), and Recommendations section + duration.
- `--json` produces the full `MobileScanReport` (target, platform, app_id, version, findings array, recommendations, scan_type: "mobile-static", duration_ms, etc.). This native shape is accepted directly by `eggsec report convert` (auto-bridged to `ScanReportData` when the `mobile` feature is enabled).
- `-o` / `--output` supported for both modes (writes to file; still prints path to stderr unless quiet).
- Structured findings feed into `ScanReportData` (via `to_scan_report_data()`) for SARIF, JUnit, HTML, Markdown, CSV, etc. pipelines. The bridge is optional and opt-in: use it (or rely on the CLI auto-bridge for `report convert`) when you need unified report consumers; otherwise consume the native `MobileScanReport` directly for lab-specific workflows.
- Compatible with the existing report tooling (native JSON works via auto-bridge; explicit bridge also available programmatically):

```bash
eggsec mobile app.apk --json -o mobile.json
eggsec report convert mobile.json -f sarif -o mobile.sarif
eggsec report convert mobile.json -f html -o mobile.html
eggsec report convert mobile.json -f markdown -o mobile.md
eggsec report convert mobile.json -f junit -o mobile.xml
```

See `docs/USAGE.md` (Report Management section) and `docs/FINDINGS_SCHEMA.md` for unified report consumers.

## Integration with Reporting Pipeline

`eggsec mobile` is intentionally a **standalone defense-lab CLI** (not a `ScanProfile` pipeline stage). It emits local `MobileScanReport` / `MobileFinding` types directly for human and `--json` use. An optional `to_scan_report_data()` bridge (in `mobile/mod.rs`) converts to the canonical `ScanReportData` used by `eggsec-output` converters (SARIF, JUnit, HTML, Markdown, CSV, JSON roundtrip, trend, etc.).

- Use the native types for lab-specific flows, regression on `Mobile*` shapes, or when you do not need unified consumers.
- Use `--json` + `eggsec report convert` (or call `to_scan_report_data` in your own tooling) when you want SARIF/JUnit/etc. or to feed into `report trend` / other unified consumers.
- No `ScanProfile` integration in Phase 1 (`mobile-static` / `mobile-regression` profiles are aspirational; see `architecture/defense_lab.md` Future and `architecture/mobile.md`).
- Design decision (Phase 1 close 2026-06-11): keep the surface standalone and lightweight; the bridge provides reporting unification without forcing the module into the main chained pipeline.

The auto-bridge in `commands/handlers/report.rs` makes the documented `--json | report convert` flow work out of the box when built with `--features mobile`. Categories in bridged output are of the form `mobile-{android,ios}-<native-category>` (e.g. `mobile-android-manifest`, `mobile-android-permission`, `mobile-ios-secret`, `mobile-ios-transport`) to preserve signal while satisfying the platform prefix. Evidence in bridged findings carries through useful details (e.g. permission name like "READ_SMS", manifest key, secret pattern like "api_key=..."); empty findings produce a valid bridge with 0 findings (tested).

## Limitations (Phase 1)

- Static analysis surface only. No runtime behavior, no Frida, no dynamic hooking, no emulator/device interaction.
- No deep DEX decompilation, full bytecode analysis, or control-flow graphs.
- No complete third-party dependency / supply-chain graph (only basic indicators and secret patterns).
- Limited to manifest, plist, network config, signing markers, and bounded small-text asset scans. Large resources and native libraries are size-capped or skipped.
- No automatic app installation, permission granting, or traffic capture.
- iOS analysis is IPA-bundle only (no .app bundles or xcarchives directly).
- No TUI tab in this phase (CLI primary).

## Recommendations

- **Lab workflow**: Build your own debug/test variants with known provenance (e.g. from CI with signing disabled only in isolated jobs). Run `eggsec mobile` as an early static gate before any dynamic work.
- Combine with:
  - SAST / dependency scanners (e.g. for full SDK enumeration)
  - Manual code review of high-risk flows
  - Authorized dynamic testing (Frida, objection, or platform debug bridges) inside a controlled lab with device isolation and no production data
  - Backend/API testing of the mobile app's server surface using the same scope and `eggsec` pipeline
  - Supply-chain / SBOM tools for third-party library tracking
- Always review findings against the app's actual data classification and threat model. Many "Medium" items are acceptable in internal tools but unacceptable for customer-facing or regulated apps.
- After lab use: securely destroy or archive test builds; do not leave debuggable or development-signed artifacts in shared locations.
- For regression: capture `--json` outputs and diff with `eggsec report diff` or your own tooling.

## Policy Note

The `mobile` command is gated through the central `CommandContext::evaluate_and_enforce_operation()` (see `config/policy_decision.rs` and `commands/handlers/mobile.rs`). It declares:

- `operation: "mobile-static"`
- `risk: OperationRisk::SafeActive`
- `required_features: ["mobile"]`

The feature must be present at compile time. `EnforcementContext` (CLI `ManualPermissive`/`ManualGuarded` etc.) will deny if the feature is missing or if broader policy rules prohibit the operation. Strict profiles (MCP, agent, CI) treat the feature requirement as mandatory. No special overrides bypass the feature gate.

## Future

- **Phase 2**: Deeper manifest/config analysis, basic library/SDK detection, improved iOS coverage, richer recommendations, and exportable evidence bundles.
- **Phase 3**: Optional pipeline integration (`mobile-static` / `mobile-regression` profiles), combined web+mobile backend testing, and gated dynamic capabilities (Frida-based instrumentation behind additional safety + capability flags and explicit lab authorization).
- Architecture document: `architecture/mobile.md`.
- TUI tab and broader `ScanProfile` support in later phases.

## Data Model (Key Types)

```rust
pub enum MobilePlatform { Android, Ios }

pub struct MobileFinding {
    pub category: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    pub evidence: Option<String>,
}

pub struct MobileScanReport {
    pub target: String,
    pub scan_type: String, // "mobile-static"
    pub platform: MobilePlatform,
    pub app_id: Option<String>,
    pub version: Option<String>,
    pub timestamp: String,
    pub findings: Vec<MobileFinding>,
    pub recommendations: Vec<String>,
    pub duration_ms: u64,
}

pub fn to_scan_report_data(result: &MobileScanReport) -> ScanReportData { ... }
```

See `crates/eggsec/src/mobile/{mod,apk,ipa}.rs` for full definitions and analyzers. Historical plan: `plans/mobile-first-handoff-plan.md`.

## Example Output (Human, abbreviated)

```
NOTE: Mobile static analysis is for authorized lab/defensive validation use only. ...
Mobile Static Analysis (android)
Target: /tmp/test.apk
App ID: com.example.vulnerable
Version: 1.0
Findings: 3

  1. [High] Debuggable build in production artifact (manifest)
     android:debuggable="true" enables debugging...
     Rec: Ensure release builds explicitly set debuggable="false"...
     Evidence: debuggable=true

  ...

Recommendations:
  - Review all findings in the context of the app's data classification...
  - Prefer platform secure storage...
  - This is static analysis only. Combine with...
Duration: 123 ms
```

(Full structured JSON available via `--json`.)
