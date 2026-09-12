//! Scope-aware outbound HTTP transport contract (Phase B).
//!
//! One EggSec-owned outbound HTTP capability contract, independent of
//! Reqwest, Hyper, Rustls, or Eggfetch. Authorization enforcement is part of
//! dispatch: [`HttpTransport::execute`] takes a `&dyn NetworkAuthority`, so a
//! caller cannot obtain an unrestricted dispatch by forgetting a scope
//! helper.
//!
//! The contract reuses the canonical authorization/scope model — it does not
//! create a second target-policy language. This crate owns the *checkpoint
//! shape* (neutral destination descriptors); the engine implements
//! [`NetworkAuthority`] over its canonical `Scope`/`TargetScope` policy.
//!
//! Checkpoint order (all fail-closed):
//! initial-URL → host → DNS → socket → redirect → re-resolution → proxy →
//! TLS-consistency. DNS results are bound to the connection path via
//! [`validate_binding`]: the authority approves a non-empty subset of the
//! resolver's candidates, and the connector dials only approved addresses.
//!
//! Start here: [`ScopedHttpRequest`], [`NetworkAuthority`],
//! [`HttpTransport`], [`RecordingFakeTransport`] (tests).

pub mod error;
#[cfg(any(test, feature = "test-util"))]
pub mod fake;
pub mod headers;
pub mod policy;
pub mod request;
pub mod resolver;
pub mod response;
pub mod transport;

pub use error::{PolicyCheckpoint, TransportError};
pub use headers::{
    apply_auth_headers, cookies_to_header_value, header_map_from_pairs, is_sensitive_header,
    merge_cookie_header, redacted_headers_debug, REDACTED,
};
pub use policy::{require_policy, NetworkAuthority};
pub use request::{
    redact_url_for_debug, reject_url_userinfo, ProxyCredential, ProxyIntent, RedirectPolicy,
    RequestBody, ScopedHttpRequest, TimeoutPolicy, TlsPolicy, TransportHints,
};
pub use resolver::{
    validate_binding, ApprovedBinding, InMemoryResolver, ResolvedCandidates,
    SystemTransportResolver, TransportResolver,
};
pub use response::{ConnectionInfo, ScopedHttpResponse};
pub use transport::HttpTransport;
// Re-export stable `http` types so engine/domain code can name transport-neutral
// header types without taking a direct `http` dependency.
pub use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};

#[cfg(any(test, feature = "test-util"))]
pub use fake::{CannedResponse, RecordedHop, RecordingFakeTransport};
