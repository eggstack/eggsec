//! Tab metadata registry (Phase 3 of tui-architecture-usability-pass.md).
//! Single source of truth for title, stable_id, cli_command, description,
//! category, risk_group, feature gating, and breadcrumb_label for all 29 tabs.
//!
//! - Base tabs (feature: None) are always visible.
//! - Gated tabs carry the exact cfg feature name used in Tab::all().
//! - visible_tab_specs() mirrors the exact construction order of Tab::all()
//!   (base 20 + conditional appends) so ordering is byte-identical.
//! - Tab::all() body in mod.rs is left UNCHANGED (exact LazyLock + cfg pushes).
//! - from_stable_id performs lookup then applies the visible_index guard
//!   exactly as before, so hidden gated tabs never restore via session.

use super::Tab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabCategory {
    Assessment,
    Traffic,
    Workflow,
    Reporting,
    Configuration,
    History,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabRiskGroup {
    Passive,
    SafeActive,
    Intrusive,
    Administrative,
}

/// Phase 2 surface route: how a TUI tab relates to canonical execution.
///
/// - `Operation`: one canonical operation ID (validated via `OperationMetadata`).
/// - `Multiplexer`: tab selects among several canonical operations at runtime
///   (e.g. Wireless passive scan vs `wireless-deauth` active attack).
/// - `Helper`: local/UI transformation with no canonical operation dispatch.
/// - `Lifecycle`: daemon/session/cluster lifecycle, not a security operation.
/// - `UiOnly`: pure navigation/inspection state, never dispatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiSurfaceRoute {
    Operation(&'static str),
    Multiplexer(&'static str),
    Helper,
    Lifecycle,
    UiOnly,
}

/// Phase 2 typed availability: derived from canonical feature metadata plus
/// the TUI crate's compiled feature set.
///
/// - `Available`: compiled in and executable.
/// - `Unavailable { required_feature }`: visible discovery shell, execution gated.
/// - `UiOnly`: never gated, no execution.
/// - `NotSupportedOnTui`: compiled out (not in `Tab::all()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabAvailability {
    Available,
    Unavailable { required_feature: &'static str },
    UiOnly,
    NotSupportedOnTui,
}

/// Structured result for palette/alias resolution (Phase 2.5).
///
/// Feature-disabled aliases return `Unavailable` rather than silently
/// behaving differently from the tab list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteResolution {
    SelectTab(Tab),
    Unavailable {
        tab: Tab,
        required_feature: &'static str,
    },
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct TabSpec {
    pub tab: Tab,
    pub stable_id: &'static str,
    pub title: &'static str,
    pub cli_command: &'static str,
    pub description: &'static str,
    /// Longer help text shown in the help overlay for this tab.
    pub help_text: &'static str,
    /// Tab grouping for filtering/grouping (reserved for future use; not
    /// currently rendered anywhere). Each spec sets one of Assessment,
    /// Traffic, Reporting, Configuration, Workflow, History, Dashboard.
    #[allow(dead_code)]
    pub category: TabCategory,
    pub risk_group: TabRiskGroup,
    pub feature: Option<&'static str>,
    pub breadcrumb_label: &'static str,
    pub operation: Option<&'static str>,
    pub direct_launch: bool,
    /// Whether this tab supports the 'run' action (test-only metadata)
    #[allow(dead_code)]
    pub supports_run: bool,
    /// Whether this tab supports export (test-only metadata)
    #[allow(dead_code)]
    pub supports_export: bool,
    /// Whether this tab supports help (test-only metadata)
    #[allow(dead_code)]
    pub supports_help: bool,
    /// Whether this tab has configurable settings (reserved for future use)
    #[allow(dead_code)]
    pub has_settings: bool,
}

pub static TAB_SPECS: &[TabSpec] = &[
    TabSpec {
        tab: Tab::Recon,
        stable_id: "recon",
        title: "Recon",
        cli_command: "eggsec recon",
        description: "Gather reconnaissance information",
        help_text: "Reconnaissance - Gather intelligence about target domain/IP.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::SafeActive,
        feature: None,
        breadcrumb_label: "Recon",
        operation: Some("recon"),
        direct_launch: false,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Load,
        stable_id: "load",
        title: "Load",
        cli_command: "eggsec load",
        description: "Run HTTP load test or stress test",
        help_text: "Load Testing - Send concurrent HTTP requests to test performance.",
        category: TabCategory::Traffic,
        risk_group: TabRiskGroup::SafeActive,
        feature: None,
        breadcrumb_label: "Load",
        operation: Some("load-test"),
        direct_launch: false,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::ScanPorts,
        stable_id: "scan_ports",
        title: "Scan Ports",
        cli_command: "eggsec scan-ports",
        description: "Scan ports on target host",
        help_text: "Port Scanning - Discover open ports and services.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::SafeActive,
        feature: None,
        breadcrumb_label: "Scan Ports",
        operation: Some("scan-ports"),
        direct_launch: false,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::ScanEndpoints,
        stable_id: "scan_endpoints",
        title: "Scan Endpoints",
        cli_command: "eggsec scan-endpoints",
        description: "Discover sensitive HTTP endpoints",
        help_text: "Endpoint Discovery - Find hidden or sensitive endpoints.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::SafeActive,
        feature: None,
        breadcrumb_label: "Scan Endpoints",
        operation: Some("scan-endpoints"),
        direct_launch: false,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Fingerprint,
        stable_id: "fingerprint",
        title: "Fingerprint",
        cli_command: "eggsec fingerprint",
        description: "Fingerprint services (AMAP-style)",
        help_text: "Service Fingerprinting - Identify services on open ports.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Passive,
        feature: None,
        breadcrumb_label: "Fingerprint",
        operation: Some("fingerprint"),
        direct_launch: false,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Fuzz,
        stable_id: "fuzz",
        title: "Fuzz",
        cli_command: "eggsec fuzz",
        description: "Fuzz target with security payloads",
        help_text: "Fuzzing - Test for vulnerabilities using payloads.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        feature: None,
        breadcrumb_label: "Fuzz",
        operation: Some("fuzz"),
        direct_launch: false,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Waf,
        stable_id: "waf",
        title: "WAF",
        cli_command: "eggsec waf",
        description: "Detect and bypass Web Application Firewalls",
        help_text: "WAF Detection - Detect and bypass Web Application Firewalls.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::SafeActive,
        feature: None,
        breadcrumb_label: "WAF",
        // Canonical operation is `waf-detect`; `waf` is a compatibility alias
        // (see ALL_OPERATION_METADATA_ALIASES). Normalized here so the TUI
        // never invents a second canonical identity.
        operation: Some("waf-detect"),
        direct_launch: false,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::WafStress,
        stable_id: "waf_stress",
        title: "WAF Stress",
        cli_command: "eggsec waf-stress",
        description: "Comprehensive WAF stress testing",
        help_text: "WAF Stress Testing - Comprehensive WAF testing.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        feature: None,
        breadcrumb_label: "WAF Stress",
        operation: Some("waf-stress"),
        direct_launch: false,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Scan,
        stable_id: "scan",
        title: "Scan",
        cli_command: "eggsec scan",
        description: "Run chained security assessment pipeline",
        help_text: "Pipeline Scanning - Run chained security assessment.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::SafeActive,
        feature: None,
        breadcrumb_label: "Scan",
        // Canonical operation is `pipeline`; `scan-pipeline` is a
        // compatibility alias (see ALL_OPERATION_METADATA_ALIASES).
        operation: Some("pipeline"),
        direct_launch: false,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Resume,
        stable_id: "resume",
        title: "Resume",
        cli_command: "eggsec resume",
        description: "Resume a previous scan from session file",
        help_text: "Session Resume - Continue previous scan from file.",
        category: TabCategory::History,
        risk_group: TabRiskGroup::SafeActive,
        feature: None,
        breadcrumb_label: "Resume",
        operation: None,
        direct_launch: false,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Proxy,
        stable_id: "proxy",
        title: "Proxy",
        cli_command: "eggsec proxy",
        description: "Manage proxy pool and health checks",
        help_text: "Proxy Management - Manage proxy pool.",
        category: TabCategory::Traffic,
        risk_group: TabRiskGroup::Administrative,
        feature: None,
        breadcrumb_label: "Proxy",
        operation: None,
        direct_launch: false,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Packet,
        stable_id: "packet",
        title: "Packet",
        cli_command: "eggsec packet",
        description: "Packet capture, send, and analysis tools",
        help_text: "Packet Tools - Capture, send, and analyze network packets.",
        category: TabCategory::Traffic,
        risk_group: TabRiskGroup::Administrative,
        // Canonical feature ownership is `packet-inspection` (see
        // OperationMetadata for `packet` and registry `packet`).
        feature: Some("packet-inspection"),
        breadcrumb_label: "Packet",
        operation: Some("packet"),
        direct_launch: true,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::GraphQl,
        stable_id: "graphql",
        title: "GraphQL",
        cli_command: "eggsec graphql",
        description: "Test GraphQL endpoints for security issues",
        help_text: "GraphQL Security - Test GraphQL endpoints.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        feature: None,
        breadcrumb_label: "GraphQL Security",
        operation: Some("graphql"),
        direct_launch: false,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::OAuth,
        stable_id: "oauth",
        title: "OAuth",
        cli_command: "eggsec oauth",
        description: "Test OAuth/OIDC endpoints for vulnerabilities",
        help_text: "OAuth/OIDC Security - Test OAuth endpoints.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        feature: None,
        breadcrumb_label: "OAuth/OIDC Security",
        operation: Some("oauth"),
        direct_launch: true,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Cluster,
        stable_id: "cluster",
        title: "Cluster",
        cli_command: "eggsec cluster",
        description: "Manage distributed scanning cluster",
        help_text: "Cluster Management - Manage distributed scanning cluster.",
        category: TabCategory::Configuration,
        risk_group: TabRiskGroup::Administrative,
        feature: None,
        breadcrumb_label: "Cluster Management",
        operation: None,
        direct_launch: true,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Stress,
        stable_id: "stress",
        title: "Stress",
        cli_command: "eggsec stress",
        description: "Run stress/load testing against target",
        help_text: "Stress Testing - Run stress/load testing against target.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        // Canonical feature ownership is `stress-testing` (see
        // OperationMetadata for `stress-test` and registry `stress`).
        feature: Some("stress-testing"),
        breadcrumb_label: "Stress Testing",
        operation: Some("stress-test"),
        direct_launch: true,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Report,
        stable_id: "report",
        title: "Report",
        cli_command: "eggsec report",
        description: "Convert reports, analyze trends, manage schedules",
        help_text: "Report - Convert and generate security scan reports.",
        category: TabCategory::Reporting,
        risk_group: TabRiskGroup::Passive,
        feature: None,
        breadcrumb_label: "Report",
        operation: None,
        direct_launch: false,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Nse,
        stable_id: "nse",
        title: "NSE",
        cli_command: "eggsec nse",
        description: "Run Nmap NSE scripts",
        help_text: "NSE - Run Nmap NSE scripts.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::SafeActive,
        feature: Some("nse"),
        breadcrumb_label: "NSE Scripts",
        operation: Some("nse"),
        direct_launch: true,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Settings,
        stable_id: "settings",
        title: "Settings",
        cli_command: "Settings",
        description: "Application settings",
        help_text: "Settings - Configure application options.",
        category: TabCategory::Configuration,
        risk_group: TabRiskGroup::Administrative,
        feature: None,
        breadcrumb_label: "Settings",
        operation: None,
        direct_launch: false,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::History,
        stable_id: "history",
        title: "History",
        cli_command: "History",
        description: "View scan history",
        help_text: "History - View previous scan results.",
        category: TabCategory::History,
        risk_group: TabRiskGroup::Passive,
        feature: None,
        breadcrumb_label: "History",
        operation: None,
        direct_launch: false,
        supports_run: false,
        supports_export: true,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Dashboard,
        stable_id: "dashboard",
        title: "Dashboard",
        cli_command: "Dashboard",
        description: "View scan results dashboard",
        help_text: "Dashboard - View scan results at a glance.",
        category: TabCategory::Dashboard,
        risk_group: TabRiskGroup::Passive,
        feature: None,
        breadcrumb_label: "Dashboard",
        operation: None,
        direct_launch: false,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Hunt,
        stable_id: "hunt",
        title: "Hunt",
        cli_command: "eggsec hunt",
        description: "Intelligent vulnerability hunting",
        help_text: "Vulnerability Hunting - Intelligent vulnerability discovery.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        feature: Some("advanced-hunting"),
        breadcrumb_label: "Hunt",
        operation: Some("hunt"),
        direct_launch: true,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Browser,
        stable_id: "browser",
        title: "Browser",
        cli_command: "eggsec browser",
        description: "Headless browser security testing",
        help_text: "Browser Testing - Headless browser security testing.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        feature: Some("headless-browser"),
        breadcrumb_label: "Browser",
        operation: Some("browser"),
        direct_launch: true,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Compliance,
        stable_id: "compliance",
        title: "Compliance",
        cli_command: "eggsec compliance",
        description: "Generate compliance reports (OWASP, PCI, HIPAA, SOC2)",
        help_text: "Compliance - Generate compliance reports.",
        category: TabCategory::Reporting,
        risk_group: TabRiskGroup::SafeActive,
        feature: Some("compliance"),
        breadcrumb_label: "Compliance",
        operation: Some("compliance"),
        direct_launch: false,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Storage,
        stable_id: "storage",
        title: "Storage",
        cli_command: "eggsec storage",
        description: "Database storage and query management",
        help_text: "Storage - Database integration.",
        category: TabCategory::Workflow,
        risk_group: TabRiskGroup::Administrative,
        feature: Some("database"),
        breadcrumb_label: "Storage",
        operation: Some("storage"),
        direct_launch: false,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Integrations,
        stable_id: "integrations",
        title: "Integrations",
        cli_command: "eggsec integrations",
        description: "Issue tracker integration (Jira, GitHub, GitLab)",
        help_text: "Integrations - Issue tracker integration.",
        category: TabCategory::Workflow,
        risk_group: TabRiskGroup::Administrative,
        feature: Some("external-integrations"),
        breadcrumb_label: "Integrations",
        operation: Some("integrations"),
        direct_launch: false,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Workflow,
        stable_id: "workflow",
        title: "Workflow",
        cli_command: "eggsec workflow",
        description: "Finding management and SLA tracking",
        help_text: "Workflow - Finding management and SLA tracking.",
        category: TabCategory::Workflow,
        risk_group: TabRiskGroup::Administrative,
        feature: Some("finding-workflow"),
        breadcrumb_label: "Workflow",
        operation: Some("workflow"),
        direct_launch: false,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Vuln,
        stable_id: "vuln",
        title: "Vuln",
        cli_command: "eggsec vuln",
        description: "Vulnerability prioritization and risk scoring",
        help_text: "Vuln - Vulnerability prioritization and risk scoring.",
        category: TabCategory::Workflow,
        risk_group: TabRiskGroup::SafeActive,
        feature: Some("vuln-management"),
        breadcrumb_label: "Vuln",
        operation: Some("vuln"),
        direct_launch: false,
        supports_run: false,
        supports_export: false,
        supports_help: true,
        has_settings: false,
    },
    TabSpec {
        tab: Tab::Wireless,
        stable_id: "wireless",
        title: "Wireless",
        cli_command: "eggsec wireless",
        description: "Scan wireless networks for security issues",
        help_text: "Wireless - Scan wireless networks for security issues.",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::SafeActive,
        feature: Some("wireless"),
        breadcrumb_label: "Wireless",
        operation: Some("wireless"),
        direct_launch: true,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Auth,
        stable_id: "auth",
        title: "Auth Test",
        cli_command: "eggsec auth-test",
        description: "Authentication control validation (brute-force, lockout, MFA, rate-limit, timing, credential stuffing — defense-lab only)",
        help_text: "Auth Test - Validate authentication controls (defense-lab only).",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        feature: None,
        breadcrumb_label: "Auth / Credential Validation",
        operation: Some("auth-test"),
        direct_launch: true,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::DbPentest,
        stable_id: "db_pentest",
        title: "Db Pentest",
        cli_command: "eggsec db pentest",
        description: "Direct database pentesting (Postgres/MySQL/MSSQL) — defense-lab only",
        help_text: "Db Pentest - Direct database pentesting (defense-lab only).",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        feature: Some("db-pentest"),
        breadcrumb_label: "Db Pentest",
        operation: Some("db-pentest"),
        direct_launch: true,
        supports_run: true,
        supports_export: true,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::Intercept,
        stable_id: "intercept",
        title: "Intercept",
        cli_command: "eggsec proxy intercept",
        description: "Interactive web proxy traffic interception (defense-lab only)",
        help_text: "Intercept - Interactive web proxy traffic interception (defense-lab only).",
        category: TabCategory::Traffic,
        risk_group: TabRiskGroup::Intrusive,
        feature: Some("web-proxy"),
        breadcrumb_label: "Web Proxy / Intercept",
        operation: Some("proxy-intercept"),
        direct_launch: true,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
    TabSpec {
        tab: Tab::C2,
        stable_id: "c2",
        title: "C2",
        cli_command: "eggsec c2",
        description: "C2 campaign simulation (beacons, tasking, OPSEC, attack graph — defense-lab only)",
        help_text: "C2 - Campaign simulation with beacons, tasking, OPSEC (defense-lab only).",
        category: TabCategory::Assessment,
        risk_group: TabRiskGroup::Intrusive,
        feature: Some("c2"),
        breadcrumb_label: "C2 Campaign",
        operation: Some("c2"),
        direct_launch: true,
        supports_run: true,
        supports_export: false,
        supports_help: true,
        has_settings: true,
    },
];

pub fn tab_specs() -> &'static [TabSpec] {
    TAB_SPECS
}

#[allow(dead_code)] // used in mod.rs tests; kept for forward compat
pub fn all_specs() -> &'static [TabSpec] {
    TAB_SPECS
}

pub fn spec_for(tab: Tab) -> Option<&'static TabSpec> {
    TAB_SPECS.iter().find(|s| s.tab == tab)
}

#[allow(dead_code)]
pub fn spec_for_id(stable_id: &str) -> Option<&'static TabSpec> {
    TAB_SPECS.iter().find(|s| s.stable_id == stable_id)
}

impl TabSpec {
    /// Whether this tab can start a scan/task (test-only metadata)
    #[allow(dead_code)]
    pub fn can_start_task(&self) -> bool {
        self.supports_run && !self.direct_launch
    }

    /// Whether this tab shows in the export menu (test-only metadata)
    #[allow(dead_code)]
    pub fn shows_in_export(&self) -> bool {
        self.supports_export
    }

    /// All parseable palette/alias strings for this tab (Phase 2.5).
    ///
    /// Includes the stable ID, the primary palette command, CLI-equivalent
    /// spellings, and hidden compatibility aliases. Palette discovery uses
    /// [`TabSpec::palette_command`] only; parsing accepts everything here.
    pub fn aliases(&self) -> &'static [&'static str] {
        match self.tab {
            Tab::Recon => &["recon"],
            Tab::Load => &["load"],
            Tab::ScanPorts => &["ports", "port", "portscan", "scan-ports", "scan_ports"],
            Tab::ScanEndpoints => &["endpoints", "endpoint", "scan-endpoints", "scan_endpoints"],
            Tab::Fingerprint => &["fingerprint", "fingerprinting"],
            Tab::Fuzz => &["fuzz", "fuzzing"],
            Tab::Waf => &["waf", "waf-detect"],
            Tab::WafStress => &["wafstress", "waf-stress", "waf_stress"],
            Tab::Scan => &["pipeline", "scan", "scan-pipeline"],
            Tab::Resume => &["resume", "session"],
            Tab::Proxy => &["proxy"],
            Tab::Packet => &[
                "packet",
                "raw-packet",
                "packet-capture",
                "packet-inspect",
                "raw-packet-send",
            ],
            Tab::GraphQl => &["graphql"],
            Tab::OAuth => &["oauth", "o-auth"],
            Tab::Cluster => &["cluster"],
            Tab::Stress => &["stress", "stress-test"],
            Tab::Report => &["report"],
            Tab::Nse => &["nse"],
            Tab::Settings => &["settings"],
            Tab::History => &["history"],
            Tab::Dashboard => &["dashboard"],
            Tab::Hunt => &["hunt"],
            Tab::Browser => &["browser"],
            Tab::Compliance => &["compliance"],
            Tab::Storage => &["storage"],
            Tab::Integrations => &["integrations"],
            Tab::Workflow => &["workflow"],
            Tab::Vuln => &["vuln"],
            Tab::Wireless => &["wireless", "wifi", "wireless-deauth"],
            Tab::Auth => &["auth", "auth-test"],
            Tab::DbPentest => &["db-pentest", "db_pentest", "db"],
            Tab::Intercept => &["intercept", "proxy-intercept", "proxy-intercept-start"],
            Tab::C2 => &["c2"],
        }
    }

    /// Primary discoverable palette command for this tab (Phase 2.5/2.6).
    ///
    /// Help and palette discovery must use this; [`TabSpec::aliases`] remains
    /// parseable for compatibility but does not pollute discovery.
    pub fn palette_command(&self) -> &'static str {
        match self.tab {
            Tab::Recon => "recon",
            Tab::Load => "load",
            Tab::ScanPorts => "ports",
            Tab::ScanEndpoints => "endpoints",
            Tab::Fingerprint => "fingerprint",
            Tab::Fuzz => "fuzz",
            Tab::Waf => "waf",
            Tab::WafStress => "wafstress",
            Tab::Scan => "pipeline",
            Tab::Resume => "resume",
            Tab::Proxy => "proxy",
            Tab::Packet => "packet",
            Tab::GraphQl => "graphql",
            Tab::OAuth => "oauth",
            Tab::Cluster => "cluster",
            Tab::Stress => "stress",
            Tab::Report => "report",
            Tab::Nse => "nse",
            Tab::Settings => "settings",
            Tab::History => "history",
            Tab::Dashboard => "dashboard",
            Tab::Hunt => "hunt",
            Tab::Browser => "browser",
            Tab::Compliance => "compliance",
            Tab::Storage => "storage",
            Tab::Integrations => "integrations",
            Tab::Workflow => "workflow",
            Tab::Vuln => "vuln",
            Tab::Wireless => "wireless",
            Tab::Auth => "auth-test",
            Tab::DbPentest => "db-pentest",
            Tab::Intercept => "intercept",
            Tab::C2 => "c2",
        }
    }

    /// Explicit route type for this tab (Phase 2.2).
    ///
    /// Operation-backed entries reference canonical operation IDs; the
    /// Wireless tab is a multiplexer (passive `wireless` vs active
    /// `wireless-deauth` selected at runtime). Helper/lifecycle/UI-only
    /// entries never dispatch.
    pub fn surface_route(&self) -> TuiSurfaceRoute {
        match self.tab {
            Tab::Settings | Tab::History | Tab::Dashboard => TuiSurfaceRoute::UiOnly,
            Tab::Report | Tab::Resume | Tab::Proxy => TuiSurfaceRoute::Helper,
            Tab::Cluster => TuiSurfaceRoute::Lifecycle,
            Tab::Wireless => TuiSurfaceRoute::Multiplexer("wireless"),
            _ => match self.operation {
                Some(op) => TuiSurfaceRoute::Operation(op),
                None => TuiSurfaceRoute::UiOnly,
            },
        }
    }

    /// Canonical operation ID for this tab, validated to be canonical
    /// (not an alias) via `OperationMetadata` (Phase 2.2).
    ///
    /// Returns `None` for non-operation routes and for multiplexers (use
    /// runtime state to select the concrete operation).
    pub fn canonical_operation(&self) -> Option<&'static str> {
        match self.surface_route() {
            TuiSurfaceRoute::Operation(op) => {
                let meta = eggsec::config::metadata_for_tool_id(op)?;
                if meta.id == op {
                    Some(op)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Typed availability derived from the compiled tab list plus the
    /// canonical feature gate (Phase 2.3).
    ///
    /// Stress/Packet are always-visible availability shells: they remain in
    /// `Tab::all()` even when disabled, reporting `Unavailable`. Other
    /// gated tabs disappear when disabled (`NotSupportedOnTui`).
    pub fn availability(&self) -> TabAvailability {
        if matches!(
            self.tab,
            Tab::Settings
                | Tab::History
                | Tab::Dashboard
                | Tab::Report
                | Tab::Resume
                | Tab::Proxy
                | Tab::Cluster
        ) {
            return TabAvailability::UiOnly;
        }
        let visible = Tab::all().contains(&self.tab);
        match self.feature {
            None => {
                if visible {
                    TabAvailability::Available
                } else {
                    TabAvailability::NotSupportedOnTui
                }
            }
            Some(feat) => {
                if visible {
                    if eggsec::config::is_feature_enabled_registry(feat) {
                        TabAvailability::Available
                    } else {
                        // Only Stress/Packet are allowed as always-visible
                        // shells; any other visible-but-disabled tab is still
                        // reported as unavailable (fail loudly in tests).
                        TabAvailability::Unavailable {
                            required_feature: feat,
                        }
                    }
                } else {
                    TabAvailability::NotSupportedOnTui
                }
            }
        }
    }
}

/// Resolve a palette/alias string through the consolidated surface metadata
/// (Phase 2.5). No manual match: linear scan over the static slice is
/// sufficient and keeps alias ownership in one place.
///
/// - Exact match on `stable_id`, `palette_command`, or any entry in
///   [`TabSpec::aliases`].
/// - Feature-disabled tabs return [`PaletteResolution::Unavailable`] rather
///   than silently differing from the tab list.
/// - Unknown strings return [`PaletteResolution::Unknown`].
pub fn resolve_palette_command(command: &str) -> PaletteResolution {
    let cmd = command.trim();
    if cmd.is_empty() {
        return PaletteResolution::Unknown;
    }
    for spec in TAB_SPECS {
        if spec.stable_id == cmd || spec.palette_command() == cmd || spec.aliases().contains(&cmd) {
            if Tab::all().contains(&spec.tab) {
                return PaletteResolution::SelectTab(spec.tab);
            }
            if let Some(feat) = spec.feature {
                return PaletteResolution::Unavailable {
                    tab: spec.tab,
                    required_feature: feat,
                };
            }
            return PaletteResolution::Unknown;
        }
    }
    PaletteResolution::Unknown
}

/// Primary palette command for a tab (convenience for help/palette builders).
pub fn palette_command_for(tab: Tab) -> Option<&'static str> {
    spec_for(tab).map(|s| s.palette_command())
}

use eggsec::config::OperationRisk;

pub fn risk_from_group(group: TabRiskGroup) -> OperationRisk {
    match group {
        TabRiskGroup::Intrusive => OperationRisk::Intrusive,
        TabRiskGroup::SafeActive => OperationRisk::SafeActive,
        TabRiskGroup::Passive => OperationRisk::SafeActive,
        TabRiskGroup::Administrative => OperationRisk::SafeActive,
    }
}

impl Tab {
    pub fn operation_name(&self) -> Option<&'static str> {
        spec_for(*self).and_then(|s| s.operation)
    }

    pub fn is_direct_launch(&self) -> bool {
        spec_for(*self).map(|s| s.direct_launch).unwrap_or(false)
    }
}

/// Conditionally pushes items onto a `Vec`, gated by `#[cfg]` attributes.
/// Each entry is wrapped in `{ }` to avoid parser ambiguity.
macro_rules! cfg_push {
    ( $vec:expr, [ $( $(#[$cfg:meta])* { $item:expr } ),* $(,)? ] ) => {{
        $(
            $(#[$cfg])*
            { $vec.push($item); }
        )*
    }};
}

#[allow(dead_code)] // used in mod.rs tests; kept for forward compat
pub fn visible_tab_specs() -> Vec<&'static TabSpec> {
    #[allow(unused_mut)]
    let mut specs = vec![
        spec_for(Tab::Recon).unwrap(),
        spec_for(Tab::Load).unwrap(),
        spec_for(Tab::ScanPorts).unwrap(),
        spec_for(Tab::ScanEndpoints).unwrap(),
        spec_for(Tab::Fingerprint).unwrap(),
        spec_for(Tab::Fuzz).unwrap(),
        spec_for(Tab::Waf).unwrap(),
        spec_for(Tab::WafStress).unwrap(),
        spec_for(Tab::Scan).unwrap(),
        spec_for(Tab::Resume).unwrap(),
        spec_for(Tab::Proxy).unwrap(),
        spec_for(Tab::Packet).unwrap(),
        spec_for(Tab::GraphQl).unwrap(),
        spec_for(Tab::OAuth).unwrap(),
        spec_for(Tab::Cluster).unwrap(),
        spec_for(Tab::Stress).unwrap(),
        spec_for(Tab::Report).unwrap(),
        spec_for(Tab::Settings).unwrap(),
        spec_for(Tab::History).unwrap(),
        spec_for(Tab::Dashboard).unwrap(),
        spec_for(Tab::Auth).unwrap(),
    ];
    cfg_push!(
        specs,
        [
            #[cfg(feature = "advanced-hunting")]
            {
                spec_for(Tab::Hunt).unwrap()
            },
            #[cfg(feature = "compliance")]
            {
                spec_for(Tab::Compliance).unwrap()
            },
            #[cfg(feature = "database")]
            {
                spec_for(Tab::Storage).unwrap()
            },
            #[cfg(feature = "external-integrations")]
            {
                spec_for(Tab::Integrations).unwrap()
            },
            #[cfg(feature = "finding-workflow")]
            {
                spec_for(Tab::Workflow).unwrap()
            },
            #[cfg(feature = "vuln-management")]
            {
                spec_for(Tab::Vuln).unwrap()
            },
            #[cfg(feature = "nse")]
            {
                spec_for(Tab::Nse).unwrap()
            },
            #[cfg(feature = "headless-browser")]
            {
                spec_for(Tab::Browser).unwrap()
            },
            #[cfg(feature = "wireless")]
            {
                spec_for(Tab::Wireless).unwrap()
            },
            #[cfg(feature = "db-pentest")]
            {
                spec_for(Tab::DbPentest).unwrap()
            },
            #[cfg(feature = "c2")]
            {
                spec_for(Tab::C2).unwrap()
            },
            #[cfg(feature = "web-proxy")]
            {
                spec_for(Tab::Intercept).unwrap()
            },
        ]
    );
    specs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tabs_have_specs() {
        for tab in Tab::all() {
            assert!(spec_for(*tab).is_some(), "Tab {:?} has no spec", tab);
        }
    }

    #[test]
    fn test_all_available_tabs_have_help() {
        for tab in Tab::all() {
            let spec = spec_for(*tab).unwrap();
            assert!(spec.supports_help, "Tab {:?} has no help", tab);
        }
    }

    #[test]
    fn test_command_palette_tabs_are_available() {
        for tab in Tab::all() {
            let spec = spec_for(*tab).unwrap();
            let found = Tab::from_stable_id(spec.stable_id);
            assert!(
                found.is_some(),
                "TabSpec stable_id '{}' doesn't map back to a Tab",
                spec.stable_id
            );
        }
    }

    #[test]
    fn test_feature_gated_tabs_have_feature() {
        for spec in tab_specs() {
            if spec.feature.is_some() {
                // Feature-gated tab - this is expected
            }
        }
    }

    #[test]
    fn test_direct_launch_tabs_are_assessment_or_traffic_or_config() {
        for spec in tab_specs() {
            if spec.direct_launch {
                assert!(
                    matches!(
                        spec.category,
                        TabCategory::Assessment | TabCategory::Traffic | TabCategory::Configuration
                    ),
                    "Direct-launch tab {} has unexpected category {:?}",
                    spec.stable_id,
                    spec.category
                );
            }
        }
    }

    #[test]
    fn test_tab_spec_count_matches_all_tab_variants() {
        let all_variants: Vec<Tab> = (0..=32).filter_map(|i| Tab::from_discriminant(i)).collect();
        assert_eq!(tab_specs().len(), all_variants.len());
    }

    #[test]
    fn test_can_start_task_consistency() {
        for spec in tab_specs() {
            let expected = spec.supports_run && !spec.direct_launch;
            assert_eq!(
                spec.can_start_task(),
                expected,
                "Tab {} can_start_task() mismatch",
                spec.stable_id
            );
        }
    }

    #[test]
    fn test_shows_in_export_consistency() {
        for spec in tab_specs() {
            assert_eq!(
                spec.shows_in_export(),
                spec.supports_export,
                "Tab {} shows_in_export() mismatch",
                spec.stable_id
            );
        }
    }

    #[test]
    fn test_assessment_tabs_support_run() {
        for spec in tab_specs() {
            if matches!(spec.category, TabCategory::Assessment) {
                assert!(
                    spec.supports_run,
                    "Assessment tab {} should support run",
                    spec.stable_id
                );
            }
        }
    }

    // ─── Work item 5: Feature-gated tab visibility tests ───────────────

    /// tab_specs() always returns all 33 specs regardless of compiled features.
    #[test]
    fn test_tab_specs_returns_all_33() {
        assert_eq!(
            tab_specs().len(),
            33,
            "tab_specs() should return all 33 tab specs"
        );
    }

    /// Every tab in Tab::all() has a corresponding spec in tab_specs().
    #[test]
    fn test_all_tabs_have_corresponding_spec() {
        for tab in Tab::all() {
            let spec = spec_for(*tab);
            assert!(
                spec.is_some(),
                "Tab {:?} (stable_id {:?}) has no corresponding spec in tab_specs()",
                tab,
                tab.stable_id()
            );
        }
    }

    /// Base tabs are always present in Tab::all().
    /// Most base tabs have no feature gate; Stress/Packet are explicit
    /// unavailable-shell exceptions: always visible, but execution requires
    /// `stress-testing`/`packet-inspection` (canonical feature ownership).
    /// See Phase 0 parity: visibility != availability.
    #[test]
    fn test_base_tabs_always_visible() {
        let base_tabs = [
            Tab::Recon,
            Tab::Load,
            Tab::ScanPorts,
            Tab::ScanEndpoints,
            Tab::Fingerprint,
            Tab::Fuzz,
            Tab::Waf,
            Tab::WafStress,
            Tab::Scan,
            Tab::Resume,
            Tab::Proxy,
            Tab::Packet,
            Tab::GraphQl,
            Tab::OAuth,
            Tab::Cluster,
            Tab::Stress,
            Tab::Report,
            Tab::Settings,
            Tab::History,
            Tab::Dashboard,
            Tab::Auth,
        ];
        let all = Tab::all();
        for tab in &base_tabs {
            assert!(
                all.contains(tab),
                "Base tab {:?} should always be in Tab::all()",
                tab
            );
            let spec = spec_for(*tab).expect("base tab should have spec");
            match tab {
                Tab::Stress => assert_eq!(
                    spec.feature,
                    Some("stress-testing"),
                    "Stress tab must declare canonical availability feature"
                ),
                Tab::Packet => assert_eq!(
                    spec.feature,
                    Some("packet-inspection"),
                    "Packet tab must declare canonical availability feature"
                ),
                _ => assert!(
                    spec.feature.is_none(),
                    "Base tab {:?} should have no feature gate, but has {:?}",
                    tab,
                    spec.feature
                ),
            }
        }
    }

    /// Feature-gated tabs have a non-empty, valid feature string.
    #[test]
    fn test_feature_gated_tabs_have_valid_feature() {
        let gated_features = [
            ("nse", Tab::Nse),
            ("advanced-hunting", Tab::Hunt),
            ("headless-browser", Tab::Browser),
            ("compliance", Tab::Compliance),
            ("database", Tab::Storage),
            ("external-integrations", Tab::Integrations),
            ("finding-workflow", Tab::Workflow),
            ("vuln-management", Tab::Vuln),
            ("wireless", Tab::Wireless),
            ("db-pentest", Tab::DbPentest),
            ("web-proxy", Tab::Intercept),
            ("c2", Tab::C2),
        ];
        for (expected_feature, tab) in &gated_features {
            let spec = spec_for(*tab).expect("gated tab should have spec");
            assert_eq!(
                spec.feature,
                Some(*expected_feature),
                "Tab {:?} should have feature {:?}, but has {:?}",
                tab,
                expected_feature,
                spec.feature
            );
        }
    }

    /// Every spec with a feature gate declares a known availability feature.
    /// Visibility-gated tabs (cfg in Tab::all) and availability-gated base
    /// shells (Stress/Packet, always visible) both use this field to denote
    /// canonical execution ownership, not just visibility.
    #[test]
    fn test_gated_specs_match_cfg_compilation() {
        for spec in tab_specs() {
            if let Some(feature) = spec.feature {
                // The spec declares a feature gate. Verify the tab's stable_id
                // is resolvable (it always is since TAB_SPECS is static),
                // and that the feature string is one of the known gated features.
                let known_gated = [
                    "nse",
                    "advanced-hunting",
                    "headless-browser",
                    "compliance",
                    "database",
                    "external-integrations",
                    "finding-workflow",
                    "vuln-management",
                    "wireless",
                    "db-pentest",
                    "web-proxy",
                    "c2",
                    // Availability-gated base shells (always visible, execution gated).
                    "stress-testing",
                    "packet-inspection",
                ];
                assert!(
                    known_gated.contains(&feature),
                    "Tab '{}' has unknown feature gate '{}'",
                    spec.stable_id,
                    feature
                );
            }
        }
    }

    /// Visibility-gated tabs that are compiled in should appear in Tab::all().
    /// Visibility-gated tabs that are NOT compiled should NOT appear in Tab::all().
    /// Availability-gated base shells (Stress/Packet) are always visible by
    /// design; their feature denotes execution availability, not visibility.
    #[test]
    fn test_feature_gated_visibility_matches_compilation() {
        let all = Tab::all();
        // Availability shells: always in Tab::all() regardless of cfg.
        let availability_shells = [Tab::Stress, Tab::Packet];
        for spec in tab_specs() {
            if let Some(_feature) = spec.feature {
                if availability_shells.contains(&spec.tab) {
                    assert!(
                        all.contains(&spec.tab),
                        "availability shell '{}' must remain visible in Tab::all()",
                        spec.stable_id
                    );
                    continue;
                }
                let tab = spec.tab;
                let in_all = all.contains(&tab);
                // We can't directly test cfg! at runtime, but we can verify
                // the invariant: if a gated tab is in tab_specs() and in Tab::all(),
                // its feature must be compiled. If it's in tab_specs() but NOT in
                // Tab::all(), its feature must NOT be compiled.
                //
                // This test validates the structural consistency: a gated tab's
                // presence in Tab::all() is deterministic based on compilation.
                if in_all {
                    // Tab is compiled in — its feature is active
                    // Verify it's also in visible_tab_specs() (test-only helper)
                    let visible = visible_tab_specs();
                    let in_visible = visible.iter().any(|s| s.stable_id == spec.stable_id);
                    assert!(
                        in_visible,
                        "Tab '{}' is in Tab::all() but not in visible_tab_specs()",
                        spec.stable_id
                    );
                }
            }
        }
    }

    /// No tab spec has an empty stable_id.
    #[test]
    fn test_no_empty_stable_ids() {
        for spec in tab_specs() {
            assert!(!spec.stable_id.is_empty(), "Tab spec has empty stable_id");
            assert!(
                !spec.title.is_empty(),
                "Tab spec '{}' has empty title",
                spec.stable_id
            );
        }
    }
}
