# Integration Next Steps - Handoff Plan

**Purpose**: Focused handoff plan for the next logical integration steps on the new modules (Wireless, Mobile, Credential Access).  
**Target Audience**: Smaller model / implementation agent  
**Date**: 2026-06-11

---

## 1. Goal

Complete the highest-value near-term integration work:

- Strengthen the optional reporting bridges for Wireless and Mobile.
- Improve documentation so users understand how these modules integrate with the rest of Eggsec.
- Clarify the intentional design of `auth-test` (local findings only).

This work should feel like a natural next step after the standalone close-out work.

---

## 2. Scope

**In Scope**:
- Review and improve `to_scan_report_data()` for Wireless and Mobile.
- Add unit tests for the conversion functions.
- Add dedicated "Integration with Reporting" sections in the documentation.
- Clarify `auth-test` output model in docs.

**Out of Scope**:
- TUI integration
- MCP / Agent integration
- Pipeline stage implementation
- New `ScanProfile` variants
- Any changes to `auth-test` findings model (keep it local-only)

---

## 3. Current State

- Wireless and Mobile already have basic `to_scan_report_data()` implementations.
- Documentation exists but lacks clear integration guidance.
- `auth-test` is correctly designed as local-only (no bridge needed).

---

## 4. Tasks

### Task 1: Improve Wireless Reporting Bridge

**Files to modify**:
- `crates/eggsec/src/wireless/mod.rs`

**Actions**:
- Review the existing `to_scan_report_data()` function.
- Ensure the following fields are well populated:
  - `severity`
  - `category` (use specific values like `wireless-rogue`, `wireless-security`, `wireless-config`)
  - `evidence`
  - `remediation`
- Handle edge cases (empty results, repeated scans).
- Add or expand unit tests for this function.
- Verify that output works with `eggsec report convert`.

**Success Criteria**:
- Conversion produces clean, useful `FindingData`.
- Tests cover main cases.

### Task 2: Add Wireless Integration Documentation

**Files to modify**:
- `docs/WIRELESS.md`
- `README.md` (minor)
- `CAPABILITIES.md` (minor)

**Actions**:
- Add a new section titled **"Integration with Reporting Pipeline"** in `docs/WIRELESS.md`.
- Explain how to use `--json` output with `eggsec report convert`.
- Show when to use native output vs converted output.
- Add 1-2 command examples.
- Update `README.md` and `CAPABILITIES.md` if the command reference needs a small integration note.

**Success Criteria**:
- A user can easily understand how to integrate wireless output with the reporting system.

### Task 3: Improve Mobile Reporting Bridge

**Files to modify**:
- `crates/eggsec/src/mobile/mod.rs`

**Actions**:
- Review `to_scan_report_data()` in `mobile/mod.rs`.
- Improve field population for both Android and iOS findings.
- Use clearer categories (`mobile-android`, `mobile-ios`, or more specific sub-categories).
- Include good evidence (e.g. permission names, manifest keys, secret patterns).
- Add unit tests for the conversion function.
- Test the flow with `eggsec report convert`.

**Success Criteria**:
- Conversion is reliable and produces high-quality findings.
- Tests exist and pass.

### Task 4: Add Mobile Integration Documentation

**Files to modify**:
- `docs/MOBILE.md`
- `README.md` (minor)
- `CAPABILITIES.md` (minor)

**Actions**:
- Add a section titled **"Integration with Reporting Pipeline"** in `docs/MOBILE.md`.
- Explain native `MobileScanReport` vs converted `ScanReportData`.
- Add usage examples.
- Update main docs references as needed.

**Success Criteria**:
- Clear guidance exists for using mobile output with reporting tools.

### Task 5: Clarify Credential Access Documentation

**Files to modify**:
- `docs/AUTH_LAB.md`
- `README.md`
- `CAPABILITIES.md`

**Actions**:
- Clearly explain that `auth-test` produces **local** `AuthTestReport` / `AuthFinding` only.
- State that it does **not** convert to canonical `FindingData` / `ScanReportData` (by design).
- Differentiate it from `ScanProfile::Auth`.
- Update relevant sections in `README.md` and `CAPABILITIES.md`.

**Success Criteria**:
- Readers understand why `auth-test` behaves differently from other modules.

### Task 6: Cross-Cutting Consistency

**Actions**:
- Do a final pass to ensure consistent language across `WIRELESS.md`, `MOBILE.md`, and `AUTH_LAB.md` regarding output models.
- Consider adding a short shared note (in `docs/` or `architecture/`) explaining the different output approaches used by these modules.

---

## 5. Recommended Execution Order

1. Task 1 (Wireless bridge)
2. Task 2 (Wireless docs)
3. Task 3 (Mobile bridge)
4. Task 4 (Mobile docs)
5. Task 5 (auth-test docs)
6. Task 6 (consistency)

---

## 6. Success Criteria (Overall)

- Wireless and Mobile have improved, tested reporting bridges.
- Clear documentation exists explaining how to integrate each module with the reporting system.
- `auth-test` design is well documented and not confused with other modules.
- The integration work feels complete for this phase.

---

**This plan is scoped to be completable by a smaller model while delivering meaningful integration progress.**