# Integration Priorities 1 & 2 Plan

**Scope**: Wireless, Mobile, and Credential Access (`auth-test`)  
**Priorities Covered**: 1 (Reporting Bridge) + 2 (Documentation & Discoverability)  
**Date**: 2026-06-11

---

## 1. Overview

This plan focuses on the first two priorities from the integration work:

- **Priority 1**: Strengthen the optional reporting bridge where it exists (wireless + mobile).
- **Priority 2**: Improve documentation and discoverability for all three modules.

**Important Note on Credential Access**:
`auth-test` is **intentionally designed** as a standalone defense-lab CLI that emits local `AuthTestReport` / `AuthFinding` types only. No `to_scan_report_data()` conversion or canonical `FindingData` mapping is planned or desired (see `architecture/auth.md`). Integration work for `auth-test` is therefore limited to documentation and clarity.

---

## 2. Per-Module Breakdown

### 2.1 Credential Access (`auth-test`)

**Integration Approach**: Documentation + Clarity only (no code changes to findings model).

**Tasks**:

- Review and improve `docs/AUTH_LAB.md` to clearly explain:
  - Why `auth-test` uses local findings only.
  - How its output differs from normal pipeline scans.
  - When to use `auth-test` vs. `ScanProfile::Auth`.
- Add or expand a section in `README.md` (Lab Defense Commands) that reinforces its standalone nature.
- Update `architecture/auth.md` if needed to ensure the "local findings only" decision is well documented.
- Add a short note in `CAPABILITIES.md` under the `auth-test` entry clarifying output model.

**Success Criteria**:
- Anyone reading the docs understands that `auth-test` produces local `Auth*` types and is not converted to the canonical reporting format.

### 2.2 Wireless

**Integration Approach**: Strengthen existing optional `to_scan_report_data()` bridge + documentation.

**Tasks – Reporting Bridge (Priority 1)**:

- Review `crates/eggsec/src/wireless/mod.rs` → `to_scan_report_data()` function:
  - Ensure all important fields are populated (severity, category, evidence, remediation).
  - Use consistent category naming (e.g. `wireless-rogue`, `wireless-security`).
  - Handle edge cases gracefully (empty results, repeated scans).
- Add or improve unit tests for the conversion function.
- Verify that `eggsec wireless ... --json` output works cleanly with `eggsec report convert`.

**Tasks – Documentation (Priority 2)**:

- Add a new section in `docs/WIRELESS.md` titled **"Integration with Reporting & Pipelines"** covering:
  - How to use the optional bridge with `eggsec report`.
  - When to use native wireless output vs. converted output.
  - Example commands.
- Update `README.md` Quick Command Reference or Lab Defense section with a short integration note.
- Ensure `CAPABILITIES.md` mentions the optional reporting bridge for wireless.

**Success Criteria**:
- Wireless JSON output can be reliably converted and used in reports.
- Documentation clearly explains the optional integration point.

### 2.3 Mobile

**Integration Approach**: Strengthen existing optional `to_scan_report_data()` bridge + documentation.

**Tasks – Reporting Bridge (Priority 1)**:

- Review `crates/eggsec/src/mobile/mod.rs` → `to_scan_report_data()` function:
  - Ensure fields are well populated for both Android and iOS findings.
  - Use clear categories (`mobile-android`, `mobile-ios`, or more specific subcategories).
  - Include useful evidence (e.g., permission name, manifest key, secret pattern).
- Add or improve unit tests for the conversion.
- Test round-trip: `eggsec mobile <file> --json` → `eggsec report convert`.

**Tasks – Documentation (Priority 2)**:

- Add a new section in `docs/MOBILE.md` titled **"Integration with Reporting Pipeline"** covering:
  - How to use `--json` output with the reporting tools.
  - Differences between native `MobileScanReport` and converted `ScanReportData`.
  - Example usage.
- Update `README.md` and `CAPABILITIES.md` to mention the optional bridge.
- Add a short note in `architecture/mobile.md` about the bridge design.

**Success Criteria**:
- Mobile JSON output works reliably with reporting tools.
- Documentation clearly explains the integration option.

---

## 3. Cross-Cutting Tasks

- Create or update a short shared section in `docs/` (or `architecture/`) explaining the different output models:
  - Pipeline scans → full `ScanReportData`
  - Wireless / Mobile → optional bridge
  - `auth-test` → local findings only
- Ensure consistent language across `WIRELESS.md`, `MOBILE.md`, `AUTH_LAB.md`, and `README.md`.

---

## 4. Recommended Work Order

1. Start with **Credential Access documentation** (easiest win, clarifies intent).
2. Do **Wireless reporting bridge review + tests**.
3. Do **Wireless documentation** updates.
4. Do **Mobile reporting bridge review + tests**.
5. Do **Mobile documentation** updates.
6. Finish with cross-cutting documentation consistency pass.

---

## 5. Out of Scope

- Any changes to `auth-test` findings model or adding conversion.
- Implementing full pipeline stages for wireless or mobile.
- Creating new `ScanProfile` variants.
- Work on Priority 3 (pipeline stage design note) or Priority 4.

---

## 6. Success Criteria (Overall)

- All three modules have clear, accurate documentation about their output and integration model.
- Wireless and mobile have robust, tested optional reporting bridges.
- Users can easily understand when and how to integrate each module with the rest of Eggsec.
- No changes are made to `auth-test` that contradict its standalone local-findings design.

---

**This plan provides focused, actionable work for the highest-value integration items across all three new modules.**