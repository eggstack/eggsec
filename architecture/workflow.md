# Workflow Module

## Purpose

Finding lifecycle management including status transitions, assignment, comments, and SLA tracking. Manages the operational workflow of security findings from discovery to resolution.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WorkflowReport` | `workflow/mod.rs` | Workflow metrics (open/in-progress/resolved counts, SLA violations) |
| `FindingStatus` | `workflow/status.rs` | Finding status state machine |
| `Assignment` | `workflow/assignment.rs` | Finding assignment record |
| `Comment` | `workflow/comments.rs` | Finding comment |
| `SlaPolicy` | `workflow/sla.rs` | SLA policy definition (severity→hours) |
| `SlaStatus` | `workflow/sla.rs` | SLA tracking and violation detection |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `WorkflowReport` with metrics calculation |
| `finding.rs` | Finding management operations |
| `status.rs` | Status workflow transitions (Open, InProgress, Resolved, etc.) |
| `assignment.rs` | Finding assignment to team members |
| `comments.rs` | Comment thread management on findings |
| `sla.rs` | SLA tracking and violation detection |

## Implementation Status

Fully implemented. Status transitions, assignment, comments, and SLA tracking are all functional with structured result types.
