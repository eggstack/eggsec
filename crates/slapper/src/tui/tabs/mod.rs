
mod cluster;
mod dashboard;
mod fingerprint;
mod fuzz;
pub mod graphql;
pub mod history;
#[cfg(feature = "advanced-hunting")]
pub mod hunt;
mod load;
#[cfg(feature = "nse")]
pub mod nse;
pub mod oauth;
pub mod packet;
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub mod plugin;
pub mod proxy;
pub mod recon;
mod report;
mod resume;
mod scan;
mod scan_endpoints;
mod scan_ports;
mod settings;
#[cfg(feature = "database")]
pub mod storage;
mod stress;
#[cfg(feature = "compliance")]
pub mod compliance;
#[cfg(feature = "external-integrations")]
pub mod integrations;
#[cfg(feature = "finding-workflow")]
pub mod workflow;
#[cfg(feature = "vuln-management")]
pub mod vuln;
#[cfg(feature = "headless-browser")]
pub mod browser;
mod waf;
mod waf_stress;

pub use cluster::ClusterTab;
pub use dashboard::DashboardTab;
pub use fingerprint::FingerprintTab;
pub use fuzz::FuzzTab;
pub use graphql::GraphQlTab;
pub use history::HistoryTab;
#[cfg(feature = "advanced-hunting")]
pub use hunt::HuntTab;
pub use load::LoadTab;
#[cfg(feature = "nse")]
pub use nse::NseTab;
pub use oauth::OAuthTab;
pub use packet::PacketTab;
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub use plugin::{PluginTab, PluginInfo};
pub use proxy::ProxyTab;
pub use recon::ReconTab;
pub use report::ReportTab;
pub use resume::ResumeTab;
pub use scan::{ScanTab, StageStatus};
pub use scan_endpoints::ScanEndpointsTab;
pub use scan_ports::ScanPortsTab;
pub use settings::SettingsTab;
#[cfg(feature = "database")]
pub use storage::StorageTab;
pub use stress::StressTab;
#[cfg(feature = "compliance")]
pub use compliance::ComplianceTab;
#[cfg(feature = "external-integrations")]
pub use integrations::IntegrationsTab;
#[cfg(feature = "finding-workflow")]
pub use workflow::WorkflowTab;
#[cfg(feature = "vuln-management")]
pub use vuln::VulnTab;
#[cfg(feature = "headless-browser")]
pub use browser::BrowserTab;
pub use waf::WafTab;
pub use waf_stress::WafStressTab;

use ratatui::{layout::Rect, Frame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Recon = 0,
    Load = 1,
    ScanPorts = 2,
    ScanEndpoints = 3,
    Fingerprint = 4,
    Fuzz = 5,
    Waf = 6,
    WafStress = 7,
    Scan = 8,
    Resume = 9,
    Proxy = 10,
    Packet = 11,
    GraphQl = 12,
    OAuth = 13,
    Cluster = 14,
    Stress = 15,
    Report = 16,
    Nse = 17,
    Plugin = 18,
    Settings = 19,
    History = 20,
    Dashboard = 21,
    Hunt = 22,
    Browser = 23,
    Compliance = 24,
    Storage = 25,
    Integrations = 26,
    Workflow = 27,
    Vuln = 28,
}

impl Tab {
    const TAB_TITLES: &'static [&'static str] = &[
        "[1] Recon",
        "[2] Load",
        "[3] Scan Ports",
        "[4] Scan Endpoints",
        "[5] Fingerprint",
        "[6] Fuzz",
        "[7] WAF",
        "[8] WAF Stress",
        "[9] Scan",
        "[0] Resume",
        "Proxy",
        "Packet",
        "GraphQL",
        "OAuth",
        "Cluster",
        "Stress",
        "Report",
        "NSE",
        "Plugins",
        "Settings",
        "History",
        "Dashboard",
        "Hunt",
        "Browser",
        "Compliance",
        "Storage",
        "Integrations",
        "Workflow",
        "Vuln",
    ];

    pub fn title(&self) -> &'static str {
        Self::TAB_TITLES[*self as usize]
    }

    pub fn cli_command(&self) -> &'static str {
        match self {
            Tab::Recon => "slapper recon",
            Tab::Load => "slapper load",
            Tab::ScanPorts => "slapper scan-ports",
            Tab::ScanEndpoints => "slapper scan-endpoints",
            Tab::Fingerprint => "slapper fingerprint",
            Tab::Fuzz => "slapper fuzz",
            Tab::Waf => "slapper waf",
            Tab::WafStress => "slapper waf-stress",
            Tab::Scan => "slapper scan",
            Tab::Resume => "slapper resume",
            Tab::Proxy => "slapper proxy",
            Tab::Packet => "slapper packet",
            Tab::GraphQl => "slapper graphql",
            Tab::OAuth => "slapper oauth",
            Tab::Cluster => "slapper cluster",
            Tab::Stress => "slapper stress",
            Tab::Report => "slapper report",
            Tab::Nse => "slapper nse",
            Tab::Plugin => "slapper plugin",
            Tab::Settings => "Settings",
            Tab::History => "History",
            Tab::Dashboard => "Dashboard",
            Tab::Hunt => "slapper hunt",
            Tab::Browser => "slapper browser",
            Tab::Compliance => "slapper compliance",
            Tab::Storage => "slapper storage",
            Tab::Integrations => "slapper integrations",
            Tab::Workflow => "slapper workflow",
            Tab::Vuln => "slapper vuln",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Tab::Recon => "Gather reconnaissance information",
            Tab::Load => "Run HTTP load test or stress test",
            Tab::ScanPorts => "Scan ports on target host",
            Tab::ScanEndpoints => "Discover sensitive HTTP endpoints",
            Tab::Fingerprint => "Fingerprint services (AMAP-style)",
            Tab::Fuzz => "Fuzz target with security payloads",
            Tab::Waf => "Detect and bypass Web Application Firewalls",
            Tab::WafStress => "Comprehensive WAF stress testing",
            Tab::Scan => "Run chained security assessment pipeline",
            Tab::Resume => "Resume a previous scan from session file",
            Tab::Proxy => "Manage proxy pool and health checks",
            Tab::Packet => "Packet capture, send, and analysis tools",
            Tab::GraphQl => "Test GraphQL endpoints for security issues",
            Tab::OAuth => "Test OAuth/OIDC endpoints for vulnerabilities",
            Tab::Cluster => "Manage distributed scanning cluster",
            Tab::Stress => "Run stress/load testing against target",
            Tab::Report => "Convert reports, analyze trends, manage schedules",
            Tab::Nse => "Run Nmap NSE scripts",
            Tab::Plugin => "Run security plugins against targets",
            Tab::Settings => "Application settings",
            Tab::History => "View scan history",
            Tab::Dashboard => "View scan results dashboard",
            Tab::Hunt => "Intelligent vulnerability hunting",
            Tab::Browser => "Headless browser security testing",
            Tab::Compliance => "Generate compliance reports (OWASP, PCI, HIPAA, SOC2)",
            Tab::Storage => "Database storage and query management",
            Tab::Integrations => "Issue tracker integration (Jira, GitHub, GitLab)",
            Tab::Workflow => "Finding management and SLA tracking",
            Tab::Vuln => "Vulnerability prioritization and risk scoring",
        }
    }

    pub fn all() -> &'static [Tab] {
        use std::sync::LazyLock;
        static TABS: LazyLock<Vec<Tab>> = LazyLock::new(|| {
            let tabs = vec![
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
            ];
            #[cfg(feature = "advanced-hunting")]
            let tabs = {
                let mut t = tabs;
                t.push(Tab::Hunt);
                t
            };
            #[cfg(feature = "compliance")]
            let tabs = {
                let mut t = tabs;
                t.push(Tab::Compliance);
                t
            };
            #[cfg(feature = "database")]
            let tabs = {
                let mut t = tabs;
                t.push(Tab::Storage);
                t
            };
            #[cfg(feature = "external-integrations")]
            let tabs = {
                let mut t = tabs;
                t.push(Tab::Integrations);
                t
            };
            #[cfg(feature = "finding-workflow")]
            let tabs = {
                let mut t = tabs;
                t.push(Tab::Workflow);
                t
            };
            #[cfg(feature = "vuln-management")]
            let tabs = {
                let mut t = tabs;
                t.push(Tab::Vuln);
                t
            };
            #[cfg(feature = "nse")]
            let tabs = {
                let mut t = tabs;
                t.push(Tab::Nse);
                t
            };
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            let tabs = {
                let mut t = tabs;
                t.push(Tab::Plugin);
                t
            };
            #[cfg(feature = "headless-browser")]
            let tabs = {
                let mut t = tabs;
                t.push(Tab::Browser);
                t
            };
            tabs
        });
        &TABS
    }

    pub fn from_index(index: usize) -> Option<Tab> {
        Self::all().get(index).copied()
    }

    pub fn visible_index(&self) -> Option<usize> {
        Self::all().iter().position(|t| t == self)
    }

    pub fn from_visible_index(index: usize) -> Option<Tab> {
        Self::from_index(index)
    }

    pub fn from_discriminant(discriminant: usize) -> Option<Tab> {
        match discriminant {
            0 => Some(Tab::Recon),
            1 => Some(Tab::Load),
            2 => Some(Tab::ScanPorts),
            3 => Some(Tab::ScanEndpoints),
            4 => Some(Tab::Fingerprint),
            5 => Some(Tab::Fuzz),
            6 => Some(Tab::Waf),
            7 => Some(Tab::WafStress),
            8 => Some(Tab::Scan),
            9 => Some(Tab::Resume),
            10 => Some(Tab::Proxy),
            11 => Some(Tab::Packet),
            12 => Some(Tab::GraphQl),
            13 => Some(Tab::OAuth),
            14 => Some(Tab::Cluster),
            15 => Some(Tab::Stress),
            16 => Some(Tab::Report),
            17 => Some(Tab::Nse),
            18 => Some(Tab::Plugin),
            19 => Some(Tab::Settings),
            20 => Some(Tab::History),
            21 => Some(Tab::Dashboard),
            22 => Some(Tab::Hunt),
            23 => Some(Tab::Browser),
            24 => Some(Tab::Compliance),
            25 => Some(Tab::Storage),
            26 => Some(Tab::Integrations),
            27 => Some(Tab::Workflow),
            28 => Some(Tab::Vuln),
            _ => None,
        }
    }

    pub fn stable_id(&self) -> &'static str {
        match self {
            Tab::Recon => "recon",
            Tab::Load => "load",
            Tab::ScanPorts => "scan_ports",
            Tab::ScanEndpoints => "scan_endpoints",
            Tab::Fingerprint => "fingerprint",
            Tab::Fuzz => "fuzz",
            Tab::Waf => "waf",
            Tab::WafStress => "waf_stress",
            Tab::Scan => "scan",
            Tab::Resume => "resume",
            Tab::Proxy => "proxy",
            Tab::Packet => "packet",
            Tab::GraphQl => "graphql",
            Tab::OAuth => "oauth",
            Tab::Cluster => "cluster",
            Tab::Stress => "stress",
            Tab::Report => "report",
            Tab::Nse => "nse",
            Tab::Plugin => "plugin",
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
        }
    }

    pub fn from_stable_id(id: &str) -> Option<Tab> {
        let tab = match id {
            "recon" => Tab::Recon,
            "load" => Tab::Load,
            "scan_ports" => Tab::ScanPorts,
            "scan_endpoints" => Tab::ScanEndpoints,
            "fingerprint" => Tab::Fingerprint,
            "fuzz" => Tab::Fuzz,
            "waf" => Tab::Waf,
            "waf_stress" => Tab::WafStress,
            "scan" => Tab::Scan,
            "resume" => Tab::Resume,
            "proxy" => Tab::Proxy,
            "packet" => Tab::Packet,
            "graphql" => Tab::GraphQl,
            "oauth" => Tab::OAuth,
            "cluster" => Tab::Cluster,
            "stress" => Tab::Stress,
            "report" => Tab::Report,
            "nse" => Tab::Nse,
            "plugin" => Tab::Plugin,
            "settings" => Tab::Settings,
            "history" => Tab::History,
            "dashboard" => Tab::Dashboard,
            "hunt" => Tab::Hunt,
            "browser" => Tab::Browser,
            "compliance" => Tab::Compliance,
            "storage" => Tab::Storage,
            "integrations" => Tab::Integrations,
            "workflow" => Tab::Workflow,
            "vuln" => Tab::Vuln,
            _ => return None,
        };
        tab.visible_index().and(Some(tab))
    }

    pub fn next(&self) -> Tab {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(&self) -> Tab {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        if idx == 0 {
            all[all.len() - 1]
        } else {
            all[idx - 1]
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TabWindow {
    pub start: usize,
    pub end: usize,
    pub selected_visible: usize,
    pub max_visible: usize,
    pub total_tabs: usize,
    pub has_prev: bool,
    pub has_next: bool,
}

#[derive(Debug, Clone)]
pub struct TabSpan {
    pub tab: Tab,
    pub global_index: usize,
    pub x_start: u16,
    pub x_end: u16,
}

impl TabWindow {
    pub fn for_width(term_width: u16, current_tab: Tab, previous_offset: u16) -> Self {
        let all_tabs = Tab::all();
        let total_tabs = all_tabs.len();

        let inner_width = (term_width as usize).saturating_sub(2);
        let range_text_len = Self::range_text_len(total_tabs, 0, total_tabs);
        let available_width = inner_width.saturating_sub(range_text_len + 2);

        let tab_widths: Vec<usize> = all_tabs.iter().map(|t| t.title().len()).collect();

        let mut max_visible = 0;
        let mut cum_width = 0;
        for (i, &w) in tab_widths.iter().enumerate() {
            cum_width += w;
            if cum_width > available_width && i > 0 {
                break;
            }
            max_visible = i + 1;
        }
        max_visible = max_visible.max(1).min(total_tabs);

        let current_idx = current_tab.visible_index().unwrap_or(0).min(total_tabs.saturating_sub(1));
        let previous_offset = previous_offset as usize;

        let clamped_offset = previous_offset.min(total_tabs.saturating_sub(max_visible));

        let start = if current_idx < clamped_offset {
            current_idx
        } else if current_idx >= clamped_offset + max_visible {
            current_idx + 1 - max_visible
        } else {
            clamped_offset
        };

        let start = start.min(total_tabs.saturating_sub(max_visible));
        let end = (start + max_visible).min(total_tabs);
        let selected_visible = current_idx.saturating_sub(start);

        let has_prev = start > 0;
        let has_next = end < total_tabs;

        Self {
            start,
            end,
            selected_visible,
            max_visible,
            total_tabs,
            has_prev,
            has_next,
        }
    }

    fn range_text_len(total_tabs: usize, start: usize, end: usize) -> usize {
        let range_text = if start > 0 || end < total_tabs {
            format!("[{}-{}/{}]", start + 1, end, total_tabs)
        } else {
            String::new()
        };
        range_text.len()
    }

    pub fn range_text(&self) -> String {
        if self.has_prev || self.has_next {
            format!("[{}-{}/{}]", self.start + 1, self.end, self.total_tabs)
        } else {
            String::new()
        }
    }

    pub fn visible_tab_spans(&self, term_width: u16) -> Vec<TabSpan> {
        let all_tabs = Tab::all();
        let inner_width = (term_width as usize).saturating_sub(2);

        if self.max_visible == 0 || self.end <= self.start {
            return Vec::new();
        }

        let tab_width = inner_width / self.max_visible;

        all_tabs[self.start..self.end]
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let global_index = self.start + i;
                let x_start = (i * tab_width) as u16;
                let x_end = ((i + 1) * tab_width) as u16;
                TabSpan {
                    tab: *tab,
                    global_index,
                    x_start,
                    x_end,
                }
            })
            .collect()
    }
}

impl Tab {
    pub fn as_tab_state<'a>(&self, app: &'a super::App) -> &'a dyn TabState {
        match self {
            Tab::Recon => &app.recon,
            Tab::Load => &app.load,
            Tab::ScanPorts => &app.scan_ports,
            Tab::ScanEndpoints => &app.scan_endpoints,
            Tab::Fingerprint => &app.fingerprint,
            Tab::Fuzz => &app.fuzz,
            Tab::Waf => &app.waf,
            Tab::WafStress => &app.waf_stress,
            Tab::Scan => &app.scan,
            Tab::Resume => &app.resume,
            Tab::Proxy => &app.proxy,
            Tab::Packet => &app.packet,
            Tab::GraphQl => &app.graphql,
            Tab::OAuth => &app.oauth,
            Tab::Cluster => &app.cluster,
            Tab::Stress => &app.stress,
            Tab::Report => &app.report,
            #[cfg(feature = "nse")]
            Tab::Nse => &app.nse,
            #[cfg(not(feature = "nse"))]
            Tab::Nse => &app.dashboard,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => &app.plugin,
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => &app.dashboard,
            Tab::Settings => &app.settings,
            Tab::History => &app.dashboard,
            Tab::Dashboard => &app.dashboard,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => &app.hunt,
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => &app.dashboard,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => &app.browser,
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => &app.dashboard,
            #[cfg(feature = "compliance")]
            Tab::Compliance => &app.compliance,
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => &app.dashboard,
            #[cfg(feature = "database")]
            Tab::Storage => &app.storage,
            #[cfg(not(feature = "database"))]
            Tab::Storage => &app.dashboard,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => &app.integrations,
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => &app.dashboard,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => &app.workflow,
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => &app.dashboard,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => &app.vuln,
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => &app.dashboard,
        }
    }

    const TAB_BREADCRUMBS: &'static [&'static str] = &[
        "Recon",
        "Load",
        "Scan Ports",
        "Scan Endpoints",
        "Fingerprint",
        "Fuzz",
        "WAF",
        "WAF Stress",
        "Scan",
        "Resume",
        "Proxy",
        "Packet",
        "GraphQL Security",
        "OAuth/OIDC Security",
        "Cluster Management",
        "Stress Testing",
        "Report",
        "NSE Scripts",
        "Plugins",
        "Settings",
        "History",
        "Dashboard",
        "Hunt",
        "Browser",
        "Compliance",
        "Storage",
        "Integrations",
        "Workflow",
        "Vuln",
    ];

    pub fn default_breadcrumb(&self) -> Vec<&'static str> {
        vec![Self::TAB_BREADCRUMBS[*self as usize]]
    }

    pub fn as_tab_state_mut<'a>(&mut self, app: &'a mut super::App) -> &'a mut dyn TabState {
        match self {
            Tab::Recon => &mut app.recon,
            Tab::Load => &mut app.load,
            Tab::ScanPorts => &mut app.scan_ports,
            Tab::ScanEndpoints => &mut app.scan_endpoints,
            Tab::Fingerprint => &mut app.fingerprint,
            Tab::Fuzz => &mut app.fuzz,
            Tab::Waf => &mut app.waf,
            Tab::WafStress => &mut app.waf_stress,
            Tab::Scan => &mut app.scan,
            Tab::Resume => &mut app.resume,
            Tab::Proxy => &mut app.proxy,
            Tab::Packet => &mut app.packet,
            Tab::GraphQl => &mut app.graphql,
            Tab::OAuth => &mut app.oauth,
            Tab::Cluster => &mut app.cluster,
            Tab::Stress => &mut app.stress,
            Tab::Report => &mut app.report,
            #[cfg(feature = "nse")]
            Tab::Nse => &mut app.nse,
            #[cfg(not(feature = "nse"))]
            Tab::Nse => &mut app.dashboard,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => &mut app.plugin,
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => &mut app.dashboard,
            Tab::Settings => &mut app.settings,
            Tab::History => &mut app.dashboard,
            Tab::Dashboard => &mut app.dashboard,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => &mut app.hunt,
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => &mut app.dashboard,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => &mut app.browser,
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => &mut app.dashboard,
            #[cfg(feature = "compliance")]
            Tab::Compliance => &mut app.compliance,
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => &mut app.dashboard,
            #[cfg(feature = "database")]
            Tab::Storage => &mut app.storage,
            #[cfg(not(feature = "database"))]
            Tab::Storage => &mut app.dashboard,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => &mut app.integrations,
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => &mut app.dashboard,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => &mut app.workflow,
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => &mut app.dashboard,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => &mut app.vuln,
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => &mut app.dashboard,
        }
    }

    pub fn as_tab_render<'a>(&self, app: &'a super::App) -> &'a dyn TabRender {
        match self {
            Tab::Recon => &app.recon,
            Tab::Load => &app.load,
            Tab::ScanPorts => &app.scan_ports,
            Tab::ScanEndpoints => &app.scan_endpoints,
            Tab::Fingerprint => &app.fingerprint,
            Tab::Fuzz => &app.fuzz,
            Tab::Waf => &app.waf,
            Tab::WafStress => &app.waf_stress,
            Tab::Scan => &app.scan,
            Tab::Resume => &app.resume,
            Tab::Proxy => &app.proxy,
            Tab::Packet => &app.packet,
            Tab::GraphQl => &app.graphql,
            Tab::OAuth => &app.oauth,
            Tab::Cluster => &app.cluster,
            Tab::Stress => &app.stress,
            Tab::Report => &app.report,
            #[cfg(feature = "nse")]
            Tab::Nse => &app.nse,
            #[cfg(not(feature = "nse"))]
            Tab::Nse => &app.dashboard,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => &app.plugin,
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => &app.dashboard,
            Tab::Settings => &app.settings,
            Tab::History => &app.dashboard,
            Tab::Dashboard => &app.dashboard,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => &app.hunt,
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => &app.dashboard,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => &app.browser,
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => &app.dashboard,
            #[cfg(feature = "compliance")]
            Tab::Compliance => &app.compliance,
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => &app.dashboard,
            #[cfg(feature = "database")]
            Tab::Storage => &app.storage,
            #[cfg(not(feature = "database"))]
            Tab::Storage => &app.dashboard,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => &app.integrations,
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => &app.dashboard,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => &app.workflow,
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => &app.dashboard,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => &app.vuln,
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => &app.dashboard,
        }
    }

    pub fn as_tab_input<'a>(&mut self, app: &'a mut super::App) -> &'a mut dyn TabInput {
        match self {
            Tab::Recon => &mut app.recon,
            Tab::Load => &mut app.load,
            Tab::ScanPorts => &mut app.scan_ports,
            Tab::ScanEndpoints => &mut app.scan_endpoints,
            Tab::Fingerprint => &mut app.fingerprint,
            Tab::Fuzz => &mut app.fuzz,
            Tab::Waf => &mut app.waf,
            Tab::WafStress => &mut app.waf_stress,
            Tab::Scan => &mut app.scan,
            Tab::Resume => &mut app.resume,
            Tab::Proxy => &mut app.proxy,
            Tab::Packet => &mut app.packet,
            Tab::GraphQl => &mut app.graphql,
            Tab::OAuth => &mut app.oauth,
            Tab::Cluster => &mut app.cluster,
            Tab::Stress => &mut app.stress,
            Tab::Report => &mut app.report,
            #[cfg(feature = "nse")]
            Tab::Nse => &mut app.nse,
            #[cfg(not(feature = "nse"))]
            Tab::Nse => &mut app.dashboard,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => &mut app.plugin,
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => &mut app.dashboard,
            Tab::Settings => &mut app.settings,
            Tab::History => &mut app.dashboard,
            Tab::Dashboard => &mut app.dashboard,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => &mut app.hunt,
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => &mut app.dashboard,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => &mut app.browser,
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => &mut app.dashboard,
            #[cfg(feature = "compliance")]
            Tab::Compliance => &mut app.compliance,
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => &mut app.dashboard,
            #[cfg(feature = "database")]
            Tab::Storage => &mut app.storage,
            #[cfg(not(feature = "database"))]
            Tab::Storage => &mut app.dashboard,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => &mut app.integrations,
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => &mut app.dashboard,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => &mut app.workflow,
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => &mut app.dashboard,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => &mut app.vuln,
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => &mut app.dashboard,
        }
    }
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Idle,
    Running,
    Completed,
    Error(String),
}

#[allow(dead_code)]
pub trait TabState {
    fn state(&self) -> AppState;
    fn progress(&self) -> f64;
    fn is_running(&self) -> bool {
        self.state() == AppState::Running
    }
    fn reset(&mut self);
    fn set_error(&mut self, _msg: String) {}
}

pub trait TabRender {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool);
    fn render_overlays(&self, _f: &mut Frame, _area: Rect) {}
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        None
    }
}

#[allow(dead_code)]
pub trait TabInput: TabState {
    fn handle_focus_next(&mut self);
    fn handle_focus_prev(&mut self);
    fn handle_char(&mut self, c: char);
    fn handle_backspace(&mut self);
    fn handle_enter(&mut self);
    fn handle_escape(&mut self);
    fn handle_up(&mut self);
    fn handle_down(&mut self);
    fn handle_left(&mut self) -> bool;
    fn handle_right(&mut self) -> bool;
    fn handle_paste(&mut self, _text: &str) {}
    fn handle_word_forward(&mut self) {
        for _ in 0..5 {
            self.handle_right();
        }
    }
    fn handle_word_backward(&mut self) {
        for _ in 0..5 {
            self.handle_left();
        }
    }
    fn handle_home(&mut self) {
        // Default: go to left edge - single step should suffice
        let _ = self.handle_left();
    }
    fn handle_end(&mut self) {
        // Default: go to right edge - single step should suffice
        let _ = self.handle_right();
    }
    fn handle_top(&mut self) {
        // Default: go to first item - single step should suffice
        self.handle_up();
    }
    fn handle_bottom(&mut self) {
        // Default: go to last item - single step should suffice
        self.handle_down();
    }
    fn handle_autocomplete(&mut self) -> bool {
        false
    }
    fn handle_search(&mut self, _query: &str) {}
    fn is_input_focused(&self) -> bool;
    fn is_at_left_edge(&self) -> bool {
        true
    }
    fn is_at_right_edge(&self) -> bool {
        true
    }
    fn stop(&mut self) {}
    fn page_up(&mut self, _page_size: usize) {}
    fn page_down(&mut self, _page_size: usize) {}
    fn reset(&mut self) {}
}
