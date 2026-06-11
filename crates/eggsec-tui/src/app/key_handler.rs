use crossterm::event::{KeyCode, KeyModifiers};

use super::App;
use super::InputMode;
use super::PendingAction;
use crate::tabs::Tab;
use crate::utils::Clipboard;
use crate::OverlayType;

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

        if self.handle_topmost_overlay(app, key) {
            return;
        }

        if self.handle_global_shortcuts(app, key) {
            return;
        }

        if self.handle_mode_specific_input(app, key) {
            return;
        }

        app.needs_redraw = false;
    }

    fn handle_global_shortcuts(&self, app: &mut App, key: &crossterm::event::KeyEvent) -> bool {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => self.handle_ctrl_c(app),
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                if !app.has_active_task() {
                    app.pending_key = None;
                    app.toggle_quick_switch();
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => app.page_up(),
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => app.page_down(),
            (KeyModifiers::NONE, KeyCode::PageUp) => app.page_up(),
            (KeyModifiers::NONE, KeyCode::PageDown) => app.page_down(),
            (KeyModifiers::NONE, KeyCode::Home) => app.handle_home(),
            (KeyModifiers::NONE, KeyCode::End) => app.handle_end(),
            (KeyModifiers::NONE, KeyCode::Up) => app.handle_up(),
            (KeyModifiers::NONE, KeyCode::Down) => app.handle_down(),
            (KeyModifiers::NONE, KeyCode::Left) => app.handle_left(),
            (KeyModifiers::NONE, KeyCode::Right) => app.handle_right(),
            (KeyModifiers::NONE, KeyCode::Esc) => self.handle_escape(app),
            (KeyModifiers::CONTROL, KeyCode::Char('/')) => {
                app.pending_key = None;
                app.toggle_help();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                app.pending_key = None;
                app.toggle_command_palette();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                app.pending_key = None;
                self.handle_ctrl_f(app);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('z')) => app.toggle_pause(),
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                app.pending_key = None;
                app.toggle_theme();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('v')) => {
                if !app.has_active_task() {
                    if let Some(text) = Clipboard::get() {
                        app.dispatcher_mut().handle_paste(&text);
                    } else {
                        tracing::debug!("Clipboard read failed or clipboard is empty");
                    }
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('y')) => {
                if app.is_paused() {
                    app.resume();
                } else if let Some(text) = app.dispatcher_mut().handle_copy() {
                    if !Clipboard::set(&text) {
                        tracing::warn!("Clipboard write failed");
                    }
                }
            }
            (KeyModifiers::NONE, KeyCode::Tab) => app.handle_focus_next(),
            (KeyModifiers::SHIFT, KeyCode::BackTab) => app.handle_focus_prev(),
            (KeyModifiers::NONE, KeyCode::Enter) => self.handle_enter(app),
            _ => return false,
        }
        true
    }

    fn handle_mode_specific_input(&self, app: &mut App, key: &crossterm::event::KeyEvent) -> bool {
        match app.mode {
            InputMode::Normal => self.handle_normal_mode_input(app, key),
            InputMode::Insert => self.handle_insert_mode_input(app, key),
        }
    }

    fn handle_normal_mode_input(&self, app: &mut App, key: &crossterm::event::KeyEvent) -> bool {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('i')) => self.handle_enter_insert_mode(app),
            (KeyModifiers::NONE, KeyCode::Char('q')) => self.handle_quit(app),
            (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                app.pending_key = None;
                app.toggle_help();
            }
            (KeyModifiers::NONE, KeyCode::Char('y')) => {
                if let Some(text) = app.dispatcher_mut().handle_copy() {
                    if !Clipboard::set(&text) {
                        tracing::warn!("Failed to copy to clipboard");
                    }
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                app.toggle_bookmark(app.current_tab);
                app.overlay.notification = Some(crate::app::notifications::Notification::new(
                    format!("Bookmarked: {}", app.current_tab.title()),
                    crate::app::notifications::NotificationSeverity::Info,
                ));
                app.needs_redraw = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('h')) => app.handle_left(),
            (KeyModifiers::NONE, KeyCode::Char('j')) => app.handle_down(),
            (KeyModifiers::NONE, KeyCode::Char('k')) => app.handle_up(),
            (KeyModifiers::NONE, KeyCode::Char('l')) => app.handle_right(),

            (KeyModifiers::NONE, KeyCode::Char('G')) => app.handle_bottom(),
            (KeyModifiers::NONE, KeyCode::Char('g')) => app.pending_key = Some(KeyCode::Char('g')),
            (KeyModifiers::NONE, KeyCode::Char('w')) => app.handle_word_forward(),
            (KeyModifiers::SHIFT, KeyCode::Char('B')) => app.handle_word_backward(),
            (KeyModifiers::NONE, KeyCode::Char('n')) => app.next_tab(),
            (KeyModifiers::NONE, KeyCode::Char('N')) => app.prev_tab(),
            (KeyModifiers::NONE, KeyCode::Char('p')) => app.prev_tab(),
            (KeyModifiers::SHIFT, KeyCode::Char('H')) => app.prev_tab(),
            (KeyModifiers::SHIFT, KeyCode::Char('L')) => app.next_tab(),
            (KeyModifiers::SHIFT, KeyCode::Char('E')) => {
                app.cycle_export_format();
                app.overlay.notification = Some(crate::app::notifications::Notification::new(
                    format!("Export format: {}", app.export_format),
                    crate::app::notifications::NotificationSeverity::Info,
                ));
                app.needs_redraw = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('/')) => app.toggle_search(false),
            (KeyModifiers::NONE, KeyCode::Char('r')) => self.handle_reset(app),
            (KeyModifiers::NONE, KeyCode::Char('s')) => self.handle_save_settings(app),
            (KeyModifiers::NONE, KeyCode::Char('d')) => self.handle_delete_entry(app),
            (KeyModifiers::NONE, KeyCode::Char('e')) => app.export_results(),
            // Tab jump: 1-9 jumps to tabs 1-9; 0 jumps to tab 10 (if available)
            (KeyModifiers::NONE, KeyCode::Char(c @ '1'..='9')) => {
                let idx = c.to_digit(10).unwrap() as usize;
                app.pending_key = None;
                if let Some(tab) = Tab::from_index(idx) {
                    app.set_current_tab_if_available(tab);
                    app.adjust_tab_scroll();
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('0')) => {
                app.pending_key = None;
                if let Some(tab) = Tab::from_index(9) {
                    app.set_current_tab_if_available(tab);
                    app.adjust_tab_scroll();
                }
            }
            _ => return false,
        }
        true
    }

    fn handle_insert_mode_input(&self, app: &mut App, key: &crossterm::event::KeyEvent) -> bool {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char(' ')) => {
                app.handle_autocomplete();
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => app.handle_backspace(),
            (KeyModifiers::NONE, KeyCode::Delete) => app.handle_delete(),
            (KeyModifiers::NONE, KeyCode::Char(c)) => app.handle_char(c),
            _ => return false,
        }
        true
    }

    fn handle_topmost_overlay(&self, app: &mut App, key: &crossterm::event::KeyEvent) -> bool {
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            return false;
        }

        match app.topmost_overlay() {
            Some(OverlayType::PolicyConfirm) => {
                // Policy confirmation (RequireConfirmation + manual override).
                // Enter = confirm with current reason (narrow semantics, dedicated flags mapped inside).
                // Esc = cancel (no spawn).
                // Any other char/backspace edits the reason line on the pending struct.
                match (key.modifiers, key.code) {
                    (KeyModifiers::NONE, KeyCode::Enter) => app.confirm_policy_action(),
                    (KeyModifiers::NONE, KeyCode::Esc) => app.cancel_policy_action(),
                    (KeyModifiers::NONE, KeyCode::Char(c)) => {
                        if let Some(p) = &mut app.overlay.pending_policy {
                            p.reason_input.push(c);
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Backspace) => {
                        if let Some(p) = &mut app.overlay.pending_policy {
                            p.reason_input.pop();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Delete) => {
                        if let Some(p) = &mut app.overlay.pending_policy {
                            p.reason_input.pop();
                        }
                    }
                    _ => {}
                }
                true
            }
            Some(OverlayType::ConfirmPopup) => {
                match (key.modifiers, key.code) {
                    (KeyModifiers::NONE, KeyCode::Enter) => app.confirm_action(),
                    (KeyModifiers::NONE, KeyCode::Esc) => app.cancel_action(),
                    (KeyModifiers::NONE, KeyCode::Char('y')) => app.confirm_action(),
                    (KeyModifiers::NONE, KeyCode::Char('n')) => app.cancel_action(),
                    _ => {}
                }
                true
            }
            Some(OverlayType::CommandPalette) => {
                self.handle_command_palette(app, key);
                true
            }
            Some(OverlayType::QuickSwitch) => {
                self.handle_quick_switch(app, key);
                true
            }
            Some(OverlayType::Search) => {
                self.handle_overlay_input(app, key);
                true
            }
            Some(OverlayType::HttpOptions) => {
                self.handle_overlay_input(app, key);
                true
            }
            Some(OverlayType::Help) => {
                self.handle_overlay_input(app, key);
                true
            }
            None => false,
        }
    }

    fn handle_ctrl_c(&self, app: &mut App) {
        if app.has_active_task() {
            app.stop_with_message("Interrupted by user");
        } else {
            app.should_quit = true;
        }
    }

    fn handle_ctrl_f(&self, app: &mut App) {
        if app.overlay.show_search {
            app.perform_search();
        } else {
            app.toggle_search(true);
            app.needs_redraw = true;
        }
    }

    fn handle_escape(&self, app: &mut App) {
        app.pending_key = None;
        match app.topmost_overlay() {
            Some(OverlayType::PolicyConfirm) => {
                app.cancel_policy_action();
            }
            Some(OverlayType::ConfirmPopup) => {
                app.cancel_action();
            }
            Some(OverlayType::CommandPalette) => {
                app.toggle_command_palette();
            }
            Some(OverlayType::Search) => {
                app.toggle_search(app.search.is_global);
            }
            Some(OverlayType::HttpOptions) => {
                app.overlay.show_http_options = false;
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
        if !app.has_active_task() {
            app.should_quit = true;
        }
    }

    fn handle_reset(&self, app: &mut App) {
        if !app.has_active_task() {
            if app.current_tab == Tab::History {
                app.request_confirmation(PendingAction::ClearHistory);
            } else {
                app.request_confirmation(PendingAction::ResetTab);
            }
        }
    }

    fn handle_save_settings(&self, app: &mut App) {
        if !app.has_active_task() && app.current_tab == Tab::Settings {
            app.request_confirmation(PendingAction::SaveSettings);
        }
    }

    fn handle_delete_entry(&self, app: &mut App) {
        if !app.has_active_task() && app.current_tab == Tab::History {
            app.request_confirmation(PendingAction::DeleteHistoryEntry);
        }
    }

    fn handle_enter(&self, app: &mut App) {
        if app.is_policy_confirm_visible() {
            app.confirm_policy_action();
        } else if app.is_confirm_popup_visible() {
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
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
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
                    if let Some(ref mut palette) = app.command_palette {
                        let max_idx = palette.results.len().saturating_sub(1);
                        if palette.selected_index > max_idx {
                            palette.selected_index = max_idx;
                        }
                    }
                }
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                if let Some(ref mut palette) = app.command_palette {
                    palette.query.push(c);
                    let new_query = palette.query.clone();
                    app.update_command_palette_query(&new_query);
                }
                if let Some(ref mut palette) = app.command_palette {
                    let max_idx = palette.results.len().saturating_sub(1);
                    if palette.selected_index > max_idx {
                        palette.selected_index = max_idx;
                    }
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
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.handle_escape(app);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('f')) if app.is_search_visible() => {
                app.perform_search();
            }
            (KeyModifiers::NONE, KeyCode::Backspace) if app.is_search_visible() => {
                app.search.query.pop();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) if app.is_search_visible() => {
                app.search.query.clear();
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) if app.is_search_visible() => {
                app.search.query.push(c);
            }
            (KeyModifiers::NONE, KeyCode::Char('h')) if app.is_http_options_visible() => {
                app.overlay.show_http_options = false;
                app.needs_redraw = true;
            }
            // Help overlay scrolling
            (KeyModifiers::NONE, KeyCode::Up | KeyCode::Char('k')) if app.is_help_visible() => {
                app.overlay.help_scroll_offset = app.overlay.help_scroll_offset.saturating_sub(1);
                app.needs_redraw = true;
            }
            (KeyModifiers::NONE, KeyCode::Down | KeyCode::Char('j')) if app.is_help_visible() => {
                app.overlay.help_scroll_offset = app.overlay.help_scroll_offset.saturating_add(1);
                app.needs_redraw = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) if app.is_help_visible() => {
                app.overlay.help_scroll_offset = 0;
                app.needs_redraw = true;
            }
            (KeyModifiers::NONE, KeyCode::Char('G')) if app.is_help_visible() => {
                // Scroll to bottom is set to a large value; the render
                // method clamps it.
                app.overlay.help_scroll_offset = usize::MAX;
                app.needs_redraw = true;
            }
            (KeyModifiers::NONE, KeyCode::PageUp) if app.is_help_visible() => {
                app.overlay.help_scroll_offset = app.overlay.help_scroll_offset.saturating_sub(10);
                app.needs_redraw = true;
            }
            (KeyModifiers::NONE, KeyCode::PageDown) if app.is_help_visible() => {
                app.overlay.help_scroll_offset = app.overlay.help_scroll_offset.saturating_add(10);
                app.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn handle_quick_switch(&self, app: &mut App, key: &crossterm::event::KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                app.close_quick_switch();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                app.close_quick_switch();
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                let results = app.get_quick_switch_results();
                if !results.is_empty() && app.quick_switch.selected < results.len() {
                    if let Some(tab) = results.get(app.quick_switch.selected) {
                        app.current_tab = **tab;
                        app.adjust_tab_scroll();
                    }
                }
                app.close_quick_switch();
            }
            (KeyModifiers::NONE, KeyCode::Up) if app.quick_switch.selected > 0 => {
                app.quick_switch.selected -= 1;
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                let results = app.get_quick_switch_results();
                if app.quick_switch.selected < results.len().saturating_sub(1) {
                    app.quick_switch.selected += 1;
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) | (KeyModifiers::NONE, KeyCode::PageUp) => {
                let results = app.get_quick_switch_results();
                if app.quick_switch.selected >= 10 {
                    app.quick_switch.selected -= 10;
                } else {
                    app.quick_switch.selected = 0;
                }
                if !results.is_empty() {
                    app.quick_switch.selected = app.quick_switch.selected.min(results.len() - 1);
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::NONE, KeyCode::PageDown) => {
                let results = app.get_quick_switch_results();
                if !results.is_empty() {
                    app.quick_switch.selected =
                        (app.quick_switch.selected + 10).min(results.len().saturating_sub(1));
                }
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                app.quick_switch.selected = 0;
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                let results = app.get_quick_switch_results();
                app.quick_switch.selected = results.len().saturating_sub(1);
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                app.quick_switch.query.pop();
                self.clamp_quick_switch_selection(app);
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                app.quick_switch.query.push(c);
                self.clamp_quick_switch_selection(app);
            }
            _ => {}
        }
    }

    fn clamp_quick_switch_selection(&self, app: &mut App) {
        let results = app.get_quick_switch_results();
        let len = results.len();
        app.quick_switch.selected = if len == 0 {
            0
        } else {
            app.quick_switch.selected.min(len - 1)
        };
    }
}

impl Default for KeyHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{create_shared_history, App};
    use crossterm::event::KeyEvent;

    fn create_test_app() -> App {
        App::new_for_testing(create_shared_history())
    }

    fn press(handler: &mut KeyHandler, app: &mut App, code: KeyCode) {
        handler.handle_key_event(app, &KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn press_ctrl(handler: &mut KeyHandler, app: &mut App, c: char) {
        handler.handle_key_event(app, &KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
    }

    #[test]
    fn test_quick_switch_down_is_not_stolen_by_tab_content() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();

        press_ctrl(&mut handler, &mut app, 'x');
        assert!(app.is_quick_switch_visible());

        press(&mut handler, &mut app, KeyCode::Down);

        assert_eq!(app.quick_switch.selected, 1);
    }

    #[test]
    fn test_quick_switch_paging_and_home_end_are_overlay_local() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();

        press_ctrl(&mut handler, &mut app, 'x');
        press(&mut handler, &mut app, KeyCode::End);
        assert_eq!(
            app.quick_switch.selected,
            app.get_quick_switch_results().len().saturating_sub(1)
        );

        press_ctrl(&mut handler, &mut app, 'u');
        assert!(app.quick_switch.selected < app.get_quick_switch_results().len());

        press(&mut handler, &mut app, KeyCode::Home);
        assert_eq!(app.quick_switch.selected, 0);
    }

    #[test]
    fn test_command_palette_down_is_not_stolen_by_tab_content() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();

        press_ctrl(&mut handler, &mut app, 'p');
        assert!(app.is_command_palette_visible());

        press(&mut handler, &mut app, KeyCode::Down);

        let palette = app.command_palette.as_ref().expect("palette should exist");
        let expected = if palette.results.len() > 1 { 1 } else { 0 };
        assert_eq!(palette.selected_index, expected);
    }

    #[test]
    fn test_search_ctrl_u_clears_query_instead_of_paging_content() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();

        app.overlay.show_search = true;
        app.search.query = "needle".to_string();

        press_ctrl(&mut handler, &mut app, 'u');

        assert!(app.search.query.is_empty());
        assert!(app.overlay.show_search);
    }

    #[test]
    fn test_confirm_popup_blocks_navigation_keys() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();
        let initial_tab = app.current_tab;

        app.request_confirmation(PendingAction::ResetTab);
        press(&mut handler, &mut app, KeyCode::Right);

        assert_eq!(app.current_tab, initial_tab);
        assert!(app.is_confirm_popup_visible());
    }

    #[test]
    fn test_backspace_does_not_edit_in_normal_mode() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();
        app.current_tab = Tab::Recon;
        app.mode = InputMode::Normal;
        app.tabs.recon.inputs.focus(0);
        app.tabs.recon.inputs.fields[0].value = "abc".to_string();
        app.tabs.recon.inputs.fields[0].cursor_pos = app.tabs.recon.inputs.fields[0].value.len();

        press(&mut handler, &mut app, KeyCode::Backspace);

        assert_eq!(app.tabs.recon.inputs.fields[0].value, "abc");
    }

    #[test]
    fn test_delete_edits_only_in_insert_mode() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();
        app.current_tab = Tab::Recon;
        app.tabs.recon.inputs.focus(0);
        app.tabs.recon.inputs.fields[0].value = "abc".to_string();
        app.tabs.recon.inputs.fields[0].cursor_pos = 1;

        app.mode = InputMode::Normal;
        press(&mut handler, &mut app, KeyCode::Delete);
        assert_eq!(app.tabs.recon.inputs.fields[0].value, "abc");

        app.mode = InputMode::Insert;
        press(&mut handler, &mut app, KeyCode::Delete);
        assert_eq!(app.tabs.recon.inputs.fields[0].value, "ac");
    }

    #[test]
    fn test_quick_switch_clamps_selection_after_filter_input() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();

        press_ctrl(&mut handler, &mut app, 'x');
        app.quick_switch.selected = app.get_quick_switch_results().len().saturating_sub(1);
        app.quick_switch.query = "recon".to_string();
        app.quick_switch.selected = app.get_quick_switch_results().len().saturating_sub(1);

        // Shrink results to a smaller set and ensure selection is clamped
        press(&mut handler, &mut app, KeyCode::Char('x'));

        let len = app.get_quick_switch_results().len();
        if len == 0 {
            assert_eq!(app.quick_switch.selected, 0);
        } else {
            assert!(app.quick_switch.selected < len);
        }
    }

    #[test]
    fn test_ctrl_c_stops_active_task_even_if_current_tab_not_running() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();
        app.current_tab = Tab::Dashboard;
        app.task_state.tab = Some(Tab::Recon);

        press_ctrl(&mut handler, &mut app, 'c');

        assert!(!app.should_quit);
        assert!(app.task_state.tab.is_none());
    }

    #[test]
    fn test_quit_is_blocked_when_active_task_exists() {
        let mut app = create_test_app();
        let mut handler = KeyHandler::new();
        app.task_state.tab = Some(Tab::Recon);

        press(&mut handler, &mut app, KeyCode::Char('q'));

        assert!(!app.should_quit);
    }
}
