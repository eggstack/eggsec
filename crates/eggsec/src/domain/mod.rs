//! Domain Module Contract
//!
//! Defines [`DomainDescriptor`] — a static metadata contract that describes
//! what a capability domain can do, how it integrates with CLI/TUI/MCP/tool
//! surfaces, and what feature gates control its availability.
//!
//! ## Design Principle
//!
//! A domain may declare what it can do. A domain may execute already-approved
//! work. A domain must not decide whether work is authorized.
//!
//! Central enforcement remains in the main `eggsec` crate or a future
//! dedicated policy crate.
//!
//! ## Placement
//!
//! This module lives in the main `eggsec` crate for Phase 3 piloting.
//! It may later move to a dedicated `eggsec-domain-core` or
//! `eggsec-policy-core` crate if extraction proves beneficial.

use crate::config::{Capability, IntendedUse, OperationMode, OperationRisk, TargetPolicyKind};

/// Categories of domains, used for classification and display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainCategory {
    /// Standard scoped assessment (recon, scanning, fuzzing, API testing).
    StandardAssessment,
    /// Local/private defense validation and regression testing.
    DefenseLab,
    /// High-risk operations requiring explicit authorization.
    HazardousLab,
    /// Adapter that bridges external protocols (REST, MCP, gRPC).
    FrontendAdapter,
    /// Adapter that produces output formats (reports, exports).
    OutputAdapter,
}

impl DomainCategory {
    /// Returns a human-readable label for the category.
    pub fn label(self) -> &'static str {
        match self {
            Self::StandardAssessment => "standard assessment",
            Self::DefenseLab => "defense lab",
            Self::HazardousLab => "hazardous lab",
            Self::FrontendAdapter => "frontend adapter",
            Self::OutputAdapter => "output adapter",
        }
    }
}

impl std::fmt::Display for DomainCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StandardAssessment => write!(f, "standard-assessment"),
            Self::DefenseLab => write!(f, "defense-lab"),
            Self::HazardousLab => write!(f, "hazardous-lab"),
            Self::FrontendAdapter => write!(f, "frontend-adapter"),
            Self::OutputAdapter => write!(f, "output-adapter"),
        }
    }
}

/// Describes how a single operation within a domain integrates with the system.
#[derive(Debug, Clone, Copy)]
pub struct OperationIntegration {
    /// Canonical operation ID (must match an `OperationMetadata.id`).
    pub operation_id: &'static str,
    /// Human-readable display name.
    pub display_name: &'static str,
    /// Operating mode for this operation.
    pub mode: OperationMode,
    /// Risk tier of this operation.
    pub risk: OperationRisk,
    /// Capabilities required by this operation.
    pub capabilities: &'static [Capability],
    /// Intended use cases.
    pub intended_uses: &'static [IntendedUse],
    /// Feature flags required to compile this operation.
    pub required_features: &'static [&'static str],
    /// Whether an explicit scope file is required.
    pub requires_explicit_scope: bool,
    /// Whether the target must be a private/local address.
    pub requires_private_or_local_target: bool,
}

/// Describes how a domain's operation maps to a CLI command.
#[derive(Debug, Clone, Copy)]
pub struct CliIntegration {
    /// CLI command ID (e.g. "db-pentest").
    pub command_id: &'static str,
    /// Operation ID this command invokes.
    pub operation_id: &'static str,
    /// Feature flag required for this CLI command (if any).
    pub feature: Option<&'static str>,
}

/// Describes how a domain's operation maps to a TUI tab.
#[derive(Debug, Clone, Copy)]
pub struct TuiIntegration {
    /// TUI tab identifier.
    pub tab_id: &'static str,
    /// Operation ID this tab invokes.
    pub operation_id: &'static str,
    /// Feature flag required for this TUI tab (if any).
    pub feature: Option<&'static str>,
}

/// Describes how a domain's operation maps to a tool (MCP/REST/gRPC).
#[derive(Debug, Clone, Copy)]
pub struct ToolIntegration {
    /// Tool ID used in MCP/REST/gRPC registration.
    pub tool_id: &'static str,
    /// Operation ID this tool invokes.
    pub operation_id: &'static str,
    /// Whether this tool is exposed via MCP by default.
    pub mcp_exposed_by_default: bool,
    /// Feature flag required for MCP exposure (if any).
    pub required_mcp_feature: Option<&'static str>,
}

/// Describes how a domain's operation maps to report output.
#[derive(Debug, Clone, Copy)]
pub struct ReportIntegration {
    /// Report kind identifier (e.g. "db-pentest", "web-proxy").
    pub report_kind: &'static str,
    /// Operation ID that produces this report.
    pub operation_id: &'static str,
    /// Whether evidence bundles are supported for this report.
    pub evidence_bundle_supported: bool,
}

/// Whether dry-run mode is supported by this domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DryRunSupport {
    /// Dry-run is always safe and available.
    AlwaysAvailable,
    /// Dry-run is available when a specific feature is enabled.
    FeatureGated(&'static str),
    /// Dry-run is not supported.
    NotSupported,
}

/// Whether evidence bundle export is supported by this domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSupport {
    /// Evidence bundles are always supported.
    AlwaysAvailable,
    /// Evidence bundles are available when a specific feature is enabled.
    FeatureGated(&'static str),
    /// Evidence bundles are not supported.
    NotSupported,
}

/// Whether baseline capture and regression comparison is supported by this domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineSupport {
    /// Baseline/regression is always available.
    AlwaysAvailable,
    /// Baseline/regression is available when a specific feature is enabled.
    FeatureGated(&'static str),
    /// Baseline/regression is not supported.
    NotSupported,
}

/// A single row in the capability matrix, generated from domain and operation metadata.
#[derive(Debug, Clone)]
pub struct CapabilityMatrixRow {
    /// Domain ID.
    pub domain_id: &'static str,
    /// Domain display name.
    pub domain_name: &'static str,
    /// Domain category.
    pub category: DomainCategory,
    /// Operation ID.
    pub operation_id: &'static str,
    /// Operation display name.
    pub operation_name: &'static str,
    /// Cargo feature gate (if any).
    pub feature: Option<&'static str>,
    /// Risk tier.
    pub risk: OperationRisk,
    /// Capabilities.
    pub capabilities: &'static [Capability],
    /// Target policy kind from OperationMetadata.
    pub target_policy: TargetPolicyKind,
    /// CLI exposure.
    pub cli: bool,
    /// TUI exposure.
    pub tui: bool,
    /// MCP/API exposure.
    pub mcp_api: bool,
    /// Dry-run support description.
    pub dry_run: &'static str,
    /// Evidence/report support description.
    pub evidence_report: &'static str,
    /// Baseline/regression support description.
    pub baseline: &'static str,
    /// Whether strict surfaces (MCP/Agent/REST/gRPC) support this domain.
    pub strict_surface: bool,
    /// Scope requirement description.
    pub scope_requirement: &'static str,
    /// Optional documentation URL or path.
    pub docs_url: Option<&'static str>,
    /// Notes.
    pub notes: &'static str,
}

/// A static descriptor for a capability domain.
///
/// `DomainDescriptor` declares what a domain can do without performing
/// authorization or execution. It is the central type in the domain
/// integration contract.
///
/// # Safety Invariants
///
/// - Descriptors do not authorize anything.
/// - Descriptors do not perform network I/O.
/// - Descriptors are constructed from static data only.
/// - MCP-exposed domains must not be hazardous by default.
#[derive(Debug, Clone, Copy)]
pub struct DomainDescriptor {
    /// Unique domain identifier (e.g. "db-pentest", "web-proxy").
    pub id: &'static str,
    /// Human-readable display name.
    pub display_name: &'static str,
    /// Brief description of the domain's purpose.
    pub description: &'static str,
    /// Classification category.
    pub category: DomainCategory,
    /// Cargo feature flag required to compile this domain (if any).
    pub required_feature: Option<&'static str>,
    /// Operations provided by this domain.
    pub operations: &'static [OperationIntegration],
    /// CLI command integrations.
    pub cli: &'static [CliIntegration],
    /// TUI tab integrations.
    pub tui: &'static [TuiIntegration],
    /// Tool (MCP/REST/gRPC) integrations.
    pub tools: &'static [ToolIntegration],
    /// Report output integrations.
    pub reports: &'static [ReportIntegration],
    /// Dry-run support level.
    pub dry_run: DryRunSupport,
    /// Evidence bundle support level.
    pub evidence: EvidenceSupport,
    /// Baseline capture and regression comparison support.
    pub baseline: BaselineSupport,
    /// Whether strict surfaces (MCP/Agent/REST/gRPC) support this domain.
    pub strict_surface_support: bool,
    /// Optional documentation URL or path (e.g. "docs/DATABASE_PENTEST.md").
    pub docs_url: Option<&'static str>,
}

/// Returns the static set of all known domain descriptors.
///
/// The returned slice is ordered by category (StandardAssessment first,
/// then DefenseLab, then HazardousLab, then adapters). Each descriptor
/// reflects the current feature set — domains behind disabled features
/// are still included in the registry (their `required_feature` field
/// indicates gating), but consumers should check feature availability
/// before attempting to use them.
pub fn all_domain_descriptors() -> &'static [DomainDescriptor] {
    &[
        // ── Standard Assessment ──
        // (future: scanner, fuzzer, waf, recon, etc.)

        // ── Defense Lab ──
        #[cfg(feature = "db-pentest")]
        DB_PENTEST_DESCRIPTOR,
        // (future: web-proxy, evasion, postex, etc.)

        // ── Hazardous Lab ──
        // (future: stress, c2, etc.)

        // ── Adapters ──
        // (future: output adapters, frontend adapters)
    ]
}

/// Look up a domain descriptor by its ID.
pub fn domain_descriptor_by_id(id: &str) -> Option<&'static DomainDescriptor> {
    all_domain_descriptors().iter().find(|d| d.id == id)
}

/// Generate capability matrix rows from all registered domain descriptors.
///
/// Each operation in each domain produces one row. Operations not in any
/// domain (standalone operations from `ALL_OPERATION_METADATA`) are not
/// included — use `all_operation_metadata()` for a complete operation list.
pub fn generate_capability_matrix() -> Vec<CapabilityMatrixRow> {
    let mut rows = Vec::new();
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            let dry_run = match domain.dry_run {
                DryRunSupport::AlwaysAvailable => "always",
                DryRunSupport::FeatureGated(f) => f,
                DryRunSupport::NotSupported => "no",
            };
            let evidence_report = match domain.evidence {
                EvidenceSupport::AlwaysAvailable => "always",
                EvidenceSupport::FeatureGated(f) => f,
                EvidenceSupport::NotSupported => "no",
            };
            let baseline = match domain.baseline {
                BaselineSupport::AlwaysAvailable => "always",
                BaselineSupport::FeatureGated(f) => f,
                BaselineSupport::NotSupported => "no",
            };
            let scope = if op.requires_explicit_scope {
                "explicit scope required"
            } else if op.requires_private_or_local_target {
                "private/local target"
            } else {
                "standard scope"
            };
            let has_cli = domain.cli.iter().any(|c| c.operation_id == op.operation_id);
            let has_tui = domain.tui.iter().any(|t| t.operation_id == op.operation_id);
            let has_mcp = domain
                .tools
                .iter()
                .any(|t| t.operation_id == op.operation_id);

            let feature = if op.required_features.is_empty() {
                domain.required_feature
            } else {
                Some(op.required_features[0])
            };

            // Derive target policy from scope requirements.
            let target_policy = if op.requires_private_or_local_target {
                TargetPolicyKind::PrivateOrLocalRequired
            } else if op.requires_explicit_scope {
                TargetPolicyKind::ExplicitScopeRequired
            } else {
                TargetPolicyKind::OptionalTarget
            };

            let docs_url = domain.docs_url;
            let notes = "";
            rows.push(CapabilityMatrixRow {
                domain_id: domain.id,
                domain_name: domain.display_name,
                category: domain.category,
                operation_id: op.operation_id,
                operation_name: op.display_name,
                feature,
                risk: op.risk,
                capabilities: op.capabilities,
                target_policy,
                cli: has_cli,
                tui: has_tui,
                mcp_api: has_mcp,
                dry_run,
                evidence_report,
                baseline,
                strict_surface: domain.strict_surface_support,
                scope_requirement: scope,
                docs_url,
                notes,
            });
        }
    }
    rows
}

// ─── Pilot Domain: db-pentest ───────────────────────────────────────────────
//
// These constants are only referenced when the `db-pentest` feature is enabled
// (in the registry and tests). Allow dead_code for no-default-features builds.
#[allow(dead_code)]
/// Static operation integration for the `db-pentest` operation.
const DB_PENTEST_OPERATION: OperationIntegration = OperationIntegration {
    operation_id: "db-pentest",
    display_name: "Database Pentesting",
    mode: OperationMode::DefenseLab,
    risk: OperationRisk::DbPentest,
    capabilities: &[Capability::DatabaseAssessment],
    intended_uses: &[IntendedUse::WebAssessment],
    required_features: &["db-pentest"],
    requires_explicit_scope: true,
    requires_private_or_local_target: false,
};

#[allow(dead_code)]
/// CLI integration for db-pentest.
const DB_PENTEST_CLI: CliIntegration = CliIntegration {
    command_id: "db-pentest",
    operation_id: "db-pentest",
    feature: Some("db-pentest"),
};

#[allow(dead_code)]
/// TUI integration for db-pentest.
const DB_PENTEST_TUI: TuiIntegration = TuiIntegration {
    tab_id: "db-pentest",
    operation_id: "db-pentest",
    feature: Some("db-pentest"),
};

#[allow(dead_code)]
/// Tool integration for db-pentest (MCP/REST/gRPC).
const DB_PENTEST_TOOL: ToolIntegration = ToolIntegration {
    tool_id: "db-pentest",
    operation_id: "db-pentest",
    mcp_exposed_by_default: false,
    required_mcp_feature: Some("db-pentest-mcp"),
};

#[allow(dead_code)]
/// Report integration for db-pentest.
const DB_PENTEST_REPORT: ReportIntegration = ReportIntegration {
    report_kind: "db-pentest",
    operation_id: "db-pentest",
    evidence_bundle_supported: true,
};

#[allow(dead_code)]
/// Static domain descriptor for the db-pentest pilot domain.
const DB_PENTEST_DESCRIPTOR: DomainDescriptor = DomainDescriptor {
    id: "db-pentest",
    display_name: "Database Pentesting",
    description: "Direct Postgres/MySQL/MSSQL/MongoDB/Redis security checks for authorized lab use",
    category: DomainCategory::DefenseLab,
    required_feature: Some("db-pentest"),
    operations: &[DB_PENTEST_OPERATION],
    cli: &[DB_PENTEST_CLI],
    tui: &[DB_PENTEST_TUI],
    tools: &[DB_PENTEST_TOOL],
    reports: &[DB_PENTEST_REPORT],
    dry_run: DryRunSupport::AlwaysAvailable,
    evidence: EvidenceSupport::AlwaysAvailable,
    baseline: BaselineSupport::AlwaysAvailable,
    strict_surface_support: true,
    docs_url: Some("docs/DATABASE_PENTEST.md"),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_category_label_is_stable() {
        assert_eq!(
            DomainCategory::StandardAssessment.label(),
            "standard assessment"
        );
        assert_eq!(DomainCategory::DefenseLab.label(), "defense lab");
        assert_eq!(DomainCategory::HazardousLab.label(), "hazardous lab");
        assert_eq!(DomainCategory::FrontendAdapter.label(), "frontend adapter");
        assert_eq!(DomainCategory::OutputAdapter.label(), "output adapter");
    }

    #[test]
    fn domain_category_display_is_kebab_case() {
        assert_eq!(
            DomainCategory::StandardAssessment.to_string(),
            "standard-assessment"
        );
        assert_eq!(DomainCategory::DefenseLab.to_string(), "defense-lab");
        assert_eq!(DomainCategory::HazardousLab.to_string(), "hazardous-lab");
        assert_eq!(
            DomainCategory::FrontendAdapter.to_string(),
            "frontend-adapter"
        );
        assert_eq!(DomainCategory::OutputAdapter.to_string(), "output-adapter");
    }

    #[test]
    fn dry_run_support_equality() {
        assert_eq!(
            DryRunSupport::AlwaysAvailable,
            DryRunSupport::AlwaysAvailable
        );
        assert_eq!(
            DryRunSupport::FeatureGated("x"),
            DryRunSupport::FeatureGated("x")
        );
        assert_ne!(DryRunSupport::AlwaysAvailable, DryRunSupport::NotSupported);
    }

    #[test]
    fn evidence_support_equality() {
        assert_eq!(
            EvidenceSupport::AlwaysAvailable,
            EvidenceSupport::AlwaysAvailable
        );
        assert_eq!(
            EvidenceSupport::FeatureGated("x"),
            EvidenceSupport::FeatureGated("x")
        );
        assert_ne!(
            EvidenceSupport::AlwaysAvailable,
            EvidenceSupport::NotSupported
        );
    }

    #[cfg(feature = "db-pentest")]
    mod db_pentest_tests {
        use super::*;

        #[test]
        fn db_pentest_descriptor_exists() {
            let d = DB_PENTEST_DESCRIPTOR;
            assert_eq!(d.id, "db-pentest");
            assert_eq!(d.display_name, "Database Pentesting");
        }

        #[test]
        fn db_pentest_category_is_defense_lab() {
            assert_eq!(DB_PENTEST_DESCRIPTOR.category, DomainCategory::DefenseLab);
        }

        #[test]
        fn db_pentest_requires_db_pentest_feature() {
            assert_eq!(DB_PENTEST_DESCRIPTOR.required_feature, Some("db-pentest"));
        }

        #[test]
        fn db_pentest_has_one_operation() {
            assert_eq!(DB_PENTEST_DESCRIPTOR.operations.len(), 1);
            assert_eq!(
                DB_PENTEST_DESCRIPTOR.operations[0].operation_id,
                "db-pentest"
            );
        }

        #[test]
        fn db_pentest_operation_risk_is_db_pentest() {
            assert_eq!(
                DB_PENTEST_DESCRIPTOR.operations[0].risk,
                OperationRisk::DbPentest
            );
        }

        #[test]
        fn db_pentest_operation_mode_is_defense_lab() {
            assert_eq!(
                DB_PENTEST_DESCRIPTOR.operations[0].mode,
                OperationMode::DefenseLab
            );
        }

        #[test]
        fn db_pentest_requires_database_assessment_capability() {
            assert!(DB_PENTEST_DESCRIPTOR.operations[0]
                .capabilities
                .contains(&Capability::DatabaseAssessment));
        }

        #[test]
        fn db_pentest_requires_explicit_scope() {
            assert!(DB_PENTEST_DESCRIPTOR.operations[0].requires_explicit_scope);
        }

        #[test]
        fn db_pentest_mcp_not_exposed_by_default() {
            assert!(!DB_PENTEST_DESCRIPTOR.tools[0].mcp_exposed_by_default);
        }

        #[test]
        fn db_pentest_mcp_requires_feature() {
            assert_eq!(
                DB_PENTEST_DESCRIPTOR.tools[0].required_mcp_feature,
                Some("db-pentest-mcp")
            );
        }

        #[test]
        fn db_pentest_dry_run_always_available() {
            assert_eq!(
                DB_PENTEST_DESCRIPTOR.dry_run,
                DryRunSupport::AlwaysAvailable
            );
        }

        #[test]
        fn db_pentest_evidence_always_available() {
            assert_eq!(
                DB_PENTEST_DESCRIPTOR.evidence,
                EvidenceSupport::AlwaysAvailable
            );
        }

        #[test]
        fn db_pentest_has_cli_integration() {
            assert_eq!(DB_PENTEST_DESCRIPTOR.cli.len(), 1);
            assert_eq!(DB_PENTEST_DESCRIPTOR.cli[0].command_id, "db-pentest");
        }

        #[test]
        fn db_pentest_has_tui_integration() {
            assert_eq!(DB_PENTEST_DESCRIPTOR.tui.len(), 1);
            assert_eq!(DB_PENTEST_DESCRIPTOR.tui[0].tab_id, "db-pentest");
        }

        #[test]
        fn db_pentest_has_report_integration() {
            assert_eq!(DB_PENTEST_DESCRIPTOR.reports.len(), 1);
            assert!(DB_PENTEST_DESCRIPTOR.reports[0].evidence_bundle_supported);
        }

        #[test]
        fn registry_includes_db_pentest() {
            let domains = all_domain_descriptors();
            assert!(domains.iter().any(|d| d.id == "db-pentest"));
        }

        #[test]
        fn lookup_by_id_works() {
            let d = domain_descriptor_by_id("db-pentest");
            assert!(d.is_some());
            assert_eq!(d.unwrap().id, "db-pentest");
        }

        #[test]
        fn lookup_missing_id_returns_none() {
            assert!(domain_descriptor_by_id("nonexistent").is_none());
        }

        #[test]
        fn descriptor_is_const_constructible() {
            // Proves the descriptor can be built at compile time.
            const _: DomainDescriptor = DB_PENTEST_DESCRIPTOR;
        }

        #[test]
        fn descriptor_does_not_authorize() {
            // The descriptor is purely metadata — it contains no enforcement
            // logic, no scope checking, no policy evaluation. This test
            // documents that invariant by construction: the type has no
            // methods that perform authorization.
            let d = DB_PENTEST_DESCRIPTOR;
            // Only metadata accessors are available.
            assert!(!d.id.is_empty());
            assert!(!d.operations.is_empty());
        }
    }

    #[test]
    fn all_domain_operation_ids_have_metadata() {
        use crate::config::metadata_for_tool_id;
        for domain in all_domain_descriptors() {
            for op in domain.operations {
                assert!(
                    metadata_for_tool_id(op.operation_id).is_some(),
                    "domain '{}' operation '{}' has no matching OperationMetadata",
                    domain.id,
                    op.operation_id
                );
            }
        }
    }

    #[test]
    fn domain_ids_are_unique() {
        let mut seen = rustc_hash::FxHashSet::default();
        for domain in all_domain_descriptors() {
            assert!(seen.insert(domain.id), "duplicate domain id: {}", domain.id);
        }
    }

    #[test]
    fn domain_operation_ids_within_domain_are_unique() {
        for domain in all_domain_descriptors() {
            let mut seen = rustc_hash::FxHashSet::default();
            for op in domain.operations {
                assert!(
                    seen.insert(op.operation_id),
                    "domain '{}' has duplicate operation id: {}",
                    domain.id,
                    op.operation_id
                );
            }
        }
    }

    #[test]
    fn capability_matrix_generation_works() {
        let rows = generate_capability_matrix();
        // With no features enabled the domain registry is empty.
        // With db-pentest enabled, at least one row should appear.
        // Either way, every row produced should have non-empty fields.
        for row in &rows {
            assert!(!row.domain_id.is_empty());
            assert!(!row.operation_id.is_empty());
            assert!(!row.operation_name.is_empty());
        }
        // When db-pentest is enabled, rows should be non-empty.
        #[cfg(feature = "db-pentest")]
        assert!(
            !rows.is_empty(),
            "capability matrix should have rows with db-pentest enabled"
        );
    }

    #[test]
    fn capability_matrix_pilot_domain_row_present() {
        #[cfg(feature = "db-pentest")]
        {
            let rows = generate_capability_matrix();
            let db_row = rows.iter().find(|r| r.operation_id == "db-pentest");
            assert!(
                db_row.is_some(),
                "db-pentest should appear in capability matrix"
            );
            let row = db_row.unwrap();
            assert_eq!(row.domain_id, "db-pentest");
            assert_eq!(row.category, DomainCategory::DefenseLab);
            assert!(row.dry_run == "always");
            assert!(row.evidence_report == "always");
        }
    }
}
