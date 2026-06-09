use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::tool::traits::ToolCategory;
use crate::tool::ToolInfo;

use super::profile::McpProfile;

/// Controls which targets a profile is allowed to interact with.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TargetPolicy {
    /// Only targets with an explicit scope configuration.
    ExplicitScopeOnly,
    /// Only loopback and RFC1918/ULA private addresses.
    LocalhostAndPrivateCidrsOnly,
    /// Loopback, private CIDRs, or targets in an explicit scope file.
    ScopeOrLocalDevOnly,
    /// Any target, subject to the scope engine.
    AnyWithScopeEngine,
}

/// Specifies which tools are visible/callable for a profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolSelector {
    /// All tools in the registry.
    All,
    /// No tools.
    None,
    /// Only tools with these exact IDs.
    Exact(Vec<String>),
    /// All tools in these categories.
    Category(Vec<String>),
    /// All tools providing these capability names.
    Capability(Vec<String>),
}

impl ToolSelector {
    /// Returns true if the given tool info matches this selector.
    pub fn matches(&self, tool: &ToolInfo) -> bool {
        match self {
            ToolSelector::All => true,
            ToolSelector::None => false,
            ToolSelector::Exact(ids) => ids.iter().any(|id| id == &tool.id),
            ToolSelector::Category(cats) => {
                let cat_str = format!("{:?}", tool.category).to_lowercase();
                cats.iter().any(|c| c.eq_ignore_ascii_case(&cat_str))
            }
            ToolSelector::Capability(cap_names) => tool
                .capabilities
                .iter()
                .any(|cap| cap_names.iter().any(|n| n == &cap.name)),
        }
    }
}

/// Central policy engine for MCP profile enforcement.
///
/// This struct encodes what a profile can see, call, and target.
/// It is used at both discovery time (filtering `tools/list`) and
/// call time (enforcing `tools/call`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpProfilePolicy {
    pub profile: McpProfile,
    pub default_target_policy: TargetPolicy,
    pub allowed_tool_ids: ToolSelector,
    pub denied_tool_ids: ToolSelector,
    pub allowed_categories: ToolSelector,
    pub denied_categories: ToolSelector,
    pub max_concurrency: usize,
    pub max_timeout_ms: u64,
    pub max_batch_size: usize,
    pub allow_streaming: bool,
    pub allow_sessions: bool,
    pub allow_plan_endpoint: bool,
    pub require_explicit_scope: bool,
    pub allow_external_network: bool,
    pub allow_stress_testing: bool,
    pub allow_packet_features: bool,
    pub allow_broad_recon: bool,
    /// Deny specific argument keys when present in a tool call.
    pub denied_argument_keys: Vec<String>,
}

impl McpProfilePolicy {
    /// Create a policy for the given profile with safe defaults.
    pub fn for_profile(profile: McpProfile) -> Self {
        match profile {
            McpProfile::OpsAgent => Self::ops_agent(),
            McpProfile::CodingAgent => Self::coding_agent(),
        }
    }

    /// Ops-agent: broad toolkit, subject to scope/auth/rate limits.
    pub fn ops_agent() -> Self {
        Self {
            profile: McpProfile::OpsAgent,
            default_target_policy: TargetPolicy::AnyWithScopeEngine,
            allowed_tool_ids: ToolSelector::All,
            denied_tool_ids: ToolSelector::None,
            allowed_categories: ToolSelector::All,
            denied_categories: ToolSelector::None,
            max_concurrency: 50,
            max_timeout_ms: 600_000,
            max_batch_size: 100,
            allow_streaming: true,
            allow_sessions: true,
            allow_plan_endpoint: true,
            require_explicit_scope: false,
            allow_external_network: true,
            allow_stress_testing: true,
            allow_packet_features: true,
            allow_broad_recon: true,
            denied_argument_keys: Vec::new(),
        }
    }

    /// Coding-agent: deny-by-default, narrow validation tools only.
    pub fn coding_agent() -> Self {
        Self {
            profile: McpProfile::CodingAgent,
            default_target_policy: TargetPolicy::ScopeOrLocalDevOnly,
            allowed_tool_ids: ToolSelector::Exact(vec![
                "scan".to_string(),
                "scan-ports".to_string(),
                "fingerprint".to_string(),
                "endpoints".to_string(),
                "waf-detect".to_string(),
                "search".to_string(),
            ]),
            denied_tool_ids: ToolSelector::None,
            allowed_categories: ToolSelector::None,
            denied_categories: ToolSelector::Exact(vec![
                "stresstesting".to_string(),
                "loadtesting".to_string(),
            ]),
            max_concurrency: 5,
            max_timeout_ms: 60_000,
            max_batch_size: 10,
            allow_streaming: true,
            allow_sessions: false,
            allow_plan_endpoint: false,
            require_explicit_scope: false,
            allow_external_network: false,
            allow_stress_testing: false,
            allow_packet_features: false,
            allow_broad_recon: false,
            denied_argument_keys: vec![
                "stealth".to_string(),
                "proxy_rotation".to_string(),
                "spoof_source".to_string(),
                "raw_packet".to_string(),
            ],
        }
    }

    /// Filter a list of tools to those visible under this policy.
    pub fn filter_tools(&self, tools: Vec<ToolInfo>) -> Vec<ToolInfo> {
        tools
            .into_iter()
            .filter(|tool| self.is_tool_visible(tool))
            .collect()
    }

    /// Check if a specific tool is visible under this policy.
    pub fn is_tool_visible(&self, tool: &ToolInfo) -> bool {
        if !self.allowed_tool_ids.matches(tool) {
            return false;
        }
        if self.denied_tool_ids.matches(tool) {
            return false;
        }
        let cat_str = format!("{:?}", tool.category).to_lowercase();
        match &self.denied_categories {
            ToolSelector::Exact(cats) if cats.iter().any(|c| c.eq_ignore_ascii_case(&cat_str)) => {
                return false;
            }
            ToolSelector::All => return false,
            _ => {}
        }
        match &self.allowed_categories {
            ToolSelector::All => {}
            ToolSelector::None => {}
            ToolSelector::Exact(cats) => {
                if !cats.iter().any(|c| c.eq_ignore_ascii_case(&cat_str)) {
                    return false;
                }
            }
            ToolSelector::Category(cats) => {
                if !cats.iter().any(|c| c.eq_ignore_ascii_case(&cat_str)) {
                    return false;
                }
            }
            ToolSelector::Capability(_) => {}
        }
        true
    }

    /// Validate that a tool call is allowed by this policy.
    ///
    /// Returns Ok(()) if allowed, Err(McpError) if denied.
    pub fn validate_tool_call(
        &self,
        tool_id: &str,
        _capability: Option<&str>,
        arguments: &serde_json::Value,
    ) -> Result<(), PolicyViolation> {
        // Build a synthetic ToolInfo to check selectors
        let category = infer_tool_category(tool_id);
        let synthetic = ToolInfo {
            id: tool_id.to_string(),
            name: tool_id.to_string(),
            category,
            description: String::new(),
            capabilities: Vec::new(),
            protocols: Vec::new(),
        };

        if !self.allowed_tool_ids.matches(&synthetic) {
            return Err(PolicyViolation::ToolDenied {
                tool_id: tool_id.to_string(),
            });
        }
        if self.denied_tool_ids.matches(&synthetic) {
            return Err(PolicyViolation::ToolDenied {
                tool_id: tool_id.to_string(),
            });
        }

        // Check tool risk against profile budget
        let risk = classify_tool_risk(tool_id);
        match risk {
            crate::config::OperationRisk::StressTest if !self.allow_stress_testing => {
                return Err(PolicyViolation::ToolDenied {
                    tool_id: tool_id.to_string(),
                });
            }
            crate::config::OperationRisk::RawPacket if !self.allow_packet_features => {
                return Err(PolicyViolation::ToolDenied {
                    tool_id: tool_id.to_string(),
                });
            }
            crate::config::OperationRisk::ExploitAdjacent => {
                return Err(PolicyViolation::ToolDenied {
                    tool_id: tool_id.to_string(),
                });
            }
            crate::config::OperationRisk::RemoteExecution => {
                return Err(PolicyViolation::ToolDenied {
                    tool_id: tool_id.to_string(),
                });
            }
            _ => {}
        }

        // Check denied argument keys
        if let serde_json::Value::Object(map) = arguments {
            for key in map.keys() {
                if self.denied_argument_keys.iter().any(|dk| dk == key) {
                    return Err(PolicyViolation::ArgumentDenied {
                        key: key.clone(),
                        tool_id: tool_id.to_string(),
                    });
                }
            }
        }

        // Check concurrency budget
        if let Some(concurrency) = arguments.get("concurrency").and_then(|v| v.as_u64()) {
            if concurrency as usize > self.max_concurrency {
                return Err(PolicyViolation::ConcurrencyExceeded {
                    requested: concurrency as usize,
                    max: self.max_concurrency,
                });
            }
        }

        // Check timeout budget
        if let Some(timeout) = arguments.get("timeout_ms").and_then(|v| v.as_u64()) {
            if timeout > self.max_timeout_ms {
                return Err(PolicyViolation::TimeoutExceeded {
                    requested_ms: timeout,
                    max_ms: self.max_timeout_ms,
                });
            }
        }

        Ok(())
    }

    /// Validate a target against this profile's target policy.
    ///
    /// Returns Ok(allowed) or an error describing the violation.
    pub fn validate_target(&self, target: &str) -> Result<(), PolicyViolation> {
        match self.default_target_policy {
            TargetPolicy::AnyWithScopeEngine => Ok(()),
            TargetPolicy::ExplicitScopeOnly => {
                if target.is_empty() {
                    return Err(PolicyViolation::TargetDenied {
                        target: target.to_string(),
                        reason: "Explicit scope required".to_string(),
                    });
                }
                Ok(())
            }
            TargetPolicy::LocalhostAndPrivateCidrsOnly => {
                if is_loopback_or_private(target) {
                    Ok(())
                } else {
                    Err(PolicyViolation::TargetDenied {
                        target: target.to_string(),
                        reason: "Only loopback and private network targets are allowed".to_string(),
                    })
                }
            }
            TargetPolicy::ScopeOrLocalDevOnly => {
                if is_loopback_or_private(target) || is_metadata_endpoint(target) {
                    if is_metadata_endpoint(target) {
                        return Err(PolicyViolation::TargetDenied {
                            target: target.to_string(),
                            reason: "Cloud metadata endpoints are denied".to_string(),
                        });
                    }
                    Ok(())
                } else {
                    Err(PolicyViolation::TargetDenied {
                        target: target.to_string(),
                        reason: "Public internet targets denied without explicit scope".to_string(),
                    })
                }
            }
        }
    }

    /// Build profile metadata for the `initialize` response.
    pub fn to_initialize_metadata(&self) -> serde_json::Value {
        let mut meta = serde_json::json!({
            "profile": self.profile.as_str(),
            "safety": {
                "default_external_network": self.allow_external_network,
                "stress_testing_available": self.allow_stress_testing,
                "broad_recon_available": self.allow_broad_recon,
                "max_concurrency": self.max_concurrency,
                "max_timeout_ms": self.max_timeout_ms,
            }
        });
        if let serde_json::Value::Object(ref mut map) = meta {
            map.insert(
                "require_explicit_scope".to_string(),
                serde_json::json!(self.require_explicit_scope),
            );
        }
        meta
    }
}

/// Reasons a policy check can fail.
#[derive(Debug, Clone)]
pub enum PolicyViolation {
    ToolDenied { tool_id: String },
    ArgumentDenied { key: String, tool_id: String },
    ConcurrencyExceeded { requested: usize, max: usize },
    TimeoutExceeded { requested_ms: u64, max_ms: u64 },
    TargetDenied { target: String, reason: String },
}

impl std::fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyViolation::ToolDenied { tool_id } => {
                write!(f, "Tool '{}' is not available for this profile", tool_id)
            }
            PolicyViolation::ArgumentDenied { key, tool_id } => {
                write!(
                    f,
                    "Argument '{}' is not allowed when calling tool '{}'",
                    key, tool_id
                )
            }
            PolicyViolation::ConcurrencyExceeded { requested, max } => {
                write!(
                    f,
                    "Requested concurrency {} exceeds profile maximum {}",
                    requested, max
                )
            }
            PolicyViolation::TimeoutExceeded {
                requested_ms,
                max_ms,
            } => {
                write!(
                    f,
                    "Requested timeout {}ms exceeds profile maximum {}ms",
                    requested_ms, max_ms
                )
            }
            PolicyViolation::TargetDenied { target, reason } => {
                write!(f, "Target '{}' denied: {}", target, reason)
            }
        }
    }
}

impl PolicyViolation {
    /// Convert to an MCP error code for JSON-RPC responses.
    pub fn to_mcp_error_code(&self) -> i32 {
        match self {
            PolicyViolation::ToolDenied { .. } => -32020,
            PolicyViolation::ArgumentDenied { .. } => -32021,
            PolicyViolation::ConcurrencyExceeded { .. } => -32022,
            PolicyViolation::TimeoutExceeded { .. } => -32023,
            PolicyViolation::TargetDenied { .. } => -32024,
        }
    }

    /// Convert to an MCP error response with an embedded [`PolicyDecision`].
    pub fn to_mcp_error_with_decision(
        &self,
        profile_policy: &McpProfilePolicy,
        tool_id: &str,
        arguments: &serde_json::Value,
        execution_policy: &crate::config::ExecutionPolicy,
        scope: Option<&crate::config::Scope>,
    ) -> super::types::McpError {
        let denial = denial_from_violation(
            self,
            profile_policy,
            tool_id,
            arguments,
            execution_policy,
            scope,
        );
        super::types::McpError {
            code: denial.code,
            message: denial.violation,
            data: Some(serde_json::to_value(&denial.policy_decision).unwrap_or_default()),
        }
    }
}

/// A policy denial that embeds both the MCP-specific violation and a shared
/// [`PolicyDecision`] for structured downstream consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPolicyDenial {
    /// Human-readable description of the violation.
    pub violation: String,
    /// MCP JSON-RPC error code.
    pub code: i32,
    /// Shared policy decision providing full evaluation context.
    pub policy_decision: crate::config::PolicyDecision,
}

/// Infer a [`ToolCategory`] from a tool ID string for selector matching.
fn infer_tool_category(tool_id: &str) -> ToolCategory {
    match tool_id {
        "stress" | "waf-stress" | "syn-flood" | "udp-flood" | "icmp-flood" => ToolCategory::Stress,
        "proxy" | "tor" => ToolCategory::Recon,
        "load" | "loadtest" | "http-bench" => ToolCategory::LoadTest,
        "fuzz" | "fuzzer" | "api-fuzz" => ToolCategory::Fuzzing,
        "recon" | "recon-all" | "subdomain" => ToolCategory::Recon,
        "waf-detect" | "waf-bypass" => ToolCategory::Waf,
        "scan" | "scan-ports" | "fingerprint" | "scan-endpoints" => ToolCategory::Scanning,
        "pipeline" | "search" => ToolCategory::Pipeline,
        "oast" => ToolCategory::Scanning,
        _ => ToolCategory::Scanning,
    }
}

/// Infer the [`OperationRisk`] for a tool based on its ID.
///
/// This is used when building [`PolicyDecision`] for MCP calls where
/// real tool metadata may not be available.
pub fn classify_tool_risk(tool_id: &str) -> crate::config::OperationRisk {
    use crate::config::OperationRisk;
    match tool_id {
        "stress" | "waf-stress" | "syn-flood" | "udp-flood" | "icmp-flood" => {
            OperationRisk::StressTest
        }
        "packet" | "raw-packet" | "packet-capture" | "packet-inspect" => OperationRisk::RawPacket,
        "proxy" | "tor" => OperationRisk::ExploitAdjacent,
        "remote" | "exec" | "ssh" => OperationRisk::RemoteExecution,
        "load" | "loadtest" | "http-bench" => OperationRisk::LoadTest,
        "fuzz" | "fuzzer" | "api-fuzz" => OperationRisk::Intrusive,
        "credential" | "brute" | "auth-test" => OperationRisk::CredentialTesting,
        _ => OperationRisk::SafeActive,
    }
}

/// Build a [`PolicyDecision`] for an MCP tool call by evaluating the tool
/// against both the MCP profile policy and the shared execution policy.
pub fn policy_decision_for_mcp_call(
    profile_policy: &McpProfilePolicy,
    tool_id: &str,
    arguments: &serde_json::Value,
    execution_policy: &crate::config::ExecutionPolicy,
    scope: Option<&crate::config::Scope>,
) -> crate::config::PolicyDecision {
    use crate::config::{IntendedUse, OperationDescriptor, OperationMode};

    let risk = classify_tool_risk(tool_id);
    let target = arguments
        .get("target")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let intended_uses = if profile_policy.profile.is_coding_agent() {
        vec![IntendedUse::CodingAgentVerification]
    } else {
        vec![IntendedUse::WebAssessment]
    };

    let descriptor = OperationDescriptor {
        operation: tool_id.to_string(),
        mode: OperationMode::StandardAssessment,
        risk,
        intended_uses,
        target: target.clone(),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: profile_policy.require_explicit_scope,
    };

    let mut decision =
        crate::config::evaluate_operation_policy(&descriptor, execution_policy, scope);

    if let Err(violation) = profile_policy.validate_tool_call(tool_id, None, arguments) {
        decision.allowed = false;
        decision
            .denied_reasons
            .push(violation.to_string());
    }

    if let Some(ref tgt) = target {
        if let Err(violation) = profile_policy.validate_target(tgt) {
            decision.allowed = false;
            decision
                .denied_reasons
                .push(violation.to_string());
        }
    }

    decision
}

/// Convert a [`PolicyViolation`] into an [`McpPolicyDenial`] that includes
/// a shared [`PolicyDecision`].
pub fn denial_from_violation(
    violation: &PolicyViolation,
    profile_policy: &McpProfilePolicy,
    tool_id: &str,
    arguments: &serde_json::Value,
    execution_policy: &crate::config::ExecutionPolicy,
    scope: Option<&crate::config::Scope>,
) -> McpPolicyDenial {
    let policy_decision =
        policy_decision_for_mcp_call(profile_policy, tool_id, arguments, execution_policy, scope);
    McpPolicyDenial {
        violation: violation.to_string(),
        code: violation.to_mcp_error_code(),
        policy_decision,
    }
}

/// Extract the hostname from a target string (strips scheme, port, path, userinfo).
///
/// Handles:
/// - `http://user:pass@host.com:8080/path` → `host.com`
/// - `https://example.com` → `example.com`
/// - `http://127.0.0.1:3000` → `127.0.0.1`
/// - `http://[::1]:8080` → `::1`
/// - `[::1]:8080` → `::1`
/// - `::1` → `::1`
/// - `localhost:8080` → `localhost`
/// - `example.com` → `example.com`
fn extract_hostname(target: &str) -> &str {
    let s = target.trim();

    // Strip scheme if present
    let s = if let Some(rest) = s.strip_prefix("http://") {
        rest
    } else if let Some(rest) = s.strip_prefix("https://") {
        rest
    } else {
        s
    };

    // Strip userinfo (user:pass@) if present
    let s = if let Some(pos) = s.find('@') {
        &s[pos + 1..]
    } else {
        s
    };

    // Strip path
    let s = s.split('/').next().unwrap_or(s);

    // Handle bracketed IPv6: [::1]:8080 or [::1]
    if let Some(inner) = s.strip_prefix('[') {
        if let Some(bracket_end) = inner.find(']') {
            return &inner[..bracket_end];
        }
        // Malformed bracket — return as-is after stripping the bracket
        return s;
    }

    // For non-bracketed hosts, split on ':' to remove port
    // But distinguish bare IPv6 (multiple colons) from host:port (one colon + digits)
    if s.contains(':') {
        // If there's exactly one ':' and the part after it parses as u16, it's host:port
        if let Some(pos) = s.rfind(':') {
            let port_part = &s[pos + 1..];
            if port_part.is_empty() || port_part.parse::<u16>().is_ok() {
                return &s[..pos];
            }
        }
        // Multiple colons or non-numeric port part — treat as bare IPv6
        return s;
    }

    s
}

/// Check if a target string points to a loopback or private network address.
fn is_loopback_or_private(target: &str) -> bool {
    let host = extract_hostname(target);

    // Handle IPv6 brackets
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);

    if let Ok(ip) = IpAddr::from_str(host) {
        return is_loopback_ip(ip) || is_private_ip(ip);
    }

    // Check well-known hostnames
    let lower = host.to_lowercase();
    if lower == "localhost" || lower == "local" {
        return true;
    }

    false
}

/// Check if target is a cloud metadata endpoint.
fn is_metadata_endpoint(target: &str) -> bool {
    let host = extract_hostname(target);
    let lower = host.to_lowercase();

    // AWS, GCP, Azure metadata endpoints
    lower == "169.254.169.254"
        || lower == "metadata.google.internal"
        || lower == "metadata.azure.internal"
        || lower == "169.254.169.254.nip.io"
        || lower.ends_with(".metadata.google.internal")
        || lower.ends_with(".metadata.azure.internal")
}

/// Check if an IPv4 address is in the CGNAT range (100.64.0.0/10, RFC 6598).
fn is_cgnat(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (octets[1] & 0xC0) == 64
}

fn is_loopback_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || is_cgnat(v4)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unicast_link_local()
                || v6.segments()[0] == 0xfe80 // link-local
                || v6.segments()[0] & 0xfe00 == 0xfc00 // ULA
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::traits::ToolCategory;

    fn make_tool(id: &str, category: ToolCategory) -> ToolInfo {
        ToolInfo {
            id: id.to_string(),
            name: id.to_string(),
            category,
            description: format!("Test tool {}", id),
            capabilities: Vec::new(),
            protocols: vec!["http".to_string()],
        }
    }

    #[test]
    fn test_ops_agent_policy_allows_all_tools() {
        let policy = McpProfilePolicy::ops_agent();
        let tools = vec![
            make_tool("scan", ToolCategory::Scanning),
            make_tool("fuzz", ToolCategory::Fuzzing),
            make_tool("recon", ToolCategory::Recon),
            make_tool("load", ToolCategory::LoadTest),
            make_tool("stress", ToolCategory::Stress),
        ];
        let filtered = policy.filter_tools(tools);
        assert_eq!(filtered.len(), 5);
    }

    #[test]
    fn test_coding_agent_policy_filters_tools() {
        let policy = McpProfilePolicy::coding_agent();
        let tools = vec![
            make_tool("scan", ToolCategory::Scanning),
            make_tool("fuzz", ToolCategory::Fuzzing),
            make_tool("recon", ToolCategory::Recon),
            make_tool("load", ToolCategory::LoadTest),
            make_tool("scan-ports", ToolCategory::Scanning),
            make_tool("fingerprint", ToolCategory::Scanning),
            make_tool("waf-detect", ToolCategory::Waf),
            make_tool("waf-bypass", ToolCategory::Waf),
            make_tool("search", ToolCategory::Pipeline),
            make_tool("endpoints", ToolCategory::Scanning),
        ];
        let filtered = policy.filter_tools(tools);
        let ids: Vec<&str> = filtered.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"scan"));
        assert!(ids.contains(&"scan-ports"));
        assert!(ids.contains(&"fingerprint"));
        assert!(ids.contains(&"waf-detect"));
        assert!(ids.contains(&"search"));
        assert!(ids.contains(&"endpoints"));
        assert!(!ids.contains(&"fuzz"));
        assert!(!ids.contains(&"recon"));
        assert!(!ids.contains(&"load"));
        assert!(!ids.contains(&"waf-bypass"));
    }

    #[test]
    fn test_coding_agent_tool_call_allowed() {
        let policy = McpProfilePolicy::coding_agent();
        assert!(policy
            .validate_tool_call("scan", None, &serde_json::json!({}))
            .is_ok());
    }

    #[test]
    fn test_coding_agent_tool_call_denied() {
        let policy = McpProfilePolicy::coding_agent();
        let result = policy.validate_tool_call("fuzz", None, &serde_json::json!({}));
        assert!(result.is_err());
        match result.unwrap_err() {
            PolicyViolation::ToolDenied { tool_id } => assert_eq!(tool_id, "fuzz"),
            _ => panic!("Expected ToolDenied"),
        }
    }

    #[test]
    fn test_coding_agent_denied_argument() {
        let policy = McpProfilePolicy::coding_agent();
        let result = policy.validate_tool_call("scan", None, &serde_json::json!({"stealth": true}));
        assert!(result.is_err());
        match result.unwrap_err() {
            PolicyViolation::ArgumentDenied { key, .. } => assert_eq!(key, "stealth"),
            _ => panic!("Expected ArgumentDenied"),
        }
    }

    #[test]
    fn test_coding_agent_concurrency_clamp() {
        let policy = McpProfilePolicy::coding_agent();
        let result =
            policy.validate_tool_call("scan", None, &serde_json::json!({"concurrency": 100}));
        assert!(result.is_err());
        match result.unwrap_err() {
            PolicyViolation::ConcurrencyExceeded { requested, max } => {
                assert_eq!(requested, 100);
                assert_eq!(max, 5);
            }
            _ => panic!("Expected ConcurrencyExceeded"),
        }
    }

    #[test]
    fn test_coding_agent_timeout_clamp() {
        let policy = McpProfilePolicy::coding_agent();
        let result =
            policy.validate_tool_call("scan", None, &serde_json::json!({"timeout_ms": 120000}));
        assert!(result.is_err());
        match result.unwrap_err() {
            PolicyViolation::TimeoutExceeded {
                requested_ms,
                max_ms,
            } => {
                assert_eq!(requested_ms, 120000);
                assert_eq!(max_ms, 60000);
            }
            _ => panic!("Expected TimeoutExceeded"),
        }
    }

    #[test]
    fn test_coding_agent_target_localhost_allowed() {
        let policy = McpProfilePolicy::coding_agent();
        assert!(policy.validate_target("http://localhost:8080").is_ok());
        assert!(policy.validate_target("http://127.0.0.1:3000").is_ok());
        assert!(policy.validate_target("http://[::1]:8080").is_ok());
    }

    #[test]
    fn test_coding_agent_target_private_allowed() {
        let policy = McpProfilePolicy::coding_agent();
        assert!(policy.validate_target("http://10.0.0.5:8080").is_ok());
        assert!(policy.validate_target("http://192.168.1.10:3000").is_ok());
        assert!(policy.validate_target("http://172.16.0.5:8080").is_ok());
    }

    #[test]
    fn test_coding_agent_target_public_denied() {
        let policy = McpProfilePolicy::coding_agent();
        let result = policy.validate_target("https://example.com");
        assert!(result.is_err());
        match result.unwrap_err() {
            PolicyViolation::TargetDenied { target, .. } => {
                assert_eq!(target, "https://example.com")
            }
            _ => panic!("Expected TargetDenied"),
        }
    }

    #[test]
    fn test_coding_agent_target_metadata_denied() {
        let policy = McpProfilePolicy::coding_agent();
        assert!(policy
            .validate_target("http://169.254.169.254/latest/meta-data")
            .is_err());
        assert!(policy
            .validate_target("http://metadata.google.internal")
            .is_err());
    }

    #[test]
    fn test_ops_agent_target_any_allowed() {
        let policy = McpProfilePolicy::ops_agent();
        assert!(policy.validate_target("https://example.com").is_ok());
        assert!(policy.validate_target("http://localhost:8080").is_ok());
        assert!(policy.validate_target("http://169.254.169.254").is_ok());
    }

    #[test]
    fn test_tool_selector_exact() {
        let sel = ToolSelector::Exact(vec!["scan".to_string(), "fuzz".to_string()]);
        let scan = make_tool("scan", ToolCategory::Scanning);
        let fuzz = make_tool("fuzz", ToolCategory::Fuzzing);
        let recon = make_tool("recon", ToolCategory::Recon);
        assert!(sel.matches(&scan));
        assert!(sel.matches(&fuzz));
        assert!(!sel.matches(&recon));
    }

    #[test]
    fn test_tool_selector_category() {
        let sel = ToolSelector::Category(vec!["scanning".to_string()]);
        let scan = make_tool("scan", ToolCategory::Scanning);
        let fuzz = make_tool("fuzz", ToolCategory::Fuzzing);
        assert!(sel.matches(&scan));
        assert!(!sel.matches(&fuzz));
    }

    #[test]
    fn test_initialize_metadata() {
        let policy = McpProfilePolicy::coding_agent();
        let meta = policy.to_initialize_metadata();
        assert_eq!(meta["profile"], "coding-agent");
        assert_eq!(meta["safety"]["max_concurrency"], 5);
        assert_eq!(meta["safety"]["max_timeout_ms"], 60000);
        assert_eq!(meta["require_explicit_scope"], false);
    }

    #[test]
    fn test_extract_hostname() {
        assert_eq!(extract_hostname("http://localhost:8080/path"), "localhost");
        assert_eq!(extract_hostname("https://example.com"), "example.com");
        assert_eq!(extract_hostname("http://127.0.0.1:3000"), "127.0.0.1");
        assert_eq!(extract_hostname("[::1]:8080"), "::1");
        assert_eq!(extract_hostname("example.com"), "example.com");
    }

    #[test]
    fn test_policy_violation_display() {
        let v = PolicyViolation::ToolDenied {
            tool_id: "fuzz".to_string(),
        };
        assert!(v.to_string().contains("fuzz"));

        let v = PolicyViolation::TargetDenied {
            target: "https://evil.com".to_string(),
            reason: "denied".to_string(),
        };
        assert!(v.to_string().contains("evil.com"));
    }

    #[test]
    fn test_policy_for_profile_dispatches() {
        let ops = McpProfilePolicy::for_profile(McpProfile::OpsAgent);
        assert_eq!(ops.profile, McpProfile::OpsAgent);
        assert!(ops.allow_external_network);

        let coding = McpProfilePolicy::for_profile(McpProfile::CodingAgent);
        assert_eq!(coding.profile, McpProfile::CodingAgent);
        assert!(!coding.allow_external_network);
    }

    // Phase 12: Additional policy tests

    #[test]
    fn test_coding_agent_timeout_within_limit_allowed() {
        let policy = McpProfilePolicy::coding_agent();
        let result =
            policy.validate_tool_call("scan", None, &serde_json::json!({"timeout_ms": 30000}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_coding_agent_concurrency_within_limit_allowed() {
        let policy = McpProfilePolicy::coding_agent();
        let result =
            policy.validate_tool_call("scan", None, &serde_json::json!({"concurrency": 3}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_policy_violation_error_codes() {
        let v = PolicyViolation::ToolDenied {
            tool_id: "fuzz".to_string(),
        };
        assert_eq!(v.to_mcp_error_code(), -32020);

        let v = PolicyViolation::ArgumentDenied {
            key: "stealth".to_string(),
            tool_id: "scan".to_string(),
        };
        assert_eq!(v.to_mcp_error_code(), -32021);

        let v = PolicyViolation::ConcurrencyExceeded {
            requested: 100,
            max: 5,
        };
        assert_eq!(v.to_mcp_error_code(), -32022);

        let v = PolicyViolation::TimeoutExceeded {
            requested_ms: 120000,
            max_ms: 60000,
        };
        assert_eq!(v.to_mcp_error_code(), -32023);

        let v = PolicyViolation::TargetDenied {
            target: "https://evil.com".to_string(),
            reason: "denied".to_string(),
        };
        assert_eq!(v.to_mcp_error_code(), -32024);
    }

    #[test]
    fn test_policy_violation_display_all_variants() {
        let v = PolicyViolation::ToolDenied {
            tool_id: "fuzz".to_string(),
        };
        assert!(v.to_string().contains("fuzz"));

        let v = PolicyViolation::ArgumentDenied {
            key: "stealth".to_string(),
            tool_id: "scan".to_string(),
        };
        assert!(v.to_string().contains("stealth"));
        assert!(v.to_string().contains("scan"));

        let v = PolicyViolation::ConcurrencyExceeded {
            requested: 100,
            max: 5,
        };
        assert!(v.to_string().contains("100"));
        assert!(v.to_string().contains("5"));

        let v = PolicyViolation::TimeoutExceeded {
            requested_ms: 120000,
            max_ms: 60000,
        };
        assert!(v.to_string().contains("120000"));
        assert!(v.to_string().contains("60000"));

        let v = PolicyViolation::TargetDenied {
            target: "https://evil.com".to_string(),
            reason: "public denied".to_string(),
        };
        assert!(v.to_string().contains("evil.com"));
        assert!(v.to_string().contains("public denied"));
    }

    #[test]
    fn test_coding_agent_target_ipv6_localhost_allowed() {
        let policy = McpProfilePolicy::coding_agent();
        assert!(policy.validate_target("http://[::1]:8080").is_ok());
    }

    #[test]
    fn test_coding_agent_target_metadata_azure_denied() {
        let policy = McpProfilePolicy::coding_agent();
        assert!(policy
            .validate_target("http://169.254.169.254/latest/meta-data")
            .is_err());
        assert!(policy
            .validate_target("http://metadata.azure.internal")
            .is_err());
    }

    #[test]
    fn test_ops_agent_allows_all_argument_keys() {
        let policy = McpProfilePolicy::ops_agent();
        let result = policy.validate_tool_call(
            "scan",
            None,
            &serde_json::json!({"stealth": true, "proxy_rotation": true, "spoof_source": true}),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_selector_all_matches_everything() {
        let sel = ToolSelector::All;
        let scan = make_tool("scan", ToolCategory::Scanning);
        let fuzz = make_tool("fuzz", ToolCategory::Fuzzing);
        assert!(sel.matches(&scan));
        assert!(sel.matches(&fuzz));
    }

    #[test]
    fn test_tool_selector_none_matches_nothing() {
        let sel = ToolSelector::None;
        let scan = make_tool("scan", ToolCategory::Scanning);
        assert!(!sel.matches(&scan));
    }

    #[test]
    fn test_tool_selector_capability() {
        let sel = ToolSelector::Capability(vec!["xss-detect".to_string()]);
        let mut tool = make_tool("scan", ToolCategory::Scanning);
        tool.capabilities.push(crate::tool::ToolCapability {
            name: "xss-detect".to_string(),
            description: "XSS detection".to_string(),
            attack_surface: vec![],
            severity_potential: vec![],
        });
        assert!(sel.matches(&tool));

        let other = make_tool("recon", ToolCategory::Recon);
        assert!(!sel.matches(&other));
    }

    #[test]
    fn test_is_loopback_or_private_various_targets() {
        assert!(is_loopback_or_private("localhost"));
        assert!(is_loopback_or_private("127.0.0.1"));
        assert!(is_loopback_or_private("10.0.0.1"));
        assert!(is_loopback_or_private("192.168.1.1"));
        assert!(is_loopback_or_private("172.16.0.1"));
        assert!(!is_loopback_or_private("8.8.8.8"));
        assert!(!is_loopback_or_private("example.com"));
    }

    #[test]
    fn test_is_metadata_endpoint_various() {
        assert!(is_metadata_endpoint("169.254.169.254"));
        assert!(is_metadata_endpoint("metadata.google.internal"));
        assert!(is_metadata_endpoint("metadata.azure.internal"));
        assert!(!is_metadata_endpoint("localhost"));
        assert!(!is_metadata_endpoint("example.com"));
    }

    #[test]
    fn test_extract_hostname_various() {
        assert_eq!(
            extract_hostname("http://user:pass@host.com:8080/path"),
            "host.com"
        );
        assert_eq!(extract_hostname("https://192.168.1.1/api"), "192.168.1.1");
        assert_eq!(extract_hostname("http://[::1]:8080"), "::1");
        assert_eq!(extract_hostname("just-hostname"), "just-hostname");
        assert_eq!(extract_hostname("::1"), "::1");
        assert_eq!(extract_hostname("localhost:8080"), "localhost");
    }

    #[test]
    fn test_infer_tool_category_stress() {
        assert_eq!(infer_tool_category("stress"), ToolCategory::Stress);
        assert_eq!(infer_tool_category("waf-stress"), ToolCategory::Stress);
        assert_eq!(infer_tool_category("syn-flood"), ToolCategory::Stress);
    }

    #[test]
    fn test_infer_tool_category_scanning() {
        assert_eq!(infer_tool_category("scan"), ToolCategory::Scanning);
        assert_eq!(infer_tool_category("scan-ports"), ToolCategory::Scanning);
        assert_eq!(infer_tool_category("fingerprint"), ToolCategory::Scanning);
        assert_eq!(infer_tool_category("scan-endpoints"), ToolCategory::Scanning);
    }

    #[test]
    fn test_infer_tool_category_other_variants() {
        assert_eq!(infer_tool_category("fuzz"), ToolCategory::Fuzzing);
        assert_eq!(infer_tool_category("recon"), ToolCategory::Recon);
        assert_eq!(infer_tool_category("load"), ToolCategory::LoadTest);
        assert_eq!(infer_tool_category("waf-detect"), ToolCategory::Waf);
        assert_eq!(infer_tool_category("search"), ToolCategory::Pipeline);
        assert_eq!(infer_tool_category("proxy"), ToolCategory::Recon);
        assert_eq!(infer_tool_category("oast"), ToolCategory::Scanning);
    }

    #[test]
    fn test_infer_tool_category_unknown_defaults_to_scanning() {
        assert_eq!(infer_tool_category("unknown-tool"), ToolCategory::Scanning);
    }

    #[test]
    fn test_coding_agent_risk_denies_stress() {
        let policy = McpProfilePolicy::coding_agent();
        let result = policy.validate_tool_call("stress", None, &serde_json::json!({}));
        assert!(result.is_err());
        match result.unwrap_err() {
            PolicyViolation::ToolDenied { tool_id } => assert_eq!(tool_id, "stress"),
            _ => panic!("Expected ToolDenied for stress tool"),
        }
    }

    #[test]
    fn test_coding_agent_risk_denies_waf_stress() {
        let policy = McpProfilePolicy::coding_agent();
        let result = policy.validate_tool_call("waf-stress", None, &serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_coding_agent_risk_denies_syn_flood() {
        let policy = McpProfilePolicy::coding_agent();
        let result = policy.validate_tool_call("syn-flood", None, &serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_coding_agent_risk_allows_scan() {
        let policy = McpProfilePolicy::coding_agent();
        assert!(policy
            .validate_tool_call("scan", None, &serde_json::json!({}))
            .is_ok());
    }

    #[test]
    fn test_coding_agent_risk_allows_waf_detect() {
        let policy = McpProfilePolicy::coding_agent();
        assert!(policy
            .validate_tool_call("waf-detect", None, &serde_json::json!({}))
            .is_ok());
    }

    #[test]
    fn test_ops_agent_risk_allows_stress() {
        let policy = McpProfilePolicy::ops_agent();
        assert!(policy
            .validate_tool_call("stress", None, &serde_json::json!({}))
            .is_ok());
    }

    #[test]
    fn test_ops_agent_risk_allows_load() {
        let policy = McpProfilePolicy::ops_agent();
        assert!(policy
            .validate_tool_call("load", None, &serde_json::json!({}))
            .is_ok());
    }

    #[test]
    fn test_ops_agent_risk_allows_fuzz() {
        let policy = McpProfilePolicy::ops_agent();
        assert!(policy
            .validate_tool_call("fuzz", None, &serde_json::json!({}))
            .is_ok());
    }

    #[test]
    fn test_coding_agent_risk_denies_remote_exec() {
        let policy = McpProfilePolicy::coding_agent();
        let result = policy.validate_tool_call("ssh", None, &serde_json::json!({}));
        assert!(result.is_err());
    }
}
