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

| Module          | Standalone Maturity | Findings Conversion | Pipeline Integration | Documentation | Notes |
|-----------------|---------------------|---------------------|----------------------|---------------|-------|
| **auth-test**   | Good                | Partial             | None                 | Good          | Needs full `to_scan_report_data()` conversion |
| **wireless**    | Very Good           | Good                | None                 | Very Good     | Strongest of the three |
| **mobile**      | Good                | Good                | None                 | Good          | Recent handler + documentation improvements |

---

## 3. Phased Plan

### Phase 1: Standalone Close-Out & Findings Unification (Highest Priority)

**Goal**: Make all three modules feel complete and consistent as standalone tools.

**Tasks**:

1. **Credential Access Findings Conversion**
   - Implement full `to_scan_report_data()` conversion in the `auth-test` handler (similar to wireless and mobile).
   - Map `AuthFinding` → `FindingData` with appropriate severity, category (`authentication` or `credential-access`), remediation, etc.
   - Ensure JSON, SARIF, JUnit, and HTML outputs work properly.

2. **Mobile Close-Out**
   - Complete the items in `plans/mobile-final-closeout-plan.md` and `plans/mobile-micro-closeout-checklist.md`.
   - Add mobile entry to `CAPABILITIES.md` (already partially done).
   - Final polish on findings quality and recommendations.

3. **Wireless Polish**
   - Address any remaining items from `plans/wireless-micro-closeout-checklist.md`.
   - Ensure rogue detection UX and repeated scan output are polished.

4. **Findings Quality Review (All Three)**
   - Review severity assignments and recommendation quality across all modules.
   - Standardize category naming where possible.

**Success Criteria**:
- All three modules produce clean, usable `FindingData`.
- Mobile and wireless are officially closed out as standalone features.
- `auth-test` has parity with the other two in reporting integration.

### Phase 2: Unified Output & Reporting Integration

**Goal**: Ensure consistent behavior when these modules are used with `eggsec report` and structured output pipelines.

**Tasks**:

- Verify that `to_scan_report_data()` works reliably for all three modules.
- Add any missing fields (e.g., `app_id`/`version` for mobile, better evidence for auth findings).
- Ensure `--json` output from standalone commands is compatible with `eggsec report convert`.
- Add examples in documentation showing how to use these modules with the reporting system.

**Success Criteria**:
- Running any of the three commands with `--json -o file.json` produces output that works cleanly with `eggsec report`.

### Phase 3: Lightweight Pipeline Stage Support

**Goal**: Allow these capabilities to be used inside `eggsec scan --profile` workflows without forcing a full architectural change.

**Recommended Approach** (Clean & Low Risk):

Create optional, self-contained stages:

- `WirelessRecon` / `WirelessAnalysis`
- `MobileStatic`
- `AuthValidation` (distinct from current `ScanProfile::Auth`)

These stages would:
- Call the existing module logic.
- Produce standard findings.
- Be opt-in via profile configuration or feature flags.

**Tasks**:
- Design and implement thin wrapper stages in `crates/eggsec/src/pipeline/stages/`.
- Register them so they can be included in custom profiles.
- Add basic profile examples (e.g., `defense-lab + wireless`).

**Out of Scope for Phase 3**:
- Making these stages mandatory in existing profiles.
- Deep refactoring of the pipeline executor.

**Success Criteria**:
- A user can create a profile that includes one or more of these new stages.
- The stages produce consistent findings alongside normal pipeline stages.

### Phase 4: Dedicated Defense Profiles (Optional but Recommended)

**Goal**: Provide convenient, opinionated profiles for common defense-lab use cases.

**Proposed New Profiles**:
- `wireless-defense` — Wireless recon + analysis + rogue detection
- `mobile-static` — Mobile APK/IPA static analysis
- `auth-validation` — Auth control validation (distinct from `auth` profile)

These profiles can combine the new modules with appropriate recon/fingerprinting stages.

**Tasks**:
- Define the new `ScanProfile` variants.
- Implement the profile stage sequences.
- Document usage and intent clearly.

---

## 4. Recommended Execution Order

1. **Phase 1** (Standalone close-out + findings conversion) — Highest immediate value.
2. **Phase 2** (Unified reporting) — Natural follow-on from Phase 1.
3. **Phase 3** (Lightweight pipeline stages) — Adds flexibility without over-engineering.
4. **Phase 4** (Dedicated profiles) — Nice-to-have for usability.

---

## 5. Risks & Mitigations

- **Risk**: Over-integrating standalone tools into the pipeline dilutes their defense-lab identity.
  **Mitigation**: Keep them primarily as standalone commands. Pipeline support should be optional and additive.
- **Risk**: `auth-test` findings conversion is complex.
  **Mitigation**: Start with core fields and iterate. Reuse patterns from wireless/mobile.
- **Risk**: Pipeline stage design becomes inconsistent.
  **Mitigation**: Keep new stages thin wrappers around existing module logic.

---

## 6. Success Criteria (Overall)

- All three modules are closed out as high-quality standalone tools.
- They produce consistent, usable findings that integrate with the reporting system.
- Users can optionally include them in pipeline profiles.
- Dedicated defense profiles exist for common use cases.
- The integration feels clean and respects the original design intent of these modules.

---

**This plan provides a pragmatic, phased path to both close out these modules and improve their integration into Eggsec without over-complicating the architecture.**