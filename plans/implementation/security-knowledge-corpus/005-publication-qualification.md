# Security Knowledge Corpus Milestone 005 — Publication qualification

Status: blocked (operational, and on a maintainer decision)

Repository baseline: `fix/cli-usability-audit` at `979dca67` (pushed); this plan lands on a branch cut from it

Source roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-5--publication-qualification`

Long-term requirements:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#3-non-goals`
- `plans/002-long-term-roadmap.md#phase-1--crate-boundaries-and-reusable-library-ownership`

Applicable ADRs:

- `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md` (decision 3)

Primary class: infrastructure

Blocked by: M002, M003, M004 (operational), **and** an explicit maintainer decision that
is not a code question.

## 1. Objective

Decide whether `eggsec-payloads`, `eggsec-secrets`, and `eggsec-service-db` should be
published as general-purpose crates for third-party consumption, and record that decision.

The deliverable of this milestone is a **decision with evidence**, not a publication.

## 2. Why this milestone is ready

It is not yet ready, and saying so is the point.

- **Operational dependency:** all three extractions must be closed so the published
  surface is known and stable.
- **Decision dependency:** ADR-0005 decision 3 defers publication deliberately. Staging
  it as a separate milestone is what keeps the extractions from inheriting an unrepayable
  public contract.

This plan is written now so the option is visible and costed, not so it can be executed
immediately.

## 3. Current implementation evidence

ADR-0005 Option C (extract and publish immediately) was rejected on sequencing, with the
rejection recorded. The evidence supporting that rejection is concrete:

**The corpora are still evolving.** The `PayloadType` enum gained `GraphQL`, `OAuth`,
`Jwt`, `Idor`, `Ssti`, and `Grpc` within the current roadmap era, and `is_advanced()`
treats exactly those six as a distinct class. `recon/secrets.rs` carries 30
`SecretType` variants, and `eggsec-python/src/git_secrets.rs` matches all of them
exhaustively — so adding or removing a variant is a cross-crate breaking change today,
which is precisely the coupling that would become a semver obligation the moment the type
is published.

**Release qualification is demonstrably expensive here.** The
`nse-runtime-extraction` subsystem has spent milestones on publication and is currently
**blocked**: M007C ran the `cargo semver-checks` gate from its own plan and found a
74-function major break (70 `register_<mod>_library(&Lua)` plus 4 `helpers::*`), none
source-compatible at the original path. That plan's own stop condition applied and
`0.2.1` was not published. The lesson generalizes: publishing early means discovering
breaks late.

**Reuse value is real but unproven.** The payload corpus competes with
SecLists/ffuf-style defaults; the secret detector competes with gitleaks/trufflehog
pattern sets; the service table competes with nmap-services. None of this has been
validated against an actual external consumer.

## 4. Invariants that must not regress

- ADR-0005 decision 3: publication MUST NOT be bundled into an extraction milestone.
- ADR-0005 decision 2: the engine re-export facade is permanent and is not a
  transitional shim to be removed on publication.
- No corpus crate gains an engine dependency in order to publish. If publication requires
  engine types, it is out of scope.
- `deny.toml` is canonical; any new dependency exception is recorded in `deny.toml` **and**
  `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a review-by date.
- Release validation is owned by `scripts/release-check.sh` and
  `scripts/release-package-graph.py`; this milestone MUST NOT invent a parallel release
  path.
- Neither this repository's CI nor any hosted CI publishes a package. Publication is a
  deliberate manual act with a qualified artifact.

## 5. Scope

### In scope

- Gather the evidence needed for a maintainer decision.
- Present the options with costs, using the `nse-runtime-extraction` experience as the
  worked precedent.
- Record the decision in an ADR, and update `architecture/capability_segregation.md`
  Phase G.

### Explicitly out of scope

- Any publication, tagging, or release. If approved, publication requires its own
  roadmap and release plan.
- Creating a neutral crate name or renaming an existing crate.
- Changing any corpus content.
- Deciding the question on the maintainer's behalf.

## 6. Required production changes

None in this milestone. This is a decision-and-record milestone. If it concludes
"defer", the only edits are to `architecture/capability_segregation.md` Phase G, the
roadmap milestone table, and `plans/registry.md`.

If it concludes "publish", **stop** and write a new subsystem roadmap. Do not begin
publication work inside this milestone.

## 7. Ordered work packages

### Work package A — Establish the surface

Intent: know exactly what would be published.

Required changes: none.

Evidence to gather:

- For each crate, its public item list and whether any public item leaks an engine type.
- Whether the corpus content is still changing — check recent commits touching the
  corpus paths for pattern/variant additions.
- The engine re-export surface that must keep working regardless.

### Work package B — Cost the release qualification

Intent: price the work honestly, using precedent rather than optimism.

Evidence to gather:

- How `nse-runtime-extraction` M007C went wrong, and what the semver gate cost. Its
  closure record `plans/closure/nse-runtime-extraction/007c-closure.md` is the reference.
- What `scripts/release-package-graph.py` requires of a newly published crate, including
  the exact archive inventory check and the standalone `cargo metadata --offline` parse.
- Whether the MSRV 1.89 baseline and the `make check` contract can be honored for a
  standalone crate outside this workspace.

### Work package C — Identify a real consumer

Intent: test the reuse thesis before committing to it.

Evidence to gather: at least one concrete external project or persona that would depend
on these crates, and what it would need that the crate does not currently offer. A
hypothetical consumer is not evidence.

### Work package D — Decide and record

Intent: close the question.

Required changes: a new ADR recording the decision — publish, defer, or reject — with
its evidence and consequences. Update `architecture/capability_segregation.md` Phase G
and the roadmap milestone table.

Acceptance evidence: the ADR exists with status accepted; the roadmap table reflects the
outcome; `plans/registry.md` records the decision.

## 8. Failure, cancellation, restart, and contention semantics

Not applicable. No code changes are in scope.

## 9. Compatibility and migration

None in this milestone.

The following are the compatibility obligations that *would* be created by publishing,
and are the substance of the decision:

- **Independent semantic versioning.** Once published, adding a `PayloadType` variant or a
  `SecretType` variant becomes a minor-version question, not an internal refactor.
  Today both are breaking changes across `eggsec-python`.
- **Corpus update policy.** A published corpus needs a stated cadence and an owner, or it
  will drift from upstream vulnerability research.
- **Naming.** The `eggsec-` prefix is appropriate for internal workspace crates and
  awkward for general-purpose libraries. Renaming after publication is worse than
  choosing badly now, which is an argument for deciding before publishing.
- **MSRV and TLS policy.** A published crate inherits this repo's MSRV 1.89 and ring-only
  TLS baseline as consumer-visible constraints.

## 10. Required tests

None. This milestone changes no code.

Work package A's public-surface inventory is evidence, not a test.

## 11. Required verification commands

```bash
cargo tree -p eggsec-payloads -p eggsec-secrets -p eggsec-service-db
make check                  # confirms the workspace baseline the decision rests on
```

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- New ADR recording the decision.
- `architecture/capability_segregation.md` Phase G — outcome.
- `plans/subsystems/security-knowledge-corpus-roadmap.md` — milestone 5 status.
- `plans/registry.md` — decision record.

## 13. Acceptance criteria

- A decision (publish / defer / reject) is recorded in an accepted ADR with evidence.
- If deferred or rejected, the rationale states what evidence would reopen the question.
- No code, manifest, or public path changed in this milestone.
- `architecture/capability_segregation.md` Phase G reflects the outcome.
- If published was decided, **no publication work was started** — a new roadmap exists
  instead.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- M002, M003, or M004 is not closed;
- no concrete external consumer has been identified;
- the decision would require re-typing corpus types to shed engine dependencies, which
  would break the re-export contract in ADR-0005 decision 2;
- publication would require a crate rename that ripples through `eggsec-python`;
- the evidence suggests the answer is "later", in which case the correct deliverable is a
  documented deferral, not a publication attempt.

## 15. Closure evidence required

`plans/closure/security-knowledge-corpus/005-closure.md` containing:

- the public-surface inventory from work package A;
- the release-qualification cost assessment from work package B, citing the
  `nse-runtime-extraction` 007c closure as precedent;
- the consumer evidence from work package C, or an explicit statement that none exists;
- the decision and the ADR that records it;
- confirmation that no code changed.

## 16. Handoff notes

This milestone exists to make a tempting "yes" require evidence. The reuse case is
genuinely attractive and the extraction makes it cheaper than it was — but "cheaper to
extract" is not "ready to publish", and the NSE subsystem is a live demonstration of how
a publication obligation can consume a subsystem for many milestones.

If the maintainer has not expressed an intent to publish, "defer with a stated reopening
condition" is a complete and legitimate outcome. Write that down and stop.

Preserve unrelated user changes. The two untracked plan files
(`plans/c2-real-simulation-differentiation-plan.md`, `plans/post-exploit-plan.md`) in the
working tree belong to the user — do not modify or delete them.