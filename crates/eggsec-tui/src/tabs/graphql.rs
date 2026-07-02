use crate::components::{Checkbox, InputField};
use crate::tabs::core::{
    field_as, render_input_fields, render_results_area, start_scan, StandardFocusArea, TabCore,
};
use crate::tabs::{TabInput, TabRender, TabState};
use crate::{tab_input_areas, tab_state_boilerplate, tc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct GraphQlTab {
    pub core: TabCore,
    pub introspection_checkbox: Checkbox,
    pub inject_checkbox: Checkbox,
    pub depth_bypass_checkbox: Checkbox,
    pub alias_overload_checkbox: Checkbox,
    pub focus_area: StandardFocusArea,
    pub focused_checkbox_index: usize,
}

impl Default for GraphQlTab {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphQlTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("GraphQL Endpoint URL"))
            .add(InputField::new("Concurrency").with_value("10"))
            .add(InputField::new("Timeout (s)").with_value("15"));

        Self {
            core: TabCore::new("Testing GraphQL...", "Results").with_inputs(inputs),
            introspection_checkbox: Checkbox::new("Introspection Tests").checked(true),
            inject_checkbox: Checkbox::new("Query Injection Tests").checked(true),
            depth_bypass_checkbox: Checkbox::new("Depth Limit Bypass").checked(true),
            alias_overload_checkbox: Checkbox::new("Alias Overload Tests").checked(true),
            focus_area: StandardFocusArea::Inputs,
            focused_checkbox_index: 0,
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn concurrency(&self) -> usize {
        field_as(&self.core, 1, 10)
    }

    pub fn timeout(&self) -> u64 {
        field_as(&self.core, 2, 15)
    }

    pub fn start(&mut self) {
        if start_scan(&mut self.core) {
            self.core.progress.total = 100;
        }
    }

    pub fn set_results(&mut self, results: GraphQlResults) {
        let view = self.core.prepare_results();

        view.add_line(Line::from(Span::styled(
            format!("GraphQL Security Test Complete: {}", results.target),
            Style::default().fg(tc!(success)),
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(Span::styled(
            "Findings:",
            Style::default().fg(tc!(warning)),
        )));

        if results.introspection_enabled {
            view.add_line(Line::from(Span::styled(
                "  [!] Introspection is ENABLED - Schema exposed",
                Style::default().fg(tc!(error)),
            )));
        } else {
            view.add_line(Line::from(Span::raw("  [+] Introspection is disabled")));
        }

        if results.depth_limit_bypassed {
            view.add_line(Line::from(Span::styled(
                "  [!] Depth limit bypass detected",
                Style::default().fg(tc!(error)),
            )));
        }

        if results.alias_overload_vulnerable {
            view.add_line(Line::from(Span::styled(
                "  [!] Alias overload vulnerability detected",
                Style::default().fg(tc!(error)),
            )));
        }

        if !results.injection_findings.is_empty() {
            view.add_line(Line::from(Span::styled(
                format!("  Injection Findings: {}", results.injection_findings.len()),
                Style::default().fg(tc!(warning)),
            )));
        }

        view.add_line(Line::from(""));
        view.add_line(Line::from(format!(
            "Requests: {} | Errors: {} | Duration: {}ms",
            results.total_requests, results.errors, results.duration_ms
        )));
    }
}

pub use eggsec::dispatch::GraphQlResults;

impl TabState for GraphQlTab {
    tab_state_boilerplate!(GraphQlTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.core.inputs.set_field_value("Concurrency", "10");
        self.core.inputs.set_field_value("Timeout (s)", "15");
        self.core.inputs.blur();
        self.focus_area = StandardFocusArea::Inputs;
        self.focused_checkbox_index = 0;
        self.inject_checkbox.checked = true;
        self.introspection_checkbox.checked = true;
        self.depth_bypass_checkbox.checked = true;
        self.alias_overload_checkbox.checked = true;
    }
}

impl TabRender for GraphQlTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref error) = self.core.error {
            let msg = error.message();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("GraphQL - Error")
                .border_style(Style::default().fg(tc!(error)));
            let paragraph = Paragraph::new(msg)
                .style(Style::default().fg(tc!(error)))
                .block(block);
            f.render_widget(paragraph, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(14),
                Constraint::Length(6),
                Constraint::Min(5),
            ])
            .split(area);

        // Input fields
        let input_block = Block::default()
            .title(" GraphQL Configuration ")
            .borders(Borders::ALL)
            .border_style(crate::tabs::core::focus_border_style(
                self.focus_area == StandardFocusArea::Inputs,
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

        // Options
        let options_block = Block::default()
            .title(" Test Options ")
            .borders(Borders::ALL)
            .border_style(crate::tabs::core::focus_border_style(
                self.focus_area == StandardFocusArea::Options,
            ));

        let options_area = chunks.get(1).copied().unwrap_or(area);
        let options_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(options_block.inner(options_area));

        f.render_widget(options_block, options_area);
        crate::tabs::core::render_checkbox_row(
            f,
            &options_chunks,
            &[
                &self.introspection_checkbox,
                &self.inject_checkbox,
                &self.depth_bypass_checkbox,
                &self.alias_overload_checkbox,
            ],
            self.focused_checkbox_index,
            self.focus_area == StandardFocusArea::Options,
        );

        // Results
        let results_area = chunks.get(2).copied().unwrap_or(area);
        render_results_area(
            f,
            results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Results",
            "Results will appear here after running",
        );
    }
}

impl TabInput for GraphQlTab {
    tab_input_areas!(
        GraphQlTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea::Inputs,
        Options: StandardFocusArea::Options,
        Results: StandardFocusArea::Results
    );

    fn handle_enter(&mut self) {
        let running = self.is_running();
        let inputs_focused = self.core.inputs.is_focused();
        crate::tabs::core::handle_enter_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
            running,
            inputs_focused,
            |_core| false,
        );
        if self.focus_area == StandardFocusArea::Options && !self.is_running() {
            let mut checkboxes = [
                &mut self.introspection_checkbox,
                &mut self.inject_checkbox,
                &mut self.depth_bypass_checkbox,
                &mut self.alias_overload_checkbox,
            ];
            crate::tabs::core::toggle_focused_checkbox(
                &mut checkboxes,
                &mut self.focused_checkbox_index,
            );
        }
    }

    fn handle_escape(&mut self) {
        let new_area = crate::tabs::core::handle_escape_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
        );
        self.focus_area = new_area;
        self.focused_checkbox_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab() -> GraphQlTab {
        GraphQlTab::new()
    }

    #[test]
    fn test_handle_enter_results_focus_no_op() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    #[test]
    fn test_handle_enter_options_toggles_checkbox() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        let before = tab.introspection_checkbox.checked;
        tab.handle_enter();
        assert_eq!(tab.introspection_checkbox.checked, !before);
        assert!(!tab.is_running());
    }

    #[test]
    fn test_handle_enter_inputs_focused_blurs() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Inputs;
        tab.core.inputs.focus(0);
        assert!(tab.core.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.core.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn test_handle_escape_resets_checkbox_index() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        tab.focused_checkbox_index = 3;
        tab.handle_escape();
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);
        assert_eq!(tab.focused_checkbox_index, 0);
    }

    #[test]
    fn test_handle_left_right_no_op_in_options() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        tab.focused_checkbox_index = 0;

        tab.handle_right();
        assert_eq!(tab.focused_checkbox_index, 0);

        tab.handle_left();
        assert_eq!(tab.focused_checkbox_index, 0);
    }

    #[test]
    fn test_checkbox_toggle_via_enter() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        tab.focused_checkbox_index = 0;
        assert!(tab.introspection_checkbox.checked);

        tab.handle_enter();
        assert!(!tab.introspection_checkbox.checked);

        tab.handle_enter();
        assert!(tab.introspection_checkbox.checked);
    }
}
