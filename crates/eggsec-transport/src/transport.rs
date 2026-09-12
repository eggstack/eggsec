//! Authorization-aware execution trait.
//!
//! [`HttpTransport::execute`] takes the authority **and** the request: there
//! is no unchecked entry point. Dropping the returned future cancels the
//! dispatch (no background tasks outlive the future); backends must not
//! spawn detached work that survives cancellation.

use crate::{NetworkAuthority, ScopedHttpRequest, ScopedHttpResponse, TransportError};

/// Scope-aware outbound HTTP transport.
///
/// Conforming implementations enforce, in order:
/// 1. initial URL canonicalization (`initial-url`);
/// 2. initial hostname / direct-IP authorization (`host`);
/// 3. DNS result authorization (`dns`) with TOCTOU-closed binding
///    ([`crate::validate_binding`]);
/// 4. selected socket address authorization immediately before connect
///    (`socket`);
/// 5. each redirect target (`redirect` + re-run of host/dns/socket);
/// 6. each reconnect / re-resolution (`reresolution`);
/// 7. proxy/upstream endpoint and ultimate destination as separate
///    concepts (`proxy`);
/// 8. TLS SNI / Host override consistency (`tls-consistency`).
pub trait HttpTransport: Send + Sync {
    /// Execute `request` under `authority` (mandatory — no unchecked path).
    fn execute<'a>(
        &'a self,
        authority: &'a dyn NetworkAuthority,
        request: ScopedHttpRequest,
    ) -> impl std::future::Future<Output = Result<ScopedHttpResponse, TransportError>> + Send + 'a;
}
