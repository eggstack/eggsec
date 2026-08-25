# Normalized Audit Events

## Role & Responsibilities

`audit.rs` provides a single `EnforcementAuditEvent` model for consistent audit records across all execution surfaces (CLI, TUI, REST, MCP, gRPC, Agent, CI). Every meaningful enforcement decision produces an audit event that can be used for debugging, compliance reporting, and agent workflow correlation.

**Non-responsibilities:**
- Audit emission never changes control flow or return values — it is purely observational.
- Audit events capture decisions, not request payloads (no full payload serialization).
- Audit persistence is handled by consumers (e.g., `AuditSummary` in `eggsec-output`, agent denial recording in `eggsec::agent`), not by `audit.rs` itself.
- Audit events do not perform authorization or policy evaluation — they record the result of `EnforcementContext::evaluate()`.

## Location & Feature Gating

| Item | Path | Feature Gate |
|------|------|:------------:|
| Audit module | `crates/eggsec/src/audit.rs` | None (always compiled) |
| Re-exports | `crates/eggsec/src/lib.rs:205-206` | None |
| `AuditSummary` (consumer) | `crates/eggsec-output/src/audit_summary.rs` | None |
| Agent denial recording (consumer) | `crates/eggsec/src/agent/mod.rs:466` | None |
| REST integration (consumer) | `crates/eggsec/src/tool/protocol/rest.rs` | `rest-api` |
| gRPC integration (consumer) | `crates/eggsec/src/tool/protocol/grpc.rs` | `grpc-api` |
| MCP integration (consumer) | `crates/eggsec/src/tool/protocol/mcp/handlers/server.rs` | `rest-api` |
| Agent integration (consumer) | `crates/eggsec/src/agent/mod.rs` | None |

## Architecture

### EnforcementAuditEvent (15 fields, `audit.rs:17`)

```rust
pub struct EnforcementAuditEvent {
    pub event_id: String,           // UUID v4 (:135)
    pub timestamp: DateTime<Utc>,   // Utc::now() (:136)
    pub surface: ExecutionSurface,  // Caller origin
    pub profile: ExecutionProfile,  // Trust boundary
    pub operation_id: String,       // Canonical operation name
    pub target: Option<String>,     // Target if applicable
    pub outcome: AuditOutcome,      // Simplified outcome
    pub decision: PolicyDecision,   // Full policy decision with decision_id
    pub confirmation_classes: Vec<ConfirmationClass>,
    pub manual_override: Option<ManualOverrideAudit>,
    pub manual_override_ignored: bool,
    pub scope: ScopeAudit,          // Scope provenance summary
    pub policy_hash: Option<String>,// SHA-256 of serialized ExecutionPolicy
    pub metadata_id: Option<String>,
    pub correlation_id: Option<String>,
}
```

Derives: `Debug`, `Clone`, `Serialize`, `Deserialize` (`:16`).

### AuditOutcome (5 variants, `audit.rs:38`)

| Variant | Source (`from_outcome`, `:49`) | Tracing Level |
|---------|-------------------------------|---------------|
| `Allow` | `EnforcementOutcome::Allow` | `info!` (`:191`) |
| `Warn` | `EnforcementOutcome::Warn` | `info!` (`:191`) |
| `Confirmed` | `RequireConfirmation` with `confirmed=true` | `info!` (`:191`) |
| `Deny` | `EnforcementOutcome::Deny` | `warn!` (`:205`) |
| `ConfirmationRequired` | `RequireConfirmation` with `confirmed=false` | `warn!` (`:205`) |

Serde: `#[serde(rename_all = "kebab-case")]` (`:37`) — serialized as `"allow"`, `"warn"`, `"confirmed"`, `"deny"`, `"confirmation-required"`.

### ManualOverrideAudit (`audit.rs:67`)

```rust
pub struct ManualOverrideAudit {
    pub reason: Option<String>,
    pub classes: Vec<String>,  // ConfirmationClass as strings
}
```

Built from `ManualOverride` + required classes via `from_override()` (`:74`). Only populated when `confirmed=true` in the parent event (`:128-132`).

### ScopeAudit (`audit.rs:87`)

```rust
pub struct ScopeAudit {
    pub source: ScopeSource,            // DefaultEmpty/ConfigFile/CliScopeFile/GeneratedPreset
    pub path: Option<String>,           // Path to scope file
    pub allow_rule_count: usize,        // Number of allow rules
    pub exclusion_rule_count: usize,    // Number of exclusion rules
    pub explicit_manifest: bool,        // Explicit provenance (required for automated profiles)
}
```

Built from `LoadedScope` via `from_loaded_scope()` (`:97`). Extracts rule counts and explicit manifest status without serializing full scope rules.

### Key Functions

| Function | Line | Signature | Purpose |
|----------|------|-----------|---------|
| `audit_event_from_enforcement_outcome()` | `:113` | 10 params → `EnforcementAuditEvent` | Primary builder for enforcement decisions |
| `audit_event_from_preflight()` | `:156` | 8 params → `EnforcementAuditEvent` | Preflight advisory (never confirms, never ignores override) |
| `emit_audit_event()` | `:183` | `&EnforcementAuditEvent` → `()` | Log at appropriate tracing level |
| `AuditOutcome::from_outcome()` | `:49` | `(&EnforcementOutcome, bool) → AuditOutcome` | Map from enforcement outcome |
| `ScopeAudit::from_loaded_scope()` | `:97` | `&LoadedScope → ScopeAudit` | Extract scope provenance |
| `ManualOverrideAudit::from_override()` | `:74` | `(&ManualOverride, &[ConfirmationClass]) → Self` | Extract override details |

### `audit_event_from_enforcement_outcome` Parameters (`:113`)

| Parameter | Type | Purpose |
|-----------|------|---------|
| `surface` | `ExecutionSurface` | Which surface made the decision |
| `enforcement` | `&EnforcementContext` | Profile, scope, policy hash |
| `descriptor` | `&OperationDescriptor` | Operation ID and target |
| `outcome` | `&EnforcementOutcome` | The decision result |
| `confirmed` | `bool` | Whether override was accepted |
| `override_ignored` | `bool` | Whether override flags were present but ignored |
| `manual_override` | `Option<&ManualOverride>` | Override details if confirmed |
| `required_classes` | `&[ConfirmationClass]` | Classes required for confirmation |
| `correlation_id` | `Option<&str>` | Request/correlation ID |
| `metadata_id` | `Option<&str>` | Optional operation metadata ID |

### `audit_event_from_preflight` (`:156`)

Delegates to `audit_event_from_enforcement_outcome` with hardcoded:
- `confirmed = false` — preflight never confirms
- `override_ignored = false` — no override ignored at preflight
- `metadata_id = None`

## Behavior / Flow

### Event Construction

1. Event ID generated as UUID v4 (`Uuid::new_v4().to_string()`, `:135`).
2. Timestamp set to `Utc::now()` (`:136`).
3. `AuditOutcome::from_outcome()` maps the 4-variant `EnforcementOutcome` to the 5-variant `AuditOutcome` based on the `confirmed` flag.
4. `ManualOverrideAudit` is populated only if `confirmed=true` AND `manual_override` is `Some` (`:128-132`).
5. `ScopeAudit` extracted from `enforcement.loaded_scope` (`:146`).
6. `policy_hash` extracted from `enforcement.policy_hash()` (`:147`) — SHA-256 of serialized `ExecutionPolicy`, 64 hex chars.

### Emission (`emit_audit_event`, `:183`)

- `Allow | Warn | Confirmed` → `tracing::info!` with structured fields (`:191-203`).
- `ConfirmationRequired | Deny` → `tracing::warn!` with identical structured fields (`:205-218`).
- `serde_json::to_string(&event.outcome).unwrap_or_default()` (`:184`) — the `unwrap_or_default` silently converts serialization failure to `"null"`. This is on the error-logging path, not a correctness concern, but worth noting.
- Fields logged: `event_id`, `decision_id`, `outcome`, `operation`, `surface`, `profile`, `target`, `scope_source`, `manual_override_ignored`.

## Integration Points

### Per-Surface Integration

| Surface | Audit Emitted | Manual Override Record | Correlation ID | Consumer File |
|---------|:------------:|----------------------|:--------------:|---------------|
| CLI | Yes | Accepted overrides include class+reason | None | `commands/handlers/` (via `EnforcementContext`) |
| TUI | Yes | Accepted overrides include class+reason | None | `eggsec-tui` (via `EnforcementContext`) |
| REST | Yes | Never (REST never confirms) | `generate_correlation_id()` | `tool/protocol/rest.rs:707` |
| MCP | Yes | Never (MCP never confirms) | JSON-RPC request id | `tool/protocol/mcp/handlers/server.rs:554` |
| gRPC | Yes | Never (gRPC never confirms) | Request correlation | `tool/protocol/grpc.rs:623` |
| Agent | Yes | Never (Agent never confirms) | None | `agent/mod.rs:958` |
| CI | Yes | Never | None | Via `EnforcementContext` |

### Agent Denial Recording (`agent/mod.rs:466`)

The `Agent` struct maintains a bounded list of recent policy denial events:
- Field: `recent_policy_denials: Mutex<Vec<EnforcementAuditEvent>>` (`:218`).
- Capacity: **50** events max (`:473-474`). Old events are drained from the front.
- Only `Deny` and `ConfirmationRequired` outcomes are recorded (`:467-468`).
- Exposed via `Agent::recent_policy_denials()` (`:479`) and included in `AgentRuntimeStatus` as `recent_denial_count` (`:516,532`).

### AuditSummary (`eggsec-output/src/audit_summary.rs:4`)

Provides a summary of audit events from JSON, counting outcomes by type. Fields: `total_events`, `allow_count`, `warn_count`, `confirmed_count`, `deny_count`, `confirmation_required_count`, `manual_override_ignored_count`, `surfaces_used`. Built via `from_serde_value()` or `from_values()`. Useful for report generation without importing the full audit model.

## Testing

All tests are in `audit.rs:222-765`. Test count: 16 tests total.

| Test | Line | What It Verifies |
|------|------|------------------|
| `scope_audit_counts_rules` | `:252` | allow/exclusion rule counts, explicit_manifest, source, path |
| `scope_audit_default_empty` | `:275` | DefaultEmpty scope → 0 rules, not explicit |
| `audit_event_from_allow_outcome` | `:285` | Allow event fields, no override |
| `audit_event_from_deny_outcome` | `:316` | Deny event, correlation_id |
| `audit_event_with_confirmed_override` | `:345` | Confirmed override with reason and classes |
| `audit_event_with_ignored_override` | `:381` | Ignored override flag, no ManualOverrideAudit |
| `audit_event_serializes_roundtrip` | `:409` | JSON serialize → deserialize roundtrip |
| `audit_outcome_from_outcome_allow` | `:438` | Allow mapping |
| `audit_outcome_from_outcome_warn` | `:452` | Warn mapping |
| `audit_outcome_from_outcome_require_confirmation_not_confirmed` | `:466` | ConfirmationRequired mapping |
| `audit_outcome_from_outcome_require_confirmation_confirmed` | `:481` | Confirmed mapping |
| `audit_outcome_from_outcome_deny` | `:496` | Deny mapping |
| `manual_override_audit_from_override` | `:511` | Override audit from ManualOverride |
| `policy_hash_is_stable` | `:525` | Same policy → same hash, 64 chars |
| `policy_hash_differs_for_different_policies` | `:534` | Different policies → different hashes |
| `rest_deny_outcome_produces_audit_event` | `:544` | REST deny with correlation_id |
| `tui_confirm_outcome_produces_audit_event` | `:590` | TUI confirmed with override |
| `agent_denied_scan_produces_audit_event` | `:642` | Agent deny, AgentStrict profile |
| `preflight_event_never_marks_confirmed` | `:688` | Preflight → ConfirmationRequired, not Confirmed |
| `emit_audit_event_does_not_panic` | `:715` | emit on Allow event |
| `emit_deny_event_does_not_panic` | `:740` | emit on Deny event |

## Invariants & Gotchas

### Invariants

1. **Purely observational** — Audit emission never changes control flow or return values.
2. **Preflight never confirms** — `audit_event_from_preflight()` always passes `confirmed=false` (`:170`).
3. **Manual override only when confirmed** — `manual_override` field is `Some` only if `confirmed=true` AND override was provided (`:128-132`).
4. **Agent denial bounded to 50** — `recent_policy_denials` drains old entries when exceeding 50 (`:473-474`).
5. **Policy hash is deterministic** — SHA-256 of serialized `ExecutionPolicy`, 64 hex chars, stable across calls (`:525-531`).
6. **UUID v4 for event_id** — Stable, unique, no coordination required.

### Gotchas

- **`emit_audit_event` uses `unwrap_or_default` on outcome serialization** (`:184`): If `serde_json::to_string(&event.outcome)` fails (shouldn't for the enum), the outcome string becomes `"null"`. This is a non-critical path (logging only).
- **`AuditOutcome` is not the same as `EnforcementOutcome`**: 5 vs 4 variants. The extra `Confirmed` variant is derived from `RequireConfirmation` + `confirmed=true`.
- **`ScopeAudit` does not serialize full rules**: Only rule counts and provenance. This is intentional — full scope serialization is expensive and not needed for audit trails.
- **Correlation ID semantics differ by surface**: REST uses generated IDs, MCP uses JSON-RPC request IDs, CLI/TUI/Agent use `None`.
- **`recent_policy_denials` uses `Mutex<Vec>`**: Not a `tokio::sync::Mutex`. Acceptable since the critical section is short (push + drain) and contention is low.

---

*Last verified against source: 2026-08-25*
