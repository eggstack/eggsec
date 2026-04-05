use crate::loadtest::metrics::LoadTestResults;
use crate::tui::components::{InputField, InputGroup, ProgressGauge, ScrollableText, Selector};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    Frame,
};

pub struct LoadTab {
    pub inputs: InputGroup,
    pub test_type_selector: Selector,
    pub results: Option<LoadTestResults>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
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
            .unwrap_or_default()
    }

    pub fn set_results(&mut self, results: LoadTestResults) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.state = AppState::Completed;
    }

    #[cfg(feature = "stress-testing")]
    pub fn set_stress_results(&mut self, target: String, stats: crate::stress::StressStats) {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};

        self.results_view.clear();

        self.results_view.add_line(Line::from(vec![Span::styled(
            "Stress Test Results",
            Style::default().fg(Color::Yellow),
        )]));
        self.results_view.add_line(Line::from(""));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Target: ", Style::default().fg(Color::Cyan)),
            Span::raw(target),
        ]));

        let pps = if stats.duration_ms > 0 {
            (stats.packets_sent * 1000) / stats.duration_ms
        } else {
            0
        };

        self.results_view.add_line(Line::from(vec![
            Span::styled("Packets: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{} sent, {} errors",
                stats.packets_sent, stats.errors
            )),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Rate: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} pps", pps)),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} ms", stats.duration_ms)),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Data Sent: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} bytes", stats.bytes_sent)),
        ]));

        self.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &LoadTestResults) {
        use ratatui::style::{Color, Style};
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
            Span::styled("Target: ", Style::default().fg(Color::Yellow)),
            Span::raw(target_url),
        ]));
        self.results_view.add_line(Line::from(""));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Requests: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{} total, {} success, {} failed",
                total_requests, successful_requests, failed_requests
            )),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("RPS: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:.2} req/s", rps)),
        ]));

        self.results_view.add_line(Line::from(""));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Latency: ", Style::default().fg(Color::Green)),
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
                Style::default().fg(Color::Yellow),
            )));
            let mut codes: Vec<_> = status_codes.iter().collect();
            codes.sort_by_key(|(k, _)| *k);
            for (code, count) in codes {
                let color = match *code {
                    200..=299 => Color::Green,
                    300..=399 => Color::Blue,
                    400..=499 => Color::Yellow,
                    _ => Color::Red,
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
                Style::default().fg(Color::Red),
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
        self.results_view.clear();
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.inputs.fields[1].value = "GET".to_string();
        self.inputs.fields[1].cursor_pos = 3;
        self.inputs.fields[2].value = "100".to_string();
        self.inputs.fields[2].cursor_pos = 3;
        self.inputs.fields[3].value = "10".to_string();
        self.inputs.fields[3].cursor_pos = 2;
        self.inputs.fields[4].value = "30".to_string();
        self.inputs.fields[4].cursor_pos = 2;
    }
}

impl TabRender for LoadTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),
                Constraint::Length(15),
                Constraint::Min(0),
            ])
            .split(area);

        let selector_area = chunks[0];
        let input_area = chunks[1];
        let results_area = chunks[2];

        self.test_type_selector.render(f, selector_area);

        if let Some(dropdown) = self.test_type_selector.dropdown_info(selector_area) {
            dropdown.render(f);
        }

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_area);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            field.render(f, input_chunks[i], insert_mode);
        }

        if self.state == AppState::Running {
            self.progress.render(f, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, Some(Color::Green));
        } else {
            use ratatui::style::Style;
            use ratatui::widgets::{Block, Borders, Paragraph};
            let placeholder = Paragraph::new("Results will appear here after running")
                .block(Block::default().borders(Borders::ALL).title("Results"))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for LoadTab {
    fn handle_focus_next(&mut self) {
        if self.test_type_selector.is_focused() {
            self.test_type_selector.blur();
            self.inputs.focus_next();
        } else if self.inputs.is_focused() {
            self.inputs.focus_next();
            if self.inputs.is_focused() {
                self.inputs.blur();
                self.test_type_selector.focus();
            }
        } else {
            self.test_type_selector.focus();
        }
    }

    fn handle_focus_prev(&mut self) {
        if self.test_type_selector.is_focused() {
            self.test_type_selector.blur();
            self.inputs.focus_prev();
        } else if self.inputs.is_focused() {
            self.inputs.focus_prev();
            if !self.inputs.is_focused() {
                self.inputs.blur();
                self.test_type_selector.focus();
            }
        } else {
            self.test_type_selector.focus();
        }
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

    fn handle_enter(&mut self) {
        if self.test_type_selector.is_focused() {
            self.test_type_selector.handle_enter();
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
        if self.test_type_selector.is_focused() {
            self.test_type_selector.blur();
        }
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.test_type_selector.is_focused() {
            self.test_type_selector.handle_up();
        } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
            self.scroll_results_up();
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.test_type_selector.is_focused() {
            self.test_type_selector.handle_down();
        } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
            self.scroll_results_down();
        } else {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.test_type_selector.is_focused() {
            self.test_type_selector.handle_left();
            true
        } else {
            self.inputs.move_left()
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.test_type_selector.is_focused() {
            self.test_type_selector.handle_right();
            true
        } else {
            self.inputs.move_right()
        }
    }

    fn is_input_focused(&self) -> bool {
        self.test_type_selector.is_focused() || self.inputs.is_focused()
    }
}
