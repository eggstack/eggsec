use crate::tc;
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{
    empty_state_paragraph, InputField, InputGroup, ScrollableText, Selector, SelectorItem,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use crate::vuln::{
    AssetCriticality, CvssScore, ExploitInfo, PrioritizedFinding, Remediation, TriageResult,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct VulnTab {
    pub inputs: InputGroup,
    pub mode_selector: Selector,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: VulnFocusArea,
    pub current_mode: VulnMode,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VulnFocusArea {
    Mode,
    Inputs,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VulnMode {
    CvssCalc,
    ExploitCheck,
    AssetAssess,
    Prioritize,
    Triage,
    Remediation,
}

impl VulnTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("CVE ID / Finding ID"))
            .add(InputField::new("Title"))
            .add(InputField::new("Description"))
            .add(InputField::new(
                "CVSS Vector (e.g. CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H)",
            ))
            .add(InputField::new(
                "Asset Type (database/web_server/api/workstation)",
            ))
            .add(InputField::new("Severity").with_value("high"));

        let mode_selector = Selector::new("Mode").items(vec![
            SelectorItem::new("CVSS Calculator", "cvss"),
            SelectorItem::new("Exploit Check", "exploit"),
            SelectorItem::new("Asset Assessment", "asset"),
            SelectorItem::new("Prioritize Findings", "prioritize"),
            SelectorItem::new("Triage", "triage"),
            SelectorItem::new("Remediation Plan", "remediation"),
        ]);

        Self {
            inputs,
            mode_selector,
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: VulnFocusArea::Mode,
            current_mode: VulnMode::CvssCalc,
            error: None,
        }
    }

    pub fn cve_id(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn title(&self) -> &str {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn description(&self) -> &str {
        self.inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn cvss_vector(&self) -> &str {
        self.inputs
            .fields
            .get(3)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn asset_type(&self) -> &str {
        self.inputs
            .fields
            .get(4)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn severity_str(&self) -> &str {
        self.inputs
            .fields
            .get(5)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn get_mode(&self) -> &str {
        match self.current_mode {
            VulnMode::CvssCalc => "cvss_calc",
            VulnMode::ExploitCheck => "exploit_check",
            VulnMode::AssetAssess => "asset_assess",
            VulnMode::Prioritize => "prioritize",
            VulnMode::Triage => "triage",
            VulnMode::Remediation => "remediation",
        }
    }

    pub fn start(&mut self) {
        self.state = AppState::Running;
        self.results_view.clear();
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn display_cvss(&mut self, vector: &str) {
        self.state = AppState::Completed;
        self.results_view.clear();

        match CvssScore::from_vector(vector) {
            Ok(cvss) => {
                self.results_view.add_line(Line::from(Span::styled(
                    "CVSS 3.1 Score",
                    Style::default().fg(tc!(accent)),
                )));
                self.results_view.add_line(Line::from(""));
                self.results_view.add_line(Line::from(format!(
                    "  Base Score:       {:.1}",
                    cvss.base_score
                )));
                self.results_view.add_line(Line::from(format!(
                    "  Temporal Score:   {:.1}",
                    cvss.temporal_score
                )));
                self.results_view.add_line(Line::from(format!(
                    "  Environmental:    {:.1}",
                    cvss.environmental_score
                )));
                self.results_view
                    .add_line(Line::from(format!("  Vector:           {}", cvss.vector)));
            }
            Err(e) => {
                self.results_view.add_line(Line::from(Span::styled(
                    format!("Invalid CVSS vector: {}", e),
                    Style::default().fg(tc!(error)),
                )));
            }
        }
    }

    pub fn display_exploit_info(&mut self, cve_id: &str, info: ExploitInfo) {
        self.state = AppState::Completed;
        self.results_view.clear();
        self.results_view.add_line(Line::from(Span::styled(
            format!("Exploitability: {}", cve_id),
            Style::default().fg(tc!(accent)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "  Public Exploit:    {}",
            if info.has_public_exploit { "Yes" } else { "No" }
        )));
        self.results_view.add_line(Line::from(format!(
            "  Exploit-DB:        {}",
            info.exploit_db_id.as_deref().unwrap_or("N/A")
        )));
        self.results_view.add_line(Line::from(format!(
            "  Metasploit:        {}",
            info.metasploit_module.as_deref().unwrap_or("N/A")
        )));
        self.results_view.add_line(Line::from(format!(
            "  CISA KEV:          {}",
            if info.in_cisa_kev { "Yes" } else { "No" }
        )));
        self.results_view.add_line(Line::from(format!(
            "  Actively Exploited: {}",
            if info.is_actively_exploited {
                "Yes"
            } else {
                "No"
            }
        )));
        self.results_view.add_line(Line::from(format!(
            "  Exploit Score:     {:.1}",
            info.exploit_score
        )));
    }

    pub fn display_asset(&mut self, asset: AssetCriticality) {
        self.state = AppState::Completed;
        self.results_view.clear();
        self.results_view.add_line(Line::from(Span::styled(
            format!("Asset Assessment: {}", asset.asset_id),
            Style::default().fg(tc!(accent)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "  Technology Score:  {:.1}",
            asset.technology_score
        )));
        self.results_view.add_line(Line::from(format!(
            "  Environment Score: {:.1}",
            asset.environment_score
        )));
        self.results_view.add_line(Line::from(format!(
            "  Data Sensitivity:  {:.1}",
            asset.data_sensitivity
        )));
        self.results_view.add_line(Line::from(format!(
            "  User Base:         {:.1}",
            asset.user_base
        )));
        self.results_view.add_line(Line::from(format!(
            "  Overall Score:     {:.1}",
            asset.overall_score
        )));
    }

    pub fn display_prioritized(&mut self, findings: Vec<PrioritizedFinding>) {
        self.state = AppState::Completed;
        self.results_view.clear();
        self.results_view.add_line(Line::from(Span::styled(
            format!("Prioritized Findings ({}):", findings.len()),
            Style::default().fg(tc!(accent)),
        )));
        self.results_view.add_line(Line::from(""));
        for f in &findings {
            self.results_view.add_line(Line::from(format!(
                "  #{} [{}] {} - Risk: {:.1} ({:?})",
                f.priority_rank,
                f.severity,
                f.title,
                f.risk_score.combined_score,
                f.risk_score.priority_level
            )));
        }
    }

    pub fn display_triage(&mut self, result: TriageResult) {
        self.state = AppState::Completed;
        self.results_view.clear();
        self.results_view.add_line(Line::from(Span::styled(
            format!("Triage: {}", result.finding_id),
            Style::default().fg(tc!(accent)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "  Status:     {:?}",
            result.triage_status
        )));
        self.results_view.add_line(Line::from(format!(
            "  Confidence: {:.0}%",
            result.confidence * 100.0
        )));
        self.results_view
            .add_line(Line::from(format!("  Reason:     {}", result.reason)));
    }

    pub fn display_remediation(&mut self, remediation: Remediation) {
        self.state = AppState::Completed;
        self.results_view.clear();
        self.results_view.add_line(Line::from(Span::styled(
            format!("Remediation: {}", remediation.title),
            Style::default().fg(tc!(accent)),
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(format!(
            "  Priority:      {:?}",
            remediation.priority
        )));
        self.results_view.add_line(Line::from(format!(
            "  Effort:        {:.1} hours",
            remediation.effort_hours
        )));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(Span::styled(
            "Steps:",
            Style::default().fg(tc!(info)),
        )));
        for (i, step) in remediation.steps.iter().enumerate() {
            self.results_view
                .add_line(Line::from(format!("  {}. {}", i + 1, step)));
        }
        if !remediation.references.is_empty() {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(Span::styled(
                "References:",
                Style::default().fg(tc!(info)),
            )));
            for r in &remediation.references {
                self.results_view.add_line(Line::from(format!("  - {}", r)));
            }
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.results_view.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.results_view.page_down(page_size);
    }
}

impl Default for VulnTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for VulnTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.mode_selector.select(0);
        self.focus_area = VulnFocusArea::Mode;
        self.current_mode = VulnMode::CvssCalc;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for VulnTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let focus = match self.focus_area {
            VulnFocusArea::Mode => "Mode",
            VulnFocusArea::Inputs => "Inputs",
            VulnFocusArea::Results => "Results",
        };
        Some(vec!["Vuln", focus])
    }

    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let input_height = match self.current_mode {
            VulnMode::CvssCalc => 9,
            VulnMode::ExploitCheck => 6,
            VulnMode::AssetAssess => 12,
            VulnMode::Prioritize => 9,
            VulnMode::Triage => 15,
            VulnMode::Remediation => 9,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(input_height), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let config_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(Style::default().fg(
                if self.focus_area != VulnFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                },
            ));
        f.render_widget(config_block, input_area);

        let input_area = config_block.inner(input_area);

        let mut sel = self.mode_selector.clone();
        sel.focused = self.focus_area == VulnFocusArea::Mode;
        sel.render(f, input_area);

        let fields_area = Rect {
            y: input_area.y + 3,
            height: input_area.height - 3,
            ..input_area
        };

        let field_indices: Vec<usize> = match self.current_mode {
            VulnMode::CvssCalc => vec![3],
            VulnMode::ExploitCheck => vec![0],
            VulnMode::AssetAssess => vec![4],
            VulnMode::Prioritize => vec![1, 5],
            VulnMode::Triage => vec![0, 1, 2, 3, 5],
            VulnMode::Remediation => vec![1, 5],
        };

        if !field_indices.is_empty() {
            let field_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(3); field_indices.len()])
                .split(fields_area);

            for (i, &idx) in field_indices.iter().enumerate() {
                if idx < self.inputs.fields.len() {
                    if let Some(chunk) = field_chunks.get(i) {
                        if let Some(field) = self.inputs.fields.get(idx) {
                            field.render(f, *chunk, insert_mode);
                        }
                    }
                }
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
            let placeholder = empty_state_paragraph(
                "Vulnerability Prioritization",
                "Select mode and press Enter",
            );
            f.render_widget(placeholder, results_area);
        }
    }
}

impl TabInput for VulnTab {
    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            VulnFocusArea::Mode => {
                self.mode_selector.blur();
                self.inputs.focus(0);
                VulnFocusArea::Inputs
            }
            VulnFocusArea::Inputs => {
                self.inputs.blur();
                VulnFocusArea::Results
            }
            VulnFocusArea::Results => {
                self.mode_selector.focus();
                VulnFocusArea::Mode
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            VulnFocusArea::Mode => VulnFocusArea::Results,
            VulnFocusArea::Inputs => {
                self.mode_selector.focus();
                VulnFocusArea::Mode
            }
            VulnFocusArea::Results => {
                self.inputs.focus(0);
                VulnFocusArea::Inputs
            }
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == VulnFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == VulnFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == VulnFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == VulnFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else if self.focus_area == VulnFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() && self.focus_area == VulnFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() && self.focus_area == VulnFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == VulnFocusArea::Inputs {
                self.inputs.move_home();
            } else if self.focus_area == VulnFocusArea::Results {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == VulnFocusArea::Inputs {
                self.inputs.move_end();
            } else if self.focus_area == VulnFocusArea::Results {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = VulnFocusArea::Inputs;
            self.inputs.focus(0);
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = VulnFocusArea::Results;
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        match self.focus_area {
            VulnFocusArea::Mode => {
                let was_open = self.mode_selector.is_open();
                self.mode_selector.handle_enter();
                if !was_open {
                    return;
                }
                self.current_mode = match self.mode_selector.selected {
                    0 => VulnMode::CvssCalc,
                    1 => VulnMode::ExploitCheck,
                    2 => VulnMode::AssetAssess,
                    3 => VulnMode::Prioritize,
                    4 => VulnMode::Triage,
                    _ => VulnMode::Remediation,
                };
            }
            VulnFocusArea::Inputs => {
                self.inputs.blur();
            }
            VulnFocusArea::Results => {
                return;
            }
        }
        if !self.is_running() {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            return;
        }
        self.mode_selector.blur();
        self.inputs.blur();
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                VulnFocusArea::Mode => self.mode_selector.handle_up(),
                VulnFocusArea::Inputs => self.inputs.focus_prev(),
                VulnFocusArea::Results => self.results_view.scroll_up(1),
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            match self.focus_area {
                VulnFocusArea::Mode => self.mode_selector.handle_down(),
                VulnFocusArea::Inputs => self.inputs.focus_next(),
                VulnFocusArea::Results => self.results_view.scroll_down(1),
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() && self.focus_area == VulnFocusArea::Inputs {
            self.inputs.move_left()
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() && self.focus_area == VulnFocusArea::Inputs {
            self.inputs.move_right()
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            VulnFocusArea::Mode => {
                self.mode_selector.items.is_empty() || self.mode_selector.selected == 0
            }
            VulnFocusArea::Inputs => {
                if let Some(idx) = self.inputs.focused {
                    self.inputs
                        .fields
                        .get(idx)
                        .map(|f| f.cursor_pos == 0)
                        .unwrap_or(true)
                } else {
                    true
                }
            }
            _ => true,
        }
    }

    fn is_at_right_edge(&self) -> bool {
        match self.focus_area {
            VulnFocusArea::Mode => {
                self.mode_selector.items.is_empty()
                    || self.mode_selector.selected
                        >= self.mode_selector.items.len().saturating_sub(1)
            }
            VulnFocusArea::Inputs => {
                if let Some(idx) = self.inputs.focused {
                    self.inputs
                        .fields
                        .get(idx)
                        .map(|f| f.cursor_pos >= f.value.len())
                        .unwrap_or(true)
                } else {
                    true
                }
            }
            _ => true,
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == VulnFocusArea::Inputs && self.inputs.is_focused()
    }
}
