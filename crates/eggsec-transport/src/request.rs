//! Transport-neutral outbound request DTOs.
//!
//! Cross-domain interfaces must exchange these EggSec-owned (or standard
//! stable `http`/`url`) types — never a concrete client's builder. Domain
//! code may keep local builders/adapters, but the dispatch boundary speaks
//! [`ScopedHttpRequest`] only.

use crate::{headers, TransportError};
use bytes::Bytes;
use http::{HeaderMap, Method};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use url::Url;

/// Replayable request body.
///
/// Only replayable bodies are representable: retries, redirect re-sends, and
/// the recording fake all require the body to be re-emittable. Streaming
/// bodies are intentionally absent in Phase B (no streaming call sites need
/// parity beyond `response.bytes()`/`.text()`; see the Phase A parity
/// matrix). A future streaming variant must prove replayability or be
/// rejected for retry/redirect paths.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RequestBody {
    /// No body.
    #[default]
    Empty,
    /// In-memory bytes (cloneable, hence replayable).
    Bytes(Bytes),
}

impl RequestBody {
    /// Empty body.
    pub fn empty() -> Self {
        Self::Empty
    }

    /// Body from bytes.
    pub fn from_bytes(data: impl Into<Bytes>) -> Self {
        Self::Bytes(data.into())
    }

    /// Body from a UTF-8 string.
    pub fn from_string(s: impl Into<String>) -> Self {
        Self::Bytes(Bytes::from(s.into()))
    }

    /// `true` for every representable variant (all are replayable by
    /// construction). Kept as an explicit method so a future non-replayable
    /// variant fails loudly at retry/redirect sites.
    pub fn is_replayable(&self) -> bool {
        true
    }

    /// Body length in bytes.
    pub fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Bytes(b) => b.len(),
        }
    }

    /// `true` when there is no body.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Per-request timeout policy (value, not global state).
///
/// Mirrors the Phase A parity requirements: per-request timeouts everywhere
/// (5s–300s), connect timeouts only where noted (stress 5s, NSE helpers
/// 10s), pool/nodelay owned by the backend. Pool idle/max-idle and
/// `tcp_nodelay` defaults stay in `eggsec-core` constants; the transport
/// carries the resolved per-request values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeoutPolicy {
    /// Total request timeout (fail-closed: always set).
    pub request_timeout: Duration,
    /// Optional connect timeout (only where the caller needs it).
    pub connect_timeout: Option<Duration>,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(30),
            connect_timeout: None,
        }
    }
}

impl TimeoutPolicy {
    /// Policy with a request timeout and no separate connect timeout.
    pub fn with_request_timeout(secs: u64) -> Self {
        Self {
            request_timeout: Duration::from_secs(secs),
            connect_timeout: None,
        }
    }

    /// Policy with both request and connect timeouts.
    pub fn with_connect_timeout(mut self, secs: u64) -> Self {
        self.connect_timeout = Some(Duration::from_secs(secs));
        self
    }
}

/// Redirect policy (EggSec policy, not a concrete client's builder).
///
/// `SameHostOnly` is the scoped-tooling gate that must survive migration
/// verbatim: cross-host redirects are **stopped, not followed**, and the
/// redirect response itself is surfaced to the caller (Phase A tests 7/9).
/// `AuthorityChecked` still follows cross-host redirects but requires the
/// authority to authorize **each** target separately (Phase A test 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectPolicy {
    /// Never follow redirects; surface the 3xx response.
    None,
    /// Follow only same-host redirects up to `max_redirects`.
    SameHostOnly {
        /// Maximum redirect hops (fail-closed on exhaustion).
        max_redirects: u8,
    },
    /// Follow any redirect up to `max_redirects`, but each target must be
    /// separately authorized via
    /// [`crate::NetworkAuthority::authorize_redirect`].
    AuthorityChecked {
        /// Maximum redirect hops.
        max_redirects: u8,
    },
}

impl Default for RedirectPolicy {
    fn default() -> Self {
        Self::SameHostOnly { max_redirects: 10 }
    }
}

impl RedirectPolicy {
    /// No redirects.
    pub fn none() -> Self {
        Self::None
    }

    /// Same-host-only with `max` hops.
    pub fn same_host(max_redirects: u8) -> Self {
        Self::SameHostOnly { max_redirects }
    }

    /// Maximum hops for this policy (`0` for [`Self::None`]).
    pub fn max_redirects(&self) -> u8 {
        match *self {
            Self::None => 0,
            Self::SameHostOnly { max_redirects } | Self::AuthorityChecked { max_redirects } => {
                max_redirects
            }
        }
    }

    /// Whether hop `redirect_count` (already followed) from `from_host` to
    /// `to_host` may be followed without consulting the authority.
    ///
    /// `AuthorityChecked` always returns `false` here: the authority must be
    /// consulted per hop. `SameHostOnly` compares `host_str()` only
    /// (distinct spellings such as `127.0.0.1` vs `localhost` stop, mirroring
    /// the DNS-rebinding concern in Phase A test 9).
    pub fn allows_without_authority(
        &self,
        from_host: Option<&str>,
        to_host: Option<&str>,
        redirect_count: u8,
    ) -> bool {
        match *self {
            Self::None => false,
            Self::SameHostOnly { max_redirects } => {
                if redirect_count >= max_redirects {
                    return false;
                }
                match (from_host, to_host) {
                    (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                    _ => false,
                }
            }
            Self::AuthorityChecked { .. } => false,
        }
    }
}

/// Proxy intent: the proxy endpoint is carried separately from the ultimate
/// destination so authorization can treat them as **two distinct decisions**
/// (Phase A test 11). Authorizing the target must never authorize the proxy
/// endpoint, nor vice versa.
#[derive(Clone, PartialEq, Eq, Default)]
pub enum ProxyIntent {
    /// Direct connection (no proxy).
    #[default]
    Direct,
    /// HTTP proxy for `http://` destinations.
    Http {
        /// Proxy endpoint URL.
        endpoint: Url,
        /// Optional `Proxy-Authorization` credential (redacted in `Debug`).
        credential: Option<ProxyCredential>,
    },
    /// Proxy for all schemes.
    All {
        /// Proxy endpoint URL.
        endpoint: Url,
        /// Optional credential (redacted in `Debug`).
        credential: Option<ProxyCredential>,
    },
}

/// Proxy credential (secret-bearing; redacted in `Debug`).
#[derive(Clone, PartialEq, Eq)]
pub struct ProxyCredential {
    username: String,
    password: String,
}

impl ProxyCredential {
    /// New credential pair.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

impl fmt::Debug for ProxyCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyCredential")
            .field("username", &headers::REDACTED)
            .field("password", &headers::REDACTED)
            .finish()
    }
}

impl fmt::Debug for ProxyIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Direct => write!(f, "Direct"),
            Self::Http {
                endpoint,
                credential,
            } => {
                let ep = redact_url_for_debug(endpoint);
                if credential.is_some() {
                    write!(f, "Http {{ endpoint: {ep}, credential: ***REDACTED*** }}")
                } else {
                    write!(f, "Http {{ endpoint: {ep}, credential: None }}")
                }
            }
            Self::All {
                endpoint,
                credential,
            } => {
                let ep = redact_url_for_debug(endpoint);
                if credential.is_some() {
                    write!(f, "All {{ endpoint: {ep}, credential: ***REDACTED*** }}")
                } else {
                    write!(f, "All {{ endpoint: {ep}, credential: None }}")
                }
            }
        }
    }
}

impl ProxyIntent {
    /// Direct connection.
    pub fn direct() -> Self {
        Self::Direct
    }

    /// Proxy endpoint URL, if any.
    pub fn endpoint(&self) -> Option<&Url> {
        match self {
            Self::Direct => None,
            Self::Http { endpoint, .. } | Self::All { endpoint, .. } => Some(endpoint),
        }
    }

    /// `true` when a proxy is configured.
    pub fn uses_proxy(&self) -> bool {
        !matches!(self, Self::Direct)
    }
}

/// TLS policy intent (value, not a concrete client's builder).
///
/// Phase A parity: EggSec uses platform roots, `insecure` mode disables
/// certificate verification only (plus hostname verification for the NSE
/// helper path), and **no** custom roots/identity/`min_tls_version`/custom
/// SNI/`https_only` appear anywhere. Those rows need no parity and are not
/// representable here; SNI/Host overrides exist only so the
/// `tls-consistency` checkpoint can reject them fail-closed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TlsPolicy {
    /// Verify the server certificate chain.
    pub verify_certificates: bool,
    /// Verify the certificate hostname.
    pub verify_hostnames: bool,
    /// Explicit SNI override (must equal the request host; otherwise the
    /// `tls-consistency` checkpoint denies).
    pub sni_override: Option<String>,
    /// Explicit Host override (same rule as SNI).
    pub host_override: Option<String>,
}

impl TlsPolicy {
    /// Verified TLS (default).
    pub fn verified() -> Self {
        Self {
            verify_certificates: true,
            verify_hostnames: true,
            sni_override: None,
            host_override: None,
        }
    }

    /// Insecure TLS: certificate (and hostname) verification off. Log-only
    /// at construction sites; authorization semantics never change
    /// (Phase A test 12).
    pub fn insecure() -> Self {
        Self {
            verify_certificates: false,
            verify_hostnames: false,
            sni_override: None,
            host_override: None,
        }
    }

    /// `true` when verification is fully on.
    pub fn is_verified(&self) -> bool {
        self.verify_certificates && self.verify_hostnames
    }

    /// Validate SNI/Host override consistency against `request_host`.
    /// Mismatched overrides are rejected fail-closed.
    pub fn check_consistency(&self, request_host: &str) -> Result<(), TransportError> {
        if let Some(ref sni) = self.sni_override {
            if !sni.eq_ignore_ascii_case(request_host) {
                return Err(TransportError::denied(
                    crate::PolicyCheckpoint::TlsConsistency,
                    format!("SNI override '{sni}' does not match request host"),
                ));
            }
        }
        if let Some(ref host) = self.host_override {
            if !host.eq_ignore_ascii_case(request_host) {
                return Err(TransportError::denied(
                    crate::PolicyCheckpoint::TlsConsistency,
                    "Host override does not match request host",
                ));
            }
        }
        Ok(())
    }
}

/// Backend-agnostic transport hints (only where current behavior requires
/// them; keep this struct narrow).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportHints {
    /// `User-Agent` header value (if the caller sets one).
    pub user_agent: Option<String>,
    /// TCP `NODELAY` (default `true` per parity matrix).
    pub tcp_nodelay: bool,
}

impl Default for TransportHints {
    fn default() -> Self {
        Self {
            user_agent: None,
            tcp_nodelay: true,
        }
    }
}

/// Scope-aware outbound HTTP request.
///
/// The dispatch boundary speaks this DTO. It intentionally exposes no
/// concrete-client builder surface: method/URL/headers/body plus the policy
/// values (timeout, redirect, proxy, TLS, hints) that the Phase A parity
/// matrix requires.
#[derive(Clone, PartialEq, Eq)]
pub struct ScopedHttpRequest {
    /// HTTP method.
    pub method: Method,
    /// Canonical request URL (userinfo rejected at construction).
    pub url: Url,
    /// Request headers (auth-context applied via [`headers::apply_auth_headers`]).
    pub headers: HeaderMap,
    /// Replayable body.
    pub body: RequestBody,
    /// Timeout policy value.
    pub timeout: TimeoutPolicy,
    /// Redirect policy value.
    pub redirect: RedirectPolicy,
    /// Proxy intent (endpoint separate from ultimate destination).
    pub proxy: ProxyIntent,
    /// TLS policy intent.
    pub tls: TlsPolicy,
    /// Transport hints.
    pub hints: TransportHints,
}

impl ScopedHttpRequest {
    /// New `GET`-style request. Rejects URLs with userinfo (`user:pass@`)
    /// fail-closed: credentials in the URL never reach the wire or logs
    /// (Phase A test 8).
    pub fn new(method: Method, url: Url) -> Result<Self, TransportError> {
        reject_url_userinfo(&url)?;
        if url.host_str().is_none() {
            return Err(TransportError::InvalidRequest(
                "request URL has no host".to_string(),
            ));
        }
        Ok(Self {
            method,
            url,
            headers: HeaderMap::new(),
            body: RequestBody::Empty,
            timeout: TimeoutPolicy::default(),
            redirect: RedirectPolicy::default(),
            proxy: ProxyIntent::Direct,
            tls: TlsPolicy::verified(),
            hints: TransportHints::default(),
        })
    }

    /// Parse `url` then construct.
    pub fn new_with_url(method: Method, url: &str) -> Result<Self, TransportError> {
        let parsed = Url::parse(url)
            .map_err(|e| TransportError::InvalidRequest(format!("invalid URL: {e}")))?;
        Self::new(method, parsed)
    }

    /// Builder: set headers from `pairs` (overwrites same-named headers).
    pub fn with_headers(mut self, pairs: &HashMap<String, String>) -> Result<Self, TransportError> {
        let map = headers::header_map_from_pairs(pairs)?;
        for (k, v) in map {
            if let Some(name) = k {
                self.headers.insert(name, v);
            }
        }
        Ok(self)
    }

    /// Builder: set a single header.
    pub fn with_header(mut self, name: &str, value: &str) -> Result<Self, TransportError> {
        let hname: http::HeaderName = name
            .parse()
            .map_err(|e| TransportError::InvalidRequest(format!("bad header name: {e}")))?;
        let hval: http::HeaderValue = value
            .parse()
            .map_err(|e| TransportError::InvalidRequest(format!("bad header value: {e}")))?;
        self.headers.insert(hname, hval);
        Ok(self)
    }

    /// Builder: set the body.
    pub fn with_body(mut self, body: RequestBody) -> Self {
        self.body = body;
        self
    }

    /// Builder: set the timeout policy.
    pub fn with_timeout(mut self, timeout: TimeoutPolicy) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builder: set the redirect policy.
    pub fn with_redirect(mut self, redirect: RedirectPolicy) -> Self {
        self.redirect = redirect;
        self
    }

    /// Builder: set the proxy intent.
    pub fn with_proxy(mut self, proxy: ProxyIntent) -> Self {
        self.proxy = proxy;
        self
    }

    /// Builder: set the TLS policy.
    pub fn with_tls(mut self, tls: TlsPolicy) -> Self {
        self.tls = tls;
        self
    }

    /// Builder: set transport hints.
    pub fn with_hints(mut self, hints: TransportHints) -> Self {
        self.hints = hints;
        self
    }

    /// Apply auth-context headers/cookies (canonical transport-neutral path).
    pub fn apply_auth_context(
        &mut self,
        auth_headers: &HashMap<String, String>,
        auth_cookies: &HashMap<String, String>,
    ) -> Result<(), TransportError> {
        headers::apply_auth_headers(&mut self.headers, auth_headers, auth_cookies)
    }

    /// Request host (lowercased by `url`, without port).
    pub fn host(&self) -> Option<&str> {
        self.url.host_str()
    }
}

impl fmt::Debug for ScopedHttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopedHttpRequest")
            .field("method", &self.method)
            .field("url", &redact_url_for_debug(&self.url))
            .field("headers", &headers::redacted_headers_debug(&self.headers))
            .field("body_len", &self.body.len())
            .field("timeout", &self.timeout)
            .field("redirect", &self.redirect)
            .field("proxy", &self.proxy)
            .field(
                "tls",
                &format_args!("TlsPolicy {{ verified: {} }}", self.tls.is_verified()),
            )
            .field("hints", &self.hints)
            .finish()
    }
}

/// Reject URLs carrying userinfo (`http://user:pass@host/`). Managed
/// navigation rejects any authority containing `@`; the transport contract
/// enforces the same rule fail-closed so credentials never reach the wire.
pub fn reject_url_userinfo(url: &Url) -> Result<(), TransportError> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(TransportError::denied(
            crate::PolicyCheckpoint::InitialUrl,
            "URL userinfo is rejected (credentials must not appear in the URL)",
        ));
    }
    Ok(())
}

/// Render a URL for `Debug`/logs with userinfo stripped (never leaks
/// `user:pass@` even if a caller bypassed construction checks).
pub fn redact_url_for_debug(url: &Url) -> String {
    let mut redacted = url.clone();
    if !redacted.username().is_empty() || redacted.password().is_some() {
        let _ = redacted.set_username("");
        let _ = redacted.set_password(None);
    }
    redacted.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_url() -> Url {
        Url::parse("http://example.com/a").expect("url")
    }

    #[test]
    fn userinfo_url_rejected_and_never_leaks() {
        let url = Url::parse("http://user:s3cr3t-pw@example.com/").expect("url");
        let err = ScopedHttpRequest::new(Method::GET, url).unwrap_err();
        assert!(err.is_denied());
        let redacted =
            redact_url_for_debug(&Url::parse("http://user:s3cr3t-pw@example.com/").expect("url"));
        assert!(!redacted.contains("s3cr3t-pw"), "leak: {redacted}");
        assert!(redacted.contains("example.com"));
    }

    #[test]
    fn debug_redacts_secrets() {
        let mut req = ScopedHttpRequest::new(Method::GET, test_url()).expect("req");
        req.headers.insert(
            http::header::AUTHORIZATION,
            http::HeaderValue::from_static("Bearer s3cr3t"),
        );
        req.headers.insert(
            http::header::COOKIE,
            http::HeaderValue::from_static("session=abc123"),
        );
        let dbg = format!("{req:?}");
        assert!(!dbg.contains("s3cr3t"), "auth leak: {dbg}");
        assert!(!dbg.contains("abc123"), "cookie leak: {dbg}");
        // TLS verification state is visible (not a secret), bodies are lengths.
        assert!(dbg.contains("verified: true"), "tls state lost: {dbg}");
    }

    #[test]
    fn same_host_policy_matches_phase_a_semantics() {
        let p = RedirectPolicy::SameHostOnly { max_redirects: 5 };
        assert!(p.allows_without_authority(Some("a.com"), Some("a.com"), 0));
        assert!(p.allows_without_authority(Some("a.com"), Some("A.COM"), 4));
        assert!(!p.allows_without_authority(Some("a.com"), Some("b.com"), 0));
        // 127.0.0.1 vs localhost stop even though both are loopback.
        assert!(!p.allows_without_authority(Some("127.0.0.1"), Some("localhost"), 0));
        assert!(!p.allows_without_authority(Some("a.com"), Some("a.com"), 5));
        assert_eq!(RedirectPolicy::None.max_redirects(), 0);
        assert!(!RedirectPolicy::None.allows_without_authority(Some("a"), Some("a"), 0));
        // AuthorityChecked always defers to the authority.
        let q = RedirectPolicy::AuthorityChecked { max_redirects: 5 };
        assert!(!q.allows_without_authority(Some("a"), Some("a"), 0));
    }

    #[test]
    fn tls_override_mismatch_denied() {
        let mut tls = TlsPolicy::verified();
        tls.sni_override = Some("evil.example".to_string());
        let err = tls.check_consistency("example.com").unwrap_err();
        assert!(err.is_denied());
        let ok = TlsPolicy::verified();
        ok.check_consistency("example.com").expect("match");
    }

    #[test]
    fn body_is_replayable_by_construction() {
        assert!(RequestBody::empty().is_replayable());
        assert!(RequestBody::from_bytes(Bytes::from_static(b"hi")).is_replayable());
    }
}
