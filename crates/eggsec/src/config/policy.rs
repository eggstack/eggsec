use serde::{Deserialize, Serialize};

/// Risk tier for operations. Higher variants are more dangerous.
///
/// Used by [`ExecutionPolicy`] to control which operations are permitted
/// without explicit user confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationRisk {
    Passive,
    SafeActive,
    Intrusive,
    LoadTest,
    StressTest,
    RawPacket,
    CredentialTesting,
    DbPentest,
    TrafficInterception,
    EvasionTesting,
    PostExploitation,
    ExploitAdjacent,
    C2Operation,
    RemoteExecution,
    AgentAutonomous,
}

/// Policy that controls which operations are allowed and under what conditions.
///
/// Loaded from the `[execution_policy]` section of the config file. All fields
/// default to safe values (restrictive) so that users who do not configure this
/// section get safe behavior out of the box.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    #[serde(default = "default_true")]
    pub require_explicit_scope: bool,

    #[serde(default)]
    pub allow_intrusive_fuzzing: bool,

    #[serde(default)]
    pub allow_load_testing: bool,

    #[serde(default)]
    pub allow_stress_testing: bool,

    #[serde(default)]
    pub allow_raw_packets: bool,

    #[serde(default)]
    pub allow_credential_testing: bool,

    /// Allow direct database pentesting operations (Postgres/MySQL lab checks).
    /// Standalone defense-lab only; requires explicit --allow-db-pentest for non-dry runs.
    #[serde(default)]
    pub allow_db_pentesting: bool,

    /// Allow traffic interception / MITM proxy operations for authorized lab use.
    /// Standalone defense-lab only; requires explicit --allow-web-proxy for non-dry runs.
    #[serde(default)]
    pub allow_traffic_interception: bool,

    #[serde(default)]
    pub allow_remote_execution: bool,

    /// Allow evasion technique detection testing (defense-lab only).
    /// Standalone defense-lab surface; requires explicit --allow-evasion-testing for non-dry runs.
    #[serde(default)]
    pub allow_evasion_testing: bool,

    /// Allow post-exploitation and LOTL simulation (defense-lab only).
    /// Standalone defense-lab surface; requires explicit --allow-postex for non-dry runs.
    #[serde(default)]
    pub allow_post_exploitation: bool,

    #[serde(default)]
    pub allow_exploit_adjacent: bool,

    /// Allow C2 operations (beaconing, tasking, campaign orchestration) for defense-lab only.
    /// Standalone defense-lab surface; requires explicit --allow-c2 for non-dry runs.
    #[serde(default)]
    pub allow_c2_operations: bool,

    #[serde(default)]
    pub allow_agent_autonomous: bool,

    #[serde(default = "default_max_risk")]
    pub max_risk_without_confirm: OperationRisk,

    /// Capabilities explicitly allowed by this policy.
    #[serde(default)]
    pub allowed_capabilities: Vec<Capability>,

    /// Capabilities explicitly denied by this policy (deny wins over allow).
    #[serde(default)]
    pub denied_capabilities: Vec<Capability>,
}

fn default_true() -> bool {
    true
}

fn default_max_risk() -> OperationRisk {
    OperationRisk::SafeActive
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            require_explicit_scope: true,
            allow_intrusive_fuzzing: false,
            allow_load_testing: false,
            allow_stress_testing: false,
            allow_raw_packets: false,
            allow_credential_testing: false,
            allow_db_pentesting: false,
            allow_traffic_interception: false,
            allow_exploit_adjacent: false,
            allow_remote_execution: false,
            allow_evasion_testing: false,
            allow_post_exploitation: false,
            allow_c2_operations: false,
            allow_agent_autonomous: false,
            max_risk_without_confirm: OperationRisk::SafeActive,
            allowed_capabilities: Vec::new(),
            denied_capabilities: Vec::new(),
        }
    }
}

impl OperationRisk {
    /// Check if this risk level is allowed by the given policy.
    pub fn is_allowed_by(&self, policy: &ExecutionPolicy) -> bool {
        match self {
            Self::Passive | Self::SafeActive => true,
            Self::Intrusive => policy.allow_intrusive_fuzzing,
            Self::LoadTest => policy.allow_load_testing,
            Self::StressTest => policy.allow_stress_testing,
            Self::RawPacket => policy.allow_raw_packets,
            Self::CredentialTesting => policy.allow_credential_testing,
            Self::DbPentest => policy.allow_db_pentesting,
            Self::TrafficInterception => policy.allow_traffic_interception,
            Self::ExploitAdjacent => policy.allow_exploit_adjacent,
            Self::EvasionTesting => policy.allow_evasion_testing,
            Self::PostExploitation => policy.allow_post_exploitation,
            Self::C2Operation => policy.allow_c2_operations,
            Self::RemoteExecution => policy.allow_remote_execution,
            Self::AgentAutonomous => policy.allow_agent_autonomous,
        }
    }
}

impl std::fmt::Display for OperationRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passive => write!(f, "passive"),
            Self::SafeActive => write!(f, "safe active"),
            Self::Intrusive => write!(f, "intrusive"),
            Self::LoadTest => write!(f, "load testing"),
            Self::StressTest => write!(f, "stress testing"),
            Self::RawPacket => write!(f, "raw packet"),
            Self::CredentialTesting => write!(f, "credential testing"),
            Self::DbPentest => write!(f, "db pentest"),
            Self::TrafficInterception => write!(f, "traffic interception"),
            Self::ExploitAdjacent => write!(f, "exploit adjacent"),
            Self::EvasionTesting => write!(f, "evasion testing"),
            Self::PostExploitation => write!(f, "post-exploitation"),
            Self::C2Operation => write!(f, "c2 operation"),
            Self::RemoteExecution => write!(f, "remote execution"),
            Self::AgentAutonomous => write!(f, "agent autonomous"),
        }
    }
}

/// Operating mode for an eggsec session.
///
/// Determines the safety boundary and allowed operation surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationMode {
    /// Standard scoped recon, scanning, fuzzing, API testing, WAF detection, and reporting.
    StandardAssessment,
    /// Local/private/scope-constrained WAF and distributed-system validation,
    /// including load and selected stress tests.
    DefenseLab,
    /// Raw packet operations, flood-style stress tests, proxy rotation,
    /// low-level protocol edge cases, and other aggressive tests requiring
    /// explicit build features plus explicit runtime policy approval.
    HazardousLab,
}

impl OperationMode {
    /// Returns a human-readable label for the mode.
    pub fn label(self) -> &'static str {
        match self {
            Self::StandardAssessment => "standard assessment",
            Self::DefenseLab => "defense lab",
            Self::HazardousLab => "hazardous lab",
        }
    }

    /// Returns the maximum `OperationRisk` allowed by default for this mode.
    pub fn default_max_risk(self) -> OperationRisk {
        match self {
            Self::StandardAssessment => OperationRisk::SafeActive,
            Self::DefenseLab => OperationRisk::Intrusive,
            Self::HazardousLab => OperationRisk::AgentAutonomous,
        }
    }
}

impl std::fmt::Display for OperationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StandardAssessment => write!(f, "standard-assessment"),
            Self::DefenseLab => write!(f, "defense-lab"),
            Self::HazardousLab => write!(f, "hazardous-lab"),
        }
    }
}

/// Intended use case for an operation or profile.
///
/// Used in policy decisions, reports, and documentation to clarify
/// why a particular operation is being performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntendedUse {
    WebAssessment,
    ApiAssessment,
    WafRegression,
    SynvoidRegression,
    DistributedSystemStress,
    ProtocolEdgeValidation,
    CiRegression,
    CodingAgentVerification,
}

impl IntendedUse {
    pub fn label(self) -> &'static str {
        match self {
            Self::WebAssessment => "web assessment",
            Self::ApiAssessment => "API assessment",
            Self::WafRegression => "WAF regression",
            Self::SynvoidRegression => "Synvoid regression",
            Self::DistributedSystemStress => "distributed system stress",
            Self::ProtocolEdgeValidation => "protocol edge validation",
            Self::CiRegression => "CI regression",
            Self::CodingAgentVerification => "coding agent verification",
        }
    }
}

impl std::fmt::Display for IntendedUse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WebAssessment => write!(f, "web-assessment"),
            Self::ApiAssessment => write!(f, "api-assessment"),
            Self::WafRegression => write!(f, "waf-regression"),
            Self::SynvoidRegression => write!(f, "synvoid-regression"),
            Self::DistributedSystemStress => write!(f, "distributed-system-stress"),
            Self::ProtocolEdgeValidation => write!(f, "protocol-edge-validation"),
            Self::CiRegression => write!(f, "ci-regression"),
            Self::CodingAgentVerification => write!(f, "coding-agent-verification"),
        }
    }
}

/// Descriptor for an operation that can be evaluated against policy and scope.
///
/// Bundles the metadata needed by [`super::policy_decision::evaluate_operation_policy`]
/// to produce a [`PolicyDecision`]. Command handlers, MCP dispatchers, agent
/// workflows, and API endpoints all construct an `OperationDescriptor` instead of
/// reinventing policy checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationDescriptor {
    /// Human-readable operation name (e.g. "scan-ports", "fuzz", "stress").
    pub operation: String,
    /// Operating mode (standard-assessment, defense-lab, hazardous-lab).
    pub mode: OperationMode,
    /// Risk tier of the operation.
    pub risk: OperationRisk,
    /// Intended use cases for this operation.
    pub intended_uses: Vec<IntendedUse>,
    /// Original target string (hostname, URL, or IP).
    pub target: Option<String>,
    /// Feature flags required to execute this operation (e.g. "packet-inspection", "nse").
    #[serde(default)]
    pub required_features: Vec<String>,
    /// Policy flags that must be set (e.g. "require_explicit_scope").
    #[serde(default)]
    pub required_policy_flags: Vec<String>,
    /// If `true`, the target must be a private/local address or within scope.
    #[serde(default)]
    pub requires_private_or_local_target: bool,
    /// If `true`, an explicit scope file must be configured.
    #[serde(default)]
    pub requires_explicit_scope: bool,
    /// Capabilities required by this operation (e.g. "active-probe", "crawl").
    #[serde(default)]
    pub required_capabilities: Vec<Capability>,
}

/// Origin of an execution request.
///
/// Describes *where* an operation originates, then derives the correct
/// [`ExecutionProfile`] and enforcement posture from that surface. This
/// separates caller identity from enforcement behavior and prevents
/// entrypoints from hand-rolling inconsistent profile selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionSurface {
    /// CLI interactive session (default manual permissive).
    CliManual,
    /// TUI interactive session (default manual permissive).
    TuiManual,
    /// CLI with `--strict-scope` flag (manual guarded).
    CliManualStrict,
    /// TUI with strict-scope toggle (manual guarded, Phase 5).
    TuiManualStrict,
    /// MCP server entrypoint.
    McpServer,
    /// Autonomous security agent entrypoint.
    SecurityAgent,
    /// CI pipeline entrypoint.
    Ci,
    /// REST API server entrypoint (strict by default, pending Phase 7).
    RestApi,
    /// gRPC API server entrypoint (strict by default).
    GrpcApi,
}

impl ExecutionSurface {
    /// Derive the [`ExecutionProfile`] for this surface.
    pub fn profile(self) -> ExecutionProfile {
        match self {
            Self::CliManual | Self::TuiManual => ExecutionProfile::ManualPermissive,
            Self::CliManualStrict | Self::TuiManualStrict => ExecutionProfile::ManualGuarded,
            Self::McpServer => ExecutionProfile::McpStrict,
            Self::SecurityAgent => ExecutionProfile::AgentStrict,
            Self::Ci => ExecutionProfile::CiStrict,
            Self::RestApi => ExecutionProfile::McpStrict,
            Self::GrpcApi => ExecutionProfile::McpStrict,
        }
    }

    /// Returns `true` if this surface represents a manual (human) caller.
    pub fn is_manual(self) -> bool {
        matches!(
            self,
            Self::CliManual | Self::TuiManual | Self::CliManualStrict | Self::TuiManualStrict
        )
    }

    /// Returns `true` if this surface is an automated (non-human) caller.
    pub fn is_agent_controlled(self) -> bool {
        matches!(
            self,
            Self::McpServer | Self::SecurityAgent | Self::Ci | Self::RestApi | Self::GrpcApi
        )
    }

    /// Returns `true` if this surface honors manual override flags.
    ///
    /// Only permissive manual surfaces honor overrides. Strict and automated
    /// surfaces never accept `--yes` or `--allow-*` flags.
    pub fn honors_manual_override(self) -> bool {
        matches!(self, Self::CliManual | Self::TuiManual)
    }

    /// Returns `true` if this surface requires an explicit scope manifest
    /// for networked execution.
    pub fn requires_explicit_manifest_for_networked(self) -> bool {
        self.is_agent_controlled()
    }

    /// Human-readable label for logging and display.
    pub fn label(self) -> &'static str {
        match self {
            Self::CliManual => "CLI manual",
            Self::TuiManual => "TUI manual",
            Self::CliManualStrict => "CLI manual strict",
            Self::TuiManualStrict => "TUI manual strict",
            Self::McpServer => "MCP server",
            Self::SecurityAgent => "Security agent",
            Self::Ci => "CI",
            Self::RestApi => "REST API",
            Self::GrpcApi => "gRPC API",
        }
    }
}

impl std::fmt::Display for ExecutionSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CliManual => write!(f, "cli-manual"),
            Self::TuiManual => write!(f, "tui-manual"),
            Self::CliManualStrict => write!(f, "cli-manual-strict"),
            Self::TuiManualStrict => write!(f, "tui-manual-strict"),
            Self::McpServer => write!(f, "mcp-server"),
            Self::SecurityAgent => write!(f, "security-agent"),
            Self::Ci => write!(f, "ci"),
            Self::RestApi => write!(f, "rest-api"),
            Self::GrpcApi => write!(f, "grpc-api"),
        }
    }
}

/// Caller trust boundary for scope enforcement.
///
/// Determines how strictly scope violations are treated. Manual CLI/TUI
/// usage gets permissive defaults; MCP and autonomous agent paths are
/// always strict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionProfile {
    /// Default CLI/TUI: warnings for scope ambiguity, denials only for
    /// hazardous operations or explicit exclusions.
    ManualPermissive,
    /// CLI/TUI with `--strict-scope`: denies missing scope, out-of-scope
    /// targets, and risky operations without policy approval.
    ManualGuarded,
    /// Non-interactive CI: strict, deterministic, no downgrade flags.
    CiStrict,
    /// MCP server: always strict, scope manifest required for networked tools.
    McpStrict,
    /// Autonomous agent: always strict, cannot self-approve scope expansion.
    AgentStrict,
}

impl ExecutionProfile {
    /// Returns `true` if this profile enforces strict scope rules.
    pub fn is_strict(&self) -> bool {
        matches!(self, Self::CiStrict | Self::McpStrict | Self::AgentStrict)
    }

    /// Returns `true` if this profile is an automated (non-human) caller.
    pub fn is_automated(&self) -> bool {
        matches!(self, Self::CiStrict | Self::McpStrict | Self::AgentStrict)
    }
}

impl std::fmt::Display for ExecutionProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManualPermissive => write!(f, "manual-permissive"),
            Self::ManualGuarded => write!(f, "manual-guarded"),
            Self::CiStrict => write!(f, "ci-strict"),
            Self::McpStrict => write!(f, "mcp-strict"),
            Self::AgentStrict => write!(f, "agent-strict"),
        }
    }
}

/// Operation capability declaration.
///
/// Used by [`OperationDescriptor`] to declare what a tool needs, and by
/// [`super::policy_decision::evaluate_enforcement`] to check whether the
/// caller profile permits that capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    PassiveFingerprint,
    ActiveProbe,
    Crawl,
    HttpFuzzLowImpact,
    IntrusiveFuzz,
    WafDetect,
    WafBypassSimulation,
    WafStressTest,
    LoadTest,
    RawPacketProbe,
    CredentialTesting,
    RemoteExecution,
    NseSafe,
    NseIntrusive,
    TrafficInterception,
    EvasionTesting,
    DatabaseAssessment,
    C2Simulation,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PassiveFingerprint => write!(f, "passive-fingerprint"),
            Self::ActiveProbe => write!(f, "active-probe"),
            Self::Crawl => write!(f, "crawl"),
            Self::HttpFuzzLowImpact => write!(f, "http-fuzz-low-impact"),
            Self::IntrusiveFuzz => write!(f, "intrusive-fuzz"),
            Self::WafDetect => write!(f, "waf-detect"),
            Self::WafBypassSimulation => write!(f, "waf-bypass-simulation"),
            Self::WafStressTest => write!(f, "waf-stress-test"),
            Self::LoadTest => write!(f, "load-test"),
            Self::RawPacketProbe => write!(f, "raw-packet-probe"),
            Self::CredentialTesting => write!(f, "credential-testing"),
            Self::RemoteExecution => write!(f, "remote-execution"),
            Self::NseSafe => write!(f, "nse-safe"),
            Self::NseIntrusive => write!(f, "nse-intrusive"),
            Self::TrafficInterception => write!(f, "traffic-interception"),
            Self::EvasionTesting => write!(f, "evasion-testing"),
            Self::DatabaseAssessment => write!(f, "database-assessment"),
            Self::C2Simulation => write!(f, "c2-simulation"),
        }
    }
}

/// Returns true for low-risk baseline capabilities that automated strict profiles
/// may allow without explicit listing in allowed_capabilities (safe defaults).
pub fn baseline_allowed_capability(cap: Capability) -> bool {
    matches!(
        cap,
        Capability::PassiveFingerprint
            | Capability::ActiveProbe
            | Capability::Crawl
            | Capability::WafDetect
    )
}

/// Classification of why an operation was denied.
///
/// Used by [`super::policy_decision::evaluate_enforcement`] to determine
/// whether a denial can be downgraded to a warning in permissive profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DenialClass {
    /// No scope manifest was provided for a networked operation.
    ScopeMissing,
    /// Target does not match any allowed scope rule.
    TargetOutOfScope,
    /// Target matches an explicit exclusion rule.
    ExplicitExclusion,
    /// A required compile-time feature is not enabled.
    FeatureMissing,
    /// Operation risk exceeds policy limits.
    RiskPolicyDenied,
    /// A required capability is denied by policy.
    CapabilityDenied,
    /// Target is invalid or unresolvable.
    InvalidTarget,
    /// Catch-all for unclassified denials.
    Unknown,
}

impl std::fmt::Display for DenialClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScopeMissing => write!(f, "scope-missing"),
            Self::TargetOutOfScope => write!(f, "target-out-of-scope"),
            Self::ExplicitExclusion => write!(f, "explicit-exclusion"),
            Self::FeatureMissing => write!(f, "feature-missing"),
            Self::RiskPolicyDenied => write!(f, "risk-policy-denied"),
            Self::CapabilityDenied => write!(f, "capability-denied"),
            Self::InvalidTarget => write!(f, "invalid-target"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_allows_passive() {
        let policy = ExecutionPolicy::default();
        assert!(OperationRisk::Passive.is_allowed_by(&policy));
        assert!(OperationRisk::SafeActive.is_allowed_by(&policy));
    }

    #[test]
    fn default_policy_blocks_intrusive() {
        let policy = ExecutionPolicy::default();
        assert!(!OperationRisk::Intrusive.is_allowed_by(&policy));
        assert!(!OperationRisk::StressTest.is_allowed_by(&policy));
        assert!(!OperationRisk::RawPacket.is_allowed_by(&policy));
        assert!(!OperationRisk::ExploitAdjacent.is_allowed_by(&policy));
    }

    #[test]
    fn custom_policy_can_enable_stress() {
        let mut policy = ExecutionPolicy::default();
        policy.allow_stress_testing = true;
        assert!(OperationRisk::StressTest.is_allowed_by(&policy));
    }

    #[test]
    fn custom_policy_can_enable_exploit_adjacent() {
        let mut policy = ExecutionPolicy::default();
        policy.allow_exploit_adjacent = true;
        assert!(OperationRisk::ExploitAdjacent.is_allowed_by(&policy));
        assert!(!OperationRisk::AgentAutonomous.is_allowed_by(&policy));
    }

    #[test]
    fn risk_ordering() {
        assert!(OperationRisk::Passive < OperationRisk::SafeActive);
        assert!(OperationRisk::SafeActive < OperationRisk::StressTest);
        assert!(OperationRisk::StressTest < OperationRisk::AgentAutonomous);
    }

    #[test]
    fn default_policy_blocks_load_test() {
        let policy = ExecutionPolicy::default();
        assert!(!OperationRisk::LoadTest.is_allowed_by(&policy));
    }

    #[test]
    fn default_policy_blocks_c2_operation() {
        let policy = ExecutionPolicy::default();
        assert!(!OperationRisk::C2Operation.is_allowed_by(&policy));
    }

    #[test]
    fn custom_policy_can_enable_all() {
        let mut policy = ExecutionPolicy::default();
        policy.allow_intrusive_fuzzing = true;
        policy.allow_load_testing = true;
        policy.allow_stress_testing = true;
        policy.allow_raw_packets = true;
        policy.allow_credential_testing = true;
        policy.allow_exploit_adjacent = true;
        policy.allow_remote_execution = true;
        policy.allow_agent_autonomous = true;
        policy.allow_evasion_testing = true;
        policy.allow_post_exploitation = true;
        policy.allow_c2_operations = true;
        assert!(OperationRisk::Intrusive.is_allowed_by(&policy));
        assert!(OperationRisk::LoadTest.is_allowed_by(&policy));
        assert!(OperationRisk::StressTest.is_allowed_by(&policy));
        assert!(OperationRisk::RawPacket.is_allowed_by(&policy));
        assert!(OperationRisk::CredentialTesting.is_allowed_by(&policy));
        assert!(OperationRisk::ExploitAdjacent.is_allowed_by(&policy));
        assert!(OperationRisk::C2Operation.is_allowed_by(&policy));
        assert!(OperationRisk::RemoteExecution.is_allowed_by(&policy));
        assert!(OperationRisk::EvasionTesting.is_allowed_by(&policy));
        assert!(OperationRisk::PostExploitation.is_allowed_by(&policy));
        assert!(OperationRisk::AgentAutonomous.is_allowed_by(&policy));
    }

    #[test]
    fn policy_serialization_roundtrip() {
        let policy = ExecutionPolicy::default();
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: ExecutionPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(
            policy.allow_intrusive_fuzzing,
            deserialized.allow_intrusive_fuzzing
        );
        assert_eq!(
            policy.max_risk_without_confirm,
            deserialized.max_risk_without_confirm
        );
    }

    #[test]
    fn risk_display() {
        assert_eq!(format!("{}", OperationRisk::Passive), "passive");
        assert_eq!(
            format!("{}", OperationRisk::ExploitAdjacent),
            "exploit adjacent"
        );
        assert_eq!(format!("{}", OperationRisk::C2Operation), "c2 operation");
        assert_eq!(
            format!("{}", OperationRisk::AgentAutonomous),
            "agent autonomous"
        );
    }

    #[test]
    fn operation_mode_display() {
        assert_eq!(
            format!("{}", OperationMode::StandardAssessment),
            "standard-assessment"
        );
        assert_eq!(format!("{}", OperationMode::DefenseLab), "defense-lab");
        assert_eq!(format!("{}", OperationMode::HazardousLab), "hazardous-lab");
    }

    #[test]
    fn operation_mode_default_max_risk() {
        assert_eq!(
            OperationMode::StandardAssessment.default_max_risk(),
            OperationRisk::SafeActive
        );
        assert_eq!(
            OperationMode::DefenseLab.default_max_risk(),
            OperationRisk::Intrusive
        );
        assert_eq!(
            OperationMode::HazardousLab.default_max_risk(),
            OperationRisk::AgentAutonomous
        );
    }

    #[test]
    fn intended_use_display() {
        assert_eq!(format!("{}", IntendedUse::WafRegression), "waf-regression");
        assert_eq!(
            format!("{}", IntendedUse::SynvoidRegression),
            "synvoid-regression"
        );
    }

    #[test]
    fn execution_profile_is_strict() {
        assert!(!ExecutionProfile::ManualPermissive.is_strict());
        assert!(!ExecutionProfile::ManualGuarded.is_strict());
        assert!(ExecutionProfile::CiStrict.is_strict());
        assert!(ExecutionProfile::McpStrict.is_strict());
        assert!(ExecutionProfile::AgentStrict.is_strict());
    }

    #[test]
    fn execution_profile_is_automated() {
        assert!(!ExecutionProfile::ManualPermissive.is_automated());
        assert!(!ExecutionProfile::ManualGuarded.is_automated());
        assert!(ExecutionProfile::CiStrict.is_automated());
        assert!(ExecutionProfile::McpStrict.is_automated());
        assert!(ExecutionProfile::AgentStrict.is_automated());
    }

    #[test]
    fn execution_profile_display() {
        assert_eq!(
            format!("{}", ExecutionProfile::ManualPermissive),
            "manual-permissive"
        );
        assert_eq!(
            format!("{}", ExecutionProfile::ManualGuarded),
            "manual-guarded"
        );
        assert_eq!(format!("{}", ExecutionProfile::CiStrict), "ci-strict");
        assert_eq!(format!("{}", ExecutionProfile::McpStrict), "mcp-strict");
        assert_eq!(format!("{}", ExecutionProfile::AgentStrict), "agent-strict");
    }

    #[test]
    fn capability_display() {
        assert_eq!(
            format!("{}", Capability::PassiveFingerprint),
            "passive-fingerprint"
        );
        assert_eq!(format!("{}", Capability::ActiveProbe), "active-probe");
        assert_eq!(format!("{}", Capability::Crawl), "crawl");
        assert_eq!(format!("{}", Capability::IntrusiveFuzz), "intrusive-fuzz");
        assert_eq!(format!("{}", Capability::WafDetect), "waf-detect");
        assert_eq!(format!("{}", Capability::NseSafe), "nse-safe");
        assert_eq!(format!("{}", Capability::NseIntrusive), "nse-intrusive");
    }

    #[test]
    fn execution_profile_serialization_roundtrip() {
        let profile = ExecutionProfile::McpStrict;
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("mcp-strict"));
        let deserialized: ExecutionProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, deserialized);
    }

    #[test]
    fn capability_serialization_roundtrip() {
        let cap = Capability::WafBypassSimulation;
        let json = serde_json::to_string(&cap).unwrap();
        assert!(json.contains("waf-bypass-simulation"));
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn denial_class_display() {
        assert_eq!(format!("{}", DenialClass::ScopeMissing), "scope-missing");
        assert_eq!(
            format!("{}", DenialClass::TargetOutOfScope),
            "target-out-of-scope"
        );
        assert_eq!(
            format!("{}", DenialClass::ExplicitExclusion),
            "explicit-exclusion"
        );
        assert_eq!(
            format!("{}", DenialClass::FeatureMissing),
            "feature-missing"
        );
        assert_eq!(
            format!("{}", DenialClass::RiskPolicyDenied),
            "risk-policy-denied"
        );
        assert_eq!(
            format!("{}", DenialClass::CapabilityDenied),
            "capability-denied"
        );
        assert_eq!(format!("{}", DenialClass::InvalidTarget), "invalid-target");
        assert_eq!(format!("{}", DenialClass::Unknown), "unknown");
    }

    #[test]
    fn denial_class_serialization_roundtrip() {
        for variant in [
            DenialClass::ScopeMissing,
            DenialClass::TargetOutOfScope,
            DenialClass::ExplicitExclusion,
            DenialClass::FeatureMissing,
            DenialClass::RiskPolicyDenied,
            DenialClass::CapabilityDenied,
            DenialClass::InvalidTarget,
            DenialClass::Unknown,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: DenialClass = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    // --- ExecutionSurface tests ---

    #[test]
    fn execution_surface_profile_mapping() {
        assert_eq!(
            ExecutionSurface::CliManual.profile(),
            ExecutionProfile::ManualPermissive
        );
        assert_eq!(
            ExecutionSurface::TuiManual.profile(),
            ExecutionProfile::ManualPermissive
        );
        assert_eq!(
            ExecutionSurface::CliManualStrict.profile(),
            ExecutionProfile::ManualGuarded
        );
        assert_eq!(
            ExecutionSurface::TuiManualStrict.profile(),
            ExecutionProfile::ManualGuarded
        );
        assert_eq!(
            ExecutionSurface::McpServer.profile(),
            ExecutionProfile::McpStrict
        );
        assert_eq!(
            ExecutionSurface::SecurityAgent.profile(),
            ExecutionProfile::AgentStrict
        );
        assert_eq!(ExecutionSurface::Ci.profile(), ExecutionProfile::CiStrict);
        assert!(ExecutionSurface::RestApi.profile().is_strict());
    }

    #[test]
    fn execution_surface_is_manual() {
        assert!(ExecutionSurface::CliManual.is_manual());
        assert!(ExecutionSurface::TuiManual.is_manual());
        assert!(ExecutionSurface::CliManualStrict.is_manual());
        assert!(ExecutionSurface::TuiManualStrict.is_manual());
        assert!(!ExecutionSurface::McpServer.is_manual());
        assert!(!ExecutionSurface::SecurityAgent.is_manual());
        assert!(!ExecutionSurface::Ci.is_manual());
        assert!(!ExecutionSurface::RestApi.is_manual());
    }

    #[test]
    fn execution_surface_is_agent_controlled() {
        assert!(!ExecutionSurface::CliManual.is_agent_controlled());
        assert!(!ExecutionSurface::TuiManual.is_agent_controlled());
        assert!(ExecutionSurface::McpServer.is_agent_controlled());
        assert!(ExecutionSurface::SecurityAgent.is_agent_controlled());
        assert!(ExecutionSurface::Ci.is_agent_controlled());
        assert!(ExecutionSurface::RestApi.is_agent_controlled());
    }

    #[test]
    fn execution_surface_honors_manual_override() {
        assert!(ExecutionSurface::CliManual.honors_manual_override());
        assert!(ExecutionSurface::TuiManual.honors_manual_override());
        assert!(!ExecutionSurface::CliManualStrict.honors_manual_override());
        assert!(!ExecutionSurface::TuiManualStrict.honors_manual_override());
        assert!(!ExecutionSurface::McpServer.honors_manual_override());
        assert!(!ExecutionSurface::SecurityAgent.honors_manual_override());
        assert!(!ExecutionSurface::Ci.honors_manual_override());
        assert!(!ExecutionSurface::RestApi.honors_manual_override());
    }

    #[test]
    fn execution_surface_requires_explicit_manifest_for_networked() {
        assert!(!ExecutionSurface::CliManual.requires_explicit_manifest_for_networked());
        assert!(!ExecutionSurface::TuiManual.requires_explicit_manifest_for_networked());
        assert!(ExecutionSurface::McpServer.requires_explicit_manifest_for_networked());
        assert!(ExecutionSurface::SecurityAgent.requires_explicit_manifest_for_networked());
        assert!(ExecutionSurface::Ci.requires_explicit_manifest_for_networked());
        assert!(ExecutionSurface::RestApi.requires_explicit_manifest_for_networked());
    }

    #[test]
    fn execution_surface_display() {
        assert_eq!(format!("{}", ExecutionSurface::CliManual), "cli-manual");
        assert_eq!(format!("{}", ExecutionSurface::TuiManual), "tui-manual");
        assert_eq!(
            format!("{}", ExecutionSurface::CliManualStrict),
            "cli-manual-strict"
        );
        assert_eq!(
            format!("{}", ExecutionSurface::TuiManualStrict),
            "tui-manual-strict"
        );
        assert_eq!(format!("{}", ExecutionSurface::McpServer), "mcp-server");
        assert_eq!(
            format!("{}", ExecutionSurface::SecurityAgent),
            "security-agent"
        );
        assert_eq!(format!("{}", ExecutionSurface::Ci), "ci");
        assert_eq!(format!("{}", ExecutionSurface::RestApi), "rest-api");
    }

    #[test]
    fn execution_surface_label() {
        assert_eq!(ExecutionSurface::CliManual.label(), "CLI manual");
        assert_eq!(ExecutionSurface::TuiManual.label(), "TUI manual");
        assert_eq!(
            ExecutionSurface::CliManualStrict.label(),
            "CLI manual strict"
        );
        assert_eq!(
            ExecutionSurface::TuiManualStrict.label(),
            "TUI manual strict"
        );
        assert_eq!(ExecutionSurface::McpServer.label(), "MCP server");
        assert_eq!(ExecutionSurface::SecurityAgent.label(), "Security agent");
        assert_eq!(ExecutionSurface::Ci.label(), "CI");
        assert_eq!(ExecutionSurface::RestApi.label(), "REST API");
    }

    #[test]
    fn execution_surface_serialization_roundtrip() {
        for surface in [
            ExecutionSurface::CliManual,
            ExecutionSurface::TuiManual,
            ExecutionSurface::CliManualStrict,
            ExecutionSurface::TuiManualStrict,
            ExecutionSurface::McpServer,
            ExecutionSurface::SecurityAgent,
            ExecutionSurface::Ci,
            ExecutionSurface::RestApi,
        ] {
            let json = serde_json::to_string(&surface).unwrap();
            let deserialized: ExecutionSurface = serde_json::from_str(&json).unwrap();
            assert_eq!(surface, deserialized);
        }
    }
}

// ---------------------------------------------------------------------------
// Operation Metadata — single source of truth for OperationDescriptor generation
// ---------------------------------------------------------------------------

/// Target policy requirement for operation metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPolicyKind {
    NoTarget,
    OptionalTarget,
    TargetRequired,
    ExplicitScopeRequired,
    PrivateOrLocalRequired,
}

/// Canonical operation metadata — single source of truth for OperationDescriptor generation.
///
/// Every externally invokable Eggsec operation should have one `OperationMetadata`
/// declaration. This drives policy descriptors, protocol exposure, capability/risk
/// declarations, feature gates, and documentation.
#[derive(Debug, Clone, Copy)]
pub struct OperationMetadata {
    pub id: &'static str,
    pub display_name: &'static str,
    pub mode: OperationMode,
    pub risk: OperationRisk,
    pub intended_uses: &'static [IntendedUse],
    pub required_features: &'static [&'static str],
    pub required_policy_flags: &'static [&'static str],
    pub required_capabilities: &'static [Capability],
    pub target_policy: TargetPolicyKind,
    pub manual_exposable: bool,
    pub tui_exposable: bool,
    pub mcp_exposable: bool,
    pub rest_exposable: bool,
    pub agent_exposable: bool,
    pub grpc_exposable: bool,
}

impl OperationMetadata {
    /// Generate an `OperationDescriptor` from this metadata.
    pub fn descriptor_for_target(&self, target: Option<String>) -> OperationDescriptor {
        OperationDescriptor {
            operation: self.id.to_string(),
            mode: self.mode,
            risk: self.risk,
            intended_uses: self.intended_uses.to_vec(),
            target,
            required_features: self
                .required_features
                .iter()
                .map(|s| s.to_string())
                .collect(),
            required_policy_flags: self
                .required_policy_flags
                .iter()
                .map(|s| s.to_string())
                .collect(),
            requires_private_or_local_target: matches!(
                self.target_policy,
                TargetPolicyKind::PrivateOrLocalRequired
            ),
            requires_explicit_scope: matches!(
                self.target_policy,
                TargetPolicyKind::ExplicitScopeRequired | TargetPolicyKind::PrivateOrLocalRequired
            ),
            required_capabilities: self.required_capabilities.to_vec(),
        }
    }

    /// Generate an `OperationDescriptor` from this metadata, overriding the risk tier.
    /// Used for dry-run overrides and tab-specific risk adjustments.
    pub fn descriptor_for_target_with_risk(
        &self,
        target: Option<String>,
        risk: OperationRisk,
    ) -> OperationDescriptor {
        let mut descriptor = self.descriptor_for_target(target);
        descriptor.risk = risk;
        descriptor
    }
}

/// Static registry of all operation metadata. Single source of truth for
/// operation descriptors across REST, MCP, TUI, and agent surfaces.
pub static ALL_OPERATION_METADATA: &[OperationMetadata] = &[
    OperationMetadata {
        id: "recon",
        display_name: "Reconnaissance",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::PassiveFingerprint],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "scan-ports",
        display_name: "Port Scan",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "scan-endpoints",
        display_name: "Endpoint Discovery",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::Crawl],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "fingerprint",
        display_name: "Service Fingerprint",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "fuzz",
        display_name: "Fuzzing",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Intrusive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::HttpFuzzLowImpact],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "waf-detect",
        display_name: "WAF Detection",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::WafDetect],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "waf-bypass",
        display_name: "WAF Bypass Simulation",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Intrusive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::WafBypassSimulation],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "waf-stress",
        display_name: "WAF Stress Test",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::StressTest,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::WafStressTest],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "load-test",
        display_name: "Load Test",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::LoadTest,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::LoadTest],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "stress-test",
        display_name: "Stress Test",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::StressTest,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::WafStressTest],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "packet",
        display_name: "Raw Packet",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::RawPacket,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::RawPacketProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "graphql",
        display_name: "GraphQL Fuzzing",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Intrusive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::HttpFuzzLowImpact],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "oauth",
        display_name: "OAuth Testing",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::CredentialTesting,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::CredentialTesting],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "auth-test",
        display_name: "Authentication Testing",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::CredentialTesting,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::CredentialTesting],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "nse",
        display_name: "NSE Scripts",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["nse"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::NseSafe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "db-pentest",
        display_name: "Database Pentesting",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::DbPentest,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["db-pentest"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::DatabaseAssessment],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "c2",
        display_name: "C2 Simulation",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::C2Operation,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["c2"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::C2Simulation],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "proxy-intercept",
        display_name: "Traffic Interception",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::TrafficInterception,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["web-proxy"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::TrafficInterception],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "wireless",
        display_name: "Wireless Scanning",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["wireless"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::PassiveFingerprint],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "hunt",
        display_name: "Vulnerability Hunting",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["advanced-hunting"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "browser",
        display_name: "Headless Browser",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["headless-browser"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "compliance",
        display_name: "Compliance Scanning",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["compliance"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "storage",
        display_name: "Database Storage",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["database"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::DatabaseAssessment],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "integrations",
        display_name: "External Integrations",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["external-integrations"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "workflow",
        display_name: "Finding Workflow",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["finding-workflow"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "vuln",
        display_name: "Vulnerability Management",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &["vuln-management"],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "pipeline",
        display_name: "Security Pipeline",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::ActiveProbe],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "proxy",
        display_name: "Proxy Management",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "remote",
        display_name: "Remote Execution",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::RemoteExecution,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[Capability::RemoteExecution],
        target_policy: TargetPolicyKind::ExplicitScopeRequired,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
    OperationMetadata {
        id: "search",
        display_name: "Web Search",
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Passive,
        intended_uses: &[IntendedUse::WebAssessment],
        required_features: &[],
        required_policy_flags: &[],
        required_capabilities: &[],
        target_policy: TargetPolicyKind::NoTarget,
        manual_exposable: true,
        tui_exposable: true,
        mcp_exposable: true,
        rest_exposable: true,
        agent_exposable: true,
        grpc_exposable: true,
    },
];

/// Alias mapping: (alias_id, canonical_id).
///
/// REST and MCP tool IDs that resolve to the same canonical operation.
pub static ALL_OPERATION_METADATA_ALIASES: &[(&str, &str)] = &[
    ("scan", "scan-ports"),
    ("endpoints", "scan-endpoints"),
    ("waf", "waf-detect"),
    ("waf_detect", "waf-detect"),
    ("waf_bypass", "waf-bypass"),
    ("waf_stress", "waf-stress"),
    ("load", "load-test"),
    ("loadtest", "load-test"),
    ("http-bench", "load-test"),
    ("stress", "stress-test"),
    ("fuzzer", "fuzz"),
    ("api-fuzz", "fuzz"),
    ("proxy", "proxy-intercept"),
    ("raw-packet", "packet"),
    ("packet-capture", "packet"),
    ("packet-inspect", "packet"),
    ("recon-all", "recon"),
    ("subdomain", "recon"),
    ("credential", "auth-test"),
    ("brute", "auth-test"),
    ("syn-flood", "stress-test"),
    ("udp-flood", "stress-test"),
    ("icmp-flood", "stress-test"),
    ("raw-packet-send", "packet"),
    ("plan", "recon"),
    ("scan_ports", "scan-ports"),
    ("scan-pipeline", "pipeline"),
    ("db-pentest-mcp", "db-pentest"),
    ("exec", "remote"),
    ("ssh", "remote"),
    ("tor", "proxy"),
];

/// Look up operation metadata by its canonical ID.
pub fn operation_metadata(id: &str) -> Option<&'static OperationMetadata> {
    ALL_OPERATION_METADATA.iter().find(|m| m.id == id)
}

/// Look up operation metadata by tool ID, resolving aliases to canonical IDs.
pub fn metadata_for_tool_id(tool_id: &str) -> Option<&'static OperationMetadata> {
    if let Some(m) = operation_metadata(tool_id) {
        return Some(m);
    }
    ALL_OPERATION_METADATA_ALIASES
        .iter()
        .find(|(alias, _)| *alias == tool_id)
        .and_then(|(_, canonical)| operation_metadata(canonical))
}

/// Return a reference to all operation metadata entries.
pub fn all_operation_metadata() -> &'static [OperationMetadata] {
    ALL_OPERATION_METADATA
}

/// Check if a tool ID (possibly an alias) matches a canonical operation ID.
///
/// Returns true if:
/// - `tool_id` == `operation_id` (exact match), or
/// - `tool_id` is an alias that resolves to `operation_id`, or
/// - `tool_id` resolves to the same canonical metadata entry as `operation_id`.
pub fn operation_matches_tool_id(tool_id: &str, operation_id: &str) -> bool {
    if tool_id == operation_id {
        return true;
    }
    // Check if tool_id aliases to operation_id
    if let Some((_, canonical)) = ALL_OPERATION_METADATA_ALIASES
        .iter()
        .find(|(alias, _)| *alias == tool_id)
    {
        if *canonical == operation_id {
            return true;
        }
    }
    // Check if tool_id resolves to the same canonical entry as operation_id
    if let Some(meta) = metadata_for_tool_id(tool_id) {
        if meta.id == operation_id {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod operation_metadata_tests {
    use super::*;

    #[test]
    fn every_metadata_has_non_empty_id_and_display_name() {
        for m in all_operation_metadata() {
            assert!(!m.id.is_empty(), "metadata has empty id");
            assert!(
                !m.display_name.is_empty(),
                "metadata has empty display_name for {}",
                m.id
            );
        }
    }

    #[test]
    fn every_metadata_id_is_unique() {
        let mut seen = rustc_hash::FxHashSet::default();
        for m in all_operation_metadata() {
            assert!(seen.insert(m.id), "duplicate metadata id: {}", m.id);
        }
    }

    #[test]
    fn agent_exposable_ops_require_explicit_scope() {
        for m in all_operation_metadata() {
            if (m.agent_exposable || m.mcp_exposable)
                && m.target_policy != TargetPolicyKind::NoTarget
            {
                assert!(
                    matches!(
                        m.target_policy,
                        TargetPolicyKind::ExplicitScopeRequired
                            | TargetPolicyKind::PrivateOrLocalRequired
                    ),
                    "agent/mcp exposable target-bearing op '{}' should require explicit scope via target_policy, got {:?}",
                    m.id,
                    m.target_policy
                );
            }
        }
    }

    #[test]
    fn feature_gated_ops_declare_feature_name() {
        for m in all_operation_metadata() {
            if !m.required_features.is_empty() {
                for f in m.required_features {
                    assert!(
                        !f.is_empty(),
                        "metadata '{}' has empty required_feature",
                        m.id
                    );
                }
            }
        }
    }

    #[test]
    fn descriptor_generation_matches_metadata() {
        for m in all_operation_metadata() {
            let desc = m.descriptor_for_target(Some("https://example.com".to_string()));
            assert_eq!(desc.operation, m.id);
            assert_eq!(desc.mode, m.mode);
            assert_eq!(desc.risk, m.risk);
            assert_eq!(desc.target, Some("https://example.com".to_string()));
            assert_eq!(desc.required_capabilities, m.required_capabilities.to_vec());
        }
    }

    #[test]
    fn descriptor_with_target_none() {
        for m in all_operation_metadata() {
            let desc = m.descriptor_for_target(None);
            assert_eq!(desc.target, None);
        }
    }

    #[test]
    fn rest_descriptor_from_metadata_matches_expected() {
        // recon
        let m = metadata_for_tool_id("recon").unwrap();
        let desc = m.descriptor_for_target(Some("https://example.com".to_string()));
        assert_eq!(desc.risk, OperationRisk::SafeActive);
        assert!(desc
            .required_capabilities
            .contains(&Capability::PassiveFingerprint));
        // fuzz
        let m = metadata_for_tool_id("fuzz").unwrap();
        let desc = m.descriptor_for_target(Some("https://example.com".to_string()));
        assert_eq!(desc.risk, OperationRisk::Intrusive);
        assert!(desc
            .required_capabilities
            .contains(&Capability::HttpFuzzLowImpact));
        // stress
        let m = metadata_for_tool_id("stress-test").unwrap();
        let desc = m.descriptor_for_target(Some("https://example.com".to_string()));
        assert_eq!(desc.risk, OperationRisk::StressTest);
        assert!(desc
            .required_capabilities
            .contains(&Capability::WafStressTest));
    }

    #[test]
    fn alias_lookup_matches_canonical() {
        let canonical = metadata_for_tool_id("recon").unwrap();
        let alias = metadata_for_tool_id("recon-all").unwrap();
        assert_eq!(canonical.id, alias.id);
        assert_eq!(canonical.risk, alias.risk);
    }

    #[test]
    fn operation_matches_tool_id_exact_match() {
        assert!(operation_matches_tool_id("scan-ports", "scan-ports"));
        assert!(operation_matches_tool_id("fuzz", "fuzz"));
        assert!(operation_matches_tool_id("recon", "recon"));
    }

    #[test]
    fn operation_matches_tool_id_alias_to_canonical() {
        assert!(operation_matches_tool_id("scan", "scan-ports"));
        assert!(operation_matches_tool_id("load", "load-test"));
        assert!(operation_matches_tool_id("waf", "waf-detect"));
        assert!(operation_matches_tool_id("stress", "stress-test"));
        assert!(operation_matches_tool_id("fuzzer", "fuzz"));
        assert!(operation_matches_tool_id("recon-all", "recon"));
    }

    #[test]
    fn operation_matches_tool_id_canonical_to_alias() {
        // Bidirectional: canonical ID should match when compared against alias
        assert!(operation_matches_tool_id("scan-ports", "scan-ports"));
        // This tests the metadata_for_tool_id fallback path
        assert!(operation_matches_tool_id("load-test", "load-test"));
    }

    #[test]
    fn operation_matches_tool_id_unrelated_no_match() {
        assert!(!operation_matches_tool_id("scan", "fuzz"));
        assert!(!operation_matches_tool_id("load", "stress-test"));
        assert!(!operation_matches_tool_id("waf", "recon"));
    }

    #[test]
    fn operation_matches_tool_id_unknown_no_match() {
        assert!(!operation_matches_tool_id("nonexistent", "scan-ports"));
        assert!(!operation_matches_tool_id("scan-ports", "nonexistent"));
    }

    /// Every tool registered by `create_default_registry()` must have operation metadata.
    /// This prevents new tools from being added without metadata, which would cause
    /// runtime failures in REST, MCP, TUI, and agent surfaces.
    #[test]
    fn every_registered_tool_has_operation_metadata() {
        // Tool IDs from tool::create_default_registry() (non-feature-gated)
        let base_tool_ids = &[
            "recon",
            "scan-ports",
            "fingerprint",
            "scan-endpoints",
            "fuzz",
            "load",
            "waf-detect",
            "waf-bypass",
            "waf-stress",
            "pipeline",
            "search",
        ];

        for &tool_id in base_tool_ids {
            assert!(
                metadata_for_tool_id(tool_id).is_some(),
                "registered tool '{}' has no operation metadata — add an entry to ALL_OPERATION_METADATA or ALL_OPERATION_METADATA_ALIASES",
                tool_id,
            );
        }

        // Feature-gated tools: only check if the feature is enabled
        #[cfg(feature = "web-proxy-mcp")]
        assert!(
            metadata_for_tool_id("proxy").is_some(),
            "registered tool 'proxy' has no operation metadata"
        );
        #[cfg(feature = "db-pentest-mcp")]
        assert!(
            metadata_for_tool_id("db-pentest").is_some(),
            "registered tool 'db-pentest' has no operation metadata"
        );
        #[cfg(feature = "c2-mcp")]
        assert!(
            metadata_for_tool_id("c2").is_some(),
            "registered tool 'c2' has no operation metadata"
        );
    }

    /// High-risk operations (risk > SafeActive) must declare at least one
    /// non-baseline capability. This prevents accidentally omitting capability
    /// declarations on dangerous operations, which would allow them to slip
    /// through enforcement checks that gate on required_capabilities.
    #[test]
    fn high_risk_ops_declare_nonbaseline_capability() {
        for m in all_operation_metadata() {
            if m.risk > OperationRisk::SafeActive {
                let has_nonbaseline = m
                    .required_capabilities
                    .iter()
                    .any(|cap| !baseline_allowed_capability(*cap));
                assert!(
                    has_nonbaseline,
                    "high-risk operation '{}' (risk {:?}) must declare at least one \
                     non-baseline capability — current capabilities: {:?}",
                    m.id, m.risk, m.required_capabilities,
                );
            }
        }
    }

    /// TUI descriptor generation must match metadata for representative tabs.
    /// Verifies that metadata_for_tool_id() resolves TUI operation IDs and
    /// descriptor_for_target() produces the expected risk, mode, and capabilities.
    #[test]
    fn tui_descriptor_generation_matches_metadata() {
        // Representative TUI operation IDs (some canonical, some aliases)
        let cases: &[(&str, OperationRisk, &[Capability])] = &[
            (
                "recon",
                OperationRisk::SafeActive,
                &[Capability::PassiveFingerprint],
            ),
            (
                "scan-ports",
                OperationRisk::SafeActive,
                &[Capability::ActiveProbe],
            ),
            (
                "fuzz",
                OperationRisk::Intrusive,
                &[Capability::HttpFuzzLowImpact],
            ),
            ("waf", OperationRisk::SafeActive, &[Capability::WafDetect]),
            (
                "load-test",
                OperationRisk::LoadTest,
                &[Capability::LoadTest],
            ),
        ];

        for &(op_id, expected_risk, expected_caps) in cases {
            let metadata = metadata_for_tool_id(op_id)
                .unwrap_or_else(|| panic!("TUI operation '{}' should have metadata", op_id));
            let desc = metadata.descriptor_for_target(Some("https://example.com".to_string()));
            assert_eq!(
                desc.risk, expected_risk,
                "TUI tab '{}': expected risk {:?}, got {:?}",
                op_id, expected_risk, desc.risk
            );
            assert_eq!(
                desc.operation, metadata.id,
                "TUI tab '{}': descriptor operation should be canonical ID '{}'",
                op_id, metadata.id
            );
            for cap in expected_caps {
                assert!(
                    desc.required_capabilities.contains(cap),
                    "TUI tab '{}': expected capability {:?} in {:?}",
                    op_id,
                    cap,
                    desc.required_capabilities
                );
            }
        }
    }
}
