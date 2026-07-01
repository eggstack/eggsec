# Adding TUI Tabs and Actions

This guide covers how to declare TUI tabs, register metadata-backed action
specs, wire up enforcement posture, and integrate direct-launch gates without
duplicating risk or scope semantics.

## 1. Declaring Tabs and Actions

TUI tabs are declared in two places that must stay in sync:

### Tab Enum

Add a variant to `Tab` in `crates/eggsec-tui/src/tabs/mod.rs`:

```rust
pub enum Tab {
    // ... existing variants
    MyNewTab = N,  // next discriminant
}
```

Add the tab to the `cfg_push_tabs!` block in `Tab::all()`. If the tab is
feature-gated, use the `#[cfg(feature = "...")]` attribute on the gated
list. If it is always available, add it to the `base` list.

### TabSpec

Add a corresponding `TabSpec` entry in `crates/eggsec-tui/src/tabs/spec.rs`:

```rust
TabSpec {
    tab: Tab::MyNewTab,
    stable_id: "my_new_tab",
    title: "My New Tab",
    cli_command: "eggsec my-new-tab",
    description: "Description of what the tab does",
    help_text: "Longer help text for the help overlay.",
    category: TabCategory::Assessment,
    risk_group: TabRiskGroup::SafeActive,
    feature: None, // or Some("my-feature") for gated tabs
    breadcrumb_label: "My New Tab",
    operation: Some("my-operation"), // canonical operation ID
    direct_launch: false,
    supports_run: true,
    supports_export: false,
    supports_help: true,
    has_settings: false,
}
```

Key fields:

- `stable_id` is used for session persistence and the quick-switch panel.
- `operation` must reference a canonical operation ID that resolves through
  `metadata_for_tool_id()`.
- `direct_launch` is true when Enter starts the operation without requiring
  a separate target input step. Direct-launch tabs go through the pre-dispatch
  enforcement gate in `handle_enter()`.

### TabSpec count invariant

The number of `TabSpec` entries in `TAB_SPECS` must equal the number of `Tab`
variants. The test `test_tab_spec_count_matches_all_tab_variants` enforces
this.

## 2. TuiActionSpec and Canonical Metadata

`TuiActionSpec` in `crates/eggsec-tui/src/app/action_spec.rs` provides
metadata-backed action descriptors that point to canonical `OperationMetadata`.

### Why this matters

The TUI must not independently invent risk, capability, or scope semantics.
Risk levels, required features, and policy flags all come from
`OperationMetadata`. `TuiActionSpec` exists to validate that TUI actions
remain consistent with the shared enforcement model.

### Adding a new action spec

Add an entry to `TUI_ACTION_SPECS`:

```rust
TuiActionSpec {
    action_id: "my-operation-run",
    operation_id: "my-operation", // must resolve via metadata_for_tool_id()
    tab_id: "my_new_tab",
    feature: None, // or Some("my-feature")
    manual_only: true,
}
```

Add a corresponding `TuiTabSpec` entry to `TUI_TAB_SPECS` that references the
action by index into `TUI_ACTION_SPECS`.

### Resolution validation

The function `action_resolves_to_metadata()` checks that an action's
`operation_id` resolves to an `OperationMetadata` entry. The test
`all_pilot_actions_resolve_to_metadata` enforces this for all registered
actions.

If you add a new action with an `operation_id` that has no matching
`OperationMetadata`, the test will fail. Add the operation to
`ALL_OPERATION_METADATA` in `crates/eggsec/src/config/policy.rs` first.

### Risk consistency

The test `intrusive_actions_are_manual_only` verifies that any action whose
metadata declares `OperationRisk::Intrusive` is marked `manual_only: true`.
This prevents high-risk operations from being programmatically exposed.

## 3. TUI Enforcement Posture

The TUI uses `EnforcementFacade` (wrapping `TuiEnforcementState`) for all
enforcement decisions. The facade is the single source of truth for the
current enforcement context, loaded scope, and cached approval tokens.

### Two postures

| Posture | Surface | Profile | Manual Override |
|---------|---------|---------|-----------------|
| **Manual** (default) | `TuiManual` | `ManualPermissive` | Honored |
| **Guarded** | `TuiManualStrict` | `ManualGuarded` | Not honored |

Toggle between them via `Ctrl+G`, which calls
`TuiEnforcementState::toggle_posture()`.

### Critical behavior difference

- `TuiManual` permits manual overrides. When a preflight returns
  `RequireConfirmation`, the user sees a confirmation overlay and can accept
  the override.
- `TuiManualStrict` does **not** honor manual overrides. Scope ambiguity is
  denied outright. This mirrors CLI `--strict-scope` semantics.

### Posture maps to enforcement profile

```rust
// In TuiEnforcementState::toggle_posture():
let new_surface = match self.surface {
    ExecutionSurface::TuiManual => ExecutionSurface::TuiManualStrict,
    ExecutionSurface::TuiManualStrict => ExecutionSurface::TuiManual,
    other => other,
};
```

The enforcement profile is derived from the surface via
`new_surface.profile()`.

### Don't make TUI stricter than agent mode by accident

The TUI's `TuiManualStrict` posture is intentionally less strict than
`AgentStrict`. Agent mode denies on anything other than `Allow` and never
honors manual overrides. `TuiManualStrict` denies on scope ambiguity but
still uses `approve_manual()` for `Warn` outcomes (the audit event records
the override). Do not add extra deny conditions in TUI code that would make
it stricter than agent mode.

## 4. Shared Policy Preflight

Shared policy preflight is **advisory** in the TUI. It evaluates a target
against the current posture and displays the result in the status bar.

### How it works

1. `TuiEnforcementState::preflight()` calls `preflight_operation()` (the
   same shared function used by CLI, REST, MCP, and agent).
2. The result is stored in `last_preflight` and displayed in the status bar.
3. The status bar shows: mode, scope provenance, allow/exclude rule counts,
   and the preflight outcome summary.

### Preflight does not gate execution

Preflight is advisory. The actual enforcement gate runs at dispatch time
via `EnforcementFacade::try_approve()` or `evaluate_and_try_approve()`.

### Preflight and execution must agree

Tests in `enforcement_facade.rs` verify that preflight and execution produce
the same outcome for the same descriptor:

- `preflight_and_execution_agree_on_allowed_action`
- `preflight_and_execution_agree_on_confirmation_action`
- `preflight_and_execution_agree_on_denied_action`
- `preflight_populates_same_outcome_as_raw_evaluate`

If you add a new enforcement path, ensure it produces the same outcome as
preflight for the same inputs.

## 5. Direct-Launch Gates and Approval

Tabs with `direct_launch: true` in their `TabSpec` start an operation when
the user presses Enter without a separate target input step. These tabs go
through a pre-dispatch enforcement gate in `handle_enter()`.

### Approval flow

1. `handle_enter()` builds an `OperationDescriptor` via
   `build_current_operation_descriptor()`.
2. The descriptor is passed to `EnforcementFacade::try_approve()`, which
   evaluates it against the current `EnforcementContext`.
3. If the outcome is `Allow`, an `ApprovedOperation` token is returned and
   cached in `pending_approved`.
4. If the outcome is `RequireConfirmation`, the confirmation overlay is
   shown. On confirmation, `confirm_override()` builds a `ManualOverride`
   and re-evaluates.
5. If the outcome is `Deny`, the operation is blocked with a notification.
6. `evaluate_and_try_approve()` consumes the cached `ApprovedOperation` to
   avoid redundant evaluation between the pre-dispatch gate and the
   dispatch call.

### Cached approval reuse

`pending_approved` caches the `ApprovedOperation` from the pre-dispatch gate.
When `evaluate_and_try_approve()` is called later, it checks if the cached
token matches the descriptor's operation and reuses it if so. This prevents
double evaluation.

```rust
// In EnforcementFacade::evaluate_and_try_approve():
if let Some(cached) = self.pending_approved.take() {
    if cached.descriptor().operation == desc.operation {
        return Ok(cached);
    }
}
self.try_approve(desc)
```

### Audit trail

Every enforcement decision emits a normalized `EnforcementAuditEvent` via
`eggsec::audit`. Events are emitted in:

- `handle_enter()` for pre-dispatch evaluation
- `evaluate_and_try_approve()` for approval
- `confirm_override()` when manual override is accepted
- `preflight()` for advisory evaluation

## 6. Theme and Startup Behavior

Theme loading and other optional resource initialization must remain
non-blocking. The TUI must start and render immediately, even if themes or
other resources are not yet loaded.

### ThemeLoadState

`ThemeLoadState` in `app/state.rs` manages background theme loading:

```rust
pub struct ThemeLoadState {
    pub receiver: Option<Receiver<ThemeLoadOutcome>>,
    pub join_handle: Option<JoinHandle<()>>,
    pub deferred_restore: Option<String>,
    pub reason: ThemeLoadReason,
}
```

### Background loading

Theme loading runs in a background thread via `spawn_theme_loader_with_reason()`.
On startup, `ThemeLoadReason::Startup` is used. Manual reloads use
`ThemeLoadReason::ManualReload`.

- `Startup` does not show a notification while loading.
- `ManualReload` shows a "Loading themes..." notification immediately.

### Deferred restore

If the user selects a theme before the background loader finishes, the
selection is stored in `deferred_restore` and applied when the loader
completes. The TUI renders with the fallback theme (`cyber-red`) in the
meantime.

### Non-blocking invariant

Never block the TUI event loop on:

- Theme file parsing or loading.
- Scope file loading (if the file is missing or malformed, use defaults).
- Session restoration failures (log a warning and continue with defaults).
- Database connections or network calls during startup.

If an optional resource fails to load, the TUI must continue operating with
fallback defaults. The user sees a notification or status bar indication,
not a frozen or crashed interface.

## 7. Required Tests

All new TUI tabs and actions must include tests. Run the TUI test suite with:

```bash
cargo test --lib -p eggsec-tui
```

### Tests for action_spec.rs

When adding a `TuiActionSpec` or `TuiTabSpec`:

| Test | What it validates |
|------|-------------------|
| `all_pilot_actions_resolve_to_metadata` | `operation_id` resolves to `OperationMetadata` |
| `all_pilot_tab_ids_are_valid` | `tab_id` maps to a valid `TabSpec` stable_id |
| `feature_strings_are_valid` | Feature strings are non-empty when present |
| `intrusive_actions_are_manual_only` | Intrusive operations are marked manual_only |
| `domain_refs_are_valid` | Domain references resolve to known `DomainDescriptor` |
| `all_pilot_tab_operations_resolve` | TabSpec operations resolve via metadata |
| `registry_enumeration` | Registry is non-empty and each tab has at least one action |

### Tests for enforcement.rs

| Test | What it validates |
|------|-------------------|
| `defaults_to_tui_manual` | Default posture is `TuiManual` / `ManualPermissive` |
| `toggle_roundtrip` | Toggle from Manual to Guarded and back |
| `toggle_clears_last_preflight` | Preflight cache is cleared on posture change |
| `preflight_safe_operation_allows_or_warns` | Safe passive ops are allowed |
| `preflight_scope_miss_triggers_confirmation_or_deny` | Scope miss in guarded mode denies |
| `honors_manual_override_in_tui_manual` | Manual mode honors overrides |
| `does_not_honor_manual_override_in_tui_guarded` | Guarded mode does not honor overrides |

### Tests for enforcement_facade.rs

| Test | What it validates |
|------|-------------------|
| `try_approve_allows_passive_in_default_scope` | Passive ops approved in default scope |
| `evaluate_and_try_approve_uses_cached_approval` | Cached token is reused |
| `take_cached_approval_rejects_mismatch` | Mismatched operations are not reused |
| `confirm_override_sets_manual_override_flags` | Manual override flags are set correctly |
| `preflight_and_execution_agree_on_*` | Preflight and execution outcomes match |

### Tests for tabs/spec.rs

| Test | What it validates |
|------|-------------------|
| `test_all_tabs_have_specs` | Every `Tab` variant has a `TabSpec` |
| `test_tab_spec_count_matches_all_tab_variants` | Spec count matches enum variant count |
| `test_feature_gated_tabs_have_valid_feature` | Gated tabs have correct feature strings |
| `test_no_empty_stable_ids` | No empty stable_id or title fields |

## Warnings

- **Do not duplicate risk/capability/scope semantics in TUI.** Risk levels,
  required features, and policy flags come from `OperationMetadata`. TUI
  action specs point to canonical metadata. If you find yourself adding a
  risk field to a TUI struct, you are duplicating the source of truth.

- **Do not block visible TUI startup on optional files/themes/resources.**
  The TUI must render immediately. Theme loading, scope file parsing, and
  session restoration happen in the background or with fallback defaults.

- **Do not make TUI stricter than agent mode by accident, but preserve
  explicit strict posture mode.** `TuiManualStrict` is less strict than
  `AgentStrict` by design. `AgentStrict` never honors manual overrides and
  denies on anything other than `Allow`. `TuiManualStrict` denies on scope
  ambiguity but allows `Warn` outcomes through `approve_manual()`. Do not
  add extra deny conditions in TUI code. If you need stricter behavior, use
  the existing `ManualGuarded` posture rather than adding new enforcement
  paths.
