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
    host_header_value, ip_literal, map_backend_error, map_redirect_build_error, same_origin,
    LogicalHop,
};

/// Scope-aware outbound HTTP transport over `eggfetch-core`.
///
/// Holds two preconfigured backend clients (verified TLS and explicit
/// insecure TLS) plus the caller-supplied DNS resolver. DNS facts always
/// come from that resolver — never from the backend's own lookup — and each
/// direct hop dispatches the logical URL with a singular socket-authorized
/// `resolved_addresses` pin, so the backend connector can only dial the
/// authorized address (no origin DNS inside Eggfetch).
///
/// Production direct load-test traffic uses this transport (first production
/// backend); supported proxied load testing uses it where the route matrix
/// allows, otherwise it fails closed.
pub struct EggfetchTransport {
    resolver: Arc<dyn TransportResolver>,
    verified: Client,
    insecure: Client,
}

impl EggfetchTransport {
    /// Build the adapter around `resolver` (facts only; policy stays with
    /// the per-call [`NetworkAuthority`]).
    ///
    /// Both backend clients negotiate H1/H2 via ALPN
    /// (`HttpVersionPolicy::Auto { allow_http3: false }`), disable automatic
    /// redirects, retries, and decompression. The verified client trusts
    /// Mozilla/WebPKI roots only (parity with the current Reqwest
    /// default); the insecure client disables certificate **and**
    /// hostname verification and is only selected for requests whose
    /// [`eggsec_transport::TlsPolicy`] is explicitly insecure. HTTP/3 stays
    /// disabled until the transport contract has an authorized QUIC
    /// resolution/binding story.
    pub fn new(resolver: Arc<dyn TransportResolver>) -> Self {
        let verified = Client::builder()
            .http_version_policy(HttpVersionPolicy::Auto { allow_http3: false })
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
            .http_version_policy(HttpVersionPolicy::Auto { allow_http3: false })
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
/// singular physical pin the backend is allowed to see.
///
/// Direct hops carry the logical URL plus exactly one socket-authorized
/// `SocketAddr` for `RequestBuilder::resolved_addresses` (logical URL stays
/// authoritative for HTTP Host, TLS SNI/certificate identity, redirects,
/// cookies, and auth; Eggfetch performs no origin DNS when the pin is
/// present). Proxied hops keep the logical URL on the wire and carry one
/// pinned proxy-peer plus one pinned ultimate-target socket for
/// `Proxy::resolved_addresses` and `proxy_target_addresses` (both
/// independently authorized). One authorization cycle selects one physical
/// address per connection leg; a DNS-approved set is an input to selection,
/// never permission for the backend to choose another member after the
/// selected-socket checkpoint.
struct AuthorizedHop {
    logical_host: String,
    port: u16,
    primary: IpAddr,
    logical_url: Url,
    selected_target: SocketAddr,
    is_https: bool,
    proxy: Option<AuthorizedProxyRoute>,
}

/// Pinned proxy route for one hop (both legs independently authorized).
///
/// Singular addresses by construction: `proxy_peer` is exactly the address
/// passed to `authorize_proxy_socket`, and `ultimate_peer` is exactly the
/// address passed to `authorize_socket`. The backend receives one-element
/// pin sets derived from these fields, so it cannot silently fail over to a
/// second DNS-approved but socket-unchecked address. A retry may select
/// another candidate only after a fresh authorization cycle.
#[derive(Debug, Clone)]
struct AuthorizedProxyRoute {
    endpoint: Url,
    proxy_peer: std::net::SocketAddr,
    ultimate_peer: std::net::SocketAddr,
    /// `ProxyIntent` routing mode (Http vs All) for backend `Proxy` selection.
    mode_is_all: bool,
    credential: Option<eggsec_transport::ProxyCredential>,
}

impl EggfetchTransport {
    /// Run the full checkpoint sequence for one logical URL and bind the
    /// singular socket-authorized address as the resolved-route pin.
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
        // Both legs are resolved/authorized/bound independently through the
        // proxy-peer checkpoints. Supported matrix (published eggfetch-core
        // 0.1.7 route-pinning release):
        // - direct HTTP/HTTPS: logical URL + singular `resolved_addresses`
        //   pin (below);
        // - HTTP/HTTPS proxy -> HTTPS origin (CONNECT) with both pins: supported;
        // - SOCKS5 local-resolution -> HTTP/HTTPS with both pins: supported;
        // - SOCKS5H remote-DNS: fail closed (ultimate IP not locally enforceable);
        // - plaintext HTTP forward-proxy: fail closed (standard proxy cannot
        //   enforce requested ultimate IP).
        if let Some(endpoint) = request.proxy.endpoint() {
            authority.authorize_proxy(endpoint, url)?;
            let proxy_host = endpoint.host_str().ok_or_else(|| {
                TransportError::InvalidRequest("proxy endpoint has no host".to_string())
            })?;
            let proxy_port = effective_port(endpoint)?;
            let proxy_literal = proxy_host.parse::<IpAddr>().is_ok();
            authority.authorize_host(proxy_host, Some(proxy_port), proxy_literal)?;

            let (proxy_candidates, proxy_approved) = match proxy_host.parse::<IpAddr>() {
                Ok(lit) => {
                    let approved = authority.authorize_proxy_resolved(proxy_host, &[lit])?;
                    (vec![lit], approved)
                }
                Err(_) => {
                    let resolved = self.resolver.resolve(proxy_host);
                    if resolved.addresses.is_empty() {
                        return Err(TransportError::ResolutionFailed {
                            host: proxy_host.to_string(),
                            reason: "proxy: no addresses".to_string(),
                        });
                    }
                    let approved =
                        authority.authorize_proxy_resolved(proxy_host, &resolved.addresses)?;
                    (resolved.addresses, approved)
                }
            };
            let proxy_binding = validate_binding(proxy_host, &proxy_candidates, &proxy_approved)
                .map_err(|e| match e {
                    TransportError::InvalidBinding { host, reason } => {
                        TransportError::denied(PolicyCheckpoint::Proxy, format!("{host}: {reason}"))
                    }
                    other => other,
                })?;
            authority.authorize_proxy_socket(proxy_host, proxy_binding.primary(), proxy_port)?;
            // Single-address binding: the backend may dial only the exact
            // address that passed the selected-socket checkpoint per leg.
            // `binding.primary()` already passed `authorize_socket` above;
            // `proxy_binding.primary()` just passed `authorize_proxy_socket`.
            // Never expand from the DNS-approved sets (`approved` /
            // `proxy_approved`): those are selection inputs, not dial permission.
            let proxy_peer = std::net::SocketAddr::new(proxy_binding.primary(), proxy_port);
            let ultimate_peer = std::net::SocketAddr::new(binding.primary(), port);

            let proxy_scheme = endpoint.scheme().to_ascii_lowercase();
            let ultimate_scheme = url.scheme().to_ascii_lowercase();
            let is_socks = proxy_scheme == "socks5" || proxy_scheme == "socks5h";
            if proxy_scheme == "socks5h" {
                return Err(TransportError::denied(
                    PolicyCheckpoint::Proxy,
                    "SOCKS5H remote-DNS cannot be ultimate-IP-pinned; fail closed for strict scoped transport",
                ));
            }
            if ultimate_scheme == "http" && !is_socks {
                return Err(TransportError::denied(
                    PolicyCheckpoint::Proxy,
                    "plaintext HTTP forward-proxy ultimate pinning unsupported; fail closed for strict scoped transport",
                ));
            }

            let (mode_is_all, credential) = match &request.proxy {
                eggsec_transport::ProxyIntent::Direct => (true, None),
                eggsec_transport::ProxyIntent::Http { credential, .. } => {
                    (false, credential.clone())
                }
                eggsec_transport::ProxyIntent::All { credential, .. } => (true, credential.clone()),
            };
            // Proxied hops keep the logical URL on the wire; the backend pins
            // both legs via `Proxy::resolved_addresses` + `proxy_target_addresses`
            // with single-element sets derived from the socket-authorized
            // bindings (see `send_hop`). Cross-origin pinned-route reuse fails
            // closed unless a new authorized snapshot is constructed (per-hop
            // authorize_hop above).
            // Proxied hops keep the logical URL; direct pins are not used.
            // The singular ultimate pin travels via `proxy_target_addresses`.
            let selected_target = SocketAddr::new(binding.primary(), port);
            return Ok(AuthorizedHop {
                logical_host: host.to_string(),
                port,
                primary: binding.primary(),
                logical_url: url.clone(),
                selected_target,
                is_https: url.scheme() == "https",
                proxy: Some(AuthorizedProxyRoute {
                    endpoint: endpoint.clone(),
                    proxy_peer,
                    ultimate_peer,
                    mode_is_all,
                    credential,
                }),
            });
        }

        // Bind: one authorization cycle selects one physical socket. The
        // backend receives exactly that address via `resolved_addresses` and
        // never consults origin DNS. Literals use the same resolved-route
        // shape with the literal/port as the singular pin (no second hidden
        // DNS path; TLS semantics preserved via the logical URL).
        let selected_target = SocketAddr::new(binding.primary(), port);
        Ok(AuthorizedHop {
            logical_host: host.to_string(),
            port,
            primary: binding.primary(),
            logical_url: url.clone(),
            selected_target,
            is_https: url.scheme() == "https",
            proxy: None,
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
            .request(logical.method.clone(), hop.logical_url.as_str())
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
        // Direct hops carry the logical URL plus exactly one socket-authorized
        // `resolved_addresses` pin (Eggfetch performs no origin DNS; logical
        // URL owns Host/SNI/redirect/cookie/auth identity). Proxied hops carry
        // the logical URL plus the single pinned proxy-peer and ultimate-target
        // addresses (both authorized above). Single-element pin sets by
        // construction: the connector has no authorized alternate to fail over
        // to. Downgrade behavior stays compatibility-equivalent (`Allow` via
        // `RedirectPolicy::new`; a `Deny` policy needs a separate neutral
        // contract change with recording-fake parity).
        let mut builder = builder
            .body(eggfetch_body(&logical.body))
            .timeout(eggfetch_timeout(&request.timeout, remaining_total))
            .redirect_policy(eggfetch_core::RedirectPolicy::new(false, 0))
            .without_retry();
        if let Some(proxy_route) = hop.proxy.as_ref() {
            let endpoint_str = proxy_route.endpoint.as_str();
            let mut proxy = if proxy_route.mode_is_all {
                eggfetch_core::Proxy::all(endpoint_str).map_err(|e| {
                    TransportError::InvalidRequest(format!("invalid proxy endpoint: {e}"))
                })?
            } else {
                eggfetch_core::Proxy::http(endpoint_str).map_err(|e| {
                    TransportError::InvalidRequest(format!("invalid proxy endpoint: {e}"))
                })?
            };
            proxy = proxy
                .resolved_addresses([proxy_route.proxy_peer])
                .map_err(|e| {
                    TransportError::InvalidRequest(format!("invalid proxy peer pin: {e}"))
                })?;
            if let Some(cred) = proxy_route.credential.as_ref() {
                let auth = eggfetch_core::ProxyAuth::basic(cred.username(), cred.password())
                    .map_err(|e| {
                        TransportError::InvalidRequest(format!("invalid proxy credential: {e}"))
                    })?;
                proxy = proxy.auth(auth);
            }
            builder = builder
                .proxy(&proxy)
                .proxy_target_addresses([proxy_route.ultimate_peer]);
        } else {
            // No proxy is configured on the backend clients; this per-request
            // override additionally pins direct routing so environment-style
            // proxy selection cannot divert a hop (environment discovery is never
            // constructed in this adapter).
            builder = builder.without_proxy();
        }
        // `transport_hints` replaces the whole hint struct, while
        // `resolved_addresses` only sets the pin field: hints first, pin last
        // so the singular pin survives. The explicit SNI hint is retained as
        // the step-1 compatibility shim (identical to the logical-URL identity
        // Eggfetch would derive); removing it needs the SNI/certificate
        // fixture proof in Workstream 2 step 2.
        builder = builder
            .decompress(false)
            .transport_hints(eggfetch_core::TransportHints {
                target: None,
                sni_hostname: sni,
                resolved_target: None,
                trace: None,
            });
        if hop.proxy.is_none() {
            builder = builder.resolved_addresses([hop.selected_target]);
        }
        let response = builder.send().await.map_err(map_backend_error)?;
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
        // Direct hops dial the socket-authorized ultimate address. Proxied
        // hops dial the socket-authorized proxy peer (the ultimate pin
        // travels in the CONNECT / SOCKS command, not the TCP tuple).
        // Single-address binding makes this truthful by construction: the
        // connector has no authorized alternate peer to report instead.
        let remote_addr = if let Some(proxy) = hop.proxy.as_ref() {
            Some(proxy.proxy_peer)
        } else {
            Some(SocketAddr::new(hop.primary, hop.port))
        };
        ScopedHttpResponse {
            status,
            headers,
            body,
            final_url,
            redirect_history: history,
            connection: Some(eggsec_transport::ConnectionInfo {
                remote_addr,
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
