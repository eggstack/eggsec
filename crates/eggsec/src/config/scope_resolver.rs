//! Host resolution for scope evaluation (Phase D WS7).
//!
//! Cohesive module extracted from `config/scope.rs`: DNS facts only. The
//! resolver reports addresses; policy decides authorization. Never reject
//! address classes here — defer to policy.
//!
//! Stable facade: `config/scope.rs` re-exports everything here, so
//! `crate::config::scope::{HostResolver, SystemResolver, ResolutionResult,
//! default_resolver}` and the corresponding `crate::config::{...}` paths
//! keep working.

use std::net::IpAddr;
use std::sync::Arc;

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn default_resolver_returns_arc_trait_object() {
        let resolver = default_resolver();
        // `localhost` should resolve on any standard system; if DNS is
        // unavailable the resolver still reports facts (possibly an error)
        // rather than panicking.
        let result = resolver.resolve_all("localhost");
        assert_eq!(result.hostname, "localhost");
        let _ = result.has_addresses();
    }
}
