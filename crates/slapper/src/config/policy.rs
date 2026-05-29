use serde::{Deserialize, Serialize};

/// Risk tier for operations. Higher variants are more dangerous.
///
/// Used by [`ExecutionPolicy`] to control which operations are permitted
/// without explicit user confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationRisk {
    Passive,
    ActiveScan,
    IntrusiveFuzz,
    LoadTest,
    StressTest,
    RawPacket,
    CredentialTesting,
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
    pub allow_agent_autonomous: bool,

    #[serde(default = "default_max_risk")]
    pub max_risk_without_confirm: OperationRisk,
}

fn default_true() -> bool {
    true
}

fn default_max_risk() -> OperationRisk {
    OperationRisk::ActiveScan
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
            allow_remote_execution: false,
            allow_agent_autonomous: false,
            max_risk_without_confirm: OperationRisk::ActiveScan,
        }
    }
}

impl OperationRisk {
    /// Check if this risk level is allowed by the given policy.
    pub fn is_allowed_by(&self, policy: &ExecutionPolicy) -> bool {
        match self {
            Self::Passive | Self::ActiveScan => true,
            Self::IntrusiveFuzz => policy.allow_intrusive_fuzzing,
            Self::LoadTest => policy.allow_load_testing,
            Self::StressTest => policy.allow_stress_testing,
            Self::RawPacket => policy.allow_raw_packets,
            Self::CredentialTesting => policy.allow_credential_testing,
            Self::RemoteExecution => policy.allow_remote_execution,
            Self::AgentAutonomous => policy.allow_agent_autonomous,
        }
    }
}

impl std::fmt::Display for OperationRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passive => write!(f, "passive"),
            Self::ActiveScan => write!(f, "active scan"),
            Self::IntrusiveFuzz => write!(f, "intrusive fuzzing"),
            Self::LoadTest => write!(f, "load testing"),
            Self::StressTest => write!(f, "stress testing"),
            Self::RawPacket => write!(f, "raw packet"),
            Self::CredentialTesting => write!(f, "credential testing"),
            Self::RemoteExecution => write!(f, "remote execution"),
            Self::AgentAutonomous => write!(f, "agent autonomous"),
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
        assert!(OperationRisk::ActiveScan.is_allowed_by(&policy));
    }

    #[test]
    fn default_policy_blocks_intrusive() {
        let policy = ExecutionPolicy::default();
        assert!(!OperationRisk::IntrusiveFuzz.is_allowed_by(&policy));
        assert!(!OperationRisk::StressTest.is_allowed_by(&policy));
        assert!(!OperationRisk::RawPacket.is_allowed_by(&policy));
    }

    #[test]
    fn custom_policy_can_enable_stress() {
        let mut policy = ExecutionPolicy::default();
        policy.allow_stress_testing = true;
        assert!(OperationRisk::StressTest.is_allowed_by(&policy));
    }

    #[test]
    fn risk_ordering() {
        assert!(OperationRisk::Passive < OperationRisk::ActiveScan);
        assert!(OperationRisk::ActiveScan < OperationRisk::StressTest);
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
        policy.allow_remote_execution = true;
        policy.allow_agent_autonomous = true;
        assert!(OperationRisk::IntrusiveFuzz.is_allowed_by(&policy));
        assert!(OperationRisk::LoadTest.is_allowed_by(&policy));
        assert!(OperationRisk::StressTest.is_allowed_by(&policy));
        assert!(OperationRisk::RawPacket.is_allowed_by(&policy));
        assert!(OperationRisk::CredentialTesting.is_allowed_by(&policy));
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
            format!("{}", OperationRisk::AgentAutonomous),
            "agent autonomous"
        );
    }
}
