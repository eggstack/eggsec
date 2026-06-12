# Dynamic Mobile Application Testing (DMAT) Loadout Design & Implementation Plan

**Date**: 2026-06-12  
**Status**: Draft — Ready for Team Review & Handoff  
**Branch**: `main` (or feature/mobile-dynamic-loadout-plan)  
**Related**: Newly added mobile static analysis (Phase 1 complete 2026-06-11), `docs/MOBILE.md`, `architecture/mobile.md`, `crates/eggsec/src/mobile/{mod,apk,ipa}.rs`, wireless active attacks loadout plans (pattern reference), `docs/SAFETY.md`, EnforcementContext / OperationRisk model, `AGENTS.override.md` in mobile dir  
**Authoring Note**: Generated via detailed analysis of current codebase using GitHub connector tools (get_file_contents on README, docs/MOBILE.md, architecture/mobile.md, Cargo.toml, cli/mobile.rs, commands/handlers/mobile.rs implied, mobile/mod.rs, apk/ipa structure, plans/ for wireless precedent). Intended as complete handoff artifact for eggstack team to expand static mobile into full dynamic loadout while preserving rigorous safety model.

---

## 1. Executive Summary

This plan outlines the design and phased implementation for expanding Eggsec's **newly added mobile static analysis module** (Phase 1, pure-Rust APK/IPA manifest & config checks, completed 2026-06-11) into a **gated Dynamic Mobile Application Testing (DMAT) loadout**.

**Goal**: Enable controlled, lab-only dynamic/runtime mobile app security testing capabilities (Android-focused initially) for defense validation, regression testing, and comprehensive mobile appsec assessment in authorized environments. The loadout must extend Eggsec's safety model (scope/policy gating where applicable, feature flags, execution budgets, auditability, explicit warnings, standalone defense-lab surface) without introducing accidental misuse vectors.

**Key Principles** (non-negotiable, mirroring wireless active precedent):
- **Defense-lab / regression focus only** — never general mobile exploitation or offensive framework. All dynamic actions on user-supplied test builds and lab devices/emulators you own or are explicitly authorized to test.
- **Standalone-complete surface** like `eggsec mobile` (static) and `eggsec wireless` (passive + advanced). MCP/agent tool exposure intentionally absent initially (consistent with mobile/wireless design decisions; reporting bridge remains available).
- **Phased & heavily gated**: New `mobile-dynamic` feature flag (depends on `mobile`). Additional runtime confirmations, device/app allowlists (lab manifests), `--allow-dynamic-mobile` style overrides (audited), packet/action budgets, dry-run always supported.
- **Leverage existing patterns**: Extend `MobileScanReport` / `MobileFinding` or introduce parallel `DynamicMobile*` types + `to_scan_report_data()` bridge evolution, EnforcementContext (`SafeActive` + new `MobileDynamic` risk tier or capability flag), CLI handler structure, future TUI tab (reuse recent 10-phase TUI architecture), reporting pipeline.
- **Pragmatic & Rust-native where possible**: Pure-Rust for analysis/bridge; ADB protocol helpers or minimal wrappers for Android device control (install, launch, logcat, permission grants); documented high-quality integration points for mitmproxy / existing Eggsec proxy pool for traffic observation; optional gated external tool wrappers (Frida, objection) only behind extra sub-feature and explicit policy.
- **Documentation-first & high-signal**: Every capability surfaces prominent legal/ethical/lab-use warnings, hardware prerequisites, provenance requirements for test APKs, and clear recommendations. Combine static + dynamic findings in unified reports.

**Deliverables**:
- This plan document: `plans/dynamic-mobile-testing-loadout-design-plan.md`.
- Feature flag `mobile-dynamic` (or `mobile-advanced`) in `crates/eggsec/Cargo.toml` (depends on `mobile`).
- New module(s) under `crates/eggsec/src/mobile/` (e.g. `dynamic.rs`, `adb.rs`, `runtime.rs` or `dynamic/` subdir).
- CLI extensions (`eggsec mobile dynamic ...` or dedicated `eggsec mobile-dynamic` subcommand group; support device targeting, dry-run, json, budgets).
- Extended or parallel report types + updated `to_scan_report_data` bridge (new categories `mobile-dynamic-android-*` etc.).
- Policy / EnforcementContext extensions (handler in `commands/handlers/mobile.rs` or new dynamic handler).
- Updated docs (`MOBILE.md` with Dynamic section, `architecture/mobile.md`, `SAFETY.md`, `CAPABILITIES.md`, README quick-ref and lab defense commands table).
- Optional future TUI tab integration (Wireless tab precedent).
- Unit + lab hardware/emulator tests.
- Example lab workflows (static baseline → dynamic observation → regression diff).

**Timeline Suggestion**: Phase 1 (Android ADB core + proxy integration + log analysis) ~3-4 weeks post-review; full initial loadout (Phases 1-2) 6-8 weeks with parallel policy/TUI work. Aggressive but realistic given wireless precedent.

**Success Criteria**: All dynamic operations require explicit feature + policy approval + device provenance note; dry-run produces valid structured output usable in `report convert` / trend; lab regression workflows (e.g. "measure permission prompt behavior or network cleartext under dynamic run vs static manifest") fully supported with before/after and temporal summaries.

---

## 2. Background & Current State

### 2.1 Mobile Static Analysis (Newly Added, Phase 1 Complete)

The mobile module (`crates/eggsec/src/mobile/mod.rs` ~15kB + apk.rs ~46kB + ipa.rs ~32kB) provides:
- Pure-Rust ZIP extraction + bounded AXML parsing (AndroidManifest.xml) for APKs; ZIP + plist parsing for IPAs.
- High-signal static findings: debuggable/allowBackup/exported components, dangerous permissions, insecure transport (cleartext/weak NSAppTransportSecurity), hardcoded secrets in assets/strings/plist, signing anomalies, WebView/JS bridge risks, basic supply-chain hints.
- `MobilePlatform` enum, `MobileFinding` (category, Severity, title, description, recommendation, optional evidence), `MobileScanReport` (target, platform, app_id, version, findings, recommendations, duration_ms, scan_type="mobile-static").
- `run_cli` dispatcher (validates .apk/.ipa, size guard 200 MiB, dispatches to apk/ipa analyzers).
- Human pretty output + `--json` + `-o/--output` + `--quiet`.
- `to_scan_report_data()` bridge (mirrors wireless) producing `ScanReportData` with platform-prefixed categories (`mobile-android-manifest`, `mobile-ios-transport`, etc.) and evidence preservation. Auto-bridged in `report convert` when feature enabled.
- Standalone CLI (`eggsec mobile <path>`), gated via `SafeActive` risk + required_features:["mobile"] in EnforcementContext / `commands/handlers/mobile.rs`.
- Explicit lab/defense framing note (unless quiet). No TUI tab, no pipeline `ScanProfile` integration in Phase 1 (aspirational `mobile-static` / `mobile-regression` profiles noted in architecture).
- Phase 1 closed 2026-06-11 with full tests, clippy, docs consistency, AGENTS.override.md, architecture/mobile.md, docs/MOBILE.md updates.

Key quotes:
> "**This is static analysis only.** The tool performs no dynamic execution, no Frida or runtime instrumentation, no device interaction..."
> Future phases aspirational for "gated dynamic capabilities (Frida-based instrumentation behind additional safety + capability flags...)" (docs/MOBILE.md Future section).

The module follows the proven standalone defense-lab pattern established by `auth-test` and passive `wireless` (local findings + optional unified bridge, no MCP/agent exposure by design).

### 2.2 Why Expand to Dynamic Now?
Static analysis catches manifest/config issues reliably and quickly. However, many critical mobile vulnerabilities are **runtime/dynamic only**:
- Actual network behavior (cleartext despite manifest claims, weak pinning bypassable at runtime, certificate transparency issues).
- Runtime permission grants/denials and user prompt behavior.
- Exported component reachability and intent-based attacks in live app context.
- Insecure storage usage patterns visible only at runtime (e.g. SharedPreferences vs Keystore actual usage).
- Crash / exception paths revealing sensitive data in logs.
- Dynamic code loading, reflection, or WebView JS bridge exploitation surface.
- Traffic interception and inspection correlated back to app components.

Adding a complementary **dynamic loadout** enables full defense-in-depth mobile validation loops: static baseline → instrumented/observed run → measure real behavior vs declared manifest → regression over app updates. This mirrors mature mobile appsec practices (static + dynamic = SAST/DAST for mobile) while staying inside Eggsec's safety philosophy. The wireless active expansion (multiple plans June 2026) provides the exact implementation pattern and cultural precedent.

### 2.3 Related Existing Capabilities
- `proxy` / stress-testing features: proxy pool management, useful for mobile MITM setup.
- `headless-browser`: precedent for controlled dynamic execution (though web-focused).
- `auth-test`: standalone high-risk defense-lab command with strict policy gating.
- `wireless-advanced`: gated active primitives with deauth frame crafting (pure Rust + pnet), CLI subcommands, TUI, dry-run, budgets, audited overrides.
- EnforcementContext central gate used everywhere.

These provide proven patterns for feature gating, warnings, local findings, optional bridges, strict enforcement in automated paths, and lab-only framing.

---

## 3. Goals, Non-Goals, and Scope

### 3.1 Primary Goals
- Deliver a curated set of **dynamic primitives** usable for:
  - Installing/launching/uninstalling user-supplied test APKs on lab Android devices/emulators via ADB (controlled, logged, reversible).
  - Setting up and correlating traffic through Eggsec proxy / mitmproxy for dynamic network observation (cleartext detection, API endpoint discovery at runtime, header analysis).
  - Capturing and analyzing runtime logs (logcat) for security-relevant events (permission prompts/denials, crashes with stack traces revealing secrets, exported intent handling).
  - Basic runtime validation of manifest-declared behaviors (e.g. does `android:usesCleartextTraffic` actually result in HTTP? Does backup work as declared?).
  - Permission runtime grant/revoke testing and observation (defense validation of app's permission handling).
  - Future: lightweight Frida-based hooking for method tracing or basic instrumentation (gated, lab-only).
- Maintain **zero accidental misuse surface**: every dynamic path requires the new feature flag + explicit policy confirmation (or audited override) + device/app provenance confirmation.
- Produce structured, reportable findings (new `mobile-dynamic-*` categories) that integrate with existing `ScanReportData`, SARIF, JUnit, HTML/Markdown, trend/diff pipelines, and static+dynamic combined reports.
- Provide excellent dry-run / planning support and human-readable + JSON output consistent with static `mobile`.
- Extend (not duplicate) the existing `Mobile*` types and bridge where sensible; keep static and dynamic separable or combinable.
- Support Android primarily in Phase 1/2 (mature ADB ecosystem); iOS dynamic as Phase 3+ or note as significantly more constrained (no easy pure-Rust equivalent to ADB without jailbreak/developer certs).

### 3.2 Non-Goals (Explicitly Out of Scope for This Plan)
- General-purpose mobile pentest framework or "all-in-one" tool (no goal to match MobSF, Frida scripts ecosystem, or objection feature parity).
- Production or customer-facing app dynamic testing without explicit authorization and isolation.
- Automatic UI automation / full Appium-style scripted flows in initial phases (too broad; aspirational later or via integration notes).
- Root exploit or privilege escalation primitives.
- iOS dynamic in Phase 1 (IPA static only; dynamic iOS requires different toolchain, often jailbroken devices or Xcode; document as future).
- Full decompilation + dynamic instrumentation from day one (keep high-signal, bounded scope).
- Changes to MCP/agent tool registry exposure for mobile (remain absent / standalone defense-lab by default; future opt-in only after security review).
- Windows/macOS host dynamic support in Phase 1 (Linux + Android emulator focus; cross-platform where ADB works).
- Unfettered app data access or exfiltration simulation without strict lab manifests and user confirmation.

### 3.3 In-Scope Dynamic Primitives (Phased)

| Phase | Primitive                          | Description                                                                 | Risk Tier      | Example CLI                                      | Safety / Gating Notes                                      | Priority |
|-------|------------------------------------|-----------------------------------------------------------------------------|----------------|--------------------------------------------------|------------------------------------------------------------|----------|
| 1     | ADB Device Connect & Validate     | Detect connected devices/emulators, validate lab manifest, basic info       | Medium         | `eggsec mobile dynamic --list-devices`          | Device allowlist/manifest, root/CAP? no, but explicit confirm | P0      |
| 1     | Controlled APK Install / Launch / Uninstall | `adb install`, launch main activity, uninstall after; with logging        | High           | `eggsec mobile dynamic app.apk --device emulator-5554 --install --launch --uninstall-after` | App package allowlist, --dry-run, confirm, budget (time/actions), provenance note | P0      |
| 1     | Runtime Logcat Analysis           | Capture logcat during run; parse for permission events, crashes, network hints, secret leaks in logs | Medium-High    | `... --capture-logs --duration 60`              | Sensitive log redaction hints, local findings only         | P0      |
| 2     | Proxy / MITM Integration          | Guide or automate setup of mitmproxy / Eggsec proxy for the device; correlate observed traffic back to app components | High           | `eggsec mobile dynamic app.apk --setup-mitm --proxy-port 8080` | Traffic budgets, header redaction in findings, lab network only | P1      |
| 2     | Runtime Permission & Behavior Validation | Grant/revoke perms via adb shell pm, observe prompt behavior or denials   | High           | `... --test-permission android.permission.CAMERA` | Explicit allowlist of testable perms, extra confirmation   | P1      |
| 3     | Basic Frida / Hooking Support     | Gated wrapper for Frida server push + simple script execution (e.g. trace methods, dump some state) | Very High      | `... --frida-script trace.js --allow-frida`     | Extra sub-feature `mobile-frida`, device must be rooted/emulator, heavy policy + hardware gate | P2      |
| 3+    | Traffic-driven Fuzz or Deeper Instrumentation | Correlate proxy traffic with static findings for targeted dynamic tests   | Very High      | Future                                         | Additional budgets, lab isolation, future phases           | Future  |

**Phase 1 Focus**: Android ADB core (connect, install/launch/uninstall with safety, logcat parsing for high-signal runtime findings). This delivers immediate value (runtime confirmation of static issues) with contained implementation risk.

---

## 4. Safety, Policy & Enforcement Model Extensions

### 4.1 Feature Flag
```toml
# crates/eggsec/Cargo.toml
mobile-dynamic = ["mobile"]  # depends on base mobile static
# Later: mobile-frida = ["mobile-dynamic"] or similar for extra gated surface
```
CLI/TUI features propagate it. Full build includes optionally.

### 4.2 Risk / Operation Classification
- Extend `OperationRisk` enum or add capability flag `MobileDynamic` / `DynamicMobileTesting`.
- CLI handler uses `SafeActive` today for static; dynamic paths require new tier or `HighRisk` + explicit `allowed_capabilities` + device manifest check.
- `EnforcementContext::evaluate()` must treat dynamic operations as non-downgradable in strict profiles (MCP/agent/CI).
- New denial/confirmation classes for "dynamic mobile requires explicit lab device authorization + test APK provenance".
- Additional policy decision record fields for device serial, app package, action type.

### 4.3 Runtime Gating & UX (Even Stricter than Static or Wireless Active)
- Prominent startup / pre-execution warning (more visible):
  > "DYNAMIC MOBILE TESTING MODE — Requires lab Android device/emulator + explicit authorization. Installs/launches user-supplied test APKs only. For defense validation ONLY. All actions logged and reversible where possible."
- `--dry-run` always supported and produces valid structured JSON (no device actions).
- `--allow-dynamic-mobile` (narrow override, audited, like other high-risk flags) + optional `--manual-override-reason` + device/app provenance confirmation prompt.
- Device & App Lab Manifest (TOML or JSON, analogous to wireless `--known-good` + scope): list of allowed device serials, allowed package names, max install count, etc. Enforced before any ADB action.
- Action / time budgets (e.g. `--max-actions 50`, `--max-duration 300s` default conservative).
- In TUI (future): Pre-flight policy indicator (device connected? manifest match? risk level), confirmation overlay, live action counter, emergency stop / uninstall-all.
- All dynamic runs produce auditable policy decision + findings even in dry-run.
- Test APK provenance: user must confirm "I built this test build myself in an isolated CI job" or similar; tool never fetches APKs from network.

### 4.4 Legal / Ethical / Documentation Requirements
Every new command, output, and finding must surface:
- "Use ONLY on applications and devices you own or have explicit written authorization to test. Supply your own test builds with controlled provenance. Securely destroy test artifacts after lab use."
- Reference to platform policies (Google Play, Apple App Store review implications for debug builds).
- Strong recommendation to use dedicated lab devices/emulators, never production hardware with real user data.
- Link to `docs/SAFETY.md` (new Dynamic Mobile section) and updated `MOBILE.md`.

### 4.5 MCP / Agent Exposure
**Recommendation**: Keep mobile (static + dynamic) as a **standalone defense-lab surface**. Do not register dynamic commands as `SecurityTool` in the tool registry initially. This preserves the intentional design decision from Phase 1 and wireless. Reporting bridge remains available for any consumer that obtains native JSON output. Future opt-in after security audit only.

---

## 5. Technical Architecture

### 5.1 Module Structure (Proposed)
```
crates/eggsec/src/mobile/
├── mod.rs                 # Existing static (keep mostly unchanged; re-export dynamic types)
├── apk.rs                 # Existing
├── ipa.rs                 # Existing
├── dynamic.rs             # NEW: High-level dynamic entry, report types, run_dynamic_cli dispatcher
├── adb.rs                 # NEW: Pure-Rust or minimal-wrapper ADB client (connect, install, shell, logcat, uninstall)
├── runtime.rs             # NEW: Log parser, permission tester, traffic correlator, finding generators
├── manifest.rs            # Shared or extend static manifest parsing for runtime comparison
└── types.rs               # Extend or add DynamicMobileReport, DynamicMobileFinding, DeviceInfo, etc.
```

`dynamic.rs` provides `run_dynamic_cli(...)`, `analyze_runtime(...)`, etc., called from updated or new handler.

Static and dynamic can share some parsing logic (e.g. permission lists, component extraction) for "static vs dynamic diff" findings.

### 5.2 Data Models (Additions / Extensions)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MobileTestMode {
    Static,           // existing
    Dynamic,          // new
    Combined,         // future: run static then dynamic in one report
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMobileFinding {
    pub category: String,      // e.g. "runtime-permission", "cleartext-observed", "crash-log", "exported-reachable"
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    pub evidence: Option<String>,  // rich: log snippet, observed URL, permission state before/after, device serial
    pub static_correlation: Option<String>, // link back to static finding ID if applicable
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMobileReport {
    pub target: String,           // APK path or package name
    pub scan_type: String,        // "mobile-dynamic"
    pub platform: MobilePlatform,
    pub device_serial: Option<String>,
    pub app_id: Option<String>,
    pub version: Option<String>,
    pub timestamp: String,
    pub findings: Vec<DynamicMobileFinding>,
    pub recommendations: Vec<String>,
    pub duration_ms: u64,
    pub actions_performed: Vec<String>,  // audit log of install/launch/logcat/etc.
    pub proxy_traffic_summary: Option<ProxySummary>, // if mitm used
}

// Bridge evolution (or new fn)
pub fn to_scan_report_data_dynamic(result: &DynamicMobileReport) -> crate::output::convert::ScanReportData { ... }
// Categories become mobile-dynamic-android-runtime-permission etc.
```

Extend or parallel the existing bridge so `eggsec mobile dynamic ... --json | eggsec report convert` works seamlessly, and combined static+dynamic reports are possible.

### 5.3 ADB Integration Layer (Pragmatic Pure-Rust Preference)
**Preferred Approach**:
- Implement minimal ADB protocol client in pure Rust (ADB over TCP is well-documented: CNXN, AUTH, OPEN, WRITE, etc. messages; many crates exist like `adb` or `rust-adb`, or roll minimal for install/shell/logcat only to avoid heavy deps).
- Or gated optional dependency on a mature crate under `mobile-dynamic`.
- Core primitives: `list_devices()`, `connect(serial)`, `install(apk_path)`, `launch(package, activity)`, `shell(cmd)`, `logcat(duration or until signal)`, `uninstall(package)`, `grant/revoke_perm`.
- All actions bounded, logged in report.actions_performed, reversible (uninstall after), with timeouts.
- For USB devices: note requirement for `adb` binary or implement USB transport later; start with TCP/emulator (5555 port) for simplicity and lab friendliness.
- Fallback: optional subprocess to system `adb` binary behind `mobile-dynamic + external-adb` sub-feature (documented, with PATH validation and version checks). Pure Rust path primary for reproducibility.

**Why not full Frida from day one?** Frida requires server push to device, JS engine, rooting/emulator often. Phase 3 gated sub-feature keeps surface small initially while delivering high value with ADB + log/proxy.

### 5.4 CLI Integration
Extend `MobileArgs` or introduce subcommand structure (clap derive, consistent with wireless `deauth` etc.):
```bash
eggsec mobile dynamic <apk-or-package> [OPTIONS]
# or
eggsec mobile-dynamic <apk> --device SERIAL [OPTIONS]
```
Key flags: `--device <serial>`, `--install`, `--launch`, `--uninstall-after`, `--capture-logs --duration <secs>`, `--setup-mitm --proxy <host:port>`, `--test-permission <perm>`, `--dry-run`, `--json`, `--output`, `--allow-dynamic-mobile`, `--lab-manifest path.toml`, `--max-actions N`, etc.

Handler dispatch after policy check (new or extended `handle_mobile_dynamic`).

Dry-run path: simulate all steps, produce full JSON report structure with 0 actions performed.

### 5.5 TUI Integration (Future Phase)
- Add Dynamic tab or expand Mobile tab (if added) with device status, action queue, live log stream, policy preflight.
- Reuse `UiAction`, `OverlayController`, global task strip, semantic risk styling (high for dynamic actions).
- Live metrics: actions completed, findings so far, time remaining vs budget.

### 5.6 Dependencies & Build
- Core for Phase 1: minimal new deps or reuse existing (tokio for async ADB if needed, serde already there). Optional `adb` crate or pure impl under feature.
- System: Android SDK / emulator or physical device with USB debugging enabled; `adb` in PATH for fallback.
- Document emulator setup (Android Studio AVD) + mitmproxy integration in `docs/MOBILE.md`.
- For proxy correlation: leverage existing Eggsec proxy types or lightweight integration notes.

### 5.7 Concurrency & Safety
- Dynamic runs should support background log capture while other Eggsec scans run (or cooperative pause).
- Hard limits on concurrent dynamic operations per device.
- Graceful cleanup: always attempt uninstall / permission restore on error or Ctrl-C.
- Sensitive data redaction in logs/findings (pattern-based like existing secret scanner).

---

## 6. Detailed CLI Command Designs (Phase 1 Focus)

### 6.1 Device Management & Validation
```bash
eggsec mobile dynamic --list-devices --json
# Dry-run validation of lab manifest + connected devices
```

### 6.2 Controlled Install / Run / Observe
```bash
# Full safe dynamic run (lab only)
sudo eggsec mobile dynamic /path/to/test-app-debug.apk \
  --device emulator-5554 \
  --install \
  --launch com.example.vulnerable/.MainActivity \
  --capture-logs --duration 120 \
  --setup-mitm --proxy 127.0.0.1:8080 \
  --uninstall-after \
  --allow-dynamic-mobile \
  --manual-override-reason "Authorized lab regression of runtime cleartext and permission behavior" \
  --lab-manifest lab-devices.toml \
  --json -o dynamic-report.json

# Dry-run (no actions, full report skeleton)
eggsec mobile dynamic app.apk --device emulator-5554 --dry-run --json
```

**Output (human)**: Device info, actions performed summary, runtime findings (e.g. "Observed HTTP cleartext to api.example.com despite manifest claim", "Permission CAMERA granted at runtime", "Crash in MainActivity revealing stack with potential secret in log"), recommendations, duration.

**JSON**: `DynamicMobileReport` (or wrapped).

### 6.3 Log Analysis & Finding Generation
Parser in `runtime.rs` scans logcat for:
- Permission-related (grant/deny, prompt shown).
- Network (cleartext URLs, SSL errors, pinning failures).
- Crashes / exceptions with interesting frames.
- Debug / verbose logs indicating insecure patterns.
- Correlates back to static findings where possible (e.g. "static declared debuggable=true; runtime confirmed via logcat").

High-signal only; bounded capture to avoid noise.

---

## 7. Reporting, Findings & Output

New finding categories (examples):
- `mobile-dynamic-android-runtime-permission`
- `mobile-dynamic-android-cleartext-observed`
- `mobile-dynamic-android-crash-log`
- `mobile-dynamic-android-exported-reachable`
- `mobile-dynamic-android-log-secret-leak`

Bridge populates `ScanReportData.findings` with rich evidence (log snippets redacted, observed endpoints, before/after state) and `remediation`.
Native JSON from dynamic commands accepted by `eggsec report convert` (auto-bridge when feature present).

Combined workflow example:
```bash
eggsec mobile app.apk --json -o static.json
eggsec mobile dynamic app.apk --device ... --json -o dynamic.json
eggsec report convert static.json dynamic.json -f html -o combined-mobile-report.html   # or trend/diff
```

Supports regression: capture baseline dynamic report, re-run after app update, diff findings.

---

## 8. Phased Implementation Roadmap

**Phase 0 (This Plan — Complete)**: Handoff document created and pushed to repo.

**Phase 1 (Android ADB Core + Log Analysis — P0, ~3-4 weeks)**:
- Feature flag + Cargo plumbing + dependency decision (pure ADB impl vs optional crate vs subprocess fallback).
- `dynamic.rs` + `adb.rs` + basic `runtime.rs` (device list/validate, install/launch/uninstall, logcat capture + parser for high-signal events).
- CLI args + handler dispatch + policy gate + lab-manifest enforcement.
- Dry-run path + full JSON schema + human output.
- Unit tests (ADB message construction, log parser on synthetic logs, policy stubs, bridge roundtrips).
- Minimal documentation examples in MOBILE.md.
- Lab emulator smoke test (Android Studio AVD + test APK with known issues).
- Update `MOBILE.md` (new "Dynamic Testing" section), `architecture/mobile.md`, `SAFETY.md`, `CAPABILITIES.md`, README lab defense commands table and quick-ref.

**Phase 2 (Proxy / MITM + Permission Testing — P1)**:
- Proxy setup automation or high-quality guided integration with existing proxy pool / mitmproxy.
- Runtime permission grant/revoke + observation.
- Richer correlation between static manifest and dynamic behavior (diff findings).
- Full TUI action stubs or expanded Mobile tab if added.
- `--known-good` / lab-manifest integration for devices + packages.
- Repeat / temporal support for dynamic runs.
- Documentation of complete lab workflow ("static baseline → dynamic confirmation → WAF/backend correlation → regression").

**Phase 3 (Frida Gated + Polish — P2)**:
- Optional `mobile-frida` sub-feature + gated Frida server push + simple script runner (trace, dump).
- Advanced policy (rooted device check, extra confirmation for hooking).
- Performance / cleanup hardening, better redaction.
- Possible limited TUI live view.

**Phase 4+ (Future)**: Deeper instrumentation, traffic-driven targeted tests, iOS dynamic notes or limited support, pipeline profile integration (`mobile-dynamic` / `mobile-regression` profiles), MCP opt-in after audit.

**Cross-Cutting Work** (parallel):
- EnforcementContext / OperationRisk / lab-manifest extensions.
- Output convert bridge tests (static + dynamic + combined).
- Hardware / emulator compatibility matrix doc.
- AGENTS.md / AGENTS.override.md updates for new dynamic surface.

**Testing Strategy**:
- Unit: ADB protocol (mock), log parsers on fixture logs, finding generators, dry-run paths, serde roundtrips (no hardware).
- Integration: Policy enforcement tests (mock EnforcementContext + manifest).
- Lab hardware/emulator: Dedicated test matrix (emulator vs physical, Android versions, known vulnerable test apps, observed vs declared behavior).
- Use `--dry-run` heavily in CI; real device tests in lab jobs only.
- Regression: Add mobile-dynamic examples to defense-lab profiles / CI once stable.

---

## 9. Risks, Edge Cases & Mitigations

| Risk / Edge Case                                      | Impact                                      | Mitigation                                                                 | Owner      |
|-------------------------------------------------------|---------------------------------------------|----------------------------------------------------------------------------|------------|
| User runs dynamic commands on production device or real user data APK | Legal / data breach / reputational         | Multi-layer warnings + policy gate + lab-manifest (serial + package allowlist) + provenance prompt + prominent disclaimers | Policy + Docs |
| ADB implementation bugs or incomplete cleanup (app left installed) | Device pollution or instability            | Strict uninstall-after, try-finally cleanup, dry-run validation, bounded actions, graceful error paths | Impl       |
| Logcat captures sensitive production-like data in test runs | Data exposure in reports                   | Aggressive redaction patterns (secrets, PII hints), local-only findings, user responsibility for test data | Impl + Docs |
| Emulator vs physical differences (permission model, SELinux, etc.) | Inconsistent findings                      | Document matrix, version-specific parsers, recommend emulator for most regression | Docs       |
| Frida phase introduces powerful hooking surface      | Misuse or policy bypass risk               | Phase 3 only, extra sub-feature + rooted-device gate + heavy confirmation + no MCP exposure | Policy     |
| Complexity of cross-platform (Android good, iOS hard) | Feature imbalance or maintenance burden    | Android-first explicit in docs; iOS dynamic noted as future/constrained; keep static IPA strong | Arch       |
| Proxy/MITM setup complexity for users                | Low adoption or misconfiguration           | Excellent guided docs + example docker-compose / scripts + one-command setup helper where safe | Docs + Impl |
| Concurrent static/dynamic or multiple dynamic runs   | Confusing state or resource contention     | Explicit device locking or cooperative pause; document best practices     | TUI/CLI    |
| Regulatory / export control implications of dynamic tooling | Compliance risk                            | Strong disclaimers + "know your local laws and platform policies" in every help text | Docs       |

**Monitoring for Abuse**: All dynamic operations produce auditable policy decisions, device/app context, and findings even in dry-run/JSON paths. Lab manifests provide additional control layer.

---

## 10. Open Questions & Decisions Needed (for Team)

1. Exact feature flag name: `mobile-dynamic` (preferred, descriptive) vs `mobile-advanced` vs `mobile-runtime`? Sub-feature for Frida later?
2. ADB strategy for Phase 1: Pure-Rust minimal client (recommended for consistency) vs optional mature crate vs subprocess `adb` wrapper from day one for broadest compatibility?
3. Should lab manifest be mandatory for dynamic (enforced) or advisory + runtime confirmation? Format (TOML like scope files?)?
4. Confirm MCP/agent exposure remains **absent** for the entire mobile surface (static + dynamic) in this round?
5. Desired default budgets (max actions, duration) and redaction strictness for log findings?
6. How deep should Phase 1 log parser go (high-signal only vs broader)? Include basic traffic summary even without full proxy integration?
7. Preference for combined static+dynamic single command in Phase 2 (`eggsec mobile analyze --dynamic`) or keep separate for clarity?
8. Any early preference on TUI tab timing (after wireless tab stabilization?)?

---

## 11. Handoff Checklist

- [ ] Review & approve this plan (team + security review).
- [ ] Merge or cherry-pick plan file to main (or dedicated feature branch).
- [ ] Create follow-up issues for Phase 1 tasks (feature flag, adb module, CLI/handler/policy, log parser, docs updates).
- [ ] Assign owners for cross-cutting (EnforcementContext extensions, lab-manifest design, bridge evolution, TUI patterns).
- [ ] Update `AGENTS.md` / mobile `AGENTS.override.md` with dynamic context if needed.
- [ ] After Phase 1 implementation: Run full test suite (`cargo test --features mobile-dynamic`), emulator smoke tests, generate sample static+dynamic reports and diffs.
- [ ] Post-implementation: Update `docs/MOBILE.md` "Dynamic Testing" section with real examples and complete lab workflow; refresh `architecture/mobile.md`.
- [ ] Consider adding short ADR in `docs/adr/` for the dynamic loadout safety model and standalone decision.

**Immediate Next Action After Handoff**: Team decides on feature flag name and ADB implementation strategy (pure Rust vs hybrid), then starts Phase 1 on a new feature branch.

---

## 12. References & Further Reading

- Current static implementation: `crates/eggsec/src/mobile/{mod,apk,ipa}.rs`, `cli/mobile.rs`, `commands/handlers/mobile.rs`
- Types & bridge: `mobile/mod.rs` (MobileScanReport, to_scan_report_data)
- Policy core: EnforcementContext, OperationRisk, SafeActive in relevant config/ and commands/ files
- Wireless precedent (exact pattern): `plans/wireless-active-attacks-loadout-design-plan.md` and follow-up wireless-*-plan.md files; `crates/eggsec/src/wireless/`, docs/WIRELESS.md
- Related standalone: `auth-test` command and handler
- Proxy / load testing: stress-testing feature and proxy management
- Full docs: `docs/MOBILE.md`, `architecture/mobile.md`, `docs/SAFETY.md`, `docs/CAPABILITIES.md`, README.md (mobile section and lab defense commands)
- AGENTS context: `crates/eggsec/src/mobile/AGENTS.override.md` and root AGENTS.md
- Future inspiration: plans/ for other loadouts; architecture/defense_lab.md

---

**End of Plan Document**

*This document is intended as a complete, self-contained handoff artifact. It captures context, rationale, detailed design, risks, edge cases, and actionable phased roadmap so the team can implement the dynamic expansion without ambiguity while preserving Eggsec's core safety, quality, and defense-lab standards.*
