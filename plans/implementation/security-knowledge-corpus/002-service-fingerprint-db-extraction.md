# Security Knowledge Corpus Milestone 002 — Service fingerprint database extraction

Status: closed (`plans/closure/security-knowledge-corpus/002-closure.md`; `eggsec-service-db` extracted, guard 114 re-pinned, guard 148 added)

Repository baseline: `fix/cli-usability-audit` at `979dca67` (pushed); this plan lands on a branch cut from it

Source roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-2--service-fingerprint-database-extraction`

Long-term requirements:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/002-long-term-roadmap.md#phase-1--crate-boundaries-and-reusable-library-ownership`

Applicable ADRs:

- `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`

Primary class: infrastructure

## 1. Objective

Extract `crates/eggsec/src/scanner/service_data.rs` (302 lines, 21 tests) into a new
leaf crate `eggsec-service-db` whose only dependency is `rustc-hash`, and re-export it
from the engine at its existing path.

This is the smallest of the three extractions and exists partly to establish the
mechanical pattern the two larger ones reuse.

## 2. Why this milestone is ready

No hard or interface dependencies. Measured coupling is **zero** `crate::` references
outside test-only `super::` paths, and the only external crate used is `rustc-hash`
(`FxHashMap` and `LazyLock`). No consumer outside `scanner/` imports its symbols.

The extraction was validated by spike: the file was copied into a scratch crate
depending only on `rustc-hash`, and all 21 tests passed unmodified with no source
changes at all.

## 3. Current implementation evidence

`crates/eggsec/src/scanner/service_data.rs` is scanner-owned service fingerprinting
knowledge, moved out of a global utility bucket in a prior phase:

- `COMMON_PORTS` — a `&[(u16, &str)]` port→service table.
- `get_service_name(port)` — port lookup.
- `guess_service_from_banner(banner)` — banner heuristics.
- `is_web_service(port)` — web-service predicate.

The module holds pure lookup tables and pure string heuristics. It performs no I/O, no
DNS, no socket access, and holds no authority state.

Consumers: none outside `scanner/`. Within `scanner/`, it is consumed by the
fingerprinting path.

**Guard 114 is a hard blocker.** At line 3121 of `scripts/check-architecture-guards.sh`
(as of baseline `979dca67`) is:

```bash
if [[ ! -f "crates/eggsec/src/scanner/service_data.rs" ]]; then
  echo "FAIL: crates/eggsec/src/scanner/service_data.rs missing (canonical owner)."
```

The same check also fails if `crates/eggsec/src/utils/service_detection.rs` reappears.
The guard's intent — that service tables have a single canonical owner distinct from the
utility bucket — is correct and must survive; only the owner's path changes.

## 4. Invariants that must not regress

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate. Service
  fingerprinting is an executor concern; the corpus crate must not acquire policy.
- `eggsec-service-db` authorizes nothing and resolves nothing. It must not gain
  `Scope`, `Capability`, or `ApprovedOperation` access.
- `eggsec-service-db`'s only workspace dependency is `rustc-hash`-equivalent; it must
  have **no** `eggsec-*` dependency, and no Tokio, HTTP, TLS, filesystem, frontend,
  engine, or transport dependency.
- Workspace path graph stays acyclic (guard 108).
- Engine path `eggsec::scanner::service_data::*` remains valid.
- `utils/service_detection.rs` must not reappear (guard 114's second half).
- Existing `LazyLock` table initialization stays lazy.

## 5. Scope

### In scope

- New crate `crates/eggsec-service-db`.
- Move `service_data.rs` content into it.
- Engine re-export preserving the existing path.
- Update guard 114 to pin the new canonical owner.
- A new guard enforcing the leaf invariant for this crate.
- `make check` per-crate test line.
- Documentation updates.

### Explicitly out of scope

- **`DEFAULT_ENDPOINTS`.** `scanner/endpoints.rs:98-348` holds 347 endpoint paths as a
  pure `&[&str]`, and the table itself is separately extractable. It is **not** in this
  milestone: the same file carries `reqwest`/`tokio`/`indicatif` HTTP fetching, five
  `#[cfg(feature = "cli")]` blocks, and a `tool-api`-gated function returning
  `crate::tool::response::Finding`. Extracting the table would require splitting one
  file in two. Deferred to a future ADR.
- `fingerprint.rs`, `udp_fingerprint.rs`, `spoof.rs`, `icmp_probe.rs`, `ports/`,
  `templates/`, `cms/` — all executor-bound.
- `scanner/fingerprint_types.rs` and `scanner/timing.rs`, which are also low-coupling
  (serde-only; `timing.rs` has exactly two `crate::constants` references). Measured and
  deferred.
- Any content change to the port table or banner heuristics. Content changes can alter
  scan results and are a capability change, not infrastructure.
- Enabling publication.

## 6. Required production changes

### Core/domain

- Create `crates/eggsec-service-db/` with a `Cargo.toml` declaring only `rustc-hash`
  (plus `tracing` if the moved code logs) and `[lib] name = "eggsec_service_db"`.
  Follow the `eggsec-udp-scan` manifest shape: a domain crate that authorizes nothing
  and reports facts.
- Move the module verbatim. No logic edits, no formatting-only churn, no reordering of
  table entries — a diff that is not a pure move makes review harder without adding
  safety.
- Add it to the root `Cargo.toml` `[workspace] members`.

### Protocol and DTOs

None. No wire format changes; the tables are compile-time constants.

### Frontend or operator surface

- In `crates/eggsec/src/scanner/mod.rs`, replace the module declaration with a
  re-export so `eggsec::scanner::service_data::*` keeps resolving. A `pub use
  eggsec_service_db as service_data;` style re-export keeps every existing
  intra-scanner call site unchanged.

### Security and authorization

None. The crate must not import `eggsec-policy`, `eggsec-transport`, or engine config.
If the implementing agent finds an authorization need, that is a stop condition.

### Documentation and static guards

- Guard 114: change the pinned path from
  `crates/eggsec/src/scanner/service_data.rs` to the crate's canonical location, keep
  the `utils/service_detection.rs` reappearance check, and update the failure message so
  it still names the canonical owner.
- New guard: assert `eggsec-service-db` has no `eggsec-*`, `tokio`, `reqwest`,
  `rustls`, `hickory-resolver`, `serde_json`-runtime-I/O, or frontend dependency, and
  that its `Cargo.toml` declares no `eggsec-core` even though `eggsec-core` would be
  acceptable. Mirror the shape of checks 121–123 for `eggsec-policy`.
- Add `cargo test -p eggsec-service-db --tests` to `make check`.
- `docs/BUILD.md` or the release package graph: only if the packaging inventory pins
  crate lists by count. Verify before editing — `eggsec-udp-scan` shipped without a
  `make check` line, so this repo tolerates new crates that are not individually
  registered there.

## 7. Ordered work packages

### Work package A — Create the crate

Intent: standalone crate with the module copied in.

Required changes: manifest, `members` entry, module file.

Acceptance evidence: `cargo test -p eggsec-service-db` passes **21 tests**, unchanged from the baseline count.

### Work package B — Re-export and remove the original

Intent: preserve source compatibility.

Required changes: scanner module re-export; delete the engine-side file.

Acceptance evidence: `cargo check --workspace --no-default-features` is green with **no** edits to any `scanner/` consumer call site.

### Work package C — Update guard 114

Intent: keep the single-owner invariant, move the pin.

Required changes: as §6.

Acceptance evidence: guards pass, and the check still fails when `utils/service_detection.rs` is artificially recreated.

### Work package D — Add the leaf guard and test line

Intent: prevent dependency creep.

Required changes: new guard check; `make check` line.

Acceptance evidence: the guard fails when a forbidden dependency is added to the new crate's manifest.

### Work package E — Documentation

Intent: keep docs and skills honest.

Required changes:

- `architecture/scanner.md` — new owner and path for service tables.
- `architecture/overview.md` — crate table row and dependency-map edge
  (`eggsec` → `eggsec-service-db`).
- `architecture/capability_segregation.md` — Phase G entry (started in M001).
- `architecture/feature_matrix.md` — if it inventories domain crates.
- `.opencode/skills/` — any claim that names the scanner's service-data owner or a
  counted set that changed. Per `AGENTS.md`, skills are not type-checked; update them
  in the same pass.

## 8. Failure, cancellation, restart, and contention semantics

Not applicable. The crate is synchronous, stateless, and does I/O-free table lookup.
`LazyLock` initialization is one-time per process and must remain lazy so scanner
startup cost is unchanged.

No tokio tasks, so no timeout wrappers. No cancellation or restart semantics change.

## 9. Compatibility and migration

No storage, protocol, configuration, or wire migration. No data migration: these are
compiled-in tables.

Source compatibility is the contract: `eggsec::scanner::service_data::*` must resolve
identically before and after. Verify with `cargo check --workspace --no-default-features`
plus `cargo check -p eggsec-python -p eggsec-tui` (the bindings and TUI fingerprint
paths reach scanner types).

Because the crate is `publish = false` and internal, no external consumer can be
affected by the module move. This is the main argument for staging extraction ahead of
publication under ADR-0005 decision 3.

## 10. Required tests

### Focused unit tests

- The 21 existing tests, carried unmodified. Count is the contract.
- If any test references the old module path, update only the path.

### Integration tests

- `cargo test -p eggsec --no-default-features --test tool_registration` — the engine's
  registration suite must be unaffected.
- `cargo test -p eggsec --features rest-api,cli --tests` per `make check`.

### Restart and recovery tests

None. No persisted state.

### Contention and cancellation tests

None. `FxHashMap` reads are immutable after `LazyLock` init.

### Security and negative tests

- Confirm the crate does not import `eggsec-policy`, `eggsec-transport`, or any engine
  module — asserted by the new guard, and additionally by `cargo tree -p
  eggsec-service-db` showing no workspace edge.

### Migration and compatibility tests

- `cargo check -p eggsec -p eggsec-python -p eggsec-tui --no-default-features` green with
  zero consumer diffs.

## 11. Required verification commands

```bash
cargo test -p eggsec-service-db --tests
cargo tree -p eggsec-service-db
make check                  # mandatory Rust contract
make check-deps             # cargo deny over --workspace --all-features
bash scripts/check-architecture-guards.sh
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

As work package E. Also update `plans/subsystems/security-knowledge-corpus-roadmap.md`
and `plans/registry.md` status when closed.

## 13. Acceptance criteria

- `crates/eggsec-service-db` exists, depends only on `rustc-hash` (and `tracing` if
  required), and has **no** `eggsec-*` dependency.
- `cargo test -p eggsec-service-db` passes 21 tests.
- `eggsec::scanner::service_data::*` still resolves; no consumer file was edited to
  accommodate the move.
- Guard 114 pins the new owner and still catches `utils/service_detection.rs`.
- The new leaf guard exists and is demonstrated to fail on a forbidden dependency.
- `make check` is green.
- `architecture/overview.md`, `architecture/scanner.md`, and Phase G of
  `architecture/capability_segregation.md` are updated in the same pass.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- any `crate::` reference other than test-only `super::` is required, meaning the
  module is not a corpus after all;
- an authorization or scope need appears — that is `eggsec-policy` ownership;
- the engine re-export cannot preserve the existing path without editing consumers;
- guard 114's intent (single canonical owner, distinct from the utility bucket) cannot be
  preserved;
- scope would expand to `endpoints.rs`, `fingerprint.rs`, or the other scanner modules;
- a port-table or banner-heuristic content change appears to be required — stop and
  report it as a capability change.

## 15. Closure evidence required

`plans/closure/security-knowledge-corpus/002-closure.md` containing:

- `cargo tree -p eggsec-service-db` output showing no workspace edge;
- `cargo test -p eggsec-service-db` output showing 21 passed;
- the diff proving the module move is content-identical;
- the updated guard 114 text and proof it still fails on the
  `utils/service_detection.rs` condition;
- the new guard check's name, plus a demonstration that it fails on a forbidden
  dependency;
- `make check` outcome and the guards outcome;
- confirmation that `cargo check -p eggsec -p eggsec-python -p eggsec-tui` was green
  with no consumer edits;
- residual findings classified by severity;
- recommendation: closed, conditionally closed, corrective pass required, or blocked.

## 16. Handoff notes

`service_data.rs` is 302 lines with a 21-test ratio and zero coupling — there is no
judgment call in this milestone beyond keeping the move mechanical. The real risk is
scope creep into `endpoints.rs`, whose 347-path table looks extractable and is not, in
this file's current state.

Preserve unrelated user changes. The two untracked plan files
(`plans/c2-real-simulation-differentiation-plan.md`, `plans/post-exploit-plan.md`) in the
working tree belong to the user — do not modify or delete them.