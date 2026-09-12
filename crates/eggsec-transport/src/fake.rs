//! No-network fake/recording transport for unit tests.
//!
//! Available when `cfg(test)` or the `test-util` feature is enabled so
//! production builds never carry it. The fake performs the full checkpoint
//! sequence (initial → host → DNS/re-resolution → socket → TLS → proxy,
//! plus `authorize_redirect` per redirect target) against the
//! caller-supplied authority and an injected [`crate::TransportResolver`],
//! records each hop, and returns canned responses — letting tests assert:
//!
//! - exact authorized destination (host, port, selected socket address);
//! - redirect authorization order;
//! - header/cookie redaction (recorded headers never leak via `Debug`);
//! - timeout / proxy / TLS policy propagation;
//! - cancellation (dropping the future performs no further hops).
//!
//! Checkpoint parity with real backends: hostname hops after the first call
//! [`NetworkAuthority::authorize_reresolution`] (checkpoint
//! [`PolicyCheckpoint::Reresolution`]) instead of
//! [`NetworkAuthority::authorize_resolved`], exactly like conforming
//! backend implementations. IP literals always use `authorize_resolved`
//! (the literal is its own fact — nothing re-resolves), and proxy-endpoint
//! DNS keeps `authorize_resolved` (conformant backends fail closed before
//! proxy DNS, so there is no backend behavior to mirror there).
//!
//! The fake performs no I/O, spawns no tasks, and honors no real clock:
//! timeouts are recorded, not elapsed.

use crate::{
    validate_binding, HttpTransport, NetworkAuthority, PolicyCheckpoint, RedirectPolicy,
    ScopedHttpRequest, ScopedHttpResponse, TransportError, TransportResolver,
};
use http::StatusCode;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use url::Url;

/// One recorded dispatch hop (a single authorized connection attempt).
#[derive(Debug, Clone)]
pub struct RecordedHop {
    /// Method (e.g. `GET`).
    pub method: String,
    /// Request URL with userinfo stripped.
    pub url: String,
    /// Request host.
    pub host: String,
    /// Effective port (explicit or scheme default).
    pub port: u16,
    /// Socket address the fake "connected" (first approved address).
    pub selected_addr: Option<IpAddr>,
    /// All approved addresses for this hop.
    pub approved_addrs: Vec<IpAddr>,
    /// Request timeout (recorded, not elapsed).
    pub request_timeout_secs: u64,
    /// Redirect policy max for this request.
    pub redirect_max: u8,
    /// Whether a proxy was configured for this request.
    pub uses_proxy: bool,
    /// Whether TLS verification was on for this request.
    pub tls_verified: bool,
    /// Whether `Authorization` was present (presence only — never the value).
    pub had_authorization: bool,
    /// Whether a `Cookie` header was present (presence only).
    pub had_cookie: bool,
    /// Checkpoint order observed for this hop (authority call sequence).
    pub checkpoint_order: Vec<PolicyCheckpoint>,
}

/// Canned response for one URL (fake returns these without I/O).
#[derive(Debug, Clone)]
pub struct CannedResponse {
    /// Status to return.
    pub status: StatusCode,
    /// Optional `Location` header for redirect simulation.
    pub location: Option<String>,
    /// Body bytes.
    pub body: Vec<u8>,
}

impl CannedResponse {
    /// 200 with body.
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: StatusCode::OK,
            location: None,
            body: body.into(),
        }
    }

    /// 3xx redirect to `location`.
    pub fn redirect(status: StatusCode, location: impl Into<String>) -> Self {
        Self {
            status,
            location: Some(location.into()),
            body: Vec::new(),
        }
    }
}

/// No-network recording fake transport.
#[derive(Default)]
pub struct RecordingFakeTransport {
    resolver: Option<Arc<dyn TransportResolver>>,
    canned: Mutex<HashMap<String, CannedResponse>>,
    hops: Mutex<Vec<RecordedHop>>,
}

impl std::fmt::Debug for RecordingFakeTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordingFakeTransport")
            .field("hops", &self.hops.lock().map(|h| h.len()).unwrap_or(0))
            .finish()
    }
}

impl RecordingFakeTransport {
    /// New fake with `resolver` for DNS facts.
    pub fn new(resolver: Arc<dyn TransportResolver>) -> Self {
        Self {
            resolver: Some(resolver),
            canned: Mutex::new(HashMap::new()),
            hops: Mutex::new(Vec::new()),
        }
    }

    /// Register a canned response for `url` (exact string match).
    pub fn with_canned(self, url: &str, response: CannedResponse) -> Self {
        self.canned
            .lock()
            .expect("canned lock")
            .insert(url.to_string(), response);
        self
    }

    /// Register via interior mutability (for shared fakes).
    pub fn add_canned(&self, url: &str, response: CannedResponse) {
        self.canned
            .lock()
            .expect("canned lock")
            .insert(url.to_string(), response);
    }

    /// All recorded hops (in order).
    pub fn hops(&self) -> Vec<RecordedHop> {
        self.hops.lock().expect("hops lock").clone()
    }

    /// Number of recorded hops.
    pub fn hop_count(&self) -> usize {
        self.hops.lock().expect("hops lock").len()
    }

    fn resolver(&self) -> Result<Arc<dyn TransportResolver>, TransportError> {
        self.resolver
            .clone()
            .ok_or_else(|| TransportError::InvalidRequest("fake has no resolver".to_string()))
    }

    fn canned_for(&self, url: &str) -> Option<CannedResponse> {
        self.canned.lock().expect("canned lock").get(url).cloned()
    }

    fn effective_port(url: &Url) -> u16 {
        url.port().unwrap_or_else(|| match url.scheme() {
            "https" => 443,
            _ => 80,
        })
    }

    fn authorize_one_hop(
        &self,
        authority: &dyn NetworkAuthority,
        request: &ScopedHttpRequest,
        url: &Url,
        hop_index: u8,
    ) -> Result<RecordedHop, TransportError> {
        let mut order = Vec::new();

        // 1. Initial URL.
        authority.authorize_initial_url(url)?;
        order.push(PolicyCheckpoint::InitialUrl);

        let host = url.host_str().ok_or_else(|| {
            TransportError::InvalidRequest("redirect target has no host".to_string())
        })?;
        let port = Self::effective_port(url);
        let is_literal: bool = host.parse::<IpAddr>().is_ok();

        // 2. Host.
        authority.authorize_host(host, Some(port), is_literal)?;
        order.push(PolicyCheckpoint::Host);

        // Hops after the first re-resolve: hostname hops then authorize via
        // `authorize_reresolution` (checkpoint `Reresolution`), mirroring
        // real backends. IP literals carry their own fact and never
        // re-resolve, so they keep `authorize_resolved` on every hop.
        let reresolves = hop_index > 0 && !is_literal;
        let dns_checkpoint = if reresolves {
            PolicyCheckpoint::Reresolution
        } else {
            PolicyCheckpoint::Dns
        };

        // 3/4. DNS + socket (literals skip resolution, still authorize socket).
        let (approved, selected) = if is_literal {
            let literal: IpAddr = host
                .parse()
                .map_err(|e| TransportError::InvalidRequest(format!("bad IP literal: {e}")))?;
            let approved = authority.authorize_resolved(host, &[literal])?;
            let binding = validate_binding(host, &[literal], &approved).map_err(|e| match e {
                TransportError::InvalidBinding { host, reason } => {
                    TransportError::denied(PolicyCheckpoint::Dns, format!("{host}: {reason}"))
                }
                other => other,
            })?;
            order.push(PolicyCheckpoint::Dns);
            let mut bound = binding;
            bound.port = port;
            authority.authorize_socket(host, bound.primary(), port)?;
            order.push(PolicyCheckpoint::Socket);
            let sel = bound.primary();
            (bound.approved, Some(sel))
        } else {
            let resolver = self.resolver()?;
            let candidates = resolver.resolve(host);
            if candidates.addresses.is_empty() {
                return Err(TransportError::ResolutionFailed {
                    host: host.to_string(),
                    reason: "no addresses".to_string(),
                });
            }
            let approved = if reresolves {
                authority.authorize_reresolution(host, &candidates.addresses)?
            } else {
                authority.authorize_resolved(host, &candidates.addresses)?
            };
            let binding =
                validate_binding(host, &candidates.addresses, &approved).map_err(|e| match e {
                    TransportError::InvalidBinding { host, reason } => {
                        TransportError::denied(dns_checkpoint, format!("{host}: {reason}"))
                    }
                    other => other,
                })?;
            order.push(dns_checkpoint);
            let mut bound = binding;
            bound.port = port;
            authority.authorize_socket(host, bound.primary(), port)?;
            order.push(PolicyCheckpoint::Socket);
            let sel = bound.primary();
            (bound.approved, Some(sel))
        };

        // 8. TLS consistency (before connect, after addressing).
        authority.check_tls_consistency(
            host,
            request.tls.sni_override.as_deref(),
            request.tls.host_override.as_deref(),
        )?;
        order.push(PolicyCheckpoint::TlsConsistency);

        // 7. Proxy (endpoint vs ultimate as distinct decisions).
        // URL-level distinction first, then the proxy endpoint's own
        // DNS/socket binding (same TOCTOU-closed path as the ultimate).
        if let Some(endpoint) = request.proxy.endpoint() {
            authority.authorize_proxy(endpoint, url)?;
            let proxy_host = endpoint.host_str().ok_or_else(|| {
                TransportError::InvalidRequest("proxy endpoint has no host".to_string())
            })?;
            let proxy_port = Self::effective_port(endpoint);
            let proxy_literal = proxy_host.parse::<IpAddr>().is_ok();
            authority.authorize_host(proxy_host, Some(proxy_port), proxy_literal)?;
            if proxy_literal {
                let literal: IpAddr = proxy_host.parse().map_err(|e| {
                    TransportError::InvalidRequest(format!("bad proxy IP literal: {e}"))
                })?;
                let approved = authority.authorize_resolved(proxy_host, &[literal])?;
                let binding =
                    validate_binding(proxy_host, &[literal], &approved).map_err(|e| match e {
                        TransportError::InvalidBinding { host, reason } => TransportError::denied(
                            PolicyCheckpoint::Proxy,
                            format!("{host}: {reason}"),
                        ),
                        other => other,
                    })?;
                authority.authorize_socket(proxy_host, binding.primary(), proxy_port)?;
            } else {
                let resolver = self.resolver()?;
                let candidates = resolver.resolve(proxy_host);
                if candidates.addresses.is_empty() {
                    return Err(TransportError::ResolutionFailed {
                        host: proxy_host.to_string(),
                        reason: "proxy: no addresses".to_string(),
                    });
                }
                let approved = authority.authorize_resolved(proxy_host, &candidates.addresses)?;
                let binding = validate_binding(proxy_host, &candidates.addresses, &approved)
                    .map_err(|e| match e {
                        TransportError::InvalidBinding { host, reason } => TransportError::denied(
                            PolicyCheckpoint::Proxy,
                            format!("{host}: {reason}"),
                        ),
                        other => other,
                    })?;
                authority.authorize_socket(proxy_host, binding.primary(), proxy_port)?;
            }
            order.push(PolicyCheckpoint::Proxy);
        }

        let hop = RecordedHop {
            method: request.method.to_string(),
            url: crate::request::redact_url_for_debug(url),
            host: host.to_string(),
            port,
            selected_addr: selected,
            approved_addrs: approved,
            request_timeout_secs: request.timeout.request_timeout.as_secs(),
            redirect_max: request.redirect.max_redirects(),
            uses_proxy: request.proxy.uses_proxy(),
            tls_verified: request.tls.is_verified(),
            had_authorization: request.headers.contains_key(http::header::AUTHORIZATION),
            had_cookie: request.headers.contains_key(http::header::COOKIE),
            checkpoint_order: order,
        };
        self.hops.lock().expect("hops lock").push(hop.clone());
        Ok(hop)
    }
}

impl HttpTransport for RecordingFakeTransport {
    async fn execute(
        &self,
        authority: &dyn NetworkAuthority,
        request: ScopedHttpRequest,
    ) -> Result<ScopedHttpResponse, TransportError> {
        // Fail-closed: request-level TLS consistency for the initial URL.
        if let Some(host) = request.host() {
            request.tls.check_consistency(host)?;
        }

        let mut current_url = request.url.clone();
        let mut history: Vec<Url> = Vec::new();
        let max = request.redirect.max_redirects();

        for hop_index in 0..=max {
            let hop = self.authorize_one_hop(authority, &request, &current_url, hop_index)?;

            let canned = self
                .canned_for(current_url.as_str())
                .unwrap_or(CannedResponse::ok(format!("fake hop {hop_index}")));

            let is_redirect = canned.status.is_redirection() && canned.location.is_some();
            if !is_redirect {
                let mut headers = http::HeaderMap::new();
                // Echo presence markers only (never secret values).
                if hop.had_authorization {
                    headers.insert("x-fake-saw-authorization", "true".parse().expect("static"));
                }
                let sni = current_url.host_str().map(str::to_string);
                return Ok(ScopedHttpResponse {
                    status: canned.status,
                    headers,
                    body: bytes::Bytes::from(canned.body),
                    final_url: current_url.clone(),
                    redirect_history: history,
                    connection: Some(crate::ConnectionInfo {
                        remote_addr: hop
                            .selected_addr
                            .map(|ip| std::net::SocketAddr::new(ip, hop.port)),
                        sni_host: sni,
                    }),
                });
            }

            // Redirect hop: resolve target, authorize, enforce policy.
            let location = canned.location.expect("redirect has location");
            let next_url = current_url.join(&location).map_err(|e| {
                TransportError::InvalidRequest(format!("bad redirect Location: {e}"))
            })?;
            authority.authorize_redirect(&current_url, &next_url)?;

            let from_host = current_url.host_str();
            let to_host = next_url.host_str();
            let followed = hop_index < max
                && (match request.redirect {
                    RedirectPolicy::None => false,
                    RedirectPolicy::SameHostOnly { .. } => request
                        .redirect
                        .allows_without_authority(from_host, to_host, hop_index),
                    RedirectPolicy::AuthorityChecked { .. } => true,
                });

            if !followed {
                // Stopped (policy or hop cap): surface the redirect response.
                return Ok(ScopedHttpResponse {
                    status: canned.status,
                    headers: http::HeaderMap::new(),
                    body: bytes::Bytes::from(canned.body),
                    final_url: current_url.clone(),
                    redirect_history: history,
                    connection: None,
                });
            }

            history.push(current_url);
            current_url = next_url;
        }

        Err(TransportError::denied(
            PolicyCheckpoint::Redirect,
            "too many redirects",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InMemoryResolver, RedirectPolicy, RequestBody};
    use http::Method;
    use std::sync::Mutex;

    struct AllowAll;

    impl NetworkAuthority for AllowAll {
        fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError> {
            crate::request::reject_url_userinfo(url)
        }
        fn authorize_host(
            &self,
            _h: &str,
            _p: Option<u16>,
            _l: bool,
        ) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_resolved(
            &self,
            _h: &str,
            c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Ok(c.to_vec())
        }
        fn authorize_socket(&self, _h: &str, _a: IpAddr, _p: u16) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_redirect(&self, _f: &Url, t: &Url) -> Result<(), TransportError> {
            self.authorize_initial_url(t)
        }
        fn authorize_proxy(&self, _p: &Url, _u: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn check_tls_consistency(
            &self,
            _h: &str,
            _s: Option<&str>,
            _o: Option<&str>,
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }

    struct DenyAll;

    impl NetworkAuthority for DenyAll {
        fn authorize_initial_url(&self, _u: &Url) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::InitialUrl,
                "deny-all",
            ))
        }
        fn authorize_host(
            &self,
            _h: &str,
            _p: Option<u16>,
            _l: bool,
        ) -> Result<(), TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::Host, "deny-all"))
        }
        fn authorize_resolved(
            &self,
            _h: &str,
            _c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::Dns, "deny-all"))
        }
        fn authorize_socket(&self, _h: &str, _a: IpAddr, _p: u16) -> Result<(), TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::Socket, "deny-all"))
        }
        fn authorize_redirect(&self, _f: &Url, _t: &Url) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::Redirect,
                "deny-all",
            ))
        }
        fn authorize_proxy(&self, _p: &Url, _u: &Url) -> Result<(), TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::Proxy, "deny-all"))
        }
        fn check_tls_consistency(
            &self,
            _h: &str,
            _s: Option<&str>,
            _o: Option<&str>,
        ) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::TlsConsistency,
                "deny-all",
            ))
        }
    }

    fn resolver() -> Arc<dyn TransportResolver> {
        Arc::new(InMemoryResolver::new().with("example.com", vec!["93.184.216.34"]))
    }

    /// Allow-all authority that records which resolution method each hop used.
    struct RecordingResolution {
        calls: Mutex<Vec<&'static str>>,
    }

    impl RecordingResolution {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.lock().expect("lock").clone()
        }
    }

    impl NetworkAuthority for RecordingResolution {
        fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError> {
            crate::request::reject_url_userinfo(url)
        }
        fn authorize_host(
            &self,
            _h: &str,
            _p: Option<u16>,
            _l: bool,
        ) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_resolved(
            &self,
            _h: &str,
            c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            self.calls.lock().expect("lock").push("resolved");
            Ok(c.to_vec())
        }
        fn authorize_reresolution(
            &self,
            _h: &str,
            c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            self.calls.lock().expect("lock").push("reresolution");
            Ok(c.to_vec())
        }
        fn authorize_socket(&self, _h: &str, _a: IpAddr, _p: u16) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_redirect(&self, _f: &Url, t: &Url) -> Result<(), TransportError> {
            self.authorize_initial_url(t)
        }
        fn authorize_proxy(&self, _p: &Url, _u: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn check_tls_consistency(
            &self,
            _h: &str,
            _s: Option<&str>,
            _o: Option<&str>,
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }

    /// Approves the first resolution but returns an empty set on
    /// re-resolution (invalid binding on later hops only).
    struct EmptyOnReresolution;

    /// Returns an empty approval set on every resolution (invalid binding
    /// on the first hop).
    struct EmptyApproval;

    impl NetworkAuthority for EmptyOnReresolution {
        fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError> {
            crate::request::reject_url_userinfo(url)
        }
        fn authorize_host(
            &self,
            _h: &str,
            _p: Option<u16>,
            _l: bool,
        ) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_resolved(
            &self,
            _h: &str,
            c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Ok(c.to_vec())
        }
        fn authorize_reresolution(
            &self,
            _h: &str,
            _c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Ok(Vec::new())
        }
        fn authorize_socket(&self, _h: &str, _a: IpAddr, _p: u16) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_redirect(&self, _f: &Url, t: &Url) -> Result<(), TransportError> {
            self.authorize_initial_url(t)
        }
        fn authorize_proxy(&self, _p: &Url, _u: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn check_tls_consistency(
            &self,
            _h: &str,
            _s: Option<&str>,
            _o: Option<&str>,
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }

    impl NetworkAuthority for EmptyApproval {
        fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError> {
            crate::request::reject_url_userinfo(url)
        }
        fn authorize_host(
            &self,
            _h: &str,
            _p: Option<u16>,
            _l: bool,
        ) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_resolved(
            &self,
            _h: &str,
            _c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Ok(Vec::new())
        }
        fn authorize_socket(&self, _h: &str, _a: IpAddr, _p: u16) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_redirect(&self, _f: &Url, t: &Url) -> Result<(), TransportError> {
            self.authorize_initial_url(t)
        }
        fn authorize_proxy(&self, _p: &Url, _u: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn check_tls_consistency(
            &self,
            _h: &str,
            _s: Option<&str>,
            _o: Option<&str>,
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn fake_records_destination_and_policies() {
        let fake = RecordingFakeTransport::new(resolver());
        let req = ScopedHttpRequest::new_with_url(Method::GET, "http://example.com/a")
            .expect("req")
            .with_timeout(crate::TimeoutPolicy::with_request_timeout(7))
            .with_body(RequestBody::empty());
        let resp = fake.execute(&AllowAll, req).await.expect("exec");
        assert_eq!(resp.status, StatusCode::OK);
        let hops = fake.hops();
        assert_eq!(hops.len(), 1);
        assert_eq!(hops[0].host, "example.com");
        assert_eq!(hops[0].request_timeout_secs, 7);
        assert_eq!(
            hops[0].checkpoint_order,
            vec![
                PolicyCheckpoint::InitialUrl,
                PolicyCheckpoint::Host,
                PolicyCheckpoint::Dns,
                PolicyCheckpoint::Socket,
                PolicyCheckpoint::TlsConsistency,
            ]
        );
    }

    #[tokio::test]
    async fn fake_denies_without_authority_approval() {
        let fake = RecordingFakeTransport::new(resolver());
        let req =
            ScopedHttpRequest::new_with_url(Method::GET, "http://example.com/a").expect("req");
        let err = fake.execute(&DenyAll, req).await.unwrap_err();
        assert!(err.is_denied());
        assert!(fake.hops().is_empty(), "denied hop must not record");
    }

    #[tokio::test]
    async fn fake_follows_same_host_redirect_in_order() {
        let r: Arc<dyn TransportResolver> =
            Arc::new(InMemoryResolver::new().with("example.com", vec!["93.184.216.34"]));
        let fake = RecordingFakeTransport::new(r)
            .with_canned(
                "http://example.com/a",
                CannedResponse::redirect(StatusCode::FOUND, "/b"),
            )
            .with_canned("http://example.com/b", CannedResponse::ok("done"));
        let req = ScopedHttpRequest::new_with_url(Method::GET, "http://example.com/a")
            .expect("req")
            .with_redirect(RedirectPolicy::SameHostOnly { max_redirects: 5 });
        let resp = fake.execute(&AllowAll, req).await.expect("exec");
        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.redirect_history.len(), 1);
        assert_eq!(fake.hop_count(), 2);
    }

    #[tokio::test]
    async fn fake_stops_cross_host_redirect() {
        let r: Arc<dyn TransportResolver> = Arc::new(
            InMemoryResolver::new()
                .with("a.example.com", vec!["93.184.216.34"])
                .with("b.example.com", vec!["93.184.216.35"]),
        );
        let fake = RecordingFakeTransport::new(r).with_canned(
            "http://a.example.com/a",
            CannedResponse::redirect(StatusCode::FOUND, "http://b.example.com/b"),
        );
        let req = ScopedHttpRequest::new_with_url(Method::GET, "http://a.example.com/a")
            .expect("req")
            .with_redirect(RedirectPolicy::SameHostOnly { max_redirects: 5 });
        let resp = fake.execute(&AllowAll, req).await.expect("exec");
        assert_eq!(resp.status, StatusCode::FOUND);
        assert_eq!(fake.hop_count(), 1, "cross-host must not dispatch");
    }

    #[tokio::test]
    async fn fake_uses_reresolution_on_later_hops() {
        let fake = RecordingFakeTransport::new(resolver()).with_canned(
            "http://example.com/a",
            CannedResponse::redirect(StatusCode::FOUND, "/b"),
        );
        let auth = RecordingResolution::new();
        let req = ScopedHttpRequest::new_with_url(Method::GET, "http://example.com/a")
            .expect("req")
            .with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 5 });
        let resp = fake.execute(&auth, req).await.expect("exec");
        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(fake.hop_count(), 2);
        assert_eq!(
            auth.calls(),
            vec!["resolved", "reresolution"],
            "first hop resolves, later hops re-resolve"
        );
        let hops = fake.hops();
        assert_eq!(
            hops[0].checkpoint_order,
            vec![
                PolicyCheckpoint::InitialUrl,
                PolicyCheckpoint::Host,
                PolicyCheckpoint::Dns,
                PolicyCheckpoint::Socket,
                PolicyCheckpoint::TlsConsistency,
            ]
        );
        assert_eq!(
            hops[1].checkpoint_order,
            vec![
                PolicyCheckpoint::InitialUrl,
                PolicyCheckpoint::Host,
                PolicyCheckpoint::Reresolution,
                PolicyCheckpoint::Socket,
                PolicyCheckpoint::TlsConsistency,
            ]
        );
    }

    #[tokio::test]
    async fn fake_maps_binding_failure_to_hop_checkpoint() {
        // Empty approval on the first hop denies at `dns`.
        let fake = RecordingFakeTransport::new(resolver());
        let req =
            ScopedHttpRequest::new_with_url(Method::GET, "http://example.com/a").expect("req");
        let err = fake.execute(&EmptyApproval, req).await.unwrap_err();
        assert!(
            matches!(
                err,
                TransportError::PolicyDenied {
                    checkpoint: PolicyCheckpoint::Dns,
                    ..
                }
            ),
            "first-hop binding failure denies at dns: {err:?}"
        );
        assert!(fake.hops().is_empty(), "denied hop must not record");

        // Empty approval on re-resolution denies at `reresolution`.
        let fake = RecordingFakeTransport::new(resolver()).with_canned(
            "http://example.com/a",
            CannedResponse::redirect(StatusCode::FOUND, "/b"),
        );
        let req = ScopedHttpRequest::new_with_url(Method::GET, "http://example.com/a")
            .expect("req")
            .with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 5 });
        let err = fake.execute(&EmptyOnReresolution, req).await.unwrap_err();
        assert!(
            matches!(
                err,
                TransportError::PolicyDenied {
                    checkpoint: PolicyCheckpoint::Reresolution,
                    ..
                }
            ),
            "later-hop binding failure denies at reresolution: {err:?}"
        );
        assert_eq!(fake.hop_count(), 1, "denied hop must not record");
    }
}
