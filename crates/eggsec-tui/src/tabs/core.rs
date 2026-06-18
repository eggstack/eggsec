use crate::app::tab_error::TabError;
use crate::components::{empty_state_paragraph, InputGroup, ProgressGauge, ScrollableText};
use crate::tabs::AppState;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::fmt;

/// Generic focus area for tabs with Inputs/Options/Results layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardFocusArea {
    Inputs,
    Options,
    Results,
}

/// Shared state fields common to most tabs.
///
/// Tabs embed this struct and add their specific fields. This eliminates
/// the duplication of `state`, `progress`, `results_view`, `error`, and
/// `inputs` across 20+ tab implementations.
pub struct TabCore {
    pub inputs: InputGroup,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub error: Option<TabError>,
}

impl TabCore {
    pub fn new(progress_label: impl Into<String>, results_title: impl Into<String>) -> Self {
        Self {
            inputs: InputGroup::new(),
            progress: ProgressGauge::new(progress_label),
            state: AppState::Idle,
            results_view: ScrollableText::new(results_title),
            error: None,
        }
    }

    pub fn with_inputs(mut self, inputs: InputGroup) -> Self {
        self.inputs = inputs;
        self
    }

    /// Returns the value of the first input field (commonly the target).
    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    /// Splits the target field by comma/newline into multiple targets.
    pub fn targets(&self) -> Vec<String> {
        let target = self.target();
        if target.is_empty() {
            return Vec::new();
        }
        target
            .split([',', '\n', '\r'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Returns true if there are multiple targets.
    pub fn is_multi_target(&self) -> bool {
        self.targets().len() > 1
    }

    /// Resets all shared fields to defaults. Tab-specific reset logic
    /// should call this then handle its own fields.
    pub fn reset_all(&mut self) {
        self.state = AppState::Idle;
        self.progress.current = 0;
        self.progress.total = 0;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }

    /// Sets the tab to Idle state.
    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    /// Updates progress counters.
    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.progress.current = completed;
        self.progress.total = total;
    }

    pub fn scroll_results_up(&mut self) {
        self.results_view.scroll_up(1);
    }

    pub fn scroll_results_down(&mut self) {
        self.results_view.scroll_down(1);
    }
}

// --- TabState delegation helpers ---

/// Implements the common `TabState` methods by delegating to `TabCore`.
/// Used by the `tab_impl!` macro.
pub fn tab_state_state(core: &TabCore) -> AppState {
    core.state.clone()
}

pub fn tab_state_progress(core: &TabCore) -> f64 {
    core.progress.percent() as f64
}

pub fn tab_state_set_error(core: &mut TabCore, error: TabError) {
    core.state = AppState::Error(error.message());
    core.error = Some(error);
    core.progress.current = 0;
}

// --- TabInput delegation helpers ---
// These implement the boilerplate input handling that is identical across
// nearly all tabs. Tabs with custom behavior (like ScanPorts' validation)
// override the relevant methods.

/// Common `handle_char` implementation.
pub fn tab_input_char(core: &mut TabCore, c: char, is_running: bool, is_inputs: bool) {
    if !is_running && is_inputs {
        core.inputs.insert(c);
    }
}

/// Common `handle_backspace` implementation.
pub fn tab_input_backspace(core: &mut TabCore, is_running: bool, is_inputs: bool) {
    if !is_running && is_inputs {
        core.inputs.backspace();
    }
}

/// Common `handle_paste` implementation.
pub fn tab_input_paste(core: &mut TabCore, text: &str, is_running: bool, is_inputs: bool) {
    if !is_running && is_inputs {
        core.inputs.paste(text);
    }
}

/// Common `handle_copy` implementation.
pub fn tab_input_copy(
    core: &TabCore,
    is_running: bool,
    is_inputs: bool,
    is_results: bool,
) -> Option<String> {
    if is_running {
        return None;
    }
    if is_results {
        return Some(core.results_view.get_content());
    }
    if is_inputs {
        return core.inputs.get_focused_value();
    }
    None
}

/// Common `handle_word_forward` implementation.
pub fn tab_input_word_forward(core: &mut TabCore, is_running: bool, is_inputs: bool) {
    if !is_running && is_inputs {
        core.inputs.move_word_forward();
    }
}

/// Common `handle_word_backward` implementation.
pub fn tab_input_word_backward(core: &mut TabCore, is_running: bool, is_inputs: bool) {
    if !is_running && is_inputs {
        core.inputs.move_word_backward();
    }
}

/// Common `handle_home` implementation.
pub fn tab_input_home(core: &mut TabCore, is_running: bool, is_inputs: bool, is_results: bool) {
    if !is_running {
        if is_inputs {
            core.inputs.move_home();
        } else if is_results {
            core.results_view.scroll_to_top();
        }
    }
}

/// Common `handle_end` implementation.
pub fn tab_input_end(core: &mut TabCore, is_running: bool, is_inputs: bool, is_results: bool) {
    if !is_running {
        if is_inputs {
            core.inputs.move_end();
        } else if is_results {
            core.results_view.scroll_to_bottom();
        }
    }
}

/// Common `handle_top` implementation.
pub fn tab_input_top(core: &mut TabCore, is_running: bool) {
    if !is_running {
        core.inputs.blur();
        core.inputs.focus(0);
    }
}

/// Common `handle_bottom` implementation.
pub fn tab_input_bottom(core: &mut TabCore, is_running: bool) {
    if !is_running {
        core.inputs.blur();
    }
}

/// Common `page_up` implementation.
pub fn tab_input_page_up(core: &mut TabCore, is_running: bool, page_size: usize) {
    if !is_running {
        core.results_view.page_up(page_size);
    }
}

/// Common `page_down` implementation.
pub fn tab_input_page_down(core: &mut TabCore, is_running: bool, page_size: usize) {
    if !is_running {
        core.results_view.page_down(page_size);
    }
}

// --- Rendering helpers ---

/// Renders the standard 4-branch results area: Running -> Error -> Results -> Empty.
pub fn render_results_area(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    error: &Option<TabError>,
    results_view: &ScrollableText,
    progress: &ProgressGauge,
    empty_title: &'static str,
    empty_text: &'static str,
) {
    match state {
        AppState::Running => {
            progress.render(f, area);
        }
        AppState::Error(_) => {
            if let Some(ref err) = error {
                let error_text = Paragraph::new(format!("Error: {}", err.message()))
                    .style(Style::default().fg(crate::tc!(error)));
                f.render_widget(error_text, area);
            }
        }
        _ => {
            if !results_view.is_empty() {
                results_view.render(f, area, None);
            } else {
                let placeholder = empty_state_paragraph(empty_title, empty_text);
                f.render_widget(placeholder, area);
            }
        }
    }
}

/// Renders the standard configuration block with focused/unfocused border styling.
pub fn render_config_block(
    f: &mut Frame,
    area: Rect,
    title: &str,
    is_config_focused: bool,
) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Style::default().fg(if is_config_focused {
            crate::tc!(border_focused)
        } else {
            crate::tc!(border)
        }));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

impl fmt::Debug for TabCore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabCore")
            .field("state", &self.state)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_core_target_empty() {
        let core = TabCore::new("test", "Results");
        assert_eq!(core.target(), "");
    }

    #[test]
    fn tab_core_target_first_field() {
        let core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new().add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        assert_eq!(core.target(), "example.com");
    }

    #[test]
    fn tab_core_targets_split() {
        let core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("a.com,b.com,c.com")),
        );
        assert_eq!(core.targets(), vec!["a.com", "b.com", "c.com"]);
        assert!(core.is_multi_target());
    }

    #[test]
    fn tab_core_targets_single() {
        let core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        assert_eq!(core.targets(), vec!["example.com"]);
        assert!(!core.is_multi_target());
    }

    #[test]
    fn tab_core_targets_empty() {
        let core = TabCore::new("test", "Results");
        assert!(core.targets().is_empty());
        assert!(!core.is_multi_target());
    }

    #[test]
    fn tab_core_reset_all() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        core.state = AppState::Running;
        core.progress.current = 50;
        core.progress.total = 100;
        core.results_view.add_line(ratatui::text::Line::from("test"));
        core.error = Some(TabError::Target("err".to_string()));

        core.reset_all();

        assert_eq!(core.state, AppState::Idle);
        assert_eq!(core.progress.current, 0);
        assert_eq!(core.progress.total, 0);
        assert!(core.results_view.is_empty());
        assert!(core.error.is_none());
        assert_eq!(core.target(), "");
    }

    #[test]
    fn tab_core_stop() {
        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        core.stop();
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn tab_core_update_progress() {
        let mut core = TabCore::new("test", "Results");
        core.update_progress(42, 100);
        assert_eq!(core.progress.current, 42);
        assert_eq!(core.progress.total, 100);
    }

    #[test]
    fn tab_state_helpers() {
        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        assert_eq!(tab_state_state(&core), AppState::Running);

        core.progress.current = 50;
        core.progress.total = 100;
        assert_eq!(tab_state_progress(&core), 50.0);

        tab_state_set_error(&mut core, TabError::Target("test error".to_string()));
        assert!(matches!(core.state, AppState::Error(_)));
        assert_eq!(core.progress.current, 0);
    }

    #[test]
    fn tab_input_helpers() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        core.inputs.focus(0);

        // char
        tab_input_char(&mut core, 'a', false, true);
        assert_eq!(core.target(), "a");

        // backspace
        tab_input_backspace(&mut core, false, true);
        assert_eq!(core.target(), "");

        // paste
        tab_input_paste(&mut core, "hello", false, true);
        assert_eq!(core.target(), "hello");

        // copy
        core.inputs.focus(0);
        let copied = tab_input_copy(&core, false, true, false);
        assert_eq!(copied, Some("hello".to_string()));

        // word operations
        tab_input_word_forward(&mut core, false, true);
        tab_input_word_backward(&mut core, false, true);

        // home/end
        tab_input_home(&mut core, false, true, false);
        tab_input_end(&mut core, false, true, false);

        // top/bottom
        tab_input_top(&mut core, false);
        tab_input_bottom(&mut core, false);

        // page up/down
        tab_input_page_up(&mut core, false, 20);
        tab_input_page_down(&mut core, false, 20);
    }

    #[test]
    fn tab_input_noop_when_running() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        core.state = AppState::Running;

        tab_input_char(&mut core, 'a', true, true);
        assert_eq!(core.target(), "");

        let copied = tab_input_copy(&core, true, true, false);
        assert!(copied.is_none());
    }

    #[test]
    fn render_results_area_running() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let core = TabCore::new("test", "Results");
        terminal
            .draw(|f| {
                render_results_area(
                    f,
                    Rect::new(0, 0, 80, 24),
                    &AppState::Running,
                    &None,
                    &core.results_view,
                    &core.progress,
                    "Results",
                    "empty",
                );
            })
            .unwrap();
    }

    #[test]
    fn render_results_area_empty() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let core = TabCore::new("test", "Results");
        terminal
            .draw(|f| {
                render_results_area(
                    f,
                    Rect::new(0, 0, 80, 24),
                    &AppState::Idle,
                    &None,
                    &core.results_view,
                    &core.progress,
                    "Results",
                    "Results will appear here",
                );
            })
            .unwrap();
    }

    #[test]
    fn render_results_area_error() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let core = TabCore::new("test", "Results");
        let err = Some(TabError::Target("test error".to_string()));
        terminal
            .draw(|f| {
                render_results_area(
                    f,
                    Rect::new(0, 0, 80, 24),
                    &AppState::Error("test error".to_string()),
                    &err,
                    &core.results_view,
                    &core.progress,
                    "Results",
                    "empty",
                );
            })
            .unwrap();
    }
}
