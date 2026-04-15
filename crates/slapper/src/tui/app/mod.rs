pub(crate) mod dispatch;
pub(crate) mod command;
pub(crate) mod error;
pub(crate) mod export;
pub(crate) mod input;
pub(crate) mod navigation;
mod options;
pub(crate) mod runner;
pub(crate) mod state_update;
pub(crate) mod task_management;

pub use crate::tui::state::create_shared_history;
pub use input::InputMode;
pub use options::GlobalHttpOptions;
pub use runner::run;

use crossterm::event::KeyCode;
use super::error::make_friendly_error;
use crate::tui::help::{HelpManager, HelpOverlay, CommandPalette, HelpContext};
use crate::tui::state::SharedHistory;
use crate::tui::tabs;
use crate::tui::tabs::{Tab, TabInput, TabState};
use dispatch::TabDispatcher;
use crate::tui::workers;
use crate::types::OutputFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    ResetTab,
    SaveSettings,
    DeleteHistoryEntry,
    ClearHistory,
}

impl PendingAction {
    pub fn message(&self) -> (String, Vec<String>) {
        match self {
            PendingAction::ResetTab => (
                "Confirm Reset".to_string(),
                vec![
                    "Are you sure you want to reset this tab?".to_string(),
                    "All current input will be lost.".to_string(),
                ],
            ),
            PendingAction::SaveSettings => (
                "Confirm Save Settings".to_string(),
                vec![
                    "Are you sure you want to save settings?".to_string(),
                    "This will overwrite your configuration file.".to_string(),
                ],
            ),
            PendingAction::DeleteHistoryEntry => (
                "Confirm Delete".to_string(),
                vec![
                    "Are you sure you want to delete this history entry?".to_string(),
                    "This action cannot be undone.".to_string(),
                ],
            ),
            PendingAction::ClearHistory => (
                "Confirm Clear History".to_string(),
                vec![
                    "Are you sure you want to clear all history?".to_string(),
                    "This action cannot be undone.".to_string(),
                ],
            ),
        }
    }

    pub fn execute(self, app: &mut App) {
        match self {
            PendingAction::ResetTab => app.reset_current_tab(),
            PendingAction::SaveSettings => app.save_settings(),
            PendingAction::DeleteHistoryEntry => app.delete_history_entry(),
            PendingAction::ClearHistory => app.clear_all_history(),
        }
    }
}

pub struct App {
    pub current_tab: Tab,
    pub should_quit: bool,
    pub mode: InputMode,
    pub recon: tabs::ReconTab,
    pub load: tabs::LoadTab,
    pub scan_ports: tabs::ScanPortsTab,
    pub scan_endpoints: tabs::ScanEndpointsTab,
    pub fingerprint: tabs::FingerprintTab,
    pub fuzz: tabs::FuzzTab,
    pub waf: tabs::WafTab,
    pub waf_stress: tabs::WafStressTab,
    pub scan: tabs::ScanTab,
    pub resume: tabs::ResumeTab,
    pub proxy: tabs::ProxyTab,
    pub packet: tabs::PacketTab,
    pub graphql: tabs::GraphQlTab,
    pub oauth: tabs::OAuthTab,
    pub cluster: tabs::ClusterTab,
    pub stress: tabs::StressTab,
    pub report: tabs::ReportTab,
    #[cfg(feature = "nse")]
    pub nse: tabs::NseTab,
    #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
    pub plugin: tabs::PluginTab,
    pub settings: tabs::SettingsTab,
    pub http_options: GlobalHttpOptions,
    pub history: SharedHistory,
    pub show_help: bool,
    pub help_tab: Option<Tab>,
    pub show_http_options: bool,
    pub show_search: bool,
    pub search_query: String,
    pub search_backup: Option<std::collections::VecDeque<crate::tui::tabs::history::HistoryEntry>>,
    pub pending_key: Option<KeyCode>,
    pub dashboard: tabs::DashboardTab,
    #[cfg(feature = "advanced-hunting")]
    pub hunt: tabs::HuntTab,
    #[cfg(feature = "headless-browser")]
    pub browser: tabs::BrowserTab,
    #[cfg(feature = "compliance")]
    pub compliance: tabs::ComplianceTab,
    #[cfg(feature = "database")]
    pub storage: tabs::StorageTab,
    #[cfg(feature = "external-integrations")]
    pub integrations: tabs::IntegrationsTab,
    #[cfg(feature = "finding-workflow")]
    pub workflow: tabs::WorkflowTab,
    #[cfg(feature = "vuln-management")]
    pub vuln: tabs::VulnTab,
    pub export_format: OutputFormat,
    pub task_handle: Option<tokio::task::JoinHandle<()>>,
    pub progress_rx: Option<tokio::sync::mpsc::Receiver<(u64, u64)>>,
    pub result_rx: Option<tokio::sync::mpsc::Receiver<workers::TaskResult>>,
    pub help_manager: HelpManager,
    pub help_overlay: Option<HelpOverlay>,
    pub command_palette: Option<CommandPalette>,
    pub help_context: HelpContext,
    pub pending_action: Option<PendingAction>,
}

impl App {
    pub fn new(history: SharedHistory) -> Self {
        Self {
            current_tab: Tab::Recon,
            should_quit: false,
            mode: InputMode::Normal,
            recon: tabs::ReconTab::new(),
            load: tabs::LoadTab::new(),
            scan_ports: tabs::ScanPortsTab::new(),
            scan_endpoints: tabs::ScanEndpointsTab::new(),
            fingerprint: tabs::FingerprintTab::new(),
            fuzz: tabs::FuzzTab::new(),
            waf: tabs::WafTab::new(),
            waf_stress: tabs::WafStressTab::new(),
            scan: tabs::ScanTab::new(),
            resume: tabs::ResumeTab::new(),
            proxy: tabs::ProxyTab::new(),
            packet: tabs::PacketTab::new(),
            graphql: tabs::GraphQlTab::new(),
            oauth: tabs::OAuthTab::new(),
            cluster: tabs::ClusterTab::new(),
            stress: tabs::StressTab::new(),
            report: tabs::ReportTab::new(),
            #[cfg(feature = "nse")]
            nse: tabs::NseTab::new(),
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            plugin: tabs::PluginTab::new(),
            settings: tabs::SettingsTab::new(),
            dashboard: tabs::DashboardTab::new(),
            #[cfg(feature = "advanced-hunting")]
            hunt: tabs::HuntTab::new(),
            #[cfg(feature = "headless-browser")]
            browser: tabs::BrowserTab::new(),
            #[cfg(feature = "compliance")]
            compliance: tabs::ComplianceTab::new(),
            #[cfg(feature = "database")]
            storage: tabs::StorageTab::new(),
            #[cfg(feature = "external-integrations")]
            integrations: tabs::IntegrationsTab::new(),
            #[cfg(feature = "finding-workflow")]
            workflow: tabs::WorkflowTab::new(),
            #[cfg(feature = "vuln-management")]
            vuln: tabs::VulnTab::new(),
            http_options: GlobalHttpOptions::default(),
            history,
            show_help: false,
            help_tab: None,
            show_http_options: false,
            show_search: false,
            search_query: String::new(),
            search_backup: None,
            pending_key: None,
            export_format: OutputFormat::Json,
            task_handle: None,
            progress_rx: None,
            result_rx: None,
            help_manager: HelpManager::new(),
            help_overlay: None,
            command_palette: None,
            help_context: HelpContext::Normal,
            pending_action: None,
        }
    }

    pub fn cycle_export_format(&mut self) {
        self.export_format = match self.export_format {
            OutputFormat::Pretty => OutputFormat::Json,
            OutputFormat::Json => OutputFormat::Compact,
            OutputFormat::Compact => OutputFormat::Csv,
            OutputFormat::Csv => OutputFormat::Html,
            OutputFormat::Html => OutputFormat::Markdown,
            OutputFormat::Markdown => OutputFormat::Sarif,
            OutputFormat::Sarif => OutputFormat::Junit,
            OutputFormat::Junit => OutputFormat::Pretty,
        };
    }

    pub fn get_export_extension(&self) -> &str {
        match self.export_format {
            OutputFormat::Pretty => "txt",
            OutputFormat::Json => "json",
            OutputFormat::Compact => "json",
            OutputFormat::Csv => "csv",
            OutputFormat::Html => "html",
            OutputFormat::Markdown => "md",
            OutputFormat::Sarif => "sarif",
            OutputFormat::Junit => "xml",
        }
    }

    pub fn is_running(&self) -> bool {
        match self.current_tab {
            Tab::Recon => self.recon.is_running(),
            Tab::Load => self.load.is_running(),
            Tab::ScanPorts => self.scan_ports.is_running(),
            Tab::ScanEndpoints => self.scan_endpoints.is_running(),
            Tab::Fingerprint => self.fingerprint.is_running(),
            Tab::Fuzz => self.fuzz.is_running(),
            Tab::Waf => self.waf.is_running(),
            Tab::WafStress => self.waf_stress.is_running(),
            Tab::Scan => self.scan.is_running(),
            Tab::Resume => self.resume.is_running(),
            Tab::Proxy => self.proxy.is_running(),
            Tab::Packet => self.packet.is_running(),
            Tab::GraphQl => self.graphql.is_running(),
            Tab::OAuth => self.oauth.is_running(),
            Tab::Cluster => self.cluster.is_running(),
            Tab::Stress => self.stress.is_running(),
            Tab::Report => self.report.is_running(),
            #[cfg(feature = "nse")]
            Tab::Nse => self.nse.is_running(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => self.plugin.is_running(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => false,
            Tab::Settings => false,
            Tab::History => false,
            Tab::Dashboard => false,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.hunt.is_running(),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => false,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.browser.is_running(),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => false,
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.compliance.is_running(),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => false,
            #[cfg(feature = "database")]
            Tab::Storage => self.storage.is_running(),
            #[cfg(not(feature = "database"))]
            Tab::Storage => false,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => self.integrations.is_running(),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => false,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => self.workflow.is_running(),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => false,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => self.vuln.is_running(),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => false,
        }
    }

    fn dispatcher_mut(&mut self) -> TabDispatcher<'_> {
        let tab_input: &mut dyn TabInput = match self.current_tab {
            Tab::Recon => &mut self.recon,
            Tab::Load => &mut self.load,
            Tab::ScanPorts => &mut self.scan_ports,
            Tab::ScanEndpoints => &mut self.scan_endpoints,
            Tab::Fingerprint => &mut self.fingerprint,
            Tab::Fuzz => &mut self.fuzz,
            Tab::Waf => &mut self.waf,
            Tab::WafStress => &mut self.waf_stress,
            Tab::Scan => &mut self.scan,
            Tab::Resume => &mut self.resume,
            Tab::Proxy => &mut self.proxy,
            Tab::Packet => &mut self.packet,
            Tab::GraphQl => &mut self.graphql,
            Tab::OAuth => &mut self.oauth,
            Tab::Cluster => &mut self.cluster,
            Tab::Stress => &mut self.stress,
            Tab::Report => &mut self.report,
            #[cfg(feature = "nse")]
            Tab::Nse => &mut self.nse,
            #[cfg(not(feature = "nse"))]
            Tab::Nse => &mut self.dashboard,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => &mut self.plugin,
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => &mut self.dashboard,
            Tab::Settings => &mut self.settings,
            Tab::History => &mut self.dashboard,
            Tab::Dashboard => &mut self.dashboard,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => &mut self.hunt,
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => &mut self.dashboard,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => &mut self.browser,
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => &mut self.dashboard,
            #[cfg(feature = "compliance")]
            Tab::Compliance => &mut self.compliance,
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => &mut self.dashboard,
            #[cfg(feature = "database")]
            Tab::Storage => &mut self.storage,
            #[cfg(not(feature = "database"))]
            Tab::Storage => &mut self.dashboard,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => &mut self.integrations,
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => &mut self.dashboard,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => &mut self.workflow,
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => &mut self.dashboard,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => &mut self.vuln,
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => &mut self.dashboard,
        };
        TabDispatcher::new(tab_input)
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        self.dispatcher_mut().stop();
    }

    pub fn handle_enter(&mut self) {
        if self.show_help {
            self.show_help = false;
            return;
        }

        match self.current_tab {
            Tab::Recon => {
                self.recon.handle_enter();
                if self.recon.is_running() {
                    self.spawn_task(self.build_recon_task());
                }
            }
            Tab::Load => {
                self.load.handle_enter();
                if self.load.is_running() {
                    self.spawn_task(self.build_load_task());
                }
            }
            Tab::ScanPorts => {
                self.scan_ports.handle_enter();
                if self.scan_ports.is_running() {
                    self.spawn_task(self.build_port_scan_task());
                }
            }
            Tab::ScanEndpoints => {
                self.scan_endpoints.handle_enter();
                if self.scan_endpoints.is_running() {
                    self.spawn_task(self.build_endpoint_scan_task());
                }
            }
            Tab::Fingerprint => {
                self.fingerprint.handle_enter();
                if self.fingerprint.is_running() {
                    self.spawn_task(self.build_fingerprint_task());
                }
            }
            Tab::Fuzz => {
                self.fuzz.handle_enter();
                if self.fuzz.is_running() {
                    self.spawn_task(self.build_fuzz_task());
                }
            }
            Tab::Waf => {
                self.waf.handle_enter();
                if self.waf.is_running() {
                    self.spawn_task(self.build_waf_task());
                }
            }
            Tab::WafStress => {
                self.waf_stress.handle_enter();
                if self.waf_stress.is_running() {
                    self.spawn_task(self.build_waf_stress_task());
                }
            }
            Tab::Scan => {
                self.scan.handle_enter();
                if self.scan.is_running() {
                    self.spawn_task(self.build_pipeline_task());
                }
            }
            Tab::Resume => self.resume.handle_enter(),
            Tab::Proxy => self.proxy.handle_enter(),
            Tab::Packet => {
                self.packet.handle_enter();
                if self.packet.is_running() {
                    match self.packet.current_view {
                        tabs::packet::PacketView::Capture => {
                            self.spawn_task(self.build_packet_capture_task());
                        }
                        tabs::packet::PacketView::Traceroute => {
                            self.spawn_task(self.build_packet_traceroute_task());
                        }
                        tabs::packet::PacketView::Send => {
                            self.spawn_task(self.build_packet_send_task());
                        }
                        _ => {}
                    }
                }
            }
            Tab::GraphQl => self.graphql.handle_enter(),
            Tab::OAuth => self.oauth.handle_enter(),
            Tab::Cluster => self.cluster.handle_enter(),
            Tab::Stress => self.stress.handle_enter(),
            Tab::Report => self.report.handle_enter(),
            #[cfg(feature = "nse")]
            Tab::Nse => self.nse.handle_enter(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => {},
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => self.plugin.handle_enter(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => {},
            Tab::Settings => self.settings.handle_enter(),
            Tab::History => {}
            Tab::Dashboard => self.dashboard.handle_enter(),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => {
                self.hunt.handle_enter();
                if self.hunt.is_running() {
                    self.spawn_task(self.build_hunt_task());
                }
            }
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => {}
            #[cfg(feature = "headless-browser")]
            Tab::Browser => {
                self.browser.handle_enter();
                if self.browser.is_running() {
                    self.spawn_task(self.build_browser_task());
                }
            }
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => {},
            #[cfg(feature = "compliance")]
            Tab::Compliance => {
                self.compliance.handle_enter();
                if self.compliance.is_running() {
                    self.spawn_task(self.build_compliance_task());
                }
            }
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => {}
            #[cfg(feature = "database")]
            Tab::Storage => {
                self.storage.handle_enter();
                if self.storage.is_running() {
                    self.spawn_task(self.build_storage_task());
                }
            }
            #[cfg(not(feature = "database"))]
            Tab::Storage => {}
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => {
                self.integrations.handle_enter();
                if self.integrations.is_running() {
                    self.spawn_task(self.build_integrations_task());
                }
            }
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => {}
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => {
                self.workflow.handle_enter();
                if self.workflow.is_running() {
                    self.spawn_task(self.build_workflow_task());
                }
            }
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => {}
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => {
                self.vuln.handle_enter();
                if self.vuln.is_running() {
                    self.spawn_task(self.build_vuln_task());
                }
            }
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => {}
        }
    }

    pub fn handle_escape(&mut self) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_escape();
            }
            return;
        }
        self.dispatcher_mut().handle_escape();
    }

    pub fn handle_char(&mut self, c: char) {
        if self.show_help {
            return;
        }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_char(c);
            }
            return;
        }
        self.dispatcher_mut().handle_char(c);
    }

    pub fn handle_backspace(&mut self) {
        if self.show_help {
            return;
        }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_backspace();
            }
            return;
        }
        self.dispatcher_mut().handle_backspace();
    }

    pub fn handle_autocomplete(&mut self) -> bool {
        if self.show_help || self.mode != InputMode::Insert {
            return false;
        }

        match self.current_tab {
            Tab::History => false,
            Tab::Dashboard => false,
            _ => self.dispatcher_mut().handle_autocomplete(),
        }
    }

    pub fn handle_up(&mut self) {
        if self.show_help {
            return;
        }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_up();
            }
            return;
        }
        self.dispatcher_mut().handle_up();
    }

    pub fn handle_down(&mut self) {
        if self.show_help {
            return;
        }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_down();
            }
            return;
        }
        self.dispatcher_mut().handle_down();
    }

    pub fn handle_left(&mut self) {
        if self.show_help {
            return;
        }

        let moved = match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_left()
                } else {
                    false
                }
            }
            Tab::Recon => self.recon.handle_left(),
            Tab::Load => self.load.handle_left(),
            Tab::ScanPorts => self.scan_ports.handle_left(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_left(),
            Tab::Fingerprint => self.fingerprint.handle_left(),
            Tab::Fuzz => self.fuzz.handle_left(),
            Tab::Waf => self.waf.handle_left(),
            Tab::WafStress => self.waf_stress.handle_left(),
            Tab::Scan => self.scan.handle_left(),
            Tab::Resume => self.resume.handle_left(),
            Tab::Proxy => self.proxy.handle_left(),
            Tab::Packet => self.packet.handle_left(),
            Tab::GraphQl => self.graphql.handle_left(),
            Tab::OAuth => self.oauth.handle_left(),
            Tab::Cluster => self.cluster.handle_left(),
            Tab::Stress => self.stress.handle_left(),
            Tab::Report => self.report.handle_left(),
            #[cfg(feature = "nse")]
            Tab::Nse => self.nse.handle_left(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => self.plugin.handle_left(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => false,
            Tab::Settings => self.settings.handle_left(),
            Tab::Dashboard => self.dashboard.handle_left(),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.hunt.handle_left(),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => false,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.browser.handle_left(),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => false,
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.compliance.handle_left(),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => false,
            #[cfg(feature = "database")]
            Tab::Storage => self.storage.handle_left(),
            #[cfg(not(feature = "database"))]
            Tab::Storage => false,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => self.integrations.handle_left(),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => false,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => self.workflow.handle_left(),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => false,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => self.vuln.handle_left(),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => false,
        };

        if !moved {
            self.prev_tab();
        }
    }

    pub fn handle_right(&mut self) {
        if self.show_help {
            return;
        }

        let moved = match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_right()
                } else {
                    false
                }
            }
            Tab::Recon => self.recon.handle_right(),
            Tab::Load => self.load.handle_right(),
            Tab::ScanPorts => self.scan_ports.handle_right(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_right(),
            Tab::Fingerprint => self.fingerprint.handle_right(),
            Tab::Fuzz => self.fuzz.handle_right(),
            Tab::Waf => self.waf.handle_right(),
            Tab::WafStress => self.waf_stress.handle_right(),
            Tab::Scan => self.scan.handle_right(),
            Tab::Resume => self.resume.handle_right(),
            Tab::Proxy => self.proxy.handle_right(),
            Tab::Packet => self.packet.handle_right(),
            Tab::GraphQl => self.graphql.handle_right(),
            Tab::OAuth => self.oauth.handle_right(),
            Tab::Cluster => self.cluster.handle_right(),
            Tab::Stress => self.stress.handle_right(),
            Tab::Report => self.report.handle_right(),
            #[cfg(feature = "nse")]
            Tab::Nse => self.nse.handle_right(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => self.plugin.handle_right(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => false,
            Tab::Settings => self.settings.handle_right(),
            Tab::Dashboard => self.dashboard.handle_right(),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.hunt.handle_right(),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => false,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.browser.handle_right(),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => false,
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.compliance.handle_right(),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => false,
            #[cfg(feature = "database")]
            Tab::Storage => self.storage.handle_right(),
            #[cfg(not(feature = "database"))]
            Tab::Storage => false,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => self.integrations.handle_right(),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => false,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => self.workflow.handle_right(),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => false,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => self.vuln.handle_right(),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => false,
        };

        if !moved {
            self.next_tab();
        }
    }

    pub fn handle_focus_next(&mut self) {
        if self.show_help {
            return;
        }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_focus_next();
            }
            return;
        }
        self.dispatcher_mut().handle_focus_next();
    }

    pub fn handle_focus_prev(&mut self) {
        if self.show_help {
            return;
        }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_focus_prev();
            }
            return;
        }
        self.dispatcher_mut().handle_focus_prev();
    }

    pub fn handle_left_or_prev_tab(&mut self) -> bool {
        if self.show_help {
            return false;
        }
        let at_left_edge = match self.current_tab {
            Tab::Recon => self.recon.is_at_left_edge(),
            Tab::Load => self.load.is_at_left_edge(),
            Tab::ScanPorts => self.scan_ports.is_at_left_edge(),
            Tab::ScanEndpoints => self.scan_endpoints.is_at_left_edge(),
            Tab::Fingerprint => self.fingerprint.is_at_left_edge(),
            Tab::Fuzz => self.fuzz.is_at_left_edge(),
            Tab::Waf => self.waf.is_at_left_edge(),
            Tab::WafStress => self.waf_stress.is_at_left_edge(),
            Tab::Scan => self.scan.is_at_left_edge(),
            Tab::Resume => self.resume.is_at_left_edge(),
            Tab::Proxy => self.proxy.is_at_left_edge(),
            Tab::Packet => self.packet.is_at_left_edge(),
            Tab::GraphQl => self.graphql.is_at_left_edge(),
            Tab::OAuth => self.oauth.is_at_left_edge(),
            Tab::Cluster => self.cluster.is_at_left_edge(),
            Tab::Stress => self.stress.is_at_left_edge(),
            Tab::Report => self.report.is_at_left_edge(),
            #[cfg(feature = "nse")]
            Tab::Nse => self.nse.is_at_left_edge(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => self.plugin.is_at_left_edge(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => false,
            Tab::Settings => self.settings.is_at_left_edge(),
            Tab::History => true,
            Tab::Dashboard => true,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.hunt.is_at_left_edge(),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => false,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.browser.is_at_left_edge(),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => false,
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.compliance.is_at_left_edge(),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => false,
            #[cfg(feature = "database")]
            Tab::Storage => self.storage.is_at_left_edge(),
            #[cfg(not(feature = "database"))]
            Tab::Storage => false,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => self.integrations.is_at_left_edge(),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => false,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => self.workflow.is_at_left_edge(),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => false,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => self.vuln.is_at_left_edge(),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => false,
        };
        if at_left_edge {
            false
        } else {
            self.handle_left();
            true
        }
    }

    pub fn handle_right_or_next_tab(&mut self) -> bool {
        if self.show_help {
            return false;
        }
        let at_right_edge = match self.current_tab {
            Tab::Recon => self.recon.is_at_right_edge(),
            Tab::Load => self.load.is_at_right_edge(),
            Tab::ScanPorts => self.scan_ports.is_at_right_edge(),
            Tab::ScanEndpoints => self.scan_endpoints.is_at_right_edge(),
            Tab::Fingerprint => self.fingerprint.is_at_right_edge(),
            Tab::Fuzz => self.fuzz.is_at_right_edge(),
            Tab::Waf => self.waf.is_at_right_edge(),
            Tab::WafStress => self.waf_stress.is_at_right_edge(),
            Tab::Scan => self.scan.is_at_right_edge(),
            Tab::Resume => self.resume.is_at_right_edge(),
            Tab::Proxy => self.proxy.is_at_right_edge(),
            Tab::Packet => self.packet.is_at_right_edge(),
            Tab::GraphQl => self.graphql.is_at_right_edge(),
            Tab::OAuth => self.oauth.is_at_right_edge(),
            Tab::Cluster => self.cluster.is_at_right_edge(),
            Tab::Stress => self.stress.is_at_right_edge(),
            Tab::Report => self.report.is_at_right_edge(),
            #[cfg(feature = "nse")]
            Tab::Nse => self.nse.is_at_right_edge(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => self.plugin.is_at_right_edge(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => false,
            Tab::Settings => self.settings.is_at_right_edge(),
            Tab::History => true,
            Tab::Dashboard => true,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.hunt.is_at_right_edge(),
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => false,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.browser.is_at_right_edge(),
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => false,
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.compliance.is_at_right_edge(),
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => false,
            #[cfg(feature = "database")]
            Tab::Storage => self.storage.is_at_right_edge(),
            #[cfg(not(feature = "database"))]
            Tab::Storage => false,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => self.integrations.is_at_right_edge(),
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => false,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => self.workflow.is_at_right_edge(),
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => false,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => self.vuln.is_at_right_edge(),
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => false,
        };
        if at_right_edge {
            false
        } else {
            self.handle_right();
            true
        }
    }

    pub fn reset_current_tab(&mut self) {
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.clear_all();
            }
            return;
        }
        self.dispatcher_mut().reset();
    }

    pub fn save_settings(&mut self) {
        if self.current_tab == Tab::Settings {
            self.settings.save_config();
        }
    }

    pub fn delete_history_entry(&mut self) {
        if let Ok(mut h) = self.history.lock() {
            h.delete_selected();
        }
    }

    pub fn clear_all_history(&mut self) {
        if let Ok(mut h) = self.history.lock() {
            h.clear_all();
        }
    }

    pub fn request_confirmation(&mut self, action: PendingAction) {
        self.pending_action = Some(action);
    }

    pub fn confirm_action(&mut self) {
        if let Some(action) = self.pending_action.take() {
            action.execute(self);
        }
    }

    pub fn cancel_action(&mut self) {
        self.pending_action = None;
    }

    pub fn is_confirm_popup_visible(&self) -> bool {
        self.pending_action.is_some()
    }

    pub fn page_up(&mut self) {
        const PAGE_SIZE: usize = 10;
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.page_up(PAGE_SIZE);
            }
            return;
        }
        self.dispatcher_mut().page_up(PAGE_SIZE);
    }

    pub fn page_down(&mut self) {
        const PAGE_SIZE: usize = 10;
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.page_down(PAGE_SIZE);
            }
            return;
        }
        self.dispatcher_mut().page_down(PAGE_SIZE);
    }

    pub fn handle_word_forward(&mut self) {
        if self.show_help { return; }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_word_forward();
            }
            return;
        }
        self.dispatcher_mut().handle_word_forward();
    }

    pub fn handle_word_backward(&mut self) {
        if self.show_help { return; }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_word_backward();
            }
            return;
        }
        self.dispatcher_mut().handle_word_backward();
    }

    pub fn handle_home(&mut self) {
        if self.show_help { return; }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_home();
            }
            return;
        }
        self.dispatcher_mut().handle_home();
    }

    pub fn handle_end(&mut self) {
        if self.show_help { return; }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_end();
            }
            return;
        }
        self.dispatcher_mut().handle_end();
    }

    pub fn handle_top(&mut self) {
        if self.show_help { return; }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_top();
            }
            return;
        }
        self.dispatcher_mut().handle_top();
    }

    pub fn handle_bottom(&mut self) {
        if self.show_help { return; }
        if self.current_tab == Tab::History {
            if let Ok(mut h) = self.history.lock() {
                h.handle_bottom();
            }
            return;
        }
        self.dispatcher_mut().handle_bottom();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::tabs::Tab;
    use crossterm::event::KeyCode;

    fn create_test_app() -> App {
        App::new(create_shared_history())
    }

    #[test]
    fn test_app_new_has_default_values() {
        let app = create_test_app();
        assert_eq!(app.current_tab, Tab::Recon);
        assert!(!app.should_quit);
        assert_eq!(app.mode, InputMode::Normal);
        assert!(!app.show_help);
        assert!(!app.show_search);
        assert!(app.search_query.is_empty());
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn test_pending_action_message() {
        assert_eq!(
            PendingAction::ResetTab.message().0,
            "Confirm Reset"
        );
        assert_eq!(
            PendingAction::SaveSettings.message().0,
            "Confirm Save Settings"
        );
        assert_eq!(
            PendingAction::DeleteHistoryEntry.message().0,
            "Confirm Delete"
        );
        assert_eq!(
            PendingAction::ClearHistory.message().0,
            "Confirm Clear History"
        );
    }

    #[test]
    fn test_pending_action_message_has_details() {
        let (_, details) = PendingAction::ResetTab.message();
        assert!(!details.is_empty());
    }

    #[test]
    fn test_request_confirmation_sets_pending_action() {
        let mut app = create_test_app();
        assert!(app.pending_action.is_none());

        app.request_confirmation(PendingAction::ResetTab);
        assert!(app.pending_action.is_some());
        assert_eq!(app.pending_action, Some(PendingAction::ResetTab));
    }

    #[test]
    fn test_confirm_action_clears_pending_action() {
        let mut app = create_test_app();
        app.request_confirmation(PendingAction::ResetTab);
        assert!(app.pending_action.is_some());

        app.confirm_action();
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn test_cancel_action_clears_pending_action() {
        let mut app = create_test_app();
        app.request_confirmation(PendingAction::ResetTab);
        assert!(app.pending_action.is_some());

        app.cancel_action();
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn test_is_confirm_popup_visible() {
        let mut app = create_test_app();
        assert!(!app.is_confirm_popup_visible());

        app.request_confirmation(PendingAction::ResetTab);
        assert!(app.is_confirm_popup_visible());

        app.cancel_action();
        assert!(!app.is_confirm_popup_visible());
    }

    #[test]
    fn test_pending_key_set_and_cleared() {
        let mut app = create_test_app();
        assert!(app.pending_key.is_none());

        app.pending_key = Some(KeyCode::Char('a'));
        assert_eq!(app.pending_key, Some(KeyCode::Char('a')));

        app.pending_key = None;
        assert!(app.pending_key.is_none());
    }

    #[test]
    fn test_help_overlay_set_and_cleared() {
        let mut app = create_test_app();
        assert!(app.help_overlay.is_none());

        app.help_overlay = None;
        assert!(app.help_overlay.is_none());
    }

    #[test]
    fn test_search_query_set_and_cleared() {
        let mut app = create_test_app();
        assert!(app.search_query.is_empty());

        app.search_query = "test query".to_string();
        assert_eq!(app.search_query, "test query");

        app.search_query.clear();
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_show_http_options_toggle() {
        let mut app = create_test_app();
        assert!(!app.show_http_options);

        app.show_http_options = true;
        assert!(app.show_http_options);

        app.show_http_options = false;
        assert!(!app.show_http_options);
    }

    #[test]
    fn test_help_context_default() {
        let app = create_test_app();
        assert_eq!(app.help_context, crate::tui::help::HelpContext::Normal);
    }

    #[test]
    fn test_is_running_false_for_all_tabs_initially() {
        let mut app = create_test_app();

        app.current_tab = Tab::Recon;
        assert!(!app.is_running());

        app.current_tab = Tab::Load;
        assert!(!app.is_running());

        app.current_tab = Tab::ScanPorts;
        assert!(!app.is_running());

        app.current_tab = Tab::Settings;
        assert!(!app.is_running());

        app.current_tab = Tab::Dashboard;
        assert!(!app.is_running());
    }

    #[test]
    fn test_app_stop_clears_task_handle() {
        let mut app = create_test_app();
        app.task_handle = None;
        app.stop();
        assert!(app.task_handle.is_none());
    }

    #[test]
    fn test_export_format_default() {
        let app = create_test_app();
        assert_eq!(app.export_format, OutputFormat::Json);
    }
}
