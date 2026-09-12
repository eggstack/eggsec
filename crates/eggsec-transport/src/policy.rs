//! Authorization-aware execution contract.
//!
//! The central invariant: **a caller cannot obtain an unrestricted network
//! dispatch by forgetting to call a scope helper.** Every execution takes a
//! `&dyn NetworkAuthority`, and a conforming transport enforces the
//! checkpoints below in order. There is no `execute_unchecked`.
//!
//! The contract reuses EggSec's canonical scope model — it does not invent a
//! second target-policy language. The transport crate owns the *checkpoint
//! shape* (neutral destination descriptors); the engine crate implements
//! `NetworkAuthority` over its canonical `Scope`/`TargetScope` policy. See
//! `eggsec::config::scope_transport::ScopeAuthority` for the binding.

use crate::{PolicyCheckpoint, TransportError};
use std::net::IpAddr;
use url::Url;

/// Scope/policy authority for outbound dispatch.
///
/// All methods are synchronous policy checks (no I/O): the transport
/// resolves via [`crate::TransportResolver`] and hands candidates to the
/// authority; the authority never re-resolves through a second path (that
/// would reopen TOCTOU). Deny fail-closed with [`TransportError::denied`].
///
/// Default methods: [`authorize_reresolution`] defaults to
/// [`authorize_resolved`](Self::authorize_resolved) so authorities only
/// override it when they need distinct retry audit. All other checkpoints
/// are required.
pub trait NetworkAuthority: Send + Sync {
    /// Checkpoint 1 — initial URL canonicalization: parseable URL, host
    /// present, userinfo rejected, no smuggled credentials.
    fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError>;

    /// Checkpoint 2 — initial hostname / direct-IP authorization: pattern
    /// match + port policy, before any DNS.
    fn authorize_host(
        &self,
        host: &str,
        port: Option<u16>,
        is_ip_literal: bool,
    ) -> Result<(), TransportError>;

    /// Checkpoint 3 — DNS result authorization: approve a non-empty subset
    /// of `candidates`. Returning an empty set or addresses outside
    /// `candidates` fails closed in [`crate::validate_binding`].
    fn authorize_resolved(
        &self,
        host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError>;

    /// Checkpoint 4 — selected socket address authorization immediately
    /// before connect (the address actually dialed, not just the candidate
    /// set).
    fn authorize_socket(&self, host: &str, addr: IpAddr, port: u16) -> Result<(), TransportError>;

    /// Checkpoint 5 — each redirect target (`from` → `to`). The transport
    /// calls this per hop *in addition* to re-running host/DNS/socket
    /// checks for the new target.
    fn authorize_redirect(&self, from: &Url, to: &Url) -> Result<(), TransportError>;

    /// Checkpoint 6 — each reconnect / re-resolution (retry, pool re-dial).
    /// Defaults to [`authorize_resolved`](Self::authorize_resolved).
    fn authorize_reresolution(
        &self,
        host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        self.authorize_resolved(host, candidates)
    }

    /// Checkpoint 7 — proxy/upstream endpoint **and** ultimate destination
    /// as separate concepts. Authorizing one must never imply the other.
    fn authorize_proxy(&self, proxy_endpoint: &Url, ultimate: &Url) -> Result<(), TransportError>;

    /// Checkpoint 8 — TLS SNI / Host override consistency where overrides
    /// are supported. Mismatched overrides deny fail-closed.
    fn check_tls_consistency(
        &self,
        request_host: &str,
        sni_override: Option<&str>,
        host_override: Option<&str>,
    ) -> Result<(), TransportError>;
}

/// Helper: strictly require `cond`, else deny at `checkpoint`.
pub fn require_policy(
    cond: bool,
    checkpoint: PolicyCheckpoint,
    reason: impl Into<String>,
) -> Result<(), TransportError> {
    if cond {
        Ok(())
    } else {
        Err(TransportError::denied(checkpoint, reason))
    }
}
