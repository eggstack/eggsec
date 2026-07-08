# Workflow Module

## Purpose

Finding lifecycle management including status transitions, assignment, comments, and SLA tracking. Manages the operational workflow of security findings from discovery to resolution.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WorkflowReport` | `workflow/mod.rs` | Workflow metrics (open/in-progress/resolved counts, SLA violations) |
| `Finding` | `workflow/finding.rs` | Finding record with status transitions |
| `FindingStatus` | `workflow/finding.rs` | Status enum: Open, InProgress, Resolved, Verified, FalsePositive |
| `StatusWorkflow` | `workflow/status.rs` | State machine enforcing valid status transitions |
| `Assignment` | `workflow/assignment.rs` | Finding assignment record (`notes: Option<String>`) |
| `Comment` | `workflow/comments.rs` | Finding comment (`is_internal: bool`) |
| `SlaPolicy` | `workflow/sla.rs` | SLA policy definition (severity→hours) |
| `SlaStatus` | `workflow/sla.rs` | SLA tracking and violation detection |

## Status Transitions

```
Open → InProgress, FalsePositive
InProgress → Resolved, Open
Resolved → Verified, Open
Verified → Open
FalsePositive → Open
```

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `WorkflowReport` with metrics calculation |
| `finding.rs` | Finding management with validated status transitions |
| `status.rs` | Status workflow transitions (Open, InProgress, Resolved, etc.) |
| `assignment.rs` | Finding assignment to team members |
| `comments.rs` | Comment thread management on findings |
| `sla.rs` | SLA tracking and violation detection |

## Design Notes

- `Finding::update_status()` validates transitions via `StatusWorkflow` before applying
- `WorkflowReport::calculate_metrics()` computes all fields from the findings list
- `assign_finding()` and `add_comment()` return their types directly (no Result wrapper)
- SLA defaults: Critical=24h, High=168h, Medium=720h, Low=2160h, Info=8760h
- Feature-gated on `finding-workflow`

## Implementation Status

Fully implemented. Status transitions, assignment, comments, and SLA tracking are all functional.
