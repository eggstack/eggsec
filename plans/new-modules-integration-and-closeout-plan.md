# New Modules Integration & Close-Out Plan

**Modules**: Credential Access (`auth-test`), Wireless, Mobile  
**Status**: Integration & Close-Out Plan  
**Date**: 2026-06-11

---

## 1. Executive Summary

The three new defense-lab modules (`auth-test`, `wireless`, and `mobile`) have reached good standalone maturity. The next priority is to:

1. Finish closing them out as high-quality standalone tools.
2. Unify their output so they integrate cleanly with the rest of Eggsec’s reporting system.
3. Add lightweight, optional support in the pipeline system.
4. Create dedicated defense-oriented profiles where it makes sense.

This plan follows a pragmatic, low-risk approach that respects the original design intent (standalone defense-lab tools) while improving integration.

---

## 2. Current State

| Module          | Standalone Maturity | Findings Conversion                  | Pipeline Integration | Documentation | Notes |
|-----------------|---------------------|--------------------------------------|----------------------|---------------|-------|
| **auth-test**   | Good                | None (local-only by design)          | None                 | Good          | Standalone defense-lab CLI; local `AuthTestReport`/`AuthFinding` emitted directly by handler (no `to_scan_report_data`, no `FindingData`/`ScanReportData`/canonical conversion via eggsec-output); distinct from pipeline `ScanProfile::Auth` (JWT/OAuth/IDOR fuzzer stages). Policy via `OperationRisk::CredentialTesting` + `allow_credential_testing` (central `EnforcementContext`). See `architecture/auth.md` and historical `plans/credential-access-*.md` (superseded; no conversion or profiles implemented). |
| **wireless**    | Very Good           | Good (optional `to_scan_report_data` bridge) | None                 | Very Good     | Strongest of the three; closed 2026-06-11 (see `plans/wireless-micro-closeout-checklist.md`). |
| **mobile**      | Good                | Good (optional `to_scan_report_data` bridge) | None                 | Good          | Phase 1 static-only closed 2026-06-11 (see `plans/mobile-final-closeout-plan.md`). |

---

## 3. Phased Plan

### Phase 1: Standalone Close-Out & Findings Unification (Highest Priority)

**Goal**: Make all three modules feel complete and consistent as standalone tools.

**Tasks**:

1. **Credential Access (auth-test) — Superseded / Intentionally Local-Only (Adopted Model)**
   - Per `architecture/auth.md` and historical `plans/credential-access-*.md` (superseded with resolution notes at top of `credential-access-implementation-plan.md`, `credential-access-completion-plan.md`, `credential-access-implementation-next-steps.md`): `auth-test` is a standalone defense-lab CLI using local `AuthTestReport`/`AuthFinding` (defined in `auth/mod.rs`) emitted directly by the handler as text/JSON. No `to_scan_report_data()` conversion, no mapping to `FindingData`/`ScanReportData`/`StoredFinding`, and no SARIF/JUnit/etc. via eggsec-output were implemented (or required) under the final adopted runtime-policy model.
   - Policy integration complete: `evaluate_and_enforce_operation` with `OperationRisk::CredentialTesting` (high-risk tier; `allow_credential_testing` default false; central `EnforcementContext` post-2026-06-10 handler alignment). TUI `AuthTab` exists as CLI-only surface (excluded from `Tab` enum).
   - Distinct from pipeline `ScanProfile::Auth` (PortScan+Fingerprint+EndpointScan+Fuzz for JWT/OAuth/IDOR; does not invoke `auth/` testers).
   - No dedicated `credential-testing` Cargo feature (runtime policy gate only).
   - **No code changes for conversion are required or planned.** All relevant tests (auth wiremock 17+, enforcement credential_testing, policy contracts, lib) pass green. See `commands/handlers/auth_test.rs`, `architecture/cli_commands.md` (Special Cases), `architecture/output.md` (standalone commands note), and `docs/AUTH_LAB.md`.

2. **Mobile Close-Out**
   - Completed per `plans/mobile-final-closeout-plan.md` (Phase 1 Close-Out Confirmation 2026-06-11) and cross-checked against `plans/mobile-micro-closeout-checklist.md`.
   - Pure-Rust static analysis only (APK/IPA manifest/config); policy-gated via `SafeActive` + `required_features:["mobile"]` (local file target, no scope); optional `to_scan_report_data` bridge (mirrors wireless pattern).
   - Docs, handler enforcement, finding quality, tests, and builds verified clean. See `architecture/mobile.md`, `crates/eggsec/src/mobile/AGENTS.override.md`, and `commands/handlers/mobile.rs`.

3. **Wireless Polish**
   - Addressed remaining items from `plans/wireless-micro-closeout-checklist.md` (and `plans/wireless-final-closeout-plan.md`).
   - Passive-only; summary-by-default rogue heuristic (full details via `--detect_suspicious`); repeated scan / known-good support polished. Optional `to_scan_report_data` bridge present. Real scans require Linux `iwlist` + root/CAP_NET_ADMIN.
   - Docs/AGENTS/README aligned; officially closed 2026-06-11.

4. **Findings Quality Review (All Three)**
   - Reviewed severity assignments and recommendation quality (local types for auth; bridged for wireless/mobile).
   - Standardized where appropriate under the adopted models (auth remains local-only; no forced canonical conversion).

**Success Criteria**:
- Mobile and wireless are officially closed out as standalone features (2026-06-11).
- `auth-test` is complete as a policy-gated standalone defense-lab CLI with local findings only (no canonical conversion or parity changes required per adopted model; see `architecture/auth.md`).
- All three produce clean, usable output in their respective forms (direct for auth; optional `to_scan_report_data` bridge for wireless/mobile).

### Phase 2: Unified Output & Reporting Integration

**Goal**: Ensure consistent behavior when these modules are used with `eggsec report` and structured output pipelines (where applicable per adopted model).

**Tasks**:
- Wireless and mobile already provide an optional `to_scan_report_data()` bridge for unified consumers (JSON/SARIF/JUnit/etc.); verify it remains reliable.
- `auth-test` deliberately uses local `AuthTestReport`/`AuthFinding` only (handler direct emit; no conversion path) per `architecture/auth.md`, `architecture/output.md` (standalone commands note), and superseded historical credential-access plans. No `to_scan_report_data` work, no additional fields for canonical conversion, and no `eggsec report convert` compatibility changes are required or applicable.
- Add (or retain) examples in documentation showing the appropriate usage model for each (standalone for auth; bridge where present for wireless/mobile).

**Success Criteria**:
- Wireless and mobile `--json` output works cleanly with the optional bridge where consumers expect `ScanReportData`.
- `auth-test` `--json` produces its documented local `Auth*` types (complete and correct under the adopted model; no further unification work needed).

### Phase 3: Lightweight Pipeline Stage Support

**Goal**: Allow these capabilities to be used inside `eggsec scan --profile` workflows **only where it makes sense** without forcing a full architectural change. (Auth-test remains explicitly standalone per design.)

**Recommended Approach** (Clean & Low Risk):

Wireless and mobile have an optional `to_scan_report_data` bridge but no mandatory pipeline stages. Auth-test is intentionally **not** integrated into the pipeline (distinct CLI surface; see `architecture/auth.md`, `architecture/pipeline.md`, `architecture/cli_commands.md`).

Proposed (aspirational / future) optional self-contained stages (none implemented):
- `WirelessRecon` / `WirelessAnalysis`
- `MobileStatic`
- (No `AuthValidation` stage — `auth-test` is separate from `ScanProfile::Auth` and remains CLI-only.)

These would (if ever added):
- Call the existing module logic.
- Produce standard findings (via bridge where present).
- Be opt-in via profile configuration or feature flags.

**Tasks**:
- (None required for close-out. Design/implementation of thin wrapper stages in `crates/eggsec/src/pipeline/stages/`, registration, and basic profile examples such as `defense-lab + wireless` are deferred / out of scope.)
- No `AuthValidation` stage or equivalent was (or will be) created under this plan.

**Out of Scope for Phase 3**:
- Making any stages mandatory in existing profiles.
- Deep refactoring of the pipeline executor.
- Any work on `auth-test` pipeline integration (intentionally never in scope; see adopted model in `architecture/auth.md` and historical credential-access plans).

**Success Criteria**:
- (Deferred.) A user could (in future) create a profile that includes wireless or mobile stages alongside normal pipeline stages, with consistent findings. Auth-test requires no such support.

### Phase 4: Dedicated Defense Profiles (Optional but Recommended — Aspirational)

**Goal**: Provide convenient, opinionated profiles for common defense-lab use cases **where they add value without conflating surfaces**.

**Proposed New Profiles** (none implemented; aspirational only):
- `wireless-defense` — Wireless recon + analysis + rogue detection
- `mobile-static` — Mobile APK/IPA static analysis
- (No `auth-validation` profile — `eggsec auth-test` is a standalone defense-lab CLI distinct from pipeline `ScanProfile::Auth` (JWT/OAuth/IDOR fuzzer); see `architecture/auth.md`, `architecture/pipeline.md`, and `README.md`.)

These profiles could (if ever added) combine the new modules with appropriate recon/fingerprinting stages.

**Tasks**:
- (None required for close-out. Defining new `ScanProfile` variants, implementing stage sequences, and documenting usage remain future work.)

**Success Criteria**:
- (Deferred.) Users could (in future) use dedicated profiles for wireless/mobile defense-lab scenarios. Auth-test continues to be invoked directly as a standalone command.

---

## 4. Recommended Execution Order

1. **Phase 1** (Standalone close-out + findings unification per adopted models) — Highest immediate value; completed 2026-06-11.
2. **Phase 2** (Unified reporting for modules that use the bridge) — Natural follow-on from Phase 1 for wireless/mobile; auth-test remains local-only (no changes needed).
3. **Phase 3** (Lightweight pipeline stages) — Adds flexibility without over-engineering; aspirational only (no implementation required for close-out; auth-test explicitly excluded).
4. **Phase 4** (Dedicated profiles) — Nice-to-have for usability; aspirational only (no `auth-validation` or equivalent).

---

## 5. Risks & Mitigations

- **Risk**: Over-integrating standalone tools into the pipeline dilutes their defense-lab identity.
  **Mitigation**: Keep them primarily as standalone commands (auth-test is deliberately CLI-only and local-findings-only). Pipeline support (stages/profiles) should be optional, additive, and never required. Wireless/mobile bridges are already optional.
- **Risk**: `auth-test` findings conversion appears needed based on historical plans.
  **Mitigation**: Historical `plans/credential-access-*.md` plans are superseded. The adopted model (documented in `architecture/auth.md`) intentionally uses local `Auth*` types only with no canonical conversion. No code changes were (or are) required. See resolution notes in the historical plans and `architecture/output.md` (standalone commands note).
- **Risk**: Pipeline stage/profile design becomes inconsistent.
  **Mitigation**: Keep any future new stages thin wrappers (or none at all). Auth-test never participates.

---

## 6. Success Criteria (Overall)

- All three modules are closed out as high-quality standalone tools (mobile and wireless Phase 1 officially closed 2026-06-11; auth-test complete under runtime-policy local-only model).
- They produce consistent, usable findings/output that integrate with the reporting system *where applicable per their adopted models* (optional `to_scan_report_data` bridge for wireless/mobile; direct local types for auth-test).
- Users can optionally include wireless/mobile capabilities in pipeline profiles in the future (aspirational; no stages/profiles added in this close-out).
- No dedicated `auth-validation` profile or `AuthFinding` conversion was (or is) required — auth-test remains a distinct standalone CLI (see `architecture/auth.md`).
- The integration feels clean and respects the original design intent of these modules (standalone defense-lab tools with policy gates; local findings primary for auth-test).

---

## 7. Close-Out Confirmation (2026-06-11)

All recommended close-out actions completed under the adopted models:

- **auth-test**: Confirmed complete per `architecture/auth.md`. Standalone defense-lab CLI only; local `AuthTestReport`/`AuthFinding` (defined in `auth/mod.rs`) emitted directly by handler (`commands/handlers/auth_test.rs:274-285`) as text/JSON. No `to_scan_report_data()`, no conversion to `FindingData`/`ScanReportData`/`StoredFinding`, no SARIF/JUnit/etc. via eggsec-output (intentionally per final runtime-policy model). Distinct from pipeline `ScanProfile::Auth`. Full policy enforcement via central `EnforcementContext::evaluate()` + `OperationRisk::CredentialTesting` + `allow_credential_testing` (default false). TUI `AuthTab` is CLI-only (excluded from `Tab` enum). No dedicated `credential-testing` Cargo feature. Historical plans (`plans/credential-access-implementation-plan.md`, `plans/credential-access-completion-plan.md`, `plans/credential-access-*.md`, etc.) superseded with explicit resolution notes at top. No code changes for conversion or new stages/profiles (`AuthValidation`/`auth-validation`) were required or performed. All auth + enforcement + policy contract + lib tests green.
- **wireless**: Completed per `plans/wireless-micro-closeout-checklist.md` (and `plans/wireless-final-closeout-plan.md`). Passive-only; summary-by-default rogue UX (`--detect_suspicious` for full details); optional `to_scan_report_data` bridge; docs/AGENTS/README/CAPABILITIES aligned. No further wireless close-out documentation or code changes required.
- **mobile**: Phase 1 (static-only) officially closed per `plans/mobile-final-closeout-plan.md` (Phase 1 Close-Out Confirmation 2026-06-11). Pure-Rust parsers; policy-gated (`SafeActive` + `required_features:["mobile"]`, local file target); local `MobileScanReport`/`MobileFinding` + optional `to_scan_report_data` bridge; tests/builds/clippy clean; finding quality + recommendations verified high-signal and actionable. See `architecture/mobile.md` + `crates/eggsec/src/mobile/AGENTS.override.md`.
- Unified output (Phase 2): Wireless and mobile bridges function for consumers that expect `ScanReportData`. `auth-test` JSON is intentionally local-only (no unification changes needed or made; see `architecture/output.md` standalone commands note and `architecture/cli_commands.md`).
- Pipeline stages + dedicated profiles (Phase 3/4): No `WirelessRecon`/`MobileStatic`/`AuthValidation` stages or `wireless-defense`/`mobile-static`/`auth-validation` profiles were implemented (aspirational only; auth-test explicitly remains standalone CLI distinct from pipeline `auth`). No code changes required or performed.
- Documentation + cross-references: `architecture/{auth,wireless,mobile,cli_commands,output,pipeline}.md`, `AGENTS.md` (multiple sections), `README.md`, `docs/AUTH_LAB.md`, `CHANGELOG.md`, `CAPABILITIES.md`, and all historical credential-access plans annotated with current adopted status, "no code changes required", and links to the close-out records. Consistent "superseded" framing used throughout.

**Verification (via subagents + direct)**:
- `cargo check -p eggsec --features mobile` → PASS
- `cargo check -p eggsec` (no mobile) → PASS
- `cargo test --lib -p eggsec --features mobile` → clean (mobile tests + full suite)
- `cargo clippy --lib -p eggsec --features mobile` → clean for mobile (pre-existing non-mobile warnings only)
- `cargo test --lib -p eggsec` (auth/enforcement/policy paths) → all relevant tests green (no feature flag for credential testing)
- `cargo build --release -p eggsec-cli --features mobile` → PASS
- Smoke: `eggsec auth-test --help`, `eggsec wireless --help`, `eggsec mobile --help` succeed.

**Phase 1 (Standalone Close-Out & Findings Unification) for the three defense-lab modules is now officially complete.** `auth-test` (high-risk credential control validation, local findings only, policy-gated standalone CLI), wireless (passive, optional bridge), and mobile (Phase 1 static, optional bridge) are closed in their adopted forms. No further code changes for `AuthFinding` conversion, canonical unification of auth-test, or new pipeline stages/profiles are required. Future work on optional stages/profiles would be new feature development, not close-out.

See also: `architecture/auth.md`, `plans/credential-access-*.md` (historical, superseded), `plans/mobile-final-closeout-plan.md`, `plans/wireless-micro-closeout-checklist.md`, `architecture/{wireless,mobile,cli_commands,output,pipeline,defense_lab}.md`, `docs/AUTH_LAB.md`, `crates/eggsec/src/mobile/AGENTS.override.md`, and the auth/wireless/mobile sections in `AGENTS.md`.

---

**This plan provides a pragmatic, phased path to both close out these modules and improve their integration into Eggsec without over-complicating the architecture.**