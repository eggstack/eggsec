use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, InputField, InputGroup, ScrollableText, Selector, SelectorItem,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use crate::workflow::finding::Finding;
use crate::workflow::finding::FindingStatus;
use crate::workflow::sla::calculate_sla;
use crate::workflow::WorkflowReport;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct WorkflowTab {
    pub inputs: InputGroup,
    pub report: Option<WorkflowReport>,
    pub findings: Vec<Finding>,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: WorkflowFocusArea,
    pub current_mode: WorkflowMode,
    pub mode_selector: Selector,
    pub severity_selector: Selector,
    pub status_selector: Selector,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkflowFocusArea {
    Mode,
    Inputs,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkflowMode {
    ListFindings,
    CreateFinding,
    AssignFinding,
    AddComment,
    ChangeStatus,
}

impl WorkflowTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Finding Title"))
            .add(InputField::new("Assignee"))
            .add(InputField::new("Comment"))
            .add(InputField::new("Finding ID"));

        let mode_selector = Selector::new("Mode").items(vec![
            SelectorItem::new("List Findings", "list"),
            SelectorItem::new("Create Finding", "create"),
            SelectorItem::new("Assign Finding", "assign"),
            SelectorItem::new("Add Comment", "comment"),
            SelectorItem::new("Change Status", "status"),
        ]);

        let severity_selector = Selector::new("Severity").items(vec![
            SelectorItem::new("Critical", "critical"),
            SelectorItem::new("High", "high"),
            SelectorItem::new("Medium", "medium"),
            SelectorItem::new("Low", "low"),
            SelectorItem::new("Info", "info"),
        ]);

        let status_selector = Selector::new("Status").items(vec![
            SelectorItem::new("Open", "open"),
            SelectorItem::new("In Progress", "in_progress"),
            SelectorItem::new("Resolved", "resolved"),
            SelectorItem::new("Verified", "verified"),
            SelectorItem::new("False Positive", "false_positive"),
        ]);

        Self {
            inputs,
            report: None,
            findings: Vec::new(),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: WorkflowFocusArea::Mode,
            current_mode: WorkflowMode::ListFindings,
            mode_selector,
            severity_selector,
            status_selector,
            error: None,
        }
    }

    pub fn title(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn assignee(&self) -> &str {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn comment(&self) -> &str {
        self.inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn finding_id(&self) -> &str {
        self.inputs
            .fields
            .get(3)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn selected_severity(&self) -> crate::types::Severity {
        match self.severity_selector.selected {
            0 => crate::types::Severity::Critical,
            1 => crate::types::Severity::High,
            2 => crate::types::Severity::Medium,
            3 => crate::types::Severity::Low,
            _ => crate::types::Severity::Info,
        }
    }

    pub fn selected_status(&self) -> FindingStatus {
        match self.status_selector.selected {
            0 => FindingStatus::Open,
            1 => FindingStatus::InProgress,
            2 => FindingStatus::Resolved,
            3 => FindingStatus::Verified,
            _ => FindingStatus::FalsePositive,
        }
    }

    pub fn get_mode(&self) -> &str {
        match self.current_mode {
            WorkflowMode::ListFindings => "list",
            WorkflowMode::CreateFinding => "create",
            WorkflowMode::AssignFinding => "assign",
            WorkflowMode::AddComment => "comment",
            WorkflowMode::ChangeStatus => "status",
        }
    }

    pub fn start(&mut self) {
        self.state = AppState::Running;
        self.results_view.clear();
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn set_findings(&mut self, findings: Vec<Finding>) {
        self.findings = findings.clone();
        self.state = AppState::Completed;
        self.results_view.clear();

        let mut report = WorkflowReport::new();
        report.total_findings = findings.len();
        report.open_findings = findings
            .iter()
            .filter(|f| matches!(f.status, FindingStatus::Open))
            .count();
        report.in_progress_findings = findings
            .iter()
            .filter(|f| matches!(f.status, FindingStatus::InProgress))
            .count();
        report.resolved_findings = findings
            .iter()
            .filter(|f| {
                matches!(f.status, FindingStatus::Resolved)
                    || matches!(f.status, FindingStatus::Verified)
            })
            .count();
        report.calculate_metrics();
        self.report = Some(report.clone());

        self.results_view.add_line(Line::from(Span::styled(
            "Workflow Summary",
            Style::default().fg(tc!(warning)),
        )));
        self.results_view.add_line(Line::from(format!(
            "Total: {} | Open: {} | In Progress: {} | Resolved: {} | SLA Violations: {}",
            report.total_findings,
            report.open_findings,
            report.in_progress_findings,
            report.resolved_findings,
            report.sla_violations,
        )));
        self.results_view.add_line(Line::from(""));

        if !findings.is_empty() {
            self.results_view.add_line(Line::from(Span::styled(
                "Findings:",
                Style::default().fg(tc!(success)),
            )));
            for f in &findings {
                let sla = calculate_sla(&f.id, f.severity, f.created_at);
                let sla_str = if sla.is_violated {
                    "SLA VIOLATED".to_string()
                } else {
                    format!("{}h remaining", sla.hours_remaining)
                };
                self.results_view.add_line(Line::from(format!(
                    "  [{}] {} - {:?} (assigned: {}) - {}",
                    f.severity,
                    f.title,
                    f.status,
                    f.assignee.as_deref().unwrap_or("unassigned"),
                    sla_str
                )));
            }
        }
    }

}

impl Default for WorkflowTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for WorkflowTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.focus_area = WorkflowFocusArea::Mode;
        self.current_mode = WorkflowMode::ListFindings;
        self.findings.clear();
        self.report = None;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.mode_selector.select(0);
        self.mode_selector.blur();
        self.severity_selector.select(0);
        self.severity_selector.blur();
        self.status_selector.select(0);
        self.status_selector.blur();
        self.inputs.blur();
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for WorkflowTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            WorkflowFocusArea::Mode => "Mode",
            WorkflowFocusArea::Inputs => "Inputs",
            WorkflowFocusArea::Results => "Results",
        };
        Some(vec!["Workflow", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let input_height = match self.current_mode {
            WorkflowMode::ListFindings => 6,
            WorkflowMode::CreateFinding => 9,
            WorkflowMode::AssignFinding => 9,
            WorkflowMode::AddComment => 9,
            WorkflowMode::ChangeStatus => 12,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(input_height), Constraint::Min(0)])
            .split(area);

        let Some(input_area) = chunks.get(0) else { return; };
        let Some(results_area) = chunks.get(1) else { return; };

        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(Style::default().fg(
                if self.focus_area != WorkflowFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        f.render_widget(config_block, input_area);

        let input_area = config_block.inner(input_area);

        let mut sel = self.mode_selector.clone();
        sel.focused = self.focus_area == WorkflowFocusArea::Mode;
        sel.render(f, input_area);

        let fields_area = Rect {
            y: input_area.y + 3,
            height: input_area.height.saturating_sub(3),
            ..input_area
        };

        let (fields, extra_slots) = match self.current_mode {
            WorkflowMode::ListFindings => (vec![], 0),
            WorkflowMode::CreateFinding => (vec![0], 1),
            WorkflowMode::AssignFinding => (vec![3, 1], 0),
            WorkflowMode::AddComment => (vec![3, 2], 0),
            WorkflowMode::ChangeStatus => (vec![3], 2),
        };

        let total_slots = fields.len() + extra_slots;
        let field_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3); total_slots])
            .split(fields_area);

        for (i, &idx) in fields.iter().enumerate() {
            if let Some(chunk) = field_chunks.get(i) {
                if idx < self.inputs.fields.len() {
                    self.inputs.fields[idx].render(f, *chunk, insert_mode);
                }
            }
        }

        if matches!(
            self.current_mode,
            WorkflowMode::CreateFinding | WorkflowMode::ChangeStatus
        ) {
            if let Some(chunk) = field_chunks.get(fields.len()) {
                let mut sev = self.severity_selector.clone();
                // These selectors are currently decorative only (no keyboard navigation)
                sev.focused = false;
                sev.render(f, *chunk);
            }
        }
        if matches!(self.current_mode, WorkflowMode::ChangeStatus) {
            if let Some(chunk) = field_chunks.get(fields.len() + 1) {
                let mut st = self.status_selector.clone();
                // These selectors are currently decorative only (no keyboard navigation)
                st.focused = false;
                st.render(f, *chunk);
            }
        }

        if self.state == AppState::Running {
            let gauge = ratatui::widgets::Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Processing..."),
                )
                .gauge_style(Style::default().fg(tc!(warning)))
                .ratio(0.5);
            f.render_widget(gauge, results_area);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_area, None);
        } else {
            let placeholder =
                empty_state_paragraph("Finding Management", "Select mode and press Enter");
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for WorkflowTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            WorkflowFocusArea::Mode => {
                self.mode_selector.blur();
                if self.current_mode == WorkflowMode::ListFindings {
                    WorkflowFocusArea::Results
                } else {
                    self.inputs.focus(0);
                    WorkflowFocusArea::Inputs
                }
            }
            WorkflowFocusArea::Inputs => {
                self.inputs.blur();
                WorkflowFocusArea::Results
            }
            WorkflowFocusArea::Results => {
                self.mode_selector.focus();
                WorkflowFocusArea::Mode
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            WorkflowFocusArea::Mode => {
                self.mode_selector.blur();
                WorkflowFocusArea::Results
            }
            WorkflowFocusArea::Inputs => {
                self.mode_selector.focus();
                WorkflowFocusArea::Mode
            }
            WorkflowFocusArea::Results => {
                if self.current_mode == WorkflowMode::ListFindings {
                    self.mode_selector.focus();
                    WorkflowFocusArea::Mode
                } else {
                    self.inputs.focus(0);
                    WorkflowFocusArea::Inputs
                }
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else if !self.is_running() && self.focus_area == WorkflowFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == WorkflowFocusArea::Inputs {
                self.inputs.move_home();
            } else if self.focus_area == WorkflowFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == WorkflowFocusArea::Inputs {
                self.inputs.move_end();
            } else if self.focus_area == WorkflowFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.inputs.blur();
            self.focus_area = WorkflowFocusArea::Mode;
            self.mode_selector.focus();
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = WorkflowFocusArea::Results;
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        match self.focus_area {
            WorkflowFocusArea::Mode => {
                let was_open = self.mode_selector.is_open();
                self.mode_selector.handle_enter();
                if !was_open {
                    return;
                }
                self.current_mode = match self.mode_selector.selected {
                    0 => WorkflowMode::ListFindings,
                    1 => WorkflowMode::CreateFinding,
                    2 => WorkflowMode::AssignFinding,
                    3 => WorkflowMode::AddComment,
                    _ => WorkflowMode::ChangeStatus,
                };
            }
            WorkflowFocusArea::Inputs => {
                self.inputs.blur();
            }
            WorkflowFocusArea::Results => {
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
        self.mode_selector.blur();
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                WorkflowFocusArea::Mode => self.mode_selector.handle_up(),
                WorkflowFocusArea::Inputs => self.inputs.focus_prev(),
                WorkflowFocusArea::Results => self.results_view.scroll_up(1),
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                WorkflowFocusArea::Mode => self.mode_selector.handle_down(),
                WorkflowFocusArea::Inputs => self.inputs.focus_next(),
                WorkflowFocusArea::Results => self.results_view.scroll_down(1),
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.inputs.move_right()
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            WorkflowFocusArea::Mode => {
                self.mode_selector.items.is_empty() || self.mode_selector.selected == 0
            }
            WorkflowFocusArea::Inputs => self.inputs.is_at_left_edge(),
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            WorkflowFocusArea::Mode => {
                self.mode_selector.items.is_empty()
                    || self.mode_selector.selected
                        >= self.mode_selector.items.len().saturating_sub(1)
            }
            WorkflowFocusArea::Inputs => self.inputs.is_at_right_edge(),
            _ => true,
        }
    }

    fn is_input_focused(&self) -> bool {
        (self.focus_area == WorkflowFocusArea::Mode && self.mode_selector.is_focused())
            || (self.focus_area == WorkflowFocusArea::Inputs && self.inputs.is_focused())
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
}
