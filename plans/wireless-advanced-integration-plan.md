# Wireless Advanced Integration Plan

**Module**: Wireless  
**Focus**: CLI, TUI, MCP, and Agentic Workflow Integration  
**Date**: 2026-06-11

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