# Security Knowledge Corpus Milestone 003 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/security-knowledge-corpus/003-secret-detection-extraction.md`

Source subsystem roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-3--secret-detection-extraction`

Repository baseline reviewed: `93a24439` (branch `docs/knowledge-corpus-crate-plans`; M002 closure `93a24439`)

Implementation commits or pull requests:

- this branch — extract `eggsec-secrets`, add guard 149, register `make check` line

## 1. Executive finding

The extraction is complete, and the milestone's defining constraint held: **the Python
bindings, `eggsec-mobile-lab`, `eggsec-tui`, and `crates/eggsec/tests/` have zero
diffs**.

`crates/eggsec/src/recon/secrets.rs` now lives at `crates/eggsec-secrets/src/lib.rs`
with **exactly one** changed line in its entire body: the `Severity` import. Everything
else — the 25 patterns, the 30 `SecretType` variants, the 3 `Confidence` tiers, and the
entropy gate — is byte-identical, and the 11-test count is unchanged.

That single-line result is what distinguishes this from a consumer-breaking extraction.
`eggsec-python/src/git_secrets.rs` matches all 30 `SecretType` variants exhaustively; had
`SecretType` been *re-defined* in the new crate instead of re-exported, that match and
three other consumers would have broken at once.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| M001's disposition confirmed and applied | M001 chose Option 3 (delete); `serde_json` **not** required | pass | §3 |
| Crate has `eggsec-core` as only workspace dep | `cargo tree` — one workspace edge | pass | §3 |
| `cargo test -p eggsec-secrets` passes 11 tests | `11 passed; 0 failed` | pass | baseline preserved |
| Module moved with only the `Severity` import changed | full body diff: 1 line | pass | §3 |
| Entropy gate byte-identical | `secret_entropy(value) < 3.5` on `AwsSecretKey`, verbatim | pass | §3 |
| `eggsec::recon::secrets::*` resolves; **zero** consumer diffs | `git diff --stat` empty | pass | §3 |
| New leaf guard exists and fails on a forbidden dep | check 148 (already covering the crate) | pass | §4 |
| Owner/facade/entropy guard exists and fails | check 149, 3 negative tests | pass | §4 |
| `make check` green | see §4 | pass | |
| Docs updated including `python_api.md` | this commit | pass | §9 |
| Pattern/variant/threshold content unchanged | single-line diff proves it | pass | |

## 3. Production implementation evidence

### M001's disposition, as applied

M001 closed with Option 3 — `utils/redaction.rs` deleted. Therefore `redaction.rs` does
**not** join this crate, and `eggsec-secrets` needs no `serde_json`. The milestone's scope
was `recon/secrets.rs` alone. Test count is 11, not 11 + 26.

### The move was mechanical

Full body diff, doc header excluded:

```
$ git show HEAD:crates/eggsec/src/recon/secrets.rs | grep -v '^//!' | grep -v '^$' > /tmp/so.txt
$ grep -v '^//!' crates/eggsec-secrets/src/lib.rs | grep -v '^$' > /tmp/sn.txt
$ diff /tmp/so.txt /tmp/sn.txt
52c52
< pub use crate::types::Severity;
---
> pub use eggsec_core::types::Severity;
```

One line. That is the entire source change, exactly as the spike predicted.

### The entropy gate is untouched

```
original (line 334):  if pattern.secret_type == SecretType::AwsSecretKey && secret_entropy(value) < 3.5 {
extracted (line 362): if pattern.secret_type == SecretType::AwsSecretKey && secret_entropy(value) < 3.5 {
```

The gate remains scoped to `SecretType::AwsSecretKey` only, at threshold `3.5`. It was
not widened, retuned, or moved. The test `low_entropy_aws_secret_candidate_is_ignored`
still passes, which is the behavioral regression check on this constant.

### Dependency closure

```
$ cargo tree -p eggsec-secrets | grep eggsec
eggsec-secrets v0.1.0 (crates/eggsec-secrets)
├── eggsec-core v0.1.0 (crates/eggsec-core)
```

`eggsec-core` is the only workspace edge. It exists solely because `Severity` lives
there — the same single-import seam that made `eggsec-policy` extractable in Phase C.

### Source compatibility — the milestone's real risk

The engine facade is:

```rust
pub use eggsec_secrets as secrets;
```

Proof that nothing else needed editing:

```
$ git diff --stat -- crates/eggsec-python/ crates/eggsec-mobile-lab/ \
    crates/eggsec-tui/ crates/eggsec/tests/
(empty)
```

Even `recon/git_secrets.rs` needed no edit: it already imported via
`crate::recon::secrets::{SecretFinding, SecretScanner}`, which the facade preserves. The
plan anticipated an import update here; the facade made it unnecessary, which is the
better outcome.

### `git_secrets.rs` correctly stayed behind

`recon/git_secrets.rs` (473 lines) remains engine-side and now imports from the corpus
crate. It is `std::process::Command::new("git")` orchestration plus filesystem walking —
subprocess orchestration is not pattern matching, and it is feature-gated behind
`git-secrets`. Extraction is deferred to a future ADR, per plan §5.

### Content integrity

| Item | Baseline | After | Method |
|---|---|---|---|
| `SecretPattern` entries in `build_patterns()` | 25 | 25 | AST count |
| distinct `SecretType` covered | 20 | 20 | AST count |
| `SecretType` variants | 30 | 30 | AST count |
| `Confidence` tiers | 3 | 3 | AST count |
| `#[test]` count | 11 | 11 | run |

## 4. Verification executed

### Commands run

```bash
cargo test -p eggsec-secrets --tests
cargo tree -p eggsec-secrets
cargo check --workspace --no-default-features
bash scripts/check-architecture-guards.sh
bash scripts/check-architecture-guards.sh   # negative tests
make check
```

### Results

| Command | Result |
|---|---|
| `cargo test -p eggsec-secrets --tests` | pass — 11 passed, 0 failed |
| `cargo tree -p eggsec-secrets` | pass — `eggsec-core` the only workspace edge |
| `cargo check --workspace --no-default-features` | pass — `Finished … in 26.56s` |
| Consumer diff check | pass — empty across bindings, mobile-lab, TUI, integration tests |
| `bash scripts/check-architecture-guards.sh` | pass — all 149 checks, `ALL PASSED` |
| Guard 148 on this crate | pass — `eggsec-secrets` already in its loop, no guard edit needed |
| Guard 149 negative tests | **pass** — 3 cases, see below |
| `make check` | pass — see §4.1 |

#### 4.1 Guard 149 negative tests

Guard 149 pins three things: canonical owner, permanent facade, and frozen entropy gate.
All three failure modes were injected:

```
# A. retune the threshold 3.5 -> 4.5
FAIL: entropy gate threshold is not 3.5: crates/eggsec-secrets/src/lib.rs:362:
      if pattern.secret_type == SecretType::AwsSecretKey && secret_entropy(value) < 4.5 {

# B. widen the gate off AwsSecretKey
FAIL: entropy gate is no longer scoped to SecretType::AwsSecretKey (must not be widened).

# C. drop the engine facade
FAIL: recon/mod.rs no longer re-exports eggsec-secrets as secrets (facade path broken).
```

Each returned script exit 1. The tree was restored and the full suite re-run clean.

#### 4.2 `make check`

Green, exit code 0, with `cargo test -p eggsec-secrets --tests` registered after
`eggsec-service-db`.

Deep checks (`make check-full`, `make check-features-individual`, `make clippy-domain`,
`make test-tui-pty`) were **not run** — no feature surface changed.
`make check-deps` ran inside `make check` and passed. `make check-python` was **not run**:
no Python binding, stub, or doc changed, and the bindings' diff is empty by proof.

## 5. Invariant review

| Source-plan invariant | Evidence |
|---|---|
| `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate | No dispatch or enforcement code touched. The diff is a file move, one `pub use`, one manifest line, and guards. |
| `eggsec-secrets` authorizes nothing, resolves nothing, opens no socket, spawns no process | No such code moved; `cargo tree` shows no network/runtime/subprocess crate; check 148 enforces no `Scope`/`ApprovedOperation`/`Capability`/`EnforcementContext`. |
| Only workspace dependency is `eggsec-core` | `cargo tree`; check 148. |
| Entropy gate byte-identical, still scoped to AWS | Single-line body diff; check 149 enforced on 3 injected mutations. |
| 25-pattern corpus is content, not a refactor target | Unchanged; AST-verified counts match baseline. |
| Workspace path graph stays acyclic (check 108) | Passes in the 149-check run; new edge is `eggsec → eggsec-secrets`, one-way. |
| `eggsec::recon::secrets::*` remains valid | Empty consumer diff + green workspace check. |
| `SecretType`/`Confidence` re-exported, never re-typed | The exhaustive 30-arm match in `eggsec-python` compiles with zero edits — this is the proof, not an assumption. |

## 6. Failure and recovery review

Not applicable. Pure synchronous matching over a `LazyLock` pattern set. No tokio task is
introduced, so no timeout wrapper applies. No cancellation, restart, or persistence
behavior exists or changed.

The one behavioral property protected is the entropy gate, verified above. A second
property — laziness — is preserved: patterns compile behind `LazyLock`, and no eager
static was introduced.

## 7. Migration and compatibility review

No storage, protocol, configuration, or wire migration.

**Type identity is the compatibility surface here.** `SecretType` appears in Python
binding signatures, so what had to remain stable was not just the path but the *type
identity*. Preserving it required a re-export, not a move-and-alias. Both bindings and
`eggsec-mobile-lab` compile with zero diffs, which is the specific regression that would
catch a re-type.

The crate is `publish = false` and internal, so no external consumer can be affected.
Rollback is a revert; `Cargo.lock` gains one internal package entry.

## 8. Security review

Stated as properties, not claimed as new capability:

- **Detection semantics unchanged.** The 25 patterns, 30 variants, 3 confidence tiers, and the `3.5` AWS-scoped entropy gate are byte-identical. Nothing about what Eggsec detects moved.
- **Reduced reach.** The detector previously lived in a crate that depends on `reqwest`, `tokio`, subprocess, and filesystem infrastructure; it now cannot reach any of them. A corpus that compiles without a network stack cannot leak scope, bypass a transport checkpoint, or exfiltrate through an outbound call. This is a direct consequence of the leaf boundary, enforced by check 148.
- **No engine secret reach.** Holding no engine dependency means the detector cannot read engine credential material.
- **Boundary clarity improved.** Detection (this crate) and masking (the report model's declarative `RedactionState`, plus `value_preview`'s 20-char truncation) are now in separate places and separately documented. M001 removed the third, orphaned implementation that made the previous picture ambiguous.
- **Authorization:** untouched. No scope, capability, or approval surface exists or was added.

## 9. Documentation and operations

Updated:

- `architecture/recon.md` — module-table row and the secret-detection section rewritten for the new owner, including the `git_secrets.rs` boundary and the detection-vs-masking split; stale `secrets.rs:NNN` line citations replaced with durable statements.
- `architecture/overview.md` — crate-table row, dependency-map node, guard bullet for `eggsec-secrets`.
- `architecture/python_api.md` — new invariant 11 stating that corpus types are re-exported, never re-typed, naming the two exhaustive matches that make this load-bearing.
- `architecture/capability_segregation.md` — Phase G status for the implemented crate; Guards bullet; footer verification note.
- `crates/eggsec-secrets/src/lib.rs` — doc header stating scope, the leaf boundary, and why the entropy gate is not a tuning knob.
- `crates/eggsec/src/recon/mod.rs` — facade marked permanent.
- `.opencode/skills/eggsec-recon/SKILL.md` — removed `secrets.rs` from the recon file inventory and noted the new owner plus the `git_secrets.rs` boundary.

Static guards:

- **Check 148** — already covered `eggsec-secrets` in its loop; policed the new manifest and source with no edit to the guard.
- **Check 149** — new; canonical owner, engine facade, and frozen entropy gate.

### Factual correction applied during this milestone

The plans and the Phase G record originally stated **26** secret patterns. Verified
against source: `build_patterns()` contains **25** `SecretPattern` entries covering **20**
of the 30 `SecretType` variants. The repository's own `architecture/recon.md` was already
correct at 25. The erroneous figure was corrected in this milestone's plan, in the
roadmap, and in `capability_segregation.md`, and the correction is recorded in that
file's footer so the change is auditable.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | `SecretScanner::scan_file` has zero consumers, and is the crate's only `std::fs` touch. | Dead public helper. Unlike the engine's internal `utils::redaction` (deleted in M001), `scan_file` is a legitimate *library* entry point for a corpus crate intended for third-party use, so it was kept rather than treated as dead weight. It is not counted by guard 148, which polices manifest dependencies, not `std` usage. | None now. If M005 defers publication, revisit whether a filesystem-touching helper belongs in a corpus advertised as I/O-free. |
| low | `recon/git_secrets.rs` (473 lines) is now an engine-side consumer of the corpus crate. | Intentional per plan §5: subprocess orchestration is a different concern from pattern matching. | A future ADR may extract it as a sibling of `eggsec-secrets`. |
| low | Ten of the 30 `SecretType` variants have no dedicated pattern in `build_patterns()`. | Pre-existing coverage gap, documented in `architecture/recon.md` since before this milestone. | Out of scope — adding patterns is a capability change belonging in its own plan. |

No critical, high, or medium findings.

## 11. Roadmap disposition

**Milestone closed and next dependency may proceed.**

No stop condition was triggered. M001's closure record was present and unambiguous
(Option 3); the exhaustive `SecretType` match compiles with zero diffs, so the
"re-shape the extraction" stop condition did not apply; no pattern, threshold, or variant
change was required; no authorization or network need appeared; and `git_secrets.rs`
and the rest of `recon` were left alone.

M004 is dependency-ready and unaffected. M005 remains blocked on M002+M003+M004 plus a
maintainer decision that is not a code question.

## 12. Registry updates

- `plans/subsystems/security-knowledge-corpus-roadmap.md` — milestone 3 status → closed; §7 milestone 3 outcome; pattern count corrected to 25.
- `plans/registry.md` — milestone 3 status → closed; `eggsec-secrets` recorded with guards 148/149.
- `plans/implementation/security-knowledge-corpus/003-secret-detection-extraction.md` — status → closed with the closure reference; pattern count corrected to 25.