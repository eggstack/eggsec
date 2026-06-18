use crate::app::tab_error::TabError;
use crate::components::{
    empty_state_paragraph, InputField, InputGroup, ScrollableText, Selector, SelectorItem,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::tc;
use eggsec::compliance::{ComplianceFramework, ComplianceReport, ComplianceStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct ComplianceTab {
    pub inputs: InputGroup,
    pub framework_selector: Selector,
    pub report: Option<ComplianceReport>,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: ComplianceFocusArea,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComplianceFocusArea {
    Inputs,
    Framework,
    Results,
}

impl ComplianceTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target"))
            .add(InputField::new("Output File (optional)"));

        let framework_selector = Selector::new("Framework").items(vec![
            SelectorItem::new("OWASP Top 10", "owasp"),
            SelectorItem::new("PCI DSS", "pci"),
            SelectorItem::new("HIPAA", "hipaa"),
            SelectorItem::new("SOC 2", "soc2"),
        ]);

        Self {
            inputs,
            framework_selector,
            report: None,
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: ComplianceFocusArea::Inputs,
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

    pub fn output_file(&self) -> Option<&str> {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .filter(|v| !v.is_empty())
    }

    pub fn selected_framework(&self) -> ComplianceFramework {
        match self.framework_selector.selected {
            0 => ComplianceFramework::OWASP,
            1 => ComplianceFramework::PCIDSS,
            2 => ComplianceFramework::HIPAA,
            _ => ComplianceFramework::SOC2,
        }
    }

    pub fn set_report(&mut self, report: ComplianceReport) {
        self.report = Some(report.clone());
        self.state = AppState::Completed;
        self.results_view.clear();

        let summary = report.summarize();

        self.results_view.add_line(Line::from(Span::styled(
            format!("Compliance Report: {}", report.framework),
            Style::default().fg(tc!(success)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Overall Score: {:.1}%", report.overall_score),
            Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Risk Level: {:?}", summary.risk_level),
            Style::default().fg(match summary.risk_level {
                eggsec::compliance::report::RiskLevel::Critical => tc!(error),
                eggsec::compliance::report::RiskLevel::High => tc!(error),
                eggsec::compliance::report::RiskLevel::Medium => tc!(warning),
                eggsec::compliance::report::RiskLevel::Low => tc!(success),
            }),
        )));
        self.results_view.add_line(Line::from(""));
        let na_count = report
            .findings
            .iter()
            .filter(|f| f.status == ComplianceStatus::NotApplicable)
            .count();
        let review_count = report
            .findings
            .iter()
            .filter(|f| f.status == ComplianceStatus::NeedsReview)
            .count();
        self.results_view.add_line(Line::from(format!(
            "Passed: {} | Failed: {} | N/A: {} | Review: {}",
            report.passed, report.failed, na_count, review_count
        )));
        self.results_view.add_line(Line::from(""));

        if !report.findings.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                "Findings:",
                Style::default().fg(tc!(warning)),
            )));
            for finding in &report.findings {
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {} - {}",
                    finding.severity, finding.requirement_id, finding.description
                )));
            }
        }
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.report = None;
            self.results_view.clear();
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }
}

impl Default for ComplianceTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ComplianceTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.report = None;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.framework_selector.select(0);
        self.focus_area = ComplianceFocusArea::Inputs;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for ComplianceTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            ComplianceFocusArea::Inputs => "Inputs",
            ComplianceFocusArea::Framework => "Framework",
            ComplianceFocusArea::Results => "Results",
        };
        Some(vec!["Compliance", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref err) = self.error {
            crate::tabs::core::render_error_block(f, area, "Compliance - Error", err);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(Style::default().fg(
                if self.focus_area != ComplianceFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        f.render_widget(&config_block, input_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(config_block.inner(input_area));

        for (i, field) in self.inputs.fields.iter().enumerate() {
            if let Some(chunk) = input_chunks.get(i) {
                field.render(f, *chunk, insert_mode);
            }
        }

        let mut sel = self.framework_selector.clone();
        sel.focused = self.focus_area == ComplianceFocusArea::Framework;
        if let Some(framework_area) = input_chunks.get(2) {
            sel.render(f, *framework_area);
        }

        if self.state == AppState::Running {
            use ratatui::widgets::{Block, Borders, Gauge};
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Generating compliance report..."),
                )
                .gauge_style(Style::default().fg(tc!(warning)))
                .ratio(0.5);
            f.render_widget(gauge, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view.render(f, results_area, None);
        } else {
            let placeholder = empty_state_paragraph(
                "Compliance Reporting",
                "Enter target, select framework, and press Enter",
            );
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for ComplianceTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ComplianceFocusArea::Inputs => {
                self.inputs.blur();
                self.framework_selector.focus();
                ComplianceFocusArea::Framework
            }
            ComplianceFocusArea::Framework => {
                self.framework_selector.blur();
                ComplianceFocusArea::Results
            }
            ComplianceFocusArea::Results => {
                self.inputs.focus(0);
                ComplianceFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ComplianceFocusArea::Inputs => {
                self.inputs.blur();
                ComplianceFocusArea::Results
            }
            ComplianceFocusArea::Framework => {
                self.framework_selector.blur();
                self.inputs.focus(0);
                ComplianceFocusArea::Inputs
            }
            ComplianceFocusArea::Results => {
                self.framework_selector.focus();
                ComplianceFocusArea::Framework
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else if self.focus_area == ComplianceFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == ComplianceFocusArea::Inputs {
                self.inputs.move_home();
            } else if self.focus_area == ComplianceFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == ComplianceFocusArea::Inputs {
                self.inputs.move_end();
            } else if self.focus_area == ComplianceFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = ComplianceFocusArea::Inputs;
        self.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = ComplianceFocusArea::Results;
        self.inputs.blur();
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        match self.focus_area {
            ComplianceFocusArea::Inputs => {
                if self.inputs.is_focused() {
                    self.inputs.blur();
                    return;
                }
            }
            ComplianceFocusArea::Framework => {
                if self.framework_selector.focused {
                    self.framework_selector.handle_enter();
                }
                return;
            }
            ComplianceFocusArea::Results => {
                return;
            }
        }

        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.inputs.blur();
        self.framework_selector.blur();
        self.focus_area = ComplianceFocusArea::Inputs;
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                ComplianceFocusArea::Framework => {
                    self.framework_selector.handle_up();
                }
                ComplianceFocusArea::Inputs => {
                    self.inputs.focus_prev();
                }
                ComplianceFocusArea::Results => {
                    self.results_view.scroll_up(1);
                }
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                ComplianceFocusArea::Framework => {
                    self.framework_selector.handle_down();
                }
                ComplianceFocusArea::Inputs => {
                    self.inputs.focus_next();
                }
                ComplianceFocusArea::Results => {
                    self.results_view.scroll_down(1);
                }
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.move_right()
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else if self.focus_area == ComplianceFocusArea::Framework {
            self.framework_selector.items.is_empty() || self.framework_selector.selected == 0
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else if self.focus_area == ComplianceFocusArea::Framework {
            self.framework_selector.items.is_empty()
                || self.framework_selector.selected
                    >= self.framework_selector.items.len().saturating_sub(1)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ComplianceFocusArea::Inputs && self.inputs.is_focused()
    }

    fn page_up(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.results_view.page_up(page_size);
    }

    fn page_down(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        self.results_view.page_down(page_size);
    }

    fn primary_target(&self) -> Option<String> {
        Some(self.target().to_string())
    }
}
