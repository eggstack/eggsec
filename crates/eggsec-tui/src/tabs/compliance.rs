use crate::app::tab_error::TabError;
use crate::components::{empty_state_paragraph, Selector, SelectorItem};
use crate::tabs::core::{
    self, render_config_block, render_error_block, render_input_fields, TabCore,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_state_boilerplate, tc};
use eggsec::compliance::{ComplianceFramework, ComplianceReport, ComplianceStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct ComplianceTab {
    pub core: TabCore,
    pub framework_selector: Selector,
    pub report: Option<ComplianceReport>,
    pub focus_area: ComplianceFocusArea,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComplianceFocusArea {
    Inputs,
    Framework,
    Results,
}

impl ComplianceTab {
    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
            .add(crate::components::InputField::new("Target"))
            .add(crate::components::InputField::new("Output File (optional)"));

        let framework_selector = Selector::new("Framework").items(vec![
            SelectorItem::new("OWASP Top 10", "owasp"),
            SelectorItem::new("PCI DSS", "pci"),
            SelectorItem::new("HIPAA", "hipaa"),
            SelectorItem::new("SOC 2", "soc2"),
        ]);

        Self {
            core: TabCore::new("Compliance", "Results").with_inputs(inputs),
            framework_selector,
            report: None,
            focus_area: ComplianceFocusArea::Inputs,
        }
    }

    pub fn target(&self) -> &str {
        self.core.target()
    }

    pub fn output_file(&self) -> Option<&str> {
        self.core
            .inputs
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
        self.core.state = AppState::Completed;
        self.core.results_view.clear();

        let summary = report.summarize();

        self.core.results_view.add_line(Line::from(Span::styled(
            format!("Compliance Report: {}", report.framework),
            Style::default().fg(tc!(success)),
        )));
        self.core.results_view.add_line(Line::from(""));
        self.core.results_view.add_line(Line::from(Span::styled(
            format!("Overall Score: {:.1}%", report.overall_score),
            Style::default().fg(tc!(warning)),
        )));
        self.core.results_view.add_line(Line::from(Span::styled(
            format!("Risk Level: {:?}", summary.risk_level),
            Style::default().fg(match summary.risk_level {
                eggsec::compliance::report::RiskLevel::Critical => tc!(error),
                eggsec::compliance::report::RiskLevel::High => tc!(error),
                eggsec::compliance::report::RiskLevel::Medium => tc!(warning),
                eggsec::compliance::report::RiskLevel::Low => tc!(success),
            }),
        )));
        self.core.results_view.add_line(Line::from(""));
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
        self.core.results_view.add_line(Line::from(format!(
            "Passed: {} | Failed: {} | N/A: {} | Review: {}",
            report.passed, report.failed, na_count, review_count
        )));
        self.core.results_view.add_line(Line::from(""));

        if !report.findings.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                "Findings:",
                Style::default().fg(tc!(warning)),
            )));
            for finding in &report.findings {
                self.core.results_view.add_line(Line::from(format!(
                    "  [{}] {} - {}",
                    finding.severity, finding.requirement_id, finding.description
                )));
            }
        }
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.core.state = AppState::Running;
            self.report = None;
            self.core.results_view.clear();
        }
    }

    pub fn stop(&mut self) {
        self.core.stop();
    }
}

impl Default for ComplianceTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ComplianceTab {
    tab_state_boilerplate!(ComplianceTab, core: core);

    fn has_selector_open(&self) -> bool {
        self.framework_selector.is_open()
    }

    fn reset(&mut self) {
        self.core.reset_all();
        self.report = None;
        self.framework_selector.select(0);
        self.focus_area = ComplianceFocusArea::Inputs;
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
        if let Some(ref err) = self.core.error {
            render_error_block(f, area, "Compliance - Error", err);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_inner = render_config_block(
            f,
            input_area,
            "Configuration",
            self.focus_area != ComplianceFocusArea::Results,
        );

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_inner);

        render_input_fields(f, &input_chunks, &self.core.inputs, insert_mode);

        let mut sel = self.framework_selector.clone();
        sel.focused = self.focus_area == ComplianceFocusArea::Framework;
        if let Some(framework_area) = input_chunks.get(2) {
            sel.render(f, *framework_area);
        }

        if self.core.state == AppState::Running {
            use ratatui::widgets::{Block, Borders, Gauge};
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(tc!(border)))
                        .title("Generating compliance report..."),
                )
                .gauge_style(Style::default().fg(tc!(warning)))
                .ratio(0.5);
            f.render_widget(gauge, results_area);
        } else if !self.core.results_view.is_empty() {
            self.core.results_view.render(f, results_area, None);
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
        self.focus_area = match self.focus_area {
            ComplianceFocusArea::Inputs => {
                self.core.inputs.blur();
                self.framework_selector.focus();
                ComplianceFocusArea::Framework
            }
            ComplianceFocusArea::Framework => {
                self.framework_selector.blur();
                ComplianceFocusArea::Results
            }
            ComplianceFocusArea::Results => {
                self.core.inputs.focus(0);
                ComplianceFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            ComplianceFocusArea::Inputs => {
                self.core.inputs.blur();
                ComplianceFocusArea::Results
            }
            ComplianceFocusArea::Framework => {
                self.framework_selector.blur();
                self.core.inputs.focus(0);
                ComplianceFocusArea::Inputs
            }
            ComplianceFocusArea::Results => {
                self.framework_selector.focus();
                ComplianceFocusArea::Framework
            }
        };
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        match self.focus_area {
            ComplianceFocusArea::Inputs => {
                if self.core.inputs.is_focused() {
                    self.core.inputs.blur();
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
        self.core.inputs.blur();
        self.framework_selector.blur();
        self.focus_area = ComplianceFocusArea::Inputs;
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            ComplianceFocusArea::Framework => {
                self.framework_selector.handle_up();
            }
            ComplianceFocusArea::Inputs => {
                self.core.inputs.focus_prev();
            }
            ComplianceFocusArea::Results => {
                self.core.results_view.scroll_up(1);
            }
        }
    }

    fn handle_down(&mut self) {
        match self.focus_area {
            ComplianceFocusArea::Framework => {
                self.framework_selector.handle_down();
            }
            ComplianceFocusArea::Inputs => {
                self.core.inputs.focus_next();
            }
            ComplianceFocusArea::Results => {
                self.core.results_view.scroll_down(1);
            }
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.is_at_left_edge()
        } else if self.focus_area == ComplianceFocusArea::Framework {
            self.framework_selector.items.is_empty()
                || self.framework_selector.selected == 0
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.is_at_right_edge()
        } else if self.focus_area == ComplianceFocusArea::Framework {
            self.framework_selector.items.is_empty()
                || self.framework_selector.selected
                    >= self.framework_selector.items.len().saturating_sub(1)
        } else {
            true
        }
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.get_focused_value()
        } else if self.focus_area == ComplianceFocusArea::Results {
            Some(self.core.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == ComplianceFocusArea::Inputs {
                self.core.inputs.move_home();
            } else if self.focus_area == ComplianceFocusArea::Results {
                self.core.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == ComplianceFocusArea::Inputs {
                self.core.inputs.move_end();
            } else if self.focus_area == ComplianceFocusArea::Results {
                self.core.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = ComplianceFocusArea::Inputs;
        self.core.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = ComplianceFocusArea::Results;
        self.core.inputs.blur();
    }

    fn page_up(&mut self, page_size: usize) {
        if !self.is_running() {
            self.core.results_view.page_up(page_size);
        }
    }

    fn page_down(&mut self, page_size: usize) {
        if !self.is_running() {
            self.core.results_view.page_down(page_size);
        }
    }

    fn stop(&mut self) {
        self.core.stop();
    }

    fn primary_target(&self) -> Option<String> {
        Some(self.target().to_string())
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() && self.focus_area == ComplianceFocusArea::Inputs {
            self.core.inputs.move_right()
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ComplianceFocusArea::Inputs && self.core.inputs.is_focused()
    }
}
