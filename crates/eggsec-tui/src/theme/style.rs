use ratatui::style::{Modifier, Style};

use super::palette::Theme;

impl Theme {
    pub fn border_style(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.colors.border_focused)
        } else {
            Style::default().fg(self.colors.border)
        }
    }

    pub fn primary(&self) -> Style {
        Style::default().fg(self.colors.primary)
    }

    pub fn secondary(&self) -> Style {
        Style::default().fg(self.colors.secondary)
    }

    pub fn accent(&self) -> Style {
        Style::default().fg(self.colors.accent)
    }

    pub fn background(&self) -> Style {
        Style::default().fg(self.colors.background)
    }

    pub fn surface(&self) -> Style {
        Style::default().fg(self.colors.surface)
    }

    pub fn text(&self) -> Style {
        Style::default().fg(self.colors.text)
    }

    pub fn text_dim(&self) -> Style {
        Style::default().fg(self.colors.text_dim)
    }

    pub fn text_bright(&self) -> Style {
        Style::default().fg(self.colors.text_bright)
    }

    pub fn success(&self) -> Style {
        Style::default().fg(self.colors.success)
    }

    pub fn warning(&self) -> Style {
        Style::default().fg(self.colors.warning)
    }

    pub fn error(&self) -> Style {
        Style::default().fg(self.colors.error)
    }

    pub fn info(&self) -> Style {
        Style::default().fg(self.colors.info)
    }

    pub fn highlight(&self) -> Style {
        Style::default().fg(self.colors.highlight)
    }

    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.colors.selected_text)
            .bg(self.colors.selected)
    }

    pub fn focus_input(&self) -> Style {
        Style::default()
            .fg(self.colors.focus_input)
            .add_modifier(Modifier::BOLD)
    }

    pub fn focus_results(&self) -> Style {
        Style::default().fg(self.colors.focus_results)
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

    pub fn tab_active(&self) -> Style {
        Style::default().fg(self.colors.tab_active)
    }

    pub fn tab_inactive(&self) -> Style {
        Style::default().fg(self.colors.tab_inactive)
    }

    pub fn status_running(&self) -> Style {
        Style::default().fg(self.colors.status_running)
    }

    pub fn status_idle(&self) -> Style {
        Style::default().fg(self.colors.status_idle)
    }

    pub fn status_error(&self) -> Style {
        Style::default().fg(self.colors.status_error)
    }

    pub fn mode_normal(&self) -> Style {
        Style::default().fg(self.colors.mode_normal)
    }

    pub fn mode_insert(&self) -> Style {
        Style::default().fg(self.colors.mode_insert)
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

    pub fn style_for_severity(&self, severity: &str) -> Style {
        match severity.to_lowercase().as_str() {
            "critical" | "high" => self.danger(),
            "medium" | "moderate" => self.warning(),
            "low" | "info" | "informational" => self.info(),
            _ => self.muted(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> Theme {
        crate::test_utils::test_theme()
    }

    #[test]
    fn all_style_methods_produce_non_default_styles() {
        let theme = test_theme();
        let styles = [
            theme.primary(),
            theme.secondary(),
            theme.accent(),
            theme.text(),
            theme.text_dim(),
            theme.text_bright(),
            theme.success(),
            theme.warning(),
            theme.error(),
            theme.info(),
            theme.highlight(),
            theme.safe(),
            theme.danger(),
            theme.muted(),
            theme.active_task(),
            theme.paused_task(),
            theme.scope_match(),
            theme.scope_miss(),
            theme.policy_required(),
            theme.policy_denied(),
            theme.tab_active(),
            theme.tab_inactive(),
            theme.status_running(),
            theme.status_idle(),
            theme.status_error(),
            theme.mode_normal(),
            theme.mode_insert(),
            theme.focus_results(),
        ];
        for (i, style) in styles.iter().enumerate() {
            assert!(
                style.fg.is_some(),
                "style method {} should set a foreground color",
                i
            );
        }
    }

    #[test]
    fn focus_input_style_is_bold() {
        let theme = test_theme();
        let style = theme.focus_input();
        assert!(style.fg.is_some());
        assert!(
            style.add_modifier.contains(ratatui::style::Modifier::BOLD),
            "focus_input should be bold"
        );
    }

    #[test]
    fn border_style_focused_vs_unfocused_differ() {
        let theme = test_theme();
        let focused = theme.border_style(true);
        let unfocused = theme.border_style(false);
        assert_ne!(focused.fg, unfocused.fg);
    }

    #[test]
    fn selected_style_has_both_fg_and_bg() {
        let theme = test_theme();
        let style = theme.selected();
        assert!(style.fg.is_some(), "selected should have foreground");
        assert!(style.bg.is_some(), "selected should have background");
    }

    #[test]
    fn style_for_risk_matches_expected() {
        let theme = test_theme();
        assert_eq!(theme.style_for_risk("passive").fg, theme.safe().fg);
        assert_eq!(theme.style_for_risk("safe").fg, theme.safe().fg);
        assert_eq!(theme.style_for_risk("intrusive").fg, theme.danger().fg);
        assert_eq!(
            theme.style_for_risk("administrative").fg,
            theme.danger().fg
        );
        assert_eq!(theme.style_for_risk("unknown").fg, theme.muted().fg);
    }

    #[test]
    fn style_for_policy_outcome_matches_expected() {
        let theme = test_theme();
        assert_eq!(
            theme.style_for_policy_outcome("allow").fg,
            theme.safe().fg
        );
        assert_eq!(
            theme.style_for_policy_outcome("deny").fg,
            theme.policy_denied().fg
        );
        assert_eq!(
            theme.style_for_policy_outcome("warn").fg,
            theme.policy_required().fg
        );
    }

    #[test]
    fn style_for_task_state_matches_expected() {
        let theme = test_theme();
        assert_eq!(
            theme.style_for_task_state("running").fg,
            theme.active_task().fg
        );
        assert_eq!(
            theme.style_for_task_state("paused").fg,
            theme.paused_task().fg
        );
        assert_eq!(
            theme.style_for_task_state("idle").fg,
            theme.muted().fg
        );
    }
}
