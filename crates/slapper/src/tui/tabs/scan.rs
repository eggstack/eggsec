use crate::cli::ScanProfile;
use crate::pipeline::{PipelineReport, Stage};
use crate::tc;
use crate::tui::app::error::make_friendly_error;
use crate::tui::components::{
    empty_state_paragraph, InputField, InputGroup, ProgressGauge, ScrollableText, Selector,
    SelectorItem,
};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
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
    pub current_stage_output: ScrollableText,
    pub report: Option<PipelineReport>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub focus_area: ScanFocusArea,
    pub error_message: Option<String>,
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
            SelectorItem::new("Stealth (evasion mode)", "stealth"),
            SelectorItem::new("Deep (mutation fuzzing)", "deep"),
            SelectorItem::new("Vuln (CVE-prioritized)", "vuln"),
            SelectorItem::new("Auth (JWT/OAuth/IDOR)", "auth"),
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
            current_stage_output: ScrollableText::new("Current Stage Output"),
            report: None,
            progress: ProgressGauge::new("Pipeline Progress"),
            state: AppState::Idle,
            focus_area: ScanFocusArea::Inputs,
            error_message: None,
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
        self.current_stage_output.add_text(line, None);
        self.current_stage_output.scroll_to_end();
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
            self.current_stage_output.clear();
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

    pub fn update_progress(&mut self, _completed: u64, _total: u64) {}

    pub fn reset_stages(&mut self) {
        for stage in &mut self.stages {
            stage.status = StageStatus::Pending;
            stage.duration_ms = 0;
            stage.result_summary.clear();
        }
        self.current_stage_output.clear();
    }

    pub fn scroll_output_up(&mut self) {
        self.current_stage_output.scroll_up(1);
    }

    pub fn scroll_output_down(&mut self) {
        self.current_stage_output.scroll_down(1);
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.current_stage_output.page_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.current_stage_output.page_down(page_size);
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
        let completed = self
            .stages
            .iter()
            .filter(|s| matches!(s.status, StageStatus::Completed))
            .count();
        (completed as f64 / self.stages.len() as f64) * 100.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.report = None;
        self.progress.current = 0;
        self.reset_stages();
        self.error_message = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        self.inputs.fields[1].value = "report.json".to_string();
        self.inputs.fields[1].cursor_pos = 11;
        self.profile_selector.select(0);
        self.output_selector.select(0);
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg);
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

        let config_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(config_area);

        self.inputs.fields[0].render(f, config_chunks[0], insert_mode);
        self.inputs.fields[1].render(f, config_chunks[1], insert_mode);

        let mut profile_sel = self.profile_selector.clone();
        profile_sel.focused = self.focus_area == ScanFocusArea::ProfileSelector;
        profile_sel.render(f, config_chunks[2]);

        let mut output_sel = self.output_selector.clone();
        output_sel.focused = self.focus_area == ScanFocusArea::OutputSelector;
        output_sel.render(f, config_chunks[3]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(main_area);

        let stages_area = main_chunks[0];
        let output_area = main_chunks[1];

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
                StageStatus::Failed(e) => make_friendly_error(&anyhow::anyhow!("{}", e)),
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
        let stages_block = Paragraph::new(stage_lines)
            .block(Block::default().borders(Borders::ALL).title(progress_text));
        f.render_widget(stages_block, stages_area);

        if let Some(ref err_msg) = self.error_message {
            let error_text = Paragraph::new(format!("Error: {}", err_msg))
                .block(Block::default().borders(Borders::ALL).title("Scan - Error"))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, output_area);
        } else if !self.current_stage_output.is_empty() {
            self.current_stage_output.render(f, output_area, None);
        } else {
            let placeholder =
                empty_state_paragraph("Current Stage Output", "Stage output will appear here");
            f.render_widget(placeholder, output_area);
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

        if let Some(info) = self.profile_selector.dropdown_info(config_chunks[2]) {
            info.render(f);
        }
        if let Some(info) = self.output_selector.dropdown_info(config_chunks[3]) {
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
    fn handle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            ScanFocusArea::Inputs => {
                self.inputs.blur();
                ScanFocusArea::ProfileSelector
            }
            ScanFocusArea::ProfileSelector => ScanFocusArea::OutputSelector,
            ScanFocusArea::OutputSelector => ScanFocusArea::Results,
            ScanFocusArea::Results => {
                self.inputs.focus(0);
                ScanFocusArea::Inputs
            }
        };
    }

    fn handle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            ScanFocusArea::Inputs => ScanFocusArea::Results,
            ScanFocusArea::ProfileSelector => {
                self.inputs.focus(0);
                ScanFocusArea::Inputs
            }
            ScanFocusArea::OutputSelector => ScanFocusArea::ProfileSelector,
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
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.get_focused_value()
        } else {
            Some(self.current_stage_output.get_content())
        }
    }

    fn handle_word_forward(&mut self) {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.move_word_forward();
        }
    }

    fn handle_word_backward(&mut self) {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.move_word_backward();
        }
    }

    fn handle_home(&mut self) {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.move_home();
        } else {
            self.current_stage_output.scroll_to_top();
        }
    }

    fn handle_end(&mut self) {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.move_end();
        } else {
            self.current_stage_output.scroll_to_bottom();
        }
    }

    fn handle_top(&mut self) {
        self.focus_area = ScanFocusArea::Inputs;
        self.inputs.focus(0);
    }

    fn handle_bottom(&mut self) {
        self.focus_area = ScanFocusArea::Inputs;
        self.inputs.blur();
    }

    fn handle_enter(&mut self) {
        if self.focus_area == ScanFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }

        if self.focus_area == ScanFocusArea::ProfileSelector {
            self.profile_selector.toggle();
            self.update_stages_for_profile();
            return;
        }

        if self.focus_area == ScanFocusArea::OutputSelector {
            self.output_selector.toggle();
            return;
        }

        if self.is_running() {
            self.stop();
        } else {
            self.start();
        }
    }

    fn handle_escape(&mut self) {
        self.inputs.blur();
        self.profile_selector.collapse();
        self.output_selector.collapse();
    }

    fn handle_up(&mut self) {
        if self.profile_selector.expanded {
            self.profile_selector.prev();
            self.update_stages_for_profile();
        } else if self.output_selector.expanded {
            self.output_selector.prev();
        } else if !self.inputs.is_focused() && !self.current_stage_output.is_empty() {
            self.scroll_output_up();
        } else if self.focus_area != ScanFocusArea::Inputs {
            self.focus_area = ScanFocusArea::Inputs;
            self.inputs.focus(0);
        } else {
            self.inputs.focus_prev();
        }
    }

    fn handle_down(&mut self) {
        if self.profile_selector.expanded {
            self.profile_selector.next();
            self.update_stages_for_profile();
        } else if self.output_selector.expanded {
            self.output_selector.next();
        } else if !self.inputs.is_focused() && !self.current_stage_output.is_empty() {
            self.scroll_output_down();
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

    fn handle_left(&mut self) -> bool {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.move_left()
        } else {
            self.current_stage_output.scroll_left(5);
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.move_right()
        } else {
            self.current_stage_output.scroll_right(5);
            true
        }
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.is_at_left_edge()
        } else {
            self.current_stage_output.is_at_left_edge()
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == ScanFocusArea::Inputs {
            self.inputs.is_at_right_edge()
        } else {
            self.current_stage_output.is_at_right_edge()
        }
    }

    fn is_input_focused(&self) -> bool {
        self.focus_area == ScanFocusArea::Inputs && self.inputs.is_focused()
    }
}
