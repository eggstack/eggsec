# Phase 2 Plan: TUI Surface and Wiring Convergence

## Status

Status: Ready for handoff.

Depends on: Phase 0 parity guards; Phase 1 canonical engine dispatch boundary.

## Objective

Finish the TUI metadata/action migration so tab discovery, command-palette routing, help, feature availability, canonical operation identity, run behavior, and CLI-equivalent generation are derived from a coherent surface model instead of several partially overlapping tables and matches.

The TUI does not need visual 1:1 parity with every CLI command. It does need explicit, testable semantics for every advertised action and every operation-backed tab.

## Starting state

At the audited baseline:

- `tabs/mod.rs` defines the `Tab` enum, manually constructs `Tab::all()`, and contains additional manual help routing.
- `tabs/spec.rs` defines `TabSpec` and claims single-source metadata, but some enumeration/feature behavior is intentionally duplicated outside it.
- `app/action_spec.rs` is explicitly a four-action pilot for recon, port scan, fuzz, and DB pentest; most tabs remain on the older path.
- `app/command.rs` manually maps command/alias strings to tabs and separately matches palette commands for execution.
- operation IDs and feature annotations are not fully normalized to canonical command/operation metadata.
- `reload-scope` is advertised but currently reports that live reload is unsupported.
- CLI-equivalent generation is another place where TUI state can drift from the actual Clap/canonical request contract.

This phase should collapse those concerns without turning canonical engine metadata into TUI layout metadata.

## Design principle

Use two layers, not one giant registry and not several overlapping ones:

```text
Canonical operation/command metadata
- operation ID
- risk / mode / target kind
- engine feature ownership
- canonical aliases / execution classification

TUI surface metadata
- tab stable ID/title
- category/order
- breadcrumb/help text
- key/palette aliases
- direct-launch behavior
- supports run/export/settings
- UI-only/unavailable behavior
- reference to canonical operation/command when operation-backed
```

The TUI layer should **reference** canonical operation/command metadata rather than copying canonical operation IDs, risk, and feature semantics as unconstrained strings.

## Workstream 2.1 — Replace the four-action pilot with one completed model

Choose one of these outcomes and complete it in this phase:

1. Fold `TUI_ACTION_SPECS` into an expanded `TabSpec`/surface registry and delete `action_spec.rs`; or
2. Promote `ActionSpec` to the canonical TUI action model and make `TabSpec` purely visual/tab metadata.

Do not retain both as overlapping partial registries.

The preferred direction is to keep one `TuiSurfaceSpec`/`TabSpec` for discoverable tabs and a typed `Action` enum for runtime UI intent. The production metadata table should cover every tab, including UI-only tabs, and explicitly indicate whether an item maps to a canonical operation.

Remove pilot comments/`allow(dead_code)` allowances that exist only because the migration was partial.

## Workstream 2.2 — Make operation references typed/canonical

Operation-backed TUI entries should resolve through the engine operation catalog rather than storing unconstrained alternate operation names.

Resolve audited discrepancies including:

- WAF: TUI `waf` versus canonical `waf-detect`;
- pipeline: TUI `scan-pipeline` versus canonical `pipeline`;
- packet operation family/modes;
- proxy versus `proxy-intercept` where these represent distinct UX concepts;
- wireless passive/active variants;
- runtime-backed management domains such as compliance/storage/integrations/workflow/vulnerability management.

Compatibility labels can remain user-facing. Canonical execution identity must not depend on the label string.

If a TUI entry launches a command multiplexer rather than one canonical operation, model that explicitly as a route type rather than inventing a pseudo-operation ID.

## Workstream 2.3 — Centralize TUI feature availability

Stop maintaining feature availability separately in `Tab::all()`, `TabSpec`, command aliases, and execution code.

Define a typed availability state derived from canonical feature metadata plus the TUI crate's compiled feature set, for example:

```text
Available
Unavailable { required_feature }
UiOnly
NotSupportedOnTui
```

Decide, per capability, whether feature-disabled tabs disappear or remain visible as unavailable discovery shells. Both policies are defensible; mixing them accidentally is not.

Specifically audit stress and packet tabs against their CLI feature gates (`stress-testing`, `packet-inspection`) and add regression tests for no-default/default/feature-enabled builds.

Keep Cargo feature forwarding in `crates/eggsec-tui/Cargo.toml` synchronized with engine feature ownership through tests where practical.

## Workstream 2.4 — Route UI input through typed actions

Follow a reified action/message architecture:

```text
terminal event / keybinding / palette command
               -> TuiAction
               -> app state update / effect request
               -> canonical execution request when needed
               -> runtime event
               -> app state update
               -> render
```

Rendering functions should not decide operation semantics or start engine work. Key handlers and palette parsing should produce the same typed `TuiAction` when they mean the same thing.

Separate navigation/local actions (switch tab, open help, copy, export view, theme, search) from execution effects (run operation, cancel, reload scope/config, attach runtime). This makes the app state machine testable without a terminal, consistent with current Ratatui application guidance.

## Workstream 2.5 — Eliminate manual command-to-tab drift

Replace the broad string match in `command_to_tab` with lookup through the consolidated TUI surface metadata.

Requirements:

- one canonical stable ID per tab;
- optional aliases stored with that tab/action spec;
- aliases tested unique unless an explicit priority rule exists;
- hidden/deprecated aliases remain parseable if compatibility requires them but need not pollute palette discovery;
- feature-disabled aliases return a structured unavailable result rather than silently behaving differently from the tab list;
- help/palette filtering use the same metadata source.

Do not use a runtime hash registry when a static slice plus lookup is sufficient.

## Workstream 2.6 — Derive help/discovery from the same surface spec

Reduce the separate exhaustive `help_entry()` mapping and repeated descriptive strings.

Keep long-form help in dedicated content where needed, but reference it from the same surface spec. The following should not disagree:

- tab title;
- palette command/stable ID;
- short description;
- feature requirement/availability;
- whether `Run` is supported;
- canonical operation/route;
- help target.

Snapshot tests are appropriate for rendered help layout, while typed tests should verify semantic metadata.

## Workstream 2.7 — Make `copy-cli` a semantic round trip

CLI-equivalent generation should not be a separately maintained approximation of operation defaults.

For an operation-backed tab:

1. derive the canonical request from TUI state;
2. use a command/request adapter to produce a stable CLI representation;
3. parse that representation through the actual Clap `Cli` parser in tests;
4. convert the parsed CLI DTO back to the canonical request;
5. assert semantic equality after normalization.

Not every TUI state requires a CLI equivalent. UI-only state should return an explicit unsupported result rather than constructing misleading commands.

Preserve shell-quoting correctness; use a dedicated argument-vector representation internally and quote only at the final clipboard/string boundary.

## Workstream 2.8 — Resolve `reload-scope`

The current palette contains a `reload-scope` command that is not implemented in the TUI. Choose and implement one of two explicit contracts.

### Preferred: implement safe live reload

Create an application-level reload effect that:

- reloads config and scope through the same loader used by CLI startup;
- validates the new configuration completely before swapping it into active state;
- atomically replaces shared policy/scope state only after successful validation;
- invalidates cached approvals and any descriptor/policy-derived UI state;
- updates displayed posture/profile/scope information;
- does not mutate an already-running task's approved execution contract;
- emits a clear success/failure event without leaving a partial configuration applied.

For running tasks, define whether the reload applies only to future tasks (preferred) or blocks until idle. Do not retroactively reinterpret an `ApprovedOperation` already executing.

### Acceptable alternative: remove the false affordance

If safe reload is intentionally out of scope, remove it from discoverable palette/help metadata and preserve a restart-required informational action only if useful. The user-facing surface must not present an unsupported action as though it were functional.

## Workstream 2.9 — TUI semantic tests

Add tests that do not require a live terminal for:

- key event -> `TuiAction` mapping;
- palette string/alias -> same `TuiAction` mapping;
- action -> state transition/effect;
- every runnable tab -> canonical operation/route;
- feature availability under representative builds;
- operation target/request construction from tab state;
- cancellation state transitions;
- exact approval-cache invalidation integration from Phase 0;
- help/palette discoverability;
- CLI-equivalent round trips.

Use Ratatui buffer/snapshot tests only for rendering regressions where they add value; do not use rendering snapshots as a substitute for semantic wiring tests.

## Workstream 2.10 — Decompose TUI files along the new boundaries

After the action/surface model is stable, split large modules by responsibility.

High-priority audited hotspots include:

```text
crates/eggsec-tui/src/tabs/core.rs
crates/eggsec-tui/src/app/command.rs
crates/eggsec-tui/src/app/help_config.rs
crates/eggsec-tui/src/app/apply.rs
crates/eggsec-tui/src/app/export.rs
crates/eggsec-tui/src/app/enforcement.rs
crates/eggsec-tui/src/app/enforcement_facade.rs
```

Split by stable concepts (surface catalog, action parsing, state reducer, execution effects, help, export, enforcement UI), not by arbitrary file size. Avoid one-module-per-tab boilerplate if tabs share a coherent family model.

## Acceptance criteria

- the four-action pilot no longer exists as a partial second metadata system;
- every `Tab` is represented by one coherent TUI surface specification;
- operation-backed entries resolve canonical operation IDs through typed/canonical metadata;
- feature availability has one TUI owner and is tested against real compiled features;
- `Tab::all`, help, palette discovery, alias lookup, and execution routing cannot independently drift;
- keybindings and palette commands produce typed actions that are testable without rendering a terminal;
- `copy-cli` round-trips through real Clap parsing for supported operation-backed tabs;
- `reload-scope` is either safely implemented with approval invalidation or removed as a false affordance;
- no operation semantics live in rendering code;
- Phase 0 and Phase 1 parity/execution tests remain green.

## Verification

At minimum:

```text
make fmt
make test-feature-matrix
make test-architecture-guards
make check
make check-feature-profiles
cargo test -p eggsec-tui
```

Also test TUI compilation/semantic tests with at least `stress-testing`, `packet-inspection`, and a representative broad feature profile. Use `make check-features-individual` when feature metadata/forwarding changes.

## Research note

Current Ratatui documentation describes `Command`/`Action`/`Message` patterns as a way to reify application intent, separate event mapping from updates, and test business logic without instantiating a terminal. This phase applies that principle narrowly to existing eggsec architecture rather than introducing a new framework:

https://ratatui.rs/tutorials/counter-async-app/actions/

## Handoff notes

Do not combine this phase with a visual redesign. Keep layouts/keybindings stable except where a duplicate/false command must be corrected. The success metric is reduced semantic ownership and stronger wiring tests, not a different appearance.

Append a completion record with before/after counts of metadata tables/manual routing matches, number of classified tabs/actions, operation-ID discrepancies resolved, and tested feature profiles.
