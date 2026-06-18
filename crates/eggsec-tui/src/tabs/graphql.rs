use crate::components::{Checkbox, InputField};
use crate::tabs::core::{field_as, render_results_area, start_scan, StandardFocusArea, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_boilerplate, tab_state_boilerplate, tc};
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
    pub checkbox_focus_index: usize,
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
            checkbox_focus_index: 0,
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
        self.core.state = AppState::Completed;
        self.core.results_view.clear();

        self.core.results_view.add_line(Line::from(Span::styled(
            format!("GraphQL Security Test Complete: {}", results.target),
            Style::default().fg(tc!(success)),
        )));
        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(Span::styled(
            "Findings:",
            Style::default().fg(tc!(warning)),
        )));

        if results.introspection_enabled {
            self.core.results_view.add_line(Line::from(Span::styled(
                "  [!] Introspection is ENABLED - Schema exposed",
                Style::default().fg(tc!(error)),
            )));
        } else {
            self.core
                .results_view
                .add_line(Line::from(Span::raw("  [+] Introspection is disabled")));
        }

        if results.depth_limit_bypassed {
            self.core.results_view.add_line(Line::from(Span::styled(
                "  [!] Depth limit bypass detected",
                Style::default().fg(tc!(error)),
            )));
        }

        if results.alias_overload_vulnerable {
            self.core.results_view.add_line(Line::from(Span::styled(
                "  [!] Alias overload vulnerability detected",
                Style::default().fg(tc!(error)),
            )));
        }

        if !results.injection_findings.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                format!("  Injection Findings: {}", results.injection_findings.len()),
                Style::default().fg(tc!(warning)),
            )));
        }

        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(format!(
            "Requests: {} | Errors: {} | Duration: {}ms",
            results.total_requests, results.errors, results.duration_ms
        )));
    }
}

#[derive(Clone, Debug)]
pub struct GraphQlResults {
    pub target: String,
    pub introspection_enabled: bool,
    pub depth_limit_bypassed: bool,
    pub alias_overload_vulnerable: bool,
    pub injection_findings: Vec<String>,
    pub total_requests: usize,
    pub errors: usize,
    pub duration_ms: u64,
}

impl TabState for GraphQlTab {
    tab_state_boilerplate!(GraphQlTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        if self.core.inputs.fields.len() > 1 {
            self.core.inputs.fields[1].value = "10".to_string();
        }
        if self.core.inputs.fields.len() > 2 {
            self.core.inputs.fields[2].value = "15".to_string();
        }
        self.core.inputs.blur();
        self.focus_area = StandardFocusArea::Inputs;
        self.checkbox_focus_index = 0;
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
                Constraint::Length(12),
                Constraint::Length(6),
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
            .title(" GraphQL Configuration ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == StandardFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        f.render_widget(input_block, chunks.first().copied().unwrap_or(area));

        for (i, field) in self.core.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        // Options
        let options_block = Block::default()
            .title(" Test Options ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == StandardFocusArea::Options {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );

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
        if let (Some(c0), Some(c1), Some(c2), Some(c3)) = (
            options_chunks.first(),
            options_chunks.get(1),
            options_chunks.get(2),
            options_chunks.get(3),
        ) {
            self.introspection_checkbox.render(f, *c0);
            self.inject_checkbox.render(f, *c1);
            self.depth_bypass_checkbox.render(f, *c2);
            self.alias_overload_checkbox.render(f, *c3);
        }

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
    tab_input_boilerplate!(
        GraphQlTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea::Inputs,
        Results: StandardFocusArea::Results
    );

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_focus_next(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                StandardFocusArea::Inputs => {
                    self.core.inputs.blur();
                    StandardFocusArea::Options
                }
                StandardFocusArea::Options => StandardFocusArea::Results,
                StandardFocusArea::Results => {
                    self.core.inputs.focus(0);
                    StandardFocusArea::Inputs
                }
            };
        }
    }

    fn handle_focus_prev(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                StandardFocusArea::Inputs => {
                    self.core.inputs.blur();
                    StandardFocusArea::Results
                }
                StandardFocusArea::Options => {
                    self.core.inputs.focus(0);
                    StandardFocusArea::Inputs
                }
                StandardFocusArea::Results => StandardFocusArea::Options,
            };
        }
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                StandardFocusArea::Inputs => self.core.inputs.focus_prev(),
                StandardFocusArea::Results => self.core.scroll_results_up(),
                _ => {}
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                StandardFocusArea::Inputs => self.core.inputs.focus_next(),
                StandardFocusArea::Results => self.core.scroll_results_down(),
                _ => {}
            }
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == StandardFocusArea::Inputs && self.core.inputs.is_focused()
    }

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
            let checkboxes = [
                &mut self.introspection_checkbox,
                &mut self.inject_checkbox,
                &mut self.depth_bypass_checkbox,
                &mut self.alias_overload_checkbox,
            ];
            let idx = self.checkbox_focus_index % checkboxes.len();
            checkboxes[idx].toggle();
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
        self.checkbox_focus_index = 0;
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        match self.focus_area {
            StandardFocusArea::Inputs => self.core.inputs.move_left(),
            StandardFocusArea::Options => {
                if self.checkbox_focus_index > 0 {
                    self.checkbox_focus_index -= 1;
                }
                true
            }
            _ => false,
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        match self.focus_area {
            StandardFocusArea::Inputs => self.core.inputs.move_right(),
            StandardFocusArea::Options => {
                let max_idx = 3;
                if self.checkbox_focus_index < max_idx {
                    self.checkbox_focus_index += 1;
                }
                true
            }
            _ => false,
        }
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            StandardFocusArea::Inputs => !self.core.inputs.can_move_left(),
            StandardFocusArea::Options => self.checkbox_focus_index == 0,
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            StandardFocusArea::Inputs => !self.core.inputs.can_move_right(),
            StandardFocusArea::Options => self.checkbox_focus_index >= 3,
            _ => true,
        }
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
}
