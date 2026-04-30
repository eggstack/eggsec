use crate::tc;
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct PlanTab {
    pub target: String,
    pub profile: String,
    pub preview: String,
    pub state: AppState,
}

impl PlanTab {
    pub fn new() -> Self {
        Self {
            target: String::new(),
            profile: "default".to_string(),
            preview: "Enter target to preview scan plan".to_string(),
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

impl TabState for PlanTab {
    fn state(&self) -> AppState {
        self.state
    }

    fn reset(&mut self) {
        self.reset();
    }
}

impl TabRender for PlanTab {
    fn render(&self, f: &mut Frame, area: Rect, _insert_mode: bool) {
        use ratatui::style::Style;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let title = Paragraph::new("Scan Plan Preview")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(tc!(info)));
        f.render_widget(title, layout[0]);

        let preview = Paragraph::new(&self.preview)
            .block(Block::default().borders(Borders::ALL).title("Plan Stages"))
            .style(Style::default().fg(tc!(text)));
        f.render_widget(preview, layout[1]);
    }
}

impl TabInput for PlanTab {
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