//! Resolver/connect contract.
//!
//! Safe model (TOCTOU-closed):
//!
//! ```text
//! resolve host -> candidate addresses
//!              -> authority filters/approves candidates
//!              -> transport receives approved address set/binding
//!              -> connector uses only those approved addresses
//! ```
//!
//! A policy check that resolves via one path and then hands only the
//! hostname to an unrelated connector that re-resolves independently does
//! **not** close DNS-rebinding risk and is forbidden by this contract.
//! The transport must validate that the authority's approved set is a
//! non-empty subset of the candidates it resolved, and must connect only
//! to an approved address.
//!
//! IPv4/IPv6 behavior: resolvers report both families without filtering;
//! policy decides. Multiple-answer ordering is deterministic (sorted) for
//! audit/test stability. There is no TTL/cache in Phase B: every policy
//! evaluation resolves fresh, and revalidation happens on each connect,
//! retry/reconnect, and redirect hop. A future cache must be explicit about
//! TTL, ordering, and invalidation, and must revalidate before connect.

use crate::TransportError;
use std::collections::{BTreeSet, HashMap};
use std::net::IpAddr;
use std::sync::Arc;

/// Deterministic DNS facts for one hostname.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCandidates {
    /// Normalized hostname that was resolved.
    pub hostname: String,
    /// All unique candidate addresses in deterministic (sorted) order.
    pub addresses: Vec<IpAddr>,
}

impl ResolvedCandidates {
    /// New candidate set (deduplicated + sorted for determinism).
    pub fn new(hostname: impl Into<String>, addresses: Vec<IpAddr>) -> Self {
        let unique: BTreeSet<IpAddr> = addresses.into_iter().collect();
        Self {
            hostname: hostname.into(),
            addresses: unique.into_iter().collect(),
        }
    }

    /// Empty (resolution returned nothing usable).
    pub fn empty(hostname: impl Into<String>) -> Self {
        Self {
            hostname: hostname.into(),
            addresses: Vec::new(),
        }
    }

    /// `true` when at least one candidate exists.
    pub fn has_addresses(&self) -> bool {
        !self.addresses.is_empty()
    }
}

/// DNS resolver abstraction for the transport contract.
///
/// Implementations report facts only; the [`crate::NetworkAuthority`]
/// decides authorization. Never reject address classes here.
pub trait TransportResolver: Send + Sync {
    /// Resolve `host` to all unique candidate addresses (deterministic order).
    fn resolve(&self, host: &str) -> ResolvedCandidates;
}

/// System resolver via `std::net::ToSocketAddrs` (no extra dependencies).
///
/// Collects all unique addresses, sorted. Reports loopback/private/link-local
/// addresses without filtering — those are policy concerns.
#[derive(Debug, Default)]
pub struct SystemTransportResolver;

impl TransportResolver for SystemTransportResolver {
    fn resolve(&self, host: &str) -> ResolvedCandidates {
        use std::net::ToSocketAddrs;
        let addrs: Vec<_> = match (host, 0u16).to_socket_addrs() {
            Ok(iter) => iter.collect(),
            Err(_) => return ResolvedCandidates::empty(host),
        };
        let unique: BTreeSet<IpAddr> = addrs.iter().map(|s| s.ip()).collect();
        ResolvedCandidates {
            hostname: host.to_string(),
            addresses: unique.into_iter().collect(),
        }
    }
}

/// Deterministic in-memory resolver for tests and fixtures.
///
/// Maps hostnames to canned address lists. Unknown hosts resolve empty.
/// Returned addresses are deduplicated + sorted, matching production order.
#[derive(Debug, Default)]
pub struct InMemoryResolver {
    responses: HashMap<String, Vec<IpAddr>>,
}

impl InMemoryResolver {
    /// Empty fixture resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add `host -> addrs` (parses each address; panics on invalid test IPs
    /// so fixture bugs fail loudly).
    pub fn with(mut self, host: &str, addrs: Vec<&str>) -> Self {
        self.responses.insert(
            host.to_string(),
            addrs
                .into_iter()
                .map(|a| a.parse().expect("test IP parses"))
                .collect(),
        );
        self
    }

    /// Shared ownership helper for transports.
    pub fn shared(self) -> Arc<dyn TransportResolver> {
        Arc::new(self)
    }
}

impl TransportResolver for InMemoryResolver {
    fn resolve(&self, host: &str) -> ResolvedCandidates {
        match self.responses.get(host) {
            Some(addrs) => ResolvedCandidates::new(host, addrs.clone()),
            None => ResolvedCandidates::empty(host),
        }
    }
}

/// Approved address binding: the authority's decision bound to the actual
/// connection path.
///
/// The transport must enforce:
/// 1. `approved` is non-empty and every entry appears in `candidates`
///    ([`validate_binding`]);
/// 2. the connector dials only an address from `approved` (first entry by
///    default; happy-eyeballs-style racing across approved entries is
///    allowed, racing across unapproved entries is not).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedBinding {
    /// Hostname this binding authorizes.
    pub host: String,
    /// Port this binding authorizes.
    pub port: u16,
    /// Approved addresses (non-empty subset of the resolved candidates).
    pub approved: Vec<IpAddr>,
}

impl ApprovedBinding {
    /// New binding (validates non-empty; subset check is [`validate_binding`]).
    pub fn new(
        host: impl Into<String>,
        port: u16,
        approved: Vec<IpAddr>,
    ) -> Result<Self, TransportError> {
        if approved.is_empty() {
            return Err(TransportError::InvalidBinding {
                host: host.into(),
                reason: "approved address set is empty".to_string(),
            });
        }
        Ok(Self {
            host: host.into(),
            port,
            approved,
        })
    }

    /// First approved address (default dial target).
    pub fn primary(&self) -> IpAddr {
        self.approved[0]
    }

    /// `true` when `addr` is approved for this binding.
    pub fn contains(&self, addr: &IpAddr) -> bool {
        self.approved.contains(addr)
    }
}

/// Validate that `approved` is a non-empty subset of `candidates`.
///
/// This is the TOCTOU gate: an authority that invents addresses the resolver
/// never returned (or approves nothing) fails closed here, before connect.
pub fn validate_binding(
    host: &str,
    candidates: &[IpAddr],
    approved: &[IpAddr],
) -> Result<ApprovedBinding, TransportError> {
    if approved.is_empty() {
        return Err(TransportError::InvalidBinding {
            host: host.to_string(),
            reason: "authority approved no addresses".to_string(),
        });
    }
    let candidate_set: BTreeSet<IpAddr> = candidates.iter().copied().collect();
    for addr in approved {
        if !candidate_set.contains(addr) {
            return Err(TransportError::InvalidBinding {
                host: host.to_string(),
                reason: format!("authority approved un-resolved address {addr}"),
            });
        }
    }
    // Port is filled by the caller (transport knows the URL port); default 0
    // here means "unspecified" and must be overwritten before connect.
    ApprovedBinding::new(host, 0, approved.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inmemory_resolver_is_deterministic_and_sorted() {
        let r =
            InMemoryResolver::new().with("h.example", vec!["::1", "93.184.216.34", "127.0.0.1"]);
        let got = r.resolve("h.example");
        let mut sorted = got.addresses.clone();
        sorted.sort();
        assert_eq!(
            got.addresses, sorted,
            "resolver order must be deterministic"
        );
        assert!(got.has_addresses());
        assert!(!r.resolve("unknown.example").has_addresses());
    }

    #[test]
    fn binding_rejects_invented_addresses() {
        let cands: Vec<IpAddr> = vec!["93.184.216.34".parse().expect("ip")];
        let evil: Vec<IpAddr> = vec!["203.0.113.99".parse().expect("ip")];
        let err = validate_binding("h.example", &cands, &evil).unwrap_err();
        assert!(matches!(err, TransportError::InvalidBinding { .. }));
        let err = validate_binding("h.example", &cands, &[]).unwrap_err();
        assert!(matches!(err, TransportError::InvalidBinding { .. }));
        let ok = validate_binding("h.example", &cands, &cands).expect("subset");
        assert!(ok.contains(&cands[0]));
    }
}
