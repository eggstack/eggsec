use ratatui::style::{Color, Modifier, Style};
use rustc_hash::FxHashMap;
use std::cell::RefCell;

thread_local! {
    pub static THEME_MANAGER: RefCell<ThemeManager> = RefCell::new(ThemeManager::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    pub name: &'static str,
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
        dark_theme()
    }
}

pub fn dark_theme() -> Theme {
    Theme {
        mode: ThemeMode::Dark,
        name: "dark",
        colors: ThemeColors {
            primary: Color::Cyan,
            secondary: Color::Blue,
            accent: Color::Yellow,
            background: Color::Black,
            foreground: Color::White,
            surface: Color::DarkGray,
            border: Color::DarkGray,
            border_focused: Color::Yellow,
            text: Color::White,
            text_dim: Color::DarkGray,
            text_bright: Color::White,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            selected: Color::Yellow,
            selected_text: Color::Black,
            highlight: Color::Magenta,
            mode_normal: Color::Green,
            mode_insert: Color::Yellow,
            tab_active: Color::Cyan,
            tab_inactive: Color::DarkGray,
            status_running: Color::Green,
            status_idle: Color::DarkGray,
            status_error: Color::Red,
            focus_normal: Color::Indexed(220),
            focus_input: Color::Indexed(51),
            focus_results: Color::Indexed(82),
        },
    }
}

pub fn light_theme() -> Theme {
    Theme {
        mode: ThemeMode::Light,
        name: "light",
        colors: ThemeColors {
            primary: Color::Blue,
            secondary: Color::Cyan,
            accent: Color::Magenta,
            background: Color::White,
            foreground: Color::Black,
            surface: Color::LightGreen,
            border: Color::LightBlue,
            border_focused: Color::Blue,
            text: Color::Black,
            text_dim: Color::DarkGray,
            text_bright: Color::Black,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
            selected: Color::Blue,
            selected_text: Color::White,
            highlight: Color::Magenta,
            mode_normal: Color::Green,
            mode_insert: Color::Red,
            tab_active: Color::Blue,
            tab_inactive: Color::LightBlue,
            status_running: Color::Green,
            status_idle: Color::DarkGray,
            status_error: Color::Red,
            focus_normal: Color::Indexed(21),
            focus_input: Color::Indexed(196),
            focus_results: Color::Indexed(28),
        },
    }
}

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
                .fg(Color::Black)
                .bg(self.colors.mode_normal)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Black)
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

pub struct ThemeManager {
    themes: FxHashMap<String, Theme>,
    current: Theme,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut themes = FxHashMap::default();
        themes.insert("dark".to_string(), dark_theme());
        themes.insert("light".to_string(), light_theme());

        Self {
            themes,
            current: dark_theme(),
        }
    }

    pub fn get_theme(&self, name: &str) -> Option<&Theme> {
        self.themes.get(name)
    }

    pub fn current(&self) -> &Theme {
        &self.current
    }

    pub fn set_theme(&mut self, name: &str) -> bool {
        if let Some(theme) = self.themes.get(name) {
            self.current = theme.clone();
            true
        } else {
            false
        }
    }

    pub fn toggle(&mut self) {
        match self.current.mode {
            ThemeMode::Dark => {
                if let Some(light) = self.themes.get("light") {
                    self.current = light.clone();
                }
            }
            ThemeMode::Light => {
                if let Some(dark) = self.themes.get("dark") {
                    self.current = dark.clone();
                }
            }
        }
    }

    pub fn list_themes(&self) -> Vec<&'static str> {
        vec!["dark", "light"]
    }

    pub fn set_accent_color(&mut self, color_name: &str) {
        let color = match color_name.to_lowercase().as_str() {
            "cyan" => Color::Cyan,
            "blue" => Color::Blue,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "red" => Color::Red,
            "magenta" => Color::Magenta,
            "white" => Color::White,
            "black" => Color::Black,
            _ => return,
        };
        self.current.colors.accent = color;
        self.current.colors.border_focused = color;
        self.current.colors.tab_active = color;
        self.current.colors.selected = color;
    }
}

#[macro_export]
macro_rules! theme {
    () => {
        &$crate::tui::theme::THEME_MANAGER.with(|tm| tm.borrow().current())
    };
}

#[macro_export]
macro_rules! tc {
    ($field:ident) => {
        $crate::tui::theme::THEME_MANAGER.with(|tm| tm.borrow().current().colors.$field)
    };
}
