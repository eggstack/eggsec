//! Conservative conversion from the protocol-neutral [`ScopeSpec`] DTO into the
//! authoritative engine `Scope`.
//!
//! Only `Scope` evaluated through `EnforcementContext` (or its
//! [`is_target_allowed`](Scope::is_target_allowed) policy implementation) may
//! answer "is execution authorized?". A `ScopeSpec` (alias
//! `eggsec_tool_core::ToolScopeSpec`) is caller intent carried
//! on a `ToolRequest`; it is never evaluated
//! directly.
//!
//! Required flow for programmatic surfaces (REST/MCP/gRPC/tool/Python):
//!
//! ```text
//! transport ScopeSpec -> parse/validate -> engine Scope/LoadedScope
//!     -> EnforcementContext -> approval -> dispatch
//! ```
//!
//! Effective authorization is the **intersection** of the configured engine
//! scope and any attached request specification: both must allow, either may
//! deny. A permissive declaration (`*`) therefore cannot override a
//! restrictive engine scope, and an empty declaration denies.

use std::net::IpAddr;
use std::str::FromStr;

use eggsec_tool_core::ScopeSpec;
use ipnetwork::IpNetwork;

use crate::config::scope::{HostResolver, Scope, ScopeError, ScopeRule, TargetScope};

/// Error from converting a `ScopeSpec` into an engine `Scope`.
///
/// Conversion fails closed: any unsupported or ambiguous declaration is an
/// error, and strict surfaces must deny when conversion fails rather than
/// guessing equivalence.
#[derive(Debug, thiserror::Error)]
pub enum ScopeSpecError {
    /// An `allowed_ips` entry is neither an IP literal nor a CIDR.
    #[error("invalid allowed_ips entry '{0}': {1}")]
    InvalidAllowedIp(String, String),
    /// A pattern containing '/' is not a parseable CIDR.
    #[error("invalid CIDR pattern '{0}': {1}")]
    InvalidCidrPattern(String, String),
    /// A declaration is empty where a value is required.
    #[error("invalid scope declaration: {0}")]
    InvalidDeclaration(String),
}

impl From<ScopeSpecError> for ScopeError {
    fn from(e: ScopeSpecError) -> Self {
        ScopeError::Validation(e.to_string())
    }
}

/// Convert a `ScopeSpec` into an engine `Scope`.
///
/// Mapping rules (conservative, fail-closed):
///
/// - Every `allowed_patterns` entry becomes an allowed `ScopeRule`; every
///   `excluded_patterns` entry becomes an excluded rule. Exclusions are never
///   dropped: an unparsable exclusion is an error, not a skip.
/// - Entries containing `/` must parse as `IpNetwork`; otherwise error.
/// - IP literals map to host-covering CIDR rules (`/32`/`/128`) so they match
///   through parsed IP/CIDR evaluation rather than string comparison.
/// - Bare hostnames with `allow_subdomains == true` expand to both the exact
///   host and a `*.host` rule, matching the engine's explicit `*.` suffix
///   semantics. With `false`, only the exact host is allowed. Entries that
///   already start with `*.` or equal `*` are kept verbatim.
/// - `*` maps to the engine wildcard rule verbatim. Note this is an explicit
///   allowance: combined with intersection evaluation it still cannot override
///   a restrictive engine scope, and standalone it follows normal engine
///   explicit-rule semantics. Callers must not attach `*` on behalf of
///   untrusted input.
/// - `allowed_ips` entries must each parse as `IpAddr` or `IpNetwork`;
///   anything else errors. IP literals become `/32`/`/128` CIDR rules.
/// - The DTO carries no port/rate-limit fields; converted scopes leave
///   `allowed_ports`/`excluded_ports`/rate limits at engine defaults so those
///   remain controlled by engine policy, never inferred from the DTO.
/// - An empty declaration (no allowed patterns and no allowed IPs) converts to
///   a deny-all scope (`require_explicit_scope = true` with no allowances) so
///   `Some(empty)` denies rather than falling back to engine defaults.
pub fn scope_from_spec(spec: &ScopeSpec) -> Result<Scope, ScopeSpecError> {
    if spec.is_empty_declaration() {
        return Ok(Scope {
            allowed_targets: Vec::new(),
            excluded_targets: vec![ScopeRule::new("*".to_string())],
            allowed_ports: None,
            excluded_ports: Vec::new(),
            max_requests_per_second: None,
            require_explicit_scope: true,
            scope_file: None,
        });
    }

    let mut allowed_targets: Vec<ScopeRule> = Vec::new();
    for pattern in &spec.allowed_patterns {
        if pattern.is_empty() {
            continue;
        }
        allowed_targets.extend(allowed_rules_for_pattern(pattern, spec.allow_subdomains)?);
    }

    for entry in &spec.allowed_ips {
        if entry.is_empty() {
            continue;
        }
        allowed_targets.push(allowed_ip_rule(entry)?);
    }

    if allowed_targets.is_empty() {
        return Err(ScopeSpecError::InvalidDeclaration(
            "no usable allowances: allowed patterns/IPs are all empty or blank".to_string(),
        ));
    }

    let mut excluded_targets: Vec<ScopeRule> = Vec::new();
    for pattern in &spec.excluded_patterns {
        if pattern.is_empty() {
            continue;
        }
        excluded_targets.extend(exclusion_rules_for_pattern(pattern)?);
    }

    Ok(Scope {
        allowed_targets,
        excluded_targets,
        allowed_ports: None,
        excluded_ports: Vec::new(),
        max_requests_per_second: None,
        require_explicit_scope: false,
        scope_file: None,
    })
}

impl TryFrom<&ScopeSpec> for Scope {
    type Error = ScopeSpecError;

    fn try_from(spec: &ScopeSpec) -> Result<Self, Self::Error> {
        scope_from_spec(spec)
    }
}

fn allowed_rules_for_pattern(
    pattern: &str,
    allow_subdomains: bool,
) -> Result<Vec<ScopeRule>, ScopeSpecError> {
    if pattern == "*" {
        return Ok(vec![ScopeRule::new("*".to_string())]);
    }
    if pattern.contains('/') {
        let network = IpNetwork::from_str(pattern)
            .map_err(|e| ScopeSpecError::InvalidCidrPattern(pattern.to_string(), e.to_string()))?;
        return Ok(vec![ScopeRule::new(network.to_string())]);
    }
    if pattern.starts_with("*.") {
        return Ok(vec![ScopeRule::new(pattern.to_string())]);
    }
    if let Ok(ip) = IpAddr::from_str(pattern) {
        return Ok(vec![allowed_ip_rule(&ip.to_string())?]);
    }
    // Bare hostname: expand subdomains explicitly when hinted.
    if allow_subdomains {
        Ok(vec![
            ScopeRule::new(pattern.to_string()),
            ScopeRule::new(format!("*.{pattern}")),
        ])
    } else {
        Ok(vec![ScopeRule::new(pattern.to_string())])
    }
}

fn exclusion_rules_for_pattern(pattern: &str) -> Result<Vec<ScopeRule>, ScopeSpecError> {
    if pattern == "*" || pattern.starts_with("*.") {
        return Ok(vec![ScopeRule::new(pattern.to_string())]);
    }
    if pattern.contains('/') {
        let network = IpNetwork::from_str(pattern)
            .map_err(|e| ScopeSpecError::InvalidCidrPattern(pattern.to_string(), e.to_string()))?;
        return Ok(vec![ScopeRule::new(network.to_string())]);
    }
    if let Ok(ip) = IpAddr::from_str(pattern) {
        return Ok(vec![allowed_ip_rule(&ip.to_string())?]);
    }
    Ok(vec![ScopeRule::new(pattern.to_string())])
}

fn allowed_ip_rule(entry: &str) -> Result<ScopeRule, ScopeSpecError> {
    if let Ok(ip) = IpAddr::from_str(entry) {
        let cidr = match ip {
            IpAddr::V4(_) => format!("{ip}/32"),
            IpAddr::V6(_) => format!("{ip}/128"),
        };
        return ScopeRule::with_cidr(cidr)
            .map_err(|e| ScopeSpecError::InvalidAllowedIp(entry.to_string(), e.to_string()));
    }
    if let Ok(network) = IpNetwork::from_str(entry) {
        return ScopeRule::with_cidr(network.to_string())
            .map_err(|e| ScopeSpecError::InvalidAllowedIp(entry.to_string(), e.to_string()));
    }
    Err(ScopeSpecError::InvalidAllowedIp(
        entry.to_string(),
        "expected IP literal or CIDR".to_string(),
    ))
}

/// Effective authorization for a target under both an engine scope and an
/// optional request `ScopeSpec`.
///
/// Returns `true` only when the engine scope allows the target **and** the
/// converted specification (when present) also allows it. Conversion failures
/// deny (fail closed). A `None` specification means no additional transport
/// restriction, so only the engine scope decides.
pub fn is_target_allowed_by_scope_and_spec(
    scope: &Scope,
    spec: Option<&ScopeSpec>,
    target: &str,
) -> Result<bool, ScopeError> {
    let engine_allowed = scope.is_target_allowed(target)?;
    if !engine_allowed {
        return Ok(false);
    }
    let Some(spec) = spec else {
        return Ok(true);
    };
    let converted = scope_from_spec(spec).map_err(ScopeError::from)?;
    converted.is_target_allowed(target)
}

/// Resolver-injected variant of [`is_target_allowed_by_scope_and_spec`] for
/// deterministic tests.
pub fn is_target_allowed_by_scope_and_spec_with_resolver(
    scope: &Scope,
    spec: Option<&ScopeSpec>,
    target: &str,
    resolver: &dyn HostResolver,
) -> Result<bool, ScopeError> {
    let engine_allowed = scope.is_target_allowed_with_resolver(target, resolver)?;
    if !engine_allowed {
        return Ok(false);
    }
    let Some(spec) = spec else {
        return Ok(true);
    };
    let converted = scope_from_spec(spec).map_err(ScopeError::from)?;
    converted.is_target_allowed_with_resolver(target, resolver)
}

/// Evaluate an already-parsed `TargetScope` against both layers.
///
/// Used when the caller already owns a parsed target (e.g. dispatch binding).
/// Both layers evaluate the same parsed target; either denial wins.
pub fn is_parsed_target_allowed_by_scope_and_spec(
    scope: &Scope,
    spec: Option<&ScopeSpec>,
    target: &TargetScope,
    target_label: &str,
) -> bool {
    let engine_allowed = scope_allows_parsed(scope, target);
    if !engine_allowed {
        return false;
    }
    let Some(spec) = spec else {
        return true;
    };
    let converted = match scope_from_spec(spec) {
        Ok(converted) => converted,
        Err(e) => {
            tracing::warn!(
                target = %target_label,
                error = %e,
                "request scope specification failed conversion; denying"
            );
            return false;
        }
    };
    let allowed = scope_allows_parsed(&converted, target);
    if !allowed {
        tracing::warn!(
            target = %target_label,
            "request scope specification denies target"
        );
    }
    allowed
}

fn scope_allows_parsed(scope: &Scope, target: &TargetScope) -> bool {
    // Mirror Scope::is_target_allowed_with_resolver evaluation for an already
    // parsed target: exclusions first, then allowlist, then non-public
    // fallback when no allowlist is configured.
    if is_explicitly_excluded(scope, target) {
        return false;
    }
    if scope.allowed_targets.is_empty() {
        if scope.require_explicit_scope {
            return false;
        }
        use crate::config::scope::{classify_address, AddressClass};
        return !target
            .resolved_addresses
            .iter()
            .map(classify_address)
            .any(|class| class != AddressClass::Loopback && class.is_non_public());
    }
    if !target.resolved_addresses.is_empty() {
        let (all_allowed, any_excluded, _) =
            target.evaluate_addresses(&scope.allowed_targets, &scope.excluded_targets);
        return all_allowed && !any_excluded;
    }
    scope
        .allowed_targets
        .iter()
        .any(|rule| rule.matches(target))
}

fn is_explicitly_excluded(scope: &Scope, target: &TargetScope) -> bool {
    if scope
        .excluded_targets
        .iter()
        .any(|rule| rule.matches(target))
    {
        return true;
    }
    if !target.resolved_addresses.is_empty() {
        for addr in &target.resolved_addresses {
            let excluded = scope.excluded_targets.iter().any(|rule| {
                rule.cidr
                    .as_ref()
                    .and_then(|cidr| {
                        IpNetwork::from_str(cidr)
                            .ok()
                            .map(|net| net.contains(*addr))
                    })
                    .unwrap_or(false)
                    || (!rule.pattern.is_empty()
                        && rule.pattern.contains('/')
                        && IpNetwork::from_str(&rule.pattern)
                            .ok()
                            .map(|net| net.contains(*addr))
                            .unwrap_or(false))
            });
            if excluded {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::scope::ResolutionResult;
    use std::collections::HashMap;

    struct FakeResolver {
        responses: HashMap<String, Vec<IpAddr>>,
    }

    impl FakeResolver {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }

        fn with_response(mut self, host: &str, addrs: Vec<IpAddr>) -> Self {
            self.responses.insert(host.to_string(), addrs);
            self
        }
    }

    impl HostResolver for FakeResolver {
        fn resolve_all(&self, host: &str) -> ResolutionResult {
            ResolutionResult {
                hostname: host.to_string(),
                addresses: self.responses.get(host).cloned().unwrap_or_default(),
                error: None,
            }
        }
    }

    fn public_resolver() -> FakeResolver {
        FakeResolver::new()
            .with_response("example.com", vec!["93.184.216.34".parse().unwrap()])
            .with_response("other.com", vec!["93.184.216.35".parse().unwrap()])
            .with_response("sub.example.com", vec!["93.184.216.36".parse().unwrap()])
    }

    fn restrictive_engine_scope() -> Scope {
        Scope {
            allowed_targets: vec![ScopeRule::new("example.com".to_string())],
            excluded_targets: Vec::new(),
            allowed_ports: None,
            excluded_ports: Vec::new(),
            max_requests_per_second: None,
            require_explicit_scope: true,
            scope_file: None,
        }
    }

    #[test]
    fn permissive_spec_cannot_override_restrictive_engine_scope() {
        let engine = restrictive_engine_scope();
        let permissive = ScopeSpec::allow_all();
        let resolver = public_resolver();
        assert_eq!(
            is_target_allowed_by_scope_and_spec_with_resolver(
                &engine,
                Some(&permissive),
                "other.com",
                &resolver
            )
            .unwrap(),
            false
        );
        assert_eq!(
            is_target_allowed_by_scope_and_spec_with_resolver(
                &engine,
                Some(&permissive),
                "example.com",
                &resolver
            )
            .unwrap(),
            true
        );
    }

    #[test]
    fn restrictive_spec_constrains_permissive_engine_scope() {
        let engine = Scope {
            allowed_targets: vec![ScopeRule::new("*".to_string())],
            require_explicit_scope: true,
            ..Default::default()
        };
        let restrictive = ScopeSpec {
            allowed_patterns: vec!["example.com".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: false,
        };
        let resolver = public_resolver();
        assert!(is_target_allowed_by_scope_and_spec_with_resolver(
            &engine,
            Some(&restrictive),
            "example.com",
            &resolver
        )
        .unwrap());
        assert!(!is_target_allowed_by_scope_and_spec_with_resolver(
            &engine,
            Some(&restrictive),
            "other.com",
            &resolver
        )
        .unwrap());
    }

    #[test]
    fn exclusions_survive_conversion() {
        let spec = ScopeSpec {
            allowed_patterns: vec!["*".to_string()],
            excluded_patterns: vec!["admin.example.com".to_string(), "10.0.0.0/8".to_string()],
            allowed_ips: Vec::new(),
            allow_subdomains: true,
        };
        let converted = scope_from_spec(&spec).unwrap();
        assert_eq!(converted.excluded_targets.len(), 2);
        let resolver = FakeResolver::new()
            .with_response("admin.example.com", vec!["93.184.216.34".parse().unwrap()]);
        assert!(!converted
            .is_target_allowed_with_resolver("admin.example.com", &resolver)
            .unwrap());
        assert!(!converted
            .is_target_allowed_with_resolver("10.0.0.5", &resolver)
            .unwrap());
    }

    #[test]
    fn wildcard_hostname_semantics_match_engine() {
        let spec = ScopeSpec {
            allowed_patterns: vec!["*.example.com".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: true,
        };
        let converted = scope_from_spec(&spec).unwrap();
        let resolver = public_resolver();
        assert!(converted
            .is_target_allowed_with_resolver("sub.example.com", &resolver)
            .unwrap());
        assert!(converted
            .is_target_allowed_with_resolver("example.com", &resolver)
            .unwrap());
        assert!(!converted
            .is_target_allowed_with_resolver("other.com", &resolver)
            .unwrap());
    }

    #[test]
    fn subdomain_hint_expands_bare_hostname() {
        let with_subdomains = ScopeSpec {
            allowed_patterns: vec!["example.com".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: true,
        };
        let converted = scope_from_spec(&with_subdomains).unwrap();
        assert_eq!(converted.allowed_targets.len(), 2);
        let resolver = public_resolver();
        assert!(converted
            .is_target_allowed_with_resolver("sub.example.com", &resolver)
            .unwrap());

        let exact_only = ScopeSpec {
            allow_subdomains: false,
            ..with_subdomains
        };
        let converted = scope_from_spec(&exact_only).unwrap();
        assert_eq!(converted.allowed_targets.len(), 1);
        assert!(!converted
            .is_target_allowed_with_resolver("sub.example.com", &resolver)
            .unwrap());
        assert!(converted
            .is_target_allowed_with_resolver("example.com", &resolver)
            .unwrap());
    }

    #[test]
    fn ipv4_and_ipv6_literals_map_to_cidr() {
        let spec = ScopeSpec {
            allowed_patterns: Vec::new(),
            excluded_patterns: Vec::new(),
            allowed_ips: vec!["10.0.0.1".to_string(), "::1".to_string()],
            allow_subdomains: false,
        };
        let converted = scope_from_spec(&spec).unwrap();
        assert_eq!(converted.allowed_targets.len(), 2);
        let cidrs: Vec<&str> = converted
            .allowed_targets
            .iter()
            .filter_map(|r| r.cidr.as_deref())
            .collect();
        assert!(cidrs.contains(&"10.0.0.1/32"));
        assert!(cidrs.contains(&"::1/128"));
        let resolver = FakeResolver::new();
        assert!(converted
            .is_target_allowed_with_resolver("10.0.0.1", &resolver)
            .unwrap());
        assert!(!converted
            .is_target_allowed_with_resolver("10.0.0.2", &resolver)
            .unwrap());
        assert!(converted
            .is_target_allowed_with_resolver("::1", &resolver)
            .unwrap());
    }

    #[test]
    fn cidr_rules_enforced() {
        let spec = ScopeSpec {
            allowed_patterns: vec!["10.0.0.0/8".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: vec!["192.168.0.0/16".to_string()],
            allow_subdomains: false,
        };
        let converted = scope_from_spec(&spec).unwrap();
        let resolver = FakeResolver::new();
        assert!(converted
            .is_target_allowed_with_resolver("10.1.2.3", &resolver)
            .unwrap());
        assert!(converted
            .is_target_allowed_with_resolver("192.168.1.1", &resolver)
            .unwrap());
        assert!(!converted
            .is_target_allowed_with_resolver("172.16.0.1", &resolver)
            .unwrap());
    }

    #[test]
    fn mixed_public_private_resolution_denied_without_explicit_private() {
        // Hostname resolving to both public and private addresses: the engine
        // requires every address to be allowed, so the private address denies.
        let spec = ScopeSpec {
            allowed_patterns: vec!["93.184.216.0/24".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: false,
        };
        let converted = scope_from_spec(&spec).unwrap();
        let resolver = FakeResolver::new().with_response(
            "mixed.example",
            vec![
                "93.184.216.34".parse().unwrap(),
                "192.168.1.10".parse().unwrap(),
            ],
        );
        assert!(!converted
            .is_target_allowed_with_resolver("mixed.example", &resolver)
            .unwrap());
    }

    #[test]
    fn loopback_requires_explicit_allowance() {
        let spec = ScopeSpec {
            allowed_patterns: vec!["example.com".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: false,
        };
        let converted = scope_from_spec(&spec).unwrap();
        let resolver = FakeResolver::new();
        assert!(!converted
            .is_target_allowed_with_resolver("127.0.0.1", &resolver)
            .unwrap());

        let loopback_spec = ScopeSpec {
            allowed_patterns: vec!["127.0.0.0/8".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: false,
        };
        let converted = scope_from_spec(&loopback_spec).unwrap();
        assert!(converted
            .is_target_allowed_with_resolver("127.0.0.1", &resolver)
            .unwrap());
    }

    #[test]
    fn wildcard_spec_does_not_silently_authorize_private() {
        // Intersection with a default (empty) engine scope: public passes,
        // private is still denied even though the declaration is `*`.
        let engine = Scope::default();
        let wildcard = ScopeSpec::allow_all();
        let resolver = FakeResolver::new();
        assert!(is_target_allowed_by_scope_and_spec_with_resolver(
            &engine,
            Some(&wildcard),
            "93.184.216.34",
            &resolver
        )
        .unwrap());
        assert!(!is_target_allowed_by_scope_and_spec_with_resolver(
            &engine,
            Some(&wildcard),
            "192.168.1.10",
            &resolver
        )
        .unwrap());
    }

    #[test]
    fn empty_declaration_denies() {
        let empty = ScopeSpec::default();
        assert!(empty.is_empty_declaration());
        let converted = scope_from_spec(&empty).unwrap();
        let resolver = public_resolver();
        assert!(!converted
            .is_target_allowed_with_resolver("example.com", &resolver)
            .unwrap());
        // Intersection also denies.
        let engine = Scope {
            allowed_targets: vec![ScopeRule::new("*".to_string())],
            ..Default::default()
        };
        assert!(!is_target_allowed_by_scope_and_spec_with_resolver(
            &engine,
            Some(&empty),
            "example.com",
            &resolver
        )
        .unwrap());
        // Absent specification leaves the engine decision untouched.
        assert!(is_target_allowed_by_scope_and_spec_with_resolver(
            &engine,
            None,
            "example.com",
            &resolver
        )
        .unwrap());
    }

    #[test]
    fn invalid_allowed_ip_fails_closed() {
        let spec = ScopeSpec {
            allowed_patterns: Vec::new(),
            excluded_patterns: Vec::new(),
            allowed_ips: vec!["not-an-ip".to_string()],
            allow_subdomains: false,
        };
        assert!(scope_from_spec(&spec).is_err());
        let engine = restrictive_engine_scope();
        let resolver = public_resolver();
        assert!(is_target_allowed_by_scope_and_spec_with_resolver(
            &engine,
            Some(&spec),
            "example.com",
            &resolver
        )
        .is_err());
    }

    #[test]
    fn invalid_cidr_pattern_fails_closed() {
        let spec = ScopeSpec {
            allowed_patterns: vec!["10.0.0.0/99".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: false,
        };
        assert!(scope_from_spec(&spec).is_err());
    }

    #[test]
    fn converted_scope_leaves_ports_and_rates_to_engine() {
        let spec = ScopeSpec {
            allowed_patterns: vec!["example.com".to_string()],
            excluded_patterns: Vec::new(),
            allowed_ips: Vec::new(),
            allow_subdomains: false,
        };
        let converted = scope_from_spec(&spec).unwrap();
        assert!(converted.allowed_ports.is_none());
        assert!(converted.excluded_ports.is_empty());
        assert!(converted.max_requests_per_second.is_none());
        // Engine port policy still applies independently.
        let mut engine = restrictive_engine_scope();
        engine.excluded_ports = vec![22];
        assert!(!engine.is_port_allowed(22));
        assert!(engine.is_port_allowed(443));
    }

    #[test]
    fn spec_serialization_round_trip() {
        let spec = ScopeSpec {
            allowed_patterns: vec!["example.com".to_string(), "*.test.com".to_string()],
            excluded_patterns: vec!["admin.example.com".to_string()],
            allowed_ips: vec!["10.0.0.1".to_string()],
            allow_subdomains: false,
        };
        let json = serde_json::to_string(&spec).unwrap();
        let restored: ScopeSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, restored);
    }

    #[test]
    fn legacy_scope_json_deserializes() {
        // Payloads written with the legacy `Scope` DTO field names.
        let legacy = r#"{
            "allowed_patterns": ["example.com"],
            "excluded_patterns": [],
            "allowed_ips": [],
            "allow_subdomains": true
        }"#;
        let spec: ScopeSpec = serde_json::from_str(legacy).unwrap();
        assert_eq!(spec.allowed_patterns, vec!["example.com"]);
        assert!(spec.allow_subdomains);
        // ... and converts faithfully.
        let converted = scope_from_spec(&spec).unwrap();
        let resolver = public_resolver();
        assert!(converted
            .is_target_allowed_with_resolver("example.com", &resolver)
            .unwrap());
    }

    #[test]
    fn agent_view_projection_round_trips_cidr() {
        // Mirrors agent convert_scope: CIDR rules travel via allowed_ips so
        // the dispatch-time conversion preserves them.
        let engine = Scope {
            allowed_targets: vec![
                ScopeRule::new("example.com".to_string()),
                ScopeRule::with_cidr("10.0.0.0/8".to_string()).unwrap(),
            ],
            excluded_targets: vec![ScopeRule::with_cidr("10.0.0.1/32".to_string()).unwrap()],
            require_explicit_scope: true,
            ..Default::default()
        };
        let mut allowed_patterns = Vec::new();
        let mut allowed_ips = Vec::new();
        for rule in &engine.allowed_targets {
            if let Some(ref cidr) = rule.cidr {
                allowed_ips.push(cidr.clone());
            } else if rule.pattern.contains('/') {
                allowed_ips.push(rule.pattern.clone());
            } else {
                allowed_patterns.push(rule.pattern.clone());
            }
        }
        let spec = ScopeSpec {
            allowed_patterns,
            excluded_patterns: engine
                .excluded_targets
                .iter()
                .filter_map(|r| r.cidr.clone())
                .collect(),
            allowed_ips,
            allow_subdomains: true,
        };
        let converted = scope_from_spec(&spec).unwrap();
        let resolver = FakeResolver::new();
        assert!(converted
            .is_target_allowed_with_resolver("10.1.2.3", &resolver)
            .unwrap());
        assert!(!converted
            .is_target_allowed_with_resolver("10.0.0.1", &resolver)
            .unwrap());
    }
}
