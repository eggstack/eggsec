# Phase 5 Handoff Plan: TUI Enforcement Posture Model

## Goal

Make TUI enforcement posture explicit, visible, and controlled by a first-class model. The TUI should remain a manual operator surface by default, but it should clearly show whether the current posture is manual permissive or guarded, what scope provenance is loaded, and whether a pending action will allow, warn, require confirmation, or deny.

This phase should improve usability without making the TUI agent-strict by default.

## Rationale

The TUI currently has an `EnforcementContext` and `LoadedScope`, which is the right foundation. The weak point is that startup enforcement is implicit and scope loading appears tied to settings-tab state. Manual users need transparent feedback, not hidden policy machinery.

The TUI should answer before execution:

- What mode am I in?
- What scope source is active?
- How many allow/exclusion rules are loaded?
- What risk/capability does this action require?
- Will the action allow, warn, require confirmation, or deny?
- If confirmation is required, what exact action or flag would satisfy it?

## Desired posture behavior

Default TUI mode:

- `ExecutionSurface::TuiManual`.
- `ExecutionProfile::ManualPermissive`.
- Warnings and explicit confirmations available.
- Manual override/confirmation state lives in TUI state, not agent/MCP requests.

Guarded TUI mode:

- `ExecutionSurface::TuiManualStrict`.
- `ExecutionProfile::ManualGuarded`.
- Scope ambiguity and confirmation cases become denials.
- Manual overrides ignored.

No TUI mode in this phase should become MCP/Agent strict. TUI guarded is a human strict mode, not an agent-controlled mode.

## Files likely to change

Primary:

- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/runner.rs`
- `crates/eggsec-tui/src/app/state.rs`
- `crates/eggsec-tui/src/app/action.rs`
- `crates/eggsec-tui/src/app/confirmation.rs`
- `crates/eggsec-tui/src/ui/...`
- `crates/eggsec-tui/src/tabs/settings...`
- `crates/eggsec-tui/src/tabs/...` for tabs that execute operations

Likely engine support:

- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/src/config/policy_decision.rs`

Tests:

- `crates/eggsec-tui/src/test_utils...`
- TUI app unit tests if present.
- Policy tests in `eggsec` if helper extraction is needed.

## Step 1: Add `TuiEnforcementState`

Create a TUI-local model. Suggested location: `crates/eggsec-tui/src/app/state.rs` or a new `crates/eggsec-tui/src/app/enforcement.rs` if preferred.

Suggested shape:

```rust
#[derive(Debug, Clone)]
pub struct TuiEnforcementState {
    pub surface: eggsec::config::ExecutionSurface,
    pub loaded_scope: eggsec::config::LoadedScope,
    pub enforcement: eggsec::config::EnforcementContext,
    pub manual_override: eggsec::config::ManualOverride,
    pub last_preflight: Option<TuiPreflightResult>,
}
```

Suggested `TuiPreflightResult`:

```rust
#[derive(Debug, Clone)]
pub struct TuiPreflightResult {
    pub operation: String,
    pub target: Option<String>,
    pub outcome_kind: TuiPreflightOutcomeKind,
    pub decision: eggsec::config::PolicyDecision,
    pub required_confirmation_classes: Vec<eggsec::config::ConfirmationClass>,
    pub suggested_cli_flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiPreflightOutcomeKind {
    Allow,
    Warn,
    RequireConfirmation,
    Deny,
}
```

Keep this TUI-local. Do not change engine outcome types merely for UI display.

## Step 2: Replace loose `App` fields with the model

`App` currently stores `enforcement` and `loaded_scope`. Replace or wrap these with `TuiEnforcementState`.

Migration option A, preferred:

```rust
pub enforcement_state: TuiEnforcementState,
```

Migration option B, lower-risk transitional:

- Keep existing `app.enforcement` and `app.loaded_scope` fields.
- Add `app.enforcement_state` and keep fields synchronized for one pass.
- Remove old fields in a later cleanup.

Prefer option A if call sites are manageable.

## Step 3: Initialize from config/scope explicitly

In `runner.rs`, replace direct construction of `EnforcementContext::manual_permissive(...)` with a TUI enforcement initialization helper:

```rust
let enforcement_state = TuiEnforcementState::load(
    ExecutionSurface::TuiManual,
    policy,
    scope_path_opt.as_deref(),
)?;
```

The helper should:

- Load `LoadedScope` using `load_scope_with_source(...)`.
- Fall back to `LoadedScope::default_empty()` only for manual default behavior.
- Construct `EnforcementContext::for_surface(...)` or the equivalent mapping.
- Store the surface.

Avoid deriving the active scope path from arbitrary settings-tab field order long term. If the settings tab is still the only source for now, isolate that behind a named helper such as `settings_active_scope_path()` so it can be replaced later.

## Step 4: Add TUI guarded/manual toggle

Add an operator-visible setting/action to toggle between:

- Manual mode (`TuiManual` / `ManualPermissive`).
- Guarded mode (`TuiManualStrict` / `ManualGuarded`).

This can be in settings, command palette, or a hotkey if one already exists for global toggles.

When toggled:

- Rebuild `EnforcementContext` with the same policy and loaded scope.
- Clear or mark stale the last preflight result.
- Add a notification explaining the new posture.

Expected notification text examples:

- `TUI enforcement posture: manual. Warnings and explicit confirmations are available.`
- `TUI enforcement posture: guarded. Scope ambiguity and confirmation cases will deny.`

## Step 5: Add preflight evaluation helper

Add a method on `App` or `TuiEnforcementState`:

```rust
pub fn preflight_operation(&mut self, descriptor: OperationDescriptor) -> TuiPreflightResult
```

It should call the same enforcement evaluator:

```rust
let outcome = self.enforcement.evaluate(&descriptor);
```

Then map to display result:

- `Allow(decision)` -> allow.
- `Warn(decision)` -> warn.
- `RequireConfirmation(decision)` -> confirmation required with class list and suggested UI action/CLI flags.
- `Deny(decision)` -> deny.

Use existing `confirmation_classes_for(...)` and `confirmation_class_strings(...)` helpers.

## Step 6: Wire preflight into action execution

Before tabs execute target-bearing/security operations, they should obtain an `OperationDescriptor` and preflight it.

For this phase, do not attempt to cover every TUI action if that is too large. Prioritize high-value tabs/actions:

1. Recon / scan.
2. WAF / WAF stress.
3. Load/stress actions if feature-enabled.
4. DB pentest tab if feature-enabled.
5. Web proxy/interception tab if feature-enabled.

For actions not yet covered, leave a TODO and avoid claiming full coverage.

Execution behavior:

- `Allow`: execute.
- `Warn`: execute but surface warning notification/status.
- `RequireConfirmation`: open confirmation overlay or require explicit TUI confirmation action before execution.
- `Deny`: do not execute; show denial reason.

Manual confirmation in TUI should set the matching `ManualOverride` fields for that action, not globally forever unless the UI explicitly has a persistent override setting.

## Step 7: Display posture and scope provenance

Add a compact status indicator somewhere visible, such as header/status bar/settings panel:

- Mode: `Manual` or `Guarded`.
- Scope: `none`, `default-empty`, `config`, `cli-scope-file`, `tui-scope-file`, etc.
- Allow rules count.
- Exclusion rules count.

Suggested compact text:

```text
Mode: Manual | Scope: scope.toml | allow: 3 | exclude: 1
```

If no explicit scope is loaded:

```text
Mode: Manual | Scope: none (warnings enabled)
```

In guarded mode with no scope:

```text
Mode: Guarded | Scope: none (targeted operations may deny)
```

## Step 8: Show CLI-equivalent preview

For preflight results that require confirmation or deny, show a copyable CLI-equivalent command where possible. If the TUI already has a copy-CLI-equivalent feature, extend it to include relevant flags.

Examples:

- `--allow-out-of-scope`
- `--allow-high-risk`
- `--allow-private-resolution`
- `--allow-cross-host-redirect`
- `--allow-nonbaseline-capability`
- `--allow-web-proxy`
- `--manual-override-reason "..."`

Do not show these flags for MCP/agent surfaces.

## Step 9: Tests

Add tests for `TuiEnforcementState`:

- Defaults to `ExecutionSurface::TuiManual`.
- TUI manual maps to `ManualPermissive`.
- TUI guarded maps to `ManualGuarded`.
- Toggling preserves loaded scope.
- Toggling clears or invalidates last preflight.
- Manual preflight for safe ambiguity returns warn.
- Guarded preflight for same descriptor returns deny.
- Manual preflight for positive scope miss returns require confirmation.
- Confirming a matching class allows dispatch through the manual override pathway.

Add UI-level tests only if existing TUI tests make that cheap. Do not create brittle snapshot tests unless the repo already uses them.

## Acceptance criteria

- TUI has a first-class enforcement posture state.
- TUI default remains manual permissive.
- TUI guarded mode exists and maps to manual guarded, not agent strict.
- Current scope provenance and rule counts are visible.
- Preflight evaluation uses shared `EnforcementContext`.
- TUI can display allow/warn/confirmation/deny before execution for prioritized actions.
- Confirmation classes map to clear UI text and CLI-equivalent flags.
- Tests cover posture toggling and representative preflight outcomes.

## Suggested validation

Run:

```bash
cargo fmt --all
cargo test -p eggsec-tui
cargo check -p eggsec-tui
cargo check -p eggsec-tui --features full
cargo test -p eggsec --lib config::policy_decision
```

If `full` pulls in too much for normal CI, use the subset that covers TUI plus common security tabs.

## Non-goals

- Do not make TUI default agent-strict.
- Do not complete every possible tab integration if it makes the phase too large; cover prioritized actions and leave explicit TODOs.
- Do not redesign the global policy evaluator.
- Do not implement REST enforcement here.
- Do not add domain crate extraction here.

## Common pitfalls

- Do not let TUI confirmation state leak into MCP/agent structures.
- Do not store broad persistent overrides without making that explicit in UI and audit.
- Do not hide denial reasons behind generic UI errors.
- Do not derive scope provenance from raw `Scope`; use `LoadedScope`.
- Do not make guarded TUI synonymous with agent mode. Guarded TUI is still a human surface.
