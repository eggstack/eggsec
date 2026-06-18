use crate::components::{
    empty_state_paragraph, InputField, InputGroup, Selector, SelectorItem,
};
use crate::tabs::core::{focus_border_style, start_scan, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_boilerplate, tab_state_boilerplate, tc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StressType {
    Http,
    Syn,
    Udp,
    Tcp,
    Icmp,
}

pub struct StressTab {
    pub core: TabCore,
    pub type_selector: Selector,
    pub focus_area: StressFocusArea,
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
            core: TabCore::new("Stress testing...", "Stress Test Results").with_inputs(inputs),
            type_selector,
            focus_area: StressFocusArea::Inputs,
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn rate(&self) -> u64 {
        self.core
            .inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(100)
    }

    pub fn duration(&self) -> u64 {
        self.core
            .inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(30)
    }

    pub fn concurrency(&self) -> usize {
        self.core
            .inputs
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
        self.core.state = AppState::Completed;
        self.core.results_view.clear();

        self.core.results_view.add_line(Line::from(Span::styled(
            format!("Stress Test Complete: {}", results.target),
            Style::default().fg(tc!(success)),
        )));
        self.core.results_view.add_line(Line::from(""));
        self.core
            .results_view
            .add_line(Line::from(format!("Type: {}", results.stress_type)));
        self.core
            .results_view
            .add_line(Line::from(format!("Duration: {}ms", results.duration_ms)));
        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(Span::styled(
            "Statistics:",
            Style::default().fg(tc!(warning)),
        )));
        self.core.results_view.add_line(Line::from(format!(
            "  Packets Sent: {}",
            results.packets_sent
        )));
        self.core
            .results_view
            .add_line(Line::from(format!("  Bytes Sent: {}", results.bytes_sent)));
        self.core.results_view.add_line(Line::from(format!(
            "  Packets/sec: {:.2}",
            results.packets_per_second
        )));
        self.core
            .results_view
            .add_line(Line::from(format!("  Errors: {}", results.errors)));

        if results.responses_received > 0 {
            self.core.results_view.add_line(Line::from(""));
            self.core.results_view.add_line(Line::from(Span::styled(
                "Response Statistics:",
                Style::default().fg(tc!(warning)),
            )));
            self.core.results_view.add_line(Line::from(format!(
                "  Responses Received: {}",
                results.responses_received
            )));
            self.core.results_view.add_line(Line::from(format!(
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
    tab_state_boilerplate!(StressTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        if let Some(field) = self.core.inputs.fields.get_mut(1) {
            field.value = "100".to_string();
            field.cursor_pos = 3;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(2) {
            field.value = "30".to_string();
            field.cursor_pos = 2;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(3) {
            field.value = "10".to_string();
            field.cursor_pos = 2;
        }
        self.type_selector.select(0);
        self.type_selector.cancel();
        self.type_selector.blur();
        self.core.inputs.blur();
        self.focus_area = StressFocusArea::Inputs;
    }
}

impl TabRender for StressTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref err) = self.core.error {
            crate::tabs::core::render_error_block(f, area, "Stress - Error", err);
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
            .split(chunks.first().copied().unwrap_or(area));

        let input_block = Block::default()
            .title(" Stress Test Configuration ")
            .borders(Borders::ALL)
            .border_style(focus_border_style(
                self.focus_area == StressFocusArea::Inputs,
            ));
        if let Some(chunk) = chunks.first() {
            f.render_widget(input_block, *chunk);
        }

        for (i, field) in self.core.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        // Type selector
        let mut selector = self.type_selector.clone();
        selector.focused = self.focus_area == StressFocusArea::TypeSelector;
        if let Some(chunk) = chunks.get(1) {
            selector.render(f, *chunk);
        }

        // Results
        if self.core.results_view.is_empty() {
            let placeholder =
                empty_state_paragraph("Results", "Results will appear here after running");
            if let Some(chunk) = chunks.get(2) {
                f.render_widget(placeholder, *chunk);
            }
        } else {
            if let Some(chunk) = chunks.get(2) {
                self.core.results_view.render(f, *chunk, None);
            }
        }

        // Progress bar if running
        if self.core.state == AppState::Running {
            let progress_area = Rect {
                x: area.x,
                y: area.y + area.height - 1,
                width: area.width,
                height: 1,
            };
            self.core.progress.render(f, progress_area);
        }
    }
}

impl TabInput for StressTab {
    tab_input_boilerplate!(
        StressTab,
        core: core,
        focus: focus_area,
        Inputs: StressFocusArea::Inputs,
        Results: StressFocusArea::Results
    );

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            StressFocusArea::Inputs => {
                self.core.inputs.blur();
                StressFocusArea::TypeSelector
            }
            StressFocusArea::TypeSelector => {
                self.type_selector.blur();
                self.type_selector.cancel();
                StressFocusArea::Results
            }
            StressFocusArea::Results => {
                self.core.inputs.focus(0);
                StressFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            StressFocusArea::Inputs => {
                self.core.inputs.blur();
                StressFocusArea::Results
            }
            StressFocusArea::TypeSelector => {
                self.type_selector.blur();
                self.core.inputs.focus(0);
                StressFocusArea::Inputs
            }
            StressFocusArea::Results => {
                self.type_selector.focus();
                StressFocusArea::TypeSelector
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == StressFocusArea::Inputs {
            self.core.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == StressFocusArea::Inputs {
            self.core.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == StressFocusArea::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_enter(&mut self) {
        if self.focus_area == StressFocusArea::Results {
            return;
        }

        if self.is_running() {
            self.stop();
            return;
        }
        match self.focus_area {
            StressFocusArea::Inputs => {
                self.core.inputs.blur();
                self.focus_area = StressFocusArea::TypeSelector;
                self.type_selector.open();
            }
            StressFocusArea::TypeSelector => {
                if self.type_selector.is_open() {
                    if self.type_selector.confirm().is_some() {
                        self.type_selector.close();
                        self.start();
                    } else {
                        tracing::warn!("Failed to confirm stress type selector");
                    }
                } else {
                    self.type_selector.open();
                }
            }
            StressFocusArea::Results => {}
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.type_selector.is_open() {
            self.type_selector.cancel();
            return;
        }
        self.core.inputs.blur();
        self.type_selector.blur();
        self.focus_area = StressFocusArea::Inputs;
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            StressFocusArea::Inputs => {
                self.core.inputs.focus_prev();
            }
            StressFocusArea::TypeSelector => {
                self.type_selector.handle_up();
            }
            StressFocusArea::Results => {
                self.core.results_view.scroll_up(1);
            }
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            StressFocusArea::Inputs => {
                self.core.inputs.focus_next();
            }
            StressFocusArea::TypeSelector => {
                self.type_selector.handle_down();
            }
            StressFocusArea::Results => {
                self.core.results_view.scroll_down(1);
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() {
            match self.focus_area {
                StressFocusArea::Inputs => self.core.inputs.move_left(),
                _ => false,
            }
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() {
            match self.focus_area {
                StressFocusArea::Inputs => self.core.inputs.move_right(),
                _ => false,
            }
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == StressFocusArea::Inputs && self.core.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            StressFocusArea::Inputs => self.core.inputs.is_at_left_edge(),
            StressFocusArea::TypeSelector => {
                self.type_selector.items.is_empty() || self.type_selector.selected == 0
            }
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            StressFocusArea::Inputs => self.core.inputs.is_at_right_edge(),
            StressFocusArea::TypeSelector => {
                self.type_selector.items.is_empty()
                    || self.type_selector.selected
                        >= self.type_selector.items.len().saturating_sub(1)
            }
            _ => true,
        }
    }
}

impl StressTab {
    pub fn start(&mut self) {
        start_scan(&mut self.core);
    }

    pub fn stop(&mut self) {
        self.core.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab() -> StressTab {
        StressTab::new()
    }

    #[test]
    fn test_enter_in_inputs_blurs_opens_selector() {
        let mut tab = create_test_tab();
        tab.focus_area = StressFocusArea::Inputs;
        tab.core.inputs.focus(0);
        assert!(tab.core.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.core.inputs.is_focused());
        assert!(!tab.is_running());
        assert!(tab.type_selector.is_open());
    }

    #[test]
    fn test_enter_in_type_selector_opens_does_not_start() {
        let mut tab = create_test_tab();
        tab.focus_area = StressFocusArea::TypeSelector;
        tab.type_selector.open();
        assert!(tab.type_selector.is_open());
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_in_results_no_op() {
        let mut tab = create_test_tab();
        tab.focus_area = StressFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }
}
