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
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct GraphQlTab {
    pub inputs: InputGroup,
    pub introspection_checkbox: Checkbox,
    pub inject_checkbox: Checkbox,
    pub depth_bypass_checkbox: Checkbox,
    pub alias_overload_checkbox: Checkbox,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: GraphQlFocusArea,
    pub checkbox_focus_index: usize,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphQlFocusArea {
    Inputs,
    Options,
    Results,
}

impl Default for GraphQlTab {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphQlTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("GraphQL Endpoint URL"))
            .add(InputField::new("Concurrency").with_value("10"))
            .add(InputField::new("Timeout (s)").with_value("15"));

        let introspection_checkbox = Checkbox::new("Introspection Tests").checked(true);
        let inject_checkbox = Checkbox::new("Query Injection Tests").checked(true);
        let depth_bypass_checkbox = Checkbox::new("Depth Limit Bypass").checked(true);
        let alias_overload_checkbox = Checkbox::new("Alias Overload Tests").checked(true);

        Self {
            inputs,
            introspection_checkbox,
            inject_checkbox,
            depth_bypass_checkbox,
            alias_overload_checkbox,
            progress: ProgressGauge::new("Testing GraphQL..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: GraphQlFocusArea::Inputs,
            checkbox_focus_index: 0,
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

    pub fn timeout(&self) -> u64 {
        self.inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(15)
    }

    pub fn set_results(&mut self, results: GraphQlResults) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            format!("GraphQL Security Test Complete: {}", results.target),
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Findings:",
            Style::default().fg(tc!(warning)),
        )));

        if results.introspection_enabled {
            self.results_view.add_line(Line::from(Span::styled(
                "  [!] Introspection is ENABLED - Schema exposed",
                Style::default().fg(tc!(error)),
            )));
        } else {
            self.results_view
                .add_line(Line::from(Span::raw("  [+] Introspection is disabled")));
        }

        if results.depth_limit_bypassed {
            self.results_view.add_line(Line::from(Span::styled(
                "  [!] Depth limit bypass detected",
                Style::default().fg(tc!(error)),
            )));
        }

        if results.alias_overload_vulnerable {
            self.results_view.add_line(Line::from(Span::styled(
                "  [!] Alias overload vulnerability detected",
                Style::default().fg(tc!(error)),
            )));
        }

        if !results.injection_findings.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("  Injection Findings: {}", results.injection_findings.len()),
                Style::default().fg(tc!(warning)),
            )));
        }

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "Requests: {} | Errors: {} | Duration: {}ms",
            results.total_requests, results.errors, results.duration_ms
        )));
    }
}

#[derive(Clone, Debug)]
pub struct GraphQlResults {
    pub target: String,
    pub introspection_enabled: bool,
    pub depth_limit_bypassed: bool,
    pub alias_overload_vulnerable: bool,
    pub injection_findings: Vec<String>,
    pub total_requests: usize,
    pub errors: usize,
    pub duration_ms: u64,
}

impl TabState for GraphQlTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results_view.clear();
        self.progress.current = 0;
        self.progress.total = 100;
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        if self.inputs.fields.len() > 1 {
            self.inputs.fields[1].value = "10".to_string();
        }
        if self.inputs.fields.len() > 2 {
            self.inputs.fields[2].value = "15".to_string();
        }
        self.inputs.blur();
        self.focus_area = GraphQlFocusArea::Inputs;
        self.checkbox_focus_index = 0;
        self.inject_checkbox.checked = true;
        self.introspection_checkbox.checked = true;
        self.depth_bypass_checkbox.checked = true;
        self.alias_overload_checkbox.checked = true;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", error.message()),
            Style::default().fg(tc!(error)),
        )));
    }
}

impl TabRender for GraphQlTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref error) = self.error {
            let msg = error.message();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("GraphQL - Error")
                .border_style(Style::default().fg(tc!(error)));
            let paragraph = Paragraph::new(msg)
                .style(Style::default().fg(tc!(error)))
                .block(block);
            f.render_widget(paragraph, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Length(6),
                Constraint::Min(5),
            ])
            .split(area);

        // Input fields
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(chunks.first().copied().unwrap_or(area));

        let input_block = Block::default()
            .title(" GraphQL Configuration ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == GraphQlFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        f.render_widget(input_block, chunks.first().copied().unwrap_or(area));

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        // Options
        let options_block = Block::default()
            .title(" Test Options ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == GraphQlFocusArea::Options {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );

        let options_area = chunks.get(1).copied().unwrap_or(area);
        let options_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(options_block.inner(options_area));

        f.render_widget(options_block, options_area);
        if let (Some(c0), Some(c1), Some(c2), Some(c3)) =
            (options_chunks.get(0), options_chunks.get(1), options_chunks.get(2), options_chunks.get(3))
        {
            self.introspection_checkbox.render(f, *c0);
            self.inject_checkbox.render(f, *c1);
            self.depth_bypass_checkbox.render(f, *c2);
            self.alias_overload_checkbox.render(f, *c3);
        }

        // Results
        let results_area = chunks.get(2).copied().unwrap_or(area);
        if self.results_view.is_empty() {
            let placeholder =
                empty_state_paragraph("Results", "Results will appear here after running");
            f.render_widget(placeholder, results_area);
        } else {
            self.results_view.render(f, results_area, None);
        }

        // Progress bar if running
        if self.state == AppState::Running {
            let progress_area = Rect {
                x: area.x,
                y: area.y + area.height - 1,
                width: area.width,
                height: 1,
            };
            self.progress.render(f, progress_area);
        }
    }
}

impl TabInput for GraphQlTab {
    fn stop(&mut self) {
        if self.state == AppState::Running {
            self.state = AppState::Idle;
        }
    }

    fn handle_focus_next(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                GraphQlFocusArea::Inputs => {
                    self.inputs.blur();
                    GraphQlFocusArea::Options
                }
                GraphQlFocusArea::Options => GraphQlFocusArea::Results,
                GraphQlFocusArea::Results => {
                    self.inputs.focus(0);
                    GraphQlFocusArea::Inputs
                }
            };
        }
    }

    fn handle_focus_prev(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                GraphQlFocusArea::Inputs => {
                    self.inputs.blur();
                    GraphQlFocusArea::Results
                }
                GraphQlFocusArea::Options => {
                    self.inputs.focus(0);
                    GraphQlFocusArea::Inputs
                }
                GraphQlFocusArea::Results => GraphQlFocusArea::Options,
            };
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else if self.focus_area == GraphQlFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.move_home();
        } else if self.focus_area == GraphQlFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.move_end();
        } else if self.focus_area == GraphQlFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = GraphQlFocusArea::Inputs;
        self.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = GraphQlFocusArea::Results;
        self.inputs.blur();
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            GraphQlFocusArea::Inputs => {
                self.inputs.blur();
            }
            GraphQlFocusArea::Options => {
                let checkboxes = [
                    &mut self.introspection_checkbox,
                    &mut self.inject_checkbox,
                    &mut self.depth_bypass_checkbox,
                    &mut self.alias_overload_checkbox,
                ];
                let idx = self.checkbox_focus_index % checkboxes.len();
                checkboxes[idx].toggle();
            }
            GraphQlFocusArea::Results => {}
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.inputs.blur();
        self.focus_area = GraphQlFocusArea::Inputs;
        self.checkbox_focus_index = 0;
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            GraphQlFocusArea::Inputs => {
                self.inputs.focus_prev();
            }
            GraphQlFocusArea::Results => {
                self.results_view.scroll_up(1);
            }
            _ => {}
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            GraphQlFocusArea::Inputs => {
                self.inputs.focus_next();
            }
            GraphQlFocusArea::Results => {
                self.results_view.scroll_down(1);
            }
            _ => {}
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        match self.focus_area {
            GraphQlFocusArea::Inputs => self.inputs.move_left(),
            GraphQlFocusArea::Options => {
                if self.checkbox_focus_index > 0 {
                    self.checkbox_focus_index -= 1;
                }
                true
            }
            _ => false,
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        match self.focus_area {
            GraphQlFocusArea::Inputs => self.inputs.move_right(),
            GraphQlFocusArea::Options => {
                let max_idx = 3;
                if self.checkbox_focus_index < max_idx {
                    self.checkbox_focus_index += 1;
                }
                true
            }
            _ => false,
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == GraphQlFocusArea::Inputs && self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            GraphQlFocusArea::Inputs => !self.inputs.can_move_left(),
            GraphQlFocusArea::Options => self.checkbox_focus_index == 0,
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            GraphQlFocusArea::Inputs => !self.inputs.can_move_right(),
            GraphQlFocusArea::Options => self.checkbox_focus_index >= 3,
            _ => true,
        }
    }
}
