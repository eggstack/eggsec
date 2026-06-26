use crate::components::{empty_state_paragraph, Selector, SelectorItem};
use crate::tabs::core::{
    self, render_config_block, render_error_block, render_input_fields, TabCore,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_state_boilerplate, tc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct NseTab {
    pub core: TabCore,
    pub script_selector: Selector,
    pub focus_area: NseFocusArea,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NseFocusArea {
    Inputs,
    ScriptSelector,
    Results,
}

impl NseTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(crate::components::InputField::new("Target Host / URL"))
            .add(crate::components::InputField::new(
                "Script Arguments (key=value,comma-sep)",
            ))
            .add(crate::components::InputField::new(
                "Custom Script Path (optional)",
            ));

        let script_selector = Selector::new("NSE Script").items(vec![
            SelectorItem::new("Default Scripts", "default"),
            SelectorItem::new("Discovery", "discovery"),
            SelectorItem::new("Banner Grab", "banner"),
            SelectorItem::new("HTTP Headers", "http-headers"),
            SelectorItem::new("DNS Check", "dns-check"),
            SelectorItem::new("SSL Certificate", "ssl-cert"),
            SelectorItem::new("Custom Script", "custom"),
        ]);

        Self {
            core: TabCore::new("NSE Scan", "NSE Results").with_inputs(inputs),
            script_selector,
            focus_area: NseFocusArea::Inputs,
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn script_args(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn custom_script(&self) -> Option<&str> {
        self.core
            .inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn script(&self) -> &str {
        self.script_selector.selected_value().unwrap_or("default")
    }

    pub fn set_results(&mut self, results: NseResults) {
        let view = &mut self.core.results_view;
        self.core.state = AppState::Completed;
        view.clear();

        view.add_line(Line::from(Span::styled(
            format!("NSE Script Results: {}", results.script),
            Style::default().fg(tc!(success)),
        )));
        view.add_line(Line::from(Span::styled(
            format!("Target: {}", results.target),
            Style::default().fg(tc!(warning)),
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(Span::styled(
            "Output:",
            Style::default().fg(tc!(info)),
        )));
        view.add_line(Line::from(""));

        for line in results.output.lines() {
            view.add_line(Line::from(line.to_string()));
        }

        if !results.errors.is_empty() {
            view.add_line(Line::from(""));
            view.add_line(Line::from(Span::styled(
                "Errors:",
                Style::default().fg(tc!(error)),
            )));
            for err in results.errors.lines() {
                view.add_line(Line::from(err.to_string()));
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct NseResults {
    pub target: String,
    pub script: String,
    pub output: String,
    pub errors: String,
    pub success: bool,
}

impl Default for NseTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for NseTab {
    tab_state_boilerplate!(NseTab, core: core);

    fn has_selector_open(&self) -> bool {
        self.script_selector.is_open()
    }

    fn reset(&mut self) {
        self.core.reset_all();
        self.script_selector.select(0);
        self.focus_area = NseFocusArea::Inputs;
    }
}

impl TabRender for NseTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            NseFocusArea::Inputs => "Inputs",
            NseFocusArea::ScriptSelector => "Script",
            NseFocusArea::Results => "Results",
        };
        Some(vec!["NSE", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref err) = self.core.error {
            render_error_block(f, area, "NSE - Error", err);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Length(4),
                Constraint::Min(5),
            ])
            .split(area);

        let input_area = chunks.first().copied().unwrap_or(area);

        let input_inner = render_config_block(
            f,
            input_area,
            "NSE Configuration",
            self.focus_area == NseFocusArea::Inputs,
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

        // Script selector
        let mut selector = self.script_selector.clone();
        selector.focused = self.focus_area == NseFocusArea::ScriptSelector;
        if let Some(selector_area) = chunks.get(1) {
            selector.render(f, *selector_area);
        }

        // Results
        if let Some(results_area) = chunks.get(2) {
            if self.core.results_view.is_empty() {
                let placeholder =
                    empty_state_paragraph("Results", "Results will appear here after running");
                f.render_widget(placeholder, *results_area);
            } else {
                self.core.results_view.render(f, *results_area, None);
            }
        }
    }
}

impl TabInput for NseTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            NseFocusArea::Inputs => {
                self.core.inputs.blur();
                self.script_selector.focus();
                NseFocusArea::ScriptSelector
            }
            NseFocusArea::ScriptSelector => {
                self.script_selector.blur();
                NseFocusArea::Results
            }
            NseFocusArea::Results => {
                self.core.inputs.focus(0);
                NseFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            NseFocusArea::Inputs => {
                self.core.inputs.blur();
                NseFocusArea::Results
            }
            NseFocusArea::ScriptSelector => {
                self.script_selector.blur();
                self.core.inputs.focus(0);
                NseFocusArea::Inputs
            }
            NseFocusArea::Results => {
                self.script_selector.focus();
                NseFocusArea::ScriptSelector
            }
        };
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        match self.focus_area {
            NseFocusArea::Inputs => {
                if self.core.inputs.is_focused() {
                    self.core.inputs.blur();
                    return;
                }
            }
            NseFocusArea::ScriptSelector => {
                if self.script_selector.focused {
                    self.script_selector.handle_enter();
                }
                return;
            }
            NseFocusArea::Results => {
                return;
            }
        }
        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        self.core.inputs.blur();
        self.script_selector.blur();
        self.focus_area = NseFocusArea::Inputs;
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            NseFocusArea::Inputs => {
                self.core.inputs.focus_prev();
            }
            NseFocusArea::ScriptSelector => {
                self.script_selector.handle_up();
            }
            NseFocusArea::Results => {
                self.core.results_view.scroll_up(1);
            }
        }
    }

    fn handle_down(&mut self) {
        match self.focus_area {
            NseFocusArea::Inputs => {
                self.core.inputs.focus_next();
            }
            NseFocusArea::ScriptSelector => {
                self.script_selector.handle_down();
            }
            NseFocusArea::Results => {
                self.core.results_view.scroll_down(1);
            }
        }
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            NseFocusArea::Inputs => self.core.inputs.is_at_left_edge(),
            NseFocusArea::ScriptSelector => {
                self.script_selector.items.is_empty() || self.script_selector.selected == 0
            }
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            NseFocusArea::Inputs => self.core.inputs.is_at_right_edge(),
            NseFocusArea::ScriptSelector => {
                self.script_selector.items.is_empty()
                    || self.script_selector.selected
                        >= self.script_selector.items.len().saturating_sub(1)
            }
            _ => true,
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.get_focused_value()
        } else if self.focus_area == NseFocusArea::Results {
            Some(self.core.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() && self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() && self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == NseFocusArea::Inputs {
                self.core.inputs.move_home();
            } else if self.focus_area == NseFocusArea::Results {
                self.core.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == NseFocusArea::Inputs {
                self.core.inputs.move_end();
            } else if self.focus_area == NseFocusArea::Results {
                self.core.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = NseFocusArea::Inputs;
        self.core.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.core.inputs.blur();
            self.focus_area = NseFocusArea::Results;
        }
    }

    fn page_up(&mut self, page_size: usize) {
        if !self.is_running() {
            self.core.results_view.page_up(page_size);
        }
    }

    fn page_down(&mut self, page_size: usize) {
        if !self.is_running() {
            self.core.results_view.page_down(page_size);
        }
    }

    fn stop(&mut self) {
        self.core.stop();
    }

    fn primary_target(&self) -> Option<String> {
        Some(self.target().to_string())
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() && self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() && self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.move_right()
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == NseFocusArea::Inputs && self.core.inputs.is_focused()
    }
}

impl NseTab {
    pub fn start(&mut self) {
        if self.target().is_empty() {
            return;
        }
        if self.core.state != AppState::Running {
            self.core.progress.current = 0;
            self.core.progress.total = 0;
            self.core.state = AppState::Running;
        }
    }
}
