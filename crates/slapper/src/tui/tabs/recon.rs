use crate::recon::FullReconResult;
use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, Checkbox, InputField, InputGroup, ProgressGauge, ScrollableText,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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
    pub focused_checkbox_index: usize,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReconFocusArea {
    Inputs,
    Options,
    Results,
}

const CHECKBOX_COLUMNS: usize = 2;
const CHECKBOX_ROWS_PER_COLUMN: usize = 8;

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
    pub no_takeover: bool,
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
            Checkbox::new("Skip Takeover Detection").checked(false),
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
            focused_checkbox_index: 0,
            error: None,
        }
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
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
            no_tech: self.option_checkboxes.first().map(|cb| cb.checked).unwrap_or(false),
            no_dns: self.option_checkboxes.get(1).map(|cb| cb.checked).unwrap_or(false),
            no_geo: self.option_checkboxes.get(2).map(|cb| cb.checked).unwrap_or(false),
            no_whois: self.option_checkboxes.get(3).map(|cb| cb.checked).unwrap_or(false),
            no_subdomains: self.option_checkboxes.get(4).map(|cb| cb.checked).unwrap_or(false),
            no_ssl: self.option_checkboxes.get(5).map(|cb| cb.checked).unwrap_or(false),
            no_dns_records: self.option_checkboxes.get(6).map(|cb| cb.checked).unwrap_or(false),
            no_js: self.option_checkboxes.get(7).map(|cb| cb.checked).unwrap_or(false),
            no_content: self.option_checkboxes.get(8).map(|cb| cb.checked).unwrap_or(false),
            no_cloud: self.option_checkboxes.get(9).map(|cb| cb.checked).unwrap_or(false),
            no_wayback: self.option_checkboxes.get(10).map(|cb| cb.checked).unwrap_or(false),
            no_cors: self.option_checkboxes.get(11).map(|cb| cb.checked).unwrap_or(false),
            no_threat: self.option_checkboxes.get(12).map(|cb| cb.checked).unwrap_or(false),
            no_cve: self.option_checkboxes.get(13).map(|cb| cb.checked).unwrap_or(false),
            no_email: self.option_checkboxes.get(14).map(|cb| cb.checked).unwrap_or(false),
            no_takeover: self.option_checkboxes.get(15).map(|cb| cb.checked).unwrap_or(false),
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
            ratatui::style::Style::default().fg(tc!(success)),
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
                ratatui::style::Style::default().fg(tc!(accent)),
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
        } else if results.tech_error.is_some() {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "Tech Stack: Failed",
                ratatui::style::Style::default().fg(tc!(error)),
            )));
            if let Some(ref err) = results.tech_error {
                self.results_view.add_line(Line::from(format!("  {}", err)));
            }
        }

        if let Some(ref geo) = results.geolocation {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "Geolocation:",
                ratatui::style::Style::default().fg(tc!(accent)),
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
                ratatui::style::Style::default().fg(tc!(error)),
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
                ratatui::style::Style::default().fg(tc!(accent)),
            )));
            if let Some(ref cert) = ssl.certificate {
                self.results_view
                    .add_line(Line::from(format!("  Subject: {}", cert.subject)));
                self.results_view
                    .add_line(Line::from(format!("  Issuer: {}", cert.issuer)));
            }
        } else if results.ssl_error.is_some() {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "SSL/TLS: Failed",
                ratatui::style::Style::default().fg(tc!(error)),
            )));
            if let Some(ref err) = results.ssl_error {
                self.results_view.add_line(Line::from(format!("  {}", err)));
            }
        }

        if let Some(ref subdomains) = results.subdomains {
            if !subdomains.subdomains.is_empty() {
                self.results_view.add_line(Line::from(""));
                self.results_view.add_line(Line::from(Span::styled(
                    format!("Subdomains ({}):", subdomains.subdomains.len()),
                    ratatui::style::Style::default().fg(tc!(accent)),
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

    fn options_row_count(&self) -> usize {
        self.option_checkboxes.len() / CHECKBOX_COLUMNS
    }

    fn options_window_start(&self, visible_rows: usize) -> usize {
        let row_count = self.options_row_count();
        if visible_rows >= row_count {
            return 0;
        }

        let focused_row = self.focused_checkbox_index % row_count;
        let max_start = row_count - visible_rows;
        focused_row.saturating_sub(visible_rows - 1).min(max_start)
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
        self.error = None;
        self.focus_area = ReconFocusArea::Inputs;
        self.focused_checkbox_index = 0;
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
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
        // Dynamic layout based on terminal height
        let (input_height, results_min) = if area.height < 24 {
            // Small terminal: use 75% for inputs, ensure some results area visible
            let h = ((area.height as f32 * 0.75) as u16).clamp(6, 16);
            (h, 2)
        } else {
            (16, 3)
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(input_height),
                Constraint::Min(results_min),
            ])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        // Simple 3-row layout: 2 input rows + options row
        let row_height = (input_area.height / 3).max(2);
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(row_height.min(input_area.height)),
                Constraint::Length(row_height.min(input_area.height.saturating_sub(row_height))),
                Constraint::Min(0),
            ])
            .split(input_area);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        let Some(options_area) = input_chunks.get(2) else {
            return;
        };
        let option_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(*options_area);

        let visible_rows = options_area.height.min(CHECKBOX_ROWS_PER_COLUMN as u16) as usize;
        if visible_rows > 0 {
            let row_offset = self.options_window_start(visible_rows);
            let row_constraints = vec![Constraint::Length(1); visible_rows];
            let left_options = Layout::default()
                .direction(Direction::Vertical)
                .constraints(row_constraints.clone())
                .split(option_chunks[0]);

            let right_options = Layout::default()
                .direction(Direction::Vertical)
                .constraints(row_constraints)
                .split(option_chunks[1]);

            let is_options_focused = self.focus_area == ReconFocusArea::Options;

            for (visible_idx, row_area) in left_options.iter().enumerate() {
                let checkbox_idx = row_offset + visible_idx;
                if let Some(cb) = self.option_checkboxes.get(checkbox_idx) {
                    cb.render_with_focus(
                        is_options_focused && checkbox_idx == self.focused_checkbox_index,
                        f,
                        *row_area,
                    );
                }
            }

            for (visible_idx, row_area) in right_options.iter().enumerate() {
                let checkbox_idx = row_offset + visible_idx + CHECKBOX_ROWS_PER_COLUMN;
                if let Some(cb) = self.option_checkboxes.get(checkbox_idx) {
                    cb.render_with_focus(
                        is_options_focused && checkbox_idx == self.focused_checkbox_index,
                        f,
                        *row_area,
                    );
                }
            }
        }

        if self.state == AppState::Running {
            self.progress.render(f, results_area);
        } else if let Some(ref err) = self.error {
            use ratatui::style::Style;
            use ratatui::widgets::{Block, Borders, Paragraph};
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Reconnaissance - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, Some(tc!(success)));
        } else {
            let cli_example = "slapper recon example.com --no-tech --no-whois";
            let placeholder = empty_state_paragraph(
                "Reconnaissance",
                format!(
                    "Enter target and press Enter to start recon\n\nCLI equivalent: {}",
                    cli_example
                ),
            );
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for ReconTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            ReconFocusArea::Inputs => {
                self.inputs.blur();
                self.focused_checkbox_index = 0;
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
            ReconFocusArea::Inputs => {
                self.inputs.blur();
                ReconFocusArea::Results
            }
            ReconFocusArea::Options => {
                self.inputs.focus(0);
                ReconFocusArea::Inputs
            }
            ReconFocusArea::Results => {
                self.focused_checkbox_index = 0;
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

    fn handle_delete(&mut self) {
        if !self.is_running() && self.focus_area == ReconFocusArea::Inputs {
            self.inputs.delete();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == ReconFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if !self.is_running() {
            if self.focus_area == ReconFocusArea::Results {
                return Some(self.results_view.get_content());
            } else if self.focus_area == ReconFocusArea::Inputs {
                return self.inputs.get_focused_value();
            }
        }
        None
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() {
            if self.focus_area == ReconFocusArea::Inputs {
                self.inputs.move_word_forward();
            }
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() {
            if self.focus_area == ReconFocusArea::Inputs {
                self.inputs.move_word_backward();
            }
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == ReconFocusArea::Inputs {
                self.inputs.move_home();
            } else if self.focus_area == ReconFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == ReconFocusArea::Inputs {
                self.inputs.move_end();
            } else if self.focus_area == ReconFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = ReconFocusArea::Inputs;
            self.inputs.focus(0);
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = ReconFocusArea::Results;
        }
    }

    fn handle_enter(&mut self) {
        if self.focus_area == ReconFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }

        if self.focus_area == ReconFocusArea::Options {
            if !self.is_running() {
                if let Some(cb) = self.option_checkboxes.get_mut(self.focused_checkbox_index) {
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
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            if self.focus_area == ReconFocusArea::Options {
                if self.focused_checkbox_index == 0 {
                    self.focused_checkbox_index = self.option_checkboxes.len() - 1;
                } else {
                    self.focused_checkbox_index = self.focused_checkbox_index.saturating_sub(1);
                }
            } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_up();
            } else {
                self.inputs.focus_prev();
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            if self.focus_area == ReconFocusArea::Options {
                if self.focused_checkbox_index >= self.option_checkboxes.len() - 1 {
                    self.focused_checkbox_index = 0;
                } else {
                    self.focused_checkbox_index += 1;
                }
            } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_results_down();
            } else {
                self.inputs.focus_next();
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() {
            if self.focus_area == ReconFocusArea::Inputs {
                self.inputs.move_left()
            } else if self.focus_area == ReconFocusArea::Options {
                if self.focused_checkbox_index == 0 {
                    false
                } else {
                    self.focused_checkbox_index = self.focused_checkbox_index.saturating_sub(1);
                    true
                }
            } else {
                true
            }
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() {
            if self.focus_area == ReconFocusArea::Inputs {
                self.inputs.move_right()
            } else if self.focus_area == ReconFocusArea::Options {
                if self.option_checkboxes.is_empty() || self.focused_checkbox_index >= self.option_checkboxes.len() - 1 {
                    false
                } else {
                    self.focused_checkbox_index += 1;
                    true
                }
            } else {
                true
            }
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ReconFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else if self.focus_area == ReconFocusArea::Options {
            self.option_checkboxes.is_empty() || self.focused_checkbox_index == 0
        } else if self.focus_area == ReconFocusArea::Results {
            self.results_view.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ReconFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else if self.focus_area == ReconFocusArea::Options {
            self.option_checkboxes.is_empty()
                || self.focused_checkbox_index >= self.option_checkboxes.len().saturating_sub(1)
        } else if self.focus_area == ReconFocusArea::Results {
            self.results_view.is_at_right_edge()
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ReconFocusArea::Inputs && self.inputs.is_focused()
    }

    fn stop(&mut self) {
        ReconTab::stop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, layout::Rect, Terminal};

    fn create_test_tab() -> ReconTab {
        ReconTab::new()
    }

    #[test]
    fn test_focus_next_sets_checkbox_index() {
        let mut tab = create_test_tab();
        assert_eq!(tab.focus_area, ReconFocusArea::Inputs);
        assert_eq!(tab.focused_checkbox_index, 0);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, ReconFocusArea::Options);
        assert_eq!(tab.focused_checkbox_index, 0);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, ReconFocusArea::Results);
    }

    #[test]
    fn test_focus_prev_restores_checkbox_index() {
        let mut tab = create_test_tab();
        tab.focus_area = ReconFocusArea::Results;
        tab.focused_checkbox_index = 5;

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, ReconFocusArea::Options);
        assert_eq!(tab.focused_checkbox_index, 0);
    }

    #[test]
    fn test_handle_up_wraps_to_last() {
        let mut tab = create_test_tab();
        tab.focus_area = ReconFocusArea::Options;
        tab.focused_checkbox_index = 0;

        tab.handle_up();
        assert_eq!(tab.focused_checkbox_index, tab.option_checkboxes.len() - 1);
    }

    #[test]
    fn test_handle_down_wraps_to_first() {
        let mut tab = create_test_tab();
        tab.focus_area = ReconFocusArea::Options;
        tab.focused_checkbox_index = tab.option_checkboxes.len() - 1;

        tab.handle_down();
        assert_eq!(tab.focused_checkbox_index, 0);
    }

    #[test]
    fn test_handle_enter_toggles_checkbox() {
        let mut tab = create_test_tab();
        tab.focus_area = ReconFocusArea::Options;
        tab.focused_checkbox_index = 0;
        assert!(!tab.option_checkboxes[0].checked);

        tab.handle_enter();
        assert!(tab.option_checkboxes[0].checked);

        tab.handle_enter();
        assert!(!tab.option_checkboxes[0].checked);
    }

    #[test]
    fn test_cycling_with_j_does_not_corrupt_checkbox_state() {
        let mut tab = create_test_tab();
        tab.focus_area = ReconFocusArea::Options;
        tab.focused_checkbox_index = 0;
        tab.option_checkboxes[0].checked = true;

        for i in 0..20 {
            tab.handle_down();
            assert_eq!(
                tab.focused_checkbox_index,
                (i + 1) % 16,
                "After {} downs, focus should be at {} not corrupted",
                i + 1,
                (i + 1) % 16
            );
        }

        assert!(
            tab.option_checkboxes[0].checked,
            "Checkbox 0 should still be checked"
        );
    }

    #[test]
    fn test_cycling_with_k_does_not_corrupt_checkbox_state() {
        let mut tab = create_test_tab();
        tab.focus_area = ReconFocusArea::Options;
        tab.focused_checkbox_index = 15;
        tab.option_checkboxes[15].checked = true;

        for i in 0..20 {
            tab.handle_up();
            let expected = (15 + 16 - (i + 1) % 16) % 16;
            assert_eq!(
                tab.focused_checkbox_index,
                expected,
                "After {} ups, focus should be at {} not corrupted",
                i + 1,
                expected
            );
        }

        assert!(
            tab.option_checkboxes[15].checked,
            "Checkbox 15 should still be checked"
        );
    }

    #[test]
    fn test_focus_cycle_completes() {
        let mut tab = create_test_tab();
        assert_eq!(tab.focus_area, ReconFocusArea::Inputs);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, ReconFocusArea::Options);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, ReconFocusArea::Results);

        tab.handle_focus_next();
        assert_eq!(
            tab.focus_area,
            ReconFocusArea::Inputs,
            "Should cycle back to Inputs"
        );
    }

    #[test]
    fn test_options_window_keeps_focused_row_visible() {
        let mut tab = create_test_tab();
        tab.focus_area = ReconFocusArea::Options;

        tab.focused_checkbox_index = 0;
        assert_eq!(tab.options_window_start(3), 0);

        tab.focused_checkbox_index = 2;
        assert_eq!(tab.options_window_start(3), 0);

        tab.focused_checkbox_index = 6;
        assert_eq!(tab.options_window_start(3), 4);

        tab.focused_checkbox_index = 7;
        assert_eq!(tab.options_window_start(3), 5);
    }

    #[test]
    fn test_render_keeps_focused_checkbox_visible_in_small_terminal() {
        let mut tab = create_test_tab();
        tab.focus_area = ReconFocusArea::Options;
        tab.focused_checkbox_index = 7;

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal
            .draw(|f| {
                tab.render(f, Rect::new(0, 0, 80, 20), false);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let focused_marker = buf
            .cell((0, 14))
            .expect("focused checkbox cell should be in bounds");
        assert_eq!(
            focused_marker.symbol(),
            ">",
            "Focused checkbox marker should remain visible after cycling through a small viewport"
        );
    }
}
