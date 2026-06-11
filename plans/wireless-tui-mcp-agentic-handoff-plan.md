# Wireless TUI + MCP + Agentic Integration Handoff Plan

**Module**: Wireless  
**Focus**: TUI, MCP, and Agentic Workflow Integration  
**Date**: 2026-06-11

---

## Resolution / Post-completion status (2026-06-11)

This plan has been executed and closed.

- **Task 1 (TUI Integration)**: Complete. `WirelessTab` (tabs/wireless.rs) + worker (workers/security.rs) exist and are fully wired. `TabSpec` entry (spec.rs) declares `risk_group: SafeActive`, `feature: Some("wireless")`, `operation: Some("wireless")`, `direct_launch: true`. Participates in `Tab::all()` / `visible_tab_specs()`, `TabStore`, command palette, help, navigation, export filename, preflight/status bar (via generic `build_current_operation_descriptor`), `copy_cli_equivalent`, `TaskBuilder`, and full `TabInput`/`TabRender`/`TabState` contracts. Primary target (interface) flows into descriptors. All feature gates and !feature fallbacks to dashboard are in place.

- **Task 2 (MCP Integration)**: Intentionally not implemented per final design decision (standalone defense-lab surface). Wireless is **not** a `SecurityTool`, is not registered in `create_default_registry()`, and has zero presence in `tool/protocol/mcp/policy.rs` (classify_tool_risk / required_capabilities_for_tool_call / infer_tool_category / McpProfilePolicy), MCP handlers/server, or agent dispatch. It remains invisible to `tools/list` / `tools/call`. This preserves the passive-only CLI/TUI focus (local interface target, root/CAP_NET_ADMIN realities) and mirrors the mobile + auth-test pattern. See architecture/wireless.md (MCP / Agentic / Tool Integration Status), architecture/defense_lab.md, architecture/cli_commands.md (Special Cases), AGENTS.md (standalone defense-lab surfaces note), and docs/USAGE.md (Output Models block).

- **Task 3 (Agentic Workflow Alignment)**: CLI handler complete and correct (`commands/handlers/wireless.rs` uses `CommandContext::evaluate_and_enforce_operation` with `OperationDescriptor` for `operation:"wireless"`, `risk:SafeActive`, `required_features:["wireless"]`, `requires_explicit_scope:false`). TUI participates identically via `TabSpec` delegation + shared `EnforcementContext::evaluate()` (preflight, `PendingPolicyConfirmation` / PolicyConfirm overlay, `ConfirmationClass` kebab strings, status bar mode/scope/risk, direct-launch retro gate). Strict profiles (`McpStrict`/`AgentStrict`/`CiStrict`) apply the feature gate + `LoadedScope` provenance rules if the central evaluator is ever reached via other paths. No capability matrix entries were added (consistent with the absent-MCP decision). Passive/defense-lab nature preserved.

- **Verification performed**: All wireless TUI + handler + policy paths exercised via feature-gated checks/tests (see commands below). Existing unit tests (parsing/analysis, no hardware) + TUI contract tests (tab metadata, visible specs roundtrips, from_stable_id guards, etc.) cover the surface. Pre-existing clippy warnings only (no new wireless-related issues).

- **Overall success criteria**: Functional TUI surface (yes); MCP exposure intentionally absent by design (documented); agentic/manual workflows consistent and safe via central evaluator (yes); passive/defense-lab nature preserved (yes).

Recommended verification commands for future agents (wireless feature):
- `cargo check -p eggsec --features wireless`
- `cargo check -p eggsec-tui --features wireless`
- `cargo test --lib -p eggsec --features wireless`
- `cargo clippy --lib -p eggsec --features wireless`

Cross-references (current post-plan state):
- architecture/wireless.md (MCP/Agentic section + Integration with Reporting Pipeline)
- architecture/defense_lab.md (Mobile/Wireless paragraphs)
- architecture/cli_commands.md (Special Cases)
- AGENTS.md (Key Types, Security Notes "Standalone Defense-Lab Surfaces", feature flags, Verification Commands, architecture index)
- docs/WIRELESS.md (TUI, Not In Scope, Integration with Reporting Pipeline, Troubleshooting)
- docs/USAGE.md (Report Management → Convert Reports → Output Models block)
- docs/CAPABILITIES.md (Lab Defense table)
- README.md (Lab Defense table, Quick Command Reference, feature/build notes)
- plans/wireless-micro-closeout-checklist.md (closeout record); plans/wireless-standalone-completion-plan.md (standalone); historical plans (first-handoff, advanced-integration, integration-work-plan, proposed stages)

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

1. **Task 3** (Agentic alignment) – Foundational for safety. (Completed via central evaluator; see resolution note.)
2. **Task 2** (MCP integration) — Intentionally deferred / not pursued per design decision (standalone defense-lab surface; see resolution note).
3. **Task 1** (TUI integration) — Completed (full tab + TabSpec + enforcement wiring + workers).
4. **Task 4** (polish) — N/A for this round (no new code; existing CLI/TUI polish from standalone completion applies).

---

## 5. Success Criteria (Overall)

- Wireless has a functional TUI surface. **Achieved.**
- Wireless is available via MCP with correct strict-profile behavior. **Intentionally absent per design decision (not a SecurityTool; see resolution note + architecture/wireless.md MCP section).**
- Agentic and manual workflows are consistent and safe. **Achieved** (central `EnforcementContext::evaluate()` + handler/TUI descriptor paths; strict rules apply where relevant).
- The passive/defense-lab nature of wireless is preserved. **Preserved.**

---

**This plan focuses on deeper integration surfaces while following Eggsec’s existing architectural patterns for scoping and enforcement. Post-execution: Task 1 and the alignment portion of Task 3 are complete; Task 2 (MCP) remains intentionally out-of-scope for the standalone defense-lab model.**