# NSE Milestone 4 Phase 05: UX, Docs, Compatibility Matrix, and Release Closure

## Purpose

Close Milestone 4 by making expanded NSE compatibility understandable to users and maintainers through CLI/TUI output, docs, compatibility matrix, architecture guards, and verification records.

This phase should convert the lower-level corpus, context, and evidence work into a usable release surface.

## Non-Goals

Do not claim full Nmap parity.

Do not hide partial/unsupported cases.

Do not block manual mode with agent-mode restrictions.

Do not add new compatibility features unless needed to close report/UX gaps.

## Workstream 1: CLI Output UX

Improve non-JSON CLI output for NSE runs:

- show profile and compatibility status;
- show rule result summary;
- show library use summary;
- show capability denials/warnings;
- show evidence count and top evidence summaries;
- keep raw output accessible.

### Acceptance Criteria

- Manual users can understand why a script was partial/unsupported without opening JSON.
- Automated users still receive full JSON reports.

## Workstream 2: TUI Display Model

If the TUI already displays NSE output, add or plan display fields for:

- compatibility badge;
- fidelity badge;
- rule status;
- libraries used;
- capability events;
- evidence list;
- raw output panel;
- warnings/errors panel.

If the TUI integration point is not ready, document the model and provide data-shape guidance for the later TUI pass.

### Acceptance Criteria

- TUI-facing data shape is clear.
- No report field requires parsing raw prose.

## Workstream 3: Compatibility Matrix

Publish a compatibility matrix in docs, likely:

```text
docs/NSE_COMPATIBILITY.md
```

or a dedicated section in `architecture/nse_integration.md`.

Required columns:

- script/library/category;
- support status;
- tested fixture IDs;
- profile compatibility;
- capability requirements;
- known gaps;
- evidence support;
- notes.

### Acceptance Criteria

- Claims tie back to registry metadata and corpus fixtures.
- Matrix distinguishes supported, partial, approximate, unsupported, deferred.
- No full Nmap parity claims.

## Workstream 4: Docs and Agent Guidance

Update:

- `README.md` NSE summary;
- `architecture/nse_integration.md`;
- `architecture/nse_capability_inventory.md`;
- `.opencode/skills/eggsec-nse/SKILL.md`;
- `AGENTS.md` and `crates/eggsec-nse/AGENTS.override.md`.

Required guidance:

- new scripts/tests should use local fixtures;
- report truthfulness is mandatory;
- compatibility claims must cite corpus/registry support;
- capability wrappers remain mandatory for side-effecting helpers;
- AgentSafe/CiSafe must remain scoped.

## Workstream 5: Architecture Guards

Add or update guards for:

- docs overclaiming Nmap parity;
- compatibility matrix entries without fixture IDs;
- report UX bypassing `NseRunReport` fields;
- evidence extraction claiming vulnerability findings without confidence;
- new side-effect helpers bypassing wrappers;
- plan-file retention if feasible.

### Acceptance Criteria

- Guards protect the main truthfulness and release claims.
- Guard messages explain remediation.

## Workstream 6: Final Verification Record

Record Milestone 4 final verification in architecture docs.

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
make test-nse
```

If any command fails because of known pre-existing issues, record exact file/line and reason.

## Workstream 7: Closure Note

Add a Milestone 4 closure note stating:

- corpus coverage achieved;
- upstream subset status;
- host/port/service context improvements;
- evidence report support;
- CLI/TUI UX state;
- known gaps and deferred work;
- recommended Milestone 5 direction.

Possible Milestone 5 candidates:

- deeper protocol-library migration;
- broader upstream script conformance;
- service probe integration;
- TUI-first compatibility debugging workflow;
- performance/caching for large corpus runs.

## Final Acceptance Criteria

Phase 05 is complete when:

- CLI output clearly summarizes compatibility, warnings, capability events, and evidence.
- TUI/report display model is documented or implemented.
- Compatibility matrix is published and tied to fixtures.
- Docs and guards protect truthfulness claims.
- Final verification is recorded.
- Milestone 4 closure note is present.
