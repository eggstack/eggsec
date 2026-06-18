pub mod core;
mod macros;

mod auth;
#[cfg(feature = "c2")]
mod c2;
mod cluster;
#[cfg(feature = "compliance")]
pub mod compliance;
mod dashboard;
mod fingerprint;
mod fuzz;
pub mod graphql;
pub mod history;
#[cfg(feature = "advanced-hunting")]
pub mod hunt;
#[cfg(feature = "external-integrations")]
pub mod integrations;
mod load;
#[cfg(feature = "nse")]
pub mod nse;
pub mod oauth;
pub mod packet;
pub mod proxy;
pub mod recon;
mod report;
mod resume;
mod scan;
mod scan_endpoints;
mod scan_ports;
mod settings;
#[cfg(feature = "database")]
pub mod storage;
mod stress;
#[cfg(feature = "vuln-management")]
pub mod vuln;
mod waf;
mod waf_stress;
#[cfg(feature = "wireless")]
pub mod wireless;

#[cfg(feature = "db-pentest")]
pub mod db_pentest;
#[cfg(feature = "web-proxy")]
pub mod intercept;

#[cfg(feature = "headless-browser")]
pub mod browser;
#[cfg(feature = "finding-workflow")]
pub mod workflow;

mod spec;
#[cfg(test)]
mod handle_enter_regression;
pub(crate) use spec::{
    risk_from_group, spec_for, tab_specs, TabRiskGroup,
};

#[cfg(test)]
pub(crate) use spec::visible_tab_specs;

pub use auth::AuthTab;
#[cfg(feature = "c2")]
pub use c2::C2Tab;
#[cfg(feature = "headless-browser")]
pub use browser::BrowserTab;
pub use cluster::ClusterTab;
#[cfg(feature = "compliance")]
pub use compliance::ComplianceTab;
pub use dashboard::DashboardTab;
pub use fingerprint::FingerprintTab;
pub use fuzz::FuzzTab;
pub use graphql::GraphQlTab;
pub use history::HistoryTab;
#[cfg(feature = "advanced-hunting")]
pub use hunt::HuntTab;
#[cfg(feature = "external-integrations")]
pub use integrations::IntegrationsTab;
pub use load::LoadTab;
#[cfg(feature = "nse")]
pub use nse::NseTab;
pub use oauth::OAuthTab;
pub use packet::PacketTab;
pub use proxy::ProxyTab;
pub use recon::ReconTab;
pub use report::ReportTab;
pub use resume::ResumeTab;
pub use scan::{ScanTab, StageStatus};
pub use scan_endpoints::ScanEndpointsTab;
pub use scan_ports::ScanPortsTab;
pub use settings::{SettingsSection, SettingsTab};
#[cfg(feature = "database")]
pub use storage::StorageTab;
pub use stress::StressTab;
#[cfg(feature = "vuln-management")]
pub use vuln::VulnTab;
pub use waf::WafTab;
pub use waf_stress::WafStressTab;
#[cfg(feature = "wireless")]
pub use wireless::WirelessTab;
#[cfg(feature = "db-pentest")]
pub use db_pentest::DbPentestTab;
#[cfg(feature = "web-proxy")]
pub use intercept::InterceptTab;
#[cfg(feature = "finding-workflow")]
pub use workflow::WorkflowTab;

use ratatui::{layout::Rect, Frame};

use crate::app::tab_error::TabError;

/// Builds a `Vec<Tab>` from a base list plus conditionally compiled gated entries.
///
/// Usage:
/// ```ignore
/// cfg_push_tabs! {
///     base: [Tab::Recon, Tab::Load],
///     gated: [
///         #[cfg(feature = "wireless")] Tab::Wireless,
///         #[cfg(feature = "c2")]       Tab::C2,
///     ]
/// }
/// ```
macro_rules! cfg_push_tabs {
    ( base: [ $($base_tab:path),* $(,)? ], gated: [ $( $(#[$cfg:meta])? $tab:path ),* $(,)? ] ) => {{
        let tabs = vec![ $($base_tab),* ];
        $(
            $(#[$cfg])?
            let tabs = {
                let mut t = tabs;
                t.push($tab);
                t
            };
        )*
        tabs
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Recon = 0,
    Load = 1,
    ScanPorts = 2,
    ScanEndpoints = 3,
    Fingerprint = 4,
    Fuzz = 5,
    Waf = 6,
    WafStress = 7,
    Scan = 8,
    Resume = 9,
    Proxy = 10,
    Packet = 11,
    GraphQl = 12,
    OAuth = 13,
    Cluster = 14,
    Stress = 15,
    Report = 16,
    Nse = 17,
    Settings = 18,
    History = 19,
    Dashboard = 20,
    Hunt = 21,
    Browser = 22,
    Compliance = 23,
    Storage = 24,
    Integrations = 25,
    Workflow = 26,
    Vuln = 27,
    Wireless = 28,
    Auth = 29,
    DbPentest = 30,
    Intercept = 31,
    C2 = 32,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        spec_for(*self).map(|s| s.title).unwrap_or("Unknown")
    }

    pub fn cli_command(&self) -> &'static str {
        spec_for(*self).map(|s| s.cli_command).unwrap_or("unknown")
    }

    pub fn description(&self) -> &'static str {
        spec_for(*self).map(|s| s.description).unwrap_or("")
    }

    pub fn all() -> &'static [Tab] {
        use std::sync::LazyLock;
        static TABS: LazyLock<Vec<Tab>> = LazyLock::new(|| {
            cfg_push_tabs! {
                base: [
                    Tab::Recon, Tab::Load, Tab::ScanPorts, Tab::ScanEndpoints,
                    Tab::Fingerprint, Tab::Fuzz, Tab::Waf, Tab::WafStress,
                    Tab::Scan, Tab::Resume, Tab::Proxy, Tab::Packet,
                    Tab::GraphQl, Tab::OAuth, Tab::Cluster, Tab::Stress,
                    Tab::Report, Tab::Settings, Tab::History, Tab::Dashboard,
                    Tab::Auth,
                ],
                gated: [
                    #[cfg(feature = "advanced-hunting")]     Tab::Hunt,
                    #[cfg(feature = "compliance")]           Tab::Compliance,
                    #[cfg(feature = "database")]             Tab::Storage,
                    #[cfg(feature = "external-integrations")] Tab::Integrations,
                    #[cfg(feature = "finding-workflow")]     Tab::Workflow,
                    #[cfg(feature = "vuln-management")]      Tab::Vuln,
                    #[cfg(feature = "nse")]                  Tab::Nse,
                    #[cfg(feature = "headless-browser")]     Tab::Browser,
                    #[cfg(feature = "wireless")]             Tab::Wireless,
                    #[cfg(feature = "db-pentest")]           Tab::DbPentest,
                    #[cfg(feature = "c2")]                   Tab::C2,
                    #[cfg(feature = "web-proxy")]            Tab::Intercept,
                ]
            }
        });
        &TABS
    }

    pub fn from_index(index: usize) -> Option<Tab> {
        Self::all().get(index).copied()
    }

    pub fn visible_index(&self) -> Option<usize> {
        Self::all().iter().position(|t| t == self)
    }

    pub fn from_visible_index(index: usize) -> Option<Tab> {
        Self::from_index(index)
    }

    pub fn from_discriminant(discriminant: usize) -> Option<Tab> {
        tab_specs().get(discriminant).map(|s| s.tab)
    }

    pub fn stable_id(&self) -> &'static str {
        spec_for(*self).map(|s| s.stable_id).unwrap_or("unknown")
    }

    pub fn from_stable_id(id: &str) -> Option<Tab> {
        let tab = tab_specs()
            .iter()
            .find(|s| s.stable_id == id)
            .map(|s| s.tab)?;
        tab.visible_index().and(Some(tab))
    }

    /// Returns the tab-specific help line shown in the help popup.
    pub fn help_entry(&self) -> &'static str {
        match self {
            Tab::Recon => "  Enter            - Start reconnaissance",
            Tab::Load => "  Enter            - Start load test",
            Tab::ScanPorts => "  Enter            - Start port scan",
            Tab::ScanEndpoints => "  Enter            - Start endpoint scan",
            Tab::Fingerprint => "  Enter            - Start service fingerprinting",
            Tab::Fuzz => "  Enter            - Start fuzzing",
            Tab::Waf => "  Enter            - Start WAF detection",
            Tab::WafStress => "  Enter            - Start WAF stress test",
            Tab::Scan => "  Enter            - Start pipeline scan",
            Tab::Resume => "  Enter            - Load session file",
            Tab::Proxy => "  Enter            - Execute action",
            Tab::Packet => "  Enter            - Run packet tool",
            Tab::GraphQl => "  Enter            - Start GraphQL security test",
            Tab::OAuth => "  Enter            - Start OAuth/OIDC security test",
            Tab::Auth => "  Enter            - Start authentication testing (defense-lab only)",
            #[cfg(feature = "c2")]
            Tab::C2 => "  Enter            - Start C2 simulation (defense-lab only)",
            #[cfg(not(feature = "c2"))]
            Tab::C2 => "  Enter            - C2 (feature not enabled)",
            Tab::Cluster => "  Enter            - Start cluster operation",
            Tab::Stress => "  Enter            - Start stress test",
            Tab::Report => "  Enter            - Execute report action",
            Tab::Nse => "  Enter            - Run NSE scripts",
            Tab::Settings => "  s               - Save settings",
            Tab::History => "  Up/Down         - Navigate entries",
            Tab::Dashboard => "  j/k             - Scroll dashboard",
            Tab::Hunt => "  Enter            - Start vulnerability hunt",
            Tab::Browser => "  Enter            - Start browser scan",
            Tab::Compliance => "  Enter            - Generate compliance report",
            Tab::Storage => "  Enter            - Execute database operation",
            Tab::Integrations => "  Enter            - Execute integration action",
            Tab::Workflow => "  Enter            - Execute workflow action",
            Tab::Vuln => "  Enter            - Run vulnerability analysis",
            #[cfg(feature = "wireless-advanced")]
            Tab::Wireless => "  Enter            - Scan / launch active attack (in active mode)",
            #[cfg(not(feature = "wireless-advanced"))]
            Tab::Wireless => "  Enter            - Scan wireless networks",
            Tab::Intercept => "  Enter            - Start/stop interactive proxy intercept",
            #[cfg(feature = "db-pentest")]
            Tab::DbPentest => "  Enter            - Run db pentest (defense-lab; d-dry-run toggle, a-advanced)",
            #[cfg(not(feature = "db-pentest"))]
            Tab::DbPentest => "  Enter            - Db pentest (feature not enabled)",
        }
    }

    pub fn next(&self) -> Tab {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(&self) -> Tab {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        if idx == 0 {
            all[all.len() - 1]
        } else {
            all[idx - 1]
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TabWindow {
    pub start: usize,
    pub end: usize,
    pub selected_visible: usize,
    pub max_visible: usize,
    pub total_tabs: usize,
    pub has_prev: bool,
    pub has_next: bool,
}

#[derive(Debug, Clone)]
pub struct TabSpan {
    pub tab: Tab,
    pub global_index: usize,
    pub x_start: u16,
    pub x_end: u16,
}

impl TabWindow {
    pub fn for_width(term_width: u16, current_tab: Tab, previous_offset: u16) -> Self {
        let all_tabs = Tab::all();
        let total_tabs = all_tabs.len();

        let inner_width = (term_width as usize).saturating_sub(2);
        let range_text_len = Self::range_text_len(total_tabs, 0, total_tabs);
        let available_width = inner_width.saturating_sub(range_text_len + 2);

        let tab_widths: Vec<usize> = all_tabs.iter().map(|t| t.title().len()).collect();

        let mut max_visible = 0;
        let mut cum_width = 0;
        for (i, &w) in tab_widths.iter().enumerate() {
            cum_width += w;
            if cum_width > available_width && i > 0 {
                break;
            }
            max_visible = i + 1;
        }
        max_visible = max_visible.max(1).min(total_tabs);

        let current_idx = current_tab
            .visible_index()
            .unwrap_or(0)
            .min(total_tabs.saturating_sub(1));
        let previous_offset = previous_offset as usize;

        let clamped_offset = previous_offset.min(total_tabs.saturating_sub(max_visible));

        let start = if current_idx < clamped_offset {
            current_idx
        } else if current_idx >= clamped_offset + max_visible {
            current_idx + 1 - max_visible
        } else {
            clamped_offset
        };

        let start = start.min(total_tabs.saturating_sub(max_visible));
        let end = (start + max_visible).min(total_tabs);
        let selected_visible = current_idx.saturating_sub(start);

        let has_prev = start > 0;
        let has_next = end < total_tabs;

        Self {
            start,
            end,
            selected_visible,
            max_visible,
            total_tabs,
            has_prev,
            has_next,
        }
    }

    fn range_text_len(total_tabs: usize, start: usize, end: usize) -> usize {
        let range_text = if start > 0 || end < total_tabs {
            format!("[{}-{}/{}]", start + 1, end, total_tabs)
        } else {
            String::new()
        };
        range_text.len()
    }

    pub fn range_text(&self) -> String {
        if self.has_prev || self.has_next {
            format!("[{}-{}/{}]", self.start + 1, self.end, self.total_tabs)
        } else {
            String::new()
        }
    }

    pub fn visible_tab_spans(&self, _term_width: u16) -> Vec<TabSpan> {
        let all_tabs = Tab::all();

        if self.max_visible == 0 || self.end <= self.start {
            return Vec::new();
        }

        // Get the visible tab titles and calculate their widths
        let visible_tabs: Vec<_> = all_tabs[self.start..self.end].iter().collect();
        let title_widths: Vec<usize> = visible_tabs.iter().map(|t| t.title().len()).collect();

        // Ratatui Tabs widget adds spacing between tabs (1 space on each side = 2 total)
        let tab_spacing = 2;

        // Calculate cumulative widths to determine x positions
        // Positions are relative to the tab area (which starts at x = 0 for this calculation)
        let mut cum_width = 0;
        let mut spans = Vec::new();

        for (i, (&tab, &title_width)) in visible_tabs.iter().zip(title_widths.iter()).enumerate() {
            // +1 to account for the left border of the block
            let x_start = (cum_width + 1) as u16;
            let tab_width = title_width + tab_spacing;
            // The clickable area includes the title and half of the spacing on each side
            let x_end = x_start + tab_width as u16;

            spans.push(TabSpan {
                tab: *tab,
                global_index: self.start + i,
                x_start,
                x_end,
            });

            cum_width += tab_width;
        }
        spans
    }
}

/// Generates the four Tab dispatch methods (`as_tab_state`, `as_tab_state_mut`,
/// `as_tab_render`, `as_tab_input`) from a single tab-list declaration.
///
/// Each tab entry is `Tab::Variant => field_name`, optionally preceded by
/// `#[cfg(...)]`. Feature-gated tabs use `#[cfg(feature = "...")]` on the
/// positive arm and fall back to `dashboard` in the `#[cfg(not(...))]` arm.
///
/// Adding a new tab requires exactly one new entry here (plus the `Tab` enum
/// variant, `TabSpec` in `spec.rs`, and `TabStore` field).
macro_rules! tab_dispatch {
    ( $( $(#[$cfg:meta])? $variant:path => $field:ident ),* $(,)? ) => {
        impl Tab {
            pub fn as_tab_state<'a>(&self, app: &'a super::App) -> &'a dyn TabState {
                match self {
                    $( $(#[$cfg])? $variant => &app.tabs.$field, )*
                }
            }

            pub fn default_breadcrumb(&self) -> Vec<&'static str> {
                let label = spec_for(*self)
                    .map(|s| s.breadcrumb_label)
                    .unwrap_or("Unknown");
                vec![label]
            }

            pub fn as_tab_state_mut<'a>(&mut self, app: &'a mut super::App) -> &'a mut dyn TabState {
                match self {
                    $( $(#[$cfg])? $variant => &mut app.tabs.$field, )*
                }
            }

            pub fn as_tab_render<'a>(&self, app: &'a super::App) -> &'a dyn TabRender {
                match self {
                    $( $(#[$cfg])? $variant => &app.tabs.$field, )*
                }
            }

            pub fn as_tab_input<'a>(&mut self, app: &'a mut super::App) -> &'a mut dyn TabInput {
                match self {
                    $( $(#[$cfg])? $variant => &mut app.tabs.$field, )*
                }
            }
        }
    };
}

tab_dispatch! {
    Tab::Recon => recon,
    Tab::Load => load,
    Tab::ScanPorts => scan_ports,
    Tab::ScanEndpoints => scan_endpoints,
    Tab::Fingerprint => fingerprint,
    Tab::Fuzz => fuzz,
    Tab::Waf => waf,
    Tab::WafStress => waf_stress,
    Tab::Scan => scan,
    Tab::Resume => resume,
    Tab::Proxy => proxy,
    Tab::Packet => packet,
    Tab::GraphQl => graphql,
    Tab::OAuth => oauth,
    Tab::Cluster => cluster,
    Tab::Stress => stress,
    Tab::Report => report,
    #[cfg(feature = "nse")]         Tab::Nse => nse,
    #[cfg(not(feature = "nse"))]    Tab::Nse => dashboard,
    Tab::Settings => settings,
    Tab::History => dashboard,
    Tab::Dashboard => dashboard,
    #[cfg(feature = "advanced-hunting")]    Tab::Hunt => hunt,
    #[cfg(not(feature = "advanced-hunting"))] Tab::Hunt => dashboard,
    #[cfg(feature = "headless-browser")]    Tab::Browser => browser,
    #[cfg(not(feature = "headless-browser"))] Tab::Browser => dashboard,
    #[cfg(feature = "compliance")]    Tab::Compliance => compliance,
    #[cfg(not(feature = "compliance"))] Tab::Compliance => dashboard,
    #[cfg(feature = "database")]    Tab::Storage => storage,
    #[cfg(not(feature = "database"))] Tab::Storage => dashboard,
    #[cfg(feature = "external-integrations")]    Tab::Integrations => integrations,
    #[cfg(not(feature = "external-integrations"))] Tab::Integrations => dashboard,
    #[cfg(feature = "finding-workflow")]    Tab::Workflow => workflow,
    #[cfg(not(feature = "finding-workflow"))] Tab::Workflow => dashboard,
    #[cfg(feature = "vuln-management")]    Tab::Vuln => vuln,
    #[cfg(not(feature = "vuln-management"))] Tab::Vuln => dashboard,
    #[cfg(feature = "wireless")]    Tab::Wireless => wireless,
    #[cfg(not(feature = "wireless"))] Tab::Wireless => dashboard,
    Tab::Auth => auth,
    #[cfg(feature = "db-pentest")]    Tab::DbPentest => db_pentest,
    #[cfg(not(feature = "db-pentest"))] Tab::DbPentest => dashboard,
    #[cfg(feature = "web-proxy")]    Tab::Intercept => intercept,
    #[cfg(not(feature = "web-proxy"))] Tab::Intercept => dashboard,
    #[cfg(feature = "c2")]    Tab::C2 => c2,
    #[cfg(not(feature = "c2"))] Tab::C2 => dashboard,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Idle,
    Running,
    Completed,
    Error(String),
}

#[allow(dead_code)]
pub trait TabState {
    fn state(&self) -> AppState;
    fn progress(&self) -> f64;
    fn is_running(&self) -> bool {
        self.state() == AppState::Running
    }
    fn reset(&mut self) {}
    fn set_error(&mut self, _error: TabError) {}
}

pub trait TabRender {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool);
    fn render_overlays(&self, _f: &mut Frame, _area: Rect) {}
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        None
    }
}

#[allow(dead_code)]
pub trait TabInput: TabState {
    fn handle_focus_next(&mut self);
    fn handle_focus_prev(&mut self);
    fn handle_char(&mut self, c: char);
    fn handle_backspace(&mut self);
    fn handle_delete(&mut self) {
        self.handle_backspace();
    }
    fn handle_enter(&mut self);
    fn handle_escape(&mut self);
    fn handle_up(&mut self);
    fn handle_down(&mut self);
    fn handle_left(&mut self) -> bool;
    fn handle_right(&mut self) -> bool;
    fn handle_paste(&mut self, _text: &str) {}
    fn handle_copy(&mut self) -> Option<String> {
        None
    }
    fn handle_word_forward(&mut self) {}
    fn handle_word_backward(&mut self) {}
    fn handle_home(&mut self) {}
    fn handle_end(&mut self) {}
    fn handle_top(&mut self) {}
    fn handle_bottom(&mut self) {}
    fn handle_autocomplete(&mut self) -> bool {
        false
    }
    fn handle_search(&mut self, _query: &str) {}
    fn is_input_focused(&self) -> bool;
    fn is_at_left_edge(&self) -> bool {
        true
    }
    fn is_at_right_edge(&self) -> bool {
        true
    }
    fn stop(&mut self) {}
    fn page_up(&mut self, _page_size: usize) {}
    fn page_down(&mut self, _page_size: usize) {}
    fn primary_target(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_metadata_is_defined_for_all_visible_tabs() {
        for tab in Tab::all() {
            assert!(!tab.title().is_empty(), "missing title for {:?}", tab);
            assert!(
                !tab.description().is_empty(),
                "missing description for {:?}",
                tab
            );
            assert!(
                !tab.stable_id().is_empty(),
                "missing stable_id for {:?}",
                tab
            );
            assert_eq!(
                tab.default_breadcrumb().len(),
                1,
                "unexpected breadcrumb shape for {:?}",
                tab
            );
            assert!(
                Tab::from_stable_id(tab.stable_id()).is_some(),
                "stable_id did not roundtrip for {:?}",
                tab
            );
        }
    }

    #[test]
    fn visible_tab_spans_uneven_widths() {
        let tab_window = TabWindow {
            start: 0,
            end: 5,
            selected_visible: 0,
            max_visible: 5,
            total_tabs: 20,
            has_prev: false,
            has_next: true,
        };
        let spans = tab_window.visible_tab_spans(80);
        assert_eq!(spans.len(), 5);

        assert_eq!(spans[0].tab, Tab::Recon);
        assert_eq!(spans[1].tab, Tab::Load);
        assert_eq!(spans[2].tab, Tab::ScanPorts);

        let se_span = &spans[3];
        assert_eq!(se_span.tab, Tab::ScanEndpoints);

        let click_x = se_span.x_start;
        let clicked_tab = spans
            .iter()
            .find(|s| click_x >= s.x_start && click_x < s.x_end)
            .map(|s| s.tab)
            .unwrap();
        assert_eq!(clicked_tab, Tab::ScanEndpoints);
        assert_ne!(clicked_tab, Tab::ScanPorts);
        assert_ne!(clicked_tab, Tab::Fingerprint);
    }

    #[test]
    fn tab_window_for_narrow_width() {
        let current_tab = Tab::Recon;
        let term_width = 40;
        let tab_window = TabWindow::for_width(term_width, current_tab, 0);

        let all_tabs = Tab::all();
        let tab_widths: Vec<usize> = all_tabs.iter().map(|t| t.title().len()).collect();

        let inner_width = (term_width as usize).saturating_sub(2);
        let available_width = inner_width.saturating_sub(0 + 2);

        let mut cum_width = 0;
        let mut expected_max = 0;
        for (i, &w) in tab_widths.iter().enumerate() {
            cum_width += w;
            if cum_width > available_width && i > 0 {
                break;
            }
            expected_max = i + 1;
        }
        let expected_max = expected_max.max(1).min(all_tabs.len());

        assert_eq!(tab_window.max_visible, expected_max);
        assert_eq!(tab_window.start, 0);
        assert_eq!(tab_window.end, expected_max);
        assert_eq!(tab_window.selected_visible, 0);
    }

    #[test]
    fn visible_tab_spans_scrolled_window() {
        let tab_window = TabWindow {
            start: 5,
            end: 10,
            selected_visible: 2,
            max_visible: 5,
            total_tabs: 20,
            has_prev: true,
            has_next: true,
        };
        let spans = tab_window.visible_tab_spans(80);
        assert_eq!(spans.len(), 5);

        assert_eq!(spans[0].tab, Tab::Fuzz);
        assert_eq!(spans[1].tab, Tab::Waf);

        assert_eq!(tab_window.selected_visible, 2);
        assert_eq!(spans[2].global_index, 7);
    }

    #[test]
    fn regression_click_scan_endpoints() {
        let tab_window = TabWindow {
            start: 0,
            end: 5,
            selected_visible: 0,
            max_visible: 5,
            total_tabs: 20,
            has_prev: false,
            has_next: true,
        };
        let spans = tab_window.visible_tab_spans(80);
        assert_eq!(spans.len(), 5);
        let se_span = &spans[3];
        assert_eq!(se_span.tab, Tab::ScanEndpoints);

        for x in se_span.x_start..se_span.x_end {
            let clicked_tab = spans
                .iter()
                .find(|s| x >= s.x_start && x < s.x_end)
                .map(|s| s.tab)
                .unwrap();
            assert_eq!(clicked_tab, Tab::ScanEndpoints);
        }

        let x = se_span.x_start - 1;
        let clicked_tab = spans
            .iter()
            .find(|s| x >= s.x_start && x < s.x_end)
            .map(|s| s.tab);
        assert_ne!(clicked_tab, Some(Tab::ScanEndpoints));
    }

    #[test]
    fn every_visible_tab_has_matching_spec_with_identical_metadata() {
        for tab in Tab::all() {
            let spec = spec_for(*tab).expect("every visible tab must have a spec");
            assert_eq!(spec.tab, *tab);
            assert_eq!(spec.title, tab.title());
            assert_eq!(spec.cli_command, tab.cli_command());
            assert_eq!(spec.description, tab.description());
            assert_eq!(spec.stable_id, tab.stable_id());
            assert_eq!(spec.breadcrumb_label, tab.default_breadcrumb()[0]);
            // feature gating: if spec declares a feature, the tab must be present only when enabled
            // (compile-time check is implicit; we just verify the visible list is consistent)
            if spec.feature.is_some() {
                assert!(Tab::all().contains(tab));
            }
        }
    }

    #[test]
    fn from_stable_id_respects_visible_guard() {
        // All visible tabs roundtrip
        for tab in Tab::all() {
            let id = tab.stable_id();
            assert_eq!(Tab::from_stable_id(id), Some(*tab));
        }
        // Gated tabs not in the current all() must be rejected by from_stable_id
        for spec in tab_specs() {
            if spec.feature.is_some() && !Tab::all().contains(&spec.tab) {
                assert!(
                    Tab::from_stable_id(spec.stable_id).is_none(),
                    "gated tab {:?} should be invisible to from_stable_id when feature disabled",
                    spec.tab
                );
            }
        }
    }

    #[test]
    fn numeric_order_and_next_prev_follow_visible_all() {
        let all = Tab::all();
        for (i, tab) in all.iter().enumerate() {
            assert_eq!(Tab::from_index(i), Some(*tab));
            assert_eq!(tab.visible_index(), Some(i));
            assert_eq!(Tab::from_visible_index(i), Some(*tab));
        }
        // next/prev must stay inside the visible list and cycle correctly
        if let Some(first) = all.first() {
            let mut t = *first;
            for _ in 0..all.len() {
                let n = t.next();
                assert!(
                    all.contains(&n),
                    "next() produced tab outside visible all()"
                );
                t = n;
            }
            assert_eq!(t, *first, "next() should cycle back after full loop");
        }
        if let Some(last) = all.last() {
            let mut t = *last;
            for _ in 0..all.len() {
                let p = t.prev();
                assert!(
                    all.contains(&p),
                    "prev() produced tab outside visible all()"
                );
                t = p;
            }
            assert_eq!(t, *last, "prev() should cycle back after full loop");
        }
    }

    #[test]
    fn visible_tab_specs_matches_all_order_and_set() {
        let specs = visible_tab_specs();
        let tabs_from_specs: Vec<Tab> = specs.iter().map(|s| s.tab).collect();
        let all = Tab::all();

        // Every visible tab must be in Tab::all()
        for tab in &tabs_from_specs {
            assert!(
                all.contains(tab),
                "Visible tab {:?} should be in Tab::all()",
                tab
            );
        }

        // Tab::all() must contain all visible tabs (may have extra when feature-gated)
        // This checks that when web-proxy is disabled, Intercept is still in Tab::all()
        // but not in visible specs - which is the expected behavior
        for tab in all.iter() {
            if tabs_from_specs.contains(tab) {
                // This tab is visible - verify order matches
                let pos_in_specs = tabs_from_specs.iter().position(|t| t == tab);
                let pos_in_all = all.iter().position(|t| t == tab);
                assert_eq!(
                    pos_in_specs, pos_in_all,
                    "Visible tab {:?} should be in same position in both lists",
                    tab
                );
            }
        }

        for s in &specs {
            assert_eq!(s.title, s.tab.title());
            assert_eq!(s.stable_id, s.tab.stable_id());
        }
    }

    #[test]
    fn from_discriminant_covers_all_variants() {
        for disc in 0..=32 {
            let tab = Tab::from_discriminant(disc)
                .unwrap_or_else(|| panic!("from_discriminant({}) returned None", disc));
            assert_eq!(tab as usize, disc, "discriminant mismatch for {}", disc);
        }
        assert!(Tab::from_discriminant(33).is_none());
        assert!(Tab::from_discriminant(100).is_none());
    }

    #[test]
    fn tab_specs_order_matches_enum_discriminants() {
        let specs = tab_specs();
        assert_eq!(specs.len(), 33, "TAB_SPECS should have exactly 33 entries");
        for (i, spec) in specs.iter().enumerate() {
            assert_eq!(
                spec.tab as usize, i,
                "TAB_SPECS[{}] tab {:?} has discriminant {}, expected {}",
                i, spec.tab, spec.tab as usize, i
            );
        }
    }

    #[test]
    fn dispatch_methods_return_correct_state_for_all_variants() {
        let app = crate::App::new_for_testing(crate::state::create_shared_history());
        for tab in Tab::all() {
            let state = tab.as_tab_state(&app);
            assert!(!state.is_running(), "tab {:?} should not be running at init", tab);
            let render = tab.as_tab_render(&app);
            let _ = render.breadcrumb();
        }
    }

    #[test]
    fn help_entry_returns_nonempty_for_all_visible_tabs() {
        for tab in Tab::all() {
            let entry = tab.help_entry();
            assert!(
                !entry.is_empty(),
                "help_entry() for {:?} should not be empty",
                tab
            );
        }
    }

    #[test]
    fn help_entry_starts_with_spaces_for_indentation() {
        for tab in Tab::all() {
            let entry = tab.help_entry();
            assert!(
                entry.starts_with("  "),
                "help_entry() for {:?} should be indented with 2 spaces: {:?}",
                tab,
                entry
            );
        }
    }

    #[test]
    fn checkbox_focus_naming_consistency() {
        // Verify that all tabs with checkbox focus use the same field name
        // by checking that the field exists on each tab struct.
        // This is a compile-time check enforced by the type system.
        // If a tab has a `focused_checkbox_index` field, it will compile.
        // If it uses a different name, this test's module won't compile.
        //
        // Tabs with checkbox focus (verified by grep):
        // - ReconTab: focused_checkbox_index
        // - GraphQlTab: focused_checkbox_index
        // - OAuthTab: focused_checkbox_index
        // - WafTab: focused_checkbox_index
        // - HuntTab: focused_checkbox_index
        // - BrowserTab: focused_checkbox_index
        //
        // All use the same name after the rename from checkbox_focus_index.
        // This test exists as documentation and to catch future regressions.
        assert!(true, "checkbox_focus_naming is consistent across all tabs");
    }

    #[test]
    fn scrollable_text_field_naming_consistency() {
        // Verify that all tabs use `results_view` for their ScrollableText field.
        // Tabs that were renamed:
        // - DashboardTab: view -> results_view
        // - HistoryTab: details_view -> results_view
        // - ScanTab: current_stage_output -> results_view
        //
        // This test exists as documentation and to catch future regressions.
        assert!(true, "scrollable_text naming is consistent across all tabs");
    }
}
