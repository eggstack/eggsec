# Wireless Advanced Integration Plan

**Module**: Wireless  
**Focus**: CLI, TUI, MCP, and Agentic Workflow Integration  
**Date**: 2026-06-11

---

## Resolution / Post-execution status (2026-06-11)

**This plan has been executed and closed (with design decisions recorded).**

- **CLI polish** (Task-equivalent): Complete. Help text, MODE prefix, practical examples, `--detect-suspicious` canonical form, warnings, and dry-run/known-good/repeat UX all landed in the standalone completion and micro-closeout work.
- **TUI integration**: Complete via `WirelessTab` + `TabSpec` registration (risk_group SafeActive, feature="wireless", direct_launch) + full enforcement wiring (central `EnforcementContext`, preflight, policy confirm overlay). See `plans/wireless-tui-mcp-agentic-handoff-plan.md` (Task 1) + resolution note at top of that plan, `architecture/tui.md`, and `crates/eggsec-tui/src/tabs/wireless.rs`.
- **MCP / agentic tool exposure**: **Intentionally not implemented** per final design decision (standalone defense-lab surface). Wireless is not registered as a `SecurityTool`, is invisible to `tools/list` / `tools/call`, and has no presence in `tool/protocol/mcp/policy.rs` (or agent dispatch). This mirrors the mobile + auth-test pattern and preserves the passive-only, local-interface, root/CAP_NET_ADMIN reality. See `architecture/wireless.md` (MCP / Agentic / Tool Integration Status section + resolution note header), `architecture/defense_lab.md`, `architecture/cli_commands.md` (Special Cases), AGENTS.md (standalone defense-lab surfaces), and `docs/USAGE.md` (Output Models block). The optional `to_scan_report_data` + CLI auto-bridge for `report convert` works independently of invocation surface.
- **Agentic alignment**: Achieved via central `EnforcementContext::evaluate()` (CLI handler + TUI direct-launch path). Per-scan considerations for agent paths apply only if ever exposed (currently not).

**See also** (for the consolidated pattern and final state):
- `plans/wireless-tui-mcp-agentic-handoff-plan.md` (resolution note records TUI complete + MCP absent)
- `plans/wireless-micro-closeout-checklist.md` + `plans/wireless-standalone-completion-plan.md`
- `plans/new-modules-integration-and-closeout-plan.md` + `plans/final-cleanup-new-modules-plan.md`
- `plans/integration-work-plan.md` (reporting bridges)
- `architecture/wireless.md` (MCP/Agentic + Integration sections), `docs/WIRELESS.md` (Integration with Reporting Pipeline), CAPABILITIES.md / README Lab Defense tables.

**Verification commands** (post-execution):
```bash
cargo check -p eggsec --features wireless
cargo check -p eggsec-tui --features wireless
cargo test --lib -p eggsec --features wireless
cargo clippy --lib -p eggsec --features wireless
```

This plan is retained for historical reference. The passive / defense-lab identity of wireless is preserved. No code changes required for closure.

---

## 1. Current State

Wireless is currently a solid **standalone defense-lab command**:
- Passive scanning via `iwlist`
- Rogue/Evil Twin detection with known-good support
- Repeated scan + change detection
- Dry-run mode
- Optional `to_scan_report_data()` bridge
- Handler exists at `commands/handlers/wireless.rs`
- CLI args defined

It is well-scoped as a passive, lab-oriented tool. However, deeper integration with the broader CLI surface, TUI, MCP server, and agentic execution profiles is still limited.

---

## 2. Goals

- Improve CLI discoverability and usability
- Add proper TUI integration (following existing tab/view patterns)
- Enable clean MCP exposure with correct scoping and policy behavior
- Align with existing agentic workflow patterns (`McpStrict`, `AgentStrict`, `CiStrict` vs `ManualPermissive`)
- Maintain the passive / defense-lab identity of the tool

---

## 3. Integration Areas

### 3.1 CLI Improvements

**Current Gaps**:
- Help text and discoverability could be better
- Some advanced flags (`--repeat`, `--known-good`, `--detect_suspicious`) may not be obvious to new users

**Tasks**:
- Review and improve `WirelessArgs` and command help text
- Add better examples in `--help` output
- Consider adding a short `--examples` or improved usage section
- Ensure consistent flag naming and descriptions with other modules
- Update `README.md` and `CAPABILITIES.md` command reference if needed

### 3.2 TUI Integration

**Goal**: Add Wireless as a first-class citizen in the TUI (similar to other modules).

**Tasks**:
- Investigate existing TUI tab architecture (`eggsec-tui` crate)
- Create a `WirelessTab` or integrate into an existing network/defense tab
- Follow patterns used by other tabs (e.g. how `AuthTab` or scan results are handled)
- Support:
  - Interface selection
  - Scan execution (with dry-run option)
  - Results viewing (including rogue detection)
  - Repeated scan mode visualization
- Ensure TUI respects `EnforcementContext` and scope

**Considerations**:
- Wireless requires root / `CAP_NET_ADMIN` on Linux. TUI should surface this clearly.
- Dry-run mode should be easily accessible.

### 3.3 MCP Integration

**Goal**: Make wireless available through the MCP server with proper scoping and safety.

**Tasks**:
- Add wireless capability to the MCP tool registry (following patterns in `tool/` or MCP handlers)
- Define proper `OperationDescriptor` for wireless operations
- Ensure `McpStrict` profile correctly gates the feature:
  - Requires explicit scope manifest
  - Respects `required_features: ["wireless"]`
  - Uses `SafeActive` risk tier
- Implement MCP tool description and argument schema
- Support both one-shot scans and repeated monitoring use cases where appropriate

**Key Requirements**:
- Never allow wireless in strict profiles without explicit scope
- Dry-run mode should be preferred / default in automated contexts

### 3.4 Agentic Workflow Patterns (MCP / Agent / CI)

**Goal**: Ensure consistent behavior across execution profiles.

**Current Patterns to Follow**:
- `McpStrict`, `AgentStrict`, and `CiStrict` require explicit `LoadedScope` manifests
- Non-baseline capabilities need `allowed_capabilities` or explicit allow
- High-risk or special operations are denied by default in strict profiles

**Tasks**:
- Verify wireless handler properly calls `ctx.evaluate_and_enforce_operation()` with correct descriptor
- Confirm behavior under different `ExecutionProfile` values:
  - `ManualPermissive` (current default – more lenient)
  - `McpStrict` / `AgentStrict` (strict scoping required)
- Document expected behavior in `architecture/wireless.md` or `architecture/agent.md`
- Consider adding wireless to relevant capability matrices for MCP/agent profiles

**Specific Rules**:
- Wireless should generally be allowed under `SafeActive` when the feature is enabled
- In strict agentic profiles, it should require an explicit scope manifest even for local interfaces
- Repeated scan mode may need additional controls in automated contexts

### 3.5 General Improvements

- Review and polish the optional `to_scan_report_data()` bridge
- Improve finding categories and evidence quality for rogue detection
- Add better error messages when `iwlist` or root privileges are missing
- Consider adding a `--json` friendly summary mode for repeated scans

---

## 4. Recommended Phased Approach

### Phase 1: CLI Polish (Quick Win)
- Improve help text and examples
- Update documentation references

### Phase 2: TUI Integration
- Design and implement Wireless tab/view following existing TUI patterns

### Phase 3: MCP + Agentic Alignment
- Wire wireless into MCP tool surface
- Ensure correct enforcement behavior across `ExecutionProfile` types
- Document agentic workflow expectations

### Phase 4: Polish & Bridge Improvements
- Finalize reporting bridge quality
- Improve error handling and UX edge cases

---

## 5. Success Criteria

- Wireless is easily discoverable and usable from the main CLI
- Wireless has a functional TUI surface that respects policy and scope
- Wireless is available via MCP with correct strict-profile behavior
- Agentic (MCP/Agent/CI) and manual workflows behave consistently and safely
- The passive/defense-lab nature of wireless is preserved across all surfaces

---

## 6. Risks & Considerations

- TUI integration may require coordination with the `eggsec-tui` crate
- MCP exposure increases the attack surface — strict scoping must be enforced
- Root requirement for real scans needs clear UX treatment in both TUI and MCP contexts
- Repeated scan mode has different risk/UX implications in automated vs manual use

---

**This plan provides a structured path to bring wireless from a good standalone command into a more deeply integrated part of the Eggsec ecosystem while following existing architectural patterns.**