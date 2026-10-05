# ADR-0005: Knowledge-corpus crate ownership and publication staging

Status: proposed

Date: 2026-10-05

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md#phase-1--crate-boundaries-and-reusable-library-ownership`
- `plans/000-long-term-specification.md#5-crate-ownership`

Affected subsystem roadmaps:

- `plans/subsystems/security-knowledge-corpus-roadmap.md`

Supersedes: nothing. Extends `architecture/capability_segregation.md`, whose Phase A–F
decisions remain in force; this ADR governs only the corpus-shaped subset.

## Context

The engine crate `eggsec` is the workspace's maintainability hotspot: roughly 195,000
lines against roughly 13,000 lines for the next-largest non-binding crate, across 41
top-level modules. The obvious response is to split large modules into per-feature
crates.

Static coupling analysis contradicts that instinct. Measured across every engine
module, almost nothing crosses the 13-distinct-`crate::`-path threshold except
`pipeline` (22 paths) and `tool` (658 references). The large modules — `fuzzer`,
`recon`, `scanner`, `waf`, `distributed`, `c2` — are **executor bodies**: they open
sockets, spawn processes, install TLS providers, and are wired into dispatch. They
are coupled because their job is to be coupled. `Phase C` already extracted the one
genuinely separable semantic cluster (`eggsec-policy`); `Phase D` rejected
`eggsec-loadtest` and `eggsec-resilience`; `WS4` rejected `eggsec-net`,
`eggsec-web-client`, `eggsec-evidence`, and `eggsec-signing`. Those rejections remain
correct.

What the analysis did surface is a category the existing decisions never addressed:
**data-shaped modules whose content is a body of domain knowledge, not logic.** These
have a different extraction story from every prior candidate:

| Module | Lines | `crate::` coupling | External deps | Tests |
|---|---|---|---|---|
| `fuzzer/payloads/` (pure-data subset) | 7,084 / 36 files | effectively 0 | `serde`, `strum`, `flate2`, `rustc-hash`, `tracing` | 233 |
| `recon/secrets.rs` | 492 | 1 (`Severity`) | `regex`, `serde`, `tracing` | 11 |
| `scanner/service_data.rs` | 302 | 0 | `rustc-hash` | 21 |

All three were verified by extraction spike, not by static reasoning alone: each was
copied into a scratch crate depending only on `eggsec-core` (a zero-internal-dependency
leaf that already owns `Severity`) plus its own third-party deps, and compiled with its
full test suite passing unmodified. The payload corpus required exactly four mechanical
edits — two `crate::types::Severity` → `eggsec_core::Severity` import rewrites, one
`crate::fuzzer::payloads` → `crate::payloads` path rewrite, the
`$crate::fuzzer::payloads::Payload` macro path, and six `get_payloads` match arms
redirected because those live-probe modules stay behind. No behavioral change.

The same analysis surfaced a defect. `utils/redaction.rs` — 366 lines, 26 tests —
has **zero production consumers**. Repo-wide, every `redact_sensitive` / `redact_json`
match is either an unrelated local function (`eggsec-python/src/checkpoint_store.rs`
defines its own) or a string literal. Meanwhile `eggsec-transport` independently
maintains two implementations of the same concept (`redacted_headers_debug`,
`redact_url_for_debug`). This is the same zero-consumer condition that `Phase D`
removed `utils::cache::ApiCache` for and that guard check 126 now guards.

The two redaction surfaces are related but not identical, and the plan must not
conflate them. `utils/redaction.rs` is evidence-grade masking — bearer tokens, basic
auth, API keys, AWS keys, JWTs, cookies, private keys, plus a JSON tree walk.
`eggsec-transport` deliberately keeps a narrower debug-formatting surface
(`redacted_headers_debug`, `redact_url_for_debug`) because it must stay dependency-light
and operate on `http` header maps and `Url` values. Whether they should be unified is
a finding for M001, not an assumption.

Two questions must be settled before implementation: what these crates are called and
whether they are published.

## Decision drivers

- Preserve every existing decision in `architecture/capability_segregation.md`; this
  ADR must not be usable as a backdoor to reopen WS4 or Phase D rejections.
- Do not create a second composition root. A corpus crate that needs engine types has
  failed the extraction.
- Keep the leaf invariant honest: a corpus crate must be compilable and testable
  without the engine closure.
- Do not create public compatibility obligations that the project cannot honor. The
  `nse-runtime-extraction` subsystem has spent milestones on release qualification and
  is currently **blocked** on a public-API gate that found a 74-function major break.
- Reproduce the `Phase C` success shape: the extracted crate's tests must run in
  isolation, with a materially narrower dependency graph than the engine.
- Resolve the redaction defect on its merits; do not use it as a vehicle for
  extraction.

## Considered options

### Option A — Leave everything in the engine

Description: record the analysis, change nothing.

Benefits: zero risk, zero churn, no new guards or manifests.

Costs: leaves 7,878 lines of near-zero-coupling corpus dragging the full engine
dependency closure; leaves a known 366-line dead-code defect in place; leaves the
`Phase C` precedent applied inconsistently (policy was extracted; its equally-leaf
corpus siblings were not). **Rejected** — the coupling measurements are objective and
reproducible, and two of three targets need no behavioral reasoning at all.

### Option B — Extract as internal workspace crates only

Description: create `eggsec-service-db`, `eggsec-secrets`, `eggsec-payloads` as
workspace members with engine re-export facades, but do not publish. Keep them
`publish = false` and `version` coupled to the engine.

Benefits: delivers the compile-graph and ownership win; creates no public compatibility
obligation; matches the `eggsec-agent` absorption pattern (extracted, then re-exported
through `eggsec::tool::agents`) exactly.

Costs: third-party reuse value is unrealized and must be deferred explicitly.

### Option C — Extract and publish immediately as general-purpose libraries

Description: extract and publish in one pass with neutral names and independent
semantic versioning.

Benefits: realizes the reuse value immediately.

Costs: creates durable public compatibility contracts on corpora that are still
changing — the payload enum gained `GraphQL`, `OAuth`, `Jwt`, `Idor`, `Ssti`, and
`Grpc` within the current roadmap era, and `recon/secrets.rs` carries 29
`SecretType` variants with an exhaustive `match` in `eggsec-python`. Independent
semantic versioning on an actively-evolving corpus is a permanent maintenance
liability, and the project has direct evidence that release qualification is
expensive: `nse-runtime-extraction` M007C is blocked precisely because a
`cargo semver-checks` gate found an unplanned 74-function major break. **Rejected
as sequencing** — the extraction itself is correct; publishing it in the same pass is not.

### Option D — Split the 6 live-probe payload modules out as well

Description: extract all 41 payload modules together, including the 6 that take
`&reqwest::Client` and perform live probing.

Benefits: single crate, no residual engine-side payload code.

Costs: those 6 modules (`graphql`, `grpc`, `idor`, `jwt`, `oauth`, `ssti`, 4,354
lines) mix payload *generation* with live *execution*. Extracting them would either
drag `reqwest` and Tokio into a corpus crate or split each module in half. **Rejected**
— the corpus/probe split is already present in the source and should be honored.

## Decision

**Option B, staged toward Option C, with Option D's split honored.**

1. **Ownership.** Knowledge corpora — domain knowledge expressed as data or
   transformation over data, with no authorization, transport, dispatch, or rendering
   responsibility — are owned by leaf crates whose only permitted workspace dependency
   is `eggsec-core`. Any module needing engine, transport, frontend, or protocol types
   is not a corpus and is not covered by this ADR.

2. **Engine compatibility is a re-export, never a re-implementation.** The engine
   re-exports corpus types at their existing paths. `eggsec::fuzzer::PayloadType` and
   `eggsec::recon::secrets::SecretFinding` MUST remain valid paths, so the Python
   bindings' exhaustive `match` over 30 `SecretType` variants and the TUI fuzz tab
   continue to compile without edits. This is the `eggsec-agent` facade pattern.

3. **Publication is a separate, later milestone.** The three crates land as
   `publish = false`. Publication requires a neutral crate name, an independent version
   line, a documented corpus-update policy, and its own release plan. It is not a
   consequence of extraction and MUST NOT be bundled into an extraction milestone.

4. **The redaction defect is dispositioned on its own.** It is resolved before the
   secret-detection extraction, because the disposition decides whether redaction joins
   the secret crate or is deleted. Deletion is the default if no genuine consumer is
   identified. `eggsec-transport`'s narrower debug-redaction surface is **not** a
   consumer candidate unless M001 demonstrates that unification preserves
   `eggsec-transport`'s dependency-light constraint.

5. **No existing rejection is reopened.** The crate names `eggsec-net`,
   `eggsec-web-client`, `eggsec-evidence`, `eggsec-signing`, `eggsec-loadtest`,
   `eggsec-resilience`, and `eggsec-utils` remain forbidden by guards 107 and 125, and
   this ADR does not weaken either.

## Consequences

### Positive

- 7,878 lines of corpus stop dragging the engine dependency closure.
- Three crates gain isolated test suites (233 / 11 / 21) that run without the engine,
  giving the corpus a faster feedback loop than in-engine unit tests.
- The `Phase C` rationale — "an independent test story with a narrower graph" — is
  applied consistently instead of selectively.
- The redaction dead-code defect is resolved with a guard, closing the loop Phase D
  opened on `utils::cache`.
- Third-party reuse is preserved as an explicit option without incurring an
  unrepayable public contract now.

### Negative

- Adds three manifests, three dependency edges, and four new guard checks to maintain.
- Corpus data is now versioned separately from engine logic even while both ship
  together, so intra-workspace version bumps must be kept in step.
- The re-export facades are permanent indirection, not removable scaffolding.

### Neutral or deferred

- Publication, neutral naming, and independent versioning: deferred to the
  publication-qualification milestone, which requires its own release plan.
- `vuln/` (1,273 lines, CVSS scoring and triage) was measured and is a plausible future
  corpus crate, but it carries a `crate::error` seam that the spike found is not yet
  free. Not in scope for this ADR.
- `recon/techdetect.rs` fingerprint tables, `scanner/endpoints.rs` `DEFAULT_ENDPOINTS`
  (347 paths), `supply_chain/`, and `compliance/` were measured and are candidates for
  a future ADR; each is deferred with its measurement recorded in the roadmap.

## Compatibility and migration

No storage, protocol, configuration, or wire-format migration. No schema change. No
data migration: the corpora are compiled-in constants, not persisted state.

Source compatibility is a hard requirement of decision 2. The engine keeps every
existing public path as a re-export; no consumer inside or outside this workspace —
Python bindings, TUI, daemon, CLI — may be required to change an import to complete an
extraction milestone. The only permitted edits are the crates' own internal import
paths and manifest wiring.

## Security and reliability implications

**Authorization.** None of these corpora authorizes anything, resolves DNS, opens a
socket, or reads a scope. A corpus crate MUST NOT acquire `Capability` checks,
`ApprovedOperation` construction, or `Scope` access. If a future corpus needs a
privilege decision, that is enforcement ownership and belongs in `eggsec-policy`.

**Secret handling.** `recon/secrets.rs` detects credential material and
`utils/redaction.rs` masks it. Extracting the detector into a crate with no engine
dependency removes any possibility of the detector reaching engine secret state — a
mildly security-positive change. The detector's entropy heuristic
(`secret_entropy(value) < 3.5`, applied only to `SecretType::AwsSecretKey` candidates so
that ordinary 40-character AWS secret-key matches do not fire on low-entropy noise)
MUST be preserved byte-for-byte; changing it changes detection results and is out of
scope.

**Reliability.** The 6 live-probe payload modules stay in the engine precisely so the
corpus crate has no network surface: a payload corpus that cannot make requests cannot
leak scope or bypass transport checkpoints. `eggsec-udp-scan` already established the
precedent that this repo accepts a crate whose job is to *report* that privilege is
required rather than acquire it.

**No DoS change.** The payload corpus is lazily built behind `LazyLock`
(`PAYLOAD_CACHE`, `ALL_PAYLOADS_CACHE`). That laziness MUST be preserved; eagerly
materializing all 40 variants at startup would regress binary init cost.

## Verification

An implementation conforms to this ADR when:

- `cargo tree -p <corpus-crate>` shows `eggsec-core` as its only workspace dependency
  and no Tokio, HTTP, TLS, filesystem, frontend, engine, or transport dependency;
- `cargo test -p <corpus-crate>` passes with the crate's full original test count
  (233 / 11 / 21 respectively), unmodified;
- the engine re-exports each corpus type at its pre-extraction path, proven by
  `cargo check -p eggsec-python -p eggsec-tui` with no consumer edits;
- the engine crate's dependency closure shrinks or holds, and no new edge points from
  a corpus crate toward the engine;
- a new guard check fails if a corpus crate gains a forbidden dependency or an engine
  import, mirroring checks 121–123 for `eggsec-policy`;
- `make check` is green, including the new per-crate test lines.

## Supersession

None.