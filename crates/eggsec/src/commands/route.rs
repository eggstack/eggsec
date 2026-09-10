//! CLI command routing contract — Phase 1 convergence.
//!
//! The static metadata registry (`registry.rs`) describes command metadata and
//! dispatch class. This module is the **single coherent owner** for the
//! `Commands → CommandRoute` conversion: it classifies every Clap variant once
//! as operation-backed, operation multiplexer, helper, or lifecycle, with
//! canonical operation IDs resolved (aliases resolved here, never at executor
//! entry).
//!
//! The conversion is exhaustive over [`crate::cli::Commands`]: adding a Clap
//! variant without updating [`route_for_commands`] is a compile error. The
//! previous dual ownership (registry validation prelude + separate exhaustive
//! `handle_command` match that silently chose the handler) is removed:
//! `handle_command` classifies once via this module and then converts
//! operation-backed variants through canonical request adapters.
//!
//! Helper and lifecycle commands remain explicit non-operation routes; they
//! are not forced into the security-operation catalog.

use crate::cli::Commands;

/// How a CLI command routes to execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRoute {
    /// Single canonical operation-backed execution (alias already resolved).
    Operation {
        command_id: &'static str,
        operation_id: &'static str,
    },
    /// Multiplexer over canonical operations. The concrete operation/request
    /// is selected per execution branch *before* approval and execution
    /// (e.g. packet capture/traceroute/send, mobile static/dynamic, pipeline
    /// scan/resume, wireless base/active).
    Multiplexer {
        command_id: &'static str,
        operations: &'static [&'static str],
    },
    /// Read-only helper/diagnostic or local transformation (config, doctor,
    /// plan, preflight, report, storage CLI, etc.). Never dispatches an
    /// operation.
    Helper { command_id: &'static str },
    /// Server/runtime lifecycle (serve, mcp-serve, agent, grpc, cluster,
    /// remote, exec, daemon client, etc.). Never dispatches an operation.
    Lifecycle { command_id: &'static str },
}

impl CommandRoute {
    /// Stable command ID for this route.
    pub fn command_id(&self) -> &'static str {
        match self {
            Self::Operation { command_id, .. }
            | Self::Multiplexer { command_id, .. }
            | Self::Helper { command_id }
            | Self::Lifecycle { command_id } => command_id,
        }
    }

    /// Canonical operation ID for single-operation routes.
    ///
    /// Returns `None` for multiplexers (branch-selected), helpers, and
    /// lifecycle commands. Multiplexer branches expose their operation set
    /// via [`Self::operations`].
    pub fn operation_id(&self) -> Option<&'static str> {
        match self {
            Self::Operation { operation_id, .. } => Some(operation_id),
            Self::Multiplexer { .. } | Self::Helper { .. } | Self::Lifecycle { .. } => None,
        }
    }

    /// Candidate operations for multiplexer routes.
    pub fn operations(&self) -> Vec<&'static str> {
        match self {
            Self::Multiplexer { operations, .. } => operations.to_vec(),
            Self::Operation { operation_id, .. } => vec![operation_id],
            Self::Helper { .. } | Self::Lifecycle { .. } => vec![],
        }
    }

    /// Whether this route performs operation-backed execution (single or
    /// multiplexer). Helpers and lifecycle commands never do.
    pub fn is_operation_backed(&self) -> bool {
        matches!(self, Self::Operation { .. } | Self::Multiplexer { .. })
    }

    /// Whether this route is an explicit non-operation path.
    pub fn is_non_operation(&self) -> bool {
        matches!(self, Self::Helper { .. } | Self::Lifecycle { .. })
    }
}

/// Classify a Clap [`Commands`] variant once.
///
/// Exhaustive: no wildcard arm. Adding a `Commands` variant without updating
/// this function is a compile error, which guarantees the registry
/// classification and the actual CLI conversion path share one owner.
///
/// Canonical operation IDs only; user-facing aliases (`waf` → `waf-detect`,
/// `scan`/`resume` → `pipeline`, `load` → `load-test`, `o-auth` → `oauth`)
/// are resolved here.
pub fn route_for_commands(cmd: &Commands) -> CommandRoute {
    match cmd {
        // ── Single-operation routes (canonical IDs) ──
        Commands::ScanPorts(_) => CommandRoute::Operation {
            command_id: "scan-ports",
            operation_id: "scan-ports",
        },
        Commands::ScanEndpoints(_) => CommandRoute::Operation {
            command_id: "scan-endpoints",
            operation_id: "scan-endpoints",
        },
        Commands::Fingerprint(_) => CommandRoute::Operation {
            command_id: "fingerprint",
            operation_id: "fingerprint",
        },
        Commands::Fuzz(_) => CommandRoute::Operation {
            command_id: "fuzz",
            operation_id: "fuzz",
        },
        // `waf` CLI alias resolves to canonical `waf-detect` before execution.
        Commands::Waf(_) => CommandRoute::Operation {
            command_id: "waf",
            operation_id: "waf-detect",
        },
        Commands::WafStress(_) => CommandRoute::Operation {
            command_id: "waf-stress",
            operation_id: "waf-stress",
        },
        Commands::Graphql(_) => CommandRoute::Operation {
            command_id: "graphql",
            operation_id: "graphql",
        },
        Commands::OAuth(_) => CommandRoute::Operation {
            command_id: "oauth",
            operation_id: "oauth",
        },
        Commands::AuthTest(_) => CommandRoute::Operation {
            command_id: "auth-test",
            operation_id: "auth-test",
        },
        Commands::Recon(_) => CommandRoute::Operation {
            command_id: "recon",
            operation_id: "recon",
        },
        // `load` CLI alias resolves to canonical `load-test`.
        Commands::Load(_) => CommandRoute::Operation {
            command_id: "load",
            operation_id: "load-test",
        },
        #[cfg(feature = "nse")]
        Commands::Nse(_) => CommandRoute::Operation {
            command_id: "nse",
            operation_id: "nse",
        },
        #[cfg(feature = "advanced-hunting")]
        Commands::Hunt(_) => CommandRoute::Operation {
            command_id: "hunt",
            operation_id: "hunt",
        },
        #[cfg(feature = "stress-testing")]
        Commands::Stress(_) => CommandRoute::Operation {
            command_id: "stress",
            operation_id: "stress-test",
        },
        #[cfg(feature = "web-proxy")]
        Commands::ProxyIntercept(_) => CommandRoute::Operation {
            command_id: "proxy-intercept",
            operation_id: "proxy-intercept",
        },
        #[cfg(feature = "wireless")]
        Commands::Wireless(_) => CommandRoute::Multiplexer {
            // Base scan vs active deauth is branch-selected from
            // WirelessArgs mode before approval (subcommand identity
            // `wireless-deauth`).
            command_id: "wireless",
            operations: &["wireless", "wireless-deauth"],
        },
        #[cfg(feature = "headless-browser")]
        Commands::Browser(_) => CommandRoute::Operation {
            command_id: "browser",
            operation_id: "browser",
        },
        #[cfg(feature = "mobile")]
        Commands::Mobile(_) => CommandRoute::Multiplexer {
            // Static vs dynamic analysis is branch-selected from the mobile
            // subcommand before approval (subcommand identities
            // `mobile-static`/`mobile-dynamic`).
            command_id: "mobile",
            operations: &["mobile-static", "mobile-dynamic"],
        },
        #[cfg(feature = "evasion")]
        Commands::Evasion(_) => CommandRoute::Operation {
            command_id: "evasion",
            operation_id: "evasion",
        },
        #[cfg(feature = "postex")]
        Commands::Postex(_) => CommandRoute::Operation {
            command_id: "postex",
            operation_id: "postex",
        },
        #[cfg(feature = "c2")]
        Commands::C2(_) => CommandRoute::Operation {
            command_id: "c2",
            operation_id: "c2",
        },
        #[cfg(feature = "db-pentest")]
        Commands::Db(_) => CommandRoute::Operation {
            command_id: "db",
            operation_id: "db-pentest",
        },
        // ── Multiplexers (branch-selected before approval) ──
        Commands::Scan(_) | Commands::Resume(_) => CommandRoute::Multiplexer {
            command_id: "scan",
            operations: &["pipeline"],
        },
        #[cfg(feature = "packet-inspection")]
        Commands::Packet(_) => CommandRoute::Multiplexer {
            command_id: "packet",
            operations: &["packet"],
        },
        #[cfg(feature = "stress-testing")]
        Commands::Icmp(_) | Commands::Traceroute(_) => CommandRoute::Multiplexer {
            command_id: "packet",
            operations: &["packet"],
        },
        // ── Helpers (explicit non-operation) ──
        Commands::Plan(_)
        | Commands::Preflight(_)
        | Commands::Ci(_)
        | Commands::Config(_)
        | Commands::PolicyExplain(_)
        | Commands::ScopeExplain(_)
        | Commands::Report(_)
        | Commands::Vuln(_)
        | Commands::Storage(_)
        | Commands::Notify(_) => CommandRoute::Helper {
            command_id: "helper",
        },
        Commands::Doctor => CommandRoute::Helper {
            command_id: "doctor",
        },
        #[cfg(feature = "sbom")]
        Commands::Sbom(_) => CommandRoute::Helper { command_id: "sbom" },
        #[cfg(feature = "ai-integration")]
        Commands::AiAnalyze(_) => CommandRoute::Helper {
            command_id: "ai-analyze",
        },
        #[cfg(feature = "stress-testing")]
        Commands::Proxy(_) => CommandRoute::Helper {
            command_id: "proxy",
        },
        // ── Lifecycle (server/runtime; explicit non-operation) ──
        Commands::Cluster(_) | Commands::Remote(_) | Commands::Exec(_) => CommandRoute::Lifecycle {
            command_id: "lifecycle",
        },
        #[cfg(feature = "rest-api")]
        Commands::Serve(_)
        | Commands::McpServe(_)
        | Commands::CodeggMcp(_)
        | Commands::Agent(_) => CommandRoute::Lifecycle {
            command_id: "lifecycle",
        },
        #[cfg(feature = "grpc-api")]
        Commands::Grpc(_) => CommandRoute::Lifecycle {
            command_id: "lifecycle",
        },
        #[cfg(feature = "daemon-client")]
        Commands::Daemon(_) | Commands::Session(_) | Commands::Task(_) => CommandRoute::Lifecycle {
            command_id: "lifecycle",
        },
    }
}

/// Classify a command ID string via the registry.
///
/// String-based callers (diagnostics, suggestions) resolve through the static
/// registry; the authoritative `Commands → CommandRoute` mapping above is
/// exhaustive over the Clap enum. Returns `None` for unknown IDs.
pub fn route_for_command_id(command_id: &str) -> Option<CommandRoute> {
    let reg = super::registry::lookup_command(command_id)?;
    if let Some(operation_id) = reg.operation_id {
        // Multiplexer command IDs share one operation family across branches.
        let is_multiplexer = matches!(
            command_id,
            "scan" | "resume" | "packet" | "icmp" | "traceroute" | "mobile" | "wireless"
        );
        if is_multiplexer {
            // Closed multiplexer set with static operation tables (no
            // runtime-constructed statics). Single-operation multiplexers
            // (scan/resume/packet family) list their canonical family.
            let operations: &'static [&'static str] = match command_id {
                "mobile" => &["mobile-static", "mobile-dynamic"],
                "wireless" => &["wireless", "wireless-deauth"],
                "scan" | "resume" => &["pipeline"],
                "packet" | "icmp" | "traceroute" => &["packet"],
                unknown => unreachable!(
                    "multiplexer command '{unknown}' has no static operation table; update route_for_command_id"
                ),
            };
            Some(CommandRoute::Multiplexer {
                command_id: reg.command_id,
                operations,
            })
        } else {
            Some(CommandRoute::Operation {
                command_id: reg.command_id,
                operation_id,
            })
        }
    } else {
        match reg.dispatch_mode {
            super::registry::CommandDispatchMode::ServerLifecycle => {
                Some(CommandRoute::Lifecycle {
                    command_id: reg.command_id,
                })
            }
            super::registry::CommandDispatchMode::HelperOnly
            | super::registry::CommandDispatchMode::CatalogOnly => Some(CommandRoute::Helper {
                command_id: reg.command_id,
            }),
            super::registry::CommandDispatchMode::RegistryBacked => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_routes_carry_canonical_ids_not_aliases() {
        // Spot-check alias resolution at the routing boundary.
        let waf = route_for_command_id("waf").expect("waf registered");
        assert_eq!(waf.operation_id(), Some("waf-detect"));

        let scan = route_for_command_id("scan").expect("scan registered");
        assert!(matches!(scan, CommandRoute::Multiplexer { .. }));
        assert!(scan.operations().contains(&"pipeline"));
    }

    #[test]
    fn helpers_and_lifecycle_are_explicit_non_operation() {
        for id in ["plan", "config", "doctor", "report", "storage"] {
            let route = route_for_command_id(id).expect("helper registered");
            assert!(
                route.is_non_operation(),
                "{id} must be an explicit non-operation route"
            );
            assert!(!route.is_operation_backed());
        }
        for id in [
            "serve",
            "mcp-serve",
            "agent",
            "cluster",
            "remote-serve",
            "exec",
        ] {
            if let Some(route) = route_for_command_id(id) {
                assert!(
                    route.is_non_operation(),
                    "{id} must be an explicit non-operation route"
                );
            }
        }
    }

    #[test]
    fn registry_operation_backed_agrees_with_route() {
        // Every registry operation-backed entry must resolve to an
        // operation-backed route (single or multiplexer). This pins the two
        // owners together: registry metadata and CommandRoute classification
        // cannot silently disagree.
        for reg in super::super::registry::REGISTERED_COMMANDS {
            if reg.operation_id.is_some() {
                let route = route_for_command_id(reg.command_id)
                    .unwrap_or_else(|| panic!("no route for {}", reg.command_id));
                assert!(
                    route.is_operation_backed(),
                    "registry operation-backed '{}' must map to an operation-backed route",
                    reg.command_id
                );
            }
        }
    }
}
