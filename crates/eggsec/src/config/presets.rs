use serde::{Deserialize, Serialize};

use super::{IntendedUse, OperationMode, OperationRisk};

/// A defense-lab preset that defines constraints for a specific lab workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseLabPreset {
    pub name: String,
    pub description: String,
    pub operation_mode: OperationMode,
    pub max_risk: OperationRisk,
    pub intended_uses: Vec<IntendedUse>,
    pub default_concurrency: usize,
    pub max_duration_secs: u64,
    pub max_requests: Option<u64>,
    pub max_packets: Option<u64>,
    pub max_payloads: Option<usize>,
    pub dns_resolution_allowed: bool,
    pub raw_sockets_allowed: bool,
    pub external_targets_allowed: bool,
    pub localhost_or_private_required: bool,
    pub output_format: String,
}

impl DefenseLabPreset {
    pub fn synvoid_local() -> Self {
        Self {
            name: "synvoid-local".to_string(),
            description: "Local Synvoid WAF and protocol validation".to_string(),
            operation_mode: OperationMode::DefenseLab,
            max_risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::SynvoidRegression, IntendedUse::WafRegression],
            default_concurrency: 10,
            max_duration_secs: 300,
            max_requests: Some(10_000),
            max_packets: None,
            max_payloads: Some(500),
            dns_resolution_allowed: false,
            raw_sockets_allowed: false,
            external_targets_allowed: false,
            localhost_or_private_required: true,
            output_format: "json".to_string(),
        }
    }

    pub fn synvoid_waf_regression() -> Self {
        Self {
            name: "synvoid-waf-regression".to_string(),
            description: "WAF payload and evasion-resistance regression for Synvoid"
                .to_string(),
            operation_mode: OperationMode::DefenseLab,
            max_risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::WafRegression],
            default_concurrency: 20,
            max_duration_secs: 600,
            max_requests: Some(50_000),
            max_packets: None,
            max_payloads: Some(2_000),
            dns_resolution_allowed: false,
            raw_sockets_allowed: false,
            external_targets_allowed: false,
            localhost_or_private_required: true,
            output_format: "json".to_string(),
        }
    }

    pub fn synvoid_protocol_edge() -> Self {
        Self {
            name: "synvoid-protocol-edge".to_string(),
            description: "Malformed protocol and TCP/TLS/HTTP edge behavior validation"
                .to_string(),
            operation_mode: OperationMode::DefenseLab,
            max_risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::ProtocolEdgeValidation],
            default_concurrency: 5,
            max_duration_secs: 120,
            max_requests: Some(1_000),
            max_packets: Some(5_000),
            max_payloads: Some(100),
            dns_resolution_allowed: false,
            raw_sockets_allowed: true,
            external_targets_allowed: false,
            localhost_or_private_required: true,
            output_format: "json".to_string(),
        }
    }

    pub fn distributed_system_smoke() -> Self {
        Self {
            name: "distributed-system-smoke".to_string(),
            description: "Lightweight distributed system resilience smoke test".to_string(),
            operation_mode: OperationMode::DefenseLab,
            max_risk: OperationRisk::LoadTest,
            intended_uses: vec![IntendedUse::DistributedSystemStress],
            default_concurrency: 5,
            max_duration_secs: 60,
            max_requests: Some(500),
            max_packets: None,
            max_payloads: None,
            dns_resolution_allowed: true,
            raw_sockets_allowed: false,
            external_targets_allowed: false,
            localhost_or_private_required: true,
            output_format: "json".to_string(),
        }
    }

    pub fn distributed_system_stress() -> Self {
        Self {
            name: "distributed-system-stress".to_string(),
            description: "Distributed system stress validation with controlled load".to_string(),
            operation_mode: OperationMode::HazardousLab,
            max_risk: OperationRisk::StressTest,
            intended_uses: vec![IntendedUse::DistributedSystemStress],
            default_concurrency: 50,
            max_duration_secs: 300,
            max_requests: Some(100_000),
            max_packets: None,
            max_payloads: None,
            dns_resolution_allowed: true,
            raw_sockets_allowed: true,
            external_targets_allowed: false,
            localhost_or_private_required: true,
            output_format: "json".to_string(),
        }
    }

    pub fn waf_regression_safe() -> Self {
        Self {
            name: "waf-regression-safe".to_string(),
            description: "Safe WAF regression testing with conservative budgets".to_string(),
            operation_mode: OperationMode::DefenseLab,
            max_risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WafRegression],
            default_concurrency: 5,
            max_duration_secs: 120,
            max_requests: Some(5_000),
            max_packets: None,
            max_payloads: Some(200),
            dns_resolution_allowed: false,
            raw_sockets_allowed: false,
            external_targets_allowed: false,
            localhost_or_private_required: true,
            output_format: "json".to_string(),
        }
    }

    pub fn waf_regression_intrusive() -> Self {
        Self {
            name: "waf-regression-intrusive".to_string(),
            description: "Intrusive WAF regression with full payload families".to_string(),
            operation_mode: OperationMode::DefenseLab,
            max_risk: OperationRisk::Intrusive,
            intended_uses: vec![IntendedUse::WafRegression],
            default_concurrency: 20,
            max_duration_secs: 600,
            max_requests: Some(50_000),
            max_packets: None,
            max_payloads: Some(2_000),
            dns_resolution_allowed: false,
            raw_sockets_allowed: false,
            external_targets_allowed: false,
            localhost_or_private_required: true,
            output_format: "json".to_string(),
        }
    }

    /// Get all built-in presets.
    pub fn built_in() -> Vec<Self> {
        vec![
            Self::synvoid_local(),
            Self::synvoid_waf_regression(),
            Self::synvoid_protocol_edge(),
            Self::distributed_system_smoke(),
            Self::distributed_system_stress(),
            Self::waf_regression_safe(),
            Self::waf_regression_intrusive(),
        ]
    }

    /// Find a preset by name.
    pub fn find(name: &str) -> Option<Self> {
        Self::built_in().into_iter().find(|p| p.name == name)
    }

    /// List all preset names.
    pub fn list_names() -> Vec<&'static str> {
        vec![
            "synvoid-local",
            "synvoid-waf-regression",
            "synvoid-protocol-edge",
            "distributed-system-smoke",
            "distributed-system-stress",
            "waf-regression-safe",
            "waf-regression-intrusive",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_presets_exist() {
        let presets = DefenseLabPreset::built_in();
        assert_eq!(presets.len(), 7);
    }

    #[test]
    fn find_preset_by_name() {
        assert!(DefenseLabPreset::find("synvoid-local").is_some());
        assert!(DefenseLabPreset::find("nonexistent").is_none());
    }

    #[test]
    fn all_presets_require_private_scope() {
        for preset in DefenseLabPreset::built_in() {
            assert!(
                preset.localhost_or_private_required,
                "preset {} must require localhost/private",
                preset.name
            );
        }
    }

    #[test]
    fn preset_serialization_roundtrip() {
        let preset = DefenseLabPreset::synvoid_local();
        let json = serde_json::to_string(&preset).unwrap();
        let deserialized: DefenseLabPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(preset.name, deserialized.name);
        assert_eq!(preset.max_risk, deserialized.max_risk);
    }
}
