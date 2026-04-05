use std::collections::HashMap;
use std::sync::Arc;

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
    pub results: Arc<Vec<CommandPaletteResult>>,
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
    pub command_palette_entries: Arc<Vec<CommandPaletteResult>>,
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

    pub fn get_command_palette_entries(&self) -> &Arc<Vec<CommandPaletteResult>> {
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
        use crate::tui::tabs::Tab;

        let mut sections = HashMap::new();

        sections.insert(
            Tab::Recon,
            HelpSection {
                title: "Reconnaissance".to_string(),
                content: "Gather target information: subdomains, DNS records, technologies, SSL certificates, wayback data, cloud assets, threat intelligence, and CVEs.".to_string(),
                commands: vec![
                    HelpCommand { key: "Enter".to_string(), description: "Start scan".to_string(), category: "Action".to_string() },
                    HelpCommand { key: "e".to_string(), description: "Export results".to_string(), category: "Export".to_string() },
                    HelpCommand { key: "Tab".to_string(), description: "Toggle target input".to_string(), category: "Navigation".to_string() },
                ],
            },
        );

        sections.insert(
            Tab::Load,
            HelpSection {
                title: "Load Testing".to_string(),
                content: "HTTP load testing with configurable concurrency, duration, and request patterns.".to_string(),
                commands: vec![
                    HelpCommand { key: "Enter".to_string(), description: "Start load test".to_string(), category: "Action".to_string() },
                    HelpCommand { key: "Space".to_string(), description: "Pause/Resume".to_string(), category: "Control".to_string() },
                ],
            },
        );

        sections.insert(
            Tab::ScanPorts,
            HelpSection {
                title: "Port Scanning".to_string(),
                content: "TCP/UDP port scanning with various scan techniques. Use --spoof for decoy scanning.".to_string(),
                commands: vec![
                    HelpCommand { key: "Enter".to_string(), description: "Start scan".to_string(), category: "Action".to_string() },
                    HelpCommand { key: "1-5".to_string(), description: "Select scan type".to_string(), category: "Selection".to_string() },
                ],
            },
        );

        sections.insert(
            Tab::ScanEndpoints,
            HelpSection {
                title: "Endpoint Discovery".to_string(),
                content:
                    "Directory and file discovery using wordlists. Supports recursive scanning."
                        .to_string(),
                commands: vec![
                    HelpCommand {
                        key: "Enter".to_string(),
                        description: "Start scan".to_string(),
                        category: "Action".to_string(),
                    },
                    HelpCommand {
                        key: "w".to_string(),
                        description: "Load wordlist".to_string(),
                        category: "Config".to_string(),
                    },
                ],
            },
        );

        sections.insert(
            Tab::Fingerprint,
            HelpSection {
                title: "Service Fingerprinting".to_string(),
                content: "Identify services and technologies running on target ports.".to_string(),
                commands: vec![HelpCommand {
                    key: "Enter".to_string(),
                    description: "Start fingerprinting".to_string(),
                    category: "Action".to_string(),
                }],
            },
        );

        sections.insert(
            Tab::Fuzz,
            HelpSection {
                title: "Fuzzing".to_string(),
                content: "Web application fuzzing with multiple payload types: SQLi, XSS, Command Injection, SSTI, XXE, etc.".to_string(),
                commands: vec![
                    HelpCommand { key: "Enter".to_string(), description: "Start fuzzing".to_string(), category: "Action".to_string() },
                    HelpCommand { key: "1-9".to_string(), description: "Select payload type".to_string(), category: "Selection".to_string() },
                    HelpCommand { key: "t".to_string(), description: "Configure threads".to_string(), category: "Config".to_string() },
                ],
            },
        );

        sections.insert(
            Tab::Waf,
            HelpSection {
                title: "WAF Detection".to_string(),
                content:
                    "Detect and identify Web Application Firewalls. Use payload testing to bypass."
                        .to_string(),
                commands: vec![HelpCommand {
                    key: "Enter".to_string(),
                    description: "Start detection".to_string(),
                    category: "Action".to_string(),
                }],
            },
        );

        sections.insert(
            Tab::WafStress,
            HelpSection {
                title: "WAF Stress Testing".to_string(),
                content: "Stress test WAFs with high-volume requests to test resilience."
                    .to_string(),
                commands: vec![HelpCommand {
                    key: "Enter".to_string(),
                    description: "Start stress test".to_string(),
                    category: "Action".to_string(),
                }],
            },
        );

        sections.insert(
            Tab::Scan,
            HelpSection {
                title: "Pipeline Scan".to_string(),
                content: "Run multiple scan types in sequence: port scan -> endpoint discovery -> fuzzing.".to_string(),
                commands: vec![
                    HelpCommand { key: "Enter".to_string(), description: "Start pipeline".to_string(), category: "Action".to_string() },
                    HelpCommand { key: "p".to_string(), description: "Configure pipeline".to_string(), category: "Config".to_string() },
                ],
            },
        );

        sections.insert(
            Tab::Resume,
            HelpSection {
                title: "Session Resume".to_string(),
                content: "Resume a previous scan session from saved state.".to_string(),
                commands: vec![
                    HelpCommand {
                        key: "Enter".to_string(),
                        description: "Load session".to_string(),
                        category: "Action".to_string(),
                    },
                    HelpCommand {
                        key: "l".to_string(),
                        description: "List sessions".to_string(),
                        category: "View".to_string(),
                    },
                ],
            },
        );

        sections.insert(
            Tab::Proxy,
            HelpSection {
                title: "Proxy".to_string(),
                content: "Start an HTTP proxy server to intercept and modify traffic.".to_string(),
                commands: vec![
                    HelpCommand {
                        key: "Enter".to_string(),
                        description: "Start proxy".to_string(),
                        category: "Action".to_string(),
                    },
                    HelpCommand {
                        key: "s".to_string(),
                        description: "View statistics".to_string(),
                        category: "View".to_string(),
                    },
                ],
            },
        );

        sections.insert(
            Tab::Packet,
            HelpSection {
                title: "Packet Crafting".to_string(),
                content: "Craft and send raw packets with custom headers and payloads.".to_string(),
                commands: vec![HelpCommand {
                    key: "Enter".to_string(),
                    description: "Send packet".to_string(),
                    category: "Action".to_string(),
                }],
            },
        );

        sections.insert(
            Tab::GraphQl,
            HelpSection {
                title: "GraphQL Testing".to_string(),
                content: "Test GraphQL endpoints for introspection, query injection, and depth limit bypass.".to_string(),
                commands: vec![
                    HelpCommand { key: "Enter".to_string(), description: "Start test".to_string(), category: "Action".to_string() },
                    HelpCommand { key: "i".to_string(), description: "Introspection".to_string(), category: "Test".to_string() },
                ],
            },
        );

        sections.insert(
            Tab::OAuth,
            HelpSection {
                title: "OAuth Testing".to_string(),
                content: "Test OAuth/OIDC authorization endpoints for security misconfigurations."
                    .to_string(),
                commands: vec![HelpCommand {
                    key: "Enter".to_string(),
                    description: "Start test".to_string(),
                    category: "Action".to_string(),
                }],
            },
        );

        sections.insert(
            Tab::Cluster,
            HelpSection {
                title: "Cluster".to_string(),
                content: "Manage distributed scanning cluster. Add workers and orchestrate scans."
                    .to_string(),
                commands: vec![
                    HelpCommand {
                        key: "a".to_string(),
                        description: "Add worker".to_string(),
                        category: "Action".to_string(),
                    },
                    HelpCommand {
                        key: "r".to_string(),
                        description: "Remove worker".to_string(),
                        category: "Action".to_string(),
                    },
                ],
            },
        );

        sections.insert(
            Tab::Stress,
            HelpSection {
                title: "Stress Testing".to_string(),
                content: "Stress test targets with high-volume requests. Supports various attack patterns.".to_string(),
                commands: vec![
                    HelpCommand { key: "Enter".to_string(), description: "Start stress test".to_string(), category: "Action".to_string() },
                ],
            },
        );

        sections.insert(
            Tab::Report,
            HelpSection {
                title: "Reporting".to_string(),
                content: "Generate reports in various formats: JSON, HTML, Markdown, SARIF, JUnit."
                    .to_string(),
                commands: vec![
                    HelpCommand {
                        key: "Enter".to_string(),
                        description: "Generate report".to_string(),
                        category: "Action".to_string(),
                    },
                    HelpCommand {
                        key: "f".to_string(),
                        description: "Select format".to_string(),
                        category: "Config".to_string(),
                    },
                ],
            },
        );

        sections.insert(
            Tab::Nse,
            HelpSection {
                title: "NSE Scripts".to_string(),
                content: "Run Nmap NSE scripts for advanced enumeration.".to_string(),
                commands: vec![HelpCommand {
                    key: "Enter".to_string(),
                    description: "Run scripts".to_string(),
                    category: "Action".to_string(),
                }],
            },
        );

        sections.insert(
            Tab::Plugin,
            HelpSection {
                title: "Plugins".to_string(),
                content: "Run custom Python or Ruby plugins for specialized testing.".to_string(),
                commands: vec![
                    HelpCommand {
                        key: "Enter".to_string(),
                        description: "Run plugin".to_string(),
                        category: "Action".to_string(),
                    },
                    HelpCommand {
                        key: "l".to_string(),
                        description: "List plugins".to_string(),
                        category: "View".to_string(),
                    },
                ],
            },
        );

        sections.insert(
            Tab::Settings,
            HelpSection {
                title: "Settings".to_string(),
                content: "Configure global options: HTTP settings, timeouts, proxies, API keys, wordlists.".to_string(),
                commands: vec![
                    HelpCommand { key: "s".to_string(), description: "Save settings".to_string(), category: "Action".to_string() },
                    HelpCommand { key: "r".to_string(), description: "Reset to defaults".to_string(), category: "Action".to_string() },
                ],
            },
        );

        sections.insert(
            Tab::History,
            HelpSection {
                title: "History".to_string(),
                content: "View scan history and results from previous sessions.".to_string(),
                commands: vec![
                    HelpCommand {
                        key: "Enter".to_string(),
                        description: "View details".to_string(),
                        category: "View".to_string(),
                    },
                    HelpCommand {
                        key: "d".to_string(),
                        description: "Delete entry".to_string(),
                        category: "Action".to_string(),
                    },
                    HelpCommand {
                        key: "r".to_string(),
                        description: "Clear all".to_string(),
                        category: "Action".to_string(),
                    },
                ],
            },
        );

        sections.insert(
            Tab::Dashboard,
            HelpSection {
                title: "Dashboard".to_string(),
                content: "Overview of all scan results, findings summary, and statistics."
                    .to_string(),
                commands: vec![
                    HelpCommand {
                        key: "Enter".to_string(),
                        description: "View details".to_string(),
                        category: "View".to_string(),
                    },
                    HelpCommand {
                        key: "f".to_string(),
                        description: "Filter by severity".to_string(),
                        category: "Filter".to_string(),
                    },
                ],
            },
        );

        Self {
            sections,
            global_commands: vec![
                HelpCommand {
                    key: "Ctrl+C".to_string(),
                    description: "Stop current operation".to_string(),
                    category: "Control".to_string(),
                },
                HelpCommand {
                    key: "q".to_string(),
                    description: "Quit application (when idle)".to_string(),
                    category: "Control".to_string(),
                },
                HelpCommand {
                    key: "s".to_string(),
                    description: "Save settings (Settings tab)".to_string(),
                    category: "Settings".to_string(),
                },
                HelpCommand {
                    key: "r".to_string(),
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
                    key: "/".to_string(),
                    description: "Toggle search".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Ctrl+U/D".to_string(),
                    description: "Page up/down".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "gg/G".to_string(),
                    description: "Go to top/bottom".to_string(),
                    category: "Navigation".to_string(),
                },
            ],
            command_palette_entries: Arc::new(vec![
                CommandPaletteResult {
                    command: "quit".to_string(),
                    description: "Exit the application".to_string(),
                    category: "System".to_string(),
                    shortcut: Some("q".to_string()),
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
                    shortcut: Some("r".to_string()),
                },
                CommandPaletteResult {
                    command: "save".to_string(),
                    description: "Save settings".to_string(),
                    category: "Settings".to_string(),
                    shortcut: Some("s".to_string()),
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
                    shortcut: Some("/".to_string()),
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
                    command: "recon".to_string(),
                    description: "Go to Reconnaissance".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("0".to_string()),
                },
                CommandPaletteResult {
                    command: "load".to_string(),
                    description: "Go to Load Testing".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("1".to_string()),
                },
                CommandPaletteResult {
                    command: "ports".to_string(),
                    description: "Go to Port Scanning".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("2".to_string()),
                },
                CommandPaletteResult {
                    command: "endpoints".to_string(),
                    description: "Go to Endpoint Discovery".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("3".to_string()),
                },
                CommandPaletteResult {
                    command: "fingerprint".to_string(),
                    description: "Go to Service Fingerprinting".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("4".to_string()),
                },
                CommandPaletteResult {
                    command: "fuzz".to_string(),
                    description: "Go to Fuzzing".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("5".to_string()),
                },
                CommandPaletteResult {
                    command: "waf".to_string(),
                    description: "Go to WAF Detection".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("6".to_string()),
                },
                CommandPaletteResult {
                    command: "wafstress".to_string(),
                    description: "Go to WAF Stress Testing".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("7".to_string()),
                },
                CommandPaletteResult {
                    command: "pipeline".to_string(),
                    description: "Go to Pipeline Scan".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("8".to_string()),
                },
                CommandPaletteResult {
                    command: "resume".to_string(),
                    description: "Go to Session Resume".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("9".to_string()),
                },
                CommandPaletteResult {
                    command: "proxy".to_string(),
                    description: "Go to Proxy".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("10".to_string()),
                },
                CommandPaletteResult {
                    command: "packet".to_string(),
                    description: "Go to Packet Crafting".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("11".to_string()),
                },
                CommandPaletteResult {
                    command: "graphql".to_string(),
                    description: "Go to GraphQL Testing".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("12".to_string()),
                },
                CommandPaletteResult {
                    command: "oauth".to_string(),
                    description: "Go to OAuth Testing".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("13".to_string()),
                },
                CommandPaletteResult {
                    command: "cluster".to_string(),
                    description: "Go to Cluster".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("14".to_string()),
                },
                CommandPaletteResult {
                    command: "stress".to_string(),
                    description: "Go to Stress Testing".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("15".to_string()),
                },
                CommandPaletteResult {
                    command: "report".to_string(),
                    description: "Go to Reporting".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("16".to_string()),
                },
                CommandPaletteResult {
                    command: "nse".to_string(),
                    description: "Go to NSE Scripts".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("17".to_string()),
                },
                CommandPaletteResult {
                    command: "plugin".to_string(),
                    description: "Go to Plugins".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("18".to_string()),
                },
                CommandPaletteResult {
                    command: "settings".to_string(),
                    description: "Go to Settings".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("19".to_string()),
                },
                CommandPaletteResult {
                    command: "history".to_string(),
                    description: "Go to History".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("20".to_string()),
                },
                CommandPaletteResult {
                    command: "dashboard".to_string(),
                    description: "Go to Dashboard".to_string(),
                    category: "Tabs".to_string(),
                    shortcut: Some("21".to_string()),
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
            ]),
        }
    }
}
