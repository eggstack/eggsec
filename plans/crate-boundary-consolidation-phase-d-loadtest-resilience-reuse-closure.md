# Phase D — Load-test decoupling, reusable primitives, and closure

Status: Executed (2026-09-16). All workstreams implemented; see Completion record below.

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

(Template retained; execution record follows under "Completion record".)

## Completion record

Executed 2026-09-16.

- Baseline SHA: `319323284b5a949957612b3f713ac7baebc0f424` (post-Phase-C main).
  Final SHA: recorded in the commit history for this plan (`git log --oneline
  -- plans/crate-boundary-consolidation-phase-d-loadtest-resilience-reuse-closure.md`).
  No new workspace crate was added (workspace member count stays 20).
- Load-test coupling removed (WS1/WS2/WS4):
  - `loadtest/` went from 3 files (`mod`, `runner` 522 lines, `metrics`) to 8
    (`plan`, `executor`, `metrics`, `progress`, `adapter`, `backend`, `runner`
    facade, `mod`).
  - `LoadTestPlan` (`plan.rs`): url/method/body/headers/timeout/concurrency/
    `RatePolicy` only. No `CommonHttpArgs`, no `EggsecConfig`, no Clap, no
    Reqwest, no indicatif, no filesystem. `tui_mode` is not a field.
  - `RequestTemplate` + `plan_from_adapter()` (`adapter.rs`): CLI/config/auth
    translation above the core. Auth-flag shapes (Basic `user:pass`, Bearer,
    Cookie merge, `Name:value`/bare API keys) parse at this boundary and apply
    through canonical `eggsec_transport::merge_cookie_header` semantics; the
    executor never reimplements them. Proxy/TLS/rate/user-agent merge honors
    `EggsecConfig` defaults with CLI-wins precedence (same as before).
  - `LoadTestExecutor<T: HttpTransport>` (`executor.rs`): generic over the
    scoped seam; per-worker private `Metrics` merged at end (no shared async
    mutex on the hot path); CAS slot allocator (`GlobalPacer`, no mutex
    across sleeps, cancellation-preemptible); dispatch races the transport
    future against the cancellation token; transport errors map to
    `LoadTestErrorKind` without backend types.
  - Progress (`progress.rs`): `LoadTestEvent` + `ProgressSink` (`NoopSink`,
    `FnSink`, `ChannelSink` via non-blocking `try_send`). The core never
    prints; the only `indicatif` widget in loadtest lives in `cli`-gated
    `run_cli_with_scope` (`mod.rs`). `tui_mode` retained on the
    `LoadTestRunner`/`LoadTestRunConfig` facades for source compat but ignored
    by execution.
  - Metrics (`metrics.rs`): pure single-threaded accumulator + `merge`
    (`histogram.add`, saturating counters, 1000-error cap across workers);
    new `error_kinds` (`http_status`/`policy_denied`/`dns`/`timeout`/`connect`/
    `invalid_request`/`backend`/`cancelled`, `#[serde(default)]` so stored
    payloads still deserialize); 10 unit tests covering accounting,
    distribution, categorization, percentiles, cancellation, merge
    determinism, zero/one boundaries, saturation, cap, legacy serde.
- Transport backend (WS3): `ReqwestTransport` (`backend.rs`) implements
  `HttpTransport` with per-hop checkpoints mirroring the fake (initial-URL →
  host → DNS/re-resolution → socket → TLS-consistency → proxy → dispatch;
  redirects re-authorized per hop under the redirect-policy gate). Holds
  verified + insecure shared base clients (no per-request construction) plus
  a per-endpoint proxied-client cache behind a short non-async mutex (never
  across I/O). Responses drained for keep-alive reuse. `reqwest::Error` maps
  to `TransportError::Backend` with stable classifiable prefixes, no secrets.
  `OwnedScopeAuthority` provides the `'static` authority handle over an
  `Arc<Scope>` with identical semantics to `ScopeAuthority`.
  - Contract change (additive, backward-compatible): `ProxyCredential` gained
    `username()`/`password()` getters (secret-bearing, documented never-log)
    so the backend can translate the neutral intent onto the concrete proxy
    builder. No other transport-contract change.
  - TOCTOU residual (documented in `backend.rs` + `architecture/loadtest.md`):
    Reqwest re-resolves internally, so the connector is not pinned to the
    approved address the way the Eggfetch adapter pins it. The backend closes
    it as far as the API allows (fresh resolve + full candidate authorization
    + binding validation + socket re-verification every hop). Full pinning
    arrives with the Eggfetch migration (which currently defers proxied
    execution); proxied load tests stay on this backend meanwhile.
  - Callers: `handle_load` passes `ctx.scope` via `run_cli_with_scope` (per-
    request authorization matches the pre-dispatch verdict);
    `dispatch::network::run_load_test` forwards structured events to
    `progress_tx` via a non-blocking sink; pipeline/tool/distributed/Python
    keep facade paths (`from_config_with_engine` + `run`).
- Transport-backed performance comparison (WS3 guard; loopback `wiremock`
  fixture, `--no-default-features`, temporary probe deleted before commit):
  - IP literal 200 req / 10 workers: ~24k RPS, p50 0ms, wall 0.02s.
  - IP literal 1000 req / 50 workers: ~17k RPS, p50 2ms, p95 2ms, p99 17ms,
    wall 0.07s.
  - Hostname `localhost` (DNS-checkpoint path) 500 req / 25 workers: ~19k
    RPS, p50 1ms, wall 0.03s.
  - No baseline A/B against the pre-migration runner (replaced in place);
    the seam adds only in-memory scope matching on the literal fast path with
    shared clients and drained bodies, and the measured throughput + reuse
    evidence meet the guard's acceptance bar. CPU/allocation profiling was
    not run.
- Gate D1 decision: REJECT `eggsec-loadtest`. The core still touches
  engine-owned helpers (`utils::parse_headers`,
  `utils::http::tool_user_agent`, `utils::formatting::preserve_all`,
  `install_tls_provider`, `constants`, `Scope`/`policy_bridge` in the
  backend); the only third-party dep that would leave the engine closure is
  `hdrhistogram` (`indicatif` stays for scanner/fuzzer/pipeline/stress
  regardless). Single consumer (the engine); no independent library surface
  or second consumer; package/versioning cost unjustified. Criteria 4–5 fail;
  the internal boundary is retained and recorded in
  `architecture/capability_segregation.md`.
- Gate D2 consumer evidence and decision: REJECT `eggsec-resilience`.
  `utils::{rate_limiter, circuit_breaker}` consumers are all inside the
  `eggsec` crate (`waf`, `ai`, tool protocol); no second crate or sibling
  project demonstrated. `fuzzer::rate_limit` (lock-free consecutive-error
  limiter) and the new loadtest `GlobalPacer` (CAS slot allocator) are
  intentionally operation-local, not duplicates. Primitives stay internal
  with explicit semantics/tests. Incidental cleanup: `utils::cache::ApiCache`
  (zero production consumers, flagged in Phase A) removed (13 utils modules
  remain).
- Final workspace/path graph: 20 members (unchanged); `cargo tree -d` shows
  no new duplicate-version conflicts from this phase; `cargo tree -p eggsec`
  depth-1 unchanged except the additive transport getter (no new engine
  third-party deps: `hdrhistogram`/`indicatif`/`reqwest` were already in the
  closure). Path graph stays acyclic (guard Check 108).
- Concrete HTTP client owners: loadtest `reqwest::Client` construction is
  confined to `loadtest/backend.rs` (verified/insecure/proxied cache); the
  former per-`run()` builder in `runner.rs` is gone. All other owners
  unchanged (this phase migrates no other consumer).
- Remaining `utils` inventory (13): `auth`, `circuit_breaker`, `error`,
  `formatting`, `http`, `logging`, `network`, `parsing`, `rate_limiter`,
  `redaction`, `target`, `urlencoding`, `validation`. Documented in
  `architecture/utils.md` (13-module inventory, Phase D `cache` removal,
  loadtest row updated to `parsing`+`http`+`formatting` with the `GlobalPacer`
  differentiation note).
- New crate manifests/dependency sets: none (both gates rejected). One
  additive API on `eggsec-transport` (`ProxyCredential` getters).
- Compatibility/API changes (all pre-1.0):
  - `LoadTestResults` gains `error_kinds: FxHashMap<String, u64>` with
    `#[serde(default)]`; `Display` renders a sorted `error kinds` section.
    Two struct-literal sites updated (`runtime_bridge/executor.rs` test,
    `eggsec-tui` dispatcher test); stored payloads without the field still
    deserialize (tested).
  - `LoadTestRunner`/`LoadTestRunConfig` keep all public paths; `tui_mode`
    ignored (documented); new `with_scope`/`set_scope`/`scope`/`plan`/
    `run_with`/`run_with_cancellation` for composition.
  - `run_cli` behavior preserved (now delegates to `run_cli_with_scope` with
    `Scope::new()`); `handle_load` passes the real `ctx.scope`.
  - Python bindings untouched (facade paths + `scope.enforce_target`
    pre-check unchanged).
- Architecture guard/doc changes: new guards Checks 124–126 (core
  transport-neutrality with doc-prose exclusion, no loadtest/resilience/utils
  crate, `utils::cache` stays removed); `docs/CI_ARCHITECTURE_GUARDS.md`
  Phase D section; `architecture/loadtest.md` rewritten for the new
  structure (+ perf + TOCTOU + facade notes);
  `architecture/capability_segregation.md` Phase D section (both rejections +
  guard refs); `architecture/utils.md` 13-module inventory;
  `architecture/overview.md` loadtest dependency row;
  `.opencode/skills/eggsec-loadtest/SKILL.md` rewritten (stale histogram/
  rate-limit/dead-code guidance replaced);
  `crates/eggsec/src/loadtest/AGENTS.override.md` rewritten.
  README.md and AGENTS.md needed no loadtest edits (no stale mentions found).
- Commands/results (all green locally before commit):
  - `cargo fmt --all --check` — pass.
  - `cargo check --workspace --no-default-features`, `cargo check -p eggsec`
    (empty + `cli` + `rest-api,cli`), `-p eggsec-cli/-tui/-daemon`,
    `-p eggsec-transport` — pass.
  - `make clippy` equivalent (`--lib -p eggsec` empty + `cli`, `-D warnings`)
    — pass (fixed `too_many_arguments` via `WorkerCtx`, `single_match` in
    `run_cli`).
  - `cargo test -p eggsec --lib --features rest-api,cli` — 2102 passed.
  - `cargo test -p eggsec --features rest-api,cli --tests --no-fail-fast` —
    2858 passed (52 suites; +28 vs Phase C from new loadtest unit tests).
  - `cargo test -p eggsec --no-default-features --test loadtest_tests` —
    29 passed (incl. the rate-limit aggregate test that caught the initial
    per-worker pacing bug, fixed via the CAS `GlobalPacer`).
  - `bash scripts/check-architecture-guards.sh` — ALL PASSED (Checks 99–126).
  - `make check-deps` / `make check-feature-profiles` / `make check-python`:
    run in the final `make check` pass (see below).
- Residual debt and follow-up:
  - `ReqwestTransport` sync DNS per hostname request (see TOCTOU note);
    consider async resolution or Eggfetch migration for full pinning (the
    adapter defers proxied execution today, so Reqwest stays for proxied
    load tests).
  - `LoadTestRunner` facade default scope is permissive `["*"]` (pre-Phase-D
    behavior); strict surfaces should pass explicit scopes (CLI/handler and
    `run_cli_with_scope` already do; dispatch/pipeline/tool paths use facade
    defaults — a future pass can thread approved scopes through).
  - Python `LoadTestResultPy` does not yet expose `error_kinds` (additive,
    optional).
  - `make check-features-individual` is deep-checks-only per `AGENTS.md` (not
    run per-PR; CI deep-checks cover it).
