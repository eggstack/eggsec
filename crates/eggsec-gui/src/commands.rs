//! The Tauri IPC command surface.
//!
//! This is the **only** path from the webview to the daemon, and every
//! command is a narrow pass-through to one operation. There is deliberately
//! no command that forwards an arbitrary daemon message, that sets the
//! session surface, or that sets the scope: the webview can ask for the
//! operation catalog, ask for a form, submit a form, and cancel work — but
//! the surface is always `DesktopGuiManual`, stamped in Rust, and the scope
//! is whatever the operator supplied.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eggsec_daemon_protocol::operations::TaskKindDescription;
use eggsec_frontend_client::RuntimeClient;
use eggsec_output::convert::ScanReportData;
use eggsec_runtime::TaskId;
use tauri::Emitter;
use tokio::sync::Mutex;

use crate::availability::{denial_from_message, Denial, DenialLedger};
use crate::config::GuiOptions;
use crate::connection::GuiConnection;
use crate::coverage::{self, TaskCoverage};
use crate::daemon::{self, DaemonAttachment, SupervisedDaemon};
use crate::export::{self, ReportFormat};
use crate::form::{self, Control};
use crate::tasks::{
    EventEffect, LogLine, TaskDashboardView, TaskRegistry, TaskRow, TaskState,
    MAX_LOG_LINES_PER_FLUSH,
};

/// The event name the GUI emits on the Tauri event bus.
pub const TASK_EVENT: &str = "eggsec://task-event";

/// The GUI's shared state.
#[derive(Default)]
pub struct GuiState {
    /// The live connection, if the operator has connected.
    inner: Mutex<Option<ConnectedState>>,
    /// The options the GUI was started with.
    options: GuiOptions,
    /// The denials the GUI has learned from live failures.
    ///
    /// Bounded: one entry per published task kind.
    denials: Arc<Mutex<DenialLedger>>,
    /// The daemon this GUI spawned and supervises, when it spawned one.
    ///
    /// A standard mutex rather than the asynchronous one: the GUI's exit
    /// path takes the daemon out of it without being able to await.
    supervised: std::sync::Mutex<Option<SupervisedDaemon>>,
}

impl GuiState {
    /// A state built from the options the GUI was started with.
    pub fn new(options: GuiOptions) -> Self {
        Self {
            inner: Mutex::new(None),
            options,
            denials: Arc::new(Mutex::new(DenialLedger::default())),
            supervised: std::sync::Mutex::new(None),
        }
    }

    /// Take the supervised daemon out of the state, if there is one.
    ///
    /// The GUI's exit path uses this: the daemon the GUI started is the
    /// GUI's to stop, and stopping it needs no runtime.
    pub fn take_supervised(&self) -> Option<SupervisedDaemon> {
        match self.supervised.lock() {
            Ok(mut guard) => guard.take(),
            Err(error) => {
                tracing::error!("the supervised daemon's handle is poisoned: {error}");
                None
            }
        }
    }

    /// The supervised daemon, without taking it out of the state.
    pub fn supervised(&self) -> Option<SupervisedDaemon> {
        match self.supervised.lock() {
            Ok(guard) => guard.clone(),
            Err(error) => {
                tracing::error!("the supervised daemon's handle is poisoned: {error}");
                None
            }
        }
    }

    /// Remember the daemon this GUI started.
    pub fn set_supervised(&self, supervised: SupervisedDaemon) {
        match self.supervised.lock() {
            Ok(mut guard) => *guard = Some(supervised),
            Err(error) => {
                tracing::error!("the supervised daemon's handle is poisoned: {error}");
            }
        }
    }
}

/// What the GUI holds once it is connected.
pub struct ConnectedState {
    /// The socket the connection was opened against.
    pub socket_path: String,
    /// The live connection.
    pub connection: GuiConnection,
    /// The bounded task view, shared with the event forwarder.
    pub tasks: Arc<Mutex<TaskRegistry>>,
}

/// The live connection's parts, cloned out from under the state lock.
///
/// A command must not hold the GUI's state lock across an await: the lock
/// would serialize every webview call behind the daemon's round trips. The
/// parts are all cheap to clone — the catalog sits behind an `Arc`, and the
/// client behind its own.
struct ConnectionParts {
    client: eggsec_frontend_client::DaemonRuntimeClient,
    session_id: eggsec_runtime::SessionId,
    catalog: std::sync::Arc<eggsec_daemon_protocol::operations::DescribeOperationsResponse>,
    capabilities: eggsec_daemon_protocol::protocol::DaemonCapabilities,
    tasks: Arc<Mutex<TaskRegistry>>,
}

impl ConnectionParts {
    /// A published task kind, by its wire variant.
    fn task_kind(&self, variant: &str) -> Option<&TaskKindDescription> {
        self.catalog
            .task_kinds
            .iter()
            .find(|task_kind| task_kind.variant == variant)
    }

    /// The catalog operation a task kind executes.
    fn operation(
        &self,
        task_kind: &TaskKindDescription,
    ) -> Option<&eggsec_daemon_protocol::operations::OperationDescription> {
        self.catalog
            .operations
            .iter()
            .find(|operation| operation.id == task_kind.operation_id)
    }

    /// Whether the connected daemon advertises this task kind's capability.
    fn supports(&self, task_kind: &TaskKindDescription) -> bool {
        self.capabilities
            .runtime
            .task_kinds
            .iter()
            .any(|capability| capability.name == task_kind.capability_name)
    }
}

impl GuiState {
    /// The live connection's parts.
    async fn parts(&self) -> Result<ConnectionParts, String> {
        let inner = self.inner.lock().await;
        let connected = inner
            .as_ref()
            .ok_or_else(|| "the GUI is not connected to a daemon yet".to_owned())?;
        Ok(ConnectionParts {
            client: connected.connection.client.clone(),
            session_id: connected.connection.session_id,
            catalog: std::sync::Arc::clone(&connected.connection.catalog),
            capabilities: connected.connection.capabilities.clone(),
            tasks: Arc::clone(&connected.tasks),
        })
    }
}

/// What the webview needs to know about the connection it just opened.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CatalogView {
    /// The socket the GUI connected to.
    pub socket_path: String,
    /// The session the GUI created.
    pub session_id: String,
    /// The daemon protocol version the daemon speaks.
    pub protocol_version: u32,
    /// How many operations the daemon published.
    pub operation_count: usize,
    /// How many task kinds the daemon published.
    pub task_kind_count: usize,
    /// Whether the connected daemon advertises no task kinds at all.
    pub no_op_executor: bool,
    /// A human-readable notice, when the connection carries a caveat the
    /// operator should see.
    pub notice: Option<String>,
}

/// One row of the operation catalog, as the webview renders it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OperationRow {
    /// The canonical operation id.
    pub id: String,
    /// The human-readable operation name.
    pub display_name: String,
    /// The assessment mode the operation belongs to.
    pub mode: String,
    /// The declared risk tier.
    pub risk: String,
    /// Whether the operation is exposed on the manual surface — which is what
    /// the GUI is.
    pub manual_exposable: bool,
}

/// One published task kind, as the webview renders it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskKindRow {
    /// The wire discriminator.
    pub variant: String,
    /// The canonical operation this task kind executes.
    pub operation_id: String,
    /// The wire capability name this task kind advertises.
    pub capability_name: String,
    /// Whether the connected daemon advertises this capability.
    pub supported: bool,
    /// Whether the operation is exposed on the manual surface.
    pub manual_exposable: bool,
    /// Why the task kind cannot be run, when it cannot.
    pub unavailable_reason: Option<String>,
}

/// A form, as the webview renders it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FormView {
    /// The task kind's wire discriminator.
    pub variant: String,
    /// The canonical operation the task kind executes.
    pub operation_id: String,
    /// The operation's human-readable name.
    pub display_name: String,
    /// The operation's declared risk tier.
    pub risk: String,
    /// The fields, in a deterministic order.
    pub controls: Vec<Control>,
    /// Whether the form may be submitted.
    pub submit_enabled: bool,
    /// Why the form cannot be submitted, when it cannot.
    pub disabled_reason: Option<String>,
}

/// The identifier of a submitted task.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskIdView {
    /// The task's identifier.
    pub task_id: String,
}

/// The result of one export.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportView {
    /// Where the export was written.
    pub path: String,
    /// The format it was written in.
    pub format: ReportFormat,
    /// The extension the format is written with.
    pub extension: &'static str,
}

/// What the webview needs to know about the daemon it is attached to.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DaemonStatusView {
    /// Which daemon the GUI is attached to.
    pub attachment: DaemonAttachment,
    /// The socket the GUI is talking to, when it is talking to one.
    pub socket_path: Option<String>,
    /// The engine features the daemon published.
    ///
    /// `None` means the daemon did not publish a set: that is **unknown**,
    /// not "no features are enabled".
    pub enabled_features: Option<Vec<String>>,
    /// How many capabilities the daemon advertises.
    pub advertised_capability_count: usize,
    /// How many task kinds the daemon published.
    pub task_kind_count: usize,
    /// The export formats the GUI can write.
    ///
    /// The list is delivered from the Rust side, so the webview never holds
    /// a catalog of its own.
    pub report_formats: Vec<crate::export::ReportFormatView>,
    /// A caveat the operator should see.
    pub notice: Option<String>,
}

/// The event the GUI publishes to the webview.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskEvent {
    /// A task's state changed.
    TaskUpsert {
        /// The task, in full.
        task: Box<TaskRow>,
    },
    /// Log lines were added to a task.
    Logs {
        /// The task the lines belong to.
        task_id: String,
        /// The lines.
        lines: Vec<LogLine>,
        /// How many lines were dropped while coalescing.
        dropped: u64,
    },
    /// The GUI's diagnostic counters moved.
    Counters {
        /// Log lines dropped across every task.
        dropped_log_lines: u64,
        /// Events about tasks the GUI does not know about.
        unknown_task_events: u64,
    },
}

/// Connect to a daemon and return the catalog it published.
///
/// The socket path comes from the webview so the operator can point the GUI
/// at a different daemon without relaunching it.
///
/// When the socket cannot be reached, the GUI reports how to start a usable
/// daemon — unless the operator named a daemon executable with
/// `--daemon-bin`, in which case the GUI starts that executable and
/// supervises it. The spawn route is opt-in: without the option the GUI
/// never starts a process.
#[tauri::command(rename_all = "snake_case")]
pub async fn connect_daemon(
    app: tauri::AppHandle,
    state: tauri::State<'_, GuiState>,
    socket: String,
) -> Result<CatalogView, String> {
    let connection = match GuiConnection::connect(&socket).await {
        Ok(connection) => connection,
        Err(error) => {
            // No named daemon executable: the operator's own diagnostic
            // stands, and nothing is started.
            let Some(binary) = state.options.daemon_bin.clone() else {
                return Err(error.to_string());
            };

            // The pre-flight liveness check. It is a connection attempt, not
            // a check that the socket file exists: a live daemon must not be
            // displaced by a spawn, and a stale socket file must not block
            // one.
            if daemon::daemon_is_live(&socket).await {
                return Err(error.to_string());
            }

            let supervised = daemon::spawn_and_supervise(&binary, &socket)
                .await
                .map_err(|error| error.to_string())?;

            let connection = match GuiConnection::connect(&socket).await {
                Ok(connection) => connection,
                Err(error) => {
                    // The daemon did not answer the GUI's own handshake, so
                    // it is stopped rather than left running unwatched.
                    supervised.terminate().await;
                    return Err(error.to_string());
                }
            };
            state.set_supervised(supervised);
            connection
        }
    };

    let view = CatalogView {
        socket_path: socket.clone(),
        session_id: connection.session_id.to_string(),
        protocol_version: connection.protocol_version,
        operation_count: connection.catalog.operations.len(),
        task_kind_count: connection.catalog.task_kinds.len(),
        no_op_executor: connection.daemon_advertises_no_task_kinds(),
        notice: if connection.daemon_advertises_no_task_kinds() {
            Some(connection.no_op_executor_diagnostic(&socket))
        } else {
            None
        },
    };

    let tasks = Arc::new(Mutex::new(TaskRegistry::new()));
    spawn_event_forwarder(
        app,
        connection.client.clone(),
        connection.session_id,
        Arc::clone(&tasks),
        Arc::clone(&state.denials),
    );

    let mut inner = state.inner.lock().await;
    *inner = Some(ConnectedState {
        socket_path: socket,
        connection,
        tasks,
    });

    Ok(view)
}

/// List the operations the connected daemon published.
///
/// The rows are the daemon's own published descriptions, rendered for
/// display. The GUI does not decide policy here: the exposure flags and the
/// risk tier come from the daemon's catalog.
#[tauri::command(rename_all = "snake_case")]
pub async fn list_operations(
    state: tauri::State<'_, GuiState>,
) -> Result<Vec<OperationRow>, String> {
    let parts = state.parts().await?;

    Ok(parts
        .catalog
        .operations
        .iter()
        .map(|operation| OperationRow {
            id: operation.id.clone(),
            display_name: operation.display_name.clone(),
            mode: operation.mode.label().to_owned(),
            risk: operation.risk.to_string(),
            manual_exposable: operation.manual_exposable,
        })
        .collect())
}

/// List the published task kinds and whether the connected daemon can run
/// them.
///
/// Availability comes from the daemon's live capabilities, never from the
/// GUI's own reasoning: the join key is the task kind's `capability_name`
/// against the capability list the daemon advertises.
#[tauri::command(rename_all = "snake_case")]
pub async fn list_task_kinds(
    state: tauri::State<'_, GuiState>,
) -> Result<Vec<TaskKindRow>, String> {
    let parts = state.parts().await?;

    Ok(parts
        .catalog
        .task_kinds
        .iter()
        .map(|task_kind| {
            let supported = parts.supports(task_kind);
            let manual_exposable = parts
                .operation(task_kind)
                .is_some_and(|operation| operation.manual_exposable);
            TaskKindRow {
                variant: task_kind.variant.clone(),
                operation_id: task_kind.operation_id.clone(),
                capability_name: task_kind.capability_name.clone(),
                supported,
                manual_exposable,
                unavailable_reason: unavailable_reason(supported, manual_exposable),
            }
        })
        .collect())
}

/// The reason a task kind cannot be submitted, if it cannot.
fn unavailable_reason(supported: bool, manual_exposable: bool) -> Option<String> {
    if !manual_exposable {
        return Some(
            "the connected daemon does not expose this operation on the manual surface".to_owned(),
        );
    }
    if !supported {
        return Some(
            "the connected daemon does not support this operation's capability".to_owned(),
        );
    }
    None
}

/// Build the form for one task kind.
///
/// The form is derived from the published request schema. There is no
/// per-operation code: a task kind the engine has not published yet gets a
/// form with no GUI change at all.
#[tauri::command(rename_all = "snake_case")]
pub async fn form_for(
    state: tauri::State<'_, GuiState>,
    variant: String,
) -> Result<FormView, String> {
    let parts = state.parts().await?;

    let task_kind = parts
        .task_kind(&variant)
        .ok_or_else(|| format!("`{variant}` is not a task kind the daemon published"))?;
    let operation = parts.operation(task_kind);

    let supported = parts.supports(task_kind);
    let manual_exposable = operation.is_some_and(|operation| operation.manual_exposable);
    let disabled_reason = unavailable_reason(supported, manual_exposable);

    Ok(FormView {
        variant: task_kind.variant.clone(),
        operation_id: task_kind.operation_id.clone(),
        display_name: operation
            .map(|operation| operation.display_name.clone())
            .unwrap_or_else(|| task_kind.variant.clone()),
        risk: operation
            .map(|operation| operation.risk.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
        controls: form::controls_for(task_kind),
        submit_enabled: disabled_reason.is_none(),
        disabled_reason,
    })
}

/// Submit a form.
///
/// The variant and the values come from the webview; the surface does not.
/// The surface is stamped in Rust, so the webview cannot widen the session's
/// authority by asking nicely.
#[tauri::command(rename_all = "snake_case")]
pub async fn submit_task(
    state: tauri::State<'_, GuiState>,
    variant: String,
    values: serde_json::Value,
) -> Result<TaskIdView, String> {
    let parts = state.parts().await?;

    let task_kind = parts
        .task_kind(&variant)
        .ok_or_else(|| format!("`{variant}` is not a task kind the daemon published"))?;

    // The catalog's exposure flags are the authority on what a manual
    // surface may run.
    let operation = parts.operation(task_kind).ok_or_else(|| {
        format!(
            "the daemon's catalog has no operation for the task kind `{}`",
            task_kind.variant
        )
    })?;
    if !operation.manual_exposable {
        return Err(format!(
            "`{}` is not exposed on the manual surface, so this GUI cannot run it",
            operation.display_name
        ));
    }

    // Availability comes from the daemon's live capabilities.
    if !parts.supports(task_kind) {
        return Err(format!(
            "the connected daemon does not support `{}`",
            operation.display_name
        ));
    }

    let request = form::assemble_request(task_kind, &values).map_err(|error| error.to_string())?;

    let task_id = parts
        .client
        .submit(parts.session_id, request)
        .await
        .map_err(|error| error.to_string())?;

    // Register the task straight away so the dashboard shows it before the
    // daemon's first event arrives.
    parts
        .tasks
        .lock()
        .await
        .ensure_task(task_id, task_kind.variant.clone());

    Ok(TaskIdView {
        task_id: task_id.to_string(),
    })
}

/// Cancel one task.
///
/// Cancellation is best-effort: a task that completes between the operator's
/// click and the daemon's processing is not an error. The event stream is
/// authoritative about what actually happened.
#[tauri::command(rename_all = "snake_case")]
pub async fn cancel_task(state: tauri::State<'_, GuiState>, task_id: String) -> Result<(), String> {
    let parts = state.parts().await?;

    let parsed = task_id
        .parse::<TaskId>()
        .map_err(|error| format!("`{task_id}` is not a task identifier: {error}"))?;

    parts
        .client
        .cancel(parts.session_id, parsed)
        .await
        .map_err(|error| error.to_string())
}

/// Cancel the active task in the session.
#[tauri::command(rename_all = "snake_case")]
pub async fn cancel_active(state: tauri::State<'_, GuiState>) -> Result<(), String> {
    let parts = state.parts().await?;

    parts
        .client
        .cancel_active(parts.session_id)
        .await
        .map_err(|error| error.to_string())
}

/// Answer a policy decision the daemon is holding for a task.
///
/// The answer names *yes* or *no*. It never names the authorisation: what
/// the answer permits is computed by the policy kernel on the daemon's
/// executor side, which re-runs its own authorisation path with an override
/// bounded by the classes the kernel demanded.
///
/// A task with nothing pending is an error the operator sees, not a silent
/// no-op: the daemon refuses the answer because there is no decision to
/// resolve.
#[tauri::command(rename_all = "snake_case")]
pub async fn approve_task(
    state: tauri::State<'_, GuiState>,
    task_id: String,
    approved: bool,
    reason: Option<String>,
) -> Result<(), String> {
    let parts = state.parts().await?;

    let parsed = task_id
        .parse::<TaskId>()
        .map_err(|error| format!("`{task_id}` is not a task identifier: {error}"))?;

    parts
        .client
        .approve_task(parts.session_id, parsed, approved, reason)
        .await
        .map_err(|error| error.to_string())
}

/// The current task dashboard.
#[tauri::command(rename_all = "snake_case")]
pub async fn task_snapshot(state: tauri::State<'_, GuiState>) -> Result<TaskDashboardView, String> {
    let parts = state.parts().await?;
    let tasks = Arc::clone(&parts.tasks);
    let registry = tasks.lock().await;
    let view = registry.to_view();
    Ok(view)
}

/// The coverage table: every published task kind, and why it is or is not
/// reachable from this GUI.
///
/// The table is derived from what the daemon published — the catalog, the
/// advertised capabilities, and the published enabled-feature set — plus the
/// denials the GUI has learned from live failures. Nothing here is
/// transcribed: a task kind the engine publishes tomorrow appears in this
/// table without a line being added.
#[tauri::command(rename_all = "snake_case")]
pub async fn coverage(state: tauri::State<'_, GuiState>) -> Result<Vec<TaskCoverage>, String> {
    let parts = state.parts().await?;
    let ledger = state.denials.lock().await;

    Ok(coverage::coverage_for_with_ledger(
        &parts.catalog.task_kinds,
        &parts.catalog.operations,
        &parts.capabilities.runtime.task_kinds,
        parts.capabilities.enabled_features.as_deref(),
        &ledger,
    ))
}

/// Export a completed task's report to a file the operator chose.
///
/// The report is fetched from the daemon over the session, rendered through
/// the shared report renderers, and written to the operator's chosen path.
/// The destination is the operator's decision alone: the GUI invents no
/// default directory and writes nowhere else.
#[tauri::command(rename_all = "snake_case")]
pub async fn export_report(
    state: tauri::State<'_, GuiState>,
    task_id: String,
    format: String,
    path: String,
) -> Result<ExportView, String> {
    let parts = state.parts().await?;

    let parsed = task_id
        .parse::<TaskId>()
        .map_err(|error| format!("`{task_id}` is not a task identifier: {error}"))?;
    let format = ReportFormat::parse(&format)
        .ok_or_else(|| format!("`{format}` is not a report format the GUI can write"))?;

    let document = parts
        .client
        .get_task_report(&parts.session_id, &parsed)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!("the daemon has no report for task `{task_id}`, so there is nothing to export")
        })?;

    let report: ScanReportData = serde_json::from_value(document).map_err(|error| {
        format!("the daemon's report for task `{task_id}` could not be read: {error}")
    })?;

    let contents = export::render(format, &report).map_err(|error| error.to_string())?;
    let destination = PathBuf::from(&path);
    export::write_export(&destination, &contents).map_err(|error| error.to_string())?;

    Ok(ExportView {
        path: destination.to_string_lossy().into_owned(),
        format,
        extension: format.extension(),
    })
}

/// The report of a completed task, as the webview renders it.
///
/// The report travels to the webview as **data**: the GUI never injects the
/// engine's HTML document into its window. That document carries an inline
/// `<style>` block and an inline `<script>` block, and the GUI's Content
/// Security Policy blocks both — rendering it faithfully would mean
/// relaxing the policy, and the report's content is attacker-influenced.
/// The frontend renders the structured content with the GUI's own stylesheet
/// and its own escaping.
#[tauri::command(rename_all = "snake_case")]
pub async fn report_for(
    state: tauri::State<'_, GuiState>,
    task_id: String,
    format: String,
) -> Result<ReportResponse, String> {
    let parts = state.parts().await?;

    let parsed = task_id
        .parse::<TaskId>()
        .map_err(|error| format!("`{task_id}` is not a task identifier: {error}"))?;
    let format = ReportFormat::parse(&format)
        .ok_or_else(|| format!("`{format}` is not a report format the GUI can write"))?;

    let document = parts
        .client
        .get_task_report(&parts.session_id, &parsed)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!("the daemon has no report for task `{task_id}`, so there is nothing to show")
        })?;

    let report: ScanReportData = serde_json::from_value(document).map_err(|error| {
        format!("the daemon's report for task `{task_id}` could not be read: {error}")
    })?;

    // The text formats have an inert rendering the frontend can show as
    // text. The HTML format does not: the GUI shows the structured content.
    let rendered = match export::render_for_display(format, &report) {
        Ok(text) => Some(text),
        Err(error) => {
            tracing::debug!(%error, "the report has no display rendering for this format");
            None
        }
    };

    Ok(ReportResponse {
        report: export::report_view(&report),
        document: rendered,
        format,
    })
}

/// The report of a completed task, as the webview renders it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReportResponse {
    /// The report's structured content.
    pub report: crate::export::ReportView,
    /// The rendered text artifact for the requested format, when the format
    /// has one.
    ///
    /// The HTML format has none: the GUI renders the report's structured
    /// content instead of injecting the engine's document.
    pub document: Option<String>,
    /// The format the document is rendered in.
    pub format: ReportFormat,
}

/// What the GUI knows about the daemon it is attached to.
///
/// The attachment state says whether the GUI is attached to a daemon the
/// operator started or supervising one it started itself. The published
/// enabled-feature set is reported as it was published: an absent set is
/// *unknown*, not an empty one.
#[tauri::command(rename_all = "snake_case")]
pub async fn daemon_status(state: tauri::State<'_, GuiState>) -> Result<DaemonStatusView, String> {
    let supervised = state.supervised();
    let inner = state.inner.lock().await;
    let connected = inner.as_ref();

    let attachment = match (&supervised, connected) {
        (Some(_), _) => DaemonAttachment::Supervised,
        (None, Some(_)) => DaemonAttachment::Operator,
        (None, None) => DaemonAttachment::Disconnected,
    };

    let capabilities = connected.map(|connected| &connected.connection.capabilities);
    let enabled_features =
        capabilities.and_then(|capabilities| capabilities.enabled_features.clone());
    let advertised_capability_count = capabilities
        .map(|capabilities| capabilities.runtime.task_kinds.len())
        .unwrap_or(0);
    let task_kind_count = connected
        .map(|connected| connected.connection.catalog.task_kinds.len())
        .unwrap_or(0);
    let socket_path = connected
        .map(|connected| connected.socket_path.clone())
        .or_else(|| {
            supervised
                .as_ref()
                .map(|supervised| supervised.socket_path().to_owned())
        });

    // A daemon that advertises capabilities and publishes no enabled
    // features can execute no feature-gated operation: the flags say it can,
    // and the engine says otherwise. The GUI reports that rather than
    // offering a submit affordance it cannot back.
    let notice = match (&enabled_features, advertised_capability_count) {
        (Some(features), count) if count > 0 && features.is_empty() => Some(format!(
            "the daemon advertises {count} capabilities but publishes no enabled engine features: \
             it can execute no feature-gated operation"
        )),
        (None, count) if count > 0 => Some(
            "the daemon publishes no enabled-feature set, so its feature-gated operations cannot \
             be verified"
                .to_owned(),
        ),
        _ => None,
    };

    Ok(DaemonStatusView {
        attachment,
        socket_path,
        enabled_features,
        advertised_capability_count,
        task_kind_count,
        report_formats: crate::export::format_views(),
        notice,
    })
}

/// Forward the daemon's event stream to the webview.
///
/// The GUI subscribes **once** per session; it does not resubscribe per
/// task. The forwarder drains the receiver and updates the bounded task
/// state, then emits to the webview. It never spawns a task per event and
/// never accumulates an unbounded queue: log lines are coalesced, and only
/// the newest [`MAX_LOG_LINES_PER_FLUSH`] are forwarded per flush, with the
/// rest reported as dropped.
///
/// A failure that names a missing feature is also **learned**: the engine's
/// own refusal is a fact about the running daemon that no published list can
/// overrule, so it is recorded against the task kind's variant and the
/// coverage table reports the variant as blocked.
///
/// **This task is exempt from the 30–300s timeout rule.** It is a
/// long-lived connection reader whose lifetime is the subscription's, and
/// killing it would silently sever the event stream. The exemption is the
/// same one the daemon client's reader task holds, and for the same reason.
fn spawn_event_forwarder(
    app: tauri::AppHandle,
    client: eggsec_frontend_client::DaemonRuntimeClient,
    session_id: eggsec_runtime::SessionId,
    tasks: Arc<Mutex<TaskRegistry>>,
    denials: Arc<Mutex<DenialLedger>>,
) {
    tokio::spawn(async move {
        let mut receiver = match client.subscribe(session_id).await {
            Ok(receiver) => receiver,
            Err(error) => {
                tracing::warn!("the GUI could not subscribe to the daemon's events: {error}");
                return;
            }
        };

        let mut pending: Vec<(String, LogLine)> = Vec::new();
        let mut flush = tokio::time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                event = receiver.recv() => {
                    match event {
                        Some(event) => {
                            let effect = {
                                let mut registry = tasks.lock().await;
                                registry.apply(&event)
                            };
                            learn_from_denial(&event, &tasks, &denials).await;
                            match effect {
                                EventEffect::TaskUpdated(task_id) => {
                                    let row = {
                                        let registry = tasks.lock().await;
                                        let row = registry
                                            .tasks()
                                            .find(|task| task.task_id == task_id)
                                            .map(TaskState::to_row);
                                        row
                                    };
                                    if let Some(row) = row {
                                        emit(&app, TaskEvent::TaskUpsert { task: Box::new(row) });
                                    }
                                }
                                EventEffect::Logs { task_id, lines } => {
                                    for line in lines {
                                        pending.push((task_id.to_string(), line));
                                    }
                                }
                                EventEffect::Ignored => {}
                            }
                        }
                        None => {
                            // The daemon's connection ended. The GUI reports
                            // the loss; it does not decide what happened to
                            // the running tasks.
                            tracing::warn!("the GUI's daemon event stream ended");
                            let (dropped_log_lines, unknown_task_events) = {
                                let registry = tasks.lock().await;
                                (registry.dropped_log_lines(), registry.unknown_task_events())
                            };
                            emit(&app, TaskEvent::Counters { dropped_log_lines, unknown_task_events });
                            break;
                        }
                    }
                }
                _ = flush.tick() => {
                    if pending.is_empty() {
                        continue;
                    }
                    // Keep the newest lines; report the rest as dropped.
                    let mut dropped = 0u64;
                    if pending.len() > MAX_LOG_LINES_PER_FLUSH {
                        dropped = (pending.len() - MAX_LOG_LINES_PER_FLUSH) as u64;
                        let start = pending.len() - MAX_LOG_LINES_PER_FLUSH;
                        pending.drain(..start);
                    }
                    // Group by task so each flush carries one entry per task.
                    let mut grouped: Vec<(String, Vec<LogLine>)> = Vec::new();
                    for (task_id, line) in pending.drain(..) {
                        match grouped.iter_mut().find(|(id, _)| *id == task_id) {
                            Some((_, lines)) => lines.push(line),
                            None => grouped.push((task_id, vec![line])),
                        }
                    }
                    for (task_id, lines) in grouped {
                        emit(&app, TaskEvent::Logs { task_id, lines, dropped });
                    }
                }
            }
        }
    });
}

/// Emit an event to the webview.
fn emit(app: &tauri::AppHandle, event: TaskEvent) {
    if let Err(error) = app.emit(TASK_EVENT, &event) {
        tracing::warn!("the GUI could not publish a task event: {error}");
    }
}

/// Record what a live denial taught the GUI.
///
/// A failure whose message names a missing feature is the engine's own
/// statement about the running daemon. It is recorded against the task
/// kind's variant, and the coverage table then reports that variant as
/// blocked — whatever the published lists say.
async fn learn_from_denial(
    event: &eggsec_runtime::RuntimeEvent,
    tasks: &Arc<Mutex<TaskRegistry>>,
    denials: &Arc<Mutex<DenialLedger>>,
) {
    let eggsec_runtime::RuntimeEvent::TaskFailed { task_id, error, .. } = event else {
        return;
    };
    let denial = denial_from_message(&error.message);
    // A feature denial and a risk-policy denial are both facts about the
    // operation's availability, so both are learned. An opaque failure is
    // not: it says nothing about whether the operation could run, so it is
    // left in the task's own error log instead of the coverage table.
    if matches!(denial, Denial::Other { .. }) {
        return;
    }

    let variant = {
        let registry = tasks.lock().await;
        let variant = registry
            .tasks()
            .find(|task| task.task_id == *task_id)
            .map(|task| task.variant.clone());
        variant
    };
    let Some(variant) = variant else {
        return;
    };

    let mut ledger = denials.lock().await;
    ledger.record(&variant, denial);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The IPC surface must stay narrow. Each command maps to one narrow
    /// operation; there is no generic "send any command to the daemon" entry
    /// point, and nothing that sets the surface or the scope.
    #[test]
    fn ipc_surface_exposes_only_narrow_commands() {
        let source = include_str!("commands.rs");
        // Scan the production half only: this test names the forbidden
        // tokens itself.
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("the module has a production half");
        for forbidden in [
            "ClientCommand::",
            "RuntimeSurface::",
            "SessionScope",
            "send_command",
        ] {
            assert!(
                !production.contains(forbidden),
                "the IPC surface must not contain {forbidden}"
            );
        }
    }

    /// The command list is exactly the declared set.
    #[test]
    fn declared_command_set_is_exact() {
        let registered = [
            "connect_daemon",
            "list_operations",
            "list_task_kinds",
            "form_for",
            "submit_task",
            "cancel_task",
            "cancel_active",
            "approve_task",
            "task_snapshot",
            "coverage",
            "export_report",
            "report_for",
            "daemon_status",
        ];
        let source = include_str!("shell.rs");
        for command in registered {
            assert!(
                source.contains(command),
                "the shell must register the {command} command"
            );
        }
    }

    /// Every command declares its argument naming explicitly.
    ///
    /// Tauri's default is camelCase from the webview. The GUI's contract is
    /// snake_case on both sides of the boundary, so the attribute must not be
    /// dropped — without it the webview's `task_id` would silently stop
    /// arriving.
    #[test]
    fn every_command_declares_its_argument_naming() {
        let source = include_str!("commands.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("the module has a production half");
        let commands = production.matches("#[tauri::command").count();
        let explicit = production
            .matches("#[tauri::command(rename_all = \"snake_case\")]")
            .count();
        assert_eq!(commands, 13, "the GUI's IPC surface is thirteen commands");
        assert_eq!(
            commands, explicit,
            "every command must declare rename_all = \"snake_case\""
        );
    }

    /// The availability rules are stated once and applied consistently.
    #[test]
    fn unavailability_is_explained() {
        assert!(unavailable_reason(true, true).is_none());
        assert!(unavailable_reason(true, false)
            .expect("a reason")
            .contains("manual surface"));
        assert!(unavailable_reason(false, true)
            .expect("a reason")
            .contains("capability"));
        assert!(unavailable_reason(false, false)
            .expect("a reason")
            .contains("manual surface"));
    }
}
