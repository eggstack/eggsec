impl super::App {
    pub(super) fn next_tab(&mut self) {
        self.clear_search_on_tab_switch();
        self.current_tab = self.current_tab.next();
        self.adjust_tab_scroll();
    }

    pub(super) fn prev_tab(&mut self) {
        self.clear_search_on_tab_switch();
        self.current_tab = self.current_tab.prev();
        self.adjust_tab_scroll();
    }

    pub(super) fn select_tab(&mut self, index: usize) {
        self.clear_search_on_tab_switch();
        if let Some(tab) = super::tabs::Tab::from_index(index) {
            let _ = self.set_current_tab_if_available(tab);
        }
    }

    pub(super) fn adjust_tab_scroll(&mut self) {
        use super::tabs::TabWindow;
        let tab_index = self.current_tab.visible_index().unwrap_or(0);
        let window = TabWindow::for_width(self.last_tab_area_width, self.current_tab, self.tab_scroll_offset);
        if tab_index < window.start {
            self.tab_scroll_offset = tab_index as u16;
        } else if tab_index >= window.end {
            self.tab_scroll_offset = (tab_index.saturating_sub(window.max_visible) + 1).max(0) as u16;
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

    pub(super) fn toggle_search(&mut self, is_global: bool) {
        if self.show_search {
            self.restore_search();
        }
        self.show_search = !self.show_search;
        if self.show_search {
            self.search_query.clear();
            self.search_is_global = is_global;
        }
    }

    pub(super) fn perform_search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }

        if self.search_is_global {
            // Perform global search using GlobalSearch
            if let Some(ref mut search) = self.global_search {
                let data = vec![
                    ("Recon", self.recon.target().to_string()),
                    ("Fingerprint", self.fingerprint.target().to_string()),
                    ("Fuzz", self.fuzz.target().to_string()),
                    ("WAF", self.waf.target().to_string()),
                    ("Scan", self.scan.target().to_string()),
                    ("Scan Endpoints", self.scan_endpoints.target().to_string()),
                    ("Scan Ports", self.scan_ports.target().to_string()),
                    ("Stress", self.stress.target().to_string()),
                ];
                search.search_from_strings(&self.search_query, &data);
            }
            // Keep search open to show results
        } else if self.current_tab == super::tabs::Tab::History {
            let query = self.search_query.clone();
            let mut h = self.history.lock(); {

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
            self.show_search = false;
        }
    }

    pub(super) fn restore_search(&mut self) {
        if self.current_tab == super::tabs::Tab::History {
            if let Some(backup) = self.search_backup.take() {
                let mut h = self.history.lock(); {

                    h.entries = backup;
                    h.selected = Some(0);
                    h.update_details_view();
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::super::{create_shared_history, App};
    use crate::tui::tabs::Tab;

    fn create_test_app() -> App {
        App::new_for_testing(create_shared_history())
    }

    #[test]
    fn test_next_tab_cycles_forward() {
        let mut app = create_test_app();
        let initial_tab = app.current_tab;
        app.next_tab();
        assert_ne!(app.current_tab, initial_tab);
        assert_eq!(app.current_tab, initial_tab.next());
    }

    #[test]
    fn test_prev_tab_cycles_backward() {
        let mut app = create_test_app();
        let initial_tab = app.current_tab;
        app.prev_tab();
        assert_ne!(app.current_tab, initial_tab);
        assert_eq!(app.current_tab, initial_tab.prev());
    }

    #[test]
    fn test_select_tab_by_index() {
        let mut app = create_test_app();
        app.select_tab(5);
        assert_eq!(app.current_tab, Tab::Fuzz);
    }

    #[test]
    fn test_select_tab_by_invalid_index_ignores() {
        let mut app = create_test_app();
        let initial_tab = app.current_tab;
        app.select_tab(999);
        assert_eq!(app.current_tab, initial_tab);
    }

    #[test]
    fn test_select_tab_by_zero_index() {
        let mut app = create_test_app();
        app.select_tab(0);
        assert_eq!(app.current_tab, Tab::Recon);
    }

    #[test]
    fn test_toggle_help() {
        let mut app = create_test_app();
        assert!(!app.show_help);
        assert!(!app.is_help_visible());

        app.toggle_help();
        assert!(app.show_help);
        assert!(app.is_help_visible());
        assert_eq!(app.help_tab, Some(Tab::Recon));

        app.toggle_help();
        assert!(!app.show_help);
        assert!(!app.is_help_visible());
        assert_eq!(app.help_tab, None);
    }

    #[test]
    fn test_toggle_help_preserves_current_tab() {
        let mut app = create_test_app();
        app.current_tab = Tab::ScanPorts;
        app.toggle_help();
        assert!(app.is_help_visible());
        assert_eq!(app.help_tab, Some(Tab::ScanPorts));

        app.current_tab = Tab::Fuzz;
        app.toggle_help();
        assert!(!app.is_help_visible());
        assert_eq!(app.help_tab, None);
    }

    #[test]
    fn test_toggle_search() {
        let mut app = create_test_app();
        assert!(!app.show_search);

        app.toggle_search(true);
        assert!(app.show_search);
        assert!(app.search_is_global);

        app.toggle_search(false);
        assert!(!app.show_search);
    }

    #[test]
    fn test_toggle_search_clears_query_on_open() {
        let mut app = create_test_app();
        app.search_query = "test query".to_string();
        app.toggle_search(false);
        assert!(app.show_search);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_is_help_visible() {
        let mut app = create_test_app();
        assert!(!app.is_help_visible());

        app.show_help = true;
        assert!(app.is_help_visible());

        app.show_help = false;
        assert!(!app.is_help_visible());
    }

    #[test]
    fn test_get_current_help_returns_non_empty_string() {
        let app = create_test_app();
        let help = app.get_current_help();
        assert!(!help.is_empty());
    }

    #[test]
    fn test_get_current_help_different_per_tab() {
        let mut app = create_test_app();

        app.current_tab = Tab::Recon;
        let recon_help = app.get_current_help();
        assert!(recon_help.contains("Recon"));

        app.current_tab = Tab::Fuzz;
        let fuzz_help = app.get_current_help();
        assert!(fuzz_help.contains("Fuzz"));

        app.current_tab = Tab::Waf;
        let waf_help = app.get_current_help();
        assert!(waf_help.contains("WAF"));
    }

    #[test]
    fn test_navigation_clears_search_on_tab_switch() {
        let mut app = create_test_app();
        app.show_search = true;
        app.search_query = "test query".to_string();

        app.next_tab();

        assert!(!app.show_search);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_prev_navigation_clears_search_on_tab_switch() {
        let mut app = create_test_app();
        app.show_search = true;
        app.search_query = "test query".to_string();

        app.prev_tab();

        assert!(!app.show_search);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_select_tab_clears_search() {
        let mut app = create_test_app();
        app.show_search = true;
        app.search_query = "test query".to_string();

        app.select_tab(3);

        assert!(!app.show_search);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_tab_visible_index() {
        use crate::tui::tabs::Tab;
        let all_tabs = Tab::all();
        for (i, tab) in all_tabs.iter().enumerate() {
            assert_eq!(tab.visible_index(), Some(i), "Tab {:?} should have visible_index {}", tab, i);
        }
    }

    #[test]
    fn test_tab_from_visible_index() {
        use crate::tui::tabs::Tab;
        let all_tabs = Tab::all();
        for (i, tab) in all_tabs.iter().enumerate() {
            assert_eq!(Tab::from_visible_index(i), Some(*tab), "from_visible_index({}) should return {:?}", i, tab);
        }
        assert_eq!(Tab::from_visible_index(999), None);
    }

#[test]
    fn test_tab_stable_id_roundtrip() {
        use crate::tui::tabs::Tab;
        let all_tabs = Tab::all();
        for tab in all_tabs {
            let id = tab.stable_id();
            assert_eq!(Tab::from_stable_id(id), Some(*tab), "stable_id {:?} should roundtrip", id);
        }
    }

    #[test]
    fn test_tab_from_stable_id_invalid() {
        use crate::tui::tabs::Tab;
        assert_eq!(Tab::from_stable_id("nonexistent"), None);
        assert_eq!(Tab::from_stable_id(""), None);
    }

    #[test]
    fn test_tab_from_discriminant() {
        use crate::tui::tabs::Tab;
        for (i, tab) in Tab::all().iter().enumerate() {
            assert_eq!(
                Tab::from_discriminant(*tab as usize),
                Some(*tab),
                "from_discriminant({}) should return {:?}",
                *tab as usize,
                tab
            );
        }
        assert_eq!(Tab::from_discriminant(999), None);
        assert_eq!(Tab::from_discriminant(0), Some(Tab::Recon));
        assert_eq!(Tab::from_discriminant(28), Some(Tab::Vuln));
    }

    #[test]
    fn test_tab_window_calculation_80_cols() {
        use crate::tui::tabs::{Tab, TabWindow};
        let window = TabWindow::for_width(80, Tab::Recon, 0);
        assert_eq!(window.max_visible, 6);
        assert_eq!(window.start, 0);
        assert!(window.end <= window.total_tabs);
        assert!(window.selected_visible < window.max_visible);
    }

    #[test]
    fn test_tab_window_calculation_40_cols() {
        use crate::tui::tabs::{Tab, TabWindow};
        let window = TabWindow::for_width(40, Tab::Recon, 0);
        assert_eq!(window.max_visible, 3);
        assert!(window.start <= window.total_tabs);
    }

    #[test]
    fn test_tab_window_calculation_120_cols() {
        use crate::tui::tabs::{Tab, TabWindow};
        let window = TabWindow::for_width(120, Tab::Recon, 0);
        assert_eq!(window.max_visible, 11);
    }

    #[test]
    fn test_tab_window_has_correct_flags() {
        use crate::tui::tabs::{Tab, TabWindow};
        let first_window = TabWindow::for_width(80, Tab::Recon, 0);
        assert!(!first_window.has_prev, "First tab should not have prev");
        assert!(first_window.has_next || first_window.end < first_window.total_tabs, "Should have next or be at end");

        let last_tab = Tab::all().last().copied().unwrap_or(Tab::Recon);
        let last_window = TabWindow::for_width(80, last_tab, 0);
        assert!(last_window.has_next == (last_window.end < last_window.total_tabs));
    }

    #[test]
    fn test_tab_window_scroll_stays_in_bounds() {
        use crate::tui::tabs::{Tab, TabWindow};
        let tabs = Tab::all();
        for tab in tabs {
            let window = TabWindow::for_width(80, *tab, 0);
            assert!(window.start <= window.end);
            assert!(window.end <= window.total_tabs);
            assert!(window.selected_visible < window.max_visible || window.max_visible == 0);
        }
    }

    #[test]
    fn test_adjust_tab_scroll_keeps_tab_visible() {
        use crate::tui::tabs::{Tab, TabWindow};
        let mut app = create_test_app();

        app.current_tab = Tab::Dashboard;
        app.adjust_tab_scroll();
        let window = TabWindow::for_width(80, app.current_tab, app.tab_scroll_offset);
        let tab_idx = app.current_tab.visible_index().unwrap_or(0);
        assert!(
            tab_idx >= window.start && tab_idx < window.end,
            "Current tab should be visible after adjust_tab_scroll"
        );
    }

    #[test]
    fn test_tab_window_always_contains_current_tab() {
        use crate::tui::tabs::{Tab, TabWindow};
        let tabs = Tab::all();
        for tab in tabs {
            for width in [40, 60, 80, 100, 120] {
                let window = TabWindow::for_width(width, *tab, 0);
                let tab_idx = tab.visible_index().unwrap_or(0);
                assert!(
                    window.start <= tab_idx && tab_idx < window.end,
                    "Tab {:?} at index {} should be visible in window [{}, {}) for width {}",
                    tab, tab_idx, window.start, window.end, width
                );
            }
        }
    }

    #[test]
    fn test_tab_window_handles_stale_offset() {
        use crate::tui::tabs::{Tab, TabWindow};
        let tabs = Tab::all();
        let stale_offset = 1000u16;
        for tab in tabs {
            let window = TabWindow::for_width(80, *tab, stale_offset);
            let tab_idx = tab.visible_index().unwrap_or(0);
            assert!(
                window.start <= tab_idx && tab_idx < window.end,
                "Tab {:?} should be visible even with stale offset",
                tab
            );
        }
    }

    #[test]
    fn test_bookmark_api_uses_stable_ids() {
        use crate::tui::tabs::Tab;
        let mut app = create_test_app();

        app.toggle_bookmark(Tab::Dashboard);
        assert!(app.is_bookmarked(Tab::Dashboard));
        assert!(!app.is_bookmarked(Tab::Recon));

        let bookmark_ids = app.get_bookmarked_tab_ids();
        assert_eq!(bookmark_ids.len(), 1);
        assert_eq!(bookmark_ids[0], "dashboard");

        app.toggle_bookmark(Tab::Settings);
        assert!(app.is_bookmarked(Tab::Settings));

        let bookmark_ids = app.get_bookmarked_tab_ids();
        assert_eq!(bookmark_ids.len(), 2);
        assert!(bookmark_ids.contains(&"dashboard".to_string()));
        assert!(bookmark_ids.contains(&"settings".to_string()));
    }

    #[test]
    fn test_adjust_tab_scroll_with_stored_width() {
        use crate::tui::tabs::{Tab, TabWindow};
        let mut app = create_test_app();

        app.last_tab_area_width = 60;
        app.current_tab = Tab::Dashboard;
        app.tab_scroll_offset = 0;
        app.adjust_tab_scroll();

        let window = TabWindow::for_width(app.last_tab_area_width, app.current_tab, app.tab_scroll_offset);
        let tab_idx = app.current_tab.visible_index().unwrap_or(0);
        assert!(
            window.start <= tab_idx && tab_idx < window.end,
            "Current tab should be visible after adjust_tab_scroll with stored width"
        );
    }
}

#[cfg(test)]
mod render_tests {
    use ratatui::{backend::TestBackend, Terminal};

    use crate::tui::tabs::Tab;
    use crate::tui::ui;
    use crate::tui::state::create_shared_history;
    use crate::tui::app::App;

    fn create_test_app() -> App {
        App::new_for_testing(create_shared_history())
    }

    #[test]
    fn test_render_at_80x24_no_panic() {
        let mut app = create_test_app();
        app.current_tab = Tab::Fuzz;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    }

    #[test]
    fn test_render_at_40x20_no_panic() {
        let mut app = create_test_app();
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    }

    #[test]
    fn test_render_at_60x20_no_panic() {
        let mut app = create_test_app();
        app.current_tab = Tab::Dashboard;
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    }

    #[test]
    fn test_render_at_120x24_no_panic() {
        let mut app = create_test_app();
        app.current_tab = Tab::Settings;
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    }

    #[test]
    fn test_render_stale_offset_at_80x24() {
        let mut app = create_test_app();
        app.tab_scroll_offset = 100;
        app.current_tab = Tab::Scan;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    }

    #[test]
    fn test_render_narrow_30_width() {
        let mut app = create_test_app();
        let backend = TestBackend::new(30, 20);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    }

    #[test]
    fn test_render_height_12_minimal() {
        let mut app = create_test_app();
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    }

    #[test]
    fn test_render_tab_bar_at_various_widths() {
        let widths = [30u16, 40, 60, 80, 120];
        for width in widths {
            let mut app = create_test_app();
            app.current_tab = Tab::Recon;
            let backend = TestBackend::new(width, 24);
            let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        }
    }

    #[test]
    fn test_render_late_tab_near_end_of_tabs() {
        let mut app = create_test_app();
        let all_tabs = Tab::all();
        if let Some(last_tab) = all_tabs.last() {
            app.current_tab = *last_tab;
            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        }
    }
}
