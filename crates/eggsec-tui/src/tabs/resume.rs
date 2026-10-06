use crate::app::tab_error::TabError;
use crate::components::InputField;
use crate::tabs::core::{
    render_config_block, render_error_block, render_input_fields, render_results_area,
    StandardFocusArea2, TabCore,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_boilerplate, tab_state_boilerplate};
use eggsec::pipeline::session::{PipelineSession, SessionEntry};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub struct ResumeTab {
    pub core: TabCore,
    pub focus_area: StandardFocusArea2,
    /// Checkpoints discovered in the session store, newest first.
    pub entries: Vec<SessionEntry>,
    /// Index of the highlighted row in `entries`.
    pub selected: usize,
}

impl ResumeTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("Session File Path (optional override)"));

        Self {
            core: TabCore::new("Loading...", "Saved Sessions").with_inputs(inputs),
            focus_area: StandardFocusArea2::Inputs,
            entries: Vec::new(),
            selected: 0,
        }
    }

    /// Path the manual field is holding, if any.
    pub fn manual_path(&self) -> &str {
        self.core
            .inputs
            .fields
            .first()
            .map(|f| f.value.trim())
            .unwrap_or_default()
    }

    /// The checkpoint a resume would use: the manual path when the operator
    /// typed one, otherwise the highlighted row.
    ///
    /// The manual field wins so an operator can reach a checkpoint that lives
    /// outside the store directory — those exist, because the CLI derives
    /// checkpoint paths from `--output` when it ends in `.session.json`.
    pub fn selected_path(&self) -> Option<String> {
        if !self.manual_path().is_empty() {
            return Some(self.manual_path().to_string());
        }
        self.entries
            .get(self.selected)
            .map(|e| e.path.to_string_lossy().to_string())
    }

    /// The resumed target, needed before dispatch to build the enforcement
    /// descriptor. It is known for a listed row, and only learnable by reading
    /// the file for a manual path — so a manual entry resolves it lazily.
    pub fn selected_target(&self) -> Option<String> {
        if self.manual_path().is_empty() {
            return self.entries.get(self.selected).map(|e| e.target.clone());
        }
        std::fs::read_to_string(self.manual_path())
            .ok()
            .and_then(|raw| serde_json::from_str::<PipelineSession>(&raw).ok())
            .map(|s| s.target)
    }

    pub fn start(&mut self) {
        let Some(path) = self.selected_path() else {
            self.core.error = Some(TabError::Target(
                "No saved session selected. Choose one from the list, or type a \
                 session file path."
                    .to_string(),
            ));
            return;
        };
        if path.trim().is_empty() {
            self.core.error = Some(TabError::Config(
                "The session file path is blank. Clear the field to use the \
                 highlighted session."
                    .to_string(),
            ));
            return;
        }
        // A checkpoint exists so an interrupted scan can continue. Once every
        // stage has completed there is nothing left to resume, and dispatching
        // it would start a no-op run whose result reads like a real one. Check
        // only for a listed row: a manual path may point at a checkpoint the
        // store never indexed, and refusing that would hide a resumable file.
        if self.manual_path().is_empty() {
            if let Some(entry) = self.entries.get(self.selected) {
                if !entry.is_resumable() {
                    // Distinguish the two reasons. A clean finish has nothing
                    // left to do; a run that ended with failed stages cannot
                    // retry them by resuming, so it needs a fresh scan. Calling
                    // the second "complete" would hide a broken assessment.
                    let message = if entry.finalized && entry.failed_stages > 0 {
                        format!(
                            "This scan ended with {} failed stage(s) and no stages \
                             left, so resuming would do nothing. Re-run the scan to \
                             retry them.",
                            entry.failed_stages
                        )
                    } else {
                        format!(
                            "This scan already completed all {} stage(s), so there is \
                             nothing to resume. Pick an interrupted scan, or type a \
                             session file path.",
                            entry.completed_stages
                        )
                    };
                    self.core.error = Some(TabError::Config(message));
                    return;
                }
            }
        }
        self.core.error = None;
        if self.core.state != AppState::Running {
            self.core.progress.current = 0;
            self.core.progress.total = 0;
            self.core.state = AppState::Running;
        }
    }

    /// Re-read the session store.
    ///
    /// Called from `ensure_input_focus`, which the app already invokes on every
    /// tab-entry path, so the list is fresh whenever the tab is opened without
    /// adding a timer. It is not a poll: nothing refreshes the list while the
    /// tab sits open.
    pub fn refresh(&mut self) {
        let dir = eggsec::pipeline::session::default_session_dir();
        self.entries = eggsec::pipeline::session::list_sessions(&dir);
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
        self.render_list();
    }

    /// Move the list selection down, clamped at the last row.
    pub fn select_next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.entries.len() - 1);
        self.render_list();
    }

    /// Move the list selection up, clamped at the first row.
    pub fn select_previous(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.render_list();
    }

    /// Rebuild the list pane from the current entries and selection.
    ///
    /// The marker matters: without it the operator cannot tell which row Enter
    /// would resume, and a list of similar targets is not self-identifying.
    fn render_list(&mut self) {
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};
        let theme = crate::theme::legacy::current_theme();

        if self.entries.is_empty() {
            self.core.results_view = crate::components::ScrollableText::new("Saved Sessions");
            return;
        }

        let lines: Vec<Line<'static>> = self
            .entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let selected = idx == self.selected;
                let style = if selected {
                    Style::default()
                        .fg(theme.colors.highlight)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.text)
                };
                let marker = if selected { "▶ " } else { "  " };
                Line::from(Span::styled(format!("{marker}{}", entry.label()), style))
            })
            .collect();
        self.core.results_view =
            crate::components::ScrollableText::new("Saved Sessions").with_lines(lines);
    }
}

impl TabState for ResumeTab {
    tab_state_boilerplate!(ResumeTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.focus_area = StandardFocusArea2::Inputs;
        self.selected = 0;
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

        // The list is the primary control, so the pane reports the selection
        // via a `▶` marker rather than a bare row count.
        let empty_text: &'static str = "No saved sessions found.\nRun a scan with a \
             .session.json output, or type a session file path.";

        render_results_area(
            f,
            results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Saved Sessions",
            // Short lines on purpose: `empty_state_paragraph` does not wrap, so
            // anything wider than the pane would be clipped mid-sentence.
            empty_text,
        );
    }
}

impl TabInput for ResumeTab {
    // `tab_input_boilerplate!` rather than `tab_input_2area!`: the area macro
    // pre-defines `handle_up`/`handle_down`, which scroll `results_view`
    // without moving this tab's `selected` index. That would scroll the list
    // while the `▶` marker stayed put, so Enter would resume a different row
    // than the one on screen. List navigation has to own those handlers.
    tab_input_boilerplate!(
        ResumeTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea2::Inputs,
        Results: StandardFocusArea2::Results
    );

    fn handle_char(&mut self, c: char) {
        let running = self.is_running();
        let on_inputs = self.focus_area == StandardFocusArea2::Inputs;
        crate::tabs::core::tab_input_char(&mut self.core, c, running, on_inputs);
    }

    fn handle_backspace(&mut self) {
        let running = self.is_running();
        let on_inputs = self.focus_area == StandardFocusArea2::Inputs;
        crate::tabs::core::tab_input_backspace(&mut self.core, running, on_inputs);
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        if self.core.inputs.is_focused() {
            self.core.inputs.blur();
        }
        // Enter resumes from either area, so the operator never has to move
        // focus back to the field to start.
        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        self.core.inputs.blur();
        self.focus_area = StandardFocusArea2::Results;
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.blur();
            self.focus_area = StandardFocusArea2::Results;
        } else {
            self.core.inputs.focus(0);
            self.focus_area = StandardFocusArea2::Inputs;
        }
    }

    fn handle_focus_prev(&mut self) {
        self.handle_focus_next();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == StandardFocusArea2::Results {
            self.select_previous();
        } else {
            self.core.inputs.blur();
            self.focus_area = StandardFocusArea2::Results;
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == StandardFocusArea2::Results {
            self.select_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.move_right()
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        crate::tabs::core::is_input_focused(self.focus_area, StandardFocusArea2::Inputs, &self.core)
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea2::Inputs {
            self.core.inputs.is_at_right_edge()
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tabs::AppState;

    /// A typed path is dispatched, not refused: the manual field exists so a
    /// checkpoint outside the session store is still reachable, and refusing
    /// it made the tab unable to do the one thing it advertises.
    #[test]
    fn start_with_manual_session_path_dispatches() {
        use crate::app::task_management::TaskBuilder;
        use eggsec_runtime::request::TaskKind;

        let mut tab = ResumeTab::new();
        tab.core.inputs.fields.get_mut(0).unwrap().value = "/tmp/scan.session.json".into();
        tab.start();

        assert!(tab.core.error.is_none(), "{:?}", tab.core.error);
        assert_eq!(tab.core.state, AppState::Running);

        let req = tab
            .build_run_request()
            .expect("a selected session must produce a request");
        let TaskKind::Resume(params) = req.task_kind else {
            panic!("expected a Resume task kind, got {:?}", req.task_kind);
        };
        assert_eq!(params.session_path, "/tmp/scan.session.json");
    }

    /// The highlighted row is the default selection, and Enter resumes it.
    #[test]
    fn start_with_no_path_and_no_sessions_reports_a_clear_error() {
        let mut tab = ResumeTab::new();
        tab.focus_area = StandardFocusArea2::Inputs;
        tab.handle_enter();
        assert!(matches!(tab.core.error, Some(TabError::Target(_))));
        assert_eq!(tab.core.state, AppState::Idle, "must not fake a run");
    }

    /// The manual field must take precedence over the highlighted row,
    /// otherwise an operator who typed a path silently resumes the wrong
    /// session because the marker happens to sit on row 0.
    #[test]
    fn manual_path_wins_over_the_selected_row() {
        let dir = std::env::temp_dir().join(format!("eggsec-resume-tab-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let listed = dir.join("listed.session.json");
        std::fs::write(&listed, "{}").unwrap();

        let mut tab = ResumeTab::new();
        tab.entries = vec![SessionEntry {
            path: listed,
            target: "example.test".into(),
            completed_stages: 1,
            remaining_stages: 2,
            modified_epoch_secs: 0,
            finalized: true,
            failed_stages: 0,
        }];
        tab.core.inputs.fields.get_mut(0).unwrap().value = "/tmp/manual.json".into();

        assert_eq!(tab.selected_path().as_deref(), Some("/tmp/manual.json"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Without a manual path the highlighted row is what gets resumed.
    #[test]
    fn selected_row_is_used_when_no_manual_path() {
        let mut tab = ResumeTab::new();
        let path = std::path::PathBuf::from("/tmp/listed.session.json");
        tab.entries = vec![SessionEntry {
            path: path.clone(),
            target: "example.test".into(),
            completed_stages: 0,
            remaining_stages: 3,
            modified_epoch_secs: 0,
            finalized: true,
            failed_stages: 0,
        }];
        assert_eq!(tab.selected_path().as_deref(), Some(path.to_str().unwrap()));
        assert_eq!(tab.selected_target().as_deref(), Some("example.test"));
    }

    /// Navigation must be clamped, not wrap: wrapping past the end would make
    /// Enter resume a session the operator never highlighted.
    #[test]
    fn list_navigation_is_clamped_to_the_rows() {
        let mut tab = ResumeTab::new();
        tab.entries = (0..2)
            .map(|i| SessionEntry {
                path: std::path::PathBuf::from(format!("/tmp/{i}.session.json")),
                target: format!("host{i}.test"),
                completed_stages: 0,
                remaining_stages: 1,
                modified_epoch_secs: 0,
                finalized: true,
                failed_stages: 0,
            })
            .collect();
        tab.render_list();

        tab.select_previous();
        assert_eq!(tab.selected, 0, "must not wrap above the first row");
        tab.select_next();
        tab.select_next();
        assert_eq!(tab.selected, 1, "must not wrap past the last row");
        tab.select_next();
        assert_eq!(tab.selected, 1);
    }

    /// A completed checkpoint has nothing to resume, so it must be refused
    /// rather than dispatched: a no-op run whose result reads like a real one
    /// is worse than an explanation.
    #[test]
    fn completed_checkpoint_is_refused_instead_of_dispatched() {
        let mut tab = ResumeTab::new();
        tab.entries = vec![SessionEntry {
            path: std::path::PathBuf::from("/tmp/done.session.json"),
            target: "example.test".into(),
            completed_stages: 5,
            remaining_stages: 0,
            modified_epoch_secs: 0,
            finalized: true,
            failed_stages: 0,
        }];
        tab.start();
        assert!(
            tab.core.error.is_some(),
            "a finished scan must not be dispatched as a no-op resume"
        );
        assert_eq!(tab.core.state, AppState::Idle, "must not fake a run");
    }

    /// A run that ended with failed stages must not be reported as "complete" —
    /// that would hide a broken assessment behind a reassuring label.
    #[test]
    fn failed_final_run_is_labelled_as_failed_not_complete() {
        let entry = SessionEntry {
            path: std::path::PathBuf::from("/tmp/partial.session.json"),
            target: "example.test".into(),
            completed_stages: 3,
            remaining_stages: 0,
            modified_epoch_secs: 0,
            finalized: true,
            failed_stages: 2,
        };
        let label = entry.label();
        assert!(!label.contains("complete"), "got {label:?}");
        assert!(label.contains("2 failed"), "got {label:?}");

        let mut tab = ResumeTab::new();
        tab.entries = vec![entry];
        tab.start();
        let msg = format!("{:?}", tab.core.error);
        assert!(msg.contains("Re-run the scan"), "got {msg}");
    }

    /// A manual path may point at a checkpoint the store never indexed, so it
    /// is not second-guessed from the row list — refusing it would hide a
    /// genuinely resumable file.
    #[test]
    fn manual_path_is_not_gated_on_the_row_list() {
        let mut tab = ResumeTab::new();
        tab.entries = vec![SessionEntry {
            path: std::path::PathBuf::from("/tmp/done.session.json"),
            target: "example.test".into(),
            completed_stages: 5,
            remaining_stages: 0,
            modified_epoch_secs: 0,
            finalized: true,
            failed_stages: 0,
        }];
        tab.core.inputs.fields.get_mut(0).unwrap().value = "/tmp/manual.json".into();
        tab.start();
        assert!(
            tab.core.error.is_none(),
            "a manual path must dispatch unchallenged: {:?}",
            tab.core.error
        );
    }

    /// The rendered list must mark the selected row, or Enter resumes a
    /// session the operator cannot tell is selected.
    #[test]
    fn list_marks_the_selected_row() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut tab = ResumeTab::new();
        tab.entries = (0..2)
            .map(|i| SessionEntry {
                path: std::path::PathBuf::from(format!("/tmp/{i}.session.json")),
                target: format!("host{i}.test"),
                completed_stages: 1,
                remaining_stages: 2,
                modified_epoch_secs: 0,
                finalized: true,
                failed_stages: 0,
            })
            .collect();
        tab.selected = 1;
        tab.render_list();

        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                tab.render(f, area, false);
            })
            .unwrap();
        let text = crate::test_utils::buffer_to_text(terminal.backend().buffer());
        assert!(text.contains("host1.test"), "row must be listed:\n{text}");
        assert!(
            text.contains('\u{25b6}'),
            "the selected row must carry the marker:\n{text}"
        );
    }

    /// The empty state must tell the operator how to get a session, not
    /// promise a run the tab cannot perform.
    #[test]
    fn empty_state_explains_how_to_get_a_session() {
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
        assert!(text.contains("No saved sessions"), "got:\n{text}");
        assert!(
            !text.contains("Session information will appear here"),
            "empty state must not promise a run:\n{text}"
        );
    }
}
