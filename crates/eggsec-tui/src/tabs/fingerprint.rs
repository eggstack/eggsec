use crate::app::tab_error::TabError;
use crate::components::InputField;
use crate::tabs::core::{render_results_area, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_boilerplate, tc};
use eggsec::scanner::fingerprint::FingerprintResults;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FingerprintFocusArea {
    Inputs,
    Results,
}

pub struct FingerprintTab {
    pub core: TabCore,
    pub results: Option<FingerprintResults>,
    pub focus_area: FingerprintFocusArea,
}

impl FingerprintTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(InputField::new("Target Host"))
            .add(
                InputField::new("Ports (comma-separated)")
                    .with_value("80,443,22,21,25,3306,5432,6379,27017"),
            )
            .add(InputField::new("Timeout (s)").with_value("5"));

        Self {
            core: TabCore::new("Fingerprinting...", "Results").with_inputs(inputs),
            results: None,
            focus_area: FingerprintFocusArea::Inputs,
        }
    }

    pub fn get_results(&self) -> Option<&FingerprintResults> {
        self.results.as_ref()
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn ports(&self) -> &str {
        self.core
            .inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn timeout(&self) -> u64 {
        self.core
            .inputs
            .fields
            .get(2)
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(5)
    }

    pub fn set_results(&mut self, results: FingerprintResults) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.core.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &FingerprintResults) {
        self.core.results_view.clear();

        let host = results.host.clone();
        let services_identified = results.services_identified;

        let fp_data: Vec<_> = results
            .results
            .iter()
            .map(|fp| {
                let banner = fp
                    .banner
                    .as_deref()
                    .unwrap_or("-")
                    .lines()
                    .next()
                    .unwrap_or("-");
                let banner_display = if banner.len() > 40 {
                    let truncate_pos = banner
                        .char_indices()
                        .take_while(|(i, _)| *i < 37)
                        .last()
                        .map(|(i, c)| i + c.len_utf8())
                        .unwrap_or(37);
                    format!("{}...", &banner[..truncate_pos])
                } else {
                    banner.to_string()
                };
                (
                    fp.port,
                    fp.service.clone(),
                    fp.version.clone(),
                    banner_display,
                )
            })
            .collect();

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Host: ", Style::default().fg(tc!(warning))),
            Span::raw(host),
        ]));

        self.core.results_view.add_line(Line::from(vec![
            Span::styled("Services identified: ", Style::default().fg(tc!(info))),
            Span::raw(services_identified.to_string()),
        ]));

        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<8}", "PORT"), Style::default().fg(tc!(warning))),
            Span::styled(
                format!("{:<15}", "SERVICE"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled(
                format!("{:<12}", "VERSION"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled("BANNER", Style::default().fg(tc!(warning))),
        ]));

        for (port, service, version, banner_display) in fp_data {
            self.core.results_view.add_line(Line::from(vec![
                Span::styled(format!("{:<8}", port), Style::default().fg(tc!(success))),
                Span::raw(format!("{:<15}", service)),
                Span::raw(format!("{:<12}", version.as_deref().unwrap_or("-"))),
                Span::styled(banner_display, Style::default().fg(tc!(text_dim))),
            ]));
        }
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.core.state = AppState::Running;
            self.core.progress.current = 0;
            self.results = None;
            self.core.results_view.clear();
            self.core.error = None;
        }
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.core.update_progress(completed, total);
    }
}

impl Default for FingerprintTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for FingerprintTab {
    fn state(&self) -> AppState {
        self.core.state.clone()
    }

    fn progress(&self) -> f64 {
        self.core.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.core.reset_all();
        if let Some(field) = self.core.inputs.fields.get_mut(1) {
            field.value = "80,443,22,21,25,3306,5432,6379,27017".to_string();
            field.cursor_pos = 36;
        }
        if let Some(field) = self.core.inputs.fields.get_mut(2) {
            field.value = "5".to_string();
            field.cursor_pos = 1;
        }
        self.focus_area = FingerprintFocusArea::Inputs;
    }

    fn set_error(&mut self, error: TabError) {
        crate::tabs::core::tab_state_set_error(&mut self.core, error);
    }
}

impl TabRender for FingerprintTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Fingerprint Configuration ")
            .border_style(Style::default().fg(
                if self.focus_area == FingerprintFocusArea::Inputs {
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
            ])
            .split(input_inner);

        for (i, field) in self.core.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(Style::default().fg(
                if self.focus_area == FingerprintFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        let results_inner = results_block.inner(results_area);
        f.render_widget(results_block, results_area);

        render_results_area(
            f,
            results_inner,
            &self.core.state,
            &self.core.error,
            &self.core.results_view,
            &self.core.progress,
            "Results",
            "Results will appear here after running",
        );
    }
}

impl TabInput for FingerprintTab {
    tab_input_boilerplate!(
        FingerprintTab,
        core: core,
        focus: focus_area,
        Inputs: FingerprintFocusArea::Inputs,
        Results: FingerprintFocusArea::Results
    );

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == FingerprintFocusArea::Inputs {
            self.core.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == FingerprintFocusArea::Inputs {
            self.core.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == FingerprintFocusArea::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            FingerprintFocusArea::Inputs => {
                self.core.inputs.blur();
                self.focus_area = FingerprintFocusArea::Results;
            }
            FingerprintFocusArea::Results => {
                self.focus_area = FingerprintFocusArea::Inputs;
                if !self.core.inputs.fields.is_empty() {
                    self.core.inputs.focus(0);
                }
            }
        }
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == FingerprintFocusArea::Results {
            if !self.core.inputs.fields.is_empty() {
                self.core
                    .inputs
                    .focus(self.core.inputs.fields.len() - 1);
            }
            self.focus_area = FingerprintFocusArea::Inputs;
        } else {
            self.core.inputs.blur();
            self.focus_area = FingerprintFocusArea::Results;
        }
    }

    fn handle_enter(&mut self) {
        if self.focus_area == FingerprintFocusArea::Results {
            return;
        }

        if self.is_running() {
            self.core.stop();
        } else if self.core.inputs.is_focused() {
            self.core.inputs.blur();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        match self.focus_area {
            FingerprintFocusArea::Inputs => self.core.inputs.blur(),
            FingerprintFocusArea::Results => {
                self.focus_area = FingerprintFocusArea::Inputs;
                self.core.inputs.focus(0);
            }
        }
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == FingerprintFocusArea::Results {
            self.core.scroll_results_up();
        } else if self.focus_area == FingerprintFocusArea::Inputs {
            if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
                self.core.scroll_results_up();
            } else {
                self.core.inputs.focus_prev();
            }
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == FingerprintFocusArea::Results {
            self.core.scroll_results_down();
        } else if self.focus_area == FingerprintFocusArea::Inputs {
            if !self.core.inputs.is_focused() && !self.core.results_view.is_empty() {
                self.core.scroll_results_down();
            } else {
                self.core.inputs.focus_next();
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == FingerprintFocusArea::Inputs {
            self.core.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == FingerprintFocusArea::Inputs {
            self.core.inputs.move_right()
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == FingerprintFocusArea::Inputs && self.core.inputs.is_focused()
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == FingerprintFocusArea::Inputs {
            self.core.inputs.fields.is_empty() || self.core.inputs.is_at_left_edge()
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == FingerprintFocusArea::Inputs {
            self.core.inputs.fields.is_empty() || self.core.inputs.is_at_right_edge()
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab() -> FingerprintTab {
        FingerprintTab::new()
    }

    #[test]
    fn test_enter_in_inputs_focused_blurs_does_not_start() {
        let mut tab = create_test_tab();
        tab.focus_area = FingerprintFocusArea::Inputs;
        tab.core.inputs.focus(0);
        assert!(tab.core.inputs.is_focused());
        tab.handle_enter();
        assert!(!tab.core.inputs.is_focused());
        assert!(!tab.is_running());
    }

    #[test]
    fn test_enter_in_results_no_op() {
        let mut tab = create_test_tab();
        tab.focus_area = FingerprintFocusArea::Results;
        tab.handle_enter();
        assert!(!tab.is_running());
    }
}
