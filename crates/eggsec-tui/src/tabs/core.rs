use crate::app::tab_error::TabError;
use crate::components::{
    empty_state_paragraph, Checkbox, InputGroup, ProgressGauge, ScrollableText,
};
use crate::tabs::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::fmt;

/// Generic focus area for tabs with Inputs/Options/Results layout.
/// Use this instead of defining a per-tab enum when the tab has exactly
/// these three focus areas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardFocusArea {
    Inputs,
    Options,
    Results,
}

/// Generic focus area for tabs with Inputs/Results layout (2-area tabs).
/// Use this instead of defining a per-tab enum when the tab has exactly
/// these two focus areas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardFocusArea2 {
    Inputs,
    Results,
}

/// Generic focus area for tabs with Selector/Inputs/Results layout (3-area tabs
/// where the middle area is a Selector dropdown rather than checkboxes).
/// Use this instead of defining a per-tab enum when the tab has exactly
/// these three focus areas with a selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardFocusAreaSelector {
    Selector,
    Inputs,
    Results,
}

/// Focus area for the Fuzz tab (6 areas: Inputs, Payload, Mode, Target, Checkbox, Results).
/// Defined here so the N-area focus helpers can reference it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuzzFocusArea {
    Inputs,
    PayloadSelector,
    ModeSelector,
    TargetSelector,
    MutationCheckbox,
    Results,
}

/// All focus areas for the Fuzz tab in tab-order.
pub const FUZZ_AREAS: [FuzzFocusArea; 6] = [
    FuzzFocusArea::Inputs,
    FuzzFocusArea::PayloadSelector,
    FuzzFocusArea::ModeSelector,
    FuzzFocusArea::TargetSelector,
    FuzzFocusArea::MutationCheckbox,
    FuzzFocusArea::Results,
];

/// Number of focus areas for the Fuzz tab.
pub const NUM_FUZZ_AREAS: usize = 6;

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

    /// Prepares the results view for new results: sets state to Completed,
    /// clears the results view, and returns a mutable reference for formatting.
    ///
    /// This eliminates the repetitive preamble in every tab's `set_results()`:
    /// ```ignore
    /// pub fn set_results(&mut self, results: MyResults) {
    ///     let view = self.core.prepare_results();
    ///     view.add_line(Line::from(Span::styled(...)));
    ///     // ... format results ...
    ///     self.results = Some(results);
    /// }
    /// ```
    pub fn prepare_results(&mut self) -> &mut ScrollableText {
        self.state = AppState::Completed;
        self.results_view.clear();
        &mut self.results_view
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

/// Common `handle_up` for 2-area (Inputs/Results) and 3-area (Inputs/Options/Results) tabs.
/// In the Inputs area: navigates input fields (or scrolls results if unfocused).
/// In the Results area: scrolls results up.
/// The Options area is a no-op (callers handle selector-specific up/down).
pub fn handle_up_2area<A: Copy + PartialEq>(core: &mut TabCore, current: A, inputs: A, results: A) {
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

/// Common `handle_down` for 2-area (Inputs/Results) and 3-area (Inputs/Options/Results) tabs.
/// In the Inputs area: navigates input fields (or scrolls results if unfocused).
/// In the Results area: scrolls results down.
/// The Options area is a no-op (callers handle selector-specific up/down).
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

/// Alias for `handle_up_2area`. The Options area is a no-op in vertical
/// navigation, so 3-area tabs use the same logic as 2-area tabs.
pub fn handle_up_3area<A: Copy + PartialEq>(core: &mut TabCore, current: A, inputs: A, results: A) {
    handle_up_2area(core, current, inputs, results);
}

/// Alias for `handle_down_2area`. The Options area is a no-op in vertical
/// navigation, so 3-area tabs use the same logic as 2-area tabs.
pub fn handle_down_3area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    results: A,
) {
    handle_down_2area(core, current, inputs, results);
}

/// Common `handle_left` for simple tabs (Inputs area only).
/// Delegates to `handle_left_n` with `inputs == current` for single-area tabs.
pub fn handle_left_simple(core: &mut TabCore, is_running: bool) -> bool {
    if !is_running {
        core.inputs.move_left()
    } else {
        false
    }
}

/// Common `handle_right` for simple tabs (Inputs area only).
/// Delegates to `handle_right_n` with `inputs == current` for single-area tabs.
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
pub fn is_at_right_edge_simple<A: Copy + PartialEq>(current: A, inputs: A, core: &TabCore) -> bool {
    if current == inputs {
        core.inputs.is_at_right_edge()
    } else {
        true
    }
}

pub fn render_checkbox_row(
    f: &mut Frame,
    chunks: &[Rect],
    checkboxes: &[&Checkbox],
    focused_index: usize,
    row_focused: bool,
) {
    for (idx, checkbox) in checkboxes.iter().enumerate() {
        if let Some(chunk) = chunks.get(idx) {
            checkbox.render_with_focus(row_focused && idx == focused_index, f, *chunk);
        }
    }
}

pub fn toggle_focused_checkbox(
    checkboxes: &mut [&mut Checkbox],
    focused_index: &mut usize,
) -> bool {
    if checkboxes.is_empty() {
        return false;
    }
    *focused_index = (*focused_index).min(checkboxes.len() - 1);
    checkboxes[*focused_index].toggle();
    true
}

/// Toggle the checkbox at `focused_index` within a `Vec<Checkbox>`.
/// Clamps the index to valid range before toggling.
pub fn toggle_focused_checkbox_vec(checkboxes: &mut [Checkbox], focused_index: &mut usize) -> bool {
    if checkboxes.is_empty() {
        return false;
    }
    *focused_index = (*focused_index).min(checkboxes.len() - 1);
    checkboxes[*focused_index].toggle();
    true
}

pub fn move_checkbox_focus_left(focused_index: &mut usize, checkbox_count: usize) -> bool {
    if checkbox_count == 0 {
        return false;
    }
    *focused_index = (*focused_index).min(checkbox_count - 1);
    if *focused_index > 0 {
        *focused_index -= 1;
    }
    true
}

pub fn move_checkbox_focus_right(focused_index: &mut usize, checkbox_count: usize) -> bool {
    if checkbox_count == 0 {
        return false;
    }
    *focused_index = (*focused_index).min(checkbox_count - 1);
    if *focused_index + 1 < checkbox_count {
        *focused_index += 1;
    }
    true
}

pub fn is_checkbox_focus_at_left_edge(focused_index: usize, checkbox_count: usize) -> bool {
    checkbox_count == 0 || focused_index == 0
}

pub fn is_checkbox_focus_at_right_edge(focused_index: usize, checkbox_count: usize) -> bool {
    checkbox_count == 0 || focused_index >= checkbox_count.saturating_sub(1)
}

/// Cycles checkbox focus up in the Options area (wrapping: 0 -> last).
pub fn handle_options_up_wrapping(focused_index: &mut usize, checkbox_count: usize) {
    if checkbox_count == 0 {
        return;
    }
    if *focused_index == 0 {
        *focused_index = checkbox_count - 1;
    } else {
        *focused_index = focused_index.saturating_sub(1);
    }
}

/// Cycles checkbox focus down in the Options area (wrapping: last -> 0).
pub fn handle_options_down_wrapping(focused_index: &mut usize, checkbox_count: usize) {
    if checkbox_count == 0 {
        return;
    }
    if *focused_index >= checkbox_count - 1 {
        *focused_index = 0;
    } else {
        *focused_index += 1;
    }
}

pub fn indexed_input_area_index<A: Copy + PartialEq>(
    current: A,
    input_areas: &[A],
) -> Option<usize> {
    input_areas.iter().position(|area| *area == current)
}

pub fn is_indexed_input_area<A: Copy + PartialEq>(current: A, input_areas: &[A]) -> bool {
    indexed_input_area_index(current, input_areas).is_some()
}

pub fn sync_indexed_input_focus<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    input_areas: &[A],
) {
    let focused = indexed_input_area_index(current, input_areas)
        .filter(|idx| *idx < core.inputs.fields.len());
    core.inputs.set_focus_for_index(focused);
}

pub fn focus_next_indexed<A: Copy + PartialEq>(current: A, input_areas: &[A], results: A) -> A {
    match indexed_input_area_index(current, input_areas) {
        Some(idx) if idx + 1 < input_areas.len() => input_areas[idx + 1],
        Some(_) => results,
        None if current == results => input_areas.first().copied().unwrap_or(current),
        None => current,
    }
}

pub fn focus_prev_indexed<A: Copy + PartialEq>(current: A, input_areas: &[A], results: A) -> A {
    match indexed_input_area_index(current, input_areas) {
        Some(0) => results,
        Some(idx) => input_areas[idx - 1],
        None if current == results => input_areas.last().copied().unwrap_or(current),
        None => current,
    }
}

pub fn focus_up_indexed<A: Copy + PartialEq>(current: A, input_areas: &[A], results: A) -> A {
    match indexed_input_area_index(current, input_areas) {
        Some(0) => current,
        Some(idx) => input_areas[idx - 1],
        None if current == results => input_areas.last().copied().unwrap_or(current),
        None => current,
    }
}

pub fn focus_down_indexed<A: Copy + PartialEq>(current: A, input_areas: &[A], results: A) -> A {
    match indexed_input_area_index(current, input_areas) {
        Some(idx) if idx + 1 < input_areas.len() => input_areas[idx + 1],
        Some(_) => results,
        None => current,
    }
}

// --- N-area focus navigation helpers ---
// These generalize the 2/3-area patterns to tabs with N focus areas (e.g. FuzzTab with 6).
// The `areas` slice lists all focus variants in tab-order (first = Inputs, last = Results).
// The first and last elements are treated as Inputs and Results respectively.

/// Generic focus-next for N-area tabs. Cycles: areas[0] -> areas[1] -> ... -> areas[N-1] -> areas[0].
/// When entering areas[0] (from the end), the first input field is focused.
/// Does NOT blur on leaving areas[0] — that is handled by escape/focus_prev.
pub fn focus_next_n<A: Copy + PartialEq>(core: &mut TabCore, current: A, areas: &[A]) -> A {
    if areas.len() < 2 {
        return current;
    }
    if let Some(idx) = areas.iter().position(|a| *a == current) {
        let next_idx = (idx + 1) % areas.len();
        if next_idx == 0 {
            // Wrapping from last area back to Inputs: focus first field
            core.inputs.focus(0);
        }
        areas[next_idx]
    } else {
        current
    }
}

/// Generic focus-prev for N-area tabs. Reverse cycles through areas.
pub fn focus_prev_n<A: Copy + PartialEq>(core: &mut TabCore, current: A, areas: &[A]) -> A {
    if areas.len() < 2 {
        return current;
    }
    if let Some(idx) = areas.iter().position(|a| *a == current) {
        let prev_idx = if idx == 0 { areas.len() - 1 } else { idx - 1 };
        if idx == 0 {
            // Leaving Inputs backward: blur
            core.inputs.blur();
        }
        if prev_idx == 0 {
            // Entering Inputs: focus last field
            let last = core.inputs.fields.len().saturating_sub(1);
            core.inputs.focus(last);
        }
        areas[prev_idx]
    } else {
        current
    }
}

/// Generic handle_up for N-area tabs. In Inputs area, navigates input fields.
/// In Results area (last), scrolls results up. In selector areas, no-op (handled by tab).
pub fn handle_up_n<A: Copy + PartialEq>(core: &mut TabCore, current: A, areas: &[A]) {
    if areas.is_empty() {
        return;
    }
    if current == areas[0] {
        // Inputs area
        if !core.inputs.is_focused() && !core.results_view.is_empty() {
            core.scroll_results_up();
        } else {
            core.inputs.focus_prev();
        }
    } else if current == areas[areas.len() - 1] {
        // Results area
        core.scroll_results_up();
    }
}

/// Generic handle_down for N-area tabs.
pub fn handle_down_n<A: Copy + PartialEq>(core: &mut TabCore, current: A, areas: &[A]) {
    if areas.is_empty() {
        return;
    }
    if current == areas[0] {
        // Inputs area
        if !core.inputs.is_focused() && !core.results_view.is_empty() {
            core.scroll_results_down();
        } else {
            core.inputs.focus_next();
        }
    } else if current == areas[areas.len() - 1] {
        // Results area
        core.scroll_results_down();
    }
}

/// Generic handle_left for N-area tabs. Only moves cursor in Inputs area.
pub fn handle_left_n<A: Copy + PartialEq>(core: &mut TabCore, current: A, inputs: A) -> bool {
    if current == inputs {
        core.inputs.move_left()
    } else {
        false
    }
}

/// Generic handle_right for N-area tabs. Only moves cursor in Inputs area.
pub fn handle_right_n<A: Copy + PartialEq>(core: &mut TabCore, current: A, inputs: A) -> bool {
    if current == inputs {
        core.inputs.move_right()
    } else {
        false
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

/// Common `handle_enter` for simple 2-area tabs (Inputs/Results).
/// Delegates to `evaluate_enter` + `execute_enter_action`.
pub fn handle_enter_2area<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    inputs: A,
    results: A,
    is_running: bool,
    inputs_focused: bool,
) {
    let action = evaluate_enter(current, inputs, results, is_running, inputs_focused);
    execute_enter_action(core, action);
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
#[allow(clippy::too_many_arguments)]
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
pub fn handle_escape_simple<A: Copy + PartialEq>(core: &mut TabCore, current: A, inputs: A) -> A {
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

// --- N-area helper macros (used by tab_input_narea!) ---

/// Returns the first expression from a list. Used as `first_area!(A, B, C)` -> `A`.
#[macro_export]
macro_rules! first_area {
    ($first:expr $(, $rest:expr)*) => {
        $first
    };
}

/// Returns the last expression from a list. Used as `last_area!(A, B, C)` -> `C`.
#[macro_export]
macro_rules! last_area {
    ($last:expr) => { $last };
    ($first:expr, $($rest:expr),+) => { last_area!($($rest),+) };
}

/// Creates a static slice reference from a list of expressions.
/// Used by `tab_input_narea!` to build the areas slice for N-area helpers.
#[macro_export]
macro_rules! narea_slice {
    ( $($area:expr),+ $(,)? ) => {
        &[ $($area),+ ]
    };
}

/// Generic `handle_escape` for N-area tabs. If running, stops. Otherwise returns
/// to the first area (typically Inputs) and focuses the first field.
pub fn handle_escape_to_first<A: Copy + PartialEq>(core: &mut TabCore, current: A, first: A) -> A {
    if core.state == AppState::Running {
        core.stop();
        current
    } else if current == first {
        core.inputs.blur();
        current
    } else {
        core.inputs.focus(0);
        first
    }
}

// --- Selector-area focus navigation helpers ---
// These are for tabs with Selector/Inputs/Results layout (e.g., LoadTab, StressTab).

/// Focus-next for selector-area tabs: Selector → Inputs → Results → Selector.
pub fn focus_next_selector<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    selector: A,
    inputs: A,
    results: A,
) -> A {
    if current == selector {
        core.inputs.focus(0);
        inputs
    } else if current == inputs {
        core.inputs.blur();
        results
    } else {
        selector
    }
}

/// Focus-prev for selector-area tabs: Selector → Results → Inputs → Selector.
pub fn focus_prev_selector<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    selector: A,
    inputs: A,
    results: A,
) -> A {
    if current == selector {
        results
    } else if current == inputs {
        selector
    } else {
        core.inputs.focus(0);
        inputs
    }
}

/// `handle_up` for selector-area tabs.
/// In Selector: no-op (caller handles selector-specific up/down).
/// In Inputs: navigate fields.
/// In Results: scroll up.
pub fn handle_up_selector<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    selector: A,
    inputs: A,
    results: A,
) {
    if current == selector {
        // Selector up/down handled by caller
    } else if current == inputs {
        if !core.inputs.is_focused() && !core.results_view.is_empty() {
            core.scroll_results_up();
        } else {
            core.inputs.focus_prev();
        }
    } else if current == results {
        core.scroll_results_up();
    }
}

/// `handle_down` for selector-area tabs.
pub fn handle_down_selector<A: Copy + PartialEq>(
    core: &mut TabCore,
    current: A,
    selector: A,
    inputs: A,
    results: A,
) {
    if current == selector {
        // Selector up/down handled by caller
    } else if current == inputs {
        if !core.inputs.is_focused() && !core.results_view.is_empty() {
            core.scroll_results_down();
        } else {
            core.inputs.focus_next();
        }
    } else if current == results {
        core.scroll_results_down();
    }
}

// --- Dynamic layout helpers ---

/// Computes a dynamic input height based on terminal size, using a ratio of the
/// available height. Returns `(input_height, results_min_height)`.
///
/// Consolidates the repeated `if area.height < 24 { ... }` pattern found across
/// multiple tab render methods.
///
/// # Arguments
/// * `area_height` - The total available height
/// * `ratio` - The fraction of `area_height` to allocate to inputs (0.0 - 1.0)
/// * `min_input` - Minimum input height (clamped)
/// * `max_input` - Maximum input height (clamped)
/// * `default_input` - Default input height when terminal is tall enough
/// * `min_results` - Minimum results area height
pub fn dynamic_layout_height(
    area_height: u16,
    ratio: f32,
    min_input: u16,
    max_input: u16,
    default_input: u16,
    min_results: u16,
) -> (u16, u16) {
    if area_height < default_input + min_results + 4 {
        let h = ((area_height as f32 * ratio) as u16).clamp(min_input, max_input);
        (h, min_results)
    } else {
        (default_input, min_results)
    }
}

// --- Focus-aware styling helpers ---

/// Returns a border `Style` that uses `border_focused` when `focused` is true,
/// or `border` otherwise. Uses the explicit theme for consistent theming.
pub fn focus_border_style(focused: bool) -> Style {
    let theme = crate::theme::legacy::current_theme();
    theme.border_style(focused)
}

// --- Rendering helpers ---

/// Renders the standard 4-branch results area: Running -> Error -> Results -> Empty.
#[allow(clippy::too_many_arguments)]
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
    let theme = crate::theme::legacy::current_theme();
    match state {
        AppState::Running => {
            progress.render(f, area);
        }
        AppState::Error(_) => {
            if let Some(ref err) = error {
                let error_text =
                    Paragraph::new(format!("Error: {}", err.message())).style(theme.error());
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
/// Uses the explicit theme for consistent theming.
pub fn render_config_block(
    f: &mut Frame,
    area: Rect,
    title: &str,
    is_config_focused: bool,
) -> Rect {
    let theme = crate::theme::legacy::current_theme();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(theme.border_style(is_config_focused));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

/// Renders an error block with the given title. Returns early pattern for render methods.
/// Used by tabs that render a full-area error when in error state.
pub fn render_error_block(f: &mut Frame, area: Rect, title: &str, error: &TabError) {
    let theme = crate::theme::legacy::current_theme();
    let error_text = Paragraph::new(format!("Error: {}", error.message()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title)),
        )
        .style(theme.error());
    f.render_widget(error_text, area);
}

/// Renders input fields from an `InputGroup` into pre-computed layout chunks.
///
/// This eliminates the duplicated loop pattern found across 15+ tab render methods:
/// ```ignore
/// for (i, field) in self.core.inputs.fields.iter().enumerate() {
///     if let Some(chunk) = input_chunks.get(i) {
///         field.render(f, *chunk, insert_mode);
///     }
/// }
/// ```
///
/// # Arguments
/// * `f` - The ratatui frame to render into
/// * `input_chunks` - Layout chunks computed for the input area
/// * `inputs` - The `InputGroup` containing fields to render
/// * `insert_mode` - Whether the TUI is in insert mode (controls cursor display)
pub fn render_input_fields(
    f: &mut Frame,
    input_chunks: &[Rect],
    inputs: &InputGroup,
    insert_mode: bool,
) {
    for (i, field) in inputs.fields.iter().enumerate() {
        if let Some(chunk) = input_chunks.get(i) {
            field.render(f, *chunk, insert_mode);
        }
    }
}

/// Configuration for a standard 2-area render layout (Inputs + Results).
pub struct Render2AreaConfig<'a> {
    pub title: &'a str,
    pub input_constraints: Vec<Constraint>,
    pub focus_area: StandardFocusArea2,
    pub inputs_focused: StandardFocusArea2,
    pub results_focused: StandardFocusArea2,
    pub empty_title: &'static str,
    pub empty_text: &'static str,
}

/// Renders a standard 2-area tab layout: a config block with input fields on
/// top, and a results area below. Returns the split `Rect` pair so callers
/// can render additional widgets if needed.
pub fn render_standard_2area(
    f: &mut Frame,
    area: Rect,
    core: &TabCore,
    config: &Render2AreaConfig<'_>,
) {
    let input_height: u16 = config
        .input_constraints
        .iter()
        .map(|c| match c {
            Constraint::Length(n) => *n,
            _ => 3,
        })
        .sum();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(input_height), Constraint::Min(0)])
        .split(area);

    let input_inner = render_config_block(
        f,
        chunks[0],
        config.title,
        config.focus_area == config.inputs_focused,
    );

    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(config.input_constraints.as_slice())
        .split(input_inner);

    render_input_fields(f, &input_chunks, &core.inputs, true);

    render_results_area(
        f,
        chunks[1],
        &core.state,
        &core.error,
        &core.results_view,
        &core.progress,
        config.empty_title,
        config.empty_text,
    );
}

type OptionsRenderFn<'a> = Box<dyn FnOnce(&mut Frame, Rect, bool) + 'a>;

/// Configuration for a standard 3-area render layout (Inputs + Options + Results).
pub struct Render3AreaConfig<'a> {
    pub title: &'a str,
    pub input_constraints: Vec<Constraint>,
    pub focus_area: StandardFocusArea,
    pub inputs_focused: StandardFocusArea,
    pub options_focused: StandardFocusArea,
    pub results_focused: StandardFocusArea,
    pub render_options: OptionsRenderFn<'a>,
    pub empty_title: &'static str,
    pub empty_text: &'static str,
}

/// Renders a standard 3-area tab layout: a config block with input fields,
/// an options area, and a results area. The `render_options` callback is
/// called to render the options section (checkboxes, selectors, etc.).
pub fn render_standard_3area(
    f: &mut Frame,
    area: Rect,
    core: &TabCore,
    config: Render3AreaConfig<'_>,
) {
    let input_height: u16 = config
        .input_constraints
        .iter()
        .map(|c| match c {
            Constraint::Length(n) => *n,
            _ => 3,
        })
        .sum();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(input_height),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let input_inner = render_config_block(
        f,
        chunks[0],
        config.title,
        config.focus_area == config.inputs_focused,
    );

    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(config.input_constraints.as_slice())
        .split(input_inner);

    render_input_fields(f, &input_chunks, &core.inputs, true);

    (config.render_options)(f, chunks[1], config.focus_area == config.options_focused);

    render_results_area(
        f,
        chunks[2],
        &core.state,
        &core.error,
        &core.results_view,
        &core.progress,
        config.empty_title,
        config.empty_text,
    );
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
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
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
        core.results_view
            .add_line(ratatui::text::Line::from("test"));
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
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
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
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
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

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        // Inputs -> Options
        let next = focus_next_3area(
            &mut core,
            Area::Inputs,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
        assert_eq!(next, Area::Options);

        // Options -> Results
        let next = focus_next_3area(
            &mut core,
            Area::Options,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
        assert_eq!(next, Area::Results);

        // Results -> Inputs (focuses first field)
        let next = focus_next_3area(
            &mut core,
            Area::Results,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
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

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        // Inputs -> Results (blurs)
        let prev = focus_prev_3area(
            &mut core,
            Area::Inputs,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
        assert_eq!(prev, Area::Results);

        // Results -> Options
        let prev = focus_prev_3area(
            &mut core,
            Area::Results,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
        assert_eq!(prev, Area::Options);

        // Options -> Inputs (focuses first field)
        let prev = focus_prev_3area(
            &mut core,
            Area::Options,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
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
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        core.inputs.focus(0);
        let result = handle_escape_simple(&mut core, 0, 0);
        assert_eq!(result, 0); // Stays in inputs area
        assert!(!core.inputs.is_focused());
    }

    #[test]
    fn handle_escape_simple_focuses_inputs_when_from_results() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
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
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
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
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        handle_enter_3area(
            &mut core,
            Area::Results,
            Area::Inputs,
            Area::Options,
            Area::Results,
            false,
            false,
            |_| false,
        );
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_enter_3area_running_stops() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        handle_enter_3area(
            &mut core,
            Area::Inputs,
            Area::Inputs,
            Area::Options,
            Area::Results,
            true,
            false,
            |_| false,
        );
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_enter_3area_inputs_focused_blurs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        core.inputs.focus(0);
        handle_enter_3area(
            &mut core,
            Area::Inputs,
            Area::Inputs,
            Area::Options,
            Area::Results,
            false,
            true,
            |_| false,
        );
        assert!(!core.inputs.is_focused());
    }

    #[test]
    fn handle_enter_3area_options_calls_action_no_start() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        let mut called = false;
        handle_enter_3area(
            &mut core,
            Area::Options,
            Area::Inputs,
            Area::Options,
            Area::Results,
            false,
            false,
            |_| {
                called = true;
                false
            },
        );
        assert!(called);
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_enter_3area_options_starts_when_action_returns_true() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        handle_enter_3area(
            &mut core,
            Area::Options,
            Area::Inputs,
            Area::Options,
            Area::Results,
            false,
            false,
            |_| true,
        );
        assert_eq!(core.state, AppState::Running);
    }

    #[test]
    fn handle_enter_3area_inputs_unfocused_starts() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        handle_enter_3area(
            &mut core,
            Area::Inputs,
            Area::Inputs,
            Area::Options,
            Area::Results,
            false,
            false,
            |_| false,
        );
        assert_eq!(core.state, AppState::Running);
    }

    #[test]
    fn handle_escape_3area_running_stops() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        let result = handle_escape_3area(
            &mut core,
            Area::Options,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
        assert_eq!(result, Area::Options);
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_escape_3area_options_returns_to_inputs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        let result = handle_escape_3area(
            &mut core,
            Area::Options,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
        assert_eq!(result, Area::Inputs);
        assert!(core.inputs.is_focused());
    }

    #[test]
    fn handle_escape_3area_results_returns_to_inputs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        let result = handle_escape_3area(
            &mut core,
            Area::Results,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
        assert_eq!(result, Area::Inputs);
        assert!(core.inputs.is_focused());
    }

    #[test]
    fn handle_escape_3area_inputs_blurs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        core.inputs.focus(0);
        let result = handle_escape_3area(
            &mut core,
            Area::Inputs,
            Area::Inputs,
            Area::Options,
            Area::Results,
        );
        assert_eq!(result, Area::Inputs);
        assert!(!core.inputs.is_focused());
    }

    #[test]
    fn handle_enter_2area_results_no_op() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        handle_enter_2area(
            &mut core,
            Area::Results,
            Area::Inputs,
            Area::Results,
            false,
            false,
        );
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_enter_2area_running_stops() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Results,
        }

        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        handle_enter_2area(
            &mut core,
            Area::Inputs,
            Area::Inputs,
            Area::Results,
            true,
            false,
        );
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_enter_2area_inputs_focused_blurs() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        core.inputs.focus(0);
        handle_enter_2area(
            &mut core,
            Area::Inputs,
            Area::Inputs,
            Area::Results,
            false,
            true,
        );
        assert!(!core.inputs.is_focused());
    }

    #[test]
    fn handle_enter_2area_inputs_unfocused_starts() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Results,
        }

        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("Target").with_value("example.com")),
        );
        handle_enter_2area(
            &mut core,
            Area::Inputs,
            Area::Inputs,
            Area::Results,
            false,
            false,
        );
        assert_eq!(core.state, AppState::Running);
    }

    #[test]
    fn focus_next_2area_cycles_correctly() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        // Inputs -> Results (blurs)
        let next = focus_next_2area(&mut core, Area::Inputs, Area::Inputs, Area::Results);
        assert_eq!(next, Area::Results);
        assert!(!core.inputs.is_focused());

        // Results -> Inputs (focuses first field)
        let next = focus_next_2area(&mut core, Area::Results, Area::Inputs, Area::Results);
        assert_eq!(next, Area::Inputs);
        assert!(core.inputs.is_focused());
    }

    #[test]
    fn focus_prev_2area_cycles_correctly() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        // Inputs -> Results (blurs)
        let prev = focus_prev_2area(&mut core, Area::Inputs, Area::Inputs, Area::Results);
        assert_eq!(prev, Area::Results);

        // Results -> Inputs (focuses last field)
        let prev = focus_prev_2area(&mut core, Area::Results, Area::Inputs, Area::Results);
        assert_eq!(prev, Area::Inputs);
        assert!(core.inputs.is_focused());
    }

    #[test]
    fn handle_up_2area_scrolls_results_when_not_focused() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        // Add some content to results
        for i in 0..20 {
            core.results_view
                .add_line(ratatui::text::Line::from(format!("line {}", i)));
        }

        // In Results area: up scrolls results
        handle_up_2area(&mut core, Area::Results, Area::Inputs, Area::Results);

        // In Inputs area with no field focused: up scrolls results
        handle_up_2area(&mut core, Area::Inputs, Area::Inputs, Area::Results);
    }

    #[test]
    fn handle_down_2area_scrolls_results_when_not_focused() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        for i in 0..20 {
            core.results_view
                .add_line(ratatui::text::Line::from(format!("line {}", i)));
        }

        // In Results area: down scrolls results
        handle_down_2area(&mut core, Area::Results, Area::Inputs, Area::Results);

        // In Inputs area with no field focused: down scrolls results
        handle_down_2area(&mut core, Area::Inputs, Area::Inputs, Area::Results);
    }

    #[test]
    fn handle_enter_2area_running_and_results_returns_idle() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Results,
        }

        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        // Running + Results area = Stop
        handle_enter_2area(
            &mut core,
            Area::Results,
            Area::Inputs,
            Area::Results,
            true,
            false,
        );
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_escape_simple_idempotent_when_idle_in_results() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        // In Results, idle: should focus inputs
        let result = handle_escape_simple(&mut core, 1, 0);
        assert_eq!(result, 0);
        assert!(core.inputs.is_focused());

        // Running: should stop
        core.state = AppState::Running;
        let result = handle_escape_simple(&mut core, 0, 0);
        assert_eq!(result, 0);
        assert_eq!(core.state, AppState::Idle);
    }

    // --- 3-area alias tests (verify aliases produce identical results to 2area) ---

    #[test]
    fn handle_up_3area_delegates_to_2area() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[allow(dead_code)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core3 = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        let mut core2 = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        // Both should produce identical behavior in Inputs area
        handle_up_3area(&mut core3, Area::Inputs, Area::Inputs, Area::Results);
        handle_up_2area(&mut core2, Area::Inputs, Area::Inputs, Area::Results);
        assert_eq!(core3.state, core2.state);

        // Both should produce identical behavior in Results area
        handle_up_3area(&mut core3, Area::Results, Area::Inputs, Area::Results);
        handle_up_2area(&mut core2, Area::Results, Area::Inputs, Area::Results);
        assert_eq!(core3.state, core2.state);
    }

    #[test]
    fn handle_down_3area_delegates_to_2area() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[allow(dead_code)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core3 = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        let mut core2 = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        handle_down_3area(&mut core3, Area::Inputs, Area::Inputs, Area::Results);
        handle_down_2area(&mut core2, Area::Inputs, Area::Inputs, Area::Results);
        assert_eq!(core3.state, core2.state);

        handle_down_3area(&mut core3, Area::Results, Area::Inputs, Area::Results);
        handle_down_2area(&mut core2, Area::Results, Area::Inputs, Area::Results);
        assert_eq!(core3.state, core2.state);
    }

    #[test]
    fn handle_up_3area_options_is_noop() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        let state_before = core.state.clone();
        // Options area should be a no-op
        handle_up_3area(&mut core, Area::Options, Area::Inputs, Area::Results);
        assert_eq!(core.state, state_before);
    }

    #[test]
    fn handle_down_3area_options_is_noop() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Area {
            Inputs,
            Options,
            Results,
        }

        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        let state_before = core.state.clone();
        handle_down_3area(&mut core, Area::Options, Area::Inputs, Area::Results);
        assert_eq!(core.state, state_before);
    }

    // --- N-area focus tests ---

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum NArea {
        Inputs,
        Payload,
        Mode,
        Target,
        Checkbox,
        Results,
    }

    const N_AREAS: &[NArea] = &[
        NArea::Inputs,
        NArea::Payload,
        NArea::Mode,
        NArea::Target,
        NArea::Checkbox,
        NArea::Results,
    ];

    #[test]
    fn focus_next_n_cycles_through_all_areas() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        let mut current = NArea::Inputs;
        let expected = [
            NArea::Payload,
            NArea::Mode,
            NArea::Target,
            NArea::Checkbox,
            NArea::Results,
            NArea::Inputs, // wraps around
        ];

        for (i, expected_area) in expected.iter().enumerate() {
            current = focus_next_n(&mut core, current, N_AREAS);
            assert_eq!(current, *expected_area, "step {}", i);
        }
    }

    #[test]
    fn test_focus_next_n_no_blur_on_leave_inputs() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        core.inputs.focus(0);
        assert!(core.inputs.is_focused());

        let next = focus_next_n(&mut core, NArea::Inputs, N_AREAS);
        assert_eq!(next, NArea::Payload);
        // focus_next_n does NOT blur — blur is handled by escape/focus_prev
        assert!(
            core.inputs.is_focused(),
            "should not blur on forward navigation"
        );
    }

    #[test]
    fn focus_next_n_focuses_on_enter_inputs() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        let current = focus_next_n(&mut core, NArea::Results, N_AREAS);
        assert_eq!(current, NArea::Inputs);
        assert!(
            core.inputs.is_focused(),
            "should focus first field on enter Inputs"
        );
    }

    #[test]
    fn focus_prev_n_cycles_backwards() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        let mut current = NArea::Results;
        let expected = [
            NArea::Checkbox,
            NArea::Target,
            NArea::Mode,
            NArea::Payload,
            NArea::Inputs,
            NArea::Results, // wraps around
        ];

        for (i, expected_area) in expected.iter().enumerate() {
            current = focus_prev_n(&mut core, current, N_AREAS);
            assert_eq!(current, *expected_area, "step {}", i);
        }
    }

    #[test]
    fn focus_prev_n_focuses_last_input_on_enter_inputs() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new()
                .add(crate::components::InputField::new("A"))
                .add(crate::components::InputField::new("B"))
                .add(crate::components::InputField::new("C")),
        );

        let current = focus_prev_n(&mut core, NArea::Payload, N_AREAS);
        assert_eq!(current, NArea::Inputs);
        // Should focus the last field
        assert_eq!(core.inputs.focused, Some(2));
    }

    #[test]
    fn focus_next_n_single_area_returns_current() {
        let mut core = TabCore::new("test", "Results");
        let current = focus_next_n(&mut core, NArea::Inputs, &[NArea::Inputs]);
        assert_eq!(current, NArea::Inputs);
    }

    #[test]
    fn focus_prev_n_unknown_area_returns_current() {
        let mut core = TabCore::new("test", "Results");
        // Area not in the list should return current unchanged
        let current = focus_prev_n(&mut core, NArea::Checkbox, &[NArea::Inputs, NArea::Results]);
        assert_eq!(current, NArea::Checkbox);
    }

    #[test]
    fn handle_up_n_scrolls_results_in_results_area() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        for i in 0..20 {
            core.results_view
                .add_line(ratatui::text::Line::from(format!("line {}", i)));
        }

        handle_up_n(&mut core, NArea::Results, N_AREAS);
        // Should not panic; results should scroll
    }

    #[test]
    fn handle_down_n_in_middle_area_is_noop() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        // Middle area (not Inputs, not Results) should be a no-op
        handle_down_n(&mut core, NArea::Mode, N_AREAS);
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_left_n_in_inputs_moves_cursor() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new().add(crate::components::InputField::new("Target").with_value("test")),
        );
        core.inputs.focus(0);

        let result = handle_left_n(&mut core, NArea::Inputs, NArea::Inputs);
        assert!(result);
    }

    #[test]
    fn handle_left_n_in_non_inputs_returns_false() {
        let mut core = TabCore::new("test", "Results");
        let result = handle_left_n(&mut core, NArea::Payload, NArea::Inputs);
        assert!(!result);
    }

    #[test]
    fn handle_right_n_in_inputs_moves_cursor() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new().add(crate::components::InputField::new("Target").with_value("test")),
        );
        core.inputs.focus(0);
        // Move to beginning so right actually moves
        if let Some(field) = core.inputs.fields.first_mut() {
            field.cursor_pos = 0;
        }

        let result = handle_right_n(&mut core, NArea::Inputs, NArea::Inputs);
        assert!(result);
    }

    #[test]
    fn handle_right_n_in_non_inputs_returns_false() {
        let mut core = TabCore::new("test", "Results");
        let result = handle_right_n(&mut core, NArea::Checkbox, NArea::Inputs);
        assert!(!result);
    }

    #[test]
    fn handle_escape_to_first_while_running_stops() {
        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        let result = handle_escape_to_first(&mut core, NArea::Mode, NArea::Inputs);
        assert_eq!(result, NArea::Mode);
        assert_eq!(core.state, AppState::Idle);
    }

    #[test]
    fn handle_escape_to_first_in_first_area_blurs() {
        let mut core = TabCore::new("test", "Results").with_inputs(
            InputGroup::new().add(crate::components::InputField::new("Target").with_value("test")),
        );
        core.inputs.focus(0);
        assert!(core.inputs.is_focused());
        let result = handle_escape_to_first(&mut core, NArea::Inputs, NArea::Inputs);
        assert_eq!(result, NArea::Inputs);
        assert!(!core.inputs.is_focused());
    }

    #[test]
    fn handle_escape_to_first_in_other_area_returns_to_first() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        let result = handle_escape_to_first(&mut core, NArea::Results, NArea::Inputs);
        assert_eq!(result, NArea::Inputs);
        assert!(core.inputs.is_focused());
    }

    // --- focus_border_style tests ---

    #[test]
    fn focus_border_style_focused_uses_border_focused_color() {
        let style = focus_border_style(true);
        // Should produce a non-default style (i.e., a foreground color is set)
        assert!(
            style.fg.is_some(),
            "focused border should have a foreground color"
        );
    }

    #[test]
    fn focus_border_style_unfocused_uses_border_color() {
        let style = focus_border_style(false);
        assert!(
            style.fg.is_some(),
            "unfocused border should have a foreground color"
        );
    }

    #[test]
    fn focus_border_style_focused_and_unfocused_differ() {
        let focused = focus_border_style(true);
        let unfocused = focus_border_style(false);
        assert_ne!(
            focused.fg, unfocused.fg,
            "focused and unfocused borders should use different colors"
        );
    }

    // --- render_standard_2area tests ---

    #[test]
    fn render_standard_2area_runs_without_panic() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        let config = Render2AreaConfig {
            title: "Test",
            input_constraints: vec![Constraint::Length(3)],
            focus_area: StandardFocusArea2::Inputs,
            inputs_focused: StandardFocusArea2::Inputs,
            results_focused: StandardFocusArea2::Results,
            empty_title: "Results",
            empty_text: "No results yet",
        };
        terminal
            .draw(|f| render_standard_2area(f, Rect::new(0, 0, 80, 24), &core, &config))
            .unwrap();
    }

    #[test]
    fn render_standard_2area_with_results_focus() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));
        let config = Render2AreaConfig {
            title: "Test",
            input_constraints: vec![Constraint::Length(3)],
            focus_area: StandardFocusArea2::Results,
            inputs_focused: StandardFocusArea2::Inputs,
            results_focused: StandardFocusArea2::Results,
            empty_title: "Results",
            empty_text: "No results yet",
        };
        terminal
            .draw(|f| render_standard_2area(f, Rect::new(0, 0, 80, 24), &core, &config))
            .unwrap();
    }

    // --- prepare_results tests ---

    #[test]
    fn prepare_results_sets_completed_state() {
        use ratatui::text::Line;

        let mut core = TabCore::new("test", "Results");
        core.state = AppState::Running;
        core.results_view.add_line(Line::from("old data"));

        {
            let view = core.prepare_results();
            view.add_line(Line::from("new data"));
        }

        assert_eq!(core.state, AppState::Completed);
        assert!(!core.results_view.is_empty());
    }

    #[test]
    fn prepare_results_clears_old_content() {
        use ratatui::text::Line;

        let mut core = TabCore::new("test", "Results");
        core.results_view.add_line(Line::from("old data"));

        let view = core.prepare_results();
        view.add_line(Line::from("new data"));

        // Old data should be gone
        let content = core.results_view.get_content();
        assert!(!content.contains("old data"));
        assert!(content.contains("new data"));
    }

    // --- selector-area focus navigation tests ---

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SelArea {
        Selector,
        Inputs,
        Results,
    }

    #[test]
    fn focus_next_selector_cycles_correctly() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        // Selector -> Inputs
        let next = focus_next_selector(
            &mut core,
            SelArea::Selector,
            SelArea::Selector,
            SelArea::Inputs,
            SelArea::Results,
        );
        assert_eq!(next, SelArea::Inputs);
        assert!(core.inputs.is_focused());

        // Inputs -> Results
        let next = focus_next_selector(
            &mut core,
            SelArea::Inputs,
            SelArea::Selector,
            SelArea::Inputs,
            SelArea::Results,
        );
        assert_eq!(next, SelArea::Results);

        // Results -> Selector
        let next = focus_next_selector(
            &mut core,
            SelArea::Results,
            SelArea::Selector,
            SelArea::Inputs,
            SelArea::Results,
        );
        assert_eq!(next, SelArea::Selector);
    }

    #[test]
    fn focus_prev_selector_cycles_correctly() {
        let mut core = TabCore::new("test", "Results")
            .with_inputs(InputGroup::new().add(crate::components::InputField::new("Target")));

        // Selector -> Results
        let prev = focus_prev_selector(
            &mut core,
            SelArea::Selector,
            SelArea::Selector,
            SelArea::Inputs,
            SelArea::Results,
        );
        assert_eq!(prev, SelArea::Results);

        // Results -> Inputs
        let prev = focus_prev_selector(
            &mut core,
            SelArea::Results,
            SelArea::Selector,
            SelArea::Inputs,
            SelArea::Results,
        );
        assert_eq!(prev, SelArea::Inputs);
        assert!(core.inputs.is_focused());

        // Inputs -> Selector
        let prev = focus_prev_selector(
            &mut core,
            SelArea::Inputs,
            SelArea::Selector,
            SelArea::Inputs,
            SelArea::Results,
        );
        assert_eq!(prev, SelArea::Selector);
    }

    #[test]
    fn handle_up_selector_in_results_scrolls() {
        use ratatui::text::Line;

        let mut core = TabCore::new("test", "Results");
        for _ in 0..10 {
            core.results_view.add_line(Line::from("line"));
        }
        core.results_view.scroll_down(5);

        handle_up_selector(
            &mut core,
            SelArea::Results,
            SelArea::Selector,
            SelArea::Inputs,
            SelArea::Results,
        );
        // Should not panic; scroll position changed
    }

    #[test]
    fn handle_down_selector_in_results_scrolls() {
        use ratatui::text::Line;

        let mut core = TabCore::new("test", "Results");
        for _ in 0..10 {
            core.results_view.add_line(Line::from("line"));
        }

        handle_down_selector(
            &mut core,
            SelArea::Results,
            SelArea::Selector,
            SelArea::Inputs,
            SelArea::Results,
        );
        // Should not panic
    }

    #[test]
    fn render_input_fields_renders_all_fields_without_panic() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let inputs = InputGroup::new()
            .add(crate::components::InputField::new("Target"))
            .add(crate::components::InputField::new("Concurrency").with_value("20"))
            .add(crate::components::InputField::new("Timeout").with_value("10"));

        let chunks = [
            Rect::new(0, 0, 80, 3),
            Rect::new(0, 3, 80, 3),
            Rect::new(0, 6, 80, 3),
        ];

        terminal
            .draw(|f| {
                render_input_fields(f, &chunks, &inputs, true);
            })
            .unwrap();
    }

    #[test]
    fn render_input_fields_handles_fewer_chunks_than_fields() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let inputs = InputGroup::new()
            .add(crate::components::InputField::new("Target"))
            .add(crate::components::InputField::new("Concurrency"))
            .add(crate::components::InputField::new("Timeout"));

        // Only 2 chunks for 3 fields - should not panic
        let chunks = [Rect::new(0, 0, 80, 3), Rect::new(0, 3, 80, 3)];

        terminal
            .draw(|f| {
                render_input_fields(f, &chunks, &inputs, false);
            })
            .unwrap();
    }

    #[test]
    fn render_input_fields_empty_input_group() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let inputs = InputGroup::new();
        let chunks: [Rect; 0] = [];

        terminal
            .draw(|f| {
                render_input_fields(f, &chunks, &inputs, true);
            })
            .unwrap();
    }

    // --- checkbox navigation helper tests ---

    #[test]
    fn handle_options_up_wrapping_cycles_from_first_to_last() {
        let mut idx = 0usize;
        handle_options_up_wrapping(&mut idx, 5);
        assert_eq!(idx, 4);
    }

    #[test]
    fn handle_options_up_wrapping_decrements_middle() {
        let mut idx = 3usize;
        handle_options_up_wrapping(&mut idx, 5);
        assert_eq!(idx, 2);
    }

    #[test]
    fn handle_options_up_wrapping_noop_when_empty() {
        let mut idx = 0usize;
        handle_options_up_wrapping(&mut idx, 0);
        assert_eq!(idx, 0);
    }

    #[test]
    fn handle_options_down_wrapping_cycles_from_last_to_first() {
        let mut idx = 4usize;
        handle_options_down_wrapping(&mut idx, 5);
        assert_eq!(idx, 0);
    }

    #[test]
    fn handle_options_down_wrapping_increments_middle() {
        let mut idx = 2usize;
        handle_options_down_wrapping(&mut idx, 5);
        assert_eq!(idx, 3);
    }

    #[test]
    fn handle_options_down_wrapping_noop_when_empty() {
        let mut idx = 0usize;
        handle_options_down_wrapping(&mut idx, 0);
        assert_eq!(idx, 0);
    }

    #[test]
    fn handle_options_wrapping_roundtrip() {
        let mut idx = 0usize;
        let count = 5;
        for _ in 0..count {
            handle_options_down_wrapping(&mut idx, count);
        }
        assert_eq!(idx, 0, "wrapping down should return to start");
        for _ in 0..count {
            handle_options_up_wrapping(&mut idx, count);
        }
        assert_eq!(idx, 0, "wrapping up should return to start");
    }
}
