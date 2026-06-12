pub(crate) mod action;
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
pub(crate) mod overlay;
pub(crate) mod runner;
pub(crate) mod state;
pub(crate) mod state_update;
pub(crate) mod tab_error;
pub(crate) mod tab_store;
pub(crate) mod task_management;
pub(crate) mod task_runtime;
pub(crate) mod theme_runtime;

pub use crate::state::create_shared_history;
pub use bookmarks::{get_bookmarked_tab_ids, is_bookmarked, toggle_bookmark};
pub use confirmation::PendingAction;
pub use input::InputMode;
pub use key_handler::KeyHandler;
pub use notifications::{Notification, NotificationSeverity};
pub use options::GlobalHttpOptions;
pub use runner::run;
pub use state::{OverlayState, QuickSwitchState, SearchState, TaskState, ThemeLoadState};
pub use tab_store::TabStore;

pub(crate) use action::{CommandPaletteInput, QuickSwitchInput, UiAction};
pub(crate) use overlay::OverlayController;

pub(crate) mod notifications;

use super::error::make_friendly_error;
use crate::help::{CommandPalette, HelpContext, HelpManager};
use crate::session::{SessionConfig, SessionManager};
use crate::state::SharedHistory;
use crate::tabs;
use crate::tabs::{Tab, TabInput};
use crate::theme::{display_theme_name, ThemeManager};
use crate::workers;
use crossterm::event::KeyCode;
use dispatch::TabDispatcher;
use eggsec::config::{
    confirmation_classes_for, EnforcementContext, EnforcementOutcome, ExecutionPolicy, LoadedScope,
    ManualOverride, OperationDescriptor, OperationMode, OperationRisk,
};
use eggsec::types::OutputFormat;
use rustc_hash::FxHashSet;
use task_management::TaskBuilder;

pub struct App {
    pub current_tab: Tab,
    pub should_quit: bool,
    pub mode: InputMode,
    pub session_manager: crate::session::SessionManager,
    pub last_auto_save: std::time::Instant,
    pub theme_manager: crate::theme::ThemeManager,
    pub tabs: TabStore,
    pub http_options: GlobalHttpOptions,
    pub history: SharedHistory,
    pub overlay: OverlayState,
    pub search: SearchState,
    pub quick_switch: QuickSwitchState,
    pub task_state: TaskState,
    pub pending_key: Option<KeyCode>,
    pub export_format: OutputFormat,
    pub help_manager: HelpManager,
    pub command_palette: Option<CommandPalette>,
    pub help_context: HelpContext,
    pub needs_redraw: bool,
    pub tab_scroll_offset: u16,
    pub last_tab_area_width: u16,
    pub bookmarks: FxHashSet<String>,
    pub theme_load: ThemeLoadState,

    /// Shared enforcement context (loaded from config + scope at startup, like CLI main).
    /// TUI defaults to ManualPermissive; --strict-scope equivalent is not a TUI flag today.
    pub enforcement: EnforcementContext,
    /// Captured scope provenance for enforcement (mirrors LoadedScope used by CLI handlers).
    pub loaded_scope: LoadedScope,
    /// Original config path (if any) for rebuilds after settings changes.
    pub config_path: Option<String>,
}

impl App {
    pub fn new(history: SharedHistory) -> Self {
        Self::new_inner(history, true)
    }

    pub fn new_for_testing(history: SharedHistory) -> Self {
        let session_manager = SessionManager::new(SessionConfig::default());
        let mut app = Self {
            current_tab: Tab::Recon,
            should_quit: false,
            mode: InputMode::Normal,
            session_manager,
            last_auto_save: std::time::Instant::now(),
            theme_manager: ThemeManager::new(),
            tabs: TabStore::new(),
            http_options: GlobalHttpOptions::default(),
            history,
            overlay: OverlayState::default(),
            search: SearchState::default(),
            quick_switch: QuickSwitchState::default(),
            task_state: TaskState::default(),
            pending_key: None,
            tab_scroll_offset: 0,
            last_tab_area_width: 80,
            export_format: OutputFormat::Json,
            help_manager: HelpManager::new(),
            command_palette: None,
            help_context: HelpContext::Normal,
            needs_redraw: true,
            bookmarks: FxHashSet::default(),
            theme_load: ThemeLoadState::default(),
            enforcement: EnforcementContext::manual_permissive(
                ExecutionPolicy::default(),
                LoadedScope::default_empty(),
            ),
            loaded_scope: LoadedScope::default_empty(),
            config_path: None,
        };
        app.update_settings_theme_selector();
        crate::theme::sync_theme_to_thread_local(app.theme_manager.current());
        app
    }

    fn new_inner(history: SharedHistory, restore_session: bool) -> Self {
        let session_manager = SessionManager::new(SessionConfig::default());

        let restored_state = if restore_session {
            match session_manager.load_latest_session() {
                Ok(state) => state,
                Err(e) => {
                    tracing::warn!("Failed to load previous session: {:?}", e);
                    None
                }
            }
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
            tabs: TabStore::new(),
            http_options: GlobalHttpOptions::default(),
            history,
            overlay: OverlayState::default(),
            search: SearchState::default(),
            quick_switch: QuickSwitchState::default(),
            task_state: TaskState::default(),
            pending_key: None,
            tab_scroll_offset: 0,
            last_tab_area_width: 80,
            export_format: OutputFormat::Json,
            help_manager: HelpManager::new(),
            command_palette: None,
            help_context: HelpContext::Normal,
            needs_redraw: true,
            bookmarks: restored_bookmarks,
            theme_load: ThemeLoadState::default(),
            enforcement: EnforcementContext::manual_permissive(
                ExecutionPolicy::default(),
                LoadedScope::default_empty(),
            ),
            loaded_scope: LoadedScope::default_empty(),
            config_path: None,
        };

        // Saved sessions can reference packaged/user themes that are not registered until
        // the background loader finishes. Defer one retry instead of blocking startup.
        if let Some(state) = &restored_state {
            if !app.theme_manager.set_theme(&state.theme_name) {
                tracing::warn!(
                    theme = %state.theme_name,
                    "theme unavailable at startup; will retry after theme load"
                );
                app.theme_load.deferred_theme_name = Some(state.theme_name.clone());
            }
        }
        crate::theme::sync_theme_to_thread_local(app.theme_manager.current());

        // Sync settings with current theme and built-in list before the background loader runs.
        app.update_settings_theme_selector();

        // Spawn background theme loading (non-blocking, non-fatal)
        app.spawn_theme_loader();

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

    /// Returns true when a task is in progress **on any tab**.
    ///
    /// Previous implementation only checked the *current* tab's state, which
    /// returned `false` when the user navigated away from the running tab.
    /// This made the status bar display "Ready" while a scan was still
    /// actively running in the background.
    pub fn is_running(&self) -> bool {
        // A running task is also visible in the task_state regardless of which
        // tab is focused.
        if self.task_state.handle.is_some() || self.task_state.tab.is_some() {
            return true;
        }
        self.current_tab.as_tab_state(self).is_running()
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
        if self.overlay.show_help {
            self.overlay.show_help = false;
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

        // Apply theme change from settings selector if pending
        if self.current_tab == super::tabs::Tab::Settings {
            if let Some(theme_name) = self.tabs.settings.take_pending_theme() {
                if self.theme_manager.set_theme(&theme_name) {
                    self.theme_load.mark_user_changed();
                    crate::theme::sync_theme_to_thread_local(self.theme_manager.current());
                    self.update_settings_theme_selector();
                    self.needs_redraw = true;
                } else {
                    tracing::warn!("Unknown theme selected: {}", theme_name);
                }
            }
        }

        if is_running {
            if let Some(task_config) = self.build_current_task() {
                // Central policy gate (TUI now shares the exact EnforcementContext + RequireConfirmation + ManualOverride model as CLI).
                // Build a descriptor that matches CLI conventions for the current tab/action.
                if let Some(desc) = self.build_current_operation_descriptor() {
                    let outcome = self.enforcement.evaluate(&desc);
                    match outcome {
                        EnforcementOutcome::Allow(_) | EnforcementOutcome::Warn(_) => {
                            // Proceed normally (warnings already logged by evaluate if any).
                            self.spawn_task(Some(task_config));
                        }
                        EnforcementOutcome::RequireConfirmation(decision) => {
                            // Pause the tab running state so the UI doesn't look like it started.
                            // Use the same pattern as stop_tab_state to stop the tab's internal running flag.
                            {
                                let mut tab = self.current_tab;
                                tab.as_tab_input(self).stop();
                            }
                            // Capture for replay on manual override confirm.
                            self.request_policy_confirmation(desc, decision, Some(task_config));
                        }
                        EnforcementOutcome::Deny(d) => {
                            {
                                let mut tab = self.current_tab;
                                tab.as_tab_input(self).stop();
                            }
                            self.set_error_for_current_tab(
                                crate::app::tab_error::TabError::Target(d.to_human_readable()),
                            );
                        }
                    }
                } else {
                    // No descriptor (non-target op or not yet mapped); proceed as before.
                    self.spawn_task(Some(task_config));
                }
            }
        }

        // Post-dispatch retroactive policy gate for direct-launch tabs (packet, stress, auth, cluster, etc.)
        // that perform their actual start inside the tab's handle_enter / run_* methods instead of (or in addition to)
        // going through build_current_task + spawn_task.
        // If such a tab just entered a running state for a target-bearing action, we evaluate here.
        // On RequireConfirmation we stop it and open the policy popup (with no pre-built TaskConfig; on confirm we will re-dispatch).
        // This brings the direct tabs in line with the shared EnforcementContext / RequireConfirmation / narrow manual override model.
        if self.is_direct_launch_tab(self.current_tab) {
            if self.current_tab.as_tab_state(self).is_running() {
                if let Some(desc) = self.build_current_operation_descriptor() {
                    let outcome = self.enforcement.evaluate(&desc);
                    match outcome {
                        EnforcementOutcome::Allow(_) | EnforcementOutcome::Warn(_) => {
                            // allowed; the tab already started its work
                        }
                        EnforcementOutcome::RequireConfirmation(decision) => {
                            {
                                let mut tab = self.current_tab;
                                tab.as_tab_input(self).stop();
                            }
                            self.request_policy_confirmation(desc, decision, None);
                        }
                        EnforcementOutcome::Deny(d) => {
                            {
                                let mut tab = self.current_tab;
                                tab.as_tab_input(self).stop();
                            }
                            self.set_error_for_current_tab(
                                crate::app::tab_error::TabError::Target(d.to_human_readable()),
                            );
                        }
                    }
                }
            }
        }
    }

    fn is_direct_launch_tab(&self, tab: Tab) -> bool {
        tab.is_direct_launch()
    }

    /// Build an OperationDescriptor for the current tab/action that is compatible with the
    /// shared enforcement evaluator (same risk/capability/operation strings used by CLI handlers).
    /// Returns None for tabs/operations that have no target-bearing networked action.
    pub fn build_current_operation_descriptor(&self) -> Option<OperationDescriptor> {
        let tab = self.current_tab;
        let spec = crate::tabs::spec_for(tab).filter(|s| s.operation.is_some())?;
        let target = self.current_tab_target();
        let risk = crate::tabs::risk_from_group(spec.risk_group);
        let op = spec.operation.unwrap().to_string();
        let required_capabilities: Vec<eggsec::config::Capability> = Vec::new();
        let required_features: Vec<String> = spec
            .feature
            .map(|f| vec![f.to_string()])
            .unwrap_or_default();
        let descriptor = OperationDescriptor {
            operation: op,
            mode: OperationMode::StandardAssessment,
            risk,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            target: if target.as_deref().unwrap_or("").is_empty() {
                None
            } else {
                target
            },
            required_features,
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities,
        };

        // Override descriptor for wireless active attacks (deauth/disassoc).
        // Active attacks are Intrusive risk under DefenseLab mode, requiring
        // policy confirmation under ManualPermissive.
        #[cfg(feature = "wireless-advanced")]
        {
            if self.current_tab == Tab::Wireless && self.tabs.wireless.active_mode {
                if let Some((
                    _interface,
                    attack_type,
                    _bssid,
                    _client,
                    _frame_count,
                    _rate_limit,
                    dry_run,
                )) = self.tabs.wireless.active_attack_config()
                {
                    let risk = if dry_run {
                        OperationRisk::SafeActive
                    } else {
                        OperationRisk::Intrusive
                    };
                    return Some(OperationDescriptor {
                        operation: format!("wireless-{attack_type}"),
                        mode: OperationMode::DefenseLab,
                        risk,
                        intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
                        required_features: vec!["wireless-advanced".to_string()],
                        target: descriptor.target,
                        required_policy_flags: Vec::new(),
                        requires_private_or_local_target: false,
                        requires_explicit_scope: false,
                        required_capabilities: Vec::new(),
                    });
                }
            }
        }

        Some(descriptor)
    }

    /// Best-effort extraction of the primary target string from the current tab (for descriptor).
    pub(crate) fn current_tab_target(&self) -> Option<String> {
        match self.current_tab {
            Tab::Recon => self.tabs.recon.primary_target(),
            Tab::ScanPorts => self.tabs.scan_ports.primary_target(),
            Tab::ScanEndpoints => self.tabs.scan_endpoints.primary_target(),
            Tab::Fingerprint => self.tabs.fingerprint.primary_target(),
            Tab::Fuzz => self.tabs.fuzz.primary_target(),
            Tab::Waf => self.tabs.waf.primary_target(),
            Tab::WafStress => self.tabs.waf_stress.primary_target(),
            Tab::Scan => self.tabs.scan.primary_target(),
            Tab::Load => self.tabs.load.primary_target(),
            Tab::Stress => self.tabs.stress.primary_target(),
            Tab::Packet => self.tabs.packet.primary_target(),
            Tab::GraphQl => self.tabs.graphql.primary_target(),
            Tab::OAuth => self.tabs.oauth.primary_target(),
            Tab::Auth => self.tabs.auth.primary_target(),
            #[cfg(feature = "nse")]
            Tab::Nse => self.tabs.nse.primary_target(),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.tabs.hunt.primary_target(),
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.tabs.browser.primary_target(),
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.tabs.compliance.primary_target(),
            #[cfg(feature = "wireless")]
            Tab::Wireless => self.tabs.wireless.primary_target(),
            _ => None,
        }
    }

    /// Produce a safe, minimal CLI equivalent for the current tab state.
    /// Returns None for non-executable tabs (Settings, History, Dashboard, Report, etc.).
    /// Never emits broad bypass flags (--yes, --allow-*, --insecure-tls, etc.).
    /// Uses primary_target + App::export_format + App::loaded_scope.path (when explicit).
    /// For Phase 8: only recon concurrency (if !=20), scan-ports ports (if != default), fuzz max-payloads (if >0) are appended as safe common options.
    pub fn copy_cli_equivalent(&self) -> Option<String> {
        use crate::utils::shell_escape;
        let tab = self.current_tab;
        let cmd = tab.cli_command();
        // Non-executable / no cli_command tabs return None (per AC and plan).
        // Report is listed in plan as example non-executable for this feature (even though spec has eggsec report).
        if cmd == "unknown"
            || cmd == "Settings"
            || cmd == "History"
            || cmd == "Dashboard"
            || cmd == "eggsec report"
            || tab == Tab::Report
        {
            return None;
        }
        // Only tabs that have cli_command starting with "eggsec " are executable.
        if !cmd.starts_with("eggsec ") {
            return None;
        }

        let target = self.current_tab_target().unwrap_or_default();
        let target_esc = if target.is_empty() {
            "''".to_string()
        } else {
            shell_escape(&target)
        };

        let mut out = format!("{} {}", cmd, target_esc);

        // Tab-specific safe portable options (conservative per plan).
        match tab {
            Tab::Recon => {
                let conc = self.tabs.recon.concurrency();
                if conc != 20 {
                    out.push_str(&format!(" --concurrency {}", conc));
                }
            }
            Tab::ScanPorts => {
                let ports = self.tabs.scan_ports.ports();
                // Only append if user changed from the UI default shown in the field.
                if ports != "1-1024" {
                    out.push_str(&format!(" --ports {}", shell_escape(ports)));
                }
                // Concurrency and timeout are common but plan says "start with ..."; we keep minimal for tests.
            }
            Tab::Fuzz => {
                // Intrusive example: include max-payloads if non-default (>0 means limited).
                let mp = self.tabs.fuzz.max_payloads();
                if mp > 0 {
                    out.push_str(&format!(" --max-payloads {}", mp));
                }
            }
            Tab::Auth => {
                if let Some(username) = self.tabs.auth.username() {
                    out.push_str(&format!(" --username {}", shell_escape(username)));
                }
                if let Some(passwords) = self.tabs.auth.password_list() {
                    out.push_str(&format!(" --wordlist {}", shell_escape(passwords)));
                }
            }
            #[cfg(feature = "wireless-advanced")]
            Tab::Wireless if self.tabs.wireless.active_mode => {
                if let Some((_, _, bssid, client, frame_count, rate_limit, dry_run)) =
                    self.tabs.wireless.active_attack_config()
                {
                    out.push_str(" deauth");
                    if let Some(bssid) = bssid {
                        out.push_str(&format!(" --bssid {}", shell_escape(&bssid)));
                    }
                    if let Some(client) = client {
                        out.push_str(&format!(" --client {}", shell_escape(&client)));
                    }
                    if frame_count != 100 {
                        out.push_str(&format!(" --count {}", frame_count));
                    }
                    if rate_limit != 10 {
                        out.push_str(&format!(" --fps {}", rate_limit));
                    }
                    if dry_run {
                        out.push_str(" --dry-run");
                    }
                }
            }
            _ => {
                // Other executable tabs fall back to target-only (per "target only + note" guidance).
            }
        }

        // Append --format only if non-default (Pretty is CLI default).
        if self.export_format != eggsec::types::OutputFormat::Pretty {
            let fmt = match self.export_format {
                eggsec::types::OutputFormat::Json => "json",
                eggsec::types::OutputFormat::Compact => "compact",
                eggsec::types::OutputFormat::Csv => "csv",
                eggsec::types::OutputFormat::Html => "html",
                eggsec::types::OutputFormat::Markdown => "markdown",
                eggsec::types::OutputFormat::Sarif => "sarif",
                eggsec::types::OutputFormat::Junit => "junit",
                _ => "pretty",
            };
            if fmt != "pretty" {
                out.push_str(&format!(" --format {}", fmt));
            }
        }

        // Scope path: ONLY if explicit source with a path (CliScopeFile or ConfigFile with path).
        // Do not invent; mirror LoadedScope usage in runner.
        if let Some(ref p) = self.loaded_scope.path {
            if self.loaded_scope.source == eggsec::config::ScopeSource::CliScopeFile
                || self.loaded_scope.source == eggsec::config::ScopeSource::ConfigFile
            {
                out.push_str(&format!(" --scope {}", shell_escape(p)));
            }
        }

        // NEVER append policy bypasses for Phase 8 (per verbatim AC and plan).
        Some(out)
    }

    fn build_current_task(&self) -> Option<workers::TaskConfig> {
        match self.current_tab {
            Tab::Recon => self.tabs.recon.build_task_config(),
            Tab::Load => self.tabs.load.build_task_config(),
            Tab::ScanPorts => self.tabs.scan_ports.build_task_config(),
            Tab::ScanEndpoints => self.tabs.scan_endpoints.build_task_config(),
            Tab::Fingerprint => self.tabs.fingerprint.build_task_config(),
            Tab::Fuzz => self.tabs.fuzz.build_task_config(),
            Tab::Waf => self.tabs.waf.build_task_config(),
            Tab::WafStress => self.tabs.waf_stress.build_task_config(),
            Tab::Scan => self.tabs.scan.build_task_config(),
            Tab::Packet => self.tabs.packet.build_task_config(),
            Tab::GraphQl => self.tabs.graphql.build_task_config(),
            Tab::OAuth => self.tabs.oauth.build_task_config(),
            Tab::Auth => self.tabs.auth.build_task_config(),
            Tab::Cluster => self.tabs.cluster.build_task_config(),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.tabs.hunt.build_task_config(),
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.tabs.browser.build_task_config(),
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.tabs.compliance.build_task_config(),
            #[cfg(feature = "database")]
            Tab::Storage => self.tabs.storage.build_task_config(),
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => self.tabs.integrations.build_task_config(),
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => self.tabs.workflow.build_task_config(),
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => self.tabs.vuln.build_task_config(),
            #[cfg(feature = "wireless")]
            Tab::Wireless => self.tabs.wireless.build_task_config(),
            _ => None,
        }
    }

    pub fn handle_escape(&mut self) {
        if self.overlay.show_help {
            self.overlay.show_help = false;
            return;
        }
        if self.mode == InputMode::Insert {
            self.mode = InputMode::Normal;
        }
        self.dispatcher_mut().handle_escape();
    }

    pub fn handle_char(&mut self, c: char) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_char(c);
    }

    pub fn handle_backspace(&mut self) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_backspace();
    }

    pub fn handle_delete(&mut self) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_delete();
    }

    pub fn handle_autocomplete(&mut self) -> bool {
        if self.overlay.show_help || self.mode != InputMode::Insert {
            return false;
        }
        self.dispatcher_mut().handle_autocomplete()
    }

    pub fn handle_up(&mut self) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_up();
    }

    pub fn handle_down(&mut self) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_down();
    }

    pub fn handle_left(&mut self) {
        if self.overlay.show_help {
            return;
        }
        if !self.dispatcher_mut().handle_left() {
            tracing::trace!("handle_left at left edge");
        }
    }

    pub fn handle_right(&mut self) {
        if self.overlay.show_help {
            return;
        }
        if !self.dispatcher_mut().handle_right() {
            tracing::trace!("handle_right at right edge");
        }
    }

    pub fn handle_focus_next(&mut self) {
        if self.overlay.show_help {
            return;
        }
        let input_focused = {
            let mut dispatcher = self.dispatcher_mut();
            dispatcher.handle_focus_next();
            dispatcher.is_input_focused()
        };
        if input_focused {
            self.mode = InputMode::Insert;
        } else {
            self.mode = InputMode::Normal;
        }
    }

    pub fn handle_focus_prev(&mut self) {
        if self.overlay.show_help {
            return;
        }
        let input_focused = {
            let mut dispatcher = self.dispatcher_mut();
            dispatcher.handle_focus_prev();
            dispatcher.is_input_focused()
        };
        if input_focused {
            self.mode = InputMode::Insert;
        } else {
            self.mode = InputMode::Normal;
        }
    }

    pub fn handle_left_or_prev_tab(&mut self) -> bool {
        if self.overlay.show_help {
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
        if self.overlay.show_help {
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
            self.tabs.settings.save_config();
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
        self.overlay.pending_action = Some(action);
    }

    pub fn confirm_action(&mut self) {
        if let Some(action) = self.overlay.pending_action.take() {
            action.execute(self);
        }
    }

    pub fn cancel_action(&mut self) {
        self.overlay.pending_action = None;
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
        self.overlay.pending_action.is_some()
    }

    /// Policy confirmation (RequireConfirmation) is pending.
    pub fn is_policy_confirm_visible(&self) -> bool {
        self.overlay.pending_policy.is_some()
    }

    /// Request a policy confirmation overlay for a RequireConfirmation outcome.
    /// Captures the descriptor, decision, computed classes, and the would-be TaskConfig.
    pub fn request_policy_confirmation(
        &mut self,
        descriptor: OperationDescriptor,
        decision: eggsec::config::PolicyDecision,
        captured_task_config: Option<crate::workers::TaskConfig>,
    ) {
        let required =
            confirmation_classes_for(&descriptor, &decision, &self.enforcement.execution_policy);
        self.overlay.pending_policy = Some(crate::app::confirmation::PendingPolicyConfirmation {
            descriptor,
            decision,
            required_classes: required,
            reason_input: String::new(),
            captured_task_config,
        });
        self.needs_redraw = true;
    }

    /// Confirm the pending policy override (if any) and spawn the captured task if permitted.
    /// Mirrors CLI CommandContext::evaluate_and_enforce_operation narrow --yes + dedicated flags logic.
    pub fn confirm_policy_action(&mut self) {
        if let Some(pending) = self.overlay.pending_policy.take() {
            let mut mo = ManualOverride::default();
            // Build override from the required classes using narrow semantics.
            // OutOfScope / TargetExpansion are satisfied by "low-risk scope discretion" (no dedicated flag needed in TUI for now;
            // we treat the act of confirming the policy popup as the operator discretion, equivalent to --yes for those two).
            // All other classes require the user to have provided a reason or we simply record the act.
            // To stay faithful to the narrow model, we set the specific allow_* for everything except the two low-risk scope ones.
            for c in &pending.required_classes {
                match c {
                    eggsec::config::ConfirmationClass::OutOfScope
                    | eggsec::config::ConfirmationClass::TargetExpansion => {
                        // low-risk scope discretion: the confirm itself acts like narrow --yes for these
                        // (no allow_out_of_scope flag is exposed in TUI UI yet; the popup confirm is the signal)
                        // We still record it; enforcement will see it via the record path below.
                    }
                    eggsec::config::ConfirmationClass::ExplicitExclusion => {
                        mo.allow_explicit_exclusion = true;
                    }
                    eggsec::config::ConfirmationClass::HighRisk => {
                        mo.allow_high_risk = true;
                    }
                    eggsec::config::ConfirmationClass::NonBaselineCapability => {
                        mo.allow_nonbaseline_capability = true;
                    }
                    eggsec::config::ConfirmationClass::PrivateResolution => {
                        mo.allow_private_resolution = true;
                    }
                    eggsec::config::ConfirmationClass::CrossHostRedirect => {
                        mo.allow_cross_host_redirect = true;
                    }
                }
            }
            mo.reason = if pending.reason_input.trim().is_empty() {
                None
            } else {
                Some(pending.reason_input.clone())
            };
            mo.assume_yes = false; // TUI confirm popup never sets broad assume_yes; narrow by design

            // Re-evaluate using the same central path the CLI uses (EnforcementContext).
            let outcome = self.enforcement.evaluate(&pending.descriptor);
            match outcome {
                EnforcementOutcome::Allow(d) | EnforcementOutcome::Warn(d) => {
                    // already allowed; unusual after we got RequireConfirmation, but proceed
                    tracing::info!(operation = %pending.descriptor.operation, "policy allowed after re-eval (no override needed)");
                    if let Some(cfg) = pending.captured_task_config {
                        self.spawn_task(Some(cfg));
                    }
                    self.needs_redraw = true;
                    // Record a decision if it carries the audit fields (best effort)
                    let _ = d; // nothing more to do
                }
                EnforcementOutcome::RequireConfirmation(decision) => {
                    // Check if our constructed override permits all required classes
                    let required_now = confirmation_classes_for(
                        &pending.descriptor,
                        &decision,
                        &self.enforcement.execution_policy,
                    );
                    let permitted = required_now.iter().all(|c| mo.permits(*c));
                    if permitted {
                        let classes_vec = eggsec::config::confirmation_class_strings(&required_now);
                        tracing::warn!(
                            operation = %decision.operation,
                            target = ?decision.target_original,
                            classes = ?classes_vec,
                            reason = ?mo.reason,
                            "manual enforcement override accepted (TUI)"
                        );
                        let mut out = decision.clone();
                        if !out.manual_override_used {
                            out = out.with_manual_override_record(mo.reason.clone(), classes_vec);
                        }
                        // Notify user (mirrors CLI "manual enforcement override accepted")
                        self.overlay.notification =
                            Some(crate::app::notifications::Notification::new(
                                format!("Policy override accepted for: {}", out.operation),
                                crate::app::notifications::NotificationSeverity::Warning,
                            ));
                        if let Some(cfg) = pending.captured_task_config {
                            self.spawn_task(Some(cfg));
                        }
                    } else {
                        // Still not permitted (should not happen for the classes we mapped)
                        self.set_error_for_current_tab(crate::app::tab_error::TabError::Target(
                            "Manual override did not satisfy all required confirmation classes"
                                .to_string(),
                        ));
                    }
                    self.needs_redraw = true;
                }
                EnforcementOutcome::Deny(d) => {
                    // Became a hard deny (profile or other change); surface it
                    self.set_error_for_current_tab(crate::app::tab_error::TabError::Target(
                        d.to_human_readable(),
                    ));
                    self.needs_redraw = true;
                }
            }
        }
    }

    /// Cancel/dismiss the pending policy confirmation (no task spawned).
    pub fn cancel_policy_action(&mut self) {
        if self.overlay.pending_policy.is_some() {
            self.overlay.pending_policy = None;
            self.needs_redraw = true;
        }
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
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_word_forward();
    }

    pub fn handle_word_backward(&mut self) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_word_backward();
    }

    pub fn handle_home(&mut self) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_home();
    }

    pub fn handle_end(&mut self) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_end();
    }

    pub fn handle_top(&mut self) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_top();
    }

    pub fn handle_bottom(&mut self) {
        if self.overlay.show_help {
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
        self.task_state.paused = !self.task_state.paused;
    }

    pub fn is_paused(&self) -> bool {
        self.task_state.paused
    }

    pub fn resume(&mut self) {
        self.task_state.paused = false;
    }

    pub fn auto_save_if_due(&mut self) {
        // Don't auto-save while a task is running. The task state is
        // transient and saving mid-scan can write a snapshot that doesn't
        // match the disk reality. The save will fire on the next tick
        // after the task completes.
        if self.task_state.handle.is_some() || self.task_state.tab.is_some() {
            return;
        }
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
        self.theme_load.mark_user_changed();
        crate::theme::sync_theme_to_thread_local(self.theme_manager.current());
        self.update_settings_theme_selector();
        self.needs_redraw = true;
        // Acknowledge the change so the user knows Ctrl+T worked.
        self.overlay.notification = Some(Notification::new(
            format!("Theme: {}", self.theme_manager.current().name),
            NotificationSeverity::Info,
        ));
    }

    pub fn update_settings_theme_selector(&mut self) {
        let themes: Vec<(String, String)> = self
            .theme_manager
            .list_themes()
            .iter()
            .map(|id| (id.to_string(), display_theme_name(id)))
            .collect();
        let current = self.theme_manager.current_name().to_string();
        self.tabs.settings.set_available_themes(&themes, &current);
    }

    pub fn current_theme(&self) -> &crate::theme::Theme {
        self.theme_manager.current()
    }

    pub fn toggle_quick_switch(&mut self) {
        if self.is_any_overlay_active() {
            return;
        }
        self.quick_switch.visible = true;
        self.quick_switch.query.clear();
        self.quick_switch.selected = 0;
        self.needs_redraw = true;
    }

    pub fn close_quick_switch(&mut self) {
        self.quick_switch.visible = false;
        self.quick_switch.query.clear();
        self.needs_redraw = true;
    }

    pub fn is_quick_switch_visible(&self) -> bool {
        self.quick_switch.visible
    }

    pub fn get_quick_switch_results(&self) -> Vec<&'static Tab> {
        let query = self.quick_switch.query.to_lowercase();
        if query.is_empty() {
            return Tab::all().iter().collect();
        }

        use crate::utils::fuzzy::fuzzy_score;
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
        self.overlay.show_search
    }

    /// Check if HTTP options popup is visible
    pub fn is_http_options_visible(&self) -> bool {
        self.overlay.show_http_options
    }

    /// Check if help popup is visible
    pub fn is_help_visible(&self) -> bool {
        self.overlay.show_help
    }

    /// Get the topmost overlay based on precedence:
    /// 1. Policy confirmation (pending_policy) - highest, for RequireConfirmation + manual override
    /// 2. Confirm popup (pending_action) - UI destructive actions
    /// 3. Command palette
    /// 4. Quick switch
    /// 5. Search
    /// 6. HTTP options
    /// 7. Help
    ///    Returns None if no overlay is active
    pub fn topmost_overlay(&self) -> Option<OverlayType> {
        if self.is_policy_confirm_visible() {
            Some(OverlayType::PolicyConfirm)
        } else if self.is_confirm_popup_visible() {
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

    // ---------------------------------------------------------------------
    // Phase 1: UiAction apply layer (central mutation point for key-driven UI)
    // ---------------------------------------------------------------------

    /// Apply a single `UiAction`. This is the canonical mutation site for
    /// actions originating from `KeyHandler` decode (Phase 1 of the
    /// architecture pass). Existing direct-mutation helpers are preserved
    /// and may be called by `apply_action` for compatibility during the
    /// transition; they continue to be used by tests, command palette
    /// execution, etc.
    ///
    /// All visible-state-changing actions set `needs_redraw = true`.
    /// Clipboard I/O and task spawning are performed here (side effects
    /// belong in apply, not in the pure-ish decode path).
    pub fn apply_action(&mut self, action: UiAction) {
        use crate::app::notifications::{Notification, NotificationSeverity};
        use crate::utils::Clipboard;

        match action {
            UiAction::Noop => {}

            UiAction::Quit => {
                self.should_quit = true;
                self.needs_redraw = true;
            }

            UiAction::StopActiveTask { message } => {
                self.stop_with_message(&message);
                self.needs_redraw = true;
            }

            UiAction::ToggleHelp => {
                self.toggle_help();
                self.needs_redraw = true;
            }

            UiAction::ToggleCommandPalette => {
                self.toggle_command_palette();
                self.needs_redraw = true;
            }

            UiAction::ToggleQuickSwitch => {
                self.toggle_quick_switch();
                self.needs_redraw = true;
            }

            UiAction::CloseQuickSwitch => {
                self.close_quick_switch();
                self.needs_redraw = true;
            }

            UiAction::ToggleSearch { global } => {
                self.toggle_search(global);
                self.needs_redraw = true;
            }

            UiAction::ToggleTheme => {
                self.toggle_theme();
                // toggle_theme already sets notification + needs_redraw
            }

            UiAction::TogglePause => {
                self.toggle_pause();
                self.needs_redraw = true;
            }

            UiAction::Resume => {
                self.resume();
                self.needs_redraw = true;
            }

            UiAction::FocusNext => {
                self.handle_focus_next();
                self.needs_redraw = true;
            }

            UiAction::FocusPrev => {
                self.handle_focus_prev();
                self.needs_redraw = true;
            }

            UiAction::PageUp => {
                self.page_up();
                self.needs_redraw = true;
            }

            UiAction::PageDown => {
                self.page_down();
                self.needs_redraw = true;
            }

            UiAction::MoveUp => {
                self.handle_up();
                self.needs_redraw = true;
            }

            UiAction::MoveDown => {
                self.handle_down();
                self.needs_redraw = true;
            }

            UiAction::MoveLeft => {
                self.handle_left();
                self.needs_redraw = true;
            }

            UiAction::MoveRight => {
                self.handle_right();
                self.needs_redraw = true;
            }

            UiAction::MoveTop => {
                self.handle_top();
                self.needs_redraw = true;
            }

            UiAction::MoveBottom => {
                self.handle_bottom();
                self.needs_redraw = true;
            }

            UiAction::MoveWordForward => {
                self.handle_word_forward();
                self.needs_redraw = true;
            }

            UiAction::MoveWordBackward => {
                self.handle_word_backward();
                self.needs_redraw = true;
            }

            UiAction::Home => {
                self.handle_home();
                self.needs_redraw = true;
            }

            UiAction::End => {
                self.handle_end();
                self.needs_redraw = true;
            }

            UiAction::Enter => {
                self.handle_enter();
                self.needs_redraw = true;
            }

            UiAction::Escape => {
                self.handle_escape();
                self.needs_redraw = true;
            }

            UiAction::EnterInsertMode => {
                self.mode = InputMode::Insert;
                self.needs_redraw = true;
            }

            UiAction::InputChar(c) => {
                self.handle_char(c);
                self.needs_redraw = true;
            }

            UiAction::Backspace => {
                self.handle_backspace();
                self.needs_redraw = true;
            }

            UiAction::Delete => {
                self.handle_delete();
                self.needs_redraw = true;
            }

            UiAction::Paste(text) => {
                if !self.has_active_task() {
                    self.dispatcher_mut().handle_paste(&text);
                    self.needs_redraw = true;
                }
            }

            UiAction::Copy => {
                if let Some(text) = self.dispatcher_mut().handle_copy() {
                    if !Clipboard::set(&text) {
                        tracing::warn!("Clipboard write failed");
                    }
                }
                self.needs_redraw = true;
            }

            UiAction::RequestPaste => {
                if !self.has_active_task() {
                    if let Some(text) = Clipboard::get() {
                        self.dispatcher_mut().handle_paste(&text);
                    } else {
                        tracing::debug!("Clipboard read failed or clipboard is empty");
                    }
                    self.needs_redraw = true;
                }
            }

            UiAction::RequestCopy => {
                if let Some(text) = self.dispatcher_mut().handle_copy() {
                    if !Clipboard::set(&text) {
                        tracing::warn!("Clipboard write failed");
                    }
                }
                self.needs_redraw = true;
            }

            UiAction::SelectTab(tab) => {
                self.set_current_tab_if_available(tab);
                self.adjust_tab_scroll();
                self.needs_redraw = true;
            }

            UiAction::NextTab => {
                self.next_tab();
                self.needs_redraw = true;
            }

            UiAction::PrevTab => {
                self.prev_tab();
                self.needs_redraw = true;
            }

            UiAction::ToggleBookmark(tab) => {
                self.toggle_bookmark(tab);
                self.overlay.notification = Some(Notification::new(
                    format!("Bookmarked: {}", tab.title()),
                    NotificationSeverity::Info,
                ));
                self.needs_redraw = true;
            }

            UiAction::CycleExportFormat => {
                self.cycle_export_format();
                self.overlay.notification = Some(Notification::new(
                    format!("Export format: {}", self.export_format),
                    NotificationSeverity::Info,
                ));
                self.needs_redraw = true;
            }

            UiAction::ExportResults => {
                self.export_results();
                self.needs_redraw = true;
            }

            UiAction::ResetCurrent => {
                if !self.has_active_task() {
                    if self.current_tab == Tab::History {
                        self.request_confirmation(PendingAction::ClearHistory);
                    } else {
                        self.request_confirmation(PendingAction::ResetTab);
                    }
                    self.needs_redraw = true;
                }
            }

            UiAction::SaveSettings => {
                if !self.has_active_task() && self.current_tab == Tab::Settings {
                    self.request_confirmation(PendingAction::SaveSettings);
                    self.needs_redraw = true;
                }
            }

            UiAction::DeleteHistoryEntry => {
                if !self.has_active_task() && self.current_tab == Tab::History {
                    self.request_confirmation(PendingAction::DeleteHistoryEntry);
                    self.needs_redraw = true;
                }
            }

            UiAction::ConfirmPendingAction => {
                self.confirm_action();
                self.needs_redraw = true;
            }

            UiAction::CancelPendingAction => {
                self.cancel_action();
                self.needs_redraw = true;
            }

            UiAction::ConfirmPolicyAction => {
                self.confirm_policy_action();
                self.needs_redraw = true;
            }

            UiAction::CancelPolicyAction => {
                self.cancel_policy_action();
                self.needs_redraw = true;
            }

            UiAction::PolicyReasonChar(c) => {
                if let Some(p) = &mut self.overlay.pending_policy {
                    p.reason_input.push(c);
                    self.needs_redraw = true;
                }
            }

            UiAction::PolicyReasonBackspace => {
                if let Some(p) = &mut self.overlay.pending_policy {
                    p.reason_input.pop();
                    self.needs_redraw = true;
                }
            }

            UiAction::CommandPaletteInput(pal_input) => {
                match pal_input {
                    CommandPaletteInput::Char(c) => {
                        let q = if let Some(ref mut palette) = self.command_palette {
                            palette.query.push(c);
                            let q = palette.query.clone();
                            let max_idx = palette.results.len().saturating_sub(1);
                            if palette.selected_index > max_idx {
                                palette.selected_index = max_idx;
                            }
                            Some(q)
                        } else {
                            None
                        };
                        if let Some(q) = q {
                            self.update_command_palette_query(&q);
                        }
                    }
                    CommandPaletteInput::Backspace => {
                        let new_query = if let Some(ref mut palette) = self.command_palette {
                            let query = palette.query.clone();
                            if !query.is_empty() {
                                palette.query.pop();
                                let new_query = palette.query.clone();
                                let max_idx = palette.results.len().saturating_sub(1);
                                if palette.selected_index > max_idx {
                                    palette.selected_index = max_idx;
                                }
                                Some(new_query)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        if let Some(q) = new_query {
                            self.update_command_palette_query(&q);
                        }
                    }
                    CommandPaletteInput::Enter => {
                        let index = self
                            .command_palette
                            .as_ref()
                            .map(|p| p.selected_index)
                            .unwrap_or(0);
                        self.select_command_palette_item(index);
                    }
                    CommandPaletteInput::Up => {
                        if let Some(ref mut palette) = self.command_palette {
                            if palette.selected_index > 0 {
                                palette.selected_index -= 1;
                            }
                            if palette.selected_index < palette.scroll_offset {
                                palette.scroll_offset = palette.selected_index;
                            }
                        }
                    }
                    CommandPaletteInput::Down => {
                        if let Some(ref mut palette) = self.command_palette {
                            let max_idx = palette.results.len().saturating_sub(1);
                            if palette.selected_index < max_idx {
                                palette.selected_index += 1;
                            }
                            palette.adjust_scroll_for_selection();
                        }
                    }
                    CommandPaletteInput::Tab => {
                        if let Some(ref mut palette) = self.command_palette {
                            let max_idx = palette.results.len().saturating_sub(1);
                            if palette.selected_index < max_idx {
                                palette.selected_index += 1;
                            }
                            palette.adjust_scroll_for_selection();
                        }
                    }
                    CommandPaletteInput::BackTab => {
                        if let Some(ref mut palette) = self.command_palette {
                            if palette.selected_index > 0 {
                                palette.selected_index -= 1;
                            }
                            if palette.selected_index < palette.scroll_offset {
                                palette.scroll_offset = palette.selected_index;
                            }
                        }
                    }
                    CommandPaletteInput::Esc | CommandPaletteInput::Close => {
                        self.toggle_command_palette();
                    }
                }
                self.needs_redraw = true;
            }

            UiAction::QuickSwitchInput(qs_input) => {
                match qs_input {
                    QuickSwitchInput::Char(c) => {
                        self.quick_switch.query.push(c);
                        self.clamp_quick_switch_selection_internal();
                    }
                    QuickSwitchInput::Backspace => {
                        self.quick_switch.query.pop();
                        self.clamp_quick_switch_selection_internal();
                    }
                    QuickSwitchInput::Enter => {
                        let results = self.get_quick_switch_results();
                        if !results.is_empty() && self.quick_switch.selected < results.len() {
                            if let Some(tab) = results.get(self.quick_switch.selected) {
                                self.current_tab = **tab;
                                self.adjust_tab_scroll();
                            }
                        }
                        self.close_quick_switch();
                    }
                    QuickSwitchInput::Up => {
                        if self.quick_switch.selected > 0 {
                            self.quick_switch.selected -= 1;
                        }
                    }
                    QuickSwitchInput::Down => {
                        let results = self.get_quick_switch_results();
                        if self.quick_switch.selected < results.len().saturating_sub(1) {
                            self.quick_switch.selected += 1;
                        }
                    }
                    QuickSwitchInput::PageUp => {
                        let results = self.get_quick_switch_results();
                        if self.quick_switch.selected >= 10 {
                            self.quick_switch.selected -= 10;
                        } else {
                            self.quick_switch.selected = 0;
                        }
                        if !results.is_empty() {
                            self.quick_switch.selected =
                                self.quick_switch.selected.min(results.len() - 1);
                        }
                    }
                    QuickSwitchInput::PageDown => {
                        let results = self.get_quick_switch_results();
                        if !results.is_empty() {
                            self.quick_switch.selected = (self.quick_switch.selected + 10)
                                .min(results.len().saturating_sub(1));
                        }
                    }
                    QuickSwitchInput::Home => {
                        self.quick_switch.selected = 0;
                    }
                    QuickSwitchInput::End => {
                        let results = self.get_quick_switch_results();
                        self.quick_switch.selected = results.len().saturating_sub(1);
                    }
                    QuickSwitchInput::Esc | QuickSwitchInput::Close => {
                        self.close_quick_switch();
                    }
                }
                self.needs_redraw = true;
            }

            UiAction::SearchQueryChar(c) => {
                if self.is_search_visible() {
                    self.search.query.push(c);
                    self.needs_redraw = true;
                }
            }

            UiAction::SearchQueryBackspace => {
                if self.is_search_visible() {
                    self.search.query.pop();
                    self.needs_redraw = true;
                }
            }

            UiAction::SearchQueryClear => {
                if self.is_search_visible() {
                    self.search.query.clear();
                    self.needs_redraw = true;
                }
            }

            UiAction::SearchPerform => {
                if self.is_search_visible() {
                    self.perform_search();
                    self.needs_redraw = true;
                }
            }

            UiAction::HelpScrollUp => {
                if self.is_help_visible() {
                    self.overlay.help_scroll_offset =
                        self.overlay.help_scroll_offset.saturating_sub(1);
                    self.needs_redraw = true;
                }
            }

            UiAction::HelpScrollDown => {
                if self.is_help_visible() {
                    self.overlay.help_scroll_offset =
                        self.overlay.help_scroll_offset.saturating_add(1);
                    self.needs_redraw = true;
                }
            }

            UiAction::HelpScrollTop => {
                if self.is_help_visible() {
                    self.overlay.help_scroll_offset = 0;
                    self.needs_redraw = true;
                }
            }

            UiAction::HelpScrollBottom => {
                if self.is_help_visible() {
                    self.overlay.help_scroll_offset = usize::MAX;
                    self.needs_redraw = true;
                }
            }

            UiAction::HelpScrollPageUp => {
                if self.is_help_visible() {
                    self.overlay.help_scroll_offset =
                        self.overlay.help_scroll_offset.saturating_sub(10);
                    self.needs_redraw = true;
                }
            }

            UiAction::HelpScrollPageDown => {
                if self.is_help_visible() {
                    self.overlay.help_scroll_offset =
                        self.overlay.help_scroll_offset.saturating_add(10);
                    self.needs_redraw = true;
                }
            }

            UiAction::HttpOptionsClose => {
                if self.is_http_options_visible() {
                    self.overlay.show_http_options = false;
                    self.needs_redraw = true;
                }
            }
        }
    }

    /// Apply a batch of actions in order.
    pub fn apply_actions(&mut self, actions: Vec<UiAction>) {
        for a in actions {
            self.apply_action(a);
        }
    }

    /// Internal helper used by QuickSwitch apply (keeps clamp logic in one place).
    fn clamp_quick_switch_selection_internal(&mut self) {
        let results = self.get_quick_switch_results();
        let len = results.len();
        self.quick_switch.selected = if len == 0 {
            0
        } else {
            self.quick_switch.selected.min(len - 1)
        };
    }
}

/// Represents the type of overlay currently shown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayType {
    /// Policy enforcement confirmation (RequireConfirmation under ManualPermissive).
    /// Highest precedence; user must provide matching manual override (narrow --yes or dedicated allow-*).
    PolicyConfirm,
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
    use crate::tabs::Tab;
    use crossterm::event::KeyCode;

    fn create_test_app() -> App {
        App::new_for_testing(create_shared_history())
    }

    fn make_theme(name: &str) -> crate::theme::Theme {
        let colors = crate::theme::builtin::dark_theme().colors;
        crate::theme::Theme {
            mode: crate::theme::ThemeMode::Dark,
            name: name.to_string(),
            colors,
        }
    }

    fn make_theme_install_report(
        loaded_themes: Vec<Result<crate::theme::Theme, crate::theme::loader::ThemeLoadError>>,
    ) -> crate::theme::install::ThemeInstallReport {
        crate::theme::install::ThemeInstallReport {
            theme_dir: None,
            installed: 0,
            skipped_existing: 0,
            loaded: loaded_themes.iter().filter(|result| result.is_ok()).count(),
            errors: Vec::new(),
            loaded_themes,
        }
    }

    #[test]
    fn test_app_new_has_default_values() {
        let app = create_test_app();
        assert_eq!(app.current_tab, Tab::Recon);
        assert!(!app.should_quit);
        assert_eq!(app.mode, InputMode::Normal);
        assert!(!app.overlay.show_help);
        assert!(!app.overlay.show_search);
        assert!(app.search.query.is_empty());
        assert!(app.overlay.pending_action.is_none());
    }

    #[test]
    fn test_new_for_testing_uses_cyber_red_and_display_labels() {
        let app = create_test_app();
        assert_eq!(app.current_theme().name, "cyber-red");
        assert!(app.theme_load.rx.is_none());
        assert!(app.theme_load.handle.is_none());
        assert!(app.theme_load.deferred_theme_name.is_none());
        assert!(!app.theme_load.changed_by_user);

        let items = &app.tabs.settings.theme_selector.items;
        assert!(!items.is_empty());
        assert_eq!(items[0].value, "cyber-red");
        assert_eq!(items[0].label, "Cyber Red");
    }

    #[test]
    fn test_deferred_theme_restore_applies_after_load() {
        let mut app = create_test_app();
        app.theme_load.deferred_theme_name = Some("Catppuccin Mocha".to_string());

        let report = make_theme_install_report(vec![Ok(make_theme("catppuccin-mocha"))]);
        app.handle_theme_install_report(report);

        assert_eq!(app.current_theme().name, "catppuccin-mocha");
        assert_eq!(
            app.tabs.settings.theme_selector.selected_value(),
            Some("catppuccin-mocha")
        );
        assert!(app.theme_load.deferred_theme_name.is_none());
        assert!(!app.theme_load.changed_by_user);
    }

    #[test]
    fn test_deferred_theme_restore_does_not_override_user_change() {
        let mut app = create_test_app();
        assert!(app.theme_manager.set_theme("dark"));
        app.theme_load.mark_user_changed();
        app.theme_load.deferred_theme_name = Some("catppuccin-mocha".to_string());

        let report = make_theme_install_report(vec![Ok(make_theme("catppuccin-mocha"))]);
        app.handle_theme_install_report(report);

        assert_eq!(app.current_theme().name, "dark");
        assert_eq!(
            app.tabs.settings.theme_selector.selected_value(),
            Some("dark")
        );
        assert!(app.theme_load.deferred_theme_name.is_none());
    }

    #[test]
    fn test_theme_loader_handle_is_joined_after_report() {
        let mut app = create_test_app();
        let (tx, rx) = std::sync::mpsc::channel();
        let (sent_tx, sent_rx) = std::sync::mpsc::channel();
        let report = make_theme_install_report(vec![]);

        let handle = std::thread::spawn(move || {
            tx.send(report).unwrap();
            sent_tx.send(()).unwrap();
        });

        sent_rx.recv().unwrap();

        app.theme_load.rx = Some(rx);
        app.theme_load.handle = Some(handle);
        app.update();

        assert!(app.theme_load.rx.is_none());
        assert!(app.theme_load.handle.is_none());
    }

    #[test]
    fn test_theme_loader_disconnected_clears_state() {
        let mut app = create_test_app();
        let (tx, rx) = std::sync::mpsc::channel::<crate::theme::install::ThemeInstallReport>();
        drop(tx);
        let handle = std::thread::spawn(|| {});

        app.theme_load.rx = Some(rx);
        app.theme_load.handle = Some(handle);
        app.update();

        assert!(app.theme_load.rx.is_none());
        assert!(app.theme_load.handle.is_none());
    }

    #[test]
    fn test_spawn_theme_loader_is_guarded() {
        let mut app = create_test_app();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(|| {});

        app.theme_load.rx = Some(rx);
        app.theme_load.handle = Some(handle);
        app.spawn_theme_loader();

        let report = make_theme_install_report(vec![]);
        tx.send(report.clone()).unwrap();
        let received = app.theme_load.rx.as_ref().unwrap().try_recv().unwrap();
        assert_eq!(received.loaded, report.loaded);
        assert_eq!(received.installed, report.installed);
        assert_eq!(received.skipped_existing, report.skipped_existing);

        app.theme_load.handle.take().unwrap().join().unwrap();
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
        assert!(app.overlay.pending_action.is_none());

        app.request_confirmation(PendingAction::ResetTab);
        assert!(app.overlay.pending_action.is_some());
        assert_eq!(app.overlay.pending_action, Some(PendingAction::ResetTab));
    }

    #[test]
    fn test_confirm_action_clears_pending_action() {
        let mut app = create_test_app();
        app.request_confirmation(PendingAction::ResetTab);
        assert!(app.overlay.pending_action.is_some());

        app.confirm_action();
        assert!(app.overlay.pending_action.is_none());
    }

    #[test]
    fn test_cancel_action_clears_pending_action() {
        let mut app = create_test_app();
        app.request_confirmation(PendingAction::ResetTab);
        assert!(app.overlay.pending_action.is_some());

        app.cancel_action();
        assert!(app.overlay.pending_action.is_none());
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
    fn test_policy_confirm_precedence_and_flow() {
        let mut app = create_test_app();
        use eggsec::config::{OperationDescriptor, OperationMode, OperationRisk};

        let desc = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            target: Some("https://example.com".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let decision = eggsec::config::PolicyDecision::denied(
            "fuzz",
            OperationMode::StandardAssessment,
            OperationRisk::Intrusive,
            vec![eggsec::config::IntendedUse::WebAssessment],
            "high risk requires confirmation",
        );

        assert!(!app.is_policy_confirm_visible());
        app.request_policy_confirmation(desc.clone(), decision, None);
        assert!(app.is_policy_confirm_visible());
        assert_eq!(
            app.topmost_overlay(),
            Some(crate::OverlayType::PolicyConfirm)
        );

        // Simulate reason typing + confirm (narrow semantics path)
        if let Some(p) = &mut app.overlay.pending_policy {
            p.reason_input.push_str("authorized test");
        }
        app.confirm_policy_action();
        // After confirm we either spawned (no real task here) or surfaced a notification / cleared state.
        // The important contract: the pending is cleared and topmost is no longer PolicyConfirm.
        assert!(!app.is_policy_confirm_visible());
    }

    #[test]
    fn test_policy_confirm_cancel_does_not_spawn() {
        let mut app = create_test_app();
        use eggsec::config::{OperationDescriptor, OperationMode, OperationRisk};

        let desc = OperationDescriptor {
            operation: "stress".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::StressTest,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            target: Some("https://lab.example".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let decision = eggsec::config::PolicyDecision::denied(
            "stress",
            OperationMode::StandardAssessment,
            OperationRisk::StressTest,
            vec![eggsec::config::IntendedUse::WebAssessment],
            "high risk",
        );

        app.request_policy_confirmation(desc, decision, None);
        assert!(app.is_policy_confirm_visible());
        app.cancel_policy_action();
        assert!(!app.is_policy_confirm_visible());
    }

    #[cfg(feature = "wireless-advanced")]
    #[test]
    fn test_wireless_active_descriptor_uses_safeactive_for_dry_run() {
        use eggsec::config::{OperationMode, OperationRisk};

        let mut app = create_test_app();
        app.current_tab = crate::tabs::Tab::Wireless;
        app.tabs.wireless.active_mode = true;
        app.tabs.wireless.inputs.fields[0].value = "wlan0".to_string();
        app.tabs.wireless.dry_run = true;

        let descriptor = app
            .build_current_operation_descriptor()
            .expect("descriptor should be present");
        assert_eq!(descriptor.operation, "wireless-deauth");
        assert_eq!(descriptor.mode, OperationMode::DefenseLab);
        assert_eq!(descriptor.risk, OperationRisk::SafeActive);
    }

    #[cfg(feature = "wireless-advanced")]
    #[test]
    fn test_wireless_active_descriptor_uses_intrusive_for_live_attack() {
        use eggsec::config::{OperationMode, OperationRisk};

        let mut app = create_test_app();
        app.current_tab = crate::tabs::Tab::Wireless;
        app.tabs.wireless.active_mode = true;
        app.tabs.wireless.inputs.fields[0].value = "wlan0".to_string();
        app.tabs.wireless.dry_run = false;

        let descriptor = app
            .build_current_operation_descriptor()
            .expect("descriptor should be present");
        assert_eq!(descriptor.operation, "wireless-deauth");
        assert_eq!(descriptor.mode, OperationMode::DefenseLab);
        assert_eq!(descriptor.risk, OperationRisk::Intrusive);
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
    fn test_search_query_set_and_cleared() {
        let mut app = create_test_app();
        assert!(app.search.query.is_empty());

        app.search.query = "test query".to_string();
        assert_eq!(app.search.query, "test query");

        app.search.query.clear();
        assert!(app.search.query.is_empty());
    }

    #[test]
    fn test_show_http_options_toggle() {
        let mut app = create_test_app();
        assert!(!app.overlay.show_http_options);

        app.overlay.show_http_options = true;
        assert!(app.overlay.show_http_options);

        app.overlay.show_http_options = false;
        assert!(!app.overlay.show_http_options);
    }

    #[test]
    fn test_help_context_default() {
        let app = create_test_app();
        assert_eq!(app.help_context, crate::help::HelpContext::Normal);
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

        app.overlay.show_search = true;
        assert!(app.is_search_visible());

        app.overlay.show_search = false;
        assert!(!app.is_search_visible());
    }

    #[test]
    fn test_http_options_visible() {
        let mut app = create_test_app();
        assert!(!app.is_http_options_visible());

        app.overlay.show_http_options = true;
        assert!(app.is_http_options_visible());

        app.overlay.show_http_options = false;
        assert!(!app.is_http_options_visible());
    }

    #[test]
    fn test_help_visible() {
        let mut app = create_test_app();
        assert!(!app.is_help_visible());

        app.overlay.show_help = true;
        assert!(app.is_help_visible());

        app.overlay.show_help = false;
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
        app.overlay.show_help = true;
        app.overlay.show_search = true;
        app.overlay.show_http_options = true;

        // Confirm popup should take precedence
        app.request_confirmation(PendingAction::ResetTab);
        assert_eq!(app.topmost_overlay(), Some(OverlayType::ConfirmPopup));
    }

    #[test]
    fn test_h_key_closes_http_options_overlay() {
        let mut app = create_test_app();
        // Show HTTP options
        app.overlay.show_http_options = true;
        assert!(app.is_http_options_visible());

        // Simulate 'h' key press behavior (from runner.rs lines 385-387)
        if app.is_http_options_visible() {
            app.overlay.show_http_options = false;
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
        let initial_focus = app.tabs.settings.focus_area;
        let initial_tab = app.current_tab;

        app.handle_right();

        assert_eq!(app.current_tab, initial_tab);
        assert_ne!(app.tabs.settings.focus_area, initial_focus);
    }

    #[test]
    fn test_handle_enter_resyncs_mode_when_input_blurs() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.mode = InputMode::Insert;
        app.tabs.recon.inputs.focus(0);

        app.handle_enter();

        assert_eq!(app.mode, InputMode::Normal);
        assert!(!app.tabs.recon.inputs.is_focused());
    }
}
