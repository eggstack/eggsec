use crate::components::{Checkbox, InputField, InputGroup};
use crate::tabs::core::{field_as, render_results_area, StandardFocusArea, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_boilerplate, tab_state_boilerplate, tc};
use eggsec::hunt::{HuntConfig, HuntReport};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct HuntTab {
    pub core: TabCore,
    pub report: Option<HuntReport>,
    pub config: HuntConfig,
    pub option_checkboxes: Vec<Checkbox>,
    pub focused_checkbox_index: usize,
    pub focus_area: StandardFocusArea,
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
            core: TabCore::new("Running vulnerability hunt...", "Results").with_inputs(inputs),
            report: None,
            config: HuntConfig::default(),
            option_checkboxes,
            focused_checkbox_index: 0,
            focus_area: StandardFocusArea::Inputs,
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn concurrency(&self) -> usize {
        field_as(&self.core, 1, 10)
    }

    pub fn timeout_ms(&self) -> u64 {
        field_as(&self.core, 2, 5000)
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

    pub fn get_results(&self) -> Option<&HuntReport> {
        self.report.as_ref()
    }

    pub fn set_report(&mut self, report: HuntReport) {
        self.report = Some(report.clone());
        self.core.state = AppState::Completed;
        self.core.results_view.clear();

        self.core.results_view.add_line(Line::from(Span::styled(
            format!("Vulnerability Hunt Complete: {}", report.target),
            Style::default().fg(tc!(success)),
        )));
        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(Span::styled(
            format!("Total findings: {}", report.total_findings),
            Style::default().fg(tc!(warning)),
        )));
        self.core.results_view.add_line(Line::from(""));

        if !report.attack_chains.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                format!("Attack Chains ({}):", report.attack_chains.len()),
                Style::default().fg(tc!(error)),
            )));
            for chain in &report.attack_chains {
                self.core.results_view.add_line(Line::from(format!(
                    "  [{}] {} - {} steps",
                    chain.severity,
                    chain.name,
                    chain.steps.len()
                )));
            }
            self.core.results_view.add_line(Line::from(""));
        }

        if !report.business_logic.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                format!("Business Logic Flaws ({}):", report.business_logic.len()),
                Style::default().fg(tc!(error)),
            )));
            for flaw in &report.business_logic {
                self.core.results_view.add_line(Line::from(format!(
                    "  [{}] {:?} - {}",
                    flaw.severity, flaw.flaw_type, flaw.description
                )));
            }
            self.core.results_view.add_line(Line::from(""));
        }

        if !report.race_conditions.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                format!("Race Conditions ({}):", report.race_conditions.len()),
                Style::default().fg(tc!(error)),
            )));
            for race in &report.race_conditions {
                self.core.results_view.add_line(Line::from(format!(
                    "  [{}] {:?} - {}",
                    race.severity, race.race_type, race.description
                )));
            }
            self.core.results_view.add_line(Line::from(""));
        }

        if !report.authz_bypasses.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                format!("AuthZ Bypasses ({}):", report.authz_bypasses.len()),
                Style::default().fg(tc!(error)),
            )));
            for bypass in &report.authz_bypasses {
                self.core.results_view.add_line(Line::from(format!(
                    "  [{}] {:?} - {}",
                    bypass.severity, bypass.bypass_type, bypass.description
                )));
            }
            self.core.results_view.add_line(Line::from(""));
        }

        if !report.session_issues.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                format!("Session Issues ({}):", report.session_issues.len()),
                Style::default().fg(tc!(warning)),
            )));
            for issue in &report.session_issues {
                self.core.results_view.add_line(Line::from(format!(
                    "  [{}] {:?} - {}",
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

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }
}

impl Default for HuntTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for HuntTab {
    tab_state_boilerplate!(HuntTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.focus_area = StandardFocusArea::Inputs;
        self.focused_checkbox_index = 0;
        for cb in &mut self.option_checkboxes {
            cb.checked = true;
        }
    }
}

impl TabRender for HuntTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            StandardFocusArea::Inputs => "Inputs",
            StandardFocusArea::Options => "Options",
            StandardFocusArea::Results => "Results",
        };
        Some(vec!["Hunt", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(14), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let is_config_focused =
            self.focus_area == StandardFocusArea::Inputs || self.focus_area == StandardFocusArea::Options;
        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(Style::default().fg(if is_config_focused {
                tc!(border_focused)
            } else {
                tc!(border)
            }));
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

        for (i, field) in self.core.inputs.fields.iter().enumerate() {
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
                self.focus_area == StandardFocusArea::Options && i == self.focused_checkbox_index;
            if let Some(area) = left.get(i) {
                checkbox.render(f, *area);
            }
        }

        for (i, cb) in self.option_checkboxes.iter().enumerate().skip(3) {
            let mut checkbox = cb.clone();
            checkbox.focused =
                self.focus_area == StandardFocusArea::Options && i == self.focused_checkbox_index;
            if let Some(area) = right.get(i.saturating_sub(3)) {
                checkbox.render(f, *area);
            }
        }

        render_results_area(
            f,
            results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Vulnerability Hunting",
            "Enter target and press Enter to start vulnerability hunting",
        );
    }
}

impl TabInput for HuntTab {
    tab_input_boilerplate!(
        HuntTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea::Inputs,
        Results: StandardFocusArea::Results
    );

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            StandardFocusArea::Inputs => {
                self.core.inputs.blur();
                self.focused_checkbox_index = 0;
                StandardFocusArea::Options
            }
            StandardFocusArea::Options => StandardFocusArea::Results,
            StandardFocusArea::Results => {
                self.core.inputs.focus(0);
                StandardFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            StandardFocusArea::Inputs => {
                self.core.inputs.blur();
                StandardFocusArea::Results
            }
            StandardFocusArea::Options => {
                self.core.inputs.focus(0);
                StandardFocusArea::Inputs
            }
            StandardFocusArea::Results => {
                self.focused_checkbox_index = 0;
                StandardFocusArea::Options
            }
        };
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        if self.focus_area == StandardFocusArea::Inputs {
            if self.core.inputs.is_focused() {
                self.core.inputs.blur();
                return;
            }
        }

        if self.focus_area == StandardFocusArea::Options {
            if let Some(checkbox) = self
                .option_checkboxes
                .get_mut(self.focused_checkbox_index)
            {
                checkbox.toggle();
            }
            return;
        }

        if self.focus_area == StandardFocusArea::Results {
            return;
        }

        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        self.core.inputs.blur();
        self.focused_checkbox_index = 0;
        self.focus_area = StandardFocusArea::Inputs;
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == StandardFocusArea::Options {
            if self.option_checkboxes.is_empty() {
                return;
            }
            if self.focused_checkbox_index == 0 {
                self.focused_checkbox_index = self.option_checkboxes.len().saturating_sub(1);
            } else {
                self.focused_checkbox_index = self.focused_checkbox_index.saturating_sub(1);
            }
        } else if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
            self.core.results_view.scroll_up(1);
        } else {
            self.core.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == StandardFocusArea::Options {
            if self.option_checkboxes.is_empty() {
                return;
            }
            if self.focused_checkbox_index >= self.option_checkboxes.len().saturating_sub(1) {
                self.focused_checkbox_index = 0;
            } else {
                self.focused_checkbox_index += 1;
            }
        } else if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
            self.core.results_view.scroll_down(1);
        } else {
            self.core.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.move_left()
        } else if self.focus_area == StandardFocusArea::Options {
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
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.move_right()
        } else if self.focus_area == StandardFocusArea::Options {
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
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.is_at_left_edge()
        } else if self.focus_area == StandardFocusArea::Options {
            self.option_checkboxes.is_empty() || self.focused_checkbox_index == 0
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.is_at_right_edge()
        } else if self.focus_area == StandardFocusArea::Options {
            self.option_checkboxes.is_empty()
                || self.focused_checkbox_index >= self.option_checkboxes.len().saturating_sub(1)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == StandardFocusArea::Inputs && self.core.inputs.is_focused()
    }
}
