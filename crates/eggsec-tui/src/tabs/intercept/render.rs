use super::types::*;
use super::utils::{format_bytes, truncate_str};
use super::InterceptTab;
use crate::components::empty_state_paragraph;
use crate::tc;
use eggsec::proxy::intercept::correlation::{
    ConfidenceScorer, CorrelationEngine, CorrelationSource,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Row, Table, TableState},
    Frame,
};

impl InterceptTab {
    pub(super) fn render_flow_list(&self, f: &mut Frame, area: Rect) {
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
            .map(|h| {
                Cell::from(*h).style(
                    Style::default()
                        .fg(tc!(accent))
                        .add_modifier(Modifier::BOLD),
                )
            });
        let header = Row::new(header_cells).height(1);

        let viewport_height = area.height.saturating_sub(3) as usize;
        let visible = self.visible_flows(viewport_height);

        let rows = visible.iter().enumerate().map(|(offset, flow)| {
            let actual_index = self.scroll_offset + offset;
            let status_color = if flow.response_status >= 200 && flow.response_status < 300 {
                tc!(success)
            } else if flow.response_status >= 400 {
                tc!(error)
            } else {
                tc!(text)
            };
            Row::new(vec![
                Cell::from(format!("{}", actual_index)),
                Cell::from(flow.method.clone()),
                Cell::from(truncate_str(&flow.host, 20)),
                Cell::from(truncate_str(&flow.path, 25)),
                Cell::from(format!("{}", flow.response_status))
                    .style(Style::default().fg(status_color)),
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tc!(border)))
                .title(format!(" Flows ({}) ", self.flows.len())),
        )
        .highlight_style(Style::default().bg(tc!(selected)))
        .highlight_symbol("> ");

        let mut table_state = self.table_state.clone();
        if let Some(selected) = self.selected_flow {
            table_state.select(Some(selected.saturating_sub(self.scroll_offset)));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    pub(super) fn render_detail_pane(&self, f: &mut Frame, area: Rect) {
        match self.selected_flow_data() {
            Some(flow) => match self.detail_pane {
                DetailPane::Headers => self.render_headers(f, area, flow),
                DetailPane::Body => self.render_body(f, area, flow),
                DetailPane::Manipulations => self.render_manipulations(f, area),
                DetailPane::Rules => self.render_rules_with_view(f, area),
                DetailPane::Timeline => self.render_timeline(f, area),
                DetailPane::WebSocket => self.render_protocol_info(f, area, "WebSocket"),
                DetailPane::Http2 => self.render_protocol_info(f, area, "HTTP/2"),
                DetailPane::Grpc => self.render_protocol_info(f, area, "gRPC"),
                DetailPane::StreamMux => self.render_stream_multiplexing(f, area),
                DetailPane::Correlation => self.render_correlation(f, area),
            },
            None => {
                if self.detail_pane == DetailPane::Timeline {
                    self.render_timeline(f, area);
                } else if self.detail_pane == DetailPane::StreamMux {
                    self.render_stream_multiplexing(f, area);
                } else if self.detail_pane == DetailPane::Correlation {
                    self.render_correlation(f, area);
                } else {
                    let placeholder =
                        empty_state_paragraph("Detail", "Select a flow to view details");
                    f.render_widget(placeholder, area);
                }
            }
        }
    }

    fn render_rules_with_view(&self, f: &mut Frame, area: Rect) {
        let rule_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        let legacy_style = if self.selected_rule_view == RuleManagementView::Legacy {
            Style::default()
                .fg(tc!(background))
                .bg(tc!(accent))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tc!(text))
        };
        let enhanced_style = if self.selected_rule_view == RuleManagementView::Enhanced {
            Style::default()
                .fg(tc!(background))
                .bg(tc!(accent))
                .add_modifier(Modifier::BOLD)
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
        let mut lines = vec![
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
                Span::styled("  Modify ", Style::default().fg(tc!(accent))),
                Span::raw("Apply automatic modifications"),
            ]),
            Line::from(""),
            Line::from("Use the CLI to configure rules:"),
            Line::from("  eggsec proxy intercept --intercept-rule <file>"),
        ];

        if self.selected_rule_view == RuleManagementView::Enhanced {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                " Enhanced Rules",
                Style::default()
                    .fg(tc!(accent))
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Condition Types: ", Style::default().fg(tc!(info))), Span::raw("HostMatches, PathMatches, MethodMatches, HeaderContains, BodyContains, BodySizeGt/Lt")]));
            lines.push(Line::from(vec![
                Span::styled("  Combinators: ", Style::default().fg(tc!(info))),
                Span::raw("AND, OR, NOT for complex conditions"),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Protocol: ", Style::default().fg(tc!(info))),
                Span::raw("ProtocolIs, WebSocketOpcodeIs, GrpcMethodIs"),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Actions: ", Style::default().fg(tc!(info))),
                Span::raw("Allow, Block, Intercept, Monitor, Modify, InjectResponse, Delay, Tag"),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Persistence: ", Style::default().fg(tc!(info))),
                Span::raw("JSON save/load via EnhancedRuleSet"),
            ]));
            lines.push(Line::from(
                "  eggsec proxy intercept --rule-set /path/to/rules.json",
            ));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc!(border)))
            .title(format!(" Rules ({:?}) ", self.selected_rule_view));
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_protocol_info(&self, f: &mut Frame, area: Rect, protocol: &str) {
        let flow = match self.selected_flow_data() {
            Some(f) => f,
            None => {
                let placeholder =
                    empty_state_paragraph("Protocol", "Select a flow to view protocol details");
                f.render_widget(placeholder, area);
                return;
            }
        };

        let flow_protocol = &flow.protocol;
        let mut lines = vec![
            Line::from(vec![Span::styled(
                format!("{} Protocol Details", protocol),
                Style::default()
                    .fg(tc!(accent))
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Flow Protocol: ", Style::default().fg(tc!(info))),
                Span::raw(flow_protocol.clone()),
            ]),
            Line::from(""),
        ];

        match protocol {
            "WebSocket" => {
                lines.push(Line::from(vec![
                    Span::styled(" Detection: ", Style::default().fg(tc!(info))),
                    Span::raw("Check Upgrade header for 'websocket' value"),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" Capture: ", Style::default().fg(tc!(info))),
                    Span::raw("WebSocket sessions captured during MITM interception"),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" Manipulation: ", Style::default().fg(tc!(info))),
                    Span::raw("Edit and replay individual frames"),
                ]));
                lines.push(Line::from(""));

                if !self.ws_sessions.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled(" Captured Sessions: ", Style::default().fg(tc!(info))),
                        Span::raw(format!("{}", self.ws_sessions.len())),
                    ]));

                    for (session_idx, session) in self.ws_sessions.iter().enumerate() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  Session {}: ", session_idx),
                                Style::default()
                                    .fg(tc!(accent))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!(
                                "{} ({} messages)",
                                session.host,
                                session.messages.len()
                            )),
                        ]));

                        let page_size = 10;
                        let messages = self.ws_messages_page(session_idx, 0, page_size);
                        for msg in messages.iter() {
                            let prefix = if msg.direction
                                == eggsec::proxy::intercept::types::ProxyFlowDirection::Request
                            {
                                "  -> "
                            } else {
                                " <-  "
                            };
                            lines.push(Line::from(vec![
                                Span::raw(prefix.to_string()),
                                Span::styled(
                                    format!("{:?}", msg.opcode),
                                    Style::default().fg(tc!(text)),
                                ),
                                Span::raw(format!(" {}", truncate_str(&msg.payload, 60))),
                            ]));
                        }

                        let total = self.ws_session_message_count(session_idx);
                        if total > page_size {
                            lines.push(Line::from(vec![Span::styled(
                                format!(
                                    "  ... {} more messages (scroll to see more)",
                                    total - page_size
                                ),
                                Style::default().fg(tc!(muted)),
                            )]));
                        }
                    }
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(" Note: ", Style::default().fg(tc!(warning))),
                        Span::raw("No WebSocket sessions captured yet"),
                    ]));
                }
            }
            "HTTP/2" => {
                lines.push(Line::from(vec![
                    Span::styled(" Detection: ", Style::default().fg(tc!(info))),
                    Span::raw("HTTP/2 identified by :scheme pseudo-header"),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" Capture: ", Style::default().fg(tc!(info))),
                    Span::raw("HTTP/2 streams with ID, priority, window updates"),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" Streams: ", Style::default().fg(tc!(info))),
                    Span::raw("Multiplexed over single TCP connection"),
                ]));
                lines.push(Line::from(""));

                if !self.http2_sessions.is_empty() {
                    let total_streams = self.total_http2_streams();
                    lines.push(Line::from(vec![
                        Span::styled(" Captured Sessions: ", Style::default().fg(tc!(info))),
                        Span::raw(format!(
                            "{} ({} total streams)",
                            self.http2_sessions.len(),
                            total_streams
                        )),
                    ]));
                    lines.push(Line::from(""));

                    if let Some(session) = self.http2_sessions.first() {
                        let (open, half_closed, closed, idle) = self.http2_stream_state_counts(0);
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  Session {}: ", 0),
                                Style::default()
                                    .fg(tc!(accent))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!(
                                "{}{} ({} streams)",
                                if session.is_secure {
                                    "https://"
                                } else {
                                    "http://"
                                },
                                session.host,
                                session.streams.len()
                            )),
                        ]));

                        lines.push(Line::from(vec![
                            Span::styled("    Stream States: ", Style::default().fg(tc!(info))),
                            Span::raw(format!(
                                "open={}, half-closed={}, closed={}, idle={}",
                                open, half_closed, closed, idle
                            )),
                        ]));

                        lines.push(Line::from(vec![
                            Span::styled("    Connection Window: ", Style::default().fg(tc!(info))),
                            Span::raw(format!(
                                "{} bytes ({} stream window)",
                                session.connection_window_size, session.stream_window_size
                            )),
                        ]));

                        if session.max_concurrent_streams > 0 {
                            lines.push(Line::from(vec![
                                Span::styled(
                                    "    Max Concurrent: ",
                                    Style::default().fg(tc!(info)),
                                ),
                                Span::raw(format!(
                                    "{} streams, max frame: {} bytes",
                                    session.max_concurrent_streams, session.max_frame_size
                                )),
                            ]));
                        }

                        let streams = self.http2_streams_for_session(0);
                        let display_count = streams.len().min(5);
                        if display_count > 0 {
                            lines.push(Line::from(""));
                            lines.push(Line::from(vec![Span::styled(
                                "    Streams:",
                                Style::default().fg(tc!(info)),
                            )]));
                            for stream in streams.iter().take(display_count) {
                                let state_str = match stream.state {
                                    eggsec::proxy::intercept::protocols::Http2StreamState::Open => "OPEN",
                                    eggsec::proxy::intercept::protocols::Http2StreamState::HalfClosedLocal => "HALF-CLOSED-LOCAL",
                                    eggsec::proxy::intercept::protocols::Http2StreamState::HalfClosedRemote => "HALF-CLOSED-REMOTE",
                                    eggsec::proxy::intercept::protocols::Http2StreamState::Closed => "CLOSED",
                                    eggsec::proxy::intercept::protocols::Http2StreamState::Idle => "IDLE",
                                };
                                let state_color = match stream.state {
                                    eggsec::proxy::intercept::protocols::Http2StreamState::Open => tc!(success),
                                    eggsec::proxy::intercept::protocols::Http2StreamState::Closed => tc!(muted),
                                    _ => tc!(warning),
                                };
                                lines.push(Line::from(vec![
                                    Span::raw(format!("      [{}] ", stream.stream_id)),
                                    Span::styled(
                                        format!("{:<6}", state_str),
                                        Style::default().fg(state_color),
                                    ),
                                    Span::raw(format!(
                                        " {} {}",
                                        stream.method,
                                        truncate_str(&stream.path, 40)
                                    )),
                                ]));
                            }
                            if streams.len() > display_count {
                                lines.push(Line::from(Span::styled(
                                    format!(
                                        "      ... {} more streams (see Stream Mux tab)",
                                        streams.len() - display_count
                                    ),
                                    Style::default().fg(tc!(muted)),
                                )));
                            }
                        }
                    }
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(" Tip: ", Style::default().fg(tc!(success))),
                        Span::raw("Switch to 'Stream Mux' tab for full multiplexing visualization"),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(" Note: ", Style::default().fg(tc!(warning))),
                        Span::raw("No HTTP/2 sessions captured yet"),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled(" Tip: ", Style::default().fg(tc!(success))),
                        Span::raw("Stream demultiplexing available in 'Stream Mux' tab"),
                    ]));
                }
            }
            "gRPC" => {
                lines.push(Line::from(vec![
                    Span::styled(" Detection: ", Style::default().fg(tc!(info))),
                    Span::raw("gRPC identified by Content-Type: application/grpc*"),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" Capture: ", Style::default().fg(tc!(info))),
                    Span::raw("gRPC calls with method type, metadata, body"),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" Methods: ", Style::default().fg(tc!(info))),
                    Span::raw("Unary, ServerStreaming, ClientStreaming, Bidirectional"),
                ]));
                lines.push(Line::from(""));

                if !self.grpc_sessions.is_empty() {
                    let total_calls: usize = self.grpc_sessions.iter().map(|s| s.calls.len()).sum();
                    let streaming_calls: usize = self
                        .grpc_sessions
                        .iter()
                        .map(|s| s.streaming_call_count())
                        .sum();
                    let error_calls: usize = self
                        .grpc_sessions
                        .iter()
                        .flat_map(|s| s.error_calls())
                        .count();

                    lines.push(Line::from(vec![
                        Span::styled(" Captured Sessions: ", Style::default().fg(tc!(info))),
                        Span::raw(format!(
                            "{} ({} total calls, {} streaming, {} errors)",
                            self.grpc_sessions.len(),
                            total_calls,
                            streaming_calls,
                            error_calls
                        )),
                    ]));
                    lines.push(Line::from(""));

                    if let Some(session) = self.grpc_sessions.first() {
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  Session {}: ", 0),
                                Style::default()
                                    .fg(tc!(accent))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!(
                                "{}{} ({} calls)",
                                if session.is_secure {
                                    "https://"
                                } else {
                                    "http://"
                                },
                                session.host,
                                session.calls.len()
                            )),
                        ]));

                        let display_count = session.calls.len().min(5);
                        if display_count > 0 {
                            lines.push(Line::from(""));
                            lines.push(Line::from(vec![Span::styled(
                                "    Calls:",
                                Style::default().fg(tc!(info)),
                            )]));
                            for call in session.calls.iter().take(display_count) {
                                let type_str = match call.method_type {
                                    eggsec::proxy::intercept::protocols::GrpcMethodType::Unary => "UNARY",
                                    eggsec::proxy::intercept::protocols::GrpcMethodType::ServerStreaming => "SERVER-STREAM",
                                    eggsec::proxy::intercept::protocols::GrpcMethodType::ClientStreaming => "CLIENT-STREAM",
                                    eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional => "BIDI",
                                };
                                let type_color = match call.method_type {
                                    eggsec::proxy::intercept::protocols::GrpcMethodType::Unary => tc!(text),
                                    eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional => tc!(accent),
                                    _ => tc!(info),
                                };
                                lines.push(Line::from(vec![
                                    Span::raw(format!("      [{:<13}] ", type_str)),
                                    Span::styled(
                                        type_str.to_string(),
                                        Style::default().fg(type_color),
                                    ),
                                    Span::raw(format!(" {}", truncate_str(&call.path, 50))),
                                ]));
                            }
                            if session.calls.len() > display_count {
                                lines.push(Line::from(Span::styled(
                                    format!(
                                        "      ... {} more calls",
                                        session.calls.len() - display_count
                                    ),
                                    Style::default().fg(tc!(muted)),
                                )));
                            }
                        }
                    }

                    if !self.grpc_security_findings.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(vec![
                            Span::styled(" Security Findings: ", Style::default().fg(tc!(warning))),
                            Span::raw(format!("{}", self.grpc_security_findings.len())),
                        ]));
                        for finding in self.grpc_security_findings.iter().take(3) {
                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("    - {}: ", finding.category),
                                    Style::default().fg(tc!(warning)),
                                ),
                                Span::raw(truncate_str(&finding.description, 50)),
                            ]));
                        }
                        if self.grpc_security_findings.len() > 3 {
                            lines.push(Line::from(Span::styled(
                                format!(
                                    "    ... {} more findings",
                                    self.grpc_security_findings.len() - 3
                                ),
                                Style::default().fg(tc!(muted)),
                            )));
                        }
                    }

                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(" Tip: ", Style::default().fg(tc!(success))),
                        Span::raw("Switch to 'Stream Mux' tab for streaming frame visualization"),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(" Note: ", Style::default().fg(tc!(warning))),
                        Span::raw("No gRPC sessions captured yet"),
                    ]));
                }
            }
            _ => {}
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc!(border)))
            .title(format!(" {} ", protocol));
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_stream_multiplexing(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Stream Multiplexing",
            Style::default()
                .fg(tc!(accent))
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![Span::styled(
            " HTTP/2 Streams",
            Style::default().fg(tc!(info)).add_modifier(Modifier::BOLD),
        )]));

        if self.http2_sessions.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No HTTP/2 sessions captured yet",
                Style::default().fg(tc!(muted)),
            )));
        } else {
            let session_idx = self
                .selected_http2_session
                .min(self.http2_sessions.len() - 1);
            let session = &self.http2_sessions[session_idx];

            lines.push(Line::from(vec![
                Span::styled("  Session: ", Style::default().fg(tc!(info))),
                Span::raw(format!(
                    "{} ({} of {}, [<]/[>] to cycle)",
                    session.host,
                    session_idx + 1,
                    self.http2_sessions.len()
                )),
            ]));

            let (open, half_closed, closed, idle) = self.http2_stream_state_counts(session_idx);
            lines.push(Line::from(vec![
                Span::styled("  States: ", Style::default().fg(tc!(info))),
                Span::styled(format!("OPEN:{} ", open), Style::default().fg(tc!(success))),
                Span::styled(
                    format!("HALF:{} ", half_closed),
                    Style::default().fg(tc!(warning)),
                ),
                Span::styled(
                    format!("CLOSED:{} ", closed),
                    Style::default().fg(tc!(muted)),
                ),
                Span::styled(format!("IDLE:{} ", idle), Style::default().fg(tc!(text))),
            ]));

            lines.push(Line::from(vec![
                Span::styled("  Windows: ", Style::default().fg(tc!(info))),
                Span::raw(format!(
                    "conn={}B stream={}B max-frame={}B max-streams={}",
                    session.connection_window_size,
                    session.stream_window_size,
                    session.max_frame_size,
                    session.max_concurrent_streams
                )),
            ]));

            let streams = self.http2_streams_for_session(session_idx);
            if !streams.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "  Stream Timeline:",
                    Style::default().fg(tc!(info)),
                )]));

                for stream in streams.iter().take(20) {
                    let (marker, color) = match stream.state {
                        eggsec::proxy::intercept::protocols::Http2StreamState::Open => {
                            ("[OPEN    ]", tc!(success))
                        }
                        eggsec::proxy::intercept::protocols::Http2StreamState::HalfClosedLocal => {
                            ("[HALF-L  ]", tc!(warning))
                        }
                        eggsec::proxy::intercept::protocols::Http2StreamState::HalfClosedRemote => {
                            ("[HALF-R  ]", tc!(warning))
                        }
                        eggsec::proxy::intercept::protocols::Http2StreamState::Closed => {
                            ("[CLOSED  ]", tc!(muted))
                        }
                        eggsec::proxy::intercept::protocols::Http2StreamState::Idle => {
                            ("[IDLE    ]", tc!(text))
                        }
                    };
                    let dur_hint = if stream.closed_at.is_some() {
                        "DONE"
                    } else {
                        "LIVE"
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("    {:>3} ", stream.stream_id),
                            Style::default().fg(tc!(accent)),
                        ),
                        Span::styled(marker, Style::default().fg(color)),
                        Span::raw(format!(
                            " {} {} ({})",
                            stream.method,
                            truncate_str(&stream.path, 30),
                            dur_hint
                        )),
                    ]));
                }
                if streams.len() > 20 {
                    lines.push(Line::from(Span::styled(
                        format!("    ... {} more streams", streams.len() - 20),
                        Style::default().fg(tc!(muted)),
                    )));
                }
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            " gRPC Streaming Frames",
            Style::default().fg(tc!(info)).add_modifier(Modifier::BOLD),
        )]));

        if self.grpc_streaming_states.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No gRPC streaming state captured yet",
                Style::default().fg(tc!(muted)),
            )));
        } else {
            let total_frames = self.total_grpc_stream_frames();
            lines.push(Line::from(vec![
                Span::styled("  Total: ", Style::default().fg(tc!(info))),
                Span::raw(format!(
                    "{} streaming call(s), {} frames",
                    self.grpc_streaming_states.len(),
                    total_frames
                )),
            ]));

            for (idx, state) in self.grpc_streaming_states.iter().enumerate() {
                let summary = state.summary();
                let type_str = match summary.method_type {
                    eggsec::proxy::intercept::protocols::GrpcMethodType::Unary => "UNARY",
                    eggsec::proxy::intercept::protocols::GrpcMethodType::ServerStreaming => {
                        "SERVER-STREAM"
                    }
                    eggsec::proxy::intercept::protocols::GrpcMethodType::ClientStreaming => {
                        "CLIENT-STREAM"
                    }
                    eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional => "BIDI",
                };
                let type_color = match summary.method_type {
                    eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional => {
                        tc!(accent)
                    }
                    eggsec::proxy::intercept::protocols::GrpcMethodType::ServerStreaming => {
                        tc!(info)
                    }
                    _ => tc!(text),
                };
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  Stream #{}: ", idx),
                        Style::default().fg(tc!(info)),
                    ),
                    Span::styled(type_str, Style::default().fg(type_color)),
                    Span::raw(format!(
                        " | {} client / {} server",
                        summary.client_frame_count, summary.server_frame_count
                    )),
                ]));

                let pct = if summary.flow_control_window > 0 {
                    (summary.bytes_in_flight as f64 / summary.flow_control_window as f64 * 100.0)
                        .min(100.0)
                } else {
                    0.0
                };
                let bar_width = 20;
                let filled = (pct / 100.0 * bar_width as f64) as usize;
                let bar: String = std::iter::repeat('#')
                    .take(filled)
                    .chain(std::iter::repeat('-').take(bar_width - filled))
                    .collect();
                let bar_color = if pct < 50.0 {
                    tc!(success)
                } else if pct < 80.0 {
                    tc!(warning)
                } else {
                    tc!(error)
                };
                lines.push(Line::from(vec![
                    Span::styled("    Flow Window: ", Style::default().fg(tc!(info))),
                    Span::styled(format!("[{}] ", bar), Style::default().fg(bar_color)),
                    Span::raw(format!(
                        "{}/{}B ({:.0}%)",
                        summary.bytes_in_flight, summary.flow_control_window, pct
                    )),
                ]));

                let frames = self.grpc_stream_frames_page(idx, 0, 5);
                if !frames.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        "    Recent Frames:",
                        Style::default().fg(tc!(muted)),
                    )]));
                    for frame in frames {
                        let arrow = if frame.direction
                            == eggsec::proxy::intercept::types::ProxyFlowDirection::Request
                        {
                            "->"
                        } else {
                            "<-"
                        };
                        let end_marker = if frame.end_stream { " [END]" } else { "" };
                        lines.push(Line::from(vec![
                            Span::raw(format!("      {} ", arrow)),
                            Span::raw(format!("{}B{}", frame.size, end_marker)),
                            Span::styled(
                                format!(" @ {}", truncate_str(&frame.timestamp, 19)),
                                Style::default().fg(tc!(muted)),
                            ),
                        ]));
                    }
                    if summary.client_frame_count + summary.server_frame_count > 5 {
                        let remaining = summary.client_frame_count + summary.server_frame_count - 5;
                        lines.push(Line::from(Span::styled(
                            format!("      ... {} more frames", remaining),
                            Style::default().fg(tc!(muted)),
                        )));
                    }
                }

                if summary.is_complete {
                    lines.push(Line::from(Span::styled(
                        "    Status: COMPLETE",
                        Style::default().fg(tc!(success)),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "    Status: ACTIVE",
                        Style::default().fg(tc!(warning)),
                    )));
                }
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Keys: ", Style::default().fg(tc!(info))),
            Span::styled("[</>]", Style::default().fg(tc!(accent))),
            Span::raw(" cycle session"),
        ]));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc!(border)))
            .title(" Stream Multiplexing ");
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_correlation(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Cross-Loadout Correlation",
            Style::default()
                .fg(tc!(accent))
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled(" Summary: ", Style::default().fg(tc!(info))),
            Span::raw(self.correlation_summary_str()),
        ]));
        lines.push(Line::from(""));

        let source_counts = self.correlation_source_counts();
        if !source_counts.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                " By Source:",
                Style::default().fg(tc!(info)),
            )]));
            for (source, count) in &source_counts {
                let source_str = match source {
                    CorrelationSource::DbPentest => "DB-Pentest",
                    CorrelationSource::AuthTest => "Auth-Test",
                    CorrelationSource::MobileDynamic => "Mobile-Dynamic",
                    CorrelationSource::Wireless => "Wireless",
                    CorrelationSource::ProxyFlow => "Proxy-Flow",
                    CorrelationSource::External => "External",
                };
                let bar = std::iter::repeat('#').take(*count).collect::<String>();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {:<14} ", source_str),
                        Style::default().fg(tc!(info)),
                    ),
                    Span::styled(bar, Style::default().fg(tc!(accent))),
                    Span::raw(format!(" ({})", count)),
                ]));
            }
        } else {
            lines.push(Line::from(Span::styled(
                " No correlation references yet",
                Style::default().fg(tc!(muted)),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            " Temporal Correlations",
            Style::default().fg(tc!(info)).add_modifier(Modifier::BOLD),
        )]));
        if self.temporal_correlations.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No temporal correlations found",
                Style::default().fg(tc!(muted)),
            )));
        } else {
            for (i, t) in self.temporal_correlations.iter().take(8).enumerate() {
                let conf_pct = (t.confidence * 100.0) as u32;
                let conf_color = if conf_pct >= 70 {
                    tc!(success)
                } else if conf_pct >= 40 {
                    tc!(warning)
                } else {
                    tc!(muted)
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("  [{}] ", i + 1)),
                    Span::styled(
                        format!("{:?} <-> {:?} ", t.a.source, t.b.source),
                        Style::default().fg(tc!(accent)),
                    ),
                    Span::styled(format!("{}ms ", t.delta_ms), Style::default().fg(tc!(info))),
                    Span::styled(format!("({}%)", conf_pct), Style::default().fg(conf_color)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("      ", Style::default()),
                    Span::raw(truncate_str(&t.a.description, 50)),
                ]));
            }
            if self.temporal_correlations.len() > 8 {
                lines.push(Line::from(Span::styled(
                    format!(
                        "  ... {} more temporal correlations",
                        self.temporal_correlations.len() - 8
                    ),
                    Style::default().fg(tc!(muted)),
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            " Behavioral Pattern Matches",
            Style::default().fg(tc!(info)).add_modifier(Modifier::BOLD),
        )]));
        if self.behavioral_matches.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No behavioral patterns matched",
                Style::default().fg(tc!(muted)),
            )));
        } else {
            for (pattern, confidence) in &self.behavioral_matches {
                let conf_pct = (confidence * 100.0) as u32;
                let conf_color = if conf_pct >= 70 {
                    tc!(success)
                } else if conf_pct >= 40 {
                    tc!(warning)
                } else {
                    tc!(muted)
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  - {} ", pattern.id),
                        Style::default().fg(tc!(accent)),
                    ),
                    Span::raw(truncate_str(&pattern.description, 45)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("      ", Style::default()),
                    Span::styled(
                        format!("confidence: {}%", conf_pct),
                        Style::default().fg(conf_color),
                    ),
                    Span::raw(format!(
                        " ({} sources required)",
                        pattern.required_sources.len()
                    )),
                ]));
            }
        }

        if let Some(idx) = self.selected_flow {
            let flow_corrs = self.correlations_for_flow(idx as u64);
            if !flow_corrs.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    format!(" Correlations for Flow #{}", idx),
                    Style::default().fg(tc!(info)).add_modifier(Modifier::BOLD),
                )]));
                for r in flow_corrs.iter().take(5) {
                    let conf_pct = (r.confidence * 100.0) as u32;
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  -> {:?} ", r.source),
                            Style::default().fg(tc!(accent)),
                        ),
                        Span::raw(truncate_str(&r.description, 40)),
                        Span::styled(format!(" ({}%)", conf_pct), Style::default().fg(tc!(muted))),
                    ]));
                }
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(" Tip: ", Style::default().fg(tc!(success))),
            Span::raw("Correlations link proxy flows to findings from other loadouts (db-pentest, auth-test, mobile-dynamic)")]));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc!(border)))
            .title(" Correlation ");
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_headers(&self, f: &mut Frame, area: Rect, flow: &super::ProxyFlow) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Request Headers",
            Style::default()
                .fg(tc!(accent))
                .add_modifier(Modifier::BOLD),
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
            Style::default()
                .fg(tc!(accent))
                .add_modifier(Modifier::BOLD),
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
            .border_style(Style::default().fg(tc!(border)))
            .title(" Headers ");
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_body(&self, f: &mut Frame, area: Rect, flow: &super::ProxyFlow) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Request Body",
            Style::default()
                .fg(tc!(accent))
                .add_modifier(Modifier::BOLD),
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
            Style::default()
                .fg(tc!(accent))
                .add_modifier(Modifier::BOLD),
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
            .border_style(Style::default().fg(tc!(border)))
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
                Span::styled(format!("[{}] ", i + 1), Style::default().fg(tc!(info))),
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
            .border_style(Style::default().fg(tc!(border)))
            .title(format!(
                " Manipulations ({}) ",
                self.manipulation_history.len()
            ));
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_timeline(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Session Timeline",
            Style::default()
                .fg(tc!(accent))
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        if let Some(ref session) = self.session {
            lines.push(Line::from(vec![
                Span::styled("  Started: ", Style::default().fg(tc!(info))),
                Span::raw(session.started_at.clone()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Ended:   ", Style::default().fg(tc!(info))),
                Span::raw(session.ended_at.clone()),
            ]));
            lines.push(Line::from(""));
        }

        if self.flows.is_empty() && self.manipulation_history.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No events yet. Start an intercept session to see the timeline.",
                Style::default().fg(tc!(muted)),
            )));
        } else {
            #[derive(Clone)]
            enum TimelineEvent {
                FlowStart(usize, String, String, String),
                FlowEnd(usize, u16, String),
                Manipulation(usize, String, String, String),
            }

            let mut events: Vec<(String, TimelineEvent)> = Vec::new();

            for flow in &self.flows {
                events.push((
                    flow.started_at.clone(),
                    TimelineEvent::FlowStart(
                        flow.index as usize,
                        flow.method.clone(),
                        flow.host.clone(),
                        flow.path.clone(),
                    ),
                ));
                events.push((
                    flow.completed_at.clone(),
                    TimelineEvent::FlowEnd(
                        flow.index as usize,
                        flow.response_status,
                        flow.host.clone(),
                    ),
                ));
            }

            for m in &self.manipulation_history {
                events.push((
                    m.timestamp.clone(),
                    TimelineEvent::Manipulation(
                        m.flow_index as usize,
                        m.field.clone(),
                        m.reason.clone(),
                        m.after.clone().unwrap_or_default(),
                    ),
                ));
            }

            events.sort_by(|a, b| a.0.cmp(&b.0));

            let display_count = events.len().min(50);
            for (_, event) in events.iter().take(display_count) {
                match event {
                    TimelineEvent::FlowStart(idx, method, host, path) => {
                        lines.push(Line::from(vec![
                            Span::styled(format!("  [{}] ", idx), Style::default().fg(tc!(accent))),
                            Span::styled(
                                format!("{} ", method),
                                Style::default()
                                    .fg(tc!(success))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!("{}{}", host, path)),
                        ]));
                    }
                    TimelineEvent::FlowEnd(idx, status, host) => {
                        let status_color = if *status >= 200 && *status < 300 {
                            tc!(success)
                        } else if *status >= 400 {
                            tc!(error)
                        } else {
                            tc!(text)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(format!("  [{}] ", idx), Style::default().fg(tc!(muted))),
                            Span::styled(format!("{} ", status), Style::default().fg(status_color)),
                            Span::raw(format!("{} ", host)),
                            Span::styled("completed", Style::default().fg(tc!(muted))),
                        ]));
                    }
                    TimelineEvent::Manipulation(idx, field, reason, after) => {
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  [{}] ", idx),
                                Style::default().fg(tc!(warning)),
                            ),
                            Span::styled(
                                "EDIT ",
                                Style::default()
                                    .fg(tc!(warning))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!("{} ", field)),
                            Span::styled(
                                format!("-> {} ({})", truncate_str(after, 30), reason),
                                Style::default().fg(tc!(muted)),
                            ),
                        ]));
                    }
                }
            }

            if events.len() > 50 {
                lines.push(Line::from(Span::styled(
                    format!("  ... {} more events", events.len() - 50),
                    Style::default().fg(tc!(muted)),
                )));
            }
        }

        if !self.manipulation_history.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Manipulations: ", Style::default().fg(tc!(info))),
                Span::raw(format!("{}", self.manipulation_history.len())),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc!(border)))
            .title(" Timeline ");
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    pub(super) fn render_action_bar(&self, f: &mut Frame, area: Rect) {
        let actions = [
            "Forward",
            "Drop",
            "Replay",
            "Pause All",
            "Resume All",
            "Save",
            "Export HAR",
        ];
        let spans: Vec<Span> = actions
            .iter()
            .enumerate()
            .flat_map(|(i, action)| {
                let is_destructive = i == 1 || i == 2;
                let style = if i == self.action_bar_index {
                    if is_destructive {
                        Style::default()
                            .fg(tc!(danger))
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
            .border_style(Style::default().fg(tc!(border)))
            .title(
                " Actions (←/→ navigate · Enter execute · D-Drop R-Replay F-Forward · Esc-back ",
            );
        let paragraph = ratatui::widgets::Paragraph::new(Line::from(spans)).block(block);
        f.render_widget(paragraph, area);
    }

    pub(super) fn render_edit_modal(&self, f: &mut Frame, area: Rect) {
        use ratatui::widgets::Paragraph;

        f.render_widget(Clear, area);
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tc!(border)))
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

        let field_para = Paragraph::new(field_name).style(
            Style::default()
                .fg(tc!(accent))
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(field_para, modal_layout[0]);

        let orig_label = Paragraph::new(format!(
            "Original: {}",
            truncate_str(&self.edit_modal.original_value, 60)
        ))
        .style(Style::default().fg(tc!(muted)));
        f.render_widget(orig_label, modal_layout[1]);

        let edit_area = modal_layout[2];
        let edit_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tc!(border)))
            .title(" Edit Value (type to modify) ");

        let edit_content = if self.edit_modal.edit_buffer.is_empty() {
            "[empty - type to add]".to_string()
        } else {
            self.edit_modal.edit_buffer.clone()
        };
        let edit_para = Paragraph::new(edit_content).style(Style::default().fg(tc!(text)));
        f.render_widget(edit_block, edit_area);
        let inner_rect = Rect::new(
            edit_area.x + 1,
            edit_area.y + 1,
            edit_area.width - 2,
            edit_area.height - 2,
        );
        f.render_widget(edit_para, inner_rect);

        let diff_label = if self.edit_modal.original_value != self.edit_modal.edit_buffer {
            format!(
                "~ Change: {} → {}",
                truncate_str(&self.edit_modal.original_value, 30),
                truncate_str(&self.edit_modal.edit_buffer, 30)
            )
        } else {
            "(no change)".to_string()
        };
        let diff_para = Paragraph::new(diff_label).style(Style::default().fg(tc!(warning)));
        f.render_widget(diff_para, modal_layout[4]);

        let reason_para =
            Paragraph::new("Reason: (optional) ").style(Style::default().fg(tc!(muted)));
        f.render_widget(reason_para, modal_layout[5]);

        let help_text = "Enter-apply  Esc-cancel  Tab-switch focus";
        let help_para = Paragraph::new(help_text).style(Style::default().fg(tc!(muted)));
        f.render_widget(help_para, modal_layout[6]);
    }

    pub(super) fn clone_for_render(&self) -> Self {
        InterceptTab {
            flows: self.flows.clone(),
            selected_flow: self.selected_flow,
            detail_pane: self.detail_pane,
            focus_area: self.focus_area,
            current_view: self.current_view,
            state: self.state.clone(),
            results_view: crate::components::ScrollableText::new("Details"),
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
            scroll_offset: self.scroll_offset,
            performance_mode: self.performance_mode,
            cached_detail: None,
            debounce: DebounceState::new(),
            ws_sessions: self.ws_sessions.clone(),
            http2_sessions: self.http2_sessions.clone(),
            grpc_sessions: self.grpc_sessions.clone(),
            grpc_streaming_states: self.grpc_streaming_states.clone(),
            grpc_security_findings: self.grpc_security_findings.clone(),
            correlation_context: self.correlation_context.clone(),
            correlation_engine: CorrelationEngine::new(),
            confidence_scorer: ConfidenceScorer::default(),
            temporal_correlations: self.temporal_correlations.clone(),
            behavioral_matches: self.behavioral_matches.clone(),
            filter_query: self.filter_query.clone(),
            filter_field: self.filter_field,
            filter_active: self.filter_active,
            selected_http2_session: self.selected_http2_session,
            selected_grpc_session: self.selected_grpc_session,
            stream_mux_scroll: self.stream_mux_scroll,
        }
    }
}
