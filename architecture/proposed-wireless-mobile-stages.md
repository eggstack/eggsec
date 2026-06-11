# Proposed: Thin Optional Pipeline Stages for Wireless and Mobile

**Context**: During the 2026-06-11 integration work (see `plans/integration-work-plan.md`), we strengthened the optional `to_scan_report_data` reporting bridges for the two standalone defense-lab modules (`wireless` and `mobile`) and documented the "standalone + optional bridge" design. Priority 3 of that plan asked for a short design note evaluating whether to add thin, optional `ScanProfile` stages (`WirelessAnalysis`, `MobileStatic`) before committing to (or rejecting) full pipeline integration.

**Current State (post-integration-work-plan)**:
- Both modules are intentionally standalone-complete defense-lab surfaces.
  - `eggsec wireless <iface>` (CLI + TUI tab, `wireless` feature): passive WiFi recon, local `WirelessScanResult` + findings emitted directly; optional bridge + CLI auto-bridge for unified reports.
  - `eggsec mobile <apk|ipa>` (CLI-only, `mobile` feature): pure-Rust static APK/IPA analysis, local `MobileScanReport`/`MobileFinding` emitted directly; optional bridge + CLI auto-bridge.
- Neither is wired into the main `ScanProfile` / pipeline stage runner (`pipeline/stage.rs`, `cli/mod.rs` ScanProfile enum).
- `auth-test` is explicitly "local-only, no bridge, no profiles" (distinct from pipeline `ScanProfile::Auth`).
- Defense-lab mode and policy treat them as separate from chained assessment profiles.
- Aspirational mentions exist in `architecture/defense_lab.md` (Future Integration) and historical plans.

**Design Questions**:
1. Should we add thin optional stages (`WirelessAnalysis`, `MobileStatic`) that can be composed into defense-lab `ScanProfile`s (or new dedicated profiles like `wireless-defense`, `mobile-static`)?
2. How would they interact with the existing `ScanProfile` system (ordering, prerequisites like scope, feature gating, output model)?
3. Should they be feature-gated behind the same `wireless`/`mobile` Cargo features, or require additional opt-in?
4. What is the value vs. complexity/risk (standalone nature, policy surface, TUI/CLI dispatch, agent/MCP exposure)?

**Pros of Adding Thin Stages**:
- Unified "run everything via `eggsec scan --profile ...`" UX for users who want wireless or mobile as part of a larger defense-lab run.
- Easier composition with existing recon/fingerprint/output stages; single RunManifest / diff / trend surface.
- Agent/MCP could express "include wireless recon" via profile rather than invoking separate standalone commands.
- Consistent with how other defense-lab profiles (defense-lab, waf-regression, etc.) are expressed today.
- The reporting bridge already exists; a stage would just invoke the existing scanner/analyzer and feed `to_scan_report_data` (or the native types) into the normal output path.

**Cons / Risks**:
- **Standalone contract**: The explicit design decision (2026-06-11) was to keep them lightweight, opt-in, and non-intrusive. Adding stages would make them "first-class" in the pipeline, potentially implying more support, TUI integration, scope requirements, and policy surface.
- **Policy / enforcement complexity**: Wireless requires root/CAP_NET_ADMIN + Linux + interface; mobile is file-local. Embedding them in profiles would require careful `OperationDescriptor` / risk / required_features handling and possibly new `ConfirmationClass` or capability bits. Current standalone handlers already go through `evaluate_and_enforce_operation` correctly.
- **TUI / UX split**: Wireless already has a tab; mobile does not (CLI-only Phase 1). A pipeline stage would surface in TUI "scan" flows, which may not match user expectations for "passive WiFi recon on this interface" or "static analysis of this lab binary I just built".
- **Feature / platform reality**: Wireless is Linux-only at runtime (iwlist); mobile is pure-Rust but gated. Pipeline profiles are expected to be more portable/cross-platform in docs. Mixing would increase "it only works on Linux with privs" surprises in general profiles.
- **Scope & target model mismatch**: Most pipeline stages are network-target oriented. Mobile is a local file path (APK/IPA). Wireless is an interface name + passive listen (no "target host"). This would require special-casing in scope enforcement, target expansion, and `LoadedScope` provenance checks.
- **Maintenance / testing surface**: Each stage adds integration tests, negative tests, policy regression coverage, TUI task wiring, output expectations, and agent visibility rules. Given that the standalone + bridge already satisfies the "CI/regression + unified reporting" use cases (per success criteria of the integration plan), the incremental value may be low.
- **Auth precedent**: We deliberately kept `auth-test` out of profiles and conversion (see `architecture/auth.md`). Adding stages for wireless/mobile would create an inconsistency unless we also revisit auth (which is explicitly out of scope per the integration plan).

**Interaction with ScanProfile System**:
- Current defense-lab profiles are defined in `cli/mod.rs` and executed via `pipeline/`. Adding new variants would be straightforward (enum + stage list).
- A `WirelessAnalysis` stage could be a no-op or thin wrapper that calls into `wireless::run_cli` logic (or the scanner directly) and contributes findings + `wireless_networks` to the `ScanReportData` / RunManifest.
- A `MobileStatic` stage would need to accept a file path (different from network targets) and would likely only be valid in defense-lab / local-only profiles.
- Feature gating: stages could be compiled in only when the corresponding Cargo feature is active (similar to how `nse-safe` requires `nse` + `nse-sandbox`).
- Ordering: wireless is passive side-channel (could run early or in parallel); mobile is offline file (could be a leaf or separate "mobile" profile).
- Output: both already produce (or bridge to) `ScanReportData`, so they would fit the existing `output/` and `eggsec-output` paths with minimal change.

**Recommended Decision (as of 2026-06-11 integration close)**:
**Defer full stage implementation.**

Rationale:
- The primary goals of the integration work (robust optional reporting bridge + clear documentation of when/how to use it + explicit "standalone by design" record) are achieved without stages.
- Standalone + bridge + auto-bridge in `report convert` already enables the documented CI/regression, SARIF/JUnit, HTML/Markdown, trend, etc. flows for both modules.
- Adding stages would increase surface area (policy, scope, TUI, agent, platform constraints) with unclear incremental user value at this time.
- The design note itself serves as the decision record. Future work can revisit if a concrete need arises (e.g., agent-driven "run full defense-lab including wireless recon on authorized lab APs" or combined mobile+backend regression profiles).
- No `ScanProfile` enum variants, no new stage implementations, and no profile additions are required or performed in this round (consistent with "Out of Scope" in `integration-work-plan.md`).

**If Reconsidered Later**:
- Start with a narrow defense-lab-only profile (e.g. `wireless-defense`) rather than injecting into general profiles.
- Keep mobile as a file-target special case or separate `mobile-static` profile.
- Re-use the existing `to_scan_report_data` (or make the stage produce native + bridge) to avoid duplicating analysis logic.
- Update policy descriptors, required capabilities/features, scope rules, and TUI tab integration explicitly.
- Add the evaluation to a future wave plan with acceptance criteria (tests, docs, agent visibility, negative policy tests).

**Files / References**:
- Plan: `plans/integration-work-plan.md` (Priority 3)
- Architecture: `architecture/{wireless,mobile,defense_lab,cli_commands,output}.md` (updated with Integration sections + design decision cross-refs)
- Implementation: `crates/eggsec/src/wireless/mod.rs` (`to_scan_report_data`), `mobile/mod.rs` (same), `commands/handlers/report.rs` (auto-bridge), `commands/handlers/{wireless,mobile}.rs`
- Docs: `docs/WIRELESS.md`, `docs/MOBILE.md`, `docs/USAGE.md`, `README.md`, `CAPABILITIES.md`, `AGENTS.md`
- Close-out: `plans/wireless-micro-closeout-checklist.md`, `plans/mobile-final-closeout-plan.md`, `plans/new-modules-integration-and-closeout-plan.md`

**Status**: Decision recorded. No code changes in this iteration. Revisit only with a new concrete use case and updated plan.

(End of design note)
