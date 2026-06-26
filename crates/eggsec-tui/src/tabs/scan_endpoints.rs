use crate::app::tab_error::TabError;
use crate::components::{Checkbox, InputField};
use crate::tabs::core::{
    field_as, render_input_fields, render_results_area, start_scan, StandardFocusArea, TabCore,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_escape, tab_input_3area, tab_state_boilerplate, tc};
use eggsec::scanner::endpoints::EndpointScanResults;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct ScanEndpointsTab {
    pub core: TabCore,
    pub results: Option<EndpointScanResults>,
    pub include_404_checkbox: Checkbox,
    pub focus_area: StandardFocusArea,
}

impl ScanEndpointsTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Concurrency").with_value("20"))
            .add(InputField::new("Timeout (s)").with_value("10"))
            .add(InputField::new("Wordlist (default: common)"));

        Self {
            core: TabCore::new("Scanning endpoints...", "Results").with_inputs(inputs),
            results: None,
            include_404_checkbox: Checkbox::new("Check for 404s").checked(true),
            focus_area: StandardFocusArea::Inputs,
        }
    }

    pub fn get_results(&self) -> Option<&EndpointScanResults> {
        self.results.as_ref()
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn concurrency(&self) -> usize {
        field_as(&self.core, 1, 20)
    }

    pub fn timeout(&self) -> u64 {
        field_as(&self.core, 2, 10)
    }

    pub fn wordlist(&self) -> Option<&str> {
        let w = crate::tabs::core::field_str(&self.core, 3);
        if w.is_empty() {
            None
        } else {
            Some(w)
        }
    }

    pub fn include_404(&self) -> bool {
        self.include_404_checkbox.checked
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }

    pub fn set_results(&mut self, results: EndpointScanResults) {
        let _view = self.core.prepare_results();
        self.update_results_view(&results);
        self.results = Some(results);
    }

    fn update_results_view(&mut self, results: &EndpointScanResults) {
        use ratatui::style::Modifier;

        self.core.results_view.clear();

        let base_url = results.base_url.clone();
        let endpoints_scanned = results.endpoints_scanned;
        let endpoints_found = results.endpoints_found;
        let interesting_findings = results.interesting_findings;

        let endpoint_data: Vec<_> = results
            .results
            .iter()
            .map(|e| {
                (
                    e.path.clone(),
                    e.status_code,
                    e.content_length,
                    e.interesting,
                )
            })
            .collect();

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("URL: ", Style::default().fg(tc!(accent))),
            Span::raw(base_url),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Scanned: ", Style::default().fg(tc!(secondary))),
            Span::raw(endpoints_scanned.to_string()),
            Span::raw(" | "),
            Span::styled("Found: ", Style::default().fg(tc!(info))),
            Span::raw(endpoints_found.to_string()),
            Span::raw(" | "),
            Span::styled("Interesting: ", Style::default().fg(tc!(error))),
            Span::raw(interesting_findings.to_string()),
        ]));

        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<40}", "PATH"), Style::default().fg(tc!(accent))),
            Span::styled(format!("{:<8}", "STATUS"), Style::default().fg(tc!(accent))),
            Span::styled(format!("{:<10}", "SIZE"), Style::default().fg(tc!(accent))),
        ]));

        for (path, status_code, content_length, interesting) in endpoint_data {
            let style = if interesting {
                Style::default().fg(tc!(error)).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_color = match status_code {
                200..=299 => tc!(success),
                300..=399 => tc!(secondary),
                400..=499 => tc!(warning),
                _ => tc!(error),
            };

            let path_display = if path.len() > 38 {
                let truncate_pos = path
                    .char_indices()
                    .take_while(|(i, _)| *i < 35)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(35);
                format!("{}...", &path[..truncate_pos])
            } else {
                path
            };

            self.core.results_view.add_line(Line::from(vec![
                Span::styled(format!("{:<40}", path_display), style),
                Span::styled(
                    format!("{:<8}", status_code),
                    Style::default().fg(status_color),
                ),
                Span::raw(format!("{:<10}", content_length.unwrap_or(0))),
            ]));
        }
    }

    pub fn start(&mut self) {
        if self.target().is_empty() {
            self.core.state = AppState::Error("Target cannot be empty".to_string());
            self.core.error =
                Some(TabError::Target("Target cannot be empty".to_string()));
            return;
        }

        let wordlist_path = self.wordlist().map(|s| s.to_string());
        if let Some(path_str) = wordlist_path {
            let path = std::path::Path::new(&path_str);
            if !path.exists() {
                self.core.state =
                    AppState::Error(format!("Wordlist file not found: {}", path_str));
                self.core.error = Some(TabError::Config(format!(
                    "Wordlist file not found: {}",
                    path_str
                )));
                return;
            }
            if !path.is_file() {
                self.core.state =
                    AppState::Error(format!("Wordlist path is not a file: {}", path_str));
                self.core.error = Some(TabError::Config(format!(
                    "Wordlist path is not a file: {}",
                    path_str
                )));
                return;
            }
        }

        start_scan(&mut self.core);
    }
}

impl Default for ScanEndpointsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ScanEndpointsTab {
    tab_state_boilerplate!(ScanEndpointsTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        if let Some(field) = self.core.inputs.fields.get_mut(1) {
            field.value = "20".to_string();
            field.cursor_pos = 2;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(2) {
            field.value = "10".to_string();
            field.cursor_pos = 2;
        }
        self.focus_area = StandardFocusArea::Inputs;
        self.include_404_checkbox.checked = true;
    }
}

impl TabRender for ScanEndpointsTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(17), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Endpoint Scan Configuration ")
            .border_style(Style::default().fg(
                if self.focus_area == StandardFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
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

        let include_404 = self.include_404_checkbox.clone();
        if let Some(chunk) = input_chunks.get(4) {
            include_404.render(f, *chunk);
        }

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

impl TabInput for ScanEndpointsTab {
    tab_input_3area!(
        ScanEndpointsTab,
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
            self.include_404_checkbox.checked = !self.include_404_checkbox.checked;
        }
    }

    tab_escape!(ScanEndpointsTab, core: core, focus: focus_area, strategy: three_area, Inputs: StandardFocusArea::Inputs, Options: StandardFocusArea::Options, Results: StandardFocusArea::Results);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab() -> ScanEndpointsTab {
        ScanEndpointsTab::new()
    }

    #[test]
    fn test_enter_in_inputs_focused_blurs_does_not_start() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Inputs;
        tab.core.inputs.focus(0);
        assert!(tab.core.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.core.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_in_options_toggles_checkbox() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        assert!(tab.include_404_checkbox.checked);
        tab.handle_enter();
        assert!(!tab.include_404_checkbox.checked);
        tab.handle_enter();
        assert!(tab.include_404_checkbox.checked);
    }

    #[test]
    fn test_enter_in_results_no_op() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    #[test]
    fn test_focus_cycle_3area() {
        let mut tab = create_test_tab();
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea::Options);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea::Results);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);
    }

    #[test]
    fn test_focus_prev_cycle_3area() {
        let mut tab = create_test_tab();
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, StandardFocusArea::Results);

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, StandardFocusArea::Options);

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);
    }

    #[test]
    fn test_escape_from_inputs_blurs() {
        let mut tab = create_test_tab();
        tab.core.inputs.focus(0);
        tab.handle_escape();
        assert!(!tab.core.inputs.is_focused());
    }

    #[test]
    fn test_escape_from_options_goes_to_inputs() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        tab.handle_escape();
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);
        assert!(tab.core.inputs.is_focused());
    }

    #[test]
    fn test_escape_when_running_stops() {
        let mut tab = create_test_tab();
        tab.core.state = AppState::Running;
        tab.handle_escape();
        assert_eq!(tab.core.state, AppState::Idle);
    }

    #[test]
    fn test_handle_char_inputs_only() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Inputs;
        tab.core.inputs.focus(0);
        tab.handle_char('a');
        assert_eq!(tab.core.target(), "a");

        tab.focus_area = StandardFocusArea::Results;
        tab.handle_char('b');
        assert_eq!(tab.core.target(), "a");
    }

    #[test]
    fn test_handle_up_down_scroll_results() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Results;
        tab.core.results_view.add_line(ratatui::text::Line::from("line1"));
        tab.core.results_view.add_line(ratatui::text::Line::from("line2"));

        tab.handle_down();
        tab.handle_up();
    }

    #[test]
    fn test_is_input_focused() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Inputs;
        assert!(!tab.is_input_focused());
        tab.core.inputs.focus(0);
        assert!(tab.is_input_focused());
        tab.focus_area = StandardFocusArea::Results;
        assert!(!tab.is_input_focused());
    }

    #[test]
    fn test_field_accessors() {
        let tab = create_test_tab();
        assert_eq!(tab.concurrency(), 20);
        assert_eq!(tab.timeout(), 10);
        assert!(tab.wordlist().is_none());
    }
}
