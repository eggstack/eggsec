use crate::scanner::ports::PortScanResults;
use crate::tui::components::{Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    Frame,
};

pub struct ScanPortsTab {
    pub inputs: InputGroup,
    pub results: Option<PortScanResults>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub udp_checkbox: Checkbox,
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
        }
    }

    pub fn get_results(&self) -> Option<&PortScanResults> {
        self.results.as_ref()
    }

    pub fn target(&self) -> &str {
        &self
            .inputs
            .fields
            .get(0)
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
        &self
            .inputs
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
        use ratatui::style::{Color, Style};
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
            Span::styled("Host: ", Style::default().fg(Color::Yellow)),
            Span::raw(host),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Ports scanned: ", Style::default().fg(Color::Cyan)),
            Span::raw(ports_scanned.to_string()),
            Span::raw(" | "),
            Span::styled("Open: ", Style::default().fg(Color::Green)),
            Span::raw(open_ports.len().to_string()),
        ]));

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<8}", "PORT"), Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:<15}", "SERVICE"),
                Style::default().fg(Color::Yellow),
            ),
        ]));

        for (port, service) in open_ports {
            self.results_view.add_line(Line::from(vec![
                Span::styled(format!("{:<8}", port), Style::default().fg(Color::Green)),
                Span::raw(format!("{:<15}", service)),
            ]));
        }
    }

    pub fn start(&mut self) {
        let target = self.target();
        if target.is_empty() {
            self.state = AppState::Error("Target cannot be empty".to_string());
            return;
        }

        for t in self.targets() {
            let old_value = std::mem::take(&mut self.inputs.fields[0].value);
            self.inputs.fields[0].value = t.clone();
            let validation = self.inputs.fields[0].validate_ip();
            self.inputs.fields[0].value = old_value;

            if !validation.valid && !t.contains('.') && !t.contains(':') {
                self.state =
                    AppState::Error(format!("Invalid target: {} - {}", t, validation.message));
                return;
            }
        }

        let port_validation = self.inputs.fields[1].validate_port_range();
        if !port_validation.valid {
            self.state =
                AppState::Error(format!("Invalid port range: {}", port_validation.message));
            return;
        }

        self.state = AppState::Running;
        self.progress.current = 0;
        self.results = None;
        self.results_view.clear();
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
        self.results_view.clear();
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.inputs.fields[1].value = "1-1024".to_string();
        self.inputs.fields[1].cursor_pos = 6;
        self.inputs.fields[2].value = "100".to_string();
        self.inputs.fields[2].cursor_pos = 3;
        self.inputs.fields[3].value = "2".to_string();
        self.inputs.fields[3].cursor_pos = 1;
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

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_area);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            field.render(f, input_chunks[i], insert_mode);
        }

        let udp_cb = self.udp_checkbox.clone();
        udp_cb.render(f, input_chunks[4]);

        if self.state == AppState::Running {
            self.progress.render(f, results_area);
        } else if self.results_view.len() > 0 {
            self.results_view
                .render_with_style(f, results_area, Color::Green);
        } else {
            use ratatui::style::Style;
            use ratatui::widgets::{Block, Borders, Paragraph};
            let placeholder = Paragraph::new("Results will appear here after running")
                .block(Block::default().borders(Borders::ALL).title("Results"))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for ScanPortsTab {
    fn handle_focus_next(&mut self) {
        self.inputs.focus_next();
    }

    fn handle_focus_prev(&mut self) {
        self.inputs.focus_prev();
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

    fn handle_enter(&mut self) {
        if self.inputs.is_focused() {
            self.inputs.blur();
        } else if self.is_running() {
            self.stop();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if !self.inputs.is_focused() && self.results_view.len() > 0 {
            self.scroll_results_up();
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if !self.inputs.is_focused() && self.results_view.len() > 0 {
            self.scroll_results_down();
        } else {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        self.inputs.move_left()
    }

    fn handle_right(&mut self) -> bool {
        self.inputs.move_right()
    }

    fn is_input_focused(&self) -> bool {
        self.inputs.is_focused()
    }
}
