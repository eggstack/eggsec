use crate::components::InputField;
use crate::tabs::core::{
    evaluate_enter, execute_enter_action, field_as, field_str,
    render_config_block, render_results_area, start_scan, StandardFocusArea2, TabCore,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_escape_2area, tab_input_2area, tab_state_boilerplate, tc};
use eggsec::scanner::fingerprint::FingerprintResults;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    Frame,
};

pub struct FingerprintTab {
    pub core: TabCore,
    pub results: Option<FingerprintResults>,
    pub focus_area: StandardFocusArea2,
}

impl FingerprintTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("Target Host"))
            .add(
                InputField::new("Ports (comma-separated)")
                    .with_value("80,443,22,21,25,3306,5432,6379,27017"),
            )
            .add(InputField::new("Timeout (s)").with_value("5"));

        Self {
            core: TabCore::new("Fingerprinting...", "Results").with_inputs(inputs),
            results: None,
            focus_area: StandardFocusArea2::Inputs,
        }
    }

    pub fn get_results(&self) -> Option<&FingerprintResults> {
        self.results.as_ref()
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn ports(&self) -> &str {
        field_str(&self.core, 1)
    }

    pub fn timeout(&self) -> u64 {
        field_as(&self.core, 2, 5)
    }

    pub fn set_results(&mut self, results: FingerprintResults) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.core.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &FingerprintResults) {
        self.core.results_view.clear();

        let host = results.host.clone();
        let services_identified = results.services_identified;

        let fp_data: Vec<_> = results
            .results
            .iter()
            .map(|fp| {
                let banner = fp
                    .banner
                    .as_deref()
                    .unwrap_or("-")
                    .lines()
                    .next()
                    .unwrap_or("-");
                let banner_display = if banner.len() > 40 {
                    let truncate_pos = banner
                        .char_indices()
                        .take_while(|(i, _)| *i < 37)
                        .last()
                        .map(|(i, c)| i + c.len_utf8())
                        .unwrap_or(37);
                    format!("{}...", &banner[..truncate_pos])
                } else {
                    banner.to_string()
                };
                (
                    fp.port,
                    fp.service.clone(),
                    fp.version.clone(),
                    banner_display,
                )
            })
            .collect();

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Host: ", Style::default().fg(tc!(warning))),
            Span::raw(host),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Services identified: ", Style::default().fg(tc!(info))),
            Span::raw(services_identified.to_string()),
        ]));

        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<8}", "PORT"), Style::default().fg(tc!(warning))),
            Span::styled(
                format!("{:<15}", "SERVICE"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled(
                format!("{:<12}", "VERSION"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled("BANNER", Style::default().fg(tc!(warning))),
        ]));

        for (port, service, version, banner_display) in fp_data {
            self.core.results_view.add_line(Line::from(vec![
                Span::styled(format!("{:<8}", port), Style::default().fg(tc!(success))),
                Span::raw(format!("{:<15}", service)),
                Span::raw(format!("{:<12}", version.as_deref().unwrap_or("-"))),
                Span::styled(banner_display, Style::default().fg(tc!(text_dim))),
            ]));
        }
    }

    pub fn start(&mut self) {
        if start_scan(&mut self.core) {
            self.results = None;
        }
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }
}

impl Default for FingerprintTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for FingerprintTab {
    tab_state_boilerplate!(FingerprintTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        if let Some(field) = self.core.inputs.fields.get_mut(1) {
            field.value = "80,443,22,21,25,3306,5432,6379,27017".to_string();
            field.cursor_pos = 36;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(2) {
            field.value = "5".to_string();
            field.cursor_pos = 1;
        }
        self.focus_area = StandardFocusArea2::Inputs;
    }
}

impl TabRender for FingerprintTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_inner = render_config_block(
            f,
            input_area,
            "Fingerprint Configuration",
            self.focus_area == StandardFocusArea2::Inputs,
        );

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_inner);

        for (i, field) in self.core.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        let results_inner = render_config_block(
            f,
            results_area,
            "Results",
            self.focus_area == StandardFocusArea2::Results,
        );

        render_results_area(
            f,
            results_inner,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Results",
            "Results will appear here after running",
        );
    }
}

impl TabInput for FingerprintTab {
    tab_input_2area!(
        FingerprintTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea2::Inputs,
        Results: StandardFocusArea2::Results
    );

    fn handle_enter(&mut self) {
        let running = self.is_running();
        let inputs_focused = self.core.inputs.is_focused();
        let action = evaluate_enter(
            self.focus_area,
            StandardFocusArea2::Inputs,
            StandardFocusArea2::Results,
            running,
            inputs_focused,
        );
        execute_enter_action(&mut self.core, action);
        if matches!(action, crate::tabs::core::EnterAction::Start) {
            self.results = None;
        }
    }

    tab_escape_2area!(FingerprintTab, core: core, focus: focus_area, Inputs: StandardFocusArea2::Inputs);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab() -> FingerprintTab {
        FingerprintTab::new()
    }

    #[test]
    fn test_enter_in_inputs_focused_blurs_does_not_start() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea2::Inputs;
        tab.core.inputs.focus(0);
        assert!(tab.core.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.core.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_in_results_no_op() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea2::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    #[test]
    fn test_focus_cycle_2area() {
        let mut tab = create_test_tab();
        assert_eq!(tab.focus_area, StandardFocusArea2::Inputs);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea2::Results);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea2::Inputs);
        assert!(tab.core.inputs.is_focused());
    }

    #[test]
    fn test_focus_prev_cycle_2area() {
        let mut tab = create_test_tab();
        assert_eq!(tab.focus_area, StandardFocusArea2::Inputs);

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, StandardFocusArea2::Results);

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, StandardFocusArea2::Inputs);
    }

    #[test]
    fn test_escape_from_inputs_blurs() {
        let mut tab = create_test_tab();
        tab.core.inputs.focus(0);
        tab.handle_escape();
        assert!(!tab.core.inputs.is_focused());
    }

    #[test]
    fn test_escape_from_results_goes_to_inputs() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea2::Results;
        tab.handle_escape();
        assert_eq!(tab.focus_area, StandardFocusArea2::Inputs);
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
        tab.focus_area = StandardFocusArea2::Inputs;
        tab.core.inputs.focus(0);
        tab.handle_char('x');
        assert_eq!(tab.core.target(), "x");

        tab.focus_area = StandardFocusArea2::Results;
        tab.handle_char('y');
        assert_eq!(tab.core.target(), "x");
    }

    #[test]
    fn test_field_accessors() {
        let tab = create_test_tab();
        assert_eq!(tab.target(), "");
        assert_eq!(tab.ports(), "80,443,22,21,25,3306,5432,6379,27017");
        assert_eq!(tab.timeout(), 5);
    }

    #[test]
    fn test_is_input_focused() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea2::Inputs;
        assert!(!tab.is_input_focused());
        tab.core.inputs.focus(0);
        assert!(tab.is_input_focused());
        tab.focus_area = StandardFocusArea2::Results;
        assert!(!tab.is_input_focused());
    }

    #[test]
    fn test_start_with_target() {
        let mut tab = create_test_tab();
        tab.core.inputs.focus(0);
        tab.core.inputs.insert('a');
        tab.core.inputs.insert('.');
        tab.core.inputs.insert('c');
        tab.core.inputs.insert('o');
        tab.core.inputs.insert('m');
        tab.core.inputs.blur();
        tab.start();
        assert!(tab.is_running());
    }

    #[test]
    fn test_start_without_target() {
        let mut tab = create_test_tab();
        tab.start();
        assert!(!tab.is_running());
    }

    #[test]
    fn test_stop() {
        let mut tab = create_test_tab();
        tab.core.state = AppState::Running;
        tab.handle_enter();
        assert!(!tab.is_running());
    }
}
