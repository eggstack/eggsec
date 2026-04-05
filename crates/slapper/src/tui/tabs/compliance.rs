use crate::compliance::{ComplianceFramework, ComplianceReport};
use crate::tui::components::{InputField, InputGroup, ScrollableText, Selector, SelectorItem};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
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
            Style::default().fg(Color::Green),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Overall Score: {:.1}%", report.overall_score * 100.0),
            Style::default().fg(Color::Yellow),
        )));
        self.results_view.add_line(Line::from(Span::styled(
            format!("Risk Level: {:?}", summary.risk_level),
            Style::default().fg(match summary.risk_level {
                crate::compliance::report::RiskLevel::Critical => Color::Red,
                crate::compliance::report::RiskLevel::High => Color::Red,
                crate::compliance::report::RiskLevel::Medium => Color::Yellow,
                crate::compliance::report::RiskLevel::Low => Color::Green,
            }),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "Passed: {} | Failed: {} | N/A: {} | Review: {}",
            report.passed,
            report.failed,
            report.total_requirements - report.passed - report.failed,
            report.findings.len()
        )));
        self.results_view.add_line(Line::from(""));

        if !report.findings.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                "Findings:",
                Style::default().fg(Color::Yellow),
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

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
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
        for field in &mut self.inputs.fields {
            field.clear();
        }
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.results_view.add_line(Line::from(Span::styled(
            format!("Error: {}", msg),
            Style::default().fg(Color::Red),
        )));
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_area);

        for (i, field) in self.inputs.fields.iter().enumerate() {
            field.render(f, input_chunks[i], insert_mode);
        }

        let mut sel = self.framework_selector.clone();
        sel.focused = self.focus_area == ComplianceFocusArea::Framework;
        sel.render(f, input_chunks[2]);

        if self.state == AppState::Running {
            use ratatui::widgets::{Block, Borders, Gauge};
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Generating compliance report..."),
                )
                .gauge_style(Style::default().fg(Color::Yellow))
                .ratio(0.5);
            f.render_widget(gauge, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, Some(Color::Green));
        } else {
            let placeholder =
                ratatui::widgets::Paragraph::new("Enter target, select framework, and press Enter")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Compliance Reporting"),
                    )
                    .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for ComplianceTab {
    fn handle_focus_next(&mut self) {
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
        self.focus_area = match self.focus_area {
            ComplianceFocusArea::Inputs => ComplianceFocusArea::Results,
            ComplianceFocusArea::Framework => {
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
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_enter(&mut self) {
        match self.focus_area {
            ComplianceFocusArea::Inputs => {
                self.inputs.blur();
            }
            ComplianceFocusArea::Framework => {
                self.framework_selector.handle_enter();
            }
            ComplianceFocusArea::Results => {}
        }

        if self.is_running() {
            self.stop();
        } else if !self.target().is_empty() {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
        self.framework_selector.blur();
    }

    fn handle_up(&mut self) {
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

    fn handle_down(&mut self) {
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

    fn handle_left(&mut self) -> bool {
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.move_right()
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ComplianceFocusArea::Inputs {
            self.inputs.fields[0].cursor_pos == 0
        } else if self.focus_area == ComplianceFocusArea::Framework {
            self.framework_selector.selected == 0
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ComplianceFocusArea::Inputs {
            let field = &self.inputs.fields[0];
            field.cursor_pos >= field.value.len()
        } else if self.focus_area == ComplianceFocusArea::Framework {
            self.framework_selector.selected
                >= self.framework_selector.items.len().saturating_sub(1)
        } else {
            true
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ComplianceFocusArea::Inputs && self.inputs.is_focused()
    }
}
