//! Engine-owned DNS acquisition and scope-resolution compatibility.
//!
//! The policy kernel ([`eggsec_policy::Scope::evaluate_facts`]) decides over
//! supplied [`eggsec_policy::TargetScope`] facts but never acquires them.
//! This module acquires facts (concrete [`SystemResolver`], `TargetScope`
//! parsing with a resolver) and provides the [`ScopeResolution`] extension
//! trait preserving the historical `Scope::is_target_allowed*` behavior for
//! engine callers.
//!
//! TOCTOU rule: authorizing a hostname/address set here does not authorize
//! arbitrary later resolution. `eggsec-transport::validate_binding` and the
//! approved-connection candidates in [`super::transport::ScopeAuthority`]
//! remain authoritative at the dispatch boundary.

use std::net::IpAddr;
use std::sync::Arc;

use eggsec_policy::{Scope, ScopeError, TargetScope};
use url::Url;

/// Result of resolving a hostname to IP addresses.
///
/// The resolver reports facts; policy decides whether they are authorized.
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// The normalized hostname that was resolved.
    pub hostname: String,
    /// All unique IP addresses returned by the resolver, in deterministic order.
    pub addresses: Vec<IpAddr>,
    /// Resolution error, if the resolver failed partially or completely.
    pub error: Option<String>,
}

impl ResolutionResult {
    /// Returns `true` if the resolver returned at least one address.
    pub fn has_addresses(&self) -> bool {
        !self.addresses.is_empty()
    }

    /// Returns the first resolved address, if any.
    pub fn first_address(&self) -> Option<IpAddr> {
        self.addresses.first().copied()
    }
}

/// Trait for DNS resolution, enabling deterministic unit tests and shared engine behavior.
///
/// The default implementation uses `std::net::ToSocketAddrs`. Tests can provide
/// a fake resolver that returns predetermined addresses without network access.
pub trait HostResolver: Send + Sync {
    /// Resolve a hostname to all unique IP addresses.
    ///
    /// Returns all addresses the system resolver returns, preserving deterministic
    /// ordering (sorted for audit/testing). Does not reject any address classes —
    /// policy decides authorization.
    fn resolve_all(&self, host: &str) -> ResolutionResult;
}

/// Default resolver using `std::net::ToSocketAddrs`.
///
/// Collects all unique addresses and returns them sorted for deterministic
/// ordering. Does not reject loopback, private, or other special addresses —
/// those are policy concerns, not resolver concerns.
pub struct SystemResolver;

impl HostResolver for SystemResolver {
    fn resolve_all(&self, host: &str) -> ResolutionResult {
        use std::collections::BTreeSet;
        use std::net::ToSocketAddrs;

        let hostname = host.to_string();

        let addrs: Vec<_> = match (host, 0u16).to_socket_addrs() {
            Ok(iter) => iter.collect(),
            Err(e) => {
                return ResolutionResult {
                    hostname,
                    addresses: Vec::new(),
                    error: Some(e.to_string()),
                };
            }
        };

        let mut unique: BTreeSet<IpAddr> = BTreeSet::new();
        for sock_addr in &addrs {
            unique.insert(sock_addr.ip());
        }

        ResolutionResult {
            hostname,
            addresses: unique.into_iter().collect(),
            error: None,
        }
    }
}

/// Create the default host resolver (system DNS).
pub fn default_resolver() -> Arc<dyn HostResolver> {
    Arc::new(SystemResolver)
}

/// Parse a target with full DNS resolution using the system resolver.
///
/// Returns all unique resolved addresses. DNS resolution errors for
/// hostnames (not literals) result in `ip: None` with an empty
/// `resolved_addresses` — callers decide whether to fail.
pub fn resolve_target_facts(target: &str) -> Result<TargetScope, ScopeError> {
    resolve_target_facts_with(target, &SystemResolver)
}

/// Parse a target with full DNS resolution using a custom resolver.
pub fn resolve_target_facts_with(
    target: &str,
    resolver: &dyn HostResolver,
) -> Result<TargetScope, ScopeError> {
    let target = target.trim();

    if target.is_empty() {
        return Err(ScopeError::InvalidTarget(target.to_string()));
    }

    // Literal IP — no resolution needed
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(TargetScope::for_ip(ip));
    }

    // URL form
    if let Ok(url) = Url::parse(target) {
        let host = url
            .host_str()
            .ok_or_else(|| ScopeError::InvalidTarget(target.to_string()))?
            .to_string();

        let result = resolver.resolve_all(&host);
        let addresses = result.addresses;

        if addresses.is_empty() {
            if let Some(ref err) = result.error {
                tracing::debug!(
                    host = %host,
                    error = %err,
                    "DNS resolution failed for URL host"
                );
            }
        }

        return Ok(TargetScope::for_host_with_addresses(host, addresses));
    }

    // Reject paths and other ambiguous forms
    if target.contains('/') || target.contains(' ') {
        return Err(ScopeError::InvalidTarget(target.to_string()));
    }

    let host = target.split(':').next().unwrap_or(target).to_string();

    if host.is_empty() {
        return Err(ScopeError::InvalidTarget(target.to_string()));
    }

    let result = resolver.resolve_all(&host);
    let addresses = result.addresses;

    if addresses.is_empty() {
        if let Some(ref err) = result.error {
            tracing::debug!(
                host = %host,
                error = %err,
                "DNS resolution failed for hostname"
            );
        }
    }

    Ok(TargetScope::for_host_with_addresses(host, addresses))
}

/// Parse a target for hostname-only matching using the system resolver.
///
/// Resolution is attempted but failures are non-fatal — `ip` and
/// `resolved_addresses` may be empty.
pub fn resolve_hostname_facts(target: &str) -> Result<TargetScope, ScopeError> {
    resolve_hostname_facts_with(target, &SystemResolver)
}

/// Parse a target for hostname-only matching using a custom resolver.
pub fn resolve_hostname_facts_with(
    target: &str,
    resolver: &dyn HostResolver,
) -> Result<TargetScope, ScopeError> {
    // Hostname-only parsing shares the tolerant resolution behavior of the
    // full parser; the scope layer decides whether CIDR rules require
    // resolved addresses.
    resolve_target_facts_with(target, resolver)
}

/// Load a [`Scope`] manifest from a TOML/YAML file.
///
/// Filesystem access stays engine-side; the parsed [`Scope`] is the pure
/// policy data type.
pub fn load_scope_from_file(path: &str) -> Result<Scope, ScopeError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ScopeError::FileRead(path.to_string(), e.to_string()))?;

    let scope: Scope = if path.ends_with(".yaml") || path.ends_with(".yml") {
        serde_yaml_neo::from_str(&content)
            .map_err(|e| ScopeError::Parse(path.to_string(), e.to_string()))?
    } else {
        toml::from_str(&content).map_err(|e| ScopeError::Parse(path.to_string(), e.to_string()))?
    };

    Ok(scope)
}

/// Engine-side scope evaluation preserving the historical
/// `Scope::is_target_allowed*` behavior: resolve first (I/O), then decide
/// via the pure [`Scope::evaluate_facts`].
///
/// Import this trait where the legacy method syntax is used:
/// `use crate::policy_bridge::resolver::ScopeResolution;`
pub trait ScopeResolution {
    /// Evaluate a target with the system resolver.
    fn is_target_allowed(&self, target: &str) -> Result<bool, ScopeError>;

    /// Evaluate a target with an injected resolver, primarily for deterministic
    /// policy tests and callers that already own a resolver.
    fn is_target_allowed_with_resolver(
        &self,
        target: &str,
        resolver: &dyn HostResolver,
    ) -> Result<bool, ScopeError>;

    /// Returns true if the target string matches any explicit exclusion rule.
    ///
    /// Used by policy enforcement to classify ExplicitExclusion denials separately
    /// from general "not in scope" denials.
    fn is_excluded(&self, target: &str) -> bool;

    /// Validate a URL's host against this scope.
    fn validate_url(&self, url: &str) -> Result<bool, ScopeError>;
}

impl ScopeResolution for Scope {
    fn is_target_allowed(&self, target: &str) -> Result<bool, ScopeError> {
        self.is_target_allowed_with_resolver(target, &SystemResolver)
    }

    fn is_target_allowed_with_resolver(
        &self,
        target: &str,
        resolver: &dyn HostResolver,
    ) -> Result<bool, ScopeError> {
        let target_scope = if self.has_ip_based_rules() {
            let scope = resolve_target_facts_with(target, resolver)?;
            if scope.ip.is_none() {
                return Err(ScopeError::DnsResolution(
                    target.to_string(),
                    "DNS resolution failed with CIDR rules configured".to_string(),
                ));
            }
            scope
        } else {
            resolve_hostname_facts_with(target, resolver)?
        };

        self.evaluate_facts(&target_scope)
    }

    fn is_excluded(&self, target: &str) -> bool {
        match resolve_hostname_facts(target) {
            Ok(ts) => self.is_explicitly_excluded(&ts),
            Err(_) => false,
        }
    }

    fn validate_url(&self, url: &str) -> Result<bool, ScopeError> {
        let parsed =
            Url::parse(url).map_err(|e| ScopeError::InvalidUrl(url.to_string(), e.to_string()))?;

        let host = parsed
            .host_str()
            .ok_or_else(|| ScopeError::InvalidUrl(url.to_string(), "No host".to_string()))?;

        self.is_target_allowed(host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_policy::ScopeRule;

    struct StaticResolver(Vec<IpAddr>);

    impl HostResolver for StaticResolver {
        fn resolve_all(&self, host: &str) -> ResolutionResult {
            ResolutionResult {
                hostname: host.to_string(),
                addresses: self.0.clone(),
                error: None,
            }
        }
    }

    #[test]
    fn static_resolver_reports_facts_without_policy() {
        let loopback: IpAddr = "127.0.0.1".parse().expect("loopback");
        let resolver = StaticResolver(vec![loopback]);
        let result = resolver.resolve_all("localhost");
        assert!(result.has_addresses());
        assert_eq!(result.first_address(), Some(loopback));
        assert_eq!(result.hostname, "localhost");
    }

    #[test]
    fn literal_ip_needs_no_resolution() {
        let ts = resolve_target_facts_with("10.0.0.1", &StaticResolver(vec![])).unwrap();
        assert_eq!(ts.host, "10.0.0.1");
        assert_eq!(ts.ip, Some("10.0.0.1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn scope_resolution_matches_pure_facts_evaluation() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));
        let resolver = StaticResolver(vec![]);
        assert!(scope
            .is_target_allowed_with_resolver("example.com", &resolver)
            .unwrap());
        assert!(!scope
            .is_target_allowed_with_resolver("other.com", &resolver)
            .unwrap());
    }
}
