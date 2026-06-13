use crate::app::tab_error::TabError;
use crate::components::{empty_state_paragraph, ScrollableText};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::tc;
use crate::workers::TaskConfig;
use eggsec::proxy::intercept::types::{
    FlowAction, InterceptSession, ManipulationRecord, ProxyFlow,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Row, Table, TableState},
    Frame,
};

#[macro_export]
macro_rules! inner {
    ($area:expr, $margin:expr) => {
        Rect::new($area.x + $margin, $area.y + $margin, $area.width - $margin * 2, $area.height - $margin * 2)
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterceptFocusArea {
    FlowList,
    DetailView,
    ActionBar,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailPane {
    Headers,
    Body,
    Manipulations,
    Rules,
    WebSocket,
    Http2,
    Grpc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolView {
    Http,
    WebSocket,
    Http2,
    Grpc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleManagementView {
    Legacy,
    Enhanced,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterceptView {
    Live,
    Session,
    Rules,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditTarget {
    RequestHeader(String),
    ResponseHeader(String),
    RequestBody,
    ResponseBody,
    Path,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditModalState {
    Closed,
    SelectingField,
    EditingValue,
    DiffPreview,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditModal {
    pub state: EditModalState,
    pub target: Option<EditTarget>,
    pub original_value: String,
    pub edit_buffer: String,
    pub reason: String,
    pub flow_index: u64,
    pub direction: eggsec::proxy::intercept::types::ProxyFlowDirection,
}

pub struct InterceptTab {
    pub flows: Vec<ProxyFlow>,
    pub selected_flow: Option<usize>,
    pub detail_pane: DetailPane,
    pub focus_area: InterceptFocusArea,
    pub current_view: InterceptView,
    pub state: AppState,
    pub results_view: ScrollableText,
    pub error: Option<TabError>,
    pub session: Option<InterceptSession>,
    pub dry_run: bool,
    pub listen_addr: String,
    pub manipulation_history: Vec<ManipulationRecord>,
    pub table_state: TableState,
    pub action_bar_index: usize,
    pub max_flows: u64,
    pub edit_modal: EditModal,
    pub pending_action: Option<crate::app::action::UiAction>,
    pub actions_log: Vec<String>,
    pub selected_protocol_view: ProtocolView,
    pub selected_rule_view: RuleManagementView,
}

impl InterceptTab {
    pub fn new() -> Self {
        Self {
            flows: Vec::new(),
            selected_flow: None,
            detail_pane: DetailPane::Headers,
            focus_area: InterceptFocusArea::FlowList,
            current_view: InterceptView::Live,
            state: AppState::Idle,
            results_view: ScrollableText::new("Details"),
            error: None,
            session: None,
            dry_run: true,
            listen_addr: "127.0.0.1:8080".to_string(),
            manipulation_history: Vec::new(),
            table_state: TableState::default(),
            action_bar_index: 0,
            max_flows: 100,
            edit_modal: EditModal {
                state: EditModalState::Closed,
                target: None,
                original_value: String::new(),
                edit_buffer: String::new(),
                reason: String::new(),
                flow_index: 0,
                direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
            },
            pending_action: None,
            actions_log: Vec::new(),
            selected_protocol_view: ProtocolView::Http,
            selected_rule_view: RuleManagementView::Legacy,
        }
    }

    pub fn listen_addr(&self) -> String {
        self.listen_addr.clone()
    }

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn max_flows(&self) -> u64 {
        self.max_flows
    }

    pub fn primary_target(&self) -> Option<String> {
        self.session
            .as_ref()
            .and_then(|s| s.target.clone())
            .or_else(|| Some(self.listen_addr.clone()))
    }

    pub fn set_session(&mut self, session: InterceptSession) {
        self.flows = session.flows.clone();
        self.manipulation_history = session.manipulations.clone();
        self.session = Some(session);
        self.state = AppState::Completed;
        if !self.flows.is_empty() {
            self.selected_flow = Some(0);
            self.table_state.select(Some(0));
        }
    }

    pub fn add_flow(&mut self, flow: ProxyFlow) {
        let idx = self.flows.len();
        self.flows.push(flow);
        if self.selected_flow.is_none() {
            self.selected_flow = Some(idx);
            self.table_state.select(Some(idx));
        }
    }

    pub fn record_manipulation(&mut self, record: ManipulationRecord) {
        self.manipulation_history.push(record);
    }

    pub fn open_edit_modal(&mut self, target: EditTarget, original_value: String) {
        self.edit_modal = EditModal {
            state: EditModalState::EditingValue,
            target: Some(target.clone()),
            original_value: original_value.clone(),
            edit_buffer: original_value,
            reason: String::new(),
            flow_index: self.selected_flow.map(|i| i as u64).unwrap_or(0),
            direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
        };
    }

    pub fn close_edit_modal(&mut self) {
        self.edit_modal.state = EditModalState::Closed;
        self.edit_modal.target = None;
        self.edit_modal.original_value.clear();
        self.edit_modal.edit_buffer.clear();
        self.edit_modal.reason.clear();
    }

    pub fn apply_edit(&mut self) {
        if self.edit_modal.state != EditModalState::EditingValue && self.edit_modal.state != EditModalState::DiffPreview {
            return;
        }

        let target = self.edit_modal.target.clone();
        let before = self.edit_modal.original_value.clone();
        let after = self.edit_modal.edit_buffer.clone();
        let reason = self.edit_modal.reason.clone();
        let flow_index = self.edit_modal.flow_index;
        let direction = self.edit_modal.direction;

        if before != after {
            if let Some(idx) = self.selected_flow {
                if let Some(flow) = self.flows.get_mut(idx) {
                    match target.as_ref() {
                        Some(EditTarget::RequestHeader(name)) => {
                            flow.request_headers.insert(name.clone(), after.clone());
                        }
                        Some(EditTarget::ResponseHeader(name)) => {
                            flow.response_headers.insert(name.clone(), after.clone());
                        }
                        Some(EditTarget::RequestBody) => {
                            flow.request_body = Some(after.clone());
                        }
                        Some(EditTarget::ResponseBody) => {
                            flow.response_body = Some(after.clone());
                        }
                        Some(EditTarget::Path) => {
                            flow.path = after.clone();
                        }
                        None => {}
                    }
                }
            }

            let record = ManipulationRecord {
                flow_index,
                direction,
                field: match target.as_ref() {
                    Some(EditTarget::RequestHeader(n)) => format!("header:{}", n),
                    Some(EditTarget::ResponseHeader(n)) => format!("response:header:{}", n),
                    Some(EditTarget::RequestBody) => "request:body".to_string(),
                    Some(EditTarget::ResponseBody) => "response:body".to_string(),
                    Some(EditTarget::Path) => "path".to_string(),
                    None => "unknown".to_string(),
                },
                before: if before.is_empty() { None } else { Some(before) },
                after: if after.is_empty() { None } else { Some(after) },
                reason: if reason.is_empty() { "manual edit".to_string() } else { reason },
                timestamp: chrono::Utc::now().to_rfc3339(),
            };

            self.record_manipulation(record.clone());

            if let Some(ref mut session) = self.session {
                session.record_manipulation(record);
            }
        }

        self.close_edit_modal();
    }

    pub fn is_edit_modal_open(&self) -> bool {
        self.edit_modal.state != EditModalState::Closed
    }

    pub fn is_running(&self) -> bool {
        self.state == AppState::Running
    }

    pub fn start_session(&mut self) {
        self.state = AppState::Running;
        self.error = None;
        self.flows.clear();
        self.manipulation_history.clear();
        self.selected_flow = None;
        self.table_state.select(None);
        self.session = Some(InterceptSession::new(&self.listen_addr, self.dry_run));
    }

    pub fn stop_session(&mut self) {
        if let Some(ref mut session) = self.session {
            session.finalize();
        }
        self.state = AppState::Idle;
    }

    fn selected_flow_data(&self) -> Option<&ProxyFlow> {
        self.selected_flow.and_then(|i| self.flows.get(i))
    }

    fn render_flow_list(&self, f: &mut Frame, area: Rect) {
        if self.flows.is_empty() {
            let placeholder = empty_state_paragraph(
                "Captured Flows",
                "No flows captured yet. Configure your browser to use the proxy.",
            );
            f.render_widget(placeholder, area);
            return;
        }

        let header_cells = ["#", "Method", "Host", "Path", "Status", "Size", "HTTPS"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1);

        let rows = self.flows.iter().enumerate().map(|(i, flow)| {
            let status_color = if flow.response_status >= 200 && flow.response_status < 300 {
                tc!(success)
            } else if flow.response_status >= 400 {
                tc!(error)
            } else {
                tc!(text)
            };
            Row::new(vec![
                Cell::from(format!("{}", i)),
                Cell::from(flow.method.clone()),
                Cell::from(truncate_str(&flow.host, 20)),
                Cell::from(truncate_str(&flow.path, 25)),
                Cell::from(format!("{}", flow.response_status)).style(Style::default().fg(status_color)),
                Cell::from(format_bytes(flow.response_body_size)),
                Cell::from(if flow.is_https { "Y" } else { "N" }),
            ])
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(4),
                Constraint::Length(8),
                Constraint::Length(22),
                Constraint::Length(27),
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Length(5),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Flows ({}) ",
            self.flows.len()
        )))
        .highlight_style(Style::default().bg(tc!(selected)))
        .highlight_symbol("> ");

        f.render_stateful_widget(table, area, &mut self.table_state.clone());
    }

    fn render_detail_pane(&self, f: &mut Frame, area: Rect) {
        match self.selected_flow_data() {
            Some(flow) => match self.detail_pane {
                DetailPane::Headers => self.render_headers(f, area, flow),
                DetailPane::Body => self.render_body(f, area, flow),
                DetailPane::Manipulations => self.render_manipulations(f, area),
                DetailPane::Rules => self.render_rules_with_view(f, area),
                DetailPane::WebSocket => self.render_protocol_placeholder(f, area, "WebSocket"),
                DetailPane::Http2 => self.render_protocol_placeholder(f, area, "HTTP/2"),
                DetailPane::Grpc => self.render_protocol_placeholder(f, area, "gRPC"),
            },
            None => {
                let placeholder = empty_state_paragraph("Detail", "Select a flow to view details");
                f.render_widget(placeholder, area);
            }
        }
    }

    fn render_rules_with_view(&self, f: &mut Frame, area: Rect) {
        let rule_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        let legacy_style = if self.selected_rule_view == RuleManagementView::Legacy {
            Style::default().fg(tc!(background)).bg(tc!(accent)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tc!(text))
        };
        let enhanced_style = if self.selected_rule_view == RuleManagementView::Enhanced {
            Style::default().fg(tc!(background)).bg(tc!(accent)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tc!(text))
        };
        let selector = ratatui::widgets::Paragraph::new(Line::from(vec![
            Span::styled(" [Legacy] ", legacy_style),
            Span::raw(" "),
            Span::styled(" [Enhanced] ", enhanced_style),
        ]));
        f.render_widget(selector, rule_layout[0]);

        self.render_rules_content(f, rule_layout[1]);
    }

    fn render_rules_content(&self, f: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(vec![Span::styled(
                "Intercept Rules",
                Style::default()
                    .fg(tc!(accent))
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Rules control which traffic is intercepted:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Allow  ", Style::default().fg(tc!(success))),
                Span::raw("Pass through without inspection"),
            ]),
            Line::from(vec![
                Span::styled("  Block  ", Style::default().fg(tc!(error))),
                Span::raw("Reject the connection"),
            ]),
            Line::from(vec![
                Span::styled("  Intercept ", Style::default().fg(tc!(warning))),
                Span::raw("Pause for manual inspection"),
            ]),
            Line::from(vec![
                Span::styled("  Monitor ", Style::default().fg(tc!(info))),
                Span::raw("Log without pausing"),
            ]),
            Line::from(vec![
                Span::styled("  Modify ", Style::default().fg(Color::Magenta)),
                Span::raw("Apply automatic modifications"),
            ]),
            Line::from(""),
            Line::from("Use the CLI to configure rules:"),
            Line::from("  eggsec proxy intercept --intercept-rule <file>"),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Rules ({:?}) ", self.selected_rule_view));
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_protocol_placeholder(&self, f: &mut Frame, area: Rect, protocol: &str) {
        let lines = vec![
            Line::from(vec![Span::styled(
                format!("{} Protocol View", protocol),
                Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(format!("{} protocol inspection is not yet implemented.", protocol)),
            Line::from("Coming in a future phase."),
        ];
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", protocol));
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_headers(&self, f: &mut Frame, area: Rect, flow: &ProxyFlow) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Request Headers",
            Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
        )]));
        for (k, v) in &flow.request_headers {
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", k), Style::default().fg(tc!(info))),
                Span::raw(v.clone()),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Response Headers",
            Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(tc!(info))),
            Span::raw(format!("{}", flow.response_status)),
        ]));
        for (k, v) in &flow.response_headers {
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", k), Style::default().fg(tc!(info))),
                Span::raw(v.clone()),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Headers ");
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_body(&self, f: &mut Frame, area: Rect, flow: &ProxyFlow) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Request Body",
            Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
        )]));
        match &flow.request_body {
            Some(body) if !body.is_empty() => {
                for line in body.lines().take(20) {
                    lines.push(Line::from(Span::raw(line.to_string())));
                }
                if body.lines().count() > 20 {
                    lines.push(Line::from(Span::styled(
                        "... (truncated)",
                        Style::default().fg(tc!(muted)),
                    )));
                }
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    "  (empty)",
                    Style::default().fg(tc!(muted)),
                )));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Response Body",
            Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
        )]));
        match &flow.response_body {
            Some(body) if !body.is_empty() => {
                for line in body.lines().take(20) {
                    lines.push(Line::from(Span::raw(line.to_string())));
                }
                if body.lines().count() > 20 {
                    lines.push(Line::from(Span::styled(
                        "... (truncated)",
                        Style::default().fg(tc!(muted)),
                    )));
                }
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    "  (empty)",
                    Style::default().fg(tc!(muted)),
                )));
            }
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Body ");
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_manipulations(&self, f: &mut Frame, area: Rect) {
        if self.manipulation_history.is_empty() {
            let placeholder =
                empty_state_paragraph("Manipulations", "No manipulations recorded yet");
            f.render_widget(placeholder, area);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for (i, m) in self.manipulation_history.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}] ", i + 1),
                    Style::default().fg(tc!(info)),
                ),
                Span::styled(
                    format!("Flow #{} ", m.flow_index),
                    Style::default().fg(tc!(accent)),
                ),
                Span::raw(format!("{}: ", m.field)),
            ]));
            if let Some(ref before) = m.before {
                lines.push(Line::from(vec![
                    Span::styled("  - ", Style::default().fg(tc!(error))),
                    Span::raw(before.clone()),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("  + ", Style::default().fg(tc!(success))),
                Span::raw(m.after.clone().unwrap_or_default()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Reason: ", Style::default().fg(tc!(muted))),
                Span::raw(m.reason.clone()),
            ]));
            lines.push(Line::from(""));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Manipulations ({}) ", self.manipulation_history.len()));
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_action_bar(&self, f: &mut Frame, area: Rect) {
        let actions = ["Forward", "Drop", "Replay", "Pause All", "Resume All", "Save", "Export HAR"];
        let spans: Vec<Span> = actions
            .iter()
            .enumerate()
            .flat_map(|(i, action)| {
                let is_destructive = i == 1 || i == 2;
                let style = if i == self.action_bar_index {
                    if is_destructive {
                        Style::default()
                            .fg(Color::Red)
                            .bg(tc!(background))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(tc!(background))
                            .bg(tc!(accent))
                            .add_modifier(Modifier::BOLD)
                    }
                } else {
                    Style::default().fg(tc!(text))
                };
                vec![Span::styled(format!(" {} ", action), style), Span::raw(" ")]
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Actions (←/→ navigate · Enter execute · D=Drop R=Replay F=Forward · Esc=back ");
        let paragraph = ratatui::widgets::Paragraph::new(Line::from(spans)).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_edit_modal(&self, f: &mut Frame, area: Rect) {
        use ratatui::widgets::Paragraph;

        f.render_widget(Clear, area);
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(" Edit ")
                .style(Style::default().bg(tc!(surface)).fg(tc!(text))),
            area,
        );

        let modal_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .margin(1)
            .split(area);

        let field_name = match &self.edit_modal.target {
            Some(EditTarget::RequestHeader(n)) => format!("Request Header: {}", n),
            Some(EditTarget::ResponseHeader(n)) => format!("Response Header: {}", n),
            Some(EditTarget::RequestBody) => "Request Body".to_string(),
            Some(EditTarget::ResponseBody) => "Response Body".to_string(),
            Some(EditTarget::Path) => "Path".to_string(),
            None => "Unknown".to_string(),
        };

        let field_para = Paragraph::new(field_name)
            .style(Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD));
        f.render_widget(field_para, modal_layout[0]);

        let orig_label = Paragraph::new(format!("Original: {}", truncate_str(&self.edit_modal.original_value, 60)))
            .style(Style::default().fg(tc!(muted)));
        f.render_widget(orig_label, modal_layout[1]);

        let edit_area = modal_layout[2];
        let edit_block = Block::default()
            .borders(Borders::ALL)
            .title(" Edit Value (type to modify) ");

        let edit_content = if self.edit_modal.edit_buffer.is_empty() {
            "[empty - type to add]".to_string()
        } else {
            self.edit_modal.edit_buffer.clone()
        };
        let edit_para = Paragraph::new(edit_content)
            .style(Style::default().fg(tc!(text)));
        f.render_widget(edit_block, edit_area);
        let inner_rect = Rect::new(edit_area.x + 1, edit_area.y + 1, edit_area.width - 2, edit_area.height - 2);
        f.render_widget(edit_para, inner_rect);

        let diff_label = if self.edit_modal.original_value != self.edit_modal.edit_buffer {
            format!("~ Change: {} → {}",
                truncate_str(&self.edit_modal.original_value, 30),
                truncate_str(&self.edit_modal.edit_buffer, 30))
        } else {
            "(no change)".to_string()
        };
        let diff_para = Paragraph::new(diff_label)
            .style(Style::default().fg(tc!(warning)));
        f.render_widget(diff_para, modal_layout[4]);

        let reason_para = Paragraph::new("Reason: (optional) ")
            .style(Style::default().fg(tc!(muted)));
        f.render_widget(reason_para, modal_layout[5]);

        let help_text = "Enter=apply  Esc=cancel  Tab=switch focus";
        let help_para = Paragraph::new(help_text)
            .style(Style::default().fg(tc!(muted)));
        f.render_widget(help_para, modal_layout[6]);
    }

    pub fn set_protocol_view(&mut self, view: ProtocolView) {
        self.selected_protocol_view = view;
    }

    pub fn toggle_rule_view(&mut self) {
        self.selected_rule_view = match self.selected_rule_view {
            RuleManagementView::Legacy => RuleManagementView::Enhanced,
            RuleManagementView::Enhanced => RuleManagementView::Legacy,
        };
    }

    fn execute_action(&mut self, action_index: usize) {
        match action_index {
            0 => {
                if let Some(idx) = self.selected_flow {
                    if let Some(ref mut session) = self.session {
                        session.record_action(idx as u64, FlowAction::Forward);
                        self.actions_log.push(format!(
                            "[{}] Forward flow #{}",
                            chrono::Local::now().format("%H:%M:%S"),
                            idx
                        ));
                    }
                }
            }
            1 => {
                if let Some(idx) = self.selected_flow {
                    if let Some(ref mut session) = self.session {
                        session.record_action(idx as u64, FlowAction::Drop);
                        self.actions_log.push(format!(
                            "[{}] DROP flow #{} (NOT actually dropped - MITM not running)",
                            chrono::Local::now().format("%H:%M:%S"),
                            idx
                        ));
                    }
                }
            }
            2 => {
                if let Some(idx) = self.selected_flow {
                    if let Some(ref mut session) = self.session {
                        session.record_action(idx as u64, FlowAction::Replay);
                        self.actions_log.push(format!(
                            "[{}] REPLAY flow #{} (NOT actually replayed - MITM not running)",
                            chrono::Local::now().format("%H:%M:%S"),
                            idx
                        ));
                    }
                }
            }
            3 => {
                self.actions_log.push(format!(
                    "[{}] Pause all (not implemented - MITM server not running)",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }
            4 => {
                self.actions_log.push(format!(
                    "[{}] Resume all (not implemented - MITM server not running)",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }
            5 => {
                if let Some(ref session) = self.session {
                    let path = format!(
                        "intercept_session_{}.json",
                        chrono::Utc::now().format("%Y%m%d_%H%M%S")
                    );
                    match session.save_to_file(&path) {
                        Ok(_) => {
                            self.actions_log.push(format!(
                                "[{}] Session saved to {}",
                                chrono::Local::now().format("%H:%M:%S"),
                                path
                            ));
                        }
                        Err(e) => {
                            self.error = Some(TabError::Unknown(format!("Failed to save: {}", e)));
                        }
                    }
                }
            }
            6 => {
                if let Some(ref session) = self.session {
                    let har = session.to_har();
                    let path = format!(
                        "intercept_session_{}.har",
                        chrono::Utc::now().format("%Y%m%d_%H%M%S")
                    );
                    match serde_json::to_string_pretty(&har) {
                        Ok(json) => match std::fs::write(&path, json) {
                            Ok(_) => {
                                self.actions_log.push(format!(
                                    "[{}] HAR exported to {}",
                                    chrono::Local::now().format("%H:%M:%S"),
                                    path
                                ));
                            }
                            Err(e) => {
                                self.error = Some(TabError::Unknown(format!("Failed to write HAR: {}", e)));
                            }
                        },
                        Err(e) => {
                            self.error = Some(TabError::Unknown(format!("Failed to serialize HAR: {}", e)));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl Default for InterceptTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for InterceptTab {
    fn state(&self) -> AppState {
        self.state.clone()
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.flows.clear();
        self.selected_flow = None;
        self.manipulation_history.clear();
        self.session = None;
        self.results_view.clear();
        self.error = None;
        self.table_state.select(None);
        self.focus_area = InterceptFocusArea::FlowList;
        self.detail_pane = DetailPane::Headers;
        self.action_bar_index = 0;
        self.selected_protocol_view = ProtocolView::Http;
        self.selected_rule_view = RuleManagementView::Legacy;
    }

    fn set_error(&mut self, error: TabError) {
        self.state = AppState::Error(error.message());
        self.error = Some(error);
    }
}

impl TabRender for InterceptTab {
    fn breadcrumb(&self) -> Option<Vec<&'static str>> {
        let pane = match self.detail_pane {
            DetailPane::Headers => "Headers",
            DetailPane::Body => "Body",
            DetailPane::Manipulations => "Manipulations",
            DetailPane::Rules => "Rules",
            DetailPane::WebSocket => "WebSocket",
            DetailPane::Http2 => "HTTP/2",
            DetailPane::Grpc => "gRPC",
        };
        Some(vec!["Intercept", pane])
    }

    fn render(&self, f: &mut Frame, area: Rect, _insert_mode: bool) {
        if let Some(ref err) = self.error {
            let error_text = ratatui::widgets::Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Intercept - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        if self.is_edit_modal_open() {
            self.render_edit_modal(f, area);
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // status bar
                Constraint::Min(8),   // flow list + detail (split horizontal)
                Constraint::Length(3), // action bar
            ])
            .split(area);

        if area.width < 60 || area.height < 15 {
            let too_small = ratatui::widgets::Paragraph::new(
                "Terminal too small for Intercept tab.\nNeed at least 60x15."
            )
            .block(Block::default().borders(Borders::ALL).title(" Too Small "))
            .style(Style::default().fg(tc!(error)));
            f.render_widget(too_small, area);
            return;
        }

        let status_area = layout[0];
        let content_area = layout[1];
        let action_area = layout[2];

        // Status bar with enforcement posture badge
        let posture_badge = if self.dry_run {
            Span::styled(" DRY-RUN ", Style::default().fg(tc!(background)).bg(tc!(success)).add_modifier(Modifier::BOLD))
        } else if self.state == AppState::Running {
            Span::styled(" LIVE ", Style::default().fg(tc!(background)).bg(tc!(warning)).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" IDLE ", Style::default().fg(tc!(muted)))
        };

        let status_text = format!(
            " {} | {} | Flows: {} | {}",
            self.listen_addr,
            if self.state == AppState::Running { "ACTIVE" } else { "IDLE" },
            self.flows.len(),
            if self.dry_run { "DRY-RUN" } else { "LIVE" }
        );
        let status = ratatui::widgets::Paragraph::new(Line::from(vec![posture_badge, Span::raw(status_text)]))
        .block(Block::default().borders(Borders::ALL).title(" Status "))
        .style(Style::default().fg(if self.state == AppState::Running { tc!(success) } else { tc!(text) }));
        f.render_widget(status, status_area);

        // Content: flow list (left) + detail pane (right)
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(content_area);

        let flow_area = content_layout[0];
        let detail_area = content_layout[1];

        // Flow list
        self.render_flow_list(f, flow_area);

        // Detail pane tabs
        let tab_names = ["Headers", "Body", "Manipulations", "Rules"];
        let tab_line: Vec<Span> = tab_names
            .iter()
            .enumerate()
            .flat_map(|(i, name)| {
                let style = if DetailPane::from_index(i) == self.detail_pane {
                    Style::default()
                        .fg(tc!(background))
                        .bg(tc!(accent))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(tc!(text))
                };
                vec![Span::styled(format!(" {} ", name), style)]
            })
            .collect();

        let detail_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(detail_area);

        let tab_bar = ratatui::widgets::Paragraph::new(Line::from(tab_line));
        f.render_widget(tab_bar, detail_layout[0]);

        // Render the detail content
        let detail_self = self.clone_for_render();
        detail_self.render_detail_pane(f, detail_layout[1]);

        // Action bar
        self.render_action_bar(f, action_area);
    }

    fn render_overlays(&self, _f: &mut Frame, _area: Rect) {}
}

impl InterceptTab {
    fn clone_for_render(&self) -> Self {
        InterceptTab {
            flows: self.flows.clone(),
            selected_flow: self.selected_flow,
            detail_pane: self.detail_pane,
            focus_area: self.focus_area,
            current_view: self.current_view,
            state: self.state.clone(),
            results_view: ScrollableText::new("Details"),
            error: self.error.clone(),
            session: self.session.clone(),
            dry_run: self.dry_run,
            listen_addr: self.listen_addr.clone(),
            manipulation_history: self.manipulation_history.clone(),
            table_state: TableState::default(),
            action_bar_index: self.action_bar_index,
            max_flows: self.max_flows,
            edit_modal: self.edit_modal.clone(),
            pending_action: None,
            actions_log: self.actions_log.clone(),
            selected_protocol_view: self.selected_protocol_view.clone(),
            selected_rule_view: self.selected_rule_view.clone(),
        }
    }
}

impl TabInput for InterceptTab {
    fn stop(&mut self) {
        if self.state == AppState::Running {
            self.stop_session();
        }
    }

    fn handle_focus_next(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                InterceptFocusArea::FlowList => InterceptFocusArea::DetailView,
                InterceptFocusArea::DetailView => InterceptFocusArea::ActionBar,
                InterceptFocusArea::ActionBar => InterceptFocusArea::FlowList,
            };
        }
    }

    fn handle_focus_prev(&mut self) {
        if !self.is_running() {
            self.focus_area = match self.focus_area {
                InterceptFocusArea::FlowList => InterceptFocusArea::ActionBar,
                InterceptFocusArea::DetailView => InterceptFocusArea::FlowList,
                InterceptFocusArea::ActionBar => InterceptFocusArea::DetailView,
            };
        }
    }

    fn handle_char(&mut self, c: char) {
        if self.is_edit_modal_open() {
            self.edit_modal.edit_buffer.push(c);
            return;
        }
        if c == 'r' && self.detail_pane == DetailPane::Rules {
            self.toggle_rule_view();
        }
    }

    fn handle_backspace(&mut self) {
        if self.is_edit_modal_open() {
            self.edit_modal.edit_buffer.pop();
            return;
        }
    }

    fn handle_enter(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }

        if self.is_edit_modal_open() {
            self.apply_edit();
            return;
        }

        if self.focus_area == InterceptFocusArea::ActionBar {
            self.execute_action(self.action_bar_index);
        }
    }

    fn handle_escape(&mut self) {
        if self.is_running() {
            self.stop();
            return;
        }
        if self.is_edit_modal_open() {
            self.close_edit_modal();
            return;
        }
        self.focus_area = InterceptFocusArea::FlowList;
    }

    fn handle_up(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            InterceptFocusArea::FlowList => {
                let i = self.selected_flow.unwrap_or(0);
                if i > 0 {
                    self.selected_flow = Some(i - 1);
                    self.table_state.select(Some(i - 1));
                }
            }
            InterceptFocusArea::DetailView => {
                self.detail_pane = match self.detail_pane {
                    DetailPane::Headers => DetailPane::Rules,
                    DetailPane::Body => DetailPane::Headers,
                    DetailPane::Manipulations => DetailPane::Body,
                    DetailPane::Rules => DetailPane::Manipulations,
                    DetailPane::WebSocket => DetailPane::Grpc,
                    DetailPane::Http2 => DetailPane::WebSocket,
                    DetailPane::Grpc => DetailPane::Http2,
                };
            }
            InterceptFocusArea::ActionBar => {}
        }
    }

    fn handle_down(&mut self) {
        if self.is_running() {
            return;
        }
        match self.focus_area {
            InterceptFocusArea::FlowList => {
                let i = self.selected_flow.unwrap_or(0);
                if i + 1 < self.flows.len() {
                    self.selected_flow = Some(i + 1);
                    self.table_state.select(Some(i + 1));
                }
            }
            InterceptFocusArea::DetailView => {
                self.detail_pane = match self.detail_pane {
                    DetailPane::Headers => DetailPane::Body,
                    DetailPane::Body => DetailPane::Manipulations,
                    DetailPane::Manipulations => DetailPane::Rules,
                    DetailPane::Rules => DetailPane::Headers,
                    DetailPane::WebSocket => DetailPane::Http2,
                    DetailPane::Http2 => DetailPane::Grpc,
                    DetailPane::Grpc => DetailPane::WebSocket,
                };
            }
            InterceptFocusArea::ActionBar => {}
        }
    }

    fn handle_left(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == InterceptFocusArea::ActionBar {
            if self.action_bar_index > 0 {
                self.action_bar_index -= 1;
            }
            true
        } else {
            false
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.is_running() {
            return false;
        }
        if self.focus_area == InterceptFocusArea::ActionBar {
            if self.action_bar_index < 6 {
                self.action_bar_index += 1;
            }
            true
        } else {
            false
        }
    }

    fn is_input_focused(&self) -> bool {
        false
    }

    fn is_at_left_edge(&self) -> bool {
        if self.focus_area == InterceptFocusArea::ActionBar {
            self.action_bar_index == 0
        } else {
            true
        }
    }

    fn is_at_right_edge(&self) -> bool {
        if self.focus_area == InterceptFocusArea::ActionBar {
            self.action_bar_index >= 6
        } else {
            true
        }
    }

    fn page_up(&mut self, _page_size: usize) {
        if !self.is_running() && self.focus_area == InterceptFocusArea::FlowList {
            let i = self.selected_flow.unwrap_or(0);
            let new_i = i.saturating_sub(20);
            self.selected_flow = Some(new_i);
            self.table_state.select(Some(new_i));
        }
    }

    fn page_down(&mut self, _page_size: usize) {
        if !self.is_running() && self.focus_area == InterceptFocusArea::FlowList {
            let i = self.selected_flow.unwrap_or(0);
            let new_i = (i + 20).min(self.flows.len().saturating_sub(1));
            self.selected_flow = Some(new_i);
            self.table_state.select(Some(new_i));
        }
    }

    fn handle_top(&mut self) {
        if !self.is_running() {
            self.focus_area = InterceptFocusArea::FlowList;
            self.selected_flow = Some(0);
            self.table_state.select(Some(0));
        }
    }

    fn handle_bottom(&mut self) {
        if !self.is_running() {
            self.focus_area = InterceptFocusArea::FlowList;
            let last = self.flows.len().saturating_sub(1);
            self.selected_flow = Some(last);
            self.table_state.select(Some(last));
        }
    }
}

impl DetailPane {
    fn from_index(i: usize) -> Self {
        match i {
            0 => DetailPane::Headers,
            1 => DetailPane::Body,
            2 => DetailPane::Manipulations,
            3 => DetailPane::Rules,
            4 => DetailPane::WebSocket,
            5 => DetailPane::Http2,
            6 => DetailPane::Grpc,
            _ => DetailPane::Headers,
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intercept_tab_new() {
        let tab = InterceptTab::new();
        assert!(tab.flows.is_empty());
        assert!(tab.manipulation_history.is_empty());
        assert_eq!(tab.state, AppState::Idle);
        assert!(tab.dry_run);
        assert_eq!(tab.listen_addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_intercept_tab_start_stop() {
        let mut tab = InterceptTab::new();
        tab.start_session();
        assert_eq!(tab.state, AppState::Running);
        assert!(tab.session.is_some());

        tab.stop_session();
        assert_eq!(tab.state, AppState::Idle);
        assert!(tab.session.as_ref().unwrap().ended_at != tab.session.as_ref().unwrap().started_at);
    }

    #[test]
    fn test_intercept_tab_add_flow() {
        let mut tab = InterceptTab::new();
        let flow = ProxyFlow {
            index: 0,
            method: "GET".to_string(),
            url: "https://example.com/".to_string(),
            host: "example.com".to_string(),
            path: "/".to_string(),
            request_headers: Default::default(),
            request_body: None,
            response_status: 200,
            response_headers: Default::default(),
            response_body: None,
            is_https: true,
            duration_ms: 100,
            request_body_size: 0,
            response_body_size: 512,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        };
        tab.add_flow(flow);
        assert_eq!(tab.flows.len(), 1);
        assert_eq!(tab.selected_flow, Some(0));
    }

    #[test]
    fn test_intercept_tab_focus_navigation() {
        let mut tab = InterceptTab::new();
        assert_eq!(tab.focus_area, InterceptFocusArea::FlowList);
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, InterceptFocusArea::DetailView);
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, InterceptFocusArea::ActionBar);
        tab.handle_focus_next();
        assert_eq!(tab.focus_area, InterceptFocusArea::FlowList);
    }

    #[test]
    fn test_intercept_tab_action_bar_navigation() {
        let mut tab = InterceptTab::new();
        tab.focus_area = InterceptFocusArea::ActionBar;
        assert_eq!(tab.action_bar_index, 0);
        tab.handle_right();
        assert_eq!(tab.action_bar_index, 1);
        tab.handle_right();
        assert_eq!(tab.action_bar_index, 2);
        tab.handle_left();
        assert_eq!(tab.action_bar_index, 1);
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512B");
        assert_eq!(format_bytes(1536), "1.5K");
        assert_eq!(format_bytes(2097152), "2.0M");
    }
}
