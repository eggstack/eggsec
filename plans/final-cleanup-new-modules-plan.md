# Final Cleanup Plan – New Modules (Wireless, Mobile, Credential Access)

**Purpose**: Close out remaining documentation, TUI, and minor integration details for the new defense-lab modules.  
**Date**: 2026-06-11

---

## 1. Executive Summary

Significant work has been completed:
- Reporting bridges for Wireless and Mobile are solid with good test coverage.
- Dedicated "Integration with Reporting Pipeline" sections have been added to `WIRELESS.md` and `MOBILE.md`.
- Wireless TUI (`WirelessTab`) is fully implemented and wired via `TabSpec`.
- `auth-test` remains intentionally local-only (as designed).

This plan addresses the final remaining cleanup items to bring everything to a polished, consistent state.

---

## 2. Current State Assessment

**Completed**:
- Wireless and Mobile reporting bridges + tests
- Integration documentation sections in WIRELESS.md and MOBILE.md
- Wireless TUI tab + TabSpec registration + enforcement wiring
- Agentic alignment via central `EnforcementContext`

**Remaining / Minor**:
- Polish `AUTH_LAB.md` for clearer explanation of local-only `auth-test` model
- Minor consistency updates in `README.md` and `CAPABILITIES.md`
- Ensure architecture docs have up-to-date cross-references
- Add a short shared note on output models (if not already present in USAGE.md)
- Verify no broken references in plan files or architecture docs

---

## 3. Cleanup Tasks

### Task 1: Polish Credential Access Documentation

**Files**:
- `docs/AUTH_LAB.md`
- `README.md` (Lab Defense section)
- `CAPABILITIES.md`

**Actions**:
- Strengthen language explaining that `auth-test` intentionally produces local `AuthTestReport` / `AuthFinding` only.
- Clearly state there is no conversion to canonical `FindingData` / `ScanReportData`.
- Differentiate it from `ScanProfile::Auth`.
- Add a concise note in the Lab Defense Commands table in `README.md` and `CAPABILITIES.md`.

**Success Criteria**:
- Readers can easily understand the intentional design difference.

### Task 2: Minor Documentation Consistency

**Files**:
- `README.md`
- `CAPABILITIES.md`
- `docs/USAGE.md` (if applicable)

**Actions**:
- Ensure the Lab Defense Commands tables mention the output model for each new module where relevant.
- Verify that references to the new plans and architecture docs are consistent.
- Add or confirm a short shared paragraph explaining the different output models used by standalone defense-lab modules (native types + optional bridge vs local-only).

### Task 3: Architecture & Cross-Reference Cleanup

**Files**:
- `architecture/wireless.md`
- `architecture/mobile.md`
- `architecture/auth.md`
- `architecture/defense_lab.md`

**Actions**:
- Ensure these files have up-to-date notes on TUI status, reporting bridge, and design decisions (standalone vs pipeline).
- Add references to the final cleanup plan and resolution notes in the advanced integration plan.

### Task 4: Plan File Hygiene

**Actions**:
- Review the main integration and advanced plans to ensure they have clear resolution / completion notes at the top (similar to what was done for the TUI/MCP plan).
- Archive or mark as superseded any very old credential-access plans if not already done.

### Task 5: Final Verification

**Actions**:
- Run `cargo check` and relevant tests with `--features wireless,mobile`.
- Spot-check that TUI builds with wireless feature.
- Verify no obvious broken links or outdated references in the new documentation sections.

---

## 4. Recommended Order

1. Task 1 (auth-test documentation polish) – Quick win
2. Task 2 (minor consistency in main docs)
3. Task 3 (architecture cross-references)
4. Task 4 (plan file hygiene)
5. Task 5 (final verification)

---

## 5. Success Criteria

- All documentation is clear, consistent, and up-to-date regarding the new modules.
- TUI for Wireless is properly documented and discoverable.
- No major gaps remain in explanations of output models or integration points.
- The new modules feel polished and well-integrated from a user/documentation perspective.

---

**This is the final cleanup plan to close out the integration work on Wireless, Mobile, and Credential Access.**