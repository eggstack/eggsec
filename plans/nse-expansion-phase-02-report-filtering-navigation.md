# NSE Expansion Phase 02: Report Filtering and Navigation

## Purpose

Improve TUI usability for NSE reports after the initial rendering panel exists.

Users should be able to quickly find denials, errors, warnings, evidence, compatibility reasons, and raw script output without scanning a long report.

## Non-Goals

Do not alter NSE execution behavior.

Do not add new report fields unless a strong UI requirement exists.

Do not hide important safety denials by default.

Do not infer vulnerability severity beyond structured evidence semantics.

## Workstream 1: Navigation Model

Define navigation targets:

- summary;
- compatibility;
- rules;
- libraries;
- capability denials;
- evidence;
- errors;
- warnings;
- raw output;
- diagnostics.

Add keyboard shortcuts consistent with the rest of the TUI.

### Suggested Shortcuts

Use existing TUI conventions where available. Possible bindings:

- `g` / `G` for top/bottom if already standard;
- `/` search if supported;
- `d` jump to denials;
- `e` jump to evidence/errors depending existing conventions;
- `r` jump to raw output;
- `Tab` or arrow keys between sections.

### Acceptance Criteria

- Shortcuts do not conflict with existing critical bindings.
- Users can reach denials/errors/evidence quickly.

## Workstream 2: Filtering

Add filters for:

- all sections;
- denials only;
- errors/warnings only;
- evidence only;
- raw output;
- compatibility/rules.

If a filter is active, display the filter state clearly.

### Acceptance Criteria

- Filtering does not mutate reports.
- Empty filter results show a clear empty state.
- Denials are never hidden without visible filter indication.

## Workstream 3: Search

If the TUI already has search, integrate NSE sections with it. If not, add a minimal section-local search only if low risk.

Search should cover:

- evidence title/summary;
- capability denial kind/reason/target;
- library names;
- rule summaries/errors;
- raw output.

### Acceptance Criteria

- Search highlights or jumps to matches.
- Search works on raw output without treating raw output as evidence.

## Workstream 4: Detail Drilldown

Add a detail view or expanded mode for individual items:

- evidence item detail;
- capability event detail;
- library report detail;
- rule report detail;
- resolver diagnostic detail.

### Acceptance Criteria

- Detail view shows source fields and confidence where available.
- Denial detail includes policy reason.

## Workstream 5: Multi-Report Handling

If multiple NSE scripts run in one scan, provide:

- report list;
- status/fidelity summary per script;
- quick sort/group by status;
- jump from list to detail.

### Acceptance Criteria

- A failed/unsupported script is easy to identify.
- Batch reports remain navigable.

## Workstream 6: Tests and Fixtures

Add TUI/view-model tests for:

- filtering denials;
- filtering evidence;
- search matches;
- empty states;
- multi-report summary counts.

### Acceptance Criteria

- Tests use synthetic `NseRunReport` instances and do not require network.
- Tests cover at least one denied report and one compatible report.

## Workstream 7: Documentation

Update TUI docs with:

- report section definitions;
- keyboard shortcuts;
- filter behavior;
- evidence/raw-output distinction;
- profile-denial semantics.

## Verification

Run:

```bash
cargo test --workspace
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --workspace --all-targets
```

Adjust clippy scope if workspace clippy is not currently clean.

## Final Acceptance Criteria

Phase 02 is complete when:

- users can jump/filter/search NSE report sections;
- denial/evidence/error states are easy to locate;
- raw output remains available but distinct;
- multi-report handling is coherent if batch reports exist;
- tests cover view-model/filter behavior;
- docs describe usage.
