//! Phase 0 frontend/runtime parity guards.
//!
//! Freezes the current intended surface contract before Phase 1/2 removes
//! redundant ownership. Derives the support matrix in tests from canonical
//! registries plus small explicit exception lists — no new production
//! registry is created here.
//!
//! Covers workstreams 0.2 (support matrix), 0.3 (Clap tree reflection),
//! 0.5 (runtime mapping guards), and 0.6 (feature-profile parity) for the
//! engine-owned surfaces (CLI registry, Clap tree, canonical operations,
//! runtime TaskKind). TUI tab parity lives in `eggsec-tui` (see
//! `tabs::spec` parity tests) because the engine must not depend on TUI.

use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "cli")]
use clap::CommandFactory;
#[cfg(feature = "cli")]
use eggsec::cli::Cli;
use eggsec::commands::registry::{CommandDispatchMode, REGISTERED_COMMANDS};
use eggsec::config::{
    all_operation_metadata, is_known_feature_registry, metadata_for_tool_id, FeatureState,
    ALL_OPERATION_METADATA_ALIASES,
};

// ─── 0.2 Support matrix ─────────────────────────────────────────────

/// Dispatch class for a CLI command registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchClass {
    OperationBacked,
    Multiplexer,
    Helper,
    Lifecycle,
}

impl DispatchClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::OperationBacked => "operation-backed",
            Self::Multiplexer => "multiplexer",
            Self::Helper => "helper",
            Self::Lifecycle => "lifecycle",
        }
    }
}

/// Surface status for a matrix row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceStatus {
    Supported,
    IntentionallyUnavailable,
    CompatibilityAlias,
}

impl SurfaceStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::IntentionallyUnavailable => "intentionally-unavailable",
            Self::CompatibilityAlias => "compatibility-alias",
        }
    }
}

/// Test-owned matrix row. Derived from canonical registries; the only
/// hand-maintained inputs are the explicit exception sets below.
#[derive(Debug)]
struct MatrixRow {
    command_id: &'static str,
    canonical_operation_id: Option<&'static str>,
    dispatch_class: DispatchClass,
    required_feature: Option<&'static str>,
    cli_visible: bool,
    status: SurfaceStatus,
}

/// Commands that share one canonical operation with siblings (multiplexer
/// families). Each entry maps command_id -> shared canonical operation.
fn multiplexer_families() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        // Pipeline family: two CLI entry points, one canonical operation.
        ("scan", "pipeline"),
        ("resume", "pipeline"),
        // Packet family: three CLI entry points share `packet`.
        ("packet", "packet"),
        ("icmp", "packet"),
        ("traceroute", "packet"),
        // Subcommand multiplexers: top-level Clap command owns the branch,
        // registry tracks the subcommand execution identity.
        ("mobile-dynamic", "mobile-dynamic"),
        ("wireless-deauth", "wireless-deauth"),
    ])
}

/// Registry entries that are subcommand identities (no top-level Clap
/// command of the same name). They are operation-backed but dispatch through
/// a parent Clap subcommand (`mobile dynamic`, `wireless deauth`).
fn subcommand_identities() -> BTreeSet<&'static str> {
    BTreeSet::from(["mobile-dynamic", "wireless-deauth"])
}

/// Clap top-level commands with no registry entry (pre-existing drift,
/// explicitly classified here so new drift fails loudly).
/// Each entry maps clap_name -> classification rationale.
fn clap_only_exceptions() -> BTreeMap<String, (&'static str, &'static str)> {
    BTreeMap::from([
        (
            "proxy".to_string(),
            (
                "helper",
                "proxy pool management (stress-testing); no operation metadata",
            ),
        ),
        (
            "daemon".to_string(),
            (
                "lifecycle",
                "daemon-client lifecycle; engine registry tracks server lifecycle only",
            ),
        ),
        (
            "session".to_string(),
            ("lifecycle", "daemon-client lifecycle"),
        ),
        ("task".to_string(), ("lifecycle", "daemon-client lifecycle")),
        (
            "codegg-mcp".to_string(),
            (
                "compatibility-alias",
                "coding-agent profile alias for mcp-serve (same command_id)",
            ),
        ),
        (
            "mcp-codegg".to_string(),
            (
                "compatibility-alias",
                "alias spelling for codegg-mcp surface",
            ),
        ),
    ])
}

/// Registry entries whose Clap top-level name differs (alias/subcommand gap).
/// Maps registry command_id -> actual Clap top-level name.
fn registry_to_clap_renames() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        // Registry tracks the server lifecycle id; Clap exposes `remote`.
        ("remote-serve", "remote"),
    ])
}

fn classify_registration(
    command_id: &'static str,
    operation_id: Option<&'static str>,
    dispatch_mode: CommandDispatchMode,
) -> (DispatchClass, SurfaceStatus) {
    match dispatch_mode {
        CommandDispatchMode::RegistryBacked => {
            let op = operation_id.expect("registry-backed must have operation_id");
            if multiplexer_families().values().any(|v| *v == op)
                && multiplexer_families()
                    .iter()
                    .filter(|(_, v)| **v == op)
                    .count()
                    > 1
            {
                (DispatchClass::Multiplexer, SurfaceStatus::Supported)
            } else if subcommand_identities().contains(command_id) {
                (DispatchClass::Multiplexer, SurfaceStatus::Supported)
            } else {
                (DispatchClass::OperationBacked, SurfaceStatus::Supported)
            }
        }
        CommandDispatchMode::HelperOnly => (DispatchClass::Helper, SurfaceStatus::Supported),
        CommandDispatchMode::ServerLifecycle => {
            (DispatchClass::Lifecycle, SurfaceStatus::Supported)
        }
        CommandDispatchMode::CatalogOnly => (
            DispatchClass::Helper,
            SurfaceStatus::IntentionallyUnavailable,
        ),
    }
}

fn build_matrix() -> Vec<MatrixRow> {
    REGISTERED_COMMANDS
        .iter()
        .map(|r| {
            let (dispatch_class, status) =
                classify_registration(r.command_id, r.operation_id, r.dispatch_mode);
            MatrixRow {
                command_id: r.command_id,
                canonical_operation_id: r.operation_id,
                dispatch_class,
                required_feature: r.feature,
                cli_visible: r.cli_visible,
                status,
            }
        })
        .collect()
}

#[test]
fn support_matrix_covers_every_registered_command() {
    let matrix = build_matrix();
    assert_eq!(
        matrix.len(),
        REGISTERED_COMMANDS.len(),
        "matrix must classify every registered command"
    );
    for row in &matrix {
        // Every row has a stable dispatch class and status (no unclassified).
        let _ = row.dispatch_class.as_str();
        let _ = row.status.as_str();
        if row.dispatch_class == DispatchClass::OperationBacked
            || row.dispatch_class == DispatchClass::Multiplexer
        {
            assert!(
                row.canonical_operation_id.is_some(),
                "command '{}' classified {:?} must have a canonical operation",
                row.command_id,
                row.dispatch_class
            );
            let op = row.canonical_operation_id.unwrap();
            assert!(
                metadata_for_tool_id(op).is_some(),
                "command '{}' operation '{}' must resolve to canonical metadata",
                row.command_id,
                op
            );
        }
    }
}

#[test]
fn support_matrix_multiplexer_families_share_canonical_identity() {
    // Multiplexer families must resolve to a single canonical operation id,
    // not create second canonical identities.
    for (cmd, canonical) in multiplexer_families() {
        // Subcommand identities are registry-only; top-level families must be registered.
        if subcommand_identities().contains(cmd) {
            continue;
        }
        let reg = REGISTERED_COMMANDS
            .iter()
            .find(|r| r.command_id == cmd)
            .unwrap_or_else(|| panic!("multiplexer command '{cmd}' must be registered"));
        assert_eq!(
            reg.operation_id,
            Some(canonical),
            "multiplexer '{cmd}' must map to canonical '{canonical}'"
        );
        let meta = metadata_for_tool_id(canonical).expect("canonical must exist");
        assert_eq!(meta.id, canonical);
    }
    // Packet family shares exactly one canonical id.
    for cmd in ["packet", "icmp", "traceroute"] {
        let reg = REGISTERED_COMMANDS
            .iter()
            .find(|r| r.command_id == cmd)
            .unwrap();
        assert_eq!(reg.operation_id, Some("packet"));
    }
    // Pipeline family shares exactly one canonical id.
    for cmd in ["scan", "resume"] {
        let reg = REGISTERED_COMMANDS
            .iter()
            .find(|r| r.command_id == cmd)
            .unwrap();
        assert_eq!(reg.operation_id, Some("pipeline"));
    }
}

// ─── 0.3 Clap tree reflection ───────────────────────────────────────
// All Clap reflection tests require the `cli` feature (the `Cli` type).

/// Collect top-level Clap subcommand names + visible aliases from the actual
/// compiled `Cli` type. Exhaustive: walks the reflected tree, not a curated list.
#[cfg(feature = "cli")]
fn reflected_clap_top_level() -> BTreeMap<String, Vec<String>> {
    let cmd = Cli::command();
    let mut out = BTreeMap::new();
    for sub in cmd.get_subcommands() {
        let name = sub.get_name().to_string();
        let aliases: Vec<String> = sub
            .get_visible_aliases()
            .chain(sub.get_aliases())
            .map(|s| s.to_string())
            .collect();
        out.insert(name, aliases);
    }
    out
}

#[cfg(feature = "cli")]
#[test]
fn cli_visible_registry_entries_exist_in_clap_tree() {
    let clap = reflected_clap_top_level();
    for reg in REGISTERED_COMMANDS {
        if !reg.cli_visible {
            continue;
        }
        if subcommand_identities().contains(reg.command_id) {
            // Subcommand identity: parent Clap presence tracks the gating
            // feature (absent when disabled, present when enabled).
            let (parent, feat) = match reg.command_id {
                "mobile-dynamic" => ("mobile", "mobile-dynamic"),
                "wireless-deauth" => ("wireless", "wireless-advanced"),
                _ => continue,
            };
            assert!(
                is_known_feature_registry(feat),
                "subcommand '{}' gates on unknown feature '{feat}'",
                reg.command_id
            );
            let enabled = eggsec::config::is_feature_enabled_registry(feat);
            // Parent itself may have its own base feature (mobile/wireless);
            // require parent exactly when the subcommand feature is enabled.
            // (mobile-dynamic requires mobile; wireless-deauth requires wireless.)
            if enabled {
                assert!(
                    clap.contains_key(parent),
                    "subcommand identity '{}' requires parent Clap command '{parent}' when feature '{feat}' is enabled",
                    reg.command_id
                );
            }
            continue;
        }
        let clap_name = registry_to_clap_renames()
            .get(reg.command_id)
            .copied()
            .unwrap_or(reg.command_id);
        // Feature-disabled commands must be absent from the reflected tree
        // for operation-backed dispatch (compile-time gating); helper and
        // lifecycle commands may remain visible as unavailable shells (e.g.
        // `storage` prints a feature hint at runtime). This keeps registry
        // tests from staying green while the Clap variant disappeared for
        // operation paths, without forcing helpers into one pattern.
        match reg.feature {
            Some(feat) => {
                assert!(
                    is_known_feature_registry(feat),
                    "command '{}' references unknown feature '{feat}'",
                    reg.command_id
                );
                let enabled = eggsec::config::is_feature_enabled_registry(feat);
                let is_operation = matches!(reg.dispatch_mode, CommandDispatchMode::RegistryBacked);
                if is_operation {
                    if enabled {
                        assert!(
                            clap.contains_key(clap_name),
                            "enabled cli_visible command '{}' (clap '{clap_name}') missing from reflected Clap tree",
                            reg.command_id
                        );
                    } else {
                        assert!(
                            !clap.contains_key(clap_name),
                            "disabled feature '{feat}' must remove Clap command '{clap_name}' (registry '{}')",
                            reg.command_id
                        );
                    }
                } else if enabled {
                    // Helper/lifecycle with feature enabled must be present
                    // (unless it is a subcommand identity handled above).
                    assert!(
                        clap.contains_key(clap_name),
                        "enabled helper/lifecycle command '{}' (clap '{clap_name}') missing from reflected Clap tree",
                        reg.command_id
                    );
                }
                // Helper/lifecycle with feature disabled may be absent (gated
                // Clap, e.g. sbom) or present as an unavailable shell (e.g.
                // storage); both are intentional and covered by the
                // feature-disabled-vs-unavailable test where relevant.
            }
            None => {
                assert!(
                    clap.contains_key(clap_name),
                    "ungated cli_visible command '{}' (clap '{clap_name}') missing from reflected Clap tree",
                    reg.command_id
                );
            }
        }
    }
}

#[cfg(feature = "cli")]
#[test]
fn every_operation_backed_clap_command_is_registered_or_excepted() {
    let clap = reflected_clap_top_level();
    let registry_ids: BTreeSet<&str> = REGISTERED_COMMANDS.iter().map(|r| r.command_id).collect();
    let renames: BTreeSet<&str> = registry_to_clap_renames().values().copied().collect();
    let exceptions = clap_only_exceptions();
    for clap_name in clap.keys() {
        let in_registry =
            registry_ids.contains(clap_name.as_str()) || renames.contains(clap_name.as_str());
        if in_registry {
            continue;
        }
        assert!(
            exceptions.contains_key(clap_name),
            "Clap top-level command '{clap_name}' has no registry entry and no explicit exception — add it to clap_only_exceptions() or REGISTERED_COMMANDS"
        );
    }
    // Declared exceptions are valid when present in the current tree, when
    // they are alias-only spellings, or when their gating feature is disabled
    // (expected absence under the active profile). This keeps the list from
    // going stale without failing under feature-disabled profiles.
    let gated_absence: BTreeMap<&str, &str> = BTreeMap::from([
        ("proxy", "stress-testing"),
        ("daemon", "daemon-client"),
        ("session", "daemon-client"),
        ("task", "daemon-client"),
    ]);
    for name in exceptions.keys() {
        // Alias-only exceptions appear as aliases, not top-level names.
        if name == "codegg-mcp" || name == "mcp-codegg" {
            continue;
        }
        if clap.contains_key(name) {
            continue;
        }
        if let Some(feat) = gated_absence.get(name.as_str()) {
            assert!(
                is_known_feature_registry(feat),
                "exception '{name}' gates on unknown feature '{feat}'"
            );
            assert!(
                !eggsec::config::is_feature_enabled_registry(feat),
                "clap-only exception '{name}' absent while feature '{feat}' is enabled — stale exception or missing Clap command"
            );
            continue;
        }
        panic!("clap-only exception '{name}' is stale (absent from Clap tree, not an alias, no gating feature)");
    }
}

#[cfg(feature = "cli")]
#[test]
fn clap_aliases_do_not_create_second_canonical_identity() {
    let clap = reflected_clap_top_level();
    // mcp-serve / codegg-mcp alias family must resolve to one registry entry.
    let mcp_reg = REGISTERED_COMMANDS
        .iter()
        .find(|r| r.command_id == "mcp-serve")
        .expect("mcp-serve must be registered");
    assert!(mcp_reg.operation_id.is_none());
    // The reflected tree must expose mcp-serve (or its alias) without a
    // second canonical operation identity.
    let has_mcp = clap.contains_key("mcp-serve")
        || clap.contains_key("codegg-mcp")
        || clap.values().any(|aliases| {
            aliases
                .iter()
                .any(|a| a == "mcp-serve" || a == "mcp-codegg")
        });
    // When rest-api is disabled the whole family disappears together.
    if eggsec::config::is_feature_enabled_registry("rest-api") {
        assert!(
            has_mcp,
            "mcp-serve family must be present when rest-api is enabled"
        );
    }
    // Aliases in the canonical alias table must never point to themselves
    // (that would be a redundant second identity).
    for (alias, canonical) in ALL_OPERATION_METADATA_ALIASES {
        assert_ne!(
            alias, canonical,
            "alias '{alias}' maps to itself — remove the redundant entry"
        );
        assert!(
            metadata_for_tool_id(canonical).is_some(),
            "alias '{alias}' points to unknown canonical '{canonical}'"
        );
    }
}

#[test]
fn helper_and_lifecycle_commands_are_classified_explicitly() {
    for reg in REGISTERED_COMMANDS {
        match reg.dispatch_mode {
            CommandDispatchMode::HelperOnly => {
                assert!(
                    reg.cli_interactive_only,
                    "HelperOnly command '{}' must be cli_interactive_only",
                    reg.command_id
                );
                assert!(
                    !reg.tui_visible,
                    "HelperOnly command '{}' must not be tui_visible",
                    reg.command_id
                );
            }
            CommandDispatchMode::ServerLifecycle => {
                assert!(
                    !reg.tui_visible,
                    "ServerLifecycle command '{}' must not be tui_visible",
                    reg.command_id
                );
                assert!(
                    !reg.cli_interactive_only,
                    "ServerLifecycle command '{}' must not be cli_interactive_only",
                    reg.command_id
                );
            }
            CommandDispatchMode::RegistryBacked => {
                assert!(
                    reg.operation_id.is_some(),
                    "RegistryBacked command '{}' must have an operation_id",
                    reg.command_id
                );
            }
            CommandDispatchMode::CatalogOnly => {}
        }
    }
    // Output-only / lifecycle examples are explicitly helper/lifecycle, not
    // operation-backed: plan, preflight, ci, config, doctor, report, notify.
    for cmd in [
        "plan",
        "preflight",
        "ci",
        "config",
        "doctor",
        "report",
        "notify",
    ] {
        let reg = REGISTERED_COMMANDS
            .iter()
            .find(|r| r.command_id == cmd)
            .unwrap_or_else(|| panic!("helper/lifecycle command '{cmd}' must be registered"));
        assert!(
            matches!(
                reg.dispatch_mode,
                CommandDispatchMode::HelperOnly | CommandDispatchMode::ServerLifecycle
            ),
            "command '{cmd}' must be helper/lifecycle, got {:?}",
            reg.dispatch_mode
        );
    }
}

#[cfg(feature = "cli")]
#[test]
fn reflected_tree_is_exhaustive_not_spot_check() {
    let clap = reflected_clap_top_level();
    // The reflected tree must contain the core operation-backed surface.
    // This is a floor, not a ceiling: the every_operation_backed test above
    // enforces exhaustiveness in the other direction.
    let floor = [
        "recon",
        "scan-ports",
        "scan-endpoints",
        "fingerprint",
        "fuzz",
        "waf",
        "waf-stress",
        "graphql",
        "oauth",
        "auth-test",
        "scan",
        "resume",
        "load",
        "report",
        "vuln",
        "storage",
        "cluster",
        "notify",
        "remote",
        "exec",
        "plan",
        "preflight",
        "ci",
        "config",
        "doctor",
        "policy-explain",
        "scope-explain",
    ];
    for cmd in floor {
        assert!(
            clap.contains_key(cmd),
            "reflected Clap tree missing core command '{cmd}' — test is not exhaustive"
        );
    }
    // Feature-gated presence must track the compiled feature set.
    let gated = [
        ("stress", "stress-testing"),
        ("packet", "packet-inspection"),
        ("nse", "nse"),
        ("hunt", "advanced-hunting"),
        ("wireless", "wireless"),
        ("browser", "headless-browser"),
        ("mobile", "mobile"),
        ("db", "db-pentest"),
        ("proxy-intercept", "web-proxy"),
        ("c2", "c2"),
        ("evasion", "evasion"),
        ("postex", "postex"),
    ];
    for (cmd, feat) in gated {
        let enabled = eggsec::config::is_feature_enabled_registry(feat);
        assert_eq!(
            clap.contains_key(cmd),
            enabled,
            "Clap command '{cmd}' presence ({}) must match feature '{feat}' enabled ({enabled})",
            clap.contains_key(cmd),
        );
    }
}

// ─── 0.5 Runtime mapping guards ─────────────────────────────────────

#[cfg(feature = "cli")]
#[test]
fn runtime_task_operation_ids_agree_between_owners() {
    use eggsec::operation_request::runtime_adapters::operation_id_for_task_kind;
    use eggsec_runtime::request::*;

    // Construct every TaskKind variant once. Adding a variant without
    // updating both owners is a compile error (exhaustive match) or a test
    // failure here (disagreement).
    let kinds: Vec<TaskKind> = vec![
        TaskKind::LoadTest(LoadTestParams {
            target: "https://example.com".into(),
            method: "GET".into(),
            ..Default::default()
        }),
        TaskKind::StressTest(StressTestParams {
            target: "https://example.com".into(),
            flood_type: "syn".into(),
            ..Default::default()
        }),
        TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ..Default::default()
        }),
        TaskKind::EndpointScan(EndpointScanParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::Fingerprint(FingerprintParams {
            target: "10.0.0.1".into(),
            ..Default::default()
        }),
        TaskKind::Fuzz(FuzzParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::Waf(WafParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::WafStress(WafStressParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::Pipeline(PipelineParams {
            target: "https://example.com".into(),
            profile: None,
        }),
        TaskKind::Recon(ReconParams {
            target: "example.com".into(),
            modules: None,
        }),
        TaskKind::PacketCapture(PacketCaptureParams::default()),
        TaskKind::PacketTraceroute(PacketTracerouteParams {
            target: "10.0.0.1".into(),
            max_hops: None,
        }),
        TaskKind::PacketSend(PacketSendParams {
            target: "10.0.0.1".into(),
            protocol: "tcp".into(),
            ..Default::default()
        }),
        TaskKind::GraphQl(GraphQlParams {
            target: "https://example.com/graphql".into(),
            ..Default::default()
        }),
        TaskKind::OAuth(OAuthParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::AuthTest(AuthTestParams {
            target: "https://example.com".into(),
            ..Default::default()
        }),
        TaskKind::Nse(NseParams {
            target: "10.0.0.1".into(),
            script: "default".into(),
            args: None,
        }),
        TaskKind::Hunt(HuntParams {
            target: "https://example.com".into(),
            hunt_type: None,
        }),
        TaskKind::Browser(BrowserParams {
            target: "https://example.com".into(),
            headless: None,
        }),
        TaskKind::Compliance(ComplianceParams {
            target: "https://example.com".into(),
            framework: None,
        }),
        TaskKind::Storage(StorageParams {
            storage_type: "findings".into(),
            path: None,
        }),
        TaskKind::Integrations(IntegrationsParams {
            integration_type: "jira".into(),
            config: None,
        }),
        TaskKind::Workflow(WorkflowParams {
            workflow_id: None,
            steps: None,
        }),
        TaskKind::Vuln(VulnParams {
            target: "https://example.com".into(),
            vuln_type: None,
        }),
        TaskKind::Wireless(WirelessParams {
            interface: None,
            duration_secs: None,
        }),
        TaskKind::WirelessActive(WirelessActiveParams {
            interface: None,
            target_bssid: None,
        }),
        TaskKind::DbPentest(DbPentestParams {
            db_type: "postgres".into(),
            target: "localhost".into(),
            ..Default::default()
        }),
        TaskKind::Intercept(InterceptParams::default()),
        TaskKind::C2(C2Params::default()),
    ];
    assert_eq!(
        kinds.len(),
        29,
        "TaskKind variant count changed — update both mapping owners"
    );
    for kind in &kinds {
        let runtime_owned = kind.operation_id();
        let engine_owned = operation_id_for_task_kind(kind)
            .unwrap_or_else(|| panic!("engine has no mapping for {kind:?}"));
        assert_eq!(
            runtime_owned, engine_owned,
            "TaskKind {kind:?}: runtime operation_id() '{runtime_owned}' disagrees with engine mapping '{engine_owned}'"
        );
        // Every runtime-mapped operation must resolve to canonical metadata
        // (or be an explicitly documented wire family member).
        assert!(
            metadata_for_tool_id(runtime_owned).is_some(),
            "TaskKind {kind:?} maps to '{runtime_owned}' with no canonical metadata"
        );
    }
}

#[cfg(feature = "cli")]
#[test]
fn runtime_target_extraction_agrees_between_owners() {
    use eggsec::operation_request::runtime_adapters::target_for_task_kind;
    use eggsec_runtime::request::*;

    let cases: Vec<TaskKind> = vec![
        TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ..Default::default()
        }),
        TaskKind::Recon(ReconParams {
            target: "example.com".into(),
            modules: None,
        }),
        TaskKind::PacketCapture(PacketCaptureParams::default()),
        TaskKind::PacketTraceroute(PacketTracerouteParams {
            target: "10.0.0.1".into(),
            max_hops: None,
        }),
        TaskKind::Storage(StorageParams {
            storage_type: "findings".into(),
            path: None,
        }),
        TaskKind::Wireless(WirelessParams {
            interface: None,
            duration_secs: None,
        }),
        TaskKind::Intercept(InterceptParams {
            target: Some("https://example.com".into()),
            ..Default::default()
        }),
        TaskKind::C2(C2Params {
            target: None,
            ..Default::default()
        }),
    ];
    for kind in &cases {
        assert_eq!(
            kind.canonical_target(),
            target_for_task_kind(kind),
            "TaskKind {kind:?}: canonical_target() disagrees with engine target_for_task_kind()"
        );
    }
}

#[test]
fn runtime_surface_conversion_is_explicit_and_round_trippable() {
    use eggsec::config::ExecutionSurface;
    use eggsec::runtime_bridge::runtime_surface_to_execution_surface;
    use eggsec_runtime::RuntimeSurface;

    let pairs: &[(RuntimeSurface, ExecutionSurface)] = &[
        (RuntimeSurface::CliManual, ExecutionSurface::CliManual),
        (
            RuntimeSurface::CliManualStrict,
            ExecutionSurface::CliManualStrict,
        ),
        (RuntimeSurface::TuiManual, ExecutionSurface::TuiManual),
        (
            RuntimeSurface::TuiManualStrict,
            ExecutionSurface::TuiManualStrict,
        ),
        (RuntimeSurface::Ci, ExecutionSurface::Ci),
        (RuntimeSurface::McpServer, ExecutionSurface::McpServer),
        (RuntimeSurface::RestApi, ExecutionSurface::RestApi),
        (RuntimeSurface::GrpcApi, ExecutionSurface::GrpcApi),
        (
            RuntimeSurface::SecurityAgent,
            ExecutionSurface::SecurityAgent,
        ),
    ];
    for (rt, expected) in pairs {
        let mapped =
            runtime_surface_to_execution_surface(rt.clone()).expect("known surface must map");
        assert_eq!(&mapped, expected);
        // Profile derivation is stable through the bridge.
        assert_eq!(mapped.profile(), expected.profile());
    }
    assert!(runtime_surface_to_execution_surface(RuntimeSurface::Unknown).is_err());
}

#[test]
fn downstream_dispatch_binding_remains_intact() {
    // Phase 0.1 hardening must not relax engine-side validation.
    // The binding gate rejects cross-target dispatch even when the frontend
    // cache is correct; this test pins the gate in the mandatory path.
    // Uses real enforcement (no `for_test` shim) so it runs without the
    // `test-helpers` feature.
    use eggsec::config::{
        EnforcementContext, ExecutionPolicy, LoadedScope, OperationDescriptor, OperationMode,
        OperationRisk,
    };
    use eggsec::tool::dispatcher::validate_request_binding;
    use eggsec::tool::{Target, ToolRequest};

    let enforcement =
        EnforcementContext::mcp_strict(ExecutionPolicy::default(), LoadedScope::default_empty());
    let descriptor = OperationDescriptor::new(
        "scan-ports".to_string(),
        OperationMode::StandardAssessment,
        OperationRisk::SafeActive,
        vec![],
        Some("127.0.0.1".to_string()),
        vec![],
        vec![],
        false,
        false,
        vec![],
    );
    let approved = enforcement
        .approve(eggsec::config::ExecutionSurface::RestApi, descriptor)
        .expect("allowlisted loopback should approve");
    let mismatched = ToolRequest {
        id: "test".to_string(),
        tool: "scan-ports".to_string(),
        target: Target::ip("127.0.0.2"),
        params: serde_json::json!({}),
        options: Default::default(),
        cancellation_token: None,
    };
    assert!(
        validate_request_binding(&approved, &mismatched).is_err(),
        "downstream binding must still reject cross-target dispatch"
    );
}

// ─── 0.6 Feature-profile parity ─────────────────────────────────────

#[test]
fn registry_features_are_known_and_fail_closed() {
    for reg in REGISTERED_COMMANDS {
        if let Some(feat) = reg.feature {
            let state = eggsec::config::feature_state(feat);
            assert!(
                !matches!(state, FeatureState::Unknown),
                "command '{}' references unknown feature '{feat}'",
                reg.command_id
            );
        }
    }
    assert_eq!(
        eggsec::config::feature_state("definitely-not-a-feature"),
        FeatureState::Unknown
    );
    assert!(!eggsec::config::is_feature_enabled_registry(
        "definitely-not-a-feature"
    ));
}

#[test]
fn canonical_feature_gates_match_registry_command_features() {
    // Operation-backed registry entries must derive their feature gate from
    // canonical OperationMetadata (no hand-maintained drift).
    for reg in REGISTERED_COMMANDS {
        if let Some(op_id) = reg.operation_id {
            if let Some(meta) = metadata_for_tool_id(op_id) {
                assert_eq!(
                    reg.feature,
                    meta.derive_command_feature(),
                    "command '{}' feature {:?} != canonical {:?} for operation '{}'",
                    reg.command_id,
                    reg.feature,
                    meta.derive_command_feature(),
                    op_id
                );
            }
        }
    }
}

#[cfg(feature = "cli")]
#[test]
fn feature_disabled_is_absence_not_unavailable_shell() {
    // Compile-time absence (Clap command missing) and runtime unavailable
    // (metadata FeatureMissing denial) are distinct states. This test pins
    // the distinction for the current audit targets.
    let clap = reflected_clap_top_level();
    for (cmd, feat) in [
        ("stress", "stress-testing"),
        ("packet", "packet-inspection"),
    ] {
        let enabled = eggsec::config::is_feature_enabled_registry(feat);
        assert_eq!(
            clap.contains_key(cmd),
            enabled,
            "Clap '{cmd}' presence must track compile-time feature '{feat}'"
        );
        // Canonical metadata always declares the gate, even when disabled.
        let op = match cmd {
            "stress" => "stress-test",
            "packet" => "packet",
            _ => continue,
        };
        let meta = metadata_for_tool_id(op).expect("canonical must exist");
        assert!(
            meta.required_features.contains(&feat),
            "canonical '{op}' must declare feature '{feat}'"
        );
    }
}

#[test]
fn canonical_operations_without_cli_commands_are_intentional() {
    // Canonical operations with no operation-backed CLI command are
    // intentionally CLI/programmatic-only, multiplexer-covered, or
    // TUI/runtime-only. New operations must be classified here, not silently
    // unexposed. Phase 2 will decide the final production metadata owner;
    // Phase 0 freezes the current intent.
    let registry_ops: BTreeSet<&str> = REGISTERED_COMMANDS
        .iter()
        .filter_map(|r| r.operation_id)
        .collect();
    // Intentionally indirect: no RegistryBacked CLI command, but deliberately
    // exposed elsewhere (flag path, lifecycle surface, TUI tab, runtime task,
    // or helper operation).
    let intentionally_indirect: BTreeSet<&str> = BTreeSet::from([
        "waf-bypass",   // via `waf --bypass` flag path, no dedicated CLI command
        "compliance",   // TUI/runtime-only (no CLI command; TUI Compliance tab + runtime task)
        "storage", // CLI `storage` is helper-only; operation via TUI Storage tab + runtime task
        "integrations", // TUI/runtime-only (no CLI command)
        "workflow", // TUI/runtime-only (no CLI command)
        "vuln",    // CLI `vuln` is helper-only; operation via TUI Vuln tab + runtime task
        "remote",  // via exec/remote lifecycle surface (no operation-backed command)
        "search",  // NoTarget helper operation (tool/programmatic surface)
    ]);
    for meta in all_operation_metadata() {
        let direct = registry_ops.contains(meta.id)
            || ALL_OPERATION_METADATA_ALIASES
                .iter()
                .any(|(_, canonical)| *canonical == meta.id && registry_ops.contains(canonical));
        if !direct {
            assert!(
                intentionally_indirect.contains(meta.id),
                "canonical operation '{}' has no CLI command and no intentional classification — add it to intentionally_indirect or REGISTERED_COMMANDS",
                meta.id
            );
        }
    }
    // Every intentional entry must still be indirect (no stale entries).
    for op in &intentionally_indirect {
        let direct = registry_ops.contains(op)
            || ALL_OPERATION_METADATA_ALIASES
                .iter()
                .any(|(_, canonical)| *canonical == *op && registry_ops.contains(canonical));
        assert!(
            !direct,
            "intentionally_indirect entry '{op}' is now directly covered — remove it from the exception list"
        );
    }
}
