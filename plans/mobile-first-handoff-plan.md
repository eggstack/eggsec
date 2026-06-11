# Mobile Feature - First Handoff Plan

**Status**: Initial Handoff Plan  
**Focus**: Bring mobile app security testing capabilities to a usable standalone state  
**Philosophy**: Defense validation first — start with safe, high-value capabilities

---

## 1. Current State

**Existing Foundation**:
- No dedicated `mobile/` module currently exists in `crates/eggsec/src/`.
- Mobile-related work is not yet present in the codebase.
- The project already has strong patterns for:
  - Feature gating (`wireless`, `stress-testing`, etc.)
  - Runtime policy enforcement via `EnforcementContext`
  - Structured findings and reporting (`to_scan_report_data`, SARIF/JUnit support)
  - CLI command structure
  - Defense-lab framing and safety messaging

**Opportunity**:
Mobile app security is a natural extension for Eggsec’s defense-validation mission. Many organizations need repeatable, scoped testing of mobile apps in lab environments (especially internal/corporate apps, partner apps, or during secure development lifecycles).

---

## 2. Goal: Usable Standalone State

By the end of the initial phase, `eggsec mobile` (or similar) should provide:

- Reliable static analysis of Android APKs and iOS IPAs in authorized lab environments.
- Detection of common high-impact mobile security issues (insecure storage, weak transport, hardcoded secrets, improper permissions, etc.).
- Clean, structured output suitable for reports and regression.
- Strong safety controls and clear lab/defense framing.
- Good CLI experience with JSON support.

**Out of Scope for First Phase**:
- Full dynamic instrumentation / Frida integration (can come later).
- Active exploitation or runtime manipulation.
- Deep reverse engineering or decompilation pipelines.
- Broad supply-chain / dependency analysis (can be added incrementally).

---

## 3. Recommended Phased Approach

### Phase 1: Foundation & Static Analysis (Primary Focus)

**Goal**: Deliver immediate value with safe, high-signal static checks.

**Key Capabilities**:
- APK parsing and basic manifest analysis (Android)
- IPA / Info.plist analysis (iOS)
- Detection of:
  - Insecure data storage (world-readable files, shared preferences, Keychain issues)
  - Insecure transport (cleartext HTTP, weak TLS configs)
  - Hardcoded secrets / API keys
  - Dangerous permissions
  - Backup/export flags
  - Debuggable / test-only builds
- Basic certificate / signing analysis
- Structured findings output

**Suggested Structure**:
- New `crates/eggsec/src/mobile/` module
- Feature gate: `mobile`
- CLI command: `eggsec mobile <apk-or-ipa>`
- Integration with existing `EnforcementContext` (risk tier: `SafeActive` or new `MobileStatic`)

### Phase 2: Enhanced Analysis & Polish

- Deeper manifest and configuration analysis
- Basic dependency / library detection
- Improved iOS support
- Better reporting and recommendations
- Optional dynamic hooks (Frida-based) behind additional safety gates

### Phase 3: Pipeline Integration (Later)

- `mobile-static` and `mobile-regression` profiles
- Integration with main scan pipeline for combined web + mobile backend testing

---

## 4. Safety & Scope Considerations

Mobile testing has specific considerations:
- Apps often contain production credentials or sensitive logic even in “test” builds.
- Dynamic analysis can be intrusive.
- Legal/authorization scope must cover the specific app binary.

**Recommended Guardrails**:
- Strong emphasis on lab / authorized defensive validation use.
- Clear requirements around app provenance (must be provided by the tester).
- Feature gating + runtime policy controls.
- Start with static analysis only (lowest risk).
- Document that dynamic capabilities (if added) require explicit opt-in and higher risk tier.

---

## 5. Suggested First Implementation Steps

1. Create the initial module skeleton (`crates/eggsec/src/mobile/mod.rs`).
2. Implement basic APK parsing (using a lightweight Rust crate or shelling out to `apkanalyzer` / `aapt` in a controlled way).
3. Add iOS IPA/Info.plist parsing.
4. Build the core analysis rules for high-impact findings.
5. Wire up CLI args and basic `run_cli()` function.
6. Add structured output + `to_scan_report_data()` support.
7. Create initial documentation (`docs/MOBILE.md`).
8. Update `CAPABILITIES.md`, `README.md`, and `SAFETY.md`.

---

## 6. Success Criteria for Initial Phase

- `eggsec mobile <file>` works for both Android and iOS binaries in lab environments.
- It produces useful, structured findings with recommendations.
- Output works in both human-readable and JSON modes.
- The feature is clearly framed as defense-lab / authorized use only.
- Good documentation exists.
- The implementation follows existing Eggsec patterns (safety, findings, reporting).

---

## 7. Risks & Mitigations

- **Risk**: Parsing mobile binaries can be complex and fragile.
  **Mitigation**: Start simple (manifest + basic file inspection) and use established tools where possible.
- **Risk**: Scope creep into dynamic analysis too early.
  **Mitigation**: Explicitly scope Phase 1 to static analysis only.
- **Risk**: Low adoption if output is not actionable.
  **Mitigation**: Focus on high-signal findings with clear remediation guidance.

---

**This is the first handoff plan for mobile. The goal is to reach a usable standalone static analysis capability before expanding into dynamic or pipeline-integrated features.**