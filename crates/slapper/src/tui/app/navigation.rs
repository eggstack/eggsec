impl super::App {
    pub(super) fn next_tab(&mut self) {
        self.clear_search_on_tab_switch();
        self.current_tab = self.current_tab.next();
    }

    pub(super) fn prev_tab(&mut self) {
        self.clear_search_on_tab_switch();
        self.current_tab = self.current_tab.prev();
    }

    pub(super) fn select_tab(&mut self, index: usize) {
        self.clear_search_on_tab_switch();
        if let Some(tab) = super::tabs::Tab::from_index(index) {
            self.current_tab = tab;
        }
    }

    fn clear_search_on_tab_switch(&mut self) {
        if self.show_search {
            self.restore_search();
            self.show_search = false;
            self.search_query.clear();
        }
    }

    pub(super) fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        if self.show_help {
            self.help_tab = Some(self.current_tab);
        } else {
            self.help_tab = None;
        }
    }

    pub(super) fn toggle_search(&mut self) {
        if self.show_search {
            self.restore_search();
        }
        self.show_search = !self.show_search;
        if self.show_search {
            self.search_query.clear();
        }
    }

    pub(super) fn perform_search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }

        if self.current_tab == super::tabs::Tab::History {
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

    pub(super) fn restore_search(&mut self) {
        if self.current_tab == super::tabs::Tab::History {
            if let Some(backup) = self.search_backup.take() {
                if let Ok(mut h) = self.history.lock() {
                    h.entries = backup;
                    h.selected = Some(0);
                    h.update_details_view();
                }
            }
        }
    }

    pub(crate) fn is_help_visible(&self) -> bool {
        self.show_help
    }

    pub(crate) fn get_current_help(&self) -> String {
        match self.current_tab {
            super::tabs::Tab::Recon => {
                "Reconnaissance - Gather intelligence about target domain/IP.".to_string()
            }
            super::tabs::Tab::Load => {
                "Load Testing - Send concurrent HTTP requests to test performance.".to_string()
            }
            super::tabs::Tab::ScanPorts => {
                "Port Scanning - Discover open ports and services.".to_string()
            }
            super::tabs::Tab::ScanEndpoints => {
                "Endpoint Discovery - Find hidden or sensitive endpoints.".to_string()
            }
            super::tabs::Tab::Fingerprint => {
                "Service Fingerprinting - Identify services on open ports.".to_string()
            }
            super::tabs::Tab::Fuzz => {
                "Fuzzing - Test for vulnerabilities using payloads.".to_string()
            }
            super::tabs::Tab::Waf => {
                "WAF Detection - Detect and bypass Web Application Firewalls.".to_string()
            }
            super::tabs::Tab::WafStress => {
                "WAF Stress Testing - Comprehensive WAF testing.".to_string()
            }
            super::tabs::Tab::Scan => {
                "Pipeline Scanning - Run chained security assessment.".to_string()
            }
            super::tabs::Tab::Resume => {
                "Session Resume - Continue previous scan from file.".to_string()
            }
            super::tabs::Tab::Proxy => "Proxy Management - Manage proxy pool.".to_string(),
            super::tabs::Tab::Packet => {
                "Packet Tools - Capture, send, and analyze network packets.".to_string()
            }
            super::tabs::Tab::GraphQl => "GraphQL Security - Test GraphQL endpoints.".to_string(),
            super::tabs::Tab::OAuth => "OAuth/OIDC Security - Test OAuth endpoints.".to_string(),
            super::tabs::Tab::Cluster => {
                "Cluster Management - Manage distributed scanning cluster.".to_string()
            }
            super::tabs::Tab::Stress => {
                "Stress Testing - Run stress/load testing against target.".to_string()
            }
            super::tabs::Tab::Report => {
                "Report - Convert and generate security scan reports.".to_string()
            }
            super::tabs::Tab::Nse => "NSE - Run Nmap NSE scripts.".to_string(),
            super::tabs::Tab::Plugin => {
                "Plugins - Manage and run security scanning plugins.".to_string()
            }
            super::tabs::Tab::Settings => "Settings - Configure application options.".to_string(),
            super::tabs::Tab::History => "History - View previous scan results.".to_string(),
            super::tabs::Tab::Dashboard => "Dashboard - View scan results at a glance.".to_string(),
            super::tabs::Tab::Hunt => {
                "Vulnerability Hunting - Intelligent vulnerability discovery.".to_string()
            }
            super::tabs::Tab::Browser => {
                "Browser Testing - Headless browser security testing.".to_string()
            }
            super::tabs::Tab::Compliance => "Compliance - Generate compliance reports.".to_string(),
            super::tabs::Tab::Storage => "Storage - Database integration.".to_string(),
            super::tabs::Tab::Integrations => {
                "Integrations - Issue tracker integration.".to_string()
            }
            super::tabs::Tab::Workflow => {
                "Workflow - Finding management and SLA tracking.".to_string()
            }
            super::tabs::Tab::Vuln => {
                "Vuln - Vulnerability prioritization and risk scoring.".to_string()
            }
        }
    }
}
