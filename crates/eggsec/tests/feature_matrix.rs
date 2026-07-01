//! Feature metadata consistency tests — validate that feature strings in
//! OperationMetadata and DomainDescriptor match actual Cargo features, and
//! that feature naming conventions and dependencies are well-formed.

use eggsec::config::all_operation_metadata;
use eggsec::domain::all_domain_descriptors;

// ─── Static Feature Registry ───────────────────────────────────────────────

/// Static snapshot of feature keys declared in `crates/eggsec/Cargo.toml [features]`.
/// Must be kept in sync with the actual Cargo features. The `snapshot_matches_cargo_toml_features`
/// test validates this automatically.
static KNOWN_EGGSEC_FEATURES: &[&str] = &[
    "tool-api",
    "insecure-tls",
    "rest-api",
    "ws-api",
    "grpc-api",
    "stress-testing",
    "packet-inspection",
    "nse",
    "nse-ssh2",
    "nse-sandbox",
    "advanced-hunting",
    "compliance",
    "external-integrations",
    "finding-workflow",
    "vuln-management",
    "full",
    "ai-integration",
    "websocket",
    "headless-browser",
    "database",
    "db-pentest",
    "db-pentest-mssql-tiberius",
    "db-pentest-mongodb",
    "db-pentest-redis",
    "db-pentest-mcp",
    "c2-mcp",
    "container",
    "cloud",
    "sbom",
    "git-secrets",
    "pdf",
    "wireless",
    "wireless-advanced",
    "evasion",
    "postex",
    "c2",
    "mobile",
    "mobile-dynamic",
    "api-schema",
    "web-proxy",
    "web-proxy-mcp",
    "transparent-proxy",
    "dynamic-plugins",
];

// ─── Feature Classification ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeatureCategory {
    /// Protocol adapters: tool-api, rest-api, grpc-api, ws-api, websocket
    ProtocolAdapter,
    /// Domain capabilities: db-pentest, mobile, wireless, web-proxy, evasion, postex, c2, nse
    DomainCapability,
    /// MCP/protocol exposure markers: db-pentest-mcp, web-proxy-mcp, c2-mcp
    ProtocolExposure,
    /// Marker-only features with no deps: advanced-hunting, compliance, etc.
    MarkerOnly,
    /// Database backend drivers: db-pentest-mssql-tiberius, db-pentest-mongodb, db-pentest-redis
    BackendDriver,
    /// Platform-sensitive features: stress-testing, packet-inspection, nse-ssh2, nse-sandbox
    PlatformSensitive,
    /// Storage/output integrations: database, sbom, container, pdf
    StorageIntegration,
    /// Aggregate features: full
    Aggregate,
    /// Security risk features: insecure-tls
    SecurityRisk,
    /// AI integration: ai-integration
    AiIntegration,
    /// Advanced extensions: mobile-dynamic, wireless-advanced
    AdvancedExtension,
}

fn classify_feature(feature: &str) -> FeatureCategory {
    match feature {
        "tool-api" | "rest-api" | "grpc-api" | "ws-api" | "websocket" => {
            FeatureCategory::ProtocolAdapter
        }
        "db-pentest" | "mobile" | "wireless" | "web-proxy" | "evasion" | "postex" | "c2"
        | "nse" => FeatureCategory::DomainCapability,
        "db-pentest-mcp" | "web-proxy-mcp" | "c2-mcp" => FeatureCategory::ProtocolExposure,
        "advanced-hunting"
        | "compliance"
        | "external-integrations"
        | "finding-workflow"
        | "vuln-management"
        | "cloud"
        | "git-secrets"
        | "api-schema" => FeatureCategory::MarkerOnly,
        "db-pentest-mssql-tiberius" | "db-pentest-mongodb" | "db-pentest-redis" => {
            FeatureCategory::BackendDriver
        }
        "stress-testing" | "packet-inspection" | "nse-ssh2" | "nse-sandbox"
        | "headless-browser" => FeatureCategory::PlatformSensitive,
        "database" | "sbom" | "container" | "pdf" => FeatureCategory::StorageIntegration,
        "full" => FeatureCategory::Aggregate,
        "insecure-tls" => FeatureCategory::SecurityRisk,
        "ai-integration" => FeatureCategory::AiIntegration,
        "mobile-dynamic" | "wireless-advanced" | "transparent-proxy" | "dynamic-plugins" => {
            FeatureCategory::AdvancedExtension
        }
        _ => panic!("unclassified feature: '{feature}' — add to classify_feature()"),
    }
}

// ─── Feature Dependency Graph ──────────────────────────────────────────────

/// Static feature dependency edges derived from `Cargo.toml [features]`.
/// Each entry is `(feature, depends_on)`.
static FEATURE_DEPENDENCIES: &[(&str, &str)] = &[
    // rest-api depends on tool-api
    ("rest-api", "tool-api"),
    // grpc-api depends on tool-api
    ("grpc-api", "tool-api"),
    // nse depends on tool-api
    ("nse", "tool-api"),
    // nse-ssh2 depends on nse
    ("nse-ssh2", "nse"),
    // nse-sandbox depends on nse
    ("nse-sandbox", "nse"),
    // ai-integration depends on tool-api
    ("ai-integration", "tool-api"),
    // db-pentest-mcp depends on db-pentest
    ("db-pentest-mcp", "db-pentest"),
    // c2-mcp depends on c2
    ("c2-mcp", "c2"),
    // c2 depends on postex and evasion
    ("c2", "postex"),
    ("c2", "evasion"),
    // wireless-advanced depends on wireless
    ("wireless-advanced", "wireless"),
    // mobile-dynamic depends on mobile
    ("mobile-dynamic", "mobile"),
    // web-proxy-mcp depends on web-proxy
    ("web-proxy-mcp", "web-proxy"),
    // transparent-proxy depends on web-proxy
    ("transparent-proxy", "web-proxy"),
    // dynamic-plugins depends on web-proxy
    ("dynamic-plugins", "web-proxy"),
    // full aggregates many features
    ("full", "stress-testing"),
    ("full", "packet-inspection"),
    ("full", "rest-api"),
    ("full", "nse"),
    ("full", "ai-integration"),
    ("full", "websocket"),
    ("full", "headless-browser"),
    ("full", "database"),
    ("full", "container"),
    ("full", "sbom"),
    ("full", "advanced-hunting"),
    ("full", "compliance"),
    ("full", "external-integrations"),
    ("full", "finding-workflow"),
    ("full", "vuln-management"),
    ("full", "wireless"),
    ("full", "wireless-advanced"),
    ("full", "mobile"),
    ("full", "mobile-dynamic"),
    ("full", "db-pentest"),
    ("full", "web-proxy"),
    ("full", "evasion"),
    ("full", "postex"),
    ("full", "c2"),
];

// ─── Tests ─────────────────────────────────────────────────────────────────

/// Every feature in `KNOWN_EGGSEC_FEATURES` must have a classification.
#[test]
fn all_known_features_are_classified() {
    for &feature in KNOWN_EGGSEC_FEATURES {
        let _ = classify_feature(feature);
    }
}

/// All OperationMetadata required_features must reference known Cargo features.
#[test]
fn operation_metadata_required_features_are_known() {
    let known: rustc_hash::FxHashSet<&str> = KNOWN_EGGSEC_FEATURES.iter().copied().collect();
    for m in all_operation_metadata() {
        for feat in m.required_features {
            assert!(
                known.contains(*feat),
                "operation '{}' references unknown feature '{}' — add it to KNOWN_EGGSEC_FEATURES",
                m.id,
                feat
            );
        }
    }
}

/// All DomainDescriptor required_feature values must reference known Cargo features.
#[test]
fn domain_descriptor_required_features_are_known() {
    let known: rustc_hash::FxHashSet<&str> = KNOWN_EGGSEC_FEATURES.iter().copied().collect();
    for domain in all_domain_descriptors() {
        if let Some(feat) = domain.required_feature {
            assert!(
                known.contains(feat),
                "domain '{}' references unknown feature '{}' — add it to KNOWN_EGGSEC_FEATURES",
                domain.id,
                feat
            );
        }
        for op in domain.operations {
            for feat in op.required_features {
                assert!(
                    known.contains(*feat),
                    "domain '{}' operation '{}' references unknown feature '{}' — add it to KNOWN_EGGSEC_FEATURES",
                    domain.id,
                    op.operation_id,
                    feat
                );
            }
        }
    }
}

/// All DomainDescriptor required_mcp_feature values must reference known Cargo features.
#[test]
fn domain_mcp_features_are_known() {
    let known: rustc_hash::FxHashSet<&str> = KNOWN_EGGSEC_FEATURES.iter().copied().collect();
    for domain in all_domain_descriptors() {
        for tool in domain.tools {
            if let Some(feat) = tool.required_mcp_feature {
                assert!(
                    known.contains(feat),
                    "domain '{}' tool '{}' references unknown MCP feature '{}' — add it to KNOWN_EGGSEC_FEATURES",
                    domain.id,
                    tool.tool_id,
                    feat
                );
            }
        }
    }
}

/// Feature names must follow naming conventions:
/// - Base domain: `<domain>` (e.g. `db-pentest`, `mobile`, `wireless`, `web-proxy`)
/// - Protocol exposure: `<domain>-mcp` (e.g. `db-pentest-mcp`, `web-proxy-mcp`, `c2-mcp`)
/// - Backend driver: `<domain>-<backend>` (e.g. `db-pentest-mongodb`)
/// - Advanced: `<domain>-advanced` or `<domain>-dynamic` (e.g. `wireless-advanced`, `mobile-dynamic`)
#[test]
fn feature_names_follow_naming_conventions() {
    for &feature in KNOWN_EGGSEC_FEATURES {
        // All features must be kebab-case (lowercase + digits + hyphens only)
        assert!(
            feature
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "feature '{}' is not kebab-case",
            feature
        );
        // Must not start or end with a hyphen
        assert!(
            !feature.starts_with('-') && !feature.ends_with('-'),
            "feature '{}' starts or ends with a hyphen",
            feature
        );
        // Must not contain consecutive hyphens
        assert!(
            !feature.contains("--"),
            "feature '{}' contains consecutive hyphens",
            feature
        );
    }

    // Verify MCP exposure naming pattern
    for &feature in KNOWN_EGGSEC_FEATURES {
        if let Some(base) = feature.strip_suffix("-mcp") {
            assert!(
                KNOWN_EGGSEC_FEATURES.contains(&base),
                "MCP feature '{}' has base '{}' but it's not in KNOWN_EGGSEC_FEATURES",
                feature,
                base
            );
        }
    }

    // Verify backend driver naming pattern
    let backend_drivers = &[
        "db-pentest-mssql-tiberius",
        "db-pentest-mongodb",
        "db-pentest-redis",
    ];
    for &feature in backend_drivers {
        assert!(
            feature.starts_with("db-pentest-"),
            "backend driver '{}' should start with 'db-pentest-'",
            feature
        );
    }
}

/// The `full` aggregate feature must include all domain capabilities (developer/lab profile).
/// Note: `full` intentionally includes advanced/lab-only features (wireless-advanced, mobile-dynamic,
/// evasion, postex, c2) as it is a developer/lab aggregate, not a conservative default.
#[test]
fn aggregate_feature_includes_domain_features() {
    // Domain capability features that the aggregate should pull in
    let domain_features = &[
        "db-pentest",
        "mobile",
        "mobile-dynamic",
        "wireless",
        "wireless-advanced",
        "web-proxy",
        "evasion",
        "postex",
        "c2",
    ];
    for &feat in domain_features {
        assert!(
            FEATURE_DEPENDENCIES
                .iter()
                .any(|&(f, dep)| f == "full" && dep == feat),
            "aggregate feature 'full' does not include domain feature '{}'",
            feat
        );
    }
}

/// Feature dependency graph must not contain cycles (DFS-based check).
#[test]
fn no_circular_feature_dependencies() {
    use rustc_hash::FxHashMap;

    // Build adjacency list
    let mut graph: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
    for &(from, to) in FEATURE_DEPENDENCIES {
        graph.entry(from).or_default().push(to);
    }

    // DFS cycle detection
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Visiting,
        Visited,
    }
    let mut state: FxHashMap<&str, State> = FxHashMap::default();

    fn has_cycle<'a>(
        node: &'a str,
        graph: &FxHashMap<&'a str, Vec<&'a str>>,
        state: &mut FxHashMap<&'a str, State>,
    ) -> bool {
        if state.get(node) == Some(&State::Visiting) {
            return true;
        }
        if state.get(node) == Some(&State::Visited) {
            return false;
        }
        state.insert(node, State::Visiting);
        if let Some(deps) = graph.get(node) {
            for &dep in deps {
                if has_cycle(dep, graph, state) {
                    return true;
                }
            }
        }
        state.insert(node, State::Visited);
        false
    }

    let all_nodes: Vec<&str> = graph.keys().copied().collect();
    for node in all_nodes {
        assert!(
            !has_cycle(node, &graph, &mut state),
            "circular feature dependency detected involving '{}'",
            node
        );
    }
}

/// Parse Cargo.toml to extract declared features, validating the static snapshot.
#[test]
fn snapshot_matches_cargo_toml_features() {
    // Read Cargo.toml from the crate root (relative to CARGO_MANIFEST_DIR)
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest_str = std::fs::read_to_string(&manifest_path).expect("failed to read Cargo.toml");
    let manifest: toml::Value = manifest_str.parse().expect("failed to parse Cargo.toml");

    let features_table = manifest
        .get("features")
        .and_then(|v| v.as_table())
        .expect("no [features] table in Cargo.toml");

    let cargo_features: rustc_hash::FxHashSet<&str> =
        features_table.keys().map(|s| s.as_str()).collect();

    // Every feature in our static snapshot must exist in Cargo.toml
    for &feature in KNOWN_EGGSEC_FEATURES {
        assert!(
            cargo_features.contains(feature),
            "SNAPSHOT feature '{}' not found in Cargo.toml [features] — update KNOWN_EGGSEC_FEATURES or Cargo.toml",
            feature
        );
    }

    // Every Cargo.toml feature should be in our snapshot (or explicitly documented as excluded)
    // Known exclusions: "default" is always present in Cargo.toml but not a real feature
    let excluded = rustc_hash::FxHashSet::from_iter(["default"]);
    for cargo_feat in &cargo_features {
        if excluded.contains(*cargo_feat) {
            continue;
        }
        assert!(
            KNOWN_EGGSEC_FEATURES.contains(cargo_feat),
            "Cargo.toml feature '{}' not in KNOWN_EGGSEC_FEATURES — add it to the snapshot",
            cargo_feat
        );
    }
}

/// Protocol exposure markers (MCP features) must require their base domain feature.
#[test]
fn protocol_exposure_markers_require_base_domain() {
    let mcp_features: &[(&str, &str)] = &[
        ("db-pentest-mcp", "db-pentest"),
        ("web-proxy-mcp", "web-proxy"),
        ("c2-mcp", "c2"),
    ];
    for &(mcp_feature, base_feature) in mcp_features {
        assert!(
            FEATURE_DEPENDENCIES
                .iter()
                .any(|&(f, dep)| f == mcp_feature && dep == base_feature),
            "MCP feature '{}' does not require base feature '{}' in dependency graph",
            mcp_feature,
            base_feature
        );
    }
}
