use super::{SettingsSection, SettingsTab};
use crate::tui::tabs::TabInput;

impl TabInput for SettingsTab {
    fn handle_focus_next(&mut self) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.focus_next(),
            SettingsSection::Scan => self.scan_inputs.focus_next(),
            SettingsSection::Proxy => self.proxy_inputs.focus_next(),
            SettingsSection::Scope => self.scope_inputs.focus_next(),
            SettingsSection::Report => self.report_inputs.focus_next(),
            SettingsSection::Schedule => self.schedule_inputs.focus_next(),
            SettingsSection::Notifications => self.notify_inputs.focus_next(),
        }
    }

    fn handle_focus_prev(&mut self) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.focus_prev(),
            SettingsSection::Scan => self.scan_inputs.focus_prev(),
            SettingsSection::Proxy => self.proxy_inputs.focus_prev(),
            SettingsSection::Scope => self.scope_inputs.focus_prev(),
            SettingsSection::Report => self.report_inputs.focus_prev(),
            SettingsSection::Schedule => self.schedule_inputs.focus_prev(),
            SettingsSection::Notifications => self.notify_inputs.focus_prev(),
        }
    }

    fn handle_char(&mut self, c: char) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.insert(c),
            SettingsSection::Scan => self.scan_inputs.insert(c),
            SettingsSection::Proxy => self.proxy_inputs.insert(c),
            SettingsSection::Scope => self.scope_inputs.insert(c),
            SettingsSection::Report => self.report_inputs.insert(c),
            SettingsSection::Schedule => self.schedule_inputs.insert(c),
            SettingsSection::Notifications => self.notify_inputs.insert(c),
        }
    }

    fn handle_backspace(&mut self) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.backspace(),
            SettingsSection::Scan => self.scan_inputs.backspace(),
            SettingsSection::Proxy => self.proxy_inputs.backspace(),
            SettingsSection::Scope => self.scope_inputs.backspace(),
            SettingsSection::Report => self.report_inputs.backspace(),
            SettingsSection::Schedule => self.schedule_inputs.backspace(),
            SettingsSection::Notifications => self.notify_inputs.backspace(),
        }
    }

    fn handle_enter(&mut self) {
        match self.current_section {
            SettingsSection::Http => {
                if self.http_inputs.is_focused() {
                    self.http_inputs.blur();
                } else if self.follow_redirects.focused {
                    self.follow_redirects.toggle();
                } else if self.verify_tls.focused {
                    self.verify_tls.toggle();
                }
            }
            SettingsSection::Scan => {
                if self.scan_inputs.is_focused() {
                    self.scan_inputs.blur();
                } else if self.stealth_mode.focused {
                    self.stealth_mode.toggle();
                }
            }
            SettingsSection::Proxy => {
                if self.proxy_inputs.is_focused() {
                    self.proxy_inputs.blur();
                } else if self.proxy_rotation_selector.focused {
                    self.proxy_rotation_selector.toggle();
                }
            }
            SettingsSection::Scope => {
                if self.scope_inputs.is_focused() {
                    self.scope_inputs.blur();
                }
            }
            SettingsSection::Report => {
                if self.report_inputs.is_focused() {
                    self.report_inputs.blur();
                }
            }
            SettingsSection::Schedule => {
                if self.schedule_inputs.is_focused() {
                    self.schedule_inputs.blur();
                }
            }
            SettingsSection::Notifications => {
                if self.notify_inputs.is_focused() {
                    self.notify_inputs.blur();
                } else if self.notify_on_complete.focused {
                    self.notify_on_complete.toggle();
                } else if self.notify_on_findings.focused {
                    self.notify_on_findings.toggle();
                } else if self.severity_selector.focused {
                    self.severity_selector.toggle();
                }
            }
        }
    }

    fn handle_escape(&mut self) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.blur(),
            SettingsSection::Scan => self.scan_inputs.blur(),
            SettingsSection::Proxy => self.proxy_inputs.blur(),
            SettingsSection::Scope => self.scope_inputs.blur(),
            SettingsSection::Report => self.report_inputs.blur(),
            SettingsSection::Schedule => self.schedule_inputs.blur(),
            SettingsSection::Notifications => self.notify_inputs.blur(),
        }
    }

    fn handle_up(&mut self) {
        let sections = [
            SettingsSection::Http,
            SettingsSection::Scan,
            SettingsSection::Proxy,
            SettingsSection::Scope,
            SettingsSection::Report,
            SettingsSection::Schedule,
            SettingsSection::Notifications,
        ];
        if let Some(idx) = sections.iter().position(|s| *s == self.current_section) {
            if idx > 0 {
                self.current_section = sections[idx - 1];
            } else {
                self.current_section = sections[sections.len() - 1];
            }
        }
    }

    fn handle_down(&mut self) {
        let sections = [
            SettingsSection::Http,
            SettingsSection::Scan,
            SettingsSection::Proxy,
            SettingsSection::Scope,
            SettingsSection::Report,
            SettingsSection::Schedule,
            SettingsSection::Notifications,
        ];
        if let Some(idx) = sections.iter().position(|s| *s == self.current_section) {
            if idx < sections.len() - 1 {
                self.current_section = sections[idx + 1];
            } else {
                self.current_section = sections[0];
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.move_left(),
            SettingsSection::Scan => self.scan_inputs.move_left(),
            SettingsSection::Proxy => self.proxy_inputs.move_left(),
            SettingsSection::Scope => self.scope_inputs.move_left(),
            SettingsSection::Report => self.report_inputs.move_left(),
            SettingsSection::Schedule => self.schedule_inputs.move_left(),
            SettingsSection::Notifications => self.notify_inputs.move_left(),
        }
    }

    fn handle_right(&mut self) -> bool {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.move_right(),
            SettingsSection::Scan => self.scan_inputs.move_right(),
            SettingsSection::Proxy => self.proxy_inputs.move_right(),
            SettingsSection::Scope => self.scope_inputs.move_right(),
            SettingsSection::Report => self.report_inputs.move_right(),
            SettingsSection::Schedule => self.schedule_inputs.move_right(),
            SettingsSection::Notifications => self.notify_inputs.move_right(),
        }
    }

    fn is_input_focused(&self) -> bool {
        SettingsTab::is_input_focused(self)
    }
}
