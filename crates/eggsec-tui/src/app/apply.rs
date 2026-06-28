use super::action::{CommandPaletteInput, QuickSwitchInput, UiAction};
use super::App;
use crate::app::confirmation::PendingAction;
use crate::app::input::InputMode;
use crate::tabs::Tab;

impl App {
    /// Central mutation point: translates `UiAction` enums into method calls.
    ///
    /// This is the single entry-point for all visible-state-changing actions
    /// produced by the key decode layer. The decode layer is pure-ish (no
    /// transition; they continue to be used by tests, command palette
    /// execution, etc.
    ///
    /// All visible-state-changing actions set `needs_redraw = true`.
    /// Clipboard I/O and task spawning are performed here (side effects
    /// belong in apply, not in the pure-ish decode path).
    pub fn apply_action(&mut self, action: UiAction) {
        // All actions except Noop and BeginGgSequence trigger a redraw.
        // Individual arms that already set needs_redraw internally (e.g.
        // ToggleTheme) are safe because setting it twice is idempotent.
        if !matches!(action, UiAction::Noop | UiAction::BeginGgSequence) {
            self.needs_redraw = true;
        }

        match action {
            UiAction::Noop => {}

            // --- Overlay & mode toggles ---
            UiAction::Quit
            | UiAction::StopActiveTask { .. }
            | UiAction::ToggleHelp
            | UiAction::ToggleCommandPalette
            | UiAction::ToggleQuickSwitch
            | UiAction::CloseQuickSwitch
            | UiAction::ToggleSearch { .. }
            | UiAction::ToggleTheme
            | UiAction::TogglePause
            | UiAction::Resume
            | UiAction::ToggleEnforcementPosture => self.apply_overlay_action(action),

            // --- Focus & cursor movement ---
            UiAction::FocusNext
            | UiAction::FocusPrev
            | UiAction::PageUp
            | UiAction::PageDown
            | UiAction::MoveUp
            | UiAction::MoveDown
            | UiAction::MoveLeft
            | UiAction::MoveRight
            | UiAction::MoveTop
            | UiAction::MoveBottom
            | UiAction::BeginGgSequence
            | UiAction::MoveWordForward
            | UiAction::MoveWordBackward
            | UiAction::Home
            | UiAction::End => self.apply_nav_action(action),

            // --- Text input & commit/cancel ---
            UiAction::Enter
            | UiAction::Escape
            | UiAction::EnterInsertMode
            | UiAction::InputChar(_)
            | UiAction::Backspace
            | UiAction::Delete
            | UiAction::Autocomplete => self.apply_input_action(action),

            // --- Clipboard ---
            UiAction::Paste(_)
            | UiAction::Copy
            | UiAction::RequestPaste
            | UiAction::RequestCopy => self.apply_clipboard_action(action),

            // --- Tab navigation & export ---
            UiAction::SelectTab(_)
            | UiAction::NextTab
            | UiAction::PrevTab
            | UiAction::ToggleBookmark(_)
            | UiAction::CycleExportFormat
            | UiAction::ExportResults => self.apply_tab_action(action),

            // --- Confirmations & settings ---
            UiAction::ResetCurrent
            | UiAction::ReloadThemes
            | UiAction::SaveSettings
            | UiAction::DeleteHistoryEntry
            | UiAction::ConfirmPendingAction
            | UiAction::CancelPendingAction
            | UiAction::ConfirmButtonToggle
            | UiAction::ConfirmPolicyAction
            | UiAction::CancelPolicyAction
            | UiAction::PolicyReasonChar(_)
            | UiAction::PolicyReasonBackspace => self.apply_confirm_action(action),

            // --- Command palette sub-input ---
            UiAction::CommandPaletteInput(_) => self.apply_palette_action(action),

            // --- Quick switch sub-input ---
            UiAction::QuickSwitchInput(_) => self.apply_quick_switch_action(action),

            // --- Search, help scroll, HTTP options ---
            UiAction::SearchQueryChar(_)
            | UiAction::SearchQueryBackspace
            | UiAction::SearchQueryClear
            | UiAction::SearchPerform
            | UiAction::HelpScrollUp
            | UiAction::HelpScrollDown
            | UiAction::HelpScrollTop
            | UiAction::HelpScrollBottom
            | UiAction::HelpScrollPageUp
            | UiAction::HelpScrollPageDown
            | UiAction::HttpOptionsClose => self.apply_overlay_content_action(action),
        }
    }

    // ── Sub-dispatchers for apply_action ─────────────────────────────────

    pub(crate) fn apply_overlay_action(&mut self, action: UiAction) {
        match action {
            UiAction::Quit => {
                self.should_quit = true;
                self.needs_redraw = true;
            }
            UiAction::StopActiveTask { message } => {
                self.stop_with_message(&message);
                self.needs_redraw = true;
            }
            UiAction::ToggleHelp => {
                self.toggle_help();
                self.needs_redraw = true;
            }
            UiAction::ToggleCommandPalette => {
                self.toggle_command_palette();
                self.needs_redraw = true;
            }
            UiAction::ToggleQuickSwitch => {
                self.toggle_quick_switch();
                self.needs_redraw = true;
            }
            UiAction::CloseQuickSwitch => {
                self.close_quick_switch();
                self.needs_redraw = true;
            }
            UiAction::ToggleSearch { global } => {
                self.toggle_search(global);
                self.needs_redraw = true;
            }
            UiAction::ToggleTheme => {
                self.toggle_theme();
                // toggle_theme already sets notification + needs_redraw
            }
            UiAction::TogglePause => {
                self.toggle_pause();
                self.needs_redraw = true;
            }
            UiAction::Resume => {
                self.resume();
                self.needs_redraw = true;
            }
            UiAction::ToggleEnforcementPosture => {
                let new_profile = self.enforcement_state.toggle_posture();
                let mode = self.enforcement_state.mode_label();
                let message = match new_profile {
                    eggsec::config::ExecutionProfile::ManualPermissive => {
                        format!(
                            "TUI enforcement posture: {}. Warnings and explicit confirmations are available.",
                            mode
                        )
                    }
                    eggsec::config::ExecutionProfile::ManualGuarded => {
                        format!(
                            "TUI enforcement posture: {}. Scope ambiguity and confirmation cases will deny.",
                            mode
                        )
                    }
                    other => format!("TUI enforcement posture: {:?}", other),
                };
                self.overlay.notification = Some(crate::app::notifications::Notification::new(
                    message,
                    crate::app::notifications::NotificationSeverity::Info,
                ));
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    pub(crate) fn apply_nav_action(&mut self, action: UiAction) {
        match action {
            UiAction::FocusNext => {
                self.handle_focus_next();
                self.maybe_refresh_theme_preview();
                self.needs_redraw = true;
            }
            UiAction::FocusPrev => {
                self.handle_focus_prev();
                self.maybe_refresh_theme_preview();
                self.needs_redraw = true;
            }
            UiAction::PageUp => {
                self.page_up();
                self.needs_redraw = true;
            }
            UiAction::PageDown => {
                self.page_down();
                self.needs_redraw = true;
            }
            UiAction::MoveUp => {
                self.handle_up();
                self.needs_redraw = true;
            }
            UiAction::MoveDown => {
                self.handle_down();
                self.needs_redraw = true;
            }
            UiAction::MoveLeft => {
                self.handle_left();
                self.maybe_refresh_theme_preview();
                self.needs_redraw = true;
            }
            UiAction::MoveRight => {
                self.handle_right();
                self.maybe_refresh_theme_preview();
                self.needs_redraw = true;
            }
            UiAction::MoveTop => {
                self.handle_top();
                self.maybe_refresh_theme_preview();
                self.needs_redraw = true;
            }
            UiAction::MoveBottom => {
                self.handle_bottom();
                self.maybe_refresh_theme_preview();
                self.needs_redraw = true;
            }
            UiAction::BeginGgSequence => {
                self.pending_key = Some(crossterm::event::KeyCode::Char('g'));
                // No visible change on first g — do not set needs_redraw
            }
            UiAction::MoveWordForward => {
                self.handle_word_forward();
                self.needs_redraw = true;
            }
            UiAction::MoveWordBackward => {
                self.handle_word_backward();
                self.needs_redraw = true;
            }
            UiAction::Home => {
                self.handle_home();
                self.needs_redraw = true;
            }
            UiAction::End => {
                self.handle_end();
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    pub(crate) fn apply_input_action(&mut self, action: UiAction) {
        match action {
            UiAction::Enter => {
                self.handle_enter();
                self.needs_redraw = true;
            }
            UiAction::Escape => {
                self.handle_escape();
                self.needs_redraw = true;
            }
            UiAction::EnterInsertMode => {
                self.mode = InputMode::Insert;
                self.needs_redraw = true;
            }
            UiAction::InputChar(c) => {
                self.handle_char(c);
                self.needs_redraw = true;
            }
            UiAction::Backspace => {
                self.handle_backspace();
                self.needs_redraw = true;
            }
            UiAction::Delete => {
                self.handle_delete();
                self.needs_redraw = true;
            }
            UiAction::Autocomplete if self.handle_autocomplete() => {
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    /// Copy current tab's copyable text to the system clipboard.
    fn clipboard_copy_from_tab(&mut self) {
        use crate::utils::Clipboard;
        if let Some(text) = self.dispatcher_mut().handle_copy() {
            if !Clipboard::set(&text) {
                tracing::warn!("Clipboard write failed");
            }
        }
    }

    pub(crate) fn apply_clipboard_action(&mut self, action: UiAction) {
        use crate::utils::Clipboard;

        match action {
            UiAction::Paste(text) if !self.has_active_task() => {
                self.dispatcher_mut().handle_paste(&text);
                self.needs_redraw = true;
            }
            UiAction::Copy | UiAction::RequestCopy => {
                self.clipboard_copy_from_tab();
                self.needs_redraw = true;
            }
            UiAction::RequestPaste if !self.has_active_task() => {
                if let Some(text) = Clipboard::get() {
                    self.dispatcher_mut().handle_paste(&text);
                } else {
                    tracing::debug!("Clipboard read failed or clipboard is empty");
                }
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    pub(crate) fn apply_tab_action(&mut self, action: UiAction) {
        use crate::app::notifications::{Notification, NotificationSeverity};

        match action {
            UiAction::SelectTab(tab) => {
                self.set_current_tab_if_available(tab);
                self.adjust_tab_scroll();
                self.needs_redraw = true;
            }
            UiAction::NextTab => {
                self.next_tab();
                self.needs_redraw = true;
            }
            UiAction::PrevTab => {
                self.prev_tab();
                self.needs_redraw = true;
            }
            UiAction::ToggleBookmark(tab) => {
                self.toggle_bookmark(tab);
                self.overlay.notification = Some(Notification::new(
                    format!("Bookmarked: {}", tab.title()),
                    NotificationSeverity::Info,
                ));
                self.needs_redraw = true;
            }
            UiAction::CycleExportFormat => {
                self.cycle_export_format();
                self.overlay.notification = Some(Notification::new(
                    format!("Export format: {}", self.export_format),
                    NotificationSeverity::Info,
                ));
                self.needs_redraw = true;
            }
            UiAction::ExportResults => {
                self.export_results();
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    pub(crate) fn apply_confirm_action(&mut self, action: UiAction) {
        match action {
            UiAction::ResetCurrent if !self.has_active_task() => {
                if self.current_tab == Tab::History {
                    self.request_confirmation(PendingAction::ClearHistory);
                } else {
                    self.request_confirmation(PendingAction::ResetTab);
                }
                self.needs_redraw = true;
            }
            UiAction::ReloadThemes
                if !self.has_active_task()
                    && self.current_tab == Tab::Settings
                    && self.tabs.settings.current_section
                        == crate::tabs::SettingsSection::Theme
                    && !self.tabs.settings.theme_selector.is_open() =>
            {
                self.spawn_theme_loader_with_reason(
                    crate::app::state::ThemeLoadReason::ManualReload,
                );
                self.needs_redraw = true;
            }
            UiAction::SaveSettings
                if !self.has_active_task() && self.current_tab == Tab::Settings =>
            {
                self.request_confirmation(PendingAction::SaveSettings);
                self.needs_redraw = true;
            }
            UiAction::DeleteHistoryEntry
                if !self.has_active_task() && self.current_tab == Tab::History =>
            {
                self.request_confirmation(PendingAction::DeleteHistoryEntry);
                self.needs_redraw = true;
            }
            UiAction::ConfirmPendingAction => {
                if self.overlay.confirm_button_index == 0 {
                    self.confirm_action();
                } else {
                    self.cancel_action();
                }
                self.needs_redraw = true;
            }
            UiAction::CancelPendingAction => {
                self.cancel_action();
                self.needs_redraw = true;
            }
            UiAction::ConfirmButtonToggle => {
                self.overlay.confirm_button_index ^= 1;
                self.needs_redraw = true;
            }
            UiAction::ConfirmPolicyAction => {
                self.confirm_policy_action();
                self.needs_redraw = true;
            }
            UiAction::CancelPolicyAction => {
                self.cancel_policy_action();
                self.needs_redraw = true;
            }
            UiAction::PolicyReasonChar(c) => {
                if let Some(p) = &mut self.overlay.pending_policy {
                    p.reason_input.push(c);
                    self.needs_redraw = true;
                }
            }
            UiAction::PolicyReasonBackspace => {
                if let Some(p) = &mut self.overlay.pending_policy {
                    p.reason_input.pop();
                    self.needs_redraw = true;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn apply_palette_action(&mut self, action: UiAction) {
        let UiAction::CommandPaletteInput(pal_input) = action else {
            return;
        };

        match pal_input {
            CommandPaletteInput::Char(c) => {
                let q = if let Some(ref mut palette) = self.command_palette {
                    palette.query.push(c);
                    let q = palette.query.clone();
                    let max_idx = palette.results.len().saturating_sub(1);
                    if palette.selected_index > max_idx {
                        palette.selected_index = max_idx;
                    }
                    Some(q)
                } else {
                    None
                };
                if let Some(q) = q {
                    self.update_command_palette_query(&q);
                }
            }
            CommandPaletteInput::Backspace => {
                let new_query = if let Some(ref mut palette) = self.command_palette {
                    let query = palette.query.clone();
                    if !query.is_empty() {
                        palette.query.pop();
                        let new_query = palette.query.clone();
                        let max_idx = palette.results.len().saturating_sub(1);
                        if palette.selected_index > max_idx {
                            palette.selected_index = max_idx;
                        }
                        Some(new_query)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(q) = new_query {
                    self.update_command_palette_query(&q);
                }
            }
            CommandPaletteInput::Enter => {
                let index = self
                    .command_palette
                    .as_ref()
                    .map(|p| p.selected_index)
                    .unwrap_or(0);
                self.select_command_palette_item(index);
            }
            CommandPaletteInput::Up => {
                if let Some(ref mut palette) = self.command_palette {
                    if palette.selected_index > 0 {
                        palette.selected_index -= 1;
                    }
                    if palette.selected_index < palette.scroll_offset {
                        palette.scroll_offset = palette.selected_index;
                    }
                }
            }
            CommandPaletteInput::Down => {
                if let Some(ref mut palette) = self.command_palette {
                    let max_idx = palette.results.len().saturating_sub(1);
                    if palette.selected_index < max_idx {
                        palette.selected_index += 1;
                    }
                    palette.adjust_scroll_for_selection();
                }
            }
            CommandPaletteInput::Tab => {
                if let Some(ref mut palette) = self.command_palette {
                    let max_idx = palette.results.len().saturating_sub(1);
                    if palette.selected_index < max_idx {
                        palette.selected_index += 1;
                    }
                    palette.adjust_scroll_for_selection();
                }
            }
            CommandPaletteInput::BackTab => {
                if let Some(ref mut palette) = self.command_palette {
                    if palette.selected_index > 0 {
                        palette.selected_index -= 1;
                    }
                    if palette.selected_index < palette.scroll_offset {
                        palette.scroll_offset = palette.selected_index;
                    }
                }
            }
            CommandPaletteInput::Esc | CommandPaletteInput::Close => {
                self.toggle_command_palette();
            }
        }
        self.needs_redraw = true;
    }

    pub(crate) fn apply_quick_switch_action(&mut self, action: UiAction) {
        let UiAction::QuickSwitchInput(qs_input) = action else {
            return;
        };

        match qs_input {
            QuickSwitchInput::Char(c) => {
                self.quick_switch.query.push(c);
                self.clamp_quick_switch_selection_internal();
            }
            QuickSwitchInput::Backspace => {
                self.quick_switch.query.pop();
                self.clamp_quick_switch_selection_internal();
            }
            QuickSwitchInput::Enter => {
                let results = self.get_quick_switch_results();
                if !results.is_empty() && self.quick_switch.selected < results.len() {
                    if let Some(tab) = results.get(self.quick_switch.selected) {
                        self.current_tab = **tab;
                        self.adjust_tab_scroll();
                    }
                }
                self.close_quick_switch();
            }
            QuickSwitchInput::Up => {
                if self.quick_switch.selected > 0 {
                    self.quick_switch.selected -= 1;
                }
            }
            QuickSwitchInput::Down => {
                let results = self.get_quick_switch_results();
                if self.quick_switch.selected < results.len().saturating_sub(1) {
                    self.quick_switch.selected += 1;
                }
            }
            QuickSwitchInput::PageUp => {
                let results = self.get_quick_switch_results();
                if self.quick_switch.selected >= 10 {
                    self.quick_switch.selected -= 10;
                } else {
                    self.quick_switch.selected = 0;
                }
                if !results.is_empty() {
                    self.quick_switch.selected = self.quick_switch.selected.min(results.len() - 1);
                }
            }
            QuickSwitchInput::PageDown => {
                let results = self.get_quick_switch_results();
                if !results.is_empty() {
                    self.quick_switch.selected =
                        (self.quick_switch.selected + 10).min(results.len().saturating_sub(1));
                }
            }
            QuickSwitchInput::Home => {
                self.quick_switch.selected = 0;
            }
            QuickSwitchInput::End => {
                let results = self.get_quick_switch_results();
                self.quick_switch.selected = results.len().saturating_sub(1);
            }
            QuickSwitchInput::Esc | QuickSwitchInput::Close => {
                self.close_quick_switch();
            }
        }
        self.needs_redraw = true;
    }

    /// Scroll the help overlay by `delta` lines (negative = up).
    fn scroll_help_overlay(&mut self, delta: isize) {
        if self.is_help_visible() {
            self.overlay.help_scroll_offset = if delta < 0 {
                self.overlay
                    .help_scroll_offset
                    .saturating_sub(delta.unsigned_abs())
            } else {
                self.overlay
                    .help_scroll_offset
                    .saturating_add(delta as usize)
            };
            self.needs_redraw = true;
        }
    }

    pub(crate) fn apply_overlay_content_action(&mut self, action: UiAction) {
        match action {
            UiAction::SearchQueryChar(c) if self.is_search_visible() => {
                self.search.query.push(c);
                self.needs_redraw = true;
            }
            UiAction::SearchQueryBackspace if self.is_search_visible() => {
                self.search.query.pop();
                self.needs_redraw = true;
            }
            UiAction::SearchQueryClear if self.is_search_visible() => {
                self.search.query.clear();
                self.needs_redraw = true;
            }
            UiAction::SearchPerform if self.is_search_visible() => {
                self.perform_search();
                self.needs_redraw = true;
            }
            UiAction::HelpScrollUp => self.scroll_help_overlay(-1),
            UiAction::HelpScrollDown => self.scroll_help_overlay(1),
            UiAction::HelpScrollPageUp => self.scroll_help_overlay(-10),
            UiAction::HelpScrollPageDown => self.scroll_help_overlay(10),
            UiAction::HelpScrollTop if self.is_help_visible() => {
                self.overlay.help_scroll_offset = 0;
                self.needs_redraw = true;
            }
            UiAction::HelpScrollBottom if self.is_help_visible() => {
                self.overlay.help_scroll_offset = u16::MAX as usize;
                self.needs_redraw = true;
            }
            UiAction::HttpOptionsClose if self.is_http_options_visible() => {
                self.overlay.show_http_options = false;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    /// Apply a batch of actions in order.
    pub fn apply_actions(&mut self, actions: Vec<UiAction>) {
        for a in actions {
            self.apply_action(a);
        }
    }

    /// Internal helper used by QuickSwitch apply (keeps clamp logic in one place).
    pub(crate) fn clamp_quick_switch_selection_internal(&mut self) {
        let results = self.get_quick_switch_results();
        let len = results.len();
        self.quick_switch.selected = if len == 0 {
            0
        } else {
            self.quick_switch.selected.min(len - 1)
        };
    }
}
