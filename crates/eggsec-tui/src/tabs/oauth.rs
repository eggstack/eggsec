use crate::components::{Checkbox, InputField};
use crate::tabs::core::{
    field_as, render_input_fields, render_results_area, start_scan, StandardFocusArea, TabCore,
};
use crate::tabs::{TabInput, TabRender, TabState};
use crate::{tab_input_boilerplate, tab_state_boilerplate, tc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct OAuthTab {
    pub core: TabCore,
    pub redirect_test_checkbox: Checkbox,
    pub scope_test_checkbox: Checkbox,
    pub state_test_checkbox: Checkbox,
    pub grant_test_checkbox: Checkbox,
    pub focus_area: StandardFocusArea,
    pub focused_checkbox_index: usize,
}

impl Default for OAuthTab {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("OAuth Authorization Endpoint URL"))
            .add(InputField::new("Client ID (optional)"))
            .add(InputField::new("Redirect URI (optional)"))
            .add(InputField::new("Concurrency").with_value("10"))
            .add(InputField::new("Timeout (s)").with_value("15"));

        Self {
            core: TabCore::new("Testing OAuth...", "Results").with_inputs(inputs),
            redirect_test_checkbox: Checkbox::new("Redirect URI Validation").checked(true),
            scope_test_checkbox: Checkbox::new("Scope Escalation Tests").checked(true),
            state_test_checkbox: Checkbox::new("State Parameter Tests").checked(true),
            grant_test_checkbox: Checkbox::new("Grant Type Tests").checked(true),
            focus_area: StandardFocusArea::Inputs,
            focused_checkbox_index: 0,
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn client_id(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn redirect_uri(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn concurrency(&self) -> usize {
        field_as(&self.core, 3, 10)
    }

    pub fn timeout(&self) -> u64 {
        field_as(&self.core, 4, 15)
    }

    pub fn start(&mut self) {
        if start_scan(&mut self.core) {
            self.core.progress.total = 100;
        }
    }

    pub fn set_results(&mut self, results: OAuthResults) {
        let view = self.core.prepare_results();

        view.add_line(Line::from(Span::styled(
            format!("OAuth/OIDC Security Test Complete: {}", results.target),
            Style::default().fg(tc!(success)),
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(Span::styled(
            "Findings:",
            Style::default().fg(tc!(warning)),
        )));

        if !results.redirect_vulnerabilities.is_empty() {
            view.add_line(Line::from(Span::styled(
                format!(
                    "  [!] Redirect URI Issues: {}",
                    results.redirect_vulnerabilities.len()
                ),
                Style::default().fg(tc!(error)),
            )));
            for vuln in &results.redirect_vulnerabilities {
                view.add_line(Line::from(format!("    - {}", vuln)));
            }
        } else {
            view.add_line(Line::from(Span::raw(
                "  [+] Redirect URI validation appears secure",
            )));
        }

        if !results.scope_vulnerabilities.is_empty() {
            view.add_line(Line::from(Span::styled(
                format!(
                    "  [!] Scope Escalation Issues: {}",
                    results.scope_vulnerabilities.len()
                ),
                Style::default().fg(tc!(error)),
            )));
        }

        if !results.state_vulnerabilities.is_empty() {
            view.add_line(Line::from(Span::styled(
                format!(
                    "  [!] State Parameter Issues: {}",
                    results.state_vulnerabilities.len()
                ),
                Style::default().fg(tc!(error)),
            )));
        }

        if !results.grant_vulnerabilities.is_empty() {
            view.add_line(Line::from(Span::styled(
                format!(
                    "  [!] Grant Type Issues: {}",
                    results.grant_vulnerabilities.len()
                ),
                Style::default().fg(tc!(error)),
            )));
        }

        view.add_line(Line::from(""));
        view.add_line(Line::from(format!(
            "Requests: {} | Errors: {} | Duration: {}ms",
            results.total_requests, results.errors, results.duration_ms
        )));
    }
}

pub use eggsec::dispatch::OAuthResults;

impl TabState for OAuthTab {
    tab_state_boilerplate!(OAuthTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.core.inputs.set_field_value("Concurrency", "10");
        self.core.inputs.set_field_value("Timeout (s)", "15");
        self.core.inputs.blur();
        self.focus_area = StandardFocusArea::Inputs;
        self.focused_checkbox_index = 0;
        self.redirect_test_checkbox.checked = true;
        self.scope_test_checkbox.checked = true;
        self.state_test_checkbox.checked = true;
        self.grant_test_checkbox.checked = true;
    }
}

impl TabRender for OAuthTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref error) = self.core.error {
            let msg = error.message();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("OAuth - Error")
                .border_style(Style::default().fg(tc!(error)));
            let paragraph = Paragraph::new(msg)
                .style(Style::default().fg(tc!(error)))
                .block(block);
            f.render_widget(paragraph, area);
            return;
        }

        // Dynamic layout based on terminal height
        let (input_height, options_height, results_min) = if area.height < 30 {
            let ih = ((area.height as f32 * 0.6) as u16).clamp(8, 17);
            let oh = ((area.height as f32 * 0.2) as u16).clamp(4, 6);
            (ih, oh, 2)
        } else {
            (17, 6, 5)
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(input_height),
                Constraint::Length(options_height),
                Constraint::Min(results_min),
            ])
            .split(area);

        // Input fields
        let input_block = Block::default()
            .title(" OAuth/OIDC Configuration ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == StandardFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
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
                Constraint::Length(3),
            ])
            .split(input_inner);

        render_input_fields(f, &input_chunks, &self.core.inputs, insert_mode);

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
        crate::tabs::core::render_checkbox_row(
            f,
            &options_chunks,
            &[
                &self.redirect_test_checkbox,
                &self.scope_test_checkbox,
                &self.state_test_checkbox,
                &self.grant_test_checkbox,
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

impl TabInput for OAuthTab {
    tab_input_boilerplate!(
        OAuthTab,
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
        self.focus_area = crate::tabs::core::focus_next_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
        );
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = crate::tabs::core::focus_prev_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
        );
    }

    fn handle_up(&mut self) {
        crate::tabs::core::handle_up_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Results,
        );
    }

    fn handle_down(&mut self) {
        crate::tabs::core::handle_down_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Results,
        );
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
            let mut checkboxes = [
                &mut self.redirect_test_checkbox,
                &mut self.scope_test_checkbox,
                &mut self.state_test_checkbox,
                &mut self.grant_test_checkbox,
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

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        match self.focus_area {
            StandardFocusArea::Inputs => self.core.inputs.move_left(),
            StandardFocusArea::Options => {
                crate::tabs::core::move_checkbox_focus_left(&mut self.focused_checkbox_index, 4)
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
                crate::tabs::core::move_checkbox_focus_right(&mut self.focused_checkbox_index, 4)
            }
            _ => false,
        }
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            StandardFocusArea::Inputs => !self.core.inputs.can_move_left(),
            StandardFocusArea::Options => {
                crate::tabs::core::is_checkbox_focus_at_left_edge(self.focused_checkbox_index, 4)
            }
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            StandardFocusArea::Inputs => !self.core.inputs.can_move_right(),
            StandardFocusArea::Options => {
                crate::tabs::core::is_checkbox_focus_at_right_edge(self.focused_checkbox_index, 4)
            }
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab() -> OAuthTab {
        OAuthTab::new()
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
        let before = tab.redirect_test_checkbox.checked;
        tab.handle_enter();
        assert_eq!(tab.redirect_test_checkbox.checked, !before);
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
        tab.focused_checkbox_index = 2;
        tab.handle_escape();
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);
        assert_eq!(tab.focused_checkbox_index, 0);
    }

    #[test]
    fn test_handle_left_right_navigates_checkboxes() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        tab.focused_checkbox_index = 0;

        tab.handle_right();
        assert_eq!(tab.focused_checkbox_index, 1);

        tab.handle_right();
        assert_eq!(tab.focused_checkbox_index, 2);

        tab.handle_left();
        assert_eq!(tab.focused_checkbox_index, 1);

        tab.handle_left();
        assert_eq!(tab.focused_checkbox_index, 0);

        tab.handle_left();
        assert_eq!(tab.focused_checkbox_index, 0);
    }
}
