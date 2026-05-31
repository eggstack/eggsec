use crate::scanner::ports::PortScanResults;
use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::ValidationResult;
use crate::tui::components::{
    empty_state_paragraph, Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanPortsFocusArea {
    Inputs,
    Options,
    Results,
}

pub struct ScanPortsTab {
    pub inputs: InputGroup,
    pub results: Option<PortScanResults>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub udp_checkbox: Checkbox,
    pub focus_area: ScanPortsFocusArea,
    pub error: Option<TabError>,
}

impl ScanPortsTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target Host"))
            .add(InputField::new("Ports (e.g., 1-1024 or 22,80,443)").with_value("1-1024"))
            .add(InputField::new("Concurrency").with_value("100"))
            .add(InputField::new("Timeout (s)").with_value("2"));

        Self {
            inputs,
            results: None,
            progress: ProgressGauge::new("Scanning ports..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            udp_checkbox: Checkbox::new("Enable UDP (requires root/sudo)").checked(false),
            focus_area: ScanPortsFocusArea::Inputs,
            error: None,
        }
    }

    pub fn get_results(&self) -> Option<&PortScanResults> {
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

    pub fn ports(&self) -> &str {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("1-1024")
    }

    pub fn concurrency(&self) -> usize {
        self.inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(100)
    }

    pub fn timeout(&self) -> u64 {
        self.inputs
            .fields
            .get(3)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(2)
    }

    pub fn udp(&self) -> bool {
        self.udp_checkbox.checked
    }

    pub fn set_results(&mut self, results: PortScanResults) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &PortScanResults) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};

        self.results_view.clear();

        let host = results.host.clone();
        let ports_scanned = results.ports_scanned;
        let open_ports: Vec<_> = results
            .open_ports
            .iter()
            .map(|p| (p.port, p.service.clone()))
            .collect();

        self.results_view.add_line(Line::from(vec![
            Span::styled("Host: ", Style::default().fg(tc!(warning))),
            Span::raw(host),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Ports scanned: ", Style::default().fg(tc!(info))),
            Span::raw(ports_scanned.to_string()),
            Span::raw(" | "),
            Span::styled("Open: ", Style::default().fg(tc!(success))),
            Span::raw(open_ports.len().to_string()),
        ]));

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<8}", "PORT"), Style::default().fg(tc!(accent))),
            Span::styled(
                format!("{:<15}", "SERVICE"),
                Style::default().fg(tc!(accent)),
            ),
        ]));

        for (port, service) in open_ports {
            self.results_view.add_line(Line::from(vec![
                Span::styled(format!("{:<8}", port), Style::default().fg(tc!(success))),
                Span::raw(format!("{:<15}", service)),
            ]));
        }
    }

    pub fn start(&mut self) {
        let target = self.target();
        if target.is_empty() {
            self.state = AppState::Error("Target cannot be empty".to_string());
            self.error = Some(TabError::Target("Target cannot be empty".to_string()));
            return;
        }

        if self.inputs.fields.len() < 2 {
            self.state = AppState::Error("Input fields not initialized".to_string());
            self.error = Some(TabError::Config("Input fields not initialized".to_string()));
            return;
        }

        if let Some(port_field) = self.inputs.fields.get(1) {
            let port_value = port_field.value.clone();
            for t in self.targets() {
                if let Some(target_field) = self.inputs.fields.get_mut(0) {
                    let old_target = std::mem::take(&mut target_field.value);
                    target_field.value = t.clone();
                    let target_validation = target_field.validate_ip();
                    target_field.value = old_target;

                    if !target_validation.valid && !t.contains('.') && !t.contains(':') {
                        self.state = AppState::Error(format!(
                            "Invalid target: {} - {}",
                            t, target_validation.message
                        ));
                        self.error = Some(TabError::Target(format!(
                            "Invalid target: {} - {}",
                            t, target_validation.message
                        )));
                        return;
                    }
                }

                if let Some(port_field) = self.inputs.fields.get_mut(1) {
                    let old_port = std::mem::take(&mut port_field.value);
                    port_field.value = port_value.clone();
                    let port_validation = port_field.validate_port_range();
                    port_field.value = old_port;

                    if !port_validation.valid {
                        self.state = AppState::Error(format!(
                            "Invalid port range: {}",
                            port_validation.message
                        ));
                        self.error = Some(TabError::Config(format!(
                            "Invalid port range: {}",
                            port_validation.message
                        )));
                        return;
                    }
                }
            }
        }

        self.state = AppState::Running;
        self.progress.current = 0;
        self.results = None;
        self.results_view.clear();
        self.error = None;
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

    fn update_field_validation(&mut self) {
        if let Some(ref mut target_field) = self.inputs.fields.get_mut(0) {
            let validation = if target_field.value.contains('.') || target_field.value.contains(':')
            {
                target_field.validate_ip()
            } else {
                ValidationResult {
                    valid: !target_field.value.is_empty(),
                    message: if target_field.value.is_empty() {
                        "Target cannot be empty".to_string()
                    } else {
                        String::new()
                    },
                }
            };
            target_field.validation = Some(validation);
        }
        if let Some(ref mut port_field) = self.inputs.fields.get_mut(1) {
            port_field.validation = Some(port_field.validate_port_range());
        }
    }
}

impl Default for ScanPortsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ScanPortsTab {
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
        self.progress.total = 0;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        if let Some(field) = self.inputs.fields.get_mut(1) {
            field.value = "1-1024".to_string();
            field.cursor_pos = 6;
        }
        if let Some(field) = self.inputs.fields.get_mut(2) {
            field.value = "100".to_string();
            field.cursor_pos = 3;
        }
        if let Some(field) = self.inputs.fields.get_mut(3) {
            field.value = "2".to_string();
            field.cursor_pos = 1;
        }
        self.focus_area = ScanPortsFocusArea::Inputs;
        self.udp_checkbox.checked = false;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for ScanPortsTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(15), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Port Scan Configuration ")
            .border_style(
                Style::default().fg(if self.focus_area == ScanPortsFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
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

        let udp_cb = self.udp_checkbox.clone();
        if let Some(chunk) = input_chunks.get(4) {
            udp_cb.render(f, *chunk);
        }

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(
                Style::default().fg(if self.focus_area == ScanPortsFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
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

impl TabInput for ScanPortsTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            ScanPortsFocusArea::Inputs => {
                self.inputs.blur();
                self.focus_area = ScanPortsFocusArea::Options;
            }
            ScanPortsFocusArea::Options => {
                self.focus_area = ScanPortsFocusArea::Results;
            }
            ScanPortsFocusArea::Results => {
                self.focus_area = ScanPortsFocusArea::Inputs;
                self.inputs.focus(0);
            }
        }
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            ScanPortsFocusArea::Inputs => {
                self.inputs.blur();
                self.focus_area = ScanPortsFocusArea::Results;
            }
            ScanPortsFocusArea::Options => {
                self.inputs.focus(0);
                self.focus_area = ScanPortsFocusArea::Inputs;
            }
            ScanPortsFocusArea::Results => {
                self.focus_area = ScanPortsFocusArea::Options;
            }
        }
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanPortsFocusArea::Inputs {
            if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_up();
            } else {
                self.inputs.focus_prev();
            }
        } else if self.focus_area == ScanPortsFocusArea::Options {
            return;
        } else if self.focus_area == ScanPortsFocusArea::Results {
            self.scroll_results_up();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanPortsFocusArea::Inputs {
            if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_down();
            } else {
                self.inputs.focus_next();
            }
        } else if self.focus_area == ScanPortsFocusArea::Options {
            return;
        } else if self.focus_area == ScanPortsFocusArea::Results {
            self.scroll_results_down();
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == ScanPortsFocusArea::Inputs {
            self.inputs.insert(c);
            self.update_field_validation();
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == ScanPortsFocusArea::Inputs {
            self.inputs.backspace();
            self.update_field_validation();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == ScanPortsFocusArea::Inputs {
            self.inputs.paste(text);
            self.update_field_validation();
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if !self.is_running() {
            if self.focus_area == ScanPortsFocusArea::Inputs {
                self.inputs.get_focused_value()
            } else if self.focus_area == ScanPortsFocusArea::Results {
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
            if self.focus_area == ScanPortsFocusArea::Inputs {
                self.inputs.move_word_forward();
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if self.focus_area == ScanPortsFocusArea::Inputs {
                self.inputs.move_word_backward();
            }
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanPortsFocusArea::Inputs {
            self.inputs.move_home();
        } else if self.focus_area == ScanPortsFocusArea::Results {
            self.results_view.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanPortsFocusArea::Inputs {
            self.inputs.move_end();
        } else if self.focus_area == ScanPortsFocusArea::Results {
            self.results_view.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.inputs.blur();
        self.focus_area = ScanPortsFocusArea::Inputs;
        self.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.inputs.blur();
        self.focus_area = ScanPortsFocusArea::Results;
    }

    fn handle_enter(&mut self) {
        if self.focus_area == ScanPortsFocusArea::Results {
            return;
        }

        if !self.is_running() && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }
        if !self.is_running() && self.focus_area == ScanPortsFocusArea::Options {
            self.udp_checkbox.checked = !self.udp_checkbox.checked;
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
        self.inputs.blur();
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == ScanPortsFocusArea::Inputs {
            self.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == ScanPortsFocusArea::Inputs {
            self.inputs.move_right()
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ScanPortsFocusArea::Inputs && self.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ScanPortsFocusArea::Inputs {
            self.inputs.fields.is_empty() || self.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ScanPortsFocusArea::Inputs {
            self.inputs.fields.is_empty() || self.inputs.is_at_right_edge()
        } else {
            true
        }
    }

    fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
    }
}
