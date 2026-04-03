#[macro_use]
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

pub use input::InputMode;
pub use options::GlobalHttpOptions;
pub use runner::run;

use crossterm::event::KeyCode;
use super::error::make_friendly_error;
use crate::tui::help::{HelpManager, HelpOverlay, CommandPalette, HelpContext};
use crate::tui::state::SharedHistory;
use crate::tui::tabs;
use crate::tui::tabs::{Tab, TabInput, TabState};
use crate::tui::workers;
use crate::output::ExportFormat;

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
    pub export_format: ExportFormat,
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
            http_options: GlobalHttpOptions::default(),
            history,
            show_help: false,
            help_tab: None,
            show_http_options: false,
            show_search: false,
            search_query: String::new(),
            search_backup: None,
            pending_key: None,
            export_format: ExportFormat::Json,
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
            ExportFormat::Json => ExportFormat::Csv,
            ExportFormat::Csv => ExportFormat::Html,
            ExportFormat::Html => ExportFormat::Markdown,
            ExportFormat::Markdown => ExportFormat::Sarif,
            ExportFormat::Sarif => ExportFormat::Junit,
            ExportFormat::Junit => ExportFormat::Json,
        };
    }

    pub fn get_export_extension(&self) -> &str {
        match self.export_format {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Html => "html",
            ExportFormat::Markdown => "md",
            ExportFormat::Sarif => "sarif",
            ExportFormat::Junit => "xml",
        }
    }

    pub fn is_running(&self) -> bool {
        dispatch_bool!(self, is_running())
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        dispatch_void!(self, stop())
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
        }
    }

    pub fn handle_escape(&mut self) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        dispatch!(self, handle_escape(), {}, ())
    }

    pub fn handle_char(&mut self, c: char) {
        if self.show_help {
            return;
        }
        dispatch_void!(self, handle_char(c))
    }

    pub fn handle_backspace(&mut self) {
        if self.show_help {
            return;
        }
        dispatch_void!(self, handle_backspace())
    }

    pub fn handle_tab(&mut self) {
        if self.show_help || self.mode != InputMode::Insert {
            return;
        }

        match self.current_tab {
            Tab::Recon => self.recon.handle_tab(),
            Tab::Load => self.load.handle_tab(),
            Tab::ScanPorts => self.scan_ports.handle_tab(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_tab(),
            Tab::Fingerprint => self.fingerprint.handle_tab(),
            Tab::Fuzz => self.fuzz.handle_tab(),
            Tab::Waf => self.waf.handle_tab(),
            Tab::WafStress => self.waf_stress.handle_tab(),
            Tab::Scan => self.scan.handle_tab(),
            Tab::Resume => self.resume.handle_tab(),
            Tab::Proxy => self.proxy.handle_tab(),
            Tab::Packet => self.packet.handle_tab(),
            Tab::GraphQl => self.graphql.handle_focus_next(),
            Tab::OAuth => self.oauth.handle_focus_next(),
            Tab::Cluster => self.cluster.handle_focus_next(),
            Tab::Stress => self.stress.handle_focus_next(),
            Tab::Report => self.report.handle_focus_next(),
            #[cfg(feature = "nse")]
            Tab::Nse => self.nse.handle_focus_next(),
            #[cfg(not(feature = "nse"))]
            Tab::Nse => {},
            #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
            Tab::Plugin => self.plugin.handle_focus_next(),
            #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
            Tab::Plugin => {},
            Tab::Settings => self.settings.handle_tab(),
            Tab::History => {}
            Tab::Dashboard => {}
        }
    }

    pub fn handle_up(&mut self) {
        if self.show_help {
            return;
        }
        dispatch_void!(self, handle_up())
    }

    pub fn handle_down(&mut self) {
        if self.show_help {
            return;
        }
        match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_down();
                }
            }
            _ => dispatch_void!(self, handle_down()),
        }
    }

    pub fn handle_left(&mut self) {
        if self.show_help {
            return;
        }

        let moved = dispatch!(self, handle_left(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_left()
            } else {
                false
            }
        }, false);

        if !moved {
            self.prev_tab();
        }
    }

    pub fn handle_right(&mut self) {
        if self.show_help {
            return;
        }

        let moved = dispatch!(self, handle_right(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_right()
            } else {
                false
            }
        }, false);

        if !moved {
            self.next_tab();
        }
    }

    pub fn handle_focus_next(&mut self) {
        if self.show_help {
            return;
        }
        match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_focus_next();
                }
            }
            _ => dispatch_void!(self, handle_focus_next()),
        }
    }

    pub fn handle_focus_prev(&mut self) {
        if self.show_help {
            return;
        }
        match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_focus_prev();
                }
            }
            _ => dispatch_void!(self, handle_focus_prev()),
        }
    }

    pub fn handle_left_or_prev_tab(&mut self) -> bool {
        if self.show_help {
            return false;
        }
        let at_left_edge = dispatch_is_at_edge!(self, is_at_left_edge, false);
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
        let at_right_edge = dispatch_is_at_edge!(self, is_at_right_edge, false);
        if at_right_edge {
            false
        } else {
            self.handle_right();
            true
        }
    }

    pub fn reset_current_tab(&mut self) {
        match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.clear_all();
                }
            }
            _ => dispatch_reset!(self),
        }
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
        dispatch_page!(self, page_up, PAGE_SIZE);
    }

    pub fn page_down(&mut self) {
        const PAGE_SIZE: usize = 10;
        dispatch_page!(self, page_down, PAGE_SIZE);
    }

    pub fn handle_word_forward(&mut self) {
        if self.show_help { return; }
        dispatch_void!(self, handle_word_forward())
    }

    pub fn handle_word_backward(&mut self) {
        if self.show_help { return; }
        dispatch_void!(self, handle_word_backward())
    }

    pub fn handle_home(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_home();
                }
            }
            _ => dispatch_void!(self, handle_home()),
        }
    }

    pub fn handle_end(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_end();
                }
            }
            _ => dispatch_void!(self, handle_end()),
        }
    }

    pub fn handle_top(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_top();
                }
            }
            _ => dispatch_void!(self, handle_top()),
        }
    }

    pub fn handle_bottom(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_bottom();
                }
            }
            _ => dispatch_void!(self, handle_bottom()),
        }
    }
}
