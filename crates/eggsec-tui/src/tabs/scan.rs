use crate::app::error::make_friendly_error;
use crate::app::tab_error::TabError;
use crate::components::{
    empty_state_paragraph, InputField, InputGroup, ProgressGauge, ScrollableText, Selector,
    SelectorItem,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::tc;
use eggsec::cli::ScanProfile;
use eggsec::pipeline::{PipelineReport, Stage};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct ScanTab {
    pub inputs: InputGroup,
    pub profile_selector: Selector,
    pub output_selector: Selector,
    pub stages: Vec<StageInfo>,
    pub results_view: ScrollableText,
    pub report: Option<PipelineReport>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub focus_area: ScanFocusArea,
    pub error: Option<TabError>,
}

#[derive(Debug, Clone)]
pub struct StageInfo {
    pub stage: Stage,
    pub status: StageStatus,
    pub duration_ms: u64,
    pub result_summary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

impl ScanTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new()
            .add(InputField::new("Target"))
            .add(InputField::new("Output File").with_value("report.json"));

        let profile_selector = Selector::new("Profile").items(vec![
            SelectorItem::new("Quick (port + fingerprint)", "quick"),
            SelectorItem::new("Endpoint (quick + endpoints)", "endpoint"),
            SelectorItem::new("Web (endpoint + fuzzing)", "web"),
            SelectorItem::new("WAF (endpoint + WAF bypass)", "waf"),
            SelectorItem::new("Full (all stages)", "full"),
            SelectorItem::new("API (GraphQL/JWT/OAuth)", "api"),
            SelectorItem::new("Recon (tech detection + CVE)", "recon"),
            SelectorItem::new("Stealth (lab realism)", "stealth"),
            SelectorItem::new("Deep (mutation fuzzing)", "deep"),
            SelectorItem::new("Vuln (CVE-prioritized)", "vuln"),
            SelectorItem::new("Auth (JWT/OAuth/IDOR)", "auth"),
            SelectorItem::new("Defense Lab (local probe suite)", "defense-lab"),
            SelectorItem::new("Synvoid Local (Synvoid validation)", "synvoid-local"),
            SelectorItem::new("WAF Regression (evasion resistance)", "waf-regression"),
            SelectorItem::new("Protocol Edge (malformed protocols)", "protocol-edge"),
            SelectorItem::new("NSE Safe (sandboxed scripts)", "nse-safe"),
        ]);

        let output_selector = Selector::new("Output Format").items(vec![
            SelectorItem::new("JSON", "json"),
            SelectorItem::new("HTML", "html"),
            SelectorItem::new("CSV", "csv"),
            SelectorItem::new("SARIF", "sarif"),
        ]);

        let stages = Self::stages_for_profile(ScanProfile::Quick);

        Self {
            inputs,
            profile_selector,
            output_selector,
            stages,
            results_view: ScrollableText::new("Current Stage Output"),
            report: None,
            progress: ProgressGauge::new("Pipeline Progress"),
            state: AppState::Idle,
            focus_area: ScanFocusArea::Inputs,
            error: None,
        }
    }

    pub fn get_report(&self) -> Option<&PipelineReport> {
        self.report.as_ref()
    }

    pub fn target(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn output_file(&self) -> &str {
        self.inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("report.json")
    }

    pub fn profile(&self) -> Option<ScanProfile> {
        match self.profile_selector.selected_value() {
            Some("quick") => Some(ScanProfile::Quick),
            Some("endpoint") => Some(ScanProfile::Endpoint),
            Some("web") => Some(ScanProfile::Web),
            Some("waf") => Some(ScanProfile::Waf),
            Some("full") => Some(ScanProfile::Full),
            Some("api") => Some(ScanProfile::Api),
            Some("recon") => Some(ScanProfile::Recon),
            Some("stealth") => Some(ScanProfile::Stealth),
            Some("deep") => Some(ScanProfile::Deep),
            Some("vuln") => Some(ScanProfile::Vuln),
            Some("auth") => Some(ScanProfile::Auth),
            Some("defense-lab") => Some(ScanProfile::DefenseLab),
            Some("synvoid-local") => Some(ScanProfile::SynvoidLocal),
            Some("waf-regression") => Some(ScanProfile::WafRegression),
            Some("protocol-edge") => Some(ScanProfile::ProtocolEdge),
            Some("nse-safe") => Some(ScanProfile::NseSafe),
            _ => Some(ScanProfile::Quick),
        }
    }

    pub fn output_format(&self) -> &str {
        self.output_selector.selected_value().unwrap_or("json")
    }

    fn stages_for_profile(profile: ScanProfile) -> Vec<StageInfo> {
        let stages = Stage::from_profile(profile);
        stages
            .into_iter()
            .map(|stage| StageInfo {
                stage,
                status: StageStatus::Pending,
                duration_ms: 0,
                result_summary: String::new(),
            })
            .collect()
    }

    pub fn update_stages_for_profile(&mut self) {
        if let Some(profile) = self.profile() {
            self.stages = Self::stages_for_profile(profile);
        }
    }

    pub fn update_stage(&mut self, stage: Stage, status: StageStatus, summary: Option<&str>) {
        if let Some(stage_info) = self.stages.iter_mut().find(|s| s.stage == stage) {
            stage_info.status = status;
            if let Some(s) = summary {
                stage_info.result_summary = s.to_string();
            }
        }
    }

    pub fn set_stage_running(&mut self, stage: Stage) {
        for s in &mut self.stages {
            if s.stage == stage {
                s.status = StageStatus::Running;
            }
        }
    }

    pub fn set_stage_completed(&mut self, stage: Stage, duration_ms: u64, summary: &str) {
        if let Some(stage_info) = self.stages.iter_mut().find(|s| s.stage == stage) {
            stage_info.status = StageStatus::Completed;
            stage_info.duration_ms = duration_ms;
            stage_info.result_summary = summary.to_string();
        }
    }

    pub fn set_stage_failed(&mut self, stage: Stage, error: &str) {
        if let Some(stage_info) = self.stages.iter_mut().find(|s| s.stage == stage) {
            stage_info.status = StageStatus::Failed(error.to_string());
        }
    }

    pub fn add_stage_output(&mut self, line: &str) {
        self.results_view.add_text(line, None);
        self.results_view.scroll_to_end();
    }

    pub fn set_report(&mut self, report: PipelineReport) {
        self.report = Some(report);
        self.state = AppState::Completed;
    }

    pub fn start(&mut self) {
        if !self.target().is_empty() {
            self.state = AppState::Running;
            self.progress.current = 0;
            self.progress.total = self.stages.len() as u64;
            self.report = None;
            self.results_view.clear();
            for stage in &mut self.stages {
                stage.status = StageStatus::Pending;
                stage.duration_ms = 0;
                stage.result_summary.clear();
            }
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.progress.current = completed;
        self.progress.total = total;
    }

    pub fn reset_stages(&mut self) {
        for stage in &mut self.stages {
            stage.status = StageStatus::Pending;
            stage.duration_ms = 0;
            stage.result_summary.clear();
        }
        self.results_view.clear();
    }

    pub fn scroll_output_up(&mut self) {
        self.results_view.scroll_up(1);
    }

    pub fn scroll_output_down(&mut self) {
        self.results_view.scroll_down(1);
    }
}

impl Default for ScanTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for ScanTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        if self.stages.is_empty() {
            return 0.0;
        }
        let completed = self
            .stages
            .iter()
            .filter(|s| matches!(s.status, StageStatus::Completed))
            .count();
        let total = self.stages.len().max(1);
        (completed as f64 / total as f64) * 100.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.report = None;
        self.progress.current = 0;
        self.progress.total = 0;
        self.reset_stages();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        if let Some(field) = self.inputs.fields.get_mut(1) {
            field.value = "report.json".to_string();
            field.cursor_pos = 10;
        }
        self.profile_selector.cancel();
        self.output_selector.cancel();
        self.focus_area = ScanFocusArea::Inputs;
        self.results_view.clear();
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for ScanTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(area);

        let config_area = chunks[0];
        let main_area = chunks[1];

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ")
            .border_style(
                Style::default().fg(if self.focus_area == ScanFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let input_inner = input_block.inner(config_area);
        f.render_widget(input_block, config_area);

        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(input_inner);

        if self.inputs.fields.len() >= 2 {
            if let Some(field) = self.inputs.fields.first() {
                field.render(f, inner_chunks[0], insert_mode);
            }
            if let Some(field) = self.inputs.fields.get(1) {
                field.render(f, inner_chunks[1], insert_mode);
            }
        }

        let mut profile_sel = self.profile_selector.clone();
        profile_sel.focused = self.focus_area == ScanFocusArea::ProfileSelector;
        if let Some(area) = inner_chunks.get(2) {
            profile_sel.render(f, *area);
        }

        let mut output_sel = self.output_selector.clone();
        output_sel.focused = self.focus_area == ScanFocusArea::OutputSelector;
        if let Some(area) = inner_chunks.get(3) {
            output_sel.render(f, *area);
        }

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(main_area);

        let stages_area = main_chunks.first().copied().unwrap_or(main_area);
        let output_area = main_chunks.get(1).copied().unwrap_or(main_area);

        let mut stage_lines: Vec<Line> = Vec::new();

        for stage_info in &self.stages {
            let (icon, status_color) = match &stage_info.status {
                StageStatus::Pending => ("○", tc!(text_dim)),
                StageStatus::Running => ("▶", tc!(warning)),
                StageStatus::Completed => ("✓", tc!(success)),
                StageStatus::Failed(_) => ("✗", tc!(error)),
            };

            let stage_name = format!("{:?}", stage_info.stage);
            let status_text = match &stage_info.status {
                StageStatus::Pending => "pending".to_string(),
                StageStatus::Running => "running".to_string(),
                StageStatus::Completed => format!("{}s", stage_info.duration_ms / 1000),
                StageStatus::Failed(e) => {
                    let msg = make_friendly_error(&anyhow::anyhow!("{}", e));
                    // Truncate long error messages to keep the status column readable.
                    // Use char-aware truncation to avoid panicking on multi-byte UTF-8.
                    if msg.chars().count() > 10 {
                        let truncated: String = msg.chars().take(9).collect();
                        format!("{}…", truncated)
                    } else {
                        msg
                    }
                }
            };

            stage_lines.push(Line::from(vec![
                Span::styled(
                    format!("{} {:<18}", icon, stage_name),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!("{:<10}", status_text),
                    Style::default().fg(status_color),
                ),
                Span::styled(&stage_info.result_summary, Style::default()),
            ]));
        }

        let completed = self
            .stages
            .iter()
            .filter(|s| matches!(s.status, StageStatus::Completed))
            .count();
        let progress_text = format!("Stages ({}/{})", completed, self.stages.len());
        let stages_block = Paragraph::new(stage_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(progress_text)
                .border_style(
                    Style::default().fg(if self.focus_area == ScanFocusArea::Results {
                        tc!(border_focused)
                    } else {
                        tc!(border)
                    }),
                ),
        );
        f.render_widget(stages_block, stages_area);

        let output_block = Block::default()
            .borders(Borders::ALL)
            .title(" Output ")
            .border_style(
                Style::default().fg(if self.focus_area == ScanFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let output_inner = output_block.inner(output_area);
        f.render_widget(output_block, output_area);

        if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, output_inner);
        } else if !self.results_view.is_empty() {
            self.results_view.render(f, output_inner, None);
        } else {
            let placeholder =
                empty_state_paragraph("Current Stage Output", "Stage output will appear here");
            f.render_widget(placeholder, output_inner);
        }
    }

    fn render_overlays(&self, f: &mut Frame, area: Rect) {
        let config_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 12,
        };

        let config_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(config_area);

        let vh = f.area().height;
        if let Some(info) = self
            .profile_selector
            .dropdown_info(config_chunks.get(2).copied().unwrap_or(config_area), vh)
        {
            info.render(f);
        }
        if let Some(info) = self
            .output_selector
            .dropdown_info(config_chunks.get(3).copied().unwrap_or(config_area), vh)
        {
            info.render(f);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanFocusArea {
    Inputs,
    ProfileSelector,
    OutputSelector,
    Results,
}

impl TabInput for ScanTab {
    fn stop(&mut self) {
        ScanTab::stop(self);
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ScanFocusArea::Inputs => {
                self.inputs.blur();
                ScanFocusArea::ProfileSelector
            }
            ScanFocusArea::ProfileSelector => {
                self.profile_selector.cancel();
                ScanFocusArea::OutputSelector
            }
            ScanFocusArea::OutputSelector => {
                self.output_selector.cancel();
                ScanFocusArea::Results
            }
            ScanFocusArea::Results => {
                self.inputs.focus(0);
                ScanFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        self.focus_area = match self.focus_area {
            ScanFocusArea::Inputs => {
                self.inputs.blur();
                ScanFocusArea::Results
            }
            ScanFocusArea::ProfileSelector => {
                self.profile_selector.cancel();
                self.inputs.focus(0);
                ScanFocusArea::Inputs
            }
            ScanFocusArea::OutputSelector => {
                self.output_selector.cancel();
                ScanFocusArea::ProfileSelector
            }
            ScanFocusArea::Results => ScanFocusArea::OutputSelector,
        };
    }

    fn handle_char(&mut self, c: char) {
        if !self.is_running() && self.focus_area == ScanFocusArea::Inputs {
            self.inputs.insert(c);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.is_running() && self.focus_area == ScanFocusArea::Inputs {
            self.inputs.backspace();
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if !self.is_running() && self.focus_area == ScanFocusArea::Inputs {
            self.inputs.paste(text);
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if !self.is_running() {
            if self.focus_area == ScanFocusArea::Inputs {
                self.inputs.get_focused_value()
            } else {
                Some(self.results_view.get_content())
            }
        } else {
            None
        }
    }

    fn handle_word_forward(&mut self) {
        if !self.is_running() && self.focus_area == ScanFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if !self.is_running() && self.focus_area == ScanFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if !self.is_running() {
            if self.focus_area == ScanFocusArea::Inputs {
                self.inputs.move_home();
            } else {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_end(&mut self) {
        if !self.is_running() {
            if self.focus_area == ScanFocusArea::Inputs {
                self.inputs.move_end();
            } else {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.profile_selector.cancel();
            self.output_selector.cancel();
            self.focus_area = ScanFocusArea::Inputs;
            self.inputs.focus(0);
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.profile_selector.cancel();
            self.output_selector.cancel();
            self.focus_area = ScanFocusArea::Results;
            self.inputs.blur();
        }
    }

    fn handle_enter(&mut self) {
        if self.focus_area == ScanFocusArea::Results {
            return;
        }

        if self.is_running() {
            self.stop();
            return;
        }
        if self.focus_area == ScanFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }

        if self.focus_area == ScanFocusArea::ProfileSelector {
            if self.profile_selector.is_open() {
                if self.profile_selector.confirm().is_none() {
                    tracing::warn!("Profile selector confirm failed");
                }
                self.update_stages_for_profile();
            } else {
                self.profile_selector.open();
            }
            return;
        }

        if self.focus_area == ScanFocusArea::OutputSelector {
            if self.output_selector.is_open() {
                if self.output_selector.confirm().is_none() {
                    tracing::warn!("Output selector confirm failed");
                }
            } else {
                self.output_selector.open();
            }
            return;
        }

        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.profile_selector.is_open() {
            self.profile_selector.cancel();
            return;
        }
        if self.output_selector.is_open() {
            self.output_selector.cancel();
            return;
        }
        match self.focus_area {
            ScanFocusArea::Inputs => self.inputs.blur(),
            ScanFocusArea::ProfileSelector => {
                self.profile_selector.cancel();
                self.focus_area = ScanFocusArea::Inputs;
                self.inputs.focus(0);
            }
            ScanFocusArea::OutputSelector => {
                self.output_selector.cancel();
                self.focus_area = ScanFocusArea::Inputs;
                self.inputs.focus(0);
            }
            ScanFocusArea::Results => {
                self.focus_area = ScanFocusArea::Inputs;
                self.inputs.focus(0);
            }
        }
        self.profile_selector.collapse();
        self.output_selector.collapse();
    }

    fn handle_up(&mut self) {
        if !self.is_running() {
            if self.profile_selector.is_open() {
                self.profile_selector.move_prev();
                self.update_stages_for_profile();
            } else if self.output_selector.is_open() {
                self.output_selector.move_prev();
            } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_output_up();
            } else if self.focus_area != ScanFocusArea::Inputs {
                self.focus_area = ScanFocusArea::Inputs;
                self.inputs.focus(0);
            } else {
                self.inputs.focus_prev();
            }
        }
    }

    fn handle_down(&mut self) {
        if !self.is_running() {
            if self.profile_selector.is_open() {
                self.profile_selector.move_next();
                self.update_stages_for_profile();
            } else if self.output_selector.is_open() {
                self.output_selector.move_next();
            } else if !self.inputs.is_focused() && !self.results_view.is_empty() {
                self.scroll_output_down();
            } else if self.focus_area == ScanFocusArea::Results
                && self.results_view.is_empty()
            {
                self.focus_area = ScanFocusArea::Inputs;
                self.inputs.focus(0);
            } else if self.focus_area == ScanFocusArea::Inputs && !self.inputs.is_focused() {
                self.focus_area = ScanFocusArea::ProfileSelector;
            } else if self.focus_area == ScanFocusArea::ProfileSelector {
                self.focus_area = ScanFocusArea::OutputSelector;
            } else if self.focus_area == ScanFocusArea::OutputSelector {
                self.focus_area = ScanFocusArea::Inputs;
                self.inputs.focus(0);
            } else {
                self.inputs.focus_next();
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        if !self.is_running() {
            if self.focus_area == ScanFocusArea::Inputs {
                self.inputs.move_left()
            } else {
                self.results_view.scroll_left(5);
                true
            }
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if !self.is_running() {
            if self.focus_area == ScanFocusArea::Inputs {
                self.inputs.move_right()
            } else {
                self.results_view.scroll_right(5);
                true
            }
        } else {
            false
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else {
            self.results_view.is_at_left_edge()
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else {
            self.results_view.is_at_right_edge()
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ScanFocusArea::Inputs && self.inputs.is_focused()
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
