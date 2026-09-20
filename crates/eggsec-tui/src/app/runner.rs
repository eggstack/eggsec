use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, MouseEvent, MouseEventKind},
    execute,
};
use ratatui::Terminal;
use std::io;

use super::App;
use super::InputMode;
use super::KeyHandler;
use crate::state;
use crate::tabs::{Tab, TabWindow};
use crate::ui;
use crate::ui::{LAYOUT_MARGIN, TAB_BAR_HEIGHT};
use crate::RuntimeMode;
use eggsec_runtime::request::TaskKind;

fn compute_tab_area(term_width: u16) -> ratatui::layout::Rect {
    ratatui::layout::Rect {
        x: LAYOUT_MARGIN,
        y: LAYOUT_MARGIN,
        width: term_width.saturating_sub(LAYOUT_MARGIN * 2),
        height: TAB_BAR_HEIGHT,
    }
}

/// Map a `TaskKind` to its originating `Tab` for snapshot hydration.
fn tab_for_task_kind(kind: &TaskKind) -> Tab {
    match kind {
        TaskKind::PortScan(_) => Tab::ScanPorts,
        TaskKind::EndpointScan(_) => Tab::ScanEndpoints,
        TaskKind::Fingerprint(_) => Tab::Fingerprint,
        TaskKind::Fuzz(_) => Tab::Fuzz,
        TaskKind::Waf(_) => Tab::Waf,
        TaskKind::WafStress(_) => Tab::WafStress,
        TaskKind::Pipeline(_) => Tab::Scan,
        TaskKind::Recon(_) => Tab::Recon,
        TaskKind::LoadTest(_) => Tab::Load,
        TaskKind::StressTest(_) => Tab::Stress,
        TaskKind::PacketCapture(_) => Tab::Packet,
        TaskKind::PacketTraceroute(_) => Tab::Packet,
        TaskKind::PacketSend(_) => Tab::Packet,
        TaskKind::GraphQl(_) => Tab::GraphQl,
        TaskKind::OAuth(_) => Tab::OAuth,
        TaskKind::AuthTest(_) => Tab::Auth,
        TaskKind::Nse(_) => Tab::Nse,
        TaskKind::Hunt(_) => Tab::Hunt,
        TaskKind::Browser(_) => Tab::Browser,
        TaskKind::Compliance(_) => Tab::Compliance,
        TaskKind::Storage(_) => Tab::Storage,
        TaskKind::Integrations(_) => Tab::Integrations,
        TaskKind::Workflow(_) => Tab::Workflow,
        TaskKind::Vuln(_) => Tab::Vuln,
        TaskKind::Wireless(_) => Tab::Wireless,
        TaskKind::WirelessActive(_) => Tab::Wireless,
        TaskKind::DbPentest(_) => Tab::DbPentest,
        TaskKind::Intercept(_) => Tab::Intercept,
        TaskKind::C2(_) => Tab::C2,
    }
}

pub fn run(config_path: Option<String>) -> Result<()> {
    run_with_mode(config_path, RuntimeMode::default())
}

/// Cleanup-safe owner for the rich-TUI terminal session (Phase B).
///
/// Acquisition uses `ratatui::try_init()`, which owns raw mode plus the
/// alternate screen and registers a panic hook restoring both. Mouse capture
/// is an Eggsec-owned extra: it is enabled only after init succeeds, and a
/// failed enable rolls back the already-acquired state before returning.
/// Cursor visibility is restored on the normal path.
///
/// Teardown rules:
/// - explicit [`TerminalSession::restore`] runs every step independently: one
///   failure never prevents later steps, and the first failure stays primary
///   with later failures attached as context;
/// - teardown is idempotent (`restored` flag), so panic-hook restoration plus
///   guard drop cannot make the terminal worse;
/// - `Drop` is a silent best-effort fallback for early return / unwind. It
///   never prints: the Ratatui hook runs at panic time (before unwinding),
///   destructors run during unwind, and double restoration is harmless.
pub(crate) struct TerminalSession {
    terminal: Option<ratatui::DefaultTerminal>,
    mouse_capture_active: bool,
    restored: bool,
}

impl TerminalSession {
    /// Acquire raw mode + alternate screen, then mouse capture.
    pub(crate) fn new() -> Result<Self> {
        let terminal = ratatui::try_init().map_err(|e| {
            // `try_init` enables raw mode before entering the alternate
            // screen; if the latter fails the former would linger, so make a
            // best-effort attempt to hand back a usable terminal.
            let _ = ratatui::try_restore();
            anyhow::anyhow!("failed to initialize TUI terminal: {e:#}")
        })?;
        let mut session = Self {
            terminal: Some(terminal),
            mouse_capture_active: false,
            restored: false,
        };
        if let Err(e) = execute!(io::stdout(), EnableMouseCapture) {
            // Mouse enable failed after raw/alternate-screen acquisition:
            // roll back before returning so no half-owned state escapes.
            let _ = ratatui::try_restore();
            session.restored = true;
            return Err(anyhow::anyhow!("failed to enable mouse capture: {e:#}"));
        }
        session.mouse_capture_active = true;
        Ok(session)
    }

    pub(crate) fn terminal(&mut self) -> &mut ratatui::DefaultTerminal {
        self.terminal
            .as_mut()
            .expect("TerminalSession terminal taken after successful acquisition")
    }

    /// Explicit fallible restore for the normal path. Independent steps,
    /// idempotent: a second call (including via `Drop`) is a no-op success.
    pub(crate) fn restore(&mut self) -> Result<()> {
        if self.restored {
            return Ok(());
        }
        self.restored = true;
        let mouse_active = &mut self.mouse_capture_active;
        let terminal = &mut self.terminal;
        restore_with_ops(
            mouse_active,
            || execute!(io::stdout(), DisableMouseCapture),
            ratatui::try_restore,
            || {
                terminal
                    .as_mut()
                    .expect("TerminalSession terminal taken after successful acquisition")
                    .show_cursor()
            },
        )
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;
        if self.mouse_capture_active {
            self.mouse_capture_active = false;
            let _ = execute!(io::stdout(), DisableMouseCapture);
        }
        let _ = ratatui::try_restore();
        if let Some(ref mut terminal) = self.terminal {
            let _ = terminal.show_cursor();
        }
    }
}

/// Run the three teardown steps independently with injectable operations.
///
/// Every step is attempted even if an earlier one fails; per-step failures
/// are collected and combined with the first failure primary. Production
/// passes the real Crossterm/Ratatui operations; tests inject recording
/// closures. `mouse_active` is cleared before the disable attempt so a retry
/// or guard drop cannot double-disable.
pub(crate) fn restore_with_ops(
    mouse_active: &mut bool,
    mut disable_mouse: impl FnMut() -> std::io::Result<()>,
    mut restore_terminal: impl FnMut() -> std::io::Result<()>,
    mut show_cursor: impl FnMut() -> std::io::Result<()>,
) -> Result<()> {
    let mut errors: Vec<anyhow::Error> = Vec::new();
    if *mouse_active {
        *mouse_active = false;
        if let Err(e) = disable_mouse() {
            errors.push(anyhow::anyhow!("failed to disable mouse capture: {e:#}"));
        }
    }
    if let Err(e) = restore_terminal() {
        errors.push(anyhow::anyhow!(
            "failed to restore terminal (raw mode / alternate screen): {e:#}"
        ));
    }
    if let Err(e) = show_cursor() {
        errors.push(anyhow::anyhow!(
            "failed to restore cursor visibility: {e:#}"
        ));
    }
    combine_cleanup_errors(errors)
}

/// Combine ordered cleanup failures: the first stays primary, later failures
/// attach as context so no diagnostic is lost.
pub(crate) fn combine_cleanup_errors(mut errors: Vec<anyhow::Error>) -> Result<()> {
    let mut iter = errors.drain(..);
    let Some(mut combined) = iter.next() else {
        return Ok(());
    };
    for rest in iter {
        combined = combined.context(format!("additional cleanup failure: {rest:#}"));
    }
    Err(combined)
}

/// Drive a daemon future to completion from the synchronous TUI body.
///
/// The CLI binary runs under `#[tokio::main]`, so constructing a nested
/// `tokio::runtime::Runtime` here panics ("Cannot start a runtime from
/// within a runtime"; caught by the Phase B PTY smoke on 2026-09-20).
/// Reuse the ambient multi-thread runtime via `block_in_place` when one
/// exists; only build a throwaway runtime for standalone hosts (unit tests,
/// non-Tokio embeddings) where no ambient runtime is present.
pub(crate) fn block_on_ambient<F>(future: F) -> Result<F::Output>
where
    F: std::future::Future,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => Ok(tokio::task::block_in_place(|| handle.block_on(future))),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| anyhow::anyhow!("failed to create tokio runtime: {e:#}"))?;
            Ok(rt.block_on(future))
        }
    }
}

/// Combine the guarded TUI body outcome with the restoration outcome.
///
/// Precedence: the body error is always primary (restoration failure attaches
/// as context); a lone restoration failure is returned; success requires
/// both. Fatal presentation happens after restoration by the caller, which
/// prints the returned error once the alternate screen is gone.
pub(crate) fn combine_body_restore(body: Result<()>, restore: Result<()>) -> Result<()> {
    match (body, restore) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(body_err), Ok(())) => Err(body_err),
        (Ok(()), Err(restore_err)) => Err(restore_err),
        (Err(body_err), Err(restore_err)) => {
            Err(body_err.context(format!("terminal restoration also failed: {restore_err:#}")))
        }
    }
}

pub fn run_with_mode(config_path: Option<String>, mode: RuntimeMode) -> Result<()> {
    // One cleanup-safe owner: acquisition happens here, and every fallible
    // step below runs either inside the guarded body or after restoration.
    // No `?` after this point can strand raw/alternate-screen state.
    let mut session = TerminalSession::new()?;
    let body_result = run_tui_body(session.terminal(), config_path, mode);
    let restore_result = session.restore();
    combine_body_restore(body_result, restore_result)
}

/// Guarded TUI body: app/runtime setup, event loop, and quick-save.
///
/// Runs while the `TerminalSession` guard is alive in the caller. Daemon
/// runtime construction and attach happen here (inside the guard), and the
/// returned error propagates to the caller for post-restoration reporting —
/// fatal loop failures are never converted to success here.
///
/// Quick-save precedence (explicit rule): quick-save failure is non-fatal. It
/// never overwrites a body failure and never changes the exit status; it is
/// reported through the established tracing diagnostic path (silent under
/// the TUI no-console policy). Returning the body error primary keeps the
/// more important runtime/render failure intact.
fn run_tui_body(
    terminal: &mut ratatui::DefaultTerminal,
    config_path: Option<String>,
    mode: RuntimeMode,
) -> Result<()> {
    // Single-writer rule (Phase A): while the alternate screen is owned,
    // Ratatui/Crossterm is the only writer to the controlling terminal. No
    // `eprintln!`/`println!`/`dbg!` or direct stdout/stderr use is allowed here;
    // recoverable conditions are surfaced through in-frame TUI state below.
    // The terminal size is captured pre-App so the warning can be routed
    // through the notification overlay instead of a direct terminal write.
    let small_terminal_warning: Option<String> = terminal
        .size()
        .ok()
        .and_then(|size| small_terminal_warning_message(size.width, size.height));

    let history = state::create_shared_history();
    let mut app = App::new(history);

    // In-frame small-terminal notice: the responsive layout plus the
    // `is_terminal_too_small` fallback already render correctly, so this
    // warning is advisory and must not duplicate into other surfaces.
    if let Some(warning) = small_terminal_warning {
        app.overlay.notification = Some(super::notifications::Notification::new(
            warning,
            super::notifications::NotificationSeverity::Warning,
        ));
    }

    // Apply runtime mode if non-default.
    if mode != RuntimeMode::default() {
        app.apply_runtime_mode(&mode);
    }

    let loaded_config = match eggsec::config::load_config(config_path.as_deref()) {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!("Failed to load TUI config: {e}");
            // User-actionable: config was requested but could not be read, so
            // surface it in-frame. Tracing remains the diagnostic facade; the
            // notification is the user-visible route (no duplicate surfaces).
            // Only overwrite the small-terminal notice if one is not already
            // present, to avoid duplicating messages.
            if app.overlay.notification.is_none() {
                app.overlay.notification = Some(super::notifications::Notification::new(
                    format!("TUI config could not be loaded; using defaults: {e}"),
                    super::notifications::NotificationSeverity::Warning,
                ));
            }
            None
        }
    };
    if let Some(ref config) = loaded_config {
        app.tabs.settings.load_config(config);
        app.session_manager.config = crate::session::SessionConfig::default()
            .with_auto_save_interval(config.auto_save_interval_secs);
    } else {
        tracing::debug!("No config file found for TUI settings; using defaults");
    }
    if let Some(path) = config_path.clone() {
        app.tabs.settings.set_config_path(path.clone());
    }
    // Initialize enforcement context + LoadedScope (exactly like CLI main.rs + CommandContext).
    // TUI always starts in ManualPermissive for interactive discretion (no --strict-scope flag in TUI).
    // Scope file path lives in the settings tab's scope_inputs (first field is typically the path/manifest).
    let scope_path_opt: Option<String> =
        app.tabs.settings.scope_inputs.fields.first().and_then(|f| {
            if f.value.trim().is_empty() {
                None
            } else {
                Some(f.value.clone())
            }
        });
    let loaded_scope = if let Some(ref sp) = scope_path_opt {
        eggsec::config::load_scope_with_source(Some(sp)).unwrap_or_else(|_| {
            eggsec::config::load_scope_with_source(None)
                .unwrap_or_else(|_| eggsec::config::LoadedScope::default_empty())
        })
    } else {
        eggsec::config::load_scope_with_source(None)
            .unwrap_or_else(|_| eggsec::config::LoadedScope::default_empty())
    };
    let policy = loaded_config
        .as_ref()
        .map(|c| c.execution_policy.clone())
        .unwrap_or_default();
    let surface = eggsec::config::ExecutionSurface::TuiManual;
    let enforcement =
        eggsec::config::EnforcementContext::for_surface(surface, policy, loaded_scope.clone());
    app.enforcement_state = super::enforcement_facade::EnforcementFacade::new(
        super::TuiEnforcementState::new(surface, loaded_scope, enforcement),
    );

    // For daemon mode, connect and attach to session before starting the event loop.
    // Evaluated for WS4: daemon attach failure is user-actionable, so it uses
    // the existing per-tab error surface (`stop_with_message`) in addition to
    // the tracing diagnostic. No notification overlay is added to avoid
    // duplicating the same message into multiple persistent surfaces.
    if mode != RuntimeMode::default() {
        let runtime_mode = app.runtime_mode.clone();
        if let Some(ref client) = app.runtime_client {
            let client_arc = client.clone();
            let rt_mode = runtime_mode.clone();
            // Inside the terminal-session guard: reuse the ambient runtime
            // (never a nested `Runtime::new`, which panics under
            // `#[tokio::main]`). Attach failure degrades to the per-tab
            // error surface, never to an early return or panic.
            block_on_ambient(async {
                if let Err(e) = attach_daemon_session(client_arc.as_ref(), &rt_mode, &mut app).await
                {
                    tracing::error!("Failed to attach to daemon session: {}", e);
                    app.stop_with_message(&format!("Daemon attach failed: {}", e));
                }
            })?;
        }
    }

    let res = run_app(terminal, &mut app);

    // Quick-save runs while the alternate screen is still owned, so no
    // in-frame surface remains to render into; per the precedence rule above
    // it stays a tracing diagnostic and never overwrites the loop result.
    if let Err(e) = app.session_manager.save_quick(&app) {
        tracing::warn!("Failed to save session on exit: {:?}", e);
    }

    res
}

/// Attach to a daemon session: create/list/subscribe based on mode flags.
async fn attach_daemon_session(
    client: &dyn crate::runtime_client::TuiRuntimeClient,
    mode: &RuntimeMode,
    app: &mut App,
) -> Result<(), String> {
    let RuntimeMode::Daemon {
        session_id,
        new_session,
        attach_latest,
        ..
    } = mode
    else {
        return Ok(());
    };

    let target_session = if *new_session {
        let scope: eggsec_runtime::session::SessionScope =
            eggsec::config::session_scope_from_loaded(app.enforcement_state.loaded_scope());
        let sid = client
            .create_session(
                eggsec_runtime::RuntimeSurface::TuiManual,
                Some(scope),
                vec![],
            )
            .await?;
        tracing::info!("Created new daemon session: {}", sid);
        Some(sid)
    } else if let Some(ref explicit_id) = session_id {
        // Strip "session:" prefix if present (SessionId Display format).
        let uuid_str = explicit_id.strip_prefix("session:").unwrap_or(explicit_id);
        let uuid: uuid::Uuid = uuid_str
            .parse()
            .map_err(|e| format!("invalid session ID '{}': {}", explicit_id, e))?;
        let parsed = eggsec_runtime::SessionId::from_uuid(uuid);
        Some(parsed)
    } else if *attach_latest {
        let sessions = client.list_sessions().await?;
        sessions
            .into_iter()
            .max_by_key(|s| s.created_at_epoch_secs)
            .map(|s| s.session_id)
    } else {
        // No explicit session flag: list and pick latest, or create new.
        let sessions = client.list_sessions().await?;
        sessions
            .into_iter()
            .max_by_key(|s| s.created_at_epoch_secs)
            .map(|s| s.session_id)
    };

    match target_session {
        Some(sid) => {
            // Hydrate from snapshot.
            let snapshot = client.snapshot(sid).await?;
            tracing::info!(
                session = %sid,
                active = snapshot.active_tasks.len(),
                completed = snapshot.completed_tasks.len(),
                "Attached to daemon session"
            );

            // Store session ID and subscribe to events.
            app.runtime_binding.session_id = Some(sid);
            let event_handle = client.subscribe(sid).await?;
            // Store the handle for the adapter to drain.
            app.runtime_binding.daemon_event_handle = Some(event_handle);

            // Hydrate adapter with pre-existing completed tasks.
            for task in &snapshot.completed_tasks {
                let tab = tab_for_task_kind(&task.task_kind);
                app.runtime_adapter.register_task(task.task_id, tab);
            }
        }
        None => {
            // No sessions exist; create a new one.
            let scope: eggsec_runtime::session::SessionScope =
                eggsec::config::session_scope_from_loaded(app.enforcement_state.loaded_scope());
            let sid = client
                .create_session(
                    eggsec_runtime::RuntimeSurface::TuiManual,
                    Some(scope),
                    vec![],
                )
                .await?;
            tracing::info!("No sessions found; created new daemon session: {}", sid);
            app.runtime_binding.session_id = Some(sid);
            let event_handle = client.subscribe(sid).await?;
            app.runtime_binding.daemon_event_handle = Some(event_handle);
        }
    }

    Ok(())
}

/// Pure advisory message for terminals below the recommended 80x24.
///
/// Returns `None` when the size is sufficient. The caller routes `Some` into
/// the in-frame notification overlay; this helper never writes to the
/// terminal itself. The responsive layout plus the `is_terminal_too_small`
/// fallback already render correctly, so the message is advisory only.
pub(crate) fn small_terminal_warning_message(width: u16, height: u16) -> Option<String> {
    if width < 80 || height < 24 {
        Some(format!(
            "Terminal size ({width}x{height}) is smaller than recommended (80x24). \
             Resize your window or scroll horizontally for full UI."
        ))
    } else {
        None
    }
}

fn handle_mouse_event(mouse_event: MouseEvent, app: &mut App) {
    let MouseEventKind::Down(button) = mouse_event.kind else {
        if let MouseEventKind::ScrollUp = mouse_event.kind {
            if app
                .command_palette
                .as_ref()
                .map(|p| p.visible)
                .unwrap_or(false)
            {
                if let Some(ref mut palette) = app.command_palette {
                    if palette.selected_index > 0 {
                        palette.selected_index -= 1;
                    }
                    if palette.selected_index < palette.scroll_offset {
                        palette.scroll_offset = palette.selected_index;
                    }
                    app.needs_redraw = true;
                }
                return;
            }
            if !app.is_any_overlay_active() {
                app.page_up();
                app.needs_redraw = true;
            }
            return;
        }
        if let MouseEventKind::ScrollDown = mouse_event.kind {
            if app
                .command_palette
                .as_ref()
                .map(|p| p.visible)
                .unwrap_or(false)
            {
                if let Some(ref mut palette) = app.command_palette {
                    let max_idx = palette.results.len().saturating_sub(1);
                    if palette.selected_index < max_idx {
                        palette.selected_index += 1;
                    }
                    palette.adjust_scroll_for_selection();
                    app.needs_redraw = true;
                }
                return;
            }
            if !app.is_any_overlay_active() {
                app.page_down();
                app.needs_redraw = true;
            }
            return;
        }
        return;
    };

    if button == crossterm::event::MouseButton::Left {
        let (term_width, _term_height) = crossterm::terminal::size().unwrap_or((80, 24));
        let tab_area = compute_tab_area(term_width);

        if app.is_any_overlay_active() {
            return;
        }

        if let Some(ref palette) = app.command_palette {
            if palette.visible {
                return;
            }
        }

        if tab_area.contains((mouse_event.column, mouse_event.row).into()) {
            let window =
                TabWindow::for_width(tab_area.width, app.current_tab, app.tab_scroll_offset);
            let spans = window.visible_tab_spans(tab_area.width);
            let click_x = mouse_event.column.saturating_sub(tab_area.x);

            for span in spans {
                if click_x >= span.x_start && click_x < span.x_end {
                    if app.set_current_tab_if_available(span.tab) {
                        app.adjust_tab_scroll();
                        app.needs_redraw = true;
                    }
                    break;
                }
            }
        }
    }
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    use crossterm::event::EventStream;
    use futures::{FutureExt, StreamExt};

    let mut key_handler = KeyHandler::new();
    let mut event_stream = EventStream::new();
    let mut pending_redraw = false;

    loop {
        app.update();

        app.auto_save_if_due();

        if app.should_quit {
            return Ok(());
        }

        if app.needs_redraw || pending_redraw {
            terminal.draw(|f| ui::draw(f, app))?;
            app.needs_redraw = false;
            pending_redraw = false;
        }

        let mut event_count = 0;
        loop {
            match event_stream.next().now_or_never() {
                Some(Some(Ok(event))) => {
                    event_count += 1;
                    match event {
                        Event::Key(key) => key_handler.handle_key_event(app, &key),
                        Event::Mouse(mouse_event) => handle_mouse_event(mouse_event, app),
                        Event::Paste(text) => {
                            if app.mode == InputMode::Insert {
                                app.dispatcher_mut().handle_paste(&text);
                            } else {
                                tracing::trace!("Paste event dropped: not in Insert mode");
                            }
                        }
                        Event::FocusGained | Event::FocusLost | Event::Resize(_, _) => {
                            pending_redraw = true;
                        }
                    }
                }
                Some(Some(Err(e))) => {
                    // Evaluated for WS4: transient event errors stay as tracing
                    // diagnostics (silent under the TUI no-console policy).
                    // They are not user-actionable per-event, so no
                    // notification surface is added.
                    tracing::warn!("Terminal event error: {:?}", e);
                }
                Some(None) => {
                    // Terminal event stream ended (e.g. terminal detached).
                    // Quit gracefully instead of spinning in a busy-loop.
                    // Evaluated for WS4: the session ends here, so no
                    // in-frame notification would be visible; keep the tracing
                    // diagnostic only.
                    tracing::warn!("Terminal event stream ended; quitting");
                    app.should_quit = true;
                    break;
                }
                None => break,
            }
        }
        if event_count == 0 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_runtime::request::{FingerprintParams, LoadTestParams, PortScanParams, ReconParams};

    #[test]
    fn tab_for_task_kind_port_scan() {
        let kind = TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ports: None,
            scan_type: None,
            timeout_ms: None,
            concurrency: None,
        });
        assert_eq!(tab_for_task_kind(&kind), Tab::ScanPorts);
    }

    #[test]
    fn tab_for_task_kind_recon() {
        let kind = TaskKind::Recon(ReconParams {
            target: "example.com".into(),
            modules: None,
        });
        assert_eq!(tab_for_task_kind(&kind), Tab::Recon);
    }

    #[test]
    fn tab_for_task_kind_load_test() {
        let kind = TaskKind::LoadTest(LoadTestParams {
            target: "http://example.com".into(),
            method: "GET".into(),
            requests: None,
            connections: None,
            duration_secs: None,
            rate_limit: None,
        });
        assert_eq!(tab_for_task_kind(&kind), Tab::Load);
    }

    #[test]
    fn tab_for_task_kind_fingerprint() {
        let kind = TaskKind::Fingerprint(FingerprintParams {
            target: "10.0.0.1".into(),
            ports: None,
            timeout_secs: None,
            concurrency: None,
        });
        assert_eq!(tab_for_task_kind(&kind), Tab::Fingerprint);
    }

    #[test]
    fn small_terminal_warning_fires_below_80x24() {
        let warning = small_terminal_warning_message(79, 24).expect("width below 80 must warn");
        assert!(warning.contains("79x24"));
        assert!(warning.contains("80x24"));

        let warning = small_terminal_warning_message(80, 23).expect("height below 24 must warn");
        assert!(warning.contains("80x23"));

        assert!(small_terminal_warning_message(80, 24).is_none());
        assert!(small_terminal_warning_message(120, 40).is_none());
    }

    fn io_err(msg: &str) -> std::io::Error {
        std::io::Error::other(msg)
    }

    #[test]
    fn restore_with_ops_all_steps_attempted_despite_failures() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let calls: Rc<RefCell<Vec<&str>>> = Rc::new(RefCell::new(Vec::new()));
        let record = |tag: &'static str| {
            let calls = Rc::clone(&calls);
            move || {
                calls.borrow_mut().push(tag);
                Err(io_err("injected failure"))
            }
        };
        let mut mouse_active = true;
        let err = restore_with_ops(
            &mut mouse_active,
            record("mouse"),
            record("terminal"),
            record("cursor"),
        )
        .expect_err("all-failing restore must fail");
        // Every step ran despite earlier failures.
        assert_eq!(*calls.borrow(), vec!["mouse", "terminal", "cursor"]);
        assert!(!mouse_active, "mouse flag clears even when disable fails");
        // First failure stays primary; later failures attach as context.
        let rendered = format!("{err:#}");
        assert!(rendered.contains("mouse"), "primary error: {rendered}");
        assert!(rendered.contains("terminal"), "context kept: {rendered}");
        assert!(rendered.contains("cursor"), "context kept: {rendered}");
    }

    #[test]
    fn restore_with_ops_skips_mouse_when_inactive() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let calls: Rc<RefCell<Vec<&str>>> = Rc::new(RefCell::new(Vec::new()));
        let record_ok = |tag: &'static str| {
            let calls = Rc::clone(&calls);
            move || {
                calls.borrow_mut().push(tag);
                Ok(())
            }
        };
        let mut mouse_active = false;
        restore_with_ops(
            &mut mouse_active,
            record_ok("mouse"),
            record_ok("terminal"),
            record_ok("cursor"),
        )
        .expect("all-ok restore must succeed");
        assert_eq!(*calls.borrow(), vec!["terminal", "cursor"]);
    }

    #[test]
    fn restore_with_ops_partial_failure_returns_first() {
        let mut mouse_active = true;
        let err = restore_with_ops(
            &mut mouse_active,
            || Ok(()),
            || Err(io_err("terminal boom")),
            || Ok(()),
        )
        .expect_err("terminal failure must surface");
        let rendered = format!("{err:#}");
        assert!(rendered.contains("terminal"), "{rendered}");
    }

    #[test]
    fn combine_cleanup_errors_empty_is_ok() {
        assert!(combine_cleanup_errors(Vec::new()).is_ok());
    }

    #[test]
    fn block_on_ambient_drives_future_without_ambient_runtime() {
        // Plain #[test]: no ambient runtime, so the standalone fallback
        // runtime must drive the future.
        let out = block_on_ambient(async { 42u32 }).expect("standalone fallback must drive future");
        assert_eq!(out, 42);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn block_on_ambient_reuses_ambient_runtime() {
        // Inside a multi-thread runtime (like the CLI `#[tokio::main]`),
        // the future must complete without a nested `Runtime::new`
        // (which would panic). A nested spawn proves other workers
        // progress while blocked.
        let out = block_on_ambient(async {
            tokio::task::spawn(async { 7u32 })
                .await
                .expect("spawn works")
        })
        .expect("ambient branch must drive future");
        assert_eq!(out, 7);
    }

    #[test]
    fn combine_body_restore_keeps_body_primary() {
        // Both succeed.
        assert!(combine_body_restore(Ok(()), Ok(())).is_ok());
        // Lone body failure propagates unchanged.
        let err = combine_body_restore(Err(anyhow::anyhow!("body boom")), Ok(()))
            .expect_err("body failure must propagate");
        assert!(format!("{err:#}").contains("body boom"));
        // Lone restore failure propagates.
        let err = combine_body_restore(Ok(()), Err(anyhow::anyhow!("restore boom")))
            .expect_err("restore failure must propagate");
        assert!(format!("{err:#}").contains("restore boom"));
        // Both fail: body stays primary, restore attaches as context.
        let err = combine_body_restore(
            Err(anyhow::anyhow!("body boom")),
            Err(anyhow::anyhow!("restore boom")),
        )
        .expect_err("combined failure must propagate");
        let rendered = format!("{err:#}");
        assert!(rendered.contains("body boom"), "primary kept: {rendered}");
        assert!(
            rendered.contains("restore boom"),
            "context kept: {rendered}"
        );
    }
}
