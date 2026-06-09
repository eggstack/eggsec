use rustc_hash::FxHashMap;

use super::builtin::{cyber_red_theme, dark_theme, light_theme};
use super::palette::{Theme, ThemeMode};

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
        themes.insert("cyber-red".to_string(), cyber_red_theme());
        themes.insert("dark".to_string(), dark_theme());
        themes.insert("light".to_string(), light_theme());

        Self {
            themes,
            current: cyber_red_theme(),
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
        let mut names: Vec<&str> = self.themes.keys().map(|s| s.as_str()).collect();
        names.sort();
        if names.is_empty() {
            return;
        }
        let current_idx = names
            .iter()
            .position(|&n| n == self.current.name)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % names.len();
        if let Some(next) = self.themes.get(names[next_idx]) {
            self.current = next.clone();
        }
    }

    pub(crate) fn set_current_for_legacy_sync(&mut self, theme: &Theme) {
        self.current = theme.clone();
    }

    pub fn list_themes(&self) -> Vec<&str> {
        let mut themes: Vec<&str> = self.themes.keys().map(|s| s.as_str()).collect();
        themes.sort();
        themes
    }

    pub fn register_theme(&mut self, theme: Theme) {
        let name = theme.name.clone();
        if name == "cyber-red" && self.themes.contains_key("cyber-red") {
            return;
        }
        self.themes.insert(name, theme);
    }

    pub fn register_theme_if_absent(&mut self, theme: Theme) -> bool {
        let name = theme.name.clone();
        if self.themes.contains_key(&name) {
            false
        } else {
            self.themes.insert(name, theme);
            true
        }
    }

    pub fn set_current_by_name(&mut self, name: &str) -> bool {
        if let Some(theme) = self.themes.get(name) {
            self.current = theme.clone();
            true
        } else {
            false
        }
    }

    pub fn current_name(&self) -> &str {
        &self.current.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_manager_new_contains_dark_and_light() {
        let manager = ThemeManager::new();
        let themes = manager.list_themes();
        assert!(themes.contains(&"dark"));
        assert!(themes.contains(&"light"));
        assert!(themes.contains(&"cyber-red"));
    }

    #[test]
    fn theme_manager_new_defaults_to_cyber_red() {
        let manager = ThemeManager::new();
        assert_eq!(manager.current().name, "cyber-red");
        assert_eq!(manager.current().mode, ThemeMode::Dark);
    }

    #[test]
    fn theme_manager_set_theme_succeeds() {
        let mut manager = ThemeManager::new();
        assert!(manager.set_theme("light"));
        assert_eq!(manager.current().mode, ThemeMode::Light);
    }

    #[test]
    fn theme_manager_set_theme_missing_returns_false() {
        let mut manager = ThemeManager::new();
        let initial_mode = manager.current().mode;
        assert!(!manager.set_theme("nonexistent"));
        assert_eq!(manager.current().mode, initial_mode);
    }

    #[test]
    fn theme_manager_toggle_switches() {
        let mut manager = ThemeManager::new();
        assert_eq!(manager.current().name, "cyber-red");
        manager.toggle();
        assert_eq!(manager.current().name, "dark");
        manager.toggle();
        assert_eq!(manager.current().name, "light");
        manager.toggle();
        assert_eq!(manager.current().name, "cyber-red");
    }

    #[test]
    fn builtin_theme_names_are_owned_strings() {
        let dark = crate::theme::builtin::dark_theme();
        let light = crate::theme::builtin::light_theme();
        let cyber_red = crate::theme::builtin::cyber_red_theme();
        assert_eq!(dark.name, "dark");
        assert_eq!(light.name, "light");
        assert_eq!(cyber_red.name, "cyber-red");
    }

    #[test]
    fn set_theme_preserves_name_on_success() {
        let mut manager = ThemeManager::new();
        manager.set_theme("light");
        assert_eq!(manager.current().name, "light");
    }

    #[test]
    fn set_theme_preserves_name_on_failure() {
        let mut manager = ThemeManager::new();
        let initial_name = manager.current().name.clone();
        manager.set_theme("nonexistent");
        assert_eq!(manager.current().name, initial_name);
    }

    #[test]
    fn legacy_sync_updates_through_setter() {
        let mut manager = ThemeManager::new();
        let light = crate::theme::builtin::light_theme();
        manager.set_current_for_legacy_sync(&light);
        assert_eq!(manager.current().mode, ThemeMode::Light);
        assert_eq!(manager.current().name, "light");
    }

    #[test]
    fn register_theme_inserts_new_theme() {
        let mut manager = ThemeManager::new();
        let custom = crate::theme::palette::Theme {
            mode: ThemeMode::Dark,
            name: "custom".to_string(),
            colors: crate::theme::builtin::dark_theme().colors,
        };
        manager.register_theme(custom);
        assert!(manager.list_themes().contains(&"custom"));
    }

    #[test]
    fn register_theme_does_not_overwrite_cyber_red() {
        let mut manager = ThemeManager::new();
        let replacement = crate::theme::palette::Theme {
            mode: ThemeMode::Dark,
            name: "cyber-red".to_string(),
            colors: crate::theme::builtin::dark_theme().colors,
        };
        manager.register_theme(replacement);
        assert_eq!(manager.current().name, "cyber-red");
        let current_colors = &manager.current().colors;
        assert_eq!(current_colors.primary, ratatui::style::Color::Red);
    }

    #[test]
    fn register_theme_if_absent_returns_true_when_new() {
        let mut manager = ThemeManager::new();
        let custom = crate::theme::palette::Theme {
            mode: ThemeMode::Dark,
            name: "neon".to_string(),
            colors: crate::theme::builtin::dark_theme().colors,
        };
        assert!(manager.register_theme_if_absent(custom));
    }

    #[test]
    fn register_theme_if_absent_returns_false_when_exists() {
        let mut manager = ThemeManager::new();
        let existing = crate::theme::palette::Theme {
            mode: ThemeMode::Dark,
            name: "dark".to_string(),
            colors: crate::theme::builtin::dark_theme().colors,
        };
        assert!(!manager.register_theme_if_absent(existing));
    }

    #[test]
    fn set_current_by_name_succeeds() {
        let mut manager = ThemeManager::new();
        assert!(manager.set_current_by_name("light"));
        assert_eq!(manager.current_name(), "light");
    }

    #[test]
    fn set_current_by_name_fails_for_unknown() {
        let mut manager = ThemeManager::new();
        assert!(!manager.set_current_by_name("unknown"));
        assert_eq!(manager.current_name(), "cyber-red");
    }

    #[test]
    fn current_name_returns_theme_name() {
        let manager = ThemeManager::new();
        assert_eq!(manager.current_name(), "cyber-red");
    }
}
