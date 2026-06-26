use crate::components::{InputField, InputGroup, Selector, SelectorItem};
use crate::tabs::core::{
    focus_border_style, render_input_fields, render_results_area, start_scan,
    StandardFocusAreaSelector, TabCore,
};
use crate::tabs::{TabInput, TabRender, TabState};
use crate::{tab_input_areas, tab_state_boilerplate, tc};
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
    pub focus_area: StandardFocusAreaSelector,
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
            focus_area: StandardFocusAreaSelector::Inputs,
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
        let view = self.core.prepare_results();

        view.add_line(Line::from(Span::styled(
            format!("Stress Test Complete: {}", results.target),
            Style::default().fg(tc!(success)),
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(format!("Type: {}", results.stress_type)));
        view.add_line(Line::from(format!("Duration: {}ms", results.duration_ms)));
        view.add_line(Line::from(""));
        view.add_line(Line::from(Span::styled(
            "Statistics:",
            Style::default().fg(tc!(warning)),
        )));
        view.add_line(Line::from(format!(
            "  Packets Sent: {}",
            results.packets_sent
        )));
        view.add_line(Line::from(format!("  Bytes Sent: {}", results.bytes_sent)));
        view.add_line(Line::from(format!(
            "  Packets/sec: {:.2}",
            results.packets_per_second
        )));
        view.add_line(Line::from(format!("  Errors: {}", results.errors)));

        if results.responses_received > 0 {
            view.add_line(Line::from(""));
            view.add_line(Line::from(Span::styled(
                "Response Statistics:",
                Style::default().fg(tc!(warning)),
            )));
            view.add_line(Line::from(format!(
                "  Responses Received: {}",
                results.responses_received
            )));
            view.add_line(Line::from(format!(
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

    fn has_selector_open(&self) -> bool {
        self.type_selector.is_open()
    }

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
        self.focus_area = StandardFocusAreaSelector::Inputs;
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
        let input_block = Block::default()
            .title(" Stress Test Configuration ")
            .borders(Borders::ALL)
            .border_style(focus_border_style(
                self.focus_area == StandardFocusAreaSelector::Inputs,
            ));
        let input_area = chunks.first().copied().unwrap_or(area);
        let input_inner = input_block.inner(input_area);
        f.render_widget(input_block, input_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_inner);

        render_input_fields(f, &input_chunks, &self.core.inputs, insert_mode);

        // Type selector
        let mut selector = self.type_selector.clone();
        selector.focused = self.focus_area == StandardFocusAreaSelector::Selector;
        if let Some(chunk) = chunks.get(1) {
            selector.render(f, *chunk);
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
}

impl TabInput for StressTab {
    tab_input_areas!(
        StressTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusAreaSelector::Inputs,
        Options: StandardFocusAreaSelector::Selector,
        Results: StandardFocusAreaSelector::Results
    );

    fn handle_enter(&mut self) {
        if self.focus_area == StandardFocusAreaSelector::Results {
            return;
        }

        if self.is_running() {
            self.stop();
            return;
        }
        match self.focus_area {
            StandardFocusAreaSelector::Inputs => {
                self.core.inputs.blur();
                self.focus_area = StandardFocusAreaSelector::Selector;
                self.type_selector.open();
            }
            StandardFocusAreaSelector::Selector => {
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
            StandardFocusAreaSelector::Results => {}
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
        self.focus_area = StandardFocusAreaSelector::Inputs;
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
        tab.focus_area = StandardFocusAreaSelector::Inputs;
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
        tab.focus_area = StandardFocusAreaSelector::Selector;
        tab.type_selector.open();
        assert!(tab.type_selector.is_open());
        tab.handle_enter();
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
