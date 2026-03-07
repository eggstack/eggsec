#![allow(dead_code)]

mod components;
mod state;
mod tabs;
mod ui;
mod workers;
mod help;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::tui::help::{HelpManager, HelpOverlay, CommandPalette, HelpContext};
use crate::tui::state::SharedHistory;
use crate::tui::tabs::{Tab, TabInput, TabState};
use crate::output::ExportFormat;

fn make_friendly_error(error: &anyhow::Error) -> String {
    let error_str = error.to_string().to_lowercase();
    
    if error_str.contains("connection refused") {
        return "Connection refused. The target may be down or not accepting connections.".to_string();
    }
    if error_str.contains("timeout") || error_str.contains("timed out") {
        return "Request timed out. The target may be slow or unreachable.".to_string();
    }
    if error_str.contains("name or service not known") || error_str.contains("dns") {
        return "DNS resolution failed. Please check the target domain is correct.".to_string();
    }
    if error_str.contains("certificate") || error_str.contains("tls") || error_str.contains("ssl") {
        return "SSL/TLS error. The website may have certificate issues.".to_string();
    }
    if error_str.contains("permission denied") {
        return "Permission denied. Try running with elevated privileges.".to_string();
    }
    if error_str.contains("rate limit") || error_str.contains("429") {
        return "Rate limited. Too many requests. Please try again later.".to_string();
    }
    if error_str.contains("unauthorized") || error_str.contains("401") || error_str.contains("forbidden") {
        return "Unauthorized. Check your API keys in the configuration.".to_string();
    }
    if error_str.contains("not found") || error_str.contains("404") {
        return "Resource not found. The target may not exist.".to_string();
    }
    if error_str.contains("no route to host") || error_str.contains("network") {
        return "Network error. Check your internet connection.".to_string();
    }
    if error_str.contains("broken pipe") || error_str.contains("reset") {
        return "Connection broken. The remote host closed the connection.".to_string();
    }
    
    format!("Error: {}", error)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Insert,
}

impl std::fmt::Display for InputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputMode::Normal => write!(f, "NOR"),
            InputMode::Insert => write!(f, "INS"),
        }
    }
}

pub fn run(config_path: Option<String>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let history = state::create_shared_history();
    let mut app = App::new(history);
    if let Some(path) = config_path {
        app.settings.set_config_path(path);
    }
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn handle_mouse_event(mouse_event: MouseEvent, app: &mut App) {
    let MouseEventKind::Down(button) = mouse_event.kind else {
        return;
    };

    if button == MouseButton::Left {
        let tab_area = ratatui::layout::Rect {
            x: 0,
            y: 1,
            width: u16::MAX,
            height: 3,
        };

        if app.show_help || app.show_search || app.show_http_options {
            return;
        }

        if let Some(ref palette) = app.command_palette {
            if palette.visible {
                return;
            }
        }

        if tab_area.contains((mouse_event.column, mouse_event.row).into()) {
            let tab_width = tab_area.width / 15;
            let tab_index = (mouse_event.column.saturating_sub(1) / tab_width) as usize;
            if tab_index < 15 {
                app.select_tab(tab_index);
            }
        }
    }
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let Some(pending) = app.pending_key.take() {
                    match (key.modifiers, key.code, pending) {
                        (_, KeyCode::Char('g'), KeyCode::Char('g')) if app.mode == InputMode::Normal => {
                            app.handle_top();
                            continue;
                        }
                        _ => {}
                    }
                }

                match (key.modifiers, key.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        if app.is_running() {
                            app.stop();
                        } else {
                            return Ok(());
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                        app.page_up();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                        app.page_down();
                    }
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        if app.show_search {
                            app.toggle_search();
                        } else {
                            app.mode = InputMode::Normal;
                            app.handle_escape();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('i')) if app.mode == InputMode::Normal => {
                        app.mode = InputMode::Insert;
                    }
                    (KeyModifiers::NONE, KeyCode::Char('q')) if app.mode == InputMode::Normal => {
                        if !app.is_running() {
                            return Ok(());
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char(' ')) if app.mode == InputMode::Normal => {
                        app.toggle_help();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('/')) => {
                        app.toggle_help();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                        app.toggle_command_palette();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('/')) if app.mode == InputMode::Normal => {
                        app.toggle_command_palette();
                    }
                    _ if app.get_command_palette().map(&|cp: &CommandPalette| cp.visible).unwrap_or(false) => {
                        match (key.modifiers, key.code) {
                            (KeyModifiers::NONE, KeyCode::Esc) => {
                                app.toggle_command_palette();
                            }
                            (KeyModifiers::NONE, KeyCode::Enter) => {
                                let index = app.command_palette.as_ref().map(|p| p.selected_index).unwrap_or(0);
                                app.select_command_palette_item(index);
                            }
                            (KeyModifiers::NONE, KeyCode::Up) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    palette.selected_index = palette.selected_index.saturating_sub(1);
                                    if palette.selected_index >= palette.results.len() {
                                        palette.selected_index = palette.results.len().saturating_sub(1);
                                    }
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Down) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    palette.selected_index = (palette.selected_index + 1).min(palette.results.len().saturating_sub(1));
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Backspace) => {
                                let query = app.command_palette.as_ref().map(|p| p.query.clone()).unwrap_or_default();
                                if !query.is_empty() {
                                    if let Some(ref mut palette) = app.command_palette {
                                        palette.query.pop();
                                        let new_query = palette.query.clone();
                                        app.update_command_palette_query(&new_query);
                                    }
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    palette.query.push(c);
                                    let new_query = palette.query.clone();
                                    app.update_command_palette_query(&new_query);
                                }
                            }
                            (KeyModifiers::NONE, KeyCode::Tab) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    palette.selected_index = (palette.selected_index + 1).min(palette.results.len().saturating_sub(1));
                                }
                            }
                            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                                if let Some(ref mut palette) = app.command_palette {
                                    palette.selected_index = palette.selected_index.saturating_sub(1);
                                }
                            }
                            _ => {}
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Tab) => {
                        if app.mode == InputMode::Insert {
                            app.handle_tab();
                        } else {
                            app.handle_focus_next();
                        }
                    }
                    (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                        app.handle_focus_prev();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) if app.mode == InputMode::Normal => {
                        app.handle_left();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) if app.mode == InputMode::Normal => {
                        app.handle_down();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) if app.mode == InputMode::Normal => {
                        app.handle_up();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Right) if app.mode == InputMode::Normal => {
                        app.handle_right();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('H')) if app.mode == InputMode::Normal => {
                        app.handle_home();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('L')) if app.mode == InputMode::Normal => {
                        app.handle_end();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('G')) if app.mode == InputMode::Normal => {
                        app.handle_bottom();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('g')) if app.mode == InputMode::Normal => {
                        app.pending_key = Some(KeyCode::Char('g'));
                    }
                    (KeyModifiers::NONE, KeyCode::Char('w')) if app.mode == InputMode::Normal => {
                        app.handle_word_forward();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('b')) if app.mode == InputMode::Normal => {
                        app.handle_word_backward();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('n')) | (KeyModifiers::NONE, KeyCode::Char('N')) if app.mode == InputMode::Normal => {
                        if key.code == KeyCode::Char('n') {
                            app.next_tab();
                        } else {
                            app.prev_tab();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('g')) => {
                        app.handle_bottom();
                    }
                    (KeyModifiers::NONE, KeyCode::Backspace) => {
                        app.handle_backspace();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) if app.mode == InputMode::Normal => {
                        app.handle_left();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) if app.mode == InputMode::Normal => {
                        app.handle_down();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) if app.mode == InputMode::Normal => {
                        app.handle_up();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Right) if app.mode == InputMode::Normal => {
                        app.handle_right();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('H')) if app.mode == InputMode::Normal => {
                        app.handle_home();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('L')) if app.mode == InputMode::Normal => {
                        app.handle_end();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('G')) if app.mode == InputMode::Normal => {
                        app.handle_bottom();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('w')) if app.mode == InputMode::Normal => {
                        app.handle_word_forward();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('b')) if app.mode == InputMode::Normal => {
                        app.handle_word_backward();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('n')) | (KeyModifiers::NONE, KeyCode::Char('N')) if app.mode == InputMode::Normal => {
                        if key.code == KeyCode::Char('n') {
                            app.next_tab();
                        } else {
                            app.prev_tab();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('p')) if app.mode == InputMode::Normal => {
                        app.prev_tab();
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char('H')) if app.mode == InputMode::Normal => {
                        app.prev_tab();
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char('L')) if app.mode == InputMode::Normal => {
                        app.next_tab();
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char('E')) if app.mode == InputMode::Normal => {
                        app.cycle_export_format();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('/')) if app.mode == InputMode::Normal => {
                        app.toggle_search();
                    }
                    (KeyModifiers::NONE, KeyCode::Char('r')) if app.mode == InputMode::Normal => {
                        if !app.is_running() {
                            app.reset_current_tab();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('s')) if app.mode == InputMode::Normal => {
                        if !app.is_running() {
                            app.save_settings();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char('d')) if app.mode == InputMode::Normal => {
                        if !app.is_running() && app.current_tab == Tab::History {
                            app.delete_history_entry();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        if app.show_search {
                            app.perform_search();
                        } else {
                            app.handle_enter();
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Char(c)) if app.show_search => {
                        app.search_query.push(c);
                    }
                    (KeyModifiers::NONE, KeyCode::Char(c)) if app.mode == InputMode::Insert => {
                        app.handle_char(c);
                    }
                    _ => {}
                }
            }

            if let Event::Mouse(mouse_event) = event::read()? {
                handle_mouse_event(mouse_event, app);
            }
        }

        app.update();
    }
}

#[derive(Clone, Default)]
pub struct GlobalHttpOptions {
    pub insecure: bool,
    pub proxy: Option<String>,
    pub proxy_auth: Option<String>,
    pub auth: Option<String>,
    pub bearer: Option<String>,
    pub cookie: Option<String>,
    pub api_key: Option<String>,
    pub user_agent: Option<String>,
    pub stealth: bool,
    pub rate_limit: Option<u32>,
    pub jitter: Option<String>,
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
    pub settings: tabs::SettingsTab,
    pub http_options: GlobalHttpOptions,
    pub history: SharedHistory,
    pub show_help: bool,
    pub help_tab: Option<Tab>,
    pub show_http_options: bool,
    pub show_search: bool,
    pub search_query: String,
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
            settings: tabs::SettingsTab::new(),
            dashboard: tabs::DashboardTab::new(),
            http_options: GlobalHttpOptions::default(),
            history,
            show_help: false,
            help_tab: None,
            show_http_options: false,
            show_search: false,
            search_query: String::new(),
            pending_key: None,
            export_format: ExportFormat::Json,
            task_handle: None,
            progress_rx: None,
            result_rx: None,
            help_manager: HelpManager::new(),
            help_overlay: None,
            command_palette: None,
            help_context: HelpContext::Normal,
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
        let tab = self.current_tab;
        match tab {
            Tab::Recon => "Reconnaissance - Gather intelligence about target domain/IP. Use Enter to start reconnaissance after entering target.".to_string(),
            Tab::Load => "Load Testing - Send concurrent HTTP requests to test performance. Enter target URL and press Enter to start.".to_string(),
            Tab::ScanPorts => "Port Scanning - Discover open ports and services. Enter host and press Enter to scan.".to_string(),
            Tab::ScanEndpoints => "Endpoint Discovery - Find hidden or sensitive endpoints using wordlists. Enter base URL and press Enter to scan.".to_string(),
            Tab::Fingerprint => "Service Fingerprinting - Identify services on open ports. Enter host and press Enter to fingerprint.".to_string(),
            Tab::Fuzz => "Fuzzing - Test for vulnerabilities using payloads. Enter target and press Enter to fuzz.".to_string(),
            Tab::Waf => "WAF Detection - Detect and bypass Web Application Firewalls. Enter target and press Enter to test.".to_string(),
            Tab::WafStress => "WAF Stress Testing - Comprehensive WAF testing with all payloads. Enter target and press Enter to stress test.".to_string(),
            Tab::Scan => "Pipeline Scanning - Run chained security assessment. Enter target and press Enter to run full pipeline.".to_string(),
            Tab::Resume => "Session Resume - Continue previous scan from file. Enter session file path and press Enter to resume.".to_string(),
            Tab::Proxy => "Proxy Management - Manage proxy pool. Select view and press Enter.".to_string(),
            Tab::Packet => "Packet Tools - Capture, send, and analyze network packets".to_string(),
            Tab::Settings => "Settings - Configure application options. Press 's' to save changes.".to_string(),
            Tab::History => "History - View previous scan results. Use arrow keys to navigate and 'd' to delete entries.".to_string(),
            Tab::Dashboard => "Dashboard - View scan results at a glance".to_string(),
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
            Tab::Settings => false,
            Tab::History => false,
            Tab::Dashboard => false,
        }
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }

        match self.current_tab {
            Tab::Recon => self.recon.stop(),
            Tab::Load => self.load.stop(),
            Tab::ScanPorts => self.scan_ports.stop(),
            Tab::ScanEndpoints => self.scan_endpoints.stop(),
            Tab::Fingerprint => self.fingerprint.stop(),
            Tab::Fuzz => self.fuzz.stop(),
            Tab::Waf => self.waf.stop(),
            Tab::WafStress => self.waf_stress.stop(),
            Tab::Scan => self.scan.stop(),
            Tab::Resume => self.resume.stop(),
            Tab::Proxy => self.proxy.stop(),
            Tab::Packet => self.packet.stop(),
            Tab::Settings => {}
            Tab::History => {}
            Tab::Dashboard => {}
        }
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

        match self.current_tab {
            Tab::Recon => self.recon.handle_escape(),
            Tab::Load => self.load.handle_escape(),
            Tab::ScanPorts => self.scan_ports.handle_escape(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_escape(),
            Tab::Fingerprint => self.fingerprint.handle_escape(),
            Tab::Fuzz => self.fuzz.handle_escape(),
            Tab::Waf => self.waf.handle_escape(),
            Tab::WafStress => self.waf_stress.handle_escape(),
            Tab::Scan => self.scan.handle_escape(),
            Tab::Resume => self.resume.handle_escape(),
            Tab::Proxy => self.proxy.handle_escape(),
            Tab::Packet => self.packet.handle_escape(),
            Tab::Settings => self.settings.handle_escape(),
            Tab::History => {}
            Tab::Dashboard => self.dashboard.handle_escape(),
        }
    }

    pub fn handle_char(&mut self, c: char) {
        if self.show_help {
            return;
        }

        match self.current_tab {
            Tab::Recon => self.recon.handle_char(c),
            Tab::Load => self.load.handle_char(c),
            Tab::ScanPorts => self.scan_ports.handle_char(c),
            Tab::ScanEndpoints => self.scan_endpoints.handle_char(c),
            Tab::Fingerprint => self.fingerprint.handle_char(c),
            Tab::Fuzz => self.fuzz.handle_char(c),
            Tab::Waf => self.waf.handle_char(c),
            Tab::WafStress => self.waf_stress.handle_char(c),
            Tab::Scan => self.scan.handle_char(c),
            Tab::Resume => self.resume.handle_char(c),
            Tab::Proxy => self.proxy.handle_char(c),
            Tab::Packet => self.packet.handle_char(c),
            Tab::Settings => self.settings.handle_char(c),
            Tab::History => {}
            Tab::Dashboard => self.dashboard.handle_char(c),
        }
    }

    pub fn handle_backspace(&mut self) {
        if self.show_help {
            return;
        }

        match self.current_tab {
            Tab::Recon => self.recon.handle_backspace(),
            Tab::Load => self.load.handle_backspace(),
            Tab::ScanPorts => self.scan_ports.handle_backspace(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_backspace(),
            Tab::Fingerprint => self.fingerprint.handle_backspace(),
            Tab::Fuzz => self.fuzz.handle_backspace(),
            Tab::Waf => self.waf.handle_backspace(),
            Tab::WafStress => self.waf_stress.handle_backspace(),
            Tab::Scan => self.scan.handle_backspace(),
            Tab::Resume => self.resume.handle_backspace(),
            Tab::Proxy => self.proxy.handle_backspace(),
            Tab::Packet => self.packet.handle_backspace(),
            Tab::Settings => self.settings.handle_backspace(),
            Tab::History => {}
            Tab::Dashboard => self.dashboard.handle_backspace(),
        }
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
            Tab::Settings => self.settings.handle_tab(),
            Tab::History => {}
            Tab::Dashboard => {}
        }
    }

    pub fn handle_up(&mut self) {
        if self.show_help {
            return;
        }

        match self.current_tab {
            Tab::Recon => self.recon.handle_up(),
            Tab::Load => self.load.handle_up(),
            Tab::ScanPorts => self.scan_ports.handle_up(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_up(),
            Tab::Fingerprint => self.fingerprint.handle_up(),
            Tab::Fuzz => self.fuzz.handle_up(),
            Tab::Waf => self.waf.handle_up(),
            Tab::WafStress => self.waf_stress.handle_up(),
            Tab::Scan => self.scan.handle_up(),
            Tab::Resume => self.resume.handle_up(),
            Tab::Proxy => self.proxy.handle_up(),
            Tab::Packet => self.packet.handle_up(),
            Tab::Settings => self.settings.handle_up(),
            Tab::History => {}
            Tab::Dashboard => {}
        }
    }

    pub fn handle_down(&mut self) {
        if self.show_help {
            return;
        }

        match self.current_tab {
            Tab::Recon => self.recon.handle_down(),
            Tab::Load => self.load.handle_down(),
            Tab::ScanPorts => self.scan_ports.handle_down(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_down(),
            Tab::Fingerprint => self.fingerprint.handle_down(),
            Tab::Fuzz => self.fuzz.handle_down(),
            Tab::Waf => self.waf.handle_down(),
            Tab::WafStress => self.waf_stress.handle_down(),
            Tab::Scan => self.scan.handle_down(),
            Tab::Resume => self.resume.handle_down(),
            Tab::Proxy => self.proxy.handle_down(),
            Tab::Packet => self.packet.handle_down(),
            Tab::Settings => self.settings.handle_down(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_down();
                }
            }
            Tab::Dashboard => self.dashboard.handle_down(),
        }
    }

    pub fn handle_left(&mut self) {
        if self.show_help {
            return;
        }

        let moved = match self.current_tab {
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
            Tab::Settings => self.settings.handle_left(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_left()
                } else {
                    false
                }
            }
            Tab::Dashboard => self.dashboard.handle_left(),
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
            Tab::Settings => self.settings.handle_right(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_right()
                } else {
                    false
                }
            }
            Tab::Dashboard => self.dashboard.handle_right(),
        };

        if !moved {
            self.next_tab();
        }
    }

    pub fn handle_focus_next(&mut self) {
        if self.show_help {
            return;
        }

        match self.current_tab {
            Tab::Recon => self.recon.handle_focus_next(),
            Tab::Load => self.load.handle_focus_next(),
            Tab::ScanPorts => self.scan_ports.handle_focus_next(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_focus_next(),
            Tab::Fingerprint => self.fingerprint.handle_focus_next(),
            Tab::Fuzz => self.fuzz.handle_focus_next(),
            Tab::Waf => self.waf.handle_focus_next(),
            Tab::WafStress => self.waf_stress.handle_focus_next(),
            Tab::Scan => self.scan.handle_focus_next(),
            Tab::Resume => self.resume.handle_focus_next(),
            Tab::Proxy => self.proxy.handle_focus_next(),
            Tab::Packet => self.packet.handle_focus_next(),
            Tab::Settings => self.settings.handle_focus_next(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_focus_next();
                }
            }
            Tab::Dashboard => self.dashboard.handle_focus_next(),
        }
    }

    pub fn handle_focus_prev(&mut self) {
        if self.show_help {
            return;
        }

        match self.current_tab {
            Tab::Recon => self.recon.handle_focus_prev(),
            Tab::Load => self.load.handle_focus_prev(),
            Tab::ScanPorts => self.scan_ports.handle_focus_prev(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_focus_prev(),
            Tab::Fingerprint => self.fingerprint.handle_focus_prev(),
            Tab::Fuzz => self.fuzz.handle_focus_prev(),
            Tab::Waf => self.waf.handle_focus_prev(),
            Tab::WafStress => self.waf_stress.handle_focus_prev(),
            Tab::Scan => self.scan.handle_focus_prev(),
            Tab::Resume => self.resume.handle_focus_prev(),
            Tab::Proxy => self.proxy.handle_focus_prev(),
            Tab::Packet => self.packet.handle_focus_prev(),
            Tab::Settings => self.settings.handle_focus_prev(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_focus_prev();
                }
            }
            Tab::Dashboard => self.dashboard.handle_focus_prev(),
        }
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
            Tab::Settings => self.settings.is_at_left_edge(),
            Tab::History => true,
            Tab::Dashboard => true,
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
            Tab::Settings => self.settings.is_at_right_edge(),
            Tab::History => true,
            Tab::Dashboard => true,
        };
        if at_right_edge {
            false
        } else {
            self.handle_right();
            true
        }
    }

    pub fn reset_current_tab(&mut self) {
        match self.current_tab {
            Tab::Recon => self.recon.reset(),
            Tab::Load => self.load.reset(),
            Tab::ScanPorts => self.scan_ports.reset(),
            Tab::ScanEndpoints => self.scan_endpoints.reset(),
            Tab::Fingerprint => self.fingerprint.reset(),
            Tab::Fuzz => self.fuzz.reset(),
            Tab::Waf => self.waf.reset(),
            Tab::WafStress => self.waf_stress.reset(),
            Tab::Scan => self.scan.reset(),
            Tab::Resume => self.resume.reset(),
            Tab::Proxy => self.proxy.reset(),
            Tab::Packet => self.packet.reset(),
            Tab::Settings => self.settings.reset(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.clear_all();
                }
            }
            Tab::Dashboard => self.dashboard.reset(),
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

    pub fn page_up(&mut self) {
        const PAGE_SIZE: usize = 10;
        match self.current_tab {
            Tab::Recon => self.recon.page_up(PAGE_SIZE),
            Tab::Load => self.load.page_up(PAGE_SIZE),
            Tab::ScanPorts => self.scan_ports.page_up(PAGE_SIZE),
            Tab::ScanEndpoints => self.scan_endpoints.page_up(PAGE_SIZE),
            Tab::Fingerprint => self.fingerprint.page_up(PAGE_SIZE),
            Tab::Fuzz => self.fuzz.page_up(PAGE_SIZE),
            Tab::Waf => self.waf.page_up(PAGE_SIZE),
            Tab::WafStress => self.waf_stress.page_up(PAGE_SIZE),
            Tab::Scan => self.scan.page_up(PAGE_SIZE),
            Tab::Resume => self.resume.page_up(PAGE_SIZE),
            Tab::Proxy => self.proxy.page_up(PAGE_SIZE),
            Tab::Packet => self.packet.page_up(PAGE_SIZE),
            Tab::Settings => {}
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.page_up(PAGE_SIZE);
                }
            }
            Tab::Dashboard => self.dashboard.page_up(PAGE_SIZE),
        }
    }

    pub fn page_down(&mut self) {
        const PAGE_SIZE: usize = 10;
        match self.current_tab {
            Tab::Recon => self.recon.page_down(PAGE_SIZE),
            Tab::Load => self.load.page_down(PAGE_SIZE),
            Tab::ScanPorts => self.scan_ports.page_down(PAGE_SIZE),
            Tab::ScanEndpoints => self.scan_endpoints.page_down(PAGE_SIZE),
            Tab::Fingerprint => self.fingerprint.page_down(PAGE_SIZE),
            Tab::Fuzz => self.fuzz.page_down(PAGE_SIZE),
            Tab::Waf => self.waf.page_down(PAGE_SIZE),
            Tab::WafStress => self.waf_stress.page_down(PAGE_SIZE),
            Tab::Scan => self.scan.page_down(PAGE_SIZE),
            Tab::Resume => self.resume.page_down(PAGE_SIZE),
            Tab::Proxy => self.proxy.page_down(PAGE_SIZE),
            Tab::Packet => self.packet.page_down(PAGE_SIZE),
            Tab::Settings => {}
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.page_down(PAGE_SIZE);
                }
            }
            Tab::Dashboard => self.dashboard.page_down(PAGE_SIZE),
        }
    }

    pub fn handle_word_forward(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::Recon => self.recon.handle_word_forward(),
            Tab::Load => self.load.handle_word_forward(),
            Tab::ScanPorts => self.scan_ports.handle_word_forward(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_word_forward(),
            Tab::Fingerprint => self.fingerprint.handle_word_forward(),
            Tab::Fuzz => self.fuzz.handle_word_forward(),
            Tab::Waf => self.waf.handle_word_forward(),
            Tab::WafStress => self.waf_stress.handle_word_forward(),
            Tab::Scan => self.scan.handle_word_forward(),
            Tab::Resume => self.resume.handle_word_forward(),
            Tab::Proxy => self.proxy.handle_word_forward(),
            Tab::Packet => self.packet.handle_word_forward(),
            Tab::Settings => {}
            Tab::History => {}
            Tab::Dashboard => self.dashboard.handle_word_forward(),
        }
    }

    pub fn handle_word_backward(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::Recon => self.recon.handle_word_backward(),
            Tab::Load => self.load.handle_word_backward(),
            Tab::ScanPorts => self.scan_ports.handle_word_backward(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_word_backward(),
            Tab::Fingerprint => self.fingerprint.handle_word_backward(),
            Tab::Fuzz => self.fuzz.handle_word_backward(),
            Tab::Waf => self.waf.handle_word_backward(),
            Tab::WafStress => self.waf_stress.handle_word_backward(),
            Tab::Scan => self.scan.handle_word_backward(),
            Tab::Resume => self.resume.handle_word_backward(),
            Tab::Proxy => self.proxy.handle_word_backward(),
            Tab::Packet => self.packet.handle_word_backward(),
            Tab::Settings => {}
            Tab::History => {}
            Tab::Dashboard => self.dashboard.handle_word_backward(),
        }
    }

    pub fn handle_home(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::Recon => self.recon.handle_home(),
            Tab::Load => self.load.handle_home(),
            Tab::ScanPorts => self.scan_ports.handle_home(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_home(),
            Tab::Fingerprint => self.fingerprint.handle_home(),
            Tab::Fuzz => self.fuzz.handle_home(),
            Tab::Waf => self.waf.handle_home(),
            Tab::WafStress => self.waf_stress.handle_home(),
            Tab::Scan => self.scan.handle_home(),
            Tab::Resume => self.resume.handle_home(),
            Tab::Proxy => self.proxy.handle_home(),
            Tab::Packet => self.packet.handle_home(),
            Tab::Settings => self.settings.handle_home(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_home();
                }
            }
            Tab::Dashboard => self.dashboard.handle_home(),
        }
    }

    pub fn handle_end(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::Recon => self.recon.handle_end(),
            Tab::Load => self.load.handle_end(),
            Tab::ScanPorts => self.scan_ports.handle_end(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_end(),
            Tab::Fingerprint => self.fingerprint.handle_end(),
            Tab::Fuzz => self.fuzz.handle_end(),
            Tab::Waf => self.waf.handle_end(),
            Tab::WafStress => self.waf_stress.handle_end(),
            Tab::Scan => self.scan.handle_end(),
            Tab::Resume => self.resume.handle_end(),
            Tab::Proxy => self.proxy.handle_end(),
            Tab::Packet => self.packet.handle_end(),
            Tab::Settings => self.settings.handle_end(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_end();
                }
            }
            Tab::Dashboard => self.dashboard.handle_end(),
        }
    }

    pub fn handle_top(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::Recon => self.recon.handle_top(),
            Tab::Load => self.load.handle_top(),
            Tab::ScanPorts => self.scan_ports.handle_top(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_top(),
            Tab::Fingerprint => self.fingerprint.handle_top(),
            Tab::Fuzz => self.fuzz.handle_top(),
            Tab::Waf => self.waf.handle_top(),
            Tab::WafStress => self.waf_stress.handle_top(),
            Tab::Scan => self.scan.handle_top(),
            Tab::Resume => self.resume.handle_top(),
            Tab::Proxy => self.proxy.handle_top(),
            Tab::Packet => self.packet.handle_top(),
            Tab::Settings => self.settings.handle_top(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_top();
                }
            }
            Tab::Dashboard => self.dashboard.handle_top(),
        }
    }

    pub fn handle_bottom(&mut self) {
        if self.show_help { return; }
        match self.current_tab {
            Tab::Recon => self.recon.handle_bottom(),
            Tab::Load => self.load.handle_bottom(),
            Tab::ScanPorts => self.scan_ports.handle_bottom(),
            Tab::ScanEndpoints => self.scan_endpoints.handle_bottom(),
            Tab::Fingerprint => self.fingerprint.handle_bottom(),
            Tab::Fuzz => self.fuzz.handle_bottom(),
            Tab::Waf => self.waf.handle_bottom(),
            Tab::WafStress => self.waf_stress.handle_bottom(),
            Tab::Scan => self.scan.handle_bottom(),
            Tab::Resume => self.resume.handle_bottom(),
            Tab::Proxy => self.proxy.handle_bottom(),
            Tab::Packet => self.packet.handle_bottom(),
            Tab::Settings => self.settings.handle_bottom(),
            Tab::History => {
                if let Ok(mut h) = self.history.lock() {
                    h.handle_bottom();
                }
            }
            Tab::Dashboard => self.dashboard.handle_bottom(),
        }
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
            Tab::Settings => "settings",
            Tab::History => "history",
            Tab::Dashboard => "dashboard",
        };
        
        let filename = format!("{}.{}", base_name, ext);

        match self.export_format {
            ExportFormat::Json => self.export_json(),
            ExportFormat::Csv => self.export_csv(&filename),
            ExportFormat::Html => self.export_json(),
            ExportFormat::Markdown => self.export_json(),
            ExportFormat::Sarif => self.export_json(),
            ExportFormat::Junit => self.export_json(),
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
            Tab::Resume => {
                // No results to export for resume tab
            }
            Tab::Settings => {
                // No results to export for settings tab
            }
            Tab::History => {
                if let Ok(h) = self.history.lock() {
                    let history_data = h.export();
                    self.save_export("history.json", history_data);
                }
            }
            Tab::Dashboard => {
                // No results to export for dashboard tab
            }
            Tab::Proxy => {
                // No results to export for proxy tab
            }
            Tab::Packet => {
                // No results to export for packet tab
            }
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

    fn save_export(&self, filename: &str, data: String) {
        
        use std::io::Write;
        
        let path = format!("./exports/{}", filename);
        let dir = std::path::Path::new("./exports");
        if !dir.exists() {
            let _ = std::fs::create_dir(dir);
        }
        
        let mut file = match std::fs::File::create(&path) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Could not create export file: {}", e);
                return;
            }
        };
        
        if let Err(e) = file.write_all(data.as_bytes()) {
            eprintln!("Could not write to export file: {}", e);
        } else {
            println!("Exported results to: {}", path);
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
            _ => {
                // Unknown command, could show error or do nothing
            }
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
            workers::TaskResult::Error(msg) => {
                match self.current_tab {
                    Tab::Recon => {
                        self.recon.set_error(msg);
                    }
                    Tab::Load => {
                        self.load.set_error(msg);
                    }
                    Tab::ScanPorts => {
                        self.scan_ports.set_error(msg);
                    }
                    Tab::ScanEndpoints => {
                        self.scan_endpoints.set_error(msg);
                    }
                    Tab::Fingerprint => {
                        self.fingerprint.set_error(msg);
                    }
                    Tab::Fuzz => {
                        self.fuzz.set_error(msg);
                    }
                    Tab::Waf => {
                        self.waf.set_error(msg);
                    }
                    Tab::WafStress => {
                        self.waf_stress.set_error(msg);
                    }
                    Tab::Scan => {
                        self.scan.set_error(msg);
                    }
                    Tab::Packet => {
                        self.packet.set_error(msg);
                    }
                    _ => {}
                }
            }
        }
    }
}
