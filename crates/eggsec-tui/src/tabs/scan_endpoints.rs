use crate::app::tab_error::TabError;
use crate::components::{Checkbox, InputField};
use crate::tabs::core::{field_as, render_results_area, start_scan, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_boilerplate, tc};
use eggsec::scanner::endpoints::EndpointScanResults;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanEndpointsFocusArea {
    Inputs,
    Options,
    Results,
}

pub struct ScanEndpointsTab {
    pub core: TabCore,
    pub results: Option<EndpointScanResults>,
    pub include_404_checkbox: Checkbox,
    pub focus_area: ScanEndpointsFocusArea,
}

impl ScanEndpointsTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("Target URL"))
            .add(InputField::new("Concurrency").with_value("20"))
            .add(InputField::new("Timeout (s)").with_value("10"))
            .add(InputField::new("Wordlist (default: common)"));

        Self {
            core: TabCore::new("Scanning endpoints...", "Results").with_inputs(inputs),
            results: None,
            include_404_checkbox: Checkbox::new("Check for 404s").checked(true),
            focus_area: ScanEndpointsFocusArea::Inputs,
        }
    }

    pub fn get_results(&self) -> Option<&EndpointScanResults> {
        self.results.as_ref()
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn concurrency(&self) -> usize {
        field_as(&self.core, 1, 20)
    }

    pub fn timeout(&self) -> u64 {
        field_as(&self.core, 2, 10)
    }

    pub fn wordlist(&self) -> Option<&str> {
        let w = crate::tabs::core::field_str(&self.core, 3);
        if w.is_empty() {
            None
        } else {
            Some(w)
        }
    }

    pub fn include_404(&self) -> bool {
        self.include_404_checkbox.checked
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }

    pub fn set_results(&mut self, results: EndpointScanResults) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.core.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &EndpointScanResults) {
        use ratatui::style::Modifier;

        self.core.results_view.clear();

        let base_url = results.base_url.clone();
        let endpoints_scanned = results.endpoints_scanned;
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

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("URL: ", Style::default().fg(tc!(accent))),
            Span::raw(base_url),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Scanned: ", Style::default().fg(tc!(secondary))),
            Span::raw(endpoints_scanned.to_string()),
            Span::raw(" | "),
            Span::styled("Found: ", Style::default().fg(tc!(info))),
            Span::raw(endpoints_found.to_string()),
            Span::raw(" | "),
            Span::styled("Interesting: ", Style::default().fg(tc!(error))),
            Span::raw(interesting_findings.to_string()),
        ]));

        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(vec![
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
                let truncate_pos = path
                    .char_indices()
                    .take_while(|(i, _)| *i < 35)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(35);
                format!("{}...", &path[..truncate_pos])
            } else {
                path
            };

            self.core.results_view.add_line(Line::from(vec![
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
        if self.target().is_empty() {
            self.core.state = AppState::Error("Target cannot be empty".to_string());
            self.core.error =
                Some(TabError::Target("Target cannot be empty".to_string()));
            return;
        }

        let wordlist_path = self.wordlist().map(|s| s.to_string());
        if let Some(path_str) = wordlist_path {
            let path = std::path::Path::new(&path_str);
            if !path.exists() {
                self.core.state =
                    AppState::Error(format!("Wordlist file not found: {}", path_str));
                self.core.error = Some(TabError::Config(format!(
                    "Wordlist file not found: {}",
                    path_str
                )));
                return;
            }
            if !path.is_file() {
                self.core.state =
                    AppState::Error(format!("Wordlist path is not a file: {}", path_str));
                self.core.error = Some(TabError::Config(format!(
                    "Wordlist path is not a file: {}",
                    path_str
                )));
                return;
            }
        }

        start_scan(&mut self.core);
    }
}

impl Default for ScanEndpointsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ScanEndpointsTab {
    fn state(&self) -> AppState {
        self.core.state.clone()
    }

    fn progress(&self) -> f64 {
        self.core.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.core.reset_all();
        if let Some(field) = self.core.inputs.fields.get_mut(1) {
            field.value = "20".to_string();
            field.cursor_pos = 2;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(2) {
            field.value = "10".to_string();
            field.cursor_pos = 2;
        }
        self.focus_area = ScanEndpointsFocusArea::Inputs;
        self.include_404_checkbox.checked = true;
    }

    fn set_error(&mut self, error: TabError) {
        crate::tabs::core::tab_state_set_error(&mut self.core, error);
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

        for (i, field) in self.core.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        let include_404 = self.include_404_checkbox.clone();
        if let Some(chunk) = input_chunks.get(4) {
            include_404.render(f, *chunk);
        }

        render_results_area(
            f,
            results_area,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Results",
            "Results will appear here after running",
        );
    }
}

impl TabInput for ScanEndpointsTab {
    tab_input_boilerplate!(
        ScanEndpointsTab,
        core: core,
        focus: focus_area,
        Inputs: ScanEndpointsFocusArea::Inputs,
        Results: ScanEndpointsFocusArea::Results
    );

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == ScanEndpointsFocusArea::Inputs {
            self.core.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == ScanEndpointsFocusArea::Inputs {
            self.core.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == ScanEndpointsFocusArea::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ScanEndpointsFocusArea::Inputs => {
                self.core.inputs.blur();
                ScanEndpointsFocusArea::Options
            }
            ScanEndpointsFocusArea::Options => ScanEndpointsFocusArea::Results,
            ScanEndpointsFocusArea::Results => {
                self.core.inputs.focus(0);
                ScanEndpointsFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ScanEndpointsFocusArea::Inputs => {
                self.core.inputs.blur();
                ScanEndpointsFocusArea::Results
            }
            ScanEndpointsFocusArea::Options => {
                self.core.inputs.focus(0);
                ScanEndpointsFocusArea::Inputs
            }
            ScanEndpointsFocusArea::Results => ScanEndpointsFocusArea::Options,
        };
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanEndpointsFocusArea::Inputs {
            if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
                self.core.scroll_results_up();
            } else {
                self.core.inputs.focus_prev();
            }
        } else if self.focus_area == ScanEndpointsFocusArea::Results {
            self.core.scroll_results_up();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == ScanEndpointsFocusArea::Inputs {
            if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
                self.core.scroll_results_down();
            } else {
                self.core.inputs.focus_next();
            }
        } else if self.focus_area == ScanEndpointsFocusArea::Results {
            self.core.scroll_results_down();
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ScanEndpointsFocusArea::Inputs && self.core.inputs.is_focused()
    }

    fn handle_enter(&mut self) {
        if self.focus_area == ScanEndpointsFocusArea::Results {
            return;
        }
        if self.is_running() {
            self.core.stop();
            return;
        }
        if self.core.inputs.is_focused() {
            self.core.inputs.blur();
            return;
        }
        if self.focus_area == ScanEndpointsFocusArea::Options {
            self.include_404_checkbox.checked = !self.include_404_checkbox.checked;
            return;
        }
        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        match self.focus_area {
            ScanEndpointsFocusArea::Inputs => self.core.inputs.blur(),
            ScanEndpointsFocusArea::Options | ScanEndpointsFocusArea::Results => {
                self.focus_area = ScanEndpointsFocusArea::Inputs;
                self.core.inputs.focus(0);
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() && self.focus_area == ScanEndpointsFocusArea::Inputs {
            self.core.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() && self.focus_area == ScanEndpointsFocusArea::Inputs {
            self.core.inputs.move_right()
        } else {
            false
        }
    }
}
