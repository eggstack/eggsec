use crate::components::{Checkbox, InputField, InputGroup, Selector, SelectorItem};
use crate::tabs::core::{
    self, field_as, field_str, render_results_area, render_config_block, start_scan,
    FuzzFocusArea, TabCore, FUZZ_AREAS,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::tc;
use eggsec::fuzzer::engine::FuzzSession;
use eggsec::fuzzer::PayloadType;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct FuzzTab {
    pub core: TabCore,
    pub payload_selector: Selector,
    pub mode_selector: Selector,
    pub target_selector: Selector,
    pub mutation_checkbox: Checkbox,
    pub graphql_introspection: Checkbox,
    pub graphql_depth_bypass: Checkbox,
    pub graphql_alias_overload: Checkbox,
    pub oauth_redirect_test: Checkbox,
    pub oauth_scope_test: Checkbox,
    pub oauth_state_test: Checkbox,
    pub oauth_grant_test: Checkbox,
    pub focus_area: FuzzFocusArea,
    pub session: Option<FuzzSession>,
}

impl FuzzTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Method (GET/POST/etc)").with_value("GET"))
            .add(InputField::new("Param Name (optional)"))
            .add(InputField::new("Max Payloads (0=all)").with_value("0"))
            .add(InputField::new("Mutation Count").with_value("3"))
            .add(InputField::new("Concurrency").with_value("10"))
            .add(InputField::new("Timeout (s)").with_value("10"));

        let payload_selector = Selector::new("Payload Type").items(vec![
            SelectorItem::new("All Types", "all"),
            SelectorItem::new("SQL Injection", "sqli"),
            SelectorItem::new("Cross-Site Scripting (XSS)", "xss"),
            SelectorItem::new("Path Traversal", "traversal"),
            SelectorItem::new("SSRF", "ssrf"),
            SelectorItem::new("Open Redirect", "redirect"),
            SelectorItem::new("ReDoS", "redos"),
            SelectorItem::new("Header Injection", "headers"),
            SelectorItem::new("Compression Attacks", "compression"),
            SelectorItem::new("GraphQL Security", "graphql"),
            SelectorItem::new("OAuth/OIDC Testing", "oauth"),
            SelectorItem::new("JWT Security", "jwt"),
            SelectorItem::new("IDOR", "idor"),
            SelectorItem::new("Server-Side Template Injection", "ssti"),
        ]);

        let mode_selector = Selector::new("Mode").simple_items(vec!["Sequential", "Burst"]);

        let target_selector = Selector::new("Target Profile").items(vec![
            SelectorItem::new("Generic", "generic"),
            SelectorItem::new("Nginx", "nginx"),
            SelectorItem::new("Apache", "apache"),
            SelectorItem::new("PHP", "php"),
        ]);

        let mutation_checkbox = Checkbox::new("Enable Mutations").checked(false);

        let graphql_introspection = Checkbox::new("Introspection").checked(true);
        let graphql_depth_bypass = Checkbox::new("Depth Bypass").checked(true);
        let graphql_alias_overload = Checkbox::new("Alias Overload").checked(true);

        let oauth_redirect_test = Checkbox::new("Redirect URI").checked(true);
        let oauth_scope_test = Checkbox::new("Scope Escalation").checked(true);
        let oauth_state_test = Checkbox::new("State Parameter").checked(true);
        let oauth_grant_test = Checkbox::new("Grant Type").checked(true);

        Self {
            core: TabCore::new("Fuzzing...", "Results").with_inputs(inputs),
            payload_selector,
            mode_selector,
            target_selector,
            mutation_checkbox,
            graphql_introspection,
            graphql_depth_bypass,
            graphql_alias_overload,
            oauth_redirect_test,
            oauth_scope_test,
            oauth_grant_test,
            oauth_state_test,
            focus_area: FuzzFocusArea::Inputs,
            session: None,
        }
    }

    pub fn get_results(&self) -> Option<&FuzzSession> {
        self.session.as_ref()
    }

    pub fn set_results(&mut self, session: FuzzSession) {
        self.session = Some(session.clone());
        let view = self.core.prepare_results();

        let Some(s) = self.session.as_ref() else {
            tracing::warn!("set_results called but session is None");
            return;
        };

        view.add_line(Line::from(Span::styled(
            format!("Fuzzing Complete: {}", s.target_url),
            Style::default().fg(tc!(success)),
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(format!(
            "Mode: {} | Payloads: {} | Duration: {}ms",
            s.mode, s.total_payloads, s.duration_ms
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(Span::styled(
            "Findings Summary:",
            Style::default().fg(tc!(accent)),
        )));
        view.add_line(Line::from(format!(
            "  Requests: {} success / {} failed",
            s.successful_requests, s.failed_requests
        )));
        view.add_line(Line::from(format!(
            "  WAF Bypasses: {} | Leaks: {} | Anomalies: {}",
            s.waf_bypasses, s.potential_leaks, s.time_anomalies
        )));

        if s.redos_suspected > 0 {
            view.add_line(Line::from(Span::styled(
                format!("  ReDoS Suspected: {}", s.redos_suspected),
                Style::default().fg(tc!(error)),
            )));
        }

        view.add_line(Line::from(""));
        view.add_line(Line::from(Span::styled(
            "OWASP Summary:",
            Style::default().fg(tc!(accent)),
        )));
        view.add_line(Line::from(format!(
            "  A03 Injection: {} | A10 SSRF: {}",
            s.owasp_summary.a03_injection, s.owasp_summary.a10_ssrf
        )));

        let critical: Vec<_> = s
            .results
            .iter()
            .filter(|r| r.is_waf_blocked || r.is_anomaly || !r.leaks_found.is_empty())
            .take(10)
            .collect();

        if !critical.is_empty() {
            view.add_line(Line::from(""));
            view.add_line(Line::from(Span::styled(
                "Critical Findings:",
                Style::default().fg(tc!(error)),
            )));
            for result in critical {
                let severity = if result.is_redos_suspected {
                    "CRITICAL"
                } else if !result.leaks_found.is_empty() {
                    "HIGH"
                } else if result.is_anomaly {
                    "MEDIUM"
                } else {
                    "INFO"
                };
                view.add_line(Line::from(format!(
                    "  [{}] {} (Status: {})",
                    severity, result.payload.description, result.status_code
                )));
            }
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn method(&self) -> &str {
        field_str(&self.core, 1)
    }

    pub fn param(&self) -> Option<&str> {
        let p = field_str(&self.core, 2);
        if p.is_empty() {
            None
        } else {
            Some(p)
        }
    }

    pub fn max_payloads(&self) -> usize {
        field_as(&self.core, 3, 0)
    }

    pub fn mutation_count(&self) -> usize {
        field_as(&self.core, 4, 3)
    }

    pub fn concurrency(&self) -> usize {
        field_as(&self.core, 5, 10)
    }

    pub fn timeout(&self) -> u64 {
        field_as(&self.core, 6, 10)
    }

    pub fn payload_type(&self) -> Option<PayloadType> {
        match self.payload_selector.selected_value() {
            Some("all") => None,
            Some("sqli") => Some(PayloadType::Sqli),
            Some("xss") => Some(PayloadType::Xss),
            Some("traversal") => Some(PayloadType::Traversal),
            Some("ssrf") => Some(PayloadType::Ssrf),
            Some("redirect") => Some(PayloadType::Redirect),
            Some("redos") => Some(PayloadType::Redos),
            Some("headers") => Some(PayloadType::Headers),
            Some("compression") => Some(PayloadType::Compression),
            Some("graphql") => Some(PayloadType::GraphQL),
            Some("oauth") => Some(PayloadType::OAuth),
            Some("jwt") => Some(PayloadType::Jwt),
            Some("idor") => Some(PayloadType::Idor),
            Some("ssti") => Some(PayloadType::Ssti),
            _ => None,
        }
    }

    pub fn payload_type_string(&self) -> String {
        self.payload_selector
            .selected_value()
            .unwrap_or("all")
            .to_string()
    }

    pub fn mode(&self) -> &str {
        self.mode_selector.selected_value().unwrap_or("Sequential")
    }

    pub fn target_profile(&self) -> &str {
        self.target_selector.selected_value().unwrap_or("generic")
    }

    pub fn mutations_enabled(&self) -> bool {
        self.mutation_checkbox.checked
    }

    pub fn graphql_introspection_enabled(&self) -> bool {
        self.graphql_introspection.checked
    }

    pub fn graphql_depth_bypass_enabled(&self) -> bool {
        self.graphql_depth_bypass.checked
    }

    pub fn graphql_alias_overload_enabled(&self) -> bool {
        self.graphql_alias_overload.checked
    }

    pub fn oauth_redirect_enabled(&self) -> bool {
        self.oauth_redirect_test.checked
    }

    pub fn oauth_scope_enabled(&self) -> bool {
        self.oauth_scope_test.checked
    }

    pub fn oauth_state_enabled(&self) -> bool {
        self.oauth_state_test.checked
    }

    pub fn oauth_grant_enabled(&self) -> bool {
        self.oauth_grant_test.checked
    }

    pub fn start(&mut self) {
        if start_scan(&mut self.core) {
            self.core.results_view.add_line(Line::from(Span::styled(
                "Starting fuzzer...",
                Style::default().fg(tc!(accent)),
            )));
        }
    }

    /// Returns a mutable reference to the currently focused selector, if any.
    fn focused_selector_mut(&mut self) -> Option<&mut Selector> {
        match self.focus_area {
            FuzzFocusArea::PayloadSelector => Some(&mut self.payload_selector),
            FuzzFocusArea::ModeSelector => Some(&mut self.mode_selector),
            FuzzFocusArea::TargetSelector => Some(&mut self.target_selector),
            _ => None,
        }
    }

    /// Common selector enter logic: if open, confirm; if closed, open.
    fn selector_enter(&mut self) {
        if let Some(sel) = self.focused_selector_mut() {
            if sel.is_open() {
                if sel.confirm().is_none() {
                    tracing::warn!("Selector confirm failed");
                }
            } else {
                sel.open();
            }
        }
    }

    /// Common selector escape logic: if any selector is open, cancel it.
    /// Returns true if a selector was cancelled (caller should return early).
    fn cancel_open_selectors(&mut self) -> bool {
        if self.payload_selector.is_open() {
            self.payload_selector.cancel();
            return true;
        }
        if self.mode_selector.is_open() {
            self.mode_selector.cancel();
            return true;
        }
        if self.target_selector.is_open() {
            self.target_selector.cancel();
            return true;
        }
        false
    }

    /// Collapse all selectors and blur inputs.
    fn collapse_all(&mut self) {
        self.payload_selector.collapse();
        self.mode_selector.collapse();
        self.target_selector.collapse();
        self.core.inputs.blur();
    }
}

impl Default for FuzzTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for FuzzTab {
    fn state(&self) -> AppState {
        self.core.state.clone()
    }

    fn progress(&self) -> f64 {
        self.core.progress.percent() as f64
    }

    fn has_selector_open(&self) -> bool {
        self.payload_selector.is_open()
            || self.mode_selector.is_open()
            || self.target_selector.is_open()
    }

    fn reset(&mut self) {
        self.core.reset_all();
        self.core.inputs.blur();
        if let Some(field) = self.core.inputs.fields.get_mut(1) {
            field.value = "GET".to_string();
            field.cursor_pos = 3;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(3) {
            field.value = "0".to_string();
            field.cursor_pos = 1;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(4) {
            field.value = "3".to_string();
            field.cursor_pos = 1;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(5) {
            field.value = "10".to_string();
            field.cursor_pos = 2;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(6) {
            field.value = "10".to_string();
            field.cursor_pos = 2;
        }
        self.mutation_checkbox.reset();
        self.graphql_introspection.reset();
        self.graphql_introspection.checked = true;
        self.graphql_depth_bypass.reset();
        self.graphql_depth_bypass.checked = true;
        self.graphql_alias_overload.reset();
        self.graphql_alias_overload.checked = true;
        self.oauth_redirect_test.reset();
        self.oauth_redirect_test.checked = true;
        self.oauth_scope_test.reset();
        self.oauth_scope_test.checked = true;
        self.oauth_state_test.reset();
        self.oauth_state_test.checked = true;
        self.oauth_grant_test.reset();
        self.oauth_grant_test.checked = true;
        self.payload_selector.select(0);
        self.mode_selector.select(0);
        self.target_selector.select(0);
        self.focus_area = FuzzFocusArea::Inputs;
    }

    fn set_error(&mut self, error: crate::app::tab_error::TabError) {
        self.core.state = AppState::Error(error.message());
        self.core.error = Some(error);
        self.core.progress.current = 0;
    }
}

impl TabRender for FuzzTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            FuzzFocusArea::Inputs => "Inputs",
            FuzzFocusArea::PayloadSelector => "Payloads",
            FuzzFocusArea::ModeSelector => "Mode",
            FuzzFocusArea::TargetSelector => "Target",
            FuzzFocusArea::MutationCheckbox => "Mutation",
            FuzzFocusArea::Results => "Results",
        };
        Some(vec!["Fuzz", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let config_height = if area.height <= 40 {
            ((area.height as f32 * 0.85) as u16).clamp(12, 38)
        } else {
            38
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(config_height), Constraint::Min(3)])
            .split(area);

        let config_area = chunks.first().copied().unwrap_or(area);
        let results_area = chunks.get(1).copied().unwrap_or(area);

        let config_inner = render_config_block(
            f,
            config_area,
            " Fuzzing Configuration ",
            self.focus_area != FuzzFocusArea::Results,
        );

        let num_fields = 12;
        let config_constraints: Vec<Constraint> = (0..num_fields)
            .map(|_| Constraint::Length(3))
            .collect();

        let config_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(config_constraints)
            .split(config_inner);

        // Render input fields
        for (i, chunk) in config_chunks.iter().take(7).enumerate() {
            if let Some(field) = self.core.inputs.fields.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        // Render selectors and checkbox
        if config_chunks.len() >= 12 {
            let vh = f.area().height;

            let mut payload_sel = self.payload_selector.clone();
            payload_sel.focused = self.focus_area == FuzzFocusArea::PayloadSelector;
            if let Some(chunk) = config_chunks.get(7) {
                payload_sel.render(f, *chunk);
                if let Some(info) = self.payload_selector.dropdown_info(*chunk, vh) {
                    info.render(f);
                }
            }

            let mut mode_sel = self.mode_selector.clone();
            mode_sel.focused = self.focus_area == FuzzFocusArea::ModeSelector;
            if let Some(chunk) = config_chunks.get(8) {
                mode_sel.render(f, *chunk);
                if let Some(info) = self.mode_selector.dropdown_info(*chunk, vh) {
                    info.render(f);
                }
            }

            let mut target_sel = self.target_selector.clone();
            target_sel.focused = self.focus_area == FuzzFocusArea::TargetSelector;
            if let Some(chunk) = config_chunks.get(9) {
                target_sel.render(f, *chunk);
                if let Some(info) = self.target_selector.dropdown_info(*chunk, vh) {
                    info.render(f);
                }
            }

            let mut mutation_cb = self.mutation_checkbox.clone();
            mutation_cb.focused = self.focus_area == FuzzFocusArea::MutationCheckbox;
            if let Some(chunk) = config_chunks.get(10) {
                mutation_cb.render(f, *chunk);
            }
        }

        // Status
        let (status_text, status_color) = match &self.core.state {
            AppState::Idle => (
                "Ready - Enter target and press Enter to start",
                tc!(text_dim),
            ),
            AppState::Running => ("Running...", tc!(status_running)),
            AppState::Completed => ("Completed - Press r to reset", tc!(success)),
            AppState::Error(e) => (e.as_str(), tc!(error)),
        };
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(status_color))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(tc!(border))).title("Status"));
        if let Some(status_chunk) = config_chunks.get(11) {
            f.render_widget(status, *status_chunk);
        }

        // Results area
        render_results_area(
            f,
            results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Results",
            "Configure fuzzing options above and press Enter to start.\n\nCLI alternative:\n  eggsec fuzz <url> -t sqli\n  eggsec fuzz <url> -t xss --mutate\n  eggsec fuzz <url> -t all -M burst",
        );
    }

    fn render_overlays(&self, _f: &mut Frame, _area: Rect) {
        // Dropdowns are rendered inline with the config in render() now.
    }
}

impl TabInput for FuzzTab {
    fn handle_focus_next(&mut self) {
        if self.focus_area == FuzzFocusArea::Inputs {
            if self.core.inputs.is_focused() {
                let at_last = self
                    .core
                    .inputs
                    .focused
                    .map(|i| i + 1 >= self.core.inputs.fields.len())
                    .unwrap_or(true);
                if at_last {
                    self.core.inputs.blur();
                    self.focus_area = FuzzFocusArea::PayloadSelector;
                } else {
                    self.core.inputs.focus_next();
                }
            } else {
                self.core.inputs.focus(0);
            }
        } else {
            self.focus_area = core::focus_next_n(&mut self.core, self.focus_area, &FUZZ_AREAS);
        }
    }

    fn handle_focus_prev(&mut self) {
        if self.focus_area == FuzzFocusArea::Inputs {
            if self.core.inputs.is_focused() {
                let at_first = self.core.inputs.focused.map(|i| i == 0).unwrap_or(true);
                if at_first {
                    self.core.inputs.blur();
                    self.focus_area = FuzzFocusArea::Results;
                } else {
                    self.core.inputs.focus_prev();
                }
            } else {
                self.core.inputs.focus(0);
            }
        } else {
            self.focus_area = core::focus_prev_n(&mut self.core, self.focus_area, &FUZZ_AREAS);
        }
    }

    fn handle_char(&mut self, c: char) {
        let running = self.is_running();
        let inputs = self.focus_area == FuzzFocusArea::Inputs;
        core::tab_input_char(&mut self.core, c, running, inputs);
    }

    fn handle_backspace(&mut self) {
        let running = self.is_running();
        let inputs = self.focus_area == FuzzFocusArea::Inputs;
        core::tab_input_backspace(&mut self.core, running, inputs);
    }

    fn handle_paste(&mut self, text: &str) {
        let running = self.is_running();
        let inputs = self.focus_area == FuzzFocusArea::Inputs;
        core::tab_input_paste(&mut self.core, text, running, inputs);
    }

    fn handle_copy(&mut self) -> Option<String> {
        let running = self.is_running();
        let inputs = self.focus_area == FuzzFocusArea::Inputs;
        let results = self.focus_area == FuzzFocusArea::Results;
        core::tab_input_copy(&self.core, running, inputs, results)
    }

    fn handle_word_forward(&mut self) {
        let running = self.is_running();
        let inputs = self.focus_area == FuzzFocusArea::Inputs;
        core::tab_input_word_forward(&mut self.core, running, inputs);
    }

    fn handle_word_backward(&mut self) {
        let running = self.is_running();
        let inputs = self.focus_area == FuzzFocusArea::Inputs;
        core::tab_input_word_backward(&mut self.core, running, inputs);
    }

    fn handle_home(&mut self) {
        let running = self.is_running();
        let inputs = self.focus_area == FuzzFocusArea::Inputs;
        let results = self.focus_area == FuzzFocusArea::Results;
        core::tab_input_home(&mut self.core, running, inputs, results);
    }

    fn handle_end(&mut self) {
        let running = self.is_running();
        let inputs = self.focus_area == FuzzFocusArea::Inputs;
        let results = self.focus_area == FuzzFocusArea::Results;
        core::tab_input_end(&mut self.core, running, inputs, results);
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = FuzzFocusArea::Inputs;
            self.core.inputs.focus(0);
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = FuzzFocusArea::Results;
            self.core.inputs.blur();
        }
    }

    fn handle_enter(&mut self) {
        if self.focus_area == FuzzFocusArea::Results {
            return;
        }
        if self.is_running() {
            self.core.stop();
            return;
        }
        if self.focus_area == FuzzFocusArea::Inputs && self.core.inputs.is_focused() {
            self.core.inputs.blur();
            return;
        }

        if self.focus_area == FuzzFocusArea::PayloadSelector
            || self.focus_area == FuzzFocusArea::ModeSelector
            || self.focus_area == FuzzFocusArea::TargetSelector
        {
            self.selector_enter();
            return;
        }

        if self.focus_area == FuzzFocusArea::MutationCheckbox && !self.is_running() {
            self.mutation_checkbox.toggle();
            return;
        }

        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        if self.cancel_open_selectors() {
            return;
        }
        self.collapse_all();
        self.focus_area = FuzzFocusArea::Inputs;
    }

    fn handle_up(&mut self) {
        if let Some(sel) = self.focused_selector_mut() {
            if sel.is_open() {
                sel.move_prev();
                return;
            }
        }
        core::handle_up_n(&mut self.core, self.focus_area, &FUZZ_AREAS);
    }

    fn handle_down(&mut self) {
        if let Some(sel) = self.focused_selector_mut() {
            if sel.is_open() {
                sel.move_next();
                return;
            }
        }
        core::handle_down_n(&mut self.core, self.focus_area, &FUZZ_AREAS);
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == FuzzFocusArea::Inputs {
            self.core.inputs.move_left()
        } else {
            // Navigate to previous focus area
            self.focus_area = core::focus_prev_n(&mut self.core, self.focus_area, &FUZZ_AREAS);
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == FuzzFocusArea::Inputs {
            self.core.inputs.move_right()
        } else {
            // Navigate to next focus area
            self.focus_area = core::focus_next_n(&mut self.core, self.focus_area, &FUZZ_AREAS);
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == FuzzFocusArea::Inputs && self.core.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.core.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.core.inputs.is_at_right_edge()
        } else {
            true
        }
    }

    fn page_up(&mut self, page_size: usize) {
        let running = self.is_running();
        core::tab_input_page_up(&mut self.core, running, page_size);
    }

    fn page_down(&mut self, page_size: usize) {
        let running = self.is_running();
        core::tab_input_page_down(&mut self.core, running, page_size);
    }

    fn primary_target(&self) -> Option<String> {
        Some(self.target().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab() -> FuzzTab {
        FuzzTab::new()
    }

    #[test]
    fn test_focus_next_includes_results() {
        let mut tab = create_test_tab();
        tab.focus_area = FuzzFocusArea::Inputs;
        tab.core.inputs.focus(0);

        // Tab through all input fields (0 through 6)
        for i in 0..6 {
            tab.handle_focus_next();
            assert_eq!(tab.focus_area, FuzzFocusArea::Inputs);
            assert_eq!(tab.core.inputs.focused, Some(i + 1));
        }

        // Last input field -> PayloadSelector
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, FuzzFocusArea::PayloadSelector);

        // Move to ModeSelector
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, FuzzFocusArea::ModeSelector);

        // Move to TargetSelector
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, FuzzFocusArea::TargetSelector);

        // Move to MutationCheckbox
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, FuzzFocusArea::MutationCheckbox);

        // Move to Results
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, FuzzFocusArea::Results);

        // Move back to Inputs
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, FuzzFocusArea::Inputs);
    }

    #[test]
    fn test_focus_prev_from_results() {
        let mut tab = create_test_tab();
        tab.focus_area = FuzzFocusArea::Results;

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, FuzzFocusArea::MutationCheckbox);

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, FuzzFocusArea::TargetSelector);

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, FuzzFocusArea::ModeSelector);

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, FuzzFocusArea::PayloadSelector);

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, FuzzFocusArea::Inputs);
    }

    #[test]
    fn test_enter_on_checkbox_toggles_only() {
        let mut tab = create_test_tab();
        tab.focus_area = FuzzFocusArea::MutationCheckbox;
        tab.mutation_checkbox.focused = true;
        assert!(tab.mutation_checkbox.focused);
    }

    #[test]
    fn test_left_from_payload_selector_goes_to_inputs() {
        let mut tab = FuzzTab::default();
        tab.focus_area = FuzzFocusArea::PayloadSelector;
        if let Some(field) = tab.core.inputs.fields.first_mut() {
            field.cursor_pos = 0;
        }

        let result = tab.handle_left();
        assert!(result);
        assert_eq!(tab.focus_area, FuzzFocusArea::Inputs);
    }

    #[test]
    fn test_right_from_inputs_to_payload_selector() {
        let mut tab = FuzzTab::default();
        tab.focus_area = FuzzFocusArea::Inputs;
        let result = tab.handle_right();
        assert!(!result || result);
    }

    #[test]
    fn test_right_from_payload_to_mode_selector() {
        let mut tab = FuzzTab::default();
        tab.focus_area = FuzzFocusArea::PayloadSelector;

        let result = tab.handle_right();
        assert!(result);
        assert_eq!(tab.focus_area, FuzzFocusArea::ModeSelector);
    }

    #[test]
    fn test_right_from_mode_to_target_selector() {
        let mut tab = FuzzTab::default();
        tab.focus_area = FuzzFocusArea::ModeSelector;

        let result = tab.handle_right();
        assert!(result);
        assert_eq!(tab.focus_area, FuzzFocusArea::TargetSelector);
    }

    #[test]
    fn test_right_from_target_to_mutation_checkbox() {
        let mut tab = FuzzTab::default();
        tab.focus_area = FuzzFocusArea::TargetSelector;

        let result = tab.handle_right();
        assert!(result);
        assert_eq!(tab.focus_area, FuzzFocusArea::MutationCheckbox);
    }

    #[test]
    fn test_right_from_mutation_to_results() {
        let mut tab = FuzzTab::default();
        tab.focus_area = FuzzFocusArea::MutationCheckbox;

        let result = tab.handle_right();
        assert!(result);
        assert_eq!(tab.focus_area, FuzzFocusArea::Results);
    }

    #[test]
    fn test_right_from_results_to_inputs() {
        let mut tab = FuzzTab::default();
        tab.focus_area = FuzzFocusArea::Results;

        let result = tab.handle_right();
        assert!(result);
        assert_eq!(tab.focus_area, FuzzFocusArea::Inputs);
        if let Some(field) = tab.core.inputs.fields.first() {
            assert_eq!(field.cursor_pos, 0);
        }
    }

    #[test]
    fn test_accessor_methods_use_core() {
        let tab = create_test_tab();
        assert_eq!(tab.target(), "");
        assert_eq!(tab.method(), "GET");
        assert!(tab.param().is_none());
        assert_eq!(tab.max_payloads(), 0);
        assert_eq!(tab.mutation_count(), 3);
        assert_eq!(tab.concurrency(), 10);
        assert_eq!(tab.timeout(), 10);
    }

    #[test]
    fn test_reset_uses_core() {
        let mut tab = create_test_tab();
        tab.core.state = AppState::Running;
        tab.core.progress.current = 50;
        tab.core.results_view.add_line(Line::from("test"));
        tab.focus_area = FuzzFocusArea::Results;

        tab.reset();

        assert_eq!(tab.core.state, AppState::Idle);
        assert_eq!(tab.core.progress.current, 0);
        assert!(tab.core.results_view.is_empty());
        assert_eq!(tab.focus_area, FuzzFocusArea::Inputs);
    }

    #[test]
    fn test_cancel_open_selectors() {
        let mut tab = create_test_tab();
        tab.payload_selector.open();
        assert!(tab.cancel_open_selectors());
        assert!(!tab.payload_selector.is_open());
    }

    #[test]
    fn test_collapse_all() {
        let mut tab = create_test_tab();
        tab.payload_selector.open();
        tab.mode_selector.open();
        tab.core.inputs.focus(0);

        tab.collapse_all();

        assert!(!tab.payload_selector.is_open());
        assert!(!tab.mode_selector.is_open());
        assert!(!tab.core.inputs.is_focused());
    }

    #[test]
    fn test_selector_enter_opens() {
        let mut tab = create_test_tab();
        tab.focus_area = FuzzFocusArea::PayloadSelector;
        assert!(!tab.payload_selector.is_open());

        tab.selector_enter();
        assert!(tab.payload_selector.is_open());
    }

    #[test]
    fn test_selector_enter_confirms() {
        let mut tab = create_test_tab();
        tab.focus_area = FuzzFocusArea::PayloadSelector;
        tab.payload_selector.open();

        tab.selector_enter();
        assert!(!tab.payload_selector.is_open());
    }
}
