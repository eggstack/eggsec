
use std::collections::HashMap;

use crate::tui::Tab;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpContext {
    Normal,
    Configuration,
    Scanning,
    Fuzzing,
    Advanced,
    CommandDiscovery,
}

#[derive(Debug, Clone)]
pub struct HelpOverlay {
    pub visible: bool,
    pub context: HelpContext,
    pub content: String,
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
    pub results: Vec<CommandPaletteResult>,
    pub selected_index: usize,
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
    pub sections: HashMap<Tab, HelpSection>,
    pub global_commands: Vec<HelpCommand>,
    pub command_palette_entries: Vec<CommandPaletteResult>,
}

pub struct HelpManager {
    pub content: HelpContent,
    pub current_context: HelpContext,
}

impl HelpManager {
    pub fn new() -> Self {
        Self {
            content: HelpContent::default(),
            current_context: HelpContext::Normal,
        }
    }

    pub fn get_help_for_tab(&self, tab: Tab) -> Option<&HelpSection> {
        self.content.sections.get(&tab)
    }

    pub fn get_global_commands(&self) -> &Vec<HelpCommand> {
        &self.content.global_commands
    }

    pub fn get_command_palette_entries(&self) -> &Vec<CommandPaletteResult> {
        &self.content.command_palette_entries
    }

    pub fn search_commands(&self, query: &str) -> Vec<CommandPaletteResult> {
        let query_lower = query.to_lowercase();
        self.content
            .command_palette_entries
            .iter()
            .filter(|cmd| {
                cmd.command.to_lowercase().contains(&query_lower)
                    || cmd.description.to_lowercase().contains(&query_lower)
                    || cmd.category.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }
}

impl Default for HelpContent {
    fn default() -> Self {
        Self {
            sections: HashMap::new(),
            global_commands: vec![
                HelpCommand {
                    key: "Ctrl+C".to_string(),
                    description: "Stop current operation".to_string(),
                    category: "Control".to_string(),
                },
                HelpCommand {
                    key: "Ctrl+Q".to_string(),
                    description: "Quit application".to_string(),
                    category: "Control".to_string(),
                },
                HelpCommand {
                    key: "Ctrl+S".to_string(),
                    description: "Save settings".to_string(),
                    category: "Settings".to_string(),
                },
                HelpCommand {
                    key: "Ctrl+R".to_string(),
                    description: "Reset current tab".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Ctrl+/".to_string(),
                    description: "Toggle help".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Ctrl+P".to_string(),
                    description: "Open command palette".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Ctrl+F".to_string(),
                    description: "Toggle search".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Ctrl+U/D".to_string(),
                    description: "Page up/down".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Ctrl+G".to_string(),
                    description: "Go to top/bottom".to_string(),
                    category: "Navigation".to_string(),
                },
            ],
            command_palette_entries: vec![
                CommandPaletteResult {
                    command: "quit".to_string(),
                    description: "Exit the application".to_string(),
                    category: "System".to_string(),
                    shortcut: Some("Ctrl+Q".to_string()),
                },
                CommandPaletteResult {
                    command: "stop".to_string(),
                    description: "Stop current operation".to_string(),
                    category: "System".to_string(),
                    shortcut: Some("Ctrl+C".to_string()),
                },
                CommandPaletteResult {
                    command: "reset".to_string(),
                    description: "Reset current tab".to_string(),
                    category: "Navigation".to_string(),
                    shortcut: Some("Ctrl+R".to_string()),
                },
                CommandPaletteResult {
                    command: "save".to_string(),
                    description: "Save settings".to_string(),
                    category: "Settings".to_string(),
                    shortcut: Some("Ctrl+S".to_string()),
                },
                CommandPaletteResult {
                    command: "help".to_string(),
                    description: "Toggle help overlay".to_string(),
                    category: "Navigation".to_string(),
                    shortcut: Some("Ctrl+/".to_string()),
                },
                CommandPaletteResult {
                    command: "search".to_string(),
                    description: "Toggle search".to_string(),
                    category: "Navigation".to_string(),
                    shortcut: Some("Ctrl+F".to_string()),
                },
                CommandPaletteResult {
                    command: "palette".to_string(),
                    description: "Open command palette".to_string(),
                    category: "Navigation".to_string(),
                    shortcut: Some("Ctrl+P".to_string()),
                },
                CommandPaletteResult {
                    command: "export".to_string(),
                    description: "Export results".to_string(),
                    category: "Data".to_string(),
                    shortcut: None,
                },
                CommandPaletteResult {
                    command: "history".to_string(),
                    description: "View scan history".to_string(),
                    category: "Data".to_string(),
                    shortcut: None,
                },
                CommandPaletteResult {
                    command: "settings".to_string(),
                    description: "Configure settings".to_string(),
                    category: "Settings".to_string(),
                    shortcut: None,
                },
                CommandPaletteResult {
                    command: "dashboard".to_string(),
                    description: "Go to Dashboard".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("0".to_string()),
                },
                CommandPaletteResult {
                    command: "recon".to_string(),
                    description: "Go to Reconnaissance".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("1".to_string()),
                },
                CommandPaletteResult {
                    command: "load".to_string(),
                    description: "Go to Load Testing".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("2".to_string()),
                },
                CommandPaletteResult {
                    command: "ports".to_string(),
                    description: "Go to Port Scanning".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("3".to_string()),
                },
                CommandPaletteResult {
                    command: "endpoints".to_string(),
                    description: "Go to Endpoint Discovery".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("4".to_string()),
                },
                CommandPaletteResult {
                    command: "fingerprint".to_string(),
                    description: "Go to Service Fingerprinting".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("5".to_string()),
                },
                CommandPaletteResult {
                    command: "fuzz".to_string(),
                    description: "Go to Fuzzing".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("6".to_string()),
                },
                CommandPaletteResult {
                    command: "waf".to_string(),
                    description: "Go to WAF Detection".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("7".to_string()),
                },
                CommandPaletteResult {
                    command: "wafstress".to_string(),
                    description: "Go to WAF Stress Testing".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("8".to_string()),
                },
                CommandPaletteResult {
                    command: "pipeline".to_string(),
                    description: "Go to Pipeline Scan".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("9".to_string()),
                },
                CommandPaletteResult {
                    command: "resume".to_string(),
                    description: "Go to Session Resume".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: None,
                },
                CommandPaletteResult {
                    command: "next-tab".to_string(),
                    description: "Go to next tab".to_string(),
                    category: "Navigation".to_string(),
                    shortcut: Some("l".to_string()),
                },
                CommandPaletteResult {
                    command: "prev-tab".to_string(),
                    description: "Go to previous tab".to_string(),
                    category: "Navigation".to_string(),
                    shortcut: Some("h".to_string()),
                },
                CommandPaletteResult {
                    command: "page-up".to_string(),
                    description: "Page up".to_string(),
                    category: "Navigation".to_string(),
                    shortcut: Some("Ctrl+U".to_string()),
                },
                CommandPaletteResult {
                    command: "page-down".to_string(),
                    description: "Page down".to_string(),
                    category: "Navigation".to_string(),
                    shortcut: Some("Ctrl+D".to_string()),
                },
                CommandPaletteResult {
                    command: "http-options".to_string(),
                    description: "View HTTP options".to_string(),
                    category: "Settings".to_string(),
                    shortcut: None,
                },
            ],
        }
    }
}
