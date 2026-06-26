use crate::components::InputField;
use crate::tabs::core::{
    render_config_block, render_error_block, render_input_fields, render_results_area,
    StandardFocusArea2, TabCore,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_2area, tab_state_boilerplate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub struct ResumeTab {
    pub core: TabCore,
    pub focus_area: StandardFocusArea2,
}

impl ResumeTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("Session File Path"));

        Self {
            core: TabCore::new("Loading...", "Session Info").with_inputs(inputs),
            focus_area: StandardFocusArea2::Inputs,
        }
    }

    pub fn session_file(&self) -> &str {
        self.core.target()
    }

    pub fn start(&mut self) {
        if !self.session_file().is_empty() {
            self.core.state = AppState::Running;
            self.core.results_view.clear();
        }
    }
}

impl Default for ResumeTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ResumeTab {
    tab_state_boilerplate!(ResumeTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.focus_area = StandardFocusArea2::Inputs;
    }
}

impl TabRender for ResumeTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref err) = self.core.error {
            render_error_block(f, area, "Resume - Error", err);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(0)])
            .split(area);

        let input_area = chunks.first().copied().unwrap_or(area);
        let results_area = chunks.get(1).copied().unwrap_or(area);

        let input_inner = render_config_block(
            f,
            input_area,
            "Resume Session",
            self.focus_area == StandardFocusArea2::Inputs,
        );

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3)])
            .split(input_inner);

        render_input_fields(f, &input_chunks, &self.core.inputs, insert_mode);

        render_results_area(
            f,
            results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Session Info",
            "Session information will appear here",
        );
    }
}

impl TabInput for ResumeTab {
    tab_input_2area!(
        ResumeTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea2::Inputs,
        Results: StandardFocusArea2::Results
    );

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }

        if self.focus_area == StandardFocusArea2::Results {
            return;
        }

        if self.core.inputs.is_focused() {
            self.core.inputs.blur();
        }
        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        self.core.inputs.blur();
        self.focus_area = StandardFocusArea2::Inputs;
    }
}
