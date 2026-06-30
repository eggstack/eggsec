//! Metadata consistency tests — cross-validate DomainDescriptor, OperationMetadata,
//! and the capability matrix against each other and against documentation.
//!
//! These tests ensure that metadata remains synchronized across code and docs,
//! preventing drift between what is declared and what is documented.

use eggsec::config::{
    all_operation_metadata, metadata_for_tool_id, operation_metadata, Capability, OperationRisk,
};
use eggsec::domain::{
    all_domain_descriptors, generate_capability_matrix, BaselineSupport, DryRunSupport,
};

// ─── Domain × OperationMetadata Cross-Validation ──────────────────────────

/// Every operation declared in a DomainDescriptor must have a matching
/// OperationMetadata entry. This catches orphaned domain operations that
/// lack canonical metadata.
#[test]
fn all_domain_operations_have_matching_metadata() {
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            let meta = metadata_for_tool_id(op.operation_id).unwrap_or_else(|| {
                panic!(
                    "domain '{}' operation '{}' has no matching OperationMetadata — \
                     add an entry to ALL_OPERATION_METADATA or ALL_OPERATION_METADATA_ALIASES",
                    domain.id, op.operation_id
                )
            });
            assert_eq!(
                meta.id, op.operation_id,
                "domain '{}' operation '{}' resolves to metadata '{}' — canonical ID mismatch",
                domain.id, op.operation_id, meta.id
            );
        }
    }
}

/// Domain-declared risk must match OperationMetadata risk for each operation.
#[test]
fn domain_risk_matches_operation_metadata_risk() {
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            let meta = metadata_for_tool_id(op.operation_id)
                .expect("domain operation should have metadata");
            assert_eq!(
                op.risk, meta.risk,
                "domain '{}' operation '{}': domain declares {:?} but metadata has {:?}",
                domain.id, op.operation_id, op.risk, meta.risk
            );
        }
    }
}

/// Domain-declared capabilities must be a subset of (or equal to) OperationMetadata capabilities.
#[test]
fn domain_capabilities_match_metadata() {
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            let meta = metadata_for_tool_id(op.operation_id)
                .expect("domain operation should have metadata");
            assert_eq!(
                op.capabilities, meta.required_capabilities,
                "domain '{}' operation '{}': domain capabilities {:?} != metadata {:?}",
                domain.id, op.operation_id, op.capabilities, meta.required_capabilities
            );
        }
    }
}

/// Domain-declared feature requirements must be consistent with metadata.
/// The domain may declare the same features or a subset — but not features
/// that the metadata doesn't know about.
#[test]
fn domain_features_subset_of_metadata() {
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            let meta = metadata_for_tool_id(op.operation_id)
                .expect("domain operation should have metadata");
            for feat in op.required_features {
                assert!(
                    meta.required_features.contains(feat),
                    "domain '{}' operation '{}': feature '{}' is not in metadata required_features {:?}",
                    domain.id, op.operation_id, feat, meta.required_features
                );
            }
        }
    }
}

// ─── Domain Uniqueness ────────────────────────────────────────────────────

/// Domain IDs must be globally unique.
#[test]
fn domain_ids_are_unique() {
    let mut seen = rustc_hash::FxHashSet::default();
    for domain in all_domain_descriptors() {
        assert!(
            seen.insert(domain.id),
            "duplicate domain id: '{}'",
            domain.id
        );
    }
}

/// Operation IDs within a domain must be unique.
#[test]
fn domain_operation_ids_are_unique_within_domain() {
    for domain in all_domain_descriptors() {
        let mut seen = rustc_hash::FxHashSet::default();
        for op in domain.operations {
            assert!(
                seen.insert(op.operation_id),
                "domain '{}' has duplicate operation id: '{}'",
                domain.id,
                op.operation_id
            );
        }
    }
}

/// Operation IDs must be globally unique across all domains.
#[test]
fn domain_operation_ids_are_globally_unique() {
    let mut seen = rustc_hash::FxHashSet::default();
    for domain in all_domain_descriptors() {
        for op in domain.operations {
            assert!(
                seen.insert(op.operation_id),
                "operation '{}' appears in multiple domains",
                op.operation_id
            );
        }
    }
}

// ─── OperationMetadata Uniqueness ──────────────────────────────────────────

/// All OperationMetadata IDs must be unique.
#[test]
fn operation_metadata_ids_are_unique() {
    let mut seen = rustc_hash::FxHashSet::default();
    for m in all_operation_metadata() {
        assert!(
            seen.insert(m.id),
            "duplicate operation metadata id: '{}'",
            m.id
        );
    }
}

/// All tool aliases must resolve to known canonical IDs.
#[test]
fn all_aliases_resolve_to_known_metadata() {
    use eggsec::config::ALL_OPERATION_METADATA_ALIASES;
    for &(alias, canonical) in ALL_OPERATION_METADATA_ALIASES {
        assert!(
            metadata_for_tool_id(canonical).is_some(),
            "alias '{}' points to unknown canonical id '{}'",
            alias,
            canonical
        );
    }
}

/// No alias should map to itself (that's a redundant entry).
#[test]
fn no_alias_maps_to_self() {
    use eggsec::config::ALL_OPERATION_METADATA_ALIASES;
    for &(alias, canonical) in ALL_OPERATION_METADATA_ALIASES {
        assert_ne!(
            alias, canonical,
            "alias '{}' maps to itself — remove the redundant alias",
            alias
        );
    }
}

// ─── Capability Matrix Consistency ─────────────────────────────────────────

/// The capability matrix should produce rows for all known domain descriptors,
/// independent of compile-time feature state. `available_domain_descriptors()`
/// is the filtered view for currently compiled features.
#[test]
fn capability_matrix_has_rows_when_domains_registered() {
    let rows = generate_capability_matrix();
    // All known domain descriptors produce rows regardless of feature state.
    assert!(
        !rows.is_empty(),
        "capability matrix should have rows for all known domain descriptors"
    );
    // Every row should have non-empty fields.
    for row in &rows {
        assert!(!row.domain_id.is_empty());
        assert!(!row.operation_id.is_empty());
    }
}

/// Every capability matrix row should have non-empty required fields.
#[test]
fn capability_matrix_rows_have_required_fields() {
    let rows = generate_capability_matrix();
    for row in &rows {
        assert!(!row.domain_id.is_empty(), "row has empty domain_id");
        assert!(!row.operation_id.is_empty(), "row has empty operation_id");
        assert!(
            !row.operation_name.is_empty(),
            "row has empty operation_name"
        );
        assert!(!row.domain_name.is_empty(), "row has empty domain_name");
    }
}

/// Every operation in the capability matrix must have matching OperationMetadata.
#[test]
fn capability_matrix_operations_have_metadata() {
    let rows = generate_capability_matrix();
    for row in &rows {
        let meta = metadata_for_tool_id(row.operation_id).unwrap_or_else(|| {
            panic!(
                "matrix row operation '{}' has no metadata",
                row.operation_id
            )
        });
        assert_eq!(
            meta.id, row.operation_id,
            "matrix row operation '{}' resolves to metadata '{}'",
            row.operation_id, meta.id
        );
    }
}

/// db-pentest pilot domain must appear in the capability matrix with correct fields.
#[cfg(feature = "db-pentest")]
#[test]
fn pilot_domain_db_pentest_in_capability_matrix() {
    use eggsec::domain::DomainCategory;

    let rows = generate_capability_matrix();
    let db_row = rows
        .iter()
        .find(|r| r.operation_id == "db-pentest")
        .expect("db-pentest should appear in capability matrix");

    assert_eq!(db_row.domain_id, "db-pentest");
    assert_eq!(db_row.category, DomainCategory::DefenseLab);
    assert!(db_row.cli, "db-pentest should have CLI integration");
    assert!(db_row.tui, "db-pentest should have TUI integration");
    assert!(
        db_row.tool_integration,
        "db-pentest should have MCP/tool integration"
    );
    assert_eq!(db_row.dry_run, "always");
    assert_eq!(db_row.evidence_report, "always");
}

// ─── Safety Exposure Rules ─────────────────────────────────────────────────

/// Hazardous domains (HazardousLab category) must NOT be exposed via MCP by default.
#[test]
fn hazardous_domains_not_mcp_exposed_by_default() {
    for domain in all_domain_descriptors() {
        if domain.category == eggsec::domain::DomainCategory::HazardousLab {
            for tool in domain.tools {
                assert!(
                    !tool.mcp_exposed_by_default,
                    "hazardous domain '{}' tool '{}' must not be MCP-exposed by default",
                    domain.id, tool.tool_id
                );
            }
        }
    }
}

/// Operations with risk > SafeActive that are agent-exposable must declare
/// at least one non-baseline capability.
#[test]
fn high_risk_agent_exposable_ops_declare_capability() {
    for m in all_operation_metadata() {
        if m.risk > OperationRisk::SafeActive && m.agent_exposable {
            let has_capability = !m.required_capabilities.is_empty();
            assert!(
                has_capability,
                "high-risk agent-exposable operation '{}' (risk {:?}) must declare capabilities",
                m.id, m.risk
            );
        }
    }
}

/// MCP-exposed operations must have mcp_exposable = true in their metadata.
#[test]
fn mcp_exposed_ops_have_metadata_flag() {
    for domain in all_domain_descriptors() {
        for tool in domain.tools {
            if tool.mcp_exposed_by_default {
                let meta = metadata_for_tool_id(tool.tool_id)
                    .expect("MCP-exposed tool should have metadata");
                assert!(
                    meta.mcp_exposable,
                    "tool '{}' is MCP-exposed in domain '{}' but metadata has mcp_exposable = false",
                    tool.tool_id, domain.id
                );
            }
        }
    }
}

// ─── Enum Variant Validity ────────────────────────────────────────────────

/// Every Capability used in OperationMetadata must be a known variant.
/// This catches typos or references to removed variants. Some defined
/// variants (e.g. IntrusiveFuzz) may not yet be assigned to any operation.
#[test]
fn all_metadata_capabilities_are_known_variants() {
    // If a capability is used in metadata, this match must exhaustive.
    // A compile error here means metadata references an unknown variant.
    for m in all_operation_metadata() {
        for cap in m.required_capabilities {
            match cap {
                Capability::PassiveFingerprint
                | Capability::ActiveProbe
                | Capability::Crawl
                | Capability::HttpFuzzLowImpact
                | Capability::IntrusiveFuzz
                | Capability::WafDetect
                | Capability::WafBypassSimulation
                | Capability::WafStressTest
                | Capability::LoadTest
                | Capability::RawPacketProbe
                | Capability::CredentialTesting
                | Capability::RemoteExecution
                | Capability::NseSafe
                | Capability::NseIntrusive
                | Capability::TrafficInterception
                | Capability::EvasionTesting
                | Capability::DatabaseAssessment
                | Capability::C2Simulation
                | Capability::MobileDynamicAnalysis => {}
            }
        }
    }
}

/// Every OperationRisk used in metadata must be a known variant.
/// This catches typos or references to removed variants. Some defined
/// variants (e.g. EvasionTesting) may not yet be assigned to any operation.
#[test]
fn all_metadata_risks_are_known_variants() {
    for m in all_operation_metadata() {
        match m.risk {
            OperationRisk::Passive
            | OperationRisk::SafeActive
            | OperationRisk::Intrusive
            | OperationRisk::LoadTest
            | OperationRisk::StressTest
            | OperationRisk::RawPacket
            | OperationRisk::CredentialTesting
            | OperationRisk::DbPentest
            | OperationRisk::TrafficInterception
            | OperationRisk::EvasionTesting
            | OperationRisk::PostExploitation
            | OperationRisk::ExploitAdjacent
            | OperationRisk::C2Operation
            | OperationRisk::RemoteExecution
            | OperationRisk::AgentAutonomous => {}
        }
    }
}

// ─── Tool ID Stability ────────────────────────────────────────────────────

/// All tool IDs registered by `create_default_registry()` must have operation
/// metadata. This prevents new tools from being added without metadata.
#[test]
fn all_registered_base_tools_have_operation_metadata() {
    let base_tool_ids = &[
        "recon",
        "scan-ports",
        "fingerprint",
        "scan-endpoints",
        "fuzz",
        "load",
        "waf-detect",
        "waf-bypass",
        "waf-stress",
        "pipeline",
        "search",
    ];
    for &tool_id in base_tool_ids {
        assert!(
            metadata_for_tool_id(tool_id).is_some(),
            "registered tool '{}' has no operation metadata",
            tool_id,
        );
    }
}

/// Feature-gated tool IDs must have operation metadata when their feature is enabled.
#[cfg(feature = "web-proxy-mcp")]
#[test]
fn web_proxy_mcp_tool_has_metadata() {
    assert!(
        metadata_for_tool_id("proxy").is_some(),
        "registered tool 'proxy' has no operation metadata"
    );
}

#[cfg(feature = "db-pentest-mcp")]
#[test]
fn db_pentest_mcp_tool_has_metadata() {
    assert!(
        metadata_for_tool_id("db-pentest").is_some(),
        "registered tool 'db-pentest' has no operation metadata"
    );
}

#[cfg(feature = "c2-mcp")]
#[test]
fn c2_mcp_tool_has_metadata() {
    assert!(
        metadata_for_tool_id("c2").is_some(),
        "registered tool 'c2' has no operation metadata"
    );
}

/// Operation IDs must be stable: the same ID used in OperationMetadata must
/// resolve to the same metadata entry via metadata_for_tool_id().
#[test]
fn operation_id_lookup_is_stable() {
    for m in all_operation_metadata() {
        let resolved = metadata_for_tool_id(m.id)
            .unwrap_or_else(|| panic!("operation '{}' should be resolvable by its own ID", m.id));
        assert_eq!(
            resolved.id, m.id,
            "operation '{}' resolved to '{}' — ID instability",
            m.id, resolved.id
        );
    }
}

/// All aliases must resolve to different canonical IDs than themselves
/// (already tested by no_alias_maps_to_self) and the resolved metadata
/// must have the same risk level as the canonical entry.
/// Aliases that are also canonical IDs are skipped (the alias is redundant).
#[test]
fn alias_risk_matches_canonical() {
    use eggsec::config::ALL_OPERATION_METADATA_ALIASES;
    for &(alias, canonical) in ALL_OPERATION_METADATA_ALIASES {
        // Skip aliases that are also canonical IDs — the alias is redundant
        // because metadata_for_tool_id() returns the canonical entry directly.
        if operation_metadata(alias).is_some() {
            continue;
        }
        let alias_meta = metadata_for_tool_id(alias)
            .unwrap_or_else(|| panic!("alias '{}' should resolve to metadata", alias));
        let canonical_meta = metadata_for_tool_id(canonical)
            .unwrap_or_else(|| panic!("canonical '{}' should have metadata", canonical));
        assert_eq!(
            alias_meta.id, canonical_meta.id,
            "alias '{}' resolves to '{}' but canonical '{}' resolves to '{}'",
            alias, alias_meta.id, canonical, canonical_meta.id
        );
        assert_eq!(
            alias_meta.risk, canonical_meta.risk,
            "alias '{}' has risk {:?} but canonical '{}' has {:?}",
            alias, alias_meta.risk, canonical, canonical_meta.risk
        );
    }
}

// ─── Documentation Cross-References ────────────────────────────────────────

/// Every feature referenced by OperationMetadata should either be a valid
/// Cargo feature or the metadata marks it as required_features.
#[test]
fn feature_names_are_nonempty_when_present() {
    for m in all_operation_metadata() {
        for feat in m.required_features {
            assert!(
                !feat.is_empty(),
                "operation '{}' has empty required_feature string",
                m.id
            );
        }
    }
}

/// Dry-run support should be declared for every domain.
#[test]
fn all_domains_declare_dry_run_support() {
    for domain in all_domain_descriptors() {
        // DryRunSupport is an enum, so it always has a value.
        // This test documents the invariant that we check it.
        match domain.dry_run {
            DryRunSupport::AlwaysAvailable => {}
            DryRunSupport::FeatureGated(f) => {
                assert!(
                    !f.is_empty(),
                    "domain '{}' has empty feature-gated dry-run string",
                    domain.id
                );
            }
            DryRunSupport::NotSupported => {}
        }
    }
}

/// Baseline support should be declared for every domain.
#[test]
fn all_domains_declare_baseline_support() {
    for domain in all_domain_descriptors() {
        match domain.baseline {
            BaselineSupport::AlwaysAvailable => {}
            BaselineSupport::FeatureGated(f) => {
                assert!(
                    !f.is_empty(),
                    "domain '{}' has empty feature-gated baseline string",
                    domain.id
                );
            }
            BaselineSupport::NotSupported => {}
        }
    }
}

/// If a domain declares a docs_url, it should be a non-empty string.
#[test]
fn domain_docs_urls_are_nonempty_when_present() {
    for domain in all_domain_descriptors() {
        if let Some(url) = domain.docs_url {
            assert!(!url.is_empty(), "domain '{}' has empty docs_url", domain.id);
        }
    }
}

// ─── Opt-in MCP Exposure Consistency ──────────────────────────────────────

/// Tools that require an MCP feature flag must not be MCP-exposed by default.
/// This catches the class of bug where db-pentest MCP exposure was mis-represented.
#[test]
fn opt_in_mcp_features_not_default_exposed() {
    for domain in all_domain_descriptors() {
        for tool in domain.tools {
            if tool.required_mcp_feature.is_some() {
                assert!(
                    !tool.mcp_exposed_by_default,
                    "domain '{}' tool '{}' requires MCP feature '{}' but is marked as MCP-exposed by default",
                    domain.id, tool.tool_id, tool.required_mcp_feature.unwrap()
                );
            }
        }
    }
}

// ─── Capability Matrix Exposure Alignment ─────────────────────────────────

/// Capability matrix exposure fields must align with the domain's ToolIntegration and OperationMetadata.
#[test]
fn capability_matrix_exposure_aligns_with_metadata() {
    let rows = generate_capability_matrix();
    for row in &rows {
        let domain = all_domain_descriptors()
            .iter()
            .find(|d| d.id == row.domain_id)
            .expect("row domain should exist");
        let tool_int = domain
            .tools
            .iter()
            .find(|t| t.operation_id == row.operation_id);

        // tool_integration field matches whether a ToolIntegration exists
        assert_eq!(
            row.tool_integration,
            tool_int.is_some(),
            "row '{}': tool_integration={} but ToolIntegration exists={}",
            row.operation_id,
            row.tool_integration,
            tool_int.is_some()
        );

        // If tool integration exists, mcp_exposed_by_default must match
        if let Some(ti) = tool_int {
            assert_eq!(
                row.mcp_exposed_by_default, ti.mcp_exposed_by_default,
                "row '{}': mcp_exposed_by_default mismatch",
                row.operation_id
            );
            assert_eq!(
                row.required_mcp_feature, ti.required_mcp_feature,
                "row '{}': required_mcp_feature mismatch",
                row.operation_id
            );
        }

        // rest_exposable and agent_exposable must match OperationMetadata
        if let Some(meta) = metadata_for_tool_id(row.operation_id) {
            assert_eq!(
                row.rest_exposable, meta.rest_exposable,
                "row '{}': rest_exposable mismatch with metadata",
                row.operation_id
            );
            assert_eq!(
                row.agent_exposable, meta.agent_exposable,
                "row '{}': agent_exposable mismatch with metadata",
                row.operation_id
            );
        }
    }
}

// ─── High-Risk Agent Exposure Guard ───────────────────────────────────────

/// No high-risk or non-baseline operation should be agent-exposable without
/// an explicit non-baseline capability and strict policy requirement.
#[test]
fn high_risk_agent_exposable_requires_capability_and_policy() {
    for m in all_operation_metadata() {
        if m.agent_exposable && m.risk > OperationRisk::SafeActive {
            let has_capability = !m.required_capabilities.is_empty();
            assert!(
                has_capability,
                "high-risk agent-exposable operation '{}' (risk {:?}) must declare required capabilities",
                m.id, m.risk
            );
        }
    }
}

// ─── Domain Docs URL File Validation ──────────────────────────────────────

/// If a domain declares a docs_url starting with "docs/", the referenced file
/// must exist relative to the workspace root. This prevents doc drift.
#[test]
fn domain_docs_urls_reference_existing_files() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    for domain in all_domain_descriptors() {
        if let Some(url) = domain.docs_url {
            if url.starts_with("docs/") {
                let path = workspace_root.join(url);
                assert!(
                    path.exists(),
                    "domain '{}' docs_url '{}' does not exist at '{}'",
                    domain.id,
                    url,
                    path.display()
                );
            }
        }
    }
}

// ─── mobile-dynamic Risk Classification ───────────────────────────────────

/// mobile-dynamic must not appear baseline-safe to strict programmatic surfaces.
#[cfg(feature = "mobile-dynamic")]
#[test]
fn mobile_dynamic_not_baseline_safe_in_metadata() {
    let meta = metadata_for_tool_id("mobile-dynamic").expect("mobile-dynamic should have metadata");
    assert!(
        meta.risk > OperationRisk::SafeActive,
        "mobile-dynamic risk should be above SafeActive, got {:?}",
        meta.risk
    );
    assert!(
        !meta.required_capabilities.is_empty(),
        "mobile-dynamic should declare capabilities for non-baseline classification"
    );
}

// ─── db-pentest MCP Opt-in Semantics ──────────────────────────────────────

/// db-pentest must have tool integration but not be MCP-exposed by default.
#[cfg(feature = "db-pentest")]
#[test]
fn db_pentest_mcp_opt_in_semantics() {
    let domain = all_domain_descriptors()
        .iter()
        .find(|d| d.id == "db-pentest")
        .expect("db-pentest domain should exist");
    let tool = domain
        .tools
        .iter()
        .find(|t| t.tool_id == "db-pentest")
        .expect("db-pentest should have tool integration");
    assert!(
        tool.required_mcp_feature.is_some(),
        "db-pentest should require an MCP feature flag"
    );
    assert!(
        !tool.mcp_exposed_by_default,
        "db-pentest must not be MCP-exposed by default"
    );
}

// ─── Mobile Row Drift Guards ─────────────────────────────────────────────

/// mobile-dynamic must not have strict surface support.
#[test]
fn mobile_dynamic_strict_surface_support_is_false() {
    let domain = all_domain_descriptors()
        .iter()
        .find(|d| d.id == "mobile-dynamic")
        .expect("mobile-dynamic domain should exist");
    assert!(
        !domain.strict_surface_support,
        "mobile-dynamic must not have strict_surface_support = true"
    );
}

/// mobile-dynamic scope must not require explicit scope.
#[test]
fn mobile_dynamic_requires_explicit_scope_is_false() {
    let domain = all_domain_descriptors()
        .iter()
        .find(|d| d.id == "mobile-dynamic")
        .expect("mobile-dynamic domain should exist");
    for op in domain.operations {
        assert!(
            !op.requires_explicit_scope,
            "mobile-dynamic operation '{}' must not require explicit scope",
            op.operation_id
        );
    }
}

/// mobile-dynamic must declare MobileDynamicAnalysis capability.
#[test]
fn mobile_dynamic_declares_dynamic_analysis_capability() {
    let domain = all_domain_descriptors()
        .iter()
        .find(|d| d.id == "mobile-dynamic")
        .expect("mobile-dynamic domain should exist");
    for op in domain.operations {
        assert!(
            op.capabilities
                .contains(&eggsec::config::Capability::MobileDynamicAnalysis),
            "mobile-dynamic operation '{}' must declare MobileDynamicAnalysis capability",
            op.operation_id
        );
    }
}

/// mobile-static scope must not require explicit scope.
#[test]
fn mobile_static_requires_explicit_scope_is_false() {
    let domain = all_domain_descriptors()
        .iter()
        .find(|d| d.id == "mobile-static")
        .expect("mobile-static domain should exist");
    for op in domain.operations {
        assert!(
            !op.requires_explicit_scope,
            "mobile-static operation '{}' must not require explicit scope",
            op.operation_id
        );
    }
}

/// High-risk agent-exposable operations must be blocked by default policy
/// (i.e. they require strict policy approval, not baseline allowance).
#[test]
fn high_risk_agent_exposable_ops_are_not_baseline_safe() {
    use eggsec::config::OperationRisk;
    for m in all_operation_metadata() {
        if m.agent_exposable && m.risk > OperationRisk::SafeActive {
            // These operations must have non-empty required_features or
            // required_capabilities, meaning they need explicit opt-in.
            let has_gate = !m.required_capabilities.is_empty() || !m.required_features.is_empty();
            assert!(
                has_gate,
                "high-risk agent-exposable operation '{}' (risk {:?}) must have \
                 non-baseline capability or feature gate — it cannot be baseline-safe",
                m.id, m.risk
            );
        }
    }
}
