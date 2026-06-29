use crate::components::InputField;
use crate::tabs::core::{
    render_config_block, render_input_fields, render_results_area, StandardFocusArea2, TabCore,
};
use crate::tabs::{TabInput, TabRender, TabState};
use crate::{tab_escape, tab_input_2area, tab_state_boilerplate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub struct WafStressTab {
    pub core: TabCore,
    pub focus_area: StandardFocusArea2,
}

impl WafStressTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Concurrency").with_value("20"))
            .add(InputField::new("Timeout (s)").with_value("10"));

        Self {
            core: TabCore::new("WAF Stress Testing...", "Results").with_inputs(inputs),
            focus_area: StandardFocusArea2::Inputs,
        }
    }

    pub fn get_results(&self) -> Option<String> {
        if self.core.results_view.is_empty() {
            None
        } else {
            Some(self.core.results_view.get_content())
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn concurrency(&self) -> usize {
        self.core
            .inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(20)
    }

    pub fn timeout(&self) -> u64 {
        self.core
            .inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }
}

impl Default for WafStressTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for WafStressTab {
    tab_state_boilerplate!(WafStressTab, core: core);

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
        self.focus_area = StandardFocusArea2::Inputs;
    }
}

impl TabRender for WafStressTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(area);

        let input_area = match chunks.first() {
            Some(area) => *area,
            None => return,
        };
        let results_area = match chunks.get(1) {
            Some(area) => *area,
            None => return,
        };

        let input_inner = render_config_block(
            f,
            input_area,
            "WAF Stress Configuration",
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

        render_input_fields(f, &input_chunks, &self.core.inputs, insert_mode);

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

impl TabInput for WafStressTab {
    tab_input_2area!(
        WafStressTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea2::Inputs,
        Results: StandardFocusArea2::Results
    );
    tab_escape!(WafStressTab, core: core, focus: focus_area, strategy: simple, Inputs: StandardFocusArea2::Inputs);

    fn handle_enter(&mut self) {
        let running = self.is_running();
        let inputs_focused = self.core.inputs.is_focused();
        crate::tabs::core::handle_enter_2area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea2::Inputs,
            StandardFocusArea2::Results,
            running,
            inputs_focused,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tabs::{AppState, TabInput, TabState};

    #[test]
    fn waf_stress_new_has_correct_fields() {
        let tab = WafStressTab::new();
        assert_eq!(tab.core.inputs.fields.len(), 3);
        assert_eq!(tab.core.inputs.fields[0].label, "Target URL");
        assert_eq!(tab.core.inputs.fields[1].label, "Concurrency");
        assert_eq!(tab.core.inputs.fields[2].label, "Timeout (s)");
        assert_eq!(tab.core.target(), "");
        assert!(!tab.is_running());
    }

    #[test]
    fn waf_stress_target_accessor() {
        let mut tab = WafStressTab::new();
        tab.core.inputs.fields[0].value = "https://example.com".to_string();
        assert_eq!(tab.target(), "https://example.com");
    }

    #[test]
    fn waf_stress_concurrency_parses() {
        let tab = WafStressTab::new();
        assert_eq!(tab.concurrency(), 20);
    }

    #[test]
    fn waf_stress_timeout_parses() {
        let tab = WafStressTab::new();
        assert_eq!(tab.timeout(), 10);
    }

    #[test]
    fn waf_stress_enter_starts_with_target() {
        let mut tab = WafStressTab::new();
        tab.core.inputs.fields[0].value = "https://example.com".to_string();
        tab.handle_enter();
        assert!(tab.is_running());
    }

    #[test]
    fn waf_stress_enter_no_start_without_target() {
        let mut tab = WafStressTab::new();
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    #[test]
    fn waf_stress_enter_results_no_op() {
        let mut tab = WafStressTab::new();
        tab.focus_area = StandardFocusArea2::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    #[test]
    fn waf_stress_escape_returns_to_inputs() {
        let mut tab = WafStressTab::new();
        tab.focus_area = StandardFocusArea2::Results;
        tab.handle_escape();
        assert_eq!(tab.focus_area, StandardFocusArea2::Inputs);
    }

    #[test]
    fn waf_stress_focus_navigation() {
        let mut tab = WafStressTab::new();
        assert_eq!(tab.focus_area, StandardFocusArea2::Inputs);
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea2::Results);
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea2::Inputs);
        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, StandardFocusArea2::Results);
    }

    #[test]
    fn waf_stress_reset_restores_defaults() {
        let mut tab = WafStressTab::new();
        tab.core.inputs.fields[0].value = "changed".to_string();
        tab.core.state = AppState::Running;
        tab.focus_area = StandardFocusArea2::Results;
        tab.reset();
        assert_eq!(tab.core.target(), "");
        assert_eq!(tab.core.state, AppState::Idle);
        assert_eq!(tab.focus_area, StandardFocusArea2::Inputs);
        assert_eq!(tab.core.inputs.fields[1].value, "20");
        assert_eq!(tab.core.inputs.fields[2].value, "10");
    }

    #[test]
    fn waf_stress_primary_target() {
        let mut tab = WafStressTab::new();
        tab.core.inputs.fields[0].value = "https://example.com".to_string();
        assert_eq!(
            tab.primary_target(),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn waf_stress_copy_results() {
        let mut tab = WafStressTab::new();
        tab.focus_area = StandardFocusArea2::Results;
        // Empty results returns Some("") (macro default behavior)
        let copied = tab.handle_copy();
        assert!(copied.is_some());
        assert!(copied.unwrap().is_empty());

        // With content, copy returns the content
        tab.core
            .results_view
            .add_line(ratatui::text::Line::from("test"));
        let copied = tab.handle_copy();
        assert!(copied.is_some());
        assert!(copied.unwrap().contains("test"));
    }
}
