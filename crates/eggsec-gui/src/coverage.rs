//! The coverage model: one reachability verdict per published task kind.
//!
//! The GUI renders a form for every task kind the daemon publishes, and the
//! operator needs to know which of those the connected daemon can actually
//! run. This module produces that answer from **published data only**:
//!
//! - the task kinds and operations the daemon's catalog publishes,
//! - the capability list the daemon advertises, and
//! - the enabled-feature set the daemon publishes (when it publishes one).
//!
//! Nothing is transcribed here. The variants, the operation ids, the
//! capability names, and the required-feature lists all arrive as arguments;
//! the verdicts are computed from them. A task kind the engine publishes
//! tomorrow is covered by this code without a line being added, which is the
//! same contract the form engine has.
//!
//! # The published feature set is a fact, not a guess
//!
//! `enabled_features` is `None` when the daemon did not publish one. `None`
//! means **unknown**, never "nothing is enabled": a daemon too old to know
//! the field must not make the GUI claim that every feature-gated operation
//! is unavailable. In that case a task kind whose required features cannot
//! be confirmed is `Unverified`, and the GUI shows it rather than hiding it.
//!
//! A **learned denial** wins over every static verdict: when the engine
//! refuses an operation at run time, that refusal is the engine's own word
//! about the running daemon, and no published list can overrule it.

use eggsec_daemon_protocol::operations::{OperationDescription, TaskKindDescription};
use eggsec_runtime::TaskCapability;

use crate::availability::DenialLedger;

/// Why an operation is or is not reachable from this GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reachability {
    /// The daemon advertises the capability and the catalog exposes the
    /// operation on the manual surface.
    Available,
    /// The catalog does not expose this operation on the manual surface.
    NotExposed,
    /// The daemon does not advertise the capability this task kind needs.
    NotAdvertised,
    /// The daemon advertises the capability but was built without one of the
    /// operation's required features.
    FeatureDisabled,
    /// The daemon's enabled-feature set is unknown, so availability cannot
    /// be verified.
    Unverified,
    /// A live denial proved the operation unavailable in this session.
    Blocked,
}

/// One row of the coverage table: every published task kind, with the
/// reason it is or is not reachable.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskCoverage {
    /// The task kind's wire discriminator.
    pub variant: String,
    /// The canonical operation the task kind executes.
    pub operation_id: String,
    /// The wire capability name the task kind advertises.
    pub capability_name: String,
    /// The engine features the operation requires.
    pub required_features: Vec<String>,
    /// The verdict.
    pub reachability: Reachability,
    /// Why the verdict is what it is, in operator-readable words.
    pub reason: String,
}

/// Build the coverage table from what the daemon published.
///
/// The table has one row per published task kind. Nothing here is
/// transcribed: the variants, operation ids, capability names, and required
/// features all come from the daemon's own published catalog, and the
/// verdicts are computed from the published capability list and the
/// published enabled-feature set.
///
/// `enabled_features` is `None` when the daemon did not publish one, which
/// means *unknown* — never "nothing is enabled". In that case a task kind
/// whose required features cannot be confirmed is `Unverified`, not
/// `FeatureDisabled`.
pub fn coverage_for(
    task_kinds: &[TaskKindDescription],
    operations: &[OperationDescription],
    advertised_capabilities: &[TaskCapability],
    enabled_features: Option<&[String]>,
) -> Vec<TaskCoverage> {
    coverage_for_with_ledger(
        task_kinds,
        operations,
        advertised_capabilities,
        enabled_features,
        &DenialLedger::default(),
    )
}

/// Build the coverage table, letting a learned denial win.
///
/// This is [`coverage_for`] plus the denial ledger: a task kind the engine
/// has already refused in this session is [`Reachability::Blocked`], and the
/// learned denial's reason replaces the static one. The static verdict is
/// kept for every other row.
pub fn coverage_for_with_ledger(
    task_kinds: &[TaskKindDescription],
    operations: &[OperationDescription],
    advertised_capabilities: &[TaskCapability],
    enabled_features: Option<&[String]>,
    ledger: &DenialLedger,
) -> Vec<TaskCoverage> {
    task_kinds
        .iter()
        .map(|task_kind| {
            let operation = operations
                .iter()
                .find(|operation| operation.id == task_kind.operation_id);
            let required_features = operation
                .map(|operation| operation.required_features.clone())
                .unwrap_or_default();

            let advertised = advertised_capabilities
                .iter()
                .any(|capability| capability.name == task_kind.capability_name);

            let (reachability, reason) =
                resolve(operation, &required_features, advertised, enabled_features);

            // A learned denial wins: the engine's own refusal is the last
            // word about the running daemon.
            let (reachability, reason) = match ledger.get(&task_kind.variant) {
                Some(denial) => (Reachability::Blocked, denial.reason()),
                None => (reachability, reason),
            };

            TaskCoverage {
                variant: task_kind.variant.clone(),
                operation_id: task_kind.operation_id.clone(),
                capability_name: task_kind.capability_name.clone(),
                required_features,
                reachability,
                reason,
            }
        })
        .collect()
}

/// Decide one task kind's reachability.
fn resolve(
    operation: Option<&OperationDescription>,
    required_features: &[String],
    advertised: bool,
    enabled_features: Option<&[String]>,
) -> (Reachability, String) {
    // The catalog decides what a manual surface may run. An operation the
    // catalog does not expose is not reachable from this GUI at all, whether
    // or not the daemon advertises a capability for it.
    let Some(operation) = operation else {
        return (
            Reachability::NotExposed,
            "the daemon's catalog publishes no operation for this task kind".to_owned(),
        );
    };
    if !operation.manual_exposable {
        return (
            Reachability::NotExposed,
            "the daemon does not expose this operation on the manual surface".to_owned(),
        );
    }

    if !advertised {
        // The daemon withholds the capability. That is a *different* fact
        // from "the daemon lacks the feature", and the reason says which one
        // it is: a daemon that has the features and still withholds the
        // capability is making a policy choice, not a build choice.
        return (
            Reachability::NotAdvertised,
            match missing_feature(required_features, enabled_features) {
                Some(feature) => format!(
                    "the daemon does not advertise this capability, and its enabled features are \
                     missing `{feature}`"
                ),
                None => match enabled_features {
                    Some(_) => "the daemon does not advertise this capability, even though it has \
                                the required features enabled"
                        .to_owned(),
                    None => "the daemon does not advertise this capability".to_owned(),
                },
            },
        );
    }

    let Some(features) = enabled_features else {
        // The daemon's feature set is unknown. There is nothing to verify
        // when the operation requires no feature at all, so it is available;
        // otherwise the GUI cannot tell, and says so instead of guessing.
        if required_features.is_empty() {
            return (
                Reachability::Available,
                "the daemon advertises this capability".to_owned(),
            );
        }
        return (
            Reachability::Unverified,
            "the daemon's enabled-feature set is unknown, so this operation's availability cannot \
             be verified"
                .to_owned(),
        );
    };

    match missing_feature(required_features, Some(features)) {
        Some(feature) => (
            Reachability::FeatureDisabled,
            format!(
                "the daemon does not have the `{feature}` feature enabled, which this operation \
                     requires"
            ),
        ),
        None => (
            Reachability::Available,
            "the daemon advertises this capability and has its required features enabled"
                .to_owned(),
        ),
    }
}

/// The first required feature the daemon's enabled-feature set is missing.
///
/// `None` means "no required feature is missing", which includes the case
/// where the operation requires no feature at all.
fn missing_feature(
    required_features: &[String],
    enabled_features: Option<&[String]>,
) -> Option<String> {
    let enabled = enabled_features?;
    required_features
        .iter()
        .find(|feature| !enabled.iter().any(|enabled| enabled == *feature))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::availability::{denial_from_message, Denial};

    fn task_kind(variant: &str, operation_id: &str, capability_name: &str) -> TaskKindDescription {
        TaskKindDescription {
            variant: variant.to_owned(),
            operation_id: operation_id.to_owned(),
            capability_name: capability_name.to_owned(),
            request_schema: serde_json::Value::Null,
        }
    }

    fn operation(
        id: &str,
        manual_exposable: bool,
        required_features: &[&str],
    ) -> OperationDescription {
        let description = eggsec_daemon_protocol::operations::describe_operations()
            .operations
            .into_iter()
            .find(|operation| operation.id == id)
            .expect("a published operation");
        OperationDescription {
            id: description.id,
            manual_exposable,
            required_features: required_features
                .iter()
                .map(|feature| (*feature).to_owned())
                .collect(),
            ..description
        }
    }

    fn capability(name: &str) -> TaskCapability {
        TaskCapability::new(name, name)
    }

    #[test]
    fn an_advertised_and_exposed_operation_is_available() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &[])];
        let capabilities = [capability("port-scan")];

        let rows = coverage_for(&kinds, &operations, &capabilities, Some(&[]));

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].reachability, Reachability::Available);
        assert!(rows[0].reason.contains("advertises"));
    }

    #[test]
    fn an_operation_the_catalog_does_not_expose_is_not_exposed() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", false, &[])];
        let capabilities = [capability("port-scan")];

        let rows = coverage_for(&kinds, &operations, &capabilities, Some(&[]));

        assert_eq!(rows[0].reachability, Reachability::NotExposed);
        assert!(rows[0].reason.contains("manual surface"));
    }

    #[test]
    fn a_capability_the_daemon_withholds_is_not_advertised() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &[])];
        let capabilities: [TaskCapability; 0] = [];

        let rows = coverage_for(&kinds, &operations, &capabilities, Some(&[]));

        assert_eq!(rows[0].reachability, Reachability::NotAdvertised);
        assert!(rows[0].reason.contains("does not advertise"));
    }

    #[test]
    fn a_withheld_capability_with_the_features_present_is_reported_differently() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &["nse"])];
        let capabilities: [TaskCapability; 0] = [];
        let features = vec!["nse".to_owned()];

        let rows = coverage_for(&kinds, &operations, &capabilities, Some(&features));

        assert_eq!(rows[0].reachability, Reachability::NotAdvertised);
        // "the daemon withholds it" and "the daemon lacks the feature" are
        // different facts, and the reason must tell them apart.
        assert!(rows[0].reason.contains("even though"));
        assert!(!rows[0].reason.contains("missing"));
    }

    #[test]
    fn a_withheld_capability_with_a_missing_feature_names_the_feature() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &["nse"])];
        let capabilities: [TaskCapability; 0] = [];
        let features = vec!["cli".to_owned()];

        let rows = coverage_for(&kinds, &operations, &capabilities, Some(&features));

        assert_eq!(rows[0].reachability, Reachability::NotAdvertised);
        assert!(rows[0].reason.contains("nse"));
    }

    #[test]
    fn a_missing_feature_is_feature_disabled() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &["nse"])];
        let capabilities = [capability("port-scan")];
        let features = vec!["cli".to_owned()];

        let rows = coverage_for(&kinds, &operations, &capabilities, Some(&features));

        assert_eq!(rows[0].reachability, Reachability::FeatureDisabled);
        assert!(rows[0].reason.contains("nse"));
        assert_eq!(rows[0].required_features, vec!["nse".to_owned()]);
    }

    #[test]
    fn an_unknown_feature_set_is_unverified_not_feature_disabled() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &["nse"])];
        let capabilities = [capability("port-scan")];

        let rows = coverage_for(&kinds, &operations, &capabilities, None);

        assert_eq!(rows[0].reachability, Reachability::Unverified);
        assert!(rows[0].reason.contains("unknown"));
    }

    #[test]
    fn an_unknown_feature_set_does_not_hide_an_operation_that_needs_no_feature() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &[])];
        let capabilities = [capability("port-scan")];

        let rows = coverage_for(&kinds, &operations, &capabilities, None);

        assert_eq!(rows[0].reachability, Reachability::Available);
    }

    #[test]
    fn an_empty_published_feature_set_is_not_the_same_as_an_unknown_one() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &["nse"])];
        let capabilities = [capability("port-scan")];
        let features: Vec<String> = Vec::new();

        let rows = coverage_for(&kinds, &operations, &capabilities, Some(&features));

        assert_eq!(rows[0].reachability, Reachability::FeatureDisabled);
    }

    #[test]
    fn a_learned_denial_wins_over_the_static_verdict() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &[])];
        let capabilities = [capability("port-scan")];

        let mut ledger = DenialLedger::default();
        ledger.record(
            "Alpha",
            denial_from_message("required feature 'nse' is not enabled"),
        );

        let rows = coverage_for_with_ledger(&kinds, &operations, &capabilities, Some(&[]), &ledger);

        assert_eq!(rows[0].reachability, Reachability::Blocked);
        assert!(rows[0].reason.contains("nse"));
    }

    #[test]
    fn a_learned_denial_blocks_even_an_available_operation() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &[])];
        let capabilities = [capability("port-scan")];

        let mut ledger = DenialLedger::default();
        ledger.record(
            "Alpha",
            Denial::Other {
                message: "nope".to_owned(),
            },
        );

        let rows = coverage_for_with_ledger(&kinds, &operations, &capabilities, None, &ledger);

        assert_eq!(rows[0].reachability, Reachability::Blocked);
        assert!(rows[0].reason.contains("nope"));
    }

    #[test]
    fn a_denial_for_another_variant_leaves_this_one_alone() {
        let kinds = [task_kind("Alpha", "scan-ports", "port-scan")];
        let operations = [operation("scan-ports", true, &[])];
        let capabilities = [capability("port-scan")];

        let mut ledger = DenialLedger::default();
        ledger.record(
            "Beta",
            Denial::Other {
                message: "nope".to_owned(),
            },
        );

        let rows = coverage_for_with_ledger(&kinds, &operations, &capabilities, Some(&[]), &ledger);

        assert_eq!(rows[0].reachability, Reachability::Available);
    }

    #[test]
    fn the_table_has_one_row_per_published_task_kind() {
        let kinds = [
            task_kind("Alpha", "scan-ports", "port-scan"),
            task_kind("Beta", "recon", "recon"),
        ];
        let operations = [
            operation("scan-ports", true, &[]),
            operation("recon", true, &[]),
        ];
        let capabilities = [capability("port-scan"), capability("recon")];

        let rows = coverage_for(&kinds, &operations, &capabilities, Some(&[]));

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].variant, "Alpha");
        assert_eq!(rows[1].variant, "Beta");
    }

    /// Whether `haystack` mentions `needle` as a whole identifier.
    ///
    /// A word boundary on both sides: `Fuzz` is a variant name, and
    /// `Fuzzing` is an ordinary word.
    fn contains_identifier(haystack: &str, needle: &str) -> bool {
        let is_boundary = |character: Option<char>| {
            character
                .map(|character| !character.is_alphanumeric() && character != '_')
                .unwrap_or(true)
        };
        haystack.match_indices(needle).any(|(index, _)| {
            is_boundary(haystack[..index].chars().next_back())
                && is_boundary(haystack[index + needle.len()..].chars().next())
        })
    }

    /// The GUI's coverage model must be **derived**: the production sources
    /// of the GUI may not name an operation id, a task-kind variant, or a
    /// capability name. A hand-written per-operation table would drift the
    /// moment the engine published a new operation, and the drift would
    /// show up as a missing row rather than as a failure.
    ///
    /// The names come from the runtime's own published catalog, so the test
    /// does not transcribe them either.
    ///
    /// The match is precise on purpose. A **variant** is matched as a whole
    /// identifier, the way the architecture guard matches it. An
    /// **operation id** or a **capability name** is matched as a complete
    /// string literal, because those names are ordinary words — `browser`,
    /// `fuzz` — and prose that happens to use one is not a transcription.
    /// A table would name them as literals.
    #[test]
    fn the_gui_production_sources_name_no_operation_variant_or_capability() {
        let published = eggsec_runtime::schema::task_kind_schemas();
        assert!(!published.is_empty(), "the runtime publishes task kinds");

        let mut variants: Vec<String> = Vec::new();
        let mut literals: Vec<String> = Vec::new();
        for entry in &published {
            variants.push(entry.variant.clone());
            literals.push(entry.operation_id.clone());
            literals.push(entry.capability_name.clone());
        }

        let sources = [
            ("availability.rs", include_str!("availability.rs")),
            ("commands.rs", include_str!("commands.rs")),
            ("config.rs", include_str!("config.rs")),
            ("connection.rs", include_str!("connection.rs")),
            ("coverage.rs", include_str!("coverage.rs")),
            ("daemon.rs", include_str!("daemon.rs")),
            ("export.rs", include_str!("export.rs")),
            ("form.rs", include_str!("form.rs")),
            ("lib.rs", include_str!("lib.rs")),
            ("shell.rs", include_str!("shell.rs")),
            ("tasks.rs", include_str!("tasks.rs")),
        ];

        for (name, source) in sources {
            let production = source
                .split("#[cfg(test)]")
                .next()
                .expect("the module has a production half");
            for variant in &variants {
                assert!(
                    !contains_identifier(production, variant),
                    "{name} names the task-kind variant `{variant}`; the coverage model must be \
                     derived from the published catalog, not transcribed"
                );
            }
            for literal in &literals {
                let quoted = format!("\"{literal}\"");
                assert!(
                    !production.contains(&quoted),
                    "{name} names the operation or capability `{literal}`; the coverage model must \
                     be derived from the published catalog, not transcribed"
                );
            }
        }
    }
}
