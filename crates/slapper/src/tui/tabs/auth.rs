use crate::tui::components::{InputField, InputGroup};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthFocusArea {
    Target,
    Username,
    Password,
    Results,
}

pub struct AuthTab {
    pub inputs: InputGroup,
    pub results: String,
    pub state: AppState,
    pub focus_area: AuthFocusArea,
    pub error_message: Option<String>,
}

impl AuthTab {
    pub fn new() -> Self {
        Self {
            inputs: InputGroup::new()
                .add(InputField::new("Target URL").with_width(40))
                .add(InputField::new("Username").with_width(30))
                .add(InputField::new("Password List").with_width(40)),
            results: "Ready for authentication testing".to_string(),
            state: AppState::Idle,
            focus_area: AuthFocusArea::Target,
            error_message: None,
        }
    }

    pub fn start(&mut self) {
        self.state = AppState::Running;
        self.error_message = None;
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn reset(&mut self) {
        self.state = AppState::Idle;
        self.error_message = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg);
    }
}

impl TabState for AuthTab {
    fn state(&self) -> AppState {
        self.state
    }

    fn reset(&mut self) {
        self.reset();
    }
}

impl TabRender for AuthTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            AuthFocusArea::Target => "Target",
            AuthFocusArea::Username => "Username",
            AuthFocusArea::Password => "Password",
            AuthFocusArea::Results => "Results",
        };
        Some(vec!["Auth", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        use ratatui::style::Style;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        let title = Paragraph::new("Authentication Testing")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title, layout[0]);

        let mut input_text = String::new();
        for (i, field) in self.inputs.fields.iter().enumerate() {
            if i > 0 {
                input_text.push_str("\n");
            }
            let label = field.label.clone();
            let value = field.value.clone();
            let focus marker = if field.focused { "*" } else { " " };
            input_text.push_str(&format!("{}{}: {}", focus marker, label, value));
        }

        if let Some(ref err) = self.error_message {
            input_text.push_str(&format!("\nError: {}", err));
        }

        let input_display = Paragraph::new(input_text)
            .block(Block::default().borders(Borders::ALL).title("Inputs"))
            .style(Style::default().fg(Color::White));
        f.render_widget(input_display, layout[1]);

        let results = Paragraph::new(&self.results)
            .block(Block::default().borders(Borders::ALL).title("Results"))
            .style(Style::default().fg(Color::White));
        f.render_widget(results, layout[3]);
    }
}

impl TabInput for AuthTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            AuthFocusArea::Target => AuthFocusArea::Username,
            AuthFocusArea::Username => AuthFocusArea::Password,
            AuthFocusArea::Password => AuthFocusArea::Results,
            AuthFocusArea::Results => AuthFocusArea::Target,
        };
        self.sync_input_focus();
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            AuthFocusArea::Target => AuthFocusArea::Results,
            AuthFocusArea::Username => AuthFocusArea::Target,
            AuthFocusArea::Password => AuthFocusArea::Username,
            AuthFocusArea::Results => AuthFocusArea::Password,
        };
        self.sync_input_focus();
    }

    fn handle_char(&mut self, c: char) {
        if let Some(idx) = self.current_input_index() {
            self.inputs.fields[idx].insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if let Some(idx) = self.current_input_index() {
            self.inputs.fields[idx].backspace();
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {}

    fn handle_up(&mut self) {}

    fn handle_down(&mut self) {}

    fn handle_left(&mut self) -> bool {
        if let Some(idx) = self.current_input_index() {
            self.inputs.fields[idx].move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if let Some(idx) = self.current_input_index() {
            self.inputs.fields[idx].move_right()
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        matches!(self.focus_area, AuthFocusArea::Target | AuthFocusArea::Username | AuthFocusArea::Password)
    }
}

impl AuthTab {
    fn current_input_index(&self) -> Option<usize> {
        match self.focus_area {
            AuthFocusArea::Target => Some(0),
            AuthFocusArea::Username => Some(1),
            AuthFocusArea::Password => Some(2),
            AuthFocusArea::Results => None,
        }
    }

    fn sync_input_focus(&mut self) {
        for (i, field) in self.inputs.fields.iter_mut().enumerate() {
            field.focused = Some(i) == self.current_input_index();
        }
    }
}