use serde::{Deserialize, Serialize};

use crate::ids::ClientId;

/// Frontend-neutral execution surface, mirroring `eggsec::config::ExecutionSurface`.
///
/// This is a local serializable mirror. Later phases can implement conversion
/// to/from `eggsec::config::ExecutionSurface`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeSurface {
    CliManual,
    CliManualStrict,
    TuiManual,
    TuiManualStrict,
    Ci,
    McpServer,
    RestApi,
    GrpcApi,
    SecurityAgent,
    Unknown,
}

impl RuntimeSurface {
    pub fn label(&self) -> &'static str {
        match self {
            Self::CliManual => "cli-manual",
            Self::CliManualStrict => "cli-manual-strict",
            Self::TuiManual => "tui-manual",
            Self::TuiManualStrict => "tui-manual-strict",
            Self::Ci => "ci",
            Self::McpServer => "mcp-server",
            Self::RestApi => "rest-api",
            Self::GrpcApi => "grpc-api",
            Self::SecurityAgent => "security-agent",
            Self::Unknown => "unknown",
        }
    }
}

impl Default for RuntimeSurface {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Frontend-neutral task kind enum covering all TUI task categories.
///
/// Each variant represents a distinct tool or operation that can be submitted
/// to the runtime. Payload structs are serializable and TUI-free.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "params")]
pub enum TaskKind {
    LoadTest(LoadTestParams),
    StressTest(StressTestParams),
    PortScan(PortScanParams),
    EndpointScan(EndpointScanParams),
    Fingerprint(FingerprintParams),
    Fuzz(FuzzParams),
    Waf(WafParams),
    WafStress(WafStressParams),
    Pipeline(PipelineParams),
    Recon(ReconParams),
    PacketCapture(PacketCaptureParams),
    PacketTraceroute(PacketTracerouteParams),
    PacketSend(PacketSendParams),
    GraphQl(GraphQlParams),
    OAuth(OAuthParams),
    AuthTest(AuthTestParams),
    Nse(NseParams),
    Hunt(HuntParams),
    Browser(BrowserParams),
    Compliance(ComplianceParams),
    Storage(StorageParams),
    Integrations(IntegrationsParams),
    Workflow(WorkflowParams),
    Vuln(VulnParams),
    Wireless(WirelessParams),
    WirelessActive(WirelessActiveParams),
    DbPentest(DbPentestParams),
    Intercept(InterceptParams),
    C2(C2Params),
}

/// A runtime request to execute a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRequest {
    pub task_kind: TaskKind,
    pub requested_by: Option<ClientId>,
    pub surface: RuntimeSurface,
    pub labels: Vec<String>,
}

// ---- Payload structs ----

/// Load test parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadTestParams {
    pub target: String,
    pub method: String,
    pub connections: Option<u32>,
    pub duration_secs: Option<u32>,
    pub rate_limit: Option<u32>,
}

/// Stress test parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StressTestParams {
    pub target: String,
    pub flood_type: String,
    pub duration_secs: Option<u32>,
    pub threads: Option<u32>,
}

/// Port scan parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortScanParams {
    pub target: String,
    pub ports: Option<String>,
    pub scan_type: Option<String>,
    pub timeout_ms: Option<u64>,
}

/// Endpoint scan parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointScanParams {
    pub target: String,
    pub methods: Option<Vec<String>>,
    pub wordlist: Option<String>,
}

/// Fingerprint parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FingerprintParams {
    pub target: String,
}

/// Fuzz parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuzzParams {
    pub target: String,
    pub payload_type: Option<String>,
    pub threads: Option<u32>,
}

/// WAF detection parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WafParams {
    pub target: String,
}

/// WAF stress test parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WafStressParams {
    pub target: String,
    pub requests: Option<u32>,
}

/// Pipeline parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineParams {
    pub target: String,
    pub profile: Option<String>,
}

/// Recon parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconParams {
    pub target: String,
    pub modules: Option<Vec<String>>,
}

/// Packet capture parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketCaptureParams {
    pub interface: Option<String>,
    pub filter: Option<String>,
    pub duration_secs: Option<u32>,
}

/// Packet traceroute parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketTracerouteParams {
    pub target: String,
    pub max_hops: Option<u32>,
}

/// Packet send parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketSendParams {
    pub target: String,
    pub protocol: String,
    pub payload: Option<String>,
}

/// GraphQL testing parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQlParams {
    pub target: String,
    pub introspection: Option<bool>,
}

/// OAuth testing parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthParams {
    pub target: String,
    pub flow: Option<String>,
}

/// Authentication test parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthTestParams {
    pub target: String,
    pub username: Option<String>,
    pub credential_list: Option<String>,
}

/// NSE script parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NseParams {
    pub target: String,
    pub script: String,
    pub args: Option<String>,
}

/// Vulnerability hunt parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HuntParams {
    pub target: String,
    pub hunt_type: Option<String>,
}

/// Browser testing parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserParams {
    pub target: String,
    pub headless: Option<bool>,
}

/// Compliance check parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplianceParams {
    pub target: String,
    pub framework: Option<String>,
}

/// Storage parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageParams {
    pub storage_type: String,
    pub path: Option<String>,
}

/// Integration parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationsParams {
    pub integration_type: String,
    pub config: Option<serde_json::Value>,
}

/// Workflow parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowParams {
    pub workflow_id: Option<String>,
    pub steps: Option<Vec<String>>,
}

/// Vulnerability parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VulnParams {
    pub target: String,
    pub vuln_type: Option<String>,
}

/// Wireless recon parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WirelessParams {
    pub interface: Option<String>,
    pub duration_secs: Option<u32>,
}

/// Wireless active (deauth/disassoc) parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WirelessActiveParams {
    pub interface: Option<String>,
    pub target_bssid: Option<String>,
}

/// Database pentest parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbPentestParams {
    pub db_type: String,
    pub target: String,
    pub port: Option<u16>,
}

/// Intercept proxy parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterceptParams {
    pub listen_port: Option<u16>,
    pub target: Option<String>,
}

/// C2 simulation parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct C2Params {
    pub profile: Option<String>,
    pub target: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_request_roundtrip() {
        let req = RunRequest {
            task_kind: TaskKind::PortScan(PortScanParams {
                target: "10.0.0.1".into(),
                ports: Some("80,443".into()),
                scan_type: Some("syn".into()),
                timeout_ms: Some(3000),
            }),
            requested_by: Some(ClientId::new()),
            surface: RuntimeSurface::CliManual,
            labels: vec!["test".into()],
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: RunRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.surface, deserialized.surface);
        assert_eq!(req.labels, deserialized.labels);
    }

    #[test]
    fn runtime_surface_label() {
        assert_eq!(RuntimeSurface::CliManual.label(), "cli-manual");
        assert_eq!(RuntimeSurface::RestApi.label(), "rest-api");
        assert_eq!(RuntimeSurface::Unknown.label(), "unknown");
    }
}
