use crate::components::{empty_state_paragraph, Selector, SelectorItem};
use crate::tabs::core::{render_error_block, TabCore};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_state_boilerplate, tc};
use eggsec::workflow::finding::Finding;
use eggsec::workflow::finding::FindingStatus;
use eggsec::workflow::sla::calculate_sla;
use eggsec::workflow::WorkflowReport;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct WorkflowTab {
    pub core: TabCore,
    pub report: Option<WorkflowReport>,
    pub findings: Vec<Finding>,
    pub focus_area: WorkflowFocusArea,
    pub current_mode: WorkflowMode,
    pub mode_selector: Selector,
    pub severity_selector: Selector,
    pub status_selector: Selector,
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
        let inputs = crate::components::InputGroup::new()
            .add(crate::components::InputField::new("Finding Title"))
            .add(crate::components::InputField::new("Assignee"))
            .add(crate::components::InputField::new("Comment"))
            .add(crate::components::InputField::new("Finding ID"));

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
            core: TabCore::new("Finding Management", "Results").with_inputs(inputs),
            report: None,
            findings: Vec::new(),
            focus_area: WorkflowFocusArea::Mode,
            current_mode: WorkflowMode::ListFindings,
            mode_selector,
            severity_selector,
            status_selector,
        }
    }

    pub fn title(&self) -> &str {
        self.core
            .inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn assignee(&self) -> &str {
        self.core
            .inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn comment(&self) -> &str {
        self.core
            .inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn finding_id(&self) -> &str {
        self.core
            .inputs
            .fields
            .get(3)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn selected_severity(&self) -> eggsec::types::Severity {
        match self.severity_selector.selected {
            0 => eggsec::types::Severity::Critical,
            1 => eggsec::types::Severity::High,
            2 => eggsec::types::Severity::Medium,
            3 => eggsec::types::Severity::Low,
            _ => eggsec::types::Severity::Info,
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
        self.core.state = AppState::Running;
        self.core.results_view.clear();
    }

    pub fn set_findings(&mut self, findings: Vec<Finding>) {
        self.findings = findings.clone();
        self.core.state = AppState::Completed;
        self.core.results_view.clear();

        let mut report = WorkflowReport::new();
        report.findings = findings;
        report.calculate_metrics();
        self.report = Some(report.clone());

        self.core.results_view.add_line(Line::from(Span::styled(
            "Workflow Summary",
            Style::default().fg(tc!(warning)),
        )));
        self.core.results_view.add_line(Line::from(format!(
            "Total: {} | Open: {} | In Progress: {} | Resolved: {} | SLA Violations: {}",
            report.total_findings,
            report.open_findings,
            report.in_progress_findings,
            report.resolved_findings,
            report.sla_violations,
        )));
        self.core.results_view.add_line(Line::from(""));

        if !report.findings.is_empty() {
            self.core.results_view.add_line(Line::from(Span::styled(
                "Findings:",
                Style::default().fg(tc!(success)),
            )));
            for f in &report.findings {
                let sla = calculate_sla(&f.id, f.severity, f.created_at);
                let sla_str = if sla.is_violated {
                    "SLA VIOLATED".to_string()
                } else {
                    format!("{}h remaining", sla.hours_remaining)
                };
                self.core.results_view.add_line(Line::from(format!(
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
    tab_state_boilerplate!(WorkflowTab, core: core);

    fn has_selector_open(&self) -> bool {
        self.mode_selector.is_open()
            || self.severity_selector.is_open()
            || self.status_selector.is_open()
    }

    fn reset(&mut self) {
        self.core.reset_all();
        self.focus_area = WorkflowFocusArea::Mode;
        self.current_mode = WorkflowMode::ListFindings;
        self.findings.clear();
        self.report = None;
        self.mode_selector.select(0);
        self.mode_selector.blur();
        self.severity_selector.select(0);
        self.severity_selector.blur();
        self.status_selector.select(0);
        self.status_selector.blur();
        self.core.inputs.blur();
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

        let Some(input_area) = chunks.get(0) else {
            return;
        };
        let Some(results_area) = chunks.get(1) else {
            return;
        };

        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(
                Style::default().fg(if self.focus_area != WorkflowFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let inner_area = config_block.inner(*input_area);
        f.render_widget(config_block, *input_area);

        let mut sel = self.mode_selector.clone();
        sel.focused = self.focus_area == WorkflowFocusArea::Mode;
        sel.render(f, inner_area);

        let fields_area = Rect {
            y: inner_area.y + 3,
            height: inner_area.height.saturating_sub(3),
            ..inner_area
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
                if idx < self.core.inputs.fields.len() {
                    self.core.inputs.fields[idx].render(f, *chunk, insert_mode);
                }
            }
        }

        if matches!(
            self.current_mode,
            WorkflowMode::CreateFinding | WorkflowMode::ChangeStatus
        ) {
            if let Some(chunk) = field_chunks.get(fields.len()) {
                let mut sev = self.severity_selector.clone();
                sev.focused = false;
                sev.render(f, *chunk);
            }
        }
        if matches!(self.current_mode, WorkflowMode::ChangeStatus) {
            if let Some(chunk) = field_chunks.get(fields.len() + 1) {
                let mut st = self.status_selector.clone();
                st.focused = false;
                st.render(f, *chunk);
            }
        }

        if self.core.state == AppState::Running {
            let gauge = ratatui::widgets::Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(tc!(border)))
                        .title("Processing..."),
                )
                .gauge_style(Style::default().fg(tc!(warning)))
                .ratio(0.5);
            f.render_widget(gauge, *results_area);
        } else if !self.core.results_view.is_empty() {
            self.core.results_view.render(f, *results_area, None);
        } else {
            let placeholder =
                empty_state_paragraph("Finding Management", "Select mode and press Enter");
            f.render_widget(placeholder, *results_area);
        }
    }
}

impl TabInput for WorkflowTab {
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            WorkflowFocusArea::Mode => {
                self.mode_selector.blur();
                if self.current_mode == WorkflowMode::ListFindings {
                    WorkflowFocusArea::Results
                } else {
                    self.core.inputs.focus(0);
                    WorkflowFocusArea::Inputs
                }
            }
            WorkflowFocusArea::Inputs => {
                self.core.inputs.blur();
                WorkflowFocusArea::Results
            }
            WorkflowFocusArea::Results => {
                self.mode_selector.focus();
                WorkflowFocusArea::Mode
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            WorkflowFocusArea::Mode => {
                self.mode_selector.blur();
                WorkflowFocusArea::Results
            }
            WorkflowFocusArea::Inputs => {
                self.core.inputs.blur();
                self.mode_selector.focus();
                WorkflowFocusArea::Mode
            }
            WorkflowFocusArea::Results => {
                if self.current_mode == WorkflowMode::ListFindings {
                    self.mode_selector.focus();
                    WorkflowFocusArea::Mode
                } else {
                    self.core.inputs.focus(0);
                    WorkflowFocusArea::Inputs
                }
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.core.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.core.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.core.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.core.inputs.get_focused_value()
        } else if !self.is_running() && self.focus_area == WorkflowFocusArea::Results {
            Some(self.core.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.core.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.core.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == WorkflowFocusArea::Inputs {
                self.core.inputs.move_home();
            } else if self.focus_area == WorkflowFocusArea::Results {
                self.core.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == WorkflowFocusArea::Inputs {
                self.core.inputs.move_end();
            } else if self.focus_area == WorkflowFocusArea::Results {
                self.core.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.core.inputs.blur();
            self.focus_area = WorkflowFocusArea::Mode;
            self.mode_selector.focus();
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.mode_selector.blur();
            self.core.inputs.blur();
            self.focus_area = WorkflowFocusArea::Results;
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.core.stop();
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
                self.core.inputs.blur();
            }
            WorkflowFocusArea::Results => {
                return;
            }
        }
        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.core.stop();
            return;
        }
        self.mode_selector.blur();
        self.core.inputs.blur();
        self.focus_area = WorkflowFocusArea::Mode;
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            WorkflowFocusArea::Mode => self.mode_selector.handle_up(),
            WorkflowFocusArea::Inputs => self.core.inputs.focus_prev(),
            WorkflowFocusArea::Results => self.core.results_view.scroll_up(1),
        }
    }

    fn handle_down(&mut self) {
        match self.focus_area {
            WorkflowFocusArea::Mode => self.mode_selector.handle_down(),
            WorkflowFocusArea::Inputs => self.core.inputs.focus_next(),
            WorkflowFocusArea::Results => self.core.results_view.scroll_down(1),
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.core.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() && self.focus_area == WorkflowFocusArea::Inputs {
            self.core.inputs.move_right()
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            WorkflowFocusArea::Mode => {
                self.mode_selector.items.is_empty() || self.mode_selector.selected == 0
            }
            WorkflowFocusArea::Inputs => self.core.inputs.is_at_left_edge(),
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
            WorkflowFocusArea::Inputs => self.core.inputs.is_at_right_edge(),
            _ => true,
        }
    }

    fn is_input_focused(&self) -> bool {
        (self.focus_area == WorkflowFocusArea::Mode && self.mode_selector.is_focused())
            || (self.focus_area == WorkflowFocusArea::Inputs && self.core.inputs.is_focused())
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
}
