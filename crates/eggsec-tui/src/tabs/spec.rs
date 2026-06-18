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
        operation: Some("waf"),
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
        operation: Some("scan-pipeline"),
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
        feature: None,
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
        feature: None,
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
    cfg_push!(specs, [
        #[cfg(feature = "advanced-hunting")]     { spec_for(Tab::Hunt).unwrap() },
        #[cfg(feature = "compliance")]           { spec_for(Tab::Compliance).unwrap() },
        #[cfg(feature = "database")]             { spec_for(Tab::Storage).unwrap() },
        #[cfg(feature = "external-integrations")] { spec_for(Tab::Integrations).unwrap() },
        #[cfg(feature = "finding-workflow")]     { spec_for(Tab::Workflow).unwrap() },
        #[cfg(feature = "vuln-management")]      { spec_for(Tab::Vuln).unwrap() },
        #[cfg(feature = "nse")]                  { spec_for(Tab::Nse).unwrap() },
        #[cfg(feature = "headless-browser")]     { spec_for(Tab::Browser).unwrap() },
        #[cfg(feature = "wireless")]             { spec_for(Tab::Wireless).unwrap() },
        #[cfg(feature = "db-pentest")]           { spec_for(Tab::DbPentest).unwrap() },
        #[cfg(feature = "c2")]                   { spec_for(Tab::C2).unwrap() },
        #[cfg(feature = "web-proxy")]            { spec_for(Tab::Intercept).unwrap() },
    ]);
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
                        TabCategory::Assessment
                            | TabCategory::Traffic
                            | TabCategory::Configuration
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
        let all_variants: Vec<Tab> = (0..=32)
            .filter_map(|i| Tab::from_discriminant(i))
            .collect();
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
}
