use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct AuthTab {
    pub target_url: String,
    pub username: String,
    pub password_list: String,
    pub mode: String,
    pub results: String,
    pub state: AppState,
}

impl AuthTab {
    pub fn new() -> Self {
        Self {
            target_url: String::new(),
            username: String::new(),
            password_list: String::new(),
            mode: "brute-force".to_string(),
            results: "Ready for authentication testing".to_string(),
            state: AppState::Idle,
        }
    }

    pub fn start(&mut self) {
        self.state = AppState::Running;
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn reset(&mut self) {
        self.state = AppState::Idle;
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
    fn render(&self, f: &mut Frame, area: Rect, _insert_mode: bool) {
        use ratatui::style::Style;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let title = Paragraph::new("Authentication Testing")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title, layout[0]);

        let results = Paragraph::new(&self.results)
            .block(Block::default().borders(Borders::ALL).title("Results"))
            .style(Style::default().fg(Color::White));
        f.render_widget(results, layout[1]);
    }
}

impl TabInput for AuthTab {
    fn handle_focus_next(&mut self) {}

    fn handle_focus_prev(&mut self) {}

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            self.target_url.push(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            self.target_url.pop();
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
        false
    }

    fn handle_right(&mut self) -> bool {
        false
    }

    fn is_input_focused(&self) -> bool {
        false
    }
}