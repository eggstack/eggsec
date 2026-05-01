use crate::tc;
use crate::tui::components::{
    Checkbox, InputField, InputGroup, ProgressGauge, RadioGroup, ScrollableText,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use crate::waf::{BypassResult, WafDetectionResult};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

    pub fn set_detection_result(&mut self, result: WafDetectionResult) {
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
                Span::styled(
                    format!("{:?}", technique),
                    Style::default().fg(tc!(accent)),
                ),
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

    pub fn page_up(&mut self, page_size: usize) {
        self.detection_view.page_up(page_size);
        self.bypass_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.detection_view.page_down(page_size);
        self.bypass_view.page_down(page_size);
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
        self.detection_view.clear();
        self.bypass_view.clear();
        for field in &mut self.inputs.fields {
            field.clear();
        }
        for cb in &mut self.technique_checkboxes {
            cb.checked = false;
        }
        self.technique_checkboxes[1].checked = true;
        self.technique_checkboxes[2].checked = true;
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(17), Constraint::Min(0)])
            .split(area);

        let config_area = chunks[0];
        let results_area = chunks[1];

        let config_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
            ])
            .split(config_area);

        self.inputs.fields[0].render(f, config_chunks[0], insert_mode);

        let mut mode = self.mode_radio.clone();
        mode.focused = self.focus_area == WafFocusArea::ModeRadio;
        mode.render(f, config_chunks[1]);

        for (i, cb) in self.technique_checkboxes.iter().enumerate() {
            let mut checkbox = cb.clone();
            checkbox.focused = self.focus_area == WafFocusArea::Techniques && i == 0;
            checkbox.render(f, config_chunks[2 + i]);
        }

        if self.state == AppState::Running {
            self.progress.render(f, results_area);
        } else {
            let results_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(results_area);

            if !self.detection_view.is_empty() {
                self.detection_view.render(f, results_chunks[0], None);
            } else {
                let placeholder = Paragraph::new("Detection results will appear here")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Detection Result"),
                    )
                    .style(Style::default().fg(tc!(text_dim)));
                f.render_widget(placeholder, results_chunks[0]);
            }

            if !self.bypass_view.is_empty() {
                self.bypass_view.render(f, results_chunks[1], None);
            } else {
                let placeholder = Paragraph::new("Bypass results will appear here")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Bypass Results"),
                    )
                    .style(Style::default().fg(tc!(text_dim)));
                f.render_widget(placeholder, results_chunks[1]);
            }
        }
    }
}

impl TabInput for WafTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            WafFocusArea::Inputs => {
                if self.inputs.is_focused() {
                    self.inputs.blur();
                }
                WafFocusArea::ModeRadio
            }
            WafFocusArea::ModeRadio => {
                self.technique_checkboxes
                    .iter_mut()
                    .for_each(|cb| cb.focused = false);
                self.technique_checkboxes[0].focused = true;
                WafFocusArea::Techniques
            }
            WafFocusArea::Techniques => {
                self.inputs.focus(0);
                WafFocusArea::Inputs
            }
            WafFocusArea::Results => WafFocusArea::Inputs,
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            WafFocusArea::Inputs => WafFocusArea::Techniques,
            WafFocusArea::ModeRadio => {
                self.inputs.focus(0);
                WafFocusArea::Inputs
            }
            WafFocusArea::Techniques => WafFocusArea::ModeRadio,
            WafFocusArea::Results => WafFocusArea::Inputs,
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

    fn handle_enter(&mut self) {
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
            for cb in &mut self.technique_checkboxes {
                if cb.focused {
                    cb.toggle();
                    break;
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
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if !self.inputs.is_focused() && !self.detection_view.is_empty() {
            self.scroll_detection_up();
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if !self.inputs.is_focused() && !self.detection_view.is_empty() {
            self.scroll_detection_down();
        } else {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.focus_area == WafFocusArea::Inputs {
            self.inputs.move_left()
        } else if self.focus_area == WafFocusArea::Techniques {
            let focused_idx = self.technique_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx == 0 {
                    return false;
                } else {
                    self.technique_checkboxes[idx].focused = false;
                    self.technique_checkboxes[idx - 1].focused = true;
                    return true;
                }
            }
            true
        } else {
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.focus_area == WafFocusArea::Inputs {
            self.inputs.move_right()
        } else if self.focus_area == WafFocusArea::Techniques {
            let focused_idx = self.technique_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx >= self.technique_checkboxes.len() - 1 {
                    return false;
                } else {
                    self.technique_checkboxes[idx].focused = false;
                    self.technique_checkboxes[idx + 1].focused = true;
                    return true;
                }
            }
            true
        } else {
            true
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == WafFocusArea::Inputs {
            let cursor_pos = self.inputs.fields[0].cursor_pos;
            cursor_pos == 0
        } else if self.focus_area == WafFocusArea::Techniques {
            let focused_idx = self.technique_checkboxes.iter().position(|cb| cb.focused);
            focused_idx == Some(0)
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == WafFocusArea::Inputs {
            let field = &self.inputs.fields[0];
            field.cursor_pos >= field.value.chars().count()
        } else if self.focus_area == WafFocusArea::Techniques {
            let focused_idx = self.technique_checkboxes.iter().position(|cb| cb.focused);
            focused_idx == Some(self.technique_checkboxes.len() - 1)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == WafFocusArea::Inputs && self.inputs.is_focused()
    }
}
