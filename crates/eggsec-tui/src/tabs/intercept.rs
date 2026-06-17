use crate::app::tab_error::TabError;
use crate::components::{empty_state_paragraph, ScrollableText};
use crate::tabs::{AppState, TabInput, TabRender, TabState};
use crate::tc;
use crate::workers::TaskConfig;
use eggsec::proxy::intercept::correlation::{
    BehavioralPattern, ConfidenceScorer, CorrelationContext, CorrelationEngine, CorrelationSource,
    TemporalCorrelation,
};
use eggsec::proxy::intercept::protocols::{
    GrpcCall, GrpcSession, GrpcStreamFrame, GrpcStreamingState, Http2Session, Http2Stream,
    WebSocketMessage, WebSocketSession,
};
#[cfg(test)]
use eggsec::proxy::intercept::protocols::WebSocketOpcode;
use eggsec::proxy::intercept::types::{
    FlowAction, InterceptSession, ManipulationRecord, ProxyFlow,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
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
    Timeline,
    WebSocket,
    Http2,
    Grpc,
    StreamMux,
    Correlation,
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

/// Debounce state for search/filter operations.
pub struct DebounceState {
    pub last_input_time: std::time::Instant,
    pub pending_filter: Option<String>,
    pub debounce_ms: u64,
}

impl DebounceState {
    pub fn new() -> Self {
        Self {
            last_input_time: std::time::Instant::now(),
            pending_filter: None,
            debounce_ms: 300,
        }
    }

    /// Returns `true` if the debounce window has elapsed and a filter is pending.
    pub fn should_apply(&self) -> bool {
        self.pending_filter.is_some()
            && self.last_input_time.elapsed().as_millis() >= self.debounce_ms as u128
    }

    /// Queue a filter string for debounced application.
    pub fn enqueue(&mut self, filter: String) {
        self.pending_filter = Some(filter);
        self.last_input_time = std::time::Instant::now();
    }

    /// Consume the pending filter if ready, returning it.
    pub fn take_if_ready(&mut self) -> Option<String> {
        if self.should_apply() {
            self.pending_filter.take()
        } else {
            None
        }
    }
}

impl Default for DebounceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached detail pane content to avoid re-parsing on every tab switch.
#[derive(Debug, Clone)]
pub enum DetailPaneContent {
    Headers(Vec<String>),
    Body(Vec<String>),
    Manipulations(usize),
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
    /// Scroll offset for virtual scrolling in the flow list.
    pub scroll_offset: usize,
    /// Whether performance mode is active (simplified rendering for >5000 flows).
    pub performance_mode: bool,
    /// Cached detail pane content: (selected_flow_index, content).
    pub cached_detail: Option<(usize, DetailPaneContent)>,
    /// Debounce state for search/filter operations.
    pub debounce: DebounceState,
    /// WebSocket sessions captured during the intercept session.
    pub ws_sessions: Vec<WebSocketSession>,
    /// HTTP/2 sessions captured during the intercept session.
    pub http2_sessions: Vec<Http2Session>,
    /// gRPC sessions captured during the intercept session.
    pub grpc_sessions: Vec<GrpcSession>,
    /// Streaming state for gRPC calls (multiplexing visualization).
    pub grpc_streaming_states: Vec<GrpcStreamingState>,
    /// Security findings from gRPC analysis.
    pub grpc_security_findings: Vec<eggsec::proxy::intercept::protocols::GrpcSecurityFinding>,
    /// Correlation context for cross-loadout linking.
    pub correlation_context: CorrelationContext,
    /// Active correlation engine for analysis.
    pub correlation_engine: CorrelationEngine,
    /// Confidence scorer for ML-based confidence computation.
    pub confidence_scorer: ConfidenceScorer,
    /// Temporal correlations (computed from references).
    pub temporal_correlations: Vec<TemporalCorrelation>,
    /// Behavioral pattern matches.
    pub behavioral_matches: Vec<(BehavioralPattern, f64)>,
    /// Active search/filter query string.
    pub filter_query: String,
    /// Which field to filter on: 0=all, 1=method, 2=host, 3=path, 4=status.
    pub filter_field: usize,
    /// Whether the filter input is focused.
    pub filter_active: bool,
    /// Selected HTTP/2 session index for stream multiplexing view.
    pub selected_http2_session: usize,
    /// Selected gRPC session index for streaming view.
    pub selected_grpc_session: usize,
    /// Scroll offset for stream multiplexing view.
    pub stream_mux_scroll: usize,
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
            scroll_offset: 0,
            performance_mode: false,
            cached_detail: None,
            debounce: DebounceState::new(),
            ws_sessions: Vec::new(),
            http2_sessions: Vec::new(),
            grpc_sessions: Vec::new(),
            grpc_streaming_states: Vec::new(),
            grpc_security_findings: Vec::new(),
            correlation_context: CorrelationContext::new(),
            correlation_engine: CorrelationEngine::new(),
            confidence_scorer: ConfidenceScorer::default(),
            temporal_correlations: Vec::new(),
            behavioral_matches: Vec::new(),
            filter_query: String::new(),
            filter_field: 0,
            filter_active: false,
            selected_http2_session: 0,
            selected_grpc_session: 0,
            stream_mux_scroll: 0,
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

    /// Get only the flows visible in the current viewport (virtual scrolling).
    pub fn visible_flows(&self, viewport_height: usize) -> &[ProxyFlow] {
        let start = self.scroll_offset.min(self.flows.len());
        let end = (start + viewport_height).min(self.flows.len());
        &self.flows[start..end]
    }

    /// Number of flows visible in the current viewport.
    fn visible_flows_len(&self) -> usize {
        let start = self.scroll_offset.min(self.flows.len());
        self.flows.len() - start
    }

    /// Adjust scroll offset so that `selected_flow` remains visible.
    pub fn ensure_selected_visible(&mut self, viewport_height: usize) {
        if let Some(idx) = self.selected_flow {
            if idx < self.scroll_offset {
                self.scroll_offset = idx;
            } else if idx >= self.scroll_offset + viewport_height {
                self.scroll_offset = idx + 1 - viewport_height;
            }
        }
    }

    /// Toggle performance mode on/off.
    pub fn toggle_performance_mode(&mut self) {
        self.performance_mode = !self.performance_mode;
        self.cached_detail = None;
    }

    /// Estimate memory usage of the intercept tab state in bytes.
    pub fn estimate_memory_usage(&self) -> usize {
        let flows_size: usize = self
            .flows
            .iter()
            .map(|f| {
                f.method.len()
                    + f.url.len()
                    + f.host.len()
                    + f.path.len()
                    + f.request_headers.values().map(|v| v.len()).sum::<usize>()
                    + f.response_headers.values().map(|v| v.len()).sum::<usize>()
                    + f.request_body.as_ref().map_or(0, |b| b.len())
                    + f.response_body.as_ref().map_or(0, |b| b.len())
            })
            .sum();

        let ws_size: usize = self
            .ws_sessions
            .iter()
            .map(|s| s.messages.iter().map(|m| m.payload.len()).sum::<usize>())
            .sum();

        let manip_size: usize = self
            .manipulation_history
            .iter()
            .map(|m| {
                m.field.len()
                    + m.before.as_ref().map_or(0, |b| b.len())
                    + m.after.as_ref().map_or(0, |a| a.len())
                    + m.reason.len()
            })
            .sum();

        flows_size + ws_size + manip_size
    }

    /// Get a page of WebSocket messages for display.
    pub fn ws_messages_page(
        &self,
        session_idx: usize,
        page: usize,
        page_size: usize,
    ) -> &[WebSocketMessage] {
        if let Some(session) = self.ws_sessions.get(session_idx) {
            let start = page * page_size;
            if start >= session.messages.len() {
                return &[];
            }
            let end = (start + page_size).min(session.messages.len());
            &session.messages[start..end]
        } else {
            &[]
        }
    }

    /// Total WebSocket message count for a session.
    pub fn ws_session_message_count(&self, session_idx: usize) -> usize {
        self.ws_sessions
            .get(session_idx)
            .map_or(0, |s| s.messages.len())
    }

    /// Get HTTP/2 streams for a given session, sorted by stream ID.
    pub fn http2_streams_for_session(&self, session_idx: usize) -> Vec<&Http2Stream> {
        self.http2_sessions
            .get(session_idx)
            .map(|s| {
                let mut streams: Vec<&Http2Stream> = s.streams.iter().collect();
                streams.sort_by_key(|stream| stream.stream_id);
                streams
            })
            .unwrap_or_default()
    }

    /// Count of streams grouped by state for the selected session.
    pub fn http2_stream_state_counts(&self, session_idx: usize) -> (usize, usize, usize, usize) {
        let (mut open, mut half_closed, mut closed, mut idle) = (0, 0, 0, 0);
        if let Some(session) = self.http2_sessions.get(session_idx) {
            for stream in &session.streams {
                use eggsec::proxy::intercept::protocols::Http2StreamState;
                match stream.state {
                    Http2StreamState::Open => open += 1,
                    Http2StreamState::HalfClosedLocal | Http2StreamState::HalfClosedRemote => {
                        half_closed += 1
                    }
                    Http2StreamState::Closed => closed += 1,
                    Http2StreamState::Idle => idle += 1,
                }
            }
        }
        (open, half_closed, closed, idle)
    }

    /// Get gRPC calls for a given session.
    pub fn grpc_calls_for_session(&self, session_idx: usize) -> Vec<&GrpcCall> {
        self.grpc_sessions
            .get(session_idx)
            .map(|s| s.calls.iter().collect())
            .unwrap_or_default()
    }

    /// Get a page of gRPC streaming frames.
    pub fn grpc_stream_frames_page(
        &self,
        streaming_idx: usize,
        page: usize,
        page_size: usize,
    ) -> Vec<&GrpcStreamFrame> {
        let mut frames: Vec<&GrpcStreamFrame> = Vec::new();
        if let Some(state) = self.grpc_streaming_states.get(streaming_idx) {
            frames.extend(state.client_frames.iter());
            frames.extend(state.server_frames.iter());
            frames.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }
        let start = page * page_size;
        if start >= frames.len() {
            return Vec::new();
        }
        let end = (start + page_size).min(frames.len());
        frames[start..end].to_vec()
    }

    /// Total streaming frame count across all states.
    pub fn total_grpc_stream_frames(&self) -> usize {
        self.grpc_streaming_states
            .iter()
            .map(|s| s.client_frames.len() + s.server_frames.len())
            .sum()
    }

    /// Get a summary line for streaming state.
    pub fn grpc_streaming_summary(&self, streaming_idx: usize) -> Option<String> {
        self.grpc_streaming_states
            .get(streaming_idx)
            .map(|s| {
                let summary = s.summary();
                format!(
                    "{:?}: {} client / {} server frames, {} bytes (window: {})",
                    summary.method_type,
                    summary.client_frame_count,
                    summary.server_frame_count,
                    summary.total_bytes,
                    summary.flow_control_window,
                )
            })
    }

    /// Compute correlation summary as a string.
    pub fn correlation_summary_str(&self) -> String {
        let s = &self.correlation_context.summary;
        if s.total_references == 0 {
            return "No correlations".to_string();
        }
        format!(
            "{} refs, {} unique sources, {} correlated flows, {:.0}% avg confidence, {} temporal, {} behavioral",
            s.total_references,
            s.unique_sources,
            s.correlated_flows,
            s.avg_confidence * 100.0,
            s.temporal_correlations,
            s.behavioral_correlations,
        )
    }

    /// Recompute temporal and behavioral correlations from the current context.
    pub fn recompute_correlations(&mut self) {
        let (temporal, behavioral) = self.correlation_engine.correlate(&mut self.correlation_context);
        self.temporal_correlations = temporal;
        self.behavioral_matches = behavioral;
    }

    /// Add an HTTP/2 session to the tab.
    pub fn add_http2_session(&mut self, session: Http2Session) {
        self.http2_sessions.push(session);
    }

    /// Add a gRPC session to the tab.
    pub fn add_grpc_session(&mut self, session: GrpcSession) {
        self.grpc_sessions.push(session);
    }

    /// Add a streaming state for visualization.
    pub fn add_grpc_streaming_state(&mut self, state: GrpcStreamingState) {
        self.grpc_streaming_states.push(state);
    }

    /// Add a security finding from gRPC analysis.
    pub fn add_grpc_security_finding(
        &mut self,
        finding: eggsec::proxy::intercept::protocols::GrpcSecurityFinding,
    ) {
        self.grpc_security_findings.push(finding);
    }

    /// Add a correlation reference.
    pub fn add_correlation_reference(
        &mut self,
        flow_index: u64,
        reference: eggsec::proxy::intercept::correlation::CorrelationReference,
    ) {
        self.correlation_context
            .add_flow_correlation(flow_index, reference);
    }

    /// Get correlations for a specific flow.
    pub fn correlations_for_flow(
        &self,
        flow_index: u64,
    ) -> Vec<&eggsec::proxy::intercept::correlation::CorrelationReference> {
        self.correlation_context.get_flow_correlations(flow_index)
    }

    /// Count correlations per source type.
    pub fn correlation_source_counts(&self) -> Vec<(CorrelationSource, usize)> {
        let mut counts: std::collections::HashMap<CorrelationSource, usize> =
            std::collections::HashMap::new();
        for r in &self.correlation_context.references {
            *counts.entry(r.source).or_insert(0) += 1;
        }
        let mut v: Vec<_> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }

    /// Get the count of HTTP/2 sessions.
    pub fn http2_session_count(&self) -> usize {
        self.http2_sessions.len()
    }

    /// Get the count of gRPC sessions.
    pub fn grpc_session_count(&self) -> usize {
        self.grpc_sessions.len()
    }

    /// Total HTTP/2 stream count across all sessions.
    pub fn total_http2_streams(&self) -> usize {
        self.http2_sessions.iter().map(|s| s.streams.len()).sum()
    }

    /// Cycle to the next HTTP/2 session.
    pub fn next_http2_session(&mut self) {
        if !self.http2_sessions.is_empty() {
            self.selected_http2_session =
                (self.selected_http2_session + 1) % self.http2_sessions.len();
            self.stream_mux_scroll = 0;
        }
    }

    /// Cycle to the previous HTTP/2 session.
    pub fn prev_http2_session(&mut self) {
        if !self.http2_sessions.is_empty() {
            self.selected_http2_session = if self.selected_http2_session == 0 {
                self.http2_sessions.len() - 1
            } else {
                self.selected_http2_session - 1
            };
            self.stream_mux_scroll = 0;
        }
    }

    /// Cycle to the next gRPC session.
    pub fn next_grpc_session(&mut self) {
        if !self.grpc_sessions.is_empty() {
            self.selected_grpc_session = (self.selected_grpc_session + 1) % self.grpc_sessions.len();
            self.stream_mux_scroll = 0;
        }
    }

    /// Cycle to the previous gRPC session.
    pub fn prev_grpc_session(&mut self) {
        if !self.grpc_sessions.is_empty() {
            self.selected_grpc_session = if self.selected_grpc_session == 0 {
                self.grpc_sessions.len() - 1
            } else {
                self.selected_grpc_session - 1
            };
            self.stream_mux_scroll = 0;
        }
    }

    /// Get flows matching the current filter query.
    pub fn filtered_flows(&self) -> Vec<(usize, &ProxyFlow)> {
        if self.filter_query.is_empty() {
            return self.flows.iter().enumerate().collect();
        }
        let q = self.filter_query.to_lowercase();
        self.flows
            .iter()
            .enumerate()
            .filter(|(_, f)| match self.filter_field {
                0 => f.method.to_lowercase().contains(&q)
                    || f.host.to_lowercase().contains(&q)
                    || f.path.to_lowercase().contains(&q)
                    || f.response_status.to_string().contains(&q),
                1 => f.method.to_lowercase().contains(&q),
                2 => f.host.to_lowercase().contains(&q),
                3 => f.path.to_lowercase().contains(&q),
                4 => f.response_status.to_string().contains(&q),
                _ => true,
            })
            .collect()
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
        // Auto-enable performance mode for high-volume sessions
        if self.flows.len() > 5000 && !self.performance_mode {
            self.performance_mode = true;
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

        // Virtual scrolling: only render flows visible in the viewport
        let viewport_height = area.height.saturating_sub(3) as usize; // subtract borders + header
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

        let mut table_state = self.table_state.clone();
        // Adjust selected position relative to viewport
        if let Some(selected) = self.selected_flow {
            table_state.select(Some(selected.saturating_sub(self.scroll_offset)));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn render_detail_pane(&self, f: &mut Frame, area: Rect) {
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
                    let placeholder = empty_state_paragraph("Detail", "Select a flow to view details");
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
            lines.push(Line::from(vec![Span::styled(" Enhanced Rules", Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD))]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Condition Types: ", Style::default().fg(tc!(info))), Span::raw("HostMatches, PathMatches, MethodMatches, HeaderContains, BodyContains, BodySizeGt/Lt")]));
            lines.push(Line::from(vec![Span::styled("  Combinators: ", Style::default().fg(tc!(info))), Span::raw("AND, OR, NOT for complex conditions")]));
            lines.push(Line::from(vec![Span::styled("  Protocol: ", Style::default().fg(tc!(info))), Span::raw("ProtocolIs, WebSocketOpcodeIs, GrpcMethodIs")]));
            lines.push(Line::from(vec![Span::styled("  Actions: ", Style::default().fg(tc!(info))), Span::raw("Allow, Block, Intercept, Monitor, Modify, InjectResponse, Delay, Tag")]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Persistence: ", Style::default().fg(tc!(info))), Span::raw("JSON save/load via EnhancedRuleSet")]));
            lines.push(Line::from("  eggsec proxy intercept --rule-set /path/to/rules.json"));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Rules ({:?}) ", self.selected_rule_view));
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_protocol_info(&self, f: &mut Frame, area: Rect, protocol: &str) {
        let flow = match self.selected_flow_data() {
            Some(f) => f,
            None => {
                let placeholder = empty_state_paragraph("Protocol", "Select a flow to view protocol details");
                f.render_widget(placeholder, area);
                return;
            }
        };

        let flow_protocol = &flow.protocol;
        let mut lines = vec![
            Line::from(vec![Span::styled(
                format!("{} Protocol Details", protocol),
                Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(" Flow Protocol: ", Style::default().fg(tc!(info))), Span::raw(flow_protocol.clone())]),
            Line::from(""),
        ];

        match protocol {
            "WebSocket" => {
                lines.push(Line::from(vec![Span::styled(" Detection: ", Style::default().fg(tc!(info))), Span::raw("Check Upgrade header for 'websocket' value")]));
                lines.push(Line::from(vec![Span::styled(" Capture: ", Style::default().fg(tc!(info))), Span::raw("WebSocket sessions captured during MITM interception")]));
                lines.push(Line::from(vec![Span::styled(" Manipulation: ", Style::default().fg(tc!(info))), Span::raw("Edit and replay individual frames")]));
                lines.push(Line::from(""));

                // Show captured WebSocket messages with pagination
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
                                Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!("{} ({} messages)", session.host, session.messages.len())),
                        ]));

                        // Show first page of messages (10 per page)
                        let page_size = 10;
                        let messages = self.ws_messages_page(session_idx, 0, page_size);
                        for (i, msg) in messages.iter().enumerate() {
                            let prefix = if msg.direction == eggsec::proxy::intercept::types::ProxyFlowDirection::Request { "  -> " } else { " <-  " };
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
                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("  ... {} more messages (scroll to see more)", total - page_size),
                                    Style::default().fg(tc!(muted)),
                                ),
                            ]));
                        }
                    }
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(" Note: ", Style::default().fg(tc!(warning))),
                        Span::raw("No WebSocket sessions captured yet"),
                    ]));
                }
            },
            "HTTP/2" => {
                lines.push(Line::from(vec![Span::styled(" Detection: ", Style::default().fg(tc!(info))), Span::raw("HTTP/2 identified by :scheme pseudo-header")]));
                lines.push(Line::from(vec![Span::styled(" Capture: ", Style::default().fg(tc!(info))), Span::raw("HTTP/2 streams with ID, priority, window updates")]));
                lines.push(Line::from(vec![Span::styled(" Streams: ", Style::default().fg(tc!(info))), Span::raw("Multiplexed over single TCP connection")]));
                lines.push(Line::from(""));

                // Show actual HTTP/2 session data if available
                if !self.http2_sessions.is_empty() {
                    let total_streams = self.total_http2_streams();
                    lines.push(Line::from(vec![
                        Span::styled(" Captured Sessions: ", Style::default().fg(tc!(info))),
                        Span::raw(format!("{} ({} total streams)", self.http2_sessions.len(), total_streams)),
                    ]));
                    lines.push(Line::from(""));

                    // Show first session details
                    if let Some(session) = self.http2_sessions.first() {
                        let (open, half_closed, closed, idle) = self.http2_stream_state_counts(0);
                        lines.push(Line::from(vec![
                            Span::styled(format!("  Session {}: ", 0),
                                Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD)),
                            Span::raw(format!("{}{} ({} streams)",
                                if session.is_secure { "https://" } else { "http://" },
                                session.host,
                                session.streams.len())),
                        ]));

                        lines.push(Line::from(vec![
                            Span::styled("    Stream States: ", Style::default().fg(tc!(info))),
                            Span::raw(format!("open={}, half-closed={}, closed={}, idle={}",
                                open, half_closed, closed, idle)),
                        ]));

                        lines.push(Line::from(vec![
                            Span::styled("    Connection Window: ", Style::default().fg(tc!(info))),
                            Span::raw(format!("{} bytes ({} stream window)",
                                session.connection_window_size, session.stream_window_size)),
                        ]));

                        if session.max_concurrent_streams > 0 {
                            lines.push(Line::from(vec![
                                Span::styled("    Max Concurrent: ", Style::default().fg(tc!(info))),
                                Span::raw(format!("{} streams, max frame: {} bytes",
                                    session.max_concurrent_streams, session.max_frame_size)),
                            ]));
                        }

                        // Show up to 5 streams
                        let streams = self.http2_streams_for_session(0);
                        let display_count = streams.len().min(5);
                        if display_count > 0 {
                            lines.push(Line::from(""));
                            lines.push(Line::from(vec![Span::styled("    Streams:",
                                Style::default().fg(tc!(info)))]));
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
                                    Span::styled(format!("{:<6}", state_str), Style::default().fg(state_color)),
                                    Span::raw(format!(" {} {}", stream.method, truncate_str(&stream.path, 40))),
                                ]));
                            }
                            if streams.len() > display_count {
                                lines.push(Line::from(Span::styled(
                                    format!("      ... {} more streams (see Stream Mux tab)", streams.len() - display_count),
                                    Style::default().fg(tc!(muted)),
                                )));
                            }
                        }
                    }
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![Span::styled(" Tip: ", Style::default().fg(tc!(success))),
                        Span::raw("Switch to 'Stream Mux' tab for full multiplexing visualization")]));
                } else {
                    lines.push(Line::from(vec![Span::styled(" Note: ", Style::default().fg(tc!(warning))),
                        Span::raw("No HTTP/2 sessions captured yet")]));
                    lines.push(Line::from(vec![Span::styled(" Tip: ", Style::default().fg(tc!(success))),
                        Span::raw("Stream demultiplexing available in 'Stream Mux' tab")]));
                }
            },
            "gRPC" => {
                lines.push(Line::from(vec![Span::styled(" Detection: ", Style::default().fg(tc!(info))), Span::raw("gRPC identified by Content-Type: application/grpc*")]));
                lines.push(Line::from(vec![Span::styled(" Capture: ", Style::default().fg(tc!(info))), Span::raw("gRPC calls with method type, metadata, body")]));
                lines.push(Line::from(vec![Span::styled(" Methods: ", Style::default().fg(tc!(info))), Span::raw("Unary, ServerStreaming, ClientStreaming, Bidirectional")]));
                lines.push(Line::from(""));

                // Show actual gRPC session data if available
                if !self.grpc_sessions.is_empty() {
                    let total_calls: usize = self.grpc_sessions.iter().map(|s| s.calls.len()).sum();
                    let streaming_calls: usize = self.grpc_sessions.iter()
                        .map(|s| s.streaming_call_count())
                        .sum();
                    let error_calls: usize = self.grpc_sessions.iter()
                        .flat_map(|s| s.error_calls())
                        .count();

                    lines.push(Line::from(vec![
                        Span::styled(" Captured Sessions: ", Style::default().fg(tc!(info))),
                        Span::raw(format!("{} ({} total calls, {} streaming, {} errors)",
                            self.grpc_sessions.len(), total_calls, streaming_calls, error_calls)),
                    ]));
                    lines.push(Line::from(""));

                    // Show first session calls
                    if let Some(session) = self.grpc_sessions.first() {
                        lines.push(Line::from(vec![
                            Span::styled(format!("  Session {}: ", 0),
                                Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD)),
                            Span::raw(format!("{}{} ({} calls)",
                                if session.is_secure { "https://" } else { "http://" },
                                session.host,
                                session.calls.len())),
                        ]));

                        // Show up to 5 calls
                        let display_count = session.calls.len().min(5);
                        if display_count > 0 {
                            lines.push(Line::from(""));
                            lines.push(Line::from(vec![Span::styled("    Calls:",
                                Style::default().fg(tc!(info)))]));
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
                                    Span::styled(type_str.to_string(), Style::default().fg(type_color)),
                                    Span::raw(format!(" {}", truncate_str(&call.path, 50))),
                                ]));
                            }
                            if session.calls.len() > display_count {
                                lines.push(Line::from(Span::styled(
                                    format!("      ... {} more calls", session.calls.len() - display_count),
                                    Style::default().fg(tc!(muted)),
                                )));
                            }
                        }
                    }

                    // Show security findings
                    if !self.grpc_security_findings.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from(vec![
                            Span::styled(" Security Findings: ", Style::default().fg(tc!(warning))),
                            Span::raw(format!("{}", self.grpc_security_findings.len())),
                        ]));
                        for finding in self.grpc_security_findings.iter().take(3) {
                            lines.push(Line::from(vec![
                                Span::styled(format!("    - {}: ", finding.category),
                                    Style::default().fg(tc!(warning))),
                                Span::raw(truncate_str(&finding.description, 50)),
                            ]));
                        }
                        if self.grpc_security_findings.len() > 3 {
                            lines.push(Line::from(Span::styled(
                                format!("    ... {} more findings", self.grpc_security_findings.len() - 3),
                                Style::default().fg(tc!(muted)),
                            )));
                        }
                    }

                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![Span::styled(" Tip: ", Style::default().fg(tc!(success))),
                        Span::raw("Switch to 'Stream Mux' tab for streaming frame visualization")]));
                } else {
                    lines.push(Line::from(vec![Span::styled(" Note: ", Style::default().fg(tc!(warning))),
                        Span::raw("No gRPC sessions captured yet")]));
                }
            },
            _ => {},
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", protocol));
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    /// Render the stream multiplexing visualization.
    /// Shows HTTP/2 streams and gRPC streaming frames in a unified view.
    fn render_stream_multiplexing(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Stream Multiplexing",
            Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        // HTTP/2 section
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
            let session_idx = self.selected_http2_session.min(self.http2_sessions.len() - 1);
            let session = &self.http2_sessions[session_idx];

            lines.push(Line::from(vec![
                Span::styled("  Session: ", Style::default().fg(tc!(info))),
                Span::raw(format!("{} ({} of {}, [<]/[>] to cycle)",
                    session.host,
                    session_idx + 1,
                    self.http2_sessions.len())),
            ]));

            let (open, half_closed, closed, idle) = self.http2_stream_state_counts(session_idx);
            lines.push(Line::from(vec![
                Span::styled("  States: ", Style::default().fg(tc!(info))),
                Span::styled(format!("OPEN:{} ", open), Style::default().fg(tc!(success))),
                Span::styled(format!("HALF:{} ", half_closed), Style::default().fg(tc!(warning))),
                Span::styled(format!("CLOSED:{} ", closed), Style::default().fg(tc!(muted))),
                Span::styled(format!("IDLE:{} ", idle), Style::default().fg(tc!(text))),
            ]));

            lines.push(Line::from(vec![
                Span::styled("  Windows: ", Style::default().fg(tc!(info))),
                Span::raw(format!("conn={}B stream={}B max-frame={}B max-streams={}",
                    session.connection_window_size,
                    session.stream_window_size,
                    session.max_frame_size,
                    session.max_concurrent_streams)),
            ]));

            // Visual timeline of streams (showing stream IDs as horizontal bars)
            let streams = self.http2_streams_for_session(session_idx);
            if !streams.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "  Stream Timeline:",
                    Style::default().fg(tc!(info)),
                )]));

                // Show ASCII visualization of stream states
                for stream in streams.iter().take(20) {
                    let (marker, color) = match stream.state {
                        eggsec::proxy::intercept::protocols::Http2StreamState::Open => ("[OPEN    ]", tc!(success)),
                        eggsec::proxy::intercept::protocols::Http2StreamState::HalfClosedLocal => ("[HALF-L  ]", tc!(warning)),
                        eggsec::proxy::intercept::protocols::Http2StreamState::HalfClosedRemote => ("[HALF-R  ]", tc!(warning)),
                        eggsec::proxy::intercept::protocols::Http2StreamState::Closed => ("[CLOSED  ]", tc!(muted)),
                        eggsec::proxy::intercept::protocols::Http2StreamState::Idle => ("[IDLE    ]", tc!(text)),
                    };
                    let dur_hint = if stream.closed_at.is_some() { "DONE" } else { "LIVE" };
                    lines.push(Line::from(vec![
                        Span::styled(format!("    {:>3} ", stream.stream_id), Style::default().fg(tc!(accent))),
                        Span::styled(marker, Style::default().fg(color)),
                        Span::raw(format!(" {} {} ({})", stream.method, truncate_str(&stream.path, 30), dur_hint)),
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

        // gRPC streaming section
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
                Span::raw(format!("{} streaming call(s), {} frames", self.grpc_streaming_states.len(), total_frames)),
            ]));

            // Show each streaming state
            for (idx, state) in self.grpc_streaming_states.iter().enumerate() {
                let summary = state.summary();
                let type_str = match summary.method_type {
                    eggsec::proxy::intercept::protocols::GrpcMethodType::Unary => "UNARY",
                    eggsec::proxy::intercept::protocols::GrpcMethodType::ServerStreaming => "SERVER-STREAM",
                    eggsec::proxy::intercept::protocols::GrpcMethodType::ClientStreaming => "CLIENT-STREAM",
                    eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional => "BIDI",
                };
                let type_color = match summary.method_type {
                    eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional => tc!(accent),
                    eggsec::proxy::intercept::protocols::GrpcMethodType::ServerStreaming => tc!(info),
                    _ => tc!(text),
                };
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(format!("  Stream #{}: ", idx), Style::default().fg(tc!(info))),
                    Span::styled(type_str, Style::default().fg(type_color)),
                    Span::raw(format!(" | {} client / {} server", summary.client_frame_count, summary.server_frame_count)),
                ]));

                // Show flow control bar
                let pct = if summary.flow_control_window > 0 {
                    (summary.bytes_in_flight as f64 / summary.flow_control_window as f64 * 100.0).min(100.0)
                } else {
                    0.0
                };
                let bar_width = 20;
                let filled = (pct / 100.0 * bar_width as f64) as usize;
                let bar: String = std::iter::repeat('#').take(filled).chain(std::iter::repeat('-').take(bar_width - filled)).collect();
                let bar_color = if pct < 50.0 { tc!(success) } else if pct < 80.0 { tc!(warning) } else { tc!(error) };
                lines.push(Line::from(vec![
                    Span::styled("    Flow Window: ", Style::default().fg(tc!(info))),
                    Span::styled(format!("[{}] ", bar), Style::default().fg(bar_color)),
                    Span::raw(format!("{}/{}B ({:.0}%)", summary.bytes_in_flight, summary.flow_control_window, pct)),
                ]));

                // Show recent frames
                let frames = self.grpc_stream_frames_page(idx, 0, 5);
                if !frames.is_empty() {
                    lines.push(Line::from(vec![Span::styled("    Recent Frames:", Style::default().fg(tc!(muted)))]));
                    for frame in frames {
                        let arrow = if frame.direction == eggsec::proxy::intercept::types::ProxyFlowDirection::Request { "->" } else { "<-" };
                        let end_marker = if frame.end_stream { " [END]" } else { "" };
                        lines.push(Line::from(vec![
                            Span::raw(format!("      {} ", arrow)),
                            Span::raw(format!("{}B{}", frame.size, end_marker)),
                            Span::styled(format!(" @ {}", truncate_str(&frame.timestamp, 19)),
                                Style::default().fg(tc!(muted))),
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

        // Legend
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  Keys: ",
            Style::default().fg(tc!(info)),
        ),
        Span::styled("[</>]", Style::default().fg(tc!(accent))),
        Span::raw(" cycle session")]));

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Stream Multiplexing ");
        let paragraph = ratatui::widgets::Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    /// Render the correlation visualization.
    /// Shows cross-loadout correlation references and computed patterns.
    fn render_correlation(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Cross-Loadout Correlation",
            Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        // Summary
        lines.push(Line::from(vec![Span::styled(" Summary: ", Style::default().fg(tc!(info))),
            Span::raw(self.correlation_summary_str())]));
        lines.push(Line::from(""));

        // Source counts
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
                    Span::styled(format!("    {:<14} ", source_str), Style::default().fg(tc!(info))),
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

        // Temporal correlations
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
                let conf_color = if conf_pct >= 70 { tc!(success) } else if conf_pct >= 40 { tc!(warning) } else { tc!(muted) };
                lines.push(Line::from(vec![
                    Span::raw(format!("  [{}] ", i + 1)),
                    Span::styled(format!("{:?} <-> {:?} ", t.a.source, t.b.source),
                        Style::default().fg(tc!(accent))),
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
                    format!("  ... {} more temporal correlations", self.temporal_correlations.len() - 8),
                    Style::default().fg(tc!(muted)),
                )));
            }
        }

        // Behavioral pattern matches
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
                let conf_color = if conf_pct >= 70 { tc!(success) } else if conf_pct >= 40 { tc!(warning) } else { tc!(muted) };
                lines.push(Line::from(vec![
                    Span::styled(format!("  - {} ", pattern.id), Style::default().fg(tc!(accent))),
                    Span::raw(truncate_str(&pattern.description, 45)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("      ", Style::default()),
                    Span::styled(format!("confidence: {}%", conf_pct), Style::default().fg(conf_color)),
                    Span::raw(format!(" ({} sources required)", pattern.required_sources.len())),
                ]));
            }
        }

        // Flow-level correlations (for the selected flow)
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
                        Span::styled(format!("  -> {:?} ", r.source), Style::default().fg(tc!(accent))),
                        Span::raw(truncate_str(&r.description, 40)),
                        Span::styled(format!(" ({}%)", conf_pct), Style::default().fg(tc!(muted))),
                    ]));
                }
            }
        }

        // Tips
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(" Tip: ", Style::default().fg(tc!(success))),
            Span::raw("Correlations link proxy flows to findings from other loadouts (db-pentest, auth-test, mobile-dynamic)")]));

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Correlation ");
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

    fn render_timeline(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Session Timeline",
            Style::default().fg(tc!(accent)).add_modifier(Modifier::BOLD),
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
            // Build a merged timeline of flows and manipulations sorted by timestamp
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

            // Render up to 50 events to avoid overflow
            let display_count = events.len().min(50);
            for (_, event) in events.iter().take(display_count) {
                match event {
                    TimelineEvent::FlowStart(idx, method, host, path) => {
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  [{}] ", idx),
                                Style::default().fg(tc!(accent)),
                            ),
                            Span::styled(
                                format!("{} ", method),
                                Style::default().fg(tc!(success)).add_modifier(Modifier::BOLD),
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
                            Span::styled(
                                format!("  [{}] ", idx),
                                Style::default().fg(tc!(muted)),
                            ),
                            Span::styled(
                                format!("{} ", status),
                                Style::default().fg(status_color),
                            ),
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
                                Style::default().fg(tc!(warning)).add_modifier(Modifier::BOLD),
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

        // Correlation summary if present
        if !self.manipulation_history.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  Manipulations: ",
                Style::default().fg(tc!(info)),
            ),
            Span::raw(format!("{}", self.manipulation_history.len())),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Timeline ");
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
        self.scroll_offset = 0;
        self.performance_mode = false;
        self.cached_detail = None;
        self.ws_sessions.clear();
        self.http2_sessions.clear();
        self.grpc_sessions.clear();
        self.grpc_streaming_states.clear();
        self.grpc_security_findings.clear();
        self.correlation_context = CorrelationContext::new();
        self.temporal_correlations.clear();
        self.behavioral_matches.clear();
        self.filter_query.clear();
        self.filter_field = 0;
        self.filter_active = false;
        self.selected_http2_session = 0;
        self.selected_grpc_session = 0;
        self.stream_mux_scroll = 0;
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
            DetailPane::Timeline => "Timeline",
            DetailPane::WebSocket => "WebSocket",
            DetailPane::Http2 => "HTTP/2",
            DetailPane::Grpc => "gRPC",
            DetailPane::StreamMux => "Stream Mux",
            DetailPane::Correlation => "Correlation",
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
            " {} | {} | Flows: {} | {}{}{}",
            self.listen_addr,
            if self.state == AppState::Running { "ACTIVE" } else { "IDLE" },
            self.flows.len(),
            if self.dry_run { "DRY-RUN" } else { "LIVE" },
            if self.performance_mode {
                format!(" | PERF | ~{}", format_bytes(self.estimate_memory_usage() as u64))
            } else {
                String::new()
            },
            if self.filter_active {
                format!(" | FILTER: /{}", self.filter_query)
            } else if !self.filter_query.is_empty() {
                format!(" | /{}", self.filter_query)
            } else {
                String::new()
            }
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
        let tab_names = ["Headers", "Body", "Manipulations", "Rules", "Timeline", "WS", "H2", "gRPC", "Mux", "Corr"];
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
        if self.filter_active {
            self.filter_query.push(c);
            return;
        }
        match c {
            'r' if self.detail_pane == DetailPane::Rules => self.toggle_rule_view(),
            'p' if !self.is_running() => self.toggle_performance_mode(),
            '/' if !self.is_running() => {
                self.filter_active = true;
                self.filter_query.clear();
            }
            _ => {}
        }
    }

    fn handle_backspace(&mut self) {
        if self.is_edit_modal_open() {
            self.edit_modal.edit_buffer.pop();
            return;
        }
        if self.filter_active {
            self.filter_query.pop();
            if self.filter_query.is_empty() {
                self.filter_active = false;
            }
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
        if self.filter_active {
            self.filter_active = false;
            self.filter_query.clear();
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
                    if i - 1 < self.scroll_offset {
                        self.scroll_offset = i - 1;
                    }
                }
            }
            InterceptFocusArea::DetailView => {
                self.detail_pane = match self.detail_pane {
                    DetailPane::Headers => DetailPane::Rules,
                    DetailPane::Body => DetailPane::Headers,
                    DetailPane::Manipulations => DetailPane::Body,
                    DetailPane::Rules => DetailPane::Timeline,
                    DetailPane::Timeline => DetailPane::Manipulations,
                    DetailPane::WebSocket => DetailPane::Grpc,
                    DetailPane::Http2 => DetailPane::WebSocket,
                    DetailPane::Grpc => DetailPane::Correlation,
                    DetailPane::StreamMux => DetailPane::Grpc,
                    DetailPane::Correlation => DetailPane::StreamMux,
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
                    let viewport = self.visible_flows_len().max(1);
                    if i + 1 >= self.scroll_offset + viewport {
                        self.scroll_offset = i + 2 - viewport;
                    }
                }
            }
            InterceptFocusArea::DetailView => {
                self.detail_pane = match self.detail_pane {
                    DetailPane::Headers => DetailPane::Body,
                    DetailPane::Body => DetailPane::Manipulations,
                    DetailPane::Manipulations => DetailPane::Timeline,
                    DetailPane::Timeline => DetailPane::Rules,
                    DetailPane::Rules => DetailPane::Headers,
                    DetailPane::WebSocket => DetailPane::Http2,
                    DetailPane::Http2 => DetailPane::Grpc,
                    DetailPane::Grpc => DetailPane::StreamMux,
                    DetailPane::StreamMux => DetailPane::Correlation,
                    DetailPane::Correlation => DetailPane::WebSocket,
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
            if new_i < self.scroll_offset {
                self.scroll_offset = new_i;
            }
        }
    }

    fn page_down(&mut self, _page_size: usize) {
        if !self.is_running() && self.focus_area == InterceptFocusArea::FlowList {
            let i = self.selected_flow.unwrap_or(0);
            let new_i = (i + 20).min(self.flows.len().saturating_sub(1));
            self.selected_flow = Some(new_i);
            self.table_state.select(Some(new_i));
            let viewport = self.visible_flows_len().max(1);
            if new_i >= self.scroll_offset + viewport {
                self.scroll_offset = new_i + 1 - viewport;
            }
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
            4 => DetailPane::Timeline,
            5 => DetailPane::WebSocket,
            6 => DetailPane::Http2,
            7 => DetailPane::Grpc,
            8 => DetailPane::StreamMux,
            9 => DetailPane::Correlation,
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

    #[test]
    fn test_visible_flows() {
        let mut tab = InterceptTab::new();
        for i in 0..10 {
            tab.flows.push(ProxyFlow {
                index: i,
                method: "GET".to_string(),
                url: format!("https://example.com/{}", i),
                host: "example.com".to_string(),
                path: format!("/{}", i),
                request_headers: Default::default(),
                request_body: None,
                response_status: 200,
                response_headers: Default::default(),
                response_body: None,
                is_https: true,
                duration_ms: 0,
                request_body_size: 0,
                response_body_size: 0,
                started_at: String::new(),
                completed_at: String::new(),
                redaction_applied: None,
                protocol: "http1".to_string(),
            });
        }

        tab.scroll_offset = 0;
        assert_eq!(tab.visible_flows(5).len(), 5);
        assert_eq!(tab.visible_flows(5)[0].index, 0);

        tab.scroll_offset = 7;
        assert_eq!(tab.visible_flows(5).len(), 3);
        assert_eq!(tab.visible_flows(5)[0].index, 7);

        tab.scroll_offset = 100;
        assert!(tab.visible_flows(5).is_empty());
    }

    #[test]
    fn test_scroll_offset_maintained_on_nav() {
        let mut tab = InterceptTab::new();
        for i in 0..30 {
            tab.flows.push(ProxyFlow {
                index: i,
                method: "GET".to_string(),
                url: format!("https://example.com/{}", i),
                host: "example.com".to_string(),
                path: format!("/{}", i),
                request_headers: Default::default(),
                request_body: None,
                response_status: 200,
                response_headers: Default::default(),
                response_body: None,
                is_https: false,
                duration_ms: 0,
                request_body_size: 0,
                response_body_size: 0,
                started_at: String::new(),
                completed_at: String::new(),
                redaction_applied: None,
                protocol: "http1".to_string(),
            });
        }

        tab.selected_flow = Some(0);
        tab.table_state.select(Some(0));

        tab.handle_down();
        assert_eq!(tab.scroll_offset, 0);
        assert_eq!(tab.selected_flow, Some(1));

        tab.handle_up();
        assert_eq!(tab.scroll_offset, 0);
        assert_eq!(tab.selected_flow, Some(0));
    }

    #[test]
    fn test_performance_mode_toggle() {
        let mut tab = InterceptTab::new();
        assert!(!tab.performance_mode);
        tab.toggle_performance_mode();
        assert!(tab.performance_mode);
        tab.toggle_performance_mode();
        assert!(!tab.performance_mode);
    }

    #[test]
    fn test_estimate_memory_usage() {
        let mut tab = InterceptTab::new();
        assert_eq!(tab.estimate_memory_usage(), 0);

        tab.flows.push(ProxyFlow {
            index: 0,
            method: "GET".to_string(),
            url: "https://example.com/".to_string(),
            host: "example.com".to_string(),
            path: "/".to_string(),
            request_headers: Default::default(),
            request_body: Some("body".to_string()),
            response_status: 200,
            response_headers: Default::default(),
            response_body: Some("response".to_string()),
            is_https: true,
            duration_ms: 0,
            request_body_size: 0,
            response_body_size: 0,
            started_at: String::new(),
            completed_at: String::new(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });

        let usage = tab.estimate_memory_usage();
        assert!(usage > 0);
    }

    #[test]
    fn test_ws_messages_page() {
        let mut tab = InterceptTab::new();
        assert!(tab.ws_messages_page(0, 0, 10).is_empty());
        assert_eq!(tab.ws_session_message_count(0), 0);

        let mut session = WebSocketSession::new("ws://example.com", "example.com", "/ws", false);
        for i in 0..25 {
            session.messages.push(WebSocketMessage {
                direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
                opcode: WebSocketOpcode::Text,
                payload: format!("msg{}", i),
                masked: false,
                payload_size: 0,
                timestamp: String::new(),
                manipulation: None,
            });
        }
        tab.ws_sessions.push(session);

        assert_eq!(tab.ws_messages_page(0, 0, 10).len(), 10);
        assert_eq!(tab.ws_messages_page(0, 0, 10)[0].payload, "msg0");
        assert_eq!(tab.ws_messages_page(0, 1, 10).len(), 10);
        assert_eq!(tab.ws_messages_page(0, 1, 10)[0].payload, "msg10");
        assert_eq!(tab.ws_messages_page(0, 2, 10).len(), 5);
        assert_eq!(tab.ws_messages_page(0, 2, 10)[0].payload, "msg20");
        assert!(tab.ws_messages_page(0, 3, 10).is_empty());
        assert!(tab.ws_messages_page(1, 0, 10).is_empty());
        assert_eq!(tab.ws_session_message_count(0), 25);
    }

    #[test]
    fn test_debounce_state() {
        let mut debounce = DebounceState::new();
        assert!(!debounce.should_apply());
        assert!(debounce.take_if_ready().is_none());

        debounce.enqueue("test".to_string());
        assert!(debounce.pending_filter.is_some());
        // Immediately after enqueue, debounce should not be ready
        assert!(!debounce.should_apply());
    }

    #[test]
    fn test_debounce_take_if_ready() {
        let mut debounce = DebounceState::new();
        debounce.enqueue("filter".to_string());
        assert!(debounce.take_if_ready().is_none());
    }

    #[test]
    fn test_performance_mode_detail_pane() {
        let mut tab = InterceptTab::new();
        tab.flows.push(ProxyFlow {
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
            duration_ms: 0,
            request_body_size: 0,
            response_body_size: 0,
            started_at: String::new(),
            completed_at: String::new(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });
        tab.selected_flow = Some(0);

        tab.toggle_performance_mode();
        assert!(tab.performance_mode);
        assert!(tab.cached_detail.is_none());
    }

    #[test]
    fn test_reset_clears_new_fields() {
        let mut tab = InterceptTab::new();
        tab.scroll_offset = 5;
        tab.performance_mode = true;
        tab.cached_detail = Some((0, DetailPaneContent::Headers(vec![])));
        tab.reset();
        assert_eq!(tab.scroll_offset, 0);
        assert!(!tab.performance_mode);
        assert!(tab.cached_detail.is_none());
        assert!(tab.ws_sessions.is_empty());
    }

    #[test]
    fn test_ensure_selected_visible() {
        let mut tab = InterceptTab::new();
        for i in 0..20 {
            tab.flows.push(ProxyFlow {
                index: i,
                method: "GET".to_string(),
                url: "https://example.com/".to_string(),
                host: "example.com".to_string(),
                path: "/".to_string(),
                request_headers: Default::default(),
                request_body: None,
                response_status: 200,
                response_headers: Default::default(),
                response_body: None,
                is_https: false,
                duration_ms: 0,
                request_body_size: 0,
                response_body_size: 0,
                started_at: String::new(),
                completed_at: String::new(),
                redaction_applied: None,
                protocol: "http1".to_string(),
            });
        }

        tab.selected_flow = Some(15);
        tab.scroll_offset = 0;
        tab.ensure_selected_visible(10);
        assert_eq!(tab.scroll_offset, 6);

        tab.selected_flow = Some(0);
        tab.scroll_offset = 10;
        tab.ensure_selected_visible(10);
        assert_eq!(tab.scroll_offset, 0);
    }

    #[test]
    fn test_stress_10000_flows_memory_estimate() {
        let mut tab = InterceptTab::new();

        // Add 10,000 flows with realistic payload sizes
        for i in 0..10_000 {
            tab.flows.push(ProxyFlow {
                index: i,
                method: if i % 2 == 0 { "GET" } else { "POST" }.to_string(),
                url: format!("https://example.com/api/endpoint/{}", i),
                host: "example.com".to_string(),
                path: format!("/api/endpoint/{}", i),
                request_headers: {
                    let mut h = std::collections::HashMap::new();
                    h.insert("User-Agent".to_string(), "stress-test/1.0".to_string());
                    h.insert("Accept".to_string(), "application/json".to_string());
                    h
                },
                request_body: if i % 2 == 0 {
                    None
                } else {
                    // 500 bytes per request body
                    Some(format!(
                        r#"{{"id": {}, "data": "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Payload padding: {}"}}"#,
                        i, "X".repeat(200)
                    ))
                },
                response_status: if i % 10 == 0 { 404 } else { 200 },
                response_headers: {
                    let mut h = std::collections::HashMap::new();
                    h.insert("Content-Type".to_string(), "application/json".to_string());
                    h
                },
                // 500 bytes per response body
                response_body: Some(format!(
                    r#"{{"status": "ok", "id": {}, "result": "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Payload: {}"}}"#,
                    i, "Y".repeat(100)
                )),
                is_https: true,
                duration_ms: (i as u64) % 1000,
                request_body_size: if i % 2 == 0 { 0 } else { 500 },
                response_body_size: 500,
                started_at: chrono::Utc::now().to_rfc3339(),
                completed_at: chrono::Utc::now().to_rfc3339(),
                redaction_applied: None,
                protocol: "http1".to_string(),
            });
        }

        // Verify memory estimate is reasonable (>10MB for 10k flows with 500B payloads)
        // Each flow has: url (~50B) + method (~4B) + host (11B) + path (~18B)
        //   + request_headers (~35B) + response_headers (~30B)
        //   + request_body (500B for odd) + response_body (500B)
        // Per flow avg: ~650B, 10k flows = ~6.5MB, plus overhead
        let usage = tab.estimate_memory_usage();
        assert!(
            usage > 5 * 1024 * 1024,
            "Memory estimate should be >5MB for 10k flows with 500B payloads, got {} bytes",
            usage
        );

        // Verify virtual scrolling works with large flow count
        let visible = tab.visible_flows(20);
        assert_eq!(visible.len(), 20);
        assert_eq!(visible[0].index, 0);

        tab.scroll_offset = 9980;
        let visible = tab.visible_flows(20);
        assert_eq!(visible.len(), 20);
        assert_eq!(visible[0].index, 9980);

        // Verify navigation works
        tab.selected_flow = Some(5000);
        tab.ensure_selected_visible(20);
        assert!(tab.scroll_offset <= 5000);
        assert!(tab.scroll_offset + 20 > 5000);
    }

    #[test]
    fn test_stress_10000_flows_performance_mode_auto_enables() {
        let mut tab = InterceptTab::new();

        // Add flows up to the threshold using add_flow() which triggers auto-enable
        for i in 0..5001 {
            tab.add_flow(ProxyFlow {
                index: i,
                method: "GET".to_string(),
                url: format!("http://example.com/{}", i),
                host: "example.com".to_string(),
                path: format!("/{}", i),
                request_headers: Default::default(),
                request_body: None,
                response_status: 200,
                response_headers: Default::default(),
                response_body: None,
                is_https: false,
                duration_ms: 0,
                request_body_size: 0,
                response_body_size: 0,
                started_at: String::new(),
                completed_at: String::new(),
                redaction_applied: None,
                protocol: "http1".to_string(),
            });
        }

        // Performance mode should auto-enable after 5000 flows
        assert!(
            tab.performance_mode,
            "Performance mode should auto-enable when flows exceed 5000"
        );
    }

    // ==================== Stream Multiplexing Visualization Tests ====================

    #[test]
    fn test_http2_streams_for_session_sorted() {
        let mut tab = InterceptTab::new();
        let mut session = Http2Session::new("api.example.com", true);

        // Add streams in non-sorted order
        session.add_stream(Http2Stream::new(5, "GET", "/b"));
        session.add_stream(Http2Stream::new(1, "GET", "/a"));
        session.add_stream(Http2Stream::new(3, "POST", "/c"));

        tab.add_http2_session(session);

        let streams = tab.http2_streams_for_session(0);
        assert_eq!(streams.len(), 3);
        assert_eq!(streams[0].stream_id, 1);
        assert_eq!(streams[1].stream_id, 3);
        assert_eq!(streams[2].stream_id, 5);
    }

    #[test]
    fn test_http2_stream_state_counts() {
        let mut tab = InterceptTab::new();
        let mut session = Http2Session::new("api.example.com", true);

        let mut s1 = Http2Stream::new(1, "GET", "/a");
        s1.state = eggsec::proxy::intercept::protocols::Http2StreamState::Open;

        let mut s2 = Http2Stream::new(3, "GET", "/b");
        s2.state = eggsec::proxy::intercept::protocols::Http2StreamState::Closed;

        let mut s3 = Http2Stream::new(5, "GET", "/c");
        s3.state = eggsec::proxy::intercept::protocols::Http2StreamState::HalfClosedRemote;

        let mut s4 = Http2Stream::new(7, "GET", "/d");
        s4.state = eggsec::proxy::intercept::protocols::Http2StreamState::Open;

        let mut s5 = Http2Stream::new(9, "GET", "/e");
        s5.state = eggsec::proxy::intercept::protocols::Http2StreamState::Idle;

        session.add_stream(s1);
        session.add_stream(s2);
        session.add_stream(s3);
        session.add_stream(s4);
        session.add_stream(s5);

        tab.add_http2_session(session);

        let (open, half_closed, closed, idle) = tab.http2_stream_state_counts(0);
        assert_eq!(open, 2);
        assert_eq!(half_closed, 1);
        assert_eq!(closed, 1);
        assert_eq!(idle, 1);
    }

    #[test]
    fn test_http2_session_count() {
        let mut tab = InterceptTab::new();
        assert_eq!(tab.http2_session_count(), 0);

        tab.add_http2_session(Http2Session::new("a.example.com", true));
        tab.add_http2_session(Http2Session::new("b.example.com", true));
        assert_eq!(tab.http2_session_count(), 2);
    }

    #[test]
    fn test_total_http2_streams() {
        let mut tab = InterceptTab::new();
        let mut s1 = Http2Session::new("a.example.com", true);
        s1.add_stream(Http2Stream::new(1, "GET", "/"));
        s1.add_stream(Http2Stream::new(3, "GET", "/"));

        let mut s2 = Http2Session::new("b.example.com", true);
        s2.add_stream(Http2Stream::new(1, "POST", "/"));
        s2.add_stream(Http2Stream::new(3, "GET", "/"));
        s2.add_stream(Http2Stream::new(5, "GET", "/"));

        tab.add_http2_session(s1);
        tab.add_http2_session(s2);

        assert_eq!(tab.total_http2_streams(), 5);
    }

    #[test]
    fn test_next_prev_http2_session() {
        let mut tab = InterceptTab::new();
        tab.add_http2_session(Http2Session::new("a.example.com", true));
        tab.add_http2_session(Http2Session::new("b.example.com", true));
        tab.add_http2_session(Http2Session::new("c.example.com", true));

        assert_eq!(tab.selected_http2_session, 0);
        tab.next_http2_session();
        assert_eq!(tab.selected_http2_session, 1);
        tab.next_http2_session();
        assert_eq!(tab.selected_http2_session, 2);
        tab.next_http2_session(); // wrap around
        assert_eq!(tab.selected_http2_session, 0);

        tab.prev_http2_session(); // wrap around backward
        assert_eq!(tab.selected_http2_session, 2);
        tab.prev_http2_session();
        assert_eq!(tab.selected_http2_session, 1);
    }

    // ==================== gRPC Streaming Visualization Tests ====================

    #[test]
    fn test_grpc_calls_for_session() {
        let mut tab = InterceptTab::new();
        let mut session = GrpcSession::new("api.example.com", true);

        let call = GrpcCall::new("/svc/Method", eggsec::proxy::intercept::protocols::GrpcMethodType::Unary);
        session.add_call(call);

        tab.add_grpc_session(session);
        let calls = tab.grpc_calls_for_session(0);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].path, "/svc/Method");
    }

    #[test]
    fn test_grpc_streaming_summary() {
        let mut tab = InterceptTab::new();
        let mut state = GrpcStreamingState::new(
            eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional,
        );
        state.flow_control_window = 65535;
        state.bytes_in_flight = 0;

        let frame = GrpcStreamFrame {
            stream_id: 1,
            direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
            payload: None,
            size: 100,
            end_stream: false,
            timestamp: String::new(),
            compressed: false,
        };
        state.add_frame(frame);

        tab.add_grpc_streaming_state(state);

        let summary = tab.grpc_streaming_summary(0);
        assert!(summary.is_some());
        let s = summary.unwrap();
        assert!(s.contains("Bidirectional"));
        assert!(s.contains("1 client / 0 server"));
    }

    #[test]
    fn test_total_grpc_stream_frames() {
        let mut tab = InterceptTab::new();
        let mut state1 = GrpcStreamingState::new(
            eggsec::proxy::intercept::protocols::GrpcMethodType::ServerStreaming,
        );
        let mut state2 = GrpcStreamingState::new(
            eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional,
        );

        // 2 client frames to state1
        for _ in 0..2 {
            state1.add_frame(GrpcStreamFrame {
                stream_id: 1,
                direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
                payload: None,
                size: 10,
                end_stream: false,
                timestamp: String::new(),
                compressed: false,
            });
        }

        // 3 server frames to state2
        for _ in 0..3 {
            state2.add_frame(GrpcStreamFrame {
                stream_id: 1,
                direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Response,
                payload: None,
                size: 20,
                end_stream: false,
                timestamp: String::new(),
                compressed: false,
            });
        }

        tab.add_grpc_streaming_state(state1);
        tab.add_grpc_streaming_state(state2);

        assert_eq!(tab.total_grpc_stream_frames(), 5);
    }

    #[test]
    fn test_grpc_stream_frames_page() {
        let mut tab = InterceptTab::new();
        let mut state = GrpcStreamingState::new(
            eggsec::proxy::intercept::protocols::GrpcMethodType::ServerStreaming,
        );

        for i in 0..15 {
            state.add_frame(GrpcStreamFrame {
                stream_id: 1,
                direction: if i % 2 == 0 {
                    eggsec::proxy::intercept::types::ProxyFlowDirection::Request
                } else {
                    eggsec::proxy::intercept::types::ProxyFlowDirection::Response
                },
                payload: None,
                size: 10,
                end_stream: false,
                timestamp: format!("2026-01-01T00:00:{:02}Z", i),
                compressed: false,
            });
        }

        tab.add_grpc_streaming_state(state);

        let page0 = tab.grpc_stream_frames_page(0, 0, 5);
        assert_eq!(page0.len(), 5);

        let page2 = tab.grpc_stream_frames_page(0, 2, 5);
        assert_eq!(page2.len(), 5);

        let page3 = tab.grpc_stream_frames_page(0, 3, 5);
        assert_eq!(page3.len(), 0); // Out of range

        let page_overflow = tab.grpc_stream_frames_page(0, 0, 20);
        assert_eq!(page_overflow.len(), 15);
    }

    #[test]
    fn test_next_prev_grpc_session() {
        let mut tab = InterceptTab::new();
        tab.add_grpc_session(GrpcSession::new("a.example.com", true));
        tab.add_grpc_session(GrpcSession::new("b.example.com", true));

        assert_eq!(tab.selected_grpc_session, 0);
        tab.next_grpc_session();
        assert_eq!(tab.selected_grpc_session, 1);
        tab.next_grpc_session(); // wrap
        assert_eq!(tab.selected_grpc_session, 0);

        tab.prev_grpc_session(); // wrap
        assert_eq!(tab.selected_grpc_session, 1);
    }

    #[test]
    fn test_grpc_session_count() {
        let mut tab = InterceptTab::new();
        assert_eq!(tab.grpc_session_count(), 0);
        tab.add_grpc_session(GrpcSession::new("a.example.com", true));
        assert_eq!(tab.grpc_session_count(), 1);
    }

    // ==================== Correlation Visualization Tests ====================

    #[test]
    fn test_correlation_summary_str_empty() {
        let tab = InterceptTab::new();
        assert_eq!(tab.correlation_summary_str(), "No correlations");
    }

    #[test]
    fn test_correlation_summary_str_with_references() {
        let mut tab = InterceptTab::new();
        let ref1 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-001",
            "SQL injection vulnerability",
        );
        let ref2 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-001",
            "Weak JWT secret",
        );
        tab.add_correlation_reference(0, ref1);
        tab.add_correlation_reference(1, ref2);

        let s = tab.correlation_summary_str();
        assert!(s.contains("2 refs"));
        assert!(s.contains("2 unique sources"));
        assert!(s.contains("2 correlated flows"));
    }

    #[test]
    fn test_correlation_source_counts() {
        let mut tab = InterceptTab::new();
        tab.add_correlation_reference(
            0,
            eggsec::proxy::intercept::correlation::CorrelationReference::new(
                CorrelationSource::DbPentest,
                "db-001",
                "test1",
            ),
        );
        tab.add_correlation_reference(
            1,
            eggsec::proxy::intercept::correlation::CorrelationReference::new(
                CorrelationSource::DbPentest,
                "db-002",
                "test2",
            ),
        );
        tab.add_correlation_reference(
            2,
            eggsec::proxy::intercept::correlation::CorrelationReference::new(
                CorrelationSource::AuthTest,
                "auth-001",
                "test3",
            ),
        );

        let counts = tab.correlation_source_counts();
        // Should be sorted by count descending
        assert_eq!(counts.len(), 2);
        assert_eq!(counts[0].0, CorrelationSource::DbPentest);
        assert_eq!(counts[0].1, 2);
        assert_eq!(counts[1].0, CorrelationSource::AuthTest);
        assert_eq!(counts[1].1, 1);
    }

    #[test]
    fn test_recompute_correlations() {
        let mut tab = InterceptTab::new();
        // Add references with different sources
        let ref1 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-001",
            "DB issue",
        );
        let ref2 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-001",
            "Auth issue",
        );
        tab.add_correlation_reference(0, ref1);
        tab.add_correlation_reference(1, ref2);

        tab.recompute_correlations();

        // Should find at least one temporal correlation (both refs present)
        assert!(!tab.temporal_correlations.is_empty());
    }

    #[test]
    fn test_correlations_for_flow() {
        let mut tab = InterceptTab::new();
        let ref1 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-001",
            "Issue for flow 5",
        );
        tab.add_correlation_reference(5, ref1);

        let corrs = tab.correlations_for_flow(5);
        assert_eq!(corrs.len(), 1);
        assert_eq!(corrs[0].finding_id, "db-001");

        // No correlations for flow 10
        let empty = tab.correlations_for_flow(10);
        assert_eq!(empty.len(), 0);
    }

    // ==================== Detail Pane Variant Tests ====================

    #[test]
    fn test_new_detail_pane_variants() {
        // Ensure all new variants exist
        let _mux = DetailPane::StreamMux;
        let _corr = DetailPane::Correlation;

        // Ensure from_index handles all new variants
        assert_eq!(DetailPane::from_index(8), DetailPane::StreamMux);
        assert_eq!(DetailPane::from_index(9), DetailPane::Correlation);
        assert_eq!(DetailPane::from_index(10), DetailPane::Headers); // overflow
    }

    #[test]
    fn test_reset_clears_stream_mux_and_correlation() {
        let mut tab = InterceptTab::new();
        tab.add_http2_session(Http2Session::new("a.com", true));
        tab.add_grpc_session(GrpcSession::new("a.com", true));
        tab.add_grpc_streaming_state(GrpcStreamingState::new(
            eggsec::proxy::intercept::protocols::GrpcMethodType::Unary,
        ));
        tab.add_correlation_reference(
            0,
            eggsec::proxy::intercept::correlation::CorrelationReference::new(
                CorrelationSource::DbPentest,
                "db-001",
                "test",
            ),
        );
        tab.selected_http2_session = 2;
        tab.stream_mux_scroll = 5;

        tab.reset();

        assert!(tab.http2_sessions.is_empty());
        assert!(tab.grpc_sessions.is_empty());
        assert!(tab.grpc_streaming_states.is_empty());
        assert!(tab.temporal_correlations.is_empty());
        assert_eq!(tab.correlation_context.references.len(), 0);
        assert_eq!(tab.selected_http2_session, 0);
        assert_eq!(tab.stream_mux_scroll, 0);
    }

    // ==================== Render Tests ====================

    #[test]
    fn test_render_stream_multiplexing_with_data() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut tab = InterceptTab::new();
        let mut session = Http2Session::new("api.example.com", true);
        session.add_stream(Http2Stream::new(1, "GET", "/api/data"));
        session.add_stream(Http2Stream::new(3, "POST", "/api/submit"));
        tab.add_http2_session(session);

        let mut grpc_state = GrpcStreamingState::new(
            eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional,
        );
        grpc_state.add_frame(GrpcStreamFrame {
            stream_id: 1,
            direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
            payload: None,
            size: 100,
            end_stream: false,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            compressed: false,
        });
        tab.add_grpc_streaming_state(grpc_state);

        tab.detail_pane = DetailPane::StreamMux;

        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 120, 40);
                tab.render_stream_multiplexing(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // Verify stream multiplexing content was rendered
        let content: String = buffer
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(content.contains("Stream Multiplexing"));
        assert!(content.contains("HTTP/2 Streams"));
        assert!(content.contains("gRPC Streaming"));
        assert!(content.contains("api.example.com"));
    }

    #[test]
    fn test_render_correlation_with_data() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut tab = InterceptTab::new();
        tab.add_correlation_reference(
            0,
            eggsec::proxy::intercept::correlation::CorrelationReference::new(
                CorrelationSource::DbPentest,
                "db-001",
                "SQL injection in /api/users",
            ),
        );
        tab.add_correlation_reference(
            0,
            eggsec::proxy::intercept::correlation::CorrelationReference::new(
                CorrelationSource::AuthTest,
                "auth-001",
                "Missing authentication on /api/admin",
            ),
        );
        tab.recompute_correlations();
        tab.selected_flow = Some(0);
        tab.detail_pane = DetailPane::Correlation;

        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 120, 40);
                tab.render_correlation(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(content.contains("Cross-Loadout Correlation"));
        assert!(content.contains("DB-Pentest"));
        assert!(content.contains("Auth-Test"));
        assert!(content.contains("Temporal Correlations"));
    }

    #[test]
    fn test_render_stream_multiplexing_empty() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut tab = InterceptTab::new();
        tab.detail_pane = DetailPane::StreamMux;

        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 120, 30);
                tab.render_stream_multiplexing(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(content.contains("Stream Multiplexing"));
        assert!(content.contains("No HTTP/2 sessions"));
    }

    #[test]
    fn test_render_correlation_empty() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut tab = InterceptTab::new();
        tab.detail_pane = DetailPane::Correlation;

        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 120, 30);
                tab.render_correlation(f, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(content.contains("Cross-Loadout Correlation"));
        assert!(content.contains("No correlations"));
    }

    #[test]
    fn test_render_protocol_info_http2_with_sessions() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut tab = InterceptTab::new();
        // Add a flow first (required for render_protocol_info)
        let flow = ProxyFlow {
            index: 0,
            method: "GET".to_string(),
            url: "https://api.example.com/data".to_string(),
            host: "api.example.com".to_string(),
            path: "/data".to_string(),
            request_headers: Default::default(),
            request_body: None,
            response_status: 200,
            response_headers: Default::default(),
            response_body: None,
            is_https: true,
            duration_ms: 50,
            request_body_size: 0,
            response_body_size: 1024,
            started_at: String::new(),
            completed_at: String::new(),
            redaction_applied: None,
            protocol: "h2".to_string(),
        };
        tab.add_flow(flow);
        tab.selected_flow = Some(0);

        // Add an HTTP/2 session
        let mut session = Http2Session::new("api.example.com", true);
        session.tune_windows(1024 * 1024, 1024 * 1024);
        session.add_stream(Http2Stream::new(1, "GET", "/data"));
        tab.add_http2_session(session);

        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 120, 30);
                tab.render_protocol_info(f, area, "HTTP/2");
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(content.contains("HTTP/2 Protocol Details"));
        assert!(content.contains("Captured Sessions"));
        assert!(content.contains("api.example.com"));
        assert!(content.contains("Stream Mux"));
    }

    #[test]
    fn test_render_protocol_info_grpc_with_sessions() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut tab = InterceptTab::new();
        let flow = ProxyFlow {
            index: 0,
            method: "POST".to_string(),
            url: "https://api.example.com/svc/Method".to_string(),
            host: "api.example.com".to_string(),
            path: "/svc/Method".to_string(),
            request_headers: Default::default(),
            request_body: None,
            response_status: 200,
            response_headers: Default::default(),
            response_body: None,
            is_https: true,
            duration_ms: 30,
            request_body_size: 50,
            response_body_size: 100,
            started_at: String::new(),
            completed_at: String::new(),
            redaction_applied: None,
            protocol: "grpc".to_string(),
        };
        tab.add_flow(flow);
        tab.selected_flow = Some(0);

        // Add a gRPC session with a call
        let mut session = GrpcSession::new("api.example.com", true);
        session.add_call(GrpcCall::new("/svc/Method", eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional));
        tab.add_grpc_session(session);

        // Add a security finding
        tab.add_grpc_security_finding(eggsec::proxy::intercept::protocols::GrpcSecurityFinding {
            service_path: "/svc/Method".to_string(),
            category: "missing_auth".to_string(),
            severity: 7,
            description: "Method does not require authentication".to_string(),
            remediation: Some("Add auth interceptor".to_string()),
        });

        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 120, 30);
                tab.render_protocol_info(f, area, "gRPC");
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(content.contains("gRPC Protocol Details"));
        assert!(content.contains("BIDI"));
        assert!(content.contains("Security Findings"));
        assert!(content.contains("missing_auth"));
    }

    #[test]
    fn test_render_detail_pane_handles_new_variants_without_flow() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        // Test StreamMux pane without a flow selected
        let mut tab = InterceptTab::new();
        tab.detail_pane = DetailPane::StreamMux;
        let area = ratatui::layout::Rect::new(0, 0, 120, 30);
        terminal
            .draw(|f| tab.render_detail_pane(f, area))
            .unwrap();

        // Test Correlation pane without a flow selected
        tab.detail_pane = DetailPane::Correlation;
        terminal
            .draw(|f| tab.render_detail_pane(f, area))
            .unwrap();
    }
}
