use crate::tui::help::CommandPalette;

impl super::App {
    pub(super) fn toggle_command_palette(&mut self) {
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

    pub(super) fn update_command_palette_query(&mut self, query: &str) {
        if let Some(ref mut palette) = self.command_palette {
            palette.query = query.to_string();
            palette.results = self.help_manager.search_commands(query);
            palette.selected_index = 0;
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
                self.toggle_search();
            }
            "palette" => {
                self.toggle_command_palette();
            }
            "export" => {
                self.export_results();
            }
            "history" => {
                self.current_tab = super::tabs::Tab::History;
            }
            "settings" => {
                self.current_tab = super::tabs::Tab::Settings;
            }
            "dashboard" => {
                self.current_tab = super::tabs::Tab::Dashboard;
            }
            "recon" => {
                self.current_tab = super::tabs::Tab::Recon;
            }
            "load" => {
                self.current_tab = super::tabs::Tab::Load;
            }
            "ports" | "port" | "portscan" => {
                self.current_tab = super::tabs::Tab::ScanPorts;
            }
            "endpoints" | "endpoint" => {
                self.current_tab = super::tabs::Tab::ScanEndpoints;
            }
            "fingerprint" | "fingerprinting" => {
                self.current_tab = super::tabs::Tab::Fingerprint;
            }
            "fuzz" | "fuzzing" => {
                self.current_tab = super::tabs::Tab::Fuzz;
            }
            "waf" => {
                self.current_tab = super::tabs::Tab::Waf;
            }
            "wafstress" | "waf-stress" => {
                self.current_tab = super::tabs::Tab::WafStress;
            }
            "pipeline" | "scan" => {
                self.current_tab = super::tabs::Tab::Scan;
            }
            "resume" | "session" => {
                self.current_tab = super::tabs::Tab::Resume;
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
