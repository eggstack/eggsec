# TUI (Terminal User Interface)

The TUI is a ratatui-based interactive terminal frontend for Eggsec. It provides real-time security scan monitoring and control across 33 feature-gated tabs, with a vim-like input model, themeable UI, daemon attach mode, and enforcement facade that mirrors CLI semantics.

See [overview.md](overview.md) for workspace context, [ui_model.md](ui_model.md) for frontend-neutral DTOs, [cli_commands.md](cli_commands.md) for CLI command dispatch, [dispatch.md](dispatch.md) for task dispatch architecture, and [runtime.md](runtime.md) for the runtime lifecycle.

## Role & Responsibilities

- Render the shell (tab bar, breadcrumb, content area, status bar) and all overlays.
- Route keyboard/mouse input through a three-layer decode pipeline (overlay → global → mode-specific).
- Spawn tasks via `eggsec::dispatch::dispatch_inner()` and receive results through typed channels and a runtime event reducer.
- Enforce policy via `TuiManual` / `TuiManualStrict` surfaces before any target-bearing dispatch.
- Persist sessions (auto-save + quick-save on exit) and restore theme, tab position, bookmarks.
- Connect to an external `eggsec-daemon` via Unix socket for remote-attach mode.

## Architecture

### Tab Inventory (33 variants)

The `Tab` enum at `tabs/mod.rs:142-176` declares 33 variants. `Tab::all()` at `tabs/mod.rs:191-220` uses `LazyLock` + `cfg_push_tabs!` to return 21 base tabs (always compiled) + 12 feature-gated tabs. `TAB_SPECS` at `tabs/spec.rs:67-662` has exactly 33 entries; the test `tab_specs_order_matches_enum_discriminants` (line 899) asserts `len == 33`.

| # | Variant | stable_id | Feature Gate | Category | Risk | Operation | direct_launch | Source Module |
|---|---------|-----------|-------------|----------|------|-----------|--------------|---------------|
| 0 | Recon | `recon` | — | Assessment | SafeActive | `recon` | no | `recon.rs` |
| 1 | Load | `load` | — | Traffic | SafeActive | `load-test` | no | `load.rs` |
| 2 | ScanPorts | `scan_ports` | — | Assessment | SafeActive | `scan-ports` | no | `scan_ports.rs` |
| 3 | ScanEndpoints | `scan_endpoints` | — | Assessment | SafeActive | `scan-endpoints` | no | `scan_endpoints.rs` |
| 4 | Fingerprint | `fingerprint` | — | Assessment | Passive | `fingerprint` | no | `fingerprint.rs` |
| 5 | Fuzz | `fuzz` | — | Assessment | Intrusive | `fuzz` | no | `fuzz.rs` |
| 6 | Waf | `waf` | — | Assessment | SafeActive | `waf` | no | `waf.rs` |
| 7 | WafStress | `waf_stress` | — | Assessment | Intrusive | `waf-stress` | no | `waf_stress.rs` |
| 8 | Scan | `scan` | — | Assessment | SafeActive | `scan-pipeline` | no | `scan.rs` |
| 9 | Resume | `resume` | — | History | SafeActive | — | no | `resume.rs` |
| 10 | Proxy | `proxy` | — | Traffic | Administrative | — | no | `proxy.rs` |
| 11 | Packet | `packet` | — | Traffic | Administrative | `packet` | **yes** | `packet.rs` |
| 12 | GraphQl | `graphql` | — | Assessment | Intrusive | `graphql` | no | `graphql.rs` |
| 13 | OAuth | `oauth` | — | Assessment | Intrusive | `oauth` | **yes** | `oauth.rs` |
| 14 | Cluster | `cluster` | — | Configuration | Administrative | — | **yes** | `cluster.rs` |
| 15 | Stress | `stress` | — | Assessment | Intrusive | `stress-test` | **yes** | `stress.rs` |
| 16 | Report | `report` | — | Reporting | Passive | — | no | `report.rs` |
| 17 | Settings | `settings` | — | Configuration | Administrative | — | no | `settings/main.rs` |
| 18 | History | `history` | — | History | Passive | — | no | `history.rs` |
| 19 | Dashboard | `dashboard` | — | Dashboard | Passive | — | no | `dashboard.rs` |
| 20 | Auth | `auth` | — | Assessment | Intrusive | `auth-test` | **yes** | `auth.rs` |
| 21 | Hunt | `hunt` | `advanced-hunting` | Assessment | Intrusive | `hunt` | **yes** | `hunt.rs` |
| 22 | Browser | `browser` | `headless-browser` | Assessment | Intrusive | `browser` | **yes** | `browser.rs` |
| 23 | Compliance | `compliance` | `compliance` | Reporting | SafeActive | `compliance` | no | `compliance.rs` |
| 24 | Storage | `storage` | `database` | Workflow | Administrative | `storage` | no | `storage.rs` |
| 25 | Integrations | `integrations` | `external-integrations` | Workflow | Administrative | `integrations` | no | `integrations.rs` |
| 26 | Workflow | `workflow` | `finding-workflow` | Workflow | Administrative | `workflow` | no | `workflow.rs` |
| 27 | Vuln | `vuln` | `vuln-management` | Workflow | SafeActive | `vuln` | no | `vuln.rs` |
| 28 | Wireless | `wireless` | `wireless` | Assessment | SafeActive | `wireless` | **yes** | `wireless.rs` |
| 29 | DbPentest | `db_pentest` | `db-pentest` | Assessment | Intrusive | `db-pentest` | **yes** | `db_pentest.rs` |
| 30 | Intercept | `intercept` | `web-proxy` | Traffic | Intrusive | `proxy-intercept` | **yes** | `intercept.rs` |
| 31 | C2 | `c2` | `c2` | Assessment | Intrusive | `c2` | **yes** | `c2.rs` |

**Summary**: 21 base + 12 gated = 33 total. 26 have operation IDs (enforcement evaluation). 12 are direct-launch (pre-dispatch policy gate in `handle_enter()`). 7 have no operation/task/descriptor (Resume, Proxy, Cluster, Report, Settings, History, Dashboard).

Tab dispatch uses the `tab_dispatch!` macro (`tabs/mod.rs:500-546`) which generates `as_tab_state`, `as_tab_state_mut`, `as_tab_render`, and `as_tab_input` methods. Feature-gated tabs fall back to `dashboard` when their feature is disabled.

### TabStore (`app/tab_store.rs`)

`TabStore` owns all tab instances as named fields (one per variant). When a feature is disabled, the gated field still exists but is only accessible through the `dashboard` fallback in the dispatch macro.

### Tab Traits (`tabs/mod.rs:563-627`)

| Trait | Methods | Purpose |
|-------|---------|---------|
| `TabState` | `state()`, `progress()`, `is_running()`, `has_selector_open()`, `reset()`, `set_error()`, `set_completed_message()` | State inspection and mutation |
| `TabRender` | `render()`, `render_overlays()`, `breadcrumb()` | Rendering |
| `TabInput` | 28 methods (see below) | Input handling |

`TabInput` provides: `handle_focus_next`, `handle_focus_prev`, `handle_char`, `handle_backspace`, `handle_delete`, `handle_enter`, `handle_escape`, `handle_up`, `handle_down`, `handle_left`, `handle_right`, `handle_paste`, `handle_copy`, `handle_word_forward`, `handle_word_backward`, `handle_home`, `handle_end`, `handle_top`, `handle_bottom`, `handle_autocomplete`, `handle_search`, `is_input_focused`, `is_at_left_edge`, `is_at_right_edge`, `stop`, `page_up`, `page_down`, `primary_target`.

### Theme System

50 packaged Halloy-format `.toml` themes are LZMA-compressed and embedded at compile time (`theme/packaged.rs:4`: `PACKAGED_THEMES_FILE_COUNT = 50`). Three built-in themes (`cyber-red`, `dark`, `light`) serve as defaults via `theme/builtin.rs`.

| Module | File | Purpose |
|--------|------|---------|
| `theme/palette.rs` | `ThemeMode`, `Theme` (37 color fields), `ThemeColors` | Theme data model |
| `theme/manager.rs` | `ThemeManager` | Registration, lookup, switching, metadata tracking (`FxHashMap<String, ThemeInfo>`) |
| `theme/loader.rs` | Parses Halloy `.toml` → `Theme`; `named_color()` for 27 CSS colors | File loading |
| `theme/install.rs` | Idempotent installer: packaged → `~/.config/eggsec/themes/` | Startup install |
| `theme/archive.rs` | LZMA decode for packaged blob | Decode |
| `theme/contrast.rs` | WCAG relative luminance, contrast ratio (min 4.5:1) | Validation |
| `theme/style.rs` | Semantic helpers: `safe`, `danger`, `muted`, `active_task`, `scope_match`, etc. | Render helpers |
| `theme/legacy.rs` | `tc!()` thread-local macro for backward compat | Legacy access |

**Background loading**: `load_and_install_themes()` runs in `std::thread::spawn`. The receiver, join handle, deferred restore request, and `ThemeLoadReason` (Startup or ManualReload) live in `ThemeLoadState`. Manual reload shows "Loading themes..." immediately; startup loads are silent. `App::update()` polls the channel. Regenerate via `python3 scripts/package_themes.py`.

**Theme preview/apply/cancel**: Opening the theme selector enters preview mode. Up/Down refreshes preview via `update_settings_theme_selector()`. Enter applies and quick-saves. Escape reverts to `applied_theme_id`. `ThemeManager.current_id()` provides the accessor.

### Component Library (`components/`)

| Component | File | Purpose |
|-----------|------|---------|
| `InputField` | `input.rs` | Text input with cursor, validation, UTF-8 byte-index invariant |
| `InputGroup` | `input.rs` | Focus-managed field group; `valid_focused_index()` stale-focus guard |
| `FormBuilder` | `input.rs` | Declarative form layout with `collect_dropdowns()` for overlay rendering |
| `Selector` | `selector.rs` | Dropdown with keyboard nav; `open()`/`close()`/`confirm()`/`cancel()` |
| `Checkbox` | `selector.rs` | Toggle checkbox |
| `RadioGroup` | `selector.rs` | Radio button group |
| `ProgressGauge` | `progress.rs` | Animated progress bar with spinner |
| `ScrollableText` | `scrollable.rs` | Scrollable text with scrollbar; empty-lines guard in scroll methods |
| `Popup` | `popup.rs` | Modal dialogs (confirm, help, info) |
| `empty_state_paragraph` | `empty_state.rs` | Empty state placeholder widget |

### Overlay System (`app/overlay.rs`)

`OverlayController` routes input through `topmost_overlay()`. Precedence (highest first):

1. `PolicyConfirm` — enforcement `RequireConfirmation` + manual override
2. `ConfirmPopup` — `PendingAction` confirmation (destructive UI actions)
3. `CommandPalette` — `Ctrl+P`
4. `QuickSwitch` — `Ctrl+X`
5. `Search` — `Ctrl+F`
6. `HttpOptions` — `h` key
7. `Help` — `Space`

Non-topmost overlays never receive input; overlay-local keys never leak.

### Search System (`search.rs`)

Global search (`Ctrl+F`) overlays a search popup. The search query is applied to the current tab's results via `TabInput::handle_search()`. Empty-state text: `"No results for '{query}'"` or `"Type to search..."`.

## Event Loop & Input Handling

### Event Loop (`app/runner.rs:64-177`)

`run_with_mode()` sets up crossterm raw mode, alternate screen, and mouse capture. The core loop (`run_app()`) follows `update() → draw() → input-check`:

1. `app.update()` drains runtime events via `TuiRuntimeAdapter::drain_and_reduce()`, then typed results from `progress_rx`/`result_rx`.
2. `app.auto_save_if_due()`.
3. `terminal.draw(|f| ui::draw(f, app))` only if `needs_redraw` or `pending_redraw`.
4. Input via non-blocking `EventStream::next().now_or_never()`. If no events, sleeps 10ms.

Exit calls `session_manager.save_quick()`.

### Key Processing Pipeline (`app/key_handler.rs`)

Three-layer decode, each returning `Vec<UiAction>`:

1. **Overlay decode** (`decode_topmost_overlay` → `OverlayController::decode`) — PolicyConfirm, ConfirmPopup, CommandPalette, QuickSwitch, Search, HttpOptions, Help.
2. **Global shortcuts** (`decode_global_shortcuts`) — `Ctrl+C`, `Ctrl+P`, `Ctrl+X`, `Ctrl+F`, `Ctrl+T`, `Ctrl+B`, `Ctrl+Z`, `Ctrl+Y`, `Shift+E`, `Space`, digit keys `1-9`/`0`, `gg` pending.
3. **Mode-specific** (`decode_mode_specific_input`) — Normal mode: hjkl, i, q, n/p, e, s, r, g/G. Insert mode: char input, backspace, delete, autocomplete, paste, Esc → Normal.

`App::apply_action()` / `apply_actions()` is the single mutation point for all key-driven UI changes.

### Input Modes (`app/input.rs`)

```rust
pub enum InputMode { Normal, Insert }
```

Normal mode: vim-like navigation. Insert mode: text input in focused field.

### The `is_running()` Guard Convention

Every input handler (`handle_up`, `handle_down`, `handle_left`, `handle_right`, `page_up`, `page_down`, `handle_focus_next`, `handle_focus_prev`, `handle_enter`, `handle_escape`, `handle_copy`, etc.) must check `!self.is_running()` before processing navigation or editing. This prevents state mutations during active scans. Violation has been a recurring class of bug across many tabs; all 33 tab implementations have been audited and fixed.

### The `reset()` Completeness Convention

`reset()` must restore ALL mutable state to defaults: `focus_area`, `selectors` (`.cancel()`), `inputs` (`.blur()`, `.clear()`), `checkboxes` (`.reset()`), `progress` counters, `results_view`, error strings, and mode flags. Missing resets cause stale state to leak across sessions.

## Daemon/Runtime Integration

### Runtime Binding (`app/mod.rs:139-137`)

`RuntimeBinding` wraps either an `EmbeddedRuntimeClient` or `DaemonRuntimeClient` behind the `TuiRuntimeClient` trait. Methods: `capabilities()`, `create_session()`, `list_sessions()`, `snapshot()`, `submit()`, `cancel()`, `cancel_active()`, `subscribe()`.

### Attach Mode (`app/runner.rs:179-272`)

CLI: `--runtime daemon --socket <path> [--session <id> | --new-session | --attach-latest]`.

`attach_daemon_session()` sends `DeclareClient { kind: ClientKind::Tui, label: "eggsec-tui" }`, creates or lists sessions, hydrates from `SessionSnapshot`, registers completed tasks in the adapter, and subscribes to events.

### Runtime Event Reducer (`app/runtime_adapter/mod.rs`)

Two-phase reduce/apply pattern:

1. `drain_and_reduce(rx)` borrows only the adapter and receiver → `Vec<TuiAction>`.
2. `apply_actions(actions, app)` is a free function taking `(Vec<TuiAction>, &mut App)`.

| RuntimeEvent | TuiAction(s) |
|---|---|
| `TaskStarted` | `TabStarted(tab, task_id)` |
| `TaskProgress` | `UpdateProgress(tab, completed, total)` |
| `TaskCompleted` | `TabCompleted(tab, outcome)` |
| `TaskFailed` | `TabError(tab, message)` |
| `TaskCancelled` | `TabCancelled(tab, reason)` |
| `TaskQueued`, `TaskLog`, `PolicyDecisionRequired`, `Audit` | No action (ignored) |

### TaskView Rendering

The TUI receives `TaskOutcome::Result(TaskResultEnvelope)` with `kind`/`summary`/`payload`. `OutcomeView::from(&outcome)` normalizes into a structured view. `renderer_for_kind(kind)` from `eggsec-ui-model` provides `ResultRendererDescriptor` with `title`, `summary_fields`, `artifact_kinds`, `supports_rich_tui`, `supports_json_detail`.

## Enforcement Facade

### TUI Surfaces

TUI uses `ExecutionSurface::TuiManual` (default, `ManualPermissive`) or `TuiManualStrict` (`ManualGuarded`). Toggle via `Ctrl+G`.

### EnforcementFacade (`app/enforcement_facade.rs`)

```rust
pub struct EnforcementFacade {
    pub state: TuiEnforcementState,
    pub(crate) pending_approved: Option<ApprovedOperation>,
}
```

Methods: `try_approve(desc)`, `evaluate_and_try_approve(desc)`, `take_cached_approval(desc)`, `confirm_override(descriptor, classes, reason)`, `audit_confirmed_override(...)`, plus delegation: `toggle_posture()`, `mode_label()`, `status_string()`, `preflight()`, `enforcement()`, `loaded_scope()`.

### Pre-Dispatch Gate

Central gate in `App::update()` before `spawn_task`. For direct-launch tabs, `handle_enter()` evaluates policy BEFORE calling the dispatcher — `Deny`/`RequireConfirmation` blocks before any side effect starts. `RequireConfirmation` uses highest-precedence `OverlayType::PolicyConfirm` with `PendingPolicyConfirmation` and reason input. On confirm, builds narrow `ManualOverride`, re-evaluates, records via `with_manual_override_record`.

### OperationMetadata as Source of Truth

Each `TabSpec` declares `operation: Option<&'static str>` mapping to a canonical `OperationMetadata` entry. `App::build_current_operation_descriptor()` (`app/operation.rs`) calls `eggsec::config::operation_metadata(op_id)` and generates the `OperationDescriptor` via `metadata.descriptor_for_target(target)`.

## Theming

### Packaged Themes Pipeline

`scripts/package_themes.py` compresses 50 `.toml` files into an LZMA blob embedded in `theme/packaged.rs` as `PACKAGED_THEMES_LZMA_BASE64`. On startup, `load_and_install_themes()` decodes the blob, installs missing themes to `~/.config/eggsec/themes/`, and loads all `.toml` files.

### Custom Theme Loading

Theme files are parsed by `theme/loader.rs` which maps Halloy TOML format to `Theme` with 37 color fields. Missing fields use defaults from built-in themes. Contrast validation (`theme/contrast.rs`) checks text/background and selected_text/selected pairs at 4.5:1 minimum. Low-contrast themes fall back to base theme with `FallbackAdjusted` status (non-fatal).

### Theme Metadata

`ThemeManager` stores `theme_info: FxHashMap<String, ThemeInfo>` with `id`, `display_name`, `mode` (Dark/Light), `source` (`BuiltIn | Packaged | Custom`), and `status` (`Loaded | FallbackAdjusted | Invalid(String) | Missing`). Query: `theme_info_list()`, `themes_with_status()`, `invalid_themes()`.

## Testing

### TestBackend Pattern

Visual regression uses `ratatui::backend::TestBackend` to render into an in-memory buffer, then `buffer_to_text()` (from `test_utils.rs`) converts to a string for assertion. This avoids snapshot files and allows `contains()` checks.

```rust
use ratatui::{backend::TestBackend, Terminal};
use crate::test_utils::buffer_to_text;

let mut app = create_test_app();
app.current_tab = Tab::Recon;
let backend = TestBackend::new(100, 24);
let mut terminal = Terminal::new(backend).unwrap();
terminal.draw(|f| draw(f, &mut app)).unwrap();
let text = buffer_to_text(terminal.backend().buffer());
assert!(text.contains("Mode:"));
```

### Test Counts

- `ui/tests.rs`: 14 tests (shell rendering, overlays, preflight indicators, empty states)
- `ui/shell.rs`: 8 tests (status bar, tab bar, breadcrumb)
- `tabs/core.rs`: 9 tests (field helpers, start/render patterns)
- `app/navigation.rs`: 16 tests (tab switching, edge detection)
- `tabs/handle_enter_regression.rs`: 40 table-driven tests across 12 tabs
- `tabs/input_accessibility.rs`: `#[cfg(test)]` module verifying unique input labels and focus traversal
- Total TUI crate: ~479 tests

### Regression Test Harness (`tabs/handle_enter_regression.rs`)

40 table-driven tests validate `handle_enter()` across all focus areas for 12 tabs: focused input blurs without starting, unfocused input with valid target starts, options toggle without starting, results area is no-op.

## Invariants & Gotchas

### Architecture Invariants

1. **No dispatch in TUI**: Worker dispatch lives in `eggsec::dispatch`. TUI submits via `spawn_task()` and receives results.
2. **Enforcement is central**: `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. TUI never bypasses it.
3. **Decode/apply split**: `KeyHandler` decodes to `Vec<UiAction>`; `App::apply_action()` applies. Testable independently.
4. **TabSpec is metadata source**: `TabSpec` carries title, stable_id, cli_command, category, risk_group, feature, operation, direct_launch. `Tab` methods delegate to `TabSpec`.
5. **Runtime dependency boundary**: `eggsec-runtime` must never depend on `eggsec`. Architecture guard enforces this.
6. **`eggsec-output` independence**: Must not depend on `eggsec` (engine) or `eggsec-runtime`.

### Conventions (from AGENTS.override.md)

1. **`is_running()` guards**: All input/navigation handlers must check `!self.is_running()` before processing.
2. **`reset()` completeness**: Must reset ALL state — focus_area, selectors (`.cancel()`), inputs (`.blur()`, `.clear()`), checkboxes, progress, results, error strings, mode flags.
3. **Bounds safety**: Use `.get(i)` not `chunks[i]`. Use `InputGroup::valid_focused_index()` not `self.focused` directly. Use `.first()` not `.get(0)`.
4. **No silent error suppression**: Never `let _ =` or `filter_map(|e| e.ok())`. Always `tracing::warn!`.
5. **FxHashMap/FxHashSet**: Use `rustc_hash::FxHashMap`/`FxHashSet` in performance paths, not std collections.
6. **Explicit `&Theme` params**: New rendering code should prefer explicit `&Theme` parameters over `tc!()` macro.
7. **TabWindow/TabSpan**: Use `TabWindow` for pagination, not raw tab count division. Never use `tab as usize` for indexing.
8. **Timeout wrappers**: All spawned tokio tasks need timeout wrappers (30-300s).
9. **Stale-focus guard**: Always use `InputGroup::valid_focused_index()` instead of direct `self.focused` indexing.

### Overlay Selector Containment

When an embedded Settings selector is open, normal-mode shortcuts are blocked via `has_settings_selector_open()` in `decode_normal_mode_input`. Only Up/Down, Enter, Escape, modifier keys, and Left/Right pass through.

### Entry Point

TUI launches automatically via `handle_no_command()` in `commands/handlers/mod.rs` when no subcommand is provided and stdout is a terminal.

### Key Bindings Summary

| Key | Action |
|-----|--------|
| `Ctrl+C` | Interrupt task or quit |
| `Ctrl+P` | Command palette |
| `Ctrl+X` | Quick switch (tab search) |
| `Ctrl+F` | Global search |
| `Ctrl+T` | Cycle all themes alphabetically |
| `Ctrl+B` | Bookmark current tab |
| `Ctrl+G` | Toggle Manual/Guarded enforcement posture |
| `Ctrl+Z` | Pause/resume active task updates |
| `Shift+E` | Export with format selection |
| `Space` | Toggle help overlay |
| `1-9`/`0` | Jump to tab by visible index |
| `gg`/`G` | Go to top/bottom |
| `n`/`p` | Next/prev tab |
| `hjkl`/arrows | Navigation |
| `i` | Enter insert mode |
| `Esc` | Return to normal / close overlay |
| `q` | Quit (no active task) |
| `e` | Export results |
| `s` | Save settings |

## Action Hints System (`app/action_hints.rs`)

Context-aware hints replace static help text. `ActionHint` contains `key` + `label` (e.g. `"C:stop"`). `get_action_hints(app)` computes hints with priority: running task → overlay-specific → insert-mode → tab-specific → settings section-aware. `format_hints()` renders the compact string. 16 unit tests cover all priority levels.

---

*Last verified against source: 2026-08-25*
