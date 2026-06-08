use crate::wireless::WirelessScanResult;
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
pub enum WirelessFocusArea {
    Inputs,
    Results,
}

pub struct WirelessTab {
    pub inputs: InputGroup,
    pub results: Option<WirelessScanResult>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: WirelessFocusArea,
    pub error: Option<TabError>,
}

impl WirelessTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Wireless Interface"));

        Self {
            inputs,
            results: None,
            progress: ProgressGauge::new("Scanning..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: WirelessFocusArea::Inputs,
            error: None,
        }
    }

    pub fn get_results(&self) -> Option<&WirelessScanResult> {
        self.results.as_ref()
    }

    pub fn interface(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn set_results(&mut self, results: WirelessScanResult) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &WirelessScanResult) {
        self.results_view.clear();

        let interface = results.interface.clone();
        let network_count = results.networks.len();

        self.results_view.add_line(Line::from(vec![
            Span::styled("Interface: ", Style::default().fg(tc!(warning))),
            Span::raw(interface),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Networks found: ", Style::default().fg(tc!(info))),
            Span::raw(network_count.to_string()),
        ]));

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<20}", "SSID"), Style::default().fg(tc!(warning))),
            Span::styled(
                format!("{:<20}", "BSSID"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled(
                format!("{:<8}", "CH"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled(
                format!("{:<12}", "SECURITY"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled("SIGNAL", Style::default().fg(tc!(warning))),
        ]));

        for network in &results.networks {
            let ssid_display = if network.ssid.len() > 18 {
                let truncate_pos = network
                    .ssid
                    .char_indices()
                    .take_while(|(i, _)| *i < 15)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(15);
                format!("{}...", &network.ssid[..truncate_pos])
            } else {
                network.ssid.clone()
            };
            self.results_view.add_line(Line::from(vec![
                Span::styled(format!("{:<20}", ssid_display), Style::default().fg(tc!(success))),
                Span::raw(format!("{:<20}", network.bssid)),
                Span::raw(format!("{:<8}", network.channel)),
                Span::styled(
                    format!("{:<12}", network.security_type.as_str()),
                    Style::default().fg(match network.security_type {
                        crate::wireless::SecurityType::Open => tc!(error),
                        crate::wireless::SecurityType::WEP => tc!(error),
                        crate::wireless::SecurityType::WPA => tc!(warning),
                        _ => tc!(success),
                    }),
                ),
                Span::raw(format!("{} dBm", network.signal_strength)),
            ]));
        }

        if !results.recommendations.is_empty() {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(vec![
                Span::styled("Recommendations:", Style::default().fg(tc!(warning))),
            ]));
            for rec in &results.recommendations {
                self.results_view.add_line(Line::from(vec![
                    Span::styled("  - ", Style::default().fg(tc!(info))),
                    Span::raw(rec.clone()),
                ]));
            }
        }
    }

    pub fn start(&mut self) {
        if !self.interface().is_empty() {
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
}

impl Default for WirelessTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for WirelessTab {
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
        self.progress.total = 0;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.focus_area = WirelessFocusArea::Inputs;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for WirelessTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Wireless Scan Configuration ")
            .border_style(Style::default().fg(
                if self.focus_area == WirelessFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        let input_inner = input_block.inner(input_area);
        f.render_widget(input_block, input_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3)])
            .split(input_inner);

        if let (Some(chunk), Some(field)) = (input_chunks.first(), self.inputs.fields.first()) {
            field.render(f, *chunk, insert_mode);
        }

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(Style::default().fg(
                if self.focus_area == WirelessFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        let results_inner = results_block.inner(results_area);
        f.render_widget(results_block, results_area);

        if self.state == AppState::Running {
            self.progress.render(f, results_inner);
        } else if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_inner);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_inner, Some(tc!(success)));
        } else {
            let placeholder =
                empty_state_paragraph("Results", "Results will appear here after scanning");
            f.render_widget(placeholder, results_inner);
        }
    }
}

impl TabInput for WirelessTab {
    fn stop(&mut self) {
        WirelessTab::stop(self);
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => {
                self.inputs.blur();
                self.focus_area = WirelessFocusArea::Results;
            }
            WirelessFocusArea::Results => {
                self.focus_area = WirelessFocusArea::Inputs;
                if !self.inputs.fields.is_empty() {
                    self.inputs.focus(0);
                }
            }
        }
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => {
                self.inputs.blur();
                self.focus_area = WirelessFocusArea::Results;
            }
            WirelessFocusArea::Results => {
                self.focus_area = WirelessFocusArea::Inputs;
                if !self.inputs.fields.is_empty() {
                    self.inputs.focus(0);
                }
            }
        }
    }

    fn handle_char(&mut self, c: char) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            return;
        }
        self.start();
    }

    fn handle_escape(&mut self) {
        self.stop();
        self.focus_area = WirelessFocusArea::Inputs;
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Results {
            self.scroll_results_up();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Results {
            self.scroll_results_down();
        }
    }

    fn handle_left(&mut self) -> bool {
        false
    }

    fn handle_right(&mut self) -> bool {
        false
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Inputs {
            self.inputs.move_home();
        } else if self.focus_area == WirelessFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Inputs {
            self.inputs.move_end();
        } else if self.focus_area == WirelessFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => {
                if !self.inputs.fields.is_empty() {
                    self.inputs.focus(0);
                }
            }
            WirelessFocusArea::Results => {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => {
                self.inputs.blur();
                self.focus_area = WirelessFocusArea::Results;
            }
            WirelessFocusArea::Results => {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == WirelessFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == WirelessFocusArea::Inputs && self.inputs.is_focused()
    }

    fn page_up(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Results {
            for _ in 0..page_size {
                self.results_view.scroll_up(1);
            }
        }
    }

    fn page_down(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Results {
            for _ in 0..page_size {
                self.results_view.scroll_down(1);
            }
        }
    }
}
