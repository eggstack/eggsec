use crate::components::{InputField, InputGroup, Selector};
use crate::tabs::core::{
    render_input_fields, render_results_area, start_scan, StandardFocusAreaSelector, TabCore,
};
use crate::tabs::{TabInput, TabRender, TabState};
use crate::{tab_state_boilerplate, tc};
use eggsec::loadtest::metrics::LoadTestResults;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};

pub struct LoadTab {
    pub core: TabCore,
    pub test_type_selector: Selector,
    pub results: Option<LoadTestResults>,
    pub focus_area: StandardFocusAreaSelector,
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
            core: TabCore::new("Load testing...", "Results").with_inputs(inputs),
            test_type_selector,
            results: None,
            focus_area: StandardFocusAreaSelector::Selector,
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
        self.core.target()
    }

    pub fn method(&self) -> &str {
        self.core
            .inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("GET")
    }

    pub fn requests(&self) -> u64 {
        self.core
            .inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(100)
    }

    pub fn concurrency(&self) -> usize {
        self.core
            .inputs
            .fields
            .get(3)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn timeout(&self) -> u64 {
        self.core
            .inputs
            .fields
            .get(4)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(30)
    }

    pub fn body(&self) -> Option<&str> {
        let b = self
            .core
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
        self.core
            .inputs
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
                tracing::trace!("Headers field not found, using empty headers");
                Vec::new()
            })
    }

    pub fn set_results(&mut self, results: LoadTestResults) {
        self.update_results_view(&results);
        self.results = Some(results);
    }

    #[cfg(feature = "stress-testing")]
    pub fn set_stress_results(&mut self, target: String, stats: eggsec::stress::StressStats) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};

        self.core.results_view.clear();

        self.core
            .results_view
            .add_line(Line::from(vec![Span::styled(
                "Stress Test Results",
                Style::default().fg(tc!(accent)),
            )]));
        self.core.results_view.add_line(Line::from(""));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Target: ", Style::default().fg(tc!(info))),
            Span::raw(target),
        ]));

        let pps = if stats.duration_ms > 0 {
            (stats.packets_sent * 1000) / stats.duration_ms
        } else {
            0
        };

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Packets: ", Style::default().fg(tc!(info))),
            Span::raw(format!(
                "{} sent, {} errors",
                stats.packets_sent, stats.errors
            )),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Rate: ", Style::default().fg(tc!(info))),
            Span::raw(format!("{} pps", pps)),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(tc!(info))),
            Span::raw(format!("{} ms", stats.duration_ms)),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Data Sent: ", Style::default().fg(tc!(info))),
            Span::raw(format!("{} bytes", stats.bytes_sent)),
        ]));
    }

    fn update_results_view(&mut self, results: &LoadTestResults) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};

        self.core.prepare_results();

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

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Target: ", Style::default().fg(tc!(accent))),
            Span::raw(target_url),
        ]));
        self.core.results_view.add_line(Line::from(""));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Requests: ", Style::default().fg(tc!(info))),
            Span::raw(format!(
                "{} total, {} success, {} failed",
                total_requests, successful_requests, failed_requests
            )),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("RPS: ", Style::default().fg(tc!(info))),
            Span::raw(format!("{:.2} req/s", rps)),
        ]));

        self.core.results_view.add_line(Line::from(""));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Latency: ", Style::default().fg(tc!(success))),
            Span::raw(format!(
                "min={:.2}ms, max={:.2}ms, mean={:.2}ms",
                min_ms, max_ms, mean_ms
            )),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::raw("         "),
            Span::raw(format!(
                "p50={:.2}ms, p90={:.2}ms, p95={:.2}ms, p99={:.2}ms",
                p50, p90, p95, p99
            )),
        ]));

        let status_codes = results.status_codes.clone();
        if !status_codes.is_empty() {
            self.core.results_view.add_line(Line::from(""));
            self.core.results_view.add_line(Line::from(Span::styled(
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
                self.core.results_view.add_line(Line::from(vec![
                    Span::styled(format!("  {}:", code), Style::default().fg(color)),
                    Span::raw(format!(" {}", count)),
                ]));
            }
        }

        let errors = results.errors.clone();
        if !errors.is_empty() {
            self.core.results_view.add_line(Line::from(""));
            self.core.results_view.add_line(Line::from(Span::styled(
                "Errors:",
                Style::default().fg(tc!(error)),
            )));
            for error in &errors {
                self.core
                    .results_view
                    .add_line(Line::from(format!("  - {}", error)));
            }
        }
    }

    pub fn start(&mut self) {
        start_scan(&mut self.core);
        self.results = None;
    }

    pub fn stop(&mut self) {
        self.core.stop();
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }

    pub fn scroll_results_up(&mut self) {
        self.core.scroll_results_up();
    }

    pub fn scroll_results_down(&mut self) {
        self.core.scroll_results_down();
    }
}

impl Default for LoadTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for LoadTab {
    tab_state_boilerplate!(LoadTab, core: core);

    fn has_selector_open(&self) -> bool {
        self.test_type_selector.is_open()
    }

    fn reset(&mut self) {
        self.core.reset_all();
        if self.core.inputs.fields.len() > 4 {
            if let Some(field) = self.core.inputs.fields.get_mut(1) {
                field.value = "GET".to_string();
                field.cursor_pos = 3;
            }
            if let Some(field) = self.core.inputs.fields.get_mut(2) {
                field.value = "100".to_string();
                field.cursor_pos = 3;
            }
            if let Some(field) = self.core.inputs.fields.get_mut(3) {
                field.value = "10".to_string();
                field.cursor_pos = 2;
            }
            if let Some(field) = self.core.inputs.fields.get_mut(4) {
                field.value = "30".to_string();
                field.cursor_pos = 2;
            }
            if let Some(field) = self.core.inputs.fields.get_mut(5) {
                field.value.clear();
            }
            if let Some(field) = self.core.inputs.fields.get_mut(6) {
                field.value.clear();
            }
        }
        self.test_type_selector.select(0);
        self.test_type_selector.blur();
        self.core.inputs.blur();
        self.focus_area = StandardFocusAreaSelector::Selector;
    }
}

impl TabRender for LoadTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let num_fields = self.core.inputs.fields.len().max(1) as u16;
        let input_height = (num_fields * 3 + 2).min(area.height.saturating_sub(8));
        let results_height = area
            .height
            .saturating_sub(6)
            .saturating_sub(input_height)
            .max(3);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),            // Selector
                Constraint::Length(input_height), // Inputs
                Constraint::Min(results_height),  // Results
            ])
            .split(area);

        if let Some(selector_area) = chunks.first() {
            self.test_type_selector.render(f, *selector_area);

            if let Some(dropdown) = self
                .test_type_selector
                .dropdown_info(*selector_area, f.area().height)
            {
                dropdown.render(f);
            }
        }

        if let Some(input_area) = chunks.get(1) {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .title(" Load Test Configuration ")
                .border_style(crate::tabs::core::focus_border_style(
                    self.focus_area == StandardFocusAreaSelector::Inputs,
                ));
            let input_inner = input_block.inner(*input_area);
            f.render_widget(input_block, *input_area);

            let num_fields = self.core.inputs.fields.len().max(1);
            let constraints: Vec<Constraint> =
                (0..num_fields).map(|_| Constraint::Length(3)).collect();

            let input_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(input_inner);

            render_input_fields(f, &input_chunks, &self.core.inputs, insert_mode);
        }

        if let Some(results_area) = chunks.get(2) {
            render_results_area(
                f,
                *results_area,
                &self.core.state,
                &self.core.error,
                &self.core.results_view,
                &self.core.progress,
                "Results",
                "Results will appear here after running",
            );
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

        let selector_area = *chunks.first().unwrap_or(&area);

        if let Some(dropdown) = self
            .test_type_selector
            .dropdown_info(selector_area, f.area().height)
        {
            dropdown.render(f);
        }
    }
}

impl TabInput for LoadTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            StandardFocusAreaSelector::Selector => {
                self.test_type_selector.blur();
                self.core.inputs.focus(0);
                StandardFocusAreaSelector::Inputs
            }
            StandardFocusAreaSelector::Inputs => {
                self.core.inputs.blur();
                StandardFocusAreaSelector::Results
            }
            StandardFocusAreaSelector::Results => {
                self.test_type_selector.focus();
                StandardFocusAreaSelector::Selector
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            StandardFocusAreaSelector::Selector => {
                self.test_type_selector.blur();
                StandardFocusAreaSelector::Results
            }
            StandardFocusAreaSelector::Inputs => {
                self.core.inputs.blur();
                self.test_type_selector.focus();
                StandardFocusAreaSelector::Selector
            }
            StandardFocusAreaSelector::Results => {
                self.core.inputs.focus(0);
                StandardFocusAreaSelector::Inputs
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            if self.focus_area == StandardFocusAreaSelector::Selector {
                self.test_type_selector.handle_char(c);
            } else if self.focus_area == StandardFocusAreaSelector::Inputs {
                self.core.inputs.insert(c);
            }
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            if self.focus_area == StandardFocusAreaSelector::Selector {
                self.test_type_selector.handle_backspace();
            } else if self.focus_area == StandardFocusAreaSelector::Inputs {
                self.core.inputs.backspace();
            }
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == StandardFocusAreaSelector::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == StandardFocusAreaSelector::Inputs {
            self.core.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == StandardFocusAreaSelector::Inputs {
            self.core.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == StandardFocusAreaSelector::Inputs {
            self.core.inputs.move_home();
        } else if self.focus_area == StandardFocusAreaSelector::Results {
            self.core.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == StandardFocusAreaSelector::Inputs {
            self.core.inputs.move_end();
        } else if self.focus_area == StandardFocusAreaSelector::Results {
            self.core.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.core.inputs.blur();
        self.focus_area = StandardFocusAreaSelector::Selector;
        self.test_type_selector.focus();
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.core.inputs.blur();
        self.focus_area = StandardFocusAreaSelector::Results;
    }

    fn handle_enter(&mut self) {
        if self.focus_area == StandardFocusAreaSelector::Results {
            return;
        }

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
        if self.core.inputs.is_focused() {
            self.core.inputs.blur();
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
        self.core.inputs.blur();
        self.focus_area = StandardFocusAreaSelector::Selector;
    }

    fn handle_up(&mut self) {
        if self.focus_area == StandardFocusAreaSelector::Selector {
            if self.test_type_selector.is_open() {
                self.test_type_selector.move_prev();
            }
        } else if self.focus_area == StandardFocusAreaSelector::Inputs {
            if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
                self.scroll_results_up();
            } else {
                self.core.inputs.focus_prev();
            }
        } else if self.focus_area == StandardFocusAreaSelector::Results {
            self.scroll_results_up();
        }
    }

    fn handle_down(&mut self) {
        if self.focus_area == StandardFocusAreaSelector::Selector {
            if self.test_type_selector.is_open() {
                self.test_type_selector.move_next();
            }
        } else if self.focus_area == StandardFocusAreaSelector::Inputs {
            if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
                self.scroll_results_down();
            } else {
                self.core.inputs.focus_next();
            }
        } else if self.focus_area == StandardFocusAreaSelector::Results {
            self.scroll_results_down();
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
        } else if self.focus_area != StandardFocusAreaSelector::Results
            && self.core.inputs.is_focused()
        {
            self.core.inputs.move_left()
        } else {
            false
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
        } else if self.focus_area != StandardFocusAreaSelector::Results
            && self.core.inputs.is_focused()
        {
            self.core.inputs.move_right()
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        self.test_type_selector.is_focused() || self.core.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.test_type_selector.is_focused() {
            if self.test_type_selector.is_open() {
                self.test_type_selector.items.is_empty() || self.test_type_selector.selected == 0
            } else {
                true
            }
        } else if self.core.inputs.is_focused() {
            self.core.inputs.is_at_left_edge()
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
        } else if self.core.inputs.is_focused() {
            self.core.inputs.is_at_right_edge()
        } else {
            true
        }
    }

    fn page_up(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.core.results_view.page_up(page_size);
    }

    fn page_down(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.core.results_view.page_down(page_size);
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        match self.focus_area {
            StandardFocusAreaSelector::Inputs => self.core.inputs.get_focused_value(),
            StandardFocusAreaSelector::Results => Some(self.core.results_view.get_content()),
            _ => None,
        }
    }

    fn primary_target(&self) -> Option<String> {
        Some(self.target().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab() -> LoadTab {
        LoadTab::new()
    }

    #[test]
    fn test_enter_in_inputs_focused_blurs_does_not_start() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusAreaSelector::Inputs;
        tab.core.inputs.focus(0);
        assert!(tab.core.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.core.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_in_selector_open_confirms_does_not_start() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusAreaSelector::Selector;
        tab.test_type_selector.focus();
        tab.test_type_selector.open();
        assert!(tab.test_type_selector.is_open());
        tab.handle_enter();
        assert!(!tab.test_type_selector.is_open());
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_in_results_no_op() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusAreaSelector::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }
}
