# Plan: Uniform Vim j/k in All TUI Dropdown Selectors

## Summary

The TUI already handles:
1. **Uniform theme application** across all panels (via `tc!()` thread-local + `theme_manager.current()`)
2. **Live theme preview** when scrolling through the theme selector in Settings (via `maybe_refresh_theme_preview()`)
3. **Vim j/k bindings** in Settings tab selectors specifically (`has_settings_selector_open()` guard)

**The gap**: j/k navigation only works in Settings tab selectors. Selectors in 27+ other tabs (Scan, Fuzz, Load, Stress, Report, etc.) do NOT get j/k support. Additionally, pressing any normal-mode key (q, r, h, l, etc.) while a non-Settings selector is open fires unintended actions because there's no guard.

## Changes

### Step 1: Add `has_selector_open()` to `TabState` trait

**File**: `crates/eggsec-tui/src/tabs/mod.rs:555`

Add to the existing trait with a default `false`:
```rust
pub trait TabState {
    fn state(&self) -> AppState;
    fn progress(&self) -> f64;
    fn is_running(&self) -> bool { ... }
    fn reset(&mut self) {}
    fn set_error(&mut self, _error: TabError) {}
    fn has_selector_open(&self) -> bool { false }  // NEW
}
```

This is the right trait because `as_tab_state(&self, &App) -> &dyn TabState` takes `&self` (no mut needed), which is what `decode_normal_mode_input` has access to.

### Step 2: Override `has_selector_open()` in each tab with selectors

Each override checks `is_open()` on its selectors. Tabs to modify:

| Tab | File | Selectors to check |
|-----|------|--------------------|
| `SettingsTab` | `settings/main.rs` | `theme_selector`, `proxy_rotation_selector`, `severity_selector` |
| `ScanTab` | `scan.rs` | `profile_selector`, `output_selector` |
| `FuzzTab` | `fuzz.rs` | `payload_selector`, `mode_selector`, `target_selector` |
| `LoadTab` | `load.rs` | `test_type_selector` |
| `StressTab` | `stress.rs` | `type_selector` |
| `ReportTab` | `report.rs` | `view_selector` |
| `ClusterTab` | `cluster.rs` | `view_selector` |
| `ProxyTab` | `proxy.rs` | `view_selector` |
| `PacketTab` | `packet.rs` | `view_selector` |
| `WorkflowTab` | `workflow.rs` | `mode_selector`, `severity_selector`, `status_selector` |
| `ComplianceTab` | `compliance.rs` | `framework_selector` |
| `VulnTab` | `vuln.rs` | `mode_selector` |
| `IntegrationsTab` | `integrations.rs` | `tracker_selector`, `mode_selector` |
| `NseTab` | `nse.rs` | `script_selector` |

Tabs with no selectors (Dashboard, History, Auth, Recon, etc.) keep the default `false`.

### Step 3: Add `has_any_tab_selector_open()` to App

**File**: `crates/eggsec-tui/src/app/mod.rs`

```rust
pub fn has_any_tab_selector_open(&self) -> bool {
    self.current_tab.as_tab_state(self).has_selector_open()
}
```

This replaces `has_settings_selector_open()` and works for all tabs uniformly. The old method can be removed.

### Step 4: Update the key handler guard

**File**: `crates/eggsec-tui/src/app/key_handler.rs:224`

Replace:
```rust
if app.has_settings_selector_open() {
```
With:
```rust
if app.has_any_tab_selector_open() {
```

The rest of the guard (j/k → MoveDown/MoveUp, everything else → Noop) stays identical.

### Step 5: Update action hints (lower priority polish)

**File**: `crates/eggsec-tui/src/app/action_hints.rs`

The `settings_hints()` already shows `"Up/Down or j/k:preview"` when theme selector is open. For other tabs, update their hint functions to show j/k hints when their selectors are open. This requires each tab's hint function to check `has_selector_open()`.

## Files to Modify (in order)

1. `crates/eggsec-tui/src/tabs/mod.rs` — Add `has_selector_open()` to `TabState` trait
2. `crates/eggsec-tui/src/app/mod.rs` — Add `has_any_tab_selector_open()`, remove `has_settings_selector_open()`
3. `crates/eggsec-tui/src/app/key_handler.rs` — Update guard reference
4. 14 tab files — Override `has_selector_open()` (one-liner each)
5. `crates/eggsec-tui/src/app/action_hints.rs` — Update hints for non-Settings tabs

## Verification

```bash
cargo check -p eggsec-tui
cargo test --lib -p eggsec-tui
```

Existing tests to verify:
- `test_j_k_navigate_settings_selector` (key_handler.rs)
- `test_theme_selector_j_previews_and_escape_reverts` (key_handler.rs)
- `test_j_k_navigate_overlay` (key_handler.rs)
