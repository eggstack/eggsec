# TUI broad-profile warning-debt cleanup

Status: Ready for handoff

Date: 2026-09-22

Executable baseline: `17cf8c3173eb49d21e0604e61f3ef60801da6367`

Planning/evidence baseline: `dd414fa3749f8f7c307a17b78ccf2b613bebb24c`

Evidence source:

- Deep Checks run `35698970796`, successful on
  `86d2670b7ce487dd1697ef120116bd811a9c5d40`;
- current HEAD after `86d2670b` changes only the Eggfetch plan record, so
  the warning-producing executable tree is unchanged.

## Purpose

Remove the known warning debt exposed by Eggsec's representative broad TUI
profile and the adjacent mobile feature profile without changing runtime
behavior, feature semantics, public APIs, or TUI layout.

This is a warning-hygiene pass, not a TUI redesign and not a workspace-wide
warning campaign.

The target is to make the affected profiles warning-clean and to leave a
repeatable proof that the warnings were removed rather than merely suppressed.

## Scope boundary

Primary scope:

```sh
cargo check -p eggsec-tui --features db-pentest,web-proxy,c2
cargo test -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast
```

Adjacent profile debt explicitly recorded during the Eggfetch 0.2.0
qualification is also in scope:

```sh
cargo check -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
```

Do not expand this pass to unrelated existing warning debt in:

- `eggsec-nse`;
- arbitrary `eggsec` integration-test helpers;
- other feature profiles unless a change in this pass directly causes the
  warning;
- dependency duplicate warnings from cargo-deny;
- unrelated clippy modernization.

If a targeted validation command exposes a warning outside these owned
surfaces, record it separately rather than silently widening this plan.

## Confirmed warning inventory

### Broad `eggsec-tui` profile

The successful Deep Checks run recorded 18 warnings for the broad TUI lib-test
profile:

1. unused import `SessionId` in
   `crates/eggsec-tui/src/app/task_runtime.rs`;
2. unused import `empty_state_paragraph` in
   `crates/eggsec-tui/src/tabs/intercept/mod.rs`;
3. unused imports `Cell`, `Clear`, `Row`, and `Table` in the same
   intercept module;
4. deprecated Ratatui `Table::highlight_style` use in
   `crates/eggsec-tui/src/tabs/intercept/render.rs`;
5. unused `exec_result` in
   `crates/eggsec-tui/src/app/enforcement_facade.rs`;
6. six obsolete `create_test_app()` locals in
   `crates/eggsec-tui/src/app/runtime_adapter/mod.rs`, with five associated
   unnecessary-`mut` warnings;
7. unused `dispatcher` in
   `crates/eggsec-tui/src/app/task_dispatcher.rs`.

The summary was:

```text
eggsec-tui (lib test) generated 18 warnings
```

Seventeen had mechanical compiler suggestions; the Ratatui deprecation
requires an API spelling update.

### Adjacent mobile feature profile

The same representative feature-profile run recorded two warnings in
`crates/eggsec/src/mobile/mod.rs` when the static `mobile` feature is built
without the CLI/dynamic paths that consume them:

- unused `crate::config::EggsecConfig`;
- unused `crate::error::{EggsecError, Result}`.

These imports are used only by feature-gated adapter entry points and should be
gated or localized to the functions that require them.

## Global constraints

Preserve:

1. all public Rust APIs;
2. CLI/TUI behavior and flags;
3. tab layout and rendering semantics;
4. runtime dispatch and task-result semantics;
5. enforcement/preflight semantics;
6. feature names and feature relationships;
7. mobile static/dynamic API behavior;
8. Ratatui selection/highlight appearance;
9. test intent and assertion coverage;
10. Rust 1.89 MSRV;
11. the repository's recent no-suppression direction.

Do not resolve warnings by adding broad:

- `#![allow(...)]`;
- module-wide `#[allow(unused)]`;
- `#[allow(deprecated)]`;
- dead `let _ = ...` suppression;
- meaningless underscore bindings solely to silence the compiler.

Where an item is genuinely unnecessary, remove it. Where a test setup value is
supposed to prove behavior, replace the dead binding with a meaningful
assertion rather than preserving decorative setup.

## Workstream 1 — Clean feature-gated engine imports

In `crates/eggsec/src/mobile/mod.rs`, make the adapter-only imports match the
functions that use them.

Preferred direction:

- gate or localize `EggsecConfig` to the union of the `cli` and
  `mobile-dynamic` entry points that require it;
- gate or localize `EggsecError` and `Result` to the same actual use sites;
- avoid changing public re-exports from `eggsec-mobile-lab`;
- do not change the `mobile` / `mobile-dynamic` feature contract.

Verify both static and dynamic profiles independently.

## Workstream 2 — Remove obsolete TUI test setup

### Runtime-adapter tests

In `crates/eggsec-tui/src/app/runtime_adapter/mod.rs`, the affected reducer
tests construct `create_test_app()` values that are never passed to the
reducer or inspected.

Remove only the obsolete locals/import if they are truly vestigial.

Do not remove the adapter state setup, task registration, event construction,
action assertions, or task-unregistration assertions that make these tests
meaningful.

After cleanup, if `create_test_app` is no longer used anywhere in that test
module, remove the now-unused import as well.

### Task-dispatcher constructor test

The current `task_dispatcher_creation` test constructs
`TuiTaskDispatcher` into an unused local and then performs a tautological
`TypeId` comparison unrelated to that value.

Replace this with a meaningful constructor invariant, preferably proving that
the created dispatcher retains the supplied `Arc<ArcSwap<TuiDispatcherContext>>`
or another direct property available to the private test module.

Do not merely rename `dispatcher` to `_dispatcher`.

## Workstream 3 — Clean test imports and enforcement test intent

### `task_runtime.rs`

Remove `SessionId` from the local test import if the test continues to use
the fully qualified `eggsec_runtime::SessionId` spelling, or consistently use
the imported spelling. Do not retain both.

### `enforcement_facade.rs`

The test
`preflight_and_execution_agree_on_confirmation_action` currently calls
`try_approve` and stores the result as `exec_result` without inspecting it.

Preserve the actual purpose of the test:

- if the call is required to exercise execution-path state, assert the
  semantically relevant result/state;
- if the call is redundant because the test only compares independent
  evaluation outcomes, remove the redundant call and update the explanatory
  comment;
- do not silence the result with `let _ =`.

The final test name/comments/assertions must agree with what it actually
proves.

## Workstream 4 — Clean intercept imports and Ratatui deprecation

In `crates/eggsec-tui/src/tabs/intercept/mod.rs`:

- remove or narrow imports that are unused in the broad profile;
- if an import is used only by tests or another feature shape, place it behind
  the narrowest truthful `cfg` or move it to its consumer module;
- do not remove functionality from the intercept tab to achieve a clean build.

In `crates/eggsec-tui/src/tabs/intercept/render.rs`:

- replace deprecated `Table::highlight_style(...)` with the Ratatui 0.30
  replacement `row_highlight_style(...)`;
- retain the existing selected-row style and highlight symbol;
- add or retain a rendering/state test if needed to prove selection behavior
  did not change.

## Workstream 5 — Warning-clean qualification

Run the exact affected profiles first:

```sh
cargo check -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
cargo check -p eggsec-tui --features db-pentest,web-proxy,c2
cargo test -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast
```

The command output must contain no warning attributable to the files/items in
this plan.

Then run warning-as-error proof for the owned TUI surface. Because the
workspace MSRV is 1.89, do not rely on newer Cargo-only warning-control
environment variables. Use an MSRV-compatible mechanism, for example a
temporary `RUSTFLAGS="-D warnings"` invocation for the targeted package/profile
or an equivalent rustc/clippy command that demonstrably compiles the same
owned code and test cfg.

Do not add crate-wide `#![deny(warnings)]` as part of this cleanup.

If a permanent low-cost gate is added, it must:

- cover the broad TUI profile without materially expanding routine CI cost;
- work on Rust 1.89;
- distinguish Eggsec/TUI warnings from third-party build output;
- not convert known unrelated NSE/workspace warning debt into a hidden scope
  expansion.

A permanent gate is optional; warning-free source is mandatory.

## Workstream 6 — Regression validation

After focused warning cleanup, run:

```sh
cargo fmt --all --check
cargo test -p eggsec-tui --lib --no-fail-fast
cargo test -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast
make check-feature-profiles
make check
make check-msrv
```

Because the intercept renderer is touched, also run the current TUI PTY/deep
smoke when available:

```sh
make test-tui-pty
```

Run `make check-full` before closure if host prerequisites are available.
Record any skip precisely.

Hosted CI for the implementation SHA must remain green. If the changes touch
only the TUI/profile source and tests but the repository's path filters do not
start Deep Checks automatically, run the current manual Deep Checks workflow
before marking the pass closed.

## Documentation and completion record

Update:

- this plan with exact warning counts before/after;
- `plans/README.md` to mark the pass executed;
- the Eggfetch 0.2.0 plan only if desired to append a short later-closure note
  for the residual TUI warning debt.

Do not rewrite the Eggfetch completion record's historical statement that the
warnings existed at adoption time. A later cross-reference is preferable to
altering history.

No architecture document change is required unless implementation reveals a
real behavior or feature-contract change, which should not occur in this pass.

## Expected files touched

Likely:

```text
crates/eggsec/src/mobile/mod.rs
crates/eggsec-tui/src/app/task_runtime.rs
crates/eggsec-tui/src/app/enforcement_facade.rs
crates/eggsec-tui/src/app/runtime_adapter/mod.rs
crates/eggsec-tui/src/app/task_dispatcher.rs
crates/eggsec-tui/src/tabs/intercept/mod.rs
crates/eggsec-tui/src/tabs/intercept/render.rs
plans/README.md
this plan
```

`Makefile` or a small verification script may change only if a durable,
MSRV-compatible warning gate is justified and remains cheap enough for the
intended validation tier.

## Non-goals

- no TUI layout redesign;
- no terminal lifecycle/logging redesign;
- no runtime-dispatch redesign;
- no enforcement-policy changes;
- no feature consolidation;
- no Ratatui upgrade;
- no dependency update;
- no mobile capability changes;
- no workspace-wide warning cleanup;
- no NSE warning cleanup;
- no broad lint-level escalation;
- no warning suppression in place of cleanup.

## Acceptance criteria

This pass is complete only when:

1. the broad TUI profile no longer emits the recorded 18 warnings;
2. the `mobile` / `mobile-dynamic` profiles no longer emit the two recorded
   `mobile/mod.rs` import warnings;
3. no `allow(unused)` or `allow(deprecated)` suppression is added to hide
   the debt;
4. obsolete runtime-adapter test setup is removed without reducing assertions;
5. the task-dispatcher constructor test uses the constructed value in a
   meaningful invariant;
6. the enforcement-facade test either uses its execution result meaningfully
   or removes the redundant execution call with corrected test intent;
7. Ratatui `row_highlight_style` replaces the deprecated API with identical
   selected-row behavior;
8. broad TUI and default TUI lib tests pass;
9. mobile static and dynamic feature checks pass;
10. `make check-feature-profiles` is green;
11. `make check` and `make check-msrv` are green;
12. TUI PTY/deep validation is green where required by current policy;
13. hosted CI for the final executable SHA is green;
14. any remaining warning debt is explicitly outside this plan's scope rather
   than silently suppressed.

## Completion record template

Append after implementation:

```text
Status:
Starting SHA:
Final implementation SHA:
Final record SHA:
Hosted CI:
Hosted Deep Checks:

Broad TUI warning count before: 18
Broad TUI warning count after:
Mobile-profile warning count before: 2
Mobile-profile warning count after:

mobile/mod.rs:
task_runtime.rs:
enforcement_facade.rs:
runtime_adapter/mod.rs:
task_dispatcher.rs:
intercept/mod.rs:
intercept/render.rs:
Permanent warning gate added:
Suppression attributes added: none

Focused mobile checks:
Focused TUI check:
Focused TUI lib tests:
Warning-as-error proof:
make check-feature-profiles:
make check:
make check-msrv:
make test-tui-pty:
make check-full:

Behavior/API delta:
Residual warning debt outside scope:
```

## Exit criterion

This line is closed when the representative broad TUI profile and the recorded
adjacent mobile profile are warning-clean, their tests and TUI rendering
semantics remain intact, and the repository has current local/hosted evidence
without using warning suppression or widening into unrelated workspace debt.
