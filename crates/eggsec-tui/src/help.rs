use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::utils::fuzzy::fuzzy_score;
use crate::Tab;

/// Context for help content rendering. Currently only `Normal` is used;
/// reserved for future use (e.g. context-sensitive help for different modes).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum HelpContext {
    Normal,
}

#[derive(Debug, Clone)]
pub struct HelpSection {
    pub title: String,
    pub content: String,
    pub commands: Vec<HelpCommand>,
}

#[derive(Debug, Clone)]
pub struct HelpCommand {
    pub key: String,
    pub description: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct CommandPalette {
    pub visible: bool,
    pub query: String,
    pub results: Arc<Vec<CommandPaletteResult>>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub popup_width: u16,
    pub popup_height: u16,
    pub last_content_height: u16,
}

impl CommandPalette {
    pub fn new(results: Arc<Vec<CommandPaletteResult>>) -> Self {
        Self {
            visible: false,
            query: String::new(),
            results,
            selected_index: 0,
            scroll_offset: 0,
            popup_width: 60,
            popup_height: 20,
            last_content_height: 15,
        }
    }

    pub fn visible_results_height(&self) -> usize {
        let computed = (self.last_content_height as usize).saturating_sub(3);
        computed.min(self.results.len()).max(1)
    }

    pub fn update_content_height(&mut self, content_height: u16) {
        self.last_content_height = content_height;
    }

    pub fn max_scroll_offset(&self) -> usize {
        self.results
            .len()
            .saturating_sub(self.visible_results_height())
    }

    pub fn adjust_scroll_for_selection(&mut self) {
        let vis = self.visible_results_height();
        let max_scroll = self.max_scroll_offset();
        if self.selected_index >= self.scroll_offset + vis {
            self.scroll_offset = self.selected_index.saturating_sub(vis).min(max_scroll);
        }
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index.min(max_scroll);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_palette_with_results(count: usize) -> CommandPalette {
        let results: Vec<CommandPaletteResult> = (0..count)
            .map(|i| CommandPaletteResult {
                command: format!("cmd_{}", i),
                description: format!("Description {}", i),
                category: "Test".to_string(),
                shortcut: None,
            })
            .collect();
        CommandPalette::new(Arc::new(results))
    }

    #[test]
    fn test_visible_results_height_with_reduced_content() {
        let mut palette = make_palette_with_results(20);
        palette.last_content_height = 10;
        assert_eq!(palette.visible_results_height(), 7);

        palette.last_content_height = 5;
        assert_eq!(palette.visible_results_height(), 2);

        palette.last_content_height = 0;
        assert_eq!(palette.visible_results_height(), 1);
    }

    #[test]
    fn test_scroll_offset_with_small_visible_area() {
        let mut palette = make_palette_with_results(20);
        palette.last_content_height = 8;
        palette.selected_index = 3;
        palette.scroll_offset = 0;
        palette.adjust_scroll_for_selection();

        assert!(
            palette.selected_index >= palette.scroll_offset,
            "Selected index should be >= scroll offset"
        );
        assert!(
            palette.selected_index < palette.scroll_offset + palette.visible_results_height(),
            "Selected index should be visible, but sel={} off={} vis={}",
            palette.selected_index,
            palette.scroll_offset,
            palette.visible_results_height()
        );
    }

    #[test]
    fn test_scroll_offset_clamped_when_selection_near_end() {
        let mut palette = make_palette_with_results(20);
        palette.last_content_height = 8;
        palette.selected_index = 18;
        palette.scroll_offset = 10;
        palette.adjust_scroll_for_selection();

        let max_scroll = palette.max_scroll_offset();
        assert!(
            palette.scroll_offset <= max_scroll,
            "Scroll offset {} should not exceed max {}",
            palette.scroll_offset,
            max_scroll
        );
    }

    #[test]
    fn test_max_scroll_offset_decreases_with_smaller_height() {
        let mut palette = make_palette_with_results(30);

        palette.last_content_height = 15;
        let max_scroll_large = palette.max_scroll_offset();

        palette.last_content_height = 8;
        let max_scroll_small = palette.max_scroll_offset();

        assert!(
            max_scroll_small > max_scroll_large,
            "Smaller visible area should allow more scroll: small={}, large={}",
            max_scroll_small,
            max_scroll_large
        );
    }
}

#[derive(Debug, Clone)]
pub struct CommandPaletteResult {
    pub command: String,
    pub description: String,
    pub category: String,
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HelpContent {
    pub sections: FxHashMap<Tab, HelpSection>,
    pub global_commands: Vec<HelpCommand>,
    pub command_palette_entries: Arc<Vec<CommandPaletteResult>>,
}

pub struct HelpManager {
    pub content: HelpContent,
}

impl HelpManager {
    pub fn new() -> Self {
        Self {
            content: HelpContent::default(),
        }
    }

    pub fn get_help_for_tab(&self, tab: Tab) -> Option<&HelpSection> {
        self.content.sections.get(&tab)
    }

    pub fn get_global_commands(&self) -> &Vec<HelpCommand> {
        &self.content.global_commands
    }

    pub fn get_command_palette_entries(&self) -> &Arc<Vec<CommandPaletteResult>> {
        &self.content.command_palette_entries
    }

    pub fn search_commands(&self, query: &str) -> Vec<CommandPaletteResult> {
        if query.is_empty() {
            return self
                .content
                .command_palette_entries
                .iter()
                .cloned()
                .collect();
        }
        let query_lower = query.to_lowercase();
        let mut scored: Vec<(u32, CommandPaletteResult)> = self
            .content
            .command_palette_entries
            .iter()
            .filter_map(|cmd| {
                let command_lower = cmd.command.to_lowercase();
                let description_lower = cmd.description.to_lowercase();
                let category_lower = cmd.category.to_lowercase();
                let score = fuzzy_score(&command_lower, &query_lower)
                    .max(fuzzy_score(&description_lower, &query_lower))
                    .max(fuzzy_score(&category_lower, &query_lower));
                if score > 0 {
                    Some((score, cmd.clone()))
                } else {
                    None
                }
            })
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.0));
        scored.into_iter().map(|(_, cmd)| cmd).collect()
    }
}

impl Default for HelpContent {
    fn default() -> Self {
        let static_data = crate::app::help_config::get_static_help_data();

        Self {
            sections: static_data.sections,
            global_commands: static_data.global_commands,
            command_palette_entries: static_data.command_palette_entries,
        }
    }
}
