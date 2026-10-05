# Security Knowledge Corpus Milestone 005 — Closure Status

Status: closed (deferral)

Source implementation plan:

- `plans/implementation/security-knowledge-corpus/005-publication-qualification.md`

Source subsystem roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-5--publication-qualification`

Repository baseline reviewed: `f22de64d` (branch `docs/knowledge-corpus-crate-plans`; M004 closure `f22de64d`)

Implementation commits or pull requests:

- this branch — `plans/adrs/ADR-0006-knowledge-corpus-publication.md` plus roadmap/registry/Phase G updates. **No code, manifest, or public path changed.**

## 1. Executive finding

**Decision: defer publication.** Recorded in `plans/adrs/ADR-0006-knowledge-corpus-publication.md`
(status: accepted). All three corpus crates remain internal `publish = false` leaves.

This is the outcome the plan's §5 and §16 describe as legitimate and complete: the
milestone's job was to make a tempting "yes" require evidence, and the evidence does not
exist yet. The blocking gap is not technical fitness — the crates are clean, leaf, and
independently tested — but that **no external consumer has been identified**, which is
the plan's own first stop condition.

The decision is a deferral with four stated reopening conditions, not a rejection. Nothing
was published, renamed, or re-versioned.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Work package A — public-surface inventory | §3 below | pass | |
| Work package B — release-qualification cost | §4 below, citing NSE M007C | pass | |
| Work package C — identify a real consumer | §5 below: none exists | **stop condition** | recorded, not worked around |
| Decision recorded in an accepted ADR | ADR-0006, status accepted | pass | |
| If deferred, rationale states what reopens the question | ADR "Reopening condition", 4 conditions | pass | |
| No code, manifest, or public path changed | `git status` shows docs only | pass | §3 |
| Phase G reflects the outcome | rewritten "Publication" section | pass | |
| Not decided on the maintainer's behalf | deferral only; no publish/reject | pass | |

## 3. Production implementation evidence

**None.** This is a decision-and-record milestone, as its plan §6 specifies. The only
edits are `ADR-0006`, the Phase G publication section, the roadmap milestone table, and
`plans/registry.md`.

### Work package A — the surface that would be published

| Crate | Top-level public items | Public modules | Workspace deps | Engine type in any public signature? |
|---|---|---|---|---|
| `eggsec-service-db` | 9 | 1 (`lib`) | **none** | no |
| `eggsec-secrets` | 6 | 1 (`lib`) | `eggsec-core` | no |
| `eggsec-payloads` | 80 | 35 | `eggsec-core` | no |

Verified: a repo-wide search for `eggsec::` / `crate::types` / `crate::error` /
`crate::tool` / `crate::config` across all three crates returns exactly **one** hit, and
it is a doc-comment string in `eggsec-payloads/src/lib.rs` naming the engine path for the
panic message. No public signature exposes an engine type. This is the property ADR-0005
decision 2 required, and it holds.

Corpus churn (`git log --since=2026-06-01` over the three original corpus paths): **14
commits**, including `e05711ab Expand payload repository: +550 payloads across 10 new
modules and 20 enhanced modules`.

The engine-side facade surface that must keep working regardless: `eggsec::scanner::service_data`,
`eggsec::recon::secrets`, `eggsec::fuzzer::payloads::{PayloadType, Payload, get_payloads,
get_payloads_cached, get_all_payloads_cached, Severity}`, `eggsec::fuzzer::{PayloadType,
Payload, get_payloads, …}`.

## 4. Verification executed

### Commands run

```bash
cargo tree -p eggsec-payloads -p eggsec-secrets -p eggsec-service-db
cargo test -p eggsec-payloads --tests
cargo test -p eggsec-secrets --tests
cargo test -p eggsec-service-db --tests
bash scripts/check-architecture-guards.sh
make check          # carried from M004; re-confirmed green on this tree
```

### Results

| Command | Result |
|---|---|
| `cargo tree` (all three) | pass — `eggsec-core` is the only workspace edge for payloads/secrets; `eggsec-service-db` has none |
| `cargo test -p eggsec-payloads` | pass — 233 tests |
| `cargo test -p eggsec-secrets` | pass — 11 tests |
| `cargo test -p eggsec-service-db` | pass — 21 tests |
| `bash scripts/check-architecture-guards.sh` | pass — 150 checks, `ALL PASSED` |
| `make check` | pass — exit 0, 4217 passed / 0 failed |

`make check` was run on this tree in M004 and no code changed since, so the recorded
result stands; the guards were re-run here to confirm the tree is still clean.

### Work package B — cost of release qualification

`scripts/release-package-graph.py` classifies a package as `private-workspace` **solely**
by `publish = false` (line 55), so flipping the field enrolls a crate in the crates.io
graph immediately. Publishing would then require:

1. an archive inventory check plus a standalone `cargo metadata --offline` parse;
2. that a published package's dependencies are **not** private-workspace **and** all
   share the same release version (lines 118–125). Since `eggsec-secrets` and
   `eggsec-payloads` depend on `eggsec-core`, publishing them means publishing
   `eggsec-core` on the same version line;
3. accepting MSRV 1.89 and ring-only TLS as consumer-visible constraints.

The cost precedent is `nse-runtime-extraction` M007C: `cargo semver-checks
check-release --baseline-version 0.2.0` reported a `function_missing` major break — **74
public functions absent** — and `0.2.1` was **not published**; a 0.3.0 migration plan was
required instead. That subsystem consumed multiple milestones on publication.

### Work package C — external consumer

**None identified.** No third-party project, tool, or persona has been identified that
would depend on these corpora. Per the plan's §14, "no concrete external consumer has been
identified" is a stop condition, and per §5 work package C, "a hypothetical consumer is
not evidence."

This is the decisive gap, and it is not something further analysis in this repository can
close — it requires an outside party.

## 5. Invariant review

| Source-plan invariant | Evidence |
|---|---|
| Publication MUST NOT be bundled into an extraction milestone | M005 changed no code; all three crates still `publish = false`. |
| Engine re-export facades are permanent, not transitional | No facade removed; all four remain and are covered by checks 114/149/150 and the M004 seam suite. |
| No corpus crate gains an engine dependency in order to publish | `cargo tree`: `eggsec-core` only, unchanged from M002–M004. |
| `deny.toml` is canonical; new exceptions recorded in it and `docs/DEPENDENCY_EXCEPTIONS.md` | No dependency added, so no exception needed. `make check-deps` (inside `make check`) passed. |
| Release validation stays in `scripts/release-check.sh` / `release-package-graph.py` | No parallel release path invented; the existing script was read for evidence only. |
| Neither CI nor any hosted CI publishes a package | Nothing published; no CI change. |

## 6. Failure and recovery review

Not applicable — no code changed, no state, no process.

## 7. Migration and compatibility review

None. No crate was renamed, re-versioned, or un-`publish`ed, so no downstream consumer
can be affected. The engine facades are untouched and remain covered by guards.

### The compatibility obligations publication *would* create

These are the substance of the decision, and are recorded so they are not discovered
later:

- **Independent semver.** Adding a `PayloadType` (40) or `SecretType` (30) variant becomes
  a minor-version question instead of an internal refactor. Today both are breaking
  changes across `eggsec-python`.
- **Corpus content is behavior.** Published payload text, severity, and tags are
  consumer-visible output. There is no way for a consumer to distinguish a corpus
  improvement from a breaking change to results.
- **Corpus update policy.** A published corpus needs a stated cadence and an owner, or it
  drifts from upstream vulnerability research.
- **Naming.** `eggsec-` is right for internal workspace crates and awkward for
  general-purpose libraries. Renaming after publication breaks every downstream import.
- **`eggsec-payloads`' panic contract.** `get_payloads` routes the 6 engine-owned advanced
  types to a documented `unreachable!`. Correct behind an engine facade; a poor contract
  for a published library whose own enum offers those variants. This is API design work,
  not a flag flip.
- **MSRV and TLS.** A published crate exposes MSRV 1.89 and ring-only TLS as consumer
  constraints.

## 8. Security review

No security-relevant change: no code, no dependency, no credential path, no enforcement
surface. The deferral does not weaken anything — the corpora remain isolated behind
permanent engine facades and guarded by checks 148/149/150.

Worth stating explicitly for the record: had `eggsec-payloads` been published as-is, a
third party calling `get_payloads(GraphQL)` would hit a panic. Shipping a panicking public
API on a security corpus is itself a small quality/hygiene concern, independent of semver.

## 9. Documentation and operations

- **New:** `plans/adrs/ADR-0006-knowledge-corpus-publication.md` (accepted) — decision,
  four-part rationale, per-crate readiness table, four reopening conditions, alternatives
  considered.
- `architecture/capability_segregation.md` — Phase G "Publication" section rewritten from
  "explicitly deferred" to the recorded decision with its evidence.
- `plans/subsystems/security-knowledge-corpus-roadmap.md` — milestone 5 closed; roadmap
  status → closed; ADR-0006 added to related ADRs; milestone 5 outcome recorded.
- `plans/registry.md` — subsystem row → **closed**; M005 row → closed (deferral) with the
  decision path; `crate-boundary-ownership` row updated from "proposes" to "delivered".
- `plans/implementation/security-knowledge-corpus/005-publication-qualification.md` — status
  → closed (deferral).

No operator-facing change: no command, flag, output, or diagnostic differs.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| high | `eggsec-payloads::get_payloads` panics for 6 of its own 40 `PayloadType` variants (the engine-owned live-probe types). | Correct for an internal crate behind an engine facade; a poor public contract. No test inside the corpus crate could catch the inverse failure, which is why M004 added the engine-side seam suite and check 150. | Blocks publication of `eggsec-payloads` only. Tracked as reopening condition 4 in ADR-0006. |
| medium | Publication is deferred while no consumer exists. | The corpora may accrue further divergence from upstream vulnerability research with no external pressure to keep them current. | Acceptable while internal; revisit on a named consumer. |
| low | 10 of 30 `SecretType` variants have no dedicated pattern. | A consumer would likely notice the coverage gap. | Out of scope; pre-existing, documented in `architecture/recon.md`. |

No critical findings.

## 11. Roadmap disposition

**Subsystem closed.** All five milestones have accepted closure records:

| Milestone | Disposition |
|---|---|
| 001 redaction disposition | closed (delete; guard 147) |
| 002 `eggsec-service-db` | closed (guards 114/148) |
| 003 `eggsec-secrets` | closed (guards 148/149) |
| 004 `eggsec-payloads` | closed (guards 148/150 + seam suite) |
| 005 publication | closed — **deferred** (ADR-0006) |

The roadmap's completion definition is satisfied: four extraction/disposition milestones
closed, all three crates satisfy the leaf invariant, engine facades keep
`eggsec`/`eggsec-python`/`eggsec-tui` green with no consumer import edits, `make check` is
green with per-crate test lines registered, Phase G carries the decision record, and the
publication decision is recorded.

**Publication is not on the roadmap.** If reopened, it requires a named consumer and gets
its own subsystem roadmap and release plan.

## 12. Registry updates

- `plans/registry.md` — `security-knowledge-corpus` row → **closed** with all five
  dispositions and both ADRs; M005 row → closed (deferral); `crate-boundary-ownership`
  row updated to reflect that Phase G delivered rather than proposes.
- `plans/subsystems/security-knowledge-corpus-roadmap.md` — status → closed; milestone 5
  closed; ADR-0006 linked; milestone 5 outcome recorded.
- `architecture/capability_segregation.md` — Phase G publication decision recorded.