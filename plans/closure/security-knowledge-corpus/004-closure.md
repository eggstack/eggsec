# Security Knowledge Corpus Milestone 004 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/security-knowledge-corpus/004-payload-corpus-extraction.md`

Source subsystem roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-4--payload-corpus-extraction`

Repository baseline reviewed: `a8bb2bcf` (branch `docs/knowledge-corpus-crate-plans`; M003 closure `a8bb2bcf`)

Implementation commits or pull requests:

- this branch — extract `eggsec-payloads`, add engine-side dispatch module, guard 150, seam integration suite

## 1. Executive finding

The extraction is complete, all 233 tests pass, and the milestone's named trap was
**designed out rather than shipped**.

The 34 pure-data payload modules (7,084 lines) now live in `crates/eggsec-payloads`. Zero
consumers were edited.

> **Correction 2026-10-06.** This closure recorded the 6 remaining modules as "live-probe"
> generators that had to stay engine-side. That was a false premise: their `get_payloads()`
> functions are pure static data, separable from the `reqwest` probers bundled in the same
> files. Those 6 payload sets have since moved into `crates/eggsec-payloads` (now **all 40**
> variants, no panic), and the cross-variant caches moved with them. The rest of this
> record — test counts, the four mechanical edits, the trap analysis, the zero-diff
> consumer result — stands as written.

The plan warned that the spike's `Vec::new()` placeholder for the 6 relocated types must
not ship, because a silent empty vector reads as "this payload type has no payloads" —
which is false, and no test inside the corpus crate could catch it. That risk is now
closed at three levels, and verified: the corpus crate routes those types to a documented
`unreachable!`; check 150 fails if a stub reappears; and a new engine-side integration
suite asserts all 6 still return real payloads.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| 34 pure-data modules extracted | 36 files in the crate (`mod.rs`→`lib.rs`, `macros.rs`, 34 modules) | pass | §3 |
| 6 live-probe modules stay engine-side | `crates/eggsec/src/fuzzer/payloads/` holds exactly those 6 | pass | |
| `cargo test -p eggsec-payloads` passes 233 tests | `233 passed; 0 failed` | pass | baseline count exact |
| Engine + `eggsec-python` + `eggsec-tui` compile, no consumer edits | `git diff --stat` empty | pass | §3 |
| Four mechanical edits only | 11 files touched by path/import rewrites | pass | §3 |
| **No `Vec::new()` stub for the 6** | documented `unreachable!` + check 150 | pass | §3, §4 |
| 6 advanced types return real payloads | 15/22/25/21/27/14 payloads; all 40 types in cached view | pass | §4 |
| `LazyLock` laziness preserved | caches remain `LazyLock`, engine-side; check 150 asserts | pass | |
| Corpus crate stays a leaf | `cargo tree`: `eggsec-core` only workspace edge | pass | |
| New guard exists and fails | check 150 + negative tests | pass | §4 |
| `make check` green | see §4 | pass | |

## 3. Production implementation evidence

### The split

| Side | Contents | Owner |
|---|---|---|
| Corpus | 34 modules, 7,084 lines | `eggsec-payloads` |
| Probe | `graphql`, `grpc`, `idor`, `jwt`, `oauth`, `ssti` (4,354 lines) | engine `fuzzer/payloads/` |
| Union | 40-arm dispatch + `PAYLOAD_CACHE` + `ALL_PAYLOADS_CACHE` + both cached accessors | engine `fuzzer/payloads/mod.rs` |

### The mechanical edits

Applied across 12 files, all import/path rewrites:

- `crate::types::Severity` → `eggsec_core::types::Severity` (`lib.rs`, `oast.rs` ×10 refs)
- `crate::fuzzer::payloads::` → `crate::` (13 occurrences across 11 files)
- `$crate::fuzzer::payloads::Payload` → `$crate::Payload` (`macros.rs` ×2)

No payload text, severity, tag, ordering, or table entry was touched.

### The design change the spike could not produce — and why it was necessary

The plan anticipated that `get_payloads` would need engine-side resolution for the 6
types and warned against the `Vec::new()` spike artifact. Resolving that concretely
surfaced a second fact: **the cross-variant caches cannot live in the corpus crate at
all.**

```rust
static PAYLOAD_CACHE: LazyLock<FxHashMap<PayloadType, Vec<Payload>>> = LazyLock::new(|| {
    for pt in PayloadType::all_variants() {   // ALL 40
        map.insert(*pt, get_payloads(*pt));
    }
    map
});
```

Six of those 40 are engine-owned. A cache in the corpus crate would either panic on
initialization or silently cover only 34 — and the latter is invisible, because
`get_payloads_cached` has an `unwrap_or_else` fallback returning a shared empty `Vec`.
`tests/fuzzer_tests.rs` would then have failed its `all_cached.len() == total` assertion,
but only incidentally.

So the caches and the 40-arm dispatch moved to a new engine-side `fuzzer/payloads/mod.rs`,
which owns the union. The corpus crate's `get_payloads` keeps all 40 arms and routes the 6
to:

```rust
fn engine_owned_probe_payloads(payload_type: PayloadType) -> Vec<Payload> {
    unreachable!(/* names the engine path and PayloadType::is_advanced() */)
}
```

This is strictly better than `Vec::new()`: a library consumer gets an immediate, explicit
error naming the correct API, rather than an empty list that looks like a valid answer.

The corpus crate deliberately does **not** export `get_payloads_cached` /
`get_all_payloads_cached`, because it cannot implement them correctly for all 40 variants.

### Source compatibility

`fuzzer/payloads/mod.rs` re-exports the corpus crate wholesale (`pub use
eggsec_payloads::*;`) alongside the 6 local modules, so `eggsec::fuzzer::payloads::*` and
`eggsec::fuzzer::{PayloadType, Payload, get_payloads, get_payloads_cached,
get_all_payloads_cached, Severity}` all resolve unchanged.

```
$ git diff --stat -- crates/eggsec-python/ crates/eggsec-tui/ \
    crates/eggsec-mobile-lab/ crates/eggsec/tests/
(empty)
```

The 6 probe modules needed no import edits either — they already imported via
`crate::fuzzer::payloads::{Payload, PayloadType, Severity}`, which the facade resolves.

### Dependency closure

```
$ cargo tree -p eggsec-payloads | grep eggsec
eggsec-payloads v0.1.0 (crates/eggsec-payloads)
├── eggsec-core v0.1.0 (crates/eggsec-core)
```

`eggsec-core` (for `Severity`), `serde`, `strum` (declared to match the engine's exact
`default-features = false, features = ["derive"]`), `flate2`, `rustc-hash`, `tracing`.
No `reqwest`, no Tokio, no TLS.

## 4. Verification executed

### Commands run

```bash
cargo test -p eggsec-payloads --tests
cargo tree -p eggsec-payloads
cargo check --workspace --no-default-features
cargo test -p eggsec --test fuzzer_payload_corpus_seam
bash scripts/check-architecture-guards.sh
bash scripts/check-architecture-guards.sh   # negative tests
cargo fmt --all --check
make check
```

### Results

| Command | Result |
|---|---|
| `cargo test -p eggsec-payloads --tests` | pass — **233 passed**, 0 failed |
| `cargo tree -p eggsec-payloads` | pass — `eggsec-core` the only workspace edge |
| `cargo check --workspace --no-default-features` | pass |
| Consumer diff check | pass — empty across bindings, TUI, integration tests |
| `cargo test -p eggsec --test fuzzer_payload_corpus_seam` | pass — 3 passed |
| `bash scripts/check-architecture-guards.sh` | pass — all 150 checks, `ALL PASSED` |
| Guard 148 / 150 negative tests | **pass** — see below |
| `cargo fmt --all --check` | pass after `cargo fmt --all` (see §4.3) |
| `make check` | pass — see §4.4 |

#### 4.1 Direct proof the 6 advanced types still resolve

Run before landing the seam suite, as a throwaway integration test, then converted into
`tests/fuzzer_payload_corpus_seam.rs`:

```
GraphQL: 15 payloads, first="{__schema{queryType{name}}}"
OAuth:   22 payloads, first="redirect_uri=https://evil.com/callback"
Jwt:     25 payloads, first="eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJ..."
Idor:    21 payloads, first="id=1"
Ssti:    27 payloads, first="{{7*7}}"
Grpc:    14 payloads, first="{\"listServices\":{}}"
distinct types present: 40
```

All 6 advanced types return real payloads, and all 40 distinct types appear in the cached
union — i.e. no silent-empty regression.

#### 4.2 Guard negative tests

Check 150 was exercised by injecting each failure it exists to catch: a probe module
missing from the engine, a probe module copied into the corpus crate, a `Vec::new()` stub
reintroduced, a cross-variant cache moved into the corpus crate, the engine delegation
removed, and `PAYLOAD_CACHE` de-lazied. Each produced a FAIL and exit 1.

A **real defect in check 148** was found and fixed during this milestone. It initially
matched the bare substring `Scope`, so the dependency-confusion payload corpus tripped it:

```
FAIL: crates/eggsec-payloads/src references authorization vocabulary
crates/eggsec-payloads/src/dep_confusion.rs:7:
  ("@scope/internal-package", "Scoped package confusion targeting internal scope", ...)
```

That is payload *text*, not authorization code. The check now matches whole identifiers
(`\bScope\b`, …), which ignores `Scoped` and `@scope/`, and it was re-verified to still
fail on genuine `pub struct Scope;` / `Capability` usage.

#### 4.3 Formatting

The first `make check` run **failed at `cargo fmt --all --check`** (exit 2) on two missing
trailing newlines in newly written files — `fuzzer/payloads/mod.rs` and
`tests/fuzzer_payload_corpus_seam.rs`. Fixed with `cargo fmt --all`; re-verified clean
before re-running. No test or guard had run at that point, so nothing was masked.

#### 4.4 `make check`

Green, exit code 0, including `cargo test -p eggsec-payloads --tests` registered after
`eggsec-secrets`, and the new seam suite picked up by `cargo test -p eggsec --features
rest-api,cli --tests`.

Deep checks (`make check-full`, `make check-features-individual`, `make clippy-domain`,
`make test-tui-pty`) were **not run** — no feature surface changed.
`make check-deps` ran inside `make check` and passed. `make check-python` was **not run**:
no Python binding, stub, or doc changed, and the bindings' diff is empty by proof.

## 5. Invariant review

| Source-plan invariant | Evidence |
|---|---|
| `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate | No dispatch or enforcement code touched. |
| Corpus crate authorizes nothing, resolves nothing, opens no socket, spawns no process | No such code moved; `cargo tree` has no network/runtime crate; check 148 enforces and now correctly ignores payload text. |
| Only workspace dependency is `eggsec-core` | `cargo tree`; check 148. |
| **`LazyLock` laziness is load-bearing** | Caches remain `LazyLock`, engine-side; no eager static introduced; check 150 fails if `PAYLOAD_CACHE` stops being a `LazyLock`. Confirmed by inspecting the moved source, not by diff. |
| 40 variants, `Payload` shape, and cached behavior unchanged | `tests/fuzzer_payload_corpus_seam.rs` asserts the variant count is still 40 and the cached union equals the per-type sum; `tests/fuzzer_tests.rs` passes unmodified. |
| No payload added, removed, re-rated, or re-tagged | Only import/path rewrites; 233-test count identical. |
| Engine paths `eggsec::fuzzer::PayloadType` and `::payloads::*` valid | Empty consumer diff + green workspace check. |
| The 6 live-probe modules remain the only payload code with network access | check 150 asserts each exists engine-side and does not exist in the corpus crate. |
| Workspace path graph acyclic (check 108) | Passes in the 150-check run. |

## 6. Failure and recovery review

Not applicable. Payload construction is synchronous data assembly behind `LazyLock`. No
tokio task is introduced, so no timeout wrapper applies. The 6 live-probe modules are
unchanged and remain the only asynchronous payload code, with the engine's existing
timeout and cancellation behavior untouched.

The one behavior deliberately changed is `eggsec_payloads::get_payloads(advanced)`: it now
panics instead of returning empty. A panic on a programming error at the boundary is the
correct trade — the alternative is a silent wrong answer — and the panic message names the
correct API and the `is_advanced()` filter.

## 7. Migration and compatibility review

No storage, protocol, configuration, or wire migration.

As in M003, the load-bearing compatibility property is **type identity**: `PayloadType`
appears in Python binding signatures and in `waf_validation.rs`'s exhaustive
`parse_payload_type`. Both compile with zero diffs, which is the specific regression that
would catch a re-type instead of a re-export.

The crate is `publish = false` and internal, so no external consumer is affected. Rollback
is a revert; `Cargo.lock` gains one internal package entry.

## 8. Security review

- **No new network surface, and a smaller one than before.** `eggsec-payloads` compiles without `reqwest`, so a payload corpus cannot make requests, leak scope, or bypass a transport checkpoint. The 6 probing modules remain engine-side and therefore remain inside the existing enforcement and timeout path — which is exactly why they were not moved.
- **Authorization:** untouched. The corpus crate has no scope, capability, or approval surface, and check 148 now enforces that without misfiring on payload text.
- **Content integrity is the security-relevant property here.** A payload corpus is security-relevant data: adding, removing, re-rating, or re-tagging a payload changes what the tool reports. Only import rewrites were made; the 233-test contract held, and `tests/fuzzer_tests.rs` (which includes `test_payload_audit_no_placeholders`) passes unmodified.
- **Regression detection improved.** 233 corpus tests now run in a ~7k-line crate in ~0.1s instead of requiring the full engine build, so payload regressions surface far faster in review.

## 9. Documentation and operations

Updated:

- `architecture/fuzzer.md` — `payloads/` row replaced with the corpus/probe seam table; new "Corpus/probe seam" section documenting the three consequences (caches cannot move, panic-over-empty, three layers of guarding); stale `payloads/mod.rs:NNN` line citations repointed at the owning crate; laziness note updated with the check-150 reference.
- `architecture/overview.md` — crate-table row, dependency-map node, guard bullet for `eggsec-payloads`.
- `architecture/capability_segregation.md` — Phase G status for the implemented crate, including the cache-ownership rationale and the trap that was designed out; Guards bullet updated.
- `crates/eggsec-payloads/src/lib.rs` — crate doc header covering the corpus/probe split, the panic contract, why the caches are absent, and why laziness is engine-owned.
- `crates/eggsec/src/fuzzer/payloads/mod.rs` — doc header explaining the split, the union ownership, and that the facade is permanent.

Static guards and tests:

- **Check 148** — leaf invariant; a substring bug found and fixed during this milestone.
- **Check 150** — new; corpus/probe seam structure.
- **`crates/eggsec/tests/fuzzer_payload_corpus_seam.rs`** — new; 3 tests covering the seam behaviorally.

Operator diagnostics: none added. No behavior, flag, output, or configuration changed.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | Check 148's authorization-vocabulary check was matching a bare substring `Scope` and would false-positive on any corpus payload whose *text* contains that word. Found and fixed in this milestone (word-boundary matching). | Before the fix, check 148 could not have been enabled for a payload corpus at all. The bug is fixed and re-verified against both payload text and real type usage. | None. Recorded because guard scripts are not type-checked and this class of defect is easy to reintroduce: any future guard matching a bare word over corpus text risks the same false positive. |
| low | `pub use eggsec_payloads::*;` in the engine's `payloads/mod.rs` is a glob re-export. | Low risk here because the corpus crate's public surface is small and all of it is meant to be visible at that path. A future rename inside the corpus crate could surface at the engine path unexpectedly. | Acceptable. Revisit if the corpus crate's public API grows unrelated helpers that should not appear under `eggsec::fuzzer::payloads`. |
| low | The 6 live-probe modules remain engine-side and therefore remain the only payload code with network access. | Intentional (ADR-0005 Option D). | None. |

No critical or high findings.

## 11. Roadmap disposition

**Milestone closed and next dependency may proceed.**

No stop condition was triggered: the bindings and TUI needed no edits; no payload,
severity, tag, or variant change was required; the 6 live-probe modules stayed
engine-side **without** dragging `reqwest` into the corpus crate, so the seam held; and
scope stayed out of `fuzzer/engine/`, the mutator, and the filters.

The one deviation from the plan's literal instructions was deliberate and is documented in
§3: the caches moved engine-side rather than staying in the corpus crate, because a
corpus-side cache cannot be correct for all 40 variants. The plan's substantive
requirement — never ship a silent-empty stub for the 6 — was met more strongly than a
mechanical move would have.

All four extraction/disposition milestones are now closed. **M005 is the only remaining
milestone**, and it is blocked on a maintainer publication decision that is not a code
question.

## 12. Registry updates

- `plans/subsystems/security-knowledge-corpus-roadmap.md` — milestone 4 status → closed; §5 target architecture marks `eggsec-payloads` landed; §7 milestone 4 outcome recorded.
- `plans/registry.md` — milestone 4 status → closed; `eggsec-payloads` recorded with guards 148/150 and the seam suite.
- `plans/implementation/security-knowledge-corpus/004-payload-corpus-extraction.md` — status → closed with the closure reference.