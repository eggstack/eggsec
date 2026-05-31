use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, InputField, InputGroup, ProgressGauge, ScrollableText,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WafStressFocusArea {
    Inputs,
    Results,
}

pub struct WafStressTab {
    pub inputs: InputGroup,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: WafStressFocusArea,
    pub error: Option<TabError>,
}

impl WafStressTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Concurrency").with_value("20"))
            .add(InputField::new("Timeout (s)").with_value("10"));

        Self {
            inputs,
            progress: ProgressGauge::new("WAF Stress Testing..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: WafStressFocusArea::Inputs,
            error: None,
        }
    }

    pub fn get_results(&self) -> Option<String> {
        if self.results_view.is_empty() {
            None
        } else {
            Some(
                self.results_view
                    .lines
                    .iter()
                    .map(|l| l.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
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
            .unwrap_or(20)
    }

    pub fn timeout(&self) -> u64 {
        self.inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.progress.current = 0;
            self.progress.total = 0;
            self.error = None;
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

    pub fn scroll_results_up(&mut self) {
        self.results_view.scroll_up(1);
    }

    pub fn scroll_results_down(&mut self) {
        self.results_view.scroll_down(1);
    }

}

impl Default for WafStressTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for WafStressTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.progress.current = 0;
        self.progress.total = 0;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        if let Some(field) = self.inputs.fields.get_mut(1) {
            field.value = "20".to_string();
            field.cursor_pos = 2;
        }
        if let Some(field) = self.inputs.fields.get_mut(2) {
            field.value = "10".to_string();
            field.cursor_pos = 2;
        }
        self.focus_area = WafStressFocusArea::Inputs;
        self.inputs.blur();
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for WafStressTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(area);

        let input_area = match chunks.get(0) {
            Some(area) => *area,
            None => return,
        };
        let results_area = match chunks.get(1) {
            Some(area) => *area,
            None => return,
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" WAF Stress Configuration ")
            .border_style(
                Style::default().fg(if self.focus_area == WafStressFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let input_inner = input_block.inner(input_area);
        f.render_widget(input_block, input_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_inner);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(
                Style::default().fg(if self.focus_area == WafStressFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let results_inner = results_block.inner(results_area);
        f.render_widget(results_block, results_area);

        if self.state == AppState::Running {
            self.progress.render(f, results_inner);
        } else if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_inner);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_inner, Some(tc!(success)));
        } else {
            let placeholder =
                empty_state_paragraph("Results", "Results will appear here after running");
            f.render_widget(placeholder, results_inner);
        }
    }
}

impl TabInput for WafStressTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            WafStressFocusArea::Inputs => {
                self.inputs.blur();
                WafStressFocusArea::Results
            }
            WafStressFocusArea::Results => {
                self.inputs.focus(0);
                WafStressFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            WafStressFocusArea::Inputs => {
                self.inputs.blur();
                WafStressFocusArea::Results
            }
            WafStressFocusArea::Results => {
                self.inputs.focus(0);
                WafStressFocusArea::Inputs
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else if self.focus_area == WafStressFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if self.focus_area == WafStressFocusArea::Inputs {
                self.inputs.move_word_forward();
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if self.focus_area == WafStressFocusArea::Inputs {
                self.inputs.move_word_backward();
            }
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == WafStressFocusArea::Inputs {
                self.inputs.move_home();
            } else if self.focus_area == WafStressFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == WafStressFocusArea::Inputs {
                self.inputs.move_end();
            } else if self.focus_area == WafStressFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = WafStressFocusArea::Inputs;
            self.inputs.focus(0);
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.inputs.blur();
            self.focus_area = WafStressFocusArea::Results;
        }
    }

    fn handle_enter(&mut self) {
        if self.focus_area == WafStressFocusArea::Results {
            return;
        }

        if self.is_running() {
            self.stop();
            return;
        }
        if self.inputs.is_focused() {
            self.inputs.blur();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WafStressFocusArea::Results {
            self.scroll_results_up();
        } else if self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WafStressFocusArea::Results {
            self.scroll_results_down();
        } else if self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.move_left()
        } else {
            self.results_view.scroll_left(5);
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.move_right()
        } else {
            self.results_view.scroll_right(5);
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == WafStressFocusArea::Inputs && self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else {
            self.results_view.is_at_left_edge()
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == WafStressFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else {
            self.results_view.is_at_right_edge()
        }
    }

    fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
    }
}
