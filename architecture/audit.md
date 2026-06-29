# Normalized Audit Events

## Purpose

`audit.rs` provides a single `EnforcementAuditEvent` model for consistent audit records across all execution surfaces. Every meaningful enforcement decision produces an audit event that can be used for debugging, compliance reporting, and agent workflow correlation.

## Event Model

```rust
pub struct EnforcementAuditEvent {
    pub event_id: String,           // UUID v4
    pub timestamp: DateTime<Utc>,   // When the decision was made
    pub surface: ExecutionSurface,  // Which surface made the decision
    pub profile: ExecutionProfile,  // Which profile was active
    pub operation_id: String,       // Canonical operation name
    pub target: Option<String>,     // Target if applicable
    pub outcome: AuditOutcome,      // Allow/Warn/Confirmed/Deny/ConfirmationRequired
    pub decision: PolicyDecision,   // Full policy decision with decision_id
    pub confirmation_classes: Vec<ConfirmationClass>, // Required classes
    pub manual_override: Option<ManualOverrideAudit>, // Override details (only when confirmed)
    pub manual_override_ignored: bool, // True if override flags present but surface ignores them
    pub scope: ScopeAudit,          // Scope provenance summary
    pub policy_hash: Option<String>, // Future: stable hash of execution policy
    pub metadata_id: Option<String>, // Optional operation metadata ID
    pub correlation_id: Option<String>, // Optional request/correlation ID
}
```

## Key Functions

| Function | Purpose |
|----------|---------|
| `audit_event_from_enforcement_outcome()` | Build event from enforcement decision at dispatch or preflight |
| `audit_event_from_preflight()` | Build event from preflight advisory evaluation |
| `emit_audit_event()` | Log at appropriate tracing level (info for allow/warn/confirmed, warn for deny) |
| `ScopeAudit::from_loaded_scope()` | Extract scope provenance summary |
| `ManualOverrideAudit::from_override()` | Extract override details for confirmed decisions |

## Per-Surface Integration

| Surface | Audit Emitted | Manual Override Record | Correlation ID |
|---------|--------------|----------------------|----------------|
| CLI (`CommandContext::evaluate_and_enforce_operation`) | Yes | Accepted overrides include class+reason | None |
| TUI (`handle_enter`, `evaluate_policy_and_dispatch`, `confirm_policy_action`) | Yes | Accepted overrides include class+reason | None |
| REST (`execute_tool`, `preflight_tool`) | Yes | Never (REST never confirms) | `generate_correlation_id()` |
| MCP (`handle_request` tool call path) | Yes | Never (MCP never confirms) | JSON-RPC request id |
| Agent (`execute_scan_with_depth` preflight + dispatch gate) | Yes | Never (Agent never confirms) | None |

## Tracing Levels

- `Allow`, `Warn`, `Confirmed`: `tracing::info!`
- `Deny`, `ConfirmationRequired`: `tracing::warn!`

## Scope Audit

`ScopeAudit` captures scope provenance without serializing full rules:

- `source`: `ScopeSource` (DefaultEmpty, ConfigFile, CliScopeFile, GeneratedPreset)
- `path`: Optional path to scope file
- `allow_rule_count`: Number of allow rules
- `exclusion_rule_count`: Number of exclusion rules
- `explicit_manifest`: Whether scope has explicit provenance (required for automated profiles)

## Design Decisions

- **UUID v4 for event_id**: Stable, unique, no coordination required
- **Optional policy_hash**: Reserved for future use when stable serialization is guaranteed
- **No full payload serialization**: Audit events capture decisions, not request payloads
- **Tracing-based emission**: Uses `tracing::info!`/`warn!` for structured logging; no separate audit database
- **Purely observational**: Audit emission never changes control flow or return values
