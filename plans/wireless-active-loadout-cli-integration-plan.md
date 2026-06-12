# Wireless Active Attacks Loadout: CLI Integration & Remaining Gaps Plan

**Date**: 2026-06-11  
**Status**: Draft — Ready for Handoff  
**Related**: `plans/wireless-active-attacks-loadout-design-plan.md`, current implementation state on `main`

---

## Executive Summary

Significant foundational work has been completed on the active wireless attacks loadout:

- `wireless-advanced` feature flag added and wired into `full`.
- New `crates/eggsec/src/wireless/active/` module with data models.
- Full deauth/disassoc frame crafting + raw socket injection implemented (`attacks/deauth.rs`).
- CLI subcommand structure (`scan` + `deauth`) defined in `cli/wireless.rs`.
- Handler dispatch + policy gating implemented in `commands/handlers/wireless.rs` (with `--allow-active-wireless` requirement).

However, the feature is not yet **end-to-end usable and documented**. This plan identifies the remaining gaps and provides a focused implementation roadmap to reach a shippable state for Phase 1 (Deauth).

**Goal**: Complete CLI integration, basic documentation, and critical supporting pieces so `eggsec wireless <iface> deauth ...` works reliably under the feature gate.

---

## Current State Assessment (as of 2026-06-11)

### Completed
- Feature flag + module scaffolding
- `ActiveWirelessAttackResult`, `ActiveAttackConfig`, helper methods
- Deauth + Disassoc frame builders + Linux raw socket injection with rate limiting
- `run_deauth()` / `run_disassoc()` producing structured results
- CLI `DeauthArgs` + `WirelessSubcommand` enum
- Handler with policy enforcement (`OperationRisk::Intrusive`) and `--allow-active-wireless` gate
- Basic human + JSON output in handler

### Partially Complete / Needs Polish
- Main `wireless/mod.rs` re-exports (needs cleanup)
- Error handling and robustness in injection path
- Unit test coverage for handler integration paths

### Not Yet Started / Major Gaps
1. **Documentation** (`docs/WIRELESS.md`, architecture, README quick reference)
2. **TUI support** for active attacks
3. **Reporting bridge** (`to_scan_report_data` equivalent for active results)
4. **Other Phase 1/2 primitives** (handshake capture trigger, basic flooding)
5. **Advanced policy** (lab manifest support, richer capability checks)
6. **MCP/Agent exposure decision** (explicitly document as "absent by design")
7. **End-to-end usage examples** and safety warnings in help text

---

## Recommended Next Steps (Prioritized)

### P0 – Make Deauth Usable End-to-End (1–2 days)

1. **Polish CLI + Handler Integration**
   - Ensure `handle_deauth` properly re-uses the existing `run_cli` pattern where possible or cleanly separates concerns.
   - Improve error messages when `wireless-advanced` is not enabled.
   - Add `--help` examples that are accurate and safe.

2. **Documentation (Critical)**
   - Update `docs/WIRELESS.md` with a new "Active Attacks" section.
   - Add usage examples for `deauth` (dry-run + real with warnings).
   - Update `WIRELESS_ABOUT` text if needed for clarity.
   - Add short section in `architecture/wireless.md`.

3. **Basic Reporting Bridge**
   - Implement or stub `to_active_scan_report_data()` (or extend existing bridge) so deauth results can flow into SARIF/JUnit/HTML reports.

### P1 – Hardening & Supporting Features (3–5 days)

- Improve robustness of `inject_frames` (better error classification, partial success handling).
- Add more comprehensive tests (integration-style tests that exercise handler + active module).
- Add TUI stub (even if just showing "active attacks require CLI for now" with link to docs).
- Decide and document MCP/agent exposure policy (recommend: remain absent for active commands).

### P2 – Broader Loadout (Future)
- Implement `capture-handshake` trigger (re-uses deauth logic).
- Basic beacon/probe flood primitive.
- Lab manifest support (authorized targets file for active ops).

---

## Detailed Gap Analysis & Tasks

### 1. Documentation (Highest Priority for Usability)

**Files to update**:
- `docs/WIRELESS.md` — Add major section "Active Wireless Attacks (wireless-advanced)"
- `architecture/wireless.md` — Add subsection on active module
- `README.md` — Update wireless examples section
- Possibly `docs/SAFETY.md` — Reference active wireless risk tier

**Content needed**:
- Clear warning that active mode requires `--features wireless-advanced`
- Full example commands with `--dry-run` and real execution
- Explanation of `--allow-active-wireless` and audit logging
- Hardware requirements (monitor mode, injection support)
- Link to the original design plan

### 2. CLI & Handler Polish

Current state is functional but could be cleaner:
- Consider moving more logic from handler into `wireless/active/` module.
- Ensure consistent use of `ctx.notify_manager` and structured output.
- Make sure dry-run path always succeeds and produces valid JSON even without privileges.

### 3. Reporting Integration

Active results should eventually feed into the standard reporting pipeline.

Recommended approach:
- Add `pub fn to_scan_report_data(result: &ActiveWirelessAttackResult) -> ScanReportData` in `wireless/active/mod.rs` (or a new `convert.rs`).
- Wire it in `eggsec report convert` when `wireless-advanced` feature is present.

### 4. TUI

For v1, a minimal approach is acceptable:
- Add note in Wireless tab: "Active attacks (deauth, etc.) are currently available via CLI only."
- Or add a disabled/placeholder action that explains the requirement.

Full TUI attack controls can come in a follow-up iteration.

### 5. Policy & Safety

Current gating is good (`Intrusive` risk + explicit flag). Future improvements:
- Support for a `lab-wireless-manifest.toml` (allowed BSSIDs + channels for active ops).
- Better integration with `EnforcementContext` for automated paths (MCP/agent).

### 6. Testing

- Add tests that exercise the full path: CLI args → handler → active module (mocked injection where possible).
- Hardware lab tests for real injection (documented separately).

---

## Proposed Handoff Checklist

- [ ] Update `docs/WIRELESS.md` with active attacks section (include examples)
- [ ] Add basic reporting bridge for `ActiveWirelessAttackResult`
- [ ] Polish handler error messages and dry-run behavior
- [ ] Add minimal TUI note/placeholder for active mode
- [ ] Decide & document MCP/agent exposure for active commands
- [ ] Add integration tests for deauth handler path
- [ ] Update `architecture/wireless.md` with new module overview
- [ ] Review unsafe code in `inject_frames` for robustness
- [ ] Create follow-up issue for handshake capture primitive

---

## Open Questions for the Team

1. Should we expose `disassoc` as a separate subcommand now, or keep it internal for now?
2. What is the desired default behavior when `--allow-active-wireless` is omitted on a non-dry-run deauth? (Current: hard error — good.)
3. Priority order for next primitive after deauth: handshake capture trigger, or basic flooding?
4. Do we want to keep active wireless commands completely out of the MCP/agent tool registry permanently, or allow opt-in later?

---

## Suggested Implementation Order

1. Documentation update (biggest usability win)
2. Reporting bridge
3. Handler polish + tests
4. TUI stub
5. Policy decision on MCP/agent

This order gets the feature to a "documented and usable via CLI" state quickly.

---

**End of Plan**