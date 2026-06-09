# Eggsec TUI Simplification Plan Before Halloy-Style Theming

## Purpose

This plan prepares `eggsec-tui` for a later Halloy-style theming pass by simplifying the current TUI architecture first. Do not implement Halloy theme loading, external theme import, or large theme format changes in this pass. The objective is to make the existing TUI easier to maintain, easier to test, and easier to theme later without changing user-facing behavior.

The current crate boundary is mostly correct: `eggsec-tui` is already a dedicated workspace crate and Ratatui/Crossterm dependencies are isolated there. The remaining problem is internal architecture. `App` currently owns too many unrelated concerns, rendering code reads theme state implicitly through thread-local macros, and task/session/search/help/tab state is centralized in one large struct. This pass should reduce those maintenance risks while keeping behavior stable.

## Non-goals

Do not add Halloy theme parsing yet.

Do not add a new user-facing theme file format yet.

Do not rewrite every tab.

Do not change the CLI invocation model.

Do not move engine functionality out of `eggsec` in this pass.

Do not attempt a full frontend/backend IPC boundary. The TUI may continue to call engine APIs through the existing worker/task layer for now.

## Current codebase observations

The workspace already contains separate crates for `eggsec-core`, `eggsec`, `eggsec-tui`, and `eggsec-cli`. This is the correct top-level direction.

`eggsec-core` is dependency-light and should remain free of Ratatui/Crossterm or presentation concerns.

`eggsec-tui` currently depends on both `eggsec-core` and `eggsec`, which is acceptable for the current binary model, but it means the TUI is an adapter over the engine crate rather than a fully isolated frontend.

`eggsec-cli` depends on both `eggsec` and `eggsec-tui`; it launches the TUI when no command is passed and stdout is a terminal. Leave this behavior unchanged.

`crates/eggsec-tui/src/lib.rs` already has a useful module outline: `app`, `components`, `help`, `search`, `session`, `state`, `tabs`, `theme`, `ui`, `utils`, and `workers`.

`crates/eggsec-tui/src/app/mod.rs` is the primary complexity hotspot. `App` owns current tab state, mode state, session persistence, theme management, all tab instances, global HTTP options, search state, task handles, result/progress receivers, help state, command palette state, bookmarks, pause state, notifications, quick switch state, and multiple feature-gated tabs.

`crates/eggsec-tui/src/theme.rs` currently defines hardcoded dark/light themes and a `ThemeManager`, plus thread-local theme state and macros. This is workable but not ideal for a later config-backed theming layer.

`crates/eggsec-tui/src/ui.rs` renders the main shell and several popups directly, and uses implicit `tc!(...)` theme lookups throughout.

## Desired end state after this pass

After this pass, the TUI should still behave the same, but the internal shape should be cleaner:

- `App` should be thinner and primarily orchestrate event dispatch, task polling, session save/restore, and rendering.
- Search, help, quick-switch, notification, task, and session-related state should be grouped into focused structs.
- Rendering should begin moving toward explicit theme access, even if legacy macros remain temporarily for compatibility.
- The current hardcoded theme behavior should remain intact, but the theme module should be split into smaller files so that a future Halloy-style loader can be added without another structural churn pass.
- The feature-gated tabs should remain supported, but tab registration and access should be less scattered.
- The pass should preserve public CLI behavior and avoid destabilizing engine/task behavior.

## Phase 1: Establish guardrails and baseline checks

Before editing, run the relevant checks and note any existing failures:

```bash
cargo fmt --all -- --check
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo test -p eggsec-tui
```

If feature combinations are commonly used in this repo, also run:

```bash
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-cli --features nse
cargo check -p eggsec-cli --features full
```

If `full` is too heavy or currently fails for unrelated optional dependencies, document that and continue with the narrower checks. Do not hide pre-existing failures.

## Phase 2: Split TUI state into focused structs

Create or expand a state module under `crates/eggsec-tui/src/app/state.rs` or `crates/eggsec-tui/src/state/` depending on the existing module layout. Prefer the least disruptive location.

Introduce focused structs with narrow responsibilities. Suggested initial grouping:

```rust
pub struct OverlayState {
    pub show_help: bool,
    pub show_http_options: bool,
    pub show_search: bool,
    pub show_quick_switch: bool,
    pub pending_action: Option<PendingAction>,
    pub notification: Option<Notification>,
}

pub struct SearchUiState {
    pub query: String,
    pub is_global: bool,
    pub global_search: Option<crate::search::GlobalSearch>,
    pub backup: Option<std::collections::VecDeque<crate::tabs::history::HistoryEntry>>,
}

pub struct QuickSwitchState {
    pub query: String,
    pub selected: usize,
}

pub struct TaskUiState {
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub inner_abort: Option<tokio::task::AbortHandle>,
    pub tab: Option<crate::tabs::Tab>,
    pub progress_rx: Option<tokio::sync::mpsc::Receiver<(u64, u64)>>,
    pub result_rx: Option<crate::workers::TaskResultReceiver>,
    pub paused: bool,
}
```

Adjust exact types to fit the current worker module. If introducing a type alias for the result receiver reduces noise, add it in `workers` or `app::task_runtime`.

Move fields out of `App` gradually. Keep compatibility methods on `App` where that avoids broad churn. For example, `app.search_query` references can initially become `app.search.query`, but if that touches too many files, add helper methods and migrate in smaller slices.

Acceptance criteria for this phase:

- `App` no longer directly owns all search, quick-switch, overlay, and task fields as flat top-level fields.
- The constructor still restores session state and initializes defaults correctly.
- Behavior of help, search, command palette, task execution, and quick switch is unchanged.
- `cargo check -p eggsec-tui` passes.

## Phase 3: Extract tab ownership and tab access into a registry-like layer

The current `App` struct owns one field per tab. That makes every new tab inflate `App` and spreads feature gating across constructor, rendering, dispatch, session, and task config code.

Do not attempt a full trait-object rewrite in this pass unless it is low-risk. Instead, introduce a lightweight `TabStore` or `TabRegistry` wrapper that owns the tab instances and centralizes tab access.

Suggested shape:

```rust
pub struct TabStore {
    pub recon: tabs::ReconTab,
    pub load: tabs::LoadTab,
    pub scan_ports: tabs::ScanPortsTab,
    pub scan_endpoints: tabs::ScanEndpointsTab,
    pub fingerprint: tabs::FingerprintTab,
    pub fuzz: tabs::FuzzTab,
    pub waf: tabs::WafTab,
    pub waf_stress: tabs::WafStressTab,
    pub scan: tabs::ScanTab,
    pub resume: tabs::ResumeTab,
    pub proxy: tabs::ProxyTab,
    pub packet: tabs::PacketTab,
    pub graphql: tabs::GraphQlTab,
    pub oauth: tabs::OAuthTab,
    pub cluster: tabs::ClusterTab,
    pub stress: tabs::StressTab,
    pub report: tabs::ReportTab,
    pub settings: tabs::SettingsTab,
    pub dashboard: tabs::DashboardTab,
    #[cfg(feature = "nse")]
    pub nse: tabs::NseTab,
    // Preserve existing feature-gated tabs here.
}
```

Then `App` should own `pub tabs: TabStore` instead of one field per tab. Update tab access helpers such as `Tab::as_tab_input`, `Tab::as_tab_state`, and `Tab::as_tab_render` to access `app.tabs.<name>`.

This is intentionally a modest first step. It does not remove all concrete tab types, but it localizes tab ownership and makes future registry/trait-object work easier.

Acceptance criteria for this phase:

- `App` no longer has one direct field per tab.
- All tab instances are initialized through `TabStore::new()` or `Default`.
- Feature-gated tab fields remain behind the same feature gates.
- Existing tab navigation and task dispatch still work.
- `cargo check -p eggsec-tui` passes with default features and at least `--features nse`.

## Phase 4: Split `theme.rs` into a small module tree without changing behavior

This phase prepares for future Halloy-style theming, but does not implement it.

Replace the single `crates/eggsec-tui/src/theme.rs` file with a module directory:

```text
crates/eggsec-tui/src/theme/
  mod.rs
  palette.rs
  builtin.rs
  manager.rs
  style.rs
  legacy.rs
```

Suggested responsibilities:

- `palette.rs`: `ThemeMode`, `Theme`, `ThemeColors`.
- `builtin.rs`: `dark_theme()`, `light_theme()`, and any built-in theme list.
- `manager.rs`: `ThemeManager` and theme registration/lookup.
- `style.rs`: methods that convert semantic theme values into Ratatui `Style`, such as `style_for_tab`, `style_for_mode`, `style_for_status`, and `border_style`.
- `legacy.rs`: temporary thread-local state and `sync_theme_to_thread_local` support.
- `mod.rs`: public re-exports and compatibility surface.

Preserve existing public names where possible so this is mostly a file organization change. Keep `dark` and `light` themes exactly equivalent in this phase.

Do not add TOML parsing, runtime theme directories, Halloy import, or color format parsing yet.

Acceptance criteria for this phase:

- Existing calls to `ThemeManager::new()`, `current()`, `set_theme()`, `toggle()`, and `list_themes()` continue to work.
- Existing dark/light colors remain unchanged unless a compile error forces a mechanical relocation.
- `tc!` and `theme!` macros may remain, but they should be clearly marked as legacy/compatibility in comments.
- `cargo check -p eggsec-tui` passes.

## Phase 5: Begin moving rendering toward explicit theme access

Do not attempt to remove every `tc!(...)` use in one pass unless the diff remains small. The goal is to introduce the explicit pattern and migrate the main shell first.

Change the top-level draw path so that `ui::draw` gets the current theme once and passes it into shell-level helpers:

```rust
pub fn draw(f: &mut Frame, app: &mut App) {
    let theme = app.theme_manager.current().clone();
    draw_tabs(f, app, &theme, tab_area);
    draw_breadcrumb(f, app, &theme, breadcrumb_area);
    draw_content(f, app, &theme, content_area);
    draw_status_bar(f, app, &theme, status_area);
}
```

If borrowing makes `clone()` undesirable, pass `&Theme` with careful scoping. Prefer correctness and clarity over micro-optimizing this during the refactor.

Migrate `draw_tabs`, `draw_breadcrumb`, `draw_status_bar`, and one or two simple popup renderers away from `tc!(...)`. Leave deeper tab rendering for later if it would create a large diff.

Acceptance criteria for this phase:

- The main shell rendering path demonstrates explicit theme dependency.
- Legacy `tc!` remains available for unmigrated components.
- No visual behavior should intentionally change.
- `cargo check -p eggsec-tui` passes.

## Phase 6: Separate layout shell helpers from popup rendering

`ui.rs` currently contains the main layout plus several popup renderers. Split it without altering behavior.

Suggested structure:

```text
crates/eggsec-tui/src/ui/
  mod.rs
  shell.rs
  popups.rs
  command_palette.rs
  status.rs
  tabs.rs
```

If converting `ui.rs` into a directory is too disruptive, create `ui_shell.rs`, `ui_popups.rs`, etc. and re-export from `ui.rs`. Prefer a module directory if straightforward.

Suggested migration order:

1. Move layout constants and top-level `draw` into `ui/mod.rs` or `ui/shell.rs`.
2. Move `draw_tabs`, `draw_breadcrumb`, `draw_content`, and `draw_status_bar` into shell/status/tab modules.
3. Move `draw_http_options_popup`, `draw_search_popup`, `draw_quick_switch`, and command palette rendering into popup-specific modules.
4. Keep function signatures narrow. Pass `&App` or narrower state structs where feasible, but do not force a full dependency inversion in this pass.

Acceptance criteria for this phase:

- `ui.rs` is no longer a large mixed rendering file, or it becomes a thin module root.
- Top-level rendering remains easy to find.
- Popups are grouped separately from the main shell.
- `cargo check -p eggsec-tui` passes.

## Phase 7: Reduce direct feature-surface noise where low-risk

The TUI crate currently forwards many engine features. Do not redesign the feature model now. Instead, clean obvious duplication and document intent.

Add comments in `crates/eggsec-tui/Cargo.toml` explaining that TUI feature flags mirror engine capabilities only where the TUI exposes corresponding tabs or controls.

If a feature is forwarded but has no TUI code behind it, consider removing that forwarded TUI feature only if `cargo check` confirms it is unused and `eggsec-cli` does not require it for user-facing behavior. Be conservative.

Acceptance criteria for this phase:

- Feature forwarding remains stable for existing users.
- Any removed feature forwarding has a clear justification.
- `cargo check -p eggsec-cli --features <affected-feature>` passes for affected features.

## Phase 8: Add targeted tests for the refactor seams

Add tests where they are cheap and deterministic.

Recommended tests:

- `ThemeManager::new()` contains `dark` and `light`.
- `ThemeManager::set_theme("light")` succeeds and changes current theme.
- `ThemeManager::set_theme("missing")` returns false and preserves current theme.
- `TabStore::default()` or `TabStore::new()` initializes required tabs.
- Session restore still tolerates unknown theme names without panic.
- Quick switch state defaults are stable.
- Search state defaults are stable.

Avoid snapshot tests for full terminal rendering in this pass unless the project already has a stable snapshot harness.

Acceptance criteria:

- `cargo test -p eggsec-tui` passes.
- Tests cover the new state structs and theme module split enough to catch mechanical regressions.

## Phase 9: Final validation

Run:

```bash
cargo fmt --all
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo test -p eggsec-tui
```

Also run at least:

```bash
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-cli --features nse
```

If practical, run:

```bash
cargo check -p eggsec-cli --features full
```

Document any failures that are unrelated to this refactor.

## Implementation guidance for smaller models

Prefer mechanical, reversible changes over clever abstractions.

Keep compatibility methods if a direct migration would touch too many files.

Do not rename user-facing commands, tabs, keybindings, session fields, or config keys unless required by a compile error.

When moving files, preserve public re-exports from module roots to reduce churn.

Do not change behavior and architecture in the same edit. Move first, then simplify.

After each phase, run `cargo check -p eggsec-tui` before continuing.

## Expected follow-up after this plan

Once this simplification pass is complete, the next plan can implement Halloy-style theming on top of the cleaner structure. That later pass should add config-backed theme loading, theme validation, semantic token fallbacks, built-in theme registration, and optionally Halloy theme conversion/import. This pass should stop before that point.
