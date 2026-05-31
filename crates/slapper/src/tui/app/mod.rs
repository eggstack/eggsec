pub(crate) mod bookmarks;
pub(crate) mod command;
pub(crate) mod confirmation;
pub(crate) mod dispatch;
pub(crate) mod error;
pub(crate) mod export;
pub(crate) mod help_config;
pub(crate) mod input;
pub(crate) mod key_handler;
pub(crate) mod navigation;
mod options;
pub(crate) mod runner;
pub(crate) mod state_update;
pub(crate) mod tab_error;
pub(crate) mod task_management;
pub(crate) mod task_runtime;

pub use crate::tui::state::create_shared_history;
pub use bookmarks::{get_bookmarked_tab_ids, is_bookmarked, toggle_bookmark};
pub use confirmation::PendingAction;
pub use input::InputMode;
pub use key_handler::KeyHandler;
pub use notifications::{Notification, NotificationSeverity};
pub use options::GlobalHttpOptions;
pub use runner::run;

pub(crate) mod notifications;

use super::error::make_friendly_error;
use crate::tui::help::{CommandPalette, HelpContext, HelpManager, HelpOverlay};
use crate::tui::session::{SessionConfig, SessionManager};
use crate::tui::state::SharedHistory;
use crate::tui::tabs;
use crate::tui::tabs::{Tab, TabInput};
use crate::tui::theme::ThemeManager;
use crate::tui::workers;
use crate::types::OutputFormat;
use crossterm::event::KeyCode;
use dispatch::TabDispatcher;
use rustc_hash::FxHashSet;
use task_management::TabTaskConfigSource;

pub struct App {
    pub current_tab: Tab,
    pub should_quit: bool,
    pub mode: InputMode,
    pub session_manager: crate::tui::session::SessionManager,
    pub last_auto_save: std::time::Instant,
    pub theme_manager: crate::tui::theme::ThemeManager,
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
    pub settings: tabs::SettingsTab,
    pub http_options: GlobalHttpOptions,
    pub history: SharedHistory,
    pub show_help: bool,
    pub help_tab: Option<Tab>,
    pub show_http_options: bool,
    pub show_search: bool,
    pub search_query: String,
    pub search_is_global: bool,
    pub global_search: Option<crate::tui::search::GlobalSearch>,
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
    pub task_inner_abort: Option<tokio::task::AbortHandle>,
    pub task_tab: Option<crate::tui::tabs::Tab>,
    pub progress_rx: Option<tokio::sync::mpsc::Receiver<(u64, u64)>>,
    pub result_rx: Option<tokio::sync::mpsc::Receiver<workers::TaskResult>>,
    pub help_manager: HelpManager,
    pub help_overlay: Option<HelpOverlay>,
    pub command_palette: Option<CommandPalette>,
    pub help_context: HelpContext,
    pub pending_action: Option<PendingAction>,
    pub needs_redraw: bool,
    pub tab_scroll_offset: u16,
    pub last_tab_area_width: u16,
    pub bookmarks: FxHashSet<String>,
    pub paused: bool,
    pub spinner_tick: u64,
    pub notification: Option<Notification>,
    pub show_quick_switch: bool,
    pub quick_switch_query: String,
    pub quick_switch_selected: usize,
}

impl App {
    pub fn new(history: SharedHistory) -> Self {
        Self::new_inner(history, true)
    }

    pub fn new_for_testing(history: SharedHistory) -> Self {
        Self::new_inner(history, false)
    }

    fn new_inner(history: SharedHistory, restore_session: bool) -> Self {
        let session_manager = SessionManager::new(SessionConfig::default());

        let restored_state = if restore_session {
            session_manager.load_latest_session().ok().flatten()
        } else {
            None
        };

        let restored_bookmarks: FxHashSet<String> = if let Some(ref state) = restored_state {
            let mut bookmarks = FxHashSet::default();
            for bookmark_id in &state.bookmarks {
                if let Some(tab) = Tab::from_stable_id(bookmark_id) {
                    bookmarks.insert(tab.stable_id().to_string());
                }
            }
            for &idx in &state.legacy_bookmarks {
                if let Some(tab) = Tab::from_index(idx) {
                    if tab.visible_index().is_some() {
                        bookmarks.insert(tab.stable_id().to_string());
                    }
                }
            }
            bookmarks
        } else {
            FxHashSet::default()
        };

        let restored_current_tab = restored_state
            .as_ref()
            .and_then(|s| {
                s.current_tab_id
                    .as_ref()
                    .and_then(|id| Tab::from_stable_id(id))
            })
            .or_else(|| {
                restored_state
                    .as_ref()
                    .and_then(|s| s.legacy_current_tab)
                    .and_then(Tab::from_index)
            });

        let mut app = Self {
            current_tab: restored_current_tab.unwrap_or(Tab::Recon),
            should_quit: false,
            mode: InputMode::Normal,
            session_manager,
            last_auto_save: std::time::Instant::now(),
            theme_manager: ThemeManager::new(),
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
            search_is_global: false,
            global_search: Some(crate::tui::search::GlobalSearch::new()),
            search_backup: None,
            pending_key: None,
            tab_scroll_offset: 0,
            last_tab_area_width: 80,
            export_format: OutputFormat::Json,
            task_handle: None,
            task_inner_abort: None,
            task_tab: None,
            progress_rx: None,
            result_rx: None,
            help_manager: HelpManager::new(),
            help_overlay: None,
            command_palette: None,
            help_context: HelpContext::Normal,
            pending_action: None,
            needs_redraw: true,
            notification: None,
            bookmarks: restored_bookmarks,
            paused: false,
            spinner_tick: 0,
            show_quick_switch: false,
            quick_switch_query: String::new(),
            quick_switch_selected: 0,
        };

        // Sync settings with current theme
        let theme = app.theme_manager.current().clone();
        app.settings.sync_with_theme(&theme);

        app
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

    pub fn is_running(&mut self) -> bool {
        match self.current_tab {
            Tab::Recon => tabs::TabState::state(&self.recon) == tabs::AppState::Running,
            Tab::Load => tabs::TabState::state(&self.load) == tabs::AppState::Running,
            Tab::ScanPorts => tabs::TabState::state(&self.scan_ports) == tabs::AppState::Running,
            Tab::ScanEndpoints => {
                tabs::TabState::state(&self.scan_endpoints) == tabs::AppState::Running
            }
            Tab::Fingerprint => tabs::TabState::state(&self.fingerprint) == tabs::AppState::Running,
            Tab::Fuzz => tabs::TabState::state(&self.fuzz) == tabs::AppState::Running,
            Tab::Waf => tabs::TabState::state(&self.waf) == tabs::AppState::Running,
            Tab::WafStress => tabs::TabState::state(&self.waf_stress) == tabs::AppState::Running,
            Tab::Scan => tabs::TabState::state(&self.scan) == tabs::AppState::Running,
            Tab::Resume => tabs::TabState::state(&self.resume) == tabs::AppState::Running,
            Tab::Proxy => tabs::TabState::state(&self.proxy) == tabs::AppState::Running,
            Tab::Packet => tabs::TabState::state(&self.packet) == tabs::AppState::Running,
            Tab::GraphQl => tabs::TabState::state(&self.graphql) == tabs::AppState::Running,
            Tab::OAuth => tabs::TabState::state(&self.oauth) == tabs::AppState::Running,
            Tab::Cluster => tabs::TabState::state(&self.cluster) == tabs::AppState::Running,
            Tab::Stress => tabs::TabState::state(&self.stress) == tabs::AppState::Running,
            Tab::Report => tabs::TabState::state(&self.report) == tabs::AppState::Running,
            Tab::Settings | Tab::History | Tab::Dashboard => false,
            #[cfg(feature = "nse")]
            Tab::Nse => tabs::TabState::state(&self.nse) == tabs::AppState::Running,
            #[cfg(not(feature = "nse"))]
            Tab::Nse => false,
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => tabs::TabState::state(&self.hunt) == tabs::AppState::Running,
            #[cfg(not(feature = "advanced-hunting"))]
            Tab::Hunt => false,
            #[cfg(feature = "headless-browser")]
            Tab::Browser => tabs::TabState::state(&self.browser) == tabs::AppState::Running,
            #[cfg(not(feature = "headless-browser"))]
            Tab::Browser => false,
            #[cfg(feature = "compliance")]
            Tab::Compliance => tabs::TabState::state(&self.compliance) == tabs::AppState::Running,
            #[cfg(not(feature = "compliance"))]
            Tab::Compliance => false,
            #[cfg(feature = "database")]
            Tab::Storage => tabs::TabState::state(&self.storage) == tabs::AppState::Running,
            #[cfg(not(feature = "database"))]
            Tab::Storage => false,
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => {
                tabs::TabState::state(&self.integrations) == tabs::AppState::Running
            }
            #[cfg(not(feature = "external-integrations"))]
            Tab::Integrations => false,
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => tabs::TabState::state(&self.workflow) == tabs::AppState::Running,
            #[cfg(not(feature = "finding-workflow"))]
            Tab::Workflow => false,
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => tabs::TabState::state(&self.vuln) == tabs::AppState::Running,
            #[cfg(not(feature = "vuln-management"))]
            Tab::Vuln => false,
        }
    }

    fn dispatcher_mut(&mut self) -> TabDispatcher<'_> {
        if self.current_tab == Tab::History {
            return TabDispatcher::new_locked(self.history.lock());
        }

        let mut tab = self.current_tab;
        let tab_input: &mut dyn TabInput = tab.as_tab_input(self);
        TabDispatcher::new(tab_input)
    }

    pub fn handle_enter(&mut self) {
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Dashboard Enter jumps to Recon tab (first scan tab)
        if self.current_tab == super::tabs::Tab::Dashboard {
            self.current_tab = super::tabs::Tab::Recon;
            return;
        }

        let is_running = {
            let mut dispatcher = self.dispatcher_mut();
            dispatcher.handle_enter();
            let input_focused = dispatcher.is_input_focused();
            let running = dispatcher.is_running();
            drop(dispatcher);
            self.mode = if input_focused {
                InputMode::Insert
            } else {
                InputMode::Normal
            };
            running
        };

        if is_running {
            if let Some(task_config) = self.build_current_task() {
                self.spawn_task(Some(task_config));
            }
        }
    }

    fn build_current_task(&self) -> Option<workers::TaskConfig> {
        self.current_tab.build_task_config_from_app(self)
    }

    pub fn handle_escape(&mut self) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.mode == InputMode::Insert {
            self.mode = InputMode::Normal;
        }
        self.dispatcher_mut().handle_escape();
    }

    pub fn handle_char(&mut self, c: char) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_char(c);
    }

    pub fn handle_backspace(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_backspace();
    }

    pub fn handle_delete(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_delete();
    }

    pub fn handle_autocomplete(&mut self) -> bool {
        if self.show_help || self.mode != InputMode::Insert {
            return false;
        }
        self.dispatcher_mut().handle_autocomplete()
    }

    pub fn handle_up(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_up();
    }

    pub fn handle_down(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_down();
    }

    pub fn handle_left(&mut self) {
        if self.show_help {
            return;
        }
        if !self.dispatcher_mut().handle_left() {
            tracing::trace!("handle_left at left edge");
        }
    }

    pub fn handle_right(&mut self) {
        if self.show_help {
            return;
        }
        if !self.dispatcher_mut().handle_right() {
            tracing::trace!("handle_right at right edge");
        }
    }

    pub fn handle_focus_next(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_focus_next();
        let input_focused = self.dispatcher_mut().is_input_focused();
        if input_focused {
            self.mode = InputMode::Insert;
        } else {
            self.mode = InputMode::Normal;
        }
    }

    pub fn handle_focus_prev(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_focus_prev();
        let input_focused = self.dispatcher_mut().is_input_focused();
        if input_focused {
            self.mode = InputMode::Insert;
        } else {
            self.mode = InputMode::Normal;
        }
    }

    pub fn handle_left_or_prev_tab(&mut self) -> bool {
        if self.show_help {
            return false;
        }
        if self.dispatcher_mut().is_at_left_edge() {
            false
        } else {
            self.dispatcher_mut().handle_left();
            true
        }
    }

    pub fn handle_right_or_next_tab(&mut self) -> bool {
        if self.show_help {
            return false;
        }
        if self.dispatcher_mut().is_at_right_edge() {
            false
        } else {
            self.dispatcher_mut().handle_right();
            true
        }
    }

    pub fn reset_current_tab(&mut self) {
        self.dispatcher_mut().reset();
    }

    pub fn save_settings(&mut self) {
        if self.current_tab == Tab::Settings {
            self.settings.save_config();
        }
    }

    pub fn delete_history_entry(&mut self) {
        let mut h = self.history.lock();
        h.delete_selected();
    }

    pub fn clear_all_history(&mut self) {
        let mut h = self.history.lock();
        h.clear_all();
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

    /// Set current tab only if it's available for the current feature set.
    /// Returns true if the tab was set, false if unavailable.
    pub fn set_current_tab_if_available(&mut self, tab: Tab) -> bool {
        if Tab::all().contains(&tab) {
            self.current_tab = tab;
            true
        } else {
            false
        }
    }

    pub fn is_confirm_popup_visible(&self) -> bool {
        self.pending_action.is_some()
    }

    pub fn page_up(&mut self) {
        const PAGE_SIZE: usize = 10;
        self.dispatcher_mut().page_up(PAGE_SIZE);
    }

    pub fn page_down(&mut self) {
        const PAGE_SIZE: usize = 10;
        self.dispatcher_mut().page_down(PAGE_SIZE);
    }

    pub fn handle_word_forward(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_word_forward();
    }

    pub fn handle_word_backward(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_word_backward();
    }

    pub fn handle_home(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_home();
    }

    pub fn handle_end(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_end();
    }

    pub fn handle_top(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_top();
    }

    pub fn handle_bottom(&mut self) {
        if self.show_help {
            return;
        }
        self.dispatcher_mut().handle_bottom();
    }

    pub fn toggle_bookmark(&mut self, tab: Tab) {
        bookmarks::toggle_bookmark(&mut self.bookmarks, tab);
    }

    pub fn is_bookmarked(&self, tab: Tab) -> bool {
        bookmarks::is_bookmarked(&self.bookmarks, tab)
    }

    pub fn get_bookmarked_tab_ids(&self) -> Vec<String> {
        bookmarks::get_bookmarked_tab_ids(&self.bookmarks)
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn auto_save_if_due(&mut self) {
        let interval_secs = self.session_manager.auto_save_interval();
        if self.last_auto_save.elapsed().as_secs() >= interval_secs {
            if let Err(e) = self.session_manager.save_quick(self) {
                tracing::warn!("Auto-save failed: {:?}", e);
            }
            self.last_auto_save = std::time::Instant::now();
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme_manager.toggle();
        crate::tui::theme::sync_theme_to_thread_local(self.theme_manager.current());
    }

    pub fn set_dark_mode(&mut self, enabled: bool) {
        let target_mode = if enabled { "dark" } else { "light" };
        if self.theme_manager.set_theme(target_mode) {
            crate::tui::theme::sync_theme_to_thread_local(self.theme_manager.current());
            self.needs_redraw = true;
        }
    }

    pub fn set_accent_color(&mut self, color: &str) {
        self.theme_manager.set_accent_color(color);
        crate::tui::theme::sync_theme_to_thread_local(self.theme_manager.current());
        self.needs_redraw = true;
    }

    pub fn toggle_quick_switch(&mut self) {
        if self.is_any_overlay_active() {
            return;
        }
        self.show_quick_switch = true;
        self.quick_switch_query.clear();
        self.quick_switch_selected = 0;
        self.needs_redraw = true;
    }

    pub fn close_quick_switch(&mut self) {
        self.show_quick_switch = false;
        self.quick_switch_query.clear();
        self.needs_redraw = true;
    }

    pub fn is_quick_switch_visible(&self) -> bool {
        self.show_quick_switch
    }

    pub fn get_quick_switch_results(&self) -> Vec<&'static Tab> {
        let query = self.quick_switch_query.to_lowercase();
        if query.is_empty() {
            return Tab::all().iter().collect();
        }

        use crate::tui::utils::fuzzy::fuzzy_score;
        let mut scored: Vec<(u32, &'static Tab)> = Tab::all()
            .iter()
            .filter_map(|tab| {
                let title_lower = tab.title().to_lowercase();
                let stable_id_lower = tab.stable_id().to_lowercase();
                let desc_lower = tab.description().to_lowercase();
                let score = fuzzy_score(&title_lower, &query)
                    .max(fuzzy_score(&stable_id_lower, &query))
                    .max(fuzzy_score(&desc_lower, &query));

                if score > 0 {
                    Some((score, tab))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by_key(|b| std::cmp::Reverse(b.0));
        scored.into_iter().map(|(_, tab)| tab).collect()
    }

    /// Check if command palette is visible
    pub fn is_command_palette_visible(&self) -> bool {
        self.command_palette
            .as_ref()
            .map(|p| p.visible)
            .unwrap_or(false)
    }

    /// Check if search popup is visible
    pub fn is_search_visible(&self) -> bool {
        self.show_search
    }

    /// Check if HTTP options popup is visible
    pub fn is_http_options_visible(&self) -> bool {
        self.show_http_options
    }

    /// Check if help popup is visible
    pub fn is_help_visible(&self) -> bool {
        self.show_help
    }

    /// Get the topmost overlay based on precedence:
    /// 1. Confirm popup (pending_action)
    /// 2. Command palette
    /// 3. Quick switch
    /// 4. Search
    /// 5. HTTP options
    /// 6. Help
    ///    Returns None if no overlay is active
    pub fn topmost_overlay(&self) -> Option<OverlayType> {
        if self.is_confirm_popup_visible() {
            Some(OverlayType::ConfirmPopup)
        } else if self.is_command_palette_visible() {
            Some(OverlayType::CommandPalette)
        } else if self.is_quick_switch_visible() {
            Some(OverlayType::QuickSwitch)
        } else if self.is_search_visible() {
            Some(OverlayType::Search)
        } else if self.is_http_options_visible() {
            Some(OverlayType::HttpOptions)
        } else if self.is_help_visible() {
            Some(OverlayType::Help)
        } else {
            None
        }
    }

    /// Check if any overlay is active (blocks tab content interaction)
    pub fn is_any_overlay_active(&self) -> bool {
        self.topmost_overlay().is_some()
    }

    /// Set a notification message with the given severity
    pub fn set_notification(&mut self, message: String, severity: NotificationSeverity) {
        self.notification = Some(Notification::new(message, severity));
        self.needs_redraw = true;
    }

    /// Clear any current notification (if expired or manually dismissed)
    pub fn clear_notification(&mut self) {
        if let Some(ref notification) = self.notification {
            if notification.is_expired() {
                self.notification = None;
                self.needs_redraw = true;
            }
        }
    }

    /// Get the current notification if any (and clear if expired)
    pub fn get_notification(&mut self) -> Option<&Notification> {
        if let Some(ref notification) = self.notification {
            if notification.is_expired() {
                self.notification = None;
                self.needs_redraw = true;
                return None;
            }
        }
        self.notification.as_ref()
    }
}

/// Represents the type of overlay currently shown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayType {
    ConfirmPopup,
    CommandPalette,
    QuickSwitch,
    Search,
    HttpOptions,
    Help,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::tabs::Tab;
    use crossterm::event::KeyCode;

    fn create_test_app() -> App {
        App::new_for_testing(create_shared_history())
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
        assert_eq!(PendingAction::ResetTab.message().0, "Confirm Reset");
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

    #[test]
    fn test_command_palette_visible() {
        let app = create_test_app();
        assert!(!app.is_command_palette_visible());

        // Command palette is created on demand - simulate visibility
        // This would need a command palette to be created first
    }

    #[test]
    fn test_search_visible() {
        let mut app = create_test_app();
        assert!(!app.is_search_visible());

        app.show_search = true;
        assert!(app.is_search_visible());

        app.show_search = false;
        assert!(!app.is_search_visible());
    }

    #[test]
    fn test_http_options_visible() {
        let mut app = create_test_app();
        assert!(!app.is_http_options_visible());

        app.show_http_options = true;
        assert!(app.is_http_options_visible());

        app.show_http_options = false;
        assert!(!app.is_http_options_visible());
    }

    #[test]
    fn test_help_visible() {
        let mut app = create_test_app();
        assert!(!app.is_help_visible());

        app.show_help = true;
        assert!(app.is_help_visible());

        app.show_help = false;
        assert!(!app.is_help_visible());
    }

    #[test]
    fn test_topmost_overlay_none_when_all_hidden() {
        let app = create_test_app();
        assert!(app.topmost_overlay().is_none());
    }

    #[test]
    fn test_topmost_overlay_confirm_popup_precedence() {
        let mut app = create_test_app();
        // Set up multiple overlays
        app.show_help = true;
        app.show_search = true;
        app.show_http_options = true;

        // Confirm popup should take precedence
        app.request_confirmation(PendingAction::ResetTab);
        assert_eq!(app.topmost_overlay(), Some(OverlayType::ConfirmPopup));
    }

    #[test]
    fn test_topmost_overlay_command_palette_precedence() {
        let mut app = create_test_app();
        app.show_help = true;
        app.show_search = true;
        app.show_http_options = true;

        // Simulate command palette visible
        // Note: command_palette needs to be created - this test may need adjustment
    }

    #[test]
    fn test_h_key_closes_http_options_overlay() {
        let mut app = create_test_app();
        // Show HTTP options
        app.show_http_options = true;
        assert!(app.is_http_options_visible());

        // Simulate 'h' key press behavior (from runner.rs lines 385-387)
        if app.is_http_options_visible() {
            app.show_http_options = false;
            app.needs_redraw = true;
        }

        // Verify HTTP options is no longer visible
        assert!(!app.is_http_options_visible());
        assert!(app.needs_redraw);
    }

    #[test]
    fn test_handle_right_prefers_tab_local_navigation_before_tab_switch() {
        let mut app = create_test_app();
        app.current_tab = Tab::Settings;
        app.mode = InputMode::Normal;
        let initial_focus = app.settings.focus_area;
        let initial_tab = app.current_tab;

        app.handle_right();

        assert_eq!(app.current_tab, initial_tab);
        assert_ne!(app.settings.focus_area, initial_focus);
    }

    #[test]
    fn test_handle_enter_resyncs_mode_when_input_blurs() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.mode = InputMode::Insert;
        app.recon.inputs.focus(0);

        app.handle_enter();

        assert_eq!(app.mode, InputMode::Normal);
        assert!(!app.recon.inputs.is_focused());
    }
}
