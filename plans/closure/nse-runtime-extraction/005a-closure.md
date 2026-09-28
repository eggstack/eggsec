# NSE Runtime Extraction Milestone 005A — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/005-provider-broker-foundation.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Repository baseline reviewed: Eggsec `7ec8ba73`; standalone `854f153f56d1abc929d9abd255f0606759342f81` (plan baseline).

Implementation commits or pull requests:

- Standalone `675269e` — "nse: add M005A provider broker foundation (clock/random/environment)"; branch `m005a-provider-broker-foundation` on `eggstack/eggsec-nse` (base `854f153`). Contains `src/providers.rs`, `NseRunRequest::with_host_services`, executor/service threading, migrated `datetime`/`rand`/`os`/`stdnse`/`nmap` paths, `tests/provider_broker_tests.rs`, boundary guards, `docs/PROVIDERS.md`, README provider section.
- No Eggsec production change (additive standalone surface; Eggsec keeps consuming crates.io `eggsec-nse 0.1.0` until a later release/adoption plan says otherwise).

## 1. Executive finding

Milestone 005A is complete. The standalone runtime exposes narrow clock/random/environment provider traits with native defaults, a cloneable per-run `NseHostServices` bundle, additive `with_host_services` injection through the canonical `execute_nse_run` pipeline, and capability-aware broker functions owning policy/preflight/provider/accounting/event sequencing. Representative `datetime`/`rand`/`os.getenv`/time/random paths in `stdnse`/`nmap` plus the default script-path environment lookup are provider-backed; existing callers without injection receive native behavior unchanged; deterministic injected-provider tests prove real NSE execution uses injected values; concurrent runs isolate provider state; static guards prevent new direct bypasses in migrated modules.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Narrow clock/random/environment traits with native defaults | `src/providers.rs`: `NseClockProvider`, `NseRandomProvider` (fill_bytes + scalar helpers), `NseEnvironmentProvider` (var + temp_dir); `Native*` impls | pass | No monolithic host trait (guard enforces). |
| Per-run service bundle, cloneable, no global mutable provider state | `NseHostServices` (Arc-backed, `native()`/`new`/`with_*`, `Debug` without leaking providers); bundle-clone unit test | pass | Concurrent tests hold different bundles. |
| Additive `NseRunRequest` injection, existing callers source-compatible | `with_host_services` + `effective_host_services`; `execute_nse_run` threads at construction; native-default integration test | pass | No constructor breakage. |
| Services reach canonical executor/library path | `ExecutorCore::with_full_policy_and_services`/`with_profile_and_services`, `NseExecutor`/`AsyncNseExecutor` counterparts + `host_services()` accessors; `register_libraries` uses `*_with_services` for migrated libs; old `register_*` shims delegate with native | pass | No secondary provider globals. |
| Broker owns policy/preflight/provider/post-operation sequencing | `broker_unix_timestamp`, `broker_random_fill`/`_f64`/`_u32`, `broker_env_var`, `broker_temp_dir` in `providers.rs` (check_cancelled → check_capability → before_blocking → provider → after_blocking) | pass | Denial/cancellation never reaches provider (counting-provider tests). |
| Denied operations do not call providers | Unit: CiSafe random/env, AgentSafe env, cancellation-before-clock (all assert `calls()==0`); integration: CiSafe rand denial + broker env denial | pass | — |
| Representative datetime/rand/environment migration | `datetime` (now/current_time/timestamp/isotime), `rand` (all fns), `os.getenv` (capability-gated, `""` fail-closed) + `os` clock/date/time/tmpdir, `stdnse` (clock/get_time/clock_ms/clock_us/time/random_string/urandom), `nmap` Lua-visible current_time/get_random_bytes/get_random/clock/clock_ms, `add_default_scripts_path_with_services` (HOME/ProgramFiles) | pass | Sleep/usleep intentionally native (chunked, cancellable); residual inventory in `docs/PROVIDERS.md`. |
| Deterministic injected-provider tests | `tests/provider_broker_tests.rs` (8 tests): fixed clock, deterministic rand replay, map env, native default, CiSafe rand/env denial, concurrent isolation, bundle cloning | pass | — |
| Concurrent per-run isolation | `concurrent_runs_isolate_provider_state` (threads with 1111111111 vs 2222222222) + unit bundle-clone test | pass | No process-global provider state. |
| Static guards prevent new direct bypasses | `scripts/check-boundaries.sh` M005A section (datetime/rand zero-tolerance; no `std::env::var` in os/executor_core; no `rand::random`/`thread_rng` in stdnse/nmap; clock only in broker fallbacks + nmap connection-metadata residual; no monolithic trait; broker presence per migrated module) | pass | Guard passes; fails loudly on new bypass. |
| Corpus/profile/report compatibility | `cargo test --features nse`: 573 passed, 1 ignored (baseline 558 + 15 new); report serialization unchanged; TimeClock allowed in all profiles so clock migration is behavior-preserving | pass | — |
| No Eggsec dependency in standalone | `./scripts/check-boundaries.sh` passes (Eggsec crate/import check) | pass | — |
| Eggsec consumer compatibility | `cargo check -p eggsec --features nse,cli` ok; `cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests` 202 passed (against published 0.1.0); patch-config check of Eggsec against modified standalone ok (disposable, lockfile restored) | pass | Main dependency unchanged; release/adoption deferred. |
| MSRV/package | `cargo +1.89.0 check --locked --features nse` ok; `cargo package` ok | pass | — |
| GO/NO-GO for 005B and 005D | GO for both (see §11) | pass | — |

## 3. Production implementation evidence

Standalone (`eggstack/eggsec-nse`, commit `675269e`):

- New `src/providers.rs` (~700 lines incl. tests): error type, 3 narrow traits, 3 native providers, `NseHostServices` bundle, 6 deterministic/counting test providers, 6 broker functions.
- `src/lib.rs`: `providers` module + public re-exports.
- `src/run.rs`: `host_services` field, `with_host_services`, `effective_host_services`, construction-time threading.
- `src/executor_core.rs`: `host_services` field, `with_full_policy_and_services`, `with_profile_and_services`, `host_services()` accessor, provider-aware `add_default_scripts_path_with_services`, migrated library registration.
- `src/executor.rs` / `src/async_executor.rs`: `with_full_policy_and_services`, `with_profile_and_services`, `host_services()` accessor, provider-aware script-path helper (sync).
- Migrated libraries with backward-compatible shims: `datetime` (3 fns brokered), `rand` (7 fns brokered), `os` (getenv capability-gated + clock/date/time/tmpdir provider-backed), `stdnse` (7 representative paths), `nmap` (Lua-visible time/random; internal connection timestamps inventoried as residual).
- New `tests/provider_broker_tests.rs` (8 integration tests).
- `scripts/check-boundaries.sh`: M005A guard section.
- New `docs/PROVIDERS.md` (contract + residual inventory); README provider section.

Eggsec: no production change. Planning/closure/registry/roadmap updates accompany this record.

## 4. Verification executed

### Commands run

```bash
# Standalone at 675269e
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --features nse
cargo package
cargo test --features nse --test provider_broker_tests
cargo test --features nse --lib providers

# Eggsec consumer (published 0.1.0, main dependency unchanged)
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests
# Disposable compatibility probe (not committed; lockfile restored afterwards):
cargo check -p eggsec --features nse,cli --config 'patch.crates-io.eggsec-nse.path="/tmp/opencode/eggsec-nse"'
```

### Results

- Standalone: fmt ok; boundaries ok (incl. new M005A guards); no-default + nse + nse-ssh2 + nse,sandbox checks ok (0 errors; warnings pre-existing); `cargo test --features nse` 573 passed, 1 ignored (23 suites); clippy ok (0 errors; pre-existing warnings incl. new-code-free); MSRV 1.89 ok; `cargo package` ok (uncommitted-changes note only, expected pre-commit).
- Focused: provider unit tests 7 passed; provider integration tests 8 passed.
- Eggsec: check ok; 202 NSE tests passed; patch-config probe against modified standalone ok (0 errors), proving additive compatibility. `Cargo.lock` restored after the probe; main still consumes crates.io 0.1.0.

## 5. Invariant review

- Existing `NseRunRequest::new` callers compile and use native providers automatically (native-default integration test + Eggsec patch probe).
- Eggsec authorization remains outside `eggsec-nse` (no Eggsec code touched; providers carry no scope/authority concepts; boundary script passes).
- `NseCapabilityContext` remains the runtime policy owner (brokers call `check_capability`/`check_cancelled`/`before|after_blocking_operation`; denial precedes provider).
- Provider traits contain no policy decisions or Eggsec scope concepts (mechanical DTOs only).
- Migrated production libraries call providers only through the broker (guards enforce `broker_` presence + direct-call bans).
- CiSafe denies randomness/environment before provider invocation (counting tests).
- AgentSafe/manual profile behavior semantically unchanged (TimeClock allowed everywhere; env/random denials match pre-existing capability policy; os.getenv denial maps to `""` preserving the Lua string contract).
- Cancellation and report/capability-event behavior compatible (broker preserves sequencing; capability events asserted in tests; full suite green).
- No `eggsec-*` dependency/import (boundary script).
- `NseRunReport` serialization unchanged (no report schema edits).
- Bundle is composition, not a monolithic trait (guard bans `trait NseHostProvider|HostProvider`).
- Native providers own `std`/chrono/rand use; migrated libraries do not (guards).

## 6. Failure and recovery review

- Provider failure maps to the same error class as the native operation (broker wraps as denied/error string; Lua signatures preserved: clock fns fall back to native time on broker failure, rand fns surface Lua errors as before, getenv returns `""`).
- Cancellation checked before provider invocation (unit test proves zero provider calls when cancelled).
- Sleep remains native chunked/cancellable; no change to timeout behavior.
- No durable state/restart behavior introduced; per-run services are ephemeral.
- Concurrent runs isolate state (integration test); no process-global mutable provider object.
- Patch-config Eggsec probe mutated `Cargo.lock`; restored via `git checkout -- Cargo.lock`. Future cross-repo probes should use a disposable worktree/copy to avoid dirtying main.

## 7. Migration and compatibility review

- Additive only. `NseRunRequest::new`, `ExecutorCore::with_full_policy`/`with_profile`, `NseExecutor`/`AsyncNseExecutor` existing constructors, and all old `register_*` library functions remain valid (shims delegate with native services).
- No persistent storage, wire protocol, or report schema migration.
- Lua-visible behavior preserved except the intended isolation fix class: `os.getenv` now enforces capability policy (AgentSafe/CiSafe return `""` instead of leaking real env) — documented, tested, and consistent with the capability inventory's pre-existing deny policy that the old direct path bypassed.
- `os.clock`/`date`/`time` and `stdnse`/`nmap` clock derivations may differ at sub-second granularity under fixed clocks by design (seconds-precision broker timestamp vs native millis/micros scaling); native defaults preserve prior behavior exactly.
- No release/publication from this slice; Eggsec main stays on crates.io 0.1.0.

## 8. Security review

- Broker denies via `NseCapabilityContext` before provider invocation (tested for CiSafe random/env, AgentSafe env, cancellation).
- Provider injection grants no authorization; availability never overrides profile policy.
- `os.getenv` fail-closed to `""` on denial (no env leakage to denied profiles).
- No secrets handled; provider errors carry no secret content.
- Sandbox path/command/network semantics untouched; filesystem/process migration deferred to 005D with current paths unchanged.
- No new privilege boundary; no transport/egress change.

## 9. Documentation and operations

- Standalone: new `docs/PROVIDERS.md` (contract, injection example, deterministic providers, residual inventory, guards); README provider section; boundary-script M005A comments; rustdoc on all public provider/broker APIs.
- Eggsec: this closure record; plan status `implemented`; registry + subsystem roadmap updates (005A closed; 005B/005D ready).
- Operator impact: none (no config/CLI/TUI/Python change; no new required construction).

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | `cargo package` notes uncommitted changes at verification time (pre-commit working tree). | None on behavior; package itself succeeded. | None; commit `675269e` is the implementation commit of record. |
| low | Patch-config Eggsec probe dirtied `Cargo.lock` (restored). | None (restored, verified clean). | Future probes use disposable worktree/copy. |
| info | Pre-existing clippy/compile warnings unchanged (96 lib warnings incl. OpenSSL deprecation, unreachable pattern, dead fields). | None on this slice. | Standalone maintenance, not M005A scope. |

No correctness or security findings remain for 005A.

## 11. Roadmap disposition

Milestone 005A closed; dependencies may proceed:

- **005B (authority-preserving network/DNS): GO** — per-run bundle, broker sequencing, and guard mechanics are stable; 005B extends the same bundle with DNS/TCP/UDP contracts per its plan. Status → ready for handoff.
- **005D (filesystem/process/portability): GO** — same foundation; may proceed in parallel with 005B per the roadmap. Status → ready for handoff.
- 005C remains blocked on 005B; 005E remains blocked on 005B+005C+005D. No corrective plan required for 005A.

## 12. Registry updates

- Mark `005-provider-broker-foundation.md` implemented; link this closure.
- Mark 005A closed in the subsystem roadmap (§7, §12 table).
- Move 005B and 005D from blocked to ready for handoff (hard dependency on 005A now closed).
- Keep 005C blocked on 005B; 005E blocked on 005B/005C/005D.


## Post-closure operational finding — M005 landing/CI corrective pass

Subsequent repository-level review found that this implementation/qualification evidence was accepted while the standalone changes remained on the stacked feature branches rather than `eggstack/eggsec-nse/main`. The relevant branch evidence for this slice is `m005a-provider-broker-foundation @ 675269e4a26314019ecc1c87a2bf9081887c1da0`. Standalone `main` is still `854f153f56d1abc929d9abd255f0606759342f81`, so the provider implementation described above is not yet canonical repository state.

The hosted GitHub Actions runs for the M005 stack are also red on Linux/macOS because `scripts/check-boundaries.sh` invokes `rg` but the workflow does not provision ripgrep. The latest cumulative 005E run `36476950685` passes MSRV, SSH runtime, and Windows but fails the Linux/macOS Rust jobs at the missing-`rg` boundary-check prerequisite.

This does not invalidate the local/focused implementation evidence recorded above, but it invalidates unconditional operational closure. The controlling corrective plan is `plans/implementation/nse-runtime-extraction/005-provider-stack-landing-ci-corrective.md`. This closure returns to `closed` only after the corrected cumulative stack has fully green branch CI, is landed on standalone `main`, and the merged main SHA has fully green post-merge CI with a corrective addendum tying this record to that SHA.

## Corrective addendum — landing/CI evidence recorded

The M005 landing/CI corrective pass is closed; see `plans/closure/nse-runtime-extraction/005-provider-stack-landing-ci-corrective-closure.md` for the full corrective evidence. The operational conditions recorded above are now satisfied:

- Original implementation SHA: `675269e4a26314019ecc1c87a2bf9081887c1da0` (`m005a-provider-broker-foundation`).
- Pre-corrective standalone main SHA: `854f153f56d1abc929d9abd255f0606759342f81`.
- Corrected cumulative stack-tip SHA: `1134c289b71a07fda21a8554782fd8101df5396f`.
- Merged standalone main SHA: `1134c289b71a07fda21a8554782fd8101df5396f` (fast-forward; no squash, no merge commit).
- `git log --pretty=format:%h 854f153..1134c28` enumerates `675269e`, `0ac9737`, `b3c43b8`, `89290f9`, `c81d84c`, and `1134c28` in the required linear order — this slice's `675269e` is an ancestor of `main` and the merged main tree contains the full provider broker foundation.
- Corrective CI/tooling change (one commit `1134c28`) repairs the boundary-check `rg` prerequisite (fail-fast diagnostic + focused regression test) and provisions ripgrep on Linux/macOS CI jobs; the Windows compile-only qualification is preserved.
- Branch CI evidence: prior red runs (`36337158009`, `36455834771`, `36461074205`, `36466449173`/`36466452158`, `36476950685`) are superseded by the corrected `main` workflow; the same matrix on `origin/main@1134c28` is green (script fail-fast path proven by `tests/boundary_check_tooling_tests.rs`, full-suite `cargo test --features nse` 639 passed + 1 ignored across 28 suites, M005 provider suites green).
- Post-merge main CI evidence: identical to the branch evidence because the corrected stack-tip SHA equals the merged main SHA (`git rev-list --count main..1134c28` is `0`).
- The boundary-check guard inventory (M005A clock/random/environment broker presence, monolithic-trait ban, broker sequencing) continues to enforce the 005A contract on the landed tree.
- Eggsec dependency remains crates.io `eggsec-nse 0.1.0`; the staged Eggsec HTTP adapter branch is not advanced by this corrective pass.
- The 0.2.0 release/adoption follow-up is now dependency-ready; this slice's provider surface (clock/random/environment traits + `NseHostServices` bundle + broker functions + native defaults) is the foundation published by that release, and its source-audit invariants travel with it.
