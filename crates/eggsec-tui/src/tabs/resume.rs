use crate::app::tab_error::TabError;
use crate::components::InputField;
use crate::tabs::core::{
    render_config_block, render_error_block, render_input_fields, render_results_area,
    StandardFocusArea2, TabCore,
};
use crate::tabs::{TabInput, TabRender, TabState};
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
        let inputs = crate::components::InputGroup::new().add(InputField::new("Session File Path"));

        Self {
            core: TabCore::new("Loading...", "Session Info").with_inputs(inputs),
            focus_area: StandardFocusArea2::Inputs,
        }
    }

    pub fn session_file(&self) -> &str {
        self.core.target()
    }

    pub fn start(&mut self) {
        if self.session_file().is_empty() {
            self.core.error = Some(TabError::Target(
                "Session file path is required to resume a scan".to_string(),
            ));
            return;
        }
        // Honest failure, not a fake run.
        //
        // `Tab::Resume` has no `operation` in its `TabSpec`, no `TaskKind`
        // variant, and no canonical executor arm — there is no runtime request
        // that can carry a session file. Entering `AppState::Running` here
        // produced a spinner that resolved to "nothing to run" while the tab
        // advertised a resumable scan. Report the real limitation instead, and
        // point at the surface that does support it.
        self.core.error = Some(TabError::Config(
            "Resuming a saved session is not available from the TUI: there is no \
             dispatchable runtime request for a session file. Use the CLI \
             (`eggsec resume <session-file>`) instead."
                .to_string(),
        ));
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
            // Short lines on purpose: `empty_state_paragraph` does not wrap, so
            // anything wider than the pane would be clipped mid-sentence.
            "No session loaded.\nResume runs from the CLI only: eggsec resume <session-file>",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tabs::AppState;

    /// `start()` used to enter `Running` with a session path, which could
    /// only resolve to "nothing to run". It must stay Idle and explain why.
    #[test]
    fn start_with_session_path_reports_a_clear_error() {
        let mut tab = ResumeTab::new();
        tab.core.inputs.fields.get_mut(0).unwrap().value = "session.json".into();
        tab.start();
        let err = tab.core.error.as_ref().expect("resume must explain itself");
        assert!(matches!(err, TabError::Config(_)), "got {err:?}");
        assert_eq!(tab.core.state, AppState::Idle, "must not fake a run");
    }

    #[test]
    fn handle_enter_without_path_reports_a_clear_error() {
        let mut tab = ResumeTab::new();
        tab.focus_area = StandardFocusArea2::Inputs;
        tab.handle_enter();
        assert!(matches!(tab.core.error, Some(TabError::Target(_))));
        assert_eq!(tab.core.state, AppState::Idle);
    }

    /// The empty state must not promise a run the tab cannot perform.
    #[test]
    fn empty_state_does_not_promise_a_run() {
        use ratatui::{backend::TestBackend, Terminal};

        let tab = ResumeTab::new();
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                tab.render(f, area, false);
            })
            .unwrap();
        let text = crate::test_utils::buffer_to_text(terminal.backend().buffer());
        assert!(text.contains("eggsec resume"), "got:\n{text}");
        assert!(
            !text.contains("Session information will appear here"),
            "empty state must not promise a run:\n{text}"
        );
    }
}
