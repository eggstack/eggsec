# Phase A — Ownership and primitive cleanup

Status: Ready for handoff

Date: 2026-09-16

Depends on: none

Roadmap: `crate-boundary-consolidation-roadmap-2026-09-16.md`

## Purpose

Prepare the engine for any justified crate extraction by fixing ownership inside the existing workspace first. This phase deliberately creates no new crate. It removes false ownership, consolidates duplicated runtime primitives, and reduces the `utils` catch-all so later extraction decisions are based on clean semantic boundaries rather than directory size.

The phase is successful even if some candidate moves are rejected. The required outcome is that each remaining cross-cutting module has an explicit owner and that duplicate implementations are not preserved merely because they already exist.

## Baseline questions to answer before editing

Record the current call sites and dependency impact for:

- `eggsec_output::schedule::{CronExpression, CronScheduler, ScanQueue, ScheduledScan, ScanType, ScanOptions, Priority}`;
- `eggsec_output::session::{ScanSession, SessionInfo, TabSessionState, InputFieldState}`;
- `utils::{cache, circuit_breaker, client_pool, http, network, output, parsing, progress, rate_limiter, redaction, service_detection, stealth, target, validation, privilege}`;
- all `ClientPool` / `OptimizedClientPool` constructors and getters;
- all runtime rate limiter types in `utils::rate_limiter` and `eggsec-output::schedule`;
- all direct users of `CronScheduler` and `ScanQueue`;
- all users of `utils::service_detection`, `utils::progress`, `utils::output`, and `utils::privilege`.

Use code search plus `cargo tree` rather than assumptions. Retain a short ownership table in the completion record.

## Workstream 1 — Establish ownership rules

Classify every candidate using these categories:

1. **Domain-specific** — move to the domain that owns the semantics. Example: service fingerprint knowledge belongs under scanner/fingerprinting, not a global utility bucket.
2. **Frontend/process-host** — move terminal rendering/progress and user-facing process behavior toward CLI/TUI/process-host ownership.
3. **Engine infrastructure** — keep in the engine if several engine domains use it but it is not independently stable enough for a crate.
4. **Protocol/data contract** — keep in an existing leaf DTO crate if it is pure data and the ownership matches that crate.
5. **Candidate reusable primitive** — keep internal in this phase, harden its semantics, and reconsider extraction only in Phase D with real consumer evidence.
6. **Dead/redundant** — remove rather than relocate.

Do not create `common`, `shared`, `helpers`, or `eggsec-utils` as a destination.

## Workstream 2 — Remove scheduling from output ownership

`eggsec-output/src/schedule.rs` currently combines four concerns: scan DTOs, a priority queue, a token-bucket limiter, and cron parsing/scheduling. None is report formatting.

### Required steps

1. Inventory internal and public-path consumers.
2. Compare `ScanQueue` behavior with `eggsec-agent::TaskScheduler`:
   - priority semantics;
   - delayed execution;
   - retry behavior;
   - task leasing;
   - cancellation;
   - persistence expectations;
   - synchronous vs async ownership.
3. Remove duplicate queue behavior where `TaskScheduler` is a strict functional superset and the dependency direction is valid.
4. Do not pollute `eggsec-agent` with scan-specific DTOs merely to reuse its queue. If scan-specific scheduling remains necessary, place the adapter/scan request types in the engine composition layer and translate into the generic scheduler.
5. Move `CronExpression`/`CronScheduler` to the narrowest real execution owner. If autonomous-agent scheduling is the only durable consumer, `eggsec-agent` is acceptable. If several engine surfaces consume cron directly, keep a small engine-owned scheduling module until Phase D rather than adding a crate.
6. Remove the local `eggsec-output::schedule::RateLimiter`; use the consolidated runtime rate-control implementation or remove rate limiting from the scheduler if the scheduler should only schedule.
7. Delete `schedule.rs` from `eggsec-output` once no legitimate output responsibility remains.

### Compatibility rule

Do not make `eggsec-output` depend upward on `eggsec`, `eggsec-agent`, runtime, CLI, or TUI just to preserve an old re-export. If the move cannot preserve the old public path without inverting dependencies, document the pre-1.0 API change in the completion record and repository docs. Internal call sites must all migrate in the same phase.

## Workstream 3 — Move frontend session persistence out of `eggsec-output`

`eggsec-output/src/session.rs` persists tab input state and tab-keyed JSON results. This is session/frontend state, not report output.

Required sequence:

1. Identify every consumer and confirm whether the model is TUI-only, shared frontend state, or engine session state.
2. If TUI-only, move it to `eggsec-tui` and keep filesystem persistence there.
3. If multiple frontends genuinely use the same state, split the pure DTOs from persistence and place the DTOs in an existing frontend-neutral owner only if their semantics align. Do not put filesystem I/O into `eggsec-ui-model` merely because it is frontend-neutral.
4. If daemon/runtime session state already supersedes this format, migrate consumers and remove the legacy session format instead of preserving two session models.
5. Remove `eggsec-output::session` once ownership is resolved.

Do not introduce a new session crate in this phase.

## Workstream 4 — Decompose `utils` by semantic owner

Treat the following as the preferred disposition, subject to call-site verification:

### Strong move/remove candidates

- `service_detection` -> scanner/fingerprint ownership.
- `progress` -> CLI/TUI/process-host or operation-specific presentation layer; core executors should expose events/callbacks, not `indicatif::ProgressStyle`.
- `output` -> CLI/TUI/process-host; engine/library code should return values/errors rather than print.
- `privilege` -> `platform` capability/prerequisite detection; keep read-only/fail-closed semantics.
- `stealth` -> the active-testing domains that actually own user-agent/header/timing behavior; do not leave evasion semantics in global utilities.
- `client_pool` -> remove unless Workstream 6 demonstrates a concrete requirement.

### Keep internal until ownership is clearer

- `cache`;
- `circuit_breaker`;
- `rate_limiter`;
- `redaction`;
- generic parsing/target/validation helpers used by multiple engine domains.

For these, prefer small engine modules named for the capability (`resilience`, `target`, `redaction`) over continuing to grow `utils`, but do not manufacture a crate boundary in Phase A.

### Avoid false consolidation

`target`, URL parsing, scope normalization, and transport URL validation are not automatically the same abstraction. Preserve the canonical scope/policy semantics and transport DTO contracts established by prior roadmaps. Remove duplication only when behavior and failure semantics are demonstrably identical.

## Workstream 5 — Consolidate and harden rate limiting

There are currently distinct layers that should remain conceptually separate:

- `eggsec-tool-core::ratelimit`: protocol/config/status DTOs;
- engine runtime implementation(s): actual token/adaptive/per-target acquisition;
- operation-specific configuration mapping.

Keep `eggsec-tool-core` data-only. Consolidate runtime implementation duplication.

### Required correctness fixes/tests

1. `PerTargetRateLimiter` must not hold the global target-map mutex while awaiting a target-specific limiter. Use a map-to-per-target-state pattern (`Arc<Mutex<_>>`, DashMap entry, or equivalent) so target A throttling cannot serialize unrelated target B.
2. Define whether success resets failure/penalty history and test the intended semantics.
3. Define burst semantics explicitly; do not have two token buckets with different implicit burst behavior under the same conceptual API.
4. Test zero/very-low/high configured rates and overflow/saturation boundaries.
5. Preserve cancellation behavior for waiters; a cancelled operation must not sleep indefinitely behind a limiter.
6. Keep status/report DTO conversion outside the core algorithm so a future reusable implementation is not coupled to `eggsec-tool-core`.

Do not create `eggsec-resilience` yet. Phase D owns that decision after real consumers are identified.

## Workstream 6 — Remove or justify the Reqwest multi-client pool

`utils::client_pool::ClientPool` pre-creates N `reqwest::Client` instances and round-robins them. Each Reqwest client already manages its own internal connection pool, so this design can shard reusable connections and TLS state without an obvious benefit.

Required steps:

1. Enumerate all production consumers.
2. For ordinary outbound HTTP consumers already eligible for `eggsec-transport`, migrate them toward the scoped transport seam instead of preserving a Reqwest pool abstraction.
3. For residual Reqwest-specific consumers, compare one long-lived cloned client against the current N-client pool under representative concurrency.
4. Remove `ClientPool` and `OptimizedClientPool` unless measurements show a requirement that a normal shared client/transport cannot satisfy.
5. If retained, document the exact property being achieved (e.g. intentionally isolated cookie/auth/pool state) and rename the abstraction to describe that property rather than “optimized”.

No new direct Reqwest consumers may be introduced as part of this cleanup.

## Workstream 7 — Harden the circuit breaker before considering reuse

The current circuit breaker is small and potentially reusable, but its semantics must be explicit first.

Required behavior decisions/tests:

- decide whether failures must be consecutive or cumulative while closed;
- decide whether a closed-state success resets the failure counter;
- limit half-open probe concurrency so timeout expiry cannot release an unbounded herd;
- define failure/success accounting for rejected calls;
- test concurrent state transitions;
- test repeated open -> half-open -> open cycles;
- keep time acquisition abstract enough that tests do not require broad Tokio `test-util` features.

Keep the implementation engine-local in this phase.

## Workstream 8 — Guard ownership regressions

Extend the existing architecture checks only where a durable rule can be expressed mechanically. Candidate checks:

- `eggsec-output` must not expose scheduling/session modules after migration;
- scanner service-detection data must not reappear under global utilities;
- no `reqwest::Client` pool abstraction reappears outside explicitly allowed owners;
- no second runtime token-bucket implementation appears in output/frontend crates;
- no new `eggsec-utils`/`common` workspace crate appears.

Prefer AST/manifest/path checks over brittle exact line matching.

## Required verification

```text
cargo check -p eggsec-output
cargo test -p eggsec-output
cargo check -p eggsec-agent
cargo test -p eggsec-agent
cargo check -p eggsec --no-default-features
cargo test -p eggsec --lib
cargo check -p eggsec-tui
cargo check --workspace --no-default-features
make check-feature-profiles
make check-features-individual
make test-architecture-guards
make check
```

Run targeted concurrency tests for the limiter and circuit breaker repeatedly under `cargo test` to catch lock/state races.

## Acceptance criteria

1. Phase A adds no workspace crate.
2. Scheduling and frontend session persistence no longer have `eggsec-output` as their implementation owner, or the completion record contains a concrete output-specific reason for any retained piece.
3. The duplicate scheduler-local rate limiter is removed.
4. `PerTargetRateLimiter` does not await while holding the global target map lock.
5. Circuit-breaker transition semantics are explicit and covered by concurrent tests.
6. `utils` has fewer unrelated responsibilities and no new catch-all module/crate replaces it.
7. Scanner/terminal/platform-specific helpers move toward their actual owners where the call graph permits it.
8. The Reqwest multi-client pool is removed or retained with benchmark/behavior evidence and an accurately named purpose.
9. No new direct Reqwest ownership is introduced.
10. Existing feature, authorization, transport, frontend, daemon, and Python profiles remain green.

## Expected files touched

- `crates/eggsec-output/src/{schedule,session}.rs` and `lib.rs`;
- `crates/eggsec-agent/` scheduler/cron ownership as justified;
- `crates/eggsec/src/utils/` and destination domain modules;
- `crates/eggsec/src/platform/`;
- `crates/eggsec/src/scanner/`;
- CLI/TUI presentation modules as required;
- architecture guards and ownership documentation;
- this plan completion record.

## Completion record template

Append after execution:

- Baseline SHA / final SHA
- Ownership table before -> after
- Removed duplicate implementations
- Reqwest client-pool disposition and measurements
- Limiter/circuit-breaker semantic decisions
- Architecture guard changes
- Commands/results
- Public API changes
- Residual debt / Phase B blockers
