#[cfg(test)]
mod tests {
    use crate::components::InputGroup;
    use crate::tabs::core::TabCore;
    use crate::tabs::TabInput;
    use crate::tabs::TabState;

    fn assert_unique_input_labels(tab_name: &str, group_name: &str, inputs: &InputGroup) {
        let duplicates = inputs.duplicate_label_names();
        assert!(
            duplicates.is_empty(),
            "{tab_name} {group_name} has duplicate input labels: {duplicates:?}"
        );
    }

    fn assert_input_group_focus_traverses_all_fields(
        tab_name: &str,
        group_name: &str,
        inputs: &mut InputGroup,
    ) {
        inputs.blur();
        let field_count = inputs.fields.len();
        for expected_idx in 0..field_count {
            inputs.focus_next();
            assert_eq!(
                inputs.focused,
                Some(expected_idx),
                "{tab_name} {group_name} did not focus field {expected_idx}"
            );
            assert!(
                inputs.focus_state_is_consistent(),
                "{tab_name} {group_name} has inconsistent focus flags at field {expected_idx}"
            );
        }
        if field_count > 0 {
            inputs.focus_next();
            assert_eq!(
                inputs.focused,
                Some(0),
                "{tab_name} {group_name} did not wrap focus to the first field"
            );
        }
    }

    fn assert_tab_core_inputs(tab_name: &str, group_name: &str, core: &mut TabCore) {
        assert_unique_input_labels(tab_name, group_name, &core.inputs);
        assert_input_group_focus_traverses_all_fields(tab_name, group_name, &mut core.inputs);
    }

    fn assert_tab_input_group(tab_name: &str, group_name: &str, inputs: &mut InputGroup) {
        assert_unique_input_labels(tab_name, group_name, inputs);
        assert_input_group_focus_traverses_all_fields(tab_name, group_name, inputs);
    }

    #[test]
    fn input_groups_report_stale_focus_as_unfocused() {
        let mut group = InputGroup::new().add(crate::components::InputField::new("Target"));
        group.focused = Some(99);
        group.fields[0].focused = false;

        assert!(!group.is_focused());
        assert!(group.focus_state_is_consistent());
    }

    #[test]
    fn set_focus_for_index_updates_index_and_flags_together() {
        let mut group = InputGroup::new()
            .add(crate::components::InputField::new("Target"))
            .add(crate::components::InputField::new("Concurrency"));

        group.set_focus_for_index(Some(1));
        assert_eq!(group.focused, Some(1));
        assert!(!group.fields[0].focused);
        assert!(group.fields[1].focused);
        assert!(group.focus_state_is_consistent());

        group.set_focus_for_index(Some(99));
        assert_eq!(group.focused, None);
        assert!(group.focus_state_is_consistent());
    }

    #[test]
    fn core_tab_inputs_have_unique_labels_and_reachable_fields() {
        assert_tab_core_inputs("Auth", "core", &mut super::super::auth::AuthTab::new().core);
        assert_tab_input_group(
            "Cluster",
            "worker",
            &mut super::super::cluster::ClusterTab::new().worker_inputs,
        );
        assert_tab_input_group(
            "Cluster",
            "coordinator",
            &mut super::super::cluster::ClusterTab::new().coordinator_inputs,
        );
        assert_tab_input_group(
            "Cluster",
            "status",
            &mut super::super::cluster::ClusterTab::new().status_inputs,
        );
        assert_tab_core_inputs("Fuzz", "core", &mut super::super::fuzz::FuzzTab::new().core);
        assert_tab_core_inputs(
            "GraphQL",
            "core",
            &mut super::super::graphql::GraphQlTab::new().core,
        );
        assert_tab_core_inputs("Load", "core", &mut super::super::load::LoadTab::new().core);
        assert_tab_core_inputs(
            "OAuth",
            "core",
            &mut super::super::oauth::OAuthTab::new().core,
        );
        assert_tab_input_group(
            "Packet",
            "inputs",
            &mut super::super::packet::PacketTab::new().inputs,
        );
        assert_tab_core_inputs(
            "Proxy",
            "core",
            &mut super::super::proxy::ProxyTab::new().core,
        );
        assert_tab_core_inputs(
            "Recon",
            "core",
            &mut super::super::recon::ReconTab::new().core,
        );
        assert_tab_input_group(
            "Report",
            "convert",
            &mut super::super::report::ReportTab::new().convert_inputs,
        );
        assert_tab_input_group(
            "Report",
            "trend",
            &mut super::super::report::ReportTab::new().trend_inputs,
        );
        assert_tab_input_group(
            "Report",
            "schedule",
            &mut super::super::report::ReportTab::new().schedule_inputs,
        );
        assert_tab_core_inputs(
            "Resume",
            "core",
            &mut super::super::resume::ResumeTab::new().core,
        );
        assert_tab_input_group(
            "Scan",
            "inputs",
            &mut super::super::scan::ScanTab::new().inputs,
        );
        let mut settings = super::super::settings::SettingsTab::new();
        assert_tab_input_group("Settings", "http", &mut settings.http_inputs);
        assert_tab_input_group("Settings", "scan", &mut settings.scan_inputs);
        assert_tab_input_group("Settings", "session", &mut settings.session_inputs);
        assert_tab_input_group("Settings", "proxy", &mut settings.proxy_inputs);
        assert_tab_input_group("Settings", "scope", &mut settings.scope_inputs);
        assert_tab_input_group("Settings", "report", &mut settings.report_inputs);
        assert_tab_input_group("Settings", "schedule", &mut settings.schedule_inputs);
        assert_tab_input_group("Settings", "notifications", &mut settings.notify_inputs);
        assert_tab_core_inputs(
            "ScanEndpoints",
            "core",
            &mut super::super::scan_endpoints::ScanEndpointsTab::new().core,
        );
        assert_tab_core_inputs(
            "ScanPorts",
            "core",
            &mut super::super::scan_ports::ScanPortsTab::new().core,
        );
        assert_tab_core_inputs(
            "Stress",
            "core",
            &mut super::super::stress::StressTab::new().core,
        );
        assert_tab_core_inputs("WAF", "core", &mut super::super::waf::WafTab::new().core);
        assert_tab_core_inputs(
            "WAF Stress",
            "core",
            &mut super::super::waf_stress::WafStressTab::new().core,
        );

        #[cfg(feature = "advanced-hunting")]
        assert_tab_core_inputs("Hunt", "core", &mut super::super::hunt::HuntTab::new().core);
        #[cfg(feature = "c2")]
        assert_tab_core_inputs("C2", "core", &mut super::super::c2::C2Tab::new().core);
        #[cfg(feature = "compliance")]
        assert_tab_core_inputs(
            "Compliance",
            "core",
            &mut super::super::compliance::ComplianceTab::new().core,
        );
        #[cfg(feature = "database")]
        {
            assert_tab_input_group(
                "Storage",
                "config",
                &mut super::super::storage::StorageTab::new().config_inputs,
            );
            assert_tab_input_group(
                "Storage",
                "query",
                &mut super::super::storage::StorageTab::new().query_inputs,
            );
        }
        #[cfg(feature = "db-pentest")]
        assert_tab_core_inputs(
            "DbPentest",
            "core",
            &mut super::super::db_pentest::DbPentestTab::new().core,
        );
        #[cfg(feature = "external-integrations")]
        {
            assert_tab_input_group(
                "Integrations",
                "config",
                &mut super::super::integrations::IntegrationsTab::new().config_inputs,
            );
            assert_tab_input_group(
                "Integrations",
                "issue",
                &mut super::super::integrations::IntegrationsTab::new().issue_inputs,
            );
        }
        #[cfg(feature = "finding-workflow")]
        assert_tab_core_inputs(
            "Workflow",
            "core",
            &mut super::super::workflow::WorkflowTab::new().core,
        );
        #[cfg(feature = "headless-browser")]
        assert_tab_core_inputs(
            "Browser",
            "core",
            &mut super::super::browser::BrowserTab::new().core,
        );
        #[cfg(feature = "nse")]
        assert_tab_core_inputs(
            "NSE",
            "core",
            &mut super::super::nse::NseTab::new().core,
        );
        #[cfg(feature = "vuln-management")]
        assert_tab_core_inputs("Vuln", "core", &mut super::super::vuln::VulnTab::new().core);
        #[cfg(feature = "wireless")]
        {
            assert_tab_input_group(
                "Wireless",
                "passive",
                &mut super::super::wireless::WirelessTab::new().inputs,
            );
            #[cfg(feature = "wireless-advanced")]
            assert_tab_input_group(
                "Wireless",
                "active",
                &mut super::super::wireless::WirelessTab::new().active_inputs,
            );
        }
    }

    #[test]
    fn settings_focus_limits_track_rendered_controls() {
        use super::super::settings::{SettingsFocusArea, SettingsSection, SettingsTab};

        let mut tab = SettingsTab::new();
        let sections = [
            (SettingsSection::Http, tab.http_inputs.fields.len() + 2),
            (SettingsSection::Scan, tab.scan_inputs.fields.len() + 1),
            (SettingsSection::Session, tab.session_inputs.fields.len()),
            (SettingsSection::Proxy, tab.proxy_inputs.fields.len() + 1),
            (SettingsSection::Scope, tab.scope_inputs.fields.len()),
            (SettingsSection::Report, tab.report_inputs.fields.len()),
            (SettingsSection::Schedule, tab.schedule_inputs.fields.len()),
            (
                SettingsSection::Notifications,
                tab.notify_inputs.fields.len() + 3,
            ),
            (SettingsSection::Theme, 1),
        ];

        for (section, control_count) in sections {
            tab.current_section = section;
            tab.focus_area = SettingsFocusArea::SectionDetail;
            assert_eq!(
                tab.max_focus_index(),
                control_count.saturating_sub(1),
                "Settings {:?} focus limit should match rendered controls",
                section
            );

            for idx in 0..control_count {
                tab.detail_focus_index = idx;
                tab.sync_component_focus();
                assert_eq!(
                    tab.detail_focus_index, idx,
                    "Settings {:?} should keep focus index {idx}",
                    section
                );
            }

            tab.detail_focus_index = control_count + 2;
            tab.sync_component_focus();
            assert_eq!(
                tab.detail_focus_index,
                control_count.saturating_sub(1),
                "Settings {:?} should clamp stale focus indexes",
                section
            );
        }
    }

    #[test]
    fn representative_tabs_keep_keyboard_access_to_start_controls() {
        let mut graph_ql = super::super::graphql::GraphQlTab::new();
        graph_ql.handle_down();
        assert!(graph_ql.is_input_focused());
        graph_ql.handle_enter();
        assert!(!graph_ql.is_input_focused());
        graph_ql.core.inputs.fields[0].value = "https://example.test/graphql".to_string();
        graph_ql.handle_enter();
        assert!(graph_ql.is_running());

        let mut oauth = super::super::oauth::OAuthTab::new();
        oauth.handle_down();
        assert!(oauth.is_input_focused());
        oauth.handle_enter();
        assert!(!oauth.is_input_focused());
        oauth.core.inputs.fields[0].value = "https://example.test/oauth/authorize".to_string();
        oauth.handle_enter();
        assert!(oauth.is_running());
    }
}
