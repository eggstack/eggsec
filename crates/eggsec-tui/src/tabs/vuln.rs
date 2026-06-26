use crate::app::tab_error::TabError;
use crate::components::{empty_state_paragraph, InputField, Selector, SelectorItem};
use crate::tabs::core::TabCore;
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::{tab_input_custom, tab_state_boilerplate, tc};
use eggsec::vuln::{
    AssetCriticality, CvssScore, ExploitInfo, PrioritizedFinding, Remediation, TriageResult,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct VulnTab {
    pub core: TabCore,
    pub mode_selector: Selector,
    pub focus_area: VulnFocusArea,
    pub current_mode: VulnMode,
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
    fn first_visible_field(&self) -> usize {
        match self.current_mode {
            VulnMode::CvssCalc => 3,
            VulnMode::ExploitCheck => 0,
            VulnMode::AssetAssess => 4,
            VulnMode::Prioritize => 1,
            VulnMode::Triage => 0,
            VulnMode::Remediation => 1,
        }
    }

    pub fn new() -> Self {
        let inputs = crate::components::InputGroup::new()
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
            core: TabCore::new("Analyzing...", "Results").with_inputs(inputs),
            mode_selector,
            focus_area: VulnFocusArea::Mode,
            current_mode: VulnMode::CvssCalc,
        }
    }

    pub fn cve_id(&self) -> &str {
        crate::tabs::core::field_str(&self.core, 0)
    }

    pub fn title(&self) -> &str {
        crate::tabs::core::field_str(&self.core, 1)
    }

    pub fn description(&self) -> &str {
        crate::tabs::core::field_str(&self.core, 2)
    }

    pub fn cvss_vector(&self) -> &str {
        crate::tabs::core::field_str(&self.core, 3)
    }

    pub fn asset_type(&self) -> &str {
        crate::tabs::core::field_str(&self.core, 4)
    }

    pub fn severity_str(&self) -> &str {
        crate::tabs::core::field_str(&self.core, 5)
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
        self.core.state = AppState::Running;
        self.core.results_view.clear();
    }

    pub fn display_cvss(&mut self, vector: &str) {
        let view = self.core.prepare_results();

        match CvssScore::from_vector(vector) {
            Ok(cvss) => {
                view.add_line(Line::from(Span::styled(
                    "CVSS 3.1 Score",
                    Style::default().fg(tc!(accent)),
                )));
                view.add_line(Line::from(""));
                view.add_line(Line::from(format!(
                    "  Base Score:       {:.1}",
                    cvss.base_score
                )));
                view.add_line(Line::from(format!(
                    "  Temporal Score:   {:.1}",
                    cvss.temporal_score
                )));
                view.add_line(Line::from(format!(
                    "  Environmental:    {:.1}",
                    cvss.environmental_score
                )));
                view.add_line(Line::from(format!("  Vector:           {}", cvss.vector)));
            }
            Err(e) => {
                view.add_line(Line::from(Span::styled(
                    format!("Invalid CVSS vector: {}", e),
                    Style::default().fg(tc!(error)),
                )));
            }
        }
    }

    pub fn display_exploit_info(&mut self, cve_id: &str, info: ExploitInfo) {
        let view = self.core.prepare_results();
        view.add_line(Line::from(Span::styled(
            format!("Exploitability: {}", cve_id),
            Style::default().fg(tc!(accent)),
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(format!(
            "  Public Exploit:    {}",
            if info.has_public_exploit { "Yes" } else { "No" }
        )));
        view.add_line(Line::from(format!(
            "  Exploit-DB:        {}",
            info.exploit_db_id.as_deref().unwrap_or("N/A")
        )));
        view.add_line(Line::from(format!(
            "  Metasploit:        {}",
            info.metasploit_module.as_deref().unwrap_or("N/A")
        )));
        view.add_line(Line::from(format!(
            "  CISA KEV:          {}",
            if info.in_cisa_kev { "Yes" } else { "No" }
        )));
        view.add_line(Line::from(format!(
            "  Actively Exploited: {}",
            if info.is_actively_exploited {
                "Yes"
            } else {
                "No"
            }
        )));
        view.add_line(Line::from(format!(
            "  Exploit Score:     {:.1}",
            info.exploit_score
        )));
    }

    pub fn display_asset(&mut self, asset: AssetCriticality) {
        let view = self.core.prepare_results();
        view.add_line(Line::from(Span::styled(
            format!("Asset Assessment: {}", asset.asset_id),
            Style::default().fg(tc!(accent)),
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(format!(
            "  Technology Score:  {:.1}",
            asset.technology_score
        )));
        view.add_line(Line::from(format!(
            "  Environment Score: {:.1}",
            asset.environment_score
        )));
        view.add_line(Line::from(format!(
            "  Data Sensitivity:  {:.1}",
            asset.data_sensitivity
        )));
        view.add_line(Line::from(format!(
            "  User Base:         {:.1}",
            asset.user_base
        )));
        view.add_line(Line::from(format!(
            "  Overall Score:     {:.1}",
            asset.overall_score
        )));
    }

    pub fn display_prioritized(&mut self, findings: Vec<PrioritizedFinding>) {
        let view = self.core.prepare_results();
        view.add_line(Line::from(Span::styled(
            format!("Prioritized Findings ({}):", findings.len()),
            Style::default().fg(tc!(accent)),
        )));
        view.add_line(Line::from(""));
        for f in &findings {
            view.add_line(Line::from(format!(
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
        let view = self.core.prepare_results();
        view.add_line(Line::from(Span::styled(
            format!("Triage: {}", result.finding_id),
            Style::default().fg(tc!(accent)),
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(format!(
            "  Status:     {:?}",
            result.triage_status
        )));
        view.add_line(Line::from(format!(
            "  Confidence: {:.0}%",
            result.confidence * 100.0
        )));
        view.add_line(Line::from(format!("  Reason:     {}", result.reason)));
    }

    pub fn display_remediation(&mut self, remediation: Remediation) {
        let view = self.core.prepare_results();
        view.add_line(Line::from(Span::styled(
            format!("Remediation: {}", remediation.title),
            Style::default().fg(tc!(accent)),
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(format!(
            "  Priority:      {:?}",
            remediation.priority
        )));
        view.add_line(Line::from(format!(
            "  Effort:        {:.1} hours",
            remediation.effort_hours
        )));
        view.add_line(Line::from(""));
        view.add_line(Line::from(Span::styled(
            "Steps:",
            Style::default().fg(tc!(info)),
        )));
        for (i, step) in remediation.steps.iter().enumerate() {
            view.add_line(Line::from(format!("  {}. {}", i + 1, step)));
        }
        if !remediation.references.is_empty() {
            view.add_line(Line::from(""));
            view.add_line(Line::from(Span::styled(
                "References:",
                Style::default().fg(tc!(info)),
            )));
            for r in &remediation.references {
                view.add_line(Line::from(format!("  - {}", r)));
            }
        }
    }
}

impl Default for VulnTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for VulnTab {
    tab_state_boilerplate!(VulnTab, core: core);

    fn progress(&self) -> f64 {
        0.0
    }

    fn has_selector_open(&self) -> bool {
        self.mode_selector.is_open()
    }

    fn reset(&mut self) {
        self.core.reset_all();
        self.mode_selector.select(0);
        self.mode_selector.blur();
        self.core.inputs.blur();
        self.focus_area = VulnFocusArea::Mode;
        self.current_mode = VulnMode::CvssCalc;
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

        let Some(input_area) = chunks.get(0) else {
            return;
        };
        let Some(results_area) = chunks.get(1) else {
            return;
        };

        let input_inner = crate::tabs::core::render_config_block(
            f,
            *input_area,
            "Configuration",
            self.focus_area != VulnFocusArea::Results,
        );

        let mut sel = self.mode_selector.clone();
        sel.focused = self.focus_area == VulnFocusArea::Mode;
        sel.render(f, input_inner);

        let fields_area = Rect {
            y: input_inner.y + 3,
            height: input_inner.height.saturating_sub(3),
            ..input_inner
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
                if idx < self.core.inputs.fields.len() {
                    if let Some(chunk) = field_chunks.get(i) {
                        if let Some(field) = self.core.inputs.fields.get(idx) {
                            field.render(f, *chunk, insert_mode);
                        }
                    }
                }
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
            let placeholder = empty_state_paragraph(
                "Vulnerability Prioritization",
                "Select mode and press Enter",
            );
            f.render_widget(placeholder, *results_area);
        }
    }
}

impl TabInput for VulnTab {
    tab_input_custom!(
        VulnTab,
        core: core,
        focus: focus_area,
        Inputs: VulnFocusArea::Inputs,
        Results: VulnFocusArea::Results
    );

    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            VulnFocusArea::Mode => {
                self.mode_selector.blur();
                self.core.inputs.focus(self.first_visible_field());
                VulnFocusArea::Inputs
            }
            VulnFocusArea::Inputs => {
                self.core.inputs.blur();
                VulnFocusArea::Results
            }
            VulnFocusArea::Results => {
                self.mode_selector.focus();
                VulnFocusArea::Mode
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            VulnFocusArea::Mode => {
                self.mode_selector.blur();
                VulnFocusArea::Results
            }
            VulnFocusArea::Inputs => {
                self.core.inputs.blur();
                self.mode_selector.focus();
                VulnFocusArea::Mode
            }
            VulnFocusArea::Results => {
                self.core.inputs.focus(self.first_visible_field());
                VulnFocusArea::Inputs
            }
        };
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.core.inputs.blur();
            self.focus_area = VulnFocusArea::Mode;
            self.mode_selector.focus();
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.mode_selector.blur();
            self.core.inputs.blur();
            self.focus_area = VulnFocusArea::Results;
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.core.stop();
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
                self.core.inputs.blur();
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
            self.core.stop();
            return;
        }
        self.mode_selector.blur();
        self.core.inputs.blur();
        self.focus_area = VulnFocusArea::Mode;
    }

    fn handle_up(&mut self) {
        match self.focus_area {
            VulnFocusArea::Mode => self.mode_selector.handle_up(),
            VulnFocusArea::Inputs => self.core.inputs.focus_prev(),
            VulnFocusArea::Results => self.core.results_view.scroll_up(1),
        }
    }

    fn handle_down(&mut self) {
        match self.focus_area {
            VulnFocusArea::Mode => self.mode_selector.handle_down(),
            VulnFocusArea::Inputs => self.core.inputs.focus_next(),
            VulnFocusArea::Results => self.core.results_view.scroll_down(1),
        }
    }

    fn is_at_left_edge(&self) -> bool {
        match self.focus_area {
            VulnFocusArea::Mode => {
                self.mode_selector.items.is_empty() || self.mode_selector.selected == 0
            }
            VulnFocusArea::Inputs => self.core.inputs.is_at_left_edge(),
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
            VulnFocusArea::Inputs => self.core.inputs.is_at_right_edge(),
            _ => true,
        }
    }
}
