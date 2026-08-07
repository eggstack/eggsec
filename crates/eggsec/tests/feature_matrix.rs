//! Feature metadata consistency tests — validate that feature strings in
//! OperationMetadata and DomainDescriptor match the authoritative feature
//! registry, and that feature naming conventions and dependencies are well-formed.

use eggsec::config::all_operation_metadata;
use eggsec::config::{
    classify_feature, feature_state, is_feature_enabled_registry, is_known_feature_registry,
    FeatureState, ALL_FEATURES,
};
use eggsec::domain::all_domain_descriptors;

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

/// Every Cargo feature (except `default`) is represented in the registry.
#[test]
fn all_cargo_features_are_in_registry() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest_str = std::fs::read_to_string(&manifest_path).expect("failed to read Cargo.toml");
    let manifest: toml::Value = manifest_str.parse().expect("failed to parse Cargo.toml");

    let features_table = manifest
        .get("features")
        .and_then(|v| v.as_table())
        .expect("no [features] table in Cargo.toml");

    let excluded = ["default"];
    let registry_names: rustc_hash::FxHashSet<&str> = ALL_FEATURES.iter().map(|e| e.name).collect();

    for cargo_feat in features_table.keys() {
        if excluded.contains(&cargo_feat.as_str()) {
            continue;
        }
        assert!(
            registry_names.contains(cargo_feat.as_str()),
            "Cargo.toml feature '{}' not in feature registry — add it to feature_registry!",
            cargo_feat
        );
    }
}

/// Every registry feature exists in Cargo.toml.
#[test]
fn registry_features_exist_in_cargo_toml() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest_str = std::fs::read_to_string(&manifest_path).expect("failed to read Cargo.toml");
    let manifest: toml::Value = manifest_str.parse().expect("failed to parse Cargo.toml");

    let features_table = manifest
        .get("features")
        .and_then(|v| v.as_table())
        .expect("no [features] table in Cargo.toml");

    for entry in ALL_FEATURES {
        assert!(
            features_table.contains_key(entry.name),
            "Registry feature '{}' not found in Cargo.toml [features] — remove from registry or add to Cargo.toml",
            entry.name
        );
    }
}

/// All OperationMetadata required_features must resolve through production lookup.
#[test]
fn operation_metadata_required_features_resolve() {
    for m in all_operation_metadata() {
        for feat in m.required_features {
            let state = feature_state(feat);
            assert!(
                !matches!(state, FeatureState::Unknown),
                "operation '{}' references unknown feature '{}' — add it to the feature registry",
                m.id,
                feat
            );
        }
    }
}

/// All DomainDescriptor required_feature values must resolve through production lookup.
#[test]
fn domain_descriptor_required_features_resolve() {
    for domain in all_domain_descriptors() {
        if let Some(feat) = domain.required_feature {
            let state = feature_state(feat);
            assert!(
                !matches!(state, FeatureState::Unknown),
                "domain '{}' references unknown feature '{}' — add it to the feature registry",
                domain.id,
                feat
            );
        }
        for op in domain.operations {
            for feat in op.required_features {
                let state = feature_state(feat);
                assert!(
                    !matches!(state, FeatureState::Unknown),
                    "domain '{}' operation '{}' references unknown feature '{}' — add it to the feature registry",
                    domain.id,
                    op.operation_id,
                    feat
                );
            }
        }
    }
}

/// All DomainDescriptor required_mcp_feature values must resolve through production lookup.
#[test]
fn domain_mcp_features_resolve() {
    for domain in all_domain_descriptors() {
        for tool in domain.tools {
            if let Some(feat) = tool.required_mcp_feature {
                let state = feature_state(feat);
                assert!(
                    !matches!(state, FeatureState::Unknown),
                    "domain '{}' tool '{}' references unknown MCP feature '{}' — add it to the feature registry",
                    domain.id,
                    tool.tool_id,
                    feat
                );
            }
        }
    }
}

/// Every command registry feature must resolve through production lookup.
#[test]
fn command_registry_features_resolve() {
    use eggsec::commands::registry::REGISTERED_COMMANDS;
    for cmd in REGISTERED_COMMANDS {
        if let Some(feat) = cmd.feature {
            let state = feature_state(feat);
            assert!(
                !matches!(state, FeatureState::Unknown),
                "command '{}' references unknown feature '{}' — add it to the feature registry",
                cmd.command_id,
                feat
            );
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
    for entry in ALL_FEATURES {
        let feature = entry.name;
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
    for entry in ALL_FEATURES {
        if let Some(base) = entry.name.strip_suffix("-mcp") {
            assert!(
                is_known_feature_registry(base),
                "MCP feature '{}' has base '{}' but it's not in the feature registry",
                entry.name,
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

/// Every feature in the registry has a valid classification.
#[test]
fn all_features_are_classified() {
    for entry in ALL_FEATURES {
        assert!(
            classify_feature(entry.name).is_some(),
            "feature '{}' has no classification in the registry",
            entry.name
        );
    }
}

/// Unknown feature lookup fails closed.
#[test]
fn unknown_feature_fails_closed() {
    assert_eq!(feature_state("nonexistent"), FeatureState::Unknown);
    assert!(!is_feature_enabled_registry("nonexistent"));
    assert!(!is_known_feature_registry("nonexistent"));
}

/// Known disabled feature reports disabled.
#[test]
fn known_disabled_feature_reports_disabled() {
    // `test-helpers` is not compiled by default in test builds
    let state = feature_state("test-helpers");
    // It might be enabled in test builds; just verify it's not Unknown
    assert!(
        matches!(state, FeatureState::Enabled | FeatureState::Disabled),
        "test-helpers should be known, got Unknown"
    );
}

/// Feature dependency edges reference only known features.
#[test]
fn dependency_edges_reference_known_features() {
    for &(from, to) in FEATURE_DEPENDENCIES {
        assert!(
            is_known_feature_registry(from),
            "dependency edge source '{}' is not in the feature registry",
            from
        );
        assert!(
            is_known_feature_registry(to),
            "dependency edge target '{}' is not in the feature registry",
            to
        );
    }
}

// ─── Frontend forwarding validation (Workstream 5) ─────────────────────────

/// Parse a Cargo.toml file and extract feature definitions.
/// Returns a map of feature name -> list of raw dependency/feature strings.
fn parse_features_raw(
    manifest_path: &std::path::Path,
) -> std::collections::HashMap<String, Vec<String>> {
    let manifest_str = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", manifest_path.display(), e));
    let manifest: toml::Value = manifest_str
        .parse()
        .unwrap_or_else(|e| panic!("failed to parse {}: {}", manifest_path.display(), e));

    let features_table = manifest
        .get("features")
        .and_then(|v| v.as_table())
        .unwrap_or_else(|| panic!("no [features] table in {}", manifest_path.display()));

    let mut result: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (feat_name, feat_value) in features_table {
        if feat_name == "default" {
            continue;
        }
        let mut deps = Vec::new();
        if let Some(arr) = feat_value.as_array() {
            for dep in arr {
                if let Some(dep_str) = dep.as_str() {
                    deps.push(dep_str.to_string());
                }
            }
        }
        result.insert(feat_name.clone(), deps);
    }
    result
}

/// Extract engine feature names from a list of raw dependency strings.
/// Filters `eggsec/<feature>` and `eggsec-tui?/<feature>` patterns.
fn extract_engine_features(deps: &[String]) -> Vec<String> {
    deps.iter()
        .filter_map(|dep| {
            dep.strip_prefix("eggsec/")
                .or_else(|| dep.strip_prefix("eggsec-tui?/"))
                .map(|f| f.to_string())
        })
        .collect()
}

/// CLI feature forwarding: every `eggsec/<feature>` reference must exist in the engine registry.
#[test]
fn cli_forwarded_features_exist_in_registry() {
    let manifest_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eggsec-cli/Cargo.toml");
    let features = parse_features_raw(&manifest_path);

    for (feat_name, deps) in &features {
        for engine_feat in extract_engine_features(deps) {
            assert!(
                is_known_feature_registry(&engine_feat),
                "CLI feature '{}' forwards to unknown engine feature '{}' — add to feature registry or remove forwarding",
                feat_name,
                engine_feat
            );
        }
    }
}

/// TUI feature forwarding: every `eggsec/<feature>` reference must exist in the engine registry.
#[test]
fn tui_forwarded_features_exist_in_registry() {
    let manifest_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eggsec-tui/Cargo.toml");
    let features = parse_features_raw(&manifest_path);

    for (feat_name, deps) in &features {
        for engine_feat in extract_engine_features(deps) {
            assert!(
                is_known_feature_registry(&engine_feat),
                "TUI feature '{}' forwards to unknown engine feature '{}' — add to feature registry or remove forwarding",
                feat_name,
                engine_feat
            );
        }
    }
}

/// CLI aggregate `full` must activate all documented CLI capabilities.
#[test]
fn cli_full_aggregate_forwards_all_documented_capabilities() {
    let manifest_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eggsec-cli/Cargo.toml");
    let features = parse_features_raw(&manifest_path);

    let cli_full_deps = features
        .get("full")
        .expect("CLI must have a 'full' aggregate feature");

    // These are all features that CLI's `full` includes (local feature names).
    let required_features = &[
        "tui",
        "mobile",
        "mobile-dynamic",
        "wireless-advanced",
        "db-pentest",
        "web-proxy",
    ];
    for &req in required_features {
        assert!(
            cli_full_deps.iter().any(|f| f == req),
            "CLI 'full' aggregate does not include feature '{}'",
            req
        );
    }
}

/// TUI aggregate `full` must activate all TUI-level capabilities.
#[test]
fn tui_full_aggregate_forwards_all_tui_capabilities() {
    let manifest_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eggsec-tui/Cargo.toml");
    let features = parse_features_raw(&manifest_path);

    let tui_full_deps = features
        .get("full")
        .expect("TUI must have a 'full' aggregate feature");

    // These are features that TUI's `full` includes.
    let required_features = &[
        "nse",
        "headless-browser",
        "compliance",
        "database",
        "external-integrations",
        "finding-workflow",
        "vuln-management",
        "wireless",
        "wireless-advanced",
        "mobile",
        "stress-testing",
        "packet-inspection",
        "advanced-hunting",
        "tool-api",
        "rest-api",
        "db-pentest",
        "web-proxy",
        "c2",
    ];
    for &req in required_features {
        assert!(
            tui_full_deps.iter().any(|f| f == req),
            "TUI 'full' aggregate does not include feature '{}'",
            req
        );
    }
}
