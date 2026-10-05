# Security Knowledge Corpus Roadmap

Status: in progress (milestone 1 closed; milestones 2–5 pending)

Long-term references:

- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md#phase-1--crate-boundaries-and-reusable-library-ownership`

Related ADRs:

- `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`

Decision record:

- `architecture/capability_segregation.md` (Phase G, added by M001)

## 1. Purpose and ownership boundary

This subsystem owns **domain knowledge expressed as data or as pure transformation
over data**, extracted from the `eggsec` engine into leaf crates.

What this subsystem owns:

- Security payload corpora (injection, traversal, deserialization, protocol-abuse
  payload sets).
- Credential/secret detection patterns and their confidence scoring.
- Service fingerprinting knowledge (port→service tables, banner heuristics).
- The redaction primitives, once their disposition is settled.

What it consumes: `eggsec-core` only, for `Severity` and shared constants.

What it must **not** own: authorization, scope resolution, transport, dispatch,
rendering, network I/O, subprocess execution, or any frontend surface. A corpus that
needs an engine type is not a corpus; see ADR-0005 decision 1.

This is deliberately a narrow subsystem. It is not "decompose the engine". Measured
across all 41 engine modules, only these are near-zero-coupling; `pipeline` (22
distinct `crate::` paths) and `tool` (658 references) are orchestrators whose coupling
is their function.

## 2. Work classification

### Invariants

- A corpus crate's only permitted workspace dependency is `eggsec-core`; it has no
  Tokio, HTTP, TLS, filesystem, frontend, engine, or transport dependency.
- Corpus crates authorize nothing, resolve nothing, open no socket, and spawn no
  process.
- Engine-side paths remain valid through re-export; no consumer may be required to
  change an import.
- Payload corpus laziness (`LazyLock` caches) is preserved.
- Guards 107 and 125 continue to forbid `eggsec-net`, `eggsec-web-client`,
  `eggsec-evidence`, `eggsec-signing`, `eggsec-loadtest`, `eggsec-resilience`, and
  `eggsec-utils`. This subsystem never weakens them.

### Capabilities

- Third-party consumers can depend on a corpus crate without the engine (deferred to
  the publication milestone; not claimed by internal extraction).

### Infrastructure

- `eggsec-payloads`, `eggsec-secrets`, `eggsec-service-db` crates with isolated test
  suites.
- Engine re-export facades at pre-extraction paths.
- Static guards enforcing the leaf invariant.

### Polish

- `architecture/overview.md` crate table and dependency-map updates.
- `docs/FEATURE_MATRIX.md` domain inventory entries.

## 3. Non-goals

- **Not** a general engine decomposition. `tool`, `pipeline`, `waf`, `distributed`,
  `c2`, `postex`, `browser`, `integrations`, `auth`, and the `recon` bulk stay in the
  engine. Their coupling is executor-bound and re-measured in M002.
- **Not** a reopening of `Phase C`/`Phase D`/`WS4` decisions. Rejections stand.
- **Not** a publication effort. ADR-0005 decision 3 defers neutral naming, independent
  versioning, and release qualification.
- **Not** a payload-corpus split of the 6 live-probe modules (`graphql`, `grpc`,
  `idor`, `jwt`, `oauth`, `ssti`).
- **Not** a change to corpus content. No payload is added, removed, or re-rated. The
  233 / 11 / 21 tests are the content contract.
- **Not** an enforcement change. No `OperationMetadata`, `Capability`, or approval-token
  effect.

## 4. Current state

`eggsec` is ~195,000 lines and is the workspace's single maintainability hotspot. The
crate-boundary subsystem (`plans/crate-boundary-consolidation-roadmap-2026-09-16.md`)
is closed; its Phase C extracted `eggsec-policy` and its Phase D rejected
`eggsec-loadtest` and `eggsec-resilience`.

The measured candidate set:

| Module | Lines | Files | `crate::` coupling | Deps | Tests | Disposition |
|---|---|---|---|---|---|---|
| `fuzzer/payloads/` pure-data | 7,084 | 36 | ~0 | `serde`, `strum`, `flate2`, `rustc-hash`, `tracing` | 233 | M004 |
| `recon/secrets.rs` | 492 | 1 | 1 | `regex`, `serde`, `tracing` | 11 | M003 |
| `scanner/service_data.rs` | 302 | 1 | 0 | `rustc-hash` | 21 | M002 |
| `utils/redaction.rs` | 366 | 1 | 0 | `regex` | 26 | M001 (defect) |

All four were validated by extraction spike: copied to scratch crates, compiled, and
tested unmodified. The payload corpus needed four mechanical edits (two `Severity`
imports, one module-path rewrite, the `$crate` macro path, and six `get_payloads` arms
redirected for the relocated live-probe types); no behavioral edits.

`fuzzer/payloads/` splits on an existing seam. 34 of the 40 payload-type modules (36 of 42 files, counting `mod.rs` and `macros.rs`) never reference
`reqwest` (7,084 lines, pure data); 6 mix payload generation with live probing
(4,354 lines). The corpus/probe split is already in the source.

`recon/secrets.rs` holds 26 compiled patterns behind a `LazyLock`, a `SecretType`
enum with 30 variants, a `Confidence` tier, and an entropy gate scoped to AWS
secret-key candidates. Its only engine reference is `Severity`.

`scanner/service_data.rs` is a port→service table plus banner heuristics with zero
workspace dependencies. Its symbols are consumed by nothing outside `scanner/`.
Architecture guard check 114 hard-pins its current path.

`utils/redaction.rs` has zero production consumers. `eggsec-transport` maintains
independent, parallel implementations.

Measured and explicitly **not** proceeding (recorded so they are not re-litigated):

| Module | Lines | Reason not proceeding |
|---|---|---|
| `fuzzer/payloads/` live-probe 6 | 4,354 | mixes generation with `reqwest` execution; ADR-0005 Option D |
| `vuln/` | 1,273 | spike found a `crate::error` seam in `cvss.rs` / `exploit.rs`; not yet free |
| `recon/techdetect.rs` | 538 | fingerprint table is separable, but the file owns an HTTP client |
| `scanner/endpoints.rs` table | 347 paths | table is pure `&[&str]`, but the file carries `cli`/`tool-api` coupling |
| `compliance/` | 793 | serde-only, zero I/O, 3 coupling paths — viable but low value; revisit |
| `supply_chain/` | 1,962 | serde-only coupling, filesystem I/O, domain-specific value |
| `utils/` whole | 2,941 | 13 unrelated files, no coherent boundary; `eggsec-utils` forbidden by guard 125 |
| `websocket/` | 1,262 | cleanest mechanically (1 path) but only 2 consumers; not a library |
| `c2/`, `postex/` | 3,795 | domain crates are defensible, but `cli::*Args` and `output::convert` coupling |
| `waf`, `pipeline`, `distributed`, `loadtest`, `auth`, `hunt` | — | executor-bound; coupling is the function |

## 5. Target architecture

Three new leaf crates, each depending on `eggsec-core` and its own third-party
libraries only:

```text
eggsec-payloads   -> eggsec-core, serde, strum, flate2, rustc-hash, tracing
eggsec-secrets    -> eggsec-core, regex, serde, tracing
eggsec-service-db -> rustc-hash

                 +-------------------+
   engine  ---> | eggsec-payloads   |  (re-export at eggsec::fuzzer::payloads)
   re-exports    +-------------------+
                 +-------------------+
              -> | eggsec-secrets    |  (re-export at eggsec::recon::secrets)
                 +-------------------+
                 +-------------------+
              -> | eggsec-service-db |  (re-export at eggsec::scanner::service_data)
                 +-------------------+

eggsec-core (unchanged leaf): Severity, SensitiveString, constants
```

Direction is one-way: engine → corpus. No corpus crate imports the engine, and the
workspace path graph stays acyclic (guard 108).

The 6 live-probe payload modules remain in `eggsec::fuzzer::payloads` and import
`Payload` / `PayloadType` from `eggsec-payloads`, keeping engine-side use of
`PayloadType` intact for `is_advanced()` and `get_payloads()` dispatch.

## 6. Dependency graph

```text
M001 redaction disposition (corrective; no dependency)
  |
  `--> M003 secret extraction (needs M001's redaction decision)
             |
M002 service extraction (independent)
             |
             `--> M005 publication qualification (operational; needs M002+M003+M004)
M004 payload extraction (independent)
```

Dependency classification:

- M001 → M003: **interface — discharged 2026-10-05.** M001 closed with Option 3
  (redaction deleted), so `eggsec-secrets` is scoped to `recon/secrets.rs` alone and
  does not require `serde_json`.
- M002, M004: no dependencies. Dependency-ready now.
- M005: **operational** on all three extractions, and additionally blocked on a
  publication decision that is not a code question.

## 7. Milestones

### Milestone 1 — Redaction dead-code disposition

Class: polish (corrective)

Objective: decide and execute the disposition of `utils/redaction.rs`, currently 366
lines and 26 tests with zero production consumers.

Dependencies: none.

Deliverable boundary: a resolved disposition plus a guard.

User or operator value: removes a latent defect and eliminates a second,
independent redaction implementation in `eggsec-transport`.

Exit conditions: disposition decided and executed; zero-consumer state cannot recur
silently; `architecture/capability_segregation.md` carries the Phase G record.

**Outcome: closed 2026-10-05.** Option 3 (delete) executed. `utils/redaction.rs`
removed (366 lines, 26 tests — disclosed loss); the three Option 1 adoption candidates
were inspected and rejected on evidence, because `findings::Evidence` has no populated
production path, `SecretFinding::value_preview` already applies a *different* tested
truncation policy, and `nse_bridge` already declares redaction declaratively. The
decisive finding: `eggsec-report-model`'s `RedactionState` is the repository's real
redaction contract, so the deleted module was a duplicate of an already-solved problem.
Guard check 147 added and demonstrated to fail on all three of its conditions.

Deferred work: n/a — the "join `eggsec-secrets`" option was not selected, so M003 is
scoped to `recon/secrets.rs` alone and needs no `serde_json`.

### Milestone 2 — Service fingerprint database extraction

Class: infrastructure

Objective: extract `scanner/service_data.rs` into `eggsec-service-db`.

Dependencies: none.

Deliverable boundary: a 302-line, zero-workspace-dependency crate with its 21 tests
running in isolation, re-exported at the engine's existing path.

User or operator value: scanner knowledge is isolated, independently testable, and
reusable; the engine's closure shrinks.

Exit conditions: `cargo test -p eggsec-service-db` green with 21 tests; guard 114
updated to the new canonical owner; new guard enforces the leaf invariant.

Deferred work: `DEFAULT_ENDPOINTS` table extraction (measured, deferred).

### Milestone 3 — Secret detection extraction

Class: infrastructure

Objective: extract `recon/secrets.rs` into `eggsec-secrets`, carrying M001's redaction
disposition.

Dependencies: M001 (interface).

Deliverable boundary: a 492-line crate (plus redaction if adopted) depending only on
`eggsec-core` + `regex`, re-exported at the engine's existing path, with the
Python bindings' exhaustive `SecretType` match untouched.

User or operator value: credential-detection knowledge becomes an isolated,
independently testable, publishable unit.

Exit conditions: `cargo test -p eggsec-secrets` green with 11 tests (plus redaction
tests if adopted); entropy heuristic byte-identical; `cargo check -p eggsec-python`
passes with no binding edits.

Deferred work: `recon/git_secrets.rs` (473 lines) — subprocess orchestration, not
pattern matching; it is a candidate for a future ADR, not this one.

### Milestone 4 — Payload corpus extraction

Class: infrastructure

Objective: extract the 34 pure-data payload modules (7,084 lines) into
`eggsec-payloads`.

Dependencies: none.

Deliverable boundary: a 7,084-line corpus crate whose 233 tests run in isolation, with
the 6 live-probe modules remaining engine-side and importing from it.

User or operator value: the single largest dependency-closure reduction in the
subsystem, and the strongest standalone-library story of the three.

Exit conditions: `cargo test -p eggsec-payloads` green with 233 tests; engine and
`eggsec-python`/`eggsec-tui` compile with no consumer edits; laziness preserved.

Deferred work: publication.

### Milestone 5 — Publication qualification

Class: infrastructure

Objective: decide and, if approved, execute publication of the corpus crates under
neutral names with independent versioning.

Dependencies: M002, M003, M004 (operational). Additionally requires an explicit
maintainer decision that is not a code question.

Deliverable boundary: either a documented deferral, or a release plan with a corpus
update policy, independent semver lines, and a `cargo semver-checks` baseline.

User or operator value: third-party reuse of Eggsec's corpus knowledge.

Exit conditions: decision recorded in an ADR. Publication, if approved, gets its own
roadmap — it is explicitly not in scope here.

## 8. Cross-cutting requirements

### Storage and migration

None. Corpora are compiled-in constants behind `LazyLock`, not persisted state. No
schema, no migration, no backfill.

### Protocol and compatibility

Source compatibility is mandatory: every pre-extraction engine path stays valid via
re-export. This is what allows M003 to leave the Python bindings' exhaustive 30-arm
`SecretType` match untouched.

### Security and authorization

No corpus crate authorizes, resolves, connects, or spawns. `Capability` checks,
`ApprovedOperation` construction, and `Scope` access stay in the engine and
`eggsec-policy`. The secret detector's entropy gate is a detection-semantics constant
and MUST NOT be tuned in this subsystem — including its deliberate narrow scoping to
`SecretType::AwsSecretKey`, which must not be widened to every pattern type. Keeping
the corpus crate network-free means a payload corpus cannot leak scope or bypass
transport checkpoints.

### Concurrency, cancellation, and recovery

Not applicable: corpus crates are synchronous and stateless beyond immutable
`LazyLock` caches. No tasks are spawned, so no timeout wrappers are required. The
existing `LazyLock` initialization is one-time-per-process and MUST remain lazy so
startup cost is unchanged.

### Observability and audit

Existing `tracing` diagnostics inside corpus code are preserved unchanged. No new
telemetry. Corpus crates add no log volume on paths that are already cached.

### Performance and resource use

Eager materialization of all 40 `PayloadType` variants at startup is a regression and
is prohibited; the `PAYLOAD_CACHE` / `ALL_PAYLOADS_CACHE` laziness is load-bearing.
`service_data` retains its `LazyLock` tables for the same reason.

### Documentation and operations

`architecture/overview.md` (crate table + dependency map),
`architecture/capability_segregation.md` (Phase G), `architecture/fuzzer.md`,
`architecture/scanner.md`, `architecture/recon.md`, `architecture/utils.md`, and
`.opencode/skills/` claims that name counted sets must be updated in the same pass as
the code, per the skills-drift rule in `AGENTS.md`.

## 9. Verification strategy

Per milestone: focused `cargo test -p <crate>` with the exact original test count, then
the mandatory contract:

```bash
make check                  # mandatory Rust contract
make check-deps             # cargo deny over --workspace --all-features
make check-features-individual  # deep check; required when the feature surface changes
bash scripts/check-architecture-guards.sh
```

Corpus crates have no feature surface of their own, so `make check-features-individual`
is required only where an engine feature gate moves. `make check-full`,
`make clippy-domain`, and `make test-tui-pty` are deep checks, not per-milestone.

Each milestone adds its `cargo test -p <crate> --tests` line to `make check`.
Note `eggsec-udp-scan` has no such line today; this subsystem does not adopt that gap
for its own crates.

## 10. Risks and decision points

| Risk | Likelihood | Mitigation |
|---|---|---|
| Guard 114 hard-fails on `service_data.rs` moving | Certain | Update guard in the same commit as M002's move |
| Python bindings' exhaustive `SecretType` match breaks | Medium | Re-export, never re-type; M003 verifies `cargo check -p eggsec-python` |
| Corpus content drifts during the move | Medium | Test counts (233/11/21) are the content contract; no content edits in scope |
| A corpus crate acquires an engine dep over time | Medium | New guard per crate, mirroring checks 121–123 for `eggsec-policy` |
| New crate breaks `check-feature-docs.py` inventory | Low | Inventory is a fixed list; add entries only where the doc contract requires |
| Publication pressure makes a corpus public prematurely | Medium | ADR-0005 decision 3; M005 is a separate decision, not a code step |
| Skills/docs drift on counted sets | Medium | Update claims in the same pass as code (`AGENTS.md` skills-drift rule) |

Open decision points requiring an ADR:

- Whether to publish the corpora (M005; deferred by design).
- Whether `vuln/`, `recon/techdetect.rs`, `DEFAULT_ENDPOINTS`, `compliance/`, or
  `supply_chain/` join this subsystem later, given each is viable but low-value today.

## 11. Completion definition

The subsystem is closed when:

- all four extraction/disposition milestones have accepted closure records;
- all three corpus crates satisfy the leaf invariant under `cargo tree` and a guard;
- the engine's re-export facades keep `cargo check -p eggsec -p eggsec-python
  -p eggsec-tui` green with no consumer import edits;
- `make check` is green with per-crate test lines registered for each new crate;
- `architecture/capability_segregation.md` carries the Phase G decision record;
- the publication decision is recorded as accepted, rejected, or deferred with reasons.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| 1 | **closed** | `plans/implementation/security-knowledge-corpus/001-redaction-dead-code-disposition.md` | `plans/closure/security-knowledge-corpus/001-closure.md` | — |
| 2 | not started | `plans/implementation/security-knowledge-corpus/002-service-fingerprint-db-extraction.md` | — | — |
| 3 | not started | `plans/implementation/security-knowledge-corpus/003-secret-detection-extraction.md` | — | M001 **discharged** (redaction deleted; `serde_json` not needed) |
| 4 | not started | `plans/implementation/security-knowledge-corpus/004-payload-corpus-extraction.md` | — | — |
| 5 | not started | `plans/implementation/security-knowledge-corpus/005-publication-qualification.md` | — | M002–M004 (operational) + maintainer decision |