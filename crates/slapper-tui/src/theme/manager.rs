use rustc_hash::FxHashMap;

use super::builtin::{dark_theme, light_theme};
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

    pub(crate) fn set_current_for_legacy_sync(&mut self, theme: &Theme) {
        self.current = theme.clone();
    }

    pub fn list_themes(&self) -> Vec<&str> {
        let mut themes: Vec<&str> = self.themes.keys().map(|s| s.as_str()).collect();
        themes.sort();
        themes
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
        assert_eq!(manager.current().mode, ThemeMode::Dark);
        manager.toggle();
        assert_eq!(manager.current().mode, ThemeMode::Light);
        manager.toggle();
        assert_eq!(manager.current().mode, ThemeMode::Dark);
    }

    #[test]
    fn builtin_theme_names_are_owned_strings() {
        let dark = crate::theme::builtin::dark_theme();
        let light = crate::theme::builtin::light_theme();
        assert_eq!(dark.name, "dark");
        assert_eq!(light.name, "light");
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
}
