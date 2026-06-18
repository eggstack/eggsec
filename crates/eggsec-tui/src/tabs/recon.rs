use crate::components::{Checkbox, InputField, InputGroup};
use crate::tabs::core::{field_as, render_results_area, start_scan, StandardFocusArea, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_escape_3area, tab_input_boilerplate, tab_state_boilerplate, tc};
use eggsec::recon::FullReconResult;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct ReconTab {
    pub core: TabCore,
    pub results: Option<FullReconResult>,
    pub options: ReconOptions,
    pub option_checkboxes: Vec<Checkbox>,
    pub focus_area: StandardFocusArea,
    pub focused_checkbox_index: usize,
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
            core: TabCore::new("Running reconnaissance...", "Results").with_inputs(inputs),
            results: None,
            options: ReconOptions::default(),
            option_checkboxes,
            focus_area: StandardFocusArea::Inputs,
            focused_checkbox_index: 0,
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn concurrency(&self) -> usize {
        field_as(&self.core, 1, 20)
    }

    pub fn get_options(&self) -> ReconOptions {
        ReconOptions {
            no_tech: self
                .option_checkboxes
                .first()
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_dns: self
                .option_checkboxes
                .get(1)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_geo: self
                .option_checkboxes
                .get(2)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_whois: self
                .option_checkboxes
                .get(3)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_subdomains: self
                .option_checkboxes
                .get(4)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_ssl: self
                .option_checkboxes
                .get(5)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_dns_records: self
                .option_checkboxes
                .get(6)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_js: self
                .option_checkboxes
                .get(7)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_content: self
                .option_checkboxes
                .get(8)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_cloud: self
                .option_checkboxes
                .get(9)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_wayback: self
                .option_checkboxes
                .get(10)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_cors: self
                .option_checkboxes
                .get(11)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_threat: self
                .option_checkboxes
                .get(12)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_cve: self
                .option_checkboxes
                .get(13)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_email: self
                .option_checkboxes
                .get(14)
                .map(|cb| cb.checked)
                .unwrap_or(false),
            no_takeover: self
                .option_checkboxes
                .get(15)
                .map(|cb| cb.checked)
                .unwrap_or(false),
        }
    }

    pub fn get_results(&self) -> Option<&FullReconResult> {
        self.results.as_ref()
    }

    pub fn set_results(&mut self, results: FullReconResult) {
        self.results = Some(results.clone());
        self.core.state = AppState::Completed;
        self.core.results_view.clear();

        self.core.results_view.add_line(Line::from(Span::styled(
            format!("Reconnaissance Complete: {}", results.target),
            Style::default().fg(tc!(success)),
        )));
        self.core.results_view.add_line(Line::from(""));

        if let Some(ref domain) = results.domain {
            self.core
                .results_view
                .add_line(Line::from(format!("Domain: {}", domain)));
        }
        if let Some(ref ip) = results.ip_address {
            self.core
                .results_view
                .add_line(Line::from(format!("IP Address: {}", ip)));
        }

        if let Some(ref tech) = results.tech_stack {
            self.core.results_view.add_line(Line::from(""));
            self.core.results_view.add_line(Line::from(Span::styled(
                "Tech Stack:",
                Style::default().fg(tc!(accent)),
            )));
            if !tech.frameworks.is_empty() {
                self.core.results_view.add_line(Line::from(format!(
                    "  Frameworks: {}",
                    tech.frameworks.join(", ")
                )));
            }
            if !tech.servers.is_empty() {
                self.core.results_view.add_line(Line::from(format!(
                    "  Servers: {}",
                    tech.servers.join(", ")
                )));
            }
            if !tech.languages.is_empty() {
                self.core.results_view.add_line(Line::from(format!(
                    "  Languages: {}",
                    tech.languages.join(", ")
                )));
            }
        } else if results.tech_error.is_some() {
            self.core.results_view.add_line(Line::from(""));
            self.core.results_view.add_line(Line::from(Span::styled(
                "Tech Stack: Failed",
                Style::default().fg(tc!(error)),
            )));
            if let Some(ref err) = results.tech_error {
                self.core
                    .results_view
                    .add_line(Line::from(format!("  {}", err)));
            }
        }

        if let Some(ref geo) = results.geolocation {
            self.core.results_view.add_line(Line::from(""));
            self.core.results_view.add_line(Line::from(Span::styled(
                "Geolocation:",
                Style::default().fg(tc!(accent)),
            )));
            if let Some(ref country) = geo.country {
                self.core
                    .results_view
                    .add_line(Line::from(format!("  Country: {}", country)));
            }
            if let Some(ref city) = geo.city {
                self.core
                    .results_view
                    .add_line(Line::from(format!("  City: {}", city)));
            }
            if let Some(ref isp) = geo.isp {
                self.core
                    .results_view
                    .add_line(Line::from(format!("  ISP: {}", isp)));
            }
        }

        if let Some(ref geo_err) = results.geoip_error {
            self.core.results_view.add_line(Line::from(""));
            self.core.results_view.add_line(Line::from(Span::styled(
                "GeoIP Error:",
                Style::default().fg(tc!(error)),
            )));
            for line in geo_err.lines().take(4) {
                self.core
                    .results_view
                    .add_line(Line::from(format!("  {}", line)));
            }
        }

        if let Some(ref ssl) = results.ssl_analysis {
            self.core.results_view.add_line(Line::from(""));
            self.core.results_view.add_line(Line::from(Span::styled(
                "SSL/TLS:",
                Style::default().fg(tc!(accent)),
            )));
            if let Some(ref cert) = ssl.certificate {
                self.core
                    .results_view
                    .add_line(Line::from(format!("  Subject: {}", cert.subject)));
                self.core
                    .results_view
                    .add_line(Line::from(format!("  Issuer: {}", cert.issuer)));
            }
        } else if results.ssl_error.is_some() {
            self.core.results_view.add_line(Line::from(""));
            self.core.results_view.add_line(Line::from(Span::styled(
                "SSL/TLS: Failed",
                Style::default().fg(tc!(error)),
            )));
            if let Some(ref err) = results.ssl_error {
                self.core
                    .results_view
                    .add_line(Line::from(format!("  {}", err)));
            }
        }

        if let Some(ref subdomains) = results.subdomains {
            if !subdomains.subdomains.is_empty() {
                self.core.results_view.add_line(Line::from(""));
                self.core.results_view.add_line(Line::from(Span::styled(
                    format!("Subdomains ({}):", subdomains.subdomains.len()),
                    Style::default().fg(tc!(accent)),
                )));
                for sub in subdomains.subdomains.iter().take(5) {
                    self.core.results_view.add_line(Line::from(format!(
                        "  - {} ({})",
                        sub.subdomain,
                        sub.ip_addresses.join(", ")
                    )));
                }
                if subdomains.subdomains.len() > 5 {
                    self.core.results_view.add_line(Line::from(format!(
                        "  ... and {} more",
                        subdomains.subdomains.len() - 5
                    )));
                }
            }
        }
    }

    pub fn start(&mut self) {
        if start_scan(&mut self.core) {
            self.results = None;
        }
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }

    fn options_row_count(&self) -> usize {
        self.option_checkboxes.len() / CHECKBOX_COLUMNS
    }

    fn options_window_start(&self, visible_rows: usize) -> usize {
        let row_count = self.options_row_count();
        if row_count == 0 {
            return 0;
        }
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
    tab_state_boilerplate!(ReconTab, core: core);

    fn reset(&mut self) {
        self.core.reset_all();
        self.focus_area = StandardFocusArea::Inputs;
        self.focused_checkbox_index = 0;
        for cb in &mut self.option_checkboxes {
            cb.checked = false;
        }
    }
}

impl TabRender for ReconTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            StandardFocusArea::Inputs => "Inputs",
            StandardFocusArea::Options => "Options",
            StandardFocusArea::Results => "Results",
        };
        Some(vec!["Recon", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let (input_height, results_min) = if area.height < 24 {
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

        let is_config_focused =
            self.focus_area == StandardFocusArea::Inputs || self.focus_area == StandardFocusArea::Options;
        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(Style::default().fg(if is_config_focused {
                tc!(border_focused)
            } else {
                tc!(border)
            }));
        let config_inner = config_block.inner(input_area);
        f.render_widget(config_block, input_area);

        let row_height = (config_inner.height / 3).max(2);
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(row_height.min(config_inner.height)),
                Constraint::Length(row_height.min(config_inner.height.saturating_sub(row_height))),
                Constraint::Min(0),
            ])
            .split(config_inner);

        for (i, field) in self.core.inputs.fields.iter().enumerate() {
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

            let is_options_focused = self.focus_area == StandardFocusArea::Options;

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

        render_results_area(
            f,
            results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Reconnaissance",
            "Enter target and press Enter to start recon\n\nCLI equivalent: eggsec recon example.com --no-tech --no-whois",
        );
    }
}

impl TabInput for ReconTab {
    tab_input_boilerplate!(
        ReconTab,
        core: core,
        focus: focus_area,
        Inputs: StandardFocusArea::Inputs,
        Results: StandardFocusArea::Results
    );

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            StandardFocusArea::Inputs => {
                self.core.inputs.blur();
                self.focused_checkbox_index = 0;
                StandardFocusArea::Options
            }
            StandardFocusArea::Options => StandardFocusArea::Results,
            StandardFocusArea::Results => {
                self.core.inputs.focus(0);
                StandardFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            StandardFocusArea::Inputs => {
                self.core.inputs.blur();
                StandardFocusArea::Results
            }
            StandardFocusArea::Options => {
                self.core.inputs.focus(0);
                StandardFocusArea::Inputs
            }
            StandardFocusArea::Results => {
                self.focused_checkbox_index = 0;
                StandardFocusArea::Options
            }
        };
    }

    fn handle_delete(&mut self) {
        if !self.is_running() && self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.delete();
        }
    }

    fn handle_enter(&mut self) {
        let running = self.is_running();
        let inputs_focused = self.core.inputs.is_focused();
        crate::tabs::core::handle_enter_3area(
            &mut self.core,
            self.focus_area,
            StandardFocusArea::Inputs,
            StandardFocusArea::Options,
            StandardFocusArea::Results,
            running,
            inputs_focused,
            |_core| false,
        );
        if self.focus_area == StandardFocusArea::Options && !self.is_running() {
            if let Some(cb) = self
                .option_checkboxes
                .get_mut(self.focused_checkbox_index)
            {
                cb.toggle();
            }
        }
    }

    tab_escape_3area!(ReconTab, core: core, focus: focus_area, Inputs: StandardFocusArea::Inputs, Options: StandardFocusArea::Options, Results: StandardFocusArea::Results);

    fn handle_up(&mut self) {
        if !self.is_running() {
            if self.focus_area == StandardFocusArea::Options && !self.option_checkboxes.is_empty() {
                if self.focused_checkbox_index == 0 {
                    self.focused_checkbox_index = self.option_checkboxes.len() - 1;
                } else {
                    self.focused_checkbox_index =
                        self.focused_checkbox_index.saturating_sub(1);
                }
            } else if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
                self.core.scroll_results_up();
            } else {
                self.core.inputs.focus_prev();
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            if self.focus_area == StandardFocusArea::Options {
                if self.option_checkboxes.is_empty()
                    || self.focused_checkbox_index >= self.option_checkboxes.len() - 1
                {
                    self.focused_checkbox_index = 0;
                } else {
                    self.focused_checkbox_index += 1;
                }
            } else if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
                self.core.scroll_results_down();
            } else {
                self.core.inputs.focus_next();
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() {
            if self.focus_area == StandardFocusArea::Inputs {
                self.core.inputs.move_left()
            } else if self.focus_area == StandardFocusArea::Options {
                if self.focused_checkbox_index == 0 {
                    false
                } else {
                    self.focused_checkbox_index =
                        self.focused_checkbox_index.saturating_sub(1);
                    true
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() {
            if self.focus_area == StandardFocusArea::Inputs {
                self.core.inputs.move_right()
            } else if self.focus_area == StandardFocusArea::Options {
                if self.option_checkboxes.is_empty()
                    || self.focused_checkbox_index >= self.option_checkboxes.len() - 1
                {
                    false
                } else {
                    self.focused_checkbox_index += 1;
                    true
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.is_at_left_edge()
        } else if self.focus_area == StandardFocusArea::Options {
            self.option_checkboxes.is_empty() || self.focused_checkbox_index == 0
        } else if self.focus_area == StandardFocusArea::Results {
            self.core.results_view.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == StandardFocusArea::Inputs {
            self.core.inputs.is_at_right_edge()
        } else if self.focus_area == StandardFocusArea::Options {
            self.option_checkboxes.is_empty()
                || self.focused_checkbox_index >= self.option_checkboxes.len().saturating_sub(1)
        } else if self.focus_area == StandardFocusArea::Results {
            self.core.results_view.is_at_right_edge()
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == StandardFocusArea::Inputs && self.core.inputs.is_focused()
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
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);
        assert_eq!(tab.focused_checkbox_index, 0);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea::Options);
        assert_eq!(tab.focused_checkbox_index, 0);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea::Results);
    }

    #[test]
    fn test_focus_prev_restores_checkbox_index() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Results;
        tab.focused_checkbox_index = 5;

        tab.handle_focus_prev();
        assert_eq!(tab.focus_area, StandardFocusArea::Options);
        assert_eq!(tab.focused_checkbox_index, 0);
    }

    #[test]
    fn test_handle_up_wraps_to_last() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        tab.focused_checkbox_index = 0;

        tab.handle_up();
        assert_eq!(tab.focused_checkbox_index, tab.option_checkboxes.len() - 1);
    }

    #[test]
    fn test_handle_down_wraps_to_first() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        tab.focused_checkbox_index = tab.option_checkboxes.len() - 1;

        tab.handle_down();
        assert_eq!(tab.focused_checkbox_index, 0);
    }

    #[test]
    fn test_handle_enter_toggles_checkbox() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
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
        tab.focus_area = StandardFocusArea::Options;
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
        tab.focus_area = StandardFocusArea::Options;
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
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea::Options);

        tab.handle_focus_next();
        assert_eq!(tab.focus_area, StandardFocusArea::Results);

        tab.handle_focus_next();
        assert_eq!(
            tab.focus_area,
            StandardFocusArea::Inputs,
            "Should cycle back to Inputs"
        );
    }

    #[test]
    fn test_options_window_keeps_focused_row_visible() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;

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
    fn test_enter_in_inputs_focused_blurs_does_not_start() {
        let mut tab = create_test_tab();
        tab.core.inputs.focus(0);
        assert!(tab.core.inputs.is_focused());
        assert_eq!(tab.focus_area, StandardFocusArea::Inputs);
        tab.handle_enter();
        assert!(!tab.core.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_in_options_toggles_does_not_start() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        tab.focused_checkbox_index = 0;
        assert!(!tab.option_checkboxes[0].checked);
        tab.handle_enter();
        assert!(tab.option_checkboxes[0].checked);
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_from_results_no_op() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }

    #[test]
    fn test_render_keeps_focused_checkbox_visible_in_small_terminal() {
        let mut tab = create_test_tab();
        tab.focus_area = StandardFocusArea::Options;
        tab.focused_checkbox_index = 7;

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal
            .draw(|f| {
                tab.render(f, Rect::new(0, 0, 80, 20), false);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let found = (0..buf.area.height).any(|y| {
            (0..buf.area.width)
                .any(|x| buf.cell((x, y)).is_some_and(|cell| cell.symbol() == "▶"))
        });
        assert!(
            found,
            "Focused checkbox marker should remain visible after cycling through a small viewport"
        );
    }

    #[test]
    fn test_target_delegates_to_core() {
        let tab = create_test_tab();
        assert_eq!(tab.target(), "");
    }

    #[test]
    fn test_stop_delegates_to_core() {
        let mut tab = create_test_tab();
        tab.core.state = AppState::Running;
        tab.core.stop();
        assert_eq!(tab.core.state, AppState::Idle);
    }
}
