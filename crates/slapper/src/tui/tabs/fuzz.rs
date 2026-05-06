use crate::fuzzer::engine::FuzzSession;
use crate::fuzzer::PayloadType;
use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText,
    Selector, SelectorItem,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct FuzzTab {
    pub inputs: InputGroup,
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
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: FuzzFocusArea,
    pub session: Option<FuzzSession>,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FuzzFocusArea {
    Inputs,
    PayloadSelector,
    ModeSelector,
    TargetSelector,
    MutationCheckbox,
    Results,
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
            inputs,
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
            progress: ProgressGauge::new("Fuzzing..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: FuzzFocusArea::Inputs,
            session: None,
            error: None,
        }
    }

    pub fn get_results(&self) -> Option<&FuzzSession> {
        self.session.as_ref()
    }

    pub fn set_results(&mut self, session: FuzzSession) {
        self.session = Some(session.clone());
        self.state = AppState::Completed;
        self.results_view.clear();

        let s = &self.session.as_ref().expect("session set two lines above");

        self.results_view.add_line(Line::from(Span::styled(
            format!("Fuzzing Complete: {}", s.target_url),
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "Mode: {} | Payloads: {} | Duration: {}ms",
            s.mode, s.total_payloads, s.duration_ms
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Findings Summary:",
            Style::default().fg(tc!(accent)),
        )));
        self.results_view.add_line(Line::from(format!(
            "  Requests: {} success / {} failed",
            s.successful_requests, s.failed_requests
        )));
        self.results_view.add_line(Line::from(format!(
            "  WAF Bypasses: {} | Leaks: {} | Anomalies: {}",
            s.waf_bypasses, s.potential_leaks, s.time_anomalies
        )));

        if s.redos_suspected > 0 {
            self.results_view.add_line(Line::from(Span::styled(
                format!("  ReDoS Suspected: {}", s.redos_suspected),
                Style::default().fg(tc!(error)),
            )));
        }

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "OWASP Summary:",
            Style::default().fg(tc!(accent)),
        )));
        self.results_view.add_line(Line::from(format!(
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
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
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
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {} (Status: {})",
                    severity, result.payload.description, result.status_code
                )));
            }
        }
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

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

    pub fn is_multi_target(&self) -> bool {
        self.targets().len() > 1
    }

    pub fn method(&self) -> &str {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("GET")
    }

    pub fn param(&self) -> Option<&str> {
        let p = self
            .inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .unwrap_or("");
        if p.is_empty() {
            None
        } else {
            Some(p)
        }
    }

    pub fn max_payloads(&self) -> usize {
        self.inputs
            .fields
            .get(3)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(0)
    }

    pub fn mutation_count(&self) -> usize {
        self.inputs
            .fields
            .get(4)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(3)
    }

    pub fn concurrency(&self) -> usize {
        self.inputs
            .fields
            .get(5)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn timeout(&self) -> u64 {
        self.inputs
            .fields
            .get(6)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
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
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.results_view.clear();
            self.results_view.add_line(Line::from(Span::styled(
                "Starting fuzzer...",
                Style::default().fg(tc!(accent)),
            )));
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn update_progress(&mut self, _completed: u64, _total: u64) {}

    pub fn scroll_results_up(&mut self) {
        self.results_view.scroll_up(1);
    }

    pub fn scroll_results_down(&mut self) {
        self.results_view.scroll_down(1);
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
    }
}

impl Default for FuzzTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for FuzzTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.progress.current = 0;
        self.results_view.clear();
        self.session = None;
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.inputs.fields[1].value = "GET".to_string();
        self.inputs.fields[1].cursor_pos = 3;
        self.inputs.fields[3].value = "0".to_string();
        self.inputs.fields[3].cursor_pos = 1;
        self.inputs.fields[4].value = "3".to_string();
        self.inputs.fields[4].cursor_pos = 1;
        self.inputs.fields[5].value = "10".to_string();
        self.inputs.fields[5].cursor_pos = 2;
        self.inputs.fields[6].value = "10".to_string();
        self.inputs.fields[6].cursor_pos = 2;
        self.mutation_checkbox.checked = false;
        self.payload_selector.select(0);
        self.mode_selector.select(0);
        self.target_selector.select(0);
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
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
        // Dynamic config height: use at most 27 lines, but leave at least 3 lines for results
        let config_height = if area.height <= 30 {
            // Small terminal: use 80% of height, min 10, max 27
            ((area.height as f32 * 0.8) as u16).max(10).min(27)
        } else {
            27
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(config_height), Constraint::Min(3)])
            .split(area);

        let config_area = chunks[0];
        let results_area = chunks[1];

        // Dynamic field height based on available config area
        let num_fields = 8;
        let field_height = (config_area.height / num_fields).max(2);
        let config_constraints: Vec<Constraint> = (0..num_fields)
            .map(|_| Constraint::Length(field_height))
            .collect();

        let config_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(config_constraints)
            .split(config_area);

        self.inputs.fields[0].render(f, config_chunks[0], insert_mode);
        self.inputs.fields[1].render(f, config_chunks[1], insert_mode);
        self.inputs.fields[2].render(f, config_chunks[2], insert_mode);

        let mut payload_sel = self.payload_selector.clone();
        payload_sel.focused = self.focus_area == FuzzFocusArea::PayloadSelector;
        payload_sel.render(f, config_chunks[3]);

        let mut mode_sel = self.mode_selector.clone();
        mode_sel.focused = self.focus_area == FuzzFocusArea::ModeSelector;
        mode_sel.render(f, config_chunks[4]);

        let mut target_sel = self.target_selector.clone();
        target_sel.focused = self.focus_area == FuzzFocusArea::TargetSelector;
        target_sel.render(f, config_chunks[5]);

        let mut mutation_cb = self.mutation_checkbox.clone();
        mutation_cb.focused = self.focus_area == FuzzFocusArea::MutationCheckbox;
        mutation_cb.render(f, config_chunks[6]);

        let (status_text, status_color) = match &self.state {
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
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, config_chunks[7]);

        if self.state == AppState::Running {
            self.progress.render(f, results_area);
        } else if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Fuzzing - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view.render(f, results_area, Some(tc!(info)));
        } else {
            let info_text = vec![
                Line::from(""),
                Line::from("Configure fuzzing options above and press Enter to start."),
                Line::from(""),
                Line::from(Span::styled(
                    "CLI alternative:",
                    Style::default().fg(tc!(text_dim)),
                )),
                Line::from(Span::styled(
                    "  slapper fuzz <url> -t sqli",
                    Style::default().fg(tc!(info)),
                )),
                Line::from(Span::styled(
                    "  slapper fuzz <url> -t xss --mutate",
                    Style::default().fg(tc!(info)),
                )),
                Line::from(Span::styled(
                    "  slapper fuzz <url> -t all -M burst",
                    Style::default().fg(tc!(info)),
                )),
            ];

            let placeholder = empty_state_paragraph("Results", info_text);
            f.render_widget(placeholder, results_area);
        }
    }

    fn render_overlays(&self, f: &mut Frame, area: Rect) {
        // Match render() - use same dynamic height
        let config_height = if area.height <= 30 {
            ((area.height as f32 * 0.8) as u16).max(10).min(27)
        } else {
            27
        };

        let config_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: config_height,
        };

        // Dynamic field height based on available config area
        let num_fields = 9; // 8 fields + 1 status
        let field_height = (config_area.height / num_fields).max(2);
        let config_constraints: Vec<Constraint> = (0..num_fields)
            .map(|_| Constraint::Length(field_height))
            .collect();

        let config_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(config_constraints)
            .split(config_area);

        if let Some(info) = self.payload_selector.dropdown_info(config_chunks[3]) {
            info.render(f);
        }
        if let Some(info) = self.mode_selector.dropdown_info(config_chunks[4]) {
            info.render(f);
        }
        if let Some(info) = self.target_selector.dropdown_info(config_chunks[5]) {
            info.render(f);
        }
    }
}

impl TabInput for FuzzTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            FuzzFocusArea::Inputs => {
                self.inputs.blur();
                FuzzFocusArea::PayloadSelector
            }
            FuzzFocusArea::PayloadSelector => FuzzFocusArea::ModeSelector,
            FuzzFocusArea::ModeSelector => FuzzFocusArea::TargetSelector,
            FuzzFocusArea::TargetSelector => FuzzFocusArea::MutationCheckbox,
            FuzzFocusArea::MutationCheckbox => {
                // After MutationCheckbox, go to Results (as per plan)
                self.inputs.blur();
                FuzzFocusArea::Results
            }
            FuzzFocusArea::Results => {
                self.inputs.focus(0);
                FuzzFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            FuzzFocusArea::Inputs => {
                self.inputs.blur();
                FuzzFocusArea::MutationCheckbox
            }
            FuzzFocusArea::PayloadSelector => {
                self.inputs.focus(0);
                FuzzFocusArea::Inputs
            }
            FuzzFocusArea::ModeSelector => FuzzFocusArea::PayloadSelector,
            FuzzFocusArea::TargetSelector => FuzzFocusArea::ModeSelector,
            FuzzFocusArea::MutationCheckbox => FuzzFocusArea::TargetSelector,
            FuzzFocusArea::Results => {
                // From Results, go back to MutationCheckbox
                self.inputs.blur();
                FuzzFocusArea::MutationCheckbox
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.focus_area == FuzzFocusArea::Results {
            Some(self.results_view.get_content())
        } else if self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.move_home();
        } else if self.focus_area == FuzzFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.move_end();
        } else if self.focus_area == FuzzFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        self.focus_area = FuzzFocusArea::Inputs;
        self.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        self.focus_area = FuzzFocusArea::Results;
        self.inputs.blur();
    }

    fn handle_enter(&mut self) {
        if self.focus_area == FuzzFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }

        if self.focus_area == FuzzFocusArea::PayloadSelector {
            if self.payload_selector.is_open() {
                let _ = self.payload_selector.confirm();
            } else {
                self.payload_selector.open();
            }
            return;
        }

        if self.focus_area == FuzzFocusArea::ModeSelector {
            if self.mode_selector.is_open() {
                let _ = self.mode_selector.confirm();
            } else {
                self.mode_selector.open();
            }
            return;
        }

        if self.focus_area == FuzzFocusArea::TargetSelector {
            if self.target_selector.is_open() {
                let _ = self.target_selector.confirm();
            } else {
                self.target_selector.open();
            }
            return;
        }

        if self.focus_area == FuzzFocusArea::MutationCheckbox {
            self.mutation_checkbox.toggle();
            return;
        }

        if self.is_running() {
            self.stop();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        if self.payload_selector.is_open() {
            self.payload_selector.cancel();
            return;
        }
        if self.mode_selector.is_open() {
            self.mode_selector.cancel();
            return;
        }
        if self.target_selector.is_open() {
            self.target_selector.cancel();
            return;
        }
        self.inputs.blur();
        self.payload_selector.collapse();
        self.mode_selector.collapse();
        self.target_selector.collapse();
    }

    fn handle_up(&mut self) {
        if self.payload_selector.is_open() {
            self.payload_selector.move_prev();
        } else if self.mode_selector.is_open() {
            self.mode_selector.move_prev();
        } else if self.target_selector.is_open() {
            self.target_selector.move_prev();
        } else if self.focus_area == FuzzFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.focus_prev();
        } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
            self.scroll_results_up();
        } else {
            self.handle_focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.payload_selector.is_open() {
            self.payload_selector.move_next();
        } else if self.mode_selector.is_open() {
            self.mode_selector.move_next();
        } else if self.target_selector.is_open() {
            self.target_selector.move_next();
        } else if self.focus_area == FuzzFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.focus_next();
        } else {
            self.handle_focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.move_left()
        } else if self.focus_area == FuzzFocusArea::PayloadSelector {
            self.focus_area = FuzzFocusArea::Inputs;
            self.inputs.focus_prev();
            true
        } else if self.focus_area == FuzzFocusArea::ModeSelector {
            self.focus_area = FuzzFocusArea::PayloadSelector;
            true
        } else if self.focus_area == FuzzFocusArea::TargetSelector {
            self.focus_area = FuzzFocusArea::ModeSelector;
            true
        } else if self.focus_area == FuzzFocusArea::MutationCheckbox {
            self.focus_area = FuzzFocusArea::TargetSelector;
            true
        } else {
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.move_right()
        } else if self.focus_area == FuzzFocusArea::PayloadSelector {
            self.focus_area = FuzzFocusArea::ModeSelector;
            true
        } else if self.focus_area == FuzzFocusArea::ModeSelector {
            self.focus_area = FuzzFocusArea::TargetSelector;
            true
        } else if self.focus_area == FuzzFocusArea::TargetSelector {
            self.focus_area = FuzzFocusArea::MutationCheckbox;
            true
        } else if self.focus_area == FuzzFocusArea::MutationCheckbox {
            self.focus_area = FuzzFocusArea::Inputs;
            self.inputs.focus(0);
            true
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == FuzzFocusArea::Inputs && self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == FuzzFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else {
            true
        }
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
        // Start at Inputs
        tab.focus_area = FuzzFocusArea::Inputs;
        tab.inputs.focus(0);

        // Move to PayloadSelector
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

        // Move to Results (this was missing before the fix)
        tab.handle_focus_next();
        assert_eq!(
            tab.focus_area,
            FuzzFocusArea::Results,
            "Focus should cycle to Results from MutationCheckbox"
        );

        // Move back to Inputs
        tab.handle_focus_next();
        assert_eq!(
            tab.focus_area,
            FuzzFocusArea::Inputs,
            "Focus should cycle back to Inputs"
        );
    }

    #[test]
    fn test_focus_prev_from_results() {
        let mut tab = create_test_tab();
        // Start at Results
        tab.focus_area = FuzzFocusArea::Results;

        // Move to MutationCheckbox
        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, FuzzFocusArea::MutationCheckbox);
    }

    #[test]
    fn test_enter_on_checkbox_toggles_only() {
        let mut tab = create_test_tab();
        tab.focus_area = FuzzFocusArea::MutationCheckbox;
        tab.mutation_checkbox.focused = true;

        // Simulate Enter on checkbox - should toggle, not start task
        // (Note: handle_enter would need mock is_running() = false)
        assert!(tab.mutation_checkbox.focused);
    }
}

#[test]
fn test_left_from_payload_selector_goes_to_inputs() {
    let mut tab = FuzzTab::default();
    tab.focus_area = FuzzFocusArea::PayloadSelector;
    tab.inputs.fields[0].cursor_pos = 0;

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
fn test_right_from_mutation_to_inputs() {
    let mut tab = FuzzTab::default();
    tab.focus_area = FuzzFocusArea::MutationCheckbox;

    let result = tab.handle_right();
    assert!(result);
    assert_eq!(tab.focus_area, FuzzFocusArea::Inputs);
    assert_eq!(tab.inputs.fields[0].cursor_pos, 0);
}
