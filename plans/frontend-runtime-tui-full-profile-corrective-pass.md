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

---

## Completion record

Status: Executed

Date: 2026-09-11

Baseline SHA: `92d3577c1ebd4e48c590c5b4ec839f1911324d5c`
Final SHA: (recorded at commit time; see `git log` for the `fix(tui)` commit on `main`)

### Compiler errors reproduced before the fix

`cargo check -p eggsec-tui --features full` at baseline failed with three E0063 errors in `crates/eggsec-tui/src/app/task_management.rs`:

```text
error[E0063]: missing fields `dry_run`, `listen_host` and `max_flows` in initializer of `InterceptParams`
   --> crates/eggsec-tui/src/app/task_management.rs:542:44
error[E0063]: missing field `dry_run` in initializer of `C2Params`
   --> crates/eggsec-tui/src/app/task_management.rs:568:37
error[E0063]: missing fields `allow_advanced`, `checks`, `dry_run` and 2 other fields in initializer of `DbPentestParams`
   --> crates/eggsec-tui/src/app/task_management.rs:600:44
```

### Fields mapped for each repaired builder and why

**DbPentest (`crates/eggsec-tui/src/app/task_management.rs`, `DbPentestTab` impl):**
- `target` ← input[1] (Target field), unchanged.
- `db_type` ← inferred from the connection-string scheme via new `detect_db_type_from_target()` (`postgres|postgresql→postgres`, `mysql→mysql`, `mongodb|mongo→mongodb`, `mssql|sqlserver→mssql`, `redis|rediss→redis`). The tab exposes no db-type control; the old code read input[2] (the Checks field, default `"all"`) as `db_type`, which would fail `DbPentestRequest::normalize()` (it requires a concrete `postgres|mysql|mssql|mongodb|redis`). Unknown schemes return `None` from the builder rather than fabricating a permissive value.
- `port` ← `None` (no port control in the tab).
- `checks` ← input[2], `None` when blank (engine normalizes absent to `"all"`).
- `max_queries` ← input[3] parsed as `u64`, `None` when blank/unparseable (engine defaults to 200).
- `max_duration` ← input[4] parsed as `u64`, `None` when blank/unparseable (engine defaults to 120).
- `dry_run` ← `Some(self.dry_run)` (tab toggle, default `true`).
- `allow_advanced` ← `Some(self.advanced)` (tab toggle, default `false`).

**Intercept (`InterceptTab` impl):**
- `listen_host` + `listen_port` ← split once from `listen_addr()` via new `parse_listen_addr()` (typed `(Option<String>, Option<u16>)`, no silent coercion of missing pieces). Previously only `listen_port` was extracted with ad-hoc inline parsing.
- `target` ← `self.primary_target()`, unchanged (falls back to `listen_addr` when no session is attached — documented empty-session semantic, not a new default).
- `dry_run` ← `Some(self.dry_run)` (tab field, default `true`).
- `max_flows` ← `Some(self.max_flows())` (tab field, default `100`).

**C2 (`C2Tab` impl):**
- `target`, `profile` ← unchanged (target + campaign inputs).
- `dry_run` ← `None` (the tab exposes no dry-run control; absence lets the canonical executor apply its documented safe default `dry_run.unwrap_or(true)`). Documented in code and test.

No permissive/safety-relevant runtime value was invented to satisfy compilation.

### Bonus fix found by the new sweep

The new TUI sweep caught a **pre-existing** compile failure unrelated to the three reported builders: `cargo check -p eggsec-tui --features vuln-management` failed with `E0433: cannot find type AppState` in `crates/eggsec-tui/src/app/state_update.rs` because the `use crate::tabs::AppState` import gate listed `database`, `external-integrations`, and `finding-workflow` but omitted `vuln-management` (which also uses `AppState::Completed`). Fixed by adding `vuln-management` to the `#[cfg(any(...))]` gate. This validates the sweep's value: TUI-only feature combinations previously had zero automated coverage.

### TUI feature count swept individually

18 declared TUI features (every entry in `crates/eggsec-tui/Cargo.toml` other than `default`/`full`), each compiled at its minimum activation set, plus the `full` aggregate — 19 TUI profiles total:

`advanced-hunting`, `c2`, `compliance`, `database`, `db-pentest`, `external-integrations`, `finding-workflow`, `headless-browser`, `mobile`, `nse`, `packet-inspection`, `rest-api`, `stress-testing`, `tool-api`, `vuln-management`, `web-proxy`, `wireless`, `wireless-advanced`, + `full`.

All 18 individual TUI profiles PASS. The orphan guard (mechanical `Cargo.toml` enumeration compared against the swept list) ensures a newly added TUI feature cannot silently escape the sweep.

### `full` aggregate result

`cargo check -p eggsec-tui --features full` — **PASS** (0 errors). Verified to need no system prerequisites beyond the Rust toolchain on a supported host: `pnet` is pure-Rust (no libpcap link at compile time), `openssl` is vendored via `eggsec-nse`, and `wireless-tools` is runtime-only. Deep Checks already installs `ripgrep protobuf-compiler libpcap-dev libssl-dev libssh2-dev pkg-config`, which is more than sufficient — **no `.github/workflows/deep-checks.yml` changes were required**.

**Gate decision:** `full` stays a **Deep Checks gate** (weekly/manual via `make check-features-individual`), not a routine push-CI gate, because it pulls the heaviest compile closure (nse vendored openssl, pnet, all domain tabs) and would materially slow every push. The broad dependency-light profile below is the routine fast gate.

### Broad routine profile result

`make check-feature-profiles` now includes:

```text
cargo check -p eggsec-tui --features db-pentest,web-proxy,c2
cargo test -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast
```

**PASS** — check clean (0 errors), 936 lib tests passed. This exercises exactly the three repaired builders plus adjacent tabs for fast cross-feature compile-drift detection between weekly deep sweeps.

### Full command/test results (exact, local)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo check -p eggsec-tui --features db-pentest,web-proxy,c2` | PASS (0 errors) |
| `cargo test -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast` | PASS (936 passed) |
| `cargo test -p eggsec-tui --lib --no-fail-fast` (default features) | PASS (874 passed) |
| `cargo test -p eggsec-tui --features db-pentest,web-proxy,c2 --lib app::task_management` | PASS (12 passed: listen-addr parsing, db/intercept/c2 builder semantics, canonical round trips, surface pinning, scheme detection) |
| `cargo check -p eggsec-tui --features full` | PASS (0 errors) |
| `make check-feature-profiles` | PASS (exit 0) |
| `make check-features-individual` | **PASS: 85, SKIP: 4, FAIL: 0** → `RESULT: OK` |
| `make test-architecture-guards` | PASS (all 98 checks, including new Check 98) |
| `make check` | PASS (exit 0) |
| `make check-python` | PASS (exit 0) |

Focused semantic tests added (`app::task_management::tests`): `parse_listen_addr_handles_typical_and_partial_inputs`, `db_pentest_builder_maps_ui_state_to_runtime_dto`, `db_pentest_builder_refuses_unknown_target_scheme`, `db_pentest_builder_omits_optional_inputs_when_blank_or_unparseable`, `db_pentest_builder_round_trips_through_canonical_conversion`, `intercept_builder_maps_ui_state_to_runtime_dto`, `intercept_builder_preserves_partial_listen_addr_components`, `intercept_builder_dry_run_default_is_preserved_as_some_true`, `c2_builder_maps_ui_state_to_runtime_dto`, `c2_builder_round_trips_through_canonical_conversion`, `builder_results_carry_tui_manual_surface`, `detect_db_type_from_target_recognizes_known_schemes`. These assert `TaskKind` variant, canonical operation ID (`db-pentest` / `proxy-intercept` / `c2`), target/listen identity, `TuiManual` surface, canonical-conversion success, TUI-controlled safety-field preservation, and documented absence behavior for unexposed fields. The pre-existing `c2.rs` builder tests (`test_build_run_request_*`) remain green.

Existing frontend/runtime convergence suites remain green via `make check` (approval binding, canonical dispatch ownership, TUI surface parity, runtime contract closure, cancellation/lifecycle). No duplicate operation/target/alias/executor mapping was introduced.

### Hosted CI/Deep Checks evidence

Local verification only at completion time; hosted `ci.yml` (push) and scheduled `deep-checks.yml` evidence to be recorded after push. Deep Checks needs no prerequisite changes (see above).

### Intentional prerequisite SKIPs (local host)

4 SKIPs, all pre-existing engine/domain gates (this host lacks `libpcap-dev` and `libssh2-dev`):

- `eggsec/nse-ssh2` (libssh2-dev)
- `eggsec/packet-inspection` (libpcap-dev)
- `eggsec/stress-testing` (libpcap-dev)
- `eggsec-nse/nse-ssh2` (libssh2-dev)

No TUI profile was skipped: TUI `stress-testing`/`packet-inspection` compile without libpcap (pure-Rust `pnet`), TUI `nse` uses vendored openssl, and TUI `full` was verified to build on this unprovisioned host — so the TUI sweep applies no SKIP gates. A Rust compile failure in any TUI profile is always FAIL, never SKIP.

### Remaining debt

None introduced by this pass. Pre-existing warnings in `full` builds (unused imports in feature-conditional tabs, deprecated `Table::highlight_style`, `sqlx-postgres` future-incompat) are untouched and out of scope. Promoting `eggsec-nse` from warn-only to `-D warnings` in `make clippy-domain` remains tracked future work (pre-existing, noted in the Makefile).

### Files touched

- `crates/eggsec-tui/src/app/task_management.rs` — repaired three builders, added `parse_listen_addr()` + `detect_db_type_from_target()` helpers and 12 semantic tests.
- `crates/eggsec-tui/src/app/state_update.rs` — added `vuln-management` to the `AppState` import gate (pre-existing bug found by the new sweep).
- `scripts/check-features-individual.sh` — new TUI enumeration section (mechanical `Cargo.toml` read), TUI orphan guard, `eggsec-tui/full` aggregate check.
- `Makefile` — broad TUI profile (`db-pentest,web-proxy,c2` check + lib tests) in `check-feature-profiles`.
- `scripts/check-architecture-guards.sh` — new Check 98 (TUI feature-profile verification: sweep section, mechanical enumeration, `full` aggregate, broad profile).
- `docs/CI_ARCHITECTURE_GUARDS.md` — broad-TUI profile row; sweep/TUI-profile currency bullets.
- `docs/VERIFICATION.md` — sweep/profile descriptions now name TUI coverage.
- `AGENTS.md` — TUI `full` vs broad-profile gate split; TUI-sweep requirement for `eggsec-tui/Cargo.toml` changes.
- `architecture/tui.md` — builder semantics, `parse_listen_addr`/`detect_db_type_from_target` contracts, feature-profile verification section.
- `.opencode/skills/eggsec-tui/SKILL.md` — builder rules, TUI profile commands, sweep/orphan-guard notes.
- This plan file — completion record.

Acceptance criteria 1–10 all satisfied.
