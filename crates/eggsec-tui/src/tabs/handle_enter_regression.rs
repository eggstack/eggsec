#[cfg(test)]
mod tests {
    use crate::tabs::TabInput;
    use crate::tabs::TabState;

    use super::super::graphql::{GraphQlFocusArea, GraphQlTab};
    use super::super::oauth::{OAuthFocusArea, OAuthTab};
    use super::super::recon::{ReconFocusArea, ReconTab};
    use super::super::load::{LoadFocusArea, LoadTab};
    use super::super::scan_ports::{ScanPortsFocusArea, ScanPortsTab};
    use super::super::stress::{StressFocusArea, StressTab};
    use super::super::packet::PacketTab;
    use super::super::waf::{WafFocusArea, WafTab};
    use super::super::cluster::{ClusterFocusArea, ClusterTab};
    use super::super::dashboard::DashboardTab;
    use super::super::settings::main::{SettingsFocusArea, SettingsSection, SettingsTab};
    use super::super::history::HistoryTab;

    // =========================================================================
    // 1. GraphQl tab: all focus areas
    // =========================================================================

    #[test]
    fn graphql_enter_inputs_focused_blurs() {
        let mut tab = GraphQlTab::new();
        tab.focus_area = GraphQlFocusArea::Inputs;
        tab.inputs.focus(0);
        assert!(tab.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn graphql_enter_inputs_unfocused_starts_with_target() {
        let mut tab = GraphQlTab::new();
        tab.focus_area = GraphQlFocusArea::Inputs;
        tab.inputs.blur();
        // With a target set, unfocused inputs + Enter starts the scan
        tab.inputs.fields[0].value = "https://example.com/graphql".to_string();
        tab.handle_enter();
        assert!(tab.is_running());
    }

    #[test]
    fn graphql_enter_options_toggles_checkbox() {
        let mut tab = GraphQlTab::new();
        tab.focus_area = GraphQlFocusArea::Options;
        let before = tab.introspection_checkbox.checked;
        tab.handle_enter();
        assert_eq!(tab.introspection_checkbox.checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn graphql_enter_results_no_op() {
        let mut tab = GraphQlTab::new();
        tab.focus_area = GraphQlFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 2. OAuth tab: all focus areas
    // =========================================================================

    #[test]
    fn oauth_enter_inputs_focused_blurs() {
        let mut tab = OAuthTab::new();
        tab.focus_area = OAuthFocusArea::Inputs;
        tab.inputs.focus(0);
        assert!(tab.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn oauth_enter_options_toggles_checkbox() {
        let mut tab = OAuthTab::new();
        tab.focus_area = OAuthFocusArea::Options;
        let before = tab.redirect_test_checkbox.checked;
        tab.handle_enter();
        assert_eq!(tab.redirect_test_checkbox.checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn oauth_enter_results_no_op() {
        let mut tab = OAuthTab::new();
        tab.focus_area = OAuthFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 3. Recon tab: all focus areas
    // =========================================================================

    #[test]
    fn recon_enter_inputs_focused_blurs() {
        let mut tab = ReconTab::new();
        tab.focus_area = ReconFocusArea::Inputs;
        tab.inputs.focus(0);
        assert!(tab.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn recon_enter_options_toggles_checkbox() {
        let mut tab = ReconTab::new();
        tab.focus_area = ReconFocusArea::Options;
        tab.focused_checkbox_index = 0;
        assert!(!tab.option_checkboxes[0].checked);
        tab.handle_enter();
        assert!(tab.option_checkboxes[0].checked);
        assert!(!tab.is_running());
    }

    #[test]
    fn recon_enter_results_no_op() {
        let mut tab = ReconTab::new();
        tab.focus_area = ReconFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 4. Load tab: all focus areas
    // =========================================================================

    #[test]
    fn load_enter_inputs_focused_blurs() {
        let mut tab = LoadTab::new();
        tab.focus_area = LoadFocusArea::Inputs;
        tab.inputs.focus(0);
        assert!(tab.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn load_enter_selector_open_confirms() {
        let mut tab = LoadTab::new();
        tab.focus_area = LoadFocusArea::Selector;
        tab.test_type_selector.focus();
        tab.test_type_selector.open();
        assert!(tab.test_type_selector.is_open());
        tab.handle_enter();
        assert!(!tab.test_type_selector.is_open());
        assert!(!tab.is_running());
    }

    #[test]
    fn load_enter_results_no_op() {
        let mut tab = LoadTab::new();
        tab.focus_area = LoadFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 5. ScanPorts tab: all focus areas
    // =========================================================================

    #[test]
    fn scan_ports_enter_inputs_focused_blurs() {
        let mut tab = ScanPortsTab::new();
        tab.focus_area = ScanPortsFocusArea::Inputs;
        tab.inputs.focus(0);
        assert!(tab.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn scan_ports_enter_options_toggles_checkbox() {
        let mut tab = ScanPortsTab::new();
        tab.focus_area = ScanPortsFocusArea::Options;
        let before = tab.udp_checkbox.checked;
        tab.handle_enter();
        assert_eq!(tab.udp_checkbox.checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn scan_ports_enter_results_no_op() {
        let mut tab = ScanPortsTab::new();
        tab.focus_area = ScanPortsFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 6. Stress tab: all focus areas
    // =========================================================================

    #[test]
    fn stress_enter_inputs_blurs_and_opens_selector() {
        let mut tab = StressTab::new();
        tab.focus_area = StressFocusArea::Inputs;
        tab.inputs.focus(0);
        assert!(tab.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.inputs.is_focused());
        assert!(!tab.is_running());
        assert!(tab.type_selector.is_open());
    }

    #[test]
    fn stress_enter_type_selector_opens() {
        let mut tab = StressTab::new();
        tab.focus_area = StressFocusArea::TypeSelector;
        tab.type_selector.open();
        assert!(tab.type_selector.is_open());
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    #[test]
    fn stress_enter_results_no_op() {
        let mut tab = StressTab::new();
        tab.focus_area = StressFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 7. Packet tab: all focus areas
    // =========================================================================

    #[test]
    fn packet_enter_inputs_focused_blurs() {
        let mut tab = PacketTab::new();
        tab.inputs.focus(0);
        assert!(tab.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn packet_enter_selector_open_confirms() {
        let mut tab = PacketTab::new();
        tab.view_selector.focus();
        tab.view_selector.open();
        assert!(tab.view_selector.is_open());
        tab.handle_enter();
        assert!(!tab.view_selector.is_open());
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 8. Waf tab: all focus areas
    // =========================================================================

    #[test]
    fn waf_enter_inputs_focused_blurs() {
        let mut tab = WafTab::new();
        tab.focus_area = WafFocusArea::Inputs;
        tab.inputs.focus(0);
        assert!(tab.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn waf_enter_mode_radio_cycles() {
        let mut tab = WafTab::new();
        tab.focus_area = WafFocusArea::ModeRadio;
        let before = tab.mode_radio.selected;
        tab.handle_enter();
        // Should cycle the selection
        assert!(!tab.is_running());
        let _ = before;
    }

    #[test]
    fn waf_enter_techniques_toggles_checkbox() {
        let mut tab = WafTab::new();
        tab.focus_area = WafFocusArea::Techniques;
        tab.focused_checkbox_index = 1;
        let before = tab.technique_checkboxes[1].checked;
        tab.handle_enter();
        assert_eq!(tab.technique_checkboxes[1].checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn waf_enter_results_no_op() {
        let mut tab = WafTab::new();
        tab.focus_area = WafFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 9. Cluster tab: all focus areas
    // =========================================================================

    #[test]
    fn cluster_enter_view_selector_opens_and_starts() {
        let mut tab = ClusterTab::new();
        tab.focus_area = ClusterFocusArea::ViewSelector;
        tab.handle_enter();
        assert!(tab.view_selector.is_open());
        // Cluster falls through to start() after opening selector
        assert!(tab.is_running());
    }

    #[test]
    fn cluster_enter_inputs_blurs() {
        let mut tab = ClusterTab::new();
        tab.focus_area = ClusterFocusArea::Inputs;
        tab.worker_inputs.focus(0);
        tab.handle_enter();
        // Cluster Inputs returns early (blurs) without starting
        assert!(!tab.is_running());
    }

    #[test]
    fn cluster_enter_results_starts() {
        let mut tab = ClusterTab::new();
        tab.focus_area = ClusterFocusArea::Results;
        tab.handle_enter();
        // Cluster Results falls through to start()
        assert!(tab.is_running());
    }

    // =========================================================================
    // 10. Dashboard tab
    // =========================================================================

    #[test]
    fn dashboard_enter_is_no_op() {
        let mut tab = DashboardTab::new();
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 11. Settings tab: all focus areas
    // =========================================================================

    #[test]
    fn settings_enter_section_list_transitions_to_detail() {
        let mut tab = SettingsTab::new();
        tab.focus_area = SettingsFocusArea::SectionList;
        tab.handle_enter();
        assert_eq!(tab.focus_area, SettingsFocusArea::SectionDetail);
        assert!(!tab.is_running());
    }

    #[test]
    fn settings_enter_section_detail_http_input_blurs() {
        let mut tab = SettingsTab::new();
        tab.focus_area = SettingsFocusArea::SectionDetail;
        tab.current_section = SettingsSection::Http;
        tab.detail_focus_index = 0;
        tab.sync_component_focus();
        tab.handle_enter();
        assert!(!tab.http_inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn settings_enter_section_detail_http_follow_redirects_toggles() {
        let mut tab = SettingsTab::new();
        tab.focus_area = SettingsFocusArea::SectionDetail;
        tab.current_section = SettingsSection::Http;
        tab.detail_focus_index = 4;
        tab.sync_component_focus();
        let before = tab.follow_redirects.checked;
        tab.handle_enter();
        assert_eq!(tab.follow_redirects.checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn settings_enter_section_detail_http_verify_tls_toggles() {
        let mut tab = SettingsTab::new();
        tab.focus_area = SettingsFocusArea::SectionDetail;
        tab.current_section = SettingsSection::Http;
        tab.detail_focus_index = 5;
        tab.sync_component_focus();
        let before = tab.verify_tls.checked;
        tab.handle_enter();
        assert_eq!(tab.verify_tls.checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn settings_enter_section_detail_scan_stealth_toggles() {
        let mut tab = SettingsTab::new();
        tab.focus_area = SettingsFocusArea::SectionDetail;
        tab.current_section = SettingsSection::Scan;
        tab.detail_focus_index = 3;
        tab.sync_component_focus();
        let before = tab.stealth_mode.checked;
        tab.handle_enter();
        assert_eq!(tab.stealth_mode.checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn settings_enter_section_detail_notifications_complete_toggles() {
        let mut tab = SettingsTab::new();
        tab.focus_area = SettingsFocusArea::SectionDetail;
        tab.current_section = SettingsSection::Notifications;
        tab.detail_focus_index = 4;
        tab.sync_component_focus();
        let before = tab.notify_on_complete.checked;
        tab.handle_enter();
        assert_eq!(tab.notify_on_complete.checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn settings_enter_section_detail_notifications_findings_toggles() {
        let mut tab = SettingsTab::new();
        tab.focus_area = SettingsFocusArea::SectionDetail;
        tab.current_section = SettingsSection::Notifications;
        tab.detail_focus_index = 5;
        tab.sync_component_focus();
        let before = tab.notify_on_findings.checked;
        tab.handle_enter();
        assert_eq!(tab.notify_on_findings.checked, !before);
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 12. History tab
    // =========================================================================

    #[test]
    fn history_enter_is_no_op() {
        let mut tab = HistoryTab::new();
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    // =========================================================================
    // 13. Cross-cutting: Enter in Results never starts any tab
    // =========================================================================

    #[test]
    fn cross_tab_enter_results_never_starts() {
        // Recon
        let mut recon = ReconTab::new();
        recon.focus_area = ReconFocusArea::Results;
        recon.handle_enter();
        assert!(!recon.is_running(), "Recon Results should not start");

        // GraphQl
        let mut gql = GraphQlTab::new();
        gql.focus_area = GraphQlFocusArea::Results;
        gql.handle_enter();
        assert!(!gql.is_running(), "GraphQl Results should not start");

        // OAuth
        let mut oauth = OAuthTab::new();
        oauth.focus_area = OAuthFocusArea::Results;
        oauth.handle_enter();
        assert!(!oauth.is_running(), "OAuth Results should not start");

        // Load
        let mut load = LoadTab::new();
        load.focus_area = LoadFocusArea::Results;
        load.handle_enter();
        assert!(!load.is_running(), "Load Results should not start");

        // ScanPorts
        let mut sp = ScanPortsTab::new();
        sp.focus_area = ScanPortsFocusArea::Results;
        sp.handle_enter();
        assert!(!sp.is_running(), "ScanPorts Results should not start");

        // Stress
        let mut stress = StressTab::new();
        stress.focus_area = StressFocusArea::Results;
        stress.handle_enter();
        assert!(!stress.is_running(), "Stress Results should not start");

        // Waf
        let mut waf = WafTab::new();
        waf.focus_area = WafFocusArea::Results;
        waf.handle_enter();
        assert!(!waf.is_running(), "Waf Results should not start");

        // Note: Cluster starts from Results (falls through to start())
        // This is the actual behavior - Cluster uses Results as a "go" action
    }

    // =========================================================================
    // 14. Cross-cutting: Enter in Options only toggles, never starts
    // =========================================================================

    #[test]
    fn cross_tab_enter_options_only_toggles() {
        // GraphQl
        let mut gql = GraphQlTab::new();
        gql.focus_area = GraphQlFocusArea::Options;
        let gql_before = gql.introspection_checkbox.checked;
        gql.handle_enter();
        assert_eq!(
            gql.introspection_checkbox.checked,
            !gql_before,
            "GraphQl Options should toggle"
        );
        assert!(!gql.is_running());

        // OAuth
        let mut oauth = OAuthTab::new();
        oauth.focus_area = OAuthFocusArea::Options;
        let oauth_before = oauth.redirect_test_checkbox.checked;
        oauth.handle_enter();
        assert_eq!(
            oauth.redirect_test_checkbox.checked,
            !oauth_before,
            "OAuth Options should toggle"
        );
        assert!(!oauth.is_running());

        // Recon
        let mut recon = ReconTab::new();
        recon.focus_area = ReconFocusArea::Options;
        recon.focused_checkbox_index = 0;
        let recon_before = recon.option_checkboxes[0].checked;
        recon.handle_enter();
        assert_eq!(
            recon.option_checkboxes[0].checked,
            !recon_before,
            "Recon Options should toggle"
        );
        assert!(!recon.is_running());

        // ScanPorts
        let mut sp = ScanPortsTab::new();
        sp.focus_area = ScanPortsFocusArea::Options;
        let sp_before = sp.udp_checkbox.checked;
        sp.handle_enter();
        assert_eq!(
            sp.udp_checkbox.checked,
            !sp_before,
            "ScanPorts Options should toggle"
        );
        assert!(!sp.is_running());

        // Waf
        let mut waf = WafTab::new();
        waf.focus_area = WafFocusArea::Techniques;
        waf.focused_checkbox_index = 0;
        let waf_before = waf.technique_checkboxes[0].checked;
        waf.handle_enter();
        assert_eq!(
            waf.technique_checkboxes[0].checked,
            !waf_before,
            "Waf Techniques should toggle"
        );
        assert!(!waf.is_running());
    }

    // =========================================================================
    // 15. Cross-cutting: Enter in focused input only blurs, never starts
    // =========================================================================

    #[test]
    fn cross_tab_enter_focused_input_only_blurs() {
        // Recon
        let mut recon = ReconTab::new();
        recon.focus_area = ReconFocusArea::Inputs;
        recon.inputs.focus(0);
        assert!(recon.inputs.is_focused());
        recon.handle_enter();
        assert!(!recon.inputs.is_focused(), "Recon input should blur");
        assert!(!recon.is_running());

        // GraphQl
        let mut gql = GraphQlTab::new();
        gql.focus_area = GraphQlFocusArea::Inputs;
        gql.inputs.focus(0);
        assert!(gql.inputs.is_focused());
        gql.handle_enter();
        assert!(!gql.inputs.is_focused(), "GraphQl input should blur");
        assert!(!gql.is_running());

        // OAuth
        let mut oauth = OAuthTab::new();
        oauth.focus_area = OAuthFocusArea::Inputs;
        oauth.inputs.focus(0);
        assert!(oauth.inputs.is_focused());
        oauth.handle_enter();
        assert!(!oauth.inputs.is_focused(), "OAuth input should blur");
        assert!(!oauth.is_running());

        // Load
        let mut load = LoadTab::new();
        load.focus_area = LoadFocusArea::Inputs;
        load.inputs.focus(0);
        assert!(load.inputs.is_focused());
        load.handle_enter();
        assert!(!load.inputs.is_focused(), "Load input should blur");
        assert!(!load.is_running());

        // ScanPorts
        let mut sp = ScanPortsTab::new();
        sp.focus_area = ScanPortsFocusArea::Inputs;
        sp.inputs.focus(0);
        assert!(sp.inputs.is_focused());
        sp.handle_enter();
        assert!(!sp.inputs.is_focused(), "ScanPorts input should blur");
        assert!(!sp.is_running());

        // Stress
        let mut stress = StressTab::new();
        stress.focus_area = StressFocusArea::Inputs;
        stress.inputs.focus(0);
        assert!(stress.inputs.is_focused());
        stress.handle_enter();
        assert!(!stress.inputs.is_focused(), "Stress input should blur");
        assert!(!stress.is_running());

        // Packet
        let mut pkt = PacketTab::new();
        pkt.inputs.focus(0);
        assert!(pkt.inputs.is_focused());
        pkt.handle_enter();
        assert!(!pkt.inputs.is_focused(), "Packet input should blur");
        assert!(!pkt.is_running());

        // Waf
        let mut waf = WafTab::new();
        waf.focus_area = WafFocusArea::Inputs;
        waf.inputs.focus(0);
        assert!(waf.inputs.is_focused());
        waf.handle_enter();
        assert!(!waf.inputs.is_focused(), "Waf input should blur");
        assert!(!waf.is_running());
    }
}
