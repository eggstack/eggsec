#[cfg(feature = "rest-api")]
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};

#[cfg(feature = "rest-api")]
pub struct McpTab {
    pub port: String,
    pub auth_token: String,
    pub clients: String,
    pub state: AppState,
}

#[cfg(feature = "rest-api")]
impl McpTab {
    pub fn new() -> Self {
        Self {
            port: "9090".to_string(),
            auth_token: String::new(),
            clients: "No connected clients".to_string(),
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

#[cfg(feature = "rest-api")]
impl TabState for McpTab {
    fn state(&self) -> AppState {
        self.state
    }

    fn reset(&mut self) {
        self.reset();
    }
}

#[cfg(feature = "rest-api")]
impl TabRender for McpTab {
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, _insert_mode: bool) {
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::Color,
            style::Style,
            widgets::{Block, Borders, Paragraph},
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let title = Paragraph::new("MCP Server")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title, layout[0]);

        let clients = Paragraph::new(&self.clients)
            .block(Block::default().borders(Borders::ALL).title("Connected Clients"))
            .style(Style::default().fg(Color::Green));
        f.render_widget(clients, layout[1]);
    }
}

#[cfg(feature = "rest-api")]
impl TabInput for McpTab {
    fn handle_focus_next(&mut self) {}

    fn handle_focus_prev(&mut self) {}

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            self.port.push(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            self.port.pop();
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