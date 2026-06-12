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

## Limitations (Phase 1 static + Phase 1 dynamic + Phase 2 closed 2026-06-12)

**Static**:
- Manifest/config surface only. No runtime behavior, no Frida, no dynamic hooking, no emulator/device interaction.
- No deep DEX decompilation, full bytecode analysis, or control-flow graphs.
- No complete third-party dependency / supply-chain graph (only basic indicators and secret patterns).
- Limited to manifest, plist, network config, signing markers, and bounded small-text asset scans. Large resources and native libraries are size-capped or skipped.
- No automatic app installation, permission granting, or traffic capture.
- iOS analysis is IPA-bundle only (no .app bundles or xcarchives directly).
- No TUI tab (CLI primary).

**Dynamic (Phase 1 + Phase 2 closed 2026-06-12)**:
- Android-only (emulator TCP primary; USB/physical via external adb convenience).
- Phase 1: No Frida / hooking; log findings high-signal only (permission events, crashes, cleartext, secrets); basic redaction.
- Phase 2 (closed): Proxy Level-1 (device global `http_proxy` config + user-provided `--traffic-capture` summary parser; no full body capture, no automatic mitmproxy management). Runtime permissions via `pm grant/revoke/list` (grant/revoke/list only; results surfaced as findings + optional `permission_state`).
- No TUI; standalone CLI only.
- Lab manifest is advisory (TOML allowlist); real safety comes from policy + explicit `--allow-dynamic-mobile` + user-controlled test builds + best-effort cleanup.
- iOS dynamic deferred (Phase 3+ or note as heavily constrained).
- Proxy config failures are non-fatal (lab use; warnings recorded in actions). Traffic capture file must be readable (mitmproxy log/HAR-like; parser is best-effort summary only). Permission ops require package installed + appropriate device state.

## Dynamic Testing Phases

Phase 1 (Android ADB core + runtime log analysis) complete 2026-06-12 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (parent design: `plans/dynamic-mobile-testing-loadout-design-plan.md`).

**Phase 2 (proxy + permissions + correlation + hygiene) closed 2026-06-12** per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (combined close-out + kickoff; executed). All under single `mobile-dynamic` feature (no sub-feature split per M1 decision). Level 1 proxy foundation + runtime permissions + `correlate_findings` + `static_correlation` + parser/report hygiene delivered. Adds `traffic_summary` + `permission_state` (optional) to `DynamicMobileReport`; bridge emits extra info findings under `mobile-dynamic-android-*` categories (still standalone defense-lab, MCP/agent absent, same pattern as wireless-active + static-mobile + auth-test). Phase 3a/3b/3c + Phase 4a (Core Correlation Engine) delivered 2026-06-12 under single mobile-dynamic (see Future). See CLI examples and "Future" below; heavy lab caveats apply.

**Phase 3a (Frida foundation + first capability) delivered 2026-06-12** per `plans/mobile-dynamic-phase3-frida-expansion-plan.md` (executed; Key Decision: all under single `mobile-dynamic`, no mobile-frida sub-feature). Runtime gated by explicit `--allow-frida` (Intrusive policy tier for real ops; dry-run always safe, hardware-free, produces valid reports). Adds `--frida-script <FILE>` (user JS) and builtin support (basic_method_trace for common sensitive methods e.g. javax.crypto.Cipher.doFinal, keystore, login/token, root/Frida detection hooks). Emits structured JSON markers from safe Frida JS; parses to "frida-method-trace", "frida-secret-extract", "frida-bypass-validation" findings + `frida_instrumentation` on report (sessions/scripts/builtins). CLI fallback to `frida` binary (best-effort; real requires frida CLI in PATH + frida-server on rooted/emulator). Handler policy + allow gate (audited); actions in audit trail; bridge emits `mobile-dynamic-android-frida-*` categories + extra info for instrumentation summary. Standalone defense-lab only (no MCP/agent). Prefer dry-run for most work. See CLI examples below and the phase3 plan for details.

**Phase 3b (additional builtins + correlation + evidence quality + richer reports) delivered 2026-06-12** per `plans/mobile-dynamic-phase3-frida-expansion-plan.md` (executed). All under single `mobile-dynamic`. Adds 3 new high-signal builtins (crypto-keystore observation, bypass/detection validation, API call tracing with param inspection) dispatchable via `--frida-script "builtin:NAME"` (crypto-keystore | bypass-validation | api-trace | basic-method-trace alias for backward). FridaScriptResult now carries `structured_output: Option<Value>` (JSON preferred; parsed from script console.log lines containing "type"). New pure `redact_frida_evidence` (secrets like api_key/sk_live_/token=/password=, byte-array len redaction) applied to Frida evidence/output in dry/real paths. Richer `FridaInstrumentation` on `DynamicMobileReport`: +start_time, structured_results (populated from structured_output), correlation_notes. `run_dynamic_cli` dry/real paths exercise/populate richer fields + new finding categories (frida-crypto-observation, frida-api-trace etc.); real path uses `frida::run_builtin` for builtin: convention. `correlate_findings` extended with Frida rules (frida-crypto + static secret/hardcoded → note; frida-api/traffic-cleartext + proxy/traffic_summary + static network → "Frida-observed call correlates with proxy traffic"; frida-bypass + debug/runtime-perm → note; frida-secret-extract + static secret). Side-effect populates `static_correlation` on matching DynamicMobileFinding. `format_dynamic_report`/`to_scan_report_data_dynamic`/`build_dynamic_recommendations` extended (richer frida section, new bridge cats mobile-dynamic-android-frida-*, structured/corr summary in extra info, rec mention when Frida+other data). Expanded tests (~6-8 new covering gens, run_builtin, structured, redact, correlate new rules, richer carrier/bridge, recs, robustness). Smoke updated with Phase 3b leg (builtin:crypto-keystore + --traffic-capture + --list-permissions; asserts richer fi + structured_results + corr_notes + new cats + redaction + bridge). CLI ABOUTs updated with convention + dry example; no new clap arg (use --frida-script "builtin:xxx"). Handler policy unchanged (frida_script presence → Intrusive for !dry). Standalone defense-lab; dry always safe/valid reports; real best-effort + audited. See plan for 3c vision.

- Gated behind `mobile-dynamic` feature (implies `mobile`).
- CLI: `eggsec mobile dynamic <target.apk> --device <serial|host:port> [options]`.
  - `--dry-run` (always safe, produces complete valid JSON/pretty report with simulated actions + sample findings).
  - Real actions: `--install`, `--launch <activity>`, `--capture-logs --duration <secs>`, `--uninstall-after`.
  - Explicit `--allow-dynamic-mobile` required for any non-dry-run (audited on policy decision).
  - Optional `--lab-manifest path.toml` (allowed_device_serials + allowed_packages; advisory in P1).
- All actions recorded in `actions_performed` audit trail; best-effort uninstall always attempted (even on error).
- High-signal runtime findings from logcat: `runtime-permission` (grant/deny), `crash-log` (with stack hints), `cleartext-observed`, `log-secret-leak` (basic redaction). Categories become `mobile-dynamic-android-*` in bridged reports.
- Pure-Rust minimal ADB-over-TCP (emulator primary: e.g. emulator-5554 or 127.0.0.1:5555). `adb devices` convenience for discovery if binary in PATH; otherwise probes common emulator ports. No new heavy deps.
- `DynamicMobileReport` / `DynamicMobileFinding` + `to_scan_report_data_dynamic()` bridge (mirrors static + active wireless). Native `--json` auto-bridged by `eggsec report convert` when feature enabled.
- Policy: `EnforcementContext` with `OperationMode::DefenseLab`, `OperationRisk::SafeActive`, `required_features: ["mobile-dynamic"]`. Standalone defense-lab surface (no MCP/agent registration; same pattern as wireless active and static mobile). Prominent lab warnings.
- No Frida in Phase 1/2 (Phase 3a delivered under single mobile-dynamic; see below). No proxy/MITM automation (Level-1 device config only), no TUI, no iOS dynamic, no pipeline `ScanProfile` in this phase (aspirational later).
- Emulator smoke test path documented; unit tests cover ADB framing/mocks, log parser on fixtures, serde, dry-run, bridge roundtrips.

See "Phase 1 Lab Setup" below, `docs/MOBILE.md` examples, and the handoff plan for full details + safety model.

**Phase 1 Lab Setup (quick)**:
1. Android Studio AVD (API 34+ recommended) or physical dev device with USB debugging + `adb`.
2. Build a debug/test APK you control (signing disabled only in isolated CI job; provenance note required).
3. `eggsec mobile static vuln.apk` first (baseline).
4. Dry-run validation (always safe): `./scripts/test-mobile-dynamic.sh` (or with your APK).
5. Full documented command: `eggsec mobile dynamic vuln.apk --device emulator-5554 --dry-run --json`.
       6. Real: `... --install --launch '.MainActivity' --capture-logs --duration 60 --uninstall-after --allow-dynamic-mobile`.
       7. `eggsec report convert dynamic.json -f html -o dynamic.html` (or trend/diff with static baseline).

**Phase 2 Lab Workflow (quick; closed 2026-06-12)**:
- Static baseline first (`eggsec mobile static ...` or the documented Phase 1 static gate).
- Dynamic run with `--proxy`/`--traffic-capture` + permission ops (see Phase 2 CLI examples below); always start with `--dry-run`.
- `eggsec report convert <json> ...` (auto-bridges `traffic_summary`/`permission_state` as extra info findings under `mobile-dynamic-android-*`).
- Regression via `report diff`/`trend` (or your tooling) against static baseline + prior dynamic runs.

**Phase 2 CLI examples** (closed 2026-06-12; heavy lab-only caveats; dry-run always safe; real requires `--allow-dynamic-mobile` + device ownership + explicit consent):
```bash
# Dry-run (safe, no device touch)
eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --json

# Proxy config (Level 1; user runs mitmproxy separately; CA on device required for HTTPS inspection)
eggsec mobile dynamic test.apk --device emulator-5554 --proxy 127.0.0.1:8080 --traffic-capture /tmp/mitm.log --install --launch '.MainActivity' --capture-logs --duration 30 --uninstall-after --allow-dynamic-mobile

# Reset proxy post-run (or on error paths)
eggsec mobile dynamic test.apk --device emulator-5554 --reset-proxy --allow-dynamic-mobile

# Runtime permissions (before/after; list current state)
eggsec mobile dynamic test.apk --device emulator-5554 --list-permissions --grant-permission android.permission.CAMERA --revoke-permission android.permission.READ_CONTACTS --allow-dynamic-mobile

# Traffic summary + permission state appear in --json report (and bridged findings via report convert)
eggsec mobile dynamic test.apk --device emulator-5554 --proxy 10.0.2.2:8080 --traffic-capture capture.log --list-permissions --allow-dynamic-mobile --json -o dyn.json
eggsec report convert dyn.json -f html -o dyn.html
```
See "Policy Note" (Intrusive for real Frida) and `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (Phase 2 closed) + `plans/mobile-dynamic-phase3-frida-expansion-plan.md` (Phase 3a executed).

The `scripts/test-mobile-dynamic.sh` script (added during Phase 1 polish) automates the dry-run happy path and provides an optional `--real` leg for local AVD runs. It is self-documenting and intended for both developer workstations and CI (dry-run leg is hardware-free). See the script header and `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (P1.2).

Example manifest (examples/lab-mobile.toml):
```toml
allowed_device_serials = ["emulator-5554", "ABCD1234"]
allowed_packages = ["com.example.vuln.test"]
```

## Phase 1 Success Criteria (achieved; Phase 1 polish complete 2026-06-12)
- `cargo build --features mobile-dynamic` / check / test / clippy clean.
- `eggsec mobile dynamic --help` shows flags; legacy `eggsec mobile <apk>` and `mobile static` continue to work.
- Dry-run produces schema-valid full `DynamicMobileReport` (actions + findings + bridge).
- Real emulator happy-path (install/launch/log/uninstall) with policy confirmation + audit trail.
- No regressions in static `mobile` functionality or existing tests.

## Phase 2 Success Criteria (achieved; Phase 2 closed 2026-06-12)
- `cargo build --features mobile-dynamic` / check / test / clippy clean.
- `eggsec mobile dynamic --help` shows Phase 2 flags (`--proxy`, `--reset-proxy`, `--traffic-capture`, `--grant-permission`, `--revoke-permission`, `--list-permissions`); legacy paths continue to work.
- Dry-run produces schema-valid `DynamicMobileReport` with `traffic_summary`/`permission_state` extensions + bridge (extra info findings under `mobile-dynamic-android-*` categories) + `static_correlation`.
- Real paths (proxy/permission actions) covered in units + smoke; policy + audit trail present.
- No regressions in static `mobile`, Phase 1 dynamic, or existing tests.

## Phase 3a Success Criteria (achieved; Phase 3a foundation delivered 2026-06-12 under single mobile-dynamic)
- `cargo build --features mobile-dynamic` / check / test / clippy clean (no mobile-frida feature).
- `eggsec mobile dynamic --help` documents `--frida-script`, `--allow-frida`; dry-run Frida legs safe.
- Dry-run with `--frida-script` produces valid `DynamicMobileReport` with `frida_instrumentation` (note/sessions/scripts/builtins), frida-* findings (method-trace, bypass-validation, etc.), actions, and bridge categories (`mobile-dynamic-android-frida-*` + extra info for instrumentation).
- Unit tests cover dry connect/execute/trace, script generation (JSON markers for target methods), result parsing, error paths when frida CLI absent, carrier population, bridge roundtrips.
- Smoke `./scripts/test-mobile-dynamic.sh` (dry + Frida leg) passes (hardware-free; validates JSON has frida_instrumentation + findings + bridge).
- Real Frida path (when frida CLI + device present): best-effort, gated by --allow-frida + policy (Intrusive), audited; falls back cleanly.
- No regressions in prior mobile functionality; all Frida under mobile-dynamic (Key Decision followed).

## Phase 3b Success Criteria (achieved; Phase 3b delivered 2026-06-12 under single mobile-dynamic)
- `cargo build --features mobile-dynamic` / check / test / clippy clean (no mobile-frida).
- `eggsec mobile dynamic --help` documents builtin: convention for crypto-keystore | bypass-validation | api-trace (via --frida-script "builtin:NAME"); dry-run safe.
- Dry-run with Phase 3b builtins + combined flags produces richer `frida_instrumentation` (start_time, structured_results, correlation_notes, multiple enabled_builtins), new frida-* finding categories, redacted evidence, and bridge categories (mobile-dynamic-android-frida-crypto-observation etc.).
- `correlate_findings` handles Frida findings (crypto+static-secret, api+traffic/network, bypass+debug-perm, secret-extract+static) and populates static_correlation. Phase 4a: scoring inside correlate_findings + `correlate_reports`/`CorrelationEngine` for full reports (timeline/summary/scores).
- Unit tests cover generate_* scripts, run_builtin (each+unknown), structured JSON parse in execute, redact_frida_evidence, richer carrier/bridge, new correlate rules, recommendations, robustness (~8 new + prior).
- Smoke `./scripts/test-mobile-dynamic.sh` (Phase 3b leg) passes (exercises builtin:crypto-keystore + traffic+perms; asserts richer fi + structured + corr_notes + new cats + redaction + bridge).
- Real path: run_builtin for supported builtins, best-effort, redaction on evidence, gated by --allow-frida + policy; errors clean for unknown builtin.
- No regressions; all under mobile-dynamic (Key Decision followed); dry always valid + produces reports.

## Phase 4a Success Criteria (achieved; Phase 4a (Core Correlation Engine + Evidence Foundation) delivered 2026-06-12 under single mobile-dynamic)

- `cargo build --features mobile-dynamic` / check / test / clippy clean (no mobile-frida or new sub-feature).
- 6 new unit tests for `CorrelationEngine`, `correlate_reports`, enriched `CorrelatedFinding` (score/correlation_type/enrichment), `CorrelationResult` (correlations + timeline + summary), `CorrelationType` variants, conservative scoring (0-100) inside `correlate_findings` + engine path, min_score filter, timeline construction (timestamps + actions + Frida start_time + regression notes from 3c).
- All ~85+ mobile-dynamic tests green; no regressions in prior `correlate_findings`/`static_correlation` paths or 3c baseline/regression/bundle flows.
- Dry-run safe, hardware-free, hermetic (pure report-driven functions); no new Cargo dependencies; serde roundtrips preserved for `CorrelatedFinding` (new fields optional with defaults) and new types (`CorrelationResult`, `CorrelationEngine`, `CorrelationType`, etc.).
- `correlate_reports(&static_report, &dynamic_report)` and `CorrelationEngine::new().with_min_score(40).correlate(...)` return `CorrelationResult` containing enriched correlations, chronological timeline, and summary (total_correlations + avg_confidence).
- Low-level `correlate_findings` now populates optional `score`, `correlation_type` (Direct/Indirect/Behavioral/CrossLayer), and `enrichment` on `CorrelatedFinding` (non-breaking extension; pre-4a callers unaffected).
- Builds cleanly on Phase 3c baseline/regression/bundles (`--baseline`, regression_notes, evidence bundles remain unchanged and can be post-processed with the engine).
- Standalone defense-lab (MCP/agent/TUI/pipeline absent); policy/allow gates and EnforcementContext usage unchanged from prior phases.
- Phase 4b TUI deferred; reporting polish delivered (human output only) per standalone defense-lab policy.
- See `plans/mobile-dynamic-phase4-actionable-intelligence-plan.md` (annotated "Phase 4a implemented" + handoff checklist finalized) + `crates/eggsec/src/mobile/dynamic.rs` (CorrelationEngine ~281, correlate_reports ~343, new types ~247-289, updated correlate_findings ~1276).

## Phase 4b Success Criteria (reviewed + reporting polish delivered 2026-06-12 under single mobile-dynamic)

- `cargo clean -p eggsec && cargo check --features mobile-dynamic && cargo test --lib -p eggsec --features mobile-dynamic` clean (no new deps, no serde/JSON/bridge changes).
- 1 new unit test: `format_dynamic_report_surfaces_phase4b_regression_and_correlation_hints` (in dynamic.rs ~2179).
- Human (native text) output now surfaces regression/timeline hints: `format_dynamic_report` (~1088) includes `regression_notes={}` count in the frida line under "Runtime extensions" + new "Correlation / Regression:" section (~1122, when regression_notes or static_correlation present) that counts them and calls out `correlate_reports` / `CorrelationEngine` / `build_timeline`; `build_dynamic_recommendations` (~1063) appends "N regression note(s) from baseline comparison..." + one "  regression: ..." bullet per note when `frida_instrumentation.regression_notes` non-empty.
- Phase 4b TUI deferred per standalone defense-lab policy (confirmed absent from TUI crate; 30 Tab variants, no Mobile tab or wiring; wireless/auth are wired precedents).
- No regressions in prior mobile-dynamic functionality or ~85+ tests; dry-run safe/hardware-free; presentation-only polish (no public API changes).
- Standalone defense-lab (MCP/agent/TUI/pipeline absent).
- See plan (updated 2026-06-12) + `crates/eggsec/src/mobile/dynamic.rs` (~1063 build_dynamic_recommendations, ~1088 format_dynamic_report, ~1122 section, new test ~2179).

## Recommendations

- **Lab workflow**: Build your own debug/test variants with known provenance (e.g. from CI with signing disabled only in isolated jobs). Run `eggsec mobile` as an early static gate before any dynamic work (see `plans/dynamic-mobile-testing-loadout-design-plan.md`).
- **Recommended workflow (Phase 2 closed)**: static baseline first (`eggsec mobile static ...` or legacy) → dynamic with proxy/permissions/correlation (`eggsec mobile dynamic ... --proxy/--traffic-capture/--grant-permission/--list-permissions --allow-dynamic-mobile`) → use `correlate_reports` / `CorrelationEngine` for full static+dynamic+Frida correlation with scores/timeline/summary (or low-level `correlate_findings`) → `eggsec report convert` (auto-bridge) / `report diff` / `report trend` against static + prior dynamic baselines. Native human output now includes regression/correlation hints (Phase 4b polish, human-only). All under `--features mobile-dynamic`.
- Combine with:
  - SAST / dependency scanners (e.g. for full SDK enumeration)
  - Manual code review of high-risk flows
  - Authorized dynamic testing (Frida, objection, or platform debug bridges) inside a controlled lab with device isolation and no production data
  - Backend/API testing of the mobile app's server surface using the same scope and `eggsec` pipeline
  - Supply-chain / SBOM tools for third-party library tracking
- Always review findings against the app's actual data classification and threat model. Many "Medium" items are acceptable in internal tools but unacceptable for customer-facing or regulated apps.
- After lab use: securely destroy or archive test builds; do not leave debuggable or development-signed artifacts in shared locations.
- For regression: capture `--json` outputs and diff with `eggsec report diff` or your own tooling.

## Troubleshooting Dynamic Runs

Common issues and resolutions (Phase 1 polish):

- **"Dynamic mobile execution requires --allow-dynamic-mobile"**: Real (non-dry-run) paths are intentionally gated. Use `--dry-run` for planning/safe validation, or pass `--allow-dynamic-mobile` (and confirm any policy prompt) for live lab runs. The flag is audited on the policy decision (see `commands/handlers/mobile.rs` and wireless deauth precedent).
- **"Frida instrumentation requires --allow-frida"**: Real (non-dry) Frida ops are Intrusive tier. Use `--dry-run --frida-script ...` for safe simulation (always works, no frida CLI/device needed). Real requires `--allow-frida` + frida CLI in PATH + frida-server on rooted/emulator device. Audited; best-effort.
- **Emulator/device not found or "offline"**: Ensure the AVD is fully booted (API 34+ recommended). For TCP emulators use `emulator-5554` or `127.0.0.1:5555`. For USB/physical: enable USB debugging, `adb devices` should list it as "device". Cold-boot the AVD or restart adb server (`adb kill-server && adb start-server`). The pure-Rust path probes common emulator ports; external `adb` is used only for `list_devices` convenience when present.
- **"--device" missing or unclear error for real runs**: The dispatcher requires `--device <serial|host:port>` for any action that touches a device. Dry-run does not. See the error at `dynamic.rs:199` (or nearby) and the script `scripts/test-mobile-dynamic.sh`.
- **Lab manifest "ignored" or load warning**: `--lab-manifest` is advisory in Phase 1 (loaded and recorded in `actions_performed`; no hard enforcement). TOML must contain `allowed_device_serials` / `allowed_packages` arrays. Failures are logged as warnings and treated as advisory (see `dynamic.rs:146`).
- **Cleanup / uninstall failures**: Best-effort uninstall is always attempted (even on error paths). If the package name cannot be inferred or the device is disconnected mid-run, manual cleanup may be needed: `adb -s <device> uninstall <package>`. Check `actions_performed` in the report for the exact sequence attempted.
- **No (or few) runtime findings**: The parser is intentionally high-signal only (permission events, crashes with frames, cleartext http://, obvious secrets like api_key/sk_live_/AIza). Normal app logs are filtered. Use a test APK with deliberate behaviors during the capture window. Long lines are truncated in evidence (~300 chars) with basic redaction applied to secret patterns.
- **Feature not enabled**: Rebuild with `--features mobile-dynamic` (or `--features full`). Legacy `eggsec mobile <apk>` and `mobile static` remain available under just `--features mobile`.
- **Proxy config failures non-fatal**: Proxy setup (`settings put global http_proxy`) is best-effort in lab; failures/warnings recorded in `actions_performed` (no hard abort). Ensure device/emulator supports global proxy and that CA is installed for HTTPS if using mitmproxy. `--reset-proxy` is always safe to attempt.
- **Traffic capture file issues**: `--traffic-capture <file>` must point to a readable file (mitmproxy log, HAR-like, or text summary). Parser is summary-only (domains, counts, cleartext hints, high-signal findings); unreadable or empty files produce no `traffic_summary` (warning in actions). Large files are handled conservatively.
- **Permission ops require package + state**: `--grant`/`--revoke`/`--list-permissions` require the package to be installed on the target device and appropriate runtime state (e.g. for runtime permissions). Results surfaced as findings + optional `permission_state` in report; mismatches with static manifest are high-signal.
- **Policy confirmation or strict profile denial**: Under `ManualPermissive` (default CLI/TUI) you may see a confirmation prompt for `RequireConfirmation` (SafeActive + DefenseLab). Strict/MCP/agent profiles treat dynamic as Deny (standalone defense-lab surface; no MCP/agent exposure by design). Use `--yes` or the specific allow flag as appropriate for your profile.
- **AVD / API level notes**: Phase 1 targets modern emulators (API 34+). Older images may emit different log tags or lack runtime permission prompts. Granting dangerous permissions at install time vs runtime can change observed log events. For Frida (Phase 3a): requires rooted image or Frida-injected emulator with frida-server; frida CLI on host.
- **"adb: command not found" (convenience only)**: The external `adb` binary is optional (used only for `list_devices` pretty-printing). Pure-Rust TCP connect still works for emulators on known ports. Install platform-tools if you want the convenience listing.
- **"frida: ..." errors on real path**: frida CLI must be discoverable (which/frida --version or direct). Use --dry-run for simulation (no CLI needed). Real errors (missing CLI, unreachable device, timeout, script load, unknown builtin) are best-effort, logged to actions, and do not fail the overall mobile-dynamic run. Unknown builtin via run_builtin: clean Validation error listing available (basic-method-trace, crypto-keystore, bypass-validation, api-trace).
- **Report bridge / convert issues**: Native `--json` DynamicMobileReport is auto-bridged by `eggsec report convert` when the feature is present (categories become `mobile-dynamic-android-*`). If mixing static + dynamic reports, the bridge preserves both `mobile-*` and `mobile-dynamic-*` categories. Phase 3a/3b: frida findings -> mobile-dynamic-android-frida-<subcat> (incl. crypto-observation, api-trace, bypass-validation); extra info for frida_instrumentation (now includes structured count / corr_notes count / start_time in description when present).
- **Phase 3b specific**: --frida-script "builtin:NAME" (not a separate flag); correlation_notes and static_correlation appear for matching Frida+static/dynamic overlaps (visible in native JSON + bridge). Redaction applies to Frida evidence containing secrets (api_key etc.) or byte arrays.

See also: `scripts/test-mobile-dynamic.sh` (dry-run + optional real leg; now includes Phase 3a Frida dry leg), `dynamic.rs` (dispatcher + manifest + audit + Frida), `adb.rs` (connect/list), `runtime.rs` (parser + redaction), `frida.rs` (Phase 3a connect/execute/basic_method_trace + helpers), `commands/handlers/mobile.rs` (policy + allow gates for dynamic + Frida), and the phase plans for the exact happy-path command.

## Policy Note

**Static** (`mobile-static`):
- Gated via `CommandContext::evaluate_and_enforce_operation()` (`commands/handlers/mobile.rs`).
- `operation: "mobile-static"`, `risk: OperationRisk::SafeActive`, `required_features: ["mobile"]`.
- Feature must be present at compile time. `EnforcementContext` denies if missing. Strict profiles treat as mandatory.

**Dynamic** (`mobile-dynamic`, Phase 1 + Phase 2 closed 2026-06-12 + Phase 3a+3b Frida delivered):
- `operation: "mobile-dynamic"`, `mode: DefenseLab`, `risk: OperationRisk::SafeActive` (Intrusive for real/non-dry Frida ops), `required_features: ["mobile-dynamic"]`.
- Non-dry-run requires explicit `--allow-dynamic-mobile` (audited; same pattern as `wireless deauth --allow-active-wireless`).
- Real Frida requires explicit `--allow-frida` (Intrusive tier; audited; same pattern).
- Additional runtime confirmation prompt under ManualPermissive for operator discretion (high-risk/Frida cases).
- Lab manifest (if provided) is loaded and recorded; enforcement is primarily policy + provenance + device/app allowlist + user responsibility.
- MCP/agent exposure intentionally absent (standalone defense-lab; reporting bridge remains usable).
- Always produces policy decision + actions audit even in dry-run.
- Phase 2 (proxy/permissions/correlation) + Phase 3a+3b (Frida) use same gate; Frida actions/findings/instrumentation (now richer) + correlation recorded in audit/trail + report. Policy note updated for Frida tier (unchanged gating).

See `config/policy_decision.rs`, `commands/handlers/mobile.rs`, and the dynamic handoff plan for exact descriptors + ConfirmationClass handling.

## Future

- **Phase 1 static** (closed 2026-06-11): high-signal APK/IPA manifest/config analysis.
- **Phase 1 dynamic** (closed 2026-06-12 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md`): Android ADB core + runtime logcat analysis.
- **Phase 2 (proxy + permissions + correlation + close-out)** (closed 2026-06-12 per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (combined)): proxy foundation (device global `http_proxy` via `--proxy`; user-managed mitmproxy/CA; `--reset-proxy`; `--traffic-capture` for summary/findings) + runtime permission testing (`--grant-permission`/`--revoke-permission`/`--list-permissions`) + `correlate_findings`/`static_correlation` + final hygiene. `traffic_summary` + `permission_state` in report; bridge categories `mobile-dynamic-android-*` (e.g. traffic-summary, permission-state). All kept under single `mobile-dynamic` feature (M1 decision: no sub-feature split). Phase 2 officially closed.
- **Phase 3a (Frida foundation + basic_method_trace) delivered 2026-06-12** (under single `mobile-dynamic` per phase3-frida-expansion-plan.md Key Decision; no separate mobile-frida sub-feature). Real connect/execute_script + first builtin (sensitive method tracing for Cipher/keystore/auth/detection); CLI --frida-script/--allow-frida; handler policy (Intrusive for real Frida); report frida_instrumentation + frida-* findings + bridge (mobile-dynamic-android-frida-*); dry-run always safe/hardware-free; smoke + tests updated. Standalone defense-lab (MCP/agent absent). See plan (executed for 3a) + CLI examples above.
- **Phase 3b (additional builtins + correlation + richer evidence/structured + expanded tests) delivered 2026-06-12** (executed per phase3-frida-expansion-plan.md). 3 new builtins (crypto-keystore, bypass-validation, api-trace) via --frida-script "builtin:NAME" (dispatch in run_builtin; structured JSON output preferred; redaction on evidence). FridaInstrumentation richer (start_time/structured_results/correlation_notes). correlate_findings extended for Frida+static/dynamic overlaps (populates static_correlation). format/bridge/recommendations updated. Expanded tests + smoke Phase 3b leg. All under single mobile-dynamic; dry safe; real best-effort + --allow-frida gate. See plan (annotated executed) + CLI examples (builtin dry + combined for correlation).
- **Phase 3c (user script library + multi-script + advanced correlation + behavioral regression + evidence bundles) delivered 2026-06-12** (executed per phase3-frida-expansion-plan.md section 5). Reusable components via --frida-script "library:common-hooks" (embedded safe hooks emitting structured frida-*; resolve/run_frida_spec unifies builtin:/library:/file). Repeatable --frida-script for multi-script sessions (accumulate script_results/structured/enabled_builtins). FridaInstrumentation +regression_notes. --baseline (MobileBaseline JSON) + compare_to_baseline (produces regression notes + optional "behavioral-regression" findings). --evidence-bundle (pure flate2 .json.gz of full report + traffic + frida structured + actions). correlate_findings deepened with 3c cross-links (Frida+traffic, Frida+perm). New APIs: MobileBaseline, capture_baseline, compare_to_baseline, export_evidence_bundle, FRIDA_LIB_COMMON_HOOKS, resolve_frida_script_spec, run_frida_spec. examples/mobile-frida/ ships samples + docs. CLI ABOUTs updated with multi + library:/builtin: + regression/bundle examples. Expanded tests + smoke Phase 3c leg (multi + library + baseline + bundle + assertions + bridge). All under single mobile-dynamic; dry-run always safe/hardware-free + valid reports/bridges/bundles; real best-effort + --allow-frida (Intrusive) gate. Standalone defense-lab (MCP/agent/TUI/pipeline absent). See plan (annotated executed 2026-06-12) + docs/examples. (iOS constrained/TUI/MCP remain future per plan.)
- **Phase 4a (Core Correlation Engine + Evidence Foundation) delivered 2026-06-12 (executed per plan)**: non-breaking extension of `correlate_findings`/`CorrelatedFinding`/`static_correlation`. Adds `CorrelationEngine` + `correlate_reports` (high-level entrypoint) + enriched `CorrelatedFinding` (optional conservative 0-100 `score`, `CorrelationType` (Direct/Indirect/Behavioral/CrossLayer), `enrichment` phrase) + `CorrelationResult` (correlations + timeline + summary). Scoring inside `correlate_findings` (and engine path). `build_timeline` from report timestamps + actions + Frida start_time + regression notes (from 3c). Builds on 3c baseline/regression/bundles. 6 new unit tests; all ~85+ mobile-dynamic tests green. Dry-run safe/hardware-free; no new deps; serde roundtrips preserved. Standalone defense-lab (MCP/agent/TUI/pipeline absent). Phase 4b TUI deferred; reporting polish delivered (human output only) per standalone policy. Prefer `correlate_reports` / `CorrelationEngine` (with optional min_score) for full static+dynamic+Frida correlation with scores/timeline/summary; `correlate_findings` still available for low-level. See `plans/mobile-dynamic-phase4-actionable-intelligence-plan.md` (annotated executed) + dynamic.rs.
- **Phase 4b (TUI + Reporting Polish) worked through 2026-06-12**: TUI Foundation/Live Correlation/Session deferred (confirmed absent from TUI crate; 30 Tab variants, no Mobile tab or wiring); reporting polish delivered (human output enhancements in format_dynamic_report + build_dynamic_recommendations to surface regression_notes + Correlation / Regression section + callouts to correlate_reports/Engine). 1 new unit test. See plan (updated 2026-06-12) + dynamic.rs (~1063 recs, ~1088 format, ~1122 section, new test ~2179). TUI remains aspirational.
- **Static Phase 2 (deeper analysis, future)**: Deeper manifest/config analysis, basic library/SDK detection, improved iOS coverage, richer recommendations, and exportable evidence bundles.
- TUI tab and `ScanProfile` pipeline profiles (`mobile-static` / `mobile-dynamic` / `mobile-regression`) remain aspirational (Phase 4b TUI deferred per standalone policy).
- MCP/agent opt-in after security audit only (intentionally absent for standalone defense-lab surfaces).
- **Recommended workflow (Phase 2 closed)**: static baseline first → dynamic with proxy/permissions/correlation → use `correlate_reports` / `CorrelationEngine` for full static+dynamic+Frida correlation with scores/timeline/summary (or `correlate_findings` for low-level) → `report convert` / `report diff` / `report trend`. All under `--features mobile-dynamic`; lab context + `--allow-dynamic-mobile` + provenance-controlled test builds + `--lab-manifest` (advisory) for real runs.

## Data Model (Key Types)

**Static (always under `mobile`)**:
```rust
pub enum MobilePlatform { Android, Ios }

pub struct MobileFinding { ... }  // category, severity, title, description, recommendation, evidence
pub struct MobileScanReport {
    pub target: String,
    pub scan_type: String, // "mobile-static"
    ...
    pub findings: Vec<MobileFinding>,
    ...
}
pub fn to_scan_report_data(result: &MobileScanReport) -> ScanReportData { ... }
```

**Dynamic (under `mobile-dynamic` feature, Phase 1 Android-only + Phase 2 closed 2026-06-12 proxy/permissions/correlation)**:
```rust
pub struct LabManifest { pub allowed_device_serials: Vec<String>, pub allowed_packages: Vec<String> }

pub struct DynamicMobileFinding {
    pub category: String,   // "runtime-permission", "crash-log", "cleartext-observed", "log-secret-leak", "traffic-summary", "permission-state", ...
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommendation: String,
    pub evidence: Option<String>,
    /// Populated by `correlate_findings` for high-value static ↔ dynamic overlaps.
    pub static_correlation: Option<String>,
}

pub struct DynamicMobileReport {
    pub target: String,
    pub scan_type: String,  // "mobile-dynamic"
    pub platform: MobilePlatform, // Android only in P1
    pub device_serial: Option<String>,
    pub app_id: Option<String>,
    pub version: Option<String>,
    pub timestamp: String,
    pub findings: Vec<DynamicMobileFinding>,
    pub recommendations: Vec<String>,
    pub duration_ms: u64,
    pub actions_performed: Vec<String>,  // full audit trail
    pub dry_run: bool,
    pub traffic_summary: Option<TrafficSummary>,  // populated when --traffic-capture is provided
    pub permission_state: Option<PermissionState>, // populated when --list-permissions is provided
}

pub fn to_scan_report_data_dynamic(result: &DynamicMobileReport) -> ScanReportData { ... }
```

Phase 4a adds (to same module): `CorrelatedFinding` (now enriched: optional `score: Option<u8>`, `correlation_type: Option<CorrelationType>`, `enrichment: Option<String>`; non-breaking), `CorrelationType` (Direct/Indirect/Behavioral/CrossLayer), `CorrelationSummary`, `CorrelationResult` (correlations + timeline + summary), `CorrelationEngine` (min_score, `correlate`), and `correlate_reports` convenience. Scoring inside `correlate_findings` + engine path; `build_timeline` for cross-layer chronology.

See `crates/eggsec/src/mobile/{mod,apk,ipa,dynamic,adb,runtime,traffic}.rs`. Historical: `plans/mobile-first-handoff-plan.md`. Dynamic: `plans/dynamic-mobile-testing-loadout-design-plan.md` + Phase 1 handoff (complete 2026-06-12) + Phase 2 closed 2026-06-12 per combined closeout+kickoff plan (all under `mobile-dynamic`). Phase 3 (Frida) + Phase 4a (Core Correlation Engine) delivered 2026-06-12. Phase 4b TUI deferred; reporting polish (human output) delivered 2026-06-12.

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
