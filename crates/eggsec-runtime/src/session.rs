use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Current time as seconds since Unix epoch.
fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::capabilities::RuntimeCapabilities;
use crate::event::{TaskProgress, TaskStatus};
use crate::ids::{SessionId, TaskId};
use crate::request::{RunRequest, RuntimeSurface, TaskKind};

/// Lightweight scope metadata bound to a session.
///
/// Mirrors the essential provenance data from `LoadedScope` without
/// depending on the `eggsec` crate. The `eggsec` crate provides
/// `From<&LoadedScope>` to convert from the full scope type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionScope {
    /// Whether scope was explicitly provided (config file, CLI, or preset).
    pub is_explicit: bool,
    /// Human-readable source label (e.g. "default-empty", "config", "cli", "preset").
    pub source: String,
    /// Optional path to the scope file.
    pub path: Option<String>,
}

/// Internal task record owned by a runtime session.
///
/// Holds the full lifecycle state of a single task: request, status,
/// progress, cancellation token, and join handle.
pub(crate) struct TaskRecord {
    pub request: RunRequest,
    pub status: TaskStatus,
    pub progress: Option<TaskProgress>,
    pub last_error: Option<String>,
    pub abort: Option<CancellationToken>,
    pub _handle: Option<JoinHandle<()>>,
}

/// Canonical runtime session state.
///
/// Owns all session-level state: task records, execution surface,
/// creation timestamp, and session capabilities. This is the single
/// source of truth for session data — frontends query this rather
/// than maintaining duplicate task lifecycle state.
///
/// `RuntimeSession` is constructible and queryable without any TUI
/// dependency, enabling future daemon, CLI, and MCP surfaces to
/// create and inspect sessions directly.
pub struct RuntimeSession {
    /// Session identifier.
    pub id: SessionId,
    /// Execution surface bound at session creation (e.g. TuiManual, McpServer).
    surface: RuntimeSurface,
    /// Scope metadata bound at session creation (provenance + explicit flag).
    scope: Option<SessionScope>,
    /// When the session was created.
    created_at: Instant,
    /// When the session was created (seconds since Unix epoch).
    created_at_utc: u64,
    /// Task records (active and completed).
    pub(crate) tasks: HashMap<TaskId, TaskRecord>,
    /// Completed task snapshots restored from a previous session snapshot.
    /// These are read-only records for history/audit; they hold no runtime handles.
    hydrated_completed: Vec<TaskSnapshot>,
}

impl RuntimeSession {
    /// Create a new session bound to the given execution surface.
    pub fn new(id: SessionId, surface: RuntimeSurface) -> Self {
        Self {
            id,
            surface,
            scope: None,
            created_at: Instant::now(),
            created_at_utc: now_epoch_secs(),
            tasks: HashMap::new(),
            hydrated_completed: Vec::new(),
        }
    }

    /// Create a new session with an explicit scope binding.
    pub fn with_scope(id: SessionId, surface: RuntimeSurface, scope: SessionScope) -> Self {
        Self {
            id,
            surface,
            scope: Some(scope),
            created_at: Instant::now(),
            created_at_utc: now_epoch_secs(),
            tasks: HashMap::new(),
            hydrated_completed: Vec::new(),
        }
    }

    /// Execution surface this session was bound to at creation.
    pub fn execution_surface(&self) -> RuntimeSurface {
        self.surface.clone()
    }

    /// Scope metadata bound at session creation, if any.
    pub fn scope(&self) -> Option<&SessionScope> {
        self.scope.as_ref()
    }

    /// When the session was created (monotonic clock).
    pub fn created_at(&self) -> Instant {
        self.created_at
    }

    /// When the session was created (seconds since Unix epoch).
    pub fn created_at_secs(&self) -> u64 {
        self.created_at_utc
    }

    /// Session-level capabilities. Currently always returns the default
    /// capability set; future versions may derive this from the surface.
    pub fn capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities::default()
    }

    /// Snapshot of all active (non-terminal) tasks.
    pub fn active_tasks(&self) -> Vec<TaskSnapshot> {
        self.tasks
            .iter()
            .filter(|(_, t)| !t.status.is_terminal())
            .map(|(id, t)| TaskSnapshot {
                task_id: *id,
                status: t.status.clone(),
                request_summary: summarize_request(&t.request.task_kind),
                progress: t.progress.clone(),
                last_error: t.last_error.clone(),
            })
            .collect()
    }

    /// Snapshot of all completed (terminal) tasks.
    ///
    /// Includes both live completed tasks and hydrated snapshots from a
    /// previous session (for daemon attach history).
    pub fn completed_tasks(&self) -> Vec<TaskSnapshot> {
        let mut results: Vec<TaskSnapshot> = self
            .tasks
            .iter()
            .filter(|(_, t)| t.status.is_terminal())
            .map(|(id, t)| TaskSnapshot {
                task_id: *id,
                status: t.status.clone(),
                request_summary: summarize_request(&t.request.task_kind),
                progress: t.progress.clone(),
                last_error: t.last_error.clone(),
            })
            .collect();
        results.extend(self.hydrated_completed.iter().cloned());
        results
    }

    /// Full session snapshot for state reporting and serialization.
    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            session_id: self.id,
            surface: self.surface.clone(),
            scope: self.scope.clone(),
            created_at_secs: self.created_at.elapsed().as_secs(),
            active_tasks: self.active_tasks(),
            completed_tasks: self.completed_tasks(),
            capabilities: self.capabilities(),
        }
    }

    /// Hydrate session state from a snapshot (for future daemon attach).
    ///
    /// Reconstructs the metadata and completed task records from a snapshot.
    /// Active task records are not restorable (they hold runtime handles),
    /// but completed task snapshots are preserved for history and audit querying.
    pub fn hydrate_from_snapshot(snapshot: SessionSnapshot) -> Self {
        Self {
            id: snapshot.session_id,
            surface: snapshot.surface,
            scope: snapshot.scope,
            created_at: Instant::now(),
            created_at_utc: now_epoch_secs(),
            tasks: HashMap::new(),
            hydrated_completed: snapshot.completed_tasks,
        }
    }
}

/// Snapshot of a single task for session state reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub task_id: TaskId,
    pub status: TaskStatus,
    pub request_summary: String,
    pub progress: Option<TaskProgress>,
    pub last_error: Option<String>,
}

/// Session snapshot containing all runtime state for a session.
///
/// This is the serializable representation of a session's state.
/// It includes the execution surface and creation time, enabling
/// frontends to display session metadata without querying the
/// runtime's internal state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: SessionId,
    /// Execution surface bound at session creation.
    pub surface: RuntimeSurface,
    /// Scope metadata bound at session creation.
    pub scope: Option<SessionScope>,
    /// Seconds since session creation (monotonic approximation).
    pub created_at_secs: u64,
    pub active_tasks: Vec<TaskSnapshot>,
    pub completed_tasks: Vec<TaskSnapshot>,
    pub capabilities: RuntimeCapabilities,
}

/// Lightweight summary of a session for listing purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: SessionId,
    pub surface: RuntimeSurface,
    pub scope: Option<SessionScope>,
    pub active_count: usize,
    pub completed_count: usize,
    pub created_at_secs: u64,
}

/// Summary of a task request for snapshot display.
pub fn summarize_request(kind: &TaskKind) -> String {
    match kind {
        TaskKind::LoadTest(p) => format!("load-test: {}", p.target),
        TaskKind::StressTest(p) => format!("stress-test: {}", p.target),
        TaskKind::PortScan(p) => format!("port-scan: {}", p.target),
        TaskKind::EndpointScan(p) => format!("endpoint-scan: {}", p.target),
        TaskKind::Fingerprint(p) => format!("fingerprint: {}", p.target),
        TaskKind::Fuzz(p) => format!("fuzz: {}", p.target),
        TaskKind::Waf(p) => format!("waf: {}", p.target),
        TaskKind::WafStress(p) => format!("waf-stress: {}", p.target),
        TaskKind::Pipeline(p) => format!("pipeline: {}", p.target),
        TaskKind::Recon(p) => format!("recon: {}", p.target),
        TaskKind::PacketCapture(_) => "packet-capture".into(),
        TaskKind::PacketTraceroute(p) => format!("traceroute: {}", p.target),
        TaskKind::PacketSend(p) => format!("packet-send: {}", p.target),
        TaskKind::GraphQl(p) => format!("graphql: {}", p.target),
        TaskKind::OAuth(p) => format!("oauth: {}", p.target),
        TaskKind::AuthTest(p) => format!("auth-test: {}", p.target),
        TaskKind::Nse(p) => format!("nse: {} [{}]", p.target, p.script),
        TaskKind::Hunt(p) => format!("hunt: {}", p.target),
        TaskKind::Browser(p) => format!("browser: {}", p.target),
        TaskKind::Compliance(p) => format!("compliance: {}", p.target),
        TaskKind::Storage(p) => format!("storage: {}", p.storage_type),
        TaskKind::Integrations(p) => format!("integration: {}", p.integration_type),
        TaskKind::Workflow(_) => "workflow".into(),
        TaskKind::Vuln(p) => format!("vuln: {}", p.target),
        TaskKind::Wireless(_) => "wireless-recon".into(),
        TaskKind::WirelessActive(_) => "wireless-active".into(),
        TaskKind::DbPentest(p) => format!("db-pentest: {} {}", p.db_type, p.target),
        TaskKind::Intercept(p) => {
            if let Some(port) = p.listen_port {
                format!("intercept: :{}", port)
            } else {
                "intercept".into()
            }
        }
        TaskKind::C2(_) => "c2".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::PortScanParams;

    #[test]
    fn session_snapshot_roundtrip() {
        let snapshot = SessionSnapshot {
            session_id: SessionId::new(),
            surface: RuntimeSurface::TuiManual,
            scope: None,
            created_at_secs: 42,
            active_tasks: vec![],
            completed_tasks: vec![],
            capabilities: RuntimeCapabilities::default(),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: SessionSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot.session_id, deserialized.session_id);
        assert_eq!(deserialized.created_at_secs, 42);
        assert!(deserialized.scope.is_none());
    }

    #[test]
    fn session_snapshot_roundtrip_with_scope() {
        let snapshot = SessionSnapshot {
            session_id: SessionId::new(),
            surface: RuntimeSurface::TuiManual,
            scope: Some(SessionScope {
                is_explicit: true,
                source: "config".into(),
                path: Some("/etc/eggsec/scope.yaml".into()),
            }),
            created_at_secs: 10,
            active_tasks: vec![],
            completed_tasks: vec![],
            capabilities: RuntimeCapabilities::default(),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: SessionSnapshot = serde_json::from_str(&json).unwrap();
        let scope = deserialized.scope.unwrap();
        assert!(scope.is_explicit);
        assert_eq!(scope.source, "config");
        assert_eq!(scope.path.as_deref(), Some("/etc/eggsec/scope.yaml"));
    }

    #[test]
    fn summarize_port_scan() {
        let kind = TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ports: None,
            scan_type: None,
            timeout_ms: None,
        });
        assert_eq!(summarize_request(&kind), "port-scan: 10.0.0.1");
    }

    #[test]
    fn runtime_session_new() {
        let id = SessionId::new();
        let session = RuntimeSession::new(id, RuntimeSurface::TuiManual);
        assert_eq!(session.id, id);
        assert_eq!(session.execution_surface(), RuntimeSurface::TuiManual);
        assert!(session.scope().is_none());
        assert!(session.active_tasks().is_empty());
        assert!(session.completed_tasks().is_empty());
    }

    #[test]
    fn runtime_session_with_scope() {
        let scope = SessionScope {
            is_explicit: true,
            source: "cli".into(),
            path: None,
        };
        let session =
            RuntimeSession::with_scope(SessionId::new(), RuntimeSurface::CliManual, scope);
        let s = session.scope().unwrap();
        assert!(s.is_explicit);
        assert_eq!(s.source, "cli");
    }

    #[test]
    fn runtime_session_capabilities() {
        let session = RuntimeSession::new(SessionId::new(), RuntimeSurface::CliManual);
        let caps = session.capabilities();
        assert!(!caps.task_kinds.is_empty());
    }

    #[test]
    fn runtime_session_hydrate_from_snapshot_preserves_completed() {
        let snapshot = SessionSnapshot {
            session_id: SessionId::new(),
            surface: RuntimeSurface::McpServer,
            scope: Some(SessionScope {
                is_explicit: true,
                source: "config".into(),
                path: None,
            }),
            created_at_secs: 10,
            active_tasks: vec![],
            completed_tasks: vec![TaskSnapshot {
                task_id: TaskId::new(),
                status: TaskStatus::Completed,
                request_summary: "port-scan: 10.0.0.1".into(),
                progress: None,
                last_error: None,
            }],
            capabilities: RuntimeCapabilities::default(),
        };
        let session = RuntimeSession::hydrate_from_snapshot(snapshot.clone());
        assert_eq!(session.id, snapshot.session_id);
        assert_eq!(session.execution_surface(), RuntimeSurface::McpServer);
        assert!(session.scope().is_some());
        assert_eq!(session.scope().unwrap().source, "config");
        assert!(session.active_tasks().is_empty());
        // Completed tasks from snapshot are restored for read-only querying.
        let completed = session.completed_tasks();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].request_summary, "port-scan: 10.0.0.1");
    }

    #[test]
    fn runtime_session_snapshot_includes_scope() {
        let scope = SessionScope {
            is_explicit: false,
            source: "default-empty".into(),
            path: None,
        };
        let session = RuntimeSession::with_scope(SessionId::new(), RuntimeSurface::RestApi, scope);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.surface, RuntimeSurface::RestApi);
        assert_eq!(snapshot.session_id, session.id);
        let s = snapshot.scope.unwrap();
        assert!(!s.is_explicit);
        assert_eq!(s.source, "default-empty");
    }

    #[test]
    fn runtime_session_snapshot_includes_surface() {
        let session = RuntimeSession::new(SessionId::new(), RuntimeSurface::RestApi);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.surface, RuntimeSurface::RestApi);
        assert_eq!(snapshot.session_id, session.id);
        assert!(snapshot.scope.is_none());
    }
}
