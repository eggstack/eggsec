use crate::app::tab_error::TabError;
use crate::components::{
    empty_state_paragraph, InputField, InputGroup, ProgressGauge, ScrollableText,
};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::tc;
#[cfg(feature = "wireless-advanced")]
use eggsec::wireless::active::ActiveWirelessAttackResult;
use eggsec::wireless::WirelessScanResult;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WirelessFocusArea {
    Inputs,
    #[cfg(feature = "wireless-advanced")]
    ActiveConfig,
    Results,
}

pub struct WirelessTab {
    pub inputs: InputGroup,
    pub results: Option<WirelessScanResult>,
    pub progress: ProgressGauge,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub focus_area: WirelessFocusArea,
    pub error: Option<TabError>,
    #[cfg(feature = "wireless-advanced")]
    pub active_mode: bool,
    #[cfg(feature = "wireless-advanced")]
    pub active_inputs: InputGroup,
    #[cfg(feature = "wireless-advanced")]
    pub dry_run: bool,
    #[cfg(feature = "wireless-advanced")]
    pub active_results: Option<ActiveWirelessAttackResult>,
}

impl WirelessTab {
    pub fn new() -> Self {
        let inputs = InputGroup::new().add(InputField::new("Wireless Interface"));

        Self {
            inputs,
            results: None,
            progress: ProgressGauge::new("Scanning..."),
            state: AppState::Idle,
            results_view: ScrollableText::new("Results"),
            focus_area: WirelessFocusArea::Inputs,
            error: None,
            #[cfg(feature = "wireless-advanced")]
            active_mode: false,
            #[cfg(feature = "wireless-advanced")]
            active_inputs: {
                let mut group = InputGroup::new();
                group = group
                    .add(InputField::new("BSSID (optional)").with_value(""))
                    .add(InputField::new("Client MAC (optional)").with_value(""))
                    .add(InputField::new("Frame Count").with_value("100"))
                    .add(InputField::new("Rate Limit (fps)").with_value("10"));
                group
            },
            #[cfg(feature = "wireless-advanced")]
            dry_run: true,
            #[cfg(feature = "wireless-advanced")]
            active_results: None,
        }
    }

    pub fn get_results(&self) -> Option<&WirelessScanResult> {
        self.results.as_ref()
    }

    #[cfg(feature = "wireless-advanced")]
    pub fn toggle_active_mode(&mut self) {
        self.active_mode = !self.active_mode;
        if !self.active_mode {
            for field in &mut self.active_inputs.fields {
                field.clear();
            }
            self.active_inputs.blur();
        }
    }

    #[cfg(feature = "wireless-advanced")]
    pub fn toggle_dry_run(&mut self) {
        self.dry_run = !self.dry_run;
    }

    #[cfg(feature = "wireless-advanced")]
    pub fn active_attack_config(
        &self,
    ) -> Option<(
        String,
        String,
        Option<String>,
        Option<String>,
        u64,
        u64,
        bool,
    )> {
        if !self.active_mode {
            return None;
        }
        let interface = self.inputs.fields.first()?.value.clone();
        if interface.is_empty() {
            return None;
        }
        let bssid = self.active_inputs.fields.get(0)?.value.trim().to_string();
        let client = self.active_inputs.fields.get(1)?.value.trim().to_string();
        let frame_count: u64 = self
            .active_inputs
            .fields
            .get(2)?
            .value
            .trim()
            .parse()
            .unwrap_or(100);
        let rate_limit: u64 = self
            .active_inputs
            .fields
            .get(3)?
            .value
            .trim()
            .parse()
            .unwrap_or(10);
        let attack_type = "deauth".to_string();
        Some((
            interface,
            attack_type,
            if bssid.is_empty() { None } else { Some(bssid) },
            if client.is_empty() {
                None
            } else {
                Some(client)
            },
            frame_count,
            rate_limit,
            self.dry_run,
        ))
    }

    #[cfg(feature = "wireless-advanced")]
    pub fn set_active_results(&mut self, results: ActiveWirelessAttackResult) {
        self.update_active_results_view(&results);
        self.active_results = Some(results);
        self.state = AppState::Completed;
    }

    #[cfg(feature = "wireless-advanced")]
    fn update_active_results_view(&mut self, result: &ActiveWirelessAttackResult) {
        use ratatui::style::{Color, Modifier};

        self.results_view.clear();

        self.results_view.add_line(Line::from(vec![Span::styled(
            format!("Active Attack: {}", result.attack_type),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]));
        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(vec![
            Span::styled("Interface: ", Style::default().fg(Color::Gray)),
            Span::raw(result.interface.clone()),
        ]));
        if let Some(ref bssid) = result.target_bssid {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Target BSSID: ", Style::default().fg(Color::Gray)),
                Span::raw(bssid.clone()),
            ]));
        }
        if let Some(ref client) = result.target_client {
            self.results_view.add_line(Line::from(vec![
                Span::styled("Target Client: ", Style::default().fg(Color::Gray)),
                Span::raw(client.clone()),
            ]));
        }
        self.results_view.add_line(Line::from(vec![
            Span::styled("Frames Sent: ", Style::default().fg(Color::Gray)),
            Span::raw(result.frames_sent.to_string()),
        ]));
        self.results_view.add_line(Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}s", result.duration_secs)),
        ]));
        self.results_view.add_line(Line::from(vec![
            Span::styled("Dry Run: ", Style::default().fg(Color::Gray)),
            Span::raw(result.dry_run.to_string()),
        ]));
        self.results_view.add_line(Line::from(""));

        if !result.findings.is_empty() {
            self.results_view.add_line(Line::from(vec![Span::styled(
                "Findings:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
            for finding in &result.findings {
                self.results_view.add_line(Line::from(vec![
                    Span::styled(
                        format!("  [{}] ", finding.severity),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(finding.description.clone()),
                ]));
                if !finding.evidence.is_empty() {
                    self.results_view.add_line(Line::from(vec![
                        Span::styled("    Evidence: ", Style::default().fg(Color::DarkGray)),
                        Span::raw(finding.evidence.clone()),
                    ]));
                }
                if !finding.remediation.is_empty() {
                    self.results_view.add_line(Line::from(vec![
                        Span::styled("    Remediation: ", Style::default().fg(Color::DarkGray)),
                        Span::raw(finding.remediation.clone()),
                    ]));
                }
            }
            self.results_view.add_line(Line::from(""));
        }

        if !result.recommendations.is_empty() {
            self.results_view.add_line(Line::from(vec![Span::styled(
                "Recommendations:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            for rec in &result.recommendations {
                self.results_view.add_line(Line::from(vec![
                    Span::styled("  • ", Style::default().fg(Color::Cyan)),
                    Span::raw(rec.clone()),
                ]));
            }
        }
    }

    pub fn interface(&self) -> &str {
        self.inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn set_results(&mut self, results: WirelessScanResult) {
        self.update_results_view(&results);
        self.results = Some(results);
        self.state = AppState::Completed;
    }

    fn update_results_view(&mut self, results: &WirelessScanResult) {
        self.results_view.clear();

        let interface = results.interface.clone();
        let network_count = results.networks.len();

        self.results_view.add_line(Line::from(vec![
            Span::styled("Interface: ", Style::default().fg(tc!(warning))),
            Span::raw(interface),
        ]));

        self.results_view.add_line(Line::from(vec![
            Span::styled("Networks found: ", Style::default().fg(tc!(info))),
            Span::raw(network_count.to_string()),
        ]));

        self.results_view.add_line(Line::from(""));
        self.results_view.add_line(Line::from(vec![
            Span::styled(format!("{:<20}", "SSID"), Style::default().fg(tc!(warning))),
            Span::styled(
                format!("{:<20}", "BSSID"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled(format!("{:<8}", "CH"), Style::default().fg(tc!(warning))),
            Span::styled(
                format!("{:<12}", "SECURITY"),
                Style::default().fg(tc!(warning)),
            ),
            Span::styled("SIGNAL", Style::default().fg(tc!(warning))),
        ]));

        for network in &results.networks {
            let ssid_display = if network.ssid.len() > 18 {
                let truncate_pos = network
                    .ssid
                    .char_indices()
                    .take_while(|(i, _)| *i < 15)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(15);
                format!("{}...", &network.ssid[..truncate_pos])
            } else {
                network.ssid.clone()
            };
            self.results_view.add_line(Line::from(vec![
                Span::styled(
                    format!("{:<20}", ssid_display),
                    Style::default().fg(tc!(success)),
                ),
                Span::raw(format!("{:<20}", network.bssid)),
                Span::raw(format!("{:<8}", network.channel)),
                Span::styled(
                    format!("{:<12}", network.security_type.as_str()),
                    Style::default().fg(match network.security_type {
                        eggsec::wireless::SecurityType::Open => tc!(error),
                        eggsec::wireless::SecurityType::WEP => tc!(error),
                        eggsec::wireless::SecurityType::WPA => tc!(warning),
                        _ => tc!(success),
                    }),
                ),
                Span::raw(format!("{} dBm", network.signal_strength)),
            ]));
        }

        if !results.recommendations.is_empty() {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(vec![Span::styled(
                "Recommendations:",
                Style::default().fg(tc!(warning)),
            )]));
            for rec in &results.recommendations {
                self.results_view.add_line(Line::from(vec![
                    Span::styled("  - ", Style::default().fg(tc!(info))),
                    Span::raw(rec.clone()),
                ]));
            }
        }

        // Active attacks notice
        #[cfg(feature = "wireless-advanced")]
        {
            self.results_view.add_line(Line::from(""));
            self.results_view.add_line(Line::from(vec![Span::styled(
                "Tip: Active attacks (deauth, disassoc) are also available from this tab.",
                Style::default().fg(tc!(info)),
            )]));
            self.results_view.add_line(Line::from(vec![Span::styled(
                "  Press 'a' to enter Active mode, fill in BSSID / Client / Frame Count / Rate Limit,",
                Style::default().fg(tc!(muted)),
            )]));
            self.results_view.add_line(Line::from(vec![Span::styled(
                "  then press Enter to launch (dry-run is on by default; press 'd' to toggle).",
                Style::default().fg(tc!(muted)),
            )]));
        }
    }

    pub fn start(&mut self) {
        if !self.interface().is_empty() {
            self.state = AppState::Running;
            self.progress.current = 0;
            self.results = None;
            self.results_view.clear();
            self.error = None;
        }
    }

    #[cfg(feature = "wireless-advanced")]
    pub fn start_active_attack(&mut self) {
        if self.active_attack_config().is_some() {
            self.state = AppState::Running;
            self.progress.current = 0;
            self.active_results = None;
            self.results_view.clear();
            self.error = None;
            // NOTE: UI state only. For direct_launch tabs, App::handle_enter()
            // detects the running state, calls build_current_task() (which uses
            // the TaskBuilder trait impl to produce WirelessActive), evaluates
            // policy via EnforcementContext, and spawns the TaskRunner if allowed.
            // Results are delivered centrally via state_update.rs -> set_active_results().
        }
    }

    pub fn stop(&mut self) {
        self.state = AppState::Idle;
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) {
        self.progress.current = completed;
        self.progress.total = total;
    }

    pub fn scroll_results_up(&mut self) {
        self.results_view.scroll_up(1);
    }

    pub fn scroll_results_down(&mut self) {
        self.results_view.scroll_down(1);
    }
}

impl Default for WirelessTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for WirelessTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        self.progress.percent() as f64
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.results = None;
        self.progress.current = 0;
        self.progress.total = 0;
        self.results_view.clear();
        self.error = None;
        for field in &mut self.inputs.fields {
            field.clear();
        }
        #[cfg(feature = "wireless-advanced")]
        {
            self.active_mode = false;
            for field in &mut self.active_inputs.fields {
                field.clear();
            }
            self.active_inputs.blur();
            self.dry_run = true;
            self.active_results = None;
        }
        self.focus_area = WirelessFocusArea::Inputs;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
        self.progress.current = 0;
    }
}

impl TabRender for WirelessTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        #[cfg(feature = "wireless-advanced")]
        let active_height: u16 = if self.active_mode { 8 } else { 0 };

        #[cfg(not(feature = "wireless-advanced"))]
        let input_height: u16 = 5;
        #[cfg(feature = "wireless-advanced")]
        let input_height: u16 = 5 + active_height;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(input_height), Constraint::Min(0)])
            .split(area);

        let input_area = chunks[0];
        let results_area = chunks[1];

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title({
                #[cfg(feature = "wireless-advanced")]
                {
                    if self.active_mode {
                        " Wireless Scan + Active Attack Configuration "
                    } else {
                        " Wireless Scan Configuration "
                    }
                }
                #[cfg(not(feature = "wireless-advanced"))]
                {
                    " Wireless Scan Configuration "
                }
            })
            .border_style(
                Style::default().fg(if self.focus_area == WirelessFocusArea::Inputs {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let input_inner = input_block.inner(input_area);
        f.render_widget(input_block, input_area);

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3)])
            .split(input_inner);

        if let (Some(chunk), Some(field)) = (input_chunks.first(), self.inputs.fields.first()) {
            field.render(f, *chunk, insert_mode);
        }

        #[cfg(feature = "wireless-advanced")]
        if self.active_mode {
            let active_block = Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Active Attack Configuration (dry_run: {}) [a] toggle ",
                    self.dry_run
                ))
                .border_style(Style::default().fg(
                    if self.focus_area == WirelessFocusArea::ActiveConfig {
                        tc!(border_focused)
                    } else {
                        tc!(border)
                    },
                ));
            let active_area = Rect {
                x: input_inner.x,
                y: input_inner.y + 3,
                width: input_inner.width,
                height: 7,
            };
            let active_inner = active_block.inner(active_area);
            f.render_widget(active_block, active_area);

            let active_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(active_inner);

            for (i, chunk) in active_chunks.iter().enumerate() {
                if let Some(field) = self.active_inputs.fields.get(i) {
                    field.render(f, *chunk, insert_mode);
                }
            }
        }

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(
                Style::default().fg(if self.focus_area == WirelessFocusArea::Results {
                    tc!(border_focused)
                } else {
                    tc!(border)
                }),
            );
        let results_inner = results_block.inner(results_area);
        f.render_widget(results_block, results_area);

        if self.state == AppState::Running {
            self.progress.render(f, results_inner);
        } else if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, results_inner);
        } else if !self.results_view.is_empty() {
            self.results_view
                .render(f, results_inner, Some(tc!(success)));
        } else {
            let placeholder =
                empty_state_paragraph("Results", "Results will appear here after scanning");
            f.render_widget(placeholder, results_inner);
        }
    }
}

impl TabInput for WirelessTab {
    fn stop(&mut self) {
        WirelessTab::stop(self);
    }

    fn handle_focus_next(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => {
                self.inputs.blur();
                #[cfg(feature = "wireless-advanced")]
                if self.active_mode {
                    self.focus_area = WirelessFocusArea::ActiveConfig;
                    if !self.active_inputs.fields.is_empty() {
                        self.active_inputs.focus(0);
                    }
                    return;
                }
                self.focus_area = WirelessFocusArea::Results;
            }
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => {
                self.active_inputs.blur();
                self.focus_area = WirelessFocusArea::Results;
            }
            WirelessFocusArea::Results => {
                self.focus_area = WirelessFocusArea::Inputs;
                if !self.inputs.fields.is_empty() {
                    self.inputs.focus(0);
                }
            }
        }
    }

    fn handle_focus_prev(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => {
                self.inputs.blur();
                self.focus_area = WirelessFocusArea::Results;
            }
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => {
                self.active_inputs.blur();
                self.focus_area = WirelessFocusArea::Inputs;
                if !self.inputs.fields.is_empty() {
                    self.inputs.focus(0);
                }
            }
            WirelessFocusArea::Results => {
                #[cfg(feature = "wireless-advanced")]
                if self.active_mode {
                    self.focus_area = WirelessFocusArea::ActiveConfig;
                    if !self.active_inputs.fields.is_empty() {
                        self.active_inputs.focus(0);
                    }
                    return;
                }
                self.focus_area = WirelessFocusArea::Inputs;
                if !self.inputs.fields.is_empty() {
                    self.inputs.focus(0);
                }
            }
        }
    }

    fn handle_char(&mut self, c: char) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => {
                #[cfg(feature = "wireless-advanced")]
                if c == 'a' && !self.inputs.is_focused() {
                    self.toggle_active_mode();
                    return;
                }
                self.inputs.insert(c);
            }
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => {
                if c == 'd' && !self.active_inputs.is_focused() {
                    self.toggle_dry_run();
                    return;
                }
                self.active_inputs.insert(c);
            }
            WirelessFocusArea::Results => {}
        }
    }

    fn handle_backspace(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => self.inputs.backspace(),
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => self.active_inputs.backspace(),
            WirelessFocusArea::Results => {}
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => self.inputs.paste(text),
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => self.active_inputs.paste(text),
            WirelessFocusArea::Results => {}
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Inputs && self.inputs.is_focused() {
            self.inputs.blur();
            return;
        }
        #[cfg(feature = "wireless-advanced")]
        if self.focus_area == WirelessFocusArea::ActiveConfig
            && self.active_inputs.is_focused()
            && self.active_mode
        {
            if self.active_attack_config().is_some() {
                self.start_active_attack();
            } else {
                self.active_inputs.blur();
            }
            return;
        }
        #[cfg(feature = "wireless-advanced")]
        if self.focus_area == WirelessFocusArea::ActiveConfig && self.active_inputs.is_focused() {
            self.active_inputs.blur();
            return;
        }
        self.start();
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        self.focus_area = WirelessFocusArea::Inputs;
        self.inputs.blur();
        #[cfg(feature = "wireless-advanced")]
        self.active_inputs.blur();
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Results {
            self.scroll_results_up();
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Results {
            self.scroll_results_down();
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs if self.inputs.is_focused() => self.inputs.move_left(),
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig if self.active_inputs.is_focused() => {
                self.active_inputs.move_left()
            }
            _ => false,
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs if self.inputs.is_focused() => self.inputs.move_right(),
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig if self.active_inputs.is_focused() => {
                self.active_inputs.move_right()
            }
            _ => false,
        }
    }

    fn handle_word_forward(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => self.inputs.move_word_forward(),
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => self.active_inputs.move_word_forward(),
            _ => {}
        }
    }

    fn handle_word_backward(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => self.inputs.move_word_backward(),
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => self.active_inputs.move_word_backward(),
            _ => {}
        }
    }

    fn handle_home(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => self.inputs.move_home(),
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => self.active_inputs.move_home(),
            WirelessFocusArea::Results => self.results_view.scroll_to_top(),
        }
    }

    fn handle_end(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => self.inputs.move_end(),
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => self.active_inputs.move_end(),
            WirelessFocusArea::Results => self.results_view.scroll_to_bottom(),
        }
    }

    fn handle_top(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => {
                if !self.inputs.fields.is_empty() {
                    self.inputs.focus(0);
                }
            }
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => {
                if !self.active_inputs.fields.is_empty() {
                    self.active_inputs.focus(0);
                }
            }
            WirelessFocusArea::Results => {
                self.results_view.scroll_to_top();
            }
        }
    }

    fn handle_bottom(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            WirelessFocusArea::Inputs => {
                self.inputs.blur();
                #[cfg(feature = "wireless-advanced")]
                if self.active_mode {
                    self.focus_area = WirelessFocusArea::ActiveConfig;
                    return;
                }
                self.focus_area = WirelessFocusArea::Results;
            }
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => {
                self.active_inputs.blur();
                self.focus_area = WirelessFocusArea::Results;
            }
            WirelessFocusArea::Results => {
                self.results_view.scroll_to_bottom();
            }
        }
    }

    fn handle_copy(&mut self) -> Option<String> {
        if self.is_running() {
            return None;
        }
        if self.focus_area == WirelessFocusArea::Results {
            Some(self.results_view.get_content())
        } else {
            None
        }
    }

    fn is_input_focused(&self) -> bool {
        match self.focus_area {
            WirelessFocusArea::Inputs => self.inputs.is_focused(),
            #[cfg(feature = "wireless-advanced")]
            WirelessFocusArea::ActiveConfig => self.active_inputs.is_focused(),
            WirelessFocusArea::Results => false,
        }
    }

    fn page_up(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Results {
            for _ in 0..page_size {
                self.results_view.scroll_up(1);
            }
        }
    }

    fn page_down(&mut self, page_size: usize) {
        if self.is_running() {
            return;
        }
        if self.focus_area == WirelessFocusArea::Results {
            for _ in 0..page_size {
                self.results_view.scroll_down(1);
            }
        }
    }

    fn primary_target(&self) -> Option<String> {
        Some(self.interface().to_string())
    }
}

#[cfg(all(test, feature = "wireless-advanced"))]
mod tests {
    use super::*;
    use crate::app::task_management::TaskBuilder;

    #[test]
    fn test_active_attack_config_none_when_inactive() {
        let tab = WirelessTab::new();
        assert!(tab.active_attack_config().is_none());
    }

    #[test]
    fn test_active_attack_config_none_without_interface() {
        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.active_inputs.fields[0].value = "AA:BB:CC:DD:EE:FF".to_string();
        assert!(tab.active_attack_config().is_none());
    }

    #[test]
    fn test_active_attack_config_returns_values() {
        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.inputs.fields[0].value = "wlan0".to_string();
        tab.active_inputs.fields[0].value = "AA:BB:CC:DD:EE:FF".to_string();
        tab.active_inputs.fields[1].value = "11:22:33:44:55:66".to_string();
        tab.active_inputs.fields[2].value = "250".to_string();
        tab.active_inputs.fields[3].value = "20".to_string();
        tab.dry_run = false;

        let cfg = tab
            .active_attack_config()
            .expect("config should be present");
        assert_eq!(cfg.0, "wlan0");
        assert_eq!(cfg.1, "deauth");
        assert_eq!(cfg.2.as_deref(), Some("AA:BB:CC:DD:EE:FF"));
        assert_eq!(cfg.3.as_deref(), Some("11:22:33:44:55:66"));
        assert_eq!(cfg.4, 250);
        assert_eq!(cfg.5, 20);
        assert!(!cfg.6);
    }

    #[test]
    fn test_active_attack_config_omits_empty_optional_macs() {
        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.inputs.fields[0].value = "wlan0".to_string();
        tab.active_inputs.fields[0].value = "".to_string();
        tab.active_inputs.fields[1].value = "".to_string();

        let cfg = tab
            .active_attack_config()
            .expect("config should be present");
        assert!(cfg.2.is_none());
        assert!(cfg.3.is_none());
    }

    #[test]
    fn test_build_task_config_returns_wireless_active_variant() {
        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.inputs.fields[0].value = "wlan0".to_string();
        tab.active_inputs.fields[0].value = "AA:BB:CC:DD:EE:FF".to_string();
        tab.dry_run = true;

        match tab
            .build_task_config()
            .expect("task config should be present")
        {
            crate::workers::TaskConfig::WirelessActive {
                interface,
                attack_type,
                bssid,
                client,
                frame_count,
                rate_limit,
                dry_run,
            } => {
                assert_eq!(interface, "wlan0");
                assert_eq!(attack_type, "deauth");
                assert_eq!(bssid.as_deref(), Some("AA:BB:CC:DD:EE:FF"));
                assert!(client.is_none());
                assert_eq!(frame_count, 100);
                assert_eq!(rate_limit, 10);
                assert!(dry_run);
            }
            _ => panic!("expected WirelessActive task config"),
        }
    }

    #[test]
    fn test_set_active_results_renders_and_completes() {
        use eggsec::wireless::active::ActiveWirelessAttackResult;

        let mut tab = WirelessTab::new();
        let result = ActiveWirelessAttackResult {
            interface: "wlan0".to_string(),
            attack_type: "deauth".to_string(),
            target_bssid: Some("AA:BB:CC:DD:EE:FF".to_string()),
            target_client: None,
            frames_sent: 100,
            duration_secs: 10,
            dry_run: true,
            findings: Vec::new(),
            raw_output: None,
            recommendations: vec!["Enable 802.11w (PMF) if supported.".to_string()],
        };

        tab.set_active_results(result);
        assert_eq!(tab.state, AppState::Completed);
        assert!(tab.active_results.is_some());
    }

    #[test]
    fn test_toggle_active_mode_clears_inputs_when_off() {
        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.active_inputs.fields[0].value = "AA:BB:CC:DD:EE:FF".to_string();
        tab.active_inputs.fields[1].value = "11:22:33:44:55:66".to_string();

        tab.toggle_active_mode();
        assert!(!tab.active_mode);
        assert_eq!(tab.active_inputs.fields[0].value, "");
        assert_eq!(tab.active_inputs.fields[1].value, "");
    }

    #[test]
    fn test_toggle_dry_run_flips_state() {
        let mut tab = WirelessTab::new();
        assert!(tab.dry_run);
        tab.toggle_dry_run();
        assert!(!tab.dry_run);
        tab.toggle_dry_run();
        assert!(tab.dry_run);
    }

    #[test]
    fn test_start_active_attack_transitions_to_running() {
        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.inputs.fields[0].value = "wlan0".to_string();
        tab.active_inputs.fields[0].value = "AA:BB:CC:DD:EE:FF".to_string();
        tab.focus_area = WirelessFocusArea::ActiveConfig;
        tab.active_inputs.blur();

        tab.start_active_attack();
        assert_eq!(tab.state, AppState::Running);
    }

    #[test]
    fn test_start_active_attack_noop_when_invalid() {
        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.inputs.fields[0].value = "".to_string();

        tab.start_active_attack();
        assert_eq!(tab.state, AppState::Idle);
    }

    #[test]
    fn test_handle_enter_in_active_config_starts_attack() {
        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.inputs.fields[0].value = "wlan0".to_string();
        tab.active_inputs.fields[0].value = "AA:BB:CC:DD:EE:FF".to_string();
        tab.focus_area = WirelessFocusArea::ActiveConfig;
        tab.active_inputs.blur();

        tab.handle_enter();
        assert_eq!(tab.state, AppState::Running);
    }

    #[test]
    fn test_handle_enter_in_active_config_invalid_blurs_inputs() {
        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.inputs.fields[0].value = "".to_string();
        tab.focus_area = WirelessFocusArea::ActiveConfig;
        tab.active_inputs.focus(0);

        tab.handle_enter();
        assert_eq!(tab.state, AppState::Idle);
        assert!(!tab.active_inputs.is_focused());
    }

    #[test]
    fn test_e2e_active_flow_handle_enter_build_task_set_results() {
        use crate::app::task_management::TaskBuilder;
        use eggsec::wireless::active::ActiveWirelessAttackResult;

        let mut tab = WirelessTab::new();
        tab.active_mode = true;
        tab.inputs.fields[0].value = "wlan0".to_string();
        tab.active_inputs.fields[0].value = "AA:BB:CC:DD:EE:FF".to_string();
        tab.dry_run = true;
        tab.focus_area = WirelessFocusArea::ActiveConfig;
        tab.active_inputs.blur();

        tab.handle_enter();
        assert_eq!(tab.state, AppState::Running);
        assert!(tab.active_results.is_none());
        assert!(tab.results_view.is_empty());

        let task = tab.build_task_config().expect("task config present");
        match task {
            crate::workers::TaskConfig::WirelessActive {
                interface,
                attack_type,
                bssid,
                client,
                frame_count,
                rate_limit,
                dry_run,
            } => {
                assert_eq!(interface, "wlan0");
                assert_eq!(attack_type, "deauth");
                assert_eq!(bssid.as_deref(), Some("AA:BB:CC:DD:EE:FF"));
                assert!(client.is_none());
                assert_eq!(frame_count, 100);
                assert_eq!(rate_limit, 10);
                assert!(dry_run);
            }
            _ => panic!("expected WirelessActive"),
        }

        let result = ActiveWirelessAttackResult {
            interface: "wlan0".to_string(),
            attack_type: "deauth".to_string(),
            target_bssid: Some("AA:BB:CC:DD:EE:FF".to_string()),
            target_client: None,
            frames_sent: 100,
            duration_secs: 5,
            dry_run: true,
            findings: vec![],
            raw_output: None,
            recommendations: vec!["Enable 802.11w".to_string()],
        };
        tab.set_active_results(result);

        assert_eq!(tab.state, AppState::Completed);
        assert!(tab.active_results.is_some());
        let content = tab.results_view.get_content();
        assert!(content.contains("Active Attack: deauth"));
        assert!(content.contains("Dry Run: true"));
        assert!(content.contains("Frames Sent: 100"));
    }
}
