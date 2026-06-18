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

/// Generic field accessor: parse field at `index` as `T`, returning `default` on failure.
pub fn field_as<T: std::str::FromStr>(core: &TabCore, index: usize, default: T) -> T {
    core.inputs
        .fields
        .get(index)
        .and_then(|f| f.value.parse().ok())
        .unwrap_or(default)
}

/// Generic field string accessor: return field value at `index` as `&str`.
pub fn field_str(core: &TabCore, index: usize) -> &str {
    core.inputs
        .fields
        .get(index)
        .map(|f| f.value.as_str())
        .unwrap_or("")
}

/// Starts a scan: sets state to Running, clears results and error.
/// Returns true if target is non-empty (scan started), false otherwise.
pub fn start_scan(core: &mut TabCore) -> bool {
    if !core.target().is_empty() {
        core.state = AppState::Running;
        core.progress.current = 0;
        core.results_view.clear();
        core.error = None;
        true
    } else {
        false
    }
}

// --- Focus navigation helpers ---

/// Focus area cycle for 3-area tabs (Inputs → Options → Results → Inputs).
pub fn focus_next_3area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    options: A,
    results: A,
) -> A {
    match current {
        _ if current == inputs => {
            core.inputs.blur();
            options
        }
        _ if current == options => results,
        _ if current == results => {
            core.inputs.focus(0);
            inputs
        }
        _ => current,
    }
}

/// Focus area cycle for 3-area tabs (reverse: Inputs → Results → Options → Inputs).
pub fn focus_prev_3area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    options: A,
    results: A,
) -> A {
    match current {
        _ if current == inputs => {
            core.inputs.blur();
            results
        }
        _ if current == options => {
            core.inputs.focus(0);
            inputs
        }
        _ if current == results => options,
        _ => current,
    }
}

/// Focus area cycle for 2-area tabs (Inputs → Results → Inputs).
pub fn focus_next_2area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    results: A,
) -> A {
    if current == inputs {
        core.inputs.blur();
        results
    } else {
        core.inputs.focus(0);
        inputs
    }
}

/// Focus area cycle for 2-area tabs (reverse: same as forward for 2-area).
pub fn focus_prev_2area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    results: A,
) -> A {
    if current == results {
        core.inputs.focus(inputs_field_count(core));
        inputs
    } else {
        core.inputs.blur();
        results
    }
}

fn inputs_field_count(core: &TabCore) -> usize {
    core.inputs.fields.len().saturating_sub(1)
}

/// Common `handle_up` for 3-area tabs where Options area has no vertical navigation.
pub fn handle_up_3area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    results: A,
) {
    if current == inputs {
        if !core.inputs.is_focused() && !core.results_view.is_empty() {
            core.scroll_results_up();
        } else {
            core.inputs.focus_prev();
        }
    } else if current == results {
        core.scroll_results_up();
    }
}

/// Common `handle_down` for 3-area tabs where Options area has no vertical navigation.
pub fn handle_down_3area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    results: A,
) {
    if current == inputs {
        if !core.inputs.is_focused() && !core.results_view.is_empty() {
            core.scroll_results_down();
        } else {
            core.inputs.focus_next();
        }
    } else if current == results {
        core.scroll_results_down();
    }
}

/// Common `handle_up` for 2-area tabs.
pub fn handle_up_2area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    results: A,
) {
    if current == inputs {
        if !core.inputs.is_focused() && !core.results_view.is_empty() {
            core.scroll_results_up();
        } else {
            core.inputs.focus_prev();
        }
    } else if current == results {
        core.scroll_results_up();
    }
}

/// Common `handle_down` for 2-area tabs.
pub fn handle_down_2area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    results: A,
) {
    if current == inputs {
        if !core.inputs.is_focused() && !core.results_view.is_empty() {
            core.scroll_results_down();
        } else {
            core.inputs.focus_next();
        }
    } else if current == results {
        core.scroll_results_down();
    }
}

/// Common `handle_left` for simple tabs (Inputs area only).
pub fn handle_left_simple(core: &mut TabCore, is_running: bool) -> bool {
    if !is_running {
        core.inputs.move_left()
    } else {
        false
    }
}

/// Common `handle_right` for simple tabs (Inputs area only).
pub fn handle_right_simple(core: &mut TabCore, is_running: bool) -> bool {
    if !is_running {
        core.inputs.move_right()
    } else {
        false
    }
}

/// Common `is_input_focused` check.
pub fn is_input_focused<A: Copy + PartialEq>(current: A, inputs: A, core: &TabCore) -> bool {
    current == inputs && core.inputs.is_focused()
}

/// Common `is_at_left_edge` for Inputs/Results tabs.
pub fn is_at_left_edge_simple<A: Copy + PartialEq>(current: A, inputs: A, core: &TabCore) -> bool {
    if current == inputs {
        core.inputs.is_at_left_edge()
    } else {
        true
    }
}

/// Common `is_at_right_edge` for Inputs/Results tabs.
pub fn is_at_right_edge_simple<A: Copy + PartialEq>(
    current: A,
    inputs: A,
    core: &TabCore,
) -> bool {
    if current == inputs {
        core.inputs.is_at_right_edge()
    } else {
        true
    }
}

/// Common `handle_enter` guard pattern for simple tabs.
/// Returns an enum indicating what action should be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterAction {
    /// Results area: no-op
    NoOp,
    /// Running: stop
    Stop,
    /// Inputs focused: blur
    BlurInputs,
    /// Default: start the scan
    Start,
}

/// Evaluates the common `handle_enter` guard pattern.
pub fn evaluate_enter<A: Copy + PartialEq>(
    current: A,
    inputs: A,
    results: A,
    is_running: bool,
    inputs_focused: bool,
) -> EnterAction {
    if current == results || is_running {
        if is_running {
            EnterAction::Stop
        } else {
            EnterAction::NoOp
        }
    } else if current == inputs && inputs_focused {
        EnterAction::BlurInputs
    } else {
        EnterAction::Start
    }
}

/// Executes the action returned by `evaluate_enter` for simple 2-area tabs.
/// Tabs that need custom behavior in the Options area should call this after
/// their own check, or use `handle_enter_3area` instead.
pub fn execute_enter_action(core: &mut TabCore, action: EnterAction) {
    match action {
        EnterAction::NoOp => {}
        EnterAction::Stop => core.stop(),
        EnterAction::BlurInputs => core.inputs.blur(),
        EnterAction::Start => {
            start_scan(core);
        }
    }
}

/// Common `handle_enter` for 3-area tabs (Inputs/Options/Results).
/// The `options_action` callback is invoked when Enter is pressed in the Options
/// area (e.g., to toggle a checkbox or confirm a selector).
/// - For simple toggle-and-return: `options_action` returns `false` (no scan start).
/// - For toggle-then-start (e.g., recon): `options_action` returns `true` to start.
pub fn handle_enter_3area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    options: A,
    results: A,
    is_running: bool,
    inputs_focused: bool,
    options_action: impl FnOnce(&mut TabCore) -> bool,
) {
    if current == results || is_running {
        if is_running {
            core.stop();
        }
        return;
    }
    if current == inputs && inputs_focused {
        core.inputs.blur();
        return;
    }
    if current == options {
        if options_action(core) {
            start_scan(core);
        }
        return;
    }
    start_scan(core);
}

/// Common `handle_escape` for simple 2-area tabs (Inputs/Results).
/// If running, stops. If in Inputs, blurs. If in Results, switches to Inputs and focuses first field.
/// Returns the new focus area.
pub fn handle_escape_simple<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
) -> A {
    if core.state == AppState::Running {
        core.stop();
        current
    } else if current == inputs {
        core.inputs.blur();
        current
    } else {
        core.inputs.focus(0);
        inputs
    }
}

/// Common `handle_escape` for 3-area tabs (Inputs/Options/Results).
/// If running, stops. If in Options or Results, returns to Inputs and focuses first field.
/// If in Inputs, blurs the current field.
pub fn handle_escape_3area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    _options: A,
    _results: A,
) -> A {
    if core.state == AppState::Running {
        core.stop();
        current
    } else if current == inputs {
        core.inputs.blur();
        current
    } else {
        core.inputs.focus(0);
        inputs
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

    #[test]
    fn field_as_returns_parsed_value() {
        let core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("URL"))
                .add(crate::components::InputField::new("Port").with_value("8080")),
        );
        assert_eq!(field_as::<usize>(&core, 1, 80), 8080);
    }

    #[test]
    fn field_as_returns_default_on_empty() {
        let core = TabCore::new("test", "Results");
        assert_eq!(field_as::<usize>(&core, 0, 42), 42);
    }

    #[test]
    fn field_as_returns_default_on_parse_error() {
        let core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Port").with_value("not_a_number")),
        );
        assert_eq!(field_as::<u64>(&core, 0, 99), 99);
    }

    #[test]
    fn field_str_returns_value() {
        let core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        assert_eq!(field_str(&core, 0), "example.com");
    }

    #[test]
    fn field_str_returns_empty_on_missing() {
        let core = TabCore::new("test", "Results");
        assert_eq!(field_str(&core, 0), "");
    }

    #[test]
    fn start_scan_with_target_succeeds() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        assert!(start_scan(&mut core));
        assert_eq!(core.state, AppState::Running);
        assert_eq!(core.progress.current, 0);
        assert!(core.results_view.is_empty());
        assert!(core.error.is_none());
    }

    #[test]
    fn start_scan_empty_target_fails() {
        let mut core = TabCore::new("test", "Results");
        assert!(!start_scan(&mut core));
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn focus_next_3area_cycles_correctly() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );

        // Inputs -> Options
        let next = focus_next_3area(&mut core, Area::Inputs, Area::Inputs, Area::Options, Area::Results);
        assert_eq!(next, Area::Options);

        // Options -> Results
        let next = focus_next_3area(&mut core, Area::Options, Area::Inputs, Area::Options, Area::Results);
        assert_eq!(next, Area::Results);

        // Results -> Inputs (focuses first field)
        let next = focus_next_3area(&mut core, Area::Results, Area::Inputs, Area::Options, Area::Results);
        assert_eq!(next, Area::Inputs);
        assert!(core.inputs.is_focused());
    }

    #[test]
    fn focus_prev_3area_cycles_correctly() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );

        // Inputs -> Results (blurs)
        let prev = focus_prev_3area(&mut core, Area::Inputs, Area::Inputs, Area::Options, Area::Results);
        assert_eq!(prev, Area::Results);

        // Results -> Options
        let prev = focus_prev_3area(&mut core, Area::Results, Area::Inputs, Area::Options, Area::Results);
        assert_eq!(prev, Area::Options);

        // Options -> Inputs (focuses first field)
        let prev = focus_prev_3area(&mut core, Area::Options, Area::Inputs, Area::Options, Area::Results);
        assert_eq!(prev, Area::Inputs);
    }

    #[test]
    fn evaluate_enter_results_no_op() {
        let action = evaluate_enter(1, 0, 1, false, false);
        assert_eq!(action, EnterAction::NoOp);
    }

    #[test]
    fn evaluate_enter_running_stops() {
        let action = evaluate_enter(0, 0, 1, true, false);
        assert_eq!(action, EnterAction::Stop);
    }

    #[test]
    fn evaluate_enter_inputs_focused_blurs() {
        let action = evaluate_enter(0, 0, 1, false, true);
        assert_eq!(action, EnterAction::BlurInputs);
    }

    #[test]
    fn evaluate_enter_inputs_unfocused_starts() {
        let action = evaluate_enter(0, 0, 1, false, false);
        assert_eq!(action, EnterAction::Start);
    }

    #[test]
    fn handle_escape_simple_stops_when_running() {
        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        let result = handle_escape_simple(&mut core, 1, 0);
        assert_eq!(result, 1); // Returns current area
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_escape_simple_blurs_when_in_inputs() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        core.inputs.focus(0);
        let result = handle_escape_simple(&mut core, 0, 0);
        assert_eq!(result, 0); // Stays in inputs area
        assert!(!core.inputs.is_focused());
    }

    #[test]
    fn handle_escape_simple_focuses_inputs_when_from_results() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        let result = handle_escape_simple(&mut core, 1, 0);
        assert_eq!(result, 0); // Returns inputs area
        assert!(core.inputs.is_focused());
    }

    #[test]
    fn execute_enter_action_stop() {
        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        execute_enter_action(&mut core, EnterAction::Stop);
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn execute_enter_action_blur_inputs() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        core.inputs.focus(0);
        execute_enter_action(&mut core, EnterAction::BlurInputs);
        assert!(!core.inputs.is_focused());
    }

    #[test]
    fn execute_enter_action_start_with_target() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        execute_enter_action(&mut core, EnterAction::Start);
        assert_eq!(core.state, AppState::Running);
    }

    #[test]
    fn execute_enter_action_start_empty_is_noop() {
        let mut core = TabCore::new("test", "Results");
        execute_enter_action(&mut core, EnterAction::Start);
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn execute_enter_action_noop() {
        let mut core = TabCore::new("test", "Results");
        execute_enter_action(&mut core, EnterAction::NoOp);
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_enter_3area_results_no_op() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        handle_enter_3area(
            &mut core, Area::Results, Area::Inputs, Area::Options, Area::Results,
            false, false,
            |_| false,
        );
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_enter_3area_running_stops() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        handle_enter_3area(
            &mut core, Area::Inputs, Area::Inputs, Area::Options, Area::Results,
            true, false,
            |_| false,
        );
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_enter_3area_inputs_focused_blurs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        core.inputs.focus(0);
        handle_enter_3area(
            &mut core, Area::Inputs, Area::Inputs, Area::Options, Area::Results,
            false, true,
            |_| false,
        );
        assert!(!core.inputs.is_focused());
    }

    #[test]
    fn handle_enter_3area_options_calls_action_no_start() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        let mut called = false;
        handle_enter_3area(
            &mut core, Area::Options, Area::Inputs, Area::Options, Area::Results,
            false, false,
            |_| { called = true; false },
        );
        assert!(called);
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_enter_3area_options_starts_when_action_returns_true() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        handle_enter_3area(
            &mut core, Area::Options, Area::Inputs, Area::Options, Area::Results,
            false, false,
            |_| true,
        );
        assert_eq!(core.state, AppState::Running);
    }

    #[test]
    fn handle_enter_3area_inputs_unfocused_starts() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        handle_enter_3area(
            &mut core, Area::Inputs, Area::Inputs, Area::Options, Area::Results,
            false, false,
            |_| false,
        );
        assert_eq!(core.state, AppState::Running);
    }

    #[test]
    fn handle_escape_3area_running_stops() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        let result = handle_escape_3area(
            &mut core, Area::Options, Area::Inputs, Area::Options, Area::Results,
        );
        assert_eq!(result, Area::Options);
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_escape_3area_options_returns_to_inputs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        let result = handle_escape_3area(
            &mut core, Area::Options, Area::Inputs, Area::Options, Area::Results,
        );
        assert_eq!(result, Area::Inputs);
        assert!(core.inputs.is_focused());
    }

    #[test]
    fn handle_escape_3area_results_returns_to_inputs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        let result = handle_escape_3area(
            &mut core, Area::Results, Area::Inputs, Area::Options, Area::Results,
        );
        assert_eq!(result, Area::Inputs);
        assert!(core.inputs.is_focused());
    }

    #[test]
    fn handle_escape_3area_inputs_blurs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area { Inputs, Options, Results }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target")),
        );
        core.inputs.focus(0);
        let result = handle_escape_3area(
            &mut core, Area::Inputs, Area::Inputs, Area::Options, Area::Results,
        );
        assert_eq!(result, Area::Inputs);
        assert!(!core.inputs.is_focused());
    }
}
