# ADR-0006: Publication of the knowledge-corpus crates

Status: accepted

Date: 2026-10-05

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/000-long-term-specification.md#3-non-goals`
- `plans/002-long-term-roadmap.md#phase-1--crate-boundaries-and-reusable-library-ownership`

Affected subsystem roadmaps:

- `plans/subsystems/security-knowledge-corpus-roadmap.md`

Related decisions:

- `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md` (decision 3 deferred publication)

Implements:

- `plans/implementation/security-knowledge-corpus/005-publication-qualification.md`
- closure: `plans/closure/security-knowledge-corpus/005-closure.md`

## Context

ADR-0005 decision 3 separated extraction from publication: extract the corpus crates
now as internal leaves, and treat publication as its own decision. Milestones 001–004
have now closed, so all three corpus crates exist, are internal, and are verified:

| Crate | Content | Tests | Workspace deps |
|---|---|---|---|
| `eggsec-service-db` | port→service tables, banner heuristics | 21 | none |
| `eggsec-secrets` | 25 credential patterns, entropy gate | 11 | `eggsec-core` |
| `eggsec-payloads` | 40 payload modules, `PayloadType` (40 variants) | 252 | `eggsec-core` |

This ADR records whether they should be published as general-purpose crates for
third-party consumption. The deliverable is a decision with evidence, not a release.

## Decision

**Defer publication.** All three crates remain `publish = false`.

This is not a rejection of the reuse thesis. It is a statement that the corpora are not
yet a stable public contract, and that the one hard requirement for even considering
publication — a real external consumer — does not currently exist.

Nothing was published, renamed, or re-versioned. No code changed.

## Rationale

### 1. The corpora are demonstrably still moving

`git log --since=2026-06-01` over the corpus paths returns **14 commits**. One of them is
`e05711ab Expand payload repository: +550 payloads across 10 new modules and 20 enhanced
modules`.

A corpus that gained 550 payloads and 10 modules in the current roadmap era is a
work-in-progress knowledge base. Publishing it would convert every subsequent corpus
improvement into a semver question, on the exact types consumers depend on:

- adding a `PayloadType` variant (40 today) breaks the Python bindings' exhaustive
  `parse_payload_type`;
- adding a `SecretType` variant (30 today) breaks `eggsec-python`'s exhaustive match;
- changing the corpus content (payload text, severity, tags) is a semver-visible change
  to every consumer's output, with no way to signal "bugfix" versus "capability".

The last point is the real cost. Published payload text is behavior, not an
implementation detail.

### 2. Release qualification is expensive in this repo, and it is already known

`scripts/release-package-graph.py` classifies a package as `private-workspace` solely
by `publish = false` (line 55). Flipping that field enrolls a crate in the crates.io
graph immediately, which brings:

- an archive inventory check and a standalone `cargo metadata --offline` parse;
- a rule that a published package's dependencies must not be private, and must all
  carry the **same release version** (`eggsec-secrets` and `eggsec-payloads` depend on
  `eggsec-core`, so publishing them means publishing `eggsec-core` on the same version
  line);
- an independent MSRV (1.89) and ring-only TLS policy as consumer-visible constraints.

The precedent for the cost is `nse-runtime-extraction` M007C. Its `cargo semver-checks`
gate found a **74-function major break** in a crate that had already shipped 0.2.0;
`plans/closure/nse-runtime-extraction/007c-closure.md` records that `0.2.1` was not
published and a 0.3.0 migration plan was required instead. That subsystem spent multiple
milestones on publication. The lesson generalizes: publishing early means discovering
breaks late, and the discovery is expensive because the corpus types are precisely the
ones the rest of the workspace matches exhaustively.

### 3. No external consumer has been identified

The plan's own stop condition is explicit: "no concrete external consumer has been
identified" means stop and report. A hypothetical consumer is not evidence.

No third-party project currently depends on these corpora. The competitive context is
real but unvalidated — the payload corpus sits against SecLists/ffuf defaults, the secret
detector against gitleaks/trufflehog pattern sets, the service table against
nmap-services. None of that has been tested against a real integration.

### 4. `eggsec-payloads` has a public API that would be wrong for a library

> **Superseded 2026-10-06 — this rationale rested on a false premise, and reopening
> condition 4 is now satisfied.** The claim below that the 6 advanced types are "generated
> by live `reqwest` probing" is wrong. Their probers do probe; their `get_payloads()`
> functions are pure static data. The payload sets have since moved into
> `eggsec-payloads`, which resolves all 40 variants and no longer panics. The remaining
> opening (conditions 1–3: a named external consumer, a stable release cycle, a corpus
> update policy) is untouched, so the ADR's **defer** decision still stands.

This emerged from M004 and is specific enough to be decisive.

The corpus crate owns all 40 `PayloadType` arms, but the 6 advanced types are generated
by live `reqwest` probing and stay in the engine. `eggsec_payloads::get_payloads` routes
those 6 to a documented `unreachable!` rather than returning an empty vector.

For an **internal** crate behind an engine facade, that is correct: a panic at a
programming-error boundary is better than a silent wrong answer, and the engine's
`get_payloads` always resolves all 40. For a **published general-purpose library**, it is
a poor contract — a third party calling `get_payloads(GraphQL)` on a crate whose own
`PayloadType` enum offers `GraphQL` gets a panic. Publishing it as-is would mean
shipping a public API that panics for 6 of its own 40 variants.

Making that publishable is real design work, not a flag flip: either splitting the enum,
or exposing a non-panicking variant-aware API. It is out of scope here by design.

### 5. Naming is decided before publishing, not after

The `eggsec-` prefix is right for internal workspace crates and awkward for
general-purpose libraries. Renaming after publication is far worse than choosing badly
now — it breaks every downstream import at once. Deferring keeps that choice open.

## Consequences

- The three corpus crates stay internal. Their engine facades remain permanent, so this
  decision costs nothing to reverse.
- The extraction work is not wasted: the corpora are now isolated, independently
  testable, and independently releasable *when* the conditions below are met.
- Publication is **not** blocked indefinitely. It is blocked on evidence.
- The maintenance burden of a public corpus (update cadence, owner, MSRV, semver) is
  not yet accepted by anyone, and should not be accepted implicitly by publishing.

## Reopening condition

This decision should be revisited when **all** of the following hold:

1. **A named external consumer exists** — a specific project or persona that would
   depend on a corpus crate, with a stated need the crate does not currently meet.
   This is the gate that is currently unmet.
2. **The corpora have been stable for one release cycle** — no `PayloadType` or
   `SecretType` variant additions, and no corpus-content changes, within a published
   version window.
3. **A corpus update policy exists** — a stated cadence and an owner, since a published
   corpus otherwise drifts from upstream vulnerability research.
4. ~~**`eggsec-payloads`'s API is library-appropriate** — the 6 advanced types are
   handled without a panic on the public path.~~ **MET 2026-10-06:** the 6 static payload
   sets moved into the corpus crate; `get_payloads` resolves all 40 variants and no public
   path panics (see Rationale §4).

### Per-crate readiness

The crates are **not** equally ready, and this is worth recording:

| Crate | Readiness | Notes |
|---|---|---|
| `eggsec-service-db` | **Closest.** Zero workspace deps, 9 public items, no engine types, no panic paths. | Would be the first to publish, and needs the least design work. Still needs a consumer (condition 1) and a version/namespace decision. |
| `eggsec-secrets` | Plausible. One `eggsec-core` dep for `Severity`; would require publishing `eggsec-core` on the same version line. | The 10 `SecretType` variants without patterns are a coverage gap a consumer would notice. |
| `eggsec-payloads` | **Was least ready**; the panic contract (§4) is resolved as of 2026-10-06. Still the largest surface (41 public modules, 252 tests), so it needs the most qualification. | Largest surface of the three, but no longer blocked on API design. |

### If publication is later approved

It gets its own subsystem roadmap and release plan. It must not be started inside this
subsystem, and it must not inherit publication as a side effect of further extraction
work. ADR-0005 decision 2 also holds: the engine re-export facades are permanent and are
**not** transitional shims to be removed on publication.

## Verification

```bash
cargo tree -p eggsec-payloads -p eggsec-secrets -p eggsec-service-db
make check
```

Run and green at the time of this ADR (see `plans/closure/security-knowledge-corpus/005-closure.md`).
No code, manifest, or public path changed in this milestone.

## Alternatives considered

- **Publish now.** Rejected: §1–§4. Would convert active corpus development into a semver
  obligation, ship a panicking public API, and require publishing `eggsec-core` on a
  shared version line — before any consumer exists.
- **Reject permanently.** Rejected: nothing in the analysis argues the corpora are
  unsuitable for reuse. The obstacle is maturity and evidence, not fitness.
- **Publish `eggsec-service-db` alone as a probe.** Tempting, since it is the only crate
  with no workspace dependency and no panic paths. Rejected for now because condition 1
  is unmet — publishing the one crate that is technically ready would create a public
  maintenance obligation with no user to justify it. Revisit first if a consumer appears.