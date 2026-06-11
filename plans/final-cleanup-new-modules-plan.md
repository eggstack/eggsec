# Final Cleanup Plan – New Modules (Wireless, Mobile, Credential Access)

**Purpose**: Close out remaining documentation, TUI, and minor integration details for the new defense-lab modules.  
**Date**: 2026-06-11

---

## Resolution / Post-execution status (2026-06-11)

**This plan has been executed and closed.** All "Remaining / Minor" items from the Current State Assessment were addressed (or confirmed already complete from prior close-out work):

- **Task 1 (Polish Credential Access Documentation)**: Complete. `docs/AUTH_LAB.md` has a dedicated "Output Model (Local Findings Only)" section (lines 56–71) clearly stating local `AuthTestReport`/`AuthFinding` only, no `to_scan_report_data` / `ScanReportData` conversion/bridge, direct emit, and explicit differentiation from `ScanProfile::Auth`. Strengthened language in README.md and CAPABILITIES.md Lab Defense tables + capability intro. See also `architecture/auth.md` (adopted model + superseded plans refs).
- **Task 2 (Minor Documentation Consistency)**: Complete. Lab Defense tables in README, CAPABILITIES, and related docs now mention the output model for each new module (auth-test: local-only/no-bridge; wireless/mobile: native types + optional `to_scan_report_data` + CLI auto-bridge). The canonical short shared "Output Models (standalone defense-lab surfaces vs. pipeline)" paragraph lives in `docs/USAGE.md` (Report Management → Convert Reports) and is cross-referenced from `architecture/output.md`, `cli_commands.md`, `defense_lab.md`, per-module docs, AGENTS.md, skills, CAPABILITIES, and README. References to new plans/architecture docs are consistent. No gaps in output model explanations.
- **Task 3 (Architecture & Cross-Reference Cleanup)**: Complete. `architecture/wireless.md` (MCP/Agentic header + "post wireless-tui-mcp-agentic-handoff-plan 2026-06-11; see plan resolution note"; Integration section with design decision), `architecture/mobile.md` (Phase 1 close 2026-06-11 note + Integration), `architecture/auth.md` (adopted model + local-only rationale + superseded credential plans), and `architecture/defense_lab.md` (consolidated "standalone defense-lab surfaces" pattern para + USAGE pointer) are all up-to-date with TUI status, reporting bridge, standalone vs. pipeline decisions, and cross-refs to final cleanup + resolution notes. `overview.md`, `cli_commands.md` (Special Cases), and `output.md` (standalone commands note + canonical USAGE pointer) also current.
- **Task 4 (Plan File Hygiene)**: Complete. Resolution / completion notes added at top of plans lacking them (modeled on `wireless-tui-mcp-agentic-handoff-plan.md`): this file, `plans/integration-work-plan.md`, `plans/wireless-advanced-integration-plan.md`, and `plans/wireless-final-closeout-plan.md`. All historical credential-access plans (`plans/credential-access-*.md`) already carried strong "Historical / Superseded / Completed (2026-06-11)" + resolution notes at top (see `credential-access-implementation-plan.md` root note, `credential-access-implementation-next-steps.md`, `credential-access-next-steps.md`, `credential-access-completion-plan.md`). Broader integration plans reference the close-out confirmations (`new-modules-integration-and-closeout-plan.md` has "Close-Out Confirmation (2026-06-11)"; `mobile-final-closeout-plan.md` and `wireless-*-closeout*` have their confirmations). No files were deleted (historical value preserved); no archiving subdir needed. Cross-refs updated to point to final-cleanup + new-modules close-out + wireless-tui handoff resolution notes.
- **Task 5 (Final Verification)**: See below. All checks/tests passed; no broken links or outdated references found in the new sections.

**See also** (canonical sources for the adopted model):
- `plans/new-modules-integration-and-closeout-plan.md` (broader close-out confirmation + verification)
- `plans/wireless-tui-mcp-agentic-handoff-plan.md` (TUI complete; MCP/agent exposure intentionally absent — resolution note at top)
- `plans/mobile-final-closeout-plan.md` (Phase 1 close confirmation)
- `architecture/{wireless,mobile,auth,cli_commands,defense_lab,output}.md`, `docs/USAGE.md` (Output Models block), `docs/AUTH_LAB.md`, `docs/WIRELESS.md` + `docs/MOBILE.md` (Integration sections), CAPABILITIES.md / README (Lab Defense tables), AGENTS.md (standalone defense-lab surfaces + USAGE pointer), and the per-skill notes in `.opencode/skills/`.

**Recommended verification commands** (executed clean):
```bash
cargo check -p eggsec --features wireless,mobile
cargo check -p eggsec-tui --features wireless
cargo test --lib -p eggsec --features wireless,mobile
cargo clippy --lib -p eggsec --features wireless,mobile
# Spot TUI build + no broken links in new docs sections
```

All documentation is now clear, consistent, and up-to-date. The new modules (wireless with TUI, mobile, auth-test local-only) feel polished. This is the final cleanup plan; it is now closed. Retained for historical reference. No code changes required.

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