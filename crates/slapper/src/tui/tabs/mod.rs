
#![allow(dead_code)]

mod cluster;
mod dashboard;
mod fingerprint;
mod fuzz;
pub mod graphql;
mod history;
mod load;
#[cfg(feature = "nse")]
pub mod nse;
pub mod oauth;
pub mod packet;
#[cfg(feature = "python-plugins")]
mod plugin;
pub mod proxy;
pub mod recon;
mod report;
mod resume;
mod scan;
mod scan_endpoints;
mod scan_ports;
mod settings;
mod stress;
mod waf;
mod waf_stress;

pub use cluster::ClusterTab;
pub use dashboard::DashboardTab;
pub use fingerprint::FingerprintTab;
pub use fuzz::FuzzTab;
pub use graphql::GraphQlTab;
pub use history::HistoryTab;
pub use load::LoadTab;
#[cfg(feature = "nse")]
pub use nse::NseTab;
pub use oauth::OAuthTab;
pub use packet::PacketTab;
#[cfg(feature = "python-plugins")]
pub use plugin::PluginTab;
pub use proxy::ProxyTab;
pub use recon::ReconTab;
pub use report::ReportTab;
pub use resume::ResumeTab;
pub use scan::{ScanTab, StageStatus};
pub use scan_endpoints::ScanEndpointsTab;
pub use scan_ports::ScanPortsTab;
pub use settings::SettingsTab;
pub use stress::StressTab;
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
        }
    }

    pub fn all() -> &'static [Tab] {
        &[
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
            Tab::Nse,
            Tab::Plugin,
            Tab::Settings,
            Tab::History,
            Tab::Dashboard,
        ]
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
        for _ in 0..100 {
            self.handle_left();
        }
    }
    fn handle_end(&mut self) {
        for _ in 0..100 {
            self.handle_right();
        }
    }
    fn handle_top(&mut self) {
        for _ in 0..100 {
            self.handle_up();
        }
    }
    fn handle_bottom(&mut self) {
        for _ in 0..100 {
            self.handle_down();
        }
    }
    fn handle_tab(&mut self) {}
    fn handle_search(&mut self, _query: &str) {}
    fn is_input_focused(&self) -> bool;
    fn is_at_left_edge(&self) -> bool {
        true
    }
    fn is_at_right_edge(&self) -> bool {
        true
    }
}
