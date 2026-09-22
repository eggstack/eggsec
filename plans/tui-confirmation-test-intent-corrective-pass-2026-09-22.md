# TUI confirmation-test intent corrective pass

Status: Executed

Date: 2026-09-22

Baseline: `625aa46bbd5f77065e29a8923c9b991c2fd8bb3e`

Parent cleanup:
[tui-broad-profile-warning-debt-cleanup-2026-09-22.md](tui-broad-profile-warning-debt-cleanup-2026-09-22.md)

## Purpose

Close the final test-contract mismatch left by the executed TUI warning-debt
cleanup without reopening that cleanup or changing runtime behavior.

The warning cleanup correctly removed an unused `try_approve` result from the
out-of-scope confirmation test. The remaining test name, doc comment, local
naming, and assertion message still describe the test as a
"preflight and execution" comparison even though the second path is now a
direct raw `EnforcementContext::evaluate` call.

This pass must make the test state exactly what it proves.

## Confirmed defect

At baseline `625aa46`,
`crates/eggsec-tui/src/app/enforcement_facade.rs` contains:

```rust
/// Preflight and execution agree: out-of-scope target → RequireConfirmation.
#[test]
fn preflight_and_execution_agree_on_confirmation_action() {
    // ...
    let preflight = facade.preflight(&desc);

    // Execution-path evaluation: the same `evaluate` the manual
    // approve path uses.
    let outcome = facade.state.enforcement.evaluate(&desc);

    // ...
    assert_eq!(
        preflight_needs_confirmation,
        exec_needs_confirmation,
        "preflight and execution must agree on confirmation requirement"
    );
}
```

The second branch does not call `try_approve`,
`evaluate_and_try_approve`, `approve_manual`, or any dispatch path. It is a
raw policy evaluation.

The test therefore overstates its coverage in four places:

1. doc comment says "preflight and execution";
2. function name says `preflight_and_execution...`;
3. local variable `exec_needs_confirmation` implies execution;
4. assertion text says "preflight and execution".

The implementation behavior is not defective; the defect is the test's
contract/name and the strength of what it asserts.

## Preferred correction

Keep this test as the targeted out-of-scope
`RequireConfirmation` mapping case and make its intent explicit.

Preferred shape:

- rename the test to something like
  `preflight_matches_raw_evaluation_for_confirmation_action` or an equally
  precise equivalent;
- change the doc comment to say preflight and raw enforcement evaluation agree
  for an out-of-scope target;
- rename `exec_needs_confirmation` to
  `raw_needs_confirmation` / `evaluated_needs_confirmation`;
- update the assertion message accordingly;
- strengthen the test so it cannot pass merely because both boolean probes are
  false.

The stronger assertion should prove both sides independently represent
`RequireConfirmation`, for example:

1. assert the preflight outcome is
   `TuiPreflightOutcomeKind::RequireConfirmation`;
2. assert the raw enforcement outcome is
   `EnforcementOutcome::RequireConfirmation(_)`;
3. optionally retain an explicit mapping/agreement assertion if useful.

The exact Rust formulation is implementation discretion; the semantic
requirement is that this test proves the expected confirmation classification,
not only equality of two booleans.

## Why not restore the removed `try_approve` call

Do not reintroduce a call solely to preserve the old "execution" wording.

`try_approve` performs more than classification:

- records `last_preflight`;
- derives confirmation classes;
- emits an audit event;
- delegates to `approve_manual` / `approve`.

The warning-cleanup pass intentionally removed an unused result whose value did
not contribute to this test's assertion. Restoring that side-effecting call
without asserting its result/state would recreate the original test smell.

Actual approval-path behavior already has nearby tests that call
`try_approve`, including:

- allowed action behavior;
- `last_preflight` population;
- guarded-mode denial;
- cached approval reuse/binding cases.

If implementation review finds a genuinely missing
`RequireConfirmation` approval-path invariant, add a separate narrowly named
test that asserts that invariant directly. Do not overload this raw-evaluation
mapping test.

## Scope

Expected implementation file:

```text
crates/eggsec-tui/src/app/enforcement_facade.rs
```

Expected planning-record updates after execution:

```text
plans/tui-confirmation-test-intent-corrective-pass-2026-09-22.md
plans/README.md
```

No production source should need to change.

## Non-goals

- no enforcement-policy changes;
- no changes to `try_approve`, `evaluate_and_try_approve`,
  `approve_manual`, or `preflight`;
- no approval-cache changes;
- no audit-event changes;
- no TUI behavior/layout changes;
- no feature changes;
- no warning suppression;
- no broader warning cleanup;
- no test deletion simply to remove the mismatch;
- no reopening of the completed Eggfetch or TUI warning campaigns.

## Focused validation

Run the renamed/reframed test directly:

```sh
cargo test -p eggsec-tui preflight_matches_raw_evaluation_for_confirmation_action --lib
```

If a different precise final test name is chosen, substitute that name.

Then run the surrounding enforcement-facade/TUI test surface:

```sh
cargo test -p eggsec-tui --lib --no-fail-fast
cargo test -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast
```

Preserve the warning-clean closure with:

```sh
RUSTFLAGS="-D warnings" cargo check -p eggsec-tui --features db-pentest,web-proxy,c2 --tests
```

Repository validation:

```sh
cargo fmt --all --check
make check-feature-profiles
make check
```

`make check-msrv` is recommended before closure if the implementation touches
anything beyond the test body/name. For the expected test-only wording/assertion
change, the already-green Rust 1.89 Deep Checks baseline remains applicable.

Hosted CI for the final implementation/record SHA must be green according to
the repository's normal path-filter behavior. A fresh manual Deep Checks run is
not required for a test-only contract rename/assertion strengthening unless
current repository policy requires it or implementation expands beyond this
scope.

## Acceptance criteria

This pass is complete only when:

1. no test/comment/local/assertion in this confirmation case calls the raw
   `evaluate` branch "execution";
2. the test name accurately describes preflight-vs-raw-evaluation comparison;
3. the test independently proves the expected
   `RequireConfirmation` classification on both sides rather than only
   comparing two booleans;
4. no redundant side-effecting `try_approve` call is reintroduced merely to
   satisfy naming;
5. production enforcement behavior is unchanged;
6. no public API or feature surface changes;
7. default and broad-profile TUI lib tests pass;
8. the broad TUI profile remains warning-clean under the existing
   warning-as-error proof;
9. `make check-feature-profiles` and `make check` are green;
10. hosted CI for the resulting code/record state is green;
11. this plan and `plans/README.md` record the executed result.

## Completion record template

Append after implementation:

```text
Status:
Starting SHA:
Implementation SHA:
Final record SHA:
Hosted CI:

Final test name:
Preflight assertion:
Raw-evaluation assertion:
Redundant try_approve reintroduced: no
Production source delta: none
Public API/feature delta: none

Focused test:
Default TUI lib tests:
Broad-profile TUI lib tests:
Warning-as-error broad-profile proof:
cargo fmt --all --check:
make check-feature-profiles:
make check:
make check-msrv:
Hosted Deep Checks:

Residual debt:
```

## Exit criterion

This line is closed when the out-of-scope confirmation test's name, comments,
locals, and assertions accurately describe and strongly prove its actual
preflight-vs-raw-evaluation contract, while the completed TUI warning cleanup
remains warning-clean and production enforcement behavior is unchanged.

## Completion record

```text
Status: Executed
Starting SHA: 4f1d375fd08e547c45ca105e94f7164ef82c6fd8
Implementation SHA: ead501aabbc8bb8644728e058debeadef56274d8
Final record SHA: e62644feb82158e13cb883c9d4bc9c6ffb23538c
  (record commit; this hosted-evidence amendment is a docs-only descendant
  with an identical executable tree)
Hosted CI: CI 35748922362 success + Code Quality 35748922260 success
  (head e62644f, push on main)

Final test name: preflight_matches_raw_evaluation_for_confirmation_action
Preflight assertion: asserts outcome_kind is
  TuiPreflightOutcomeKind::RequireConfirmation independently (not only a
  boolean probe compared against the other side)
Raw-evaluation assertion: asserts raw EnforcementContext::evaluate outcome is
  EnforcementOutcome::RequireConfirmation(_) independently
Redundant try_approve reintroduced: no
Production source delta: none
Public API/feature delta: none

Focused test: cargo test -p eggsec-tui
  preflight_matches_raw_evaluation_for_confirmation_action --lib
  → 1 passed
Default TUI lib tests: cargo test -p eggsec-tui --lib --no-fail-fast
  → 882 passed
Broad-profile TUI lib tests: cargo test -p eggsec-tui
  --features db-pentest,web-proxy,c2 --lib --no-fail-fast
  → 944 passed
Warning-as-error broad-profile proof: RUSTFLAGS="-D warnings" cargo check
  -p eggsec-tui --features db-pentest,web-proxy,c2 --tests
  → 0 errors (only third-party sqlx future-incompat notice remains)
cargo fmt --all --check: clean
make check-feature-profiles: green
make check: green
make check-msrv: not rerun; change is test-body/name plus a docs reference
  update only, so the already-green Rust 1.89 Deep Checks baseline remains
  applicable per the plan's focused-validation note
Hosted Deep Checks: not required (test-only contract rename/assertion
  strengthening; hosted CI for the resulting state follows the repository's
  normal path-filter behavior)

Residual debt: none. Repo-wide sweep confirms the only remaining
  `preflight_and_execution_agree_on_confirmation_action` mentions are the
  historical baseline quotes in this plan's "Confirmed defect" section and
  the parent warning-cleanup plan, both intentionally retained as record.
  README.md, AGENTS.md, skills, and architecture docs contain no stale
  references; docs/extending/tui-actions.md was updated to the new test
  name and contract.
```
