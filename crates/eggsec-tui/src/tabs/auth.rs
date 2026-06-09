use crate::tc;
use crate::app::tab_error::TabError;
use crate::components::{InputField, InputGroup};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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
    pub error: Option<TabError>,
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
            error: None,
        }
    }

    pub fn start(&mut self) {
        self.state = AppState::Running;
        self.error = None;
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn reset(&mut self) {
        self.state = AppState::Idle;
        self.error = None;
        self.focus_area = AuthFocusArea::Target;
        self.results = "Ready for authentication testing".to_string();
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }

    fn set_error_state(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabState for AuthTab {
    fn state(&self) -> AppState {
        self.state
    }

    fn reset(&mut self) {
        AuthTab::reset(self);
    }

    fn set_error(&mut self, error: TabError) {
        AuthTab::set_error_state(self, error);
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
        use crate::components::FormBuilder;

        if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(Block::default().borders(Borders::ALL).title("Auth - Error"))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(11), // 3 inputs * 3 + 2 borders
                Constraint::Min(0),
            ])
            .split(area);

        let Some(title_area) = layout.get(0) else { return; };
        let Some(inputs_area) = layout.get(1) else { return; };
        let Some(results_area) = layout.get(2) else { return; };

        let title = Paragraph::new("Authentication Testing")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(tc!(info)));
        f.render_widget(title, *title_area);

        let mut builder = FormBuilder::new("Inputs").row_height(3);
        for field in &self.inputs.fields {
            builder = builder.add_input(field.clone());
        }
        builder.render(f, *inputs_area, insert_mode);

        let results = if self.results.is_empty() {
            crate::components::empty_state_paragraph("Results", "No results yet")
        } else {
            Paragraph::new(self.results.as_str())
                .block(Block::default().borders(Borders::ALL).title("Results"))
                .style(Style::default().fg(tc!(text)))
        };
        f.render_widget(results, *results_area);
    }
}

impl TabInput for AuthTab {
    fn stop(&mut self) {
        AuthTab::stop(self);
    }

    fn handle_focus_next(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                AuthFocusArea::Target => AuthFocusArea::Username,
                AuthFocusArea::Username => AuthFocusArea::Password,
                AuthFocusArea::Password => AuthFocusArea::Results,
                AuthFocusArea::Results => AuthFocusArea::Target,
            };
            self.sync_input_focus();
        }
    }

    fn handle_focus_prev(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                AuthFocusArea::Target => AuthFocusArea::Results,
                AuthFocusArea::Username => AuthFocusArea::Target,
                AuthFocusArea::Password => AuthFocusArea::Username,
                AuthFocusArea::Results => AuthFocusArea::Password,
            };
            self.sync_input_focus();
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.insert(c);
                }
            }
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.backspace();
                }
            }
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.paste(text);
                }
            }
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.move_word_forward();
                }
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.move_word_backward();
                }
            }
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.move_home();
                }
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if let Some(idx) = self.current_input_index() {
                if let Some(field) = self.inputs.fields.get_mut(idx) {
                    field.move_end();
                }
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = AuthFocusArea::Target;
            self.sync_input_focus();
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = AuthFocusArea::Results;
            self.sync_input_focus();
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        if self.focus_area == AuthFocusArea::Results {
            return;
        }

        if self.target().is_empty() {
            return;
        }

        if self.is_input_focused() {
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
        self.focus_area = AuthFocusArea::Results;
        self.sync_input_focus();
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            if self.focus_area == AuthFocusArea::Results {
                self.focus_area = AuthFocusArea::Password;
                if self.inputs.fields.len() > 2 {
                    self.inputs.focus(2);
                }
            } else if self.focus_area == AuthFocusArea::Password {
                self.focus_area = AuthFocusArea::Username;
                if self.inputs.fields.len() > 1 {
                    self.inputs.focus(1);
                }
            } else if self.focus_area == AuthFocusArea::Username {
                self.focus_area = AuthFocusArea::Target;
                if !self.inputs.fields.is_empty() {
                    self.inputs.focus(0);
                }
            } else if self.focus_area == AuthFocusArea::Target {
                self.inputs.focus_prev();
                if !self.inputs.is_focused() {
                    if self.inputs.fields.is_empty() {
                        return;
                    }
                    self.inputs.focus(self.inputs.fields.len() - 1);
                }
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            if self.focus_area == AuthFocusArea::Target {
                self.focus_area = AuthFocusArea::Username;
                if self.inputs.fields.len() > 1 {
                    self.inputs.focus(1);
                }
            } else if self.focus_area == AuthFocusArea::Username {
                self.focus_area = AuthFocusArea::Password;
                if self.inputs.fields.len() > 2 {
                    self.inputs.focus(2);
                }
            } else if self.focus_area == AuthFocusArea::Password {
                self.focus_area = AuthFocusArea::Results;
                self.inputs.blur();
            } else if self.focus_area == AuthFocusArea::Results {
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() {
            self.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() {
            self.inputs.move_right()
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.is_input_focused() {
            self.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.is_input_focused() {
            self.inputs.is_at_right_edge()
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        matches!(self.focus_area, AuthFocusArea::Target | AuthFocusArea::Username | AuthFocusArea::Password)
    }
}

impl AuthTab {
    fn current_input_index(&self) -> Option<usize> {
        match self.focus_area {
            AuthFocusArea::Target if self.inputs.fields.len() > 0 => Some(0),
            AuthFocusArea::Username if self.inputs.fields.len() > 1 => Some(1),
            AuthFocusArea::Password if self.inputs.fields.len() > 2 => Some(2),
            AuthFocusArea::Target | AuthFocusArea::Username | AuthFocusArea::Password => None,
            AuthFocusArea::Results => None,
        }
    }

    fn sync_input_focus(&mut self) {
        for (i, field) in self.inputs.fields.iter_mut().enumerate() {
            field.focused = Some(i) == self.current_input_index();
        }
    }
}