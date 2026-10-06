use super::nse_report_view::{
    render_filtered_report, render_report_sections, NseReportSection, NseSectionContent,
};
use crate::app::tab_error::TabError;
use crate::components::{empty_state_paragraph, Selector, SelectorItem};
use crate::tabs::core::{render_config_block, render_error_block, render_input_fields, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_boilerplate, tab_state_boilerplate, tc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    Frame,
};

pub struct NseTab {
    pub core: TabCore,
    pub script_selector: Selector,
    pub focus_area: NseFocusArea,
    #[cfg(feature = "nse")]
    pub structured_report: Option<eggsec::nse::NseRunReport>,
    #[cfg(feature = "nse")]
    pub report_sections: Vec<NseSectionContent>,
    #[cfg(feature = "nse")]
    pub report_filter: Option<NseReportSection>,
    #[cfg(feature = "nse")]
    pub report_search: String,
    #[cfg(feature = "nse")]
    pub detail_view_active: bool,
    #[cfg(feature = "nse")]
    pub detail_section_index: usize,
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
            #[cfg(feature = "nse")]
            structured_report: None,
            #[cfg(feature = "nse")]
            report_sections: Vec::new(),
            #[cfg(feature = "nse")]
            report_filter: None,
            #[cfg(feature = "nse")]
            report_search: String::new(),
            #[cfg(feature = "nse")]
            detail_view_active: false,
            #[cfg(feature = "nse")]
            detail_section_index: 0,
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

        #[cfg(feature = "nse")]
        if let Some(report) = results.report {
            self.structured_report = Some(report.clone());
            self.report_sections = render_report_sections(&report);
            self.report_filter = None;
            self.report_search.clear();
            self.detail_view_active = false;
            self.detail_section_index = 0;
            let lines = render_filtered_report(&self.report_sections, self.report_filter, None);
            for line in lines {
                view.add_line(line);
            }
            return;
        }

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

pub use eggsec::dispatch::NseResults;

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
        self.core.inputs.blur();
        self.script_selector.select(0);
        self.script_selector.blur();
        self.focus_area = NseFocusArea::Inputs;
        #[cfg(feature = "nse")]
        {
            self.structured_report = None;
            self.report_sections.clear();
            self.report_filter = None;
            self.report_search.clear();
            self.detail_view_active = false;
            self.detail_section_index = 0;
        }
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
                // Build title with filter/search info
                let mut title = "NSE Results".to_string();
                #[cfg(feature = "nse")]
                {
                    if let Some(filter) = self.filter_label() {
                        title = format!("NSE Results [{}]", filter);
                    }
                    if !self.report_search.is_empty() {
                        let preview: String = self.report_search.chars().take(20).collect();
                        title = format!("{} /{}", title, preview);
                    }
                }
                let mut view = self.core.results_view.clone();
                view.title = title;
                view.render(f, *results_area, None);
            }
        }
    }

    fn render_overlays(&self, f: &mut Frame, area: Rect) {
        if self.core.error.is_some() {
            return;
        }

        // Mirrors the selector row layout in `render`.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Length(4),
                Constraint::Min(5),
            ])
            .split(area);

        if let Some(selector_area) = chunks.get(1) {
            if let Some(dropdown) = self
                .script_selector
                .dropdown_info(*selector_area, f.area().height)
            {
                dropdown.render(f);
            }
        }
    }
}

impl TabInput for NseTab {
    tab_input_boilerplate!(
        NseTab,
        core: core,
        focus: focus_area,
        Inputs: NseFocusArea::Inputs,
        Results: NseFocusArea::Results
    );

    fn handle_char(&mut self, c: char) {
        let running = self.is_running();
        let inputs = self.focus_area == NseFocusArea::Inputs;

        // Search mode: all chars go to search query when in Results focus
        #[cfg(feature = "nse")]
        if self.focus_area == NseFocusArea::Results && self.detail_view_active {
            self.search_push_char(c);
            return;
        }

        crate::tabs::core::tab_input_char(&mut self.core, c, running, inputs);

        // Filter cycling: 'f' in Results focus
        #[cfg(feature = "nse")]
        if self.focus_area == NseFocusArea::Results && c == 'f' && !running && self.has_report() {
            self.cycle_report_filter();
        }

        // Section jump: '1'-'8' in Results focus
        #[cfg(feature = "nse")]
        if self.focus_area == NseFocusArea::Results && !running && self.has_report() {
            if let Some(digit) = c.to_digit(10) {
                let idx = digit as usize;
                if idx >= 1 && idx <= NseReportSection::ALL.len() {
                    self.jump_to_section(idx - 1);
                }
            }
            // Start search mode
            if c == 's' {
                self.detail_view_active = true;
                self.report_search.clear();
                return;
            }
        }
    }

    fn handle_backspace(&mut self) {
        #[cfg(feature = "nse")]
        if self.detail_view_active && !self.report_search.is_empty() {
            self.search_pop_char();
            return;
        }

        let running = self.is_running();
        let inputs = self.focus_area == NseFocusArea::Inputs;
        crate::tabs::core::tab_input_backspace(&mut self.core, running, inputs);
    }

    fn handle_paste(&mut self, text: &str) {
        let running = self.is_running();
        let inputs = self.focus_area == NseFocusArea::Inputs;
        crate::tabs::core::tab_input_paste(&mut self.core, text, running, inputs);
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            NseFocusArea::Inputs => NseFocusArea::ScriptSelector,
            NseFocusArea::ScriptSelector => NseFocusArea::Results,
            NseFocusArea::Results => NseFocusArea::Inputs,
        };
        self.core.inputs.set_focus_for_index(match self.focus_area {
            NseFocusArea::Inputs => Some(0),
            _ => None,
        });
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            NseFocusArea::Inputs => NseFocusArea::Results,
            NseFocusArea::ScriptSelector => NseFocusArea::Inputs,
            NseFocusArea::Results => NseFocusArea::ScriptSelector,
        };
        self.core.inputs.set_focus_for_index(match self.focus_area {
            NseFocusArea::Inputs => Some(0),
            _ => None,
        });
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == NseFocusArea::Inputs {
            self.core.inputs.move_right()
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == NseFocusArea::Inputs && self.core.inputs.is_focused()
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
        // Clear search first, then filter, then normal escape
        #[cfg(feature = "nse")]
        if self.detail_view_active && !self.report_search.is_empty() {
            self.search_clear();
            return;
        }
        #[cfg(feature = "nse")]
        if self.detail_view_active || self.report_filter.is_some() {
            self.clear_filter();
            return;
        }
        self.core.inputs.blur();
        self.script_selector.blur();
        self.focus_area = NseFocusArea::Inputs;
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
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
        if self.is_running() {
            return;
        }
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
}

impl NseTab {
    pub fn start(&mut self) {
        if self.target().is_empty() {
            // Previously a silent `return`: the tab stayed Idle with no
            // explanation, which read as "nothing happens". Say why.
            self.core.error = Some(TabError::Target(
                "Target host / URL is required to run an NSE scan".to_string(),
            ));
            return;
        }
        // A custom script path used to be refused here because `NseParams` had
        // nowhere to carry it, so the only canonical executor could run a named
        // built-in script. `NseParams::custom_script` now carries the path and
        // the engine resolves it through `NseScriptSource::File` /
        // `ScriptResolver`, so the operator gets the script they actually
        // picked. Keep a blank-path check rather than trusting the builder:
        // a whitespace-only path would otherwise resolve as a literal filename.
        if let Some(path) = self.custom_script() {
            if path.trim().is_empty() {
                self.core.error = Some(TabError::Config(
                    "The custom NSE script path is blank. Clear the field to use \
                     the selected built-in script."
                        .to_string(),
                ));
                return;
            }
        }
        self.core.error = None;
        if self.core.state != AppState::Running {
            self.core.progress.current = 0;
            self.core.progress.total = 0;
            self.core.state = AppState::Running;
        }
    }

    /// True when the operator asked for a custom script rather than a
    /// built-in one, either via the selector or a typed path.
    pub fn uses_custom_script(&self) -> bool {
        self.custom_script().is_some() || self.script() == "custom"
    }

    /// Cycle through report filters: None → Summary → Compatibility → ... → Diagnostics → None.
    #[cfg(feature = "nse")]
    pub fn cycle_report_filter(&mut self) {
        self.report_filter = match self.report_filter {
            None => Some(NseReportSection::Summary),
            Some(NseReportSection::Summary) => Some(NseReportSection::Compatibility),
            Some(NseReportSection::Compatibility) => Some(NseReportSection::RuleEvaluation),
            Some(NseReportSection::RuleEvaluation) => Some(NseReportSection::Libraries),
            Some(NseReportSection::Libraries) => Some(NseReportSection::CapabilityDenials),
            Some(NseReportSection::CapabilityDenials) => Some(NseReportSection::Evidence),
            Some(NseReportSection::Evidence) => Some(NseReportSection::RawOutput),
            Some(NseReportSection::RawOutput) => Some(NseReportSection::Diagnostics),
            Some(NseReportSection::Diagnostics) => None,
        };
        self.refresh_filtered_view();
    }

    /// Jump to a specific section by index (0-based among visible sections).
    #[cfg(feature = "nse")]
    pub fn jump_to_section(&mut self, index: usize) {
        if index < NseReportSection::ALL.len() {
            self.detail_section_index = index;
            self.detail_view_active = true;
            self.refresh_filtered_view();
        }
    }

    /// Clear the active filter, showing all sections.
    #[cfg(feature = "nse")]
    pub fn clear_filter(&mut self) {
        self.report_filter = None;
        self.report_search.clear();
        self.detail_view_active = false;
        self.detail_section_index = 0;
        self.refresh_filtered_view();
    }

    /// Add a character to the search query.
    #[cfg(feature = "nse")]
    pub fn search_push_char(&mut self, c: char) {
        self.report_search.push(c);
        self.refresh_filtered_view();
    }

    /// Remove the last character from the search query.
    #[cfg(feature = "nse")]
    pub fn search_pop_char(&mut self) {
        self.report_search.pop();
        self.refresh_filtered_view();
    }

    /// Clear the search query.
    #[cfg(feature = "nse")]
    pub fn search_clear(&mut self) {
        self.report_search.clear();
        self.refresh_filtered_view();
    }

    /// Refresh the results view based on current filter and search state.
    #[cfg(feature = "nse")]
    fn refresh_filtered_view(&mut self) {
        if self.report_sections.is_empty() {
            return;
        }
        let view = &mut self.core.results_view;
        view.clear();

        let filter = if self.detail_view_active {
            NseReportSection::ALL
                .get(self.detail_section_index)
                .copied()
        } else {
            self.report_filter
        };

        let search = if self.report_search.is_empty() {
            None
        } else {
            Some(self.report_search.as_str())
        };

        let lines = render_filtered_report(&self.report_sections, filter, search);
        for line in lines {
            view.add_line(line);
        }
    }

    /// Get the current filter label for display.
    #[cfg(feature = "nse")]
    pub fn filter_label(&self) -> Option<&'static str> {
        if let Some(filter) = self.report_filter {
            Some(filter.label())
        } else if self.detail_view_active {
            NseReportSection::ALL
                .get(self.detail_section_index)
                .map(|s| s.label())
        } else {
            None
        }
    }

    /// Check if a structured report is loaded.
    #[cfg(feature = "nse")]
    pub fn has_report(&self) -> bool {
        self.structured_report.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_dropdown_is_drawn_when_expanded() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut tab = NseTab::new();
        tab.script_selector.focus();
        tab.focus_area = NseFocusArea::ScriptSelector;
        tab.handle_enter();
        assert!(tab.script_selector.is_open());

        // Anchor mirrors `render_overlays`: the second layout row.
        let anchor = Rect::new(0, 12, 80, 4);
        let info = tab
            .script_selector
            .dropdown_info(anchor, 24)
            .expect("expanded script selector must yield a dropdown");
        assert_eq!(info.area.y, anchor.y + anchor.height);
        assert!(info
            .items
            .iter()
            .any(|(i, label, _)| *i == 2 && label == "Banner Grab"));

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                tab.render(f, area, false);
                tab.render_overlays(f, area);
            })
            .unwrap();

        let text = crate::test_utils::buffer_to_text(terminal.backend().buffer());
        // Only the expanded list shows the non-selected scripts.
        assert!(
            text.contains("Banner Grab") && text.contains("Custom Script"),
            "expanded script dropdown should be drawn"
        );
    }

    /// A missing target used to leave the tab silently Idle.
    #[test]
    fn start_without_target_reports_an_error() {
        let mut tab = NseTab::new();
        tab.focus_area = NseFocusArea::Inputs;
        tab.handle_enter();
        assert!(tab.core.error.is_some(), "empty target must explain itself");
        assert_eq!(tab.core.state, AppState::Idle);
    }

    /// A custom script path now crosses `NseParams::custom_script`, so Enter
    /// starts the run instead of refusing. The path must reach the request
    /// unchanged: it is the operator's chosen script, and substituting the
    /// built-in one would run a different check than the one on screen.
    #[test]
    fn start_with_custom_script_path_dispatches_the_path() {
        use crate::app::task_management::TaskBuilder;
        use eggsec_runtime::request::TaskKind;

        let mut tab = NseTab::new();
        tab.core.inputs.fields.get_mut(0).unwrap().value = "example.com".into();
        tab.core.inputs.fields.get_mut(2).unwrap().value = "/tmp/custom.nse".into();
        assert!(tab.uses_custom_script());

        tab.start();
        assert!(tab.core.error.is_none(), "{:?}", tab.core.error);
        assert_eq!(tab.core.state, AppState::Running);

        let req = tab
            .build_run_request()
            .expect("a runnable NSE tab must produce a request");
        let TaskKind::Nse(params) = req.task_kind else {
            panic!("expected an Nse task kind, got {:?}", req.task_kind);
        };
        assert_eq!(
            params.custom_script.as_deref(),
            Some("/tmp/custom.nse"),
            "the operator's script path must reach the engine verbatim"
        );
    }

    /// A whitespace-only path would otherwise resolve as a literal filename.
    #[test]
    fn start_with_blank_custom_script_path_reports_an_error() {
        let mut tab = NseTab::new();
        tab.core.inputs.fields.get_mut(0).unwrap().value = "example.com".into();
        tab.core.inputs.fields.get_mut(2).unwrap().value = "   ".into();

        tab.start();
        assert!(
            tab.core.error.is_some(),
            "a blank custom script path must be reported, not dispatched"
        );
        assert_eq!(tab.core.state, AppState::Idle);
    }

    #[test]
    fn start_with_built_in_script_enters_running() {
        let mut tab = NseTab::new();
        tab.core.inputs.fields.get_mut(0).unwrap().value = "example.com".into();
        assert!(!tab.uses_custom_script());
        tab.start();
        assert!(tab.core.error.is_none());
        assert_eq!(tab.core.state, AppState::Running);
    }
}
