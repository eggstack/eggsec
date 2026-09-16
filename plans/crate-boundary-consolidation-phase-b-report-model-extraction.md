# Phase B — Report model extraction

Status: Ready for handoff

Date: 2026-09-16

Depends on: Phase A

Roadmap: `crate-boundary-consolidation-roadmap-2026-09-16.md`

## Purpose

Create a dependency-light `eggsec-report-model` crate containing the stable serializable report/evidence data contracts that domain crates exchange, while leaving rendering, filesystem loading, format conversion, trend/baseline analysis, and presentation behavior in `eggsec-output`.

This is the highest-confidence new crate in the roadmap because several domain crates currently depend on `eggsec-output` for report/finding DTOs. The extraction is accepted only if those consumers can stop depending on formatter/orchestration behavior they do not use.

## Why this boundary is different from prior rejected extractions

This is not a module-count split. The current dependency direction already demonstrates a shared data contract:

- `eggsec-db-lab` depends on `eggsec-output` for `ScanReportData`;
- `eggsec-mobile-lab` depends on `eggsec-output` for report conversion/data;
- `eggsec-web-proxy` depends on `eggsec-output` for `ScanReportData` / `FindingData` and normalized evidence types;
- `eggsec-output` itself mixes DTO definitions with HTML/JUnit/SARIF/CSV/Markdown rendering and related dependencies.

A pure report-model crate lets producers depend on the contract without paying for or semantically coupling to the renderer crate. That is a real dependency-direction improvement.

## Gate B0 — Prove the dependency payoff before creating the crate

Before adding the workspace member, capture:

```text
cargo tree -p eggsec-output
cargo tree -p eggsec-db-lab
cargo tree -p eggsec-mobile-lab
cargo tree -p eggsec-web-proxy
cargo tree -d
```

Then inventory every symbol imported from `eggsec-output` by each domain crate. Classify each import as:

- pure report/evidence DTO;
- rendering/conversion API;
- baseline/trend/dedup analysis;
- persistence/I/O;
- accidental/unused.

Proceed with `eggsec-report-model` only if at least one real consumer can replace its `eggsec-output` dependency with the model crate or materially narrow the use of `eggsec-output`. The expected result is all three domain crates above for their pure DTO paths, but the completion record must report actual findings rather than assume this.

## Target crate

Proposed package:

```text
crates/eggsec-report-model/
  Cargo.toml
  src/
    lib.rs
    report.rs
    finding.rs        # only if a split improves ownership
    envelope.rs
    summary.rs
```

The exact internal file names are flexible. The crate boundary is not.

### Allowed responsibilities

- serializable report DTOs;
- serializable normalized finding DTOs;
- evidence/provenance/redaction-state DTOs;
- policy/audit/baseline/diff summary DTOs when they contain data only;
- data-only enums and stable conversion-free identifiers;
- `Display`/simple parsing only where it is intrinsic to the data type and does not pull presentation dependencies.

### Forbidden responsibilities

- filesystem reads/writes;
- JSON file loading helpers;
- HTML/Markdown/CSV/JUnit/SARIF generation;
- XML dependencies;
- async I/O or Tokio;
- scheduling/cron/queues;
- terminal output/progress;
- hostname/environment discovery;
- LRU/cache behavior;
- engine policy evaluation;
- network access;
- domain execution.

## Workstream 1 — Split data from conversion in `eggsec-output::convert`

Today `convert.rs` defines `ScanReportData`, `FindingData`, `PortData`, `ServiceData`, and `WirelessNetworkReportData` alongside filesystem loading and conversion functions.

Move the pure data types into `eggsec-report-model` and leave these behaviors in `eggsec-output`:

- `load_scan_report`;
- `convert_to_junit`;
- `convert_to_sarif`;
- `convert_to_html`;
- `convert_to_markdown`;
- `convert_to_csv`;
- any renderer-specific adapters/builders.

`eggsec-output` should import the model types and continue to provide rendering functions over `&ScanReportData` or equivalent.

### Compatibility

Re-export moved DTOs from `eggsec-output` where doing so is dependency-safe:

```rust
pub use eggsec_report_model::{
    FindingData, PortData, ScanReportData, ServiceData,
    WirelessNetworkReportData,
};
```

This preserves existing `eggsec_output::ScanReportData`-style imports while making `eggsec-report-model` the canonical owner.

## Workstream 2 — Move the normalized evidence envelope

`eggsec-output/src/envelope.rs` already describes itself as a protocol-neutral contract for domain crates. That contract belongs in the model crate.

Candidate types include:

- `EvidenceKind`;
- `EvidenceSource`;
- `RedactionState`;
- `RedactionPolicy`;
- `EvidenceItem`;
- `EvidenceManifest`;
- `FindingRecord`;
- `ReportEnvelope`;
- `BaselineSummary`;
- `ToolMetadata`;
- any other struct in the module that is pure data.

Before moving each item, separate constructor/environment behavior from the struct definition. If a helper reads host/environment state, timestamps implicit process state, files, or other runtime facts, keep that helper in `eggsec-output` (or the actual producer) and construct the model explicitly.

Do not move output-specific redaction algorithms merely because the envelope has a `RedactionState`; the model describes redaction state, it does not own redaction execution.

## Workstream 3 — Move pure summaries, not analysis engines

Evaluate these modules individually:

- `policy_summary.rs` — expected move: pure serializable summary DTO;
- `audit_summary.rs` — move if data-only;
- `diff.rs` — move `DiffSummary` if data-only, retain comparison behavior in output;
- `baseline.rs` — retain analysis/comparison behavior in output; move only stable summary DTOs;
- `trend.rs` — retain trend computation in output; move DTOs only if multiple producers need them independently;
- `agent.rs` — do not move wholesale. Move types only if they are truly report-contract types consumed outside output.

The extraction should be minimal. Do not use Phase B to reorganize every struct in `eggsec-output`.

## Workstream 4 — Define a deliberately small dependency set

Target `eggsec-report-model` dependencies should be limited to data concerns. Expected candidates:

```text
eggsec-core       # only if shared primitive types are actually used
serde
serde_json        # only if Value is part of the stable model
chrono            # only for typed timestamps already represented this way
uuid              # only if UUID is a field type rather than String
rustc-hash        # only if serialized contract actually requires the map type
```

Prefer standard collections in a public serialized model unless a specialized map type is required for compatibility/performance. Avoid leaking implementation-specific containers into a stable contract without reason.

The crate must not depend on:

```text
tokio
quick-xml
hostname
lru
reqwest
rustls
axum/tonic
clap/ratatui/crossterm
eggsec
eggsec-output
```

Add an architecture guard for the forbidden workspace/dependency directions.

## Workstream 5 — Migrate producers to the canonical model

For each producer crate:

### `eggsec-db-lab`

Replace pure report DTO imports from `eggsec-output` with `eggsec-report-model`. Remove the `eggsec-output` dependency if no formatter/analysis API remains in use.

### `eggsec-mobile-lab`

Perform the same migration. Keep `eggsec-output` only if the domain crate genuinely renders output itself; prefer returning model values and letting the host/output crate render them.

### `eggsec-web-proxy`

Move report/evidence DTO imports to the model crate. Do not change interception/TLS/network ownership as part of this phase.

### `eggsec`

Use the model crate as the canonical DTO owner. Preserve `eggsec::output` compatibility facades where appropriate through `eggsec-output` re-exports rather than duplicating types.

### Other consumers

Search the entire workspace. If a crate only consumes report data, point it at the model. If it renders, it may depend on both model and output.

## Workstream 6 — Preserve serialization compatibility

Before moving types, capture representative fixtures for:

- a normal scan report;
- findings with optional evidence/remediation/CWE fields;
- port/service data;
- wireless report data;
- a normalized evidence envelope;
- policy summary;
- baseline/diff summary if moved.

After extraction, prove that serialized JSON shape remains byte-equivalent where deterministic ordering permits, or semantic-JSON-equivalent otherwise. Preserve existing `serde(default)`, aliases, skip rules, enum rename conventions, and timestamp formats.

Add round-trip tests in `eggsec-report-model` independent of `eggsec-output`.

## Workstream 7 — Keep rendering above the model

Refactor `eggsec-output` to make the layering obvious:

```text
eggsec-report-model  <- data only
        ^
        |
eggsec-output        <- rendering/conversion/analysis
        ^
        |
process hosts / engine
```

Format-specific builders may define private/internal render models, but canonical findings/reports should originate from `eggsec-report-model`.

Do not move PDF/report modules from `eggsec` into the model merely to centralize output; engine-coupled formatters remain where their dependencies require unless separately justified.

## Workstream 8 — Update package/release/architecture machinery

Because a workspace member is added:

- add it to root workspace members and workspace path dependencies if useful;
- add package metadata consistent with other public leaf crates;
- update release package graph/order if the crate is publishable;
- update architecture overview/dependency map;
- update dependency/capability guard scripts;
- update docs that state `eggsec-output` is the canonical owner of report DTOs;
- ensure manual publication order accounts for `eggsec-report-model` before consumers.

Do not publish as part of this plan unless the repository's normal manual release process is separately invoked.

## Required verification

```text
cargo check -p eggsec-report-model
cargo test -p eggsec-report-model
cargo tree -p eggsec-report-model
cargo check -p eggsec-output
cargo test -p eggsec-output
cargo check -p eggsec-db-lab
cargo check -p eggsec-mobile-lab
cargo check -p eggsec-web-proxy
cargo check -p eggsec --no-default-features
cargo check --workspace --no-default-features
cargo package -p eggsec-report-model --no-verify
make check-feature-profiles
make check-features-individual
make test-architecture-guards
make check
make check-python
```

Also compare before/after `cargo tree` output for each migrated domain and record which transitive dependencies disappear or which unrelated crate edge is removed.

## Acceptance criteria

1. `eggsec-report-model` contains only data contracts and data-intrinsic behavior.
2. It has no Tokio, filesystem, network, renderer, frontend, or engine dependency.
3. `eggsec-output` depends on the model, never the reverse.
4. At least one domain crate removes its `eggsec-output` dependency; expected target is all DTO-only domain consumers.
5. No domain crate imports formatter behavior merely to construct a report DTO.
6. Existing JSON/report schema compatibility is preserved by fixture/round-trip tests.
7. `eggsec-output` re-exports moved DTOs when dependency-safe so ordinary downstream paths need not break immediately.
8. Scheduling and frontend-session behavior are not moved into `eggsec-report-model`.
9. Workspace/release/architecture guards encode the new dependency direction.
10. Full feature and Python/frontend checks remain green.

## Expected files touched

- new `crates/eggsec-report-model/`;
- root `Cargo.toml`;
- `crates/eggsec-output/src/{lib,convert,envelope,policy_summary,audit_summary,diff,...}.rs` as applicable;
- manifests/imports for `eggsec-db-lab`, `eggsec-mobile-lab`, `eggsec-web-proxy`, `eggsec`, and other actual consumers;
- architecture/release/guard documentation and scripts;
- serialization fixtures/tests;
- this plan completion record.

## Completion record template

Append after execution:

- Baseline SHA / final SHA
- Exact types moved
- Dependencies of `eggsec-report-model`
- Consumer edge before -> after
- Transitive dependency deltas
- Serialization compatibility evidence
- Compatibility re-exports/API changes
- Package/release graph changes
- Commands/results
- Residual model/output ownership debt
