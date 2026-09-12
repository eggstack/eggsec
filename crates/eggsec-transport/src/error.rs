//! Transport errors and policy checkpoints.
//!
//! [`TransportError::PolicyDenied`] is the fail-closed denial used by every
//! authorization checkpoint. The [`PolicyCheckpoint`] identifies *where* in
//! the dispatch path the denial happened so callers can distinguish an
//! initial-URL rejection from a redirect, proxy, or TLS-consistency denial
//! without parsing message strings.

use std::fmt;

/// Authorization checkpoint in the outbound dispatch path.
///
/// The order below mirrors the order a conforming [`crate::HttpTransport`]
/// implementation must enforce:
/// initial URL → host → DNS → socket → redirect → re-resolution → proxy → TLS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyCheckpoint {
    /// Initial URL canonicalization (parse, userinfo rejection, host present).
    InitialUrl,
    /// Initial hostname / direct-IP authorization (pattern + port).
    Host,
    /// DNS result authorization (candidate addresses vs policy).
    Dns,
    /// Selected socket address authorization immediately before connect.
    Socket,
    /// Redirect target authorization (each hop).
    Redirect,
    /// Reconnect / re-resolution authorization (retry, pool re-dial).
    Reresolution,
    /// Proxy/upstream endpoint authorization (distinct from ultimate target).
    Proxy,
    /// TLS SNI / Host override consistency.
    TlsConsistency,
}

impl fmt::Display for PolicyCheckpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::InitialUrl => "initial-url",
            Self::Host => "host",
            Self::Dns => "dns",
            Self::Socket => "socket",
            Self::Redirect => "redirect",
            Self::Reresolution => "reresolution",
            Self::Proxy => "proxy",
            Self::TlsConsistency => "tls-consistency",
        };
        write!(f, "{s}")
    }
}

/// Scope-aware transport failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    /// Request failed validation before any authorization (malformed URL,
    /// missing host, invalid header value, inconsistent TLS overrides, ...).
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Policy denied the dispatch at the given checkpoint (fail-closed).
    #[error("policy denied at {checkpoint}: {reason}")]
    PolicyDenied {
        checkpoint: PolicyCheckpoint,
        reason: String,
    },

    /// DNS resolution failed or returned no usable candidates.
    #[error("resolution failed for '{host}': {reason}")]
    ResolutionFailed { host: String, reason: String },

    /// Authority returned an address set that is not a non-empty subset of
    /// the resolver candidates (TOCTOU binding violation).
    #[error("authority binding invalid for '{host}': {reason}")]
    InvalidBinding { host: String, reason: String },

    /// Backend transport failure (connect, TLS handshake, timeout, ...).
    /// Concrete backends (Phase D) surface their own errors through this
    /// variant; the contract layer never invents a second scope language.
    #[error("transport failure: {0}")]
    Backend(String),
}

impl TransportError {
    /// Fail-closed policy denial at `checkpoint`.
    pub fn denied(checkpoint: PolicyCheckpoint, reason: impl Into<String>) -> Self {
        Self::PolicyDenied {
            checkpoint,
            reason: reason.into(),
        }
    }

    /// Returns `true` for fail-closed policy denials.
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::PolicyDenied { .. })
    }
}
