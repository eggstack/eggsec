use crate::hunt::{HuntConfig, HuntReport};
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

pub struct HuntTab {
    pub inputs: InputGroup,
    pub report: Option<HuntReport>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub config: HuntConfig,
    pub option_checkboxes: Vec<Checkbox>,
    pub focused_checkbox_index: usize,
    pub focus_area: HuntFocusArea,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HuntFocusArea {
    Inputs,
    Options,
    Results,
}

impl HuntTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Concurrency").with_value("10"))
            .add(InputField::new("Timeout (ms)").with_value("5000"));

        let option_checkboxes = vec![
            Checkbox::new("Attack Chains").checked(true),
            Checkbox::new("Business Logic").checked(true),
            Checkbox::new("Race Conditions").checked(true),
            Checkbox::new("Authorization Bypass").checked(true),
            Checkbox::new("Session Security").checked(true),
        ];

        Self {
            inputs,
            report: None,
            progress: ProgressGauge::new("Running vulnerability hunt..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            config: HuntConfig::default(),
            option_checkboxes,
            focused_checkbox_index: 0,
            focus_area: HuntFocusArea::Inputs,
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

    pub fn concurrency(&self) -> usize {
        self.inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn timeout_ms(&self) -> u64 {
        self.inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(5000)
    }

    pub fn get_config(&self) -> HuntConfig {
        HuntConfig {
            check_attack_chains: self
                .option_checkboxes
                .get(0)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            check_business_logic: self
                .option_checkboxes
                .get(1)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            check_race_conditions: self
                .option_checkboxes
                .get(2)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            check_authz_bypass: self
                .option_checkboxes
                .get(3)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            check_session: self
                .option_checkboxes
                .get(4)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            concurrency: self.concurrency(),
            timeout_ms: self.timeout_ms(),
        }
    }

    pub fn set_report(&mut self, report: HuntReport) {
        self.report = Some(report.clone());
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            format!("Vulnerability Hunt Complete: {}", report.target),
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Total findings: {}", report.total_findings),
            Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(""));

        if !report.attack_chains.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("Attack Chains ({}):", report.attack_chains.len()),
                Style::default().fg(tc!(error)),
            )));
            for chain in &report.attack_chains {
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {} - {} steps",
                    chain.severity,
                    chain.name,
                    chain.steps.len()
                )));
            }
            self.results_view.add_line(Line::from(""));
        }

        if !report.business_logic.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("Business Logic Flaws ({}):", report.business_logic.len()),
                Style::default().fg(tc!(error)),
            )));
            for flaw in &report.business_logic {
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {:?} - {}",
                    flaw.severity, flaw.flaw_type, flaw.description
                )));
            }
            self.results_view.add_line(Line::from(""));
        }

        if !report.race_conditions.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("Race Conditions ({}):", report.race_conditions.len()),
                Style::default().fg(tc!(error)),
            )));
            for race in &report.race_conditions {
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {:?} - {}",
                    race.severity, race.race_type, race.description
                )));
            }
            self.results_view.add_line(Line::from(""));
        }

        if !report.authz_bypasses.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("AuthZ Bypasses ({}):", report.authz_bypasses.len()),
                Style::default().fg(tc!(error)),
            )));
            for bypass in &report.authz_bypasses {
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {:?} - {}",
                    bypass.severity, bypass.bypass_type, bypass.description
                )));
            }
            self.results_view.add_line(Line::from(""));
        }

        if !report.session_issues.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("Session Issues ({}):", report.session_issues.len()),
                Style::default().fg(tc!(warning)),
            )));
            for issue in &report.session_issues {
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {:?} - {}",
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

impl Default for HuntTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for HuntTab {
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
        self.focus_area = HuntFocusArea::Inputs;
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

impl TabRender for HuntTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            HuntFocusArea::Inputs => "Inputs",
            HuntFocusArea::Options => "Options",
            HuntFocusArea::Results => "Results",
        };
        Some(vec!["Hunt", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(
                Style::default().fg(
                    if self.focus_area == HuntFocusArea::Inputs
                        || self.focus_area == HuntFocusArea::Options
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
                Constraint::Length(3),
                Constraint::Length(3),
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
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(*cb_area);

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(2); 3])
            .split(cb_chunks[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(2); 2])
            .split(cb_chunks[1]);

        for (i, cb) in self.option_checkboxes.iter().enumerate().take(3) {
            let mut checkbox = cb.clone();
            checkbox.focused =
                self.focus_area == HuntFocusArea::Options && i == self.focused_checkbox_index;
            if let Some(area) = left.get(i) {
                checkbox.render(f, *area);
            }
        }

        for (i, cb) in self.option_checkboxes.iter().enumerate().skip(3) {
            let mut checkbox = cb.clone();
            checkbox.focused =
                self.focus_area == HuntFocusArea::Options && i == self.focused_checkbox_index;
            if let Some(area) = right.get(i.saturating_sub(3)) {
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
                        .title("Vulnerability Hunt - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, None);
        } else {
            let placeholder = empty_state_paragraph(
                "Vulnerability Hunting",
                "Enter target and press Enter to start vulnerability hunting",
            );
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for HuntTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            HuntFocusArea::Inputs => {
                self.inputs.blur();
                self.focused_checkbox_index = 0;
                HuntFocusArea::Options
            }
            HuntFocusArea::Options => HuntFocusArea::Results,
            HuntFocusArea::Results => {
                self.inputs.focus(0);
                HuntFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            HuntFocusArea::Inputs => {
                self.inputs.blur();
                HuntFocusArea::Results
            }
            HuntFocusArea::Options => {
                self.inputs.focus(0);
                HuntFocusArea::Inputs
            }
            HuntFocusArea::Results => {
                self.focused_checkbox_index = 0;
                HuntFocusArea::Options
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == HuntFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == HuntFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == HuntFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == HuntFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else if self.focus_area == HuntFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() && self.focus_area == HuntFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() && self.focus_area == HuntFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == HuntFocusArea::Inputs {
                self.inputs.move_home();
            } else if self.focus_area == HuntFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == HuntFocusArea::Inputs {
                self.inputs.move_end();
            } else if self.focus_area == HuntFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = HuntFocusArea::Inputs;
            self.inputs.focus(0);
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = HuntFocusArea::Results;
            self.inputs.blur();
            self.focused_checkbox_index = 0;
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.focus_area == HuntFocusArea::Inputs {
            if self.inputs.is_focused() {
                self.inputs.blur();
                return;
            }
        }

        if self.focus_area == HuntFocusArea::Options {
            if let Some(checkbox) = self.option_checkboxes.get_mut(self.focused_checkbox_index) {
                checkbox.toggle();
            }
            return;
        }

        if self.focus_area == HuntFocusArea::Results {
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
        if self.focus_area == HuntFocusArea::Options {
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
        if self.focus_area == HuntFocusArea::Options {
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
        if self.focus_area == HuntFocusArea::Inputs {
            self.inputs.move_left()
        } else if self.focus_area == HuntFocusArea::Options {
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
        if self.focus_area == HuntFocusArea::Inputs {
            self.inputs.move_right()
        } else if self.focus_area == HuntFocusArea::Options {
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
        if self.focus_area == HuntFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else if self.focus_area == HuntFocusArea::Options {
            self.option_checkboxes.is_empty() || self.focused_checkbox_index == 0
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == HuntFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else if self.focus_area == HuntFocusArea::Options {
            self.option_checkboxes.is_empty()
                || self.focused_checkbox_index >= self.option_checkboxes.len().saturating_sub(1)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == HuntFocusArea::Inputs && self.inputs.is_focused()
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
