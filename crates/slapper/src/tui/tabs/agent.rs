use crate::tc;
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentFocusArea {
    Main,
}

pub struct AgentTab {
    pub target: String,
    pub portfolio_path: String,
    pub memory_dir: String,
    pub poll_interval: String,
    pub status: String,
    pub state: AppState,
    pub focus_area: AgentFocusArea,
}

impl AgentTab {
    pub fn new() -> Self {
        Self {
            target: String::new(),
            portfolio_path: String::new(),
            memory_dir: "~/.config/slapper/memory".to_string(),
            poll_interval: "60".to_string(),
            status: "Idle".to_string(),
            state: AppState::Idle,
            focus_area: AgentFocusArea::Main,
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

impl TabState for AgentTab {
    fn state(&self) -> AppState {
        self.state
    }

    fn reset(&mut self) {
        self.reset();
    }
}

impl TabRender for AgentTab {
    fn render(&self, f: &mut Frame, area: Rect, _insert_mode: bool) {
        use ratatui::style::Style;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let title = Paragraph::new("Agent Configuration")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(tc!(info)));
        f.render_widget(title, layout[0]);

        let status = Paragraph::new(format!("Status: {}", self.status))
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::default().fg(tc!(success)));
        f.render_widget(status, layout[1]);
    }
}

impl TabInput for AgentTab {
    fn handle_focus_next(&mut self) {}

    fn handle_focus_prev(&mut self) {}

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            self.target.push(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            self.target.pop();
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