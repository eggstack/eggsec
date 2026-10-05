# Security Knowledge Corpus Milestone 003 — Secret detection extraction

Status: closed (`plans/closure/security-knowledge-corpus/003-closure.md`; `eggsec-secrets` extracted with a one-line body diff, zero consumer diffs, guard 149 added)

Repository baseline: `fix/cli-usability-audit` at `979dca67` (pushed); this plan lands on a branch cut from it

Source roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-3--secret-detection-extraction`

Long-term requirements:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/002-long-term-roadmap.md#phase-1--crate-boundaries-and-reusable-library-ownership`

Applicable ADRs:

- `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`

Primary class: infrastructure

Blocked by: `plans/implementation/security-knowledge-corpus/001-redaction-dead-code-disposition.md`
(interface dependency — M001's disposition decides whether `utils/redaction.rs` joins
this crate).

## 1. Objective

Extract `crates/eggsec/src/recon/secrets.rs` (492 lines, 11 tests) into a new leaf crate
`eggsec-secrets`, re-exported from the engine at its existing path, without editing the
Python bindings.

## 2. Why this milestone is ready

**Interface dependency only.** M001 must be closed because its disposition decides this
milestone's scope: if `redaction.rs` is adopted into `eggsec-secrets`, that code moves
here; if it is deleted or wired elsewhere, it does not. Starting before M001 closes risks
a second move.

There is no hard dependency. Measured coupling from `secrets.rs` is **exactly one**
`crate::` reference — `pub use crate::types::Severity;` — and `Severity` is itself a
re-export of `eggsec_core::types::Severity` (`crates/eggsec/src/types.rs:16`). External
dependencies are `regex`, `serde`, and `tracing` only. No `crate::error`, no Tokio, no
`reqwest`.

The extraction was validated by spike: `secrets.rs` was copied into a scratch crate
depending on `eggsec-core` + `regex` + `serde` + `tracing` and compiled with all **11
tests passing unmodified**.

## 3. Current implementation evidence

`crates/eggsec/src/recon/secrets.rs` is credential-detection knowledge:

- `SecretFinding` — a detected credential result.
- `SecretType` — an enum with **30 variants**.
- `Confidence` — a confidence tier.
- `SecretPattern` / `build_patterns()` — **25 patterns** covering 20 of the 30 variants, compiled behind a
  `LazyLock<Vec<SecretPattern>>`.
- `SecretScanner` — the scanning facade.
- `secret_entropy(value) -> f64` — Shannon-entropy gate.
- `scan_content(&str) -> Vec<SecretFinding>` — the pure scan entry point.

The hot path is pure: regex matching plus an entropy gate that is applied **only** to
AWS secret-key candidates — `if pattern.secret_type == SecretType::AwsSecretKey &&
secret_entropy(value) < 3.5` — so that ordinary 40-character matches do not fire on
low-entropy noise. It is not a global threshold. `scan_file()` touches
`std::fs` and is a small separable helper.

**Consumers.** Engine-side: `recon/git_secrets.rs`, and three auth-probe modules
(`recon/ftp_auth.rs`, `recon/smtp_auth.rs`, `recon/ssh_auth.rs`). Cross-crate:
`eggsec-python/src/git_secrets.rs`, `eggsec-python/src/engine.rs`,
`eggsec-python/src/async_engine.rs`, `eggsec-mobile-lab/src/ipa.rs`, and
`crates/eggsec/tests/recon_secrets_tests.rs`.

**The binding constraint is the Python bindings.**
`crates/eggsec-python/src/git_secrets.rs` matches all 30 `SecretType` variants
exhaustively. If `SecretType` is re-defined in the new crate rather than re-exported,
that match breaks and every consumer must be edited — which ADR-0005 decision 2 forbids.
`eggsec-mobile-lab/src/ipa.rs:154` contains a comment that explicitly avoids this module,
which indicates the coupling is already felt at the boundary.

**`git_secrets.rs` is out of scope.** It is 473 lines of `std::process::Command::new("git")`
orchestration plus filesystem walking — subprocess orchestration, not pattern matching. It
stays in the engine and imports from the new crate. A future ADR may extract it as a
sibling; this milestone does not.

## 4. Invariants that must not regress

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate. Secret
  detection is not a policy decision.
- `eggsec-secrets` authorizes nothing, resolves nothing, opens no socket, and spawns no
  process. It must not gain `Scope`, `Capability`, or `ApprovedOperation` access.
- `eggsec-secrets`'s only workspace dependency is `eggsec-core`; no Tokio, HTTP, TLS,
  filesystem, frontend, engine, or transport dependency. If `redaction.rs` joins it,
  `serde_json` is additionally acceptable (for `redact_json`).
- **The entropy gate is a detection-semantics constant.** The condition
  `pattern.secret_type == SecretType::AwsSecretKey && secret_entropy(value) < 3.5` MUST
  remain byte-identical, including its narrow scoping to AWS secret keys. Widening it to
  every pattern type, or changing the threshold, changes detection results and is a
  capability change, not infrastructure.
- The 25-pattern corpus is content, not refactor target. No pattern is added, removed,
  or re-ranked.
- Workspace path graph stays acyclic (guard 108).
- Engine path `eggsec::recon::secrets::*` remains valid.
- `SecretType` and `Confidence` are **re-exported**, never re-typed, so the Python
  bindings' exhaustive match compiles unchanged.

## 5. Scope

### In scope

- New crate `crates/eggsec-secrets`.
- Move `secrets.rs` content into it.
- Engine re-export preserving `eggsec::recon::secrets::*`.
- `recon/git_secrets.rs` updated to import from the new crate (this is an expected
  source edit inside the engine, not a consumer break).
- A new guard enforcing the leaf invariant.
- `make check` per-crate test line.
- Documentation updates.

### Explicitly out of scope

- `recon/git_secrets.rs` extraction — subprocess orchestration, deferred to a future ADR.
- Any change to the 25 patterns, the 30 `SecretType` variants, the `Confidence` tiers, or
  the entropy threshold.
- Adding new secret patterns "while we're here". Content changes are a capability change
  and would invalidate the 11-test count as a contract.
- `recon/techdetect.rs`, `recon/content.rs`, and the rest of the `recon` bulk.
- M001's redaction work, beyond receiving its output.
- Enabling publication.

## 6. Required production changes

### Core/domain

- Create `crates/eggsec-secrets/` with a manifest declaring `eggsec-core`, `regex`,
  `serde` (derive), `tracing`, and — only if M001 adopted it — `serde_json`. Set
  `publish = false`.
- Add it to the root `Cargo.toml` `[workspace] members`.
- Move the module content verbatim. The one mechanical change is
  `pub use crate::types::Severity;` → `pub use eggsec_core::types::Severity;`, or an
  equivalent import of the re-exported type.
- Keep `SecretFinding`, `SecretType`, `Confidence`, `SecretScanner`, `scan_content`,
  and `secret_entropy` public at the crate root so the engine re-export is a single
  `pub use`.

### Protocol and DTOs

No wire-format change. Note that `SecretType` appears in Python binding signatures, so
its **type identity** must remain reachable at the same path even though the defining
crate changes.

### Runtime and concurrency

Synchronous and stateless. `LazyLock` pattern compilation stays lazy.

### Frontend or operator surface

- `crates/eggsec/src/recon/mod.rs`: re-export the new crate's items so
  `eggsec::recon::secrets::*` keeps resolving.
- `crates/eggsec/src/recon/git_secrets.rs`: update its `crate::recon::secrets::*` imports
  to the re-exported path. Expected and allowed.

### Security and authorization

None. The crate must not import `eggsec-policy`, `eggsec-transport`, or engine config.

One security-relevant property to preserve deliberately: the detector holds no engine
secret state and no engine credential material. Keeping it in a crate with no engine
dependency means it cannot reach engine secrets — a mildly security-positive change that
should be noted in the closure record rather than claimed as a new capability.

### Documentation and static guards

- New guard: assert `eggsec-secrets` has no `tokio`, `reqwest`, `rustls`,
  `hickory-resolver`, frontend, engine, or transport dependency; that its only
  `eggsec-*` dependency is `eggsec-core`; and that it contains no `Scope` /
  `ApprovedOperation` / `Capability` reference.
- Add `cargo test -p eggsec-secrets --tests` to `make check`.
- Update `docs/FEATURE_MATRIX.md` domain inventory only if its fixed list is
  authoritative — `eggsec-udp-scan` is not currently listed, so verify before editing.

## 7. Ordered work packages

### Work package A — Confirm M001's disposition

Intent: fix this milestone's scope.

Required changes: read M001's closure record. If it adopted `redaction.rs`, plan to move
it here and add `serde_json` to the manifest; if not, exclude it.

Acceptance evidence: the scope decision is stated in the closure record.

### Work package B — Create the crate and move the module

Intent: standalone crate.

Required changes: manifest, `members` entry, module file, the single `Severity` import
rewrite.

Acceptance evidence: `cargo test -p eggsec-secrets` passes **11 tests**, unchanged. If M001 adopted redaction, that test count rises and the closure record states both numbers.

### Work package C — Re-export and update engine call sites

Intent: preserve source compatibility.

Required changes: `recon/mod.rs` re-export; `recon/git_secrets.rs` import update; delete
the engine-side file.

Acceptance evidence: `cargo check --workspace --no-default-features` green; `git diff` shows **no** changes under `crates/eggsec-python/` or `crates/eggsec-mobile-lab/`.

### Work package D — Guard and test line

Intent: prevent dependency creep and register the tests.

Required changes: new guard check; `make check` line.

Acceptance evidence: the guard fails when `eggsec-policy` or `tokio` is added to the new crate's manifest.

### Work package E — Documentation

Required changes:

- `architecture/recon.md` — new owner for secret detection; note that `git_secrets`
  remains engine-side.
- `architecture/overview.md` — crate table row and dependency-map edge.
- `architecture/capability_segregation.md` — Phase G entry.
- `architecture/python_api.md` — confirm the bindings' `SecretType` mapping is
  unchanged and say so explicitly, since this is the milestone's main risk.
- `.opencode/skills/` — any claim naming the secret-detection owner.

## 8. Failure, cancellation, restart, and contention semantics

Not applicable. Pure synchronous matching over a `LazyLock` pattern set. No tokio tasks,
so no timeout wrappers. No cancellation or restart behavior changes.

If `redaction.rs` joins the crate (M001 Option 1), its functions remain pure and
side-effect free.

## 9. Compatibility and migration

No storage, protocol, configuration, or wire migration.

Source compatibility is the load-bearing requirement here: the Python bindings match 30
`SecretType` variants exhaustively, and `eggsec-mobile-lab` references the module. Both
must compile with **zero** diffs. This is what distinguishes a correct extraction from a
consumer-breaking one.

## 10. Required tests

### Focused unit tests

- The 11 existing tests, carried unmodified. Count is the contract.
- If M001 adopted redaction, its 26 tests move with it and are re-run.

### Integration tests

- `cargo test -p eggsec --features rest-api,cli --tests` per `make check`.
- `crates/eggsec/tests/recon_secrets_tests.rs` passes unmodified.

### Restart and recovery tests

None. No persisted state.

### Contention and cancellation tests

None.

### Security and negative tests

- Confirm a known fake credential in each of a few pattern classes is still detected
  after the move, and that a low-entropy candidate is still rejected by the unchanged
  gate. This is a regression check on the entropy constant, not a new test.
- Confirm `cargo tree -p eggsec-secrets` shows `eggsec-core` as its only workspace
  edge.

### Migration and compatibility tests

- `cargo check -p eggsec -p eggsec-python -p eggsec-mobile-lab --no-default-features`
  green with zero consumer diffs. This is the specific regression that would catch a
  re-type instead of a re-export.

## 11. Required verification commands

```bash
cargo test -p eggsec-secrets --tests
cargo tree -p eggsec-secrets
make check                  # mandatory Rust contract
make check-deps
bash scripts/check-architecture-guards.sh
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

As work package E.

## 13. Acceptance criteria

- `crates/eggsec-secrets` exists with `eggsec-core` as its only workspace dependency and
  no Tokio/HTTP/TLS/filesystem/frontend/engine/transport dependency.
- `cargo test -p eggsec-secrets` passes 11 tests (plus redaction tests if adopted),
  unmodified.
- `eggsec::recon::secrets::*` resolves; `git diff` shows no changes to
  `eggsec-python`, `eggsec-mobile-lab`, or `crates/eggsec/tests/`.
- The entropy threshold is byte-identical.
- `cargo tree -p eggsec-secrets` shows no workspace edge other than `eggsec-core`.
- The new guard exists and is demonstrated to fail on a forbidden dependency.
- `make check` is green.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- M001's closure record is absent or its disposition is ambiguous;
- preserving `eggsec::recon::secrets::*` would require editing the Python bindings or
  `eggsec-mobile-lab` — this is the milestone's defining constraint, and a violation
  means the extraction must be re-shaped (for example by moving the engine-facing
  types rather than the crate-facing ones);
- any pattern, threshold, or variant change appears to be required;
- an authorization, scope, or network need appears;
- scope would expand to `git_secrets.rs` or the rest of `recon`.

## 15. Closure evidence required

`plans/closure/security-knowledge-corpus/003-closure.md` containing:

- M001's disposition and how it was applied;
- `cargo test -p eggsec-secrets` output with the exact test count;
- `cargo tree -p eggsec-secrets` output;
- `git diff --stat` proving zero changes under `crates/eggsec-python/`,
  `crates/eggsec-mobile-lab/`, and `crates/eggsec/tests/`;
- a before/after of the entropy threshold line showing it unchanged;
- the new guard check's name and a demonstration that it fails on a forbidden dependency;
- `make check` outcome and guards outcome;
- residual findings classified by severity;
- recommendation: closed, conditionally closed, corrective pass required, or blocked.

## 16. Handoff notes

The extraction itself is mechanical and spike-validated. The whole milestone's risk sits
in one place: the Python bindings' exhaustive `SecretType` match. Verify that diff is
empty early, not at the end — if a re-export cannot preserve it, the shape of the
extraction needs rethinking rather than patching.

Do not "improve" the pattern corpus. The 25 patterns and the 3.5 entropy gate are the
contract; a richer corpus is a capability change belonging in its own plan.

Preserve unrelated user changes. The two untracked plan files
(`plans/c2-real-simulation-differentiation-plan.md`, `plans/post-exploit-plan.md`) in the
working tree belong to the user — do not modify or delete them.