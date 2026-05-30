use super::{SettingsFocusArea, SettingsSection, SettingsTab};
use crate::tui::tabs::{TabInput, TabState};

impl TabInput for SettingsTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == SettingsFocusArea::SectionList {
            self.focus_area = SettingsFocusArea::SectionDetail;
            self.detail_focus_index = 0;
        } else {
            let max = self.max_focus_index();
            if self.detail_focus_index >= max {
                self.focus_area = SettingsFocusArea::SectionList;
                self.detail_focus_index = 0;
            } else {
                self.detail_focus_index += 1;
            }
        }
        self.sync_component_focus();
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == SettingsFocusArea::SectionList {
            self.focus_area = SettingsFocusArea::SectionDetail;
            self.detail_focus_index = self.max_focus_index();
        } else {
            if self.detail_focus_index == 0 {
                self.focus_area = SettingsFocusArea::SectionList;
            } else {
                self.detail_focus_index -= 1;
            }
        }
        self.sync_component_focus();
    }

    fn handle_char(&mut self, c: char) {
        if self.is_running() {
            return;
        }
        match self.current_section {
            SettingsSection::Http => self.http_inputs.insert(c),
            SettingsSection::Scan => self.scan_inputs.insert(c),
            SettingsSection::Session => self.session_inputs.insert(c),
            SettingsSection::Proxy => self.proxy_inputs.insert(c),
            SettingsSection::Scope => self.scope_inputs.insert(c),
            SettingsSection::Report => self.report_inputs.insert(c),
            SettingsSection::Schedule => self.schedule_inputs.insert(c),
            SettingsSection::Notifications => self.notify_inputs.insert(c),
            SettingsSection::Theme => {}
        }
    }

    fn handle_backspace(&mut self) {
        if self.is_running() {
            return;
        }
        match self.current_section {
            SettingsSection::Http => self.http_inputs.backspace(),
            SettingsSection::Scan => self.scan_inputs.backspace(),
            SettingsSection::Session => self.session_inputs.backspace(),
            SettingsSection::Proxy => self.proxy_inputs.backspace(),
            SettingsSection::Scope => self.scope_inputs.backspace(),
            SettingsSection::Report => self.report_inputs.backspace(),
            SettingsSection::Schedule => self.schedule_inputs.backspace(),
            SettingsSection::Notifications => self.notify_inputs.backspace(),
            SettingsSection::Theme => {}
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if self.is_running() {
            return;
        }
        match self.current_section {
            SettingsSection::Http => self.http_inputs.paste(text),
            SettingsSection::Scan => self.scan_inputs.paste(text),
            SettingsSection::Session => self.session_inputs.paste(text),
            SettingsSection::Proxy => self.proxy_inputs.paste(text),
            SettingsSection::Scope => self.scope_inputs.paste(text),
            SettingsSection::Report => self.report_inputs.paste(text),
            SettingsSection::Schedule => self.schedule_inputs.paste(text),
            SettingsSection::Notifications => self.notify_inputs.paste(text),
            SettingsSection::Theme => {}
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        match self.current_section {
            SettingsSection::Http => self.http_inputs.get_focused_value(),
            SettingsSection::Scan => self.scan_inputs.get_focused_value(),
            SettingsSection::Session => self.session_inputs.get_focused_value(),
            SettingsSection::Proxy => self.proxy_inputs.get_focused_value(),
            SettingsSection::Scope => self.scope_inputs.get_focused_value(),
            SettingsSection::Report => self.report_inputs.get_focused_value(),
            SettingsSection::Schedule => self.schedule_inputs.get_focused_value(),
            SettingsSection::Notifications => self.notify_inputs.get_focused_value(),
            SettingsSection::Theme => None,
        }
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        match self.current_section {
            SettingsSection::Http => self.http_inputs.move_word_forward(),
            SettingsSection::Scan => self.scan_inputs.move_word_forward(),
            SettingsSection::Session => self.session_inputs.move_word_forward(),
            SettingsSection::Proxy => self.proxy_inputs.move_word_forward(),
            SettingsSection::Scope => self.scope_inputs.move_word_forward(),
            SettingsSection::Report => self.report_inputs.move_word_forward(),
            SettingsSection::Schedule => self.schedule_inputs.move_word_forward(),
            SettingsSection::Notifications => self.notify_inputs.move_word_forward(),
            SettingsSection::Theme => {}
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        match self.current_section {
            SettingsSection::Http => self.http_inputs.move_word_backward(),
            SettingsSection::Scan => self.scan_inputs.move_word_backward(),
            SettingsSection::Session => self.session_inputs.move_word_backward(),
            SettingsSection::Proxy => self.proxy_inputs.move_word_backward(),
            SettingsSection::Scope => self.scope_inputs.move_word_backward(),
            SettingsSection::Report => self.report_inputs.move_word_backward(),
            SettingsSection::Schedule => self.schedule_inputs.move_word_backward(),
            SettingsSection::Notifications => self.notify_inputs.move_word_backward(),
            SettingsSection::Theme => {}
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        match self.current_section {
            SettingsSection::Http => self.http_inputs.move_home(),
            SettingsSection::Scan => self.scan_inputs.move_home(),
            SettingsSection::Session => self.session_inputs.move_home(),
            SettingsSection::Proxy => self.proxy_inputs.move_home(),
            SettingsSection::Scope => self.scope_inputs.move_home(),
            SettingsSection::Report => self.report_inputs.move_home(),
            SettingsSection::Schedule => self.schedule_inputs.move_home(),
            SettingsSection::Notifications => self.notify_inputs.move_home(),
            SettingsSection::Theme => {}
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        match self.current_section {
            SettingsSection::Http => self.http_inputs.move_end(),
            SettingsSection::Scan => self.scan_inputs.move_end(),
            SettingsSection::Session => self.session_inputs.move_end(),
            SettingsSection::Proxy => self.proxy_inputs.move_end(),
            SettingsSection::Scope => self.scope_inputs.move_end(),
            SettingsSection::Report => self.report_inputs.move_end(),
            SettingsSection::Schedule => self.schedule_inputs.move_end(),
            SettingsSection::Notifications => self.notify_inputs.move_end(),
            SettingsSection::Theme => {}
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.current_section = SettingsSection::Http;
        self.detail_focus_index = 0;
        self.sync_component_focus();
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.current_section = SettingsSection::Theme;
        self.detail_focus_index = 0;
        self.sync_component_focus();
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == SettingsFocusArea::SectionList {
            self.focus_area = SettingsFocusArea::SectionDetail;
            self.detail_focus_index = 0;
            self.sync_component_focus();
            return;
        }

        let idx = self.detail_focus_index;
        match self.current_section {
            SettingsSection::Http => {
                if idx < 4 {
                    self.http_inputs.blur();
                } else if idx == 4 {
                    self.follow_redirects.toggle();
                } else {
                    self.verify_tls.toggle();
                }
            }
            SettingsSection::Scan => {
                if idx < 3 {
                    self.scan_inputs.blur();
                } else {
                    self.stealth_mode.toggle();
                }
            }
            SettingsSection::Session => {
                self.session_inputs.blur();
            }
            SettingsSection::Proxy => {
                if idx < 2 {
                    self.proxy_inputs.blur();
                } else {
                    if self.proxy_rotation_selector.is_open() {
                        if self.proxy_rotation_selector.confirm().is_none() {
                            tracing::warn!("Failed to confirm proxy rotation selector");
                        }
                    } else {
                        self.proxy_rotation_selector.open();
                    }
                }
            }
            SettingsSection::Scope => {
                self.scope_inputs.blur();
            }
            SettingsSection::Report => {
                self.report_inputs.blur();
            }
            SettingsSection::Schedule => {
                self.schedule_inputs.blur();
            }
            SettingsSection::Notifications => {
                if idx < 4 {
                    self.notify_inputs.blur();
                } else if idx == 4 {
                    self.notify_on_complete.toggle();
                } else if idx == 5 {
                    self.notify_on_findings.toggle();
                } else {
                    if self.severity_selector.is_open() {
                        if self.severity_selector.confirm().is_none() {
                            tracing::warn!("Failed to confirm severity selector");
                        }
                    } else {
                        self.severity_selector.open();
                    }
                }
            }
            SettingsSection::Theme => {
                if idx == 0 {
                    self.dark_mode.toggle();
                } else {
                    if self.accent_color.is_open() {
                        if self.accent_color.confirm().is_none() {
                            tracing::warn!("Failed to confirm accent color selector");
                        }
                    } else {
                        self.accent_color.open();
                    }
                }
            }
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            return;
        }
        if self.proxy_rotation_selector.is_open() {
            self.proxy_rotation_selector.cancel();
            return;
        }
        if self.severity_selector.is_open() {
            self.severity_selector.cancel();
            return;
        }
        if self.accent_color.is_open() {
            self.accent_color.cancel();
            return;
        }
        self.focus_area = SettingsFocusArea::SectionList;
        self.sync_component_focus();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == SettingsFocusArea::SectionList {
            let sections = [
                SettingsSection::Http,
                SettingsSection::Scan,
                SettingsSection::Proxy,
                SettingsSection::Scope,
                SettingsSection::Report,
                SettingsSection::Schedule,
                SettingsSection::Notifications,
                SettingsSection::Theme,
            ];
            if let Some(idx) = sections.iter().position(|s| *s == self.current_section) {
                if idx > 0 {
                    self.current_section = sections[idx - 1];
                } else {
                    self.current_section = sections[sections.len() - 1];
                }
            }
            self.detail_focus_index = 0;
            self.sync_component_focus();
        } else {
            if self.proxy_rotation_selector.is_open() {
                self.proxy_rotation_selector.move_prev();
                return;
            }
            if self.severity_selector.is_open() {
                self.severity_selector.move_prev();
                return;
            }
            if self.accent_color.is_open() {
                self.accent_color.move_prev();
                return;
            }
            // In detail view, up moves between fields
            if self.detail_focus_index > 0 {
                self.detail_focus_index -= 1;
            } else {
                self.detail_focus_index = self.max_focus_index();
            }
            self.sync_component_focus();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == SettingsFocusArea::SectionList {
            let sections = [
                SettingsSection::Http,
                SettingsSection::Scan,
                SettingsSection::Proxy,
                SettingsSection::Scope,
                SettingsSection::Report,
                SettingsSection::Schedule,
                SettingsSection::Notifications,
                SettingsSection::Theme,
            ];
            if let Some(idx) = sections.iter().position(|s| *s == self.current_section) {
                if idx < sections.len() - 1 {
                    self.current_section = sections[idx + 1];
                } else {
                    self.current_section = sections[0];
                }
            }
            self.detail_focus_index = 0;
            self.sync_component_focus();
        } else {
            if self.proxy_rotation_selector.is_open() {
                self.proxy_rotation_selector.move_next();
                return;
            }
            if self.severity_selector.is_open() {
                self.severity_selector.move_next();
                return;
            }
            if self.accent_color.is_open() {
                self.accent_color.move_next();
                return;
            }
            // In detail view, down moves between fields
            let max = self.max_focus_index();
            if self.detail_focus_index < max {
                self.detail_focus_index += 1;
            } else {
                self.detail_focus_index = 0;
            }
            self.sync_component_focus();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == SettingsFocusArea::SectionDetail {
            if self.is_input_focused() {
                if self.is_at_left_edge() {
                    self.focus_area = SettingsFocusArea::SectionList;
                    self.sync_component_focus();
                    true
                } else {
                    match self.current_section {
                        SettingsSection::Http => self.http_inputs.move_left(),
                        SettingsSection::Scan => self.scan_inputs.move_left(),
                        SettingsSection::Proxy => self.proxy_inputs.move_left(),
                        SettingsSection::Scope => self.scope_inputs.move_left(),
                        SettingsSection::Report => self.report_inputs.move_left(),
                        SettingsSection::Schedule => self.schedule_inputs.move_left(),
                        SettingsSection::Notifications => self.notify_inputs.move_left(),
                        _ => false,
                    }
                }
            } else {
                self.focus_area = SettingsFocusArea::SectionList;
                self.sync_component_focus();
                true
            }
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == SettingsFocusArea::SectionList {
            self.focus_area = SettingsFocusArea::SectionDetail;
            self.sync_component_focus();
            true
        } else if self.is_input_focused() {
            match self.current_section {
                SettingsSection::Http => self.http_inputs.move_right(),
                SettingsSection::Scan => self.scan_inputs.move_right(),
                SettingsSection::Proxy => self.proxy_inputs.move_right(),
                SettingsSection::Scope => self.scope_inputs.move_right(),
                SettingsSection::Report => self.report_inputs.move_right(),
                SettingsSection::Schedule => self.schedule_inputs.move_right(),
                SettingsSection::Notifications => self.notify_inputs.move_right(),
                _ => false,
            }
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == SettingsFocusArea::SectionList {
            true
        } else if self.is_input_focused() {
            match self.current_section {
                SettingsSection::Http => self.http_inputs.is_at_left_edge(),
                SettingsSection::Scan => self.scan_inputs.is_at_left_edge(),
                SettingsSection::Proxy => self.proxy_inputs.is_at_left_edge(),
                SettingsSection::Scope => self.scope_inputs.is_at_left_edge(),
                SettingsSection::Report => self.report_inputs.is_at_left_edge(),
                SettingsSection::Schedule => self.schedule_inputs.is_at_left_edge(),
                SettingsSection::Notifications => self.notify_inputs.is_at_left_edge(),
                _ => true,
            }
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == SettingsFocusArea::SectionDetail && self.is_input_focused() {
            match self.current_section {
                SettingsSection::Http => self.http_inputs.is_at_right_edge(),
                SettingsSection::Scan => self.scan_inputs.is_at_right_edge(),
                SettingsSection::Proxy => self.proxy_inputs.is_at_right_edge(),
                SettingsSection::Scope => self.scope_inputs.is_at_right_edge(),
                SettingsSection::Report => self.report_inputs.is_at_right_edge(),
                SettingsSection::Schedule => self.schedule_inputs.is_at_right_edge(),
                SettingsSection::Notifications => self.notify_inputs.is_at_right_edge(),
                _ => true,
            }
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        SettingsTab::is_input_focused(self)
    }
}
