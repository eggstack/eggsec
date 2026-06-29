# Phase 9 Handoff Plan: Preflight Everywhere

## Goal

Expose shared enforcement evaluation as a first-class preflight capability across CLI, TUI, MCP, REST, security agent, and CI-style flows.

A user or program should be able to ask Eggsec: “What would happen if I ran this operation?” and receive the same policy/scope/capability decision that would be used immediately before dispatch.

This phase improves usability, debuggability, and agent safety without changing the core dual-mode semantics.

## Current context

The repo already has several partial preflight-like flows:

- `CommandContext::evaluate_and_enforce_operation()` is the CLI command enforcement path.
- TUI stores `last_preflight` and displays policy results.
- MCP/REST now evaluate before dispatch.
- Agent evaluates per scan before dispatch.

What is missing is a shared preflight API and consistent output shape across surfaces.

## Design principle

Preflight must not duplicate enforcement logic.

Every preflight result should be produced by:

1. Constructing the same `OperationDescriptor` that dispatch would use.
2. Evaluating the same `EnforcementContext` that dispatch would use.
3. Returning a serializable explanation of the result.

There should be no separate preview policy path.

## Files likely to change

- `crates/eggsec/src/config/policy_decision.rs`
- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/src/commands/handlers/mod.rs`
- CLI command definitions under `crates/eggsec/src/cli` or `crates/eggsec-cli`
- `crates/eggsec-tui/src/app/enforcement.rs`
- `crates/eggsec-tui/src/ui/...`
- `crates/eggsec/src/tool/protocol/rest.rs`
- MCP protocol handlers
- Agent modules where scheduled decisions are logged/reported

## Proposed shared type

Add a serializable preflight type near enforcement code:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightResult {
    pub surface: ExecutionSurface,
    pub profile: ExecutionProfile,
    pub descriptor: OperationDescriptor,
    pub outcome_kind: PreflightOutcomeKind,
    pub decision: PolicyDecision,
    pub required_confirmation_classes: Vec<ConfirmationClass>,
    pub manual_override_honored: bool,
    pub scope_source: ScopeSource,
    pub scope_path: Option<String>,
    pub suggested_cli_flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreflightOutcomeKind {
    Allow,
    Warn,
    RequireConfirmation,
    Deny,
}
```

If `OperationDescriptor` is too large/noisy for some API responses, provide a compact wrapper later. Start with full fidelity.

## Shared evaluator helper

Add a method on `EnforcementContext` or a free helper:

```rust
pub fn preflight_operation(
    surface: ExecutionSurface,
    enforcement: &EnforcementContext,
    descriptor: OperationDescriptor,
    manual_override: Option<&ManualOverride>,
) -> PreflightResult
```

Rules:

- Always call `enforcement.evaluate(&descriptor)`.
- Derive required confirmation classes only for `RequireConfirmation`.
- `manual_override_honored` should be true only when `surface.honors_manual_override()` and all required classes are permitted by the override.
- Suggested flags should be omitted for automated surfaces or clearly marked as manual-only.
- The result should not dispatch anything.

## CLI preflight

Add or extend CLI commands so users can inspect policy decisions without running operations.

Possible commands:

```bash
eggsec preflight <operation> <target> [flags]
eggsec policy explain <operation> <target> [flags]
eggsec plan <operation> <target> [flags]
```

Use whichever command structure best matches the repo. Avoid creating multiple redundant commands.

Required CLI behavior:

- Uses the same config, scope, execution surface, and manual override flags as the eventual command.
- Prints human-readable output by default.
- Prints JSON when `--json` is set.
- Does not execute tools.
- Names required flags/classes for manual mode.
- Does not suggest manual flags for MCP/agent/REST/CI strict contexts.

Example human output:

```text
Operation: fuzz
Target: https://example.com
Surface: CLI manual
Profile: manual-permissive
Outcome: confirmation required
Classes: high-risk, nonbaseline-capability
Suggested flags: --allow-high-risk --allow-nonbaseline-capability
Scope: scope.toml
```

## TUI preflight

TUI already has `TuiPreflightResult`. Refactor it to wrap or convert from the shared `PreflightResult`.

Required behavior:

- Status bar or action panel shows current preflight outcome.
- Confirmation overlay uses shared required classes and suggested flags.
- Guarded mode displays hard-deny outcomes clearly.
- Manual mode displays warnings/confirmation clearly.
- Preflight updates before execution for task and direct-launch tabs.

Do not keep a separate TUI-only class calculation if the shared preflight helper exists.

## REST preflight

Add a REST endpoint or request parameter.

Preferred endpoint:

```http
POST /api/v1/tools/{tool_id}/preflight
```

Request body matches execute request:

```json
{
  "target": "https://example.com",
  "target_type": "url",
  "params": {},
  "options": {}
}
```

Response is `PreflightResult`.

Alternative:

```http
POST /api/v1/tools/{tool_id}/execute?dry_run=true
```

Prefer a separate endpoint. It avoids ambiguity and makes it impossible for clients to accidentally execute.

REST preflight must use `ExecutionSurface::RestApi` and strict enforcement. It should not honor manual overrides.

## MCP preflight

Add a tool or method for agent-facing preflight.

Options:

- Add MCP tool: `eggsec_preflight`.
- Add optional `dry_run` parameter to existing tools only if the MCP tool schemas support it cleanly.

Preferred: dedicated preflight tool with structured result.

MCP preflight must:

- Use the same metadata-derived descriptor as execution.
- Use the same `McpStrict` enforcement context.
- Return machine-readable denied reasons/classes.
- Never dispatch.

## Agent preflight

Add preflight logging/trace for scheduled scans:

- Before a scheduled scan executes, generate preflight result.
- If denied, log/store the preflight decision in agent runtime status or memory.
- If allowed, proceed as today.

If there is an agent status command, include recent policy denials or last preflight summary.

Do not let agent preflight become a separate approval mechanism. Agent execution must still evaluate immediately before dispatch.

## CI preflight

If CI commands exist, add a preflight/report mode:

```bash
eggsec ci preflight --scope scope.toml ...
```

or incorporate preflight decision into CI report output.

CI preflight should be strict and deterministic.

## Tests

Required tests:

- Shared preflight result matches `EnforcementContext::evaluate()` outcome.
- Manual preflight includes suggested manual flags.
- Automated preflight omits or marks manual flags as unavailable.
- REST preflight endpoint does not dispatch.
- MCP preflight tool does not dispatch.
- TUI preflight uses shared helper and still displays expected classes.
- Agent denied preflight records/logs decision and does not dispatch.
- Preflight result for a representative metadata operation matches actual execute-path descriptor.

## Acceptance criteria

- A shared `PreflightResult` exists.
- CLI can preflight a representative operation without executing.
- TUI uses the shared preflight structure or a lossless wrapper.
- REST has a preflight endpoint or unambiguous preflight mode.
- MCP has a preflight tool or method.
- Agent logs/stores preflight denials for scheduled scans.
- Preflight uses the same descriptor/evaluator as dispatch.
- Tests prove preflight does not dispatch.

## Validation commands

Run:

```bash
cargo fmt --all
cargo test -p eggsec --features rest-api --lib
cargo test -p eggsec-tui
cargo check -p eggsec-cli --features rest-api
```

If MCP has integration tests, run the relevant MCP test target too.

## Non-goals

- Do not change enforcement semantics.
- Do not make REST manual-interactive.
- Do not let preflight approval replace immediate pre-dispatch evaluation.
- Do not implement type-level dispatch tokens yet.
