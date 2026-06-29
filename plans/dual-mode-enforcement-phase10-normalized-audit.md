# Phase 10 Handoff Plan: Normalized Audit Events

> **Status: COMPLETED** — Implementation done. All acceptance criteria met. All plan steps complete. Tests pass (2351 lib tests). Docs updated.

## Goal

Normalize audit events for enforcement decisions across manual and automated surfaces. Every meaningful policy decision should produce a consistent audit record that identifies the execution surface, profile, scope provenance, operation metadata, decision outcome, confirmation classes, and whether any manual override was accepted or ignored.

This phase makes Eggsec easier to debug, safer for agent workflows, and more useful for compliance/reporting.

## Current context

The repo already emits some tracing logs for manual overrides and agent/runtime behavior. TUI policy confirmation also records accepted overrides through tracing. However, audit shape is not yet centralized.

The desired model is:

- Manual CLI/TUI can record warnings and accepted confirmations.
- Automated surfaces record allow/deny decisions, but never accepted manual overrides.
- Agent/MCP/REST decisions can be traced and correlated with tool calls.
- Reports and logs can reconstruct why a dispatch did or did not happen.

## Files likely to change

- `crates/eggsec/src/config/policy_decision.rs`
- `crates/eggsec/src/config/policy.rs`
- New module: `crates/eggsec/src/audit.rs` or `crates/eggsec/src/enforcement/audit.rs`
- `crates/eggsec/src/commands/handlers/mod.rs`
- `crates/eggsec/src/tool/protocol/rest.rs`
- MCP handlers
- Agent modules
- TUI confirmation/preflight modules
- Output/report crates if audit events should be included in reports

## Proposed audit model

Add a serializable audit event:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementAuditEvent {
    pub event_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub surface: ExecutionSurface,
    pub profile: ExecutionProfile,
    pub operation_id: String,
    pub target: Option<String>,
    pub outcome: AuditOutcome,
    pub decision: PolicyDecision,
    pub confirmation_classes: Vec<ConfirmationClass>,
    pub manual_override: Option<ManualOverrideAudit>,
    pub manual_override_ignored: bool,
    pub scope: ScopeAudit,
    pub policy_hash: Option<String>,
    pub metadata_id: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuditOutcome {
    Allow,
    Warn,
    Confirmed,
    Deny,
    ConfirmationRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualOverrideAudit {
    pub reason: Option<String>,
    pub classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeAudit {
    pub source: ScopeSource,
    pub path: Option<String>,
    pub allow_rule_count: usize,
    pub exclusion_rule_count: usize,
    pub explicit_manifest: bool,
}
```

Avoid serializing full scope rules in routine audit events. Counts and provenance are enough by default. Full scope hash can be added if needed.

## Step 1: Add audit builder helpers

Create helpers:

```rust
pub fn audit_event_from_preflight(... ) -> EnforcementAuditEvent;
pub fn audit_event_from_enforcement_outcome(... ) -> EnforcementAuditEvent;
pub fn scope_audit(loaded_scope: &LoadedScope) -> ScopeAudit;
```

Inputs should include:

- `ExecutionSurface`
- `EnforcementContext`
- `OperationDescriptor`
- `EnforcementOutcome`
- Optional `ManualOverride`
- Optional correlation/request ID
- Optional metadata ID

Do not make audit construction require dispatch. It should work for preflight and execution.

## Step 2: Add stable event IDs and hashes

Use UUIDs for `event_id` initially.

For `policy_hash`, prefer a stable hash of serialized `ExecutionPolicy`. If serialization is not stable yet, omit or use best-effort with clear comment.

For future compatibility, keep hashes optional.

## Step 3: CLI audit integration

In `CommandContext::evaluate_and_enforce_operation()`:

- Emit audit event for `Allow` and `Warn` at debug/info level.
- Emit audit event for `RequireConfirmation` without matching override.
- Emit audit event for accepted manual override with `AuditOutcome::Confirmed`.
- Emit audit event for `Deny`.

Manual override accepted should include:

- Classes.
- Reason.
- Surface/profile.
- Target.
- Scope provenance.

Manual strict or automated contexts receiving manual override flags should set `manual_override_ignored = true` if such flags are present.

## Step 4: TUI audit integration

In TUI policy evaluation:

- Emit preflight audit event for `Deny` and `RequireConfirmation`.
- Emit accepted override audit event in `confirm_policy_action()`.
- Include TUI surface and scope provenance.

Avoid excessive log spam for every cursor movement or non-executable tab action. Audit only enforcement decisions for executable operations.

## Step 5: REST audit integration

REST execute path should audit:

- Policy denial.
- Warning-denial if Phase 7 denies warnings.
- Successful allow before dispatch.
- Dispatcher error may optionally emit separate tool-execution audit, but enforcement audit should be separate.

Include REST request/correlation ID. REST already has `generate_correlation_id()`; use it if available.

## Step 6: MCP audit integration

MCP tool execution should audit:

- Tool call received.
- Enforcement allow/deny.
- Manual override ignored if request attempts unsupported fields.
- Correlation/request ID.

Do not include raw sensitive tool params in audit events by default.

## Step 7: Agent audit integration

Agent scan path should audit:

- Scheduled scan preflight result.
- Immediate pre-dispatch enforcement result.
- Denials preventing dispatch.

Persist recent enforcement denials in agent runtime state if useful:

```rust
pub recent_policy_denials: Vec<EnforcementAuditEvent>
```

Bound this list to a small number, such as last 50.

## Step 8: Output/report integration

If Eggsec reports have a policy/enforcement section, include a summary:

- Number of enforcement decisions.
- Number of warnings.
- Number of denied operations.
- Number of manual confirmations.
- Surfaces involved.

Do not dump every audit event into normal reports unless a verbose/audit option is enabled.

## Step 9: Tests

Required tests:

- Audit event includes surface/profile/operation/target.
- Scope audit counts allow/exclusion rules correctly.
- Manual accepted override includes classes and reason.
- Automated surface with manual override flags marks `manual_override_ignored` or does not include accepted override.
- REST denial produces policy audit event.
- TUI confirmation produces confirmed audit event.
- Agent denied scan produces audit event and does not dispatch.

## Acceptance criteria

- A single audit event model exists.
- CLI, TUI, REST, MCP, and agent use the model for enforcement decisions.
- Manual confirmations are recorded with class and reason.
- Automated surfaces never record accepted manual overrides.
- Scope provenance is included.
- Tests cover representative manual and automated audit events.

## Validation commands

Run:

```bash
cargo fmt --all
cargo test -p eggsec --features rest-api --lib
cargo test -p eggsec-tui
cargo check -p eggsec-cli --features rest-api
```

## Non-goals

- Do not build a full audit database.
- Do not serialize full request payloads by default.
- Do not change enforcement decisions.
- Do not implement type-level dispatch tokens yet.
