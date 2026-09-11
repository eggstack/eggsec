---
name: eggsec-findings-workflow
description: "Finding lifecycle and workflow management - use when working with the canonical Finding schema, finding stores, status transitions, assignment, comments, or SLA tracking."
---

# Eggsec Findings & Workflow Skill

Canonical finding schema plus the human workflow (status, assignment, SLA) built on top of it.

## Modules

**`crates/eggsec/src/findings/`** — canonical schema (target model for unification):

| File | Contents |
|------|----------|
| `mod.rs` | `Finding` (canonical record), `Confidence`, `EvidenceKind`, `Evidence`, `AffectedAsset`, `FindingLocation`, `Reproduction`, `FindingType`, `FindingSource` |
| `lifecycle.rs` | Lifecycle states for the canonical finding |
| `store.rs` | Persistence for canonical findings |

**`crates/eggsec/src/workflow/`** — operator workflow (assignment, comments, SLA):

| File | Contents |
|------|----------|
| `mod.rs` | `WorkflowReport` (totals + open/in-progress/resolved + SLA violations) |
| `finding.rs` | Workflow-side `Finding` / `FindingStatus` |
| `status.rs` | Status transition rules |
| `assignment.rs` | Finding assignment |
| `comments.rs` | Finding comments |
| `sla.rs` | `calculate_sla()` — SLA computation |

> Note (`findings/mod.rs` docs): module-specific finding types (`tool::finding::Finding`,
> `output::agent::AgentFinding`, `workflow::finding::Finding`) are NOT yet migrated to the
> canonical schema. New code should target the canonical `findings::Finding`; do not widen
> the divergence by inventing another parallel type.

## Operation & Gating

- Operation id `workflow` (`config/policy_catalog.rs`): `StandardAssessment` / `SafeActive`, requires feature `finding-workflow` and explicit scope.
- TUI tabs `Workflow` (feature `finding-workflow`) and `Storage` (feature `database`, `WorkflowReport`-adjacent persistence).

## Common Tasks

### Adding a Finding Field
1. Add it to the canonical `findings::Finding` first.
2. Update `store.rs` serialization and `lifecycle.rs` if the field affects states.
3. Migrate consumers one at a time; keep backward-compatible deserialization (`#[serde(default)]`).

### Adding a Status Transition
1. Encode the rule in `workflow/status.rs` (single owner of transition legality).
2. Illegal transitions must be rejected with a typed error, not silently ignored.
3. Update SLA handling in `sla.rs` if resolution semantics change.

## Resources
- `architecture/findings.md` - canonical schema deep-dive
- `architecture/workflow.md` - workflow/lifecycle deep-dive
- `architecture/overview.md` (Module Index) - findings + workflow rows
