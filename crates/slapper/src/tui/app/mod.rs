#[macro_use]
pub(crate) mod dispatch;
pub(crate) mod error;
pub mod input;
mod options;
mod runner;

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

    pub fn next_tab(&mut self) {
        self.current_tab = self.current_tab.next();
    }

    pub fn prev_tab(&mut self) {
        self.current_tab = self.current_tab.prev();
    }

    pub fn select_tab(&mut self, index: usize) {
        if let Some(tab) = Tab::from_index(index) {
            self.current_tab = tab;
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        if self.show_help {
            self.help_tab = Some(self.current_tab);
        } else {
            self.help_tab = None;
        }
    }

    pub fn toggle_search(&mut self) {
        if self.show_search {
            self.restore_search();
        }
        self.show_search = !self.show_search;
        if self.show_search {
            self.search_query.clear();
        }
    }

    pub fn perform_search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }

        if self.current_tab == Tab::History {
            let query = self.search_query.clone();
            if let Ok(mut h) = self.history.lock() {
                self.search_backup = Some(h.entries.clone());
                
                let results: Vec<_> = h.search(&query).into_iter().cloned().collect();
                h.entries.clear();
                for entry in results {
                    h.entries.push_front(entry);
                }
                if !h.entries.is_empty() {
                    h.selected = Some(0);
                    h.update_details_view();
                }
            }
        }
        
        self.show_search = false;
    }
    
    pub fn restore_search(&mut self) {
        if self.current_tab == Tab::History {
            if let Some(backup) = self.search_backup.take() {
                if let Ok(mut h) = self.history.lock() {
                    h.entries = backup;
                    h.selected = Some(0);
                    h.update_details_view();
                }
            }
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

    pub fn is_help_visible(&self) -> bool {
        self.show_help
    }

    pub fn get_current_help(&self) -> String {
        match self.current_tab {
            Tab::Recon => "Reconnaissance - Gather intelligence about target domain/IP.".to_string(),
            Tab::Load => "Load Testing - Send concurrent HTTP requests to test performance.".to_string(),
            Tab::ScanPorts => "Port Scanning - Discover open ports and services.".to_string(),
            Tab::ScanEndpoints => "Endpoint Discovery - Find hidden or sensitive endpoints.".to_string(),
            Tab::Fingerprint => "Service Fingerprinting - Identify services on open ports.".to_string(),
            Tab::Fuzz => "Fuzzing - Test for vulnerabilities using payloads.".to_string(),
            Tab::Waf => "WAF Detection - Detect and bypass Web Application Firewalls.".to_string(),
            Tab::WafStress => "WAF Stress Testing - Comprehensive WAF testing.".to_string(),
            Tab::Scan => "Pipeline Scanning - Run chained security assessment.".to_string(),
            Tab::Resume => "Session Resume - Continue previous scan from file.".to_string(),
            Tab::Proxy => "Proxy Management - Manage proxy pool.".to_string(),
            Tab::Packet => "Packet Tools - Capture, send, and analyze network packets.".to_string(),
            Tab::GraphQl => "GraphQL Security - Test GraphQL endpoints.".to_string(),
            Tab::OAuth => "OAuth/OIDC Security - Test OAuth endpoints.".to_string(),
            Tab::Cluster => "Cluster Management - Manage distributed scanning cluster.".to_string(),
            Tab::Stress => "Stress Testing - Run stress/load testing against target.".to_string(),
            Tab::Report => "Report - Convert and generate security scan reports.".to_string(),
            Tab::Nse => "NSE - Run Nmap NSE scripts.".to_string(),
            Tab::Plugin => "Plugins - Manage and run security scanning plugins.".to_string(),
            Tab::Settings => "Settings - Configure application options.".to_string(),
            Tab::History => "History - View previous scan results.".to_string(),
            Tab::Dashboard => "Dashboard - View scan results at a glance.".to_string(),
        }
    }

    pub fn is_running(&self) -> bool {
        dispatch_is_running!(self)
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        dispatch_stop!(self);
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
        dispatch_void_with_special!(self, handle_escape(), {});
    }

    pub fn handle_char(&mut self, c: char) {
        if self.show_help {
            return;
        }
        dispatch_void_with_special!(self, handle_char(c), {});
    }

    pub fn handle_backspace(&mut self) {
        if self.show_help {
            return;
        }
        dispatch_void_with_special!(self, handle_backspace(), {});
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
        dispatch_void_with_special!(self, handle_up(), {});
    }

    pub fn handle_down(&mut self) {
        if self.show_help {
            return;
        }
        dispatch_void_with_special!(self, handle_down(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_down();
            }
        });
    }

    pub fn handle_left(&mut self) {
        if self.show_help {
            return;
        }

        let moved = dispatch_bool_with_special!(self, handle_left(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_left()
            } else {
                false
            }
        });

        if !moved {
            self.prev_tab();
        }
    }

    pub fn handle_right(&mut self) {
        if self.show_help {
            return;
        }

        let moved = dispatch_bool_with_special!(self, handle_right(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_right()
            } else {
                false
            }
        });

        if !moved {
            self.next_tab();
        }
    }

    pub fn handle_focus_next(&mut self) {
        if self.show_help {
            return;
        }
        dispatch_void_with_special!(self, handle_focus_next(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_focus_next();
            }
        });
    }

    pub fn handle_focus_prev(&mut self) {
        if self.show_help {
            return;
        }
        dispatch_void_with_special!(self, handle_focus_prev(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_focus_prev();
            }
        });
    }

    pub fn handle_left_or_prev_tab(&mut self) -> bool {
        if self.show_help {
            return false;
        }
        let at_left_edge = dispatch_is_at_left_edge!(self);
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
        let at_right_edge = dispatch_is_at_right_edge!(self);
        if at_right_edge {
            false
        } else {
            self.handle_right();
            true
        }
    }

    pub fn reset_current_tab(&mut self) {
        dispatch_reset!(self);
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
        dispatch_void_with_special!(self, handle_word_forward(), {});
    }

    pub fn handle_word_backward(&mut self) {
        if self.show_help { return; }
        dispatch_void_with_special!(self, handle_word_backward(), {});
    }

    pub fn handle_home(&mut self) {
        if self.show_help { return; }
        dispatch_void_with_special!(self, handle_home(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_home();
            }
        });
    }

    pub fn handle_end(&mut self) {
        if self.show_help { return; }
        dispatch_void_with_special!(self, handle_end(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_end();
            }
        });
    }

    pub fn handle_top(&mut self) {
        if self.show_help { return; }
        dispatch_void_with_special!(self, handle_top(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_top();
            }
        });
    }

    pub fn handle_bottom(&mut self) {
        if self.show_help { return; }
        dispatch_void_with_special!(self, handle_bottom(), {
            if let Ok(mut h) = self.history.lock() {
                h.handle_bottom();
            }
        });
    }

    pub fn export_results(&mut self) {
        let ext = self.get_export_extension();
        let base_name = match self.current_tab {
            Tab::Recon => "recon_results",
            Tab::Load => "load_results",
            Tab::ScanPorts => "port_scan_results",
            Tab::ScanEndpoints => "endpoint_scan_results",
            Tab::Fingerprint => "fingerprint_results",
            Tab::Fuzz => "fuzz_results",
            Tab::Waf => "waf_results",
            Tab::WafStress => "waf_stress_results",
            Tab::Scan => "pipeline_scan_report",
            Tab::Resume => "resume_results",
            Tab::Proxy => "proxy_results",
            Tab::Packet => "packet_results",
            Tab::GraphQl => "graphql_results",
            Tab::OAuth => "oauth_results",
            Tab::Cluster => "cluster_status",
            Tab::Stress => "stress_results",
            Tab::Report => "report_results",
            Tab::Nse => "nse_results",
            Tab::Plugin => "plugin_results",
            Tab::Settings => "settings",
            Tab::History => "history",
            Tab::Dashboard => "dashboard",
        };
        
        let filename = format!("{}.{}", base_name, ext);

        match self.export_format {
            ExportFormat::Json => self.export_json(),
            ExportFormat::Csv => self.export_csv(&filename),
            ExportFormat::Html | ExportFormat::Markdown | ExportFormat::Sarif | ExportFormat::Junit => {
                self.export_json();
                self.export_converted(&filename);
            }
        }
    }

    fn export_json(&mut self) {
        match self.current_tab {
            Tab::Recon => {
                if let Some(results) = self.recon.get_results() {
                    self.save_export("recon_results.json", serde_json::to_string_pretty(results).unwrap_or_default());
                }
            }
            Tab::Load => {
                if let Some(results) = self.load.get_results() {
                    self.save_export("load_results.json", serde_json::to_string_pretty(results).unwrap_or_default());
                }
            }
            Tab::ScanPorts => {
                if let Some(results) = self.scan_ports.get_results() {
                    self.save_export("port_scan_results.json", serde_json::to_string_pretty(results).unwrap_or_default());
                }
            }
            Tab::ScanEndpoints => {
                if let Some(results) = self.scan_endpoints.get_results() {
                    self.save_export("endpoint_scan_results.json", serde_json::to_string_pretty(results).unwrap_or_default());
                }
            }
            Tab::Fingerprint => {
                if let Some(results) = self.fingerprint.get_results() {
                    self.save_export("fingerprint_results.json", serde_json::to_string_pretty(results).unwrap_or_default());
                }
            }
            Tab::Fuzz => {
                if let Some(results) = self.fuzz.get_results() {
                    self.save_export("fuzz_results.json", serde_json::to_string_pretty(results).unwrap_or_default());
                }
            }
            Tab::Waf => {
                if let Some(results) = self.waf.get_detection_result() {
                    self.save_export("waf_detection_results.json", serde_json::to_string_pretty(results).unwrap_or_default());
                }
                if let Some(results) = self.waf.get_bypass_results() {
                    self.save_export("waf_bypass_results.json", serde_json::to_string_pretty(results).unwrap_or_default());
                }
            }
            Tab::WafStress => {
                if let Some(results) = self.waf_stress.get_results() {
                    self.save_export("waf_stress_results.json", results);
                }
            }
            Tab::Scan => {
                if let Some(report) = self.scan.get_report() {
                    self.save_export("pipeline_scan_report.json", serde_json::to_string_pretty(report).unwrap_or_default());
                }
            }
            Tab::Resume => {}
            Tab::GraphQl => {}
            Tab::OAuth => {}
            Tab::Cluster => {}
            Tab::Stress => {}
            Tab::Report => {}
            Tab::Nse => {}
            Tab::Plugin => {}
            Tab::Settings => {}
            Tab::History => {
                if let Ok(h) = self.history.lock() {
                    let history_data = h.export();
                    self.save_export("history.json", history_data);
                }
            }
            Tab::Dashboard => {}
            Tab::Proxy => {}
            Tab::Packet => {}
        }
    }

    fn export_csv(&mut self, filename: &str) {
        use crate::output::csv::{CsvExporter, EndpointCsv, PortCsv};
        
        match self.current_tab {
            Tab::ScanPorts => {
                if let Some(results) = self.scan_ports.get_results() {
                    let ports: Vec<PortCsv> = results.open_ports.iter().map(|p| PortCsv {
                        host: results.host.clone(),
                        port: p.port,
                        protocol: "tcp".to_string(),
                        service: Some(p.service.clone()),
                        version: None,
                        state: "open".to_string(),
                    }).collect();
                    let csv = CsvExporter::export_ports(&ports);
                    self.save_export(filename, csv);
                }
            }
            Tab::ScanEndpoints => {
                if let Some(results) = self.scan_endpoints.get_results() {
                    let endpoints: Vec<EndpointCsv> = results.results.iter().map(|e| EndpointCsv {
                        url: format!("{}/{}", results.base_url, e.path),
                        method: "GET".to_string(),
                        status: e.status_code,
                        content_type: None,
                        content_length: e.content_length.unwrap_or(0),
                    }).collect();
                    let csv = CsvExporter::export_endpoints(&endpoints);
                    self.save_export(filename, csv);
                }
            }
            _ => {
                self.export_json();
            }
        }
    }

    fn export_converted(&mut self, filename: &str) {
        use crate::output::convert::load_scan_report;
        
        let base_name = filename.trim_end_matches(".html").trim_end_matches(".md")
            .trim_end_matches(".sarif").trim_end_matches(".junit")
            .trim_end_matches(".json");
        
        let json_filename = format!("{}.json", base_name);
        let json_path = format!("./exports/{}", json_filename);
        
        if let Ok(report) = load_scan_report(&json_path) {
            let converted = match self.export_format {
                ExportFormat::Html => crate::output::convert::convert_to_html(&report),
                ExportFormat::Markdown => crate::output::convert::convert_to_markdown(&report),
                ExportFormat::Sarif => crate::output::convert::convert_to_sarif(&report),
                ExportFormat::Junit => crate::output::convert::convert_to_junit(&report),
                _ => return,
            };
            self.save_export(filename, converted);
        }
    }

    fn save_export(&self, filename: &str, data: String) {
        use std::io::Write;
        
        let path = format!("./exports/{}", filename);
        let dir = std::path::Path::new("./exports");
        if !dir.exists() {
            let _ = std::fs::create_dir_all(dir);
        }
        
        let mut file = match std::fs::File::create(&path) {
            Ok(file) => file,
            Err(e) => {
                tracing::error!("Could not create export file: {}", e);
                return;
            }
        };
        
        if let Err(e) = file.write_all(data.as_bytes()) {
            tracing::error!("Could not write to export file: {}", e);
        } else {
            tracing::info!("Exported results to: {}", path);
        }
    }

    pub fn toggle_command_palette(&mut self) {
        if let Some(ref mut palette) = self.command_palette {
            palette.visible = !palette.visible;
            if palette.visible {
                palette.query.clear();
                palette.results = self.help_manager.get_command_palette_entries().clone();
                palette.selected_index = 0;
            }
        } else {
            let palette = CommandPalette {
                visible: true,
                query: String::new(),
                results: self.help_manager.get_command_palette_entries().clone(),
                selected_index: 0,
            };
            self.command_palette = Some(palette);
        }
    }

    pub fn update_command_palette_query(&mut self, query: &str) {
        if let Some(ref mut palette) = self.command_palette {
            palette.query = query.to_string();
            palette.results = self.help_manager.search_commands(query);
            palette.selected_index = 0;
        }
    }

    pub fn select_command_palette_item(&mut self, index: usize) {
        let command = if let Some(ref palette) = self.command_palette {
            palette.results.get(index).map(|r| r.command.clone())
        } else {
            None
        };

        if let Some(cmd) = command {
            self.execute_command(&cmd);
            if let Some(ref mut palette) = self.command_palette {
                palette.visible = false;
            }
        }
    }

    pub fn execute_command(&mut self, command: &str) {
        match command {
            "quit" | "exit" => {
                if !self.is_running() {
                    self.should_quit = true;
                }
            }
            "stop" => {
                self.stop();
            }
            "reset" => {
                self.reset_current_tab();
            }
            "save" => {
                self.save_settings();
            }
            "help" => {
                self.toggle_help();
            }
            "search" => {
                self.toggle_search();
            }
            "palette" => {
                self.toggle_command_palette();
            }
            "export" => {
                self.export_results();
            }
            "history" => {
                self.current_tab = Tab::History;
            }
            "settings" => {
                self.current_tab = Tab::Settings;
            }
            "dashboard" => {
                self.current_tab = Tab::Dashboard;
            }
            "recon" => {
                self.current_tab = Tab::Recon;
            }
            "load" => {
                self.current_tab = Tab::Load;
            }
            "ports" | "port" | "portscan" => {
                self.current_tab = Tab::ScanPorts;
            }
            "endpoints" | "endpoint" => {
                self.current_tab = Tab::ScanEndpoints;
            }
            "fingerprint" | "fingerprinting" => {
                self.current_tab = Tab::Fingerprint;
            }
            "fuzz" | "fuzzing" => {
                self.current_tab = Tab::Fuzz;
            }
            "waf" => {
                self.current_tab = Tab::Waf;
            }
            "wafstress" | "waf-stress" => {
                self.current_tab = Tab::WafStress;
            }
            "pipeline" | "scan" => {
                self.current_tab = Tab::Scan;
            }
            "resume" | "session" => {
                self.current_tab = Tab::Resume;
            }
            "next-tab" | "next" => {
                self.next_tab();
            }
            "prev-tab" | "previous" | "prev" => {
                self.prev_tab();
            }
            "page-up" => {
                self.page_up();
            }
            "page-down" => {
                self.page_down();
            }
            "http-options" | "http" => {
                self.show_http_options = !self.show_http_options;
            }
            _ => {}
        }
    }

    pub fn get_command_palette(&self) -> Option<&CommandPalette> {
        self.command_palette.as_ref()
    }

    pub fn get_command_palette_mut(&mut self) -> Option<&mut CommandPalette> {
        self.command_palette.as_mut()
    }

    pub fn set_help_context(&mut self, context: HelpContext) {
        self.help_context = context;
    }

    fn spawn_task(&mut self, config: Option<workers::TaskConfig>) {
        if let Some(config) = config {
            let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(100);
            let (result_tx, result_rx) = tokio::sync::mpsc::channel(1);
            
            let runner = workers::TaskRunner::new(config, progress_tx, result_tx.clone());
            let error_tx = result_tx.clone();
            
            self.progress_rx = Some(progress_rx);
            self.result_rx = Some(result_rx);
            
            self.task_handle = Some(tokio::spawn(async move {
                match runner.run().await {
                    Ok(_) => {}
                    Err(e) => {
                        let friendly_error = make_friendly_error(&e);
                        tracing::error!("Task failed: {}", friendly_error);
                        let _ = error_tx.send(workers::TaskResult::Error(friendly_error)).await;
                    }
                }
            }));
        }
    }

    fn build_recon_task(&self) -> Option<workers::TaskConfig> {
        let target = self.recon.target();
        if target.is_empty() { return None; }
        
        Some(workers::TaskConfig::Recon {
            target: target.to_string(),
            concurrency: self.recon.concurrency(),
            options: self.recon.get_options(),
        })
    }

    fn build_load_task(&self) -> Option<workers::TaskConfig> {
        let target = self.load.target();
        if target.is_empty() { return None; }
        
        if self.load.is_stress_test() {
            Some(workers::TaskConfig::StressTest {
                target: target.to_string(),
                stress_type: self.load.stress_type().to_string(),
                rate: self.load.requests(),
                duration: 60,
                concurrency: self.load.concurrency(),
            })
        } else {
            Some(workers::TaskConfig::LoadTest {
                target: target.to_string(),
                requests: self.load.requests(),
                concurrency: self.load.concurrency(),
                timeout: std::time::Duration::from_secs(self.load.timeout()),
            })
        }
    }

    fn build_port_scan_task(&self) -> Option<workers::TaskConfig> {
        let target = self.scan_ports.target();
        if target.is_empty() { return None; }
        
        Some(workers::TaskConfig::PortScan {
            target: target.to_string(),
            ports: self.scan_ports.ports().to_string(),
            concurrency: self.scan_ports.concurrency(),
            timeout: std::time::Duration::from_secs(self.scan_ports.timeout()),
        })
    }

    fn build_endpoint_scan_task(&self) -> Option<workers::TaskConfig> {
        let target = self.scan_endpoints.target();
        if target.is_empty() { return None; }
        
        Some(workers::TaskConfig::EndpointScan {
            target: target.to_string(),
            concurrency: self.scan_endpoints.concurrency(),
            timeout: std::time::Duration::from_secs(self.scan_endpoints.timeout()),
            wordlist: self.scan_endpoints.wordlist().map(|s| s.to_string()),
        })
    }

    fn build_fingerprint_task(&self) -> Option<workers::TaskConfig> {
        let target = self.fingerprint.target();
        if target.is_empty() { return None; }
        
        Some(workers::TaskConfig::Fingerprint {
            target: target.to_string(),
            ports: self.fingerprint.ports().to_string(),
            timeout: std::time::Duration::from_secs(self.fingerprint.timeout()),
        })
    }

    fn build_fuzz_task(&self) -> Option<workers::TaskConfig> {
        let target = self.fuzz.target();
        if target.is_empty() { return None; }
        
        Some(workers::TaskConfig::Fuzz {
            target: target.to_string(),
            payload_type: self.fuzz.payload_type_string(),
            mode: self.fuzz.mode().to_string(),
            mutations: self.fuzz.mutations_enabled(),
            mutation_count: self.fuzz.mutation_count(),
            method: self.fuzz.method().to_string(),
            param: self.fuzz.param().map(|s| s.to_string()),
            concurrency: self.fuzz.concurrency(),
            timeout: self.fuzz.timeout(),
            graphql_introspection: self.fuzz.graphql_introspection_enabled(),
            graphql_depth_bypass: self.fuzz.graphql_depth_bypass_enabled(),
            graphql_alias_overload: self.fuzz.graphql_alias_overload_enabled(),
            oauth_redirect_test: self.fuzz.oauth_redirect_enabled(),
            oauth_scope_test: self.fuzz.oauth_scope_enabled(),
            oauth_state_test: self.fuzz.oauth_state_enabled(),
            oauth_grant_test: self.fuzz.oauth_grant_enabled(),
        })
    }

    fn build_waf_task(&self) -> Option<workers::TaskConfig> {
        let target = self.waf.target();
        if target.is_empty() { return None; }
        
        Some(workers::TaskConfig::Waf {
            target: target.to_string(),
            bypass_mode: self.waf.is_bypass_mode(),
            techniques: self.waf.enabled_techniques(),
        })
    }

    fn build_waf_stress_task(&self) -> Option<workers::TaskConfig> {
        let target = self.waf_stress.target();
        if target.is_empty() { return None; }
        
        Some(workers::TaskConfig::Fuzz {
            target: target.to_string(),
            payload_type: "all".to_string(),
            mode: "Burst".to_string(),
            mutations: false,
            mutation_count: 0,
            method: "GET".to_string(),
            param: None,
            concurrency: self.waf_stress.concurrency(),
            timeout: self.waf_stress.timeout(),
            graphql_introspection: false,
            graphql_depth_bypass: false,
            graphql_alias_overload: false,
            oauth_redirect_test: false,
            oauth_scope_test: false,
            oauth_state_test: false,
            oauth_grant_test: false,
        })
    }

    fn build_pipeline_task(&self) -> Option<workers::TaskConfig> {
        let target = self.scan.target();
        if target.is_empty() { return None; }
        let profile = self.scan.profile()?;
        
        Some(workers::TaskConfig::Pipeline {
            target: target.to_string(),
            profile,
            output_file: String::new(),
            output_format: "json".to_string(),
        })
    }

    fn build_packet_capture_task(&self) -> Option<workers::TaskConfig> {
        let interface = self.packet.target();
        if interface.is_empty() { return None; }
        
        Some(workers::TaskConfig::PacketCapture {
            interface: interface.to_string(),
            filter: self.packet.filter().to_string(),
            max_packets: self.packet.max_packets(),
            output_file: self.packet.output_file().map(|s| s.to_string()),
        })
    }

    fn build_packet_traceroute_task(&self) -> Option<workers::TaskConfig> {
        let target = self.packet.target();
        if target.is_empty() { return None; }
        
        Some(workers::TaskConfig::PacketTraceroute {
            target: target.to_string(),
            max_hops: 30,
        })
    }

    fn build_packet_send_task(&self) -> Option<workers::TaskConfig> {
        let target = self.packet.target();
        if target.is_empty() { return None; }
        
        let port = self.packet.filter().parse().unwrap_or(80);
        let count = self.packet.max_packets() as u32;
        
        Some(workers::TaskConfig::PacketSend {
            target: target.to_string(),
            port,
            count,
            packet_size: 64,
        })
    }

    pub fn update(&mut self) {
        if let Some(ref mut rx) = self.progress_rx {
            use tokio::sync::mpsc;
            match rx.try_recv() {
                Ok((completed, total)) => {
                    match self.current_tab {
                        Tab::Recon => self.recon.update_progress(completed, total),
                        Tab::Load => self.load.update_progress(completed, total),
                        Tab::ScanPorts => self.scan_ports.update_progress(completed, total),
                        Tab::ScanEndpoints => self.scan_endpoints.update_progress(completed, total),
                        Tab::Fingerprint => self.fingerprint.update_progress(completed, total),
                        Tab::Fuzz => self.fuzz.update_progress(completed, total),
                        Tab::Waf => self.waf.update_progress(completed, total),
                        Tab::WafStress => self.waf_stress.update_progress(completed, total),
                        Tab::Scan => self.scan.update_progress(
                            self.scan.stages.iter().filter(|s| matches!(s.status, tabs::StageStatus::Completed)).count() as u64,
                            self.scan.stages.len() as u64
                        ),
                        _ => {}
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.progress_rx = None;
                }
            }
        }

        if let Some(ref mut rx) = self.result_rx {
            use tokio::sync::mpsc;
            match rx.try_recv() {
                Ok(result) => {
                    self.handle_result(result);
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.result_rx = None;
                }
            }
        }
    }

    fn handle_result(&mut self, result: workers::TaskResult) {
        match result {
            workers::TaskResult::LoadTest(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_load_test_result(
                        &r.target_url,
                        r.total_requests,
                        r.successful_requests,
                        r.failed_requests,
                        r.requests_per_second,
                        r.latency_mean_ms,
                    );
                }
                self.load.set_results(r);
            }
            #[cfg(feature = "stress-testing")]
            workers::TaskResult::StressTest { target, stats } => {
                let pps = if stats.duration_ms > 0 {
                    (stats.packets_sent * 1000) / stats.duration_ms
                } else {
                    0
                };
                if let Ok(mut h) = self.history.lock() {
                    h.add_load_test_result(
                        "stress-test",
                        stats.packets_sent,
                        stats.packets_sent.saturating_sub(stats.errors),
                        stats.errors,
                        pps as f64,
                        0.0,
                    );
                }
                self.load.set_stress_results(target.clone(), stats);
            }
            workers::TaskResult::PortScan(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_port_scan_result(
                        &r.host,
                        r.ports_scanned as usize,
                        r.open_ports.iter().map(|p| p.port).collect(),
                    );
                }
                self.scan_ports.set_results(r);
            }
            workers::TaskResult::EndpointScan(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_endpoint_scan_result(
                        &r.base_url,
                        r.endpoints_found,
                        r.interesting_findings,
                    );
                }
                self.scan_endpoints.set_results(r);
            }
            workers::TaskResult::Fingerprint(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_fingerprint_result(
                        &r.host,
                        r.services_identified,
                        r.results.iter().map(|fp| format!("{}: {}", fp.port, fp.service)).collect(),
                    );
                }
                self.fingerprint.set_results(r);
            }
            workers::TaskResult::WafDetection(r) => {
                let waf_name = r.waf_name.clone().unwrap_or_default();
                if let Ok(mut h) = self.history.lock() {
                    h.add_waf_result(
                        "<target>",
                        r.waf_name.is_some(),
                        &waf_name,
                        0,
                    );
                }
                self.waf.set_detection_result(r);
            }
            workers::TaskResult::WafBypass { detection, bypasses } => {
                let success_count = bypasses.iter().filter(|b| b.success).count();
                let waf_name = detection.waf_name.clone().unwrap_or_default();
                if let Ok(mut h) = self.history.lock() {
                    h.add_waf_result(
                        "<target>",
                        detection.waf_name.is_some(),
                        &waf_name,
                        success_count,
                    );
                }
                self.waf.set_detection_result(detection);
                self.waf.set_bypass_results(bypasses);
            }
            workers::TaskResult::Pipeline(r) => {
                let completed = r.stage_results.iter().filter(|s| s.success).count();
                if let Ok(mut h) = self.history.lock() {
                    h.add_pipeline_result(
                        &r.target,
                        completed,
                        r.stage_results.len(),
                        r.total_duration_ms,
                    );
                }
                self.scan.set_report(r);
            }
            workers::TaskResult::Fuzz(session) => {
                self.fuzz.set_results(session);
            }
            workers::TaskResult::Recon(r) => {
                if let Ok(mut h) = self.history.lock() {
                    h.add_recon_result(
                        &r.target,
                        r.domain.clone().unwrap_or_default(),
                        r.ip_address.clone().unwrap_or_default(),
                    );
                }
                self.recon.set_results(r);
            }
            workers::TaskResult::PacketCapture { packets_captured, output_file } => {
                self.packet.set_capture_results(packets_captured, output_file);
            }
            workers::TaskResult::PacketTraceroute { hops } => {
                self.packet.set_traceroute_results(hops);
            }
            workers::TaskResult::PacketSend { packets_sent, bytes_sent } => {
                self.packet.set_send_results(packets_sent, bytes_sent);
            }
            workers::TaskResult::GraphQl(r) => {
                self.graphql.set_results(r);
            }
            workers::TaskResult::OAuth(r) => {
                self.oauth.set_results(r);
            }
            #[cfg(feature = "nse")]
            workers::TaskResult::Nse(r) => {
                self.nse.set_results(r);
            }
            workers::TaskResult::Error(msg) => {
                match self.current_tab {
                    Tab::Recon => self.recon.set_error(msg),
                    Tab::Load => self.load.set_error(msg),
                    Tab::ScanPorts => self.scan_ports.set_error(msg),
                    Tab::ScanEndpoints => self.scan_endpoints.set_error(msg),
                    Tab::Fingerprint => self.fingerprint.set_error(msg),
                    Tab::Fuzz => self.fuzz.set_error(msg),
                    Tab::Waf => self.waf.set_error(msg),
                    Tab::WafStress => self.waf_stress.set_error(msg),
                    Tab::Scan => self.scan.set_error(msg),
                    Tab::Packet => self.packet.set_error(msg),
                    _ => {}
                }
            }
        }
    }
}
