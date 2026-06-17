use rustc_hash::FxHashMap;

use super::builtin::{cyber_red_theme, dark_theme, light_theme};
use super::canonical_theme_id;
use super::palette::Theme;

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
        let canonical = canonical_theme_id(name);
        self.themes.get(canonical.as_str())
    }

    pub fn current(&self) -> &Theme {
        &self.current
    }

    pub fn set_theme(&mut self, name: &str) -> bool {
        let canonical = canonical_theme_id(name);
        if let Some(theme) = self.themes.get(canonical.as_str()) {
            self.current = theme.clone();
            true
        } else {
            tracing::debug!(name = %canonical, "theme not found in manager");
            false
        }
    }

    /// Cycle to the next registered theme in alphabetical order.
    ///
    /// Wraps around at the end so users can reach any theme (including custom
    /// packaged themes) without needing to re-open the Settings selector. The
    /// previous implementation only rotated the three built-in themes, which
    /// trapped users on a custom theme who hit Ctrl+T by accident.
    pub fn toggle(&mut self) {
        let themes = self.list_theme_ids_owned();
        if themes.is_empty() {
            return;
        }
        let current_name = self.current.name.clone();
        let next = match themes.iter().position(|t| t == &current_name) {
            Some(idx) => themes[(idx + 1) % themes.len()].clone(),
            None => themes[0].clone(),
        };
        if !self.set_theme(&next) {
            tracing::debug!(theme = %next, "theme cycling failed to set theme");
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

    /// Return owned theme IDs (used where the caller needs to retain
    /// ownership across mutable borrows of `self`).
    pub fn list_theme_ids_owned(&self) -> Vec<String> {
        let mut themes: Vec<String> = self.themes.keys().cloned().collect();
        themes.sort();
        themes
    }

    pub fn register_theme(&mut self, theme: Theme) {
        let mut theme = theme;
        theme.name = canonical_theme_id(&theme.name);
        let name = theme.name.clone();
        if name == "cyber-red" && self.themes.contains_key("cyber-red") {
            return;
        }
        self.themes.insert(name, theme);
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
        assert_eq!(manager.current().mode, crate::theme::ThemeMode::Dark);
    }

    #[test]
    fn theme_manager_set_theme_succeeds() {
        let mut manager = ThemeManager::new();
        assert!(manager.set_theme("light"));
        assert_eq!(manager.current().mode, crate::theme::ThemeMode::Light);
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
        let first = manager.current().name.clone();
        manager.toggle();
        let second = manager.current().name.clone();
        assert_ne!(first, second);
        manager.toggle();
        // 3 themes - one full cycle returns to start
        assert_eq!(manager.current().name, "cyber-red");
    }

    #[test]
    fn theme_manager_toggle_cycles_through_custom_themes() {
        let mut manager = ThemeManager::new();
        let custom = crate::theme::palette::Theme {
            mode: crate::theme::ThemeMode::Dark,
            name: "custom".to_string(),
            colors: crate::theme::builtin::dark_theme().colors,
        };
        manager.register_theme(custom);
        assert!(manager.set_theme("custom"));
        manager.toggle();
        // Custom themes must be reachable via Ctrl+T, not lost.
        let after = manager.current().name.clone();
        assert_ne!(after, "custom", "toggle should advance to next theme");
    }

    #[test]
    fn theme_manager_toggle_wraps_around_full_set() {
        let mut manager = ThemeManager::new();
        let total = manager.list_themes().len();
        let first = manager.current().name.clone();
        for _ in 0..total {
            manager.toggle();
        }
        assert_eq!(manager.current().name, first);
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
        assert_eq!(manager.current().mode, crate::theme::ThemeMode::Light);
        assert_eq!(manager.current().name, "light");
    }

    #[test]
    fn register_theme_inserts_new_theme() {
        let mut manager = ThemeManager::new();
        let custom = crate::theme::palette::Theme {
            mode: crate::theme::ThemeMode::Dark,
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
            mode: crate::theme::ThemeMode::Dark,
            name: "Cyber Red".to_string(),
            colors: crate::theme::builtin::dark_theme().colors,
        };
        manager.register_theme(replacement);
        assert_eq!(manager.current().name, "cyber-red");
        let current_colors = &manager.current().colors;
        assert_eq!(current_colors.primary, ratatui::style::Color::Red);
        assert_eq!(manager.list_themes().len(), 3);
        assert!(manager.list_themes().contains(&"cyber-red"));
        assert!(!manager.list_themes().contains(&"Cyber Red"));
        assert_eq!(
            manager
                .get_theme("Cyber Red")
                .map(|theme| theme.name.as_str()),
            Some("cyber-red")
        );
    }

    #[test]
    fn current_name_returns_theme_name() {
        let manager = ThemeManager::new();
        assert_eq!(manager.current_name(), "cyber-red");
    }

    #[test]
    fn semantic_colors_present_and_distinct_on_builtins() {
        // Phase 10: verify the new semantic fields are initialized on all 3 builtins + default (cyber-red)
        let cyber = crate::theme::builtin::cyber_red_theme();
        assert_ne!(cyber.colors.safe, cyber.colors.danger);
        assert_ne!(cyber.colors.active_task, cyber.colors.paused_task);
        assert_ne!(cyber.colors.scope_match, cyber.colors.scope_miss);
        assert_ne!(cyber.colors.policy_required, cyber.colors.policy_denied);
        assert_ne!(cyber.colors.safe, cyber.colors.muted);

        let dark = crate::theme::builtin::dark_theme();
        assert_ne!(dark.colors.safe, dark.colors.danger);
        assert_ne!(dark.colors.active_task, dark.colors.paused_task);

        let light = crate::theme::builtin::light_theme();
        assert_ne!(light.colors.safe, light.colors.danger);
        assert_ne!(light.colors.active_task, light.colors.paused_task);

        // default() must be cyber-red and have the fields set
        let def = crate::theme::palette::Theme::default();
        assert_eq!(def.name, "cyber-red");
        assert_ne!(def.colors.safe, def.colors.danger);
        // sanity that helpers resolve without panic and return fg
        let s = def.safe();
        assert!(s.fg.is_some());
        let p = def.style_for_policy_outcome("confirm");
        assert!(p.fg.is_some());
        let t = def.style_for_task_state("running");
        assert!(t.fg.is_some());
        let r = def.style_for_risk("intrusive");
        assert!(r.fg.is_some());
    }
}
