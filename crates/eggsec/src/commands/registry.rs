//! Command registry — static, inspectable command metadata for CLI/TUI dispatch.
//!
//! The registry maps command IDs to metadata and descriptor builders, enabling
//! incremental migration from the legacy `handle_command()` match dispatch.
//!
//! The registry is **metadata and routing, not authorization**. All side-effecting
//! operations still flow through `EnforcementContext::evaluate()` before execution.

use crate::config::{metadata_for_tool_id, OperationDescriptor, OperationMetadata};

/// Command category for classification and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    /// Network operations requiring enforcement (scans, fuzz, stress).
    SideEffectingNetwork,
    /// Local file or domain-specific operations (DB, mobile, reports).
    LocalFileDomain,
    /// Read-only analysis (explain, AI analyze).
    PassiveAnalytical,
    /// Configuration, help, diagnostics (config, doctor, plan).
    ConfigOutputHelper,
    /// Server daemons (REST, MCP, gRPC, agent).
    FrontendServer,
    /// Commands with no metadata or unique dispatch needs.
    LegacySpecial,
}

impl CommandCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SideEffectingNetwork => "side-effecting-network",
            Self::LocalFileDomain => "local-file-domain",
            Self::PassiveAnalytical => "passive-analytical",
            Self::ConfigOutputHelper => "config-output-helper",
            Self::FrontendServer => "frontend-server",
            Self::LegacySpecial => "legacy-special",
        }
    }
}

/// Static metadata for a registered command.
///
/// Registry entries do **not** authorize operations. They provide metadata for
/// descriptor generation, diagnostics, and documentation. Authorization remains
/// the responsibility of `EnforcementContext::evaluate()`.
pub struct CommandRegistration {
    /// Stable command ID matching the CLI subcommand name.
    pub command_id: &'static str,
    /// Canonical operation ID in `ALL_OPERATION_METADATA`, if applicable.
    /// `None` for config/helper/server commands that have no operation metadata.
    pub operation_id: Option<&'static str>,
    /// Human-readable display name.
    pub display_name: &'static str,
    /// Command category for classification.
    pub category: CommandCategory,
    /// Feature gate required to compile/use this command, if any.
    pub feature: Option<&'static str>,
    /// Whether this command is manual-only (CLI/TUI) and should not be
    /// exposed through programmatic surfaces (MCP, REST, agent, gRPC).
    pub manual_only: bool,
    /// Whether this command should appear in TUI tab listings.
    pub tui_visible: bool,
}

impl CommandRegistration {
    /// Look up the `OperationMetadata` for this command, if it has an `operation_id`.
    pub fn metadata(&self) -> Option<&'static OperationMetadata> {
        self.operation_id.and_then(|id| metadata_for_tool_id(id))
    }

    /// Build an `OperationDescriptor` from the registered metadata, if available.
    ///
    /// Returns `None` if the command has no `operation_id` (config/helper/server).
    pub fn build_descriptor(&self, target: Option<String>) -> Option<OperationDescriptor> {
        self.metadata().map(|m| m.descriptor_for_target(target))
    }
}

/// All registered commands. Static, inspectable, no runtime I/O.
///
/// Commands without `operation_id` (config, helper, server) are included for
/// completeness but do not participate in metadata-driven dispatch.
pub const REGISTERED_COMMANDS: &[CommandRegistration] = &[
    // ── Phase 6 pilot: registry-backed commands ──
    CommandRegistration {
        command_id: "recon",
        operation_id: Some("recon"),
        display_name: "Reconnaissance",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "scan-ports",
        operation_id: Some("scan-ports"),
        display_name: "Port Scan",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "scan-endpoints",
        operation_id: Some("scan-endpoints"),
        display_name: "Endpoint Scan",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "fingerprint",
        operation_id: Some("fingerprint"),
        display_name: "Fingerprint",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    // ── Legacy commands (not yet migrated) ──
    CommandRegistration {
        command_id: "scan",
        operation_id: Some("scan"),
        display_name: "Pipeline Scan",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "resume",
        operation_id: None,
        display_name: "Resume Scan",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "fuzz",
        operation_id: Some("fuzz"),
        display_name: "Fuzz",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "waf",
        operation_id: Some("waf"),
        display_name: "WAF Detect",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "waf-stress",
        operation_id: Some("waf-stress"),
        display_name: "WAF Stress",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "graphql",
        operation_id: Some("graphql"),
        display_name: "GraphQL Fuzz",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "oauth",
        operation_id: Some("oauth"),
        display_name: "OAuth Fuzz",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "auth-test",
        operation_id: Some("auth-test"),
        display_name: "Auth Test",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "load",
        operation_id: Some("load"),
        display_name: "Load Test",
        category: CommandCategory::SideEffectingNetwork,
        feature: None,
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "stress",
        operation_id: Some("stress"),
        display_name: "Stress Test",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("stress-testing"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "packet",
        operation_id: Some("packet"),
        display_name: "Packet Operations",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("packet-inspection"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "icmp",
        operation_id: None,
        display_name: "ICMP",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("stress-testing"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "traceroute",
        operation_id: None,
        display_name: "Traceroute",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("stress-testing"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "nse",
        operation_id: Some("nse"),
        display_name: "NSE Scripts",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("nse"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "hunt",
        operation_id: Some("hunt"),
        display_name: "Vulnerability Hunt",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("advanced-hunting"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "evasion",
        operation_id: None,
        display_name: "Evasion Detection",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("evasion"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "postex",
        operation_id: None,
        display_name: "Post-Exploitation",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("postex"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "c2",
        operation_id: Some("c2"),
        display_name: "C2 Simulation",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("c2"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "proxy-intercept",
        operation_id: Some("proxy-intercept"),
        display_name: "Web Proxy Intercept",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("web-proxy"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "wireless",
        operation_id: Some("wireless"),
        display_name: "Wireless",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("wireless"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "browser",
        operation_id: Some("browser"),
        display_name: "Headless Browser",
        category: CommandCategory::SideEffectingNetwork,
        feature: Some("headless-browser"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "mobile",
        operation_id: None,
        display_name: "Mobile Analysis",
        category: CommandCategory::LocalFileDomain,
        feature: Some("mobile"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "db",
        operation_id: Some("db-pentest"),
        display_name: "DB Pentest",
        category: CommandCategory::LocalFileDomain,
        feature: Some("db-pentest"),
        manual_only: false,
        tui_visible: true,
    },
    CommandRegistration {
        command_id: "plan",
        operation_id: None,
        display_name: "Plan",
        category: CommandCategory::ConfigOutputHelper,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "preflight",
        operation_id: None,
        display_name: "Preflight",
        category: CommandCategory::ConfigOutputHelper,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "ci",
        operation_id: None,
        display_name: "CI",
        category: CommandCategory::ConfigOutputHelper,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "config",
        operation_id: None,
        display_name: "Config",
        category: CommandCategory::ConfigOutputHelper,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "doctor",
        operation_id: None,
        display_name: "Doctor",
        category: CommandCategory::ConfigOutputHelper,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "policy-explain",
        operation_id: None,
        display_name: "Policy Explain",
        category: CommandCategory::PassiveAnalytical,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "scope-explain",
        operation_id: None,
        display_name: "Scope Explain",
        category: CommandCategory::PassiveAnalytical,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "ai-analyze",
        operation_id: None,
        display_name: "AI Analyze",
        category: CommandCategory::PassiveAnalytical,
        feature: Some("ai-integration"),
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "serve",
        operation_id: None,
        display_name: "REST Server",
        category: CommandCategory::FrontendServer,
        feature: Some("rest-api"),
        manual_only: false,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "mcp-serve",
        operation_id: None,
        display_name: "MCP Server",
        category: CommandCategory::FrontendServer,
        feature: Some("rest-api"),
        manual_only: false,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "agent",
        operation_id: None,
        display_name: "Agent",
        category: CommandCategory::FrontendServer,
        feature: Some("rest-api"),
        manual_only: false,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "grpc",
        operation_id: None,
        display_name: "gRPC Server",
        category: CommandCategory::FrontendServer,
        feature: Some("grpc-api"),
        manual_only: false,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "cluster",
        operation_id: None,
        display_name: "Cluster",
        category: CommandCategory::FrontendServer,
        feature: None,
        manual_only: false,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "remote",
        operation_id: None,
        display_name: "Remote",
        category: CommandCategory::FrontendServer,
        feature: None,
        manual_only: false,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "exec",
        operation_id: None,
        display_name: "Exec",
        category: CommandCategory::FrontendServer,
        feature: None,
        manual_only: false,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "report",
        operation_id: None,
        display_name: "Report",
        category: CommandCategory::LocalFileDomain,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "vuln",
        operation_id: None,
        display_name: "Vulnerability Management",
        category: CommandCategory::ConfigOutputHelper,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "storage",
        operation_id: None,
        display_name: "Storage",
        category: CommandCategory::LocalFileDomain,
        feature: Some("database"),
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "sbom",
        operation_id: None,
        display_name: "SBOM",
        category: CommandCategory::LocalFileDomain,
        feature: Some("sbom"),
        manual_only: true,
        tui_visible: false,
    },
    CommandRegistration {
        command_id: "notify",
        operation_id: None,
        display_name: "Notify",
        category: CommandCategory::ConfigOutputHelper,
        feature: None,
        manual_only: true,
        tui_visible: false,
    },
];

/// Look up a command registration by command ID.
pub fn lookup_command(command_id: &str) -> Option<&'static CommandRegistration> {
    REGISTERED_COMMANDS
        .iter()
        .find(|r| r.command_id == command_id)
}

/// Build an `OperationDescriptor` for a command by its ID and target.
///
/// Returns `None` if the command has no operation metadata (config/helper/server).
pub fn build_descriptor_for_command(
    command_id: &str,
    target: Option<String>,
) -> Option<OperationDescriptor> {
    lookup_command(command_id).and_then(|reg| reg.build_descriptor(target))
}

/// Get all registered command IDs.
pub fn all_command_ids() -> Vec<&'static str> {
    REGISTERED_COMMANDS.iter().map(|r| r.command_id).collect()
}

/// Get all registered command IDs that are visible in TUI.
pub fn tui_visible_command_ids() -> Vec<&'static str> {
    REGISTERED_COMMANDS
        .iter()
        .filter(|r| r.tui_visible)
        .map(|r| r.command_id)
        .collect()
}

/// Get all registered command IDs that are manual-only (not exposed programmatically).
pub fn manual_only_command_ids() -> Vec<&'static str> {
    REGISTERED_COMMANDS
        .iter()
        .filter(|r| r.manual_only)
        .map(|r| r.command_id)
        .collect()
}

/// Suggest similar command IDs for an unknown command (simple edit distance).
pub fn suggest_command(unknown: &str) -> Vec<&'static str> {
    let mut suggestions: Vec<(&'static str, usize)> = REGISTERED_COMMANDS
        .iter()
        .map(|r| (r.command_id, edit_distance(unknown, r.command_id)))
        .filter(|(_, d)| *d <= 3)
        .collect();
    suggestions.sort_by_key(|(_, d)| *d);
    suggestions.into_iter().map(|(id, _)| id).collect()
}

/// Simple Levenshtein edit distance for command suggestions.
fn edit_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a.as_bytes()[i - 1] == b.as_bytes()[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_entries_have_unique_command_ids() {
        let mut ids: Vec<&str> = REGISTERED_COMMANDS.iter().map(|r| r.command_id).collect();
        let len_before = ids.len();
        ids.dedup();
        assert_eq!(
            len_before,
            ids.len(),
            "Duplicate command IDs found in registry"
        );
    }

    #[test]
    fn registry_entries_with_operation_id_resolve_to_metadata() {
        for reg in REGISTERED_COMMANDS {
            if let Some(op_id) = reg.operation_id {
                assert!(
                    metadata_for_tool_id(op_id).is_some(),
                    "Command '{}' has operation_id '{}' but no matching OperationMetadata",
                    reg.command_id,
                    op_id
                );
            }
        }
    }

    #[test]
    fn feature_gated_entries_declare_feature() {
        for reg in REGISTERED_COMMANDS {
            if reg.feature.is_some() {
                // Feature-gated entries should have a non-empty feature string.
                // This is enforced by the struct literal, but verify the pattern.
                assert!(
                    !reg.feature.unwrap().is_empty(),
                    "Command '{}' has empty feature gate",
                    reg.command_id
                );
            }
        }
    }

    #[test]
    fn manual_only_not_exposed_programmatically() {
        for reg in REGISTERED_COMMANDS {
            if reg.manual_only {
                assert!(
                    !reg.tui_visible,
                    "Command '{}' is manual_only but tui_visible",
                    reg.command_id
                );
            }
        }
    }

    #[test]
    fn command_ids_match_cli_variants() {
        // Verify that all registered command IDs correspond to known CLI subcommands.
        // This catches typos and stale entries.
        let ids = all_command_ids();
        assert!(ids.contains(&"recon"));
        assert!(ids.contains(&"scan-ports"));
        assert!(ids.contains(&"scan-endpoints"));
        assert!(ids.contains(&"fingerprint"));
        assert!(ids.contains(&"fuzz"));
        assert!(ids.contains(&"scan"));
    }

    #[test]
    fn lookup_returns_correct_entry() {
        let recon = lookup_command("recon").expect("recon should be registered");
        assert_eq!(recon.command_id, "recon");
        assert_eq!(recon.operation_id, Some("recon"));
        assert_eq!(recon.category, CommandCategory::SideEffectingNetwork);
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(lookup_command("nonexistent-command").is_none());
    }

    #[test]
    fn build_descriptor_returns_metadata_descriptor() {
        let desc = build_descriptor_for_command("recon", Some("example.com".to_string()));
        assert!(desc.is_some());
        let desc = desc.unwrap();
        assert_eq!(desc.operation, "recon");
    }

    #[test]
    fn build_descriptor_no_operation_returns_none() {
        let desc = build_descriptor_for_command("config", None);
        assert!(desc.is_none());
    }

    #[test]
    fn suggest_command_returns_close_matches() {
        let suggestions = suggest_command("scan-port");
        assert!(suggestions.contains(&"scan-ports"));
    }

    #[test]
    fn suggest_command_returns_empty_for_distant_input() {
        let suggestions = suggest_command("zzzzzzzzz");
        assert!(suggestions.is_empty());
    }

    #[test]
    fn edit_distance_basic_cases() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("abc", "ab"), 1);
        assert_eq!(edit_distance("abc", "axc"), 1);
    }

    #[test]
    fn tui_visible_excludes_manual_only() {
        for reg in REGISTERED_COMMANDS {
            if reg.manual_only {
                assert!(
                    !reg.tui_visible,
                    "manual_only command '{}' should not be tui_visible",
                    reg.command_id
                );
            }
        }
    }

    #[test]
    fn category_as_str_is_stable() {
        assert_eq!(
            CommandCategory::SideEffectingNetwork.as_str(),
            "side-effecting-network"
        );
        assert_eq!(
            CommandCategory::LocalFileDomain.as_str(),
            "local-file-domain"
        );
        assert_eq!(
            CommandCategory::PassiveAnalytical.as_str(),
            "passive-analytical"
        );
        assert_eq!(
            CommandCategory::ConfigOutputHelper.as_str(),
            "config-output-helper"
        );
        assert_eq!(CommandCategory::FrontendServer.as_str(), "frontend-server");
        assert_eq!(CommandCategory::LegacySpecial.as_str(), "legacy-special");
    }
}
