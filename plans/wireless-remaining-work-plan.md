# Wireless Feature - Remaining Work Plan (Final Polish)

**Status**: Close-out / Final Polish  
**Date**: 2026-06-11  
**Goal**: Finish the remaining items to reach a clean, professional standalone state.

---

## 1. Current State Snapshot

**Recently Completed**:
- Major enhancements to passive scanning (WPS, hidden SSIDs, transition mode detection).
- Significantly improved analysis and rogue/Evil Twin heuristic (including known-good suppression).
- Strong CLI improvements: repeated scans, dry-run mode, known-good file support, change detection between scans, and temporal summaries.
- New `docs/WIRELESS.md` created.
- Handler integration with policy enforcement already in place.

**Remaining Gaps**:
- Documentation across the project is still incomplete (main docs lag behind the new capabilities).
- Rogue detection UX could be refined (currently gated behind `--detect_suspicious`).
- Some final robustness and output polish opportunities remain.
- No dedicated wireless defense/regression profiles yet (can be deferred).

---

## 2. Prioritized Remaining Tasks

### Task 1: Documentation Updates (Highest Priority)

**Goal**: Make the feature easy to discover and use.

**Actions**:
- Review and improve `docs/WIRELESS.md`:
  - Add more practical examples (single scan, repeated monitoring, dry-run, known-good).
  - Clarify rogue detection behavior and when to use `--detect_suspicious`.
  - Add a "Best Practices" section for lab/defensive use.
- Update main documentation:
  - `CAPABILITIES.md` — expand the wireless entry with new capabilities.
  - `README.md` — add wireless to the command reference and lab/defense sections.
  - `SAFETY.md` — add or expand wireless-specific safety notes (root requirements, passive vs active).

### Task 2: Rogue / Suspicious Detection UX Refinement

**Goal**: Make rogue detection more usable by default without being noisy.

**Actions**:
- Consider changing the default behavior so that rogue/suspicious findings are shown by default (or at least summarized), with an option to hide details.
- Improve severity scoring for rogue candidates (security config differences already elevate to Medium — good).
- Add a short explanation in output when rogue candidates are detected.

**Files**:
- `crates/eggsec/src/wireless/mod.rs` (mainly `run_cli` and `analyze_networks` output logic)

### Task 3: Final Robustness & Polish

**Actions**:
- Improve error messages and recovery in repeated scan mode (currently continues on error — good, but could be clearer).
- Review output formatting for long repeated scans (ensure it remains readable).
- Add a short summary line even in JSON mode when using `--repeat` (optional but nice).
- Ensure all new flags (`--dry-run`, `--known-good`, `--detect_suspicious`, `--repeat`) are well documented in `--help`.

### Task 4: Optional — Wireless Defense Profile (Low Priority)

If time allows, define a simple `wireless-defense` or `wireless-regression` profile that can be used with the main `scan` command for repeatable wireless posture checks.

This can be deferred if the focus remains strictly on standalone usability for now.

---

## 3. Recommended Implementation Order

1. **Task 1 (Documentation)** — Highest impact for usability and adoption.
2. **Task 2 (Rogue UX)** — Improves the defensive value of the tool.
3. **Task 3 (Robustness & Polish)** — Makes the tool feel finished.
4. Task 4 only if desired.

---

## 4. Success Criteria

- `docs/WIRELESS.md` is comprehensive and accurate.
- Main project documentation properly reflects current wireless capabilities.
- Rogue/suspicious detection is useful and not overly hidden by default.
- The CLI feels polished and professional for repeated/monitoring use cases.
- No major rough edges in error handling or output for common workflows.
- The feature is ready to be considered "complete" in its standalone form.

---

**This plan focuses on the final 10-20% of work needed to close out wireless as a clean standalone capability.**