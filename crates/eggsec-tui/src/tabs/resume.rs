use crate::app::tab_error::TabError;
use crate::components::{empty_state_paragraph, InputField, InputGroup, ScrollableText};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::tc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResumeFocusArea {
    Inputs,
    Results,
}

pub struct ResumeTab {
    pub inputs: InputGroup,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: ResumeFocusArea,
    pub error: Option<TabError>,
}

impl ResumeTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new().add(InputField::new("Session File Path"));

        Self {
            inputs,
            state: AppState::Idle,
            results_view: ScrollableText::new("Session Info"),
            focus_area: ResumeFocusArea::Inputs,
            error: None,
        }
    }

    pub fn session_file(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn start(&mut self) {
        if !self.session_file().is_empty() {
            self.state = AppState::Running;
            self.results_view.clear();
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn update_progress(&mut self, _completed: u64, _total: u64) {}

    pub fn scroll_results_up(&mut self) {
        self.results_view.scroll_up(1);
    }

    pub fn scroll_results_down(&mut self) {
        self.results_view.scroll_down(1);
    }
}

impl Default for ResumeTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ResumeTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.focus_area = ResumeFocusArea::Inputs;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error.clone());
        self.results_view.clear();
        use ratatui::style::Style;
        use ratatui::text::Span;
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", error.message()),
            Style::default().fg(tc!(error)),
        )));
    }
}

impl TabRender for ResumeTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(0)])
            .split(area);

        let input_area = chunks.first().copied().unwrap_or(area);
        let results_area = chunks.get(1).copied().unwrap_or(area);

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Resume Session ")
            .border_style(
                Style::default().fg(if self.focus_area == ResumeFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let input_inner = input_block.inner(input_area);
        f.render_widget(input_block, input_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3)])
            .split(input_inner);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(" Session Info ")
            .border_style(
                Style::default().fg(if self.focus_area == ResumeFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let results_inner = results_block.inner(results_area);
        f.render_widget(results_block, results_area);

        if !self.results_view.is_empty() {
            self.results_view.render(f, results_inner, None);
        } else {
            let placeholder = empty_state_paragraph(
                "Session Info",
                "Enter session file path and press Enter to resume a previous scan.\n\n\
                 Examples:\n\
                    eggsec resume session.json\n\
                    eggsec resume /path/to/session.json",
            );
            f.render_widget(placeholder, results_inner);
        }
    }
}

impl TabInput for ResumeTab {
    fn stop(&mut self) {
        ResumeTab::stop(self);
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ResumeFocusArea::Inputs => {
                self.inputs.blur();
                ResumeFocusArea::Results
            }
            ResumeFocusArea::Results => {
                self.inputs.focus(0);
                ResumeFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ResumeFocusArea::Results => {
                self.inputs.focus(0);
                ResumeFocusArea::Inputs
            }
            ResumeFocusArea::Inputs => {
                self.inputs.blur();
                ResumeFocusArea::Results
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else if self.focus_area == ResumeFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.move_home();
        } else if self.focus_area == ResumeFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.move_end();
        } else if self.focus_area == ResumeFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = ResumeFocusArea::Inputs;
        self.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.inputs.blur();
        self.focus_area = ResumeFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        if self.focus_area == ResumeFocusArea::Results {
            return;
        }

        if self.inputs.is_focused() {
            self.inputs.blur();
        }
        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.inputs.blur();
        self.focus_area = ResumeFocusArea::Inputs;
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ResumeFocusArea::Inputs {
            if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_up();
            } else {
                self.inputs.focus_prev();
            }
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ResumeFocusArea::Inputs {
            if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_down();
            } else {
                self.inputs.focus_next();
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        self.inputs.move_left()
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        self.inputs.move_right()
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ResumeFocusArea::Inputs
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ResumeFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else {
            true
        }
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
