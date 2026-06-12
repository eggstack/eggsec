use std::sync::Arc;

use crate::help::{CommandPalette, CommandPaletteResult};
use crate::tabs::Tab;

fn command_to_tab(command: &str) -> Option<Tab> {
    match command {
        "history" => Some(Tab::History),
        "settings" => Some(Tab::Settings),
        "dashboard" => Some(Tab::Dashboard),
        "recon" => Some(Tab::Recon),
        "load" => Some(Tab::Load),
        "ports" | "port" | "portscan" => Some(Tab::ScanPorts),
        "endpoints" | "endpoint" => Some(Tab::ScanEndpoints),
        "fingerprint" | "fingerprinting" => Some(Tab::Fingerprint),
        "fuzz" | "fuzzing" => Some(Tab::Fuzz),
        "waf" => Some(Tab::Waf),
        "wafstress" | "waf-stress" => Some(Tab::WafStress),
        "pipeline" | "scan" => Some(Tab::Scan),
        "resume" | "session" => Some(Tab::Resume),
        "proxy" => Some(Tab::Proxy),
        "packet" => Some(Tab::Packet),
        "graphql" => Some(Tab::GraphQl),
        "oauth" => Some(Tab::OAuth),
        "auth" | "auth-test" => Some(Tab::Auth),
        "cluster" => Some(Tab::Cluster),
        "stress" => Some(Tab::Stress),
        "report" => Some(Tab::Report),
        #[cfg(feature = "nse")]
        "nse" => Some(Tab::Nse),
        #[cfg(feature = "advanced-hunting")]
        "hunt" => Some(Tab::Hunt),
        #[cfg(feature = "headless-browser")]
        "browser" => Some(Tab::Browser),
        #[cfg(feature = "compliance")]
        "compliance" => Some(Tab::Compliance),
        #[cfg(feature = "database")]
        "storage" => Some(Tab::Storage),
        #[cfg(feature = "external-integrations")]
        "integrations" => Some(Tab::Integrations),
        #[cfg(feature = "finding-workflow")]
        "workflow" => Some(Tab::Workflow),
        #[cfg(feature = "vuln-management")]
        "vuln" => Some(Tab::Vuln),
        #[cfg(feature = "wireless")]
        "wireless" | "wifi" => Some(Tab::Wireless),
        _ => None,
    }
}

fn filter_commands_by_availability(entries: &mut Arc<Vec<CommandPaletteResult>>) {
    let available_tabs = Tab::all();
    let mut filtered = Vec::new();
    for entry in entries.iter() {
        if let Some(tab) = command_to_tab(&entry.command) {
            if available_tabs.contains(&tab) {
                filtered.push(entry.clone());
            }
        } else {
            filtered.push(entry.clone()); // Non-tab commands are always available
        }
    }
    *entries = Arc::new(filtered);
}

impl super::App {
    pub(super) fn toggle_command_palette(&mut self) {
        if let Some(ref mut palette) = self.command_palette {
            palette.visible = !palette.visible;
            if palette.visible {
                palette.query.clear();
                palette.results = self.help_manager.get_command_palette_entries().clone();
                filter_commands_by_availability(&mut palette.results);
                palette.selected_index = 0;
                palette.scroll_offset = 0;
            }
        } else {
            let mut palette =
                CommandPalette::new(self.help_manager.get_command_palette_entries().clone());
            palette.visible = true;
            self.command_palette = Some(palette);
        }
    }

    pub(super) fn update_command_palette_query(&mut self, query: &str) {
        if let Some(ref mut palette) = self.command_palette {
            palette.query = query.to_string();
            palette.results = Arc::new(self.help_manager.search_commands(query));
            filter_commands_by_availability(&mut palette.results);
            palette.selected_index = 0;
            palette.scroll_offset = 0;
        }
    }

    pub(super) fn select_command_palette_item(&mut self, index: usize) {
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

    pub(super) fn execute_command(&mut self, command: &str) {
        if let Some(tab) = command_to_tab(command) {
            if !self.set_current_tab_if_available(tab) {
                tracing::debug!("Command target tab not available: {:?}", tab);
            }
            return;
        }

        match command {
            "quit" | "exit" if !self.is_running() => {
                self.should_quit = true;
            }
            "stop" | "stop-task" => {
                self.stop();
            }
            "pause" | "pause-task" => {
                if self.has_active_task() && !self.is_paused() {
                    self.toggle_pause();
                } else if !self.has_active_task() {
                    self.overlay.notification = Some(super::notifications::Notification::new(
                        "No active task to pause".to_string(),
                        super::notifications::NotificationSeverity::Warning,
                    ));
                }
            }
            "resume" | "resume-task" => {
                if self.has_active_task() && self.is_paused() {
                    self.resume();
                } else if !self.has_active_task() {
                    self.overlay.notification = Some(super::notifications::Notification::new(
                        "No active task to resume".to_string(),
                        super::notifications::NotificationSeverity::Warning,
                    ));
                }
            }
            "jump-active" => {
                if let Some(t) = self.active_task_tab() {
                    let _ = self.set_current_tab_if_available(t);
                } else {
                    self.overlay.notification = Some(super::notifications::Notification::new(
                        "No active task to jump to".to_string(),
                        super::notifications::NotificationSeverity::Info,
                    ));
                }
            }
            "reset" => {
                self.reset_current_tab();
            }
            "save" | "save-settings" => {
                if self.current_tab == Tab::Settings && !self.has_active_task() {
                    self.request_confirmation(super::confirmation::PendingAction::SaveSettings);
                } else if self.current_tab == Tab::Settings && self.has_active_task() {
                    self.overlay.notification = Some(super::notifications::Notification::new(
                        "Cannot save settings while a task is running".to_string(),
                        super::notifications::NotificationSeverity::Warning,
                    ));
                } else {
                    self.overlay.notification = Some(super::notifications::Notification::new(
                        "Save settings is only available on the Settings tab".to_string(),
                        super::notifications::NotificationSeverity::Info,
                    ));
                }
            }
            "help" | "help-current" => {
                self.toggle_help();
            }
            "search" | "open-search" => {
                self.toggle_search(true);
            }
            "global-search" => {
                self.toggle_search(false);
            }
            "palette" => {
                self.toggle_command_palette();
            }
            "export" => {
                self.export_results();
            }
            "cycle-export" => {
                self.cycle_export_format();
                self.overlay.notification = Some(super::notifications::Notification::new(
                    format!("Export format: {}", self.export_format),
                    super::notifications::NotificationSeverity::Info,
                ));
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
                self.overlay.show_http_options = !self.overlay.show_http_options;
            }
            "theme" => {
                self.toggle_theme();
            }
            "run" | "run-current" => {
                if !self.is_running() {
                    self.handle_enter();
                } else {
                    self.overlay.notification = Some(super::notifications::Notification::new(
                        "Cannot run while a task is active".to_string(),
                        super::notifications::NotificationSeverity::Warning,
                    ));
                }
            }
            "quick-switch" | "open-quick" => {
                self.toggle_quick_switch();
            }
            "copy-cli" | "copy-cli-equivalent" => {
                if let Some(cmd) = self.copy_cli_equivalent() {
                    if crate::utils::Clipboard::set(&cmd) {
                        let short = if cmd.len() > 60 {
                            format!("{}...", &cmd[..57])
                        } else {
                            cmd.clone()
                        };
                        self.overlay.notification = Some(super::notifications::Notification::new(
                            format!("CLI command copied: {}", short),
                            super::notifications::NotificationSeverity::Info,
                        ));
                    } else {
                        tracing::warn!("Clipboard write failed for copy-cli");
                        self.overlay.notification = Some(super::notifications::Notification::new(
                            "Clipboard write failed".to_string(),
                            super::notifications::NotificationSeverity::Warning,
                        ));
                    }
                } else {
                    self.overlay.notification = Some(super::notifications::Notification::new(
                        "No CLI equivalent for current tab".to_string(),
                        super::notifications::NotificationSeverity::Info,
                    ));
                }
            }
            "reload-scope" => {
                self.overlay.notification = Some(super::notifications::Notification::new(
                    "Reload scope/config not supported in this build (use CLI or restart TUI)"
                        .to_string(),
                    super::notifications::NotificationSeverity::Info,
                ));
            }
            "clear-history" | "delete-history" => {
                if self.current_tab == Tab::History && !self.has_active_task() {
                    if command == "clear-history" {
                        self.request_confirmation(super::confirmation::PendingAction::ClearHistory);
                    } else {
                        self.request_confirmation(
                            super::confirmation::PendingAction::DeleteHistoryEntry,
                        );
                    }
                } else if self.current_tab == Tab::History && self.has_active_task() {
                    self.overlay.notification = Some(super::notifications::Notification::new(
                        "Cannot modify history while a task is running".to_string(),
                        super::notifications::NotificationSeverity::Warning,
                    ));
                } else {
                    self.overlay.notification = Some(super::notifications::Notification::new(
                        "History commands are only available on the History tab".to_string(),
                        super::notifications::NotificationSeverity::Info,
                    ));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn get_command_palette(&self) -> Option<&CommandPalette> {
        self.command_palette.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{create_shared_history, App};
    use crate::tabs::Tab;

    fn create_test_app() -> App {
        App::new_for_testing(create_shared_history())
    }

    #[test]
    fn test_execute_command_quit_when_not_running() {
        let mut app = create_test_app();
        app.should_quit = false;
        app.execute_command("quit");
        assert!(app.should_quit);
    }

    #[test]
    fn test_execute_command_exit_alias() {
        let mut app = create_test_app();
        app.should_quit = false;
        app.execute_command("exit");
        assert!(app.should_quit);
    }

    #[test]
    fn test_execute_command_navigation_recon() {
        let mut app = create_test_app();
        app.current_tab = Tab::Fuzz;
        app.execute_command("recon");
        assert_eq!(app.current_tab, Tab::Recon);
    }

    #[test]
    fn test_execute_command_navigation_load() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("load");
        assert_eq!(app.current_tab, Tab::Load);
    }

    #[test]
    fn test_execute_command_navigation_ports() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("ports");
        assert_eq!(app.current_tab, Tab::ScanPorts);
    }

    #[test]
    fn test_execute_command_navigation_port_alias() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("portscan");
        assert_eq!(app.current_tab, Tab::ScanPorts);
    }

    #[test]
    fn test_execute_command_navigation_endpoints() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("endpoints");
        assert_eq!(app.current_tab, Tab::ScanEndpoints);
    }

    #[test]
    fn test_execute_command_navigation_fuzz() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("fuzz");
        assert_eq!(app.current_tab, Tab::Fuzz);
    }

    #[test]
    fn test_execute_command_navigation_fuzzing_alias() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("fuzzing");
        assert_eq!(app.current_tab, Tab::Fuzz);
    }

    #[test]
    fn test_execute_command_navigation_waf() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("waf");
        assert_eq!(app.current_tab, Tab::Waf);
    }

    #[test]
    fn test_execute_command_navigation_wafstress() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("wafstress");
        assert_eq!(app.current_tab, Tab::WafStress);
    }

    #[test]
    fn test_execute_command_navigation_scan() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("scan");
        assert_eq!(app.current_tab, Tab::Scan);
    }

    #[test]
    fn test_execute_command_navigation_pipeline() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("pipeline");
        assert_eq!(app.current_tab, Tab::Scan);
    }

    #[test]
    fn test_execute_command_navigation_resume() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("resume");
        assert_eq!(app.current_tab, Tab::Resume);
    }

    #[test]
    fn test_execute_command_navigation_history() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("history");
        assert_eq!(app.current_tab, Tab::History);
    }

    #[test]
    fn test_execute_command_navigation_settings() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("settings");
        assert_eq!(app.current_tab, Tab::Settings);
    }

    #[test]
    fn test_execute_command_navigation_dashboard() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("dashboard");
        assert_eq!(app.current_tab, Tab::Dashboard);
    }

    #[test]
    fn test_execute_command_next_tab() {
        let mut app = create_test_app();
        let initial_tab = app.current_tab;
        app.execute_command("next");
        assert_eq!(app.current_tab, initial_tab.next());
    }

    #[test]
    fn test_execute_command_prev_tab() {
        let mut app = create_test_app();
        let initial_tab = app.current_tab;
        app.execute_command("prev");
        assert_eq!(app.current_tab, initial_tab.prev());
    }

    #[test]
    fn test_execute_command_toggle_help() {
        let mut app = create_test_app();
        assert!(!app.overlay.show_help);
        app.execute_command("help");
        assert!(app.overlay.show_help);
    }

    #[test]
    fn test_execute_command_toggle_search() {
        let mut app = create_test_app();
        assert!(!app.overlay.show_search);
        app.execute_command("search");
        assert!(app.overlay.show_search);
        assert!(app.search.is_global); // Command palette does global search
    }

    #[test]
    fn test_execute_command_toggle_http_options() {
        let mut app = create_test_app();
        assert!(!app.overlay.show_http_options);
        app.execute_command("http");
        assert!(app.overlay.show_http_options);
        app.execute_command("http");
        assert!(!app.overlay.show_http_options);
    }

    #[test]
    fn test_execute_command_unknown_is_ignored() {
        let mut app = create_test_app();
        let initial_tab = app.current_tab;
        app.execute_command("unknown-command");
        assert_eq!(app.current_tab, initial_tab);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_execute_command_page_up() {
        let mut app = create_test_app();
        app.execute_command("page-up");
    }

    #[test]
    fn test_execute_command_page_down() {
        let mut app = create_test_app();
        app.execute_command("page-down");
    }

    #[test]
    fn test_get_command_palette_initially_none() {
        let app = create_test_app();
        assert!(app.get_command_palette().is_none());
    }

    #[test]
    fn test_toggle_command_palette_creates_palette() {
        let mut app = create_test_app();
        assert!(app.command_palette.is_none());
        app.toggle_command_palette();
        assert!(app.command_palette.is_some());
        let palette = app.get_command_palette().unwrap();
        assert!(palette.visible);
        assert!(palette.query.is_empty());
        assert_eq!(palette.selected_index, 0);
    }

    #[test]
    fn test_toggle_command_palette_toggles_visibility() {
        let mut app = create_test_app();
        app.toggle_command_palette();
        assert!(app.get_command_palette().unwrap().visible);

        app.toggle_command_palette();
        assert!(!app.get_command_palette().unwrap().visible);
    }

    #[test]
    fn test_update_command_palette_query() {
        let mut app = create_test_app();
        app.toggle_command_palette();
        app.update_command_palette_query("test");
        let palette = app.get_command_palette().unwrap();
        assert_eq!(palette.query, "test");
    }

    #[test]
    fn test_toggle_command_palette_resets_state() {
        let mut app = create_test_app();
        app.toggle_command_palette();
        app.toggle_command_palette();
        app.toggle_command_palette();
        let palette = app.get_command_palette().unwrap();
        assert_eq!(palette.selected_index, 0);
        assert_eq!(palette.scroll_offset, 0);
    }

    #[test]
    fn test_command_to_tab_cluster() {
        use super::command_to_tab;
        assert_eq!(command_to_tab("cluster"), Some(Tab::Cluster));
    }

    #[test]
    fn test_command_to_tab_all_tabs_mappable() {
        use super::command_to_tab;
        let known_commands = [
            ("recon", Tab::Recon),
            ("load", Tab::Load),
            ("ports", Tab::ScanPorts),
            ("endpoints", Tab::ScanEndpoints),
            ("fingerprint", Tab::Fingerprint),
            ("fuzz", Tab::Fuzz),
            ("waf", Tab::Waf),
            ("wafstress", Tab::WafStress),
            ("scan", Tab::Scan),
            ("resume", Tab::Resume),
            ("proxy", Tab::Proxy),
            ("packet", Tab::Packet),
            ("graphql", Tab::GraphQl),
            ("oauth", Tab::OAuth),
            ("cluster", Tab::Cluster),
            ("auth-test", Tab::Auth),
            ("stress", Tab::Stress),
            ("report", Tab::Report),
            ("history", Tab::History),
            ("settings", Tab::Settings),
            ("dashboard", Tab::Dashboard),
        ];
        for (cmd, expected_tab) in known_commands {
            assert_eq!(
                command_to_tab(cmd),
                Some(expected_tab),
                "command '{}' should map to {:?}",
                cmd,
                expected_tab
            );
        }
    }

    #[test]
    fn test_execute_command_cluster_via_set_current_tab() {
        let mut app = create_test_app();
        app.current_tab = Tab::Dashboard;
        app.execute_command("cluster");
        assert_eq!(
            app.current_tab,
            Tab::Cluster,
            "execute_command('cluster') should switch to Cluster tab"
        );
    }

    #[test]
    fn test_execute_command_navigation_cluster() {
        let mut app = create_test_app();
        app.current_tab = Tab::Fuzz;
        app.execute_command("cluster");
        assert_eq!(app.current_tab, Tab::Cluster);
    }

    #[test]
    fn test_all_tabs_reachable_via_next_tab() {
        let all_tabs = Tab::all();
        for target_tab in all_tabs {
            let app = create_test_app();
            let mut current = app.current_tab;
            let mut found = false;
            for _ in 0..all_tabs.len() {
                if current == *target_tab {
                    found = true;
                    break;
                }
                current = current.next();
            }
            assert!(
                found,
                "Tab {:?} should be reachable via next_tab()",
                target_tab
            );
        }
    }

    #[test]
    fn test_all_tabs_reachable_via_prev_tab() {
        let all_tabs = Tab::all();
        for target_tab in all_tabs {
            let app = create_test_app();
            let mut current = app.current_tab;
            let mut found = false;
            for _ in 0..all_tabs.len() {
                if current == *target_tab {
                    found = true;
                    break;
                }
                current = current.prev();
            }
            assert!(
                found,
                "Tab {:?} should be reachable via prev_tab()",
                target_tab
            );
        }
    }

    #[test]
    fn test_tab_from_stable_id_cluster() {
        assert_eq!(Tab::from_stable_id("cluster"), Some(Tab::Cluster));
    }

    #[test]
    fn test_command_palette_cluster_visibility() {
        let mut app = create_test_app();
        app.toggle_command_palette();
        let palette = app.get_command_palette().unwrap();
        let cluster_results: Vec<_> = palette
            .results
            .iter()
            .filter(|r| r.command == "cluster")
            .collect();
        assert!(
            !cluster_results.is_empty(),
            "Command palette should contain 'cluster' command"
        );
    }

    #[test]
    fn test_execute_command_global_action_theme() {
        let mut app = create_test_app();
        let initial = app.theme_manager.current_name().to_string();
        app.execute_command("theme");
        assert_ne!(
            app.theme_manager.current_name(),
            initial,
            "theme command should cycle theme"
        );
        assert!(app.overlay.notification.is_some());
    }

    #[test]
    fn test_execute_command_tab_action_recon() {
        let mut app = create_test_app();
        app.current_tab = Tab::Load;
        app.execute_command("recon");
        assert_eq!(app.current_tab, Tab::Recon);
    }

    #[test]
    fn test_execute_command_unavailable_clear_history_when_not_on_history() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.execute_command("clear-history");
        assert!(app.overlay.notification.is_some());
        let notif = app.overlay.notification.as_ref().unwrap();
        assert!(notif
            .message
            .contains("History commands are only available on the History tab"));
    }

    #[test]
    fn test_execute_command_run_current_when_idle() {
        let mut app = create_test_app();
        assert!(!app.is_running());
        app.execute_command("run");
        // handle_enter may change mode or start something; we just ensure it does not panic and palette path works
        assert!(
            app.overlay.notification.is_none()
                || !app
                    .overlay
                    .notification
                    .as_ref()
                    .unwrap()
                    .message
                    .contains("Cannot run")
        );
    }

    #[test]
    fn test_execute_command_stop_pause_resume_jump_with_no_task_sets_notification() {
        let mut app = create_test_app();
        assert!(!app.has_active_task());
        app.execute_command("stop");
        // stop is unconditional but safe
        app.execute_command("pause");
        assert!(app.overlay.notification.is_some());
        let msg = app.overlay.notification.as_ref().unwrap().message.clone();
        assert!(msg.contains("No active task"));
        app.execute_command("resume");
        // resume notification may overwrite
        app.execute_command("jump-active");
        assert!(app.overlay.notification.is_some());
    }

    // ===== Phase 8: Copy CLI equivalent tests (AC-mandated cases) =====

    #[test]
    fn test_copy_cli_recon_produces_target_only_by_default() {
        let app = create_test_app();
        // Recon is default current_tab in test app; target empty in new_for_testing
        let cli = app.copy_cli_equivalent();
        assert!(cli.is_some());
        let s = cli.unwrap();
        assert!(s.starts_with("eggsec recon"));
        // No --yes or broad flags
        assert!(!s.contains("--yes"));
        assert!(!s.contains("--allow-"));
        // Default target is empty -> ''
        assert!(s.contains("''") || s.contains("recon "));
    }

    #[test]
    fn test_copy_cli_recon_with_target_and_concurrency() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        // Set target via tab input (first field)
        if let Some(f) = app.tabs.recon.inputs.fields.first_mut() {
            f.value = "example.com".to_string();
        }
        // Set non-default concurrency (second field)
        if let Some(f) = app.tabs.recon.inputs.fields.get_mut(1) {
            f.value = "50".to_string();
        }
        let cli = app.copy_cli_equivalent().unwrap();
        // Safe hostname is unquoted by shell_escape
        assert!(cli.contains("eggsec recon example.com"));
        assert!(cli.contains("--concurrency 50"));
        assert!(!cli.contains("--yes"));
    }

    #[test]
    fn test_copy_cli_scan_ports_with_target_and_ports() {
        let mut app = create_test_app();
        app.current_tab = Tab::ScanPorts;
        if let Some(f) = app.tabs.scan_ports.inputs.fields.first_mut() {
            f.value = "10.0.0.1".to_string();
        }
        if let Some(f) = app.tabs.scan_ports.inputs.fields.get_mut(1) {
            f.value = "22,80,443".to_string();
        }
        let cli = app.copy_cli_equivalent().unwrap();
        // Host unquoted (safe), ports with comma get quoted
        assert!(cli.contains("eggsec scan-ports 10.0.0.1"));
        assert!(cli.contains("--ports '22,80,443'"));
        assert!(!cli.contains("--yes"));
    }

    #[test]
    fn test_copy_cli_intrusive_fuzz_produces_command() {
        let mut app = create_test_app();
        app.current_tab = Tab::Fuzz;
        if let Some(f) = app.tabs.fuzz.inputs.fields.first_mut() {
            f.value = "https://target.test".to_string();
        }
        // Non-default max payloads to trigger option
        if let Some(f) = app.tabs.fuzz.inputs.fields.get_mut(3) {
            f.value = "100".to_string();
        }
        let cli = app.copy_cli_equivalent().unwrap();
        // https:// has safe chars per our escape set, so unquoted
        assert!(cli.contains("eggsec fuzz https://target.test"));
        assert!(cli.contains("--max-payloads 100"));
        assert!(!cli.contains("--yes"));
    }

    #[test]
    fn test_copy_cli_non_executable_tabs_return_none() {
        let mut app = create_test_app();
        // Per plan AC: settings/history/dashboard (Report has cli_command but we treat UI tabs as non for this test)
        for t in [Tab::Settings, Tab::History, Tab::Dashboard] {
            app.current_tab = t;
            let cli = app.copy_cli_equivalent();
            assert!(cli.is_none(), "expected None for {:?}", t);
        }
    }

    #[test]
    fn test_copy_cli_appends_format_when_non_default() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        if let Some(f) = app.tabs.recon.inputs.fields.first_mut() {
            f.value = "target".to_string();
        }
        app.export_format = eggsec::types::OutputFormat::Json;
        let cli = app.copy_cli_equivalent().unwrap();
        assert!(cli.contains("--format json"));
        assert!(!cli.contains("--yes"));
    }

    #[test]
    fn test_copy_cli_omits_broad_policy_flags() {
        let mut app = create_test_app();
        app.current_tab = Tab::Fuzz;
        if let Some(f) = app.tabs.fuzz.inputs.fields.first_mut() {
            f.value = "https://x".to_string();
        }
        // Even if we had overrides in enforcement (TUI never puts them in CLI copy)
        let cli = app.copy_cli_equivalent().unwrap();
        assert!(!cli.contains("--yes"));
        assert!(!cli.contains("--allow-"));
        assert!(!cli.contains("--insecure"));
    }

    #[test]
    fn test_execute_command_copy_cli_non_executable_sets_notification() {
        let mut app = create_test_app();
        app.current_tab = Tab::Settings;
        app.execute_command("copy-cli");
        let n = app.overlay.notification.as_ref().unwrap();
        assert!(n.message.contains("No CLI equivalent"));
    }

    #[test]
    fn test_execute_command_copy_cli_executable_copies_or_fails_gracefully() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        if let Some(f) = app.tabs.recon.inputs.fields.first_mut() {
            f.value = "safe-target".to_string();
        }
        app.execute_command("copy-cli");
        // Either success message or graceful clipboard-fail message (both acceptable per AC)
        let n = app.overlay.notification.as_ref().unwrap();
        assert!(
            n.message.contains("CLI command copied")
                || n.message.contains("Clipboard write failed"),
            "got: {}",
            n.message
        );
    }

    #[cfg(feature = "wireless-advanced")]
    #[test]
    fn test_copy_cli_wireless_active_mode_produces_deauth_command() {
        let mut app = create_test_app();
        app.current_tab = Tab::Wireless;
        if let Some(f) = app.tabs.wireless.inputs.fields.first_mut() {
            f.value = "wlan0".to_string();
        }
        app.tabs.wireless.active_mode = true;
        if let Some(f) = app.tabs.wireless.active_inputs.fields.first_mut() {
            f.value = "AA:BB:CC:DD:EE:FF".to_string();
        }
        if let Some(f) = app.tabs.wireless.active_inputs.fields.get_mut(1) {
            f.value = "11:22:33:44:55:66".to_string();
        }
        // Non-default values
        if let Some(f) = app.tabs.wireless.active_inputs.fields.get_mut(2) {
            f.value = "250".to_string();
        }
        if let Some(f) = app.tabs.wireless.active_inputs.fields.get_mut(3) {
            f.value = "20".to_string();
        }
        app.tabs.wireless.dry_run = false;
        let cli = app.copy_cli_equivalent().unwrap();
        assert!(cli.contains("eggsec wireless wlan0 deauth"));
        assert!(cli.contains("--bssid"));
        assert!(cli.contains("AA:BB:CC:DD:EE:FF"));
        assert!(cli.contains("--client 11:22:33:44:55:66"));
        assert!(cli.contains("--count 250"));
        assert!(cli.contains("--fps 20"));
        // No broad bypass flags.
        assert!(!cli.contains("--allow-"));
        assert!(!cli.contains("--yes"));
        // No --dry-run when live.
        assert!(!cli.contains("--dry-run"));
    }

    #[cfg(feature = "wireless-advanced")]
    #[test]
    fn test_copy_cli_wireless_active_mode_dry_run_includes_flag() {
        let mut app = create_test_app();
        app.current_tab = Tab::Wireless;
        if let Some(f) = app.tabs.wireless.inputs.fields.first_mut() {
            f.value = "wlan0".to_string();
        }
        app.tabs.wireless.active_mode = true;
        if let Some(f) = app.tabs.wireless.active_inputs.fields.first_mut() {
            f.value = "AA:BB:CC:DD:EE:FF".to_string();
        }
        app.tabs.wireless.dry_run = true;
        let cli = app.copy_cli_equivalent().unwrap();
        assert!(cli.contains("eggsec wireless wlan0 deauth"));
        assert!(cli.contains("--bssid AA:BB:CC:DD:EE:FF"));
        assert!(cli.contains("--dry-run"));
        assert!(!cli.contains("--allow-"));
    }

    #[cfg(feature = "wireless-advanced")]
    #[test]
    fn test_copy_cli_wireless_passive_mode_omits_deauth() {
        let mut app = create_test_app();
        app.current_tab = Tab::Wireless;
        if let Some(f) = app.tabs.wireless.inputs.fields.first_mut() {
            f.value = "wlan0".to_string();
        }
        // active_mode stays false (default)
        let cli = app.copy_cli_equivalent().unwrap();
        assert!(cli.contains("eggsec wireless wlan0"));
        assert!(!cli.contains("deauth"));
    }
}
