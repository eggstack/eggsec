use crate::hunt::{HuntConfig, HuntReport};
use crate::tc;
use crate::tui::components::{Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
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
    pub focus_area: HuntFocusArea,
    pub error_message: Option<String>,
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
            focus_area: HuntFocusArea::Inputs,
            error_message: None,
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
            check_attack_chains: self.option_checkboxes[0].checked,
            check_business_logic: self.option_checkboxes[1].checked,
            check_race_conditions: self.option_checkboxes[2].checked,
            check_authz_bypass: self.option_checkboxes[3].checked,
            check_session: self.option_checkboxes[4].checked,
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
            ratatui::style::Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Total findings: {}", report.total_findings),
            ratatui::style::Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(""));

        if !report.attack_chains.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("Attack Chains ({}):", report.attack_chains.len()),
                ratatui::style::Style::default().fg(tc!(error)),
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
                ratatui::style::Style::default().fg(tc!(error)),
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
                ratatui::style::Style::default().fg(tc!(error)),
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
                ratatui::style::Style::default().fg(tc!(error)),
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
                ratatui::style::Style::default().fg(tc!(warning)),
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

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
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
        self.results_view.clear();
        self.error_message = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg);
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

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_area);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            field.render(f, input_chunks[i], insert_mode);
        }

        let cb_area = input_chunks[3];
        let cb_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(cb_area);

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
            checkbox.focused = self.focus_area == HuntFocusArea::Options;
            checkbox.render(f, left[i]);
        }

        for (i, cb) in self.option_checkboxes.iter().enumerate().skip(3) {
            let mut checkbox = cb.clone();
            checkbox.focused = self.focus_area == HuntFocusArea::Options;
            checkbox.render(f, right[i - 3]);
        }

        if self.state == AppState::Running {
            self.progress.render(f, results_area);
        } else if let Some(ref err_msg) = self.error_message {
            use ratatui::style::Style;
            use ratatui::widgets::{Block, Borders, Paragraph};
            let error_text = Paragraph::new(format!("Error: {}", err_msg))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Vulnerability Hunt - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, Some(tc!(success)));
        } else {
            use ratatui::widgets::{Block, Borders, Paragraph};
            let placeholder =
                Paragraph::new("Enter target and press Enter to start vulnerability hunting")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Vulnerability Hunting"),
                    )
                    .style(Style::default().fg(tc!(text_dim)));
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for HuntTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            HuntFocusArea::Inputs => {
                self.inputs.blur();
                self.option_checkboxes
                    .iter_mut()
                    .for_each(|cb| cb.focused = false);
                self.option_checkboxes[0].focused = true;
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
        self.focus_area = match self.focus_area {
            HuntFocusArea::Inputs => HuntFocusArea::Results,
            HuntFocusArea::Options => {
                self.inputs.focus(0);
                HuntFocusArea::Inputs
            }
            HuntFocusArea::Results => {
                self.option_checkboxes
                    .iter_mut()
                    .for_each(|cb| cb.focused = false);
                self.option_checkboxes[0].focused = true;
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

    fn handle_enter(&mut self) {
        if self.focus_area == HuntFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }

        if self.focus_area == HuntFocusArea::Options {
            for cb in &mut self.option_checkboxes {
                if cb.focused {
                    cb.toggle();
                    break;
                }
            }
            return;
        }

        if self.is_running() {
            self.stop();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.focus_area == HuntFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx == 0 {
                    if let Some(last) = self.option_checkboxes.last_mut() {
                        last.focused = true;
                    }
                } else {
                    self.option_checkboxes[idx - 1].focused = true;
                }
                self.option_checkboxes[idx].focused = false;
            } else if let Some(first) = self.option_checkboxes.first_mut() {
                first.focused = true;
            }
        } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
            self.results_view.scroll_up(1);
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.focus_area == HuntFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx == self.option_checkboxes.len() - 1 {
                    self.option_checkboxes[0].focused = true;
                } else {
                    self.option_checkboxes[idx + 1].focused = true;
                }
                self.option_checkboxes[idx].focused = false;
            } else {
                self.option_checkboxes[0].focused = true;
            }
        } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
            self.results_view.scroll_down(1);
        } else {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.focus_area == HuntFocusArea::Inputs {
            self.inputs.move_left()
        } else if self.focus_area == HuntFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx == 0 {
                    return false;
                } else {
                    self.option_checkboxes[idx].focused = false;
                    self.option_checkboxes[idx - 1].focused = true;
                    return true;
                }
            }
            true
        } else {
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.focus_area == HuntFocusArea::Inputs {
            self.inputs.move_right()
        } else if self.focus_area == HuntFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx >= self.option_checkboxes.len() - 1 {
                    return false;
                } else {
                    self.option_checkboxes[idx].focused = false;
                    self.option_checkboxes[idx + 1].focused = true;
                    return true;
                }
            }
            true
        } else {
            true
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == HuntFocusArea::Inputs {
            self.inputs.fields[0].cursor_pos == 0
        } else if self.focus_area == HuntFocusArea::Options {
            self.option_checkboxes.iter().position(|cb| cb.focused) == Some(0)
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == HuntFocusArea::Inputs {
            let field = &self.inputs.fields[0];
            field.cursor_pos >= field.value.chars().count()
        } else if self.focus_area == HuntFocusArea::Options {
            self.option_checkboxes.iter().position(|cb| cb.focused)
                == Some(self.option_checkboxes.len() - 1)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == HuntFocusArea::Inputs && self.inputs.is_focused()
    }
}
