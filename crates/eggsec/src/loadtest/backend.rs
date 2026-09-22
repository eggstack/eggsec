//! Reqwest-backed [`eggsec_transport::HttpTransport`] for load testing.
//!
//! Production backend behind the transport seam (Phase D WS3). The
//! load-test core never touches Reqwest: it issues
//! [`eggsec_transport::ScopedHttpRequest`]s through [`HttpTransport`] and the
//! caller-supplied [`NetworkAuthority`]. This module owns the only
//! `reqwest::Client` contact point for load testing.
//!
//! Checkpoint order per hop mirrors the recording fake
//! (initial-URL → host → DNS/re-resolution → socket → TLS-consistency →
//! proxy → dispatch), with redirect targets additionally passing through
//! `authorize_redirect` plus the redirect-policy gate before the next hop.
//!
//! TOCTOU note: Reqwest re-resolves hostnames internally, so unlike the
//! Eggfetch adapter (pinned-IP wire URLs) this backend cannot bind the
//! connector to the exact approved address. It closes the gap as far as the
//! Reqwest API allows: every hop resolves via the injected
//! [`TransportResolver`], authorizes the full candidate set, validates the
//! binding, re-verifies the dialed address, and re-authorizes on every
//! redirect/re-resolution. Proxy peers are resolved/authorized through the
//! proxy-peer checkpoints (`authorize_proxy_resolved`/`authorize_proxy_socket`)
//! but likewise cannot be pinned under Reqwest. Direct load-test traffic
//! migrates to the Eggfetch adapter for physical pinning; proxied execution
//! migrates with the qualified proxy-pinning release.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use eggsec_transport::{
    validate_binding, HttpTransport, NetworkAuthority, PolicyCheckpoint, ProxyIntent,
    RedirectPolicy, ScopedHttpRequest, ScopedHttpResponse, SystemTransportResolver, TransportError,
    TransportResolver,
};
use url::Url;

/// `'static` scope authority for executor composition.
///
/// `ScopeAuthority` borrows its scope, which cannot be stored in the
/// `Arc<dyn NetworkAuthority>` the executor owns. This wrapper owns an
/// `Arc<Scope>` and delegates every checkpoint to a borrowed
/// `ScopeAuthority`, preserving identical policy semantics with an owned
/// (`'static`) handle.
#[derive(Debug, Clone)]
pub struct OwnedScopeAuthority {
    scope: Arc<crate::config::Scope>,
}

impl OwnedScopeAuthority {
    /// Own `scope` for `'static` authority use.
    #[must_use]
    pub fn new(scope: crate::config::Scope) -> Self {
        Self {
            scope: Arc::new(scope),
        }
    }
}

impl NetworkAuthority for OwnedScopeAuthority {
    fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope).authorize_initial_url(url)
    }
    fn authorize_host(
        &self,
        host: &str,
        port: Option<u16>,
        is_ip_literal: bool,
    ) -> Result<(), TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope).authorize_host(
            host,
            port,
            is_ip_literal,
        )
    }
    fn authorize_resolved(
        &self,
        host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope).authorize_resolved(host, candidates)
    }
    fn authorize_socket(&self, host: &str, addr: IpAddr, port: u16) -> Result<(), TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope).authorize_socket(host, addr, port)
    }
    fn authorize_redirect(&self, from: &Url, to: &Url) -> Result<(), TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope).authorize_redirect(from, to)
    }
    fn authorize_reresolution(
        &self,
        host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope)
            .authorize_reresolution(host, candidates)
    }
    fn authorize_proxy(&self, proxy_endpoint: &Url, ultimate: &Url) -> Result<(), TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope)
            .authorize_proxy(proxy_endpoint, ultimate)
    }
    fn authorize_proxy_resolved(
        &self,
        proxy_host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope)
            .authorize_proxy_resolved(proxy_host, candidates)
    }
    fn authorize_proxy_socket(
        &self,
        proxy_host: &str,
        addr: IpAddr,
        port: u16,
    ) -> Result<(), TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope)
            .authorize_proxy_socket(proxy_host, addr, port)
    }
    fn check_tls_consistency(
        &self,
        request_host: &str,
        sni_override: Option<&str>,
        host_override: Option<&str>,
    ) -> Result<(), TransportError> {
        crate::policy_bridge::ScopeAuthority::new(&self.scope).check_tls_consistency(
            request_host,
            sni_override,
            host_override,
        )
    }
}

/// Scope-aware Reqwest backend for load-test dispatch.
///
/// Transition backend: direct routes migrate to [`eggsec_transport_eggfetch`]
/// where physically pinned; proxied routes stay here until the Eggfetch
/// proxy migration qualifies. All construction is fail-closed: client or
/// proxy build failures return [`TransportError`] and never broaden
/// redirect/proxy/TLS/DNS policy.
#[derive(Clone)]
pub struct ReqwestTransport {
    resolver: Arc<dyn TransportResolver>,
    verified: reqwest::Client,
    insecure: reqwest::Client,
    /// Cached proxied clients keyed by full connection/client identity.
    /// Guarded by a short non-async mutex held only to clone-or-insert
    /// (never across I/O). Credentials are never logged: the key carries
    /// only a fingerprint (SHA-256), never plaintext.
    proxied: Arc<Mutex<HashMap<ProxiedClientKey, reqwest::Client>>>,
}

/// Proxied-client cache identity.
///
/// Every connection/client-affecting dimension participates: proxy endpoint,
/// routing mode, TLS verification, and proxy credential identity
/// (fingerprint only). Keying by endpoint + TLS alone would let request B
/// inherit request A's proxy credential.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProxiedClientKey {
    endpoint: String,
    verified: bool,
    /// 0 = `ProxyIntent::Http`, 1 = `ProxyIntent::All`.
    mode: u8,
    /// SHA-256 over `username\x00password` when a credential is present.
    credential_fp: Option<[u8; 32]>,
}

fn credential_fingerprint(cred: Option<&eggsec_transport::ProxyCredential>) -> Option<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let c = cred?;
    let mut hasher = Sha256::new();
    hasher.update(c.username().as_bytes());
    hasher.update([0u8]);
    hasher.update(c.password().as_bytes());
    let out = hasher.finalize();
    let mut fp = [0u8; 32];
    fp.copy_from_slice(&out);
    Some(fp)
}

impl ReqwestTransport {
    /// Build around `resolver` (facts only; policy stays per-call).
    ///
    /// Both base clients disable automatic redirects (the transport loop
    /// follows them under authority), retries, and decompression surprises;
    /// the verified client trusts platform roots, the insecure client is
    /// selected only for explicitly insecure requests.
    ///
    /// Fail-closed: verified or insecure client build failures return an
    /// error. No `reqwest::Client::new()` default and no verified/insecure
    /// cross-fallback are permitted: construction failure must never broaden
    /// redirect, TLS, or pool policy.
    pub fn new(resolver: Arc<dyn TransportResolver>) -> Result<Self, TransportError> {
        crate::install_tls_provider();
        let verified = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .tcp_nodelay(true)
            .pool_max_idle_per_host(crate::constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(std::time::Duration::from_secs(
                crate::constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .build()
            .map_err(|e| {
                TransportError::Backend(format!("failed to build verified loadtest client: {e}"))
            })?;
        let insecure = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .tcp_nodelay(true)
            .danger_accept_invalid_certs(true)
            .pool_max_idle_per_host(crate::constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(std::time::Duration::from_secs(
                crate::constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .build()
            .map_err(|e| {
                TransportError::Backend(format!("failed to build insecure loadtest client: {e}"))
            })?;
        tracing::warn!(
            "loadtest transport armed an explicit insecure-TLS client; \
             insecure dispatches stay opt-in per request and always require \
             NetworkAuthority approval"
        );
        Ok(Self {
            resolver,
            verified,
            insecure,
            proxied: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Build with the system resolver (production default).
    pub fn with_system_resolver() -> Result<Self, TransportError> {
        Self::new(Arc::new(SystemTransportResolver))
    }

    fn base_client(&self, verified: bool) -> &reqwest::Client {
        if verified {
            &self.verified
        } else {
            &self.insecure
        }
    }

    /// Client for this request: direct base client, or a cached proxied
    /// client for the request's proxy intent.
    ///
    /// Fail-closed: invalid proxy endpoints and proxy-client build failures
    /// return [`TransportError`] before origin I/O. A requested proxy never
    /// falls back to a direct base client, and no `127.0.0.1:1` placeholder
    /// proxy is substituted. Proxy credentials are secret-bearing and never
    /// logged; the cache key carries only a fingerprint.
    fn client_for(&self, request: &ScopedHttpRequest) -> Result<reqwest::Client, TransportError> {
        let verified = request.tls.is_verified();
        let Some(endpoint) = request.proxy.endpoint() else {
            return Ok(self.base_client(verified).clone());
        };
        let (mode, credential) = match &request.proxy {
            ProxyIntent::Direct => (0u8, None),
            ProxyIntent::Http { credential, .. } => (0u8, credential.as_ref()),
            ProxyIntent::All { credential, .. } => (1u8, credential.as_ref()),
        };
        let key = ProxiedClientKey {
            endpoint: endpoint.as_str().to_string(),
            verified,
            mode,
            credential_fp: credential_fingerprint(credential),
        };
        if let Ok(cache) = self.proxied.lock() {
            if let Some(client) = cache.get(&key) {
                return Ok(client.clone());
            }
        }
        crate::install_tls_provider();
        // Validate the endpoint before any I/O: an invalid proxy URL fails
        // here, never as a direct origin request. Only http/https/socks5(+h)
        // proxy schemes are supported; anything else fails closed (no
        // placeholder, no direct fallback).
        let endpoint_str = endpoint.as_str();
        match endpoint.scheme() {
            "http" | "https" | "socks5" | "socks5h" => {}
            other => {
                return Err(TransportError::InvalidRequest(format!(
                    "unsupported proxy scheme '{other}' (expected http/https/socks5/socks5h)"
                )));
            }
        }
        let mut proxy = reqwest::Proxy::all(endpoint_str).map_err(|e| {
            TransportError::InvalidRequest(format!("invalid proxy endpoint '{endpoint_str}': {e}"))
        })?;
        // Credential is secret-bearing; never log its value.
        if let Some(cred) = credential {
            // Re-validate the endpoint for the authenticated builder so a
            // credential cannot mask an invalid endpoint via fallback.
            let authed = reqwest::Proxy::all(endpoint_str).map_err(|e| {
                TransportError::InvalidRequest(format!(
                    "invalid proxy endpoint '{endpoint_str}': {e}"
                ))
            })?;
            proxy = authed.basic_auth(cred.username(), cred.password());
        }
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .tcp_nodelay(true)
            .proxy(proxy)
            .pool_max_idle_per_host(crate::constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(std::time::Duration::from_secs(
                crate::constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ));
        if !verified {
            builder = builder.danger_accept_invalid_certs(true);
        }
        let client = builder.build().map_err(|e| {
            TransportError::Backend(format!("failed to build proxied loadtest client: {e}"))
        })?;
        if let Ok(mut cache) = self.proxied.lock() {
            cache.insert(key, client.clone());
        }
        Ok(client)
    }

    fn authorize_hop(
        &self,
        authority: &dyn NetworkAuthority,
        request: &ScopedHttpRequest,
        url: &Url,
        hop_index: u8,
    ) -> Result<(), TransportError> {
        authority.authorize_initial_url(url)?;
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(TransportError::InvalidRequest(format!(
                "unsupported URL scheme '{scheme}' (expected http/https)"
            )));
        }
        let host = url
            .host_str()
            .ok_or_else(|| TransportError::InvalidRequest("request URL has no host".to_string()))?;
        let port = url.port_or_known_default().ok_or_else(|| {
            TransportError::InvalidRequest("request URL has no usable port".to_string())
        })?;
        let is_literal = host.parse::<IpAddr>().is_ok();
        authority.authorize_host(host, Some(port), is_literal)?;

        if is_literal {
            let addr: IpAddr = host
                .parse()
                .map_err(|_| TransportError::denied(PolicyCheckpoint::Host, "bad IP literal"))?;
            let approved = authority.authorize_resolved(host, &[addr])?;
            let binding = validate_binding(host, &[addr], &approved).map_err(|e| match e {
                TransportError::InvalidBinding { host, reason } => {
                    TransportError::denied(PolicyCheckpoint::Dns, format!("{host}: {reason}"))
                }
                other => other,
            })?;
            authority.authorize_socket(host, binding.primary(), port)?;
        } else {
            let resolved = self.resolver.resolve(host);
            if resolved.addresses.is_empty() {
                return Err(TransportError::ResolutionFailed {
                    host: host.to_string(),
                    reason: "no addresses".to_string(),
                });
            }
            let checkpoint = if hop_index == 0 {
                PolicyCheckpoint::Dns
            } else {
                PolicyCheckpoint::Reresolution
            };
            let approved = if hop_index == 0 {
                authority.authorize_resolved(host, &resolved.addresses)?
            } else {
                authority.authorize_reresolution(host, &resolved.addresses)?
            };
            let binding =
                validate_binding(host, &resolved.addresses, &approved).map_err(|e| match e {
                    TransportError::InvalidBinding { host, reason } => {
                        TransportError::denied(checkpoint, format!("{host}: {reason}"))
                    }
                    other => other,
                })?;
            authority.authorize_socket(host, binding.primary(), port)?;
        }

        authority.check_tls_consistency(
            host,
            request.tls.sni_override.as_deref(),
            request.tls.host_override.as_deref(),
        )?;
        if let Some(endpoint) = request.proxy.endpoint() {
            authority.authorize_proxy(endpoint, url)?;
            // Proxy-peer binding through the Eggsec resolver/authority path.
            // Reqwest still resolves the proxy internally (TOCTOU documented
            // at the module head), so this authorizes the peer set without
            // claiming physical pinning — full pinning arrives with the
            // Eggfetch proxy migration.
            let proxy_host = endpoint.host_str().ok_or_else(|| {
                TransportError::InvalidRequest("proxy endpoint has no host".to_string())
            })?;
            let proxy_port = endpoint.port_or_known_default().ok_or_else(|| {
                TransportError::InvalidRequest("proxy endpoint has no usable port".to_string())
            })?;
            let proxy_literal = proxy_host.parse::<IpAddr>().is_ok();
            authority.authorize_host(proxy_host, Some(proxy_port), proxy_literal)?;
            if proxy_literal {
                let literal: IpAddr = proxy_host.parse().map_err(|_| {
                    TransportError::denied(PolicyCheckpoint::Proxy, "bad proxy IP literal")
                })?;
                let approved = authority.authorize_proxy_resolved(proxy_host, &[literal])?;
                let binding =
                    validate_binding(proxy_host, &[literal], &approved).map_err(|e| match e {
                        TransportError::InvalidBinding { host, reason } => TransportError::denied(
                            PolicyCheckpoint::Proxy,
                            format!("{host}: {reason}"),
                        ),
                        other => other,
                    })?;
                authority.authorize_proxy_socket(proxy_host, binding.primary(), proxy_port)?;
            } else {
                let resolved = self.resolver.resolve(proxy_host);
                if resolved.addresses.is_empty() {
                    return Err(TransportError::ResolutionFailed {
                        host: proxy_host.to_string(),
                        reason: "proxy: no addresses".to_string(),
                    });
                }
                let approved =
                    authority.authorize_proxy_resolved(proxy_host, &resolved.addresses)?;
                let binding = validate_binding(proxy_host, &resolved.addresses, &approved)
                    .map_err(|e| match e {
                        TransportError::InvalidBinding { host, reason } => TransportError::denied(
                            PolicyCheckpoint::Proxy,
                            format!("{host}: {reason}"),
                        ),
                        other => other,
                    })?;
                authority.authorize_proxy_socket(proxy_host, binding.primary(), proxy_port)?;
            }
        }
        Ok(())
    }

    async fn send_one(
        &self,
        client: &reqwest::Client,
        request: &ScopedHttpRequest,
        url: &Url,
    ) -> Result<reqwest::Response, TransportError> {
        let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|e| TransportError::InvalidRequest(format!("bad method: {e}")))?;
        let mut builder = client
            .request(method, url.clone())
            .timeout(request.timeout.request_timeout);
        for (name, value) in request.headers.iter() {
            if name.as_str().eq_ignore_ascii_case("host") {
                continue;
            }
            let value_str = value.to_str().map_err(|_| {
                TransportError::InvalidRequest(format!(
                    "header '{}' value is not valid UTF-8",
                    name.as_str()
                ))
            })?;
            builder = builder.header(name, value_str);
        }
        let user_agent_name: eggsec_transport::HeaderName = "user-agent"
            .parse()
            .map_err(|e| TransportError::InvalidRequest(format!("bad header name: {e}")))?;
        if !request.headers.contains_key(&user_agent_name) {
            if let Some(ref ua) = request.hints.user_agent {
                builder = builder.header("user-agent", ua.as_str());
            }
        }
        builder = match &request.body {
            eggsec_transport::RequestBody::Empty => builder,
            eggsec_transport::RequestBody::Bytes(b) => builder.body(b.clone()),
        };
        builder.send().await.map_err(map_reqwest_error)
    }
}

fn map_reqwest_error(e: reqwest::Error) -> TransportError {
    // Stable, classifiable message prefixes (see
    // `LoadTestErrorKind::classify_backend_message`). No header/secret
    // material is included: reqwest URL display strips userinfo by
    // construction and our URLs never carry it (rejected at plan time).
    if e.is_timeout() {
        TransportError::Backend(format!("request timed out: {e}"))
    } else if e.is_connect() {
        TransportError::Backend(format!("connection failed: {e}"))
    } else if e.is_body() || e.is_decode() {
        TransportError::Backend(format!("response body failed: {e}"))
    } else if e.is_builder() {
        TransportError::InvalidRequest(format!("invalid request: {e}"))
    } else if let Some(status) = e.status() {
        TransportError::Backend(format!("request failed with status {status}: {e}"))
    } else {
        TransportError::Backend(format!("transport failure: {e}"))
    }
}

impl HttpTransport for ReqwestTransport {
    async fn execute(
        &self,
        authority: &dyn NetworkAuthority,
        request: ScopedHttpRequest,
    ) -> Result<ScopedHttpResponse, TransportError> {
        if let Some(host) = request.host() {
            request.tls.check_consistency(host)?;
        }
        let mut current_url = request.url.clone();
        let mut history: Vec<Url> = Vec::new();
        let max = request.redirect.max_redirects();
        let start = Instant::now();

        for hop_index in 0..=max {
            self.authorize_hop(authority, &request, &current_url, hop_index)?;

            let remaining = request
                .timeout
                .request_timeout
                .saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return Err(TransportError::Backend(format!(
                    "request timeout of {}s exhausted before hop {hop_index}",
                    request.timeout.request_timeout.as_secs()
                )));
            }
            let client = self.client_for(&request)?;
            let response = self.send_one(&client, &request, &current_url).await?;
            let status = response.status();
            let headers = response.headers().clone();
            let remote = response.remote_addr();
            // Drain the body for connection reuse (parity with the
            // pre-migration runner, which consumed every body).
            let body = response.bytes().await.map_err(map_reqwest_error)?;

            let location = headers
                .get("location")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let is_redirect = status.is_redirection() && location.is_some();
            if !is_redirect {
                return Ok(ScopedHttpResponse {
                    status,
                    headers,
                    body,
                    final_url: current_url.clone(),
                    redirect_history: history,
                    connection: Some(eggsec_transport::ConnectionInfo {
                        remote_addr: remote,
                        sni_host: current_url.host_str().map(str::to_string),
                    }),
                });
            }
            let Some(location) = location else {
                return Err(TransportError::InvalidRequest(
                    "redirect status without Location header".to_string(),
                ));
            };
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
                return Ok(ScopedHttpResponse {
                    status,
                    headers,
                    body,
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
    use eggsec_transport::{ProxyCredential, ProxyIntent};
    use std::sync::Arc;

    fn test_transport() -> ReqwestTransport {
        ReqwestTransport::new(Arc::new(SystemTransportResolver)).expect("transport builds")
    }

    fn proxied_request(
        endpoint: &str,
        username: &str,
        password: &str,
        verified: bool,
    ) -> ScopedHttpRequest {
        use eggsec_transport::{Method, TlsPolicy};
        let url = Url::parse("http://example.com/").expect("url");
        let endpoint_url = Url::parse(endpoint).expect("proxy endpoint");
        let cred = ProxyCredential::new(username, password);
        let intent = ProxyIntent::All {
            endpoint: endpoint_url,
            credential: Some(cred),
        };
        let mut req = ScopedHttpRequest::new(Method::GET, url).expect("req");
        req = req.with_proxy(intent);
        req = req.with_tls(if verified {
            TlsPolicy::verified()
        } else {
            TlsPolicy::insecure()
        });
        req
    }

    #[test]
    fn construction_has_no_policy_changing_fallback() {
        // Verified/insecure construction succeeds without silent fallback.
        // The absence of `Client::new()` / verified-fallback is enforced by guard.
        let _t1 = test_transport();
        let _t2 = ReqwestTransport::with_system_resolver().expect("system transport builds");
    }

    #[test]
    fn invalid_proxy_endpoint_fails_before_origin_io() {
        let transport = test_transport();
        use eggsec_transport::Method;
        let url = Url::parse("http://example.com/").expect("url");
        // Unsupported proxy scheme parses as URL but reqwest rejects it;
        // transport must error, never direct, never placeholder.
        let bad_endpoint = Url::parse("gopher://127.0.0.1:8080").expect("url parses");
        let intent = ProxyIntent::All {
            endpoint: bad_endpoint,
            credential: None,
        };
        let mut req = ScopedHttpRequest::new(Method::GET, url).expect("req");
        req = req.with_proxy(intent);
        let err = transport.client_for(&req).unwrap_err();
        assert!(
            !format!("{err:?}").contains("127.0.0.1:1"),
            "no placeholder proxy substitution: {err:?}"
        );
    }

    #[test]
    fn proxy_credentials_are_cache_isolated() {
        let transport = test_transport();
        let req_a = proxied_request("http://127.0.0.1:8080", "alice", "pw-a", true);
        let req_b = proxied_request("http://127.0.0.1:8080", "bob", "pw-b", true);
        let _ca = transport.client_for(&req_a).expect("client A builds");
        let _cb = transport.client_for(&req_b).expect("client B builds");
        let cache = transport.proxied.lock().expect("cache lock");
        assert_eq!(
            cache.len(),
            2,
            "two credential sets for one endpoint must not share a cached client"
        );
    }

    #[test]
    fn tls_mode_partitions_proxy_cache() {
        let transport = test_transport();
        let req_verified = proxied_request("http://127.0.0.1:8080", "u", "p", true);
        let req_insecure = proxied_request("http://127.0.0.1:8080", "u", "p", false);
        transport
            .client_for(&req_verified)
            .expect("verified builds");
        transport
            .client_for(&req_insecure)
            .expect("insecure builds");
        let cache = transport.proxied.lock().expect("cache lock");
        assert_eq!(cache.len(), 2, "TLS mode must partition the proxy cache");
    }

    #[test]
    fn credential_fingerprint_never_exposes_secret() {
        let fp_a =
            credential_fingerprint(Some(&ProxyCredential::new("alice", "pw-a"))).expect("fp");
        let fp_b = credential_fingerprint(Some(&ProxyCredential::new("bob", "pw-b"))).expect("fp");
        assert_ne!(fp_a, fp_b);
        let dbg = format!(
            "{:?}",
            ProxiedClientKey {
                endpoint: "http://127.0.0.1:8080".to_string(),
                verified: true,
                mode: 1,
                credential_fp: Some(fp_a),
            }
        );
        assert!(
            !dbg.contains("alice"),
            "credential leak in key Debug: {dbg}"
        );
        assert!(!dbg.contains("pw-a"), "credential leak in key Debug: {dbg}");
        assert!(credential_fingerprint(None).is_none());
    }
}
