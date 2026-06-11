# Wireless TUI + MCP + Agentic Integration Handoff Plan

**Module**: Wireless  
**Focus**: TUI, MCP, and Agentic Workflow Integration  
**Date**: 2026-06-11

---

## 1. Goal

Bring Wireless from a strong standalone command into deeper integration with:
- The TUI
- The MCP server
- Agentic / strict execution profiles (`McpStrict`, `AgentStrict`, `CiStrict`)

This follows the existing architectural patterns used by other modules in Eggsec.

---

## 2. Current State

- Wireless has good CLI + handler integration.
- It has an optional reporting bridge.
- No TUI tab exists yet.
- No MCP tool exposure.
- Behavior under strict agentic profiles has not been explicitly aligned.

---

## 3. Tasks

### Task 1: TUI Integration

**Goal**: Add Wireless support in the TUI following existing patterns.

**Actions**:
- Explore the TUI architecture in `eggsec-tui` (tabs, views, command dispatch).
- Create or extend a tab for Wireless (e.g. `WirelessTab` or integrate into a Network/Defense tab).
- Support core flows:
  - Interface selection
  - Dry-run vs real scan toggle
  - Single scan + repeated scan mode
  - Results display (including rogue candidates)
- Ensure the TUI calls through `CommandContext` and respects `EnforcementContext`.
- Handle root / `CAP_NET_ADMIN` requirement gracefully in the UI.

**Success Criteria**:
- Wireless is usable from the TUI with reasonable UX.
- Policy and scope enforcement is respected.

### Task 2: MCP Integration

**Goal**: Expose Wireless as an MCP tool with proper scoping.

**Actions**:
- Add Wireless to the MCP tool registry / router.
- Define a proper tool schema (arguments: interface, repeat, dry_run, known_good, detect_suspicious, etc.).
- Implement the handler that calls the existing `run_cli` logic or a shared core function.
- Ensure `McpStrict` behavior:
  - Requires explicit scope manifest
  - Respects `required_features: ["wireless"]`
  - Uses `SafeActive` risk tier
- Prefer / default to dry-run mode in automated contexts.

**Success Criteria**:
- Wireless is callable via MCP.
- Strict profiles correctly gate access and require proper scope.

### Task 3: Agentic Workflow Alignment

**Goal**: Ensure consistent and safe behavior across execution profiles.

**Actions**:
- Verify the handler properly uses `ctx.evaluate_and_enforce_operation()` with a well-defined `OperationDescriptor`.
- Document expected behavior for:
  - `ManualPermissive` (current default)
  - `McpStrict` / `AgentStrict` / `CiStrict`
- Confirm that in strict profiles, wireless requires an explicit `LoadedScope` manifest.
- Add notes in `architecture/wireless.md` or `architecture/agent.md` about agentic usage.
- Consider adding wireless to capability matrices used by MCP/Agent profiles.

**Success Criteria**:
- Wireless behaves safely and predictably in both manual and agentic/strict contexts.

### Task 4: General Polish (Optional but Recommended)

- Improve error messages when `iwlist` or privileges are missing.
- Consider a cleaner JSON summary mode for repeated scans.
- Review the reporting bridge one more time for any gaps found during TUI/MCP work.

---

## 4. Recommended Phased Approach

1. **Task 3** (Agentic alignment) – Foundational for safety.
2. **Task 2** (MCP integration)
3. **Task 1** (TUI integration)
4. **Task 4** (polish)

---

## 5. Success Criteria (Overall)

- Wireless has a functional TUI surface.
- Wireless is available via MCP with correct strict-profile behavior.
- Agentic and manual workflows are consistent and safe.
- The passive/defense-lab nature of wireless is preserved.

---

**This plan focuses on deeper integration surfaces while following Eggsec’s existing architectural patterns for scoping and enforcement.**