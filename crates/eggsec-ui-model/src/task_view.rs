use serde::{Deserialize, Serialize};

use eggsec_runtime::event::{TaskProgress, TaskStatus};
use eggsec_runtime::ids::TaskId;
use eggsec_runtime::request::TaskKind;
use eggsec_runtime::session::TaskSnapshot;

/// Frontend-neutral task view for list/detail displays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskView {
    pub task_id: TaskId,
    pub status: TaskStatus,
    pub status_label: String,
    pub task_kind: String,
    pub task_kind_label: String,
    pub request_summary: String,
    pub progress: Option<TaskProgressView>,
    pub last_error: Option<String>,
    pub has_outcome: bool,
    pub outcome_kind: Option<String>,
}

impl From<&TaskSnapshot> for TaskView {
    fn from(t: &TaskSnapshot) -> Self {
        Self {
            task_id: t.task_id,
            status: t.status.clone(),
            status_label: status_label(&t.status).into(),
            task_kind: format!("{:?}", t.task_kind),
            task_kind_label: task_kind_label(&t.task_kind).into(),
            request_summary: t.request_summary.clone(),
            progress: t.progress.as_ref().map(TaskProgressView::from),
            last_error: t.last_error.clone(),
            has_outcome: t.outcome.is_some(),
            outcome_kind: t.outcome.as_ref().map(outcome_kind_str),
        }
    }
}

/// Frontend-neutral progress view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressView {
    pub completed: u64,
    pub total: Option<u64>,
    pub percentage: Option<f64>,
    pub message: Option<String>,
}

impl From<&TaskProgress> for TaskProgressView {
    fn from(p: &TaskProgress) -> Self {
        Self {
            completed: p.completed,
            total: p.total,
            percentage: p
                .total
                .filter(|&t| t > 0)
                .map(|t| (p.completed as f64 / t as f64) * 100.0),
            message: p.message.clone(),
        }
    }
}

fn status_label(s: &TaskStatus) -> &'static str {
    match s {
        TaskStatus::Queued => "Queued",
        TaskStatus::Running => "Running",
        TaskStatus::Completing => "Completing",
        TaskStatus::Completed => "Completed",
        TaskStatus::Failed => "Failed",
        TaskStatus::Cancelled => "Cancelled",
        TaskStatus::TimedOut => "Timed Out",
    }
}

fn task_kind_label(kind: &TaskKind) -> &'static str {
    match kind {
        TaskKind::LoadTest(_) => "Load Test",
        TaskKind::StressTest(_) => "Stress Test",
        TaskKind::PortScan(_) => "Port Scan",
        TaskKind::EndpointScan(_) => "Endpoint Scan",
        TaskKind::Fingerprint(_) => "Fingerprint",
        TaskKind::Fuzz(_) => "Fuzz",
        TaskKind::Waf(_) => "WAF Detection",
        TaskKind::WafStress(_) => "WAF Stress",
        TaskKind::Pipeline(_) => "Pipeline",
        TaskKind::Recon(_) => "Recon",
        TaskKind::PacketCapture(_) => "Packet Capture",
        TaskKind::PacketTraceroute(_) => "Traceroute",
        TaskKind::PacketSend(_) => "Packet Send",
        TaskKind::GraphQl(_) => "GraphQL",
        TaskKind::OAuth(_) => "OAuth",
        TaskKind::AuthTest(_) => "Auth Test",
        TaskKind::Nse(_) => "NSE Script",
        TaskKind::Hunt(_) => "Vuln Hunt",
        TaskKind::Browser(_) => "Browser",
        TaskKind::Compliance(_) => "Compliance",
        TaskKind::Storage(_) => "Storage",
        TaskKind::Integrations(_) => "Integration",
        TaskKind::Workflow(_) => "Workflow",
        TaskKind::Vuln(_) => "Vuln Scan",
        TaskKind::Wireless(_) => "Wireless Recon",
        TaskKind::WirelessActive(_) => "Wireless Active",
        TaskKind::DbPentest(_) => "DB Pentest",
        TaskKind::Intercept(_) => "Intercept Proxy",
        TaskKind::C2(_) => "C2 Simulation",
    }
}

fn outcome_kind_str(o: &eggsec_runtime::event::TaskOutcome) -> String {
    match o {
        eggsec_runtime::event::TaskOutcome::Json(_) => "json".into(),
        eggsec_runtime::event::TaskOutcome::Text(_) => "text".into(),
        eggsec_runtime::event::TaskOutcome::Artifact { .. } => "artifact".into(),
        eggsec_runtime::event::TaskOutcome::Result(env) => env.kind.clone(),
        eggsec_runtime::event::TaskOutcome::Empty => "empty".into(),
    }
}
