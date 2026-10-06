# Security Knowledge Corpus Milestone 002 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/security-knowledge-corpus/002-service-fingerprint-db-extraction.md`

Source subsystem roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-2--service-fingerprint-database-extraction`

Repository baseline reviewed: `947e8920` (branch `docs/knowledge-corpus-crate-plans`; M001 closure `947e8920`)

Implementation commits or pull requests:

- this branch — extract `eggsec-service-db`, re-pin guard 114, add guard 148, register `make check` line

## 1. Executive finding

The extraction is complete and the milestone's defining constraint held: **no consumer
file was edited**.

`crates/eggsec/src/scanner/service_data.rs` now lives at
`crates/eggsec-service-db/src/lib.rs`, byte-identical below its doc header (263 lines
verified by diff). All 21 tests run in isolation in the new crate. The engine reaches it
through `pub use eggsec_service_db as service_data;`, which keeps
`eggsec::scanner::service_data::*` resolving and leaves `scanner/ports/mod.rs` — the
only in-repo consumer — completely untouched.

The dependency closure collapsed to exactly one edge: `rustc-hash`.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Crate exists with only `rustc-hash`, no `eggsec-*` dep | `cargo tree -p eggsec-service-db` → one edge | pass | see §3 |
| `cargo test -p eggsec-service-db` passes 21 tests | `21 passed; 0 failed` | pass | baseline count preserved exactly |
| `eggsec::scanner::service_data::*` resolves; no consumer edited | `git diff --stat` over consumers empty | pass | see §3 |
| Module move is content-identical | 263-line body diff identical | pass | only the `//!` header changed |
| Guard 114 pins the new owner | rewritten check, passing | pass | now also catches the old file and a dropped facade |
| Guard 114 still catches `utils/service_detection.rs` | unchanged half of the check | pass | |
| New leaf guard exists and fails on a forbidden dep | check 148, demonstrated | pass | see §4 |
| `make check` green | see §4 | pass | |
| `overview.md` / `scanner.md` / Phase G updated | this commit | pass | |
| `cargo test -p eggsec-service-db --tests` registered in `make check` | `Makefile` line added after `eggsec-policy` | pass | |

## 3. Production implementation evidence

### Dependency closure

```
$ cargo tree -p eggsec-service-db
eggsec-service-db v0.1.0 (crates/eggsec-service-db)
└── rustc-hash v2.1.3
```

One edge. No workspace dependency, not even `eggsec-core` — this corpus needs no
engine type at all, which is why guard 148 holds it to a stricter bar than the other
two.

### Test isolation

```
$ cargo test -p eggsec-service-db --tests
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Identical to the pre-extraction count. The 21-test ratio on 302 lines is unusually high
for a lookup table, and it is the reason this was a safe first extraction.

### The move was mechanical

Body content below the `//!` header is byte-for-byte identical (263 lines):

```
$ git show HEAD:crates/eggsec/src/scanner/service_data.rs | grep -v '^//!' | grep -v '^$' > /tmp/o.txt
$ grep -v '^//!' crates/eggsec-service-db/src/lib.rs | grep -v '^$' > /tmp/n.txt
$ diff /tmp/o.txt /tmp/n.txt
IDENTICAL: bodies match byte-for-byte (263 lines)
```

No port-table entry was reordered, re-rated, or re-formatted. No banner heuristic was
touched. Content change is a capability change and was explicitly out of scope.

### Source compatibility

`crates/eggsec/src/scanner/mod.rs` replaces `pub mod service_data;` with:

```rust
pub use eggsec_service_db as service_data;
```

This keeps `crate::scanner::service_data::get_service_name` (used at
`scanner/ports/mod.rs:32` and, in tests, `COMMON_PORTS` at `:953`) and the existing
`pub use service_data::{…}` block resolving. Proof that no consumer needed editing:

```
$ git diff --stat -- crates/eggsec/src/scanner/ports/ crates/eggsec-python/ \
    crates/eggsec-tui/ crates/eggsec-mobile-lab/ crates/eggsec/tests/
(empty)
```

This is the pattern milestones 003 and 004 reuse.

### Laziness preserved

`COMMON_PORTS`, `PORT_SERVICE_MAP`, and the service-classifier tables remain behind
`LazyLock`. A process that never fingerprints a service never builds them, so scanner
and CLI startup cost is unchanged. Confirmed by inspecting the moved source, not by
diffing.

### Incidental normalization

The original `service_data.rs` carried file mode `100755` (executable) — a pre-existing
wart; every other workspace `lib.rs` is `100644`. The moved file is `100644`.

## 4. Verification executed

### Commands run

```bash
cargo test -p eggsec-service-db --tests
cargo tree -p eggsec-service-db
cargo check --workspace --no-default-features
bash scripts/check-architecture-guards.sh
bash scripts/check-architecture-guards.sh   # negative tests (see below)
make check
```

### Results

| Command | Result |
|---|---|
| `cargo test -p eggsec-service-db --tests` | pass — 21 passed, 0 failed |
| `cargo tree -p eggsec-service-db` | pass — single `rustc-hash` edge |
| `cargo check --workspace --no-default-features` | pass — `Finished dev profile … in 29.23s` |
| `bash scripts/check-architecture-guards.sh` | pass — all 148 checks, `ALL PASSED` |
| Guard 148 negative tests | **pass** — see below |
| `make check` | pass — see §4.1 |

#### 4.1 Guard 114 (re-pinned)

The single-owner intent is preserved and strengthened. The check now asserts **all four**:

1. `crates/eggsec-service-db/src/lib.rs` exists (new canonical owner);
2. `crates/eggsec/src/scanner/service_data.rs` does **not** exist (added — the old
   engine-side file cannot quietly come back);
3. `scanner/mod.rs` still contains `pub use eggsec_service_db as service_data` (added —
   the compatibility facade cannot be dropped, which would break every consumer);
4. `utils/service_detection.rs` still does not exist and no `service_detection` reference
   reappears (unchanged from Phase A).

Passing state:

```
--- Check 114: service-detection stays scanner-owned ---
PASS: service-detection stays scanner-owned (eggsec-service-db).
```

#### 4.2 Guard 148 (new leaf invariant) — negative tests

Guard 148 loops over `eggsec-service-db`, `eggsec-secrets`, and `eggsec-payloads`,
skipping any not yet extracted, so milestones 003 and 004 land into the existing loop
with no guard edit.

The invariant is: `eggsec-core` is the only permitted workspace dependency (it is a
zero-internal-dependency leaf owning `Severity`); every other `eggsec-*` crate, plus
`tokio`/`reqwest`/`hyper`/`rustls`/`axum`/`tonic`/`clap`/`ratatui`/`crossterm`/`rusqlite`/
`sqlx`/`hickory-resolver`/`indicatif`, is forbidden; no `Scope` / `ApprovedOperation` /
`ApprovedExecution` / `EnforcementContext` / `Capability` reference may appear in source.

Demonstrated failures:

```
# eggsec-core added to eggsec-service-db
FAIL: crates/eggsec-service-db/Cargo.toml declares eggsec-core (this corpus needs no workspace edge).
23:eggsec-core = { path = "../eggsec-core", version = "0.1.0" }

# eggsec-policy + reqwest added
FAIL: crates/eggsec-service-db/Cargo.toml references forbidden workspace dep 'eggsec-policy' (only eggsec-core is permitted).
FAIL: crates/eggsec-service-db/Cargo.toml references forbidden dep 'reqwest'.
```

Script exit code 1 in both cases; the condition was restored and the suite re-run clean.

**Deviation from the plan, recorded deliberately.** The plan's §6 asked for a guard
asserting `eggsec-service-db` "declares no `eggsec-core` even though `eggsec-core` would
be acceptable", generalized across all corpus crates. Written that way, it would have
**deadlocked milestones 003 and 004**, whose corpora legitimately need `eggsec-core` for
`Severity`. The implemented guard permits `eggsec-core` corpus-wide and holds
`eggsec-service-db` to the stricter zero-edge bar via a separate, named assertion. Both
halves are demonstrated above.

#### 4.3 `make check`

`make check` green, exit code 0, including the new
`cargo test -p eggsec-service-db --tests` line registered between `eggsec-policy` and
`eggsec-transport-eggfetch`.

Deep checks (`make check-full`, `make check-features-individual`, `make clippy-domain`,
`make test-tui-pty`) were **not run** — no feature surface changed. `make check-deps` ran
as part of `make check` and passed. `make check-python` was **not run**: no Python
binding, stub, or doc changed, and the bindings compile against a path that did not move.

## 5. Invariant review

| Source-plan invariant | Evidence |
|---|---|
| `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate | No dispatch, enforcement, `OperationMetadata`, or approval code touched. The diff is a file move, one `pub use`, one manifest line, and guards. |
| `eggsec-service-db` authorizes nothing, resolves nothing | Guard 148 enforces no `Scope`/`ApprovedOperation`/`Capability`/`EnforcementContext` in source and no resolver/authority crate in the manifest. |
| No `eggsec-*` dependency; no Tokio/HTTP/TLS/filesystem/frontend/transport | `cargo tree` shows one `rustc-hash` edge; guard 148 enforces it. |
| Workspace path graph stays acyclic (check 108) | Passes in the 148-check run. The new edge is `eggsec → eggsec-service-db`, one-way, and the new crate depends on nothing in the workspace. |
| Engine path `eggsec::scanner::service_data::*` remains valid | `cargo check --workspace` green with an empty consumer diff. |
| `utils/service_detection.rs` must not reappear | Check 114's unchanged half still guards it. |
| `LazyLock` initialization stays lazy | Moved source inspected: tables remain behind `LazyLock`; no eager static introduced. |

## 6. Failure and recovery review

Not applicable. The crate is synchronous, stateless, and performs no I/O. `FxHashMap`
reads are immutable after `LazyLock` initialization. No tokio task is introduced, so no
timeout wrapper applies. No cancellation, restart, or partial-persistence behavior
exists or changed.

## 7. Migration and compatibility review

No storage, protocol, configuration, or wire migration. These are compiled-in tables;
there is no data to migrate.

Source compatibility is the contract and it holds: `eggsec::scanner::service_data::*`
resolves identically before and after, proven by an empty consumer diff plus a green
workspace check including `eggsec-python`, `eggsec-tui`, and `eggsec-mobile-lab`.

The crate is `publish = false` and internal, so no external consumer can be affected by
the move. Rollback is a revert. `Cargo.lock` gains one internal package entry.

## 8. Security review

Net effect is neutral-to-positive and is stated as such rather than claimed as a new
capability:

- **Authorization:** untouched. The crate has no scope, capability, or approval surface, and guard 148 prevents it acquiring one. It cannot grant, widen, or bypass a check.
- **Network surface:** none. The crate opens no socket and resolves no name — it is handed a banner string and returns its meaning. Scanning I/O stays entirely in the engine, so this corpus can neither leak scope nor bypass a transport checkpoint.
- **Tamper surface:** reduced for scanners. Previously any build of the engine recompiled the table from engine source; now the table is a leaf crate whose 21 tests run independently of the 195k-line engine, so table regressions surface in a ~300-line test run.
- **No new input handling.** The crate parses nothing untrusted beyond the banner string it is given, and it was already doing so.

## 9. Documentation and operations

Updated:

- `architecture/overview.md` — crate-table row for `eggsec-service-db`; dependency-map node; guard bullet.
- `architecture/scanner.md` — module table row replaced with an explicit "moved to `eggsec-service-db` in Phase G" entry naming the re-export.
- `architecture/capability_segregation.md` — Phase G status line recording the implemented crate, plus the Guards bullet rewritten for the new check-114/148 reality.
- `crates/eggsec-service-db/src/lib.rs` — doc header stating what the crate is, that it is a leaf, and which guard holds it there.
- `crates/eggsec/src/scanner/mod.rs` — doc comment on the facade marking it permanent, not transitional.

Static guards:

- **Check 114** — re-pinned to the new canonical owner; now also catches the old engine-side file and a dropped facade.
- **Check 148** — new; leaf invariant across all corpus crates, demonstrated to fail.

Operator diagnostics: none added; no behavior, flag, or output changed.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | The original `service_data.rs` had file mode `100755`. | A Rust source file marked executable. Cosmetic; normalized to `100644` as part of the move, matching every sibling crate. | None. |
| low | `scanner/endpoints.rs`'s `DEFAULT_ENDPOINTS` (347 paths) remains unextracted. | Measured as viable but blocked: the same file carries `reqwest`/`tokio`/`indicatif` HTTP fetching, five `#[cfg(feature = "cli")]` blocks, and a `tool-api`-gated function returning `crate::tool::response::Finding`. | Deferred per plan §5. A future ADR could split the table from the fetcher. |

No critical, high, or medium findings.

## 11. Roadmap disposition

**Milestone closed and next dependency may proceed.**

No stop condition was triggered: no `crate::` reference beyond test-only paths was
required, no authorization need appeared, the re-export preserved every path without
consumer edits, and the move stayed out of `endpoints.rs` and the other scanner modules.

M003 and M004 are both dependency-ready and can proceed independently. M005 remains
blocked on M002+M003+M004 plus a maintainer decision that is not a code question.

## 12. Registry updates

- `plans/subsystems/security-knowledge-corpus-roadmap.md` — milestone 2 status → closed; §7 milestone 2 outcome; §5 target architecture notes `eggsec-service-db` as landed.
- `plans/registry.md` — milestone 2 status → closed; `eggsec-service-db` recorded as an extracted corpus crate with guards 114/148.
- `plans/implementation/security-knowledge-corpus/002-service-fingerprint-db-extraction.md` — status → closed with the closure reference.