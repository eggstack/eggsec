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
    MalformedProtocol,
    Regression,
    Compatibility,
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

/// Metadata describing a specific probe's purpose, risk, and requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeMetadata {
    pub id: String,
    pub name: String,
    pub intent: ProbeIntent,
    pub risk: ProbeRisk,
    pub requires_explicit_scope: bool,
    pub requires_budget: bool,
    pub compatibility_source: Option<String>,
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
    fn probe_metadata_round_trip() {
        let meta = ProbeMetadata {
            id: "tcp-syn-001".to_string(),
            name: "TCP SYN Discovery".to_string(),
            intent: ProbeIntent::Discovery,
            risk: ProbeRisk::SafeActive,
            requires_explicit_scope: false,
            requires_budget: false,
            compatibility_source: Some("nmap:default".to_string()),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: ProbeMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, meta);
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
            (ProbeIntent::MalformedProtocol, "\"malformed-protocol\""),
            (ProbeIntent::Regression, "\"regression\""),
            (ProbeIntent::Compatibility, "\"compatibility\""),
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
}
