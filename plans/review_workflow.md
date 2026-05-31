# Workflow Architecture Review
**Document:** architecture/workflow.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 30

## Verified Claims
- `WorkflowReport` struct: Verified at `crates/slapper/src/workflow/mod.rs:22` with fields `total_findings`, `open_findings`, `in_progress_findings`, `resolved_findings`, `sla_violations`
- `FindingStatus` enum: Verified at `crates/slapper/src/workflow/finding.rs:19` with variants `Open`, `InProgress`, `Resolved`, `Verified`, `FalsePositive`
- `Assignment` struct: Verified at `crates/slapper/src/workflow/assignment.rs:6` with fields `id`, `finding_id`, `user_id`, `assigned_at`, `assigned_by`, `notes`
- `Comment` struct: Verified at `crates/slapper/src/workflow/comments.rs:6` with fields `id`, `finding_id`, `user_id`, `content`, `created_at`, `is_internal`
- `SlaTracking`: Documented as `SlaTracking`. Actual types are `SlaPolicy` (`sla.rs:5`) and `SlaStatus` (`sla.rs:48`). There is no struct named `SlaTracking`.
- Status workflow transitions: Verified at `crates/slapper/src/workflow/status.rs:7` with `StatusWorkflow::can_transition()` implementing a state machine
- Finding management operations: Verified at `crates/slapper/src/workflow/finding.rs:27` with `Finding::new()`, `assign()`, `update_status()`
- Assignment to team members: Verified at `crates/slapper/src/workflow/assignment.rs:35` with `assign_finding()` function
- Comment thread management: Verified at `crates/slapper/src/workflow/comments.rs:35` with `add_comment()` function
- SLA tracking and violation detection: Verified at `crates/slapper/src/workflow/sla.rs:57` with `calculate_sla()` function
- All files present: `mod.rs`, `finding.rs`, `status.rs`, `assignment.rs`, `comments.rs`, `sla.rs` - verified

## Discrepancies
- **`SlaTracking` type name**: Documented as `SlaTracking` at `workflow/sla.rs`. Actual types are `SlaPolicy` (`sla.rs:5`) and `SlaStatus` (`sla.rs:48`). No `SlaTracking` struct exists.
- **`WorkflowReport` location**: Documented as `workflow/mod.rs`. Verified correct, but the `calculate_metrics()` method (`mod.rs:35`) simply sets `sla_violations = open_findings`, which is a trivial/incorrect calculation. All open findings are counted as SLA violations regardless of their actual SLA status.

## Bugs Found
- **Incorrect SLA violation calculation**: `WorkflowReport::calculate_metrics()` at `crates/slapper/src/workflow/mod.rs:36` sets `self.sla_violations = self.open_findings`. This incorrectly treats all open findings as SLA violations. The actual SLA calculation uses `SlaPolicy::get_policy()` and `calculate_sla()` in `sla.rs:57-75`, which considers severity-based target hours and actual time elapsed. `calculate_metrics()` does not use these.

## Improvement Opportunities
- Fix `calculate_metrics()` to use `calculate_sla()` from `sla.rs` for accurate SLA violation counting
- Rename `SlaTracking` in documentation to `SlaPolicy` and `SlaStatus`
- Document `StatusWorkflow` struct (`status.rs:4`), `AssignmentRequest` (`assignment.rs:29`), `CommentRequest` (`comments.rs:29`), `RemediationPriority` if applicable
- The `Finding` struct in `workflow/finding.rs` has different fields than the global `Finding` in `findings.rs` - consider clarifying the relationship

## Stale Items
- `SlaTracking` type name is stale - should be `SlaPolicy` and `SlaStatus`
