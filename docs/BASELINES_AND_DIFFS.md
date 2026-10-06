# Baselines and Differential Scans

> Status: the `eggsec report diff` subcommand described in earlier revisions
> of this document does **not** exist. `eggsec report` currently offers
> `convert`, `trend`, and `schedule` only
> (`crates/eggsec/src/cli/misc.rs:216`). Compare scans over time with the
> per-command `--baseline` flags (`mobile dynamic`, `db pentest`) and the
> trend/baseline/diff analysis in `eggsec-output` — see
> `architecture/output.md`.

Comparing scan results over time is done in one of three ways.

## 1. `eggsec report trend` (CLI)

Compare two result files in chronological order:

```bash
eggsec report trend before.json after.json -o trend.json
```

`ReportTrendArgs` takes exactly two inputs — `before` and `after`
(`crates/eggsec/src/cli/misc.rs`). This is the CLI surface for
cross-run comparison. There is no `diff` sibling; for a two-point
comparison, `trend` is the tool.

## 2. Per-command `--baseline` flags

Several domain commands take a baseline path directly and emit regression
findings alongside the main report:

| Command | Flags | Behavior |
|---------|-------|----------|
| `eggsec mobile dynamic` | `--baseline <PATH>` | Compares the run against a prior `MobileBaseline` JSON and emits regression notes plus optional `behavioral-regression` findings |
| `eggsec db pentest` | `--baseline <PATH>`, `--baseline-label <LABEL>`, `--baseline-output <PATH>`, `--capture-baseline` | Compares against a `DbBaseline`, emitting `DbRegressionResult` (new/resolved findings, severity increases/decreases) |

```bash
# Capture a baseline, then compare a later run against it
eggsec db pentest "$DSN" --dry-run --capture-baseline --baseline-output baseline.json
eggsec db pentest "$DSN" --dry-run --baseline baseline.json --json -o after.json
```

The fuzz surface has separate response-diffing switches (`--diffing`,
`--capture-baseline`) that compare individual HTTP responses during a
session rather than whole-report baselines.

## 3. `eggsec-output` analysis library

Programmatic comparison lives in `eggsec-output` and operates on the
canonical contracts owned by `eggsec-report-model`:

| File | Purpose |
|------|---------|
| `crates/eggsec-output/src/trend.rs` | Multi-run trend analysis |
| `crates/eggsec-output/src/baseline.rs` | `BaselineComparison` — finding-level new/resolved/unchanged classification by id |
| `crates/eggsec-output/src/diff.rs` | Re-exports `DiffSummary` from `eggsec-report-model` (canonical owner) — numeric diff envelope for `RunManifest` |

`BaselineComparison::compare()` classifies findings by `AgentFinding.id`:
ids present in the current run but not the baseline are **new**, ids in the
baseline but not the current run are **resolved**, and ids in both are
**unchanged**.

Domain-specific baseline formats remain domain-owned and structurally
different — see [REPORT_EVIDENCE_MODEL.md](REPORT_EVIDENCE_MODEL.md) for
the `DbBaseline` / `MobileBaseline` / `BundleDiff` comparison.