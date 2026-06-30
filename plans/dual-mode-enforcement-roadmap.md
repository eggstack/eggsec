# Dual-Mode Enforcement Roadmap

> **Status: IMPLEMENTED / CLOSED** — All 12 phases complete as of 2026-06-30. This document is now historical reference; the architecture is stable and enforced at the type level.

## Purpose

Eggsec needs one unified enforcement vocabulary with two clearly separated operating postures:

1. **Manual operator usage** for CLI and TUI. This mode should remain useful in the same practical sense as established legitimate security tooling. It should warn, explain, require explicit confirmation for operator-discretion cases, and audit overrides, but it should not inherit agent-grade manifest and allowlist requirements by default.
2. **Agent-facing usage** for MCP, the security agent, CI-like noninteractive verification, and any future scheduled/model-controlled surfaces. This mode must be strict, explicitly scoped, non-overridable, and revalidated immediately before dispatch.

The goal is not to make Eggsec globally stricter. The goal is to make strictness contextual and reliable: humans get explicit discretion; agents get hard boundaries.

## Architectural principle

Keep policy semantics centralized, but keep execution posture separate.

The existing `ExecutionProfile`, `OperationDescriptor`, `EnforcementContext`, `LoadedScope`, `ManualOverride`, and `EnforcementOutcome` model is the right foundation. The next step is to prevent callers from constructing inconsistent enforcement contexts and to make the manual/agent distinction explicit across all entrypoints.

The intended model is:

| Surface | Default posture | Scope behavior | Overrides | Dispatch rule |
| --- | --- | --- | --- | --- |
| CLI manual | Manual permissive | Warn/confirm for ambiguity | Honored only here | Allow, warn, or confirmed |
| TUI manual | Manual permissive | Warn/confirm for ambiguity | Honored only here | Allow, warn, or confirmed |
| CLI strict | Manual guarded | Hard-deny ambiguity | Ignored | Deny unless cleanly in policy |
| TUI guarded | Manual guarded | Hard-deny ambiguity | Ignored | Deny unless cleanly in policy |
| MCP | Agent/automated strict | Explicit manifest required | Ignored | Deny unless cleanly in policy |
| Security agent | Agent/automated strict | Explicit manifest required | Ignored | Deny unless cleanly in policy |
| CI | Agent/automated strict | Explicit manifest required | Ignored | Deny unless cleanly in policy |
| REST API | Strict by default unless explicitly local/manual | Explicit manifest required for strict API | Ignored in strict API | Deny unless cleanly in policy |

Manual mode must never be treated as unsafe merely because it is not agent-strict. Agent mode must never inherit manual discretion.

## Phase overview

### Phase 1: Mode contract documentation

Create a canonical document describing the two operating families, their permitted behaviors, and the invariants that all future code must preserve. This prevents future hardening work from accidentally making manual CLI/TUI behave like MCP.

Deliverables:

- `docs/ENFORCEMENT_MODES.md` or equivalent architecture document.
- Explicit mode matrix for CLI, TUI, MCP, security agent, CI, and REST.
- Definitions of `Allow`, `Warn`, `RequireConfirmation`, and `Deny` semantics per mode.
- List of classes where manual discretion is allowed and classes where no mode may bypass enforcement.

### Phase 2: First-class `ExecutionSurface`

Introduce a semantic caller-origin enum, separate from `ExecutionProfile`. `ExecutionProfile` describes enforcement behavior; `ExecutionSurface` describes who is calling and how to derive the correct behavior.

Expected surfaces:

- `CliManual`
- `TuiManual`
- `CliManualStrict`
- `TuiManualStrict`
- `McpServer`
- `SecurityAgent`
- `Ci`
- `RestApi`

Deliverables:

- New `ExecutionSurface` type in the config/policy layer.
- Central mapping from surface to `ExecutionProfile`.
- Helper methods such as `honors_manual_override()`, `is_agent_controlled()`, `requires_explicit_manifest_for_networked()`, and `default_bind_policy()` where relevant.
- Entry points updated to derive enforcement from surface rather than hand-rolling profile selection.

### Phase 3: AgentStrict correction and defense in depth

Ensure the security agent can never inherit manual enforcement. This is the first correctness fix.

Deliverables:

- Top-level CLI profile selection maps `Commands::Agent(_)` to `ExecutionSurface::SecurityAgent` and therefore `ExecutionProfile::AgentStrict`.
- `handle_agent()` defensively rebuilds `EnforcementContext::agent_strict(...)` before constructing `AgentConfig`.
- Agent runtime rejects or normalizes any non-agent-strict enforcement context supplied programmatically.
- Tests proving manual override flags have no effect in agent execution.

### Phase 4: Manual CLI/TUI discretion preservation

Add tests and minor ergonomics changes to ensure manual operation remains practical and does not regress into agent-style gating.

Deliverables:

- Manual-mode regression tests for warnings, confirmation, and accepted overrides.
- Confirmation class behavior checked for out-of-scope, target expansion, private resolution, cross-host redirect, high risk, nonbaseline capability, traffic interception, and explicit exclusions.
- `--yes` remains narrow and does not authorize high-risk/private-resolution/redirect/nonbaseline classes.
- TUI status text and CLI error text make the manual path explicit rather than surprising.

### Phase 5: TUI enforcement posture model

Make TUI enforcement first-class instead of an implicit side effect of settings fields.

Deliverables:

- `TuiEnforcementState` model containing surface, loaded scope, enforcement context, and manual override state.
- Visible enforcement posture indicator in the TUI.
- Guarded/manual toggle equivalent to CLI `--strict-scope`.
- Preflight result display for pending actions: allow, warn, confirm, deny.
- CLI-equivalent command preview including needed `--allow-*` flags.

### Phase 6: Metadata-derived operation descriptors

Reduce drift between CLI commands, TUI actions, MCP tools, REST tools, and operation descriptors.

Deliverables:

- Canonical `OperationMetadata` for every command/tool.
- Descriptor generation from metadata plus runtime target arguments.
- Registry validation that every externally invokable tool has metadata.
- Tests ensuring no MCP/REST/agent tool is exposed without risk, capabilities, feature gates, and target policy.

### Phase 7: REST API posture correction

REST currently risks being a weaker parallel dispatch path. Decide whether it is strict programmatic API by default, or split it into explicit local/manual and strict/agent APIs.

Preferred default: REST is strict unless launched in an explicitly named local/manual mode.

Deliverables:

- REST state carries `EnforcementContext`, not only raw `Scope`.
- `/api/v1/tools/{tool_id}/execute` evaluates enforcement before dispatch.
- Raw `ToolDispatcher::dispatch()` is not reachable from REST without an approved decision.
- REST tests for missing manifest, out-of-scope targets, high-risk tools, nonbaseline capabilities, and override rejection.

### Phase 8: Enforcement matrix tests

Create a regression suite that protects both sides of the model.

Deliverables:

- Matrix across surfaces: CLI manual, TUI manual, guarded manual, MCP, security agent, CI, REST.
- Matrix across scope states: none/default empty, explicit allow match, explicit allow miss, explicit exclusion.
- Matrix across risks and capabilities.
- Matrix across override flags.
- Assertions that manual discretion does not leak into agents and agent strictness does not leak into default manual use.

### Phase 9: Preflight everywhere

Expose the same enforcement evaluation as a dry-run/preflight operation across surfaces.

Deliverables:

- CLI preflight via existing `plan`/`policy-explain` style flows.
- TUI inline action preflight.
- MCP/agent denied responses include machine-readable decision data.
- REST dry-run/preflight parameter or endpoint.

### Phase 10: Normalized audit events

Normalize manual and automated audit event shapes.

Deliverables:

- Manual audit event: surface, outcome, classes, override reason, target, scope source, metadata id.
- Automated audit event: surface, outcome, policy/scope identity, descriptor, capabilities, no accepted override.
- Decision IDs consistently propagated to reports and logs.

### Phase 11: Domain crate extraction

Gradually move high-risk or dependency-heavy domains out of the main `eggsec` crate while keeping enforcement centralized.

Priority candidates:

- `eggsec-db-lab`
- `eggsec-web-proxy`
- `eggsec-mobile`
- `eggsec-wireless`
- `eggsec-evasion-lab`
- `eggsec-postex-lab`
- `eggsec-c2-lab`

Domain crates should expose metadata and execution functions. They should not own enforcement decisions.

### Phase 12: Type-level enforced dispatch

Move from convention to type-level enforcement for automated surfaces.

Deliverables:

- `ApprovedOperation` token produced only by enforcement evaluation.
- `EnforcedDispatcher::dispatch_checked(...)` for MCP/agent/REST/CI.
- Direct dispatcher access restricted where possible.
- Static or test-based checks for unauthorized dispatch paths.

## Success criteria

This roadmap is complete when the following statements are true:

- Manual CLI/TUI default behavior remains productive and comparable to legitimate security tools.
- Manual strict mode exists for operators who want hard enforcement.
- MCP and the security agent cannot execute networked operations without explicit scope provenance.
- MCP and the security agent ignore manual override flags by construction.
- REST cannot bypass shared enforcement.
- Every externally invokable operation has canonical metadata that drives descriptors, docs, and exposure policy.
- The enforcement matrix test suite catches both kinds of regression: over-gating manual workflows and under-gating agents.

## Recommended implementation order

1. Phase 1: document the contract.
2. Phase 2: introduce `ExecutionSurface`.
3. Phase 3: fix security-agent strictness.
4. Phase 4: preserve manual-mode behavior with regression tests.
5. Phase 5: formalize TUI posture.
6. Phase 7: fix REST API posture.
7. Phase 8: add the full enforcement matrix.
8. Phase 6: metadata-derived descriptors.
9. Phase 9: preflight everywhere.
10. Phase 10: normalized audit events.
11. Phase 11: domain extraction.
12. Phase 12: type-level enforced dispatch.

Phase 7 appears before Phase 6 in implementation order because REST dispatch posture is a nearer-term safety correctness issue. Metadata-derived descriptors are important, but REST should not remain a weak dispatch path while waiting for the larger metadata refactor.
