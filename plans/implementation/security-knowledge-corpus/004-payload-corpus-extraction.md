# Security Knowledge Corpus Milestone 004 — Payload corpus extraction

Status: ready for handoff

Repository baseline: `fix/cli-usability-audit` at `979dca67` (pushed); this plan lands on a branch cut from it

Source roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-4--payload-corpus-extraction`

Long-term requirements:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/002-long-term-roadmap.md#phase-1--crate-boundaries-and-reusable-library-ownership`

Applicable ADRs:

- `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`

Primary class: infrastructure

## 1. Objective

Extract the 34 pure-data payload modules (7,084 lines, 233 tests) from
`crates/eggsec/src/fuzzer/payloads/` into a new leaf crate `eggsec-payloads`, leaving
the 6 live-probe modules engine-side, with no consumer import changes.

This is the largest dependency-closure reduction in the subsystem.

## 2. Why this milestone is ready

No hard or interface dependencies.

The corpus splits on a seam that already exists in the source, not one invented for this
milestone: **34 of the 40 payload-type modules (36 of 42 files, counting `mod.rs` and
`macros.rs`) never reference `reqwest`** (7,084 lines of pure data),
while 6 mix payload generation with live probing (4,354 lines: `graphql`, `grpc`,
`idor`, `jwt`, `oauth`, `ssti`). ADR-0005 Option D records rejecting the alternative of
extracting all 41 together.

Measured coupling from the pure-data subset to engine code is effectively zero: only
self-referential `crate::fuzzer::payloads::` paths, plus one `crate::types::Severity`
re-export in `mod.rs`. External dependencies are `serde`, `strum` (derive), `flate2`,
`rustc-hash`, and `tracing`.

The extraction was validated by spike. The 36 files were copied into a scratch crate
depending on `eggsec-core` + `serde` + `strum` + `flate2` + `rustc-hash` + `tracing`, and
**233 tests passed unmodified** after exactly four mechanical edits:

1. `crate::types::Severity` → `eggsec_core::Severity` (in `mod.rs` and `oast.rs`).
2. `crate::fuzzer::payloads::{…}` → `crate::payloads::{…}` across the moved files.
3. The `$crate::fuzzer::payloads::Payload` macro path in `macros.rs`.
4. Six `get_payloads` match arms redirected, because those modules stay behind.

No behavioral change was required.

## 3. Current implementation evidence

`crates/eggsec/src/fuzzer/payloads/` holds 42 files totalling 11,438 lines: two
framework files (`mod.rs`, `macros.rs`) plus 40 payload-type modules, of which 34 are
pure data and 6 mix in live probing.

- **`mod.rs` (236)** — the `PayloadType` enum with **40 variants**, its `Display` impl,
  `is_advanced()`, `all_variants()`, the `Payload` struct
  (`payload_type`, `payload`, `description`, `severity`, `tags`), the
  `PAYLOAD_CACHE` / `ALL_PAYLOADS_CACHE` `LazyLock`s, and the
  `get_payloads(PayloadType) -> Vec<Payload>` dispatch (40 match arms).
- **`macros.rs` (89)** — the `payload_vec!` builder macro, whose body references
  `$crate::fuzzer::payloads::Payload`.
- **34 pure-data modules** — `cache`, `cmd`, `compression`, `css_inject`, `csv`,
  `dep_confusion`, `deser`, `dom_clobber`, `expression`, `headers`, `host`, `html_inject`,
  `latex`, `ldap`, `mass_assign`, `nosql`, `oast`, `prototype`, `race`, `redirect`,
  `redos`, `saml`, `soap`, `sqli`, `ssi`, `ssrf`, `traversal`, `viewstate`, `websocket`,
  `xpath`, `xs_leak`, `xslt`, `xss`, `xxe`.
- **6 live-probe modules** — `graphql`, `grpc`, `idor`, `jwt`, `oauth`, `ssti`. Each
  takes `&reqwest::Client` and performs asynchronous probing. **These stay.**

Consumers (engine + cross-crate):

| Consumer | Kind |
|---|---|
| `eggsec-python/src/oauth.rs` (21 refs), `graphql.rs` (17), `engine.rs`, `async_engine.rs`, `waf_validation.rs` | cross-crate |
| `eggsec-tui/src/tabs/fuzz.rs` | cross-crate |
| `eggsec/src/fuzzer/{advanced,grammar,calibration,filters,mod}.rs` | engine |
| `eggsec/src/fuzzer/engine/{core,utils,types,execution,advanced}.rs` | engine |
| `eggsec/src/dispatch/api.rs`, `eggsec/src/waf/bypass/evasion.rs`, `eggsec/src/ai/mod.rs`, `eggsec/src/fuzzer/api_schema/mod.rs` | engine |

`eggsec-python/src/waf_validation.rs:401` includes an exhaustive
`parse_payload_type` mapping every string alias to a `PayloadType` variant, so
`PayloadType`'s path identity must be preserved exactly.

## 4. Invariants that must not regress

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate. Payload
  generation is not a policy decision, and the corpus crate must not acquire one.
- `eggsec-payloads` authorizes nothing, resolves nothing, opens no socket, and spawns no
  process. Its only workspace dependency is `eggsec-core`; no Tokio, `reqwest`, `rustls`,
  filesystem, frontend, engine, or transport dependency.
- **`LazyLock` laziness is load-bearing.** `PAYLOAD_CACHE` and `ALL_PAYLOADS_CACHE`
  MUST stay lazy. Eagerly materializing all 40 variants at startup is a measured
  regression and is prohibited.
- `PayloadType`'s 40 variants, the `Payload` struct shape, and `get_payloads` /
  `get_payloads_cached` / `get_all_payloads_cached` behavior are unchanged. No payload
  is added, removed, re-rated, or re-tagged.
- Engine path `eggsec::fuzzer::PayloadType` and `eggsec::fuzzer::payloads::*` remain
  valid.
- The 6 live-probe modules remain in the engine and are the only payload code with
  network access.
- Workspace path graph stays acyclic (guard 108).

## 5. Scope

### In scope

- New crate `crates/eggsec-payloads`.
- Move `mod.rs`, `macros.rs`, and the 34 pure-data modules.
- Engine re-export preserving existing paths.
- Update the 6 live-probe modules' imports to the re-exported crate.
- A new guard enforcing the leaf invariant.
- `make check` per-crate test line.
- Documentation updates.

### Explicitly out of scope

- **The 6 live-probe modules.** `graphql`, `grpc`, `idor`, `jwt`, `oauth`, `ssti` stay in
  `eggsec::fuzzer::payloads` and import `Payload` / `PayloadType` from the new crate.
  This is ADR-0005 Option D.
- Any payload content change. The 233-test count is the contract.
- `fuzzer/engine/`, `fuzzer/mutator.rs`, `fuzzer/filters.rs`, `fuzzer/grammar.rs`,
  `fuzzer/advanced.rs` — executor-bound.
- `fuzzer/redos_detect.rs`, `fuzzer/waf_fingerprint.rs`, `fuzzer/state.rs`,
  `fuzzer/chain.rs`, `fuzzer/diff.rs`.
- Introducing new payload types "while we're here".
- Enabling publication.

## 6. Required production changes

### Core/domain

- Create `crates/eggsec-payloads/` with a manifest declaring `eggsec-core`, `serde`
  (derive), `strum` (derive, `default-features = false` to match the engine's
  declaration), `flate2`, `rustc-hash`, `tracing`. Set `publish = false`.
- Add it to the root `Cargo.toml` `[workspace] members`.
- Move `mod.rs`, `macros.rs`, and the 34 data modules verbatim.
- Apply the four mechanical edits identified by the spike (§2). No other source change.
- Keep `Payload`, `PayloadType`, `Severity` (re-export), `get_payloads`,
  `get_payloads_cached`, `get_all_payloads_cached`, `PayloadType::is_advanced`, and
  `PayloadType::all_variants` public at the crate root.
- `get_payloads` must retain all 42 arms. For the 6 relocated types, the engine — not
  the corpus crate — resolves them. The spike's `Vec::new()` placeholder is a proof
  artifact and **must not** ship: returning an empty vector for `GraphQL` would be a
  silent capability regression. The engine keeps its own dispatch for those 6.

### Protocol and DTOs

No wire-format change. `PayloadType` appears in Python binding signatures, so its type
identity must remain reachable at the same path.

### Runtime and concurrency

Synchronous; `LazyLock` caches stay lazy. No new tasks.

### Frontend or operator surface

- `crates/eggsec/src/fuzzer/mod.rs`: re-export the corpus crate's public items so
  `eggsec::fuzzer::{PayloadType, Payload, get_payloads, …}` resolves unchanged.
- The 6 live-probe modules: update `use super::{…}` / `crate::fuzzer::payloads::{…}`
  imports to the re-exported path. Expected and allowed.
- Engine call sites listed in §3 should require **no** edits.

### Security and authorization

None. The corpus crate must not import `eggsec-policy`, `eggsec-transport`, or engine
config.

A security property worth stating in the closure record rather than claiming as new: the
extracted crate has no network surface at all, so a payload corpus cannot itself make
requests, leak scope, or bypass transport checkpoints. That is a direct consequence of
keeping the 6 live-probe modules behind.

### Documentation and static guards

- New guard: assert `eggsec-payloads` has no `tokio`, `reqwest`, `rustls`,
  frontend, engine, or transport dependency; that its only `eggsec-*` dependency is
  `eggsec-core`; and that it contains no `Scope` / `ApprovedOperation` / `Capability`
  reference.
- Add `cargo test -p eggsec-payloads --tests` to `make check`.
- Verify whether `docs/FEATURE_MATRIX.md`'s domain-crate inventory is authoritative
  before editing; `eggsec-udp-scan` is not listed today.

## 7. Ordered work packages

### Work package A — Create the crate with `mod.rs` and `macros.rs`

Intent: establish the crate shape first, because the macro path rewrite is the one edit
that touches every moved module.

Required changes: manifest, `members` entry, `mod.rs` (with the 42-arm `get_payloads`
reduced only as specified in §6), `macros.rs` with
`$crate::fuzzer::payloads::Payload` → `$crate::Payload`.

Acceptance evidence: `cargo check -p eggsec-payloads` succeeds with the 6 relocated
types routed to engine-owned dispatch, not to empty vectors.

### Work package B — Move the 34 data modules

Intent: bulk move.

Required changes: copy the files; apply the `crate::fuzzer::payloads::` → `crate::payloads::`
rewrite in `oast.rs` and any others that reference the old path.

Acceptance evidence: `cargo test -p eggsec-payloads` passes **233 tests**, unchanged.

### Work package C — Re-export and update the 6 live-probe modules

Intent: preserve source compatibility.

Required changes: `fuzzer/mod.rs` re-export; update the 6 modules' imports; delete the
engine-side copies of the moved files.

Acceptance evidence: `cargo check --workspace --no-default-features` green; `git diff` shows **no** changes under `crates/eggsec-python/` or `crates/eggsec-tui/`.

### Work package D — Guard and test line

Required changes: new guard check; `make check` line.

Acceptance evidence: the guard fails when `reqwest` is added to the new crate's manifest.

### Work package E — Documentation

Required changes:

- `architecture/fuzzer.md` — new corpus owner; explicitly document that the 6
  live-probe modules remain engine-side and why.
- `architecture/overview.md` — crate table row and dependency-map edge.
- `architecture/capability_segregation.md` — Phase G entry.
- `architecture/python_api.md` — confirm `parse_payload_type`'s 42-variant mapping is
  unchanged and say so explicitly.
- `.opencode/skills/` — any claim naming the payload corpus owner or a payload count.
  Per `AGENTS.md`, skill claims about counted sets (payloads, techniques) drift silently
  unless updated in the same pass as the code.

## 8. Failure, cancellation, restart, and contention semantics

Not applicable. The corpus is synchronous data construction behind `LazyLock`. No tokio
tasks are introduced, so no timeout wrappers are required.

The one behavioral property to protect is laziness: `PAYLOAD_CACHE` and
`ALL_PAYLOADS_CACHE` are `LazyLock`, and the cache-miss path builds payloads on first
access. If the extraction accidentally forces initialization (for example by an eager
static in the new crate), startup cost regresses across every binary including the TUI.
Confirm laziness is preserved by inspecting the moved `mod.rs`, not by reading a diff.

## 9. Compatibility and migration

No storage, protocol, configuration, or wire migration.

Source compatibility is the contract: `eggsec-python`'s exhaustive `parse_payload_type`
and the TUI fuzz tab must compile with zero diffs. Preserving `PayloadType` as a
re-export (not a re-definition) is what makes this achievable.

Because the crate is `publish = false` and internal, no external consumer can be affected.
This is the argument for staging extraction ahead of publication under ADR-0005
decision 3.

## 10. Required tests

### Focused unit tests

- The 233 existing tests, carried unmodified. Count is the contract.
- `macros.rs`'s own `#[cfg(test)]` tests included in that count.

### Integration tests

- `cargo test -p eggsec --features rest-api,cli --tests` per `make check`.
- The fuzzer integration suites in `crates/eggsec/tests/`.

### Restart and recovery tests

None. No persisted state.

### Contention and cancellation tests

None.

### Security and negative tests

- Confirm a representative payload from several classes (SQLi, XSS, traversal, command
  injection, deserialization) is still returned with unchanged payload text, severity,
  and tags after the move.
- Confirm `cargo tree -p eggsec-payloads` shows no `reqwest`, no `tokio`, and no
  workspace edge other than `eggsec-core`.
- Confirm the 6 live-probe types still resolve to real payloads through the engine's
  dispatch (not empty vectors).

### Migration and compatibility tests

- `cargo check -p eggsec -p eggsec-python -p eggsec-tui --no-default-features` green
  with zero consumer diffs.

## 11. Required verification commands

```bash
cargo test -p eggsec-payloads --tests
cargo tree -p eggsec-payloads
make check                  # mandatory Rust contract
make check-deps
bash scripts/check-architecture-guards.sh
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

As work package E.

## 13. Acceptance criteria

- `crates/eggsec-payloads` exists with `eggsec-core` as its only workspace dependency and
  no `reqwest`/Tokio/TLS/filesystem/frontend/engine/transport dependency.
- `cargo test -p eggsec-payloads` passes **233 tests**, unmodified.
- `eggsec::fuzzer::{PayloadType, Payload}` and `eggsec::fuzzer::payloads::*` resolve;
  `git diff` shows no changes to `eggsec-python` or `eggsec-tui`.
- The 6 live-probe modules remain in the engine and still produce real payloads.
- `LazyLock` laziness is preserved; no eager payload materialization was introduced.
- The new guard exists and is demonstrated to fail on a forbidden dependency.
- `make check` is green.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- preserving `eggsec::fuzzer::PayloadType` would require editing the Python bindings or
  the TUI;
- any payload, severity, tag, or variant change appears to be required;
- the 6 live-probe modules cannot be kept engine-side without dragging `reqwest` into
  the corpus crate — that would mean the seam does not hold, and the milestone needs
  re-scoping rather than a workaround;
- an authorization, scope, or network need appears in the corpus;
- scope would expand into `fuzzer/engine/`, the mutator, or the filters.

## 15. Closure evidence required

`plans/closure/security-knowledge-corpus/004-closure.md` containing:

- `cargo test -p eggsec-payloads` output showing 233 passed;
- `cargo tree -p eggsec-payloads` output;
- `git diff --stat` proving zero changes under `crates/eggsec-python/` and
  `crates/eggsec-tui/`;
- the diff showing the moved modules are content-identical apart from the four
  mechanical edits;
- confirmation that `PAYLOAD_CACHE` / `ALL_PAYLOADS_CACHE` are still `LazyLock`;
- a test or check demonstrating the 6 live-probe payload types still return non-empty
  results through the engine's dispatch;
- the new guard check's name and a demonstration that it fails on a forbidden dependency;
- `make check` outcome and guards outcome;
- residual findings classified by severity;
- recommendation: closed, conditionally closed, corrective pass required, or blocked.

## 16. Handoff notes

This is the largest milestone in the subsystem, and the risk is not the move — it is the
temptation to tidy up 7,000 lines of payload data while moving it. Resist it: every
non-mechanical edit is a content change to a security corpus, and the 233-test count only
means something if the corpus itself is untouched.

The specific trap is the spike's `Vec::new()` placeholder for the 6 relocated types. It
was a proof-of-compilation artifact. Shipping it would silently return empty payload
lists for GraphQL, OAuth, JWT, IDOR, SSTI, and gRPC — a capability regression that no
test in the corpus crate would catch, because those modules stay behind. The engine must
retain real dispatch for them.

Preserve unrelated user changes. The two untracked plan files
(`plans/c2-real-simulation-differentiation-plan.md`, `plans/post-exploit-plan.md`) in the
working tree belong to the user — do not modify or delete them.