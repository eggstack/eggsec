# Integration Work Plan – New Defense Modules

**Focus**: Wireless + Mobile (auth-test remains standalone by design)  
**Date**: 2026-06-11  
**Goal**: Improve integration of wireless and mobile into Eggsec’s reporting and (optionally) pipeline systems while keeping their standalone nature intact.

---

## 1. Context & Guiding Principles

- `auth-test` is **intentionally standalone** with local `AuthTestReport`/`AuthFinding` only. No `to_scan_report_data()` conversion or pipeline integration is planned or desired (per `architecture/auth.md`).
- `wireless` and `mobile` already have **optional** `to_scan_report_data()` bridges. The goal is to make these bridges reliable and well-documented.
- We want **lightweight, opt-in** integration — not forced blending into the main pipeline.
- Keep changes minimal, consistent with existing patterns, and low-risk.

---

## 2. Current State of Integration

| Module   | `to_scan_report_data()` | Used in `eggsec report`? | Pipeline Stage? | Documentation of Bridge |
|----------|--------------------------|---------------------------|-----------------|--------------------------|
| wireless | Yes (optional)           | Works if used             | No              | Partial                  |
| mobile   | Yes (optional)           | Works if used             | No              | Partial                  |
| auth-test| No (by design)           | N/A                       | No              | N/A (local only)         |

---

## 3. Integration Work Priorities

### Priority 1: Strengthen Reporting Bridge (Wireless + Mobile)

**Goal**: Make the optional `to_scan_report_data()` conversion robust and easy to use.

**Tasks**:

1. Review and improve `to_scan_report_data()` implementations in:
   - `crates/eggsec/src/wireless/mod.rs`
   - `crates/eggsec/src/mobile/mod.rs`
2. Ensure all key fields are populated:
   - Severity, title, description, category, location, evidence, remediation
   - Proper categorization (`wireless-*`, `mobile-android`, `mobile-ios`)
3. Add or improve unit tests for the conversion functions.
4. Update `docs/WIRELESS.md` and `docs/MOBILE.md` with clear sections on:
   - How to use `--json` output with `eggsec report convert`
   - When the bridge is useful vs. when to use the native report types
5. Add a short example in `README.md` or `docs/USAGE.md` showing the flow.

**Success Criteria**:
- `eggsec wireless <iface> --json -o out.json` can be fed into `eggsec report convert` without issues.
- Same for mobile.
- Documentation clearly explains the optional nature of the bridge.

### Priority 2: Documentation & Discoverability

**Tasks**:

- Add a short “Integration with Reporting Pipeline” section to both `WIRELESS.md` and `MOBILE.md`.
- Update `CAPABILITIES.md` (if needed) to mention the optional reporting bridge.
- Ensure `architecture/wireless.md` and `architecture/mobile.md` document the bridge design decision.

### Priority 3: Lightweight Pipeline Stage Exploration (Future / Optional)

**Goal**: Explore whether thin, optional stages make sense for wireless and mobile.

**Approach**:
- Do **not** implement full stages yet.
- First, create a short design note (in `architecture/` or as an ADR) evaluating:
  - Pros/cons of adding `WirelessAnalysis` and `MobileStatic` stages.
  - How they would interact with existing `ScanProfile` system.
  - Whether they should be feature-gated or always available when the module is built.

**Recommended Output**:
- A short design document (`architecture/proposed-wireless-mobile-stages.md` or similar).
- Decision: Proceed / Defer / Reject full stage implementation.

**Note**: This is explicitly lower priority. We can ship strong standalone + reporting bridge support without pipeline stages.

### Priority 4: Polish & Consistency

**Tasks**:

- Ensure consistent severity mapping and category naming between wireless and mobile.
- Review error handling and edge cases in the bridge functions.
- Make sure `--json` output from both commands is stable and well-documented.

---

## 4. Recommended Execution Order

1. **Priority 1** – Strengthen and document the reporting bridges (highest value).
2. **Priority 2** – Improve documentation and examples.
3. **Priority 4** – Polish and consistency pass.
4. **Priority 3** – Only if we decide pipeline stages are worth the complexity (create design note first).

---

## 5. Out of Scope

- Any changes to `auth-test` regarding findings conversion or pipeline integration.
- Creating full `WirelessRecon` / `MobileStatic` pipeline stages in this round (design note only).
- Adding new `ScanProfile` variants (e.g. `wireless-defense`, `mobile-static`).

---

## 6. Success Criteria

- Wireless and mobile `--json` output works reliably with the existing reporting tools via the optional bridge.
- Documentation clearly explains how (and when) to use the integration points.
- The integration feels clean, optional, and non-intrusive.
- We have a clear decision record on whether to pursue pipeline stages in the future.

---

**This plan focuses on practical, high-value integration work without over-engineering or changing the fundamental standalone nature of these modules.**