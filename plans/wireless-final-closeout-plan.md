# Wireless Feature - Final Close-Out / Completion Plan

**Status**: Final Close-Out  
**Date**: 2026-06-11  
**Goal**: Complete the last items so wireless can be considered finished as a clean standalone capability.

---

## Resolution / Post-execution status (2026-06-11)

**This plan has been executed and closed.**

All items from the "Remaining" list were completed:
- Main project documentation (`CAPABILITIES.md`, `README.md`, `SAFETY.md` + cross-module consistency) updated with accurate wireless description, Lab Defense table rows, feature notes, system deps, and "standalone defense-lab" language (TUI tab under feature; MCP/agent tool exposure intentionally absent).
- Rogue UX polish: default human output summarizes candidates by count + hint ("use --detect-suspicious to show full details"); `--detect-suspicious` for full findings + recs; `--known-good` suppression documented for lab baselines (affects human/repeat UX + diffs only; bridge always includes rogue via `analyze_networks(..., None)`).
- Consistency pass across `docs/WIRELESS.md` (incl. full "Integration with Reporting Pipeline" section), architecture/wireless.md, CAPABILITIES, README, AGENTS.md, skills, and plan cross-refs. No broken/outdated references remained.

Wireless is now in a finished standalone state (CLI primary + optional TUI tab under `wireless` feature; passive-only; optional reporting bridge + CLI auto-bridge; MCP/agent exposure intentionally absent per design decision). See the broader close-out record in `plans/wireless-micro-closeout-checklist.md` and the TUI/MCP/agentic resolution in `plans/wireless-tui-mcp-agentic-handoff-plan.md` (top resolution block).

**See also**:
- `plans/new-modules-integration-and-closeout-plan.md` + `plans/final-cleanup-new-modules-plan.md`
- `plans/integration-work-plan.md`
- `architecture/wireless.md`, `architecture/defense_lab.md`, `architecture/cli_commands.md`, `docs/WIRELESS.md`, CAPABILITIES.md Lab Defense, AGENTS.md (standalone note)

**Verification** (all passed at close):
```bash
cargo check -p eggsec --features wireless
cargo test --lib -p eggsec --features wireless
cargo check -p eggsec-tui --features wireless
cargo clippy --lib -p eggsec --features wireless
```

This plan is retained for historical reference. No further changes required.

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