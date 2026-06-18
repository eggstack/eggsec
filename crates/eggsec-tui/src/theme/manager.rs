use rustc_hash::FxHashMap;

use super::builtin::{cyber_red_theme, dark_theme, light_theme};
use super::canonical_theme_id;
use super::contrast::{check_contrast, contrast_ratio};
use super::display_theme_name;
use super::palette::{Theme, ThemeMode};

/// Metadata about a registered theme.
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    /// Canonical theme ID.
    pub id: String,
    /// Display name (title-cased from ID).
    pub display_name: String,
    /// Theme mode (Dark/Light).
    pub mode: ThemeMode,
    /// Source of the theme.
    pub source: ThemeSource,
    /// Load status.
    pub status: ThemeLoadStatus,
    /// Pre-adjustment contrast warnings (preserved for FallbackAdjusted themes).
    pub contrast_warnings: Vec<String>,
}

/// Where a theme was loaded from.
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeSource {
    /// Built-in hardcoded theme.
    BuiltIn,
    /// Installed from the packaged archive.
    Packaged,
    /// User-provided custom theme from ~/.config/eggsec/themes/.
    Custom,
}

/// Result status of loading a theme.
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeLoadStatus {
    /// Theme loaded successfully.
    Loaded,
    /// Theme loaded with contrast-safe fallback colors applied.
    FallbackAdjusted,
    /// Theme file exists but couldn't be loaded.
    Invalid(String),
    /// Theme referenced but file not found.
    Missing,
}

pub struct ThemeManager {
    themes: FxHashMap<String, Theme>,
    theme_info: FxHashMap<String, ThemeInfo>,
    current: Theme,
    current_id: String,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut themes = FxHashMap::default();
        let mut theme_info = FxHashMap::default();

        let builtins = [
            ("cyber-red", cyber_red_theme()),
            ("dark", dark_theme()),
            ("light", light_theme()),
        ];

        for (id, theme) in builtins {
            let mode = theme.mode;
            themes.insert(id.to_string(), theme);
            theme_info.insert(
                id.to_string(),
                ThemeInfo {
                    id: id.to_string(),
                    display_name: display_theme_name(id),
                    mode,
                    source: ThemeSource::BuiltIn,
                    status: ThemeLoadStatus::Loaded,
                    contrast_warnings: Vec::new(),
                },
            );
        }

        Self {
            themes,
            theme_info,
            current: cyber_red_theme(),
            current_id: "cyber-red".to_string(),
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
            self.current_id = canonical;
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
        self.register_theme_with_source(theme, ThemeSource::Custom);
    }

    pub fn register_theme_with_source(&mut self, theme: Theme, source: ThemeSource) {
        let mut theme = theme;
        theme.name = canonical_theme_id(&theme.name);
        let name = theme.name.clone();
        if name == "cyber-red" && self.themes.contains_key("cyber-red") {
            return;
        }
        let mode = theme.mode;
        let display_name = display_theme_name(&theme.name);
        self.themes.insert(name.clone(), theme);
        self.theme_info.insert(
            name.clone(),
            ThemeInfo {
                id: name,
                display_name,
                mode,
                source,
                status: ThemeLoadStatus::Loaded,
                contrast_warnings: Vec::new(),
            },
        );
    }

    pub fn mark_theme_invalid(&mut self, id: &str, reason: String) {
        if let Some(info) = self.theme_info.get_mut(id) {
            info.status = ThemeLoadStatus::Invalid(reason);
        }
    }

    pub fn mark_theme_fallback_adjusted(&mut self, id: &str, warnings: Vec<String>) {
        if let Some(info) = self.theme_info.get_mut(id) {
            info.status = ThemeLoadStatus::FallbackAdjusted;
            info.contrast_warnings = warnings;
        }
    }

    /// Register metadata for a theme that failed to load, so it appears
    /// in the Settings metadata even though no `Theme` was created.
    pub fn register_theme_invalid(&mut self, id: &str, source: ThemeSource, reason: String) {
        let canonical = canonical_theme_id(id);
        self.theme_info.insert(
            canonical.clone(),
            ThemeInfo {
                id: canonical,
                display_name: display_theme_name(id),
                mode: crate::theme::ThemeMode::Dark,
                source,
                status: ThemeLoadStatus::Invalid(reason),
                contrast_warnings: Vec::new(),
            },
        );
    }

    pub fn current_name(&self) -> &str {
        &self.current.name
    }

    pub fn current_id(&self) -> &str {
        &self.current_id
    }

    /// Return metadata for a registered theme.
    pub fn get_info(&self, id: &str) -> Option<&ThemeInfo> {
        let canonical = canonical_theme_id(id);
        self.theme_info.get(canonical.as_str())
    }

    /// Return metadata for all registered themes, sorted by ID.
    pub fn theme_info_list(&self) -> Vec<ThemeInfo> {
        let mut infos: Vec<ThemeInfo> = self.theme_info.values().cloned().collect();
        infos.sort_by(|a, b| a.id.cmp(&b.id));
        infos
    }

    /// Return metadata for all registered themes, sorted by display name.
    pub fn get_all_info(&self) -> Vec<&ThemeInfo> {
        let mut info: Vec<&ThemeInfo> = self.theme_info.values().collect();
        info.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        info
    }

    /// Total number of registered themes.
    pub fn theme_count(&self) -> usize {
        self.themes.len()
    }

    /// Number of themes with Loaded status.
    pub fn loaded_count(&self) -> usize {
        self.theme_info
            .values()
            .filter(|i| i.status == ThemeLoadStatus::Loaded)
            .count()
    }

    /// Number of themes with Invalid status.
    pub fn invalid_count(&self) -> usize {
        self.theme_info
            .values()
            .filter(|i| matches!(i.status, ThemeLoadStatus::Invalid(_)))
            .count()
    }

    /// Validate WCAG contrast ratios for a theme, returning warnings for
    /// any RGB color pair below the 4.5:1 minimum. Non-RGB colors (terminal
    /// constants) are skipped since their effective contrast depends on the
    /// terminal palette.
    pub fn validate_contrast(&self, theme_id: &str) -> Vec<String> {
        let canonical = canonical_theme_id(theme_id);
        let Some(theme) = self.themes.get(canonical.as_str()) else {
            return vec![format!("theme '{theme_id}' not found")];
        };
        let mut warnings = Vec::new();

        let pairs = [
            ("text", "background", theme.colors.text, theme.colors.background),
            ("selected_text", "selected", theme.colors.selected_text, theme.colors.selected),
            ("text_dim", "background", theme.colors.text_dim, theme.colors.background),
            ("warning", "background", theme.colors.warning, theme.colors.background),
            ("error", "background", theme.colors.error, theme.colors.background),
            ("success", "background", theme.colors.success, theme.colors.background),
            ("mode_normal", "background", theme.colors.mode_normal, theme.colors.background),
            ("mode_insert", "background", theme.colors.mode_insert, theme.colors.background),
            ("focus_input", "background", theme.colors.focus_input, theme.colors.background),
        ];

        for (fg_name, bg_name, fg, bg) in pairs {
            // Only validate RGB colors; terminal palette colors are not measurable.
            if !matches!(fg, ratatui::style::Color::Rgb(..))
                || !matches!(bg, ratatui::style::Color::Rgb(..))
            {
                continue;
            }
            if !check_contrast(fg, bg, 4.5) {
                let ratio = contrast_ratio(fg, bg);
                warnings.push(format!(
                    "{fg_name}/{bg_name} contrast ratio {:.2}:1 is below 4.5:1 minimum",
                    ratio,
                ));
            }
        }

        warnings
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

    #[test]
    fn builtin_themes_have_theme_info() {
        let manager = ThemeManager::new();
        for id in &["cyber-red", "dark", "light"] {
            let info = manager.get_info(id).unwrap();
            assert_eq!(info.id, *id);
            assert_eq!(info.source, ThemeSource::BuiltIn);
            assert_eq!(info.status, ThemeLoadStatus::Loaded);
            assert!(!info.display_name.is_empty());
        }
    }

    #[test]
    fn get_info_returns_none_for_missing_theme() {
        let manager = ThemeManager::new();
        assert!(manager.get_info("nonexistent").is_none());
    }

    #[test]
    fn get_all_info_sorted_by_display_name() {
        let manager = ThemeManager::new();
        let all = manager.get_all_info();
        assert_eq!(all.len(), 3);
        let names: Vec<&str> = all.iter().map(|i| i.display_name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn theme_count_loaded_count_invalid_count() {
        let mut manager = ThemeManager::new();
        assert_eq!(manager.theme_count(), 3);
        assert_eq!(manager.loaded_count(), 3);
        assert_eq!(manager.invalid_count(), 0);

        let custom = crate::theme::palette::Theme {
            mode: crate::theme::ThemeMode::Dark,
            name: "custom".to_string(),
            colors: crate::theme::builtin::dark_theme().colors,
        };
        manager.register_theme(custom);
        assert_eq!(manager.theme_count(), 4);
        assert_eq!(manager.loaded_count(), 4);

        manager.mark_theme_invalid("custom", "bad toml".to_string());
        assert_eq!(manager.loaded_count(), 3);
        assert_eq!(manager.invalid_count(), 1);
    }

    #[test]
    fn register_theme_with_source_records_info() {
        let mut manager = ThemeManager::new();
        let custom = crate::theme::palette::Theme {
            mode: crate::theme::ThemeMode::Light,
            name: "my-theme".to_string(),
            colors: crate::theme::builtin::light_theme().colors,
        };
        manager.register_theme_with_source(custom, ThemeSource::Packaged);
        let info = manager.get_info("my-theme").unwrap();
        assert_eq!(info.source, ThemeSource::Packaged);
        assert_eq!(info.mode, ThemeMode::Light);
        assert_eq!(info.display_name, "My Theme");
    }

    #[test]
    fn mark_theme_fallback_adjusted() {
        let mut manager = ThemeManager::new();
        let warnings = vec!["text/background contrast ratio 2.5:1 is below 4.5:1 minimum".to_string()];
        manager.mark_theme_fallback_adjusted("dark", warnings.clone());
        let info = manager.get_info("dark").unwrap();
        assert_eq!(info.status, ThemeLoadStatus::FallbackAdjusted);
        assert_eq!(info.contrast_warnings, warnings);
    }

    #[test]
    fn validate_contrast_builtins_no_warnings() {
        let manager = ThemeManager::new();
        for id in &["cyber-red", "dark", "light"] {
            let warnings = manager.validate_contrast(id);
            assert!(
                warnings.is_empty(),
                "unexpected warnings for {id}: {warnings:?}"
            );
        }
    }

    #[test]
    fn validate_contrast_missing_theme() {
        let manager = ThemeManager::new();
        let warnings = manager.validate_contrast("nonexistent");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("not found"));
    }
}
