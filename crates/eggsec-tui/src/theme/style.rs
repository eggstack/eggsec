use ratatui::style::{Modifier, Style};

use super::palette::Theme;

// NOTE: These helper methods are currently unused because the TUI still uses
// the thread-local `tc!` macro or constructs styles inline. They are retained
// for future adoption by refactoring tab renderers to pass `&Theme` explicitly,
// which aligns with the architectural guidance in AGENTS.override.md.  When
// adopting these methods, double-check that `style_for_mode` uses the correct
// foreground/background fields—the existing code used `selected_text` for the
// mode badge, whereas the status bar renders with `mode_normal`/`mode_insert`
// backgrounds.
#[allow(dead_code)]
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
}
