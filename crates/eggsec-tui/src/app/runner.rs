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

fn compute_tab_area(term_width: u16) -> ratatui::layout::Rect {
    ratatui::layout::Rect {
        x: LAYOUT_MARGIN,
        y: LAYOUT_MARGIN,
        width: term_width.saturating_sub(LAYOUT_MARGIN * 2),
        height: TAB_BAR_HEIGHT,
    }
}

pub fn run(config_path: Option<String>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    if let Ok(size) = terminal.size() {
        if size.width < 80 || size.height < 24 {
            eprintln!(
                "Warning: Terminal size ({}x{}) is smaller than recommended (80x24). \
                 Some UI elements may not display correctly.",
                size.width, size.height
            );
        }
    }

    let history = state::create_shared_history();
    let mut app = App::new(history);
    if let Ok(config) = eggsec::config::load_config(config_path.as_deref()) {
        app.tabs.settings.load_config(&config);
        app.session_manager.config = crate::session::SessionConfig::default()
            .with_auto_save_interval(config.auto_save_interval_secs);
    } else {
        tracing::debug!("No config file found for TUI settings; using defaults");
    }
    if let Some(path) = config_path {
        app.tabs.settings.set_config_path(path);
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
