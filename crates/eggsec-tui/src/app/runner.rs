use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use super::App;
use super::InputMode;
use super::KeyHandler;
use crate::state;
use crate::tabs::TabWindow;
use crate::ui;
use crate::ui::{LAYOUT_MARGIN, TAB_BAR_HEIGHT};
use crate::RuntimeMode;

fn compute_tab_area(term_width: u16) -> ratatui::layout::Rect {
    ratatui::layout::Rect {
        x: LAYOUT_MARGIN,
        y: LAYOUT_MARGIN,
        width: term_width.saturating_sub(LAYOUT_MARGIN * 2),
        height: TAB_BAR_HEIGHT,
    }
}

pub fn run(config_path: Option<String>) -> Result<()> {
    run_with_mode(config_path, RuntimeMode::default())
}

pub fn run_with_mode(config_path: Option<String>, mode: RuntimeMode) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    if let Ok(size) = terminal.size() {
        if size.width < 80 || size.height < 24 {
            eprintln!(
                "Warning: Terminal size ({}x{}) is smaller than recommended (80x24). \
                 Resize your window or scroll horizontally for full UI.",
                size.width, size.height
            );
        }
    }

    let history = state::create_shared_history();
    let mut app = App::new(history);

    // Apply runtime mode if non-default.
    if mode != RuntimeMode::default() {
        app.apply_runtime_mode(&mode);
    }

    let loaded_config = match eggsec::config::load_config(config_path.as_deref()) {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!("Failed to load TUI config: {e}");
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
    if mode != RuntimeMode::default() {
        let runtime_mode = app.runtime_mode.clone();
        if let Some(ref client) = app.runtime_client {
            let client_arc = client.clone();
            let rt_mode = runtime_mode.clone();
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                if let Err(e) = attach_daemon_session(client_arc.as_ref(), &rt_mode, &mut app).await
                {
                    tracing::error!("Failed to attach to daemon session: {}", e);
                    app.stop_with_message(&format!("Daemon attach failed: {}", e));
                }
            });
        }
    }

    let res = run_app(&mut terminal, &mut app);

    if let Err(e) = app.session_manager.save_quick(&app) {
        tracing::warn!("Failed to save session on exit: {:?}", e);
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        tracing::error!("TUI exited with error: {:?}", err);
    }

    Ok(())
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
            app.enforcement_state.loaded_scope().into();
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
            .max_by_key(|s| s.created_at_secs)
            .map(|s| s.session_id)
    } else {
        // No explicit session flag: list and pick latest, or create new.
        let sessions = client.list_sessions().await?;
        sessions
            .into_iter()
            .max_by_key(|s| s.created_at_secs)
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
                let tab = crate::tabs::Tab::Recon; // Default mapping for hydrated tasks.
                app.runtime_adapter.register_task(task.task_id, tab);
            }
        }
        None => {
            // No sessions exist; create a new one.
            let scope: eggsec_runtime::session::SessionScope =
                app.enforcement_state.loaded_scope().into();
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
                    tracing::warn!("Terminal event error: {:?}", e);
                }
                Some(None) => {
                    // Terminal event stream ended (e.g. terminal detached).
                    // Quit gracefully instead of spinning in a busy-loop.
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
