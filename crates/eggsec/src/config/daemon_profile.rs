//! The daemon service profile — the engine feature set a GUI-serving daemon compiles.
//!
//! A daemon that hosts the desktop GUI must be able to run the operations the
//! GUI can submit, and must not carry the engine's serving surfaces, its
//! security-risk switches, or its lab-only extras. This module owns that
//! decision as **data**, so the daemon's manifest, the operator documentation,
//! and the compiled binary can all be checked against one list.
//!
//! # The inclusion rule
//!
//! An engine feature is included iff
//!
//! 1. a code path reachable from one of the runtime's task kinds is
//!    `cfg`-gated on it (this is the [`eggsec_policy::ALL_OPERATION_METADATA`]
//!    half — see [`required_profile_features`]),
//! 2. it is the `cli` **host gate** — `cli` gates `commands`, `dispatch` and
//!    `runtime_bridge`, which is where `EggsecRuntimeExecutor` lives, so
//!    without it the daemon cannot host the executor it is built around, or
//! 3. it is a transitive dependency of an included feature and is declared
//!    explicitly here, so the effective set equals the declared set.
//!
//! # The relation to `full`
//!
//! [`crate::config::FULL_MEMBERS`] is a **curated developer/lab aggregate**:
//! it enables capabilities the daemon has no use for (`mobile`,
//! `mobile-dynamic`, `ai-integration`, `container`) and it enables the
//! `rest-api` serving surface, which would give a daemon that already speaks
//! the Unix socket a second, unintended HTTP surface. This profile is a
//! different aggregate with a different purpose, and the two are independent:
//! neither is a subset of the other by construction, and each carries its own
//! exclusion list.
//!
//! # The relation to the policy catalog
//!
//! The required half of the profile is **derived** from
//! [`eggsec_policy::ALL_OPERATION_METADATA`] — the distinct non-empty
//! `required_features` of the catalog — plus the `cli` host gate plus the
//! explicitly-declared transitive members. The derived names are never
//! hand-written, so a new catalog entry that requires a new feature fails the
//! conformance test until the profile is updated.
//!
//! The catalog also covers CLI/TUI-only surfaces the daemon never dispatches;
//! those features are listed in
//! [`CATALOG_FEATURES_OUTSIDE_THE_DAEMON_SCOPE`] with their reason and are
//! declared through the transitive/member list instead.
//!
//! # The build-side / wire-side boundary
//!
//! This module is a **build-side** contract: it describes what the daemon
//! binary was compiled with. The **wire-side** publication of the enabled
//! feature set — the handshake field a client reads to learn what the daemon
//! can do — belongs to the desktop-GUI frontend milestone that owns the
//! protocol's capability negotiation. Nothing here adds a protocol field, and
//! nothing here persists or negotiates the feature set at runtime: a feature
//! that was compiled out cannot be turned on later, which is exactly why the
//! profile is a build recipe and not a runtime setting.
//!
//! The module is compiled **unconditionally** — it is data, not a capability —
//! so the conformance checks also run in the `--no-default-features` build.

use std::fmt;

use super::feature_registry::is_known_feature;

/// The engine features the daemon's `service` profile enables.
///
/// This list is the single source of truth for the profile: the daemon's
/// `service` cargo feature in `crates/eggsec-daemon/Cargo.toml` must name
/// exactly these features, and the operator documentation's included-features
/// table must name exactly these features. Both are checked mechanically.
pub static DAEMON_SERVICE_PROFILE: &[&str] = &[
    "advanced-hunting",
    "c2",
    "cli",
    "compliance",
    "database",
    "db-pentest",
    "evasion",
    "external-integrations",
    "finding-workflow",
    "headless-browser",
    "nse",
    "packet-inspection",
    "postex",
    "stress-testing",
    "tool-api",
    "vuln-management",
    "web-proxy",
    "websocket",
    "wireless",
    "wireless-advanced",
];

/// The engine features the daemon's `service` profile excludes.
///
/// The complement of [`DAEMON_SERVICE_PROFILE`] over the engine's registry.
/// Each exclusion carries a rationale in `docs/DAEMON_FEATURE_PROFILE.md`.
pub static DAEMON_SERVICE_EXCLUDED: &[&str] = &[
    "ai-integration",
    "api-schema",
    "c2-mcp",
    "cloud",
    "config-watch",
    "container",
    "daemon-client",
    "db-pentest-mcp",
    "db-pentest-mongodb",
    "db-pentest-mssql-tiberius",
    "db-pentest-redis",
    "dynamic-plugins",
    "email-notifications",
    "full",
    "git-secrets",
    "grpc-api",
    "insecure-tls",
    "logging-subscriber",
    "mobile",
    "mobile-dynamic",
    "nse-sandbox",
    "nse-ssh2",
    "pdf",
    "rest-api",
    "sbom",
    "test-helpers",
    "transparent-proxy",
    "udp-scan",
    "web-proxy-mcp",
    "ws-api",
];

/// The excluded features whose exclusion is **absolute**.
///
/// A candidate profile that enables any of these is invalid, not merely
/// discouraged: the serving surfaces would add a second protocol the
/// operator did not ask for, `insecure-tls` bypasses certificate
/// verification, `test-helpers` is a test-only marker, `full` is an
/// aggregate (and would make the profile's own definition circular), and the
/// `*-mcp` markers would expose MCP tools over a daemon that speaks the Unix
/// socket.
pub static DAEMON_SERVICE_FORBIDDEN: &[&str] = &[
    "c2-mcp",
    "daemon-client",
    "db-pentest-mcp",
    "full",
    "grpc-api",
    "insecure-tls",
    "rest-api",
    "test-helpers",
    "web-proxy-mcp",
    "ws-api",
];

/// The features the daemon needs that the policy catalog does not name.
///
/// Each entry is a member the profile enables without a catalog entry
/// requiring it, with the reason it is required. The catalog-derived half of
/// [`required_profile_features`] is computed from the catalog itself; this
/// list is the explicitly-declared remainder.
static DECLARED_MEMBERS: &[(&str, &str)] = &[
    (
        "cli",
        "host gate: `cli` gates commands/dispatch/runtime_bridge, where EggsecRuntimeExecutor lives",
    ),
    (
        "tool-api",
        "transitive: `nse` = [\"tool-api\", ...]; declared so the effective set equals the declared set",
    ),
    (
        "postex",
        "transitive: `c2` = [\"postex\", \"evasion\"]; declared so the effective set equals the declared set",
    ),
    (
        "evasion",
        "transitive: `c2` = [\"postex\", \"evasion\"]; declared so the effective set equals the declared set",
    ),
    (
        "websocket",
        "behaviour gate: the Fuzz task kind only runs live WebSocket tests when it is compiled in",
    ),
];

/// The catalog features the daemon does not take from the catalog half.
///
/// The policy catalog covers CLI/TUI-only operations the daemon never
/// dispatches. Their features are declared through
/// [`DECLARED_MEMBERS`] (or left out entirely) instead of being read out of
/// the catalog, so the catalog-derived half of [`required_profile_features`]
/// is exactly the set the daemon's task kinds can reach.
static CATALOG_FEATURES_OUTSIDE_THE_DAEMON_SCOPE: &[(&str, &str)] = &[
    (
        "mobile",
        "no mobile task kind in the runtime's request surface; excluding it keeps `zip` out of the shipped closure",
    ),
    (
        "mobile-dynamic",
        "no mobile task kind in the runtime's request surface; its live path needs `adb` and a lab device",
    ),
    (
        "evasion",
        "reached through the `c2` domain rather than as an operation of its own; declared as a transitive member",
    ),
    (
        "postex",
        "reached through the `c2` domain rather than as an operation of its own; declared as a transitive member",
    ),
];

/// The catalog-derived half of the required set.
///
/// Computed from the policy catalog — never hand-written — so a new catalog
/// entry that requires a feature the daemon cannot reach changes this set and
/// fails the conformance check until the profile is updated.
fn catalog_required_features() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for metadata in eggsec_policy::ALL_OPERATION_METADATA {
        for feature in metadata.required_features.iter().copied() {
            let out_of_scope = CATALOG_FEATURES_OUTSIDE_THE_DAEMON_SCOPE
                .iter()
                .any(|(name, _)| *name == feature);
            if out_of_scope {
                continue;
            }
            names.push(feature);
        }
    }
    names.sort_unstable();
    names.dedup();
    names
}

/// The engine features the daemon's profile must enable.
///
/// The union of the catalog-derived half (the distinct non-empty
/// `required_features` of [`eggsec_policy::ALL_OPERATION_METADATA`], minus
/// the catalog features the daemon does not dispatch) with the explicitly
/// declared members.
///
/// The result is sorted and deduplicated so the function is deterministic.
pub fn required_profile_features() -> Vec<&'static str> {
    let mut names = catalog_required_features();
    names.extend(DECLARED_MEMBERS.iter().map(|(name, _)| *name));
    names.sort_unstable();
    names.dedup();
    names
}

/// A single way a candidate profile fails the conformance check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileViolation {
    /// The name is not an engine feature at all.
    UnknownFeature {
        /// The offending name.
        feature: String,
    },
    /// The name is an engine feature the daemon profile must never enable.
    ForbiddenFeature {
        /// The offending name.
        feature: String,
        /// Why the feature is absolutely excluded.
        reason: &'static str,
    },
    /// A feature the profile requires is absent from the candidate list.
    MissingRequiredFeature {
        /// The missing feature.
        feature: &'static str,
    },
    /// The candidate list names the same feature more than once.
    DuplicateFeature {
        /// The repeated name.
        feature: String,
    },
    /// The candidate list enables a feature that is neither required by the
    /// profile nor one of the explicitly declared members.
    UnrequiredFeature {
        /// The offending name.
        feature: String,
    },
}

impl fmt::Display for ProfileViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFeature { feature } => {
                write!(f, "unknown engine feature '{feature}'")
            }
            Self::ForbiddenFeature { feature, reason } => {
                write!(f, "forbidden engine feature '{feature}': {reason}")
            }
            Self::MissingRequiredFeature { feature } => {
                write!(f, "missing required engine feature '{feature}'")
            }
            Self::DuplicateFeature { feature } => {
                write!(f, "duplicate engine feature '{feature}'")
            }
            Self::UnrequiredFeature { feature } => {
                write!(
                    f,
                    "engine feature '{feature}' is neither required by the profile nor a declared member"
                )
            }
        }
    }
}

/// Check a candidate profile against the daemon's feature contract.
///
/// The function is pure and deterministic: it reads only the registry, the
/// profile constants and the policy catalog, and it never touches the
/// filesystem, the network, or any mutable state. It reports every way the
/// candidate list fails:
///
/// - an unknown name (not in the engine's registry),
/// - a forbidden name,
/// - a missing required name,
/// - a duplicate entry,
/// - a name that is neither required nor a declared member.
///
/// The violations are returned in a stable order: one entry per offending
/// candidate in candidate order, followed by the missing required features in
/// sorted order.
pub fn profile_violations(profile: &[&str]) -> Vec<ProfileViolation> {
    let required = required_profile_features();
    let mut violations = Vec::new();
    let mut seen: Vec<&str> = Vec::new();

    for feature in profile {
        if seen.contains(feature) {
            violations.push(ProfileViolation::DuplicateFeature {
                feature: (*feature).to_string(),
            });
            continue;
        }
        seen.push(feature);

        if !is_known_feature(feature) {
            violations.push(ProfileViolation::UnknownFeature {
                feature: (*feature).to_string(),
            });
            continue;
        }

        if DAEMON_SERVICE_FORBIDDEN.contains(feature) {
            violations.push(ProfileViolation::ForbiddenFeature {
                feature: (*feature).to_string(),
                reason: "the daemon profile must never enable this feature",
            });
            continue;
        }

        if !required.contains(feature) {
            violations.push(ProfileViolation::UnrequiredFeature {
                feature: (*feature).to_string(),
            });
        }
    }

    for feature in &required {
        if !profile.contains(feature) {
            violations.push(ProfileViolation::MissingRequiredFeature { feature });
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::feature_registry::ALL_FEATURES;

    #[test]
    fn the_documented_profile_has_no_violations() {
        assert_eq!(
            profile_violations(DAEMON_SERVICE_PROFILE),
            Vec::<ProfileViolation>::new()
        );
    }

    #[test]
    fn the_documented_profile_excludes_nothing_it_claims_to_exclude() {
        for feature in DAEMON_SERVICE_PROFILE {
            assert!(
                !DAEMON_SERVICE_EXCLUDED.contains(feature),
                "feature '{feature}' is both included and excluded by the daemon profile"
            );
        }

        let registry: Vec<&str> = ALL_FEATURES.iter().map(|entry| entry.name).collect();
        let mut classified: Vec<&str> = DAEMON_SERVICE_PROFILE
            .iter()
            .chain(DAEMON_SERVICE_EXCLUDED.iter())
            .copied()
            .collect();
        classified.sort_unstable();
        classified.dedup();

        let mut expected = registry.clone();
        expected.sort_unstable();
        assert_eq!(
            classified, expected,
            "the included and excluded sets must partition the engine's registry"
        );
    }

    #[test]
    fn the_required_set_is_derived_from_the_policy_catalog() {
        // The catalog-derived half must be exactly the distinct non-empty
        // `required_features` of the catalog, minus the features the daemon
        // does not dispatch. A hand-written list would not survive a new
        // catalog entry.
        let mut expected: Vec<&str> = eggsec_policy::ALL_OPERATION_METADATA
            .iter()
            .flat_map(|metadata| metadata.required_features.iter().copied())
            .filter(|feature| {
                !CATALOG_FEATURES_OUTSIDE_THE_DAEMON_SCOPE
                    .iter()
                    .any(|(name, _)| name == feature)
            })
            .collect();
        expected.sort_unstable();
        expected.dedup();

        assert_eq!(catalog_required_features(), expected);

        // And the declared members are exactly the non-catalog remainder.
        let mut declared: Vec<&str> = DECLARED_MEMBERS.iter().map(|(name, _)| *name).collect();
        declared.sort_unstable();
        let mut all: Vec<&str> = expected.clone();
        all.extend(declared.iter().copied());
        all.sort_unstable();
        all.dedup();
        assert_eq!(required_profile_features(), all);
    }

    #[test]
    fn the_required_set_contains_the_cli_host_gate() {
        assert!(required_profile_features().contains(&"cli"));
        let catalog_names: Vec<&str> = eggsec_policy::ALL_OPERATION_METADATA
            .iter()
            .flat_map(|metadata| metadata.required_features.iter().copied())
            .collect();
        assert!(
            !catalog_names.contains(&"cli"),
            "`cli` is a host gate, not a catalog requirement"
        );
    }

    #[test]
    fn the_documented_profile_covers_the_required_set() {
        for feature in required_profile_features() {
            assert!(
                DAEMON_SERVICE_PROFILE.contains(&feature),
                "required feature '{feature}' is missing from the daemon profile"
            );
        }
    }

    #[test]
    fn the_transitive_members_are_declared_explicitly() {
        for member in ["postex", "evasion", "tool-api"] {
            assert!(
                DAEMON_SERVICE_PROFILE.contains(&member),
                "declared transitive member '{member}' is missing from the daemon profile"
            );
            assert!(
                DECLARED_MEMBERS.iter().any(|(name, _)| *name == member),
                "transitive member '{member}' is not declared with a reason"
            );
        }
    }

    #[test]
    fn the_profile_enables_no_forbidden_feature() {
        for feature in DAEMON_SERVICE_PROFILE {
            assert!(
                !DAEMON_SERVICE_FORBIDDEN.contains(feature),
                "forbidden feature '{feature}' is enabled by the daemon profile"
            );
        }
    }

    #[test]
    fn a_profile_with_an_unknown_feature_is_rejected() {
        let violations = profile_violations(&["not-a-real-feature"]);
        assert!(
            violations.iter().any(|v| matches!(
                v,
                ProfileViolation::UnknownFeature { feature } if feature == "not-a-real-feature"
            )),
            "expected an unknown-feature violation, got {violations:?}"
        );
    }

    #[test]
    fn a_profile_with_a_forbidden_feature_is_rejected() {
        let violations = profile_violations(&["insecure-tls"]);
        assert!(
            violations.iter().any(|v| matches!(
                v,
                ProfileViolation::ForbiddenFeature { feature, .. } if feature == "insecure-tls"
            )),
            "expected a forbidden-feature violation, got {violations:?}"
        );
    }

    #[test]
    fn a_profile_missing_a_required_feature_is_rejected() {
        let candidate: Vec<&str> = DAEMON_SERVICE_PROFILE
            .iter()
            .copied()
            .filter(|feature| *feature != "nse")
            .collect();
        let violations = profile_violations(&candidate);
        assert!(
            violations.iter().any(|v| matches!(
                v,
                ProfileViolation::MissingRequiredFeature { feature } if *feature == "nse"
            )),
            "expected a missing-required-feature violation, got {violations:?}"
        );
    }

    #[test]
    fn a_profile_with_a_duplicate_feature_is_rejected() {
        let mut candidate: Vec<&str> = DAEMON_SERVICE_PROFILE.to_vec();
        candidate.push("nse");
        let violations = profile_violations(&candidate);
        assert!(
            violations.iter().any(|v| matches!(
                v,
                ProfileViolation::DuplicateFeature { feature } if feature == "nse"
            )),
            "expected a duplicate-feature violation, got {violations:?}"
        );
    }

    #[test]
    fn a_profile_with_an_unrequired_feature_is_rejected() {
        let mut candidate: Vec<&str> = DAEMON_SERVICE_PROFILE.to_vec();
        candidate.push("sbom");
        let violations = profile_violations(&candidate);
        assert!(
            violations.iter().any(|v| matches!(
                v,
                ProfileViolation::UnrequiredFeature { feature } if feature == "sbom"
            )),
            "expected an unrequired-feature violation, got {violations:?}"
        );
    }

    #[test]
    fn violations_name_the_offending_feature() {
        let violations = profile_violations(&["rest-api"]);
        assert!(
            violations
                .iter()
                .any(|v| v.to_string().contains("rest-api")),
            "violation text must name the offending feature, got {violations:?}"
        );
    }
}
