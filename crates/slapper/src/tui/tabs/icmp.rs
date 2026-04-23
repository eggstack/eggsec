#[cfg(feature = "stress-testing")]
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};

#[cfg(feature = "stress-testing")]
pub struct IcmpTab {
    pub target: String,
    pub count: String,
    pub timeout: String,
    pub results: String,
    pub state: AppState,
}

#[cfg(feature = "stress-testing")]
impl IcmpTab {
    pub fn new() -> Self {
        Self {
            target: String::new(),
            count: "4".to_string(),
            timeout: "2".to_string(),
            results: "Ready for ping".to_string(),
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
impl TabState for IcmpTab {
    fn state(&self) -> AppState {
        self.state
    }

    fn reset(&mut self) {
        self.reset();
    }
}

#[cfg(feature = "stress-testing")]
impl TabRender for IcmpTab {
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

        let title = Paragraph::new("ICMP Probing")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title, layout[0]);

        let results = Paragraph::new(&self.results)
            .block(Block::default().borders(Borders::ALL).title("Results"))
            .style(Style::default().fg(Color::White));
        f.render_widget(results, layout[1]);
    }
}

#[cfg(feature = "stress-testing")]
impl TabInput for IcmpTab {
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