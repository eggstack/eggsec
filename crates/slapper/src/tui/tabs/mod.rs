
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
mod plugin;
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
pub use plugin::PluginTab;
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
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Recon => "[1] Recon",
            Tab::Load => "[2] Load",
            Tab::ScanPorts => "[3] Scan Ports",
            Tab::ScanEndpoints => "[4] Scan Endpoints",
            Tab::Fingerprint => "[5] Fingerprint",
            Tab::Fuzz => "[6] Fuzz",
            Tab::Waf => "[7] WAF",
            Tab::WafStress => "[8] WAF Stress",
            Tab::Scan => "[9] Scan",
            Tab::Resume => "[10] Resume",
            Tab::Proxy => "[11] Proxy",
            Tab::Packet => "[12] Packet",
            Tab::GraphQl => "[13] GraphQL",
            Tab::OAuth => "[14] OAuth",
            Tab::Cluster => "[15] Cluster",
            Tab::Stress => "[16] Stress",
            Tab::Report => "[17] Report",
            Tab::Nse => "[18] NSE",
            Tab::Plugin => "[19] Plugins",
            Tab::Settings => "[20] Settings",
            Tab::History => "[21] History",
            Tab::Dashboard => "[22] Dashboard",
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => "[23] Hunt",
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => "[23] Hunt",
            #[cfg(feature = "headless-browser")]
            Tab::Browser => "[24] Browser",
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => "[24] Browser",
            #[cfg(feature = "compliance")]
            Tab::Compliance => "[25] Compliance",
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => "[25] Compliance",
            #[cfg(feature = "database")]
            Tab::Storage => "[26] Storage",
            #[cfg(not(feature = "database"))]
            Tab::Storage => "[26] Storage",
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => "[27] Integrations",
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => "[27] Integrations",
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => "[28] Workflow",
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => "[28] Workflow",
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => "[29] Vuln",
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => "[29] Vuln",
        }
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
pub trait TabInput {
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
}
