use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    pub name: String,
    pub colors: ThemeColors,
}

#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub background: Color,
    pub foreground: Color,
    pub surface: Color,
    pub border: Color,
    pub border_focused: Color,
    pub text: Color,
    pub text_dim: Color,
    pub text_bright: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub selected: Color,
    pub selected_text: Color,
    pub highlight: Color,
    pub mode_normal: Color,
    pub mode_insert: Color,
    pub tab_active: Color,
    pub tab_inactive: Color,
    pub status_running: Color,
    pub status_idle: Color,
    pub status_error: Color,
    pub focus_normal: Color,
    pub focus_input: Color,
    pub focus_results: Color,
}

impl Default for Theme {
    fn default() -> Self {
        super::builtin::dark_theme()
    }
}
