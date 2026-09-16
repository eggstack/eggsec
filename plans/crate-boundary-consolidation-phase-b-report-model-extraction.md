# Phase B — Report model extraction

Status: Executed (2026-09-16). All workstreams implemented; see Completion record below.

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

## Completion record

Executed 2026-09-16.

- Baseline SHA: `ee0bffe124c7573016bd3ef9f71561fb7cc07d55` (Phase A
  implementation head; working tree clean at start).
  Final implementation SHA: recorded in the commit history for this plan
  (implementation commit + this record; `git log --oneline --
  plans/crate-boundary-consolidation-phase-b-report-model-extraction.md`).
  Workspace member count 18 -> 19 (no other member added/removed).
- Gate B0 payoff (verified by `rg`, not assumed): every `eggsec-output`
  import in the four domain crates classified as pure report/evidence DTO —
  zero rendering/conversion (`load_scan_report`, `convert_to_*`), zero
  baseline/trend/dedup, zero persistence/I-O imports:
  - `eggsec-db-lab/src/bridge.rs`: `convert::{FindingData, ScanReportData}` +
    `envelope::{BaselineSummary, EvidenceItem, EvidenceKind, EvidenceSource,
    FindingRecord, RedactionState, ReportEnvelope, ToolMetadata}`;
  - `eggsec-mobile-lab/src/{lib,dynamic}.rs`: `convert::{FindingData,
    ScanReportData}` + `envelope::{EvidenceItem, EvidenceKind,
    EvidenceSource, FindingRecord, RedactionState, ReportEnvelope,
    ToolMetadata}`;
  - `eggsec-web-proxy/src/intercept/bridge.rs`:
    `convert::{FindingData, ScanReportData}`;
  - `eggsec-nse/src/bridge.rs` (+ `tests/{bridge,evidence}_tests.rs`):
    `envelope::{EvidenceItem, EvidenceKind, EvidenceSource, FindingRecord,
    RedactionState, ReportEnvelope, ToolMetadata}`.
  All four proceed to the model; none retains an `eggsec-output` edge.
- Exact types moved (verbatim, same serde attrs) into
  `crates/eggsec-report-model/src/`:
  - `report.rs`: `ScanReportData`, `FindingData`, `PortData`, `ServiceData`,
    `WirelessNetworkReportData` (from `eggsec-output::convert`; conversion
    functions, `load_scan_report`, and all `From` impls stay in output);
  - `envelope.rs`: `EvidenceKind` (+ intrinsic `Display`), `EvidenceSource`,
    `RedactionState`, `RedactionPolicy`, `EvidenceItem`, `EvidenceManifest`,
    `FindingRecord`, `BaselineSummary`, `ToolMetadata`, `ReportEnvelope`
    (the `From<&AgentFinding> for FindingRecord` impl stays in output — it
    couples the contract to an output-side type the model must not import);
  - `summary.rs`: `PolicySummary`, `DiffSummary` (pure DTOs only).
  - Deliberately NOT moved (minimal extraction per WS3): `agent.rs` wholesale
    (`AgentFinding` family stays; output-local + engine consumers only),
    `baseline::BaselineComparison`, `trend::{ResultComparator,
    TrendAnalyzer}` (+ LRU history and all trend DTOs — no external producer
    needs them independently), `audit_summary` struct + aggregation (kept
    together to avoid splitting struct from behavior; no domain consumer).
- Dependencies of `eggsec-report-model` (direct, depth-1 `cargo tree`):
  `chrono`, `eggsec-core`, `serde`, `serde_json`, `uuid` only. No `tokio`,
  `quick-xml`, `hostname`, `lru`, `rustc-hash`, `unicode-normalization`,
  `reqwest`/`rustls`, frontend, engine, or `eggsec-output` edge.
  `serde_json` is used for the canonical in-memory `to_json`/`from_json`
  helpers (no `serde_json::Value` fields, no file loading); `chrono`/`uuid`
  back the already-present `DateTime<Utc>` fields and `new()`/`Default`
  constructors (report identity/timestamps are intrinsic to the DTO, not
  host/environment discovery — no `hostname`/fs/env behavior moved).
  `BaselineSummary::severity_deltas` uses `std::collections::HashMap`
  (no `rustc-hash` leak into the stable contract).
- Consumer edge before -> after:
  - `eggsec-db-lab`: `eggsec-output` -> `eggsec-report-model` (dep removed).
  - `eggsec-mobile-lab`: `eggsec-output` -> `eggsec-report-model` (dep removed).
  - `eggsec-web-proxy`: `eggsec-output` -> `eggsec-report-model` (dep removed).
  - `eggsec-nse`: `eggsec-output` -> `eggsec-report-model` (dep removed).
  - `eggsec-output`: + `eggsec-report-model` (correct direction; never reverse).
  - `eggsec` (engine): unchanged (`eggsec-output` only; DTOs flow through the
    re-export facade — no new edge, no cycle).
- Transitive dependency deltas (depth-1 `cargo tree` before -> after):
  - `eggsec-db-lab` loses `eggsec-output` and with it the transitive
    `hostname`, `lru`, `quick-xml`, `rustc-hash`, `unicode-normalization`,
    `uuid` renderer-only surface (keeps its own direct `chrono`/`tokio`/etc.).
  - `eggsec-mobile-lab` loses the `eggsec-output` edge (`hostname`, `lru`,
    `unicode-normalization`, `rustc-hash` via output gone; keeps its own
    direct `quick-xml` for manifest parsing).
  - `eggsec-web-proxy`, `eggsec-nse`: `eggsec-output` edge gone (DTOs now via
    the 5-dep model instead of the 11-dep renderer).
  - `cargo tree -d` shows no new duplicate-version conflicts attributable to
    this phase (model reuses the workspace `chrono`/`uuid`/`serde` pins).
- Serialization compatibility evidence:
  - Temporary `eggsec-output` example dumped representative fixtures BEFORE
    the move (scan report with evidence/remediation/CWE, ports, services,
    wireless, policy summary; `cve_ids` alias probe; envelope with evidence +
    policy + baseline + tool metadata + redaction policy; diff + audit
    shapes) to `/tmp/phase-b-before.txt`; re-ran the identical dumper AFTER
    the move (paths resolve to the model via re-exports) to
    `/tmp/phase-b-after.txt`: `diff` reports FIXTURES IDENTICAL
    (byte-equivalent). Example deleted after verification (not retained).
  - New `crates/eggsec-report-model/tests/roundtrip.rs` (11 tests,
    independent of `eggsec-output`): full report shape + skip rules,
    `cve_ids` alias, wireless `#[serde(default)]` flags, policy/diff
    round-trips, `snake_case` enum renames, intrinsic `Display`, builder APIs,
    severity preservation, manifest redaction counts, baseline flags,
    manifest refresh.
  - Existing `eggsec-output` unit + `tests/report_envelope.rs` suites pass
    unchanged through the re-export facade (93 tests across model+output).
- Compatibility re-exports/API changes (all pre-1.0; dependency-safe, no
  cycle): `eggsec_output::{convert,envelope,policy_summary,diff}` modules
  `pub use` the model types, so `eggsec_output::ScanReportData`,
  `eggsec_output::convert::ScanReportData`, `eggsec::output::*` and all
  envelope paths keep resolving. `RedactionPolicy` was previously reachable
  only via `eggsec_output::envelope::` (never root-re-exported) — unchanged.
  No engine public-path change except a doc comment. Domain-bridge function
  signatures are unchanged apart from the canonical type paths.
- Package/release graph changes: root workspace members +1
  (`crates/eggsec-report-model`); `python
  scripts/release-package-graph.py validate` passes; `order` is acyclic with
  `eggsec-report-model` before all consumers (`core, agent, report-model,
  db-lab, mobile-lab, nse, output, runtime, daemon-protocol, tool-core,
  transport, transport-eggfetch, ui-model, web-proxy, eggsec, daemon`).
  `docs/RELEASING.md` validated order refreshed (also fixed pre-existing
  drift: transport crates were missing from the list). `cargo package -p
  eggsec-report-model --no-verify` is not locally runnable for ANY leaf
  crate (fails identically for pre-existing `eggsec-tool-core`: unpublished
  path dep `eggsec-core`); workspace packaging is validated by
  `make release-check` on the clean tree instead.
- Architecture guard changes (`scripts/check-architecture-guards.sh`, Checks
  118–120; `docs/CI_ARCHITECTURE_GUARDS.md` documents them):
  - 118: model crate exists, manifests none of
    `tokio`/renderers/TLS/frontend/engine/`eggsec-output` deps, and `src/`
    has no `std::fs`/`tokio::`/`reqwest::`/`quick_xml`/`hostname::`/`LruCache`
    uses (code-structure match; doc prose may name the consumer).
  - 119: output manifests the model, model never references output, no moved
    DTO is redefined under `eggsec-output/src/`, facades re-export the model.
  - 120: all four domain crates manifest the model, carry no `eggsec-output`
    dep, and contain no `eggsec_output::` imports in `src/`/`tests/`.
- Commands/results (all green locally before commit):
  - `cargo fmt --all --check` — pass (after `cargo fmt --all`).
  - `make check` (exit 0): fmt, `--workspace --no-default-features`,
    `-p eggsec`, `-p eggsec-cli` (+ `--no-default-features`), `check-deps`
    (deny ×5), `clippy` (now incl. `-p eggsec-report-model`, `-D warnings`),
    engine doc tests (21), `tool_registration`+`loadtest_tests` (29), engine
    `rest-api,cli` suite, `eggsec-output` (82) + `eggsec-report-model` (11),
    `transport-eggfetch`, `eggsec-tui --lib`, guards ALL PASSED (99–120).
  - `cargo test -p eggsec-db-lab -p eggsec-mobile-lab -p eggsec-web-proxy` —
    525 passed, 1 ignored.
  - `cargo test -p eggsec-nse --features nse --tests` — 554 passed.
  - `make check-feature-profiles` — pass (incl. 936 TUI profile tests).
  - `make release-check` — run on the clean post-commit tree (see below).
  - `make check-features-individual` is deep-checks-only per `AGENTS.md` and
    was not run per-PR (same precedent as Phase A).
  - `make check-python` not run: no Python bindings/stubs/docs/scripts
    changed (engine diff is a doc comment; `AGENTS.md` scopes it to Python
    changes).
- Residual model/output ownership debt:
  - `trend::{ScanResult, ResultSummary, Finding, ComparisonResult,
    TrendAnalysis}` DTOs stay in output with the analyzer (no independent
    producer; revisit only with consumer evidence).
  - `AuditSummary` struct stays with its aggregation in output (same reason).
  - `From<&AgentFinding>` conversions stay in output (model must not import
    output-side types); if `AgentFinding` is ever canonicalized, revisit.
  - `eggsec` engine still reaches DTOs via the output facade rather than a
    direct model edge (no payoff to a direct edge today; revisit if the
    engine sheds its output dependency).
  - Pre-existing stale notes untouched as out of scope:
    `architecture/api_extraction_boundary.md:292` ("CronScheduler already in
    eggsec-output" — moved to `eggsec-agent::cron` in Phase A) and the
    `schedule.rs`/`session.rs` rows in
    `crates/eggsec/src/output/AGENTS.override.md` (ditto).
