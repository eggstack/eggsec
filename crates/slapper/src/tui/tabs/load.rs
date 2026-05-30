use crate::loadtest::metrics::LoadTestResults;
use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, InputField, InputGroup, ProgressGauge, ScrollableText, Selector,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadFocusArea {
    Selector,
    Inputs,
    Results,
}

pub struct LoadTab {
    pub inputs: InputGroup,
    pub test_type_selector: Selector,
    pub results: Option<LoadTestResults>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: LoadFocusArea,
    pub error: Option<TabError>,
}

impl LoadTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target URL/Host"))
            .add(InputField::new("Method (GET/POST/etc)").with_value("GET"))
            .add(InputField::new("Total Requests").with_value("100"))
            .add(InputField::new("Concurrency").with_value("10"))
            .add(InputField::new("Timeout (s)").with_value("30"))
            .add(InputField::new("Request Body (optional)"))
            .add(InputField::new("Headers (Key:Value, optional)"));

        #[cfg(feature = "stress-testing")]
        let test_type_selector = Selector::new("Test Type").simple_items(vec![
            "HTTP Load",
            "SYN Flood",
            "UDP Flood",
            "TCP Flood",
            "ICMP Ping Flood",
        ]);

        #[cfg(not(feature = "stress-testing"))]
        let test_type_selector = Selector::new("Test Type").simple_items(vec!["HTTP Load"]);

        Self {
            inputs,
            test_type_selector,
            results: None,
            progress: ProgressGauge::new("Load testing..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: LoadFocusArea::Selector,
            error: None,
        }
    }

    pub fn is_stress_test(&self) -> bool {
        self.test_type_selector.selected > 0
    }

    pub fn stress_type(&self) -> &str {
        match self.test_type_selector.selected {
            1 => "syn",
            2 => "udp",
            3 => "tcp",
            4 => "icmp",
            _ => "http",
        }
    }

    pub fn stress_type_name(&self) -> &str {
        self.test_type_selector
            .selected_label()
            .unwrap_or("HTTP Load")
    }

    pub fn get_results(&self) -> Option<&LoadTestResults> {
        self.results.as_ref()
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn method(&self) -> &str {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("GET")
    }

    pub fn requests(&self) -> u64 {
        self.inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(100)
    }

    pub fn concurrency(&self) -> usize {
        self.inputs
            .fields
            .get(3)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn timeout(&self) -> u64 {
        self.inputs
            .fields
            .get(4)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(30)
    }

    pub fn body(&self) -> Option<&str> {
        let b = self
            .inputs
            .fields
            .get(5)
            .map(|f| f.value.as_str())
            .unwrap_or("");
        if b.is_empty() {
            None
        } else {
            Some(b)
        }
    }

    pub fn headers(&self) -> Vec<String> {
        self.inputs
            .fields
            .get(6)
            .map(|f| {
                f.value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| {
                tracing::warn!("Failed to parse headers");
                Vec::new()
            })
    }

    pub fn set_results(&mut self, results: LoadTestResults) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.state = AppState::Completed;
    }

    #[cfg(feature = "stress-testing")]
    pub fn set_stress_results(&mut self, target: String, stats: crate::stress::StressStats) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};

        self.results_view.clear();

        self.results_view.add_line(Line::from(vec![Span::styled(
            "Stress Test Results",
            Style::default().fg(tc!(accent)),
        )]));
        self.results_view.add_line(Line::from(""));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Target: ", Style::default().fg(tc!(info))),
            Span::raw(target),
        ]));

        let pps = if stats.duration_ms > 0 {
            (stats.packets_sent * 1000) / stats.duration_ms
        } else {
            0
        };

        self.results_view.add_line(Line::from(vec![
            Span::styled("Packets: ", Style::default().fg(tc!(info))),
            Span::raw(format!(
                "{} sent, {} errors",
                stats.packets_sent, stats.errors
            )),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Rate: ", Style::default().fg(tc!(info))),
            Span::raw(format!("{} pps", pps)),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(tc!(info))),
            Span::raw(format!("{} ms", stats.duration_ms)),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Data Sent: ", Style::default().fg(tc!(info))),
            Span::raw(format!("{} bytes", stats.bytes_sent)),
        ]));

        self.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &LoadTestResults) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};

        self.results_view.clear();

        let target_url = results.target_url.clone();
        let total_requests = results.total_requests;
        let successful_requests = results.successful_requests;
        let failed_requests = results.failed_requests;
        let rps = results.requests_per_second;
        let min_ms = results.latency_min_ms;
        let max_ms = results.latency_max_ms;
        let mean_ms = results.latency_mean_ms;
        let p50 = results.latency_p50_ms;
        let p90 = results.latency_p90_ms;
        let p95 = results.latency_p95_ms;
        let p99 = results.latency_p99_ms;

        self.results_view.add_line(Line::from(vec![
            Span::styled("Target: ", Style::default().fg(tc!(accent))),
            Span::raw(target_url),
        ]));
        self.results_view.add_line(Line::from(""));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Requests: ", Style::default().fg(tc!(info))),
            Span::raw(format!(
                "{} total, {} success, {} failed",
                total_requests, successful_requests, failed_requests
            )),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("RPS: ", Style::default().fg(tc!(info))),
            Span::raw(format!("{:.2} req/s", rps)),
        ]));

        self.results_view.add_line(Line::from(""));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Latency: ", Style::default().fg(tc!(success))),
            Span::raw(format!(
                "min={:.2}ms, max={:.2}ms, mean={:.2}ms",
                min_ms, max_ms, mean_ms
            )),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::raw("         "),
            Span::raw(format!(
                "p50={:.2}ms, p90={:.2}ms, p95={:.2}ms, p99={:.2}ms",
                p50, p90, p95, p99
            )),
        ]));

        let status_codes = results.status_codes.clone();
        if !status_codes.is_empty() {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "Status Codes:",
                Style::default().fg(tc!(accent)),
            )));
            let mut codes: Vec<_> = status_codes.iter().collect();
            codes.sort_by_key(|(k, _)| *k);
            for (code, count) in codes {
                let color = match *code {
                    200..=299 => tc!(success),
                    300..=399 => tc!(secondary),
                    400..=499 => tc!(warning),
                    _ => tc!(error),
                };
                self.results_view.add_line(Line::from(vec![
                    Span::styled(format!("  {}:", code), Style::default().fg(color)),
                    Span::raw(format!(" {}", count)),
                ]));
            }
        }

        let errors = results.errors.clone();
        if !errors.is_empty() {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "Errors:",
                Style::default().fg(tc!(error)),
            )));
            for error in &errors {
                self.results_view
                    .add_line(Line::from(format!("  - {}", error)));
            }
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
        self.progress.current = completed.min(total);
        self.progress.total = total.max(1);
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

impl Default for LoadTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for LoadTab {
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
        if self.inputs.fields.len() > 4 {
            if let Some(field) = self.inputs.fields.get_mut(1) {
                field.value = "GET".to_string();
                field.cursor_pos = 3;
            }
            if let Some(field) = self.inputs.fields.get_mut(2) {
                field.value = "100".to_string();
                field.cursor_pos = 3;
            }
            if let Some(field) = self.inputs.fields.get_mut(3) {
                field.value = "10".to_string();
                field.cursor_pos = 2;
            }
            if let Some(field) = self.inputs.fields.get_mut(4) {
                field.value = "30".to_string();
                field.cursor_pos = 2;
            }
        }
        self.test_type_selector.select(0);
        self.focus_area = LoadFocusArea::Selector;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for LoadTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        // Use dynamic height for input section based on terminal height
        let input_height = if area.height <= 24 {
            ((area.height as f32 * 0.6) as u16).clamp(6, 15)
        } else {
            15
        };
        let results_height = if area.height <= 24 {
            ((area.height as f32 * 0.4) as u16).max(3)
        } else {
            0
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),            // Selector
                Constraint::Length(input_height), // Inputs
                Constraint::Min(results_height),  // Results
            ])
            .split(area);

        let selector_area = chunks[0];
        let input_area = chunks[1];
        let results_area = chunks[2];

        self.test_type_selector.render(f, selector_area);

        if let Some(dropdown) = self.test_type_selector.dropdown_info(selector_area) {
            dropdown.render(f);
        }

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Load Test Configuration ")
            .border_style(
                Style::default().fg(if self.focus_area == LoadFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let input_inner = input_block.inner(input_area);
        f.render_widget(input_block, input_area);

        let num_fields = self.inputs.fields.len().max(1);
        let field_height = (input_inner.height / num_fields as u16).max(2);
        let constraints: Vec<Constraint> = (0..num_fields)
            .map(|_| Constraint::Length(field_height))
            .collect();

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(input_inner);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(
                Style::default().fg(if self.focus_area == LoadFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
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
                .render(f, results_inner, None);
        } else {
            let placeholder =
                empty_state_paragraph("Results", "Results will appear here after running");
            f.render_widget(placeholder, results_inner);
        }
    }

    fn render_overlays(&self, f: &mut Frame, area: Rect) {
        let input_height = if area.height <= 24 {
            ((area.height as f32 * 0.6) as u16).clamp(6, 15)
        } else {
            15
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),
                Constraint::Length(input_height),
                Constraint::Min(0),
            ])
            .split(area);

        let selector_area = chunks[0];

        if let Some(dropdown) = self.test_type_selector.dropdown_info(selector_area) {
            dropdown.render(f);
        }
    }
}

impl TabInput for LoadTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            LoadFocusArea::Selector => {
                self.test_type_selector.blur();
                self.inputs.focus(0);
                LoadFocusArea::Inputs
            }
            LoadFocusArea::Inputs => {
                self.inputs.blur();
                LoadFocusArea::Results
            }
            LoadFocusArea::Results => {
                self.test_type_selector.focus();
                LoadFocusArea::Selector
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            LoadFocusArea::Selector => {
                self.test_type_selector.blur();
                LoadFocusArea::Results
            }
            LoadFocusArea::Inputs => {
                self.inputs.blur();
                self.test_type_selector.focus();
                LoadFocusArea::Selector
            }
            LoadFocusArea::Results => {
                self.inputs.focus(0);
                LoadFocusArea::Inputs
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            if self.test_type_selector.is_focused() {
                self.test_type_selector.handle_char(c);
            } else {
                self.inputs.insert(c);
            }
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            if self.test_type_selector.is_focused() {
                self.test_type_selector.handle_backspace();
            } else {
                self.inputs.backspace();
            }
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && !self.test_type_selector.is_focused() {
            self.inputs.paste(text);
        }
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == LoadFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == LoadFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == LoadFocusArea::Inputs {
            self.inputs.move_home();
        } else if self.focus_area == LoadFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == LoadFocusArea::Inputs {
            self.inputs.move_end();
        } else if self.focus_area == LoadFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = LoadFocusArea::Selector;
        self.test_type_selector.focus();
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = LoadFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.test_type_selector.is_focused() {
            if self.test_type_selector.is_open() {
                if self.test_type_selector.confirm().is_none() {
                    tracing::warn!("Failed to confirm load test type selector");
                }
            } else {
                self.test_type_selector.open();
            }
            return;
        }
        if self.inputs.is_focused() {
            self.inputs.blur();
        } else if self.is_running() {
            self.stop();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.test_type_selector.is_open() {
            self.test_type_selector.cancel();
            return;
        }
        if self.test_type_selector.is_focused() {
            self.test_type_selector.blur();
        }
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == LoadFocusArea::Selector {
            if self.test_type_selector.is_open() {
                self.test_type_selector.move_prev();
            }
        } else if self.focus_area == LoadFocusArea::Inputs {
            if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_up();
            } else {
                self.inputs.focus_prev();
            }
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == LoadFocusArea::Selector {
            if self.test_type_selector.is_open() {
                self.test_type_selector.move_next();
            }
        } else if self.focus_area == LoadFocusArea::Inputs {
            if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_down();
            } else {
                self.inputs.focus_next();
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.test_type_selector.is_focused() {
            if self.test_type_selector.is_open() {
                self.test_type_selector.move_prev();
                true
            } else {
                false
            }
        } else {
            self.inputs.move_left()
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.test_type_selector.is_focused() {
            if self.test_type_selector.is_open() {
                self.test_type_selector.move_next();
                true
            } else {
                false
            }
        } else {
            self.inputs.move_right()
        }
    }

    fn is_input_focused(&self) -> bool {
        self.test_type_selector.is_focused() || self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.test_type_selector.is_focused() {
            if self.test_type_selector.is_open() {
                self.test_type_selector.items.is_empty() || self.test_type_selector.selected == 0
            } else {
                true
            }
        } else if self.inputs.is_focused() {
            self.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.test_type_selector.is_focused() {
            if self.test_type_selector.is_open() {
                self.test_type_selector.items.is_empty()
                    || self.test_type_selector.selected
                        >= self.test_type_selector.items.len().saturating_sub(1)
            } else {
                true
            }
        } else if self.inputs.is_focused() {
            self.inputs.is_at_right_edge()
        } else {
            true
        }
    }
}
