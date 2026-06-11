# Integration Documentation Handoff Plan

**Focus**: Documentation improvements for Wireless, Mobile, and Credential Access  
**Date**: 2026-06-11

---

## 1. Goal

Add clear, user-friendly documentation that explains how the new modules integrate with Eggsec’s reporting system and clarifies their different output models.

This is the highest-remaining documentation gap after the reporting bridge improvements.

---

## 2. Scope

**In Scope**:
- Add "Integration with Reporting Pipeline" sections to `WIRELESS.md` and `MOBILE.md`.
- Improve clarity in `AUTH_LAB.md` about `auth-test` being local-only.
- Minor updates to `README.md` and `CAPABILITIES.md` for consistency.
- Ensure language is consistent across docs.

**Out of Scope**:
- Code changes to the bridges themselves.
- TUI, MCP, or pipeline stage work.

---

## 3. Tasks

### Task 1: Wireless Documentation

**Files**:
- `docs/WIRELESS.md` (main)
- `README.md` (minor)
- `CAPABILITIES.md` (minor)

**Actions**:
- Add a new top-level section: **"Integration with Reporting Pipeline"**
- Explain:
  - Native `WirelessScanResult` vs converted `ScanReportData`
  - When to use `--json` + `eggsec report convert`
  - Benefits of the bridge (SARIF, JUnit, unified reports)
- Add 1-2 practical examples.
- Mention that rogue detection and repeated scan summaries are preserved in native output.

**Success Criteria**:
- A user can confidently use wireless output with the reporting tools after reading the section.

### Task 2: Mobile Documentation

**Files**:
- `docs/MOBILE.md` (main)
- `README.md` (minor)
- `CAPABILITIES.md` (minor)

**Actions**:
- Add a new section: **"Integration with Reporting Pipeline"**
- Explain:
  - Native `MobileScanReport` vs `ScanReportData`
  - Platform-specific categories in the bridge
  - How to convert APK/IPA analysis results
- Add examples for both Android and iOS.

**Success Criteria**:
- Clear guidance exists for integrating mobile static analysis output.

### Task 3: Credential Access Documentation

**Files**:
- `docs/AUTH_LAB.md` (primary)
- `README.md`
- `CAPABILITIES.md`

**Actions**:
- Strengthen language around `auth-test`:
  - Explicitly state it produces **local** `AuthTestReport` / `AuthFinding` only.
  - Clearly say there is **no** conversion to canonical `FindingData` / `ScanReportData` (by design).
  - Differentiate it from `ScanProfile::Auth` (which is pipeline-based JWT/OAuth/IDOR testing).
- Add a short note in the Lab Defense Commands section of `README.md`.

**Success Criteria**:
- Readers understand why `auth-test` behaves differently and when to use it.

### Task 4: Consistency Pass

**Actions**:
- Review the new sections for consistent terminology across all three modules.
- Consider adding a short shared note (perhaps in `docs/USAGE.md` or a new small file) explaining the different output models used by these defense-lab modules.

---

## 4. Recommended Order

1. Task 1 (Wireless docs)
2. Task 2 (Mobile docs)
3. Task 3 (Credential Access docs)
4. Task 4 (consistency)

---

## 5. Success Criteria (Overall)

- All three modules have clear documentation about their integration points and output models.
- Users can easily understand how to use Wireless and Mobile with `eggsec report`.
- `auth-test` design is well explained and not confused with other modules.

---

**This is a focused, documentation-only handoff plan.**