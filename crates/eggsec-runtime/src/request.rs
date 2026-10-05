use serde::{Deserialize, Serialize};

use crate::ids::ClientId;

/// Frontend-neutral execution surface for wire transport.
///
/// This is a serializable wire DTO for protocol compatibility (session
/// attachment, request transport, audit labels). It is NOT a second semantic
/// owner: the canonical enforcement identity is
/// `eggsec::config::ExecutionSurface`, owned by the engine. Conversion in both
/// directions lives exhaustively in `eggsec::runtime_bridge::surface`
/// (`runtime_surface_to_execution_surface` /
/// `execution_surface_to_runtime_surface`); `Unknown` is rejected at the
/// boundary rather than silently mapped. Adding a variant without updating
/// that conversion is a compile/test failure by design.
///
/// `eggsec-runtime` must stay isolated from engine/domain crates (see
/// architecture guard 22), so the canonical enum cannot live here; the wire
/// DTO plus exhaustive bridge conversion is the stable boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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
    #[default]
    Unknown,
}

impl std::fmt::Display for RuntimeSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadTestParams {
    pub target: String,
    pub method: String,
    /// Total requests to send. When absent, legacy callers use `connections`.
    pub requests: Option<u64>,
    pub connections: Option<u32>,
    pub duration_secs: Option<u32>,
    pub rate_limit: Option<u32>,
    /// Optional request body. Rejected for bodyless methods by normalization.
    #[serde(default)]
    pub body: Option<String>,
    /// Request headers as `Name: Value` entries, validated by normalization.
    #[serde(default)]
    pub headers: Option<Vec<String>>,
}

/// Stress test parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StressTestParams {
    pub target: String,
    pub flood_type: String,
    pub rate_pps: Option<u64>,
    pub duration_secs: Option<u32>,
    pub threads: Option<u32>,
}

/// Port scan parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortScanParams {
    pub target: String,
    pub ports: Option<String>,
    pub scan_type: Option<String>,
    /// Scan UDP instead of TCP. `#[serde(default)]` so payloads written
    /// before this field existed still deserialize.
    #[serde(default)]
    pub udp: Option<bool>,
    pub timeout_ms: Option<u64>,
    pub concurrency: Option<usize>,
}

/// Endpoint scan parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointScanParams {
    pub target: String,
    pub methods: Option<Vec<String>>,
    pub wordlist: Option<String>,
    pub concurrency: Option<usize>,
    pub timeout_secs: Option<u64>,
    /// Keep 404 responses in the result set. `None` means "use the engine
    /// default" (exclude 404s), which is what the CLI's opt-in
    /// `--include-404` flag implies when absent. The TUI's checkbox defaults
    /// to on, so it sends `Some(true)` explicitly.
    ///
    /// `#[serde(default)]` is required: serde does not treat a missing
    /// `Option` field as `None`, so without it this struct would fail to
    /// deserialize any payload written before the field existed.
    #[serde(default)]
    pub include_404: Option<bool>,
}

/// Fingerprint parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FingerprintParams {
    pub target: String,
    pub ports: Option<String>,
    pub timeout_secs: Option<u64>,
    pub concurrency: Option<usize>,
}

/// Fuzz parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuzzParams {
    pub target: String,
    pub payload_type: Option<String>,
    pub threads: Option<u32>,
    pub mode: Option<String>,
    pub mutations: Option<bool>,
    pub mutation_count: Option<usize>,
    pub method: Option<String>,
    pub param: Option<String>,
    pub timeout: Option<u64>,
    pub graphql_introspection: Option<bool>,
    pub graphql_depth_bypass: Option<bool>,
    pub graphql_alias_overload: Option<bool>,
    pub oauth_redirect_test: Option<bool>,
    pub oauth_scope_test: Option<bool>,
    pub oauth_state_test: Option<bool>,
    pub oauth_grant_test: Option<bool>,
}

/// WAF detection parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WafParams {
    pub target: String,
    pub bypass_mode: Option<bool>,
    pub techniques: Option<Vec<String>>,
}

/// WAF stress test parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WafStressParams {
    pub target: String,
    pub requests: Option<u32>,
    pub concurrency: Option<usize>,
}

/// Pipeline parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineParams {
    pub target: String,
    pub profile: Option<String>,
    /// Report format for `output_file` (pretty|json|compact|html|csv|sarif|junit|markdown).
    #[serde(default)]
    pub output_format: Option<String>,
    /// Destination for the rendered report, relative to the export directory.
    #[serde(default)]
    pub output_file: Option<String>,
}

/// Recon parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconParams {
    pub target: String,
    pub modules: Option<Vec<String>>,
}

/// Packet capture parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketCaptureParams {
    pub interface: Option<String>,
    pub filter: Option<String>,
    pub duration_secs: Option<u32>,
    pub max_packets: Option<usize>,
    pub promiscuous: Option<bool>,
}

/// Packet traceroute parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketTracerouteParams {
    pub target: String,
    pub max_hops: Option<u32>,
}

/// Packet send parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketSendParams {
    pub target: String,
    pub protocol: String,
    pub payload: Option<String>,
    pub port: Option<u16>,
    pub count: Option<u32>,
    pub packet_size: Option<usize>,
}

/// GraphQL testing parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQlParams {
    pub target: String,
    pub introspection: Option<bool>,
    pub inject: Option<bool>,
    pub depth_bypass: Option<bool>,
    pub alias_overload: Option<bool>,
    pub concurrency: Option<usize>,
    pub timeout_secs: Option<u64>,
}

/// OAuth testing parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthParams {
    pub target: String,
    pub flow: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub redirect_test: Option<bool>,
    pub scope_test: Option<bool>,
    pub state_test: Option<bool>,
    pub grant_test: Option<bool>,
    pub concurrency: Option<usize>,
    pub timeout_secs: Option<u64>,
}

/// Authentication test parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthTestParams {
    pub target: String,
    pub username: Option<String>,
    pub credential_list: Option<String>,
    pub credential_file: Option<String>,
    pub max_attempts: Option<usize>,
    pub concurrency: Option<usize>,
    pub timeout_secs: Option<u64>,
}

/// NSE script parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NseParams {
    pub target: String,
    pub script: String,
    pub args: Option<String>,
    /// Path to a user-provided `.nse` script, resolved by the engine's
    /// `NseScriptSource::File` / `ScriptResolver` rather than by a direct
    /// filesystem read.
    ///
    /// When set it takes precedence over `script`, which still carries the
    /// built-in identity for reporting and validation. Honours the resolver's
    /// policy gate, extension allowlist and root containment — never a
    /// permissive "load whatever path was typed".
    ///
    /// Honoured only for manual profiles. An automated profile paired with a
    /// custom script is refused by the engine rather than executed, so this
    /// field cannot become a remote-script-execution primitive if the NSE
    /// automated-surface quarantine (M007) is later lifted.
    #[serde(default)]
    pub custom_script: Option<String>,
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageParams {
    pub storage_type: String,
    pub path: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub max_connections: Option<u32>,
    /// connect|list_scans|list_findings|search_cve
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub scan_id: Option<String>,
    #[serde(default)]
    pub cve_id: Option<String>,
    #[serde(default)]
    pub severity_filter: Option<String>,
    /// Name of the environment variable holding the password. The password
    /// itself is never carried on the wire.
    #[serde(default)]
    pub password_env: Option<String>,
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

/// Default attack mode for an unconfigured active wireless request.
fn default_wireless_attack_type() -> String {
    "deauth".to_string()
}

/// Default frame budget for an unconfigured active wireless request.
fn default_wireless_frame_count() -> u64 {
    100
}

/// Default frame rate for an unconfigured active wireless request.
fn default_wireless_rate_limit() -> u64 {
    10
}

/// Fail-safe default: an active wireless request that does not say otherwise
/// is simulated, never transmitted.
fn default_wireless_dry_run() -> bool {
    true
}

/// Wireless active (deauth/disassoc) parameters.
///
/// SAFETY: `dry_run` is `true` in *both* the `Default` impl and the
/// `#[serde(default = ...)]` path, so a payload written before these fields
/// existed — or any payload that omits them — simulates the attack instead of
/// transmitting live frames. Never invert this default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WirelessActiveParams {
    pub interface: Option<String>,
    pub target_bssid: Option<String>,
    /// Active attack mode: `deauth` or `disassoc`. Defaults to `deauth`.
    #[serde(default = "default_wireless_attack_type")]
    pub attack_type: String,
    /// Target client MAC for a directed (per-client) attack. `None` broadcasts.
    #[serde(default)]
    pub client: Option<String>,
    /// Frames to emit. The engine clamps this to 1000.
    #[serde(default = "default_wireless_frame_count")]
    pub frame_count: u64,
    /// Frames per second. The engine clamps this to 100.
    #[serde(default = "default_wireless_rate_limit")]
    pub rate_limit: u64,
    /// `true` simulates without transmitting. Defaults to `true` (fail-safe).
    #[serde(default = "default_wireless_dry_run")]
    pub dry_run: bool,
}

impl Default for WirelessActiveParams {
    /// Fail-safe defaults: an unconfigured active attack is a dry run.
    fn default() -> Self {
        Self {
            interface: None,
            target_bssid: None,
            attack_type: default_wireless_attack_type(),
            client: None,
            frame_count: default_wireless_frame_count(),
            rate_limit: default_wireless_rate_limit(),
            dry_run: default_wireless_dry_run(),
        }
    }
}

/// Database pentest parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbPentestParams {
    pub db_type: String,
    pub target: String,
    pub port: Option<u16>,
    pub checks: Option<String>,
    pub max_queries: Option<u64>,
    pub max_duration: Option<u64>,
    pub dry_run: Option<bool>,
    pub allow_advanced: Option<bool>,
}

/// Intercept proxy parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterceptParams {
    pub listen_port: Option<u16>,
    pub target: Option<String>,
    pub listen_host: Option<String>,
    pub dry_run: Option<bool>,
    pub max_flows: Option<u64>,
}

/// C2 simulation parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct C2Params {
    pub profile: Option<String>,
    pub target: Option<String>,
    pub dry_run: Option<bool>,
}

impl TaskKind {
    /// Wire capability name for this task kind variant.
    ///
    /// This name must match the entries in [`RuntimeCapabilities::task_kinds`]
    /// for the task to be supported by a given runtime configuration. It is
    /// transport metadata, not the canonical operation identity (see
    /// [`Self::operation_id`]).
    pub fn capability_name(&self) -> &'static str {
        match self {
            TaskKind::LoadTest(_) => "load-test",
            TaskKind::StressTest(_) => "stress-test",
            TaskKind::PortScan(_) => "port-scan",
            TaskKind::EndpointScan(_) => "endpoint-scan",
            TaskKind::Fingerprint(_) => "fingerprint",
            TaskKind::Fuzz(_) => "fuzz",
            TaskKind::Waf(_) => "waf",
            TaskKind::WafStress(_) => "waf-stress",
            TaskKind::Pipeline(_) => "pipeline",
            TaskKind::Recon(_) => "recon",
            TaskKind::PacketCapture(_) => "packet-capture",
            TaskKind::PacketTraceroute(_) => "traceroute",
            TaskKind::PacketSend(_) => "packet-send",
            TaskKind::GraphQl(_) => "graphql",
            TaskKind::OAuth(_) => "oauth",
            TaskKind::AuthTest(_) => "auth-test",
            TaskKind::Nse(_) => "nse",
            TaskKind::Hunt(_) => "hunt",
            TaskKind::Browser(_) => "browser",
            TaskKind::Compliance(_) => "compliance",
            TaskKind::Storage(_) => "storage",
            TaskKind::Integrations(_) => "integration",
            TaskKind::Workflow(_) => "workflow",
            TaskKind::Vuln(_) => "vuln",
            TaskKind::Wireless(_) => "wireless",
            TaskKind::WirelessActive(_) => "wireless-active",
            TaskKind::DbPentest(_) => "db-pentest",
            TaskKind::Intercept(_) => "intercept",
            TaskKind::C2(_) => "c2",
        }
    }

    /// Canonical engine operation ID for this task kind (wire-compat accessor).
    ///
    /// This is the single wire-side match for operation identity. Engine
    /// adapters (`operation_id_for_task_kind`, `descriptor_for_run_request`,
    /// `CanonicalOperationRequest::from_task_kind`) delegate to this method
    /// rather than maintaining parallel matches, so the wire representation
    /// cannot silently diverge. The engine-side canonical identity for an
    /// already-converted request is
    /// `CanonicalOperationRequest::operation_id()`; this method exists so
    /// telemetry/capability checks before engine conversion derive from the
    /// same exhaustive table.
    ///
    /// Exhaustive (no wildcard): adding a `TaskKind` variant without updating
    /// this function is a compile error.
    ///
    /// Wire-only packet capture/traceroute/send kinds share the `packet`
    /// operation family explicitly (they do not fall through string aliases).
    /// Interface-bound or target-less kinds map to their operation but carry
    /// `None` from [`Self::canonical_target`]; the bridge then validates
    /// against `OperationMetadata` target policy and fails explicitly.
    pub fn operation_id(&self) -> &'static str {
        match self {
            TaskKind::LoadTest(_) => "load-test",
            TaskKind::StressTest(_) => "stress-test",
            TaskKind::PortScan(_) => "scan-ports",
            TaskKind::EndpointScan(_) => "scan-endpoints",
            TaskKind::Fingerprint(_) => "fingerprint",
            TaskKind::Fuzz(_) => "fuzz",
            TaskKind::Waf(_) => "waf-detect",
            TaskKind::WafStress(_) => "waf-stress",
            TaskKind::Pipeline(_) => "pipeline",
            TaskKind::Recon(_) => "recon",
            TaskKind::PacketCapture(_) => "packet",
            TaskKind::PacketTraceroute(_) => "packet",
            TaskKind::PacketSend(_) => "packet",
            TaskKind::GraphQl(_) => "graphql",
            TaskKind::OAuth(_) => "oauth",
            TaskKind::AuthTest(_) => "auth-test",
            TaskKind::Nse(_) => "nse",
            TaskKind::Hunt(_) => "hunt",
            TaskKind::Browser(_) => "browser",
            TaskKind::Compliance(_) => "compliance",
            TaskKind::Storage(_) => "storage",
            TaskKind::Integrations(_) => "integrations",
            TaskKind::Workflow(_) => "workflow",
            TaskKind::Vuln(_) => "vuln",
            TaskKind::Wireless(_) => "wireless",
            TaskKind::WirelessActive(_) => "wireless",
            TaskKind::DbPentest(_) => "db-pentest",
            TaskKind::Intercept(_) => "proxy-intercept",
            TaskKind::C2(_) => "c2",
        }
    }

    /// Canonical target for this task kind (`None` for `NoTarget` or
    /// interface-bound operations). Wire-compat accessor; engine adapters
    /// delegate to this method (see [`Self::operation_id`]).
    ///
    /// Exhaustive for the same compile-failure guarantee as
    /// [`Self::operation_id`].
    pub fn canonical_target(&self) -> Option<String> {
        match self {
            TaskKind::LoadTest(p) => Some(p.target.clone()),
            TaskKind::StressTest(p) => Some(p.target.clone()),
            TaskKind::PortScan(p) => Some(p.target.clone()),
            TaskKind::EndpointScan(p) => Some(p.target.clone()),
            TaskKind::Fingerprint(p) => Some(p.target.clone()),
            TaskKind::Fuzz(p) => Some(p.target.clone()),
            TaskKind::Waf(p) => Some(p.target.clone()),
            TaskKind::WafStress(p) => Some(p.target.clone()),
            TaskKind::Pipeline(p) => Some(p.target.clone()),
            TaskKind::Recon(p) => Some(p.target.clone()),
            TaskKind::PacketCapture(_) => None,
            TaskKind::PacketTraceroute(p) => Some(p.target.clone()),
            TaskKind::PacketSend(p) => Some(p.target.clone()),
            TaskKind::GraphQl(p) => Some(p.target.clone()),
            TaskKind::OAuth(p) => Some(p.target.clone()),
            TaskKind::AuthTest(p) => Some(p.target.clone()),
            TaskKind::Nse(p) => Some(p.target.clone()),
            TaskKind::Hunt(p) => Some(p.target.clone()),
            TaskKind::Browser(p) => Some(p.target.clone()),
            TaskKind::Compliance(p) => Some(p.target.clone()),
            TaskKind::Storage(_) => None,
            TaskKind::Integrations(_) => None,
            TaskKind::Workflow(_) => None,
            TaskKind::Vuln(p) => Some(p.target.clone()),
            TaskKind::Wireless(_) => None,
            TaskKind::WirelessActive(_) => None,
            TaskKind::DbPentest(p) => Some(p.target.clone()),
            TaskKind::Intercept(p) => p.target.clone(),
            TaskKind::C2(p) => p.target.clone(),
        }
    }
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
                concurrency: None,
                udp: None,
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

    /// SAFETY regression guard: an unconfigured active attack must never arm a
    /// live deauth, on either the `Default` or the deserialization path.
    #[test]
    fn wireless_active_params_default_is_dry_run() {
        let params = WirelessActiveParams::default();
        assert!(params.dry_run, "Default must fail safe to a dry run");
        assert_eq!(params.attack_type, "deauth");
        assert_eq!(params.frame_count, 100);
        assert_eq!(params.rate_limit, 10);
        assert!(params.client.is_none());
    }

    #[test]
    fn wireless_active_params_legacy_payload_dry_runs() {
        let legacy = r#"{"interface":"wlan0","target_bssid":"aa:bb:cc:dd:ee:ff"}"#;
        let params: WirelessActiveParams = serde_json::from_str(legacy).unwrap();
        assert!(params.dry_run, "payload without dry_run must dry run");
        assert_eq!(params.attack_type, "deauth");
        assert_eq!(params.frame_count, 100);
        assert_eq!(params.rate_limit, 10);
    }

    #[test]
    fn wireless_active_params_roundtrip_preserves_live_fields() {
        let req = RunRequest {
            task_kind: TaskKind::WirelessActive(WirelessActiveParams {
                interface: Some("wlan0".into()),
                target_bssid: Some("aa:bb:cc:dd:ee:ff".into()),
                attack_type: "disassoc".into(),
                client: Some("11:22:33:44:55:66".into()),
                frame_count: 25,
                rate_limit: 5,
                dry_run: false,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: RunRequest = serde_json::from_str(&json).unwrap();
        match back.task_kind {
            TaskKind::WirelessActive(p) => {
                assert_eq!(p.attack_type, "disassoc");
                assert_eq!(p.client.as_deref(), Some("11:22:33:44:55:66"));
                assert_eq!(p.frame_count, 25);
                assert_eq!(p.rate_limit, 5);
                assert!(!p.dry_run);
            }
            other => panic!("expected wireless-active, got {other:?}"),
        }
    }
}
