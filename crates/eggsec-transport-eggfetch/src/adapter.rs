//! [`EggfetchTransport`]: scope-aware [`eggsec_transport::HttpTransport`]
//! over `eggfetch-core` public APIs.
//!
//! Checkpoint order per hop mirrors the recording fake exactly
//! (initial-URL → host → DNS/re-resolution → socket → TLS-consistency →
//! proxy → dispatch), with redirect targets additionally passing through
//! `authorize_redirect` plus the redirect-policy gate before the next hop.

use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use eggfetch_core::{
    redirect::{build_redirect_request, is_redirect_status},
    Client, HttpVersionPolicy, TlsConfig, TrustStore,
};
use eggsec_transport::{
    validate_binding, HttpTransport, NetworkAuthority, PolicyCheckpoint, RedirectPolicy,
    ScopedHttpRequest, ScopedHttpResponse, TransportError, TransportResolver,
};
use url::Url;

use crate::mapping::{
    effective_port, eggfetch_body, eggfetch_timeout, ensure_http_scheme, extract_redirect_body,
    host_header_value, ip_literal, map_backend_error, map_redirect_build_error, pin_wire_url,
    same_origin, LogicalHop,
};

/// Scope-aware outbound HTTP transport over `eggfetch-core`.
///
/// Holds two preconfigured backend clients (verified TLS and explicit
/// insecure TLS) plus the caller-supplied DNS resolver. DNS facts always
/// come from that resolver — never from the backend's own lookup — and the
/// wire URL is pinned to the approved IP literal before dispatch, so the
/// backend connector can only dial the authorized address.
///
/// No production EggSec consumer uses this type yet (Phase D migrates
/// consumers one at a time).
pub struct EggfetchTransport {
    resolver: Arc<dyn TransportResolver>,
    verified: Client,
    insecure: Client,
}

impl EggfetchTransport {
    /// Build the adapter around `resolver` (facts only; policy stays with
    /// the per-call [`NetworkAuthority`]).
    ///
    /// Both backend clients pin HTTP/1.1, disable automatic redirects,
    /// retries, and decompression. The verified client trusts
    /// Mozilla/WebPKI roots only (parity with the current Reqwest
    /// default); the insecure client disables certificate **and**
    /// hostname verification and is only selected for requests whose
    /// [`eggsec_transport::TlsPolicy`] is explicitly insecure.
    pub fn new(resolver: Arc<dyn TransportResolver>) -> Self {
        let verified = Client::builder()
            .http_version_policy(HttpVersionPolicy::Http1Only)
            .follow_redirects(false)
            .max_redirects(0)
            .automatic_decompression(false)
            .tls_config(
                TlsConfig::builder()
                    .trust_store(TrustStore::WebPkiOnly)
                    .build(),
            )
            .build();
        let insecure = Client::builder()
            .http_version_policy(HttpVersionPolicy::Http1Only)
            .follow_redirects(false)
            .max_redirects(0)
            .automatic_decompression(false)
            .tls_config(
                TlsConfig::builder()
                    .danger_accept_invalid_certs(true)
                    .build(),
            )
            .build();
        tracing::warn!(
            "eggfetch transport armed an explicit insecure-TLS client; \
             insecure dispatches stay opt-in per request and always require \
             NetworkAuthority approval"
        );
        Self {
            resolver,
            verified,
            insecure,
        }
    }

    fn backend_for(&self, request: &ScopedHttpRequest) -> &Client {
        if request.tls.is_verified() {
            &self.verified
        } else {
            &self.insecure
        }
    }
}

impl std::fmt::Debug for EggfetchTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        // No resolver/client internals and no request state: nothing
        // secret-adjacent ever lives on this type (headers/credentials are
        // per-request values, never stored here).
        f.debug_struct("EggfetchTransport").finish_non_exhaustive()
    }
}

/// Authorized single-hop dispatch plan: the logical destination plus the
/// pinned wire URL the backend is allowed to see.
struct AuthorizedHop {
    logical_host: String,
    port: u16,
    primary: IpAddr,
    wire_url: Url,
    is_https: bool,
}

impl EggfetchTransport {
    /// Run the full checkpoint sequence for one logical URL and bind the
    /// approved address to the wire URL.
    fn authorize_hop(
        &self,
        authority: &dyn NetworkAuthority,
        request: &ScopedHttpRequest,
        url: &Url,
        hop_index: u8,
    ) -> Result<AuthorizedHop, TransportError> {
        // 1. Initial URL.
        authority.authorize_initial_url(url)?;
        ensure_http_scheme(url)?;

        let host = url
            .host_str()
            .ok_or_else(|| TransportError::InvalidRequest("request URL has no host".to_string()))?;
        let port = effective_port(url)?;
        let literal = ip_literal(url);
        let is_literal = literal.is_some();

        // 2. Host (pattern + port pre-check; literals get full CIDR check).
        authority.authorize_host(host, Some(port), is_literal)?;

        // 3/6. DNS facts + authority approval. Literals carry their own
        // fact (mirroring the fake); hostname hops resolve fresh every
        // time so re-resolution re-invokes authorization.
        let (candidates, approved) = match literal {
            Some(addr) => {
                let approved = authority.authorize_resolved(host, &[addr])?;
                (vec![addr], approved)
            }
            None => {
                let resolved = self.resolver.resolve(host);
                if resolved.addresses.is_empty() {
                    return Err(TransportError::ResolutionFailed {
                        host: host.to_string(),
                        reason: "no addresses".to_string(),
                    });
                }
                let approved = if hop_index == 0 {
                    authority.authorize_resolved(host, &resolved.addresses)?
                } else {
                    // Checkpoint 6: re-resolution delegates to
                    // `authorize_resolved` by default, but custom
                    // authorities may audit retries/redirects distinctly.
                    authority.authorize_reresolution(host, &resolved.addresses)?
                };
                (resolved.addresses, approved)
            }
        };
        let dns_checkpoint = if hop_index == 0 {
            PolicyCheckpoint::Dns
        } else {
            PolicyCheckpoint::Reresolution
        };
        let mut binding = validate_binding(host, &candidates, &approved).map_err(|e| match e {
            TransportError::InvalidBinding { host, reason } => {
                TransportError::denied(dns_checkpoint, format!("{host}: {reason}"))
            }
            other => other,
        })?;
        binding.port = port;

        // 4. Socket: the address actually dialed, re-verified pre-connect.
        authority.authorize_socket(host, binding.primary(), port)?;

        // 8 (contract order: TLS consistency before proxy). The
        // request-level check runs once up front in `execute`; the
        // authority check runs per hop so override smuggling across
        // redirects fails closed.
        authority.check_tls_consistency(
            host,
            request.tls.sni_override.as_deref(),
            request.tls.host_override.as_deref(),
        )?;

        // 7. Proxy endpoint vs ultimate destination are distinct decisions.
        // The adapter observes the authority verdict, then fails closed:
        // proxied execution (including remote-DNS SOCKS semantics, where
        // the local process never sees the target IP) is deferred past
        // Phase C, and the `proxy` feature is not compiled in, so no
        // weaker path exists underneath.
        if let Some(endpoint) = request.proxy.endpoint() {
            authority.authorize_proxy(endpoint, url)?;
            return Err(TransportError::denied(
                PolicyCheckpoint::Proxy,
                "proxied execution is deferred in Phase C (direct-only adapter)",
            ));
        }

        // Bind: the connector only ever sees the approved IP literal.
        let wire_url = match literal {
            Some(_) => url.clone(),
            None => pin_wire_url(url, binding.primary())?,
        };
        Ok(AuthorizedHop {
            logical_host: host.to_string(),
            port,
            primary: binding.primary(),
            wire_url,
            is_https: url.scheme() == "https",
        })
    }

    /// Dispatch one authorized hop through the backend.
    async fn send_hop(
        &self,
        request: &ScopedHttpRequest,
        logical: &LogicalHop,
        hop: &AuthorizedHop,
        remaining_total: std::time::Duration,
    ) -> Result<eggfetch_core::Response, TransportError> {
        let mut wire_headers = logical.headers.clone();
        // The adapter owns `Host`: logical host (+ non-default port),
        // never the pinned literal and never a caller-smuggled value.
        let host_value = host_header_value(&logical.url)?;
        wire_headers.insert(
            http::header::HOST,
            host_value
                .parse()
                .map_err(|e| TransportError::InvalidRequest(format!("bad Host value: {e}")))?,
        );
        if let Some(ref ua) = request.hints.user_agent {
            if !wire_headers.contains_key(http::header::USER_AGENT) {
                wire_headers.insert(
                    http::header::USER_AGENT,
                    ua.parse().map_err(|e| {
                        TransportError::InvalidRequest(format!("bad User-Agent value: {e}"))
                    })?,
                );
            }
        }

        let mut builder = self
            .backend_for(request)
            .request(logical.method.clone(), hop.wire_url.as_str())
            .map_err(map_redirect_build_error)?;
        for (name, value) in wire_headers.iter() {
            let value_str = value.to_str().map_err(|_| {
                TransportError::InvalidRequest(format!(
                    "header '{}' value is not valid UTF-8",
                    name.as_str()
                ))
            })?;
            builder = builder.header(name.as_str(), value_str);
        }
        let sni = hop.is_https.then(|| hop.logical_host.clone());
        let response = builder
            .body(eggfetch_body(&logical.body))
            .timeout(eggfetch_timeout(&request.timeout, remaining_total))
            .redirect_policy(eggfetch_core::RedirectPolicy::new(false, 0))
            .without_retry()
            // No proxy is ever configured on the backend clients; this
            // per-request override additionally pins direct routing so
            // environment-style proxy selection cannot divert a hop.
            .without_proxy()
            .decompress(false)
            .transport_hints(eggfetch_core::TransportHints {
                target: None,
                sni_hostname: sni,
                trace: None,
            })
            .send()
            .await
            .map_err(map_backend_error)?;
        Ok(response)
    }

    /// Compute the next logical hop with Eggfetch's public redirect
    /// primitive (method rewrite, sensitive-header stripping, and
    /// body-replay rules stay owned upstream), then restore
    /// same-origin credentials and drop the adapter-owned `Host`.
    fn advance_logical(
        &self,
        logical: &LogicalHop,
        status: http::StatusCode,
        location: &str,
    ) -> Result<LogicalHop, TransportError> {
        let egg_headers = eggfetch_core::Headers::from(logical.headers.clone());
        let egg_request = self
            .verified
            .request(logical.method.clone(), logical.url.as_str())
            .map_err(map_redirect_build_error)?
            .headers(egg_headers)
            .body(eggfetch_body(&logical.body))
            .build()
            .map_err(map_redirect_build_error)?;
        let next = build_redirect_request(&egg_request, status, location)
            .map_err(map_redirect_build_error)?;
        let next_url = next.url().clone();
        let mut next_headers = next.headers().clone().into_inner();
        // Eggfetch strips `authorization`/`proxy-authorization` on every
        // hop and re-applies client-configured auth per hop. The adapter
        // carries auth in per-request headers (no client auth is
        // configured), so same-origin hops restore them explicitly —
        // matching Reqwest same-host semantics — while cross-origin hops
        // keep the stripped state.
        if same_origin(&logical.url, &next_url) {
            for name in ["authorization", "proxy-authorization"] {
                let header_name: http::HeaderName = name
                    .parse()
                    .map_err(|e| TransportError::InvalidRequest(format!("bad header name: {e}")))?;
                if let Some(value) = logical.headers.get(&header_name) {
                    next_headers.insert(header_name, value.clone());
                }
            }
        }
        next_headers.remove(http::header::HOST);
        let next_body = extract_redirect_body(next.body())?;
        Ok(LogicalHop {
            method: next.method().clone(),
            url: next_url,
            headers: next_headers,
            body: next_body,
        })
    }

    fn map_response(
        status: http::StatusCode,
        headers: http::HeaderMap,
        body: Bytes,
        final_url: Url,
        history: Vec<Url>,
        hop: &AuthorizedHop,
    ) -> ScopedHttpResponse {
        ScopedHttpResponse {
            status,
            headers,
            body,
            final_url,
            redirect_history: history,
            connection: Some(eggsec_transport::ConnectionInfo {
                remote_addr: Some(SocketAddr::new(hop.primary, hop.port)),
                sni_host: hop.is_https.then(|| hop.logical_host.clone()),
            }),
        }
    }
}

impl HttpTransport for EggfetchTransport {
    async fn execute(
        &self,
        authority: &dyn NetworkAuthority,
        request: ScopedHttpRequest,
    ) -> Result<ScopedHttpResponse, TransportError> {
        // Fail-closed: request-level TLS consistency for the initial URL.
        if let Some(host) = request.host() {
            request.tls.check_consistency(host)?;
        }

        let mut logical = LogicalHop {
            method: request.method.clone(),
            url: request.url.clone(),
            headers: request.headers.clone(),
            body: match &request.body {
                eggsec_transport::RequestBody::Empty => Bytes::new(),
                eggsec_transport::RequestBody::Bytes(b) => b.clone(),
            },
        };
        // The adapter owns `Host` end to end; drop any caller-supplied
        // value before the first hop (each hop re-sets it from the
        // logical URL).
        logical.headers.remove(http::header::HOST);

        let mut history: Vec<Url> = Vec::new();
        let max = request.redirect.max_redirects();
        let start = Instant::now();

        for hop_index in 0..=max {
            let hop = self.authorize_hop(authority, &request, &logical.url, hop_index)?;

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
            let mut response = self.send_hop(&request, &logical, &hop, remaining).await?;
            let status = response.status();
            let headers = response.headers().clone();
            let location = headers
                .get(http::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let body = response.bytes().await.map_err(map_backend_error)?;

            // Only statuses Eggfetch itself would follow count as
            // redirects (300/304/305 and friends are surfaced verbatim —
            // never followed into a weaker policy path).
            let Some(location) = location else {
                return Ok(Self::map_response(
                    status,
                    headers,
                    body,
                    logical.url,
                    history,
                    &hop,
                ));
            };
            if !is_redirect_status(status) {
                return Ok(Self::map_response(
                    status,
                    headers,
                    body,
                    logical.url,
                    history,
                    &hop,
                ));
            }
            let next = self.advance_logical(&logical, status, &location)?;

            // Every redirect target is authorized before dispatch, in
            // addition to the full per-hop sequence on the next loop
            // iteration (mirrors the fake: `authorize_redirect` now, full
            // `authorize_hop` next).
            authority.authorize_redirect(&logical.url, &next.url)?;

            let from_host = logical.url.host_str();
            let to_host = next.url.host_str();
            let followed = hop_index < max
                && (match request.redirect {
                    RedirectPolicy::None => false,
                    RedirectPolicy::SameHostOnly { .. } => request
                        .redirect
                        .allows_without_authority(from_host, to_host, hop_index),
                    RedirectPolicy::AuthorityChecked { .. } => true,
                });
            if !followed {
                // Stopped (policy or hop cap): surface the redirect
                // response itself, exactly like the fake.
                return Ok(Self::map_response(
                    status,
                    headers,
                    body,
                    logical.url,
                    history,
                    &hop,
                ));
            }

            history.push(logical.url);
            logical = next;
        }

        Err(TransportError::Backend(
            "redirect loop exhausted without surfacing a response".to_string(),
        ))
    }
}
