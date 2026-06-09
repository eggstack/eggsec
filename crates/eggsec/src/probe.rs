//! Shared probe intent and risk vocabulary for security testing profiles.
//!
//! This module defines the canonical enums used across scanner, NSE, WAF,
//! loadtest, and defense-lab profiles to describe what a probe is trying
//! to achieve and what risk tier it belongs to. Profiles compile into
//! probe plans that carry these metadata tags, enabling guardrails,
//! budget requirements, and explicit opt-in behavior.

use serde::{Deserialize, Serialize};

/// Intent categories for security probes.
///
/// This vocabulary is shared across scanner, NSE, WAF, loadtest,
/// and defense-lab profiles to describe what a probe is trying to achieve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeIntent {
    Discovery,
    Fingerprint,
    ServiceValidation,
    WafEvaluation,
    EvasionResistance,
    LoadBearing,
    Stress,
}

/// Risk classification for security probes.
///
/// Used to determine guardrails, budget requirements, and explicit opt-in
/// behavior for different classes of testing activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeRisk {
    Passive,
    SafeActive,
    Intrusive,
    Credentialed,
    Stress,
    ExploitAdjacent,
}

impl ProbeRisk {
    /// Returns a numeric risk level for ordering/comparison.
    ///
    /// Higher values indicate higher risk. Used to enforce risk budgets:
    /// a stage is skipped if its risk level exceeds the profile's budget.
    pub fn risk_level(self) -> u8 {
        match self {
            ProbeRisk::Passive => 0,
            ProbeRisk::SafeActive => 1,
            ProbeRisk::Intrusive => 2,
            ProbeRisk::Credentialed => 3,
            ProbeRisk::Stress => 4,
            ProbeRisk::ExploitAdjacent => 5,
        }
    }

    /// Returns `true` if this risk level requires explicit user opt-in.
    pub fn requires_opt_in(self) -> bool {
        matches!(
            self,
            ProbeRisk::Credentialed
                | ProbeRisk::Intrusive
                | ProbeRisk::Stress
                | ProbeRisk::ExploitAdjacent
        )
    }

    pub fn to_operation_risk(self) -> crate::config::OperationRisk {
        match self {
            ProbeRisk::Passive => crate::config::OperationRisk::Passive,
            ProbeRisk::SafeActive => crate::config::OperationRisk::SafeActive,
            ProbeRisk::Intrusive => crate::config::OperationRisk::Intrusive,
            ProbeRisk::Credentialed => crate::config::OperationRisk::CredentialTesting,
            ProbeRisk::Stress => crate::config::OperationRisk::StressTest,
            ProbeRisk::ExploitAdjacent => crate::config::OperationRisk::ExploitAdjacent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_intent_discovery_serializes_kebab_case() {
        let json = serde_json::to_string(&ProbeIntent::Discovery).unwrap();
        assert_eq!(json, "\"discovery\"");
    }

    #[test]
    fn probe_risk_safe_active_serializes_kebab_case() {
        let json = serde_json::to_string(&ProbeRisk::SafeActive).unwrap();
        assert_eq!(json, "\"safe-active\"");
    }

    #[test]
    fn probe_intent_all_variants_serialize() {
        let cases = &[
            (ProbeIntent::Discovery, "\"discovery\""),
            (ProbeIntent::Fingerprint, "\"fingerprint\""),
            (ProbeIntent::ServiceValidation, "\"service-validation\""),
            (ProbeIntent::WafEvaluation, "\"waf-evaluation\""),
            (ProbeIntent::EvasionResistance, "\"evasion-resistance\""),
            (ProbeIntent::LoadBearing, "\"load-bearing\""),
            (ProbeIntent::Stress, "\"stress\""),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_string(variant).unwrap();
            assert_eq!(json, *expected, "ProbeIntent::{:?} serialization", variant);
        }
    }

    #[test]
    fn probe_risk_all_variants_serialize() {
        let cases = &[
            (ProbeRisk::Passive, "\"passive\""),
            (ProbeRisk::SafeActive, "\"safe-active\""),
            (ProbeRisk::Intrusive, "\"intrusive\""),
            (ProbeRisk::Credentialed, "\"credentialed\""),
            (ProbeRisk::Stress, "\"stress\""),
            (ProbeRisk::ExploitAdjacent, "\"exploit-adjacent\""),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_string(variant).unwrap();
            assert_eq!(json, *expected, "ProbeRisk::{:?} serialization", variant);
        }
    }

    #[test]
    fn probe_risk_levels_are_ordered() {
        assert!(ProbeRisk::Passive.risk_level() < ProbeRisk::SafeActive.risk_level());
        assert!(ProbeRisk::SafeActive.risk_level() < ProbeRisk::Intrusive.risk_level());
        assert!(ProbeRisk::Intrusive.risk_level() < ProbeRisk::Credentialed.risk_level());
        assert!(ProbeRisk::Credentialed.risk_level() < ProbeRisk::Stress.risk_level());
        assert!(ProbeRisk::Stress.risk_level() < ProbeRisk::ExploitAdjacent.risk_level());
    }

    #[test]
    fn probe_risk_requires_opt_in() {
        assert!(!ProbeRisk::Passive.requires_opt_in());
        assert!(!ProbeRisk::SafeActive.requires_opt_in());
        assert!(ProbeRisk::Credentialed.requires_opt_in());
        assert!(ProbeRisk::Intrusive.requires_opt_in());
        assert!(ProbeRisk::Stress.requires_opt_in());
        assert!(ProbeRisk::ExploitAdjacent.requires_opt_in());
    }
}
