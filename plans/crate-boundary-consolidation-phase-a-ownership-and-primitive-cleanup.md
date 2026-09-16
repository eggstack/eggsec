# Phase A — Ownership and primitive cleanup

Status: Executed (2026-09-16). All workstreams implemented; see Completion record below.

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

## Completion record

Executed 2026-09-16.

- Baseline SHA: `992839ee552ffa100c529c5bebeb00c495e74a6b` (plan handoff head).
  Final implementation SHA: recorded in the commit history for this plan
  (implementation commit + this record; `git log --oneline -- plans/crate-boundary-consolidation-phase-a-ownership-and-primitive-cleanup.md`).
  No new workspace crate was added (acceptance criterion 1 holds;
  `cargo metadata` member count unchanged at 18).
- Ownership table before -> after (call sites verified by `rg`, not assumptions):

  | Module / type | Before | After |
  |---|---|---|
  | `CronExpression` / `CronScheduler` | `eggsec-output::schedule` (report crate owning orchestration) | `eggsec-agent::cron` (only durable consumer is autonomous-agent scheduling); engine uses `eggsec::agent::{CronExpression, CronScheduler}` re-export |
  | `ScanQueue` / `ScheduledScan` / `ScanType` / `ScanOptions` / `Priority` (output copy) / `ScheduleStatus` | `eggsec-output::schedule`, zero external consumers | Removed. Canonical queue is `eggsec-agent::TaskScheduler` (strict superset: priority, delayed execution via `scheduled_for`, retry via `retry_count`/`max_retries`, leasing + lease reclamation, cancellation, outcomes). Scan DTOs were dead code; per the plan they were not moved into `eggsec-agent` |
  | `eggsec-output::schedule::RateLimiter` | second token bucket in output crate | Removed (acceptance criterion 3) |
  | `ScanSession` / `SessionInfo` / `TabSessionState` / `InputFieldState` | `eggsec-output::session`, zero external consumers | Removed. Superseded by daemon/runtime durable sessions (`SessionId`, persisted snapshots) + frontend `AppState`; no second session model is preserved |
  | `utils::service_detection` | global utility bucket | `scanner::service_data` (tables, banner heuristics, classifiers); `scanner::ports` updated, `scanner/mod.rs` re-exports |
  | `utils::progress` (`indicatif` styles) | global utilities, zero production consumers | Removed (dead); frontends own presentation |
  | `utils::output` (`print_*`) | global utilities, zero production consumers | Removed (dead); engine returns values/errors, CLI/TUI own presentation |
  | `utils::privilege` | feature-gated utilities | `platform::{is_root, check_privileged, require_root}` (read-only/fail-closed; `is_root` already lived there dependency-light); callers in `stress/{udp,utils}`, `scanner/ports/spoofed`, `eggsec-tui` packet tab migrated; feature gate removed |
  | `utils::stealth` | global utilities | Removed. `StealthConfig`/`BrowserFingerprint`/`TlsFingerprint`/`default_user_agents` were dead (only `tool_user_agent()` was used, by loadtest + fuzzer). Honest identifier moved to `utils::http::tool_user_agent()`; evasion semantics not relocated |
  | `utils::{cache, circuit_breaker, rate_limiter, redaction}` + parsing/target/validation | mixed bucket | Kept engine-internal as engine infrastructure (documented in `utils/mod.rs` header + `architecture/utils.md`); no crate boundary manufactured in Phase A |
  | `Agent::scheduler: CronScheduler` field | stored but never read (helpers ignored it via `_scheduler`) | Removed; `cron_should_run_for`/`cron_should_run_target` are stateless over the schedule string + `last_scan` |
- Removed duplicate implementations:
  - `ScanQueue` (vs `TaskScheduler` superset, see above).
  - `eggsec-output::schedule::RateLimiter` (vs `utils::rate_limiter::RateLimiter`; burst semantics now single-sourced: token bucket burst = 1s).
  - `utils::service_detection` copy owned by scanner (single owner now).
  - `ClientPool` / `OptimizedClientPool` (see below).
  - `fuzzer::rate_limit` (lock-free consecutive-error limiter + non-blocking token bucket) intentionally retained as operation-specific mapping per WS5 layering; reconsidered only in Phase D with consumer evidence.
- Reqwest client-pool disposition and measurements:
  - Removed `utils::client_pool::{ClientPool, OptimizedClientPool}` entirely.
  - Production consumers enumerated: only `utils::http` static pools (both `ClientPool::new(10, …)` with identical config) + bench. No consumer relied on isolated cookie/auth/pool state — all pooled clients shared timeout/UA/proxy, so no isolation property existed to preserve.
  - Replaced with single long-lived cloned clients (`SHARED_HTTP_CLIENT`, `SHARED_INSECURE_HTTP_CLIENT`) built with the same pool settings + `same_host_redirect_policy`. Each `reqwest::Client` already manages its own internal connection pool; the N-client round-robin sharded reusable connections/TLS state without benefit.
  - No benchmark-behavior evidence was required for retention because the identical-config inspection shows no property a shared client lacks; the bench now measures shared-client clone cost instead of pool construction. No new direct Reqwest consumers were introduced (acceptance criterion 9).
- Limiter semantic decisions (`utils::rate_limiter`):
  - `PerTargetRateLimiter` no longer awaits while holding the global map lock (acceptance criterion 4): map stores `Arc<Mutex<AdaptiveRateLimiter>>` per target; `limiter_for()` clones the `Arc` under a short lock, then awaits the per-target mutex. Added `test_per_target_isolation` + concurrent-targets test (run 3x).
  - Success does NOT immediately clear failure history: one failure halves the rate + arms 5s cooldown; one success does not restore it; 10 fast successes raise the rate and reset both counters (tested).
  - Burst is explicit: `RateLimiter` burst = `permits_per_second` (1s); `new(0)` clamps to 1 (never indefinite sleep); high-rate saturation tested (`u32::MAX`).
  - Cancellation: all `acquire` paths use bounded Tokio sleeps and are drop-cancellable; callers must race via `eggsec-runtime::race_with_cancel` / `tokio::select!`. Tested via `tokio::time::timeout` preemption for both `SharedRateLimiter` and `AdaptiveRateLimiter`.
  - DTO separation: `RateLimitStatus` conversion isolated in a `status_adapter` block; core `acquire`/`refill`/`record_response` never take DTO types.
  - `SharedRateLimiter::acquire` releases the inner lock while sleeping (previously held across sleep).
- Circuit-breaker semantic decisions (`utils::circuit_breaker`, engine-local):
  - Consecutive (not cumulative) failures while Closed: Closed-state success resets `failure_count` (tested, including the 2-fail/success/2-fail-stays-closed/3rd-fail-opens sequence).
  - Half-open probe concurrency limited to 1 via `AtomicBool` CAS: timeout expiry admits a single probe; concurrent `is_available()` calls while a probe is in flight are rejected (tested, including 10-task race admitting exactly 1).
  - Rejected calls are not counted (`total_calls`/`total_failures` only for admitted check-then-record calls; tested).
  - HalfOpen failure re-opens immediately and resets probe count; `success_threshold` successes close (tested, including repeated open->half-open->open cycles).
  - Time stays `std::time::Instant` only; tests use short real timeouts (20–40ms), no Tokio `test-util` (acceptance criterion 5).
- Architecture guard changes (`scripts/check-architecture-guards.sh`, Checks 113–117; `docs/CI_ARCHITECTURE_GUARDS.md` documents them):
  - 113: `eggsec-output` owns no scheduling/session (no `schedule.rs`/`session.rs`, no `pub mod`, no queue/session types; `eggsec-agent::cron` exists and is exported).
  - 114: service knowledge stays scanner-owned (no `utils/service_detection.rs`, no `use/mod …service_detection`; `scanner::service_data` exists).
  - 115: no Reqwest pool abstraction (no `utils/client_pool.rs`, no `ClientPool` types/imports).
  - 116: no second token bucket in output/frontend crates.
  - 117: no `eggsec-utils`/`common`/`shared`/`helpers` crate or catch-all module; removed `output`/`progress`/`stealth`/`privilege` utils do not reappear.
  - Checks match code structure (`struct`/`use`/`mod`/paths), not docs prose, so ownership notes in comments do not trip them.
- Commands/results (all green locally before commit):
  - `cargo fmt --all --check` — pass.
  - `cargo check --workspace --no-default-features`, `cargo check -p eggsec`, `cargo check -p eggsec-cli`, `cargo check -p eggsec-cli --no-default-features` — pass.
  - `make check-deps` (`cargo deny --workspace --all-features check` ×5) — pass after `cargo update -p rustls@0.23.43 --precise 0.23.45` (RUSTSEC-2026-0285, new since Phase F review) + `cargo update -p libssh2-sys` (yanked 0.3.2 → 0.3.3); `Cargo.lock` delta is limited to those two upgrades.
  - `make clippy` (engine empty + `cli` + 8 leaf crates) — pass, `-D warnings`.
  - `cargo test -p eggsec --doc` — 21 passed.
  - `cargo test -p eggsec --no-default-features --test tool_registration --test loadtest_tests` — 29 passed.
  - `cargo test -p eggsec --features rest-api,cli --tests --no-fail-fast` — 3005 passed (52 suites).
  - `cargo test -p eggsec-output --tests` — 82 passed; `cargo test -p eggsec-transport-eggfetch --tests` — 41 passed; `cargo test -p eggsec-tui --lib` — 874 passed; `cargo test -p eggsec-agent` — 30 passed.
  - Limiter (17) + circuit-breaker (11) tests each run 3x consecutively — stable.
  - `bash scripts/check-architecture-guards.sh` — ALL PASSED (Checks 99–117).
  - `make check-feature-profiles` — pass. `make check-features-individual` is deep-checks-only per `AGENTS.md` and was not run per-PR.
- Public API changes (all pre-1.0; no compatibility shims — shims would have inverted dependencies, e.g. making `eggsec-output` depend upward on `eggsec-agent`):
  - Removed `eggsec_output::{schedule, session}` modules and `CronExpression`, `CronScheduler`, `Priority`, `ScanOptions`, `ScanQueue`, `ScanType`, `ScanSession`, `SessionInfo` paths. Canonical cron path is `eggsec_agent::{CronExpression, CronScheduler}` (also re-exported as `eggsec::agent::{CronExpression, CronScheduler}`).
  - Removed `eggsec::output::schedule` / `eggsec::output::session` paths (were `pub use eggsec_output::*` re-exports).
  - Removed `eggsec::utils::{client_pool::{ClientPool, OptimizedClientPool}, output::{print_*}, progress::*, service_detection::*, stealth::*, privilege::*}`. Replacements: `eggsec::scanner::service_data::*`, `eggsec::platform::{is_root, check_privileged, require_root}`, `eggsec::utils::http::tool_user_agent()`, `eggsec::utils::{get_shared_http_client, get_shared_insecure_http_client}` (same signatures, single-client backend).
  - `RateLimiter::new(0)` now clamps to 1 (was indefinite sleep); `SharedRateLimiter` gained `try_acquire()`; `PerTargetRateLimiter::new(0)` clamps; `CircuitBreaker::new` clamps thresholds to ≥1 and Closed success resets failures (previously cumulative).
- Residual debt / Phase B blockers:
  - `fuzzer::rate_limit` vs `utils::rate_limiter` coexistence is intentional for now (operation-specific vs generic); Phase D owns the reuse decision with consumer evidence.
  - `utils::cache::ApiCache` has zero production consumers (found during inventory); left in place as engine infrastructure but flagged as a removal candidate for Phase D.
  - `eggsec-agent::cron::CronScheduler` aggregation (`should_run`/`next_run` over an expression list) is lightly used by the engine (which matches statelessly per portfolio string); Phase D may simplify it further.
  - `Cargo.lock` carries the rustls/libssh2-sys patch upgrades noted above; no `deny.toml` exception was added (both were fixable by upgrade, per `docs/DEPENDENCY_EXCEPTIONS.md` policy).
  - Phase B (report-model extraction) is unblocked: `eggsec-output` is now reports-only with no scheduling/session edges.
