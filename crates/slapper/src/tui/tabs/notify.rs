#[cfg(feature = "rest-api")]
use crate::tc;
#[cfg(feature = "rest-api")]
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};

#[cfg(feature = "rest-api")]
pub struct NotifyTab {
    pub webhook_url: String,
    pub secret: String,
    pub title: String,
    pub logs: String,
    pub state: AppState,
}

#[cfg(feature = "rest-api")]
impl NotifyTab {
    pub fn new() -> Self {
        Self {
            webhook_url: String::new(),
            secret: String::new(),
            title: "Slapper Alert".to_string(),
            logs: "Ready".to_string(),
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
impl TabState for NotifyTab {
    fn state(&self) -> AppState {
        self.state
    }

    fn reset(&mut self) {
        self.reset();
    }
}

#[cfg(feature = "rest-api")]
impl TabRender for NotifyTab {
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

        let title = Paragraph::new("Webhook Notifications")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(tc!(info)));
        f.render_widget(title, layout[0]);

        let logs = Paragraph::new(&self.logs)
            .block(Block::default().borders(Borders::ALL).title("Notifications"))
            .style(Style::default().fg(tc!(text)));
        f.render_widget(logs, layout[1]);
    }
}

#[cfg(feature = "rest-api")]
impl TabInput for NotifyTab {
    fn handle_focus_next(&mut self) {}

    fn handle_focus_prev(&mut self) {}

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            self.webhook_url.push(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            self.webhook_url.pop();
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