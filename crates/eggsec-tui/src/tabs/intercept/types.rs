use eggsec::proxy::intercept::types::ProxyFlowDirection;

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
    pub direction: ProxyFlowDirection,
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
