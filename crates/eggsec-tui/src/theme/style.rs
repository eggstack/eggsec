use ratatui::style::{Modifier, Style};

use super::palette::Theme;

impl Theme {
    pub fn style_for_tab(&self, active: bool) -> Style {
        if active {
            Style::default()
                .fg(self.colors.tab_active)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.colors.tab_inactive)
        }
    }

    pub fn style_for_mode(&self, normal: bool) -> Style {
        if normal {
            Style::default()
                .fg(self.colors.selected_text)
                .bg(self.colors.mode_normal)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(self.colors.selected_text)
                .bg(self.colors.mode_insert)
                .add_modifier(Modifier::BOLD)
        }
    }

    pub fn style_for_status(&self, running: bool, has_error: bool) -> Style {
        if has_error {
            Style::default().fg(self.colors.status_error)
        } else if running {
            Style::default().fg(self.colors.status_running)
        } else {
            Style::default().fg(self.colors.status_idle)
        }
    }

    pub fn border_style(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.colors.border_focused)
        } else {
            Style::default().fg(self.colors.border)
        }
    }

    pub fn safe(&self) -> Style {
        Style::default().fg(self.colors.safe)
    }

    pub fn danger(&self) -> Style {
        Style::default().fg(self.colors.danger)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.colors.muted)
    }

    pub fn active_task(&self) -> Style {
        Style::default().fg(self.colors.active_task)
    }

    pub fn paused_task(&self) -> Style {
        Style::default().fg(self.colors.paused_task)
    }

    pub fn scope_match(&self) -> Style {
        Style::default().fg(self.colors.scope_match)
    }

    pub fn scope_miss(&self) -> Style {
        Style::default().fg(self.colors.scope_miss)
    }

    pub fn policy_required(&self) -> Style {
        Style::default().fg(self.colors.policy_required)
    }

    pub fn policy_denied(&self) -> Style {
        Style::default().fg(self.colors.policy_denied)
    }

    pub fn style_for_risk(&self, risk: &str) -> Style {
        match risk {
            "passive" | "safe" => self.safe(),
            "intrusive" => self.danger(),
            "admin" | "administrative" => self.danger(),
            _ => self.muted(),
        }
    }

    pub fn style_for_policy_outcome(&self, outcome: &str) -> Style {
        match outcome {
            "run" | "allow" => self.safe(),
            "warn" => self.policy_required(),
            "confirm" | "require" => self.policy_required(),
            "deny" => self.policy_denied(),
            _ => self.muted(),
        }
    }

    pub fn style_for_task_state(&self, state: &str) -> Style {
        match state {
            "running" | "active" => self.active_task(),
            "paused" => self.paused_task(),
            _ => self.muted(),
        }
    }
}
