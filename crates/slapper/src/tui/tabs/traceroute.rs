#[cfg(feature = "stress-testing")]
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};

#[cfg(feature = "stress-testing")]
pub struct TracerouteTab {
    pub target: String,
    pub max_hops: String,
    pub timeout: String,
    pub hops: String,
    pub state: AppState,
}

#[cfg(feature = "stress-testing")]
impl TracerouteTab {
    pub fn new() -> Self {
        Self {
            target: String::new(),
            max_hops: "30".to_string(),
            timeout: "2".to_string(),
            hops: "Enter target to trace route".to_string(),
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

#[cfg(feature = "stress-testing")]
impl TabState for TracerouteTab {
    fn state(&self) -> AppState {
        self.state
    }

    fn reset(&mut self) {
        self.reset();
    }
}

#[cfg(feature = "stress-testing")]
impl TabRender for TracerouteTab {
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

        let title = Paragraph::new("Traceroute")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title, layout[0]);

        let hops = Paragraph::new(&self.hops)
            .block(Block::default().borders(Borders::ALL).title("Hops"))
            .style(Style::default().fg(Color::White));
        f.render_widget(hops, layout[1]);
    }
}

#[cfg(feature = "stress-testing")]
impl TabInput for TracerouteTab {
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