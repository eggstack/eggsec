use serde::{Deserialize, Serialize};

use crate::capabilities::RuntimeCapabilities;
use crate::event::TaskProgress;
use crate::event::TaskStatus;
use crate::ids::{SessionId, TaskId};
use crate::request::TaskKind;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: SessionId,
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
            active_tasks: vec![],
            completed_tasks: vec![],
            capabilities: RuntimeCapabilities::default(),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: SessionSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot.session_id, deserialized.session_id);
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
}
