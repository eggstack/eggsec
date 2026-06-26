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

mod render;
mod types;
mod utils;
#[cfg(test)]
mod tests;

pub use types::*;
use utils::{truncate_str, format_bytes};

#[macro_export]
macro_rules! inner {
    ($area:expr, $margin:expr) => {
        Rect::new($area.x + $margin, $area.y + $margin, $area.width - $margin * 2, $area.height - $margin * 2)
    };
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
                        Ok(json) => match tokio::fs::write(&path, json).await {
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
        self.close_edit_modal();
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
                        .border_style(Style::default().fg(tc!(error)))
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
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(tc!(error))).title(" Too Small "))
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
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(tc!(border))).title(" Status "))
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

    fn page_up(&mut self, page_size: usize) {
        if !self.is_running() && self.focus_area == InterceptFocusArea::FlowList {
            let i = self.selected_flow.unwrap_or(0);
            let new_i = i.saturating_sub(page_size);
            self.selected_flow = Some(new_i);
            self.table_state.select(Some(new_i));
            if new_i < self.scroll_offset {
                self.scroll_offset = new_i;
            }
        }
    }

    fn page_down(&mut self, page_size: usize) {
        if !self.is_running() && self.focus_area == InterceptFocusArea::FlowList {
            let i = self.selected_flow.unwrap_or(0);
            let new_i = (i + page_size).min(self.flows.len().saturating_sub(1));
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

