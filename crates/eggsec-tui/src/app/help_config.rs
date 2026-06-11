use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::help::{CommandPaletteResult, HelpCommand, HelpSection};
use crate::tabs::Tab;

pub struct StaticHelpData {
    pub sections: FxHashMap<Tab, HelpSection>,
    pub global_commands: Vec<HelpCommand>,
    pub command_palette_entries: Arc<Vec<CommandPaletteResult>>,
}

pub fn get_static_help_data() -> StaticHelpData {
    let mut sections = FxHashMap::default();

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
            content:
                "HTTP load testing with configurable concurrency, duration, and request patterns. Use Tab to navigate between target input, method selector, and results."
                    .to_string(),
            commands: vec![
                HelpCommand {
                    key: "Enter".to_string(),
                    description: "Start load test".to_string(),
                    category: "Action".to_string(),
                },
                HelpCommand {
                    key: "Tab".to_string(),
                    description: "Move focus".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Space".to_string(),
                    description: "Pause/Resume".to_string(),
                    category: "Control".to_string(),
                },
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
            content: "Directory and file discovery using wordlists. Supports recursive scanning."
                .to_string(),
            commands: vec![HelpCommand {
                key: "Enter".to_string(),
                description: "Start scan".to_string(),
                category: "Action".to_string(),
            }],
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
            content: "Web application fuzzing with multiple payload types: SQLi, XSS, Command Injection, SSTI, XXE, etc. Tab cycles through target input, method selector, payload selector, and results."
                .to_string(),
            commands: vec![
                HelpCommand { key: "Enter".to_string(), description: "Start fuzzing".to_string(), category: "Action".to_string() },
                HelpCommand { key: "Tab".to_string(), description: "Move focus".to_string(), category: "Navigation".to_string() },
                HelpCommand { key: "Up/Down".to_string(), description: "Navigate selector".to_string(), category: "Navigation".to_string() },
                HelpCommand { key: "Esc".to_string(), description: "Close selector".to_string(), category: "Navigation".to_string() },
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
            content: "Stress test WAFs with high-volume requests to test resilience.".to_string(),
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
            content:
                "Run multiple scan types in sequence: port scan -> endpoint discovery -> fuzzing. Tab cycles through profile selector, target input, and results."
                    .to_string(),
            commands: vec![
                HelpCommand {
                    key: "Enter".to_string(),
                    description: "Start pipeline".to_string(),
                    category: "Action".to_string(),
                },
                HelpCommand {
                    key: "Tab".to_string(),
                    description: "Move focus".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Up/Down".to_string(),
                    description: "Navigate selector".to_string(),
                    category: "Navigation".to_string(),
                },
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
            content:
                "Test GraphQL endpoints for introspection, query injection, and depth limit bypass."
                    .to_string(),
            commands: vec![
                HelpCommand {
                    key: "Enter".to_string(),
                    description: "Start test".to_string(),
                    category: "Action".to_string(),
                },
                HelpCommand {
                    key: "i".to_string(),
                    description: "Introspection".to_string(),
                    category: "Test".to_string(),
                },
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
            content: "Manage distributed scanning cluster. Worker mode connects to a coordinator; Coordinator mode runs the master node. Use Tab to navigate between Mode selector, Inputs, and Results."
                .to_string(),
            commands: vec![
                HelpCommand {
                    key: "Tab".to_string(),
                    description: "Move focus to next area".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Enter".to_string(),
                    description: "Select/open selector".to_string(),
                    category: "Action".to_string(),
                },
                HelpCommand {
                    key: "Esc".to_string(),
                    description: "Close dropdown".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Up/Down".to_string(),
                    description: "Navigate selector options".to_string(),
                    category: "Navigation".to_string(),
                },
            ],
        },
    );

    sections.insert(
        Tab::Stress,
        HelpSection {
            title: "Stress Testing".to_string(),
            content:
                "Stress test targets with high-volume requests. Supports various attack patterns."
                    .to_string(),
            commands: vec![HelpCommand {
                key: "Enter".to_string(),
                description: "Start stress test".to_string(),
                category: "Action".to_string(),
            }],
        },
    );

    sections.insert(
        Tab::Report,
        HelpSection {
            title: "Reporting".to_string(),
            content: "Generate reports in various formats: JSON, HTML, Markdown, SARIF, JUnit. Tab cycles through format selector, input fields, and results."
                .to_string(),
            commands: vec![
                HelpCommand {
                    key: "Enter".to_string(),
                    description: "Generate report".to_string(),
                    category: "Action".to_string(),
                },
                HelpCommand {
                    key: "Tab".to_string(),
                    description: "Move focus".to_string(),
                    category: "Navigation".to_string(),
                },
                HelpCommand {
                    key: "Up/Down".to_string(),
                    description: "Navigate selector".to_string(),
                    category: "Navigation".to_string(),
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
        Tab::Settings,
        HelpSection {
            title: "Settings".to_string(),
            content:
                "Configure global options: HTTP settings, timeouts, proxies, API keys, wordlists."
                    .to_string(),
            commands: vec![
                HelpCommand {
                    key: "s".to_string(),
                    description: "Save settings".to_string(),
                    category: "Action".to_string(),
                },
                HelpCommand {
                    key: "r".to_string(),
                    description: "Reset to defaults".to_string(),
                    category: "Action".to_string(),
                },
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
            content: "Overview of all scan results, findings summary, and statistics.".to_string(),
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

    sections.insert(
        Tab::Hunt,
        HelpSection {
            title: "Vulnerability Hunting".to_string(),
            content: "Advanced vulnerability hunting with intelligent discovery rules.".to_string(),
            commands: vec![HelpCommand {
                key: "Enter".to_string(),
                description: "Start hunting".to_string(),
                category: "Action".to_string(),
            }],
        },
    );

    sections.insert(
        Tab::Browser,
        HelpSection {
            title: "Headless Browser".to_string(),
            content: "Headless browser testing for modern single-page applications.".to_string(),
            commands: vec![HelpCommand {
                key: "Enter".to_string(),
                description: "Open browser".to_string(),
                category: "Action".to_string(),
            }],
        },
    );

    sections.insert(
        Tab::Compliance,
        HelpSection {
            title: "Compliance".to_string(),
            content: "Generate compliance reports for various standards (OWASP, PCI, etc.)."
                .to_string(),
            commands: vec![HelpCommand {
                key: "Enter".to_string(),
                description: "Run check".to_string(),
                category: "Action".to_string(),
            }],
        },
    );

    sections.insert(
        Tab::Storage,
        HelpSection {
            title: "Database Storage".to_string(),
            content: "Manage database storage for scan results and findings.".to_string(),
            commands: vec![HelpCommand {
                key: "Enter".to_string(),
                description: "Query DB".to_string(),
                category: "Action".to_string(),
            }],
        },
    );

    sections.insert(
        Tab::Integrations,
        HelpSection {
            title: "Integrations".to_string(),
            content: "Integrate with issue trackers and external security tools.".to_string(),
            commands: vec![HelpCommand {
                key: "Enter".to_string(),
                description: "Sync findings".to_string(),
                category: "Action".to_string(),
            }],
        },
    );

    sections.insert(
        Tab::Workflow,
        HelpSection {
            title: "Workflow".to_string(),
            content: "Manage security finding lifecycles and remediation workflows.".to_string(),
            commands: vec![HelpCommand {
                key: "Enter".to_string(),
                description: "Update status".to_string(),
                category: "Action".to_string(),
            }],
        },
    );

    sections.insert(
        Tab::Vuln,
        HelpSection {
            title: "Vulnerability Management".to_string(),
            content: "Track and prioritize vulnerabilities across multiple targets.".to_string(),
            commands: vec![HelpCommand {
                key: "Enter".to_string(),
                description: "Manage findings".to_string(),
                category: "Action".to_string(),
            }],
        },
    );

    #[cfg(feature = "wireless")]
    sections.insert(
        Tab::Wireless,
        HelpSection {
            title: "Wireless Scanning".to_string(),
            content: "Scan wireless networks for security issues. Detects Open, WEP, WPA, WPA2, WPA3, and Enterprise (802.1X) networks.".to_string(),
            commands: vec![
                HelpCommand { key: "Enter".to_string(), description: "Start scan".to_string(), category: "Action".to_string() },
                HelpCommand { key: "Tab".to_string(), description: "Toggle focus".to_string(), category: "Navigation".to_string() },
                HelpCommand { key: "Esc".to_string(), description: "Stop scan".to_string(), category: "Control".to_string() },
            ],
        },
    );

    let global_commands = vec![
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
        HelpCommand {
            key: "Ctrl+Z".to_string(),
            description: "Pause/Resume".to_string(),
            category: "Control".to_string(),
        },
        HelpCommand {
            key: "Ctrl+T".to_string(),
            description: "Cycle built-in theme".to_string(),
            category: "Settings".to_string(),
        },
        HelpCommand {
            key: "Ctrl+V".to_string(),
            description: "Paste from clipboard".to_string(),
            category: "Edit".to_string(),
        },
    ];

    let command_palette_entries = Arc::new(vec![
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
            command: "pause".to_string(),
            description: "Pause active task".to_string(),
            category: "System".to_string(),
            shortcut: Some("Ctrl+Z".to_string()),
        },
        CommandPaletteResult {
            command: "resume".to_string(),
            description: "Resume paused task".to_string(),
            category: "System".to_string(),
            shortcut: Some("Ctrl+Y".to_string()),
        },
        CommandPaletteResult {
            command: "resume-task".to_string(),
            description: "Resume paused task".to_string(),
            category: "System".to_string(),
            shortcut: Some("Ctrl+Y".to_string()),
        },
        CommandPaletteResult {
            command: "pause-task".to_string(),
            description: "Pause active task".to_string(),
            category: "System".to_string(),
            shortcut: Some("Ctrl+Z".to_string()),
        },
        CommandPaletteResult {
            command: "stop-task".to_string(),
            description: "Stop active task".to_string(),
            category: "System".to_string(),
            shortcut: Some("Ctrl+C".to_string()),
        },
        CommandPaletteResult {
            command: "jump-active".to_string(),
            description: "Jump to active task tab (if any)".to_string(),
            category: "System".to_string(),
            shortcut: None,
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
            command: "help-current".to_string(),
            description: "Open help for current tab".to_string(),
            category: "Navigation".to_string(),
            shortcut: Some("Ctrl+/".to_string()),
        },
        CommandPaletteResult {
            command: "search".to_string(),
            description: "Toggle search (local)".to_string(),
            category: "Navigation".to_string(),
            shortcut: Some("/".to_string()),
        },
        CommandPaletteResult {
            command: "global-search".to_string(),
            description: "Toggle global search".to_string(),
            category: "Navigation".to_string(),
            shortcut: Some("Ctrl+Shift+/".to_string()),
        },
        CommandPaletteResult {
            command: "open-search".to_string(),
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
            shortcut: Some("e".to_string()),
        },
        CommandPaletteResult {
            command: "cycle-export".to_string(),
            description: "Cycle export format".to_string(),
            category: "Data".to_string(),
            shortcut: None,
        },
        CommandPaletteResult {
            command: "theme".to_string(),
            description: "Cycle theme (next)".to_string(),
            category: "Appearance".to_string(),
            shortcut: Some("Ctrl+T".to_string()),
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
            command: "settings".to_string(),
            description: "Go to Settings".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("18".to_string()),
        },
        CommandPaletteResult {
            command: "history".to_string(),
            description: "Go to History".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("19".to_string()),
        },
        CommandPaletteResult {
            command: "dashboard".to_string(),
            description: "Go to Dashboard".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("20".to_string()),
        },
        CommandPaletteResult {
            command: "hunt".to_string(),
            description: "Go to Vulnerability Hunting".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("21".to_string()),
        },
        CommandPaletteResult {
            command: "browser".to_string(),
            description: "Go to Headless Browser".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("22".to_string()),
        },
        CommandPaletteResult {
            command: "compliance".to_string(),
            description: "Go to Compliance".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("23".to_string()),
        },
        CommandPaletteResult {
            command: "storage".to_string(),
            description: "Go to Database Storage".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("24".to_string()),
        },
        CommandPaletteResult {
            command: "integrations".to_string(),
            description: "Go to Integrations".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("25".to_string()),
        },
        CommandPaletteResult {
            command: "workflow".to_string(),
            description: "Go to Workflow".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("26".to_string()),
        },
        CommandPaletteResult {
            command: "vuln".to_string(),
            description: "Go to Vulnerability Management".to_string(),
            category: "Tabs".to_string(),
            shortcut: Some("27".to_string()),
        },
        CommandPaletteResult {
            command: "next-tab".to_string(),
            description: "Go to next tab".to_string(),
            category: "Navigation".to_string(),
            shortcut: Some("n".to_string()),
        },
        CommandPaletteResult {
            command: "prev-tab".to_string(),
            description: "Go to previous tab".to_string(),
            category: "Navigation".to_string(),
            shortcut: Some("p".to_string()),
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
        CommandPaletteResult {
            command: "run".to_string(),
            description: "Run current tab/action (Enter)".to_string(),
            category: "Action".to_string(),
            shortcut: Some("Enter".to_string()),
        },
        CommandPaletteResult {
            command: "run-current".to_string(),
            description: "Run current tab/action".to_string(),
            category: "Action".to_string(),
            shortcut: Some("Enter".to_string()),
        },
        CommandPaletteResult {
            command: "quick-switch".to_string(),
            description: "Open quick switch".to_string(),
            category: "Navigation".to_string(),
            shortcut: Some("Ctrl+Q".to_string()),
        },
        CommandPaletteResult {
            command: "open-quick".to_string(),
            description: "Open quick switch".to_string(),
            category: "Navigation".to_string(),
            shortcut: Some("Ctrl+Q".to_string()),
        },
        CommandPaletteResult {
            command: "copy-cli".to_string(),
            description: "Copy CLI equivalent of current tab".to_string(),
            category: "Data".to_string(),
            shortcut: None,
        },
        CommandPaletteResult {
            command: "reload-scope".to_string(),
            description: "Reload scope/config".to_string(),
            category: "Settings".to_string(),
            shortcut: None,
        },
        CommandPaletteResult {
            command: "save-settings".to_string(),
            description: "Save settings (contextual on Settings)".to_string(),
            category: "Settings".to_string(),
            shortcut: Some("s".to_string()),
        },
        CommandPaletteResult {
            command: "clear-history".to_string(),
            description: "Clear history (contextual on History)".to_string(),
            category: "Data".to_string(),
            shortcut: Some("r".to_string()),
        },
        CommandPaletteResult {
            command: "delete-history".to_string(),
            description: "Delete history entry (contextual on History)".to_string(),
            category: "Data".to_string(),
            shortcut: Some("d".to_string()),
        },
    ]);

    StaticHelpData {
        sections,
        global_commands,
        command_palette_entries,
    }
}
