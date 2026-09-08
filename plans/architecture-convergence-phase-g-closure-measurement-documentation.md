# Phase G Plan: Closure, Measurement, and Documentation Reconciliation

## Status

Status: Ready for implementation.

## Objective

Close the architecture-convergence roadmap against the actual final implementation head. Remove temporary migration state, reconcile active documentation with source, and record direct measurements showing whether the line of work reduced synchronization burden and improved capability maturity.

This is a bounded closure phase. It must not become another feature-development phase or evidence framework.

## Preconditions

Phases A-F must either be executed or have explicit accepted blockers. Do not mark the roadmap complete while compatibility migrations introduced by these plans are still open-ended.

## Primary areas

```text
plans/architecture-convergence-*.md
plans/README.md
README.md
docs/ARCHITECTURE.md
docs/ARCHITECTURE_INVARIANTS.md
docs/COMMAND_REGISTRY.md
docs/TOOL_REGISTRATION.md
docs/FEATURE_MATRIX.md
docs/BUILD.md
docs/VERIFICATION.md
docs/EXTENSIBILITY.md
docs/DAEMON.md
docs/WEB_PROXY.md
docs/MOBILE.md
docs/WIRELESS.md
docs/python/domain-maturity.md
docs/python/
Makefile
.github/workflows/
scripts/check-architecture-guards.sh
```

## Non-goals

This phase does not add features, redesign APIs, change authorization policy, reopen historical plans, or introduce permanent dashboards/ledgers solely to prove completion.

## Workstream 1 — Reconfirm final source truth

On the exact final implementation head, inventory:

- workspace crates and dependency directions;
- scope/specification types and authoritative authorization owner;
- Cargo default/main/domain features;
- feature registry entries and aggregate semantics;
- canonical operations and aliases;
- operation-backed CLI commands and dispatch modes;
- runtime task mappings;
- protocol adapter ownership;
- agent ownership/injection boundary;
- Python stable/provisional/experimental domains;
- daemon protocol version and parity matrix;
- platform integration profiles.

Do not copy numbers from prior plans without recounting them.

## Workstream 2 — Remove temporary migration state

Search for and classify:

```text
LegacyWrapped
registry_backed
TODO remove after phase
compatibility conversion temporary aliases
old Scope DTO names
deprecated request constructors
old protocol bridge modules
old runtime parameter mirrors
placeholder browser/proxy session paths
```

Remove items whose compatibility window has ended. Retain semver/public compatibility facades only when intentional, document them as stable shims, and add no deadline that cannot realistically be enforced.

## Workstream 3 — Documentation reconciliation

Update active documentation to match final source exactly.

Special attention:

- `FEATURE_MATRIX.md` defaults/aggregate semantics;
- `COMMAND_REGISTRY.md` removal/completion of legacy dispatch distinctions;
- `ARCHITECTURE.md` crate ownership and protocol/agent boundaries;
- scope documentation distinguishing specification from authorization;
- Python maturity tables based on actual parity evidence;
- daemon local/remote behavior and protocol compatibility;
- platform prerequisites and deep-check cadence;
- verification commands and lint/feature sweep ownership.

Run existing documentation reference/example checks. If source facts are now generated mechanically, remove hand-maintained duplicate tables rather than keeping both.

## Workstream 4 — Complexity and duplication measurements

Record direct before/after measurements against the roadmap baseline where practical:

- count of public/internal `Scope`-like authorization/spec types;
- count of operation-backed commands still using transitional dispatch modes;
- count of separately defined per-operation request parameter structs across engine/runtime/tool adapters;
- count of feature declarations not directly compiled in a maintained profile;
- line/file size of the principal policy/runtime/daemon hotspots;
- number of protocol adapters hosted inside the main engine crate;
- count of Python browser/proxy placeholder methods;
- daemon parity checklist pass/fail totals;
- required versus skipped platform integration tests.

The goal is not arbitrary line-count reduction. Explain what each measurement means architecturally.

## Workstream 5 — Dependency/artifact measurements

Because protocol extraction and shared-contract work may change dependencies, record:

- `cargo tree` for standard CLI, headless/library, daemon, protocol adapter crates, and Python extension profiles;
- release artifact sizes for standard supported artifacts where reproducible;
- duplicate major dependency generations that were added/removed;
- optional native/system prerequisites.

Do not add hard size gates unless a separate product requirement exists.

## Workstream 6 — Verification closure

Run at minimum:

```text
make check
make check-python
make check-full
make check-features-individual   # if introduced in Phase B
```

Also run the relevant daemon/protocol/browser/proxy/platform profiles introduced by this roadmap.

Record exact commands and results. Distinguish:

- pass;
- expected platform skip;
- unavailable external prerequisite;
- known blocker.

Do not write “all checks pass” when a required profile was not run.

## Workstream 7 — Hosted CI confirmation

Confirm the actual CI status for the final pushed commit where accessible. Do not infer hosted success solely from local checks.

If optional scheduled/manual deep checks have not yet run on the final head, state that explicitly and leave the corresponding phase/roadmap condition open until they do or the maintainer accepts the documented local evidence.

## Workstream 8 — Update plan status records

For each Phase A-F plan, append the completion record with:

- baseline SHA;
- final implementation SHA;
- significant compatibility decisions;
- verification performed;
- blockers/accepted exclusions.

Then mark this Phase G and the roadmap executed only when acceptance criteria are satisfied.

Update `plans/README.md` so historical dependency/architecture work remains clearly separate from this roadmap.

## Workstream 9 — Residual backlog classification

Any remaining issue discovered during closure must be classified as:

- correctness/security blocker — roadmap cannot close;
- required roadmap acceptance gap — roadmap cannot close;
- accepted platform limitation — document and close only if roadmap criteria allow it;
- future capability/optimization — record outside this roadmap without reopening implementation scope.

Do not turn optional optimization findings into indefinite blockers.

## Acceptance criteria

- all prior phase status records reflect actual implementation state;
- source/documentation feature/default/command/maturity claims agree;
- no unintentional temporary migration layer remains;
- one authoritative scope authorization owner is documented and enforced;
- legacy command-dispatch migration state is absent for operation-backed commands;
- feature direct-compilation coverage is complete under the Phase B contract;
- protocol/agent dependency boundaries match the Phase D target or have explicit accepted exceptions;
- browser/daemon/proxy maturity claims match execution parity evidence;
- platform integration profiles have non-vacuous test evidence;
- required local verification passes;
- hosted CI status is recorded accurately;
- before/after complexity/dependency measurements are recorded without inventing hard gates;
- manual release/publication policy remains unchanged;
- the roadmap is marked executed only after the exact final head is validated.

## Completion record

When complete, append the closure summary and link any final architecture/verification documents used as canonical references.