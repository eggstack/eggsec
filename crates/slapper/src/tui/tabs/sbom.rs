#[cfg(feature = "sbom")]
use crate::tc;
#[cfg(feature = "sbom")]
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};

#[cfg(feature = "sbom")]
pub struct SbomTab {
    pub output_path: String,
    pub format: String,
    pub output: String,
    pub state: AppState,
}

#[cfg(feature = "sbom")]
impl SbomTab {
    pub fn new() -> Self {
        Self {
            output_path: String::new(),
            format: "cyclonedx".to_string(),
            output: "No SBOM generated".to_string(),
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

#[cfg(feature = "sbom")]
impl TabState for SbomTab {
    fn state(&self) -> AppState {
        self.state
    }

    fn reset(&mut self) {
        self.reset();
    }
}

#[cfg(feature = "sbom")]
impl TabRender for SbomTab {
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

        let title = Paragraph::new("SBOM Generation")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(tc!(info)));
        f.render_widget(title, layout[0]);

        let output = Paragraph::new(&self.output)
            .block(Block::default().borders(Borders::ALL).title("SBOM"))
            .style(Style::default().fg(tc!(text)));
        f.render_widget(output, layout[1]);
    }
}

#[cfg(feature = "sbom")]
impl TabInput for SbomTab {
    fn handle_focus_next(&mut self) {}

    fn handle_focus_prev(&mut self) {}

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            self.output_path.push(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            self.output_path.pop();
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