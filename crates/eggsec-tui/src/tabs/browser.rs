use crate::components::{empty_state_paragraph, Checkbox};
use crate::tabs::core::{
    self, render_config_block, render_error_block, render_input_fields, StandardFocusArea, TabCore,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_3area, tab_state_boilerplate, tc};
use eggsec::browser::{BrowserConfig, BrowserReport};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct BrowserTab {
    pub core: TabCore,
    pub report: Option<BrowserReport>,
    pub config: BrowserConfig,
    pub option_checkboxes: Vec<Checkbox>,
    pub focused_checkbox_index: usize,
    pub focus_area: StandardFocusArea,
}

impl BrowserTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(crate::components::InputField::new("Target URL"))
            .add(
                crate::components::InputField::new("Timeout (ms)")
                    .with_value(&eggsec_core::constants::DEFAULT_BROWSER_TIMEOUT_MS.to_string()),
            );

        let option_checkboxes = vec![
            Checkbox::new("DOM XSS Scan").checked(true),
            Checkbox::new("SPA Route Discovery").checked(true),
            Checkbox::new("Client Security Checks").checked(true),
        ];

        Self {
            core: TabCore::new("Running browser scan...", "Results").with_inputs(inputs),
            report: None,
            config: BrowserConfig::default(),
            option_checkboxes,
            focused_checkbox_index: 0,
            focus_area: StandardFocusArea::Inputs,
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn timeout_ms(&self) -> u64 {
        self.core
            .inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(eggsec_core::constants::DEFAULT_BROWSER_TIMEOUT_MS)
    }

    pub fn get_config(&self) -> BrowserConfig {
        BrowserConfig {
            check_dom_xss: self
                .option_checkboxes
                .first()
                .map(|cb| cb.checked)
                .unwrap_or(false),
            discover_spa_routes: self
                .option_checkboxes
                .get(1)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            check_client_security: self
                .option_checkboxes
                .get(2)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            timeout_ms: self.timeout_ms(),
            xss_payload: BrowserConfig::default().xss_payload,
        }
    }

    pub fn set_report(&mut self, report: BrowserReport) {
        self.report = Some(report.clone());
        self.core.state = AppState::Completed;
        self.core.results_view.clear();

        self.core.results_view.add_line(Line::from(Span::styled(
            format!("Browser Scan Complete: {}", report.target),
            Style::default().fg(tc!(success)),
        )));
        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(Span::styled(
            format!("Total findings: {}", report.total_findings),
            Style::default().fg(tc!(warning)),
        )));
        self.core.results_view.add_line(Line::from(""));

        if !report.dom_xss.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                format!("DOM XSS Findings ({}):", report.dom_xss.len()),
                Style::default().fg(tc!(error)),
            )));
            for finding in &report.dom_xss {
                self.core.results_view.add_line(Line::from(format!(
                    "  [{}] {} -> {} at {}",
                    finding.severity, finding.source, finding.sink, finding.location
                )));
            }
            self.core.results_view.add_line(Line::from(""));
        }

        if !report.spa_routes.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                format!("SPA Routes Discovered ({}):", report.spa_routes.len()),
                Style::default().fg(tc!(info)),
            )));
            for route in &report.spa_routes {
                self.core.results_view.add_line(Line::from(format!(
                    "  {} (via: {})",
                    route.path, route.discovered_via
                )));
            }
            self.core.results_view.add_line(Line::from(""));
        }

        if !report.client_issues.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                format!("Client Issues ({}):", report.client_issues.len()),
                Style::default().fg(tc!(warning)),
            )));
            for issue in &report.client_issues {
                self.core.results_view.add_line(Line::from(format!(
                    "  [{}] {} - {}",
                    issue.severity, issue.issue_type, issue.description
                )));
            }
        }
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.core.state = AppState::Running;
            self.core.progress.current = 0;
            self.core.progress.total = 0;
            self.report = None;
            self.core.results_view.clear();
        }
    }
}

impl Default for BrowserTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for BrowserTab {
    tab_state_boilerplate!(BrowserTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.report = None;
        self.focus_area = StandardFocusArea::Inputs;
        self.focused_checkbox_index = 0;
        for cb in &mut self.option_checkboxes {
            cb.checked = true;
        }
    }
}

impl TabRender for BrowserTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            StandardFocusArea::Inputs => "Inputs",
            StandardFocusArea::Options => "Options",
            StandardFocusArea::Results => "Results",
        };
        Some(vec!["Browser", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(14), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let config_inner = render_config_block(
            f,
            input_area,
            "Configuration",
            self.focus_area == StandardFocusArea::Inputs
                || self.focus_area == StandardFocusArea::Options,
        );

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(config_inner);

        render_input_fields(f, &input_chunks, &self.core.inputs, insert_mode);

        let Some(cb_area) = input_chunks.get(3) else {
            return;
        };
        let cb_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(*cb_area);

        for (i, cb) in self.option_checkboxes.iter().enumerate() {
            let mut checkbox = cb.clone();
            checkbox.focused =
                self.focus_area == StandardFocusArea::Options && i == self.focused_checkbox_index;
            if let Some(area) = cb_chunks.get(i) {
                checkbox.render(f, *area);
            }
        }

        if self.core.state == AppState::Running {
            self.core.progress.render(f, results_area);
        } else if let Some(ref err) = self.core.error {
            render_error_block(f, results_area, "Browser Scan - Error", err);
        } else if !self.core.results_view.is_empty() {
            self.core.results_view.render(f, results_area, None);
        } else {
            let placeholder = empty_state_paragraph(
                "Headless Browser Testing",
                "Enter target URL and press Enter to start browser scan",
            );
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for BrowserTab {
    tab_input_3area!(
        BrowserTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea::Inputs,
        Options: StandardFocusArea::Options,
        Results: StandardFocusArea::Results
    );

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = crate::tabs::core::focus_next_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
        );
        if self.focus_area == StandardFocusArea::Options {
            self.focused_checkbox_index = 0;
        }
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = crate::tabs::core::focus_prev_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
        );
        if self.focus_area == StandardFocusArea::Options {
            self.focused_checkbox_index = 0;
        }
    }

    fn handle_enter(&mut self) {
        let running = self.is_running();
        let inputs_focused = self.core.inputs.is_focused();
        crate::tabs::core::handle_enter_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
            running,
            inputs_focused,
            |_core| false,
        );
        if self.focus_area == StandardFocusArea::Options && !self.is_running() {
            crate::tabs::core::toggle_focused_checkbox_vec(
                &mut self.option_checkboxes,
                &mut self.focused_checkbox_index,
            );
        }
    }

    fn handle_escape(&mut self) {
        let new_area = crate::tabs::core::handle_escape_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
        );
        self.focus_area = new_area;
        self.focused_checkbox_index = 0;
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            if self.focus_area == StandardFocusArea::Options {
                crate::tabs::core::handle_options_up_wrapping(
                    &mut self.focused_checkbox_index,
                    self.option_checkboxes.len(),
                );
            } else {
                crate::tabs::core::handle_up_3area(
                    &mut self.core,
                    self.focus_area,
                    StandardFocusArea::Inputs,
                    StandardFocusArea::Results,
                );
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            if self.focus_area == StandardFocusArea::Options {
                crate::tabs::core::handle_options_down_wrapping(
                    &mut self.focused_checkbox_index,
                    self.option_checkboxes.len(),
                );
            } else {
                crate::tabs::core::handle_down_3area(
                    &mut self.core,
                    self.focus_area,
                    StandardFocusArea::Inputs,
                    StandardFocusArea::Results,
                );
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == StandardFocusArea::Options {
            crate::tabs::core::move_checkbox_focus_left(
                &mut self.focused_checkbox_index,
                self.option_checkboxes.len(),
            )
        } else {
            crate::tabs::core::handle_left_simple(&mut self.core, false)
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == StandardFocusArea::Options {
            crate::tabs::core::move_checkbox_focus_right(
                &mut self.focused_checkbox_index,
                self.option_checkboxes.len(),
            )
        } else {
            crate::tabs::core::handle_right_simple(&mut self.core, false)
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea::Options {
            crate::tabs::core::is_checkbox_focus_at_left_edge(
                self.focused_checkbox_index,
                self.option_checkboxes.len(),
            )
        } else {
            crate::tabs::core::is_at_left_edge_simple(
                self.focus_area,
                StandardFocusArea::Inputs,
                &self.core,
            )
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea::Options {
            crate::tabs::core::is_checkbox_focus_at_right_edge(
                self.focused_checkbox_index,
                self.option_checkboxes.len(),
            )
        } else {
            crate::tabs::core::is_at_right_edge_simple(
                self.focus_area,
                StandardFocusArea::Inputs,
                &self.core,
            )
        }
    }

    fn is_input_focused(&self) -> bool {
        crate::tabs::core::is_input_focused(
            self.focus_area,
            StandardFocusArea::Inputs,
            &self.core,
        )
    }

    fn page_up(&mut self, page_size: usize) {
        if !self.is_running() {
            self.core.results_view.page_up(page_size);
        }
    }

    fn page_down(&mut self, page_size: usize) {
        if !self.is_running() {
            self.core.results_view.page_down(page_size);
        }
    }

    fn primary_target(&self) -> Option<String> {
        Some(self.target().to_string())
    }
}
