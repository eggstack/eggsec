pub(crate) mod action;
pub(crate) mod action_hints;
pub(crate) mod apply;
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
pub(crate) mod operation;
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
use crossterm::event::KeyCode;
use dispatch::TabDispatcher;
use eggsec::config::{
    confirmation_classes_for, EnforcementContext, EnforcementOutcome, ExecutionPolicy, LoadedScope,
    ManualOverride, OperationDescriptor,
};
use eggsec::types::OutputFormat;
use rustc_hash::FxHashSet;

/// Generates simple delegation methods on App that forward to the current tab's
/// TabInput dispatcher, guarded by the help overlay check.
macro_rules! app_delegate {
    ($( pub fn $name:ident(&mut self) ; )*) => {
        $(
            pub fn $name(&mut self) {
                if self.overlay.show_help {
                    return;
                }
                self.dispatcher_mut().$name();
            }
        )*
    };
}

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
                    self.overlay.notification = Some(Notification::new(
                        format!(
                            "Theme: {}",
                            display_theme_name(&self.theme_manager.current().name)
                        ),
                        NotificationSeverity::Info,
                    ));
                    // Best-effort quick-save so the theme choice persists
                    // across restarts without requiring [s] Save.
                    if let Err(e) = self.session_manager.save_quick(self) {
                        tracing::debug!("Quick-save after theme apply failed: {}", e);
                    }
                } else {
                    tracing::warn!("Unknown theme selected: {}", theme_name);
                    self.overlay.notification = Some(Notification::new(
                        format!("Theme not available: {}", theme_name),
                        NotificationSeverity::Warning,
                    ));
                }
            }
        }

        if is_running {
            if let Some(task_config) = self.build_current_task() {
                if let Some(desc) = self.build_current_operation_descriptor() {
                    self.evaluate_policy_and_dispatch(desc, Some(task_config));
                } else {
                    self.spawn_task(Some(task_config));
                }
            }
        }

        // Post-dispatch retroactive policy gate for direct-launch tabs (packet, stress, auth, cluster, etc.)
        if self.is_direct_launch_tab(self.current_tab) {
            if self.current_tab.as_tab_state(self).is_running() {
                if let Some(desc) = self.build_current_operation_descriptor() {
                    self.evaluate_policy_and_dispatch(desc, None);
                }
            }
        }
    }

    /// Central policy evaluation + dispatch. Handles the Allow/Warn/RequireConfirmation/Deny
    /// outcome from `EnforcementContext::evaluate()`, stopping the tab and requesting
    /// confirmation or surfacing errors as needed.
    fn evaluate_policy_and_dispatch(
        &mut self,
        desc: OperationDescriptor,
        task_config: Option<crate::workers::TaskConfig>,
    ) {
        let outcome = self.enforcement.evaluate(&desc);
        match outcome {
            EnforcementOutcome::Allow(_) | EnforcementOutcome::Warn(_) => {
                if let Some(cfg) = task_config {
                    self.spawn_task(Some(cfg));
                }
                // For direct-launch tabs, the tab already started; nothing more to do.
            }
            EnforcementOutcome::RequireConfirmation(decision) => {
                {
                    let mut tab = self.current_tab;
                    tab.as_tab_input(self).stop();
                }
                self.request_policy_confirmation(desc, decision, task_config);
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


    pub fn handle_escape(&mut self) {
        if self.overlay.show_help {
            self.overlay.show_help = false;
            return;
        }
        if self.mode == InputMode::Insert {
            self.mode = InputMode::Normal;
        }
        self.dispatcher_mut().handle_escape();
        self.maybe_refresh_theme_preview();
    }

    pub fn handle_char(&mut self, c: char) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_char(c);
    }

    app_delegate! {
        pub fn handle_backspace(&mut self);
        pub fn handle_delete(&mut self);
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
        self.maybe_refresh_theme_preview();
    }

    pub fn handle_down(&mut self) {
        if self.overlay.show_help {
            return;
        }
        self.dispatcher_mut().handle_down();
        self.maybe_refresh_theme_preview();
    }

    /// If the Settings tab's theme selector moved, refresh the preview colors.
    fn maybe_refresh_theme_preview(&mut self) {
        if self.current_tab == Tab::Settings
            && self.tabs.settings.needs_theme_preview_refresh
        {
            self.tabs.settings.needs_theme_preview_refresh = false;
            let selected_id = self
                .tabs
                .settings
                .theme_selector
                .selected_value()
                .map(|s| s.to_string())
                .unwrap_or_else(|| self.theme_manager.current_id().to_string());
            self.tabs.settings.resolved_theme_colors = self
                .theme_manager
                .get_theme(&selected_id)
                .map(|t| t.colors.clone());
        }
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
                    eggsec::config::ConfirmationClass::TrafficInterception => {
                        mo.allow_web_proxy = true;
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
                EnforcementOutcome::Allow(_) | EnforcementOutcome::Warn(_) => {
                    // already allowed; unusual after we got RequireConfirmation, but proceed
                    tracing::info!(operation = %pending.descriptor.operation, "policy allowed after re-eval (no override needed)");
                    if let Some(cfg) = pending.captured_task_config {
                        self.spawn_task(Some(cfg));
                    }
                    self.needs_redraw = true;
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

    app_delegate! {
        pub fn handle_word_forward(&mut self);
        pub fn handle_word_backward(&mut self);
        pub fn handle_home(&mut self);
        pub fn handle_end(&mut self);
        pub fn handle_top(&mut self);
        pub fn handle_bottom(&mut self);
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
        // Use the human-readable display name (e.g. "Catppuccin Mocha") rather
        // than the raw canonical ID (e.g. "catppuccin-mocha").
        self.overlay.notification = Some(Notification::new(
            format!("Theme: {}", display_theme_name(&self.theme_manager.current().name)),
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
        let current_id = self.theme_manager.current_id().to_string();
        self.tabs.settings.applied_theme_id = Some(current_id);
        self.tabs.settings.set_available_themes(&themes, &current);
        // Resolve the selected theme's colors for preview rendering.
        let selected_id = self
            .tabs
            .settings
            .theme_selector
            .selected_value()
            .map(|s| s.to_string())
            .unwrap_or_else(|| current.clone());
        self.tabs.settings.resolved_theme_colors = self
            .theme_manager
            .get_theme(&selected_id)
            .map(|t| t.colors.clone());
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

    /// Returns true if any Settings embedded selector is currently open.
    /// Embedded selectors are not overlays (topmost_overlay returns None when
    /// only a selector is open), so this guard prevents normal-mode shortcuts
    /// from leaking into actions while the user is navigating a selector.
    pub fn has_settings_selector_open(&self) -> bool {
        self.current_tab == Tab::Settings
            && (self.tabs.settings.theme_selector.is_open()
                || self.tabs.settings.proxy_rotation_selector.is_open()
                || self.tabs.settings.severity_selector.is_open())
    }

    // ---------------------------------------------------------------------
    // Phase 1: UiAction apply layer — moved to apply.rs
    // ---------------------------------------------------------------------

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

    fn make_theme_record(name: &str) -> crate::theme::install::LoadedThemeRecord {
        crate::theme::install::LoadedThemeRecord {
            result: Ok(make_theme(name)),
            file_stem: name.to_string(),
            source: crate::theme::ThemeSource::Custom,
            contrast_warnings: Vec::new(),
        }
    }

    fn make_theme_install_report(
        loaded_themes: Vec<crate::theme::install::LoadedThemeRecord>,
    ) -> crate::theme::install::ThemeInstallReport {
        let adjusted = loaded_themes.iter().filter(|r| !r.contrast_warnings.is_empty()).count();
        crate::theme::install::ThemeInstallReport {
            theme_dir: None,
            installed: 0,
            skipped_existing: 0,
            loaded: loaded_themes.iter().filter(|r| r.result.is_ok()).count(),
            adjusted,
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

        let report = make_theme_install_report(vec![make_theme_record("catppuccin-mocha")]);
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

        let report = make_theme_install_report(vec![make_theme_record("catppuccin-mocha")]);
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
        let loaded = report.loaded;
        let installed = report.installed;
        let skipped_existing = report.skipped_existing;
        tx.send(report).unwrap();
        let received = app.theme_load.rx.as_ref().unwrap().try_recv().unwrap();
        assert_eq!(received.loaded, loaded);
        assert_eq!(received.installed, installed);
        assert_eq!(received.skipped_existing, skipped_existing);

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
        app.tabs.recon.core.inputs.focus(0);

        app.handle_enter();

        assert_eq!(app.mode, InputMode::Normal);
        assert!(!app.tabs.recon.core.inputs.is_focused());
    }
}
