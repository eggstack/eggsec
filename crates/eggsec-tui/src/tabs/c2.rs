use crate::app::tab_error::TabError;
use crate::components::InputField;
use crate::tabs::core::{render_results_area, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};

use crate::{tab_input_indexed, tab_state_boilerplate, tc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum C2FocusArea {
    Target,
    Campaign,
    Results,
}

const C2_INPUT_AREAS: &[C2FocusArea] = &[C2FocusArea::Target, C2FocusArea::Campaign];

pub struct C2Tab {
    pub core: TabCore,
    pub focus_area: C2FocusArea,
}

impl C2Tab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(
                InputField::new("Target")
                    .with_width(50)
                    .with_value("localhost"),
            )
            .add(
                InputField::new("Campaign Profile")
                    .with_width(30)
                    .with_value("apt29"),
            );

        Self {
            core: TabCore::new("Running C2 simulation...", "C2 Results").with_inputs(inputs),
            focus_area: C2FocusArea::Target,
        }
    }

    fn sync_input_focus(&mut self) {
        crate::tabs::core::sync_indexed_input_focus(
            &mut self.core,
            self.focus_area,
            C2_INPUT_AREAS,
        );
    }

    pub fn target(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn campaign(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn primary_target(&self) -> Option<String> {
        self.target().map(|s| s.to_string())
    }

}

impl Default for C2Tab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for C2Tab {
    tab_state_boilerplate!(C2Tab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.core.inputs.clear_all_fields();
        self.core.inputs.blur();
        self.focus_area = C2FocusArea::Target;
    }
}

impl TabRender for C2Tab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            C2FocusArea::Target => "Target",
            C2FocusArea::Campaign => "Campaign",
            C2FocusArea::Results => "Results",
        };
        Some(vec!["C2", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        use crate::components::FormBuilder;

        if let Some(ref err) = self.core.error {
            crate::tabs::core::render_error_block(f, area, "C2 - Error", err);
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(9),
                Constraint::Min(8),
            ])
            .split(area);

        let Some(title_area) = layout.get(0) else {
            return;
        };
        let Some(inputs_area) = layout.get(1) else {
            return;
        };
        let Some(results_area) = layout.get(2) else {
            return;
        };

        let title = Paragraph::new(
            "C2 Campaign Simulation — Defense-lab only | Beacons, Tasking, OPSEC, Attack Graph",
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("\u{26a0} Defense Lab")
                .border_style(Style::default().fg(tc!(border))),
        )
        .style(Style::default().fg(tc!(warning)));
        f.render_widget(title, *title_area);

        let mut builder = FormBuilder::new("Inputs").row_height(3);
        for field in &self.core.inputs.fields {
            builder = builder.add_input(field.clone());
        }
        builder.render(f, *inputs_area, insert_mode);

        render_results_area(
            f,
            *results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "C2 Campaign Results",
            "Ready for C2 simulation. Select a campaign profile and press Enter.\n\nAvailable profiles: apt29, carbanak, default",
        );
    }
}

impl TabInput for C2Tab {
    fn stop(&mut self) {
        self.core.stop();
    }

    tab_input_indexed!(
        C2Tab,
        core: core,
        focus: focus_area,
        InputAreas: C2_INPUT_AREAS,
        Results: C2FocusArea::Results
    );

    fn handle_focus_next(&mut self) {
        self.focus_area = crate::tabs::core::focus_next_indexed(
            self.focus_area,
            C2_INPUT_AREAS,
            C2FocusArea::Results,
        );
        self.sync_input_focus();
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = crate::tabs::core::focus_prev_indexed(
            self.focus_area,
            C2_INPUT_AREAS,
            C2FocusArea::Results,
        );
        self.sync_input_focus();
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        if self.focus_area == C2FocusArea::Results {
            return;
        }

        if self.target().map_or(true, |t| t.is_empty()) {
            let err = TabError::Target("Target is required for C2 simulation".to_string());
            self.core.state = AppState::Error(err.message());
            self.core.error = Some(err);
            return;
        }

        if self.is_input_focused() {
            self.core.inputs.blur();
        }
        self.core.state = AppState::Running;
        self.core.error = None;
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.core.inputs.blur();
        self.focus_area = C2FocusArea::Results;
        self.sync_input_focus();
    }

    fn handle_up(&mut self) {
        self.focus_area = crate::tabs::core::focus_up_indexed(
            self.focus_area,
            C2_INPUT_AREAS,
            C2FocusArea::Results,
        );
        self.sync_input_focus();
    }

    fn handle_down(&mut self) {
        self.focus_area = crate::tabs::core::focus_down_indexed(
            self.focus_area,
            C2_INPUT_AREAS,
            C2FocusArea::Results,
        );
        self.sync_input_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_returns_none_when_empty() {
        let mut tab = C2Tab::new();
        tab.core.inputs.fields[0].value.clear();
        assert!(tab.target().is_none());
    }

    #[test]
    fn test_target_returns_value_when_set() {
        let mut tab = C2Tab::new();
        tab.core.inputs.fields[0].value = "10.0.0.1".to_string();
        assert_eq!(tab.target(), Some("10.0.0.1"));
    }

    #[test]
    fn test_campaign_returns_default_when_empty() {
        let tab = C2Tab::new();
        assert_eq!(tab.campaign(), Some("apt29"));
    }

    #[test]
    fn test_build_run_request_returns_none_without_target() {
        use crate::app::task_management::TaskBuilder;
        let mut tab = C2Tab::new();
        tab.core.inputs.fields[0].value.clear();
        assert!(tab.build_run_request().is_none());
    }

    #[test]
    fn test_build_run_request_uses_ui_values() {
        use crate::app::task_management::TaskBuilder;
        let mut tab = C2Tab::new();
        tab.core.inputs.fields[0].value = "10.0.0.1".to_string();
        tab.core.inputs.fields[1].value = "carbanak".to_string();

        let req = tab.build_run_request().unwrap();
        match req.task_kind {
            eggsec_runtime::request::TaskKind::C2(params) => {
                assert_eq!(params.target.as_deref(), Some("10.0.0.1"));
                assert_eq!(params.profile.as_deref(), Some("carbanak"));
            }
            _ => panic!("Expected TaskKind::C2"),
        }
    }

    #[test]
    fn non_first_input_fields_are_editable() {
        let mut tab = C2Tab::new();
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, C2FocusArea::Campaign);
        assert!(tab.is_input_focused());

        tab.core.inputs.fields[1].clear();
        tab.handle_char('c');

        assert_eq!(tab.core.inputs.fields[1].value, "c");
    }

    #[test]
    fn input_labels_are_unique() {
        let tab = C2Tab::new();
        assert_eq!(
            tab.core.inputs.duplicate_label_names(),
            Vec::<String>::new()
        );
    }
}
