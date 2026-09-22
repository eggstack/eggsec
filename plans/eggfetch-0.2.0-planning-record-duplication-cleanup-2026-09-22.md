# Eggfetch 0.2.0 planning-record duplication cleanup

Status: Ready for handoff

Date: 2026-09-22

Baseline: `dd414fa3749f8f7c307a17b78ccf2b613bebb24c`

Parent plan:
[eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md](eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md)

## Purpose

Correct the duplicated `## Exit criterion` section in the executed Eggfetch
0.2.0 adoption plan without altering the implementation record, qualification
evidence, historical conclusions, or current transport state.

The duplicate is documentation-only. The Eggfetch 0.2.0 adoption itself is
already executed and qualified; this pass must not reopen or reinterpret that
work.

## Confirmed defect

At baseline `dd414fa`,
`plans/eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md`
contains two consecutive, byte-identical `## Exit criterion` sections
immediately before:

```text
## Completion record (executed 2026-09-22)
```

Only one exit-criterion section is intended.

The completion record already contains the final implementation SHA, hosted CI
and Deep Checks evidence, dependency checksums, test results, architecture
guard disposition, and residual debt. Those records are authoritative and must
remain intact.

## Required change

Edit only:

```text
plans/eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md
```

Remove exactly one of the two duplicate `## Exit criterion` sections.

Do not:

- rewrite the remaining exit criterion;
- edit the completion record;
- alter recorded SHAs, checksums, workflow run IDs, test counts, or conclusions;
- change the historical 0.1.7 references;
- change the recorded residual-debt statement;
- update transport source, Cargo manifests, lockfiles, guards, architecture
  docs, or tests;
- conflate this docs-only cleanup with the separate TUI warning-debt cleanup.

## Verification

After the edit:

1. the plan contains exactly one `## Exit criterion` heading;
2. the surviving exit-criterion body is unchanged from the executed record;
3. `## Completion record (executed 2026-09-22)` follows it exactly once;
4. the completion-record body is byte-for-byte unchanged;
5. the rest of the plan diff contains no unrelated edits;
6. Markdown links remain valid relative links.

Suggested checks:

```sh
rg -n '^## Exit criterion$' plans/eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md
git diff --check
git diff -- plans/eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md
```

Expected result for the first command: exactly one match.

No Rust build or test run is required for this docs-only correction unless
repository policy has changed by implementation time.

## Acceptance criteria

This pass is complete when:

1. exactly one `## Exit criterion` remains;
2. no completion evidence is changed;
3. no production or test source is changed;
4. the diff is limited to removal of the duplicate block;
5. the plan remains marked executed;
6. the cleanup commit is recorded below.

## Completion record template

Append after implementation:

```text
Status: Executed
Starting SHA:
Cleanup SHA:
Diff scope:
Exit criterion heading count:
Completion record changed: no
Other files changed: none
Verification:
```

## Exit criterion

The documentation defect is closed when the executed Eggfetch 0.2.0 plan has a
single exit criterion and its implementation/qualification record remains
otherwise unchanged.
