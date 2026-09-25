# Archive

Completed, superseded, or abandoned interim planning is retained here for traceability once it no longer represents active work.

## Rules

- Archive moves MUST preserve traceability and SHOULD retain original filenames and subsystem grouping.
- Grandfathered flat-era plans (2026-07 through 2026-09) remain at the `plans/` top level under their original filenames because `architecture/*.md` links to those paths. They are the retained engineering record; do not move them here merely to tidy the directory.
- What belongs here: superseded drafts, abandoned proposals, and interim documents whose content has been fully subsumed by a newer plan that links back to them.
- Canonical long-term documents (`plans/000-*.md`, `plans/001-*.md`, `plans/002-*.md`, `plans/003-*.md`) and accepted ADRs MUST NOT be archived merely because their initial implementation completed.

## Retention policy

Plans in `plans/` are retained (mark `Status: Executed` for completed flat-era plans, or record a closure file for new-style milestones). Do not delete useful handoff history solely to satisfy a static guard.
