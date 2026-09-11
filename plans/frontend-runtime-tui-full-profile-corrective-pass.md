# Frontend/runtime TUI full-profile corrective pass

Status: Ready for handoff

Date: 2026-09-10

Baseline: `aba8e29625319c9b3c70b4324210cd68623d33d8`

## Purpose

Close the one material verification gap left after the frontend/runtime convergence roadmap: `eggsec-tui` advertises a `full` feature aggregate as maximum TUI capability, but that aggregate does not compile, and the current exhaustive feature sweep does not exercise TUI feature profiles.

This is a bounded corrective pass. Do not reopen the completed dispatch, approval-binding, TUI surface-model, or runtime-contract architecture work unless implementation proves a regression in those invariants.

## Confirmed current state

At the baseline SHA:

- The frontend/runtime Phase 0-3 implementation is present and normal hosted CI is green.
- `make check` covers the default TUI library tests but does not compile the TUI `full` aggregate.
- `make check-features-individual` exhaustively iterates engine features plus domain/daemon/CLI/Python profiles, but has no `eggsec-tui` feature sweep.
- `.github/workflows/deep-checks.yml` runs that same feature sweep, so scheduled deep checks cannot detect TUI-only feature-combination compile regressions.
- `crates/eggsec-tui/Cargo.toml` describes `full` as enabling maximum capability / every tab and control.
- The Phase 2 and Phase 3 completion records both document a pre-existing E0063 failure in `crates/eggsec-tui/src/app/task_management.rs` when broad/full TUI feature sets are enabled.

The stale constructors are currently:

- `DbPentestParams { db_type, target, port }`, while the runtime DTO also requires `checks`, `max_queries`, `max_duration`, `dry_run`, and `allow_advanced`.
- `InterceptParams { ... }`, while the runtime DTO also exposes `listen_host`, `dry_run`, and `max_flows` in addition to the existing fields. Audit the current constructor against the complete runtime DTO rather than assuming only the compiler-reported field list.
- `C2Params { target, profile }`, while the runtime DTO also requires `dry_run`.

All of those added runtime fields are optional/defaultable today. The corrective implementation must still map real TUI state where it exists; it must not mask semantic drift with a blanket `..Default::default()` unless that is demonstrably the intended boundary contract.

## Objectives

1. Make `cargo check -p eggsec-tui --features full` pass on a correctly provisioned supported host.
2. Ensure each TUI feature declaration is exercised by an automated compile sweep, not just engine features.
3. Add a representative broad TUI profile to routine verification that does not depend on privileged hardware or unavailable system services.
4. Preserve the canonical request, approval, dispatch, cancellation, and TUI surface-model ownership established by the completed frontend/runtime convergence roadmap.
5. Record enough closure evidence that another DTO/feature drift regression is caught automatically.

## Non-goals

- No new offensive capability.
- No visual TUI redesign.
- No command, operation-ID, alias, or policy model redesign.
- No replacement of `TaskKind` or the runtime wire contract.
- No broad decomposition of existing TUI hotspot files.
- No attempt to make every mathematically possible Cargo feature combination a supported product configuration.
- No branch-protection or repository-administration changes in this code pass.

## Workstream 1 — Repair feature-gated TUI runtime request builders

### 1.1 Database pentest builder

Update the `#[cfg(feature = "db-pentest")]` `TaskBuilder` path in `crates/eggsec-tui/src/app/task_management.rs` so `DbPentestParams` is constructed against the current runtime DTO.

For each of:

- `checks`
- `max_queries`
- `max_duration`
- `dry_run`
- `allow_advanced`

apply this decision rule:

1. If the corresponding value already exists in `DbPentestTab` state/configuration, map it explicitly.
2. If the TUI does not expose that control, use the canonical absence/default semantics expected by the runtime adapter (`None` unless the canonical request conversion documents another value).
3. Do not invent a permissive value merely to satisfy compilation.

Add a focused test that builds a representative DB pentest `RunRequest` and verifies operation identity, target, and any TUI-controlled safety fields survive the TUI -> runtime DTO -> canonical request conversion.

### 1.2 Intercept builder

Audit the complete current `InterceptParams` definition against the `InterceptTab` builder.

Explicitly account for:

- `listen_port`
- `target`
- `listen_host`
- `dry_run`
- `max_flows`

Prefer extracting host/port once from the TUI listen address rather than maintaining inconsistent parsing for separate fields. If `dry_run` or flow limits are represented in TUI state, preserve them; otherwise use the runtime's canonical optional semantics.

Add a focused builder/conversion test. In particular, verify that fixing compilation does not alter the approved target/listen semantics or accidentally turn an absent safety limit into an enabled/unbounded mode.

### 1.3 C2 builder

Update the feature-gated C2 builder for the current `C2Params` DTO and explicitly account for `dry_run`.

If the TUI has a dry-run/simulation control, wire it. If it does not, preserve the canonical runtime absence/default behavior and document that choice in the test.

Add a focused request/conversion test for the enabled C2 feature profile.

### 1.4 Builder drift guard

Do not introduce a second metadata table solely to track DTO fields. Rust struct construction should remain the primary compile-time guard against field additions.

Where builders intentionally leave optional fields unset, prefer explicit field initialization when it conveys security semantics. Use `..Default::default()` only for DTOs where omitted fields are genuinely non-semantic convenience defaults and a test pins the intended behavior.

## Workstream 2 — Establish TUI feature-profile verification

### 2.1 Add TUI coverage to `scripts/check-features-individual.sh`

Add an `eggsec-tui` section that mechanically reads `crates/eggsec-tui/Cargo.toml` and compiles every declared feature other than `default` and `full` at its minimum activation set.

Requirements:

- Every declared TUI feature must be covered automatically; adding a new TUI feature cannot silently escape the sweep.
- Respect feature dependencies already expressed in Cargo (for example, `wireless-advanced` includes `wireless`) rather than duplicating them in shell metadata unless an external prerequisite requires special handling.
- System-prerequisite skips must be explicit and named, using the same PASS/FAIL/SKIP discipline as the existing engine sweep.
- A Rust compile failure is always FAIL, never SKIP.
- Compile `eggsec-tui --features full` as a separate aggregate check after the individual TUI feature sweep.

If the TUI `full` profile inherits a real system prerequisite from engine features, provision it in Deep Checks or gate it with the existing named-prerequisite mechanism. Do not classify source compilation errors as prerequisite skips.

### 2.2 Add a routine representative broad profile

Extend `make check-feature-profiles` with a broad TUI profile that exercises the feature-gated builders responsible for this regression, at minimum:

- `db-pentest`
- `web-proxy`
- `c2`

Include other dependency-light TUI features where useful to increase interaction coverage, but keep this routine profile deterministic and suitable for normal developer/CI hosts.

The purpose is fast detection of cross-feature TUI compile drift between weekly deep sweeps. It is not a substitute for the `full` aggregate in Deep Checks.

### 2.3 Preserve the deep-check oracle

`.github/workflows/deep-checks.yml` already invokes `make check-features-individual`; once the script covers TUI profiles, that workflow becomes the exhaustive TUI feature compile oracle as well.

Verify the Deep Checks runner installs every native prerequisite required to compile the advertised TUI `full` aggregate. Add only the missing build prerequisites, not runtime/hardware dependencies.

### 2.4 Optional routine `full` gate

Only add `cargo check -p eggsec-tui --features full` to the normal `make check`/push CI path if it can compile without materially increasing native/system setup or making ordinary CI platform-sensitive.

Otherwise keep `full` in Deep Checks and keep the broad dependency-light profile in `make check-feature-profiles`. Document this split clearly. Do not force system-heavy dependencies into every push solely for symmetry.

## Workstream 3 — Pin semantic parity, not merely compilation

The original failure is a useful signal that TUI builders can lag runtime DTO evolution. Add tests around the repaired builders so future changes fail for semantic drift as well as missing struct fields.

For DB pentest, intercept, and C2 representative requests, assert as applicable:

- expected `TaskKind` variant;
- canonical operation ID;
- target/listen identity;
- runtime surface remains `TuiManual`/the expected strict variant;
- canonical conversion succeeds;
- policy-relevant/safety fields exposed by the TUI are preserved;
- fields not exposed by the TUI resolve to the documented safe/canonical absence behavior.

Reuse existing `CanonicalOperationRequest::from_task_kind`, descriptor/binding helpers, and surface wiring tests. Do not create TUI-local canonicalization rules.

## Workstream 4 — Verification contract and guard updates

Update the verification documentation and architecture guards only where they encode a durable structural invariant.

Good static invariants include:

- the individual-feature sweep contains a TUI feature enumeration section;
- the TUI `full` aggregate is included in the deep feature sweep;
- the broad TUI profile remains present in `make check-feature-profiles`.

Do not grep for individual DTO field names as a long-term architecture guard; Rust compilation and semantic tests are the correct owners for that contract.

Update the relevant maintainer/agent documentation so feature changes to `eggsec-tui/Cargo.toml` require running the TUI feature sweep.

## Required verification

Run and record exact results for:

```text
cargo fmt --all -- --check
cargo check -p eggsec-tui --features db-pentest,web-proxy,c2
cargo test -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast
cargo check -p eggsec-tui --features full
make check-feature-profiles
make check-features-individual
make test-architecture-guards
make check
make check-python
```

Also run any focused semantic tests added for DB pentest, intercept, and C2 request construction/conversion.

If `full` needs native packages, execute that check in the same provisioned environment used by Deep Checks and record the prerequisite. A missing prerequisite may explain an environment failure; it may not be used to waive a Rust E0063/E0308/etc. source failure.

## Acceptance criteria

This corrective pass is complete only when all of the following are true:

1. `cargo check -p eggsec-tui --features full` succeeds on a provisioned supported host.
2. DB pentest, intercept, and C2 TUI builders compile against the current runtime DTOs and have semantic conversion tests.
3. No permissive/safety-relevant runtime value was invented merely to repair compilation.
4. `scripts/check-features-individual.sh` covers every declared `eggsec-tui` feature plus `full`.
5. A new TUI feature added to Cargo cannot silently escape the individual-feature sweep.
6. `make check-feature-profiles` contains a routine broad TUI profile covering at least `db-pentest,web-proxy,c2`.
7. Deep Checks provisions the build prerequisites necessary for the TUI `full` aggregate and runs the expanded sweep.
8. Existing frontend/runtime convergence tests remain green: approval binding, canonical dispatch ownership, TUI surface parity, runtime contract closure, and cancellation/lifecycle tests.
9. No duplicate operation/target/alias/executor mapping is introduced.
10. The completion record identifies whether `full` is a routine push-CI gate or a Deep Checks gate and why.

## Suggested implementation order

1. Reproduce the broad/full TUI compile failure at the baseline SHA and capture the exact compiler diagnostics.
2. Repair the three stale builders with explicit semantic mappings.
3. Add focused builder/canonical-conversion tests.
4. Verify the dependency-light `db-pentest,web-proxy,c2` profile.
5. Verify `eggsec-tui --features full` in a correctly provisioned environment.
6. Extend `scripts/check-features-individual.sh` to enumerate TUI features and compile `full`.
7. Add the routine broad TUI profile to `make check-feature-profiles`.
8. Reconcile Deep Checks prerequisites.
9. Run the full required verification set.
10. Append a completion record to this plan with baseline/final SHAs, exact commands/results, feature counts, any prerequisite SKIPs, and residual debt.

## Expected files touched

Likely implementation files:

- `crates/eggsec-tui/src/app/task_management.rs`
- TUI semantic test modules adjacent to task/surface wiring
- `scripts/check-features-individual.sh`
- `Makefile`
- `.github/workflows/deep-checks.yml` only if build prerequisites are missing
- `scripts/check-architecture-guards.sh` only for durable sweep/gate invariants
- relevant architecture/agent documentation
- this plan's completion record

Runtime DTO definitions should not need to change merely to make stale TUI constructors compile. If implementation appears to require changing `DbPentestParams`, `InterceptParams`, or `C2Params`, first prove why the current runtime contract is wrong rather than adapting the canonical contract to a lagging frontend.

## Closure record requirements

When executed, append:

- baseline SHA and final SHA;
- compiler errors reproduced before the fix;
- fields mapped for each repaired builder and why;
- TUI feature count swept individually;
- `full` aggregate result;
- broad routine profile result;
- full command/test results;
- hosted CI/Deep Checks evidence if available;
- any intentional prerequisite SKIPs;
- remaining debt with an owner/removal criterion.

Do not mark this pass complete solely because the three constructors compile. The missing automated TUI feature coverage is part of the defect and must be closed in the same pass.
