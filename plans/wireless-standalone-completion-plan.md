# Wireless Feature - Standalone Completion Plan

**Status**: Final Polish Phase  
**Goal**: Bring wireless to a clean, usable standalone state  
**Date**: 2026-06-11

---

## 1. Executive Summary

The wireless module has received substantial improvements and is now functionally capable for passive reconnaissance and basic security assessment. The remaining work is primarily **polish, documentation, and final robustness** to make it feel like a complete, professional standalone tool.

**Current State**:
- Good passive scanning with WPS, hidden SSID, transition mode, and weak signal detection.
- Basic rogue AP / Evil Twin heuristic exists.
- Improved analysis and recommendations.
- Handler integration with policy enforcement is in place.
- Output is functional but could be more polished.

**Target State**:
A clean, well-documented standalone wireless reconnaissance and basic assessment tool that security teams can confidently use in lab and authorized defensive contexts.

---

## 2. Definition of "Clean Usable Standalone State"

By the end of this plan, `eggsec wireless` should deliver:

- Reliable passive network discovery with useful security insights.
- Clear, actionable findings and recommendations.
- Good user experience (formatting, warnings, repeated scan support).
- Strong safety messaging and documentation.
- Basic rogue/suspicious network detection that is useful for defense validation.
- Output that works well both standalone and when fed into reporting pipelines.

---

## 3. Prioritized Tasks

### Task 1: Documentation (Highest Priority)

**Goal**: Make the feature self-documenting and easy to adopt.

**Actions**:
- Create `docs/WIRELESS.md` with:
  - Overview and use cases (defense validation / lab reconnaissance)
  - Requirements (root/CAP_NET_ADMIN, `iwlist` / `wireless-tools`, interface in managed mode)
  - Safety warnings and scope guidance
  - Example commands and output interpretation
  - Explanation of findings (WPS, rogue detection, transition mode, etc.)
  - Recommended workflows (single scan vs repeated monitoring)
- Update `README.md`, `CAPABILITIES.md`, and `SAFETY.md` to properly reference the wireless command.

### Task 2: CLI Polish & User Experience

**Goal**: Make the command pleasant and professional to use.

**Actions**:
- Improve output formatting in `run_cli()` (better tables, consistent styling, color support if feasible).
- Enhance repeated scan mode (`--repeat`):
  - Show diff between scans (new networks, security changes, signal changes).
  - Add summary of changes over time.
- Improve warning messages (make them clearer and less alarming for legitimate lab use).
- Add `--help` examples and better descriptions.

**Files**:
- `crates/eggsec/src/wireless/mod.rs` (mainly `run_cli`)
- `crates/eggsec/src/cli/wireless.rs`

### Task 3: Strengthen Rogue / Suspicious Network Detection

**Goal**: Make the existing rogue detection heuristic more useful and reliable.

**Actions**:
- Improve the current heuristic in `analyze_networks()`:
  - Better scoring for "possible rogue" (e.g., same SSID + different security = higher severity).
  - Detect known-good networks appearing with unexpected BSSIDs.
- Add an optional `--known-good` file or simple allowlist mechanism for lab use.
- Clearly label findings as "heuristic / passive detection" with appropriate severity (currently Low).

### Task 4: Final Robustness & Edge Cases

**Actions**:
- Improve error handling in `scan()` and `parse_scan_output()` (graceful handling of malformed `iwlist` output).
- Add better handling when no wireless interface is available or `iwlist` is missing.
- Consider adding a `--dry-run` / planning mode.
- Ensure JSON output is always valid and complete even on partial failures.

### Task 5: Minor Feature Enhancements (Optional)

If time allows:
- Add signal strength trend tracking across repeated scans.
- Simple channel utilization summary.
- Export to common formats (CSV) in addition to JSON.

These are nice-to-haves and can be deferred if the first four tasks are prioritized.

---

## 4. Recommended Implementation Order

1. **Task 1 (Documentation)** — Unblocks adoption and reduces support burden.
2. **Task 2 (CLI Polish)** — Improves daily usability the most.
3. **Task 3 (Rogue Detection)** — Adds defensive value.
4. **Task 4 (Robustness)** — Makes the tool feel solid.
5. Task 5 only if time remains.

---

## 5. Safety & Messaging

Wireless has unique operational considerations (root access, potential for disruption). The plan emphasizes:
- Clear, prominent warnings in both CLI and documentation.
- Consistent "lab / defensive validation" framing.
- Proper integration with existing `EnforcementContext` (already partially done).

---

## 6. Success Criteria

- New `docs/WIRELESS.md` exists and is high quality.
- `eggsec wireless` produces clean, professional output with actionable findings.
- Repeated scan mode provides useful change detection.
- Rogue/suspicious network detection is usable and well-documented.
- The tool feels complete and reliable as a standalone wireless assessment utility.
- Main documentation (`README.md`, `CAPABILITIES.md`) accurately reflects current capabilities.

---

**This plan focuses on finishing touches to reach a clean, professional standalone state before investing in deeper pipeline/profile integration.**