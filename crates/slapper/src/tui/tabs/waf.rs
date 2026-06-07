use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, Checkbox, InputField, InputGroup, ProgressGauge, RadioGroup,
    ScrollableText,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use crate::waf::{BypassResult, WafDetectionResult};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct WafTab {
    pub inputs: InputGroup,
    pub mode_radio: RadioGroup,
    pub technique_checkboxes: Vec<Checkbox>,
    pub detection_result: Option<WafDetectionResult>,
    pub bypass_results: Vec<BypassResult>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub detection_view: ScrollableText,
    pub bypass_view: ScrollableText,
    pub focus_area: WafFocusArea,
    pub focused_checkbox_index: usize,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WafFocusArea {
    Inputs,
    ModeRadio,
    Techniques,
    Results,
}

impl WafTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new().add(InputField::new("Target URL"));

        let mode_radio = RadioGroup::new("Mode").options(vec!["Detect Only", "Detect + Bypass"]);

        let technique_checkboxes = vec![
            Checkbox::new("Header Manipulation"),
            Checkbox::new("User-Agent Rotation").checked(true),
            Checkbox::new("X-Forwarded-For Spoof").checked(true),
            Checkbox::new("Encoding Bypass"),
            Checkbox::new("Chunked Encoding"),
            Checkbox::new("HTTP Smuggling"),
        ];

        Self {
            inputs,
            mode_radio,
            technique_checkboxes,
            detection_result: None,
            bypass_results: Vec::new(),
            progress: ProgressGauge::new("WAF Testing..."),
            state: AppState::Idle,
            detection_view: ScrollableText::new("Detection Result"),
            bypass_view: ScrollableText::new("Bypass Results"),
            focus_area: WafFocusArea::Inputs,
            focused_checkbox_index: 0,
            error: None,
        }
    }

    pub fn get_detection_result(&self) -> Option<&WafDetectionResult> {
        self.detection_result.as_ref()
    }

    pub fn get_bypass_results(&self) -> Option<&Vec<BypassResult>> {
        if self.bypass_results.is_empty() {
            None
        } else {
            Some(&self.bypass_results)
        }
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn is_bypass_mode(&self) -> bool {
        self.mode_radio.selected == Some(1)
    }

    pub fn enabled_techniques(&self) -> Vec<String> {
        self.technique_checkboxes
            .iter()
            .filter(|cb| cb.checked)
            .map(|cb| cb.label.clone())
            .collect()
    }

    pub fn set_results(&mut self, result: WafDetectionResult) {
        self.detection_result = Some(result.clone());
        self.update_detection_view(&result);
        if !self.is_bypass_mode() {
            self.state = AppState::Completed;
        }
    }

    pub fn set_bypass_results(&mut self, results: Vec<BypassResult>) {
        self.bypass_results = results.clone();
        self.update_bypass_view(&results);
        self.state = AppState::Completed;
    }

    fn update_detection_view(&mut self, result: &WafDetectionResult) {
        self.detection_view.clear();

        let waf_name = result
            .waf_name
            .clone()
            .unwrap_or_else(|| "None".to_string());
        let has_waf = result.waf_name.is_some() && waf_name != "None";
        let confidence = result.confidence;
        let matched_headers = result.matched_headers.clone();
        let matched_cookies = result.matched_cookies.clone();
        let matched_patterns = result.matched_patterns.clone();

        self.detection_view.add_line(Line::from(vec![
            Span::styled("WAF Status: ", Style::default().fg(tc!(accent))),
            if has_waf {
                Span::styled(
                    "WAF Detected!",
                    Style::default().fg(tc!(error)).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("No WAF Detected", Style::default().fg(tc!(success)))
            },
        ]));

        if has_waf {
            self.detection_view.add_line(Line::from(""));
            self.detection_view.add_line(Line::from(vec![
                Span::styled("WAF Name: ", Style::default().fg(tc!(info))),
                Span::raw(waf_name),
            ]));
            self.detection_view.add_line(Line::from(vec![
                Span::styled("Confidence: ", Style::default().fg(tc!(info))),
                Span::raw(format!("{}%", confidence)),
            ]));

            if !matched_headers.is_empty() {
                self.detection_view.add_line(Line::from(""));
                self.detection_view.add_line(Line::from(Span::styled(
                    "Matched Headers:",
                    Style::default().fg(tc!(accent)),
                )));
                for header in &matched_headers {
                    self.detection_view
                        .add_line(Line::from(format!("  • {}", header)));
                }
            }

            if !matched_cookies.is_empty() {
                self.detection_view.add_line(Line::from(""));
                self.detection_view.add_line(Line::from(Span::styled(
                    "Matched Cookies:",
                    Style::default().fg(tc!(accent)),
                )));
                for cookie in &matched_cookies {
                    self.detection_view
                        .add_line(Line::from(format!("  • {}", cookie)));
                }
            }

            if !matched_patterns.is_empty() {
                self.detection_view.add_line(Line::from(""));
                self.detection_view.add_line(Line::from(Span::styled(
                    "Matched Patterns:",
                    Style::default().fg(tc!(accent)),
                )));
                for pattern in &matched_patterns {
                    self.detection_view
                        .add_line(Line::from(format!("  • {}", pattern)));
                }
            }
        }
    }

    fn update_bypass_view(&mut self, results: &[BypassResult]) {
        self.bypass_view.clear();

        let success_count = results.iter().filter(|r| r.success).count();
        let total = results.len();

        let bypass_data: Vec<_> = results
            .iter()
            .map(|r| (r.success, r.technique, r.description.clone(), r.status_code))
            .collect();

        self.bypass_view.add_line(Line::from(vec![
            Span::styled("Successful Bypasses: ", Style::default().fg(tc!(success))),
            Span::raw(format!("{}/{}", success_count, total)),
        ]));
        self.bypass_view.add_line(Line::from(""));

        for (success, technique, description, status_code) in bypass_data {
            let (icon, color) = if success {
                ("✓", tc!(success))
            } else {
                ("✗", tc!(error))
            };

            self.bypass_view.add_line(Line::from(vec![
                Span::styled(format!("[{}] ", icon), Style::default().fg(color)),
                Span::styled(format!("{:?}", technique), Style::default().fg(tc!(accent))),
            ]));

            if !description.is_empty() {
                self.bypass_view
                    .add_line(Line::from(vec![Span::raw("    "), Span::raw(description)]));
            }

            if success {
                self.bypass_view.add_line(Line::from(vec![
                    Span::raw("    Status: "),
                    Span::styled(status_code.to_string(), Style::default().fg(tc!(success))),
                ]));
            }
            self.bypass_view.add_line(Line::from(""));
        }
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.progress.current = 0;
            self.detection_result = None;
            self.bypass_results.clear();
            self.detection_view.clear();
            self.bypass_view.clear();
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.progress.current = completed;
        self.progress.total = total;
    }

    pub fn scroll_detection_up(&mut self) {
        self.detection_view.scroll_up(1);
    }

    pub fn scroll_detection_down(&mut self) {
        self.detection_view.scroll_down(1);
    }

    pub fn scroll_bypass_up(&mut self) {
        self.bypass_view.scroll_up(1);
    }

    pub fn scroll_bypass_down(&mut self) {
        self.bypass_view.scroll_down(1);
    }

}

impl Default for WafTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for WafTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.detection_result = None;
        self.bypass_results.clear();
        self.progress.current = 0;
        self.progress.total = 0;
        self.detection_view.clear();
        self.bypass_view.clear();
        self.error = None;
        self.focused_checkbox_index = 0;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        for cb in &mut self.technique_checkboxes {
            cb.checked = false;
        }
        if let Some(cb) = self.technique_checkboxes.get_mut(1) {
            cb.checked = true;
        }
        if let Some(cb) = self.technique_checkboxes.get_mut(2) {
            cb.checked = true;
        }
        self.mode_radio.select(0);
        self.inputs.blur();
        self.focus_area = WafFocusArea::Inputs;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for WafTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            WafFocusArea::Inputs => "Inputs",
            WafFocusArea::ModeRadio => "Mode",
            WafFocusArea::Techniques => "Techniques",
            WafFocusArea::Results => "Results",
        };
        Some(vec!["WAF", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref err) = self.error {
            use ratatui::widgets::{Block, Borders, Paragraph};
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(Block::default().borders(Borders::ALL).title("WAF - Error"))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(19), Constraint::Min(0)])
            .split(area);

        let config_area = chunks.first().copied().unwrap_or(area);
        let results_area = chunks.get(1).copied().unwrap_or(area);

        use crate::tui::components::FormBuilder;
        let mut builder = FormBuilder::new(" WAF Configuration ").row_height(3);

        // Target URL
        if let Some(field) = self.inputs.fields.first() {
            builder = builder.add_input(field.clone());
        }

        // Mode
        let mut mode = self.mode_radio.clone();
        mode.focused = self.focus_area == WafFocusArea::ModeRadio;
        builder = builder.add_radio(mode);

        // Techniques
        for (i, cb) in self.technique_checkboxes.iter().enumerate() {
            let mut checkbox = cb.clone();
            checkbox.focused =
                self.focus_area == WafFocusArea::Techniques && i == self.focused_checkbox_index;
            builder = builder.add_checkbox(checkbox);
        }

        builder.render(f, config_area, insert_mode);

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(
                Style::default().fg(if self.focus_area == WafFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let results_inner = results_block.inner(results_area);
        f.render_widget(results_block, results_area);

        if self.state == AppState::Running {
            self.progress.render(f, results_inner);
        } else {
            let results_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(results_inner);

            if let Some(results_chunks_0) = results_chunks.first().copied() {
                if !self.detection_view.is_empty() {
                    self.detection_view.render(f, results_chunks_0, None);
                } else {
                    let placeholder =
                        empty_state_paragraph("Detection Result", "Detection results will appear here");
                    f.render_widget(placeholder, results_chunks_0);
                }
            }

            if let Some(results_chunks_1) = results_chunks.get(1).copied() {
                if !self.bypass_view.is_empty() {
                    self.bypass_view.render(f, results_chunks_1, None);
                } else {
                    let placeholder =
                        empty_state_paragraph("Bypass Results", "Bypass results will appear here");
                    f.render_widget(placeholder, results_chunks_1);
                }
            }
        }
    }
}

impl TabInput for WafTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            WafFocusArea::Inputs => {
                if self.inputs.is_focused() {
                    self.inputs.blur();
                }
                WafFocusArea::ModeRadio
            }
            WafFocusArea::ModeRadio => {
                self.focused_checkbox_index = 0;
                WafFocusArea::Techniques
            }
            WafFocusArea::Techniques => WafFocusArea::Results,
            WafFocusArea::Results => {
                self.inputs.focus(0);
                WafFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            WafFocusArea::Inputs => {
                self.inputs.blur();
                WafFocusArea::Results
            }
            WafFocusArea::ModeRadio => {
                self.inputs.focus(0);
                WafFocusArea::Inputs
            }
            WafFocusArea::Techniques => WafFocusArea::ModeRadio,
            WafFocusArea::Results => {
                self.focused_checkbox_index = 0;
                WafFocusArea::Techniques
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == WafFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == WafFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == WafFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        match self.focus_area {
            WafFocusArea::Inputs => self.inputs.get_focused_value(),
            WafFocusArea::Results => {
                let mut content = self.detection_view.get_content();
                content.push_str("\n\n");
                content.push_str(&self.bypass_view.get_content());
                Some(content)
            }
            _ => None,
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if self.focus_area == WafFocusArea::Inputs {
                self.inputs.move_word_forward();
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if self.focus_area == WafFocusArea::Inputs {
                self.inputs.move_word_backward();
            }
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == WafFocusArea::Inputs {
                self.inputs.move_home();
            } else if self.focus_area == WafFocusArea::Results {
                self.detection_view.scroll_to_top();
                self.bypass_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == WafFocusArea::Inputs {
                self.inputs.move_end();
            } else if self.focus_area == WafFocusArea::Results {
                self.detection_view.scroll_to_bottom();
                self.bypass_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = WafFocusArea::Inputs;
            self.inputs.focus(0);
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.inputs.blur();
            self.focus_area = WafFocusArea::Results;
        }
    }

    fn handle_enter(&mut self) {
        if self.focus_area == WafFocusArea::Results {
            return;
        }

        if self.is_running() {
            self.stop();
            return;
        }
        if self.focus_area == WafFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }

        if self.focus_area == WafFocusArea::ModeRadio {
            if let Some(sel) = self.mode_radio.selected {
                self.mode_radio.select((sel + 1) % 2);
            }
            return;
        }

        if self.focus_area == WafFocusArea::Techniques {
            if !self.technique_checkboxes.is_empty() {
                if let Some(cb) = self
                    .technique_checkboxes
                    .get_mut(self.focused_checkbox_index)
                {
                    cb.toggle();
                }
            }
            return;
        }

        if self.is_running() {
            self.stop();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.focus_area == WafFocusArea::Techniques {
            self.focused_checkbox_index = 0;
            self.focus_area = WafFocusArea::Inputs;
        }
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WafFocusArea::ModeRadio {
            let len = self.mode_radio.options.len();
            if len > 0 {
                let cur = self.mode_radio.selected.unwrap_or(0);
                let prev = if cur == 0 { len - 1 } else { cur - 1 };
                self.mode_radio.select(prev);
            }
        } else if self.focus_area == WafFocusArea::Techniques {
            if !self.technique_checkboxes.is_empty() && self.focused_checkbox_index > 0 {
                self.focused_checkbox_index -= 1;
            }
        } else if self.focus_area == WafFocusArea::Inputs {
            self.inputs.focus_prev();
        } else if self.focus_area == WafFocusArea::Results {
            self.scroll_detection_up();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WafFocusArea::ModeRadio {
            let len = self.mode_radio.options.len();
            if len > 0 {
                let cur = self.mode_radio.selected.unwrap_or(0);
                self.mode_radio.select((cur + 1) % len);
            }
        } else if self.focus_area == WafFocusArea::Techniques {
            if self.focused_checkbox_index < self.technique_checkboxes.len().saturating_sub(1) {
                self.focused_checkbox_index += 1;
            }
        } else if self.focus_area == WafFocusArea::Inputs {
            self.inputs.focus_next();
        } else if self.focus_area == WafFocusArea::Results {
            self.scroll_detection_down();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == WafFocusArea::Inputs {
            self.inputs.move_left()
        } else if self.focus_area == WafFocusArea::Techniques {
            if self.technique_checkboxes.is_empty() || self.focused_checkbox_index == 0 {
                false
            } else {
                self.focused_checkbox_index = self.focused_checkbox_index.saturating_sub(1);
                true
            }
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == WafFocusArea::Inputs {
            self.inputs.move_right()
        } else if self.focus_area == WafFocusArea::Techniques {
            if self.focused_checkbox_index >= self.technique_checkboxes.len().saturating_sub(1) {
                false
            } else {
                self.focused_checkbox_index += 1;
                true
            }
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == WafFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else if self.focus_area == WafFocusArea::Techniques {
            self.technique_checkboxes.is_empty() || self.focused_checkbox_index == 0
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == WafFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else if self.focus_area == WafFocusArea::Techniques {
            self.technique_checkboxes.is_empty()
                || self.focused_checkbox_index >= self.technique_checkboxes.len().saturating_sub(1)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == WafFocusArea::Inputs && self.inputs.is_focused()
    }

    fn page_up(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.detection_view.page_up(page_size);
        self.bypass_view.page_up(page_size);
    }

    fn page_down(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.detection_view.page_down(page_size);
        self.bypass_view.page_down(page_size);
    }
}
