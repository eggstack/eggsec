use crate::tui::components::{InputField, InputGroup, ScrollableText};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct ResumeTab {
    pub inputs: InputGroup,
    pub state: AppState,
    pub results_view: ScrollableText,
}

impl ResumeTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new().add(InputField::new("Session File Path"));

        Self {
            inputs,
            state: AppState::Idle,
            results_view: ScrollableText::new("Session Info"),
        }
    }

    pub fn session_file(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn is_running(&self) -> bool {
        self.state == AppState::Running
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

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
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
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }

    fn set_error(&mut self, msg: String) {
        use ratatui::style::Style;
        use ratatui::text::Span;
        self.state = AppState::Error(msg.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", msg),
            Style::default().fg(Color::Red),
        )));
    }
}

impl TabRender for ResumeTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3)])
            .split(input_area);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            field.render(f, input_chunks[i], insert_mode);
        }

        if !self.results_view.is_empty() {
            self.results_view
                .render_with_style(f, results_area, Color::Cyan);
        } else {
            let placeholder = Paragraph::new(
                "Enter session file path and press Enter to resume a previous scan.\n\n\
                 Examples:\n\
                   slapper resume session.json\n\
                   slapper resume /path/to/session.json",
            )
            .block(Block::default().borders(Borders::ALL).title("Session Info"))
            .style(Color::DarkGray);
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for ResumeTab {
    fn handle_focus_next(&mut self) {
        self.inputs.focus_next();
    }

    fn handle_focus_prev(&mut self) {
        self.inputs.focus_prev();
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            self.inputs.backspace();
        }
    }

    fn handle_enter(&mut self) {
        if self.inputs.is_focused() {
            self.inputs.blur();
        } else if self.is_running() {
            self.stop();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if !self.inputs.is_focused() && !self.results_view.is_empty() {
            self.scroll_results_up();
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if !self.inputs.is_focused() && !self.results_view.is_empty() {
            self.scroll_results_down();
        } else {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        self.inputs.move_left()
    }

    fn handle_right(&mut self) -> bool {
        self.inputs.move_right()
    }

    fn is_input_focused(&self) -> bool {
        self.inputs.is_focused()
    }
}
