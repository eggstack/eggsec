# NSE Expansion Phase 01: TUI Report Rendering

## Purpose

Expose structured NSE results in the TUI so manual users can understand script outcomes without reading raw JSON or CLI prose.

The implementation should consume `NseRunReport` or `ReportEnvelope` directly. Do not scrape human-formatted output.

## Non-Goals

Do not redesign the TUI architecture.

Do not change NSE execution semantics.

Do not add new NSE script compatibility.

Do not invent vulnerability findings from raw script output.

Do not remove JSON/CLI report output.

## Workstream 1: Locate Current TUI Report Surfaces

Inspect current TUI crates/modules and identify:

- where scan/script results are displayed;
- whether domain-specific report panels already exist;
- how errors/warnings/evidence are represented;
- how keyboard navigation and focus panes are structured;
- how manual CLI/TUI mode invokes NSE execution.

### Acceptance Criteria

- Implementation notes identify the concrete files/modules to modify.
- No report model duplication is introduced.

## Workstream 2: Define NSE TUI View Model

Map structured report fields into UI sections:

1. Summary:
   - target;
   - script name;
   - profile;
   - source;
   - elapsed time.
2. Compatibility:
   - status;
   - fidelity;
   - unsupported features;
   - approximations.
3. Rule evaluation:
   - rule kind;
   - evaluated/matched state;
   - exactness;
   - context source;
   - errors.
4. Libraries:
   - name;
   - category;
   - loaded/attempted/failed state;
   - side effects;
   - warnings.
5. Capability denials:
   - kind;
   - target;
   - reason;
   - allowed/denied.
6. Evidence:
   - kind;
   - title;
   - confidence;
   - summary;
   - source fields.
7. Raw output:
   - script output;
   - truncation state if needed.
8. Diagnostics:
   - resolver diagnostics;
   - errors;
   - warnings.

### Acceptance Criteria

- The view model uses existing fields from `NseRunReport` / `ReportEnvelope`.
- The view model does not require prose parsing.

## Workstream 3: Implement Initial TUI Panel

Add a first-pass NSE report panel with:

- summary header;
- compatibility badge/status text;
- collapsible or sectioned areas for rules/libraries/denials/evidence/output;
- visible errors/warnings;
- raw output available but not dominant.

### Acceptance Criteria

- Manual users can see whether a script was compatible, partial, unsupported, or failed.
- Capability denials are prominent and not confused with target vulnerabilities.
- Evidence items are visible as observations.

## Workstream 4: Report Selection and Data Flow

Ensure the panel can render:

- a single report from a manual NSE run;
- multiple reports from a scan batch if already supported;
- empty/no-report states;
- failed execution states.

### Acceptance Criteria

- No panic on empty or partial reports.
- No data loss between report generation and rendering.

## Workstream 5: Tests

Add tests appropriate to the current TUI architecture:

- pure view-model tests if TUI rendering is hard to snapshot;
- widget rendering smoke tests if existing patterns support it;
- sample reports for compatible, partial, denied, and failed cases.

### Acceptance Criteria

- Tests cover at least one compatible report and one denial-heavy report.
- Tests assert that raw output and evidence remain separate.

## Workstream 6: Docs

Update:

- `architecture/nse_report_display_contract.md`;
- TUI docs or README sections if applicable;
- agent guidance for future NSE UI changes.

### Acceptance Criteria

- Docs state the TUI consumes structured reports.
- Docs state raw output is not evidence by itself.

## Verification

Run:

```bash
cargo check --workspace
cargo test -p eggsec-nse --features nse --test runtime_smoke_tests
cargo test -p eggsec-nse --features nse --test format_tests
cargo test --workspace
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
```

Adjust workspace commands if the repo has narrower standard verification commands.

## Final Acceptance Criteria

Phase 01 is complete when:

- the TUI has an NSE report view;
- the view consumes structured report fields;
- compatible/partial/denied/error states are visible;
- evidence and raw output are visually distinct;
- tests cover the view model or rendering path;
- docs describe the TUI report contract.
