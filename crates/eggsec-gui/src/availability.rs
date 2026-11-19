//! Availability resolution: what a live denial taught the GUI.
//!
//! The GUI cannot compute whether an operation is available: the daemon
//! publishes the capability list and the catalog publishes the requirement
//! list, and the engine's own verdict is the only authority on whether a
//! feature is actually on. So the GUI does two things and no more:
//!
//! 1. It joins the **published** facts — the catalog's `required_features`,
//!    the daemon's advertised capabilities, and the daemon's published
//!    enabled-feature set — into a reachability verdict
//!    (`crate::coverage`). That join is a derivation, not a policy.
//! 2. It records what the **engine** said when it refused: a denial naming a
//!    missing feature is a fact about the running daemon that no published
//!    list can contradict. This module owns that second half.
//!
//! Nothing here names an operation, a task kind, or a capability. The
//! messages are matched by shape, not by content.

use std::collections::BTreeMap;

/// What a live denial taught the GUI about an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Denial {
    /// The engine denied the operation because a required feature is off.
    FeatureMissing {
        /// The feature the engine named.
        feature: String,
    },
    /// The engine denied the operation because the operator's execution policy
    /// does not allow its risk tier.
    ///
    /// This is the fact a widened capability list makes visible: the daemon
    /// advertises the capability and has the features, but the operator's
    /// risk policy is the operator's own consent gate, and it says no. The
    /// GUI reports that as such instead of reporting the operation as
    /// available.
    RiskPolicyDenied {
        /// The risk tier the engine named.
        risk: String,
    },
    /// The engine denied the operation for another reason.
    Other {
        /// The denial message, verbatim.
        message: String,
    },
}

impl Denial {
    /// The operator-readable reason this denial gives.
    ///
    /// A feature denial names the feature, because "the engine said no" is
    /// not an actionable answer and "the engine's feature is not enabled" is.
    /// A risk-policy denial names the risk tier, because the actionable
    /// answer is "the execution policy does not allow this risk tier", not
    /// "the engine said no".
    pub fn reason(&self) -> String {
        match self {
            Denial::FeatureMissing { feature } => {
                format!("the engine denied it because the `{feature}` feature is not enabled")
            }
            Denial::RiskPolicyDenied { risk } => format!(
                "the engine denied it because the `{risk}` risk tier is not allowed by the current \
                 execution policy"
            ),
            Denial::Other { message } => format!("the engine denied it: {message}"),
        }
    }
}

/// The marker the policy layer uses when a required feature is off.
///
/// The engine's own denial message is `required feature '<X>' is not
/// enabled` (`crates/eggsec-policy/src/decision.rs`); the dispatch layer
/// phrases the same fact as `requires feature '<X>' which is not compiled
/// in`. Both name the feature, and the GUI recognises both: a denial that
/// names a feature is a fact about the daemon's build, and a denial that
/// does not is an opaque failure the GUI reports verbatim.
const POLICY_FEATURE_MARKER: &str = "required feature '";
const DISPATCH_FEATURE_MARKER: &str = "requires feature '";

/// The marker the policy layer uses when the operator's risk policy refuses.
///
/// The engine's denial message is `operation risk '<X>' is not allowed by
/// current execution policy`
/// (`crates/eggsec-policy/src/decision.rs`).
const POLICY_RISK_MARKER: &str = "is not allowed by current execution policy";

/// Read a denial out of a daemon error message.
///
/// A message that names a missing feature yields [`Denial::FeatureMissing`];
/// a message that names a refused risk tier yields
/// [`Denial::RiskPolicyDenied`]; every other message yields
/// [`Denial::Other`]. A denial is never guessed: the GUI records what the
/// engine said, it does not infer a reason the engine did not give.
pub fn denial_from_message(message: &str) -> Denial {
    let feature = quoted_after(message, POLICY_FEATURE_MARKER)
        .or_else(|| quoted_after(message, DISPATCH_FEATURE_MARKER));
    match feature {
        Some(feature) => Denial::FeatureMissing { feature },
        None => match risk_before(message, POLICY_RISK_MARKER) {
            Some(risk) => Denial::RiskPolicyDenied { risk },
            None => Denial::Other {
                message: message.to_owned(),
            },
        },
    }
}

/// The quoted word that follows `marker`, if there is one.
fn quoted_after(message: &str, marker: &str) -> Option<String> {
    let (_, rest) = message.split_once(marker)?;
    let end = rest.find('\'')?;
    let feature = rest[..end].trim();
    if feature.is_empty() {
        None
    } else {
        Some(feature.to_owned())
    }
}

/// The quoted word immediately before `marker`, if there is one.
///
/// The policy layer phrases the risk denial as `operation risk '<X>' is not
/// allowed by current execution policy`, so the risk tier is the last
/// quoted word before the marker.
fn risk_before(message: &str, marker: &str) -> Option<String> {
    let (before, _) = message.split_once(marker)?;
    let close = before.rfind('\'')?;
    let head = &before[..close];
    let open = head.rfind('\'')?;
    let risk = head[open + 1..].trim();
    if risk.is_empty() {
        None
    } else {
        Some(risk.to_owned())
    }
}

/// The unavailability the GUI has learned from live denials.
///
/// The ledger holds **one entry per task-kind variant**, so its size is
/// bounded by the number of published task kinds and not by the number of
/// denials: a second denial for the same variant replaces the first. A
/// variant the GUI has never seen a denial for has no entry, and the
/// coverage model falls back to the published facts for it.
///
/// The map is ordered so the coverage table and the diagnostics are
/// deterministic.
#[derive(Debug, Clone, Default)]
pub struct DenialLedger {
    denials: BTreeMap<String, Denial>,
}

impl DenialLedger {
    /// Record the denial learned for a task-kind variant.
    ///
    /// The most recent denial wins: it is the engine's latest word about
    /// that variant.
    pub fn record(&mut self, variant: &str, denial: Denial) {
        self.denials.insert(variant.to_owned(), denial);
    }

    /// The denial learned for a task-kind variant, if there is one.
    pub fn get(&self, variant: &str) -> Option<&Denial> {
        self.denials.get(variant)
    }

    /// Whether the ledger holds no denial at all.
    pub fn is_empty(&self) -> bool {
        self.denials.is_empty()
    }

    /// How many variants the ledger holds a denial for.
    ///
    /// Bounded by the published task-kind count.
    pub fn len(&self) -> usize {
        self.denials.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_policy_denial_names_the_missing_feature() {
        let denial = denial_from_message("required feature 'nse' is not enabled");
        assert_eq!(
            denial,
            Denial::FeatureMissing {
                feature: "nse".to_owned()
            }
        );
    }

    #[test]
    fn a_dispatch_denial_names_the_missing_feature() {
        let denial = denial_from_message(
            "operation 'x' requires feature 'browser' which is not compiled in",
        );
        assert_eq!(
            denial,
            Denial::FeatureMissing {
                feature: "browser".to_owned()
            }
        );
    }

    /// A risk-policy denial is recognised as such, and it names the risk
    /// tier the operator has to allow.
    ///
    /// This is the fact a widened capability list makes visible: the daemon
    /// advertises the capability and has the features, but the operator's
    /// risk policy says no. The GUI reports that as a policy refusal, not as
    /// an available operation and not as an opaque failure.
    #[test]
    fn a_risk_policy_denial_is_recognised() {
        let message = "enforcement denied: operation risk 'StressTest' is not allowed by \
                       current execution policy";
        let denial = denial_from_message(message);
        assert_eq!(
            denial,
            Denial::RiskPolicyDenied {
                risk: "StressTest".to_owned()
            }
        );
        assert!(denial.reason().contains("StressTest"));
        assert!(denial.reason().contains("execution policy"));
    }

    /// A denial that names no risk tier and no feature stays opaque: the GUI
    /// reports what the engine said and guesses nothing.
    #[test]
    fn an_unnamed_denial_stays_opaque() {
        let denial = denial_from_message("enforcement denied: operation denied by policy");
        assert!(matches!(denial, Denial::Other { .. }));
    }

    #[test]
    fn a_denial_without_a_feature_is_reported_verbatim() {
        let denial = denial_from_message("target unreachable");
        assert_eq!(
            denial,
            Denial::Other {
                message: "target unreachable".to_owned()
            }
        );
    }

    #[test]
    fn an_empty_feature_name_is_not_a_feature_denial() {
        let denial = denial_from_message("required feature '' is not enabled");
        assert!(matches!(denial, Denial::Other { .. }));
    }

    #[test]
    fn the_ledger_holds_one_entry_per_variant() {
        let mut ledger = DenialLedger::default();
        assert!(ledger.is_empty());
        assert_eq!(ledger.len(), 0);

        ledger.record(
            "alpha",
            Denial::FeatureMissing {
                feature: "one".to_owned(),
            },
        );
        ledger.record(
            "alpha",
            Denial::FeatureMissing {
                feature: "two".to_owned(),
            },
        );
        ledger.record(
            "beta",
            Denial::Other {
                message: "nope".to_owned(),
            },
        );

        // Two variants, three denials: the ledger is bounded by the variant
        // count, not by the denial count.
        assert_eq!(ledger.len(), 2);
        assert!(!ledger.is_empty());
        assert_eq!(
            ledger.get("alpha"),
            Some(&Denial::FeatureMissing {
                feature: "two".to_owned()
            })
        );
        assert_eq!(
            ledger.get("beta"),
            Some(&Denial::Other {
                message: "nope".to_owned()
            })
        );
        assert_eq!(ledger.get("gamma"), None);
    }

    #[test]
    fn a_feature_denial_names_the_feature_in_its_reason() {
        let denial = Denial::FeatureMissing {
            feature: "nse".to_owned(),
        };
        assert!(denial.reason().contains("nse"));
    }
}
