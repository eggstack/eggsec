use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use super::App;
use super::KeyHandler;
use crate::tui::state;
use crate::tui::tabs::TabWindow;
use crate::tui::ui;

/// Layout constants matching `ui::draw()` — change these if the layout changes.
const LAYOUT_MARGIN: u16 = 1;
const TAB_BAR_HEIGHT: u16 = 3;

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
    if let Some(path) = config_path {
        app.settings.set_config_path(path);
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
            if !app.is_any_overlay_active() || app.is_command_palette_visible() {
                if app
                    .command_palette
                    .as_ref()
                    .map(|p| p.visible)
                    .unwrap_or(false)
                {
                    return;
                }
                app.page_up();
                app.needs_redraw = true;
            }
            return;
        }
        if let MouseEventKind::ScrollDown = mouse_event.kind {
            if !app.is_any_overlay_active() || app.is_command_palette_visible() {
                if app
                    .command_palette
                    .as_ref()
                    .map(|p| p.visible)
                    .unwrap_or(false)
                {
                    return;
                }
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
    let mut key_handler = KeyHandler::new();

    loop {
        app.spinner_tick = app.spinner_tick.wrapping_add(1);

        app.update();
        app.auto_save_if_due();

        if app.should_quit {
            return Ok(());
        }

        if app.needs_redraw {
            terminal.draw(|f| ui::draw(f, app))?;
            app.needs_redraw = false;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;

            if let Event::Key(key) = &event {
                key_handler.handle_key_event(app, key);
            }

            if let Event::Mouse(mouse_event) = event {
                handle_mouse_event(mouse_event, app);
            }
        }
    }
}
