//! The GUI's bounded view of the daemon's event stream.
//!
//! The event stream is the source of truth. This module holds a *bounded
//! view* of it: the current status of each task, its progress, and a capped
//! ring of log lines.
//!
//! # Bounded retention is a hard requirement
//!
//! [`RuntimeEvent::TaskLog`] is the high-cardinality event type. A task that
//! emits thousands of lines must not be able to grow the GUI's memory without
//! bound, so each task keeps at most [`MAX_LOG_LINES_PER_TASK`] lines and
//! counts the rest. The count is reported, never silently swallowed: an
//! operator who sees "42 earlier lines dropped" knows the view is partial.

use std::collections::HashMap;

use eggsec_runtime::event::{LogLevel, PolicyPrompt, RuntimeEvent, TaskOutcome, TaskProgress};
use eggsec_runtime::request::TaskKind;
use eggsec_runtime::TaskId;
use eggsec_ui_model::renderer_registry::renderer_for_kind;

/// The maximum number of log lines retained per task.
///
/// 200 lines is enough to diagnose a failed task and small enough that even
/// a hundred concurrent tasks stay well inside a desktop application's
/// comfort zone. The cap is a named constant, not a tuning knob: exceeding
/// it drops the *oldest* lines and increments a visible counter.
pub const MAX_LOG_LINES_PER_TASK: usize = 200;

/// The maximum number of log lines forwarded to the webview in one batch.
///
/// A task that logs faster than the UI renders must not build an unbounded
/// queue in the GUI. The forwarder keeps the newest lines and reports how
/// many it dropped.
pub const MAX_LOG_LINES_PER_FLUSH: usize = 50;

/// The status the GUI shows for a task.
///
/// This is the GUI's own view of the runtime's status, not the runtime's
/// enum: the GUI adds `AwaitingPolicy`, which the runtime reports as a
/// separate event rather than a status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiTaskStatus {
    /// Submitted, not started.
    Queued,
    /// Running.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with an error.
    Failed,
    /// Cancelled.
    Cancelled,
    /// Blocked on a policy decision the connected daemon cannot accept.
    AwaitingPolicy,
    /// The GUI has seen the task but cannot classify its state.
    Unknown,
}

/// One log line.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LogLine {
    /// The runtime's log level.
    pub level: LogLevel,
    /// The log message.
    pub message: String,
}

/// A bounded ring of log lines for one task.
#[derive(Debug, Clone)]
pub struct LogRing {
    lines: std::collections::VecDeque<LogLine>,
    capacity: usize,
    dropped: u64,
}

impl LogRing {
    /// A ring with the default capacity.
    pub fn new() -> Self {
        Self::with_capacity(MAX_LOG_LINES_PER_TASK)
    }

    /// A ring with an explicit capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            lines: std::collections::VecDeque::with_capacity(capacity.min(1024)),
            capacity,
            dropped: 0,
        }
    }

    /// Append a line, dropping the oldest one when the ring is full.
    ///
    /// Returns `true` when a line was dropped, so the caller can count the
    /// loss rather than swallow it.
    pub fn push(&mut self, line: LogLine) -> bool {
        if self.capacity == 0 {
            self.dropped = self.dropped.saturating_add(1);
            return true;
        }
        let mut dropped = false;
        while self.lines.len() >= self.capacity {
            if self.lines.pop_front().is_some() {
                self.dropped = self.dropped.saturating_add(1);
                dropped = true;
            }
        }
        self.lines.push_back(line);
        dropped
    }

    /// The retained lines, oldest first.
    pub fn lines(&self) -> impl Iterator<Item = &LogLine> {
        self.lines.iter()
    }

    /// How many lines were dropped because the ring was full.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// How many lines are retained.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the ring holds no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

impl Default for LogRing {
    fn default() -> Self {
        Self::new()
    }
}

/// The GUI's state for one task.
#[derive(Debug, Clone)]
pub struct TaskState {
    /// The task's identifier.
    pub task_id: TaskId,
    /// The task kind's wire discriminator, as the runtime tagged it.
    pub variant: String,
    /// The task's current status.
    pub status: GuiTaskStatus,
    /// The latest progress report.
    pub progress: Option<TaskProgress>,
    /// The capped log ring.
    pub logs: LogRing,
    /// The outcome, once the task completed.
    pub outcome: Option<TaskOutcome>,
    /// The failure message, once the task failed.
    pub error: Option<String>,
    /// The pending policy prompt, when the task is blocked on a decision.
    pub policy_prompt: Option<PolicyPrompt>,
}

impl TaskState {
    /// A fresh task entry.
    pub fn new(task_id: TaskId, variant: String) -> Self {
        Self {
            task_id,
            variant,
            status: GuiTaskStatus::Queued,
            progress: None,
            logs: LogRing::new(),
            outcome: None,
            error: None,
            policy_prompt: None,
        }
    }
}

/// What applying an event to the registry did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventEffect {
    /// The event changed a task's state; the webview should refresh it.
    TaskUpdated(TaskId),
    /// The event added log lines to a task.
    Logs {
        /// The task the lines belong to.
        task_id: TaskId,
        /// The lines that were added.
        lines: Vec<LogLine>,
    },
    /// The event was not about a task the GUI tracks.
    Ignored,
}

/// Every task the GUI knows about, with bounded per-task retention.
#[derive(Debug, Default)]
pub struct TaskRegistry {
    tasks: HashMap<TaskId, TaskState>,
    /// Insertion order, so the dashboard does not reshuffle between renders.
    order: Vec<TaskId>,
    /// Log lines dropped because a task's ring was full.
    dropped_log_lines: u64,
    /// Events about tasks the GUI does not know about.
    unknown_task_events: u64,
}

impl TaskRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one event from the daemon's stream.
    pub fn apply(&mut self, event: &RuntimeEvent) -> EventEffect {
        match event {
            RuntimeEvent::TaskQueued {
                task_id, request, ..
            } => {
                let variant = task_kind_variant(&request.task_kind);
                if !self.tasks.contains_key(task_id) {
                    self.order.push(*task_id);
                    self.tasks
                        .insert(*task_id, TaskState::new(*task_id, variant.clone()));
                }
                let entry = self
                    .tasks
                    .get_mut(task_id)
                    .expect("the task was just inserted");
                entry.variant = variant;
                entry.status = GuiTaskStatus::Queued;
                EventEffect::TaskUpdated(*task_id)
            }
            RuntimeEvent::TaskStarted { task_id, .. } => match self.tasks.get_mut(task_id) {
                Some(task) => {
                    task.status = GuiTaskStatus::Running;
                    EventEffect::TaskUpdated(*task_id)
                }
                None => {
                    self.unknown_task_events = self.unknown_task_events.saturating_add(1);
                    EventEffect::Ignored
                }
            },
            RuntimeEvent::TaskProgress {
                task_id, progress, ..
            } => match self.tasks.get_mut(task_id) {
                Some(task) => {
                    task.progress = Some(progress.clone());
                    task.status = GuiTaskStatus::Running;
                    EventEffect::TaskUpdated(*task_id)
                }
                None => {
                    self.unknown_task_events = self.unknown_task_events.saturating_add(1);
                    EventEffect::Ignored
                }
            },
            RuntimeEvent::TaskLog {
                task_id,
                level,
                message,
                ..
            } => match task_id {
                Some(task_id) => match self.tasks.get_mut(task_id) {
                    Some(task) => {
                        let line = LogLine {
                            level: level.clone(),
                            message: message.clone(),
                        };
                        if task.logs.push(line.clone()) {
                            self.dropped_log_lines = self.dropped_log_lines.saturating_add(1);
                        }
                        EventEffect::Logs {
                            task_id: *task_id,
                            lines: vec![line],
                        }
                    }
                    None => {
                        self.unknown_task_events = self.unknown_task_events.saturating_add(1);
                        EventEffect::Ignored
                    }
                },
                // A session-level log line belongs to no task.
                None => EventEffect::Ignored,
            },
            RuntimeEvent::PolicyDecisionRequired {
                task_id, prompt, ..
            } => match task_id {
                Some(task_id) => match self.tasks.get_mut(task_id) {
                    Some(task) => {
                        task.status = GuiTaskStatus::AwaitingPolicy;
                        task.policy_prompt = Some(prompt.clone());
                        EventEffect::TaskUpdated(*task_id)
                    }
                    None => {
                        self.unknown_task_events = self.unknown_task_events.saturating_add(1);
                        EventEffect::Ignored
                    }
                },
                None => {
                    self.unknown_task_events = self.unknown_task_events.saturating_add(1);
                    EventEffect::Ignored
                }
            },
            RuntimeEvent::TaskCompleted {
                task_id, outcome, ..
            } => match self.tasks.get_mut(task_id) {
                Some(task) => {
                    task.status = GuiTaskStatus::Completed;
                    task.outcome = Some(outcome.clone());
                    EventEffect::TaskUpdated(*task_id)
                }
                None => {
                    self.unknown_task_events = self.unknown_task_events.saturating_add(1);
                    EventEffect::Ignored
                }
            },
            RuntimeEvent::TaskFailed { task_id, error, .. } => match self.tasks.get_mut(task_id) {
                Some(task) => {
                    task.status = GuiTaskStatus::Failed;
                    task.error = Some(error.message.clone());
                    EventEffect::TaskUpdated(*task_id)
                }
                None => {
                    self.unknown_task_events = self.unknown_task_events.saturating_add(1);
                    EventEffect::Ignored
                }
            },
            RuntimeEvent::TaskCancelled {
                task_id, reason, ..
            } => match self.tasks.get_mut(task_id) {
                Some(task) => {
                    task.status = GuiTaskStatus::Cancelled;
                    task.error = reason.clone();
                    EventEffect::TaskUpdated(*task_id)
                }
                None => {
                    self.unknown_task_events = self.unknown_task_events.saturating_add(1);
                    EventEffect::Ignored
                }
            },
            RuntimeEvent::SessionCreated { .. }
            | RuntimeEvent::Snapshot { .. }
            | RuntimeEvent::SessionClosed { .. }
            | RuntimeEvent::Audit { .. } => EventEffect::Ignored,
        }
    }

    /// Register a task the GUI submitted, before the daemon's event for it
    /// arrives.
    pub fn ensure_task(&mut self, task_id: TaskId, variant: String) {
        if !self.tasks.contains_key(&task_id) {
            self.order.push(task_id);
            self.tasks.insert(task_id, TaskState::new(task_id, variant));
        }
    }

    /// The tasks, in the order they were first seen.
    pub fn tasks(&self) -> impl Iterator<Item = &TaskState> {
        self.order
            .iter()
            .filter_map(|task_id| self.tasks.get(task_id))
    }

    /// The number of tracked tasks.
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Whether no task is tracked.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// The total number of log lines dropped across every task.
    pub fn dropped_log_lines(&self) -> u64 {
        self.dropped_log_lines
    }

    /// The number of events about tasks the GUI does not know about.
    pub fn unknown_task_events(&self) -> u64 {
        self.unknown_task_events
    }
}

/// The wire variant of a task kind, read from the runtime's own tagged
/// representation.
///
/// This is how the GUI learns which operation a task is running without
/// naming any variant: the runtime's `TaskKind` is adjacently tagged, so its
/// discriminator is data.
pub fn task_kind_variant(task_kind: &TaskKind) -> String {
    serde_json::to_value(task_kind)
        .ok()
        .and_then(|value| {
            value
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// One artifact reference, as the webview renders it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactView {
    /// The artifact's identifier.
    pub artifact_id: String,
    /// The artifact's kind.
    pub kind: String,
    /// Where the artifact was written, when it was.
    pub path: Option<String>,
    /// The artifact's media type, when it has one.
    pub mime_type: Option<String>,
    /// The artifact's summary, when it has one.
    pub summary: Option<String>,
}

/// A rendered result.
///
/// The result view is driven by the shared renderer registry, never by a
/// GUI-side table: the registry says which kinds exist, what to call them,
/// and which payload fields make a useful summary line. A kind the registry
/// does not know degrades to a generic JSON render — it never renders as an
/// empty view.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResultView {
    /// The result kind, as the engine reported it.
    pub kind: String,
    /// The registry's human label for the kind.
    pub kind_label: String,
    /// The result's summary, when it has one.
    pub summary: Option<String>,
    /// The registry's summary fields, resolved against the payload.
    pub summary_fields: Vec<(String, String)>,
    /// The raw result payload.
    pub payload: serde_json::Value,
    /// The artifacts the result produced.
    pub artifacts: Vec<ArtifactView>,
    /// Whether offering the raw JSON detail is meaningful for this kind.
    pub supports_json_detail: bool,
    /// Whether the kind is unknown to the renderer registry.
    pub generic: bool,
}

/// Build the rendered view of a task outcome.
pub fn result_view(outcome: &TaskOutcome) -> Option<ResultView> {
    match outcome {
        TaskOutcome::Result(envelope) => {
            let renderer = renderer_for_kind(&envelope.kind);
            let summary_fields = renderer
                .map(|descriptor| {
                    descriptor
                        .summary_fields
                        .iter()
                        .filter_map(|key| {
                            envelope
                                .payload
                                .get(*key)
                                .map(|value| ((*key).to_owned(), scalar_to_string(value)))
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(ResultView {
                kind: envelope.kind.clone(),
                kind_label: renderer
                    .map_or("Unknown", |descriptor| descriptor.title)
                    .to_owned(),
                summary: envelope.summary.clone(),
                summary_fields,
                payload: envelope.payload.clone(),
                artifacts: envelope
                    .artifacts
                    .iter()
                    .map(|artifact| ArtifactView {
                        artifact_id: artifact.id.clone(),
                        kind: artifact.kind.clone(),
                        path: artifact.path.clone(),
                        mime_type: artifact.mime_type.clone(),
                        summary: artifact.summary.clone(),
                    })
                    .collect(),
                supports_json_detail: renderer
                    .is_none_or(|descriptor| descriptor.supports_json_detail),
                generic: renderer.is_none(),
            })
        }
        TaskOutcome::Json(value) => Some(ResultView {
            kind: "json".to_owned(),
            kind_label: "JSON".to_owned(),
            summary: value
                .get("summary")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            summary_fields: Vec::new(),
            payload: value.clone(),
            artifacts: Vec::new(),
            supports_json_detail: true,
            generic: true,
        }),
        TaskOutcome::Text(text) => Some(ResultView {
            kind: "text".to_owned(),
            kind_label: "Text".to_owned(),
            summary: Some(text.clone()),
            summary_fields: Vec::new(),
            payload: serde_json::Value::String(text.clone()),
            artifacts: Vec::new(),
            supports_json_detail: true,
            generic: true,
        }),
        TaskOutcome::Artifact {
            artifact_id,
            summary,
        } => Some(ResultView {
            kind: "artifact".to_owned(),
            kind_label: "Artifact".to_owned(),
            summary: summary.clone(),
            summary_fields: Vec::new(),
            payload: serde_json::Value::Null,
            artifacts: vec![ArtifactView {
                artifact_id: artifact_id.clone(),
                kind: "artifact".to_owned(),
                path: None,
                mime_type: None,
                summary: summary.clone(),
            }],
            supports_json_detail: true,
            generic: true,
        }),
        TaskOutcome::Empty => None,
    }
}

/// Render a payload value as the short string a summary line shows.
fn scalar_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => "null".to_owned(),
        other => other.to_string(),
    }
}

/// One task, as the webview renders it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskRow {
    /// The task's identifier.
    pub task_id: String,
    /// The task kind's wire variant.
    pub variant: String,
    /// The task's current status.
    pub status: GuiTaskStatus,
    /// How much of the task is done.
    pub completed: u64,
    /// How much the task has to do, when it says.
    pub total: Option<u64>,
    /// The latest progress message.
    pub message: Option<String>,
    /// The retained log lines, oldest first.
    pub logs: Vec<LogLine>,
    /// How many log lines were dropped for this task.
    pub dropped_log_lines: u64,
    /// The rendered result, once the task completed.
    pub result: Option<ResultView>,
    /// The failure message, once the task failed.
    pub error: Option<String>,
}

impl TaskState {
    /// The webview's view of this task.
    pub fn to_row(&self) -> TaskRow {
        TaskRow {
            task_id: self.task_id.to_string(),
            variant: self.variant.clone(),
            status: self.status,
            completed: self
                .progress
                .as_ref()
                .map_or(0, |progress| progress.completed),
            total: self.progress.as_ref().and_then(|progress| progress.total),
            message: self
                .progress
                .as_ref()
                .and_then(|progress| progress.message.clone()),
            logs: self.logs.lines().cloned().collect(),
            dropped_log_lines: self.logs.dropped(),
            result: self.outcome.as_ref().and_then(result_view),
            error: self.error.clone(),
        }
    }
}

/// The dashboard payload.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskDashboardView {
    /// The tasks, in the order they were first seen.
    pub tasks: Vec<TaskRow>,
    /// Log lines dropped across every task.
    pub dropped_log_lines: u64,
    /// Events about tasks the GUI does not know about.
    pub unknown_task_events: u64,
}

impl TaskRegistry {
    /// The webview's view of the dashboard.
    pub fn to_view(&self) -> TaskDashboardView {
        TaskDashboardView {
            tasks: self.tasks().map(TaskState::to_row).collect(),
            dropped_log_lines: self.dropped_log_lines(),
            unknown_task_events: self.unknown_task_events(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_runtime::event::{RuntimeErrorInfo, TaskResultEnvelope};
    use eggsec_runtime::ids::SessionId;
    use eggsec_runtime::request::PortScanParams;

    fn port_scan() -> TaskKind {
        TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".to_owned(),
            ports: None,
            scan_type: None,
            timeout_ms: None,
            concurrency: None,
            udp: None,
        })
    }

    fn request() -> eggsec_runtime::RunRequest {
        eggsec_runtime::RunRequest {
            task_kind: port_scan(),
            requested_by: None,
            surface: eggsec_runtime::request::RuntimeSurface::DesktopGuiManual,
            labels: Vec::new(),
        }
    }

    fn session() -> SessionId {
        SessionId::new()
    }

    #[test]
    fn the_status_follows_the_event_stream() {
        let mut registry = TaskRegistry::new();
        let task_id = TaskId::new();

        registry.apply(&RuntimeEvent::TaskQueued {
            session_id: session(),
            task_id,
            request: request(),
        });
        assert_eq!(
            registry.tasks().next().expect("a task").status,
            GuiTaskStatus::Queued
        );

        registry.apply(&RuntimeEvent::TaskStarted {
            session_id: session(),
            task_id,
        });
        assert_eq!(
            registry.tasks().next().expect("a task").status,
            GuiTaskStatus::Running
        );

        registry.apply(&RuntimeEvent::TaskProgress {
            session_id: session(),
            task_id,
            progress: TaskProgress {
                completed: 3,
                total: Some(10),
                message: Some("scanning".to_owned()),
            },
        });
        let task = registry.tasks().next().expect("a task");
        assert_eq!(task.status, GuiTaskStatus::Running);
        assert_eq!(task.progress.as_ref().expect("progress").completed, 3);

        registry.apply(&RuntimeEvent::TaskCompleted {
            session_id: session(),
            task_id,
            outcome: TaskOutcome::Text("done".to_owned()),
        });
        assert_eq!(
            registry.tasks().next().expect("a task").status,
            GuiTaskStatus::Completed
        );
    }

    #[test]
    fn a_failure_carries_the_error_message() {
        let mut registry = TaskRegistry::new();
        let task_id = TaskId::new();
        registry.apply(&RuntimeEvent::TaskQueued {
            session_id: session(),
            task_id,
            request: request(),
        });
        registry.apply(&RuntimeEvent::TaskFailed {
            session_id: session(),
            task_id,
            error: RuntimeErrorInfo {
                message: "target unreachable".to_owned(),
                code: None,
                details: None,
            },
        });
        let task = registry.tasks().next().expect("a task");
        assert_eq!(task.status, GuiTaskStatus::Failed);
        assert_eq!(task.error.as_deref(), Some("target unreachable"));
    }

    #[test]
    fn a_cancellation_is_a_status_and_not_an_error() {
        let mut registry = TaskRegistry::new();
        let task_id = TaskId::new();
        registry.apply(&RuntimeEvent::TaskQueued {
            session_id: session(),
            task_id,
            request: request(),
        });
        registry.apply(&RuntimeEvent::TaskCancelled {
            session_id: session(),
            task_id,
            reason: Some("operator cancelled".to_owned()),
        });
        let task = registry.tasks().next().expect("a task");
        assert_eq!(task.status, GuiTaskStatus::Cancelled);
        assert_eq!(task.error.as_deref(), Some("operator cancelled"));
    }

    #[test]
    fn a_policy_decision_marks_the_task_and_grants_no_approval() {
        let mut registry = TaskRegistry::new();
        let task_id = TaskId::new();
        registry.apply(&RuntimeEvent::TaskQueued {
            session_id: session(),
            task_id,
            request: request(),
        });
        registry.apply(&RuntimeEvent::PolicyDecisionRequired {
            session_id: session(),
            task_id: Some(task_id),
            prompt: PolicyPrompt {
                message: "this operation needs an explicit approval".to_owned(),
                confirmation_class: None,
                requires_explicit_approval: true,
            },
        });
        let task = registry.tasks().next().expect("a task");
        assert_eq!(task.status, GuiTaskStatus::AwaitingPolicy);
        assert!(task.policy_prompt.is_some());
    }

    /// A log flood retains at most the cap and reports what it dropped.
    #[test]
    fn a_log_flood_is_bounded_and_counted() {
        let mut registry = TaskRegistry::new();
        let task_id = TaskId::new();
        registry.apply(&RuntimeEvent::TaskQueued {
            session_id: session(),
            task_id,
            request: request(),
        });

        for index in 0..10_000u32 {
            registry.apply(&RuntimeEvent::TaskLog {
                session_id: session(),
                task_id: Some(task_id),
                level: LogLevel::Info,
                message: format!("line {index}"),
            });
        }

        let task = registry.tasks().next().expect("a task");
        assert_eq!(
            task.logs.len(),
            MAX_LOG_LINES_PER_TASK,
            "the ring must retain at most the cap"
        );
        assert!(
            task.logs.dropped() > 0,
            "the dropped-line counter must be non-zero after a flood"
        );
        // The newest lines are the ones that survive.
        let last = task.logs.lines().last().expect("a retained line");
        assert_eq!(last.message, "line 9999");
    }

    /// Events about tasks the GUI does not know about are counted, not
    /// silently dropped.
    #[test]
    fn unknown_task_events_are_counted() {
        let mut registry = TaskRegistry::new();
        let unknown = TaskId::new();
        registry.apply(&RuntimeEvent::TaskStarted {
            session_id: session(),
            task_id: unknown,
        });
        registry.apply(&RuntimeEvent::TaskCompleted {
            session_id: session(),
            task_id: unknown,
            outcome: TaskOutcome::Empty,
        });
        assert_eq!(registry.unknown_task_events(), 2);
        assert!(registry.is_empty());
    }

    /// The task's variant is read from the runtime's own tagged
    /// representation — the GUI names no variant itself.
    #[test]
    fn the_variant_is_read_from_the_task_kind() {
        assert_eq!(task_kind_variant(&port_scan()), "PortScan");
    }

    /// The registry keeps insertion order, so the dashboard does not
    /// reshuffle between renders.
    #[test]
    fn tasks_keep_their_insertion_order() {
        let mut registry = TaskRegistry::new();
        let first = TaskId::new();
        let second = TaskId::new();
        registry.apply(&RuntimeEvent::TaskQueued {
            session_id: session(),
            task_id: first,
            request: request(),
        });
        registry.apply(&RuntimeEvent::TaskQueued {
            session_id: session(),
            task_id: second,
            request: request(),
        });
        let order: Vec<TaskId> = registry.tasks().map(|task| task.task_id).collect();
        assert_eq!(order, vec![first, second]);
    }

    /// A structured outcome reaches the GUI as a result the renderer
    /// registry can describe.
    #[test]
    fn a_structured_outcome_is_renderable() {
        let mut registry = TaskRegistry::new();
        let task_id = TaskId::new();
        registry.apply(&RuntimeEvent::TaskQueued {
            session_id: session(),
            task_id,
            request: request(),
        });
        registry.apply(&RuntimeEvent::TaskCompleted {
            session_id: session(),
            task_id,
            outcome: TaskOutcome::Result(TaskResultEnvelope {
                kind: "port-scan".to_owned(),
                summary: Some("12 open ports".to_owned()),
                payload: serde_json::json!({"ports_open": 12}),
                artifacts: Vec::new(),
            }),
        });
        let task = registry.tasks().next().expect("a task");
        let outcome = task.outcome.as_ref().expect("an outcome");
        let TaskOutcome::Result(envelope) = outcome else {
            panic!("expected a structured outcome");
        };
        assert!(renderer_for_kind(&envelope.kind).is_some());
    }
}
