use crate::tc;
use crate::tui::components::{Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct OAuthTab {
    pub inputs: InputGroup,
    pub redirect_test_checkbox: Checkbox,
    pub scope_test_checkbox: Checkbox,
    pub state_test_checkbox: Checkbox,
    pub grant_test_checkbox: Checkbox,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: OAuthFocusArea,
    pub checkbox_focus_index: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OAuthFocusArea {
    Inputs,
    Options,
    Results,
}

impl Default for OAuthTab {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("OAuth Authorization Endpoint URL"))
            .add(InputField::new("Client ID (optional)"))
            .add(InputField::new("Redirect URI (optional)"))
            .add(InputField::new("Concurrency").with_value("10"))
            .add(InputField::new("Timeout (s)").with_value("15"));

        let redirect_test_checkbox = Checkbox::new("Redirect URI Validation").checked(true);
        let scope_test_checkbox = Checkbox::new("Scope Escalation Tests").checked(true);
        let state_test_checkbox = Checkbox::new("State Parameter Tests").checked(true);
        let grant_test_checkbox = Checkbox::new("Grant Type Tests").checked(true);

        Self {
            inputs,
            redirect_test_checkbox,
            scope_test_checkbox,
            state_test_checkbox,
            grant_test_checkbox,
            progress: ProgressGauge::new("Testing OAuth..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: OAuthFocusArea::Inputs,
            checkbox_focus_index: 0,
            error_message: None,
        }
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn client_id(&self) -> Option<&str> {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn redirect_uri(&self) -> Option<&str> {
        self.inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn concurrency(&self) -> usize {
        self.inputs
            .fields
            .get(3)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn timeout(&self) -> u64 {
        self.inputs
            .fields
            .get(4)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(15)
    }

    pub fn set_results(&mut self, results: OAuthResults) {
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            format!("OAuth/OIDC Security Test Complete: {}", results.target),
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Findings:",
            Style::default().fg(tc!(warning)),
        )));

        if !results.redirect_vulnerabilities.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!(
                    "  [!] Redirect URI Issues: {}",
                    results.redirect_vulnerabilities.len()
                ),
                Style::default().fg(tc!(error)),
            )));
            for vuln in &results.redirect_vulnerabilities {
                self.results_view
                    .add_line(Line::from(format!("    - {}", vuln)));
            }
        } else {
            self.results_view.add_line(Line::from(Span::raw(
                "  [+] Redirect URI validation appears secure",
            )));
        }

        if !results.scope_vulnerabilities.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!(
                    "  [!] Scope Escalation Issues: {}",
                    results.scope_vulnerabilities.len()
                ),
                Style::default().fg(tc!(error)),
            )));
        }

        if !results.state_vulnerabilities.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!(
                    "  [!] State Parameter Issues: {}",
                    results.state_vulnerabilities.len()
                ),
                Style::default().fg(tc!(error)),
            )));
        }

        if !results.grant_vulnerabilities.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                format!(
                    "  [!] Grant Type Issues: {}",
                    results.grant_vulnerabilities.len()
                ),
                Style::default().fg(tc!(error)),
            )));
        }

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "Requests: {} | Errors: {} | Duration: {}ms",
            results.total_requests, results.errors, results.duration_ms
        )));
    }
}

#[derive(Clone, Debug)]
pub struct OAuthResults {
    pub target: String,
    pub redirect_vulnerabilities: Vec<String>,
    pub scope_vulnerabilities: Vec<String>,
    pub state_vulnerabilities: Vec<String>,
    pub grant_vulnerabilities: Vec<String>,
    pub total_requests: usize,
    pub errors: usize,
    pub duration_ms: u64,
}

impl TabState for OAuthTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results_view.clear();
        self.progress.current = 0;
        self.error_message = None;
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", msg),
            Style::default().fg(tc!(error)),
        )));
    }
}

impl OAuthTab {
    pub fn stop(&mut self) {
        if self.state == AppState::Running {
            self.state = AppState::Idle;
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.scroll_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.scroll_down(page_size);
    }

    pub fn handle_word_forward(&mut self) {
        for _ in 0..5 {
            self.handle_right();
        }
    }

    pub fn handle_word_backward(&mut self) {
        for _ in 0..5 {
            self.handle_left();
        }
    }

    pub fn handle_home(&mut self) {
        for _ in 0..100 {
            self.handle_left();
        }
    }

    pub fn handle_end(&mut self) {
        for _ in 0..100 {
            self.handle_right();
        }
    }

    pub fn handle_top(&mut self) {
        for _ in 0..100 {
            self.results_view.scroll_up(1);
        }
    }

    pub fn handle_bottom(&mut self) {
        for _ in 0..100 {
            self.results_view.scroll_down(1);
        }
    }
}

impl TabRender for OAuthTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        // Dynamic layout based on terminal height
        let (input_height, options_height, results_min) = if area.height < 30 {
            // Small terminal: use percentages
            let ih = ((area.height as f32 * 0.6) as u16).max(8).min(16);
            let oh = ((area.height as f32 * 0.2) as u16).max(4).min(6);
            (ih, oh, 2)
        } else {
            (16, 6, 5)
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(input_height),
                Constraint::Length(options_height),
                Constraint::Min(results_min),
            ])
            .split(area);

        // Input fields - dynamic height based on available space
        let num_inputs = 5;
        let field_height = (chunks[0].height / num_inputs as u16).max(2);
        let constraints: Vec<Constraint> = (0..num_inputs)
            .map(|_| Constraint::Length(field_height))
            .collect();
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(chunks[0]);

        let input_block = Block::default()
            .title(" OAuth/OIDC Configuration ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == OAuthFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        f.render_widget(input_block, chunks[0]);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if i < input_chunks.len() {
                field.render(f, input_chunks[i], insert_mode);
            }
        }

        // Options
        let options_block = Block::default()
            .title(" Test Options ")
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if self.focus_area == OAuthFocusArea::Options {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );

        let options_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(options_block.inner(chunks[1]));

        f.render_widget(options_block, chunks[1]);
        self.redirect_test_checkbox.render(f, options_chunks[0]);
        self.scope_test_checkbox.render(f, options_chunks[1]);
        self.state_test_checkbox.render(f, options_chunks[2]);
        self.grant_test_checkbox.render(f, options_chunks[3]);

        // Results
        self.results_view.render(f, chunks[2], None);

        // Progress bar if running
        if self.state == AppState::Running {
            let progress_area = Rect {
                x: area.x,
                y: area.y + area.height - 1,
                width: area.width,
                height: 1,
            };
            self.progress.render(f, progress_area);
        }
    }
}

impl TabInput for OAuthTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            OAuthFocusArea::Inputs => {
                self.inputs.blur();
                OAuthFocusArea::Options
            }
            OAuthFocusArea::Options => OAuthFocusArea::Results,
            OAuthFocusArea::Results => {
                self.inputs.focus(0);
                OAuthFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            OAuthFocusArea::Inputs => {
                self.inputs.blur();
                OAuthFocusArea::Results
            }
            OAuthFocusArea::Options => {
                self.inputs.focus(0);
                OAuthFocusArea::Inputs
            }
            OAuthFocusArea::Results => OAuthFocusArea::Options,
        };
    }

    fn handle_char(&mut self, c: char) {
        if self.focus_area == OAuthFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.focus_area == OAuthFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_enter(&mut self) {
        match self.focus_area {
            OAuthFocusArea::Inputs => {
                self.inputs.blur();
            }
            OAuthFocusArea::Options => {
                let checkboxes = [
                    &mut self.redirect_test_checkbox,
                    &mut self.scope_test_checkbox,
                    &mut self.state_test_checkbox,
                    &mut self.grant_test_checkbox,
                ];
                let idx = self.checkbox_focus_index % checkboxes.len();
                checkboxes[idx].toggle();
            }
            OAuthFocusArea::Results => {}
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            OAuthFocusArea::Inputs => {
                self.inputs.focus_prev();
            }
            OAuthFocusArea::Results => {
                self.results_view.scroll_up(1);
            }
            _ => {}
        }
    }

    fn handle_down(&mut self) {
        match self.focus_area {
            OAuthFocusArea::Inputs => {
                self.inputs.focus_next();
            }
            OAuthFocusArea::Results => {
                self.results_view.scroll_down(1);
            }
            _ => {}
        }
    }

    fn handle_left(&mut self) -> bool {
        match self.focus_area {
            OAuthFocusArea::Inputs => self.inputs.move_left(),
            OAuthFocusArea::Options => {
                if self.checkbox_focus_index > 0 {
                    self.checkbox_focus_index -= 1;
                }
                true
            }
            _ => false,
        }
    }

    fn handle_right(&mut self) -> bool {
        match self.focus_area {
            OAuthFocusArea::Inputs => self.inputs.move_right(),
            OAuthFocusArea::Options => {
                self.checkbox_focus_index = (self.checkbox_focus_index + 1).min(3);
                true
            }
            _ => false,
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == OAuthFocusArea::Inputs && self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            OAuthFocusArea::Inputs => !self.inputs.can_move_left(),
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            OAuthFocusArea::Inputs => !self.inputs.can_move_right(),
            _ => true,
        }
    }
}
