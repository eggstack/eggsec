use std::sync::Arc;

use crate::tui::help::{CommandPalette, CommandPaletteResult};
use crate::tui::tabs::Tab;

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
        "cluster" => Some(Tab::Cluster),
        "stress" => Some(Tab::Stress),
        "report" => Some(Tab::Report),
        #[cfg(feature = "nse")]
        "nse" => Some(Tab::Nse),
        #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
        "plugin" => Some(Tab::Plugin),
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
            let mut palette = CommandPalette::new(self.help_manager.get_command_palette_entries().clone());
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
                self.toggle_search(true); // Global search from command palette
            }
            "palette" => {
                self.toggle_command_palette();
            }
            "export" => {
                self.export_results();
            }
            "history" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::History);
            }
            "settings" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::Settings);
            }
            "dashboard" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::Dashboard);
            }
            "recon" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::Recon);
            }
            "load" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::Load);
            }
            "ports" | "port" | "portscan" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::ScanPorts);
            }
            "endpoints" | "endpoint" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::ScanEndpoints);
            }
            "fingerprint" | "fingerprinting" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::Fingerprint);
            }
            "fuzz" | "fuzzing" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::Fuzz);
            }
            "waf" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::Waf);
            }
            "wafstress" | "waf-stress" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::WafStress);
            }
            "pipeline" | "scan" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::Scan);
            }
            "resume" | "session" => {
                let _ = self.set_current_tab_if_available(super::tabs::Tab::Resume);
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

    pub(crate) fn get_command_palette(&self) -> Option<&CommandPalette> {
        self.command_palette.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{create_shared_history, App};
    use crate::tui::tabs::Tab;

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
        assert!(!app.show_help);
        app.execute_command("help");
        assert!(app.show_help);
    }

    #[test]
    fn test_execute_command_toggle_search() {
        let mut app = create_test_app();
        assert!(!app.show_search);
        app.execute_command("search");
        assert!(app.show_search);
        assert!(app.search_is_global); // Command palette does global search
    }

    #[test]
    fn test_execute_command_toggle_http_options() {
        let mut app = create_test_app();
        assert!(!app.show_http_options);
        app.execute_command("http");
        assert!(app.show_http_options);
        app.execute_command("http");
        assert!(!app.show_http_options);
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
}
