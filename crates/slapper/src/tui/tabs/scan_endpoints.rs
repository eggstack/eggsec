use crate::scanner::endpoints::EndpointScanResults;
use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanEndpointsFocusArea {
    Inputs,
    Options,
    Results,
}

pub struct ScanEndpointsTab {
    pub inputs: InputGroup,
    pub results: Option<EndpointScanResults>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub include_404_checkbox: Checkbox,
    pub focus_area: ScanEndpointsFocusArea,
    pub error: Option<TabError>,
}

impl ScanEndpointsTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Concurrency").with_value("20"))
            .add(InputField::new("Timeout (s)").with_value("10"))
            .add(InputField::new("Wordlist (default: common)"));

        Self {
            inputs,
            results: None,
            progress: ProgressGauge::new("Scanning endpoints..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            include_404_checkbox: Checkbox::new("Check for 404s").checked(true),
            focus_area: ScanEndpointsFocusArea::Inputs,
            error: None,
        }
    }

    pub fn get_results(&self) -> Option<&EndpointScanResults> {
        self.results.as_ref()
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

    pub fn concurrency(&self) -> usize {
        self.inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(20)
    }

    pub fn timeout(&self) -> u64 {
        self.inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(10)
    }

    pub fn wordlist(&self) -> Option<&str> {
        let w = self
            .inputs
            .fields
            .get(3)
            .map(|f| f.value.as_str())
            .unwrap_or("");
        if w.is_empty() {
            None
        } else {
            Some(w)
        }
    }

    pub fn include_404(&self) -> bool {
        self.include_404_checkbox.checked
    }

    pub fn set_results(&mut self, results: EndpointScanResults) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &EndpointScanResults) {
        use ratatui::style::Modifier;

        self.results_view.clear();

        let base_url = results.base_url.clone();
        let endpoints_found = results.endpoints_found;
        let interesting_findings = results.interesting_findings;

        let endpoint_data: Vec<_> = results
            .results
            .iter()
            .map(|e| {
                (
                    e.path.clone(),
                    e.status_code,
                    e.content_length,
                    e.interesting,
                )
            })
            .collect();

        self.results_view.add_line(Line::from(vec![
            Span::styled("URL: ", Style::default().fg(tc!(accent))),
            Span::raw(base_url),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Found: ", Style::default().fg(tc!(info))),
            Span::raw(endpoints_found.to_string()),
            Span::raw(" | "),
            Span::styled("Interesting: ", Style::default().fg(tc!(error))),
            Span::raw(interesting_findings.to_string()),
        ]));

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<40}", "PATH"), Style::default().fg(tc!(accent))),
            Span::styled(format!("{:<8}", "STATUS"), Style::default().fg(tc!(accent))),
            Span::styled(format!("{:<10}", "SIZE"), Style::default().fg(tc!(accent))),
        ]));

        for (path, status_code, content_length, interesting) in endpoint_data {
            let style = if interesting {
                Style::default().fg(tc!(error)).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_color = match status_code {
                200..=299 => tc!(success),
                300..=399 => tc!(secondary),
                400..=499 => tc!(warning),
                _ => tc!(error),
            };

            let path_display = if path.len() > 38 {
                format!("{}...", &path[..35])
            } else {
                path
            };

            self.results_view.add_line(Line::from(vec![
                Span::styled(format!("{:<40}", path_display), style),
                Span::styled(
                    format!("{:<8}", status_code),
                    Style::default().fg(status_color),
                ),
                Span::raw(format!("{:<10}", content_length.unwrap_or(0))),
            ]));
        }
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.progress.current = 0;
            self.results = None;
            self.results_view.clear();
            self.error = None;
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

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

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
    }
}

impl Default for ScanEndpointsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ScanEndpointsTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results = None;
        self.progress.current = 0;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        if let Some(field) = self.inputs.fields.get_mut(1) {
            field.value = "20".to_string();
            field.cursor_pos = 2;
        }
        if let Some(field) = self.inputs.fields.get_mut(2) {
            field.value = "10".to_string();
            field.cursor_pos = 2;
        }
        self.focus_area = ScanEndpointsFocusArea::Inputs;
        self.include_404_checkbox.checked = true;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for ScanEndpointsTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(15), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Endpoint Scan Configuration ")
            .border_style(Style::default().fg(
                if self.focus_area == ScanEndpointsFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        let input_inner = input_block.inner(input_area);
        f.render_widget(input_block, input_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_inner);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        let include_404 = self.include_404_checkbox.clone();
        if let Some(chunk) = input_chunks.get(4) {
            include_404.render(f, *chunk);
        }

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(Style::default().fg(
                if self.focus_area == ScanEndpointsFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        let results_inner = results_block.inner(results_area);
        f.render_widget(results_block, results_area);

        if self.state == AppState::Running {
            self.progress.render(f, results_inner);
        } else if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_inner);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_inner, Some(tc!(success)));
        } else {
            let placeholder =
                empty_state_paragraph("Results", "Results will appear here after running");
            f.render_widget(placeholder, results_inner);
        }
    }
}

impl TabInput for ScanEndpointsTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            ScanEndpointsFocusArea::Inputs => {
                self.inputs.blur();
                self.focus_area = ScanEndpointsFocusArea::Options;
            }
            ScanEndpointsFocusArea::Options => {
                self.focus_area = ScanEndpointsFocusArea::Results;
            }
            ScanEndpointsFocusArea::Results => {
                self.focus_area = ScanEndpointsFocusArea::Inputs;
                self.inputs.focus(0);
            }
        }
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            ScanEndpointsFocusArea::Inputs => {
                self.focus_area = ScanEndpointsFocusArea::Results;
            }
            ScanEndpointsFocusArea::Options => {
                self.inputs.focus(0);
                self.focus_area = ScanEndpointsFocusArea::Inputs;
            }
            ScanEndpointsFocusArea::Results => {
                self.focus_area = ScanEndpointsFocusArea::Options;
            }
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if !self.is_running() {
            if self.focus_area == ScanEndpointsFocusArea::Inputs {
                self.inputs.get_focused_value()
            } else if self.focus_area == ScanEndpointsFocusArea::Results {
                Some(self.results_view.get_content())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if self.focus_area == ScanEndpointsFocusArea::Inputs {
                self.inputs.move_word_forward();
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if self.focus_area == ScanEndpointsFocusArea::Inputs {
                self.inputs.move_word_backward();
            }
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanEndpointsFocusArea::Inputs {
            self.inputs.move_home();
        } else if self.focus_area == ScanEndpointsFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanEndpointsFocusArea::Inputs {
            self.inputs.move_end();
        } else if self.focus_area == ScanEndpointsFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = ScanEndpointsFocusArea::Inputs;
        self.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = ScanEndpointsFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }
        if !self.is_running() && self.focus_area == ScanEndpointsFocusArea::Options {
            self.include_404_checkbox.checked = !self.include_404_checkbox.checked;
            return;
        }
        self.start();
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanEndpointsFocusArea::Inputs {
            if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_up();
            } else {
                self.inputs.focus_prev();
            }
        } else if self.focus_area == ScanEndpointsFocusArea::Options {
            return;
        } else if self.focus_area == ScanEndpointsFocusArea::Results {
            self.scroll_results_up();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanEndpointsFocusArea::Inputs {
            if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_down();
            } else {
                self.inputs.focus_next();
            }
        } else if self.focus_area == ScanEndpointsFocusArea::Options {
            return;
        } else if self.focus_area == ScanEndpointsFocusArea::Results {
            self.scroll_results_down();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        self.inputs.move_left()
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        self.inputs.move_right()
    }

    fn is_input_focused(&self) -> bool {
        self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ScanEndpointsFocusArea::Inputs {
            self.inputs.fields.is_empty() || self.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ScanEndpointsFocusArea::Inputs {
            self.inputs.fields.is_empty() || self.inputs.is_at_right_edge()
        } else {
            true
        }
    }
}
