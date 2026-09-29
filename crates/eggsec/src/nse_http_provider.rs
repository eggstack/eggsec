//! Engine-owned runtime HTTP provider adapter (005C, replayed in M006B).
//!
//! Implements the standalone runtime's [`NseHttpProvider`] over
//! [`HttpTransport`] plus an injected [`NetworkAuthority`], reusing
//! [`nse_http_capability`] request mapping, TLS policy, and redirect
//! posture. No `reqwest`/concrete-client types appear here; the transport
//! backend owns execution.
//!
//! Provenance: logical replay of `m005c-eggsec-http-adapter@4ade61a`
//! onto current main against the `eggsec-nse 0.2.0` registry contract.
//! The stale branch history was not merged.
//!
//! Authority rules (plan work packages E/F):
//!
//! - The adapter never manufactures authority: the `Arc<dyn
//!   NetworkAuthority>` comes from the caller (an already-authorized
//!   Eggsec execution), and out-of-scope/cross-host requests fail closed
//!   inside the transport with no native fallback.
//! - The runtime DTO's `insecure_tls` flag never flows into the scoped
//!   request. Only the construction-time profile decision does, so scripts
//!   cannot escalate verification off.
//! - The provider contract is synchronous; the async transport future runs
//!   on the ambient runtime when present (blocking context) or on an
//!   ephemeral current-thread runtime otherwise. In-flight adapter calls
//!   are bounded by the request timeout policy. Callers inside async
//!   (non-blocking) contexts must restructure to an async provider instead
//!   of calling this bridge.
//!
//! Production dispatch threading (which Eggsec execution point supplies a
//! live scope) is a prerequisite integration step: current NSE dispatch has
//! no `Scope`/`ApprovedExecution`, so manual dispatch keeps the native
//! runtime provider until the NSE enforcement path exists. This module
//! provides the adapter, its mapping/authority tests, and the constructor
//! the future threading will use — no production dispatch is rewired here.
//!
//! Dormant status (M006B): this adapter compiles and tests against the
//! real 0.2.0 registry contract but has no production caller. Automated
//! NSE execution through it remains disabled until the M007
//! protocol-library capability-gating/scope-threading milestone.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use eggsec_transport::{HttpTransport, NetworkAuthority, ScopedHttpResponse, TlsPolicy};
use rustc_hash::FxHashMap;

use super::nse_http_capability;
use crate::nse::{NseHttpError, NseHttpProvider, NseHttpRequest, NseHttpResponse};

/// Engine-owned HTTP provider adapter for NSE script execution.
///
/// Constructed with an already-authorized transport authority; see the
/// module docs for the authority rules.
pub struct NseHttpTransportProvider<T: HttpTransport> {
    transport: Arc<T>,
    authority: Arc<dyn NetworkAuthority>,
    insecure_tls: bool,
    default_timeout: Duration,
}

impl<T: HttpTransport> NseHttpTransportProvider<T> {
    /// Build the adapter.
    ///
    /// - `transport`: scope-aware backend owning execution.
    /// - `authority`: already-authorized Eggsec authority (e.g. an owned
    ///   scope authority bound to the operation's scope snapshot).
    /// - `insecure_tls`: profile decision from the construction point
    ///   (`allows_insecure_tls`); the only TLS intent the adapter honors.
    pub fn new(
        transport: Arc<T>,
        authority: Arc<dyn NetworkAuthority>,
        insecure_tls: bool,
    ) -> Self {
        Self {
            transport,
            authority,
            insecure_tls,
            default_timeout: Duration::from_secs(
                super::nse_http_capability::NSE_DEFAULT_TIMEOUT_SECS,
            ),
        }
    }

    /// Override the default per-request timeout (connect timeout stays at
    /// the NSE parity value).
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout.max(Duration::from_secs(1));
        self
    }

    fn tls_policy(&self) -> TlsPolicy {
        if self.insecure_tls {
            TlsPolicy::insecure()
        } else {
            TlsPolicy::verified()
        }
    }

    fn scoped_request(
        &self,
        req: &NseHttpRequest,
    ) -> Result<eggsec_transport::ScopedHttpRequest, NseHttpError> {
        let mut headers = FxHashMap::default();
        for (name, value) in &req.headers {
            headers.insert(name.clone(), value.clone());
        }
        let body = if req.body.is_empty() {
            None
        } else {
            Some(req.body.clone())
        };
        // Request timeouts honor the DTO; the adapter default applies
        // only when the DTO carries no timeout. Connect timeout stays at
        // the NSE parity value inside the shared builder.
        let timeout_secs = if req.timeout.is_zero() {
            self.default_timeout.as_secs()
        } else {
            req.timeout.as_secs()
        }
        .max(1);
        nse_http_capability::build_scoped_request_with_tls_policy(
            self.tls_policy(),
            req.method.as_str(),
            &req.url,
            &headers,
            body,
            timeout_secs,
        )
        .map_err(NseHttpError::Request)
    }

    fn map_response(resp: ScopedHttpResponse) -> NseHttpResponse {
        NseHttpResponse {
            status: resp.status.as_u16(),
            headers: resp
                .headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
            body: resp.body.to_vec(),
            final_url: resp.final_url.to_string(),
            // The transport contract does not carry the wire version;
            // leave it empty rather than fabricate one.
            version: String::new(),
        }
    }
}

impl<T: HttpTransport> NseHttpProvider for NseHttpTransportProvider<T> {
    fn request(&self, req: &NseHttpRequest) -> Result<NseHttpResponse, NseHttpError> {
        // Structural non-escalation: `req.insecure_tls` (script-influenced
        // DTO content) is deliberately ignored; only the
        // construction-time profile decision shapes the scoped request.
        let scoped = self.scoped_request(req)?;
        let fut = self.transport.execute(self.authority.as_ref(), scoped);
        block_on_transport(fut)
            .map(Self::map_response)
            .map_err(map_transport_error)
    }
}

fn map_transport_error(err: eggsec_transport::TransportError) -> NseHttpError {
    use eggsec_transport::TransportError as TE;
    if err.is_denied() {
        return NseHttpError::Denied(err.to_string());
    }
    match err {
        TE::InvalidRequest(msg) => NseHttpError::Request(msg),
        TE::ResolutionFailed { reason, .. } => NseHttpError::Connection(reason),
        TE::InvalidBinding { reason, .. } => NseHttpError::Connection(reason),
        TE::Backend(msg) if is_timeout_message(&msg) => NseHttpError::Timeout,
        TE::Backend(msg) => NseHttpError::Connection(msg),
        TE::PolicyDenied { .. } => NseHttpError::Denied(err.to_string()),
    }
}

fn is_timeout_message(msg: &str) -> bool {
    let lower = msg.to_ascii_lowercase();
    lower.contains("timed out") || lower.contains("timeout") || lower.contains("deadline")
}

/// Run a transport future to completion from the synchronous provider
/// contract: ambient runtime when present in a blocking context, else an
/// ephemeral current-thread runtime (mirrors the runtime bridge posture).
fn block_on_transport<F>(fut: F) -> Result<ScopedHttpResponse, eggsec_transport::TransportError>
where
    F: Future<Output = Result<ScopedHttpResponse, eggsec_transport::TransportError>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(fut),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    eggsec_transport::TransportError::Backend(format!(
                        "NSE HTTP bridge runtime unavailable: {e}"
                    ))
                })?;
            runtime.block_on(fut)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loadtest::OwnedScopeAuthority;
    use crate::nse::NseHttpMethod;
    use eggsec_policy::{Scope, ScopeRule};
    use eggsec_transport::fake::{CannedResponse, RecordingFakeTransport};
    use eggsec_transport::InMemoryResolver;

    fn recording_fake() -> RecordingFakeTransport {
        let resolver = InMemoryResolver::new()
            .with("webscan.test", vec!["127.0.0.1"])
            .with("evil.test", vec!["192.0.2.1"])
            .shared();
        RecordingFakeTransport::new(resolver).with_canned(
            "http://webscan.test/",
            CannedResponse::ok(b"fake-body".to_vec()),
        )
    }

    fn loopback_scope() -> Scope {
        Scope {
            allowed_targets: vec![ScopeRule {
                pattern: String::new(),
                cidr: Some("127.0.0.0/8".to_string()),
                description: None,
            }],
            ..Scope::default()
        }
    }

    /// Default scope allows loopback (default policy exempts it) — the
    /// permissive test authority.
    fn permissive_scope() -> Scope {
        Scope::default()
    }

    /// Explicit-scope-required with no allowed targets denies everything —
    /// the denying test authority.
    fn deny_scope() -> Scope {
        Scope {
            require_explicit_scope: true,
            ..Scope::default()
        }
    }

    fn runtime_get(url: &str, host: &str) -> NseHttpRequest {
        NseHttpRequest {
            method: NseHttpMethod::Get,
            url: url.to_string(),
            host: host.to_string(),
            headers: vec![("X-Probe".to_string(), "1".to_string())],
            body: Vec::new(),
            timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            insecure_tls: false,
        }
    }

    #[test]
    fn in_scope_request_reaches_transport_with_same_target() {
        let transport = Arc::new(recording_fake());
        let authority: Arc<dyn NetworkAuthority> =
            Arc::new(OwnedScopeAuthority::new(loopback_scope()));
        let provider = NseHttpTransportProvider::new(transport.clone(), authority, false);

        let resp = provider
            .request(&runtime_get("http://webscan.test/", "webscan.test"))
            .expect("in-scope serves");

        assert_eq!(resp.status, 200);
        assert_eq!(resp.body_text(), "fake-body");
        let hops = transport.hops();
        assert_eq!(hops.len(), 1, "exactly one authorized hop");
        assert_eq!(hops[0].host, "webscan.test");
        assert_eq!(hops[0].url, "http://webscan.test/");
        assert_eq!(
            hops[0].approved_addrs,
            vec!["127.0.0.1".parse::<std::net::IpAddr>().unwrap()],
            "transport must carry the same authorized target"
        );
        assert!(
            hops[0].tls_verified,
            "verified adapter keeps verification on"
        );
        assert!(hops[0].had_authorization == false, "no auth header sent");
        // Checkpoint order proves the authority (not the URL) drove dispatch.
        assert!(
            hops[0].checkpoint_order.len() >= 3,
            "initial/host/dns checkpoints expected, got {:?}",
            hops[0].checkpoint_order
        );
    }

    #[test]
    fn out_of_scope_fails_closed_without_transport_contact() {
        let transport = Arc::new(recording_fake());
        let authority: Arc<dyn NetworkAuthority> =
            Arc::new(OwnedScopeAuthority::new(loopback_scope()));
        let provider = NseHttpTransportProvider::new(transport.clone(), authority, false);

        let err = provider
            .request(&runtime_get("http://evil.test/", "evil.test"))
            .expect_err("out-of-scope must fail closed");
        assert!(matches!(err, NseHttpError::Denied(_)), "{err:?}");
        assert_eq!(
            transport.hop_count(),
            0,
            "denied requests never reach transport hops"
        );
    }

    #[test]
    fn deny_all_authority_fails_closed() {
        let transport = Arc::new(recording_fake());
        let authority: Arc<dyn NetworkAuthority> = Arc::new(OwnedScopeAuthority::new(deny_scope()));
        let provider = NseHttpTransportProvider::new(transport.clone(), authority, false);

        let err = provider
            .request(&runtime_get("http://webscan.test/", "webscan.test"))
            .expect_err("deny-all must fail");
        assert!(matches!(err, NseHttpError::Denied(_)), "{err:?}");
        assert_eq!(transport.hop_count(), 0);
    }

    #[test]
    fn allow_all_authority_serves_without_scope() {
        let transport = Arc::new(recording_fake());
        let authority: Arc<dyn NetworkAuthority> =
            Arc::new(OwnedScopeAuthority::new(permissive_scope()));
        let provider = NseHttpTransportProvider::new(transport, authority, false);

        let resp = provider
            .request(&runtime_get("http://webscan.test/", "webscan.test"))
            .expect("allow-all serves");
        assert_eq!(resp.status, 200);
    }

    #[test]
    fn script_tls_intent_never_escalates() {
        // The runtime DTO claims insecure, but the verified adapter must
        // still dispatch verified: scripts cannot escalate.
        let transport = Arc::new(recording_fake());
        let authority: Arc<dyn NetworkAuthority> =
            Arc::new(OwnedScopeAuthority::new(permissive_scope()));
        let provider = NseHttpTransportProvider::new(transport.clone(), authority, false);

        let mut req = runtime_get("http://webscan.test/", "webscan.test");
        req.insecure_tls = true;
        provider.request(&req).expect("serves");

        let hops = transport.hops();
        assert_eq!(hops.len(), 1);
        assert!(
            hops[0].tls_verified,
            "DTO insecure intent must not escalate a verified adapter"
        );
    }

    #[test]
    fn insecure_adapter_arms_bypass_explicitly() {
        // The bypass comes from the construction-time profile decision,
        // never from script content.
        let transport = Arc::new(recording_fake());
        let authority: Arc<dyn NetworkAuthority> =
            Arc::new(OwnedScopeAuthority::new(permissive_scope()));
        let provider = NseHttpTransportProvider::new(transport.clone(), authority, true);

        let req = runtime_get("http://webscan.test/", "webscan.test");
        assert!(!req.insecure_tls, "fixture DTO is verified");
        provider.request(&req).expect("serves");

        let hops = transport.hops();
        assert_eq!(hops.len(), 1);
        assert!(
            !hops[0].tls_verified,
            "construction-time insecure intent must apply"
        );
    }

    #[test]
    fn invalid_url_maps_to_request_error_without_transport() {
        let transport = Arc::new(recording_fake());
        let authority: Arc<dyn NetworkAuthority> =
            Arc::new(OwnedScopeAuthority::new(permissive_scope()));
        let provider = NseHttpTransportProvider::new(transport.clone(), authority, false);

        let err = provider
            .request(&runtime_get(
                "http://user:pass@webscan.test/",
                "webscan.test",
            ))
            .expect_err("userinfo URL must fail");
        assert!(matches!(err, NseHttpError::Request(_)), "{err:?}");
        assert_eq!(transport.hop_count(), 0);
    }

    #[test]
    fn no_concrete_client_types_in_adapter() {
        // Compile-time boundary mirror of the capability seam test: the
        // adapter must not name concrete clients either.
        let src = include_str!("nse_http_provider.rs");
        // (Banned literals are concat-built so this test does not match
        // itself in the include_str! scan.)
        let banned: Vec<String> = vec![
            ["reqwest", "::"].concat(),
            "eggfetch".to_string(),
            ["rustls", "::"].concat(),
            ["tokio_rustls", "::"].concat(),
            ["hickory_resolver", "::"].concat(),
            "RequestBuilder".to_string(),
            "Client::builder".to_string(),
        ];
        for b in &banned {
            let code_lines: Vec<&str> = src
                .lines()
                .filter(|l| {
                    let t = l.trim_start();
                    !(t.starts_with("//!")
                        || t.starts_with("//")
                        || t.contains(".concat()")
                        || t.contains(".to_string()"))
                })
                .collect();
            let joined = code_lines.join("\n");
            assert!(
                !joined.contains(b.as_str()),
                "adapter code must not mention '{b}'"
            );
        }
    }
}
