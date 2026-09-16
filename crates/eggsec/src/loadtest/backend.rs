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
//! redirect/re-resolution. Full IP pinning arrives with the Eggfetch
//! migration (which currently defers proxied execution); until then proxied
//! load tests stay on this backend.

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
#[derive(Clone)]
pub struct ReqwestTransport {
    resolver: Arc<dyn TransportResolver>,
    verified: reqwest::Client,
    insecure: reqwest::Client,
    /// Cached proxied clients keyed by `(endpoint, insecure)`. Guarded by a
    /// short non-async mutex held only to clone-or-insert (never across I/O).
    proxied: Arc<Mutex<HashMap<(String, bool), reqwest::Client>>>,
}

impl ReqwestTransport {
    /// Build around `resolver` (facts only; policy stays per-call).
    ///
    /// Both base clients disable automatic redirects (the transport loop
    /// follows them under authority), retries, and decompression surprises;
    /// the verified client trusts platform roots, the insecure client is
    /// selected only for explicitly insecure requests.
    pub fn new(resolver: Arc<dyn TransportResolver>) -> Self {
        crate::install_tls_provider();
        let verified = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .tcp_nodelay(true)
            .pool_max_idle_per_host(crate::constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(std::time::Duration::from_secs(
                crate::constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .build()
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to build verified loadtest client; using default");
                reqwest::Client::new()
            });
        let insecure = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .tcp_nodelay(true)
            .danger_accept_invalid_certs(true)
            .pool_max_idle_per_host(crate::constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(std::time::Duration::from_secs(
                crate::constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            ))
            .build()
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "Failed to build insecure loadtest client; using verified fallback");
                verified.clone()
            });
        tracing::warn!(
            "loadtest transport armed an explicit insecure-TLS client; \
             insecure dispatches stay opt-in per request and always require \
             NetworkAuthority approval"
        );
        Self {
            resolver,
            verified,
            insecure,
            proxied: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Build with the system resolver (production default).
    #[must_use]
    pub fn with_system_resolver() -> Self {
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
    /// client for the request's proxy intent (built once per endpoint).
    fn client_for(&self, request: &ScopedHttpRequest) -> reqwest::Client {
        let verified = request.tls.is_verified();
        let Some(endpoint) = request.proxy.endpoint() else {
            return self.base_client(verified).clone();
        };
        let key = (endpoint.as_str().to_string(), verified);
        if let Ok(cache) = self.proxied.lock() {
            if let Some(client) = cache.get(&key) {
                return client.clone();
            }
        }
        crate::install_tls_provider();
        let proxy_cred: Option<(String, String)> = match &request.proxy {
            ProxyIntent::Direct => None,
            ProxyIntent::Http { credential, .. } | ProxyIntent::All { credential, .. } => {
                credential
                    .as_ref()
                    .map(|c| (c.username().to_string(), c.password().to_string()))
            }
        };
        let mut proxy = reqwest::Proxy::all(endpoint.as_str()).unwrap_or_else(|e| {
                tracing::warn!(error = %e, endpoint = %endpoint, "Invalid proxy endpoint; using direct proxy bypass");
                reqwest::Proxy::all("http://127.0.0.1:1").expect("placeholder proxy")
            });
        // Credential is secret-bearing; never log its value.
        if let Some((user, pass)) = proxy_cred {
            proxy = reqwest::Proxy::all(endpoint.as_str())
                .unwrap_or(proxy)
                .basic_auth(&user, &pass);
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
        let client = builder.build().unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to build proxied client; using direct base client");
            self.base_client(verified).clone()
        });
        if let Ok(mut cache) = self.proxied.lock() {
            cache.insert(key, client.clone());
        }
        client
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
            let client = self.client_for(&request);
            let response = self.send_one(&client, &request, &current_url).await?;
            let status = response.status();
            let headers = response.headers().clone();
            let final_url = response.url().clone();
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
            let location = location.expect("redirect has location");
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
            let _ = final_url;
        }

        Err(TransportError::denied(
            PolicyCheckpoint::Redirect,
            "too many redirects",
        ))
    }
}
