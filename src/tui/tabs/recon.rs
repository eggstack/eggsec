use crate::recon::FullReconResult;
use crate::tui::components::{Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    text::{Line, Span},
    Frame,
};

pub struct ReconTab {
    pub inputs: InputGroup,
    pub results: Option<FullReconResult>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub options: ReconOptions,
    pub option_checkboxes: Vec<Checkbox>,
    pub focus_area: ReconFocusArea,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReconFocusArea {
    Inputs,
    Options,
    Results,
}

#[derive(Debug, Clone, Default)]
pub struct ReconOptions {
    pub no_tech: bool,
    pub no_dns: bool,
    pub no_geo: bool,
    pub no_whois: bool,
    pub no_subdomains: bool,
    pub no_ssl: bool,
    pub no_dns_records: bool,
    pub no_js: bool,
    pub no_content: bool,
    pub no_cloud: bool,
    pub no_wayback: bool,
    pub no_cors: bool,
    pub no_threat: bool,
    pub no_cve: bool,
    pub no_email: bool,
}

impl ReconTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target (domain or URL)"))
            .add(InputField::new("Concurrency").with_value("20"));

        let option_checkboxes = vec![
            Checkbox::new("Skip Tech Detection").checked(false),
            Checkbox::new("Skip DNS Lookup").checked(false),
            Checkbox::new("Skip Geolocation").checked(false),
            Checkbox::new("Skip WHOIS").checked(false),
            Checkbox::new("Skip Subdomains").checked(false),
            Checkbox::new("Skip SSL/TLS").checked(false),
            Checkbox::new("Skip DNS Records").checked(false),
            Checkbox::new("Skip JS Analysis").checked(false),
            Checkbox::new("Skip Content Discovery").checked(false),
            Checkbox::new("Skip Cloud Assets").checked(false),
            Checkbox::new("Skip Wayback").checked(false),
            Checkbox::new("Skip CORS").checked(false),
            Checkbox::new("Skip Threat Intel").checked(false),
            Checkbox::new("Skip CVE Mapping").checked(false),
            Checkbox::new("Skip Email Discovery").checked(false),
        ];

        Self {
            inputs,
            results: None,
            progress: ProgressGauge::new("Running reconnaissance..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            options: ReconOptions::default(),
            option_checkboxes,
            focus_area: ReconFocusArea::Inputs,
            error_message: None,
        }
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .get(0)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn concurrency(&self) -> usize {
        self.inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(20)
    }

    pub fn get_options(&self) -> ReconOptions {
        ReconOptions {
            no_tech: self.option_checkboxes[0].checked,
            no_dns: self.option_checkboxes[1].checked,
            no_geo: self.option_checkboxes[2].checked,
            no_whois: self.option_checkboxes[3].checked,
            no_subdomains: self.option_checkboxes[4].checked,
            no_ssl: self.option_checkboxes[5].checked,
            no_dns_records: self.option_checkboxes[6].checked,
            no_js: self.option_checkboxes[7].checked,
            no_content: self.option_checkboxes[8].checked,
            no_cloud: self.option_checkboxes[9].checked,
            no_wayback: self.option_checkboxes[10].checked,
            no_cors: self.option_checkboxes[11].checked,
            no_threat: self.option_checkboxes[12].checked,
            no_cve: self.option_checkboxes[13].checked,
            no_email: self.option_checkboxes[14].checked,
        }
    }

    pub fn get_results(&self) -> Option<&FullReconResult> {
        self.results.as_ref()
    }

    pub fn set_results(&mut self, results: FullReconResult) {
        self.results = Some(results.clone());
        self.state = AppState::Completed;
        self.results_view.clear();

        self.results_view.add_line(Line::from(Span::styled(
            format!("Reconnaissance Complete: {}", results.target),
            ratatui::style::Style::default().fg(Color::Green),
        )));
        self.results_view.add_line(Line::from(""));

        if let Some(ref domain) = results.domain {
            self.results_view
                .add_line(Line::from(format!("Domain: {}", domain)));
        }
        if let Some(ref ip) = results.ip_address {
            self.results_view
                .add_line(Line::from(format!("IP Address: {}", ip)));
        }

        if let Some(ref tech) = results.tech_stack {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "Tech Stack:",
                ratatui::style::Style::default().fg(Color::Yellow),
            )));
            if !tech.frameworks.is_empty() {
                self.results_view.add_line(Line::from(format!(
                    "  Frameworks: {}",
                    tech.frameworks.join(", ")
                )));
            }
            if !tech.servers.is_empty() {
                self.results_view.add_line(Line::from(format!(
                    "  Servers: {}",
                    tech.servers.join(", ")
                )));
            }
            if !tech.languages.is_empty() {
                self.results_view.add_line(Line::from(format!(
                    "  Languages: {}",
                    tech.languages.join(", ")
                )));
            }
        }

        if let Some(ref geo) = results.geolocation {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "Geolocation:",
                ratatui::style::Style::default().fg(Color::Yellow),
            )));
            if let Some(ref country) = geo.country {
                self.results_view
                    .add_line(Line::from(format!("  Country: {}", country)));
            }
            if let Some(ref city) = geo.city {
                self.results_view
                    .add_line(Line::from(format!("  City: {}", city)));
            }
            if let Some(ref isp) = geo.isp {
                self.results_view
                    .add_line(Line::from(format!("  ISP: {}", isp)));
            }
        }

        if let Some(ref geo_err) = results.geoip_error {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "GeoIP Error:",
                ratatui::style::Style::default().fg(Color::Red),
            )));
            for line in geo_err.lines().take(4) {
                self.results_view
                    .add_line(Line::from(format!("  {}", line)));
            }
        }

        if let Some(ref ssl) = results.ssl_analysis {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "SSL/TLS:",
                ratatui::style::Style::default().fg(Color::Yellow),
            )));
            if let Some(ref cert) = ssl.certificate {
                self.results_view
                    .add_line(Line::from(format!("  Subject: {}", cert.subject)));
                self.results_view
                    .add_line(Line::from(format!("  Issuer: {}", cert.issuer)));
            }
        }

        if let Some(ref subdomains) = results.subdomains {
            if !subdomains.subdomains.is_empty() {
                self.results_view.add_line(Line::from(""));
                self.results_view.add_line(Line::from(Span::styled(
                    format!("Subdomains ({}):", subdomains.subdomains.len()),
                    ratatui::style::Style::default().fg(Color::Yellow),
                )));
                for sub in subdomains.subdomains.iter().take(5) {
                    self.results_view.add_line(Line::from(format!(
                        "  - {} ({})",
                        sub.subdomain,
                        sub.ip_addresses.join(", ")
                    )));
                }
                if subdomains.subdomains.len() > 5 {
                    self.results_view.add_line(Line::from(format!(
                        "  ... and {} more",
                        subdomains.subdomains.len() - 5
                    )));
                }
            }
        }
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.progress.current = 0;
            self.results = None;
            self.results_view.clear();
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

impl Default for ReconTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ReconTab {
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
        self.error_message = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg);
        self.progress.current = 0;
    }
}

impl TabRender for ReconTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            ReconFocusArea::Inputs => "Inputs",
            ReconFocusArea::Options => "Options",
            ReconFocusArea::Results => "Results",
        };
        Some(vec!["Recon", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(16), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(10),
            ])
            .split(input_area);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            field.render(f, input_chunks[i], insert_mode);
        }

        let options_area = input_chunks[2];
        let option_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(options_area);

        let left_options = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(2); 8])
            .split(option_chunks[0]);

        let right_options = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(2); 8])
            .split(option_chunks[1]);

        for (i, cb) in self.option_checkboxes.iter().enumerate().take(8) {
            let mut checkbox = cb.clone();
            checkbox.focused = self.focus_area == ReconFocusArea::Options;
            checkbox.render(f, left_options[i]);
        }

        for (i, cb) in self.option_checkboxes.iter().enumerate().skip(8) {
            let mut checkbox = cb.clone();
            checkbox.focused = self.focus_area == ReconFocusArea::Options;
            checkbox.render(f, right_options[i - 8]);
        }

        if self.state == AppState::Running {
            self.progress.render(f, results_area);
        } else if let Some(ref err_msg) = self.error_message {
            use ratatui::style::Style;
            use ratatui::widgets::{Block, Borders, Paragraph};
            let error_text = Paragraph::new(format!("Error: {}", err_msg))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Reconnaissance - Error"),
                )
                .style(Style::default().fg(Color::Red));
            f.render_widget(error_text, results_area);
        } else if self.results_view.len() > 0 {
            self.results_view
                .render_with_style(f, results_area, Color::Green);
        } else {
            use ratatui::style::Style;
            use ratatui::widgets::{Block, Borders, Paragraph};
            let cli_example = "slapper recon example.com --no-tech --no-whois";
            let placeholder = Paragraph::new(format!(
                "Enter target and press Enter to start recon\n\nCLI equivalent: {}",
                cli_example
            ))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Reconnaissance"),
            )
            .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for ReconTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            ReconFocusArea::Inputs => {
                self.inputs.blur();
                self.option_checkboxes
                    .iter_mut()
                    .for_each(|cb| cb.focused = false);
                self.option_checkboxes[0].focused = true;
                ReconFocusArea::Options
            }
            ReconFocusArea::Options => ReconFocusArea::Results,
            ReconFocusArea::Results => {
                self.inputs.focus(0);
                ReconFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            ReconFocusArea::Inputs => ReconFocusArea::Results,
            ReconFocusArea::Options => {
                self.inputs.focus(0);
                ReconFocusArea::Inputs
            }
            ReconFocusArea::Results => {
                self.option_checkboxes
                    .iter_mut()
                    .for_each(|cb| cb.focused = false);
                self.option_checkboxes[0].focused = true;
                ReconFocusArea::Options
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == ReconFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == ReconFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_enter(&mut self) {
        if self.focus_area == ReconFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }

        if self.focus_area == ReconFocusArea::Options {
            for cb in &mut self.option_checkboxes {
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
        if self.focus_area == ReconFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx == 0 {
                    self.option_checkboxes.last_mut().unwrap().focused = true;
                } else {
                    self.option_checkboxes[idx - 1].focused = true;
                }
                self.option_checkboxes[idx].focused = false;
            } else {
                self.option_checkboxes[0].focused = true;
            }
        } else if !self.inputs.is_focused() && self.results_view.len() > 0 {
            self.scroll_results_up();
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.focus_area == ReconFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx == self.option_checkboxes.len() - 1 {
                    self.option_checkboxes[0].focused = true;
                } else {
                    self.option_checkboxes[idx + 1].focused = true;
                }
                self.option_checkboxes[idx].focused = false;
            } else {
                self.option_checkboxes[0].focused = true;
            }
        } else if !self.inputs.is_focused() && self.results_view.len() > 0 {
            self.scroll_results_down();
        } else {
            self.inputs.focus_next();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.focus_area == ReconFocusArea::Inputs {
            self.inputs.move_left()
        } else if self.focus_area == ReconFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx == 0 {
                    return false;
                } else {
                    self.option_checkboxes[idx].focused = false;
                    self.option_checkboxes[idx - 1].focused = true;
                    return true;
                }
            }
            true
        } else {
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.focus_area == ReconFocusArea::Inputs {
            self.inputs.move_right()
        } else if self.focus_area == ReconFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            if let Some(idx) = focused_idx {
                if idx >= self.option_checkboxes.len() - 1 {
                    return false;
                } else {
                    self.option_checkboxes[idx].focused = false;
                    self.option_checkboxes[idx + 1].focused = true;
                    return true;
                }
            }
            true
        } else {
            true
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ReconFocusArea::Inputs {
            let cursor_pos = self.inputs.fields[0].cursor_pos;
            cursor_pos == 0
        } else if self.focus_area == ReconFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            focused_idx == Some(0)
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ReconFocusArea::Inputs {
            let field = &self.inputs.fields[0];
            field.cursor_pos >= field.value.len()
        } else if self.focus_area == ReconFocusArea::Options {
            let focused_idx = self.option_checkboxes.iter().position(|cb| cb.focused);
            focused_idx == Some(self.option_checkboxes.len() - 1)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ReconFocusArea::Inputs && self.inputs.is_focused()
    }
}
