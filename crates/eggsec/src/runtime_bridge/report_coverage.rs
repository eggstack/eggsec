//! The report coverage table: which canonical operations have a report form.
//!
//! This module is the **single source of truth** for the report bridge's
//! coverage. It states, for every canonical operation id, whether the
//! operation's result maps onto the canonical report model, and — when it does
//! not — the fact the result is missing.
//!
//! The table is read by the bridge (which decides whether to build a
//! document), published by the engine (which tells the world what it can
//! produce), republished verbatim by the daemon, and rendered by the GUI. No
//! crate restates it.
//!
//! # The clean-mapping criterion
//!
//! A result **variant** has a **clean report mapping** when its result carries
//!
//! - (C1) an **identity** — a target or an equivalent subject — so the
//!   document can say what it describes, and
//! - (C2) at least one **content** item that maps onto the canonical model
//!   without a fact being invented: a finding that carries its own severity,
//!   title, description and location, or a port, service, or
//!   wireless-network record.
//!
//! A missing **timestamp** or **duration** does not disqualify a variant: the
//! bridge leaves those fields empty rather than inventing them, and the
//! canonical model tolerates it. A missing **identity** does disqualify it —
//! a document with an empty `target` does not say what it is about, and every
//! renderer carries that emptiness into the artifact.
//!
//! # Why the verdict is per operation
//!
//! The table is keyed by **operation id**, and the variants of one operation
//! share a verdict. That is what makes a per-task-kind verdict possible at
//! all: the GUI's coverage table has one row per published task kind, and a
//! task kind may produce more than one result variant. Three operations carry
//! more than one variant — `waf-detect`, `storage` and `integrations` — and
//! in all three the variants agree, which is a measured fact about the result
//! types rather than a convenience.
//!
//! # Totality
//!
//! Every [`TaskResult`](crate::dispatch::TaskResult) variant is accounted for
//! exactly once: the twenty-eight rows below name thirty-four of them, and
//! the terminal error shape is named beside them. The match in
//! [`result_variant_name`] is exhaustive, so adding a variant to the enum
//! breaks the build here until it is named — and the static guard checks the
//! same fact from the source text.

use crate::dispatch::TaskResult;

pub use eggsec_runtime::report::ReportForm;

/// One operation's report-form verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportCoverageRow {
    /// The canonical operation id.
    pub operation_id: &'static str,
    /// The `TaskResult` variants the operation can produce.
    ///
    /// This is the unit the bridge dispatches on, so it is what the table is
    /// checked against: a variant under two operations would make the
    /// operation's verdict ambiguous.
    pub variants: &'static [&'static str],
    /// The verdict.
    pub form: ReportForm,
    /// Why the operation has no report form.
    ///
    /// Empty for a canonical operation. For an excluded one it names the
    /// field the result is missing, or the absence that disqualifies it, in
    /// operator-readable words.
    pub reason: &'static str,
}

/// The coverage table: one row per canonical operation id.
///
/// The rows are in the enum's declaration order, grouped by operation id, so
/// the table reads alongside the result types it judges.
pub const REPORT_COVERAGE: &[ReportCoverageRow] = &[
    ReportCoverageRow {
        operation_id: "load-test",
        variants: &["LoadTest"],
        form: ReportForm::NoCleanMapping,
        reason: "the result carries only transport-error strings and a status-code count map, so \
                 it has no finding with a severity, a title or a location",
    },
    ReportCoverageRow {
        operation_id: "stress-test",
        variants: &["StressTest"],
        form: ReportForm::NoCleanMapping,
        reason: "the result carries only counters — packets sent, bytes sent and error counts — \
                 and no finding, no port and no service",
    },
    ReportCoverageRow {
        operation_id: "scan-ports",
        variants: &["PortScan"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "scan-endpoints",
        variants: &["EndpointScan"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "fingerprint",
        variants: &["Fingerprint"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "waf-detect",
        variants: &["WafDetection", "WafBypass"],
        form: ReportForm::NoCleanMapping,
        reason: "neither variant carries a target: the detection and the bypass record the WAF's \
                 own facts, and the subject lives in the request",
    },
    ReportCoverageRow {
        operation_id: "waf-stress",
        variants: &["WafStress"],
        form: ReportForm::NoCleanMapping,
        reason: "the result is a bare list of bypass attempts with no subject and no duration",
    },
    ReportCoverageRow {
        operation_id: "pipeline",
        variants: &["Pipeline"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "fuzz",
        variants: &["Fuzz"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "recon",
        variants: &["Recon"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "packet-capture",
        variants: &["PacketCapture"],
        form: ReportForm::NoCleanMapping,
        reason: "the result names no interface and carries only a packet count and an output \
                 file, so the captured traffic has no home in the canonical model",
    },
    ReportCoverageRow {
        operation_id: "packet",
        variants: &["PacketTraceroute", "PacketSend"],
        form: ReportForm::NoCleanMapping,
        reason: "neither variant names the host it ran against: the hops and the two counters \
                 carry no subject",
    },
    ReportCoverageRow {
        operation_id: "graphql",
        variants: &["GraphQl"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "oauth",
        variants: &["OAuth"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "nse",
        variants: &["Nse"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "hunt",
        variants: &["Hunt"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "browser",
        variants: &["Browser"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "compliance",
        variants: &["Compliance"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "storage",
        variants: &["Storage", "StorageListScans", "StorageListFindings"],
        form: ReportForm::NoCleanMapping,
        reason:
            "the result is a bare acknowledgement or a catalogue read: stored scans and stored \
                 findings are lifecycle views, not a scan of one subject",
    },
    ReportCoverageRow {
        operation_id: "integrations",
        variants: &[
            "Integrations",
            "IntegrationsCreateIssue",
            "IntegrationsSearchIssues",
        ],
        form: ReportForm::NoCleanMapping,
        reason: "the result is a tracker work item or a list of them and carries no target: an \
                 issue is not a fact about a scanned subject",
    },
    ReportCoverageRow {
        operation_id: "workflow",
        variants: &["Workflow"],
        form: ReportForm::NoCleanMapping,
        reason: "the report carries counts and findings but no target field, so a workflow run is \
                 a lifecycle view over stored findings",
    },
    ReportCoverageRow {
        operation_id: "vuln",
        variants: &["Vuln"],
        form: ReportForm::NoCleanMapping,
        reason: "the assessment carries no target and its prioritized findings carry no location, \
                 so there is no subject to report on",
    },
    ReportCoverageRow {
        operation_id: "wireless",
        variants: &["Wireless"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "wireless-deauth",
        variants: &["WirelessActive"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "auth-test",
        variants: &["Auth"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "db-pentest",
        variants: &["DbPentest"],
        form: ReportForm::Canonical,
        reason: "",
    },
    ReportCoverageRow {
        operation_id: "proxy-intercept",
        variants: &["Intercept"],
        form: ReportForm::NoCleanMapping,
        reason: "the session's target is optional and absent whenever the session had none, and \
                 the captured exchanges have no collection in the canonical model",
    },
    ReportCoverageRow {
        operation_id: "c2",
        variants: &["C2"],
        form: ReportForm::Canonical,
        reason: "",
    },
];

/// The result shapes that no operation produces.
///
/// The terminal error shape is a task's *failure*, not an operation's result:
/// there is no scan, no subject and no duration to report, so a "report" of a
/// failure would be a document with an empty target and no findings. It is
/// named here so the coverage accounts for every result variant exactly once.
pub const TERMINAL_RESULT_VARIANTS: &[&str] = &["Error"];

/// The coverage row for a canonical operation id.
///
/// `None` means the table has no row for that operation — an internal
/// inconsistency, because the table covers every canonical operation id.
pub fn coverage_for_operation(operation_id: &str) -> Option<&'static ReportCoverageRow> {
    REPORT_COVERAGE
        .iter()
        .find(|row| row.operation_id == operation_id)
}

/// The coverage row that names a result variant.
///
/// `None` means no operation produces that variant.
pub fn coverage_for_variant(variant: &str) -> Option<&'static ReportCoverageRow> {
    REPORT_COVERAGE
        .iter()
        .find(|row| row.variants.contains(&variant))
}

/// Declare the engine's result-variant vocabulary.
///
/// The macro generates two things from one list: the names themselves, and an
/// **exhaustive** match over [`TaskResult`]. The match is what keeps the two
/// in step — adding a variant to the enum makes the match non-exhaustive, so
/// the build fails here until the variant is named, and the table cannot fall
/// behind the enum without the compiler saying so.
macro_rules! declare_result_variants {
    ($(
        $(#[$meta:meta])*
        $variant:ident
    ),* $(,)?) => {
        /// Every `TaskResult` variant's canonical name.
        ///
        /// Derived from the enum: the list and the match below are generated
        /// from one declaration, and the match is exhaustive.
        pub const RESULT_VARIANT_NAMES: &[&str] = &[
            $(
                $(#[$meta])*
                stringify!($variant),
            )*
        ];

        /// The canonical name of a result's variant.
        ///
        /// The match is exhaustive over [`TaskResult`]: a new variant breaks
        /// the build here until it is named.
        pub fn result_variant_name(result: &TaskResult) -> &'static str {
            match result {
                $(
                    $(#[$meta])*
                    TaskResult::$variant { .. } => stringify!($variant),
                )*
            }
        }
    };
}

declare_result_variants! {
    LoadTest,
    #[cfg(feature = "stress-testing")]
    StressTest,
    PortScan,
    EndpointScan,
    Fingerprint,
    WafDetection,
    WafBypass,
    WafStress,
    Pipeline,
    Fuzz,
    Recon,
    PacketCapture,
    PacketTraceroute,
    PacketSend,
    GraphQl,
    OAuth,
    #[cfg(feature = "nse")]
    Nse,
    #[cfg(feature = "advanced-hunting")]
    Hunt,
    #[cfg(feature = "headless-browser")]
    Browser,
    #[cfg(feature = "compliance")]
    Compliance,
    #[cfg(feature = "database")]
    Storage,
    #[cfg(feature = "database")]
    StorageListScans,
    #[cfg(feature = "database")]
    StorageListFindings,
    #[cfg(feature = "external-integrations")]
    Integrations,
    #[cfg(feature = "external-integrations")]
    IntegrationsCreateIssue,
    #[cfg(feature = "external-integrations")]
    IntegrationsSearchIssues,
    #[cfg(feature = "finding-workflow")]
    Workflow,
    #[cfg(feature = "vuln-management")]
    Vuln,
    #[cfg(feature = "wireless")]
    Wireless,
    #[cfg(feature = "wireless-advanced")]
    WirelessActive,
    Auth,
    #[cfg(feature = "db-pentest")]
    DbPentest,
    #[cfg(feature = "web-proxy")]
    Intercept,
    #[cfg(feature = "c2")]
    C2,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    /// The canonical operation ids, derived from the task-kind table rather
    /// than transcribed: the coverage table is checked against the owner.
    fn canonical_operation_ids() -> Vec<String> {
        let mut ids: Vec<String> = eggsec_runtime::TaskKind::all_variants()
            .iter()
            .map(|kind| kind.operation_id().to_owned())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    #[test]
    fn the_coverage_table_covers_every_canonical_operation() {
        let canonical = canonical_operation_ids();
        let mut covered: Vec<String> = REPORT_COVERAGE
            .iter()
            .map(|row| row.operation_id.to_owned())
            .collect();
        covered.sort();

        assert_eq!(
            covered, canonical,
            "the coverage table's operation ids are exactly the canonical task-kind table's"
        );
        assert_eq!(
            covered.len(),
            REPORT_COVERAGE.len(),
            "no operation id appears under two rows"
        );
    }

    #[test]
    fn every_excluded_operation_names_the_missing_fact() {
        for row in REPORT_COVERAGE {
            match row.form {
                ReportForm::Canonical => assert!(
                    row.reason.is_empty(),
                    "`{}` is canonical and carries no reason",
                    row.operation_id
                ),
                ReportForm::NoCleanMapping => {
                    assert!(
                        !row.reason.is_empty(),
                        "`{}` is excluded and must name the fact it is missing",
                        row.operation_id
                    );
                    // A reason that names a field or an absence says which
                    // one: it states that something is missing, absent, or
                    // the only thing the result carries.
                    let names_an_absence = [" no ", "not ", "only", "bare", "neither"]
                        .iter()
                        .any(|marker| row.reason.contains(marker));
                    assert!(
                        names_an_absence,
                        "`{}` must name a field or an absence of its result type: {}",
                        row.operation_id, row.reason
                    );
                }
            }
        }
    }

    /// Every result variant is accounted for exactly once.
    ///
    /// The table is **build-independent**: it names all thirty-five variants
    /// the result type declares, feature-gated ones included, because the
    /// coverage is a statement about the engine's result model rather than
    /// about one build of it. The build-level vocabulary — the variants this
    /// build's `TaskResult` actually has — is therefore a subset of the
    /// table's, and the test checks that subset here.
    ///
    /// The source-level equality — every variant the enum *declares* is
    /// named by the table, and the table names nothing else — is the static
    /// guard's job: it reads the declarations, so it sees the feature gates
    /// the compiler hides from a build.
    #[test]
    fn every_result_variant_is_accounted_for_exactly_once() {
        let mut owners: BTreeMap<&str, &str> = BTreeMap::new();
        for row in REPORT_COVERAGE {
            assert!(
                !row.variants.is_empty(),
                "`{}` names no result variant",
                row.operation_id
            );
            for variant in row.variants {
                let previous = owners.insert(*variant, row.operation_id);
                assert!(
                    previous.is_none(),
                    "`{variant}` is claimed by both `{}` and `{}`",
                    previous.unwrap_or_default(),
                    row.operation_id
                );
            }
        }
        for variant in TERMINAL_RESULT_VARIANTS {
            let previous = owners.insert(*variant, "");
            assert!(
                previous.is_none(),
                "the terminal variant `{variant}` is claimed twice"
            );
        }

        for name in RESULT_VARIANT_NAMES {
            assert!(
                owners.contains_key(name),
                "the coverage does not account for the result variant `{name}`"
            );
        }

        assert_eq!(
            owners.len(),
            35,
            "the coverage accounts for every result variant the engine declares"
        );
        assert!(
            owners.contains_key("Error"),
            "the terminal error shape is named by the coverage"
        );
    }

    #[test]
    fn the_variants_of_one_operation_share_a_verdict() {
        // The verdict is per operation, so a multi-variant operation states
        // one verdict for all of its variants. The table is keyed that way;
        // this asserts the keying is unambiguous.
        for row in REPORT_COVERAGE {
            for variant in row.variants {
                let found = coverage_for_variant(variant).expect("the variant has a row");
                assert_eq!(
                    found.operation_id, row.operation_id,
                    "`{variant}` is claimed by more than one operation"
                );
            }
        }
        assert_eq!(
            coverage_for_variant("WafDetection").map(|row| row.variants),
            Some(&["WafDetection", "WafBypass"][..])
        );
        assert_eq!(
            coverage_for_variant("StorageListFindings").map(|row| row.variants.len()),
            Some(3)
        );
        assert_eq!(
            coverage_for_variant("IntegrationsSearchIssues").map(|row| row.variants.len()),
            Some(3)
        );
    }

    #[test]
    fn the_table_publishes_every_row() {
        assert_eq!(
            REPORT_COVERAGE.len(),
            28,
            "one row per canonical operation id"
        );
        let canonical = coverage_for_operation("scan-ports").expect("a canonical operation");
        assert_eq!(canonical.form, ReportForm::Canonical);
        assert!(canonical.reason.is_empty());

        let excluded = coverage_for_operation("waf-detect").expect("an excluded operation");
        assert_eq!(excluded.form, ReportForm::NoCleanMapping);
        assert!(!excluded.reason.is_empty());
    }
}
