use crate::browser::{BrowserConfig, BrowserReport};
use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct BrowserTab {
    pub inputs: InputGroup,
    pub report: Option<BrowserReport>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub config: BrowserConfig,
    pub option_checkboxes: Vec<Checkbox>,
    pub focused_checkbox_index: usize,
    pub focus_area: BrowserFocusArea,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrowserFocusArea {
    Inputs,
    Options,
    Results,
}

impl BrowserTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Timeout (ms)").with_value("60000"));

        let option_checkboxes = vec![
            Checkbox::new("DOM XSS Scan").checked(true),
            Checkbox::new("SPA Route Discovery").checked(true),
            Checkbox::new("Client Security Checks").checked(true),
        ];

        Self {
            inputs,
            report: None,
            progress: ProgressGauge::new("Running browser scan..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            config: BrowserConfig::default(),
            option_checkboxes,
            focused_checkbox_index: 0,
            focus_area: BrowserFocusArea::Inputs,
            error: None,
        }
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn timeout_ms(&self) -> u64 {
        self.inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(60000)
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
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            format!("Browser Scan Complete: {}", report.target),
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Total findings: {}", report.total_findings),
            Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(""));

        if !report.dom_xss.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("DOM XSS Findings ({}):", report.dom_xss.len()),
                Style::default().fg(tc!(error)),
            )));
            for finding in &report.dom_xss {
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {} -> {} at {}",
                    finding.severity, finding.source, finding.sink, finding.location
                )));
            }
            self.results_view.add_line(Line::from(""));
        }

        if !report.spa_routes.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("SPA Routes Discovered ({}):", report.spa_routes.len()),
                Style::default().fg(tc!(info)),
            )));
            for route in &report.spa_routes {
                self.results_view.add_line(Line::from(format!(
                    "  {} (via: {})",
                    route.path, route.discovered_via
                )));
            }
            self.results_view.add_line(Line::from(""));
        }

        if !report.client_issues.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("Client Issues ({}):", report.client_issues.len()),
                Style::default().fg(tc!(warning)),
            )));
            for issue in &report.client_issues {
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {} - {}",
                    issue.severity, issue.issue_type, issue.description
                )));
            }
        }
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.progress.current = 0;
            self.progress.total = 0;
            self.report = None;
            self.results_view.clear();
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.progress.current = completed;
        self.progress.total = total;
    }

}

impl Default for BrowserTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for BrowserTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.report = None;
        self.progress.current = 0;
        self.progress.total = 0;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.focus_area = BrowserFocusArea::Inputs;
        self.focused_checkbox_index = 0;
        for cb in &mut self.option_checkboxes {
            cb.checked = true;
        }
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for BrowserTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            BrowserFocusArea::Inputs => "Inputs",
            BrowserFocusArea::Options => "Options",
            BrowserFocusArea::Results => "Results",
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

        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(
                Style::default().fg(
                    if self.focus_area == BrowserFocusArea::Inputs
                        || self.focus_area == BrowserFocusArea::Options
                    {
                        tc!(border_focused)
                    } else {
                        tc!(border)
                    },
                ),
            );
        let config_inner = config_block.inner(input_area);
        f.render_widget(config_block, input_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(config_inner);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

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
                self.focus_area == BrowserFocusArea::Options && i == self.focused_checkbox_index;
            if let Some(area) = cb_chunks.get(i) {
                checkbox.render(f, *area);
            }
        }

        if self.state == AppState::Running {
            self.progress.render(f, results_area);
        } else if let Some(ref err) = self.error {
            use ratatui::widgets::{Block, Borders, Paragraph};
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Browser Scan - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, None);
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
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            BrowserFocusArea::Inputs => {
                self.inputs.blur();
                self.focused_checkbox_index = 0;
                BrowserFocusArea::Options
            }
            BrowserFocusArea::Options => BrowserFocusArea::Results,
            BrowserFocusArea::Results => {
                self.inputs.focus(0);
                BrowserFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            BrowserFocusArea::Inputs => {
                self.inputs.blur();
                BrowserFocusArea::Results
            }
            BrowserFocusArea::Options => {
                self.inputs.focus(0);
                BrowserFocusArea::Inputs
            }
            BrowserFocusArea::Results => {
                self.focused_checkbox_index = 0;
                BrowserFocusArea::Options
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == BrowserFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == BrowserFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == BrowserFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == BrowserFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else if self.focus_area == BrowserFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if self.focus_area == BrowserFocusArea::Inputs {
                self.inputs.move_word_forward();
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if self.focus_area == BrowserFocusArea::Inputs {
                self.inputs.move_word_backward();
            }
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == BrowserFocusArea::Inputs {
                self.inputs.move_home();
            } else if self.focus_area == BrowserFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == BrowserFocusArea::Inputs {
                self.inputs.move_end();
            } else if self.focus_area == BrowserFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = BrowserFocusArea::Inputs;
            self.inputs.focus(0);
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = BrowserFocusArea::Results;
            self.inputs.blur();
            self.focused_checkbox_index = 0;
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.focus_area == BrowserFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }

        if self.focus_area == BrowserFocusArea::Options {
            if let Some(checkbox) = self.option_checkboxes.get_mut(self.focused_checkbox_index)
            {
                checkbox.toggle();
            }
            return;
        }

        if self.focus_area == BrowserFocusArea::Results {
            return;
        }

        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.inputs.blur();
        self.focused_checkbox_index = 0;
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == BrowserFocusArea::Options {
            if self.option_checkboxes.is_empty() {
                return;
            }
            if self.focused_checkbox_index == 0 {
                self.focused_checkbox_index = self.option_checkboxes.len().saturating_sub(1);
            } else {
                self.focused_checkbox_index = self.focused_checkbox_index.saturating_sub(1);
            }
        } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
            self.results_view.scroll_up(1);
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == BrowserFocusArea::Options {
            if self.option_checkboxes.is_empty() {
                return;
            }
            if self.focused_checkbox_index >= self.option_checkboxes.len().saturating_sub(1) {
                self.focused_checkbox_index = 0;
            } else {
                self.focused_checkbox_index += 1;
            }
        } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
            self.results_view.scroll_down(1);
        } else {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == BrowserFocusArea::Inputs {
            self.inputs.move_left()
        } else if self.focus_area == BrowserFocusArea::Options {
            if self.option_checkboxes.is_empty() {
                return false;
            }
            if self.focused_checkbox_index == 0 {
                false
            } else {
                self.focused_checkbox_index = self.focused_checkbox_index.saturating_sub(1);
                true
            }
        } else {
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == BrowserFocusArea::Inputs {
            self.inputs.move_right()
        } else if self.focus_area == BrowserFocusArea::Options {
            if self.option_checkboxes.is_empty() {
                return false;
            }
            if self.focused_checkbox_index >= self.option_checkboxes.len().saturating_sub(1) {
                false
            } else {
                self.focused_checkbox_index += 1;
                true
            }
        } else {
            true
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == BrowserFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else if self.focus_area == BrowserFocusArea::Options {
            self.option_checkboxes.is_empty() || self.focused_checkbox_index == 0
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == BrowserFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else if self.focus_area == BrowserFocusArea::Options {
            self.option_checkboxes.is_empty()
                || self.focused_checkbox_index >= self.option_checkboxes.len().saturating_sub(1)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == BrowserFocusArea::Inputs && self.inputs.is_focused()
    }

    fn page_up(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.results_view.page_up(page_size);
    }

    fn page_down(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.results_view.page_down(page_size);
    }
}
