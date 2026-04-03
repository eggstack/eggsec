use crate::tui::components::{Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphQlFocusArea {
    Inputs,
    Options,
    Results,
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
            Style::default().fg(Color::Green),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Findings:",
            Style::default().fg(Color::Yellow),
        )));

        if results.introspection_enabled {
            self.results_view.add_line(Line::from(Span::styled(
                "  [!] Introspection is ENABLED - Schema exposed",
                Style::default().fg(Color::Red),
            )));
        } else {
            self.results_view
                .add_line(Line::from(Span::raw("  [+] Introspection is disabled")));
        }

        if results.depth_limit_bypassed {
            self.results_view.add_line(Line::from(Span::styled(
                "  [!] Depth limit bypass detected",
                Style::default().fg(Color::Red),
            )));
        }

        if results.alias_overload_vulnerable {
            self.results_view.add_line(Line::from(Span::styled(
                "  [!] Alias overload vulnerability detected",
                Style::default().fg(Color::Red),
            )));
        }

        if !results.injection_findings.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!("  Injection Findings: {}", results.injection_findings.len()),
                Style::default().fg(Color::Yellow),
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
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", msg),
            Style::default().fg(Color::Red),
        )));
    }
}

impl GraphQlTab {
    pub fn stop(&mut self) {
        if self.state == AppState::Running {
            self.state = AppState::Idle;
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.scroll_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.scroll_down(page_size);
    }

    pub fn handle_word_forward(&mut self) {
        for _ in 0..5 {
            self.handle_right();
        }
    }

    pub fn handle_word_backward(&mut self) {
        for _ in 0..5 {
            self.handle_left();
        }
    }

    pub fn handle_home(&mut self) {
        for _ in 0..100 {
            self.handle_left();
        }
    }

    pub fn handle_end(&mut self) {
        for _ in 0..100 {
            self.handle_right();
        }
    }

    pub fn handle_top(&mut self) {
        for _ in 0..100 {
            self.results_view.scroll_up(1);
        }
    }

    pub fn handle_bottom(&mut self) {
        for _ in 0..100 {
            self.results_view.scroll_down(1);
        }
    }
}

impl TabRender for GraphQlTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
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
            .split(chunks[0]);

        let input_block = Block::default()
            .title(" GraphQL Configuration ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == GraphQlFocusArea::Inputs {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            );
        f.render_widget(input_block, chunks[0]);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if i < input_chunks.len() {
                field.render(f, input_chunks[i], insert_mode);
            }
        }

        // Options
        let options_block = Block::default()
            .title(" Test Options ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == GraphQlFocusArea::Options {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            );

        let options_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(options_block.inner(chunks[1]));

        f.render_widget(options_block, chunks[1]);
        self.introspection_checkbox.render(f, options_chunks[0]);
        self.inject_checkbox.render(f, options_chunks[1]);
        self.depth_bypass_checkbox.render(f, options_chunks[2]);
        self.alias_overload_checkbox.render(f, options_chunks[3]);

        // Results
        self.results_view.render(f, chunks[2]);

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
    fn handle_focus_next(&mut self) {
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

    fn handle_focus_prev(&mut self) {
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

    fn handle_char(&mut self, c: char) {
        if self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.focus_area == GraphQlFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_enter(&mut self) {
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
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
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
        match self.focus_area {
            GraphQlFocusArea::Inputs => self.inputs.move_right(),
            GraphQlFocusArea::Options => {
                self.checkbox_focus_index = (self.checkbox_focus_index + 1).min(3);
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
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            GraphQlFocusArea::Inputs => !self.inputs.can_move_right(),
            _ => true,
        }
    }
}
