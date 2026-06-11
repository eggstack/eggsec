# Wireless Feature - Final Close-Out / Completion Plan

**Status**: Final Close-Out  
**Date**: 2026-06-11  
**Goal**: Complete the last items so wireless can be considered finished as a clean standalone capability.

---

## 1. Executive Summary

Wireless has reached a strong standalone state. The core functionality, CLI experience, rogue detection with known-good support, repeated scan/change detection, and `docs/WIRELESS.md` are all in good shape.

The remaining work is narrow and mostly documentation-focused. Completing these items will allow us to mark the standalone phase as complete.

---

## 2. Current State

**Done**:
- Passive scanning with WPS, hidden SSID, transition mode, and weak signal detection.
- Rogue/Evil Twin heuristic with known-good allowlist suppression.
- Repeated scans + change detection + temporal summaries.
- Dry-run mode for planning/CI.
- Good CLI flags and help text.
- Solid `docs/WIRELESS.md` with usage, workflows, and best practices.

**Remaining**:
- Project-level documentation updates (`CAPABILITIES.md`, `README.md`, `SAFETY.md`).
- Minor final polish on rogue output UX and documentation consistency.

---

## 3. Final Tasks

### Task 1: Update Main Project Documentation (Highest Priority)

**Actions**:
- Update `docs/CAPABILITIES.md`:
  - Expand the wireless entry to reflect current capabilities (repeated scans, rogue detection, known-good, dry-run, etc.).
- Update `README.md`:
  - Add `eggsec wireless` to the command reference.
  - Mention it briefly in the lab/defense validation section.
- Update `docs/SAFETY.md`:
  - Add or expand the wireless section with root/CAP_NET_ADMIN requirements and passive-only framing.

### Task 2: Minor Rogue UX & Documentation Polish

**Actions**:
- Review the current default rogue output behavior in `run_cli()`.
  - Consider whether showing a short summary + hint by default is still the best UX, or if a one-line list of candidate SSIDs would be clearer.
- Ensure `docs/WIRELESS.md` and CLI help text are perfectly aligned on rogue behavior.
- Add a short note in `WIRELESS.md` about how rogue findings integrate with `to_scan_report_data()` / reporting pipeline.

### Task 3: Quick Consistency Pass

**Actions**:
- Do a final review pass over `WIRELESS.md` for any outdated examples or missing flags.
- Ensure all new CLI flags (`--repeat`, `--dry-run`, `--known-good`, `--detect_suspicious`) are mentioned in the main help text and documentation.
- Verify that error messages in repeated scan mode are clear and user-friendly.

---

## 4. Recommended Order

1. Task 1 (Main documentation) — Highest visibility and adoption impact.
2. Task 2 (Rogue UX polish)
3. Task 3 (Consistency pass)

These tasks are small and can likely be completed in one focused session.

---

## 5. Success Criteria

- `CAPABILITIES.md`, `README.md`, and `SAFETY.md` accurately reflect the current wireless feature.
- Rogue detection UX feels clean and well-documented.
- No inconsistencies between code, help text, and `WIRELESS.md`.
- Wireless can be confidently marked as complete in its standalone form.

---

**This is the final close-out plan. Once these items are done, wireless should be in a finished standalone state ready for broader use or future pipeline integration work.**