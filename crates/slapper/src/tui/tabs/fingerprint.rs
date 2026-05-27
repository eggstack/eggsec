use crate::scanner::fingerprint::FingerprintResults;
use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, InputField, InputGroup, ProgressGauge, ScrollableText,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FingerprintFocusArea {
    Inputs,
    Results,
}

pub struct FingerprintTab {
    pub inputs: InputGroup,
    pub results: Option<FingerprintResults>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: FingerprintFocusArea,
    pub error: Option<TabError>,
}

impl FingerprintTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target Host"))
            .add(
                InputField::new("Ports (comma-separated)")
                    .with_value("80,443,22,21,25,3306,5432,6379,27017"),
            )
            .add(InputField::new("Timeout (s)").with_value("5"));

        Self {
            inputs,
            results: None,
            progress: ProgressGauge::new("Fingerprinting..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: FingerprintFocusArea::Inputs,
            error: None,
        }
    }

    pub fn get_results(&self) -> Option<&FingerprintResults> {
        self.results.as_ref()
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn ports(&self) -> &str {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn timeout(&self) -> u64 {
        self.inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(5)
    }

    pub fn set_results(&mut self, results: FingerprintResults) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &FingerprintResults) {
        self.results_view.clear();

        let host = results.host.clone();
        let services_identified = results.services_identified;

        let fp_data: Vec<_> = results
            .results
            .iter()
            .map(|fp| {
                let banner = fp
                    .banner
                    .as_deref()
                    .unwrap_or("-")
                    .lines()
                    .next()
                    .unwrap_or("-");
                let banner_display = if banner.len() > 40 {
                    format!("{}...", &banner[..37])
                } else {
                    banner.to_string()
                };
                (
                    fp.port,
                    fp.service.clone(),
                    fp.version.clone(),
                    banner_display,
                )
            })
            .collect();

        self.results_view.add_line(Line::from(vec![
            Span::styled("Host: ", Style::default().fg(tc!(warning))),
            Span::raw(host),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Services identified: ", Style::default().fg(tc!(info))),
            Span::raw(services_identified.to_string()),
        ]));

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<8}", "PORT"), Style::default().fg(tc!(warning))),
            Span::styled(
                format!("{:<15}", "SERVICE"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled(
                format!("{:<12}", "VERSION"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled("BANNER", Style::default().fg(tc!(warning))),
        ]));

        for (port, service, version, banner_display) in fp_data {
            self.results_view.add_line(Line::from(vec![
                Span::styled(format!("{:<8}", port), Style::default().fg(tc!(success))),
                Span::raw(format!("{:<15}", service)),
                Span::raw(format!("{:<12}", version.as_deref().unwrap_or("-"))),
                Span::styled(banner_display, Style::default().fg(tc!(text_dim))),
            ]));
        }
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.progress.current = 0;
            self.results = None;
            self.results_view.clear();
            self.error = None;
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.progress.current = completed;
        self.progress.total = total;
    }

    pub fn scroll_results_up(&mut self) {
        self.results_view.scroll_up(1);
    }

    pub fn scroll_results_down(&mut self) {
        self.results_view.scroll_down(1);
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
    }
}

impl Default for FingerprintTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for FingerprintTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results = None;
        self.progress.current = 0;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        if let Some(field) = self.inputs.fields.get_mut(1) {
            field.value = "80,443,22,21,25,3306,5432,6379,27017".to_string();
            field.cursor_pos = 35;
        }
        if let Some(field) = self.inputs.fields.get_mut(2) {
            field.value = "5".to_string();
            field.cursor_pos = 1;
        }
        self.focus_area = FingerprintFocusArea::Inputs;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for FingerprintTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_area);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        if self.state == AppState::Running {
            self.progress.render(f, results_area);
        } else if let Some(ref err) = self.error {
            use ratatui::style::Style;
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Fingerprint - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, Some(tc!(success)));
        } else {
            let placeholder =
                empty_state_paragraph("Results", "Results will appear here after running");
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for FingerprintTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        if self.inputs.is_focused() {
            self.inputs.focus_next();
            if !self.inputs.is_focused() {
                self.focus_area = FingerprintFocusArea::Results;
            }
        } else {
            self.focus_area = FingerprintFocusArea::Results;
        }
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == FingerprintFocusArea::Results {
            if !self.inputs.fields.is_empty() {
                self.inputs.focus(self.inputs.fields.len() - 1);
            }
            self.focus_area = FingerprintFocusArea::Inputs;
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() {
            self.inputs.paste(text);
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if self.focus_area == FingerprintFocusArea::Inputs {
                self.inputs.move_word_forward();
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if self.focus_area == FingerprintFocusArea::Inputs {
                self.inputs.move_word_backward();
            }
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == FingerprintFocusArea::Inputs {
                self.inputs.move_home();
            } else if self.focus_area == FingerprintFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == FingerprintFocusArea::Inputs {
                self.inputs.move_end();
            } else if self.focus_area == FingerprintFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = FingerprintFocusArea::Inputs;
            self.inputs.focus(0);
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = FingerprintFocusArea::Results;
            self.inputs.blur();
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
        } else if self.inputs.is_focused() {
            self.inputs.blur();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == FingerprintFocusArea::Results {
            self.scroll_results_up();
        } else if self.inputs.is_focused() {
            self.inputs.focus_prev();
        } else if !self.results_view.is_empty() {
            self.scroll_results_up();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == FingerprintFocusArea::Results {
            self.scroll_results_down();
        } else if self.inputs.is_focused() {
            self.inputs.focus_next();
        } else if !self.results_view.is_empty() {
            self.scroll_results_down();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        self.inputs.move_left()
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        self.inputs.move_right()
    }

    fn is_input_focused(&self) -> bool {
        self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == FingerprintFocusArea::Inputs {
            self.inputs.fields.is_empty() || self.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == FingerprintFocusArea::Inputs {
            self.inputs.fields.is_empty() || self.inputs.is_at_right_edge()
        } else {
            true
        }
    }
}
