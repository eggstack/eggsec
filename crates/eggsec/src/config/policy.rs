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
    ExploitAdjacent,
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

    #[serde(default)]
    pub allow_remote_execution: bool,

    #[serde(default)]
    pub allow_exploit_adjacent: bool,

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
            allow_exploit_adjacent: false,
            allow_remote_execution: false,
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
            Self::ExploitAdjacent => policy.allow_exploit_adjacent,
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
            Self::ExploitAdjacent => write!(f, "exploit adjacent"),
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
        matches!(
            self,
            Self::CiStrict | Self::McpStrict | Self::AgentStrict
        )
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
        }
    }
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
        assert!(OperationRisk::Intrusive.is_allowed_by(&policy));
        assert!(OperationRisk::LoadTest.is_allowed_by(&policy));
        assert!(OperationRisk::StressTest.is_allowed_by(&policy));
        assert!(OperationRisk::RawPacket.is_allowed_by(&policy));
        assert!(OperationRisk::CredentialTesting.is_allowed_by(&policy));
        assert!(OperationRisk::ExploitAdjacent.is_allowed_by(&policy));
        assert!(OperationRisk::RemoteExecution.is_allowed_by(&policy));
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
        assert_eq!(format!("{}", ExecutionProfile::ManualPermissive), "manual-permissive");
        assert_eq!(format!("{}", ExecutionProfile::ManualGuarded), "manual-guarded");
        assert_eq!(format!("{}", ExecutionProfile::CiStrict), "ci-strict");
        assert_eq!(format!("{}", ExecutionProfile::McpStrict), "mcp-strict");
        assert_eq!(format!("{}", ExecutionProfile::AgentStrict), "agent-strict");
    }

    #[test]
    fn capability_display() {
        assert_eq!(format!("{}", Capability::PassiveFingerprint), "passive-fingerprint");
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
}
