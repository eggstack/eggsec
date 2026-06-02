# Workflow Module Architecture Review

**Document:** architecture/workflow.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 31

## Verified Claims
- [WorkflowReport]: Verified at `crates/slapper/src/workflow/mod.rs:24` with metrics calculation
- [FindingStatus]: Verified at `crates/slapper/src/workflow/finding.rs:19-25` (Open, InProgress, Resolved, Verified, FalsePositive)
- [Assignment]: Verified at `crates/slapper/src/workflow/assignment.rs:6`
- [Comment]: Verified at `crates/slapper/src/workflow/comments.rs:6`
- [SlaPolicy]: Verified at `crates/slapper/src/workflow/sla.rs:5`
- [SlaStatus]: Verified at `crates/slapper/src/workflow/sla.rs:48`
- [Files: mod.rs, finding.rs, status.rs, assignment.rs, comments.rs, sla.rs]: Verified

## Discrepancies
- None significant.

## Bugs Found
- None found.

## Improvement Opportunities
- [SLA calculation ignores resolved/false-positive/verified findings]: `crates/slapper/src/workflow/mod.rs:38-48` only checks `FindingStatus::Open` for SLA violations. Resolved findings that were open but now resolved still had SLA violations that went untracked. This may be intentional but could hide violations that occurred before resolution (priority: low)
- [No actual persistence in Assignment/Comment]: The `Assignment::new()` and `Comment::new()` functions create in-memory structs but don't persist to a database. If the application restarts, all assignments and comments are lost. The storage module has `StoredFinding` but no `StoredAssignment` or `StoredComment` (priority: medium)

## Stale Items
- None.

## Code Interrogation Findings
- [StatusWorkflow::can_transition() missing transitions]: The allowed transitions at `status.rs:7-18` don't include FalsePositive as a valid target from any state. A finding marked as false positive cannot be re-opened or transitioned to any other state. This may be intentional (false positives are terminal) but should be documented.
- [SLA policies hardcoded]: `sla.rs:11-34` defines default policies with hardcoded hour values (Critical=24h, High=168h, Medium=720h, Low=2160h, Info=8760h). These should be configurable per organization.
- [WorkflowReport::calculate_metrics() iterates findings twice]: Line 38-47 filters for `FindingStatus::Open` then filters again for SLA violation. This is O(2n). Could be done in single pass.