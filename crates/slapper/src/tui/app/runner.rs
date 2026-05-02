use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use super::input::InputMode;
use super::App;
use super::PendingAction;
use crate::tui::help::CommandPalette;
use crate::tui::state;
use crate::tui::tabs::{Tab, TabWindow};
use crate::tui::ui;
use crate::tui::utils::Clipboard;
use crate::tui::OverlayType;

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
            let window = TabWindow::for_width(tab_area.width, app.current_tab, app.tab_scroll_offset);
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
                if let Some(pending) = app.pending_key.take() {
                    match (key.modifiers, key.code, pending) {
                        (_, KeyCode::Char('g'), KeyCode::Char('g'))
                            if app.mode == InputMode::Normal =>
                        {
                            app.handle_top();
                            continue;
                        }
                        _ => {}
                    }
                }

                app.needs_redraw = true;

                match (key.modifiers, key.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        if app.is_running() {
                            app.stop();
                        } else {
                            return Ok(());
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                        app.page_up();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                        app.page_down();
                    }
                    (KeyModifiers::NONE, KeyCode::PageUp) => {
                        app.page_up();
                    }
                    (KeyModifiers::NONE, KeyCode::PageDown) => {
                        app.page_down();
                    }
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        app.pending_key = None;
                        match app.topmost_overlay() {
                            Some(OverlayType::ConfirmPopup) => {
                                app.cancel_action();
                            }
                            Some(OverlayType::CommandPalette) => {
                                app.toggle_command_palette();
                            }
                             Some(OverlayType::Search) => {
                                 app.toggle_search(app.search_is_global);
                             }
                            Some(OverlayType::HttpOptions) => {
                                app.show_http_options = false;
                                app.needs_redraw = true;
                            }
                            Some(OverlayType::Help) => {
                                app.toggle_help();
                            }
                            None => {
                                app.mode = InputMode::Normal;
                                app.handle_escape();
                            }
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('i')) if app.mode == InputMode::Normal => {
                        app.pending_key = None;
                        app.mode = InputMode::Insert;
                    }
                    (KeyModifiers::NONE, KeyCode::Char('q')) if app.mode == InputMode::Normal => {
                        if !app.is_running() {
                            return Ok(());
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char(' ')) if app.mode == InputMode::Normal => {
                        app.toggle_help();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('/')) => {
                        app.toggle_help();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                        app.toggle_command_palette();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                        // Toggle global search
                        if app.show_search {
                            // If already showing search, perform search with current query
                            app.perform_search();
                        } else {
                            // Open global search prompt using toggle_search to set search_is_global
                            app.toggle_search(true);
                            app.needs_redraw = true;
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('z')) => {
                        app.toggle_pause();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                        app.toggle_theme();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('v')) => {
                        if let Some(text) = Clipboard::get() {
                            app.dispatcher_mut().handle_paste(&text);
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('y')) => {
                        if app.is_paused() {
                            app.resume();
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('b')) if app.mode == InputMode::Normal => {
                        app.toggle_bookmark(app.current_tab);
                    }
                    _ if app
                        .get_command_palette()
                        .map(&|cp: &CommandPalette| cp.visible)
                        .unwrap_or(false) =>
                    {
                        match (key.modifiers, key.code) {
                            (KeyModifiers::NONE, KeyCode::Esc) => {
                                app.toggle_command_palette();
                            }
                            (KeyModifiers::NONE, KeyCode::Enter) => {
                                let index = app
                                    .command_palette
                                    .as_ref()
                                    .map(|p| p.selected_index)
                                    .unwrap_or(0);
                                app.select_command_palette_item(index);
                            }
                            (KeyModifiers::NONE, KeyCode::Up) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    if palette.selected_index > 0 {
                                        palette.selected_index -= 1;
                                    }
                                    if palette.selected_index < palette.scroll_offset {
                                        palette.scroll_offset = palette.selected_index;
                                    }
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Down) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    let max_idx = palette.results.len().saturating_sub(1);
                                    if palette.selected_index < max_idx {
                                        palette.selected_index += 1;
                                    }
                                    palette.adjust_scroll_for_selection();
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Backspace) => {
                                let query = app
                                    .command_palette
                                    .as_ref()
                                    .map(|p| p.query.clone())
                                    .unwrap_or_default();
                                if !query.is_empty() {
                                    if let Some(ref mut palette) = app.command_palette {
                                        palette.query.pop();
                                        let new_query = palette.query.clone();
                                        app.update_command_palette_query(&new_query);
                                    }
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    palette.query.push(c);
                                    let new_query = palette.query.clone();
                                    app.update_command_palette_query(&new_query);
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Tab) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    let max_idx = palette.results.len().saturating_sub(1);
                                    if palette.selected_index < max_idx {
                                        palette.selected_index += 1;
                                    }
                                    palette.adjust_scroll_for_selection();
                                }
                            }
                            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    if palette.selected_index > 0 {
                                        palette.selected_index -= 1;
                                    }
                                    if palette.selected_index < palette.scroll_offset {
                                        palette.scroll_offset = palette.selected_index;
                                    }
                                }
                            }
                             _ => {}
                         }
                     }
                    // Prevent tab content keys when other overlays are active
                    _ if app.is_search_visible() || app.is_http_options_visible() || app.is_help_visible() => {
                        // Handle keys for active overlays; Esc is handled above
                        match (key.modifiers, key.code) {
                            (KeyModifiers::NONE, KeyCode::Enter) if app.is_search_visible() => {
                                app.perform_search();
                            }
                            (KeyModifiers::NONE, KeyCode::Backspace) if app.is_search_visible() => {
                                app.search_query.pop();
                            }
                            (KeyModifiers::CONTROL, KeyCode::Char('u')) if app.is_search_visible() => {
                                app.search_query.clear();
                            }
                            (KeyModifiers::NONE, KeyCode::Char(c)) if app.is_search_visible() => {
                                app.search_query.push(c);
                            }
                            _ => {}
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Tab) => {
                        app.handle_focus_next();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char(' ')) => {
                        if app.mode == InputMode::Insert {
                            app.handle_autocomplete();
                        }
                    }
                    (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                        app.handle_focus_prev();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('h')) if app.is_http_options_visible() => {
                        app.show_http_options = false;
                        app.needs_redraw = true;
                    }
                    (KeyModifiers::NONE, KeyCode::Char('h'))
                    | (KeyModifiers::NONE, KeyCode::Left)
                        if app.mode == InputMode::Normal =>
                    {
                        app.handle_left();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('j'))
                    | (KeyModifiers::NONE, KeyCode::Down)
                        if app.mode == InputMode::Normal =>
                    {
                        app.handle_down();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('k'))
                    | (KeyModifiers::NONE, KeyCode::Up)
                        if app.mode == InputMode::Normal =>
                    {
                        app.handle_up();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('l'))
                    | (KeyModifiers::NONE, KeyCode::Right)
                        if app.mode == InputMode::Normal =>
                    {
                        app.handle_right();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('H')) if app.mode == InputMode::Normal => {
                        app.handle_home();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('L')) if app.mode == InputMode::Normal => {
                        app.handle_end();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('G')) if app.mode == InputMode::Normal => {
                        app.handle_bottom();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('g')) if app.mode == InputMode::Normal => {
                        app.pending_key = Some(KeyCode::Char('g'));
                    }
                    (KeyModifiers::NONE, KeyCode::Char('w')) if app.mode == InputMode::Normal => {
                        app.handle_word_forward();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('b')) if app.mode == InputMode::Normal => {
                        app.handle_word_backward();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('n'))
                    | (KeyModifiers::NONE, KeyCode::Char('N'))
                        if app.mode == InputMode::Normal =>
                    {
                        if key.code == KeyCode::Char('n') {
                            app.next_tab();
                        } else {
                            app.prev_tab();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Backspace) => {
                        app.handle_backspace();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('p')) if app.mode == InputMode::Normal => {
                        app.prev_tab();
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char('H')) if app.mode == InputMode::Normal => {
                        app.prev_tab();
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char('L')) if app.mode == InputMode::Normal => {
                        app.next_tab();
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char('E')) if app.mode == InputMode::Normal => {
                        app.cycle_export_format();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('/')) if app.mode == InputMode::Normal => {
                        app.toggle_search(false);
                    }
                    (KeyModifiers::NONE, KeyCode::Char('r')) if app.mode == InputMode::Normal => {
                        if !app.is_running() {
                            if app.current_tab == Tab::History {
                                app.request_confirmation(PendingAction::ClearHistory);
                            } else {
                                app.request_confirmation(PendingAction::ResetTab);
                            }
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('0')) if app.mode == InputMode::Normal => {
                        app.select_tab(9);
                    }
                    (KeyModifiers::NONE, KeyCode::Char(c))
                        if app.mode == InputMode::Normal && ('1'..='9').contains(&c) =>
                    {
                        let idx = c.to_digit(10).unwrap() as usize - 1;
                        if idx < Tab::all().len() {
                            app.select_tab(idx);
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('s')) if app.mode == InputMode::Normal => {
                        if !app.is_running() && app.current_tab == Tab::Settings {
                            app.request_confirmation(PendingAction::SaveSettings);
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('d')) if app.mode == InputMode::Normal => {
                        if !app.is_running() && app.current_tab == Tab::History {
                            app.request_confirmation(PendingAction::DeleteHistoryEntry);
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('e')) if app.mode == InputMode::Normal => {
                        // Export results for current tab
                        app.export_results();
                    }
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        if app.is_confirm_popup_visible() {
                            app.confirm_action();
                        } else {
                            app.handle_enter();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char(c)) if app.mode == InputMode::Insert => {
                        app.handle_char(c);
                    }
                    _ => {
                        app.needs_redraw = false;
                    }
                }
            }

            if let Event::Mouse(mouse_event) = event {
                handle_mouse_event(mouse_event, app);
            }
        }
    }
}
