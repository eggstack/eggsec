use crate::config::{all_operation_metadata, OperationMetadata, OperationRisk};
use crate::domain::all_domain_descriptors;
use crate::tool::traits::ToolCategory;

/// Canonical tool registration — single source of truth for tool listing across
/// MCP, REST, gRPC, and agent surfaces.
///
/// Each registration derives from [`OperationMetadata`] and is cross-referenced
/// with [`DomainDescriptor`](crate::domain::DomainDescriptor) tool integrations
/// for MCP exposure settings.
#[derive(Debug, Clone, Copy)]
pub struct ToolRegistration {
    pub tool_id: &'static str,
    pub operation_id: &'static str,
    pub display_name: &'static str,
    pub source: ToolRegistrationSource,
    pub feature: Option<&'static str>,
    pub required_mcp_feature: Option<&'static str>,
    /// Whether the operation metadata declares this tool as MCP-exposable.
    pub mcp_metadata_exposable: bool,
    /// Whether the tool appears in the default MCP tool listing (conservative).
    pub mcp_default_visible: bool,
    pub rest_exposable: bool,
    pub grpc_exposable: bool,
    pub agent_exposable: bool,
    pub category: ToolCategory,
}

/// Origin of a tool registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRegistrationSource {
    /// Base tool always registered (recon, scanner, fuzzer, etc.)
    Base,
    /// Feature-gated tool (web-proxy-mcp, db-pentest-mcp, c2-mcp)
    FeatureGated(&'static str),
    /// Domain-provided tool integration
    Domain(&'static str),
}

/// Determine whether an operation should be visible in the default MCP listing.
///
/// Conservative: passive/safe-active, metadata-exposable, and no feature gate.
fn default_mcp_visible_for_operation(meta: &OperationMetadata) -> bool {
    matches!(
        meta.risk,
        OperationRisk::Passive | OperationRisk::SafeActive
    ) && meta.mcp_exposable
        && meta.required_features.is_empty()
}

fn operation_category(meta: &OperationMetadata) -> ToolCategory {
    match meta.id {
        "recon" => ToolCategory::Recon,
        "scan-ports" | "scan-endpoints" | "fingerprint" => ToolCategory::Scanning,
        "fuzz" | "graphql" => ToolCategory::Fuzzing,
        "waf-detect" | "waf-bypass" | "waf-stress" => ToolCategory::Waf,
        "load-test" => ToolCategory::LoadTest,
        "stress-test" => ToolCategory::Stress,
        "pipeline" => ToolCategory::Pipeline,
        _ => ToolCategory::Recon,
    }
}

/// Returns all known tool registrations by deriving from [`OperationMetadata`]
/// and [`DomainDescriptor`](crate::domain::DomainDescriptor) tool integrations.
///
/// This is the canonical source for tool listing across all surfaces.
pub fn all_tool_registrations() -> Vec<ToolRegistration> {
    let domains = all_domain_descriptors();
    let mut registrations = Vec::new();

    for meta in all_operation_metadata() {
        let mut tool_id = meta.id;
        let mut source = ToolRegistrationSource::Base;
        let mut feature = meta.required_features.first().copied();
        let mut mcp_metadata_exposable = meta.mcp_exposable;
        let mut mcp_default_visible = default_mcp_visible_for_operation(meta);
        let mut required_mcp_feature: Option<&str> = None;

        for domain in domains {
            if let Some(tool) = domain.tools.iter().find(|t| t.operation_id == meta.id) {
                tool_id = tool.tool_id;
                source = ToolRegistrationSource::Domain(domain.id);
                mcp_metadata_exposable = meta.mcp_exposable;
                mcp_default_visible = tool.mcp_exposed_by_default;
                required_mcp_feature = tool.required_mcp_feature;
                feature = domain.required_feature;
                break;
            }
        }

        if let ToolRegistrationSource::Base = source {
            if let Some(f) = feature {
                source = ToolRegistrationSource::FeatureGated(f);
            }
        }

        registrations.push(ToolRegistration {
            tool_id,
            operation_id: meta.id,
            display_name: meta.display_name,
            source,
            feature,
            required_mcp_feature,
            mcp_metadata_exposable,
            mcp_default_visible,
            rest_exposable: meta.rest_exposable,
            grpc_exposable: meta.grpc_exposable,
            agent_exposable: meta.agent_exposable,
            category: operation_category(meta),
        });
    }

    registrations
}

/// Resolve a tool_id to its canonical registration, checking both direct
/// match and alias resolution. Returns the registration if found.
pub fn resolve_tool_registration(tool_id: &str) -> Option<ToolRegistration> {
    all_tool_registrations()
        .into_iter()
        .find(|r| r.tool_id == tool_id || r.operation_id == tool_id)
}

/// Returns tool registrations visible under the given MCP profile policy.
///
/// This implements **Model A** (profile-expanded metadata-exposable listing):
/// the OpsAgent profile intentionally returns all tools with
/// `mcp_metadata_exposable = true`. This is **not** the conservative default
/// listing — for that, use [`mcp_tool_registrations_default_visible`]. Strict
/// runtime policy (`EnforcementContext::evaluate()` + `ApprovedOperation`) is
/// still required before any tool listed here can execute.
///
/// - `"ops-agent"`: profile-expanded — every `mcp_metadata_exposable` tool
/// - `"coding-agent"`: hardcoded narrow allowlist (scan-ports, fingerprint,
///   scan-endpoints, endpoints, waf-detect, search)
/// - any other: empty
///
/// See `docs/TOOL_REGISTRATION.md` for the full exposure model.
pub fn mcp_tool_registrations(profile: &str) -> Vec<ToolRegistration> {
    let all = all_tool_registrations();
    match profile {
        "ops-agent" => all
            .into_iter()
            .filter(|r| r.mcp_metadata_exposable)
            .collect(),
        "coding-agent" => {
            let coding_agent_ids = [
                "scan",
                "scan-ports",
                "fingerprint",
                "scan-endpoints",
                "endpoints",
                "waf-detect",
                "search",
            ];
            all.into_iter()
                .filter(|r| {
                    coding_agent_ids.contains(&r.tool_id)
                        || coding_agent_ids.contains(&r.operation_id)
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Returns tool registrations that are visible in the default MCP listing.
///
/// This is the conservative subset: passive/safe-active operations with
/// `mcp_metadata_exposable = true` and no feature gate requirement.
pub fn mcp_tool_registrations_default_visible() -> Vec<ToolRegistration> {
    all_tool_registrations()
        .into_iter()
        .filter(|r| r.mcp_default_visible)
        .collect()
}

/// Returns tool registrations exposed via the REST API.
pub fn rest_tool_registrations() -> Vec<ToolRegistration> {
    all_tool_registrations()
        .into_iter()
        .filter(|r| r.rest_exposable)
        .collect()
}

/// Returns tool registrations exposed via the gRPC API.
pub fn grpc_tool_registrations() -> Vec<ToolRegistration> {
    all_tool_registrations()
        .into_iter()
        .filter(|r| r.grpc_exposable)
        .collect()
}

/// Returns tool registrations exposed to the security agent.
pub fn agent_tool_registrations() -> Vec<ToolRegistration> {
    all_tool_registrations()
        .into_iter()
        .filter(|r| r.agent_exposable)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_registrations_have_non_empty_ids() {
        for reg in all_tool_registrations() {
            assert!(!reg.tool_id.is_empty());
            assert!(!reg.operation_id.is_empty());
            assert!(!reg.display_name.is_empty());
        }
    }

    #[test]
    fn base_tools_are_always_present() {
        let regs = all_tool_registrations();
        let ids: Vec<&str> = regs.iter().map(|r| r.tool_id).collect();
        for &expected in &[
            "recon",
            "scan-ports",
            "fingerprint",
            "scan-endpoints",
            "fuzz",
            "load-test",
            "waf-detect",
            "waf-bypass",
            "waf-stress",
            "pipeline",
            "search",
        ] {
            assert!(
                ids.contains(&expected),
                "base tool '{}' missing from registrations",
                expected
            );
        }
    }

    #[test]
    fn base_tools_have_no_feature_gate() {
        let regs = all_tool_registrations();
        let base_ids = [
            "recon",
            "scan-ports",
            "fingerprint",
            "scan-endpoints",
            "fuzz",
            "load-test",
            "waf-detect",
            "waf-bypass",
            "waf-stress",
            "pipeline",
            "search",
        ];
        for reg in &regs {
            if base_ids.contains(&reg.tool_id) {
                assert_eq!(
                    reg.source,
                    ToolRegistrationSource::Base,
                    "base tool '{}' should have Base source",
                    reg.tool_id
                );
            }
        }
    }

    #[test]
    fn mcp_ops_agent_returns_all_metadata_exposable() {
        let regs = mcp_tool_registrations("ops-agent");
        assert!(!regs.is_empty());
        for reg in &regs {
            assert!(
                reg.mcp_metadata_exposable,
                "ops-agent registration '{}' should be mcp_metadata_exposable",
                reg.tool_id
            );
        }
    }

    #[test]
    fn mcp_coding_agent_returns_coding_tools() {
        let regs = mcp_tool_registrations("coding-agent");
        let ids: Vec<&str> = regs.iter().map(|r| r.tool_id).collect();
        assert!(
            regs.len() >= 4,
            "coding-agent should have at least 4 tools, got {}",
            regs.len()
        );
        for &expected in &["scan-ports", "fingerprint", "waf-detect", "search"] {
            assert!(
                ids.contains(&expected),
                "coding-agent should include '{}', got: {:?}",
                expected,
                ids
            );
        }
    }

    #[test]
    fn mcp_unknown_profile_returns_empty() {
        let regs = mcp_tool_registrations("unknown-profile");
        assert!(regs.is_empty());
    }

    #[test]
    fn rest_registrations_are_all_rest_exposable() {
        let regs = rest_tool_registrations();
        assert!(!regs.is_empty());
        for reg in &regs {
            assert!(reg.rest_exposable);
        }
    }

    #[test]
    fn grpc_registrations_are_all_grpc_exposable() {
        let regs = grpc_tool_registrations();
        assert!(!regs.is_empty());
        for reg in &regs {
            assert!(reg.grpc_exposable);
        }
    }

    #[test]
    fn agent_registrations_are_all_agent_exposable() {
        let regs = agent_tool_registrations();
        assert!(!regs.is_empty());
        for reg in &regs {
            assert!(reg.agent_exposable);
        }
    }

    #[test]
    fn every_registration_has_operation_metadata() {
        use crate::config::metadata_for_tool_id;
        for reg in all_tool_registrations() {
            assert!(
                metadata_for_tool_id(reg.operation_id).is_some(),
                "registration '{}' has no matching OperationMetadata for operation '{}'",
                reg.tool_id,
                reg.operation_id
            );
        }
    }

    #[test]
    fn registration_source_matches_feature_state() {
        for reg in all_tool_registrations() {
            match reg.source {
                ToolRegistrationSource::Base => {
                    assert!(
                        reg.feature.is_none(),
                        "Base source tool '{}' should have no feature",
                        reg.tool_id
                    );
                }
                ToolRegistrationSource::FeatureGated(f) => {
                    assert_eq!(reg.feature, Some(f));
                }
                ToolRegistrationSource::Domain(_) => {
                    // Domain tools may or may not have a feature
                }
            }
        }
    }
}
