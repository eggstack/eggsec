use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    InputField, InputGroup, ProgressGauge, ScrollableText, Selector, SelectorItem,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

#[derive(Clone, Copy, PartialEq)]
pub enum StressType {
    Http,
    Syn,
    Udp,
    Tcp,
    Icmp,
}

pub struct StressTab {
    pub inputs: InputGroup,
    pub type_selector: Selector,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: StressFocusArea,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StressFocusArea {
    Inputs,
    TypeSelector,
    Results,
}

impl Default for StressTab {
    fn default() -> Self {
        Self::new()
    }
}

impl StressTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target URL/Host"))
            .add(InputField::new("Rate (requests/sec or packets/sec)").with_value("100"))
            .add(InputField::new("Duration (seconds)").with_value("30"))
            .add(InputField::new("Concurrency").with_value("10"));

        let type_selector = Selector::new("Stress Type").items(vec![
            SelectorItem::new("HTTP Flood", "http"),
            SelectorItem::new("SYN Flood", "syn"),
            SelectorItem::new("UDP Flood", "udp"),
            SelectorItem::new("TCP Flood", "tcp"),
            SelectorItem::new("ICMP Flood", "icmp"),
        ]);

        Self {
            inputs,
            type_selector,
            progress: ProgressGauge::new("Stress testing..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Stress Test Results"),
            focus_area: StressFocusArea::Inputs,
            error: None,
        }
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn rate(&self) -> u64 {
        self.inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(100)
    }

    pub fn duration(&self) -> u64 {
        self.inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(30)
    }

    pub fn concurrency(&self) -> usize {
        self.inputs
            .fields
            .get(3)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn stress_type(&self) -> StressType {
        match self.type_selector.selected_value() {
            Some("syn") => StressType::Syn,
            Some("udp") => StressType::Udp,
            Some("tcp") => StressType::Tcp,
            Some("icmp") => StressType::Icmp,
            _ => StressType::Http,
        }
    }

    pub fn set_results(&mut self, results: StressResults) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            format!("Stress Test Complete: {}", results.target),
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view
            .add_line(Line::from(format!("Type: {}", results.stress_type)));
        self.results_view
            .add_line(Line::from(format!("Duration: {}ms", results.duration_ms)));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Statistics:",
            Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(format!(
            "  Packets Sent: {}",
            results.packets_sent
        )));
        self.results_view
            .add_line(Line::from(format!("  Bytes Sent: {}", results.bytes_sent)));
        self.results_view.add_line(Line::from(format!(
            "  Packets/sec: {:.2}",
            results.packets_per_second
        )));
        self.results_view
            .add_line(Line::from(format!("  Errors: {}", results.errors)));

        if results.responses_received > 0 {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "Response Statistics:",
                Style::default().fg(tc!(warning)),
            )));
            self.results_view.add_line(Line::from(format!(
                "  Responses Received: {}",
                results.responses_received
            )));
            self.results_view.add_line(Line::from(format!(
                "  Avg Latency: {:.2}ms",
                results.avg_latency_ms
            )));
        }
    }
}

#[derive(Clone, Debug)]
pub struct StressResults {
    pub target: String,
    pub stress_type: String,
    pub duration_ms: u64,
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub packets_per_second: f64,
    pub errors: u64,
    pub responses_received: u64,
    pub avg_latency_ms: f64,
}

impl TabState for StressTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results_view.clear();
        self.progress.current = 0;
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        if self.inputs.fields.len() > 3 {
            self.inputs.fields[1].value = "100".to_string();
            self.inputs.fields[1].cursor_pos = 3;
            self.inputs.fields[2].value = "30".to_string();
            self.inputs.fields[2].cursor_pos = 2;
            self.inputs.fields[3].value = "10".to_string();
            self.inputs.fields[3].cursor_pos = 2;
        }
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for StressTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref err) = self.error {
            use ratatui::widgets::Paragraph;
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Stress - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(14),
                Constraint::Length(3),
                Constraint::Min(5),
            ])
            .split(area);

        // Input fields
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(chunks[0]);

        let input_block = Block::default()
            .title(" Stress Test Configuration ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == StressFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        f.render_widget(input_block, chunks[0]);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if i < input_chunks.len() {
                field.render(f, input_chunks[i], insert_mode);
            }
        }

        // Type selector
        let mut selector = self.type_selector.clone();
        selector.focused = self.focus_area == StressFocusArea::TypeSelector;
        selector.render(f, chunks[1]);

        // Results
        self.results_view.render(f, chunks[2], None);

        // Progress bar if running
        if self.state == AppState::Running {
            let progress_area = Rect {
                x: area.x,
                y: area.y + area.height - 1,
                width: area.width,
                height: 1,
            };
            self.progress.render(f, progress_area);
        }
    }
}

impl TabInput for StressTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            StressFocusArea::Inputs => {
                self.inputs.blur();
                StressFocusArea::TypeSelector
            }
            StressFocusArea::TypeSelector => {
                self.type_selector.blur();
                StressFocusArea::Results
            }
            StressFocusArea::Results => {
                self.inputs.focus(0);
                StressFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            StressFocusArea::Inputs => {
                self.inputs.blur();
                StressFocusArea::Results
            }
            StressFocusArea::TypeSelector => {
                self.inputs.focus(0);
                StressFocusArea::Inputs
            }
            StressFocusArea::Results => {
                self.type_selector.focus();
                StressFocusArea::TypeSelector
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if self.focus_area == StressFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.focus_area == StressFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if self.focus_area == StressFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.focus_area == StressFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else if self.focus_area == StressFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if self.focus_area == StressFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.focus_area == StressFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.focus_area == StressFocusArea::Inputs {
            self.inputs.move_home();
        } else if self.focus_area == StressFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.focus_area == StressFocusArea::Inputs {
            self.inputs.move_end();
        } else if self.focus_area == StressFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        self.focus_area = StressFocusArea::Inputs;
        self.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        self.focus_area = StressFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        match self.focus_area {
            StressFocusArea::Inputs => {
                self.inputs.blur();
            }
            StressFocusArea::TypeSelector => {
                self.type_selector.handle_enter();
            }
            StressFocusArea::Results => {}
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
        self.type_selector.blur();
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            StressFocusArea::Inputs => {
                self.inputs.focus_prev();
            }
            StressFocusArea::TypeSelector => {
                self.type_selector.handle_up();
            }
            StressFocusArea::Results => {
                self.results_view.scroll_up(1);
            }
        }
    }

    fn handle_down(&mut self) {
        match self.focus_area {
            StressFocusArea::Inputs => {
                self.inputs.focus_next();
            }
            StressFocusArea::TypeSelector => {
                self.type_selector.handle_down();
            }
            StressFocusArea::Results => {
                self.results_view.scroll_down(1);
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        match self.focus_area {
            StressFocusArea::Inputs => self.inputs.move_left(),
            _ => false,
        }
    }

    fn handle_right(&mut self) -> bool {
        match self.focus_area {
            StressFocusArea::Inputs => self.inputs.move_right(),
            _ => false,
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == StressFocusArea::Inputs && self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            StressFocusArea::Inputs => self.inputs.is_at_left_edge(),
            StressFocusArea::TypeSelector => self.type_selector.selected == 0,
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            StressFocusArea::Inputs => self.inputs.is_at_right_edge(),
            StressFocusArea::TypeSelector => {
                self.type_selector.selected >= self.type_selector.items.len().saturating_sub(1)
            }
            _ => true,
        }
    }
}

impl StressTab {
    pub fn stop(&mut self) {
        if self.state == AppState::Running {
            self.state = AppState::Idle;
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.scroll_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.scroll_down(page_size);
    }

    pub fn handle_word_forward(&mut self) {
        for _ in 0..5 {
            self.handle_right();
        }
    }

    pub fn handle_word_backward(&mut self) {
        for _ in 0..5 {
            self.handle_left();
        }
    }

    pub fn handle_home(&mut self) {
        for _ in 0..100 {
            self.handle_left();
        }
    }

    pub fn handle_end(&mut self) {
        for _ in 0..100 {
            self.handle_right();
        }
    }

    pub fn handle_top(&mut self) {
        for _ in 0..100 {
            self.results_view.scroll_up(1);
        }
    }

    pub fn handle_bottom(&mut self) {
        for _ in 0..100 {
            self.results_view.scroll_down(1);
        }
    }
}
