use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::capabilities::RuntimeCapabilities;
use crate::event::{TaskProgress, TaskStatus};
use crate::ids::{SessionId, TaskId};
use crate::request::{RunRequest, RuntimeSurface, TaskKind};

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
    /// When the session was created.
    created_at: Instant,
    /// Task records (active and completed).
    pub(crate) tasks: HashMap<TaskId, TaskRecord>,
}

impl RuntimeSession {
    /// Create a new session bound to the given execution surface.
    pub fn new(id: SessionId, surface: RuntimeSurface) -> Self {
        Self {
            id,
            surface,
            created_at: Instant::now(),
            tasks: HashMap::new(),
        }
    }

    /// Execution surface this session was bound to at creation.
    pub fn execution_surface(&self) -> RuntimeSurface {
        self.surface.clone()
    }

    /// When the session was created (monotonic clock).
    pub fn created_at(&self) -> Instant {
        self.created_at
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
    pub fn completed_tasks(&self) -> Vec<TaskSnapshot> {
        self.tasks
            .iter()
            .filter(|(_, t)| t.status.is_terminal())
            .map(|(id, t)| TaskSnapshot {
                task_id: *id,
                status: t.status.clone(),
                request_summary: summarize_request(&t.request.task_kind),
                progress: t.progress.clone(),
                last_error: t.last_error.clone(),
            })
            .collect()
    }

    /// Full session snapshot for state reporting and serialization.
    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            session_id: self.id,
            surface: self.surface.clone(),
            created_at_secs: self.created_at.elapsed().as_secs(),
            active_tasks: self.active_tasks(),
            completed_tasks: self.completed_tasks(),
            capabilities: self.capabilities(),
        }
    }

    /// Hydrate session state from a snapshot (for future daemon attach).
    ///
    /// This reconstructs the metadata portion of a session from a snapshot.
    /// Task records themselves are not restorable (they hold runtime handles),
    /// but the snapshot preserves their final state for querying.
    pub fn hydrate_from_snapshot(snapshot: SessionSnapshot) -> Self {
        Self {
            id: snapshot.session_id,
            surface: snapshot.surface,
            created_at: Instant::now(),
            tasks: HashMap::new(),
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
    /// Seconds since session creation (monotonic approximation).
    pub created_at_secs: u64,
    pub active_tasks: Vec<TaskSnapshot>,
    pub completed_tasks: Vec<TaskSnapshot>,
    pub capabilities: RuntimeCapabilities,
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
            created_at_secs: 42,
            active_tasks: vec![],
            completed_tasks: vec![],
            capabilities: RuntimeCapabilities::default(),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: SessionSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot.session_id, deserialized.session_id);
        assert_eq!(deserialized.created_at_secs, 42);
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
        assert!(session.active_tasks().is_empty());
        assert!(session.completed_tasks().is_empty());
    }

    #[test]
    fn runtime_session_capabilities() {
        let session = RuntimeSession::new(SessionId::new(), RuntimeSurface::CliManual);
        let caps = session.capabilities();
        assert!(!caps.task_kinds.is_empty());
    }

    #[test]
    fn runtime_session_hydrate_from_snapshot() {
        let snapshot = SessionSnapshot {
            session_id: SessionId::new(),
            surface: RuntimeSurface::McpServer,
            created_at_secs: 10,
            active_tasks: vec![],
            completed_tasks: vec![],
            capabilities: RuntimeCapabilities::default(),
        };
        let session = RuntimeSession::hydrate_from_snapshot(snapshot.clone());
        assert_eq!(session.id, snapshot.session_id);
        assert_eq!(session.execution_surface(), RuntimeSurface::McpServer);
        assert!(session.active_tasks().is_empty());
    }

    #[test]
    fn runtime_session_snapshot_includes_surface() {
        let session = RuntimeSession::new(SessionId::new(), RuntimeSurface::RestApi);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.surface, RuntimeSurface::RestApi);
        assert_eq!(snapshot.session_id, session.id);
    }
}
