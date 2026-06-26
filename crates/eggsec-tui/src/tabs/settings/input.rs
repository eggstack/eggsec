use super::{SettingsFocusArea, SettingsSection, SettingsTab};
use crate::tabs::{TabInput, TabState};

impl TabInput for SettingsTab {
    fn handle_focus_next(&mut self) {
        if self.theme_selector.is_open() {
            self.theme_selector.cancel();
            self.restore_theme_preview_selection();
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
        if self.theme_selector.is_open() {
            self.theme_selector.cancel();
            self.restore_theme_preview_selection();
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
            SettingsSection::Theme => {
                if c == 'r' && !self.theme_selector.is_open() {
                    self.pending_theme_reload = true;
                }
            }
            _ => {
                if let Some(inputs) = self.current_text_inputs_mut() {
                    inputs.insert(c);
                }
            }
        }
    }

    fn handle_backspace(&mut self) {
        if self.is_running() {
            return;
        }
        if let Some(inputs) = self.current_text_inputs_mut() {
            inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if self.is_running() {
            return;
        }
        if let Some(inputs) = self.current_text_inputs_mut() {
            inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        self.current_text_inputs()
            .and_then(|inputs| inputs.get_focused_value())
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        if let Some(inputs) = self.current_text_inputs_mut() {
            inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        if let Some(inputs) = self.current_text_inputs_mut() {
            inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if let Some(inputs) = self.current_text_inputs_mut() {
            inputs.move_home();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if let Some(inputs) = self.current_text_inputs_mut() {
            inputs.move_end();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        if self.theme_selector.is_open() {
            self.theme_selector.cancel();
            self.restore_theme_preview_selection();
        }
        self.current_section = SettingsSection::Http;
        self.detail_focus_index = 0;
        self.sync_component_focus();
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        if self.theme_selector.is_open() {
            self.theme_selector.cancel();
            self.restore_theme_preview_selection();
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
                if self.theme_selector.is_open() {
                    if let Some(item) = self.theme_selector.confirm() {
                        self.pending_theme_name = Some(item.value.clone());
                    } else {
                        tracing::warn!("Failed to confirm theme selector");
                    }
                } else {
                    self.theme_selector.open();
                    self.needs_theme_preview_refresh = true;
                }
            }
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
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
        if self.theme_selector.is_open() {
            self.theme_selector.cancel();
            // Restore selector to the applied theme so the dropdown
            // reflects what's actually active, not whatever the user
            // was previewing before they pressed Escape.
            self.restore_theme_preview_selection();
            return;
        }
        self.focus_area = SettingsFocusArea::SectionList;
        self.sync_component_focus();
    }

    fn handle_up(&mut self) {
        if self.focus_area == SettingsFocusArea::SectionList {
            let sections = [
                SettingsSection::Http,
                SettingsSection::Scan,
                SettingsSection::Session,
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
            if self.theme_selector.is_open() {
                self.theme_selector.move_prev();
                self.needs_theme_preview_refresh = true;
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
        if self.focus_area == SettingsFocusArea::SectionList {
            let sections = [
                SettingsSection::Http,
                SettingsSection::Scan,
                SettingsSection::Session,
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
            if self.theme_selector.is_open() {
                self.theme_selector.move_next();
                self.needs_theme_preview_refresh = true;
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
                    self.current_text_inputs_mut()
                        .map(|inputs| inputs.move_left())
                        .unwrap_or(false)
                }
            } else {
                if self.theme_selector.is_open() {
                    self.theme_selector.cancel();
                    self.restore_theme_preview_selection();
                }
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
            self.current_text_inputs_mut()
                .map(|inputs| inputs.move_right())
                .unwrap_or(false)
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == SettingsFocusArea::SectionList {
            true
        } else if self.is_input_focused() {
            self.current_text_inputs()
                .map(|inputs| inputs.is_at_left_edge())
                .unwrap_or(true)
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == SettingsFocusArea::SectionDetail && self.is_input_focused() {
            self.current_text_inputs()
                .map(|inputs| inputs.is_at_right_edge())
                .unwrap_or(true)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        SettingsTab::is_input_focused(self)
    }
}
