# Wireless Active Attacks Loadout: TUI Integration & Final Polish Plan

**Date**: 2026-06-11  
**Status**: Draft — Ready for Handoff  
**Related Plans**:
- `plans/wireless-active-attacks-loadout-design-plan.md` (original design)
- `plans/wireless-active-loadout-cli-integration-plan.md` (previous integration focus)

---

## Executive Summary

Significant progress has been made on the active wireless attacks loadout:

- Core deauth/disassoc implementation is functional
- CLI subcommands and handler integration are working
- Documentation in `docs/WIRELESS.md` has been substantially updated
- Reporting bridge (`to_active_scan_report_data`) has been implemented

**This plan focuses on the remaining high-value work** to bring the feature to a polished, shippable state for Phase 1:

1. **Full TUI integration** for active attacks (currently the largest missing piece)
2. Wiring the reporting bridge into `eggsec report convert`
3. Robustness, testing, and final polish items

**Goal**: Make active deauth usable and consistent across CLI, TUI, and reporting, while documenting the current state clearly for future phases.

---

## Current State (as of latest)

| Component                    | Status          | Notes |
|-----------------------------|-----------------|-------|
| Core deauth logic           | Good            | Frame building + injection implemented |
| CLI (subcommands + args)    | Good            | `deauth` subcommand works |
| Handler + Policy gating     | Good            | `Intrusive` risk + `--allow-active-wireless` |
| Documentation               | Good            | Major "Active Attacks (Phase 1)" section added |
| Reporting Bridge            | Good            | `to_active_scan_report_data` exists + tested |
| **TUI**                     | **Missing**     | No active attack support yet |
| Report Convert wiring       | Partial         | Bridge exists but not fully wired into convert handler |
| Integration tests           | Partial         | Unit tests exist; end-to-end handler tests limited |
| Advanced policy (lab manifest) | Not started  | Future improvement |

---

## Prioritized Remaining Work

### P0 – TUI Integration (Highest Impact)

Make active attacks usable from the TUI, consistent with the recent TUI architecture improvements (UiAction, preflight, policy indicators, confirmation overlays).

**Key Tasks**:

1. **Extend Wireless Tab**
   - Add mode toggle or dedicated section for "Active Attacks" (behind `wireless-advanced` feature).
   - Create input form for deauth parameters (BSSID, optional Client, Count, Reason Code, Broadcast toggle).
   - Add rate limit / max frames controls with sensible defaults.

2. **Policy & Safety Integration**
   - Reuse existing `PendingPolicyConfirmation` / preflight flow.
   - Show clear high-risk warning and require explicit confirmation (mirrors `--allow-active-wireless`).
   - Display current enforcement posture (feature enabled? policy level?).

3. **Execution & Results**
   - Trigger `handle_deauth` (or equivalent library call) via a background worker.
   - Display live progress (frames sent / rate).
   - Show final `ActiveWirelessAttackResult` in a results panel (findings, recommendations).
   - Support export to JSON / file.

4. **UX Consistency**
   - Use semantic risk styling (High risk = appropriate color/token).
   - Add "Dry Run" toggle with clear visual distinction.
   - Handle feature-not-enabled state gracefully (show build instruction).

**Suggested File Changes**:
- `crates/eggsec-tui/src/tabs/wireless.rs` (main tab logic)
- Possibly new worker in `crates/eggsec-tui/src/workers/` or extend existing security worker
- Update `TabSpec` / descriptor for active actions

### P1 – Reporting Pipeline Completion

1. Wire `to_active_scan_report_data` into `eggsec report convert` handler so native JSON from `deauth` commands is automatically bridged when `wireless-advanced` is enabled.
2. Ensure `wireless-active-deauth` (and future `wireless-active-*`) categories appear correctly in SARIF, JUnit, HTML, etc.

**File**: `crates/eggsec/src/commands/handlers/report.rs` (or output convert logic)

### P2 – Robustness & Testing

- Improve error handling in `inject_frames` (better classification of socket errors, partial success reporting).
- Add integration-style tests that exercise the full path: CLI args → handler → active module (with mocked injection where possible).
- Add TUI-specific tests for the new active UI components.
- Consider adding a `--dry-run` only test mode that doesn't require monitor mode.

### P3 – Future / Nice-to-Have

- Basic lab manifest support (authorized BSSIDs/channels for active operations).
- Placeholder or stub for Phase 2 (handshake capture).
- Update architecture diagrams if the module structure has stabilized.
- Consider exposing limited active capabilities to MCP/agent in a future controlled opt-in (current design decision: keep absent).

---

## Detailed TUI Wiring Recommendations

The TUI should follow the existing patterns established in the recent architecture updates:

- Use `UiAction` for launching active attacks.
- Leverage `OverlayController` for confirmation dialogs.
- Show policy preflight status before execution.
- Use the global task strip for long-running injection.
- Keep active attacks clearly separated from passive scanning (different risk profile).

**Minimal Viable TUI for Phase 1**:
- A collapsible or tabbed "Active" section inside the Wireless tab.
- Form-based input + big "Dry Run" and "Execute" buttons.
- Confirmation overlay that explicitly states this will transmit frames.
- Results view showing findings + recommendations.

Full interactive attack control can evolve in later iterations.

---

## Handoff Checklist

- [ ] Implement TUI active attack section (inputs + confirmation + results)
- [ ] Wire `to_active_scan_report_data` into report convert handler
- [ ] Add integration tests for deauth handler path
- [ ] Improve error handling / robustness in frame injection
- [ ] Update any remaining references in `architecture/wireless.md` if needed
- [ ] Add TUI note in `docs/WIRELESS.md` (even if minimal)
- [ ] Review and clean up any TODOs left in `active/` module
- [ ] Decide on lab manifest feature (include in this plan or defer?)
- [ ] Create issue for Phase 2 (handshake capture)

---

## Suggested Implementation Order

1. **TUI integration** (biggest remaining user-facing gap)
2. **Report convert wiring** (completes the reporting story)
3. **Testing + robustness polish**
4. **Documentation updates** for TUI and final state

This order delivers the most value quickly.

---

## Open Questions

1. How deep should the initial TUI integration go? (Minimal form + confirmation vs. more advanced live monitoring?)
2. Should we add a dedicated "Active Attacks" tab or keep everything inside the existing Wireless tab for now?
3. Do we want to support monitor interface auto-detection/creation in the TUI, or require users to provide `--monitor-iface`?
4. Priority for Phase 2 primitives: Handshake capture with deauth trigger, or basic flooding first?

---

**End of Plan**