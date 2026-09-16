# Phase D — Load-test decoupling, reusable primitives, and closure

Status: Ready for handoff

Date: 2026-09-16

Depends on: Phases A-C

Roadmap: `crate-boundary-consolidation-roadmap-2026-09-16.md`

## Purpose

Complete the crate-boundary pass by removing the remaining engine/UI/concrete-client coupling from HTTP load testing, deciding whether load testing now justifies an independent crate, deciding whether hardened resilience primitives have enough real reuse to justify a shared library, and recording final dependency/ownership measurements.

This phase contains decision gates. It is valid for the final result to keep load testing or resilience primitives inside `eggsec` if extraction would only add package/versioning overhead without removing a real dependency edge or serving an independent consumer.

## Workstream 1 — Make load testing a clean engine component first

`crates/eggsec/src/loadtest/runner.rs` currently mixes:

- load-test plan/configuration;
- engine `EggsecError`;
- `EggsecConfig` adaptation;
- `CommonHttpArgs` adaptation;
- Reqwest method/client/proxy/TLS construction;
- authentication-header parsing;
- user-agent policy;
- concurrency/rate scheduling;
- metrics collection;
- `indicatif` terminal progress;
- report conversion.

Do not extract this shape into a crate. First split it internally into a transport-neutral executor and engine/process-host adapters.

### Target internal API

Aim for an API conceptually similar to:

```text
LoadTestPlan
  target/request template
  request count or duration budget
  concurrency
  timeout
  explicit rate policy

LoadTestExecutor<T: HttpTransport>
  transport
  NetworkAuthority supplied at composition
  cancellation token
  optional progress/event sink

LoadTestResults / metrics
  pure result data
```

The exact type names may differ, but the core executor must not know about Clap args, `EggsecConfig`, terminal progress widgets, Reqwest builders, or filesystem output.

## Workstream 2 — Separate configuration adaptation from execution

Keep Eggsec-specific configuration mapping above the load-test core.

Create one adapter path that translates:

- CLI/TUI/tool/runtime request DTOs;
- `EggsecConfig` defaults;
- auth context/common HTTP settings;
- policy-approved target and risk budget;

into a plain load-test plan plus a scoped transport/request template.

Do not store `CommonHttpArgs` inside the reusable plan. It is an Eggsec surface adapter type, not a load-test domain primitive.

Header/auth application should use the canonical transport-neutral helpers established by the network-dependency roadmap rather than reimplementing Basic/Bearer/API-key/cookie parsing inside the runner.

## Workstream 3 — Replace concrete Reqwest ownership with the scoped transport seam

The core load-test executor should issue requests through `eggsec_transport::HttpTransport` and a caller-supplied `NetworkAuthority`.

Requirements:

1. Every request still passes the existing destination/scope checkpoints.
2. Redirect handling remains transport-controlled and re-authorized per hop.
3. Proxy/TLS intent is represented through transport DTOs, not Reqwest types.
4. Response bodies are consumed/drained as required by the transport implementation for connection reuse.
5. Transport errors are classified into load-test result categories without exposing concrete backend error types.
6. Cancellation terminates request issuance and rate waiters promptly.
7. Concurrency does not serialize through a shared global mutex in metrics or rate-control code.
8. The Eggfetch adapter remains a backend implementation, not a dependency of the load-test domain API.

### Performance guard

Because load testing is throughput-sensitive, compare the transport-backed implementation with the baseline runner using loopback/local fixtures. Record:

- requests/second;
- p50/p95/p99 latency overhead;
- CPU time if available;
- allocation or memory signal if practical;
- behavior at representative concurrency levels;
- connection reuse evidence where observable.

A scoped transport seam is mandatory for architecture/security, but avoid accidental per-request setup that destroys connection reuse or makes load testing unusably slow. Optimize the adapter/transport implementation rather than bypassing authorization.

## Workstream 4 — Remove terminal progress from the core executor

The load-test core should emit progress/state through a minimal callback/event interface or use the existing runtime event model where appropriate.

`indicatif::ProgressBar` / `ProgressStyle` belongs in CLI/process-host presentation, not the executor.

Required behavior:

- CLI may attach an Indicatif renderer;
- TUI consumes structured progress;
- library/Python/daemon execution may omit progress or consume events;
- the executor never prints directly;
- TUI mode is not a boolean stored in the domain executor merely to suppress terminal output.

Remove `tui_mode` from the core contract once structured progress is in place.

## Workstream 5 — Make metrics independently testable

Keep latency/status/error aggregation pure and concurrency-safe.

Review the current metrics lock granularity. If every completed request serializes on one async mutex, consider an accumulator design that reduces contention while preserving deterministic final percentiles/counters.

Required tests:

- total/success/failed accounting;
- status-code distribution;
- transport-error categorization;
- latency histogram percentiles;
- cancellation/partial-run totals;
- high-concurrency updates;
- zero/one request boundary cases;
- overflow/saturation where counters convert between integer widths.

Do not couple metrics to report renderers; convert results to `eggsec-report-model` above or at a narrow adapter boundary.

## Gate D1 — Decide whether to create `eggsec-loadtest`

After Workstreams 1-5, measure the candidate extraction.

Create `eggsec-loadtest` only if all are true:

1. core load-test files have no imports from the `eggsec` engine crate;
2. the candidate can depend only on narrow contracts such as `eggsec-transport`, `eggsec-report-model` if genuinely needed, Tokio task/sync/time features, histogram/serde/tracing primitives, and other load-test-specific dependencies;
3. engine configuration/frontends can adapt into the crate without reverse dependencies;
4. the extraction removes load-test-specific dependencies/code from the `eggsec` compile unit or supplies a real independently useful library surface;
5. package/versioning/release cost is justified by the isolated testability/dependency boundary;
6. performance through the injected transport remains acceptable.

If the gate fails, keep the cleaned load-test module inside `eggsec` and record why. The decoupling work is still required and still valuable.

### If extracted

Preferred direction:

```text
eggsec-transport      eggsec-report-model (optional)
       ^                     ^
        \                   /
          eggsec-loadtest
                ^
                |
              eggsec
```

`eggsec-loadtest` must not depend on `eggsec`, CLI/TUI, Eggfetch, Reqwest, Rustls, or config loading.

## Workstream 6 — Evaluate a reusable resilience library only against real consumers

Phase A leaves hardened circuit-breaker/rate-control primitives internal. Revisit them now.

### Candidate surface

Potentially reusable concepts:

- circuit breaker with bounded half-open probes;
- token bucket / paced permit acquisition;
- adaptive rate controller;
- per-key/per-target limiter;
- jitter/backoff primitives;
- cancellation-aware wait behavior.

### Gate D2

Do not create `eggsec-resilience` or a neutral Eggstack crate unless:

1. at least two independent crates or sibling projects have an immediate use for the same semantics;
2. the API contains no Eggsec request/status/policy DTOs;
3. the primitives can be tested without broad runtime/network dependencies;
4. extracting removes duplicate implementations rather than creating adapters around existing ones;
5. clock/cancellation behavior is explicit and testable;
6. concurrency behavior is documented and free of global-lock-await problems;
7. the chosen repository/package ownership makes sense for consumers.

If only Eggsec uses the algorithms, keep a focused internal `resilience` module. A clean module is better than a speculative library.

If sibling-project reuse is demonstrated but a separate Eggstack repository is the better owner, record that prerequisite rather than creating an Eggsec-branded crate that other projects must depend on awkwardly.

## Workstream 7 — Treat existing `eggsec-transport` as the reference reusable boundary

No extraction is needed: it is already a standalone leaf crate.

During closure, document which qualities made it successful so future extractions use the same bar:

- implementation-neutral API;
- narrow dependency set;
- no process-host/runtime ownership beyond what the contract requires;
- clear security invariant;
- fake/test implementation;
- one-way backend adapter (`eggsec-transport-eggfetch`);
- no dependency back to the engine.

Do not rename/generalize `eggsec-transport` solely for aesthetics. Reconsider a neutral package identity only when a real non-Eggsec consumer exists and the API can remain backward compatible.

## Workstream 8 — Evaluate cross-project reuse of `eggsec-policy` only after extraction proves stable

If Phase C creates `eggsec-policy`, keep its first iteration Eggsec-named and behaviorally identical. Do not simultaneously redesign it as a universal authorization framework.

A future neutralization is justified only if another project needs the same concepts (operation risk/capability, target scope, execution surface, approval binding) without Eggsec-specific operation catalog coupling.

Record potential consumers and incompatibilities, but no cross-repository rewrite is required for Phase D closure.

## Workstream 9 — Final dependency and ownership measurements

Capture before/after for the full roadmap:

```text
cargo metadata --no-deps
cargo tree -d
cargo tree -p eggsec
cargo tree -p eggsec-output
cargo tree -p eggsec-report-model          # if created
cargo tree -p eggsec-policy                # if created
cargo tree -p eggsec-loadtest              # if created
cargo tree -p eggsec-db-lab
cargo tree -p eggsec-mobile-lab
cargo tree -p eggsec-web-proxy
```

Record:

- workspace member count;
- path dependency edges;
- direct dependencies per affected crate;
- transitive package counts for affected artifacts where useful;
- concrete HTTP client owners;
- Tokio feature sets for any new crate;
- modules remaining under `crates/eggsec/src/utils`;
- duplicate rate limiter/circuit breaker implementations;
- report DTO owner;
- policy DTO/algorithm owner;
- load-test concrete-client/UI ownership;
- public compatibility facades retained.

Do not claim success based on Cargo.toml line count alone.

## Workstream 10 — Update architecture and guards

Update at minimum as applicable:

- `architecture/overview.md`;
- `architecture/capability_segregation.md`;
- `architecture/config.md` / policy documentation;
- output/report architecture docs;
- loadtest architecture doc;
- utils architecture doc;
- dependency/release docs;
- `AGENTS.md` and relevant `.opencode/skills` guidance;
- architecture guard scripts and docs;
- `plans/README.md` with this roadmap's final executed/active state.

Guards should enforce dependency direction and forbidden capability edges, not exact file counts that make routine refactoring painful.

## Required verification

Always run:

```text
cargo check --workspace --no-default-features
cargo check -p eggsec
cargo test -p eggsec --lib
cargo check -p eggsec-cli
cargo check -p eggsec-tui
cargo check -p eggsec-daemon
cargo check -p eggsec-python
cargo tree -d
make check-feature-profiles
make check-features-individual
make test-architecture-guards
make check
make check-python
```

If created, additionally run isolated check/test/package commands for:

```text
eggsec-report-model
eggsec-policy
eggsec-loadtest
```

Run local-fixture load-test performance comparisons after the transport migration and record exact commands/host assumptions.

## Acceptance criteria

1. The load-test core constructs no Reqwest client and exposes no Reqwest type.
2. Load-test core logic contains no Clap/TUI/Indicatif/config-file/output-file ownership.
3. Request execution goes through the scoped transport contract without weakening DNS/redirect/proxy/TLS authorization invariants.
4. Progress is structured and optional; core execution never prints.
5. Load-test metrics are independently tested and do not impose avoidable global contention.
6. `eggsec-loadtest` is created only if Gate D1 demonstrates a real independent dependency/API boundary; otherwise rejection is recorded.
7. A resilience crate is created only if Gate D2 has real independent consumers and removes duplication; otherwise primitives stay internal.
8. `eggsec-transport` remains the reference implementation-neutral leaf boundary and is not renamed speculatively.
9. Any `eggsec-policy` reuse discussion does not destabilize the freshly extracted policy API.
10. Final before/after dependency/ownership measurements are retained in an architecture closure document or this plan completion record.
11. Architecture guards match the final dependency direction.
12. Full feature/frontend/daemon/Python checks are green.

## Expected files touched

- `crates/eggsec/src/loadtest/`;
- engine config/request adapters and runtime bridge for load-test construction;
- CLI/TUI load-test progress rendering;
- `eggsec-transport` integration call sites (contract changes only if genuinely required and backward-compatible);
- optional new `crates/eggsec-loadtest/` if Gate D1 passes;
- internal resilience module or optional separate crate decision record if Gate D2 passes;
- architecture/guard/release docs;
- `plans/README.md` final roadmap state;
- this plan completion record.

## Completion record template

Append after execution:

- Baseline SHA / final SHA
- Load-test coupling removed
- Transport-backed performance comparison
- Gate D1 decision and dependency tree
- Gate D2 consumer evidence and decision
- Final workspace/path graph
- Concrete HTTP client owners
- Remaining `utils` inventory
- New crate manifests/dependency sets
- Compatibility/API changes
- Architecture guard/doc changes
- Commands/results
- Residual debt and any future cross-repository follow-up
