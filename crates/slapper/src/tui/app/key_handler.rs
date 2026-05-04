use crossterm::event::{KeyCode, KeyModifiers};

use super::App;
use super::InputMode;
use super::PendingAction;
use crate::tui::tabs::Tab;
use crate::tui::OverlayType;
use crate::tui::utils::Clipboard;

pub struct KeyHandler;

impl KeyHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_key_event(&mut self, app: &mut App, key: &crossterm::event::KeyEvent) {
        if let Some(pending) = app.pending_key.take() {
            match (key.modifiers, key.code, pending) {
                (_, KeyCode::Char('g'), KeyCode::Char('g')) if app.mode == InputMode::Normal => {
                    app.handle_top();
                    return;
                }
                _ => {}
            }
        }

        app.needs_redraw = true;

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.handle_ctrl_c(app);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                app.toggle_quick_switch();
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
            (KeyModifiers::NONE, KeyCode::Home) => {
                app.handle_home();
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                app.handle_end();
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                app.handle_up();
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                app.handle_down();
            }
            (KeyModifiers::NONE, KeyCode::Left) => {
                app.handle_left();
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                app.handle_right();
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.handle_escape(app);
            }
            (KeyModifiers::NONE, KeyCode::Char('i')) if app.mode == InputMode::Normal => {
                self.handle_enter_insert_mode(app);
            }
            (KeyModifiers::NONE, KeyCode::Char('q')) if app.mode == InputMode::Normal => {
                self.handle_quit(app);
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
                self.handle_ctrl_f(app);
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
            _ if app.is_quick_switch_visible() => {
                self.handle_quick_switch(app, key);
            }
            _ if app
                .get_command_palette()
                .map(&|cp: &crate::tui::help::CommandPalette| cp.visible)
                .unwrap_or(false) =>
            {
                self.handle_command_palette(app, key);
            }
            _ if app.is_search_visible() || app.is_http_options_visible() || app.is_help_visible() => {
                self.handle_overlay_input(app, key);
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
            (KeyModifiers::NONE, KeyCode::Char('h')) if app.mode == InputMode::Normal => {
                app.handle_left();
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) if app.mode == InputMode::Normal => {
                app.handle_down();
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) if app.mode == InputMode::Normal => {
                app.handle_up();
            }
            (KeyModifiers::NONE, KeyCode::Char('l')) if app.mode == InputMode::Normal => {
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
                self.handle_reset(app);
            }
            (KeyModifiers::NONE, KeyCode::Char('s')) if app.mode == InputMode::Normal => {
                self.handle_save_settings(app);
            }
            (KeyModifiers::NONE, KeyCode::Char('d')) if app.mode == InputMode::Normal => {
                self.handle_delete_entry(app);
            }
            (KeyModifiers::NONE, KeyCode::Char('e')) if app.mode == InputMode::Normal => {
                app.export_results();
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                self.handle_enter(app);
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) if app.mode == InputMode::Insert => {
                app.handle_char(c);
            }
            _ => {
                app.needs_redraw = false;
            }
        }
    }

    fn handle_ctrl_c(&self, app: &mut App) {
        if app.is_running() {
            app.stop();
        } else {
            app.should_quit = true;
        }
    }

    fn handle_ctrl_f(&self, app: &mut App) {
        if app.show_search {
            app.perform_search();
        } else {
            app.toggle_search(true);
            app.needs_redraw = true;
        }
    }

    fn handle_escape(&self, app: &mut App) {
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
            Some(OverlayType::QuickSwitch) => {
                app.close_quick_switch();
            }
            None => {
                if app.mode == InputMode::Insert {
                    app.mode = InputMode::Normal;
                    app.dispatcher_mut().handle_escape();
                }
            }
        }
    }

    fn handle_enter_insert_mode(&self, app: &mut App) {
        app.pending_key = None;
        app.mode = InputMode::Insert;
    }

    fn handle_quit(&self, app: &mut App) {
        if !app.is_running() {
            app.should_quit = true;
        }
    }

    fn handle_reset(&self, app: &mut App) {
        if !app.is_running() {
            if app.current_tab == Tab::History {
                app.request_confirmation(PendingAction::ClearHistory);
            } else {
                app.request_confirmation(PendingAction::ResetTab);
            }
        }
    }

    fn handle_save_settings(&self, app: &mut App) {
        if !app.is_running() && app.current_tab == Tab::Settings {
            app.request_confirmation(PendingAction::SaveSettings);
        }
    }

    fn handle_delete_entry(&self, app: &mut App) {
        if !app.is_running() && app.current_tab == Tab::History {
            app.request_confirmation(PendingAction::DeleteHistoryEntry);
        }
    }

    fn handle_enter(&self, app: &mut App) {
        if app.is_confirm_popup_visible() {
            app.confirm_action();
        } else {
            app.handle_enter();
        }
    }

    fn handle_command_palette(&self, app: &mut App, key: &crossterm::event::KeyEvent) {
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

    fn handle_overlay_input(&self, app: &mut App, key: &crossterm::event::KeyEvent) {
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

    fn handle_quick_switch(&self, app: &mut App, key: &crossterm::event::KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                app.close_quick_switch();
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                let results = app.get_quick_switch_results();
                if !results.is_empty() && app.quick_switch_selected < results.len() {
                    if let Some(tab) = results.get(app.quick_switch_selected) {
                        app.current_tab = **tab;
                        app.adjust_tab_scroll();
                    }
                }
                app.close_quick_switch();
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                if app.quick_switch_selected > 0 {
                    app.quick_switch_selected -= 1;
                }
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                let results = app.get_quick_switch_results();
                if app.quick_switch_selected < results.len().saturating_sub(1) {
                    app.quick_switch_selected += 1;
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) | (KeyModifiers::NONE, KeyCode::PageUp) => {
                if app.quick_switch_selected >= 10 {
                    app.quick_switch_selected -= 10;
                } else {
                    app.quick_switch_selected = 0;
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('d')) | (KeyModifiers::NONE, KeyCode::PageDown) => {
                let results = app.get_quick_switch_results();
                app.quick_switch_selected = (app.quick_switch_selected + 10).min(results.len().saturating_sub(1));
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                app.quick_switch_selected = 0;
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                let results = app.get_quick_switch_results();
                app.quick_switch_selected = results.len().saturating_sub(1);
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                app.quick_switch_query.pop();
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                app.quick_switch_query.push(c);
            }
            _ => {}
        }
    }
}

impl Default for KeyHandler {
    fn default() -> Self {
        Self::new()
    }
}