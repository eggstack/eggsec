# TUI (Terminal User Interface)

The TUI is a ratatui-based interactive terminal frontend for Eggsec. It provides real-time security scan monitoring and control across 33 feature-gated tabs, with a vim-like input model, themeable UI, daemon attach mode, and enforcement facade that mirrors CLI semantics.

See [overview.md](overview.md) for workspace context, [ui_model.md](ui_model.md) for frontend-neutral DTOs, [cli_commands.md](cli_commands.md) for CLI command dispatch, [dispatch.md](dispatch.md) for task dispatch architecture, and [runtime.md](runtime.md) for the runtime lifecycle.

**Corrections (2026-10-06 re-verification)**: the 33-variant tab table, `TabSpec` field values, 21+12 base/gated split, 27 operation-backed / 12 direct-launch / 6 operation-less summaries, 50 packaged themes, 37 theme color fields, 27 named CSS colors, 7-overlay precedence, and the guard rules (`.get(i)`, no-console-writer, `TerminalSession`, `block_on_ambient`) all re-verified as **correct**. Stale items fixed: shifted `tabs/mod.rs`/`spec.rs`/`runner.rs` line cites; `intercept.rs` → `intercept/`; dead `has_settings_selector_open()` → `has_any_tab_selector_open()`; incomplete global-shortcut list (Ctrl+U, Ctrl+D, Ctrl+/, Ctrl+V, `w`/`b`, `/`, `Shift+H`/`Shift+L`); per-file test counts (shell 7→16, core 96→98, navigation 53→55, action-hints 16→20, crate total ~983→1,128); `handle_enter_regression` described as table-driven (it is per-tab); and a new section for the custom NSE script dispatch path, which was previously undocumented.

## Role & Responsibilities

- Render the shell (tab bar, breadcrumb, content area, status bar) and all overlays.
- Route keyboard/mouse input through a three-layer decode pipeline (overlay → global → mode-specific).
- Spawn tasks via the shallow `TuiTaskDispatcher` adapter (`TaskKind → canonical request → dispatch::execute_canonical`, single engine executor owner shared with daemon execution) and receive results through typed channels and a runtime event reducer.
- Enforce policy via `TuiManual` / `TuiManualStrict` surfaces before any target-bearing dispatch.
- Persist sessions (auto-save + quick-save on exit) and restore theme, tab position, bookmarks.
- Connect to an external `eggsec-daemon` via Unix socket for remote-attach mode.

## Architecture

### Tab Inventory (33 variants)

The `Tab` enum at `tabs/mod.rs:154–188` declares 33 variants (discriminants 0..=32). `Tab::all()` at `tabs/mod.rs:203–232` uses `LazyLock` + `cfg_push_tabs!` to return 21 base tabs (always compiled) + 12 feature-gated tabs. `TAB_SPECS` at `tabs/spec.rs:113–721` has exactly 33 entries; the tests `test_tab_spec_count_matches_all_tab_variants` (`spec.rs:1141`, asserts `tab_specs().len() == all_variants.len()`) and `test_tab_specs_returns_all_33` (`spec.rs:1188`, asserts `len == 33`) pin the count.

| # | Variant | stable_id | Feature Gate | Category | Risk | Operation | direct_launch | Source Module |
|---|---------|-----------|-------------|----------|------|-----------|--------------|---------------|
| 0 | Recon | `recon` | — | Assessment | SafeActive | `recon` | no | `recon.rs` |
| 1 | Load | `load` | — | Traffic | SafeActive | `load-test` | no | `load.rs` |
| 2 | ScanPorts | `scan_ports` | — | Assessment | SafeActive | `scan-ports` | no | `scan_ports.rs` |
| 3 | ScanEndpoints | `scan_endpoints` | — | Assessment | SafeActive | `scan-endpoints` | no | `scan_endpoints.rs` |
| 4 | Fingerprint | `fingerprint` | — | Assessment | Passive | `fingerprint` | no | `fingerprint.rs` |
| 5 | Fuzz | `fuzz` | — | Assessment | Intrusive | `fuzz` | no | `fuzz.rs` |
| 6 | Waf | `waf` | — | Assessment | SafeActive | `waf-detect` | no | `waf.rs` |
| 7 | WafStress | `waf_stress` | — | Assessment | Intrusive | `waf-stress` | no | `waf_stress.rs` |
| 8 | Scan | `scan` | — | Assessment | SafeActive | `pipeline` | no | `scan.rs` |
| 9 | Resume | `resume` | — | History | SafeActive | `pipeline` | no | `resume.rs` |
| 10 | Proxy | `proxy` | — | Traffic | Administrative | — | no | `proxy.rs` |
| 11 | Packet | `packet` | `packet-inspection` (availability shell; always visible) | Traffic | Administrative | `packet` | **yes** | `packet.rs` |
| 12 | GraphQl | `graphql` | — | Assessment | Intrusive | `graphql` | no | `graphql.rs` |
| 13 | OAuth | `oauth` | — | Assessment | Intrusive | `oauth` | **yes** | `oauth.rs` |
| 14 | Cluster | `cluster` | — | Configuration | Administrative | — | **yes** | `cluster.rs` |
| 15 | Stress | `stress` | `stress-testing` (availability shell; always visible) | Assessment | Intrusive | `stress-test` | **yes** | `stress.rs` |
| 16 | Report | `report` | — | Reporting | Passive | — | no | `report.rs` |
| 17 | Nse | `nse` | `nse` | Assessment | SafeActive | `nse` | **yes** | `nse.rs` |
| 18 | Settings | `settings` | — | Configuration | Administrative | — | no | `settings/main.rs` |
| 19 | History | `history` | — | History | Passive | — | no | `history.rs` |
| 20 | Dashboard | `dashboard` | — | Dashboard | Passive | — | no | `dashboard.rs` |
| 21 | Hunt | `hunt` | `advanced-hunting` | Assessment | Intrusive | `hunt` | **yes** | `hunt.rs` |
| 22 | Browser | `browser` | `headless-browser` | Assessment | Intrusive | `browser` | **yes** | `browser.rs` |
| 23 | Compliance | `compliance` | `compliance` | Reporting | SafeActive | `compliance` | no | `compliance.rs` |
| 24 | Storage | `storage` | `database` | Workflow | Administrative | `storage` | no | `storage.rs` |
| 25 | Integrations | `integrations` | `external-integrations` | Workflow | Administrative | `integrations` | no | `integrations.rs` |
| 26 | Workflow | `workflow` | `finding-workflow` | Workflow | Administrative | `workflow` | no | `workflow.rs` |
| 27 | Vuln | `vuln` | `vuln-management` | Workflow | SafeActive | `vuln` | no | `vuln.rs` |
| 28 | Wireless | `wireless` | `wireless` | Assessment | SafeActive | `wireless` | **yes** | `wireless.rs` |
| 29 | Auth | `auth` | — | Assessment | Intrusive | `auth-test` | **yes** | `auth.rs` |
| 30 | DbPentest | `db_pentest` | `db-pentest` | Assessment | Intrusive | `db-pentest` | **yes** | `db_pentest.rs` |
| 31 | Intercept | `intercept` | `web-proxy` | Traffic | Intrusive | `proxy-intercept` | **yes** | `intercept/` (mod + render + types + utils) |
| 32 | C2 | `c2` | `c2` | Assessment | Intrusive | `c2` | **yes** | `c2.rs` |

**Summary**: 21 base + 12 gated = 33 total. 27 have operation IDs (enforcement evaluation). 12 are direct-launch (pre-dispatch policy gate in `handle_enter()`). 6 have no operation/task/descriptor (Proxy, Cluster, Report, Settings, History, Dashboard).

Resume shares the canonical `pipeline` operation: it runs the same stage set from a saved checkpoint, and `route_for_command_id` already maps the `resume` CLI command to `["pipeline"]` while the CLI handler resolves enforcement through that same registry. So Resume needs no `OperationMetadata` entry of its own, and two tabs may legitimately declare one operation.

Phase 0 parity resolutions (see `crates/eggsec-tui/src/parity.rs` and `crates/eggsec/tests/frontend_surface_matrix.rs`):
- Waf tab uses canonical `waf-detect` (`waf` remains a tested compatibility alias).
- Scan tab uses canonical `pipeline` (`scan-pipeline` remains a tested alias).
- Stress/Packet tabs declare canonical availability features (`stress-testing`, `packet-inspection`) but remain visible as unavailable shells when disabled (visibility != availability).
- Proxy (helper, no operation) vs Intercept (`proxy-intercept`, `web-proxy`) are distinct capabilities.
- `compliance`/`storage`/`integrations`/`workflow`/`vuln` are operation-backed TUI/runtime tabs; their CLI commands (where present) are helper-only.
- No TUI tabs for `waf-bypass`, `remote`, `search`, `mobile-static`, `mobile-dynamic`, `evasion`, `postex` (intentionally CLI/programmatic-only; `wireless-deauth` via Wireless active-mode override).

Tab dispatch uses the `tab_dispatch!` macro (`tabs/mod.rs:490–525`) which generates `as_tab_state`, `as_tab_state_mut`, `as_tab_render`, and `as_tab_input` methods. Feature-gated tabs fall back to `dashboard` when their feature is disabled.

### Surface Model (Phase 2)

`TabSpec` (`tabs/spec.rs`) is the single production owner for discoverable
tabs; the four-action pilot (`app/action_spec.rs`, `TUI_ACTION_SPECS`) was
deleted. The typed surface layer references canonical engine metadata instead
of copying IDs/risk/features as unconstrained strings:

| Type | Location | Meaning |
|------|----------|---------|
| `TuiSurfaceRoute` | `tabs/spec.rs` | `Operation(canonical_id)`, `Multiplexer("wireless")` (passive scan vs `wireless-deauth` active attack), `Helper`, `Lifecycle`, `UiOnly` |
| `TabAvailability` | `tabs/spec.rs` | `Available`, `Unavailable { required_feature }` (Stress/Packet always-visible shells), `UiOnly`, `NotSupportedOnTui` |
| `PaletteResolution` | `tabs/spec.rs` | `SelectTab`, `Unavailable { required_feature }`, `Unknown` |
| `TabSpec::aliases()` / `palette_command()` | `tabs/spec.rs` | Every parseable string vs the one discoverable command; hidden aliases (`scan-pipeline`, `waf-detect`, `o-auth`, `wifi`, `portscan`) stay parseable but never pollute discovery |
| `resolve_palette_command()` | `tabs/spec.rs` | Single-owner static-slice lookup (no manual match, no hash registry) |
| `tabs/surface.rs` | catalog queries | `discoverable_palette_commands()` (one per `Tab::all()`, in order), alias-uniqueness, help/discovery agreement, shell invariants |
| `app/palette.rs` | action bridge | `parse_palette_action()` / `global_action_for()` map palette strings to the same `UiAction` the key handler emits; `PaletteAction` (`SelectTab`/`Global`/`Unavailable`/`Unknown`) separates navigation/local actions from execution effects |
| `app/command.rs` | dispatcher | `command_to_tab()` (available-only) + `execute_command()` routed through `parse_palette_action()`; unavailable shells notify the required feature instead of silent divergence |

`copy-cli` is a semantic round trip (`app/operation.rs: cli_argv()` argv
vector, quoting only at the clipboard boundary via `shell_escape`):
TUI state -> argv -> real Clap `Cli::try_parse_from` -> `cli_adapters` canonical
request -> semantic equality. UI-only tabs return explicit `None`. Format
mapping emits only flags the real tree accepts (`--json` for Json-mapped tabs;
`--format` only for commands declaring it; TUI-only `Max Payloads` omitted).
`reload-scope` was removed from discoverable palette/help (Phase 2.8 acceptable
alternative); direct invocation explains the restart-required contract without
implying a live reload.

### TabStore (`app/tab_store.rs`)

`TabStore` owns all tab instances as named fields (one per variant). When a feature is disabled, the gated field still exists but is only accessible through the `dashboard` fallback in the dispatch macro.

### Tab Traits (`tabs/mod.rs:590–676`)

| Trait | Methods | Purpose |
|-------|---------|---------|
| `TabState` | `state()`, `progress()`, `is_running()`, `has_selector_open()`, `reset()`, `set_error()`, `set_completed_message()` (plus a `#[cfg(test)] set_state()`) | State inspection and mutation |
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

### Event Loop (`app/runner.rs`)

Terminal lifecycle is owned by one cleanup-safe session (`TerminalSession`
in `app/runner.rs`, Phase B). Acquisition uses `ratatui::try_init()` (raw
mode + alternate screen + panic-hook registration); mouse capture is an
Eggsec-owned extra enabled only after init succeeds, with rollback of the
already-acquired state if the enable fails. The core loop (`run_app()`)
follows `update() → draw() → input-check`:

1. `app.update()` drains runtime events via `TuiRuntimeAdapter::drain_and_reduce()`, then typed results from `progress_rx`/`result_rx`.
2. `app.auto_save_if_due()`.
3. `terminal.draw(|f| ui::draw(f, app))` only if `needs_redraw` or `pending_redraw`.
4. Input via non-blocking `EventStream::next().now_or_never()`. If no events, sleeps 10ms.

`run_with_mode()` acquires the session, runs the guarded body
(`run_tui_body`: app/runtime setup incl. daemon Tokio work, event loop,
quick-save), then restores and returns the combined outcome — so no `?`
after acquisition can strand terminal modes. Teardown runs every step
independently (one failure never blocks later steps; first failure stays
primary with the rest as context) and is idempotent, with a silent
best-effort `Drop` fallback for early return/unwind. The Ratatui panic hook
runs at panic time (before unwinding) and restores raw/alternate-screen
state; the guard drop then pairs mouse capture during unwind. No competing
hooks are installed (unit tests use injectable cleanup operations, never
repeated `try_init`, so hooks cannot stack recursively).

Fatal loop errors propagate to the caller and are presented only after the
alternate screen is gone (the CLI prints the returned error
post-restoration); they are never logged-and-converted to success.
Quick-save failure is non-fatal by explicit rule: it never overwrites a
body failure nor changes the exit status, and is reported through the
tracing diagnostic path (silent under the TUI no-console policy).

Daemon sync/async bridging reuses the ambient runtime
(`block_on_ambient`): the CLI runs under `#[tokio::main]`, where a nested
`tokio::runtime::Runtime::new()` panics. A throwaway runtime is built only
for standalone hosts (tests, non-Tokio embeddings) with no ambient
runtime. Exit calls `session_manager.save_quick()` inside the guarded body.

### Single-Terminal-Writer Ownership (Phase A)

While the rich TUI owns the alternate screen, Ratatui/Crossterm is the only
writer to the controlling terminal:

- The CLI resolves rich-TUI launch intent before subscriber construction
  (`rich_tui_launch_requested` in `eggsec-cli/src/main.rs`) and initializes
  logging with `ConsoleLogging::Disabled` (`init_logging_with_console`).
  Rich TUI mode installs no stdout/stderr tracing formatter; tracing call
  sites remain valid diagnostics but emit no terminal bytes.
- Stdout *and* stderr are both forbidden as side channels while the alternate
  screen is owned; stderr is never used as an alternate TUI writer.
- Production `eggsec-tui` code contains no direct `println!` / `eprintln!` /
  `print!` / `eprint!` / `dbg!` on live paths (guard Check 138). The former
  post-`EnterAlternateScreen` `eprintln!` small-terminal warning now routes
  through `small_terminal_warning_message()` into the in-frame notification
  overlay (`NotificationSeverity::Warning`); the responsive layout plus the
  `is_terminal_too_small` fallback already render the degraded UI.
- Recoverable user-relevant runner conditions use existing TUI state, never
  direct writes and never duplicated across surfaces: malformed TUI config →
  notification overlay (tracing stays as diagnostic); daemon attach failure →
  per-tab error via `stop_with_message` (tracing stays as diagnostic);
  transient event errors / event-stream end → tracing diagnostic + graceful
   quit (no visible surface would persist); post-loop quick-save failure →
   tracing diagnostic, non-fatal by rule (see Cleanup-Safe Lifecycle
   section below).
- No new default persistent TUI log directory was introduced; file logging
  still occurs only when an existing caller passes `log_dir` (agent memory
  dir), as file-only JSON when console is disabled.

### Cleanup-Safe Lifecycle and Child-Process Closure (Phase B)

- Ratatui owns terminal bytes during rich mode (alternate screen); tracing
  sink selection is a frontend/process-host responsibility (console disabled
  for TUI launches per the section above).
- Terminal lifecycle is cleanup-safe: one `TerminalSession` owner, guarded
  body, independent/idempotent teardown, silent `Drop` fallback, no
  competing panic hooks (see Event Loop). User-facing fatal output occurs
  only after restoration.
- Child-process output reachable from the TUI is captured, not inherited.
  Whole-workspace audit (`rg -n
  'std::process::Command|tokio::process::Command|Command::new|Stdio::inherit'
  crates/`, 2026-09-20): `crates/eggsec-tui/src` contains no process-spawn
  sites at all; every production site uses `Command::output()` capture or
  explicit `Stdio::piped()` consumed by the operation (NSE
  `wrappers::process_exec` / `io.popen` / `nmap` probes, wireless `iwlist`,
  recon `git` history reads, transparent-proxy `iptables`, distributed
  remote-command executor, mobile-lab `adb`/`frida` probes). Zero
  `Stdio::inherit()` sites exist workspace-wide for stdout/stderr. No
  TUI-reachable process inherits stdout/stderr while rich mode is active, so
  no new process wrapper was introduced (isolated sites only; guard Check
  139a pins `Stdio::inherit()` out of `eggsec-tui`).
- Changed child-output capture is bounded where output can be unbounded:
  no process-execution call sites were changed in this phase (all already
  capture), so no new bounding was required; the pre-existing
  capability-gated NSE `process_exec` path remains the only owner of
  untrusted child output and stays sandbox-gated.
- PTY coverage (`scripts/tui_pty_smoke.py`, `make test-tui-pty`, wired into
  `make check-full` / Deep Checks): Unix/Linux-scoped stdlib-`pty` smoke
  asserting a representative config-load warning is absent from the raw PTY
  stream, alternate-screen entry/exit is present, and the child exits 0 —
  plus a daemon-attach-failure case proving guarded-body restoration. Named
  platform skip on Windows; PTY coverage does not replace unit/architecture
  guards (session unit tests use injectable cleanup operations).

See [logging.md](logging.md) for the console emission policy and
`docs/CI_ARCHITECTURE_GUARDS.md` for the Check 138/139 regression guards.

### Key Processing Pipeline (`app/key_handler.rs`)

Three-layer decode, each returning `Vec<UiAction>`:

1. **Overlay decode** (`decode_topmost_overlay` → `OverlayController::decode`) — PolicyConfirm, ConfirmPopup, CommandPalette, QuickSwitch, Search, HttpOptions, Help.
2. **Global shortcuts** (`decode_global_shortcuts`) — `Ctrl+C`, `Ctrl+X`, `Ctrl+U`/`Ctrl+D` (and bare PageUp/PageDown), `Ctrl+/`, `Ctrl+P`, `Ctrl+F`, `Ctrl+T`, `Ctrl+G`, `Ctrl+V`, `Ctrl+Z`, `Ctrl+Y`, Home/End, arrows, `Esc`, `Tab`/`Shift+BackTab`, `Enter`, plus the `gg` pending sequence. Normal-mode-only keys (`Space`, `Ctrl+B`, digits, `hjkl`, …) live in layer 3.
3. **Mode-specific** (`decode_mode_specific_input`) — Normal mode: `hjkl`, `i`, `q`, `y`, `w`/`b`/`B`, `n`/`N`/`p`, `Shift+H`/`Shift+L`, `e`, `s` (Settings only), `d` (History only), `r`, `/`, `G`/`g`. Insert mode: char input, backspace, delete, autocomplete, paste, Esc → Normal.

`App::apply_action()` / `apply_actions()` is the single mutation point for all key-driven UI changes.

### Input Modes (`app/input.rs`)

```rust
pub enum InputMode { Normal, Insert }
```

Normal mode: vim-like navigation. Insert mode: text input in focused field.

### The `is_running()` Guard Convention

Every input handler (`handle_up`, `handle_down`, `handle_left`, `handle_right`, `page_up`, `page_down`, `handle_focus_next`, `handle_focus_prev`, `handle_enter`, `handle_escape`, `handle_copy`, etc.) must check `!self.is_running()` before processing navigation or editing. This prevents state mutations during active scans. Violation has been a recurring class of bug across many tabs.

The user-visible symptom is **dead input**, not a stale internal field: a focus handler that runs mid-scan moves the focus ring, but `handle_char`/`handle_backspace` still refuse to edit because `is_running` is true, so the operator watches a focus indicator move while every keystroke is silently discarded. The scan must be cancelled before the form is usable again.

All three shared input macros (`tab_input_2area!`, `tab_input_3area!`, `tab_input_narea!`) and every hand-written `impl TabInput` now carry the guard. `tabs/mid_run_navigation.rs` pins the invariant: it drives every runnable tab into `Running` and asserts that no navigation key moves focus off the input area, plus a companion test that navigation still works while idle (so the guard cannot be applied unconditionally).

`tabs/mod.rs:609` exposes a `#[cfg(test)] fn set_state` on `TabState`, generated by `tab_state_boilerplate!` and by hand for the tabs with a manual `TabState` impl. Production code never calls it — a tab reaches `Running` only through a dispatch. Tabs with no task lifecycle (`Dashboard`, `History`, `Settings`) report `AppState::Idle` unconditionally and have no hook.

### The Tab-Entry Focus Invariant

`InputGroup::new()` leaves every field unfocused while tabs default their focus area to their input area, so a freshly entered tab would show a focus ring on a field that silently discards input. `App::sync_input_focus_for_current_tab()` calls `TabInput::ensure_input_focus()` on every tab-entry path to repair that.

**Every** `TabInput` impl must provide the hook. Tabs generated through the shared macros get it from `tab_input_boilerplate!`; `tab_input_indexed!` and `tab_input_custom!` generate it too. Tabs with a hand-written `impl TabInput` must implement it themselves, and the hook is a no-op when the tab opens on a non-input control (a selector or a section list). `core::ensure_group_field_focused` is the `InputGroup` counterpart of `core::ensure_first_field_focused` for tabs that own a group directly.

`tabs/tab_entry_focus.rs` pins this: every tab must leave a drivable control focused on entry, and re-entering must not change that. The documented exceptions are `Dashboard` and `History` (no input fields at all) and `Settings` (opens on its section list, which renders its own `▶` focus marker).

### The `reset()` Completeness Convention

`reset()` must restore ALL mutable state to defaults: `focus_area`, `selectors` (`.cancel()`), `inputs` (`.blur()`, `.clear()`), `checkboxes` (`.reset()`), `progress` counters, `results_view`, error strings, and mode flags. Missing resets cause stale state to leak across sessions.

### Scan Session Store and the Resume Picker

Three unrelated "session" concepts exist and must not be conflated:

| Concept | Type | Location | Role |
|---|---|---|---|
| Runtime session | `eggsec_runtime::session::SessionSnapshot` | `crates/eggsec-runtime/src/session.rs` | in-memory runtime/daemon state; not a file |
| TUI UI state | `eggsec_tui::session::SessionState` | `crates/eggsec-tui/src/session.rs` | bookmarks/theme/last-tab; listed by the History tab |
| Scan checkpoint | `eggsec::pipeline::session::PipelineSession` | `crates/eggsec/src/pipeline/session.rs` | the only thing resume consumes |

`pipeline::session::default_session_dir()` resolves `EGGSEC_SESSION_DIR`, then
the platform data dir + `scan-sessions`. It is deliberately *not* the TUI's
`sessions/` directory: the schemas are unrelated, so sharing one directory would
make every checkpoint fail to deserialize as UI state and vice versa.
`list_sessions()` projects each readable checkpoint to a `SessionEntry` (target,
stage counts, mtime) for the picker, skipping unreadable files with a
`tracing::warn!` so one corrupt file cannot hide every other session.

Checkpoints are opt-in, from both directions. `PipelineParams.session_path` is
`None` by default, so a TUI scan writes nothing unless it asks. On the CLI,
`--save-session` opts a scan in; without it the only checkpoint that exists is
the long-standing derivation from a `.session.json` `--output`, which is
preserved exactly. A failure to create the store under `--save-session` is
logged at `error` rather than swallowed, so a scan the operator asked to be
resumable never silently runs without a checkpoint.

`store_path_for()` sanitizes the target into a single filename component
(no separators, never dot-prefixed, so a checkpoint stays visible in a plain
directory listing) and appends a UTC timestamp so repeat scans of one host do not
overwrite each other.

The picker keeps a manual path field because checkpoints written outside the
store are still reachable; the manual path deliberately takes precedence over the
highlighted row.

### Checkpoint Outcomes

A checkpoint is only meaningful for an *interrupted* scan, so the picker must
distinguish three end states rather than guessing from `remaining_stages`,
which is empty both for a clean finish and for a run that died on its last
attempted stage:

| State | `remaining_stages` | `finalized` | `failed_stages` | Picker |
|---|---|---|---|---|
| In progress | non-empty | `false` | — | resumable, "N left" |
| Clean finish | empty | `true` | empty | "complete", not resumable |
| Ended with failures | empty | `true` | non-empty | "N failed — re-run to retry", not resumable |
| Aborted before final save | empty | `false` | — | "stopped early", not resumable |

`finalized` and `failed_stages` are `#[serde(default)]`, so checkpoints written
before these fields existed still load and are read as *not* finalized — the
safe direction, since such a file is offered as resumable rather than declared
complete. `Pipeline::run()` only reaches its final save on the success path, so
a hard error leaves a mid-run checkpoint behind, which is why the aborted case
exists.

`App::sync_input_focus_for_current_tab()` refreshes the list on every tab-entry
path. That is a pull, not a poll: nothing refreshes while the tab sits open.

Resume dispatch reuses the existing `"pipeline"` result renderer, so a resumed
run renders through the same path as a scan. `TaskKind::Resume::canonical_target()`
is `None` by design (the target lives in the checkpoint), and the tab supplies it
from the selected entry for the enforcement descriptor — the same thing the CLI
handler does before dispatch.

## Daemon/Runtime Integration

### Runtime Binding (`app/mod.rs:152`)

`RuntimeBinding` wraps either an `EmbeddedRuntimeClient` or `DaemonRuntimeClient` behind the `TuiRuntimeClient` trait. Methods: `capabilities()`, `create_session()`, `list_sessions()`, `snapshot()`, `submit()`, `cancel()`, `cancel_active()`, `subscribe()`.

### Attach Mode (`app/runner.rs:415–507`)

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
    pending_approved: Option<CachedApproval>, // exact descriptor + scope/policy/surface/override generation
}
```

Phase 0.1 exact binding: cached reuse requires `ApprovedOperation::matches_descriptor()` (derived `PartialEq`, future fields automatic) plus unchanged scope fingerprint, policy hash, surface/profile, and manual-override state. Stale tokens are discarded for fresh evaluation; engine `validate_request_binding` remains the final gate. `toggle_posture()` and override changes invalidate the cache; `invalidate_cached_approval()` covers future live reload.

Methods: `try_approve(desc)`, `evaluate_and_try_approve(desc)`, `take_cached_approval(desc)`, `set_cached_approval(approved)`, `clear_cached_approval()`, `invalidate_cached_approval()`, `confirm_override(descriptor, classes, reason)`, `audit_confirmed_override(...)`, plus delegation and read accessors: `state()`, `state_mut()`, `toggle_posture()`, `mode_label()`, `status_string()`, `scope_label()`, `allow_rule_count()`, `exclusion_rule_count()`, `is_guarded()`, `preflight()`, `enforcement()`, `loaded_scope()`.

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

Counts below are `#[test]` / `#[tokio::test]` attributes in source (no cargo run for this re-verification).

- `ui/tests.rs`: 16 tests (shell rendering, overlays, preflight indicators, empty states)
- `ui/shell.rs`: 16 tests (status bar, tab bar, breadcrumb, notification-priority status field)
- `tabs/core.rs`: 98 tests (field helpers, start/render patterns, per-tab cases)
- `app/navigation.rs`: 55 tests (tab switching, edge detection)
- `tabs/handle_enter_regression.rs`: 40 tests across 12 tabs (GraphQl, OAuth, Recon, Load, ScanPorts, Stress, Packet, Waf, Cluster, Dashboard, Settings, History)
- `tabs/input_accessibility.rs`: `#[cfg(test)]` module verifying unique input labels and focus traversal
- `tabs/mid_run_navigation.rs`: mid-run `is_running()` navigation invariant + the idle counter-test
- `tabs/tab_entry_focus.rs`: per-tab entry-focus invariant
- `app/task_management.rs`: 37 tests over feature-gated runtime request builders (db-pentest, intercept, C2, NSE) plus canonical-conversion round trips
- `app/action_hints.rs`: 20 tests covering hint priority levels
- Total TUI crate `src/`: **1,128** test attributes across 121 `.rs` files

### Runtime Request Builders (`app/task_management.rs`)

Each runnable tab implements `TaskBuilder::build_run_request()` to construct a `RunRequest` (a `TaskKind` + `RuntimeSurface::TuiManual`) from current tab state. The builders are shallow adapters: they map UI state to the runtime DTO and rely on `CanonicalOperationRequest::from_task_kind` for canonical identity/target/validation. They introduce no TUI-local canonicalization rules.

Semantic rules for safety-relevant fields:
- Map TUI-exposed controls explicitly (e.g. `dry_run`/`advanced` toggles on the DbPentest tab, `dry_run`/`max_flows` on Intercept).
- For fields the TUI does not expose, use the runtime's canonical absence semantics (`None`) so the engine applies its documented safe default — never invent a permissive value to satisfy compilation.
- `detect_db_type_from_target()` infers the mandatory `db_type` from the connection-string scheme (the tab has no db-type control); unknown schemes return `None` from the builder rather than fabricating a value.
- `parse_listen_addr()` splits the Intercept listen address into typed host/port components without silently coercing missing pieces.

### Custom NSE Script Dispatch (Nse tab)

The Nse tab can dispatch an operator-supplied script path, not only a named
built-in. The path crosses `NseParams::custom_script` (`eggsec-runtime/src/request.rs:327–331`,
`#[serde(default)]` so pre-existing serialized requests still decode) and
`dispatch::canonical_execution` forwards it to `run_nse` instead of pinning
`None` (`canonical_execution.rs:1584–1592`).

`NseTab::start()` (`tabs/nse.rs:520–549`) still refuses a whitespace-only path,
which would otherwise resolve as a literal filename, and reports it as a
`TabError::Config` rather than silently falling back to the selected built-in.
`NseTab::uses_custom_script()` (`tabs/nse.rs:556–558`) is true when a path is set
or the selector is on `custom`. Resolution stays with the engine's
`NseScriptSource::File` / `ScriptResolver` (allow-script-files → existence →
extension allowlist → canonical root containment), not a TUI filesystem read.

### Feature-Profile Verification

- `scripts/check-features-individual.sh` mechanically enumerates every declared `eggsec-tui` feature (other than `default`/`full`) plus the `eggsec-tui/full` aggregate. Adding a TUI feature to `Cargo.toml` without sweep coverage fails the orphan guard. TUI profiles need no system prerequisites (pnet is pure-Rust, openssl is vendored, wireless-tools is runtime-only).
- `make check-feature-profiles` includes the broad dependency-light TUI profile (`db-pentest,web-proxy,c2` check + lib tests) for fast cross-feature compile-drift detection between weekly deep sweeps. `eggsec-tui/full` remains a Deep Checks gate.
- Guard 98 (`scripts/check-architecture-guards.sh`) pins these three invariants; DTO field-level contracts belong to Rust compilation and the builder semantic tests, not to grep guards.

### Regression Test Harness (`tabs/handle_enter_regression.rs`)

40 tests validate `handle_enter()` across all focus areas for 12 tabs: focused input blurs without starting, unfocused input with valid target starts, options toggle without starting, results area is no-op. They are written per tab rather than table-driven, because each tab has its own focus-area enum.

## Invariants & Gotchas

### Architecture Invariants

1. **No dispatch in TUI**: Worker dispatch lives in `eggsec::dispatch`. TUI submits via `spawn_task()` and receives results.
2. **Enforcement is central**: `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. TUI never bypasses it.
3. **Decode/apply split**: `KeyHandler` decodes to `Vec<UiAction>`; `App::apply_action()` applies. Testable independently.
4. **TabSpec is metadata source**: `TabSpec` carries `tab`, `stable_id`, `title`, `cli_command`, `description`, `help_text`, `breadcrumb_label`, `category`, `risk_group`, `feature`, `operation`, `direct_launch`, plus the reserved/test-only `supports_run` / `supports_export` / `supports_help` / `has_settings` flags. `Tab` methods delegate to `TabSpec`.
5. **Runtime dependency boundary**: `eggsec-runtime` must never depend on `eggsec`. Architecture guard enforces this.
6. **`eggsec-output` independence**: Must not depend on `eggsec` (engine) or `eggsec-runtime`.

### Conventions (from AGENTS.override.md)

1. **`is_running()` guards**: All input/navigation handlers must check `!self.is_running()` before processing — including `handle_focus_next`/`handle_focus_prev`/`handle_up`/`handle_down`, which mutate focus mid-scan. Pinned by `tabs/mid_run_navigation.rs`.
2. **`reset()` completeness**: Must reset ALL state — focus_area, selectors (`.cancel()`), inputs (`.blur()`, `.clear()`), checkboxes, progress, results, error strings, mode flags.
3. **Tab-entry focus**: Every `TabInput` impl must provide `ensure_input_focus`; otherwise a freshly entered tab discards all input. Pinned by `tabs/tab_entry_focus.rs`.
4. **Bounds safety**: Use `.get(i)` not `chunks[i]`. Use `InputGroup::valid_focused_index()` not `self.focused` directly. Use `.first()` not `.get(0)`.
5. **No silent error suppression**: Never `let _ =` or `filter_map(|e| e.ok())`. Always `tracing::warn!`.
6. **FxHashMap/FxHashSet**: Use `rustc_hash::FxHashMap`/`FxHashSet` in performance paths, not std collections.
7. **Explicit `&Theme` params**: New rendering code should prefer explicit `&Theme` parameters over `tc!()` macro.
8. **TabWindow/TabSpan**: Use `TabWindow` for pagination, not raw tab count division. Never use `tab as usize` for indexing.
9. **Timeout wrappers**: All spawned tokio tasks need timeout wrappers (30-300s).
10. **Stale-focus guard**: Always use `InputGroup::valid_focused_index()` instead of direct `self.focused` indexing.
11. **Single-writer terminal rule**: Never `println!` / `eprintln!` / `print!` / `eprint!` / `dbg!` or touch `stdout` / `stderr` directly in production TUI code. Keep `tracing` diagnostics; surface user-actionable conditions through `Notification`, per-tab error, popup, or status state. See the Single-Terminal-Writer section above.
12. **Cleanup-safe lifecycle**: Terminal setup/teardown lives in `TerminalSession` (`app/runner.rs`) over `ratatui::try_init()`; never reintroduce open-coded raw/alternate-screen calls, nested `tokio::runtime::Runtime::new()` on the daemon path (use `runner::block_on_ambient`), competing panic hooks, or `Stdio::inherit()`. Guard Check 139 pins this.

### Overlay Selector Containment

Embedded selectors are not overlays (`topmost_overlay()` returns `None` while one is open), so the normal-mode decoder is gated by `App::has_any_tab_selector_open()` (`app/mod.rs:1254`, used at `app/key_handler.rs:238`). While any tab selector is open, every normal-mode key decodes to `UiAction::Noop` except `j`/`k`, which pass through for Vim-style selector navigation. (The older per-tab `has_settings_selector_open()` guard no longer exists.) Tabs expose their own state through `TabState::has_selector_open()`.

### Entry Point

TUI launches from `eggsec-cli/src/main.rs` when no subcommand is provided and stdout is a terminal (`rich_tui_launch_requested(has_command, stdout_is_terminal, tui_feature)`; the call site passes `cli.command.is_some()` for `has_command` — guard Check 139d). Launch intent is resolved before logging initialization so the subscriber can be installed with `ConsoleLogging::Disabled`.

### Key Bindings Summary

| Key | Action |
|-----|--------|
| `Ctrl+C` | Interrupt task or quit |
| `Ctrl+P` | Command palette |
| `Ctrl+X` | Quick switch (tab search) |
| `Ctrl+F` | Global search (re-run when already open) |
| `Ctrl+T` | Cycle all themes alphabetically |
| `Ctrl+G` | Toggle Manual/Guarded enforcement posture |
| `Ctrl+B` | Bookmark current tab |
| `Ctrl+Z` | Pause active task updates (no-op at idle) |
| `Ctrl+U` / `Ctrl+D` | Page up / page down |
| `Ctrl+V` | Paste (idle only) |
| `Ctrl+Y` | Copy, or resume a paused task |
| `Ctrl+/` | Toggle help overlay |
| `Shift+E` | Cycle export format |
| `Space` | Toggle help overlay |
| `1-9`/`0` | Jump to tab by visible index |
| `gg`/`G` | Go to top/bottom |
| `n`/`p`/`N` | Next/prev tab |
| `Shift+H`/`Shift+L` | Prev/next tab |
| `hjkl`/arrows | Navigation |
| `w`/`b`/`B` | Word forward/backward |
| `i` | Enter insert mode |
| `/` | Tab-local search |
| `Esc` | Return to normal / close overlay |
| `q` | Quit (no active task) |
| `y` | Copy |
| `e` | Export results |
| `s` | Save settings (Settings tab, idle) |
| `d` | Delete history entry (History tab, idle) |
| `r` | Reset current tab (Settings > Theme reloads themes) |

## Action Hints System (`app/action_hints.rs`)

Context-aware hints replace static help text. `ActionHint` contains `key` + `label` (e.g. `"C:stop"`). `get_action_hints(app)` computes hints with priority: running task → overlay-specific (including `HttpOptions`) → insert-mode → tab-specific → settings section-aware. `format_hints()` renders the compact string. 20 unit tests cover all priority levels.

---

*Last verified against source: 2026-10-06 (full re-verification of tab inventory, `TAB_SPECS` field values, trait/line cites, theme counts, overlay precedence, key-decoding layers, selector containment, and source-level test counts); earlier passes 2026-08-25 / 2026-09-20 / 2026-09-22 / 2026-09-25*
