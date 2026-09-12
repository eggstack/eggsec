//! Pure DTO conversions between the transport contract and `eggfetch-core`.
//!
//! No I/O, no policy decisions: every fallible conversion fails closed with
//! [`eggsec_transport::TransportError`].

use bytes::Bytes;
use eggsec_transport::{TimeoutPolicy, TransportError};
use http::Method;
use std::net::IpAddr;
use std::time::Duration;
use url::{Host, Url};

/// Effective port for an `http(s)` URL (explicit or scheme default).
pub(crate) fn effective_port(url: &Url) -> Result<u16, TransportError> {
    url.port_or_known_default().ok_or_else(|| {
        TransportError::InvalidRequest(format!(
            "URL '{}' has no usable port",
            eggsec_transport::redact_url_for_debug(url)
        ))
    })
}

/// Reject non-`http(s)` schemes fail-closed (`eggfetch-core` only speaks
/// HTTP; anything else must never reach dispatch).
pub(crate) fn ensure_http_scheme(url: &Url) -> Result<(), TransportError> {
    match url.scheme() {
        "http" | "https" => Ok(()),
        other => Err(TransportError::InvalidRequest(format!(
            "unsupported URL scheme '{other}': only http and https are allowed"
        ))),
    }
}

/// Direct-IP literal carried by the URL, if any (bracket-agnostic).
pub(crate) fn ip_literal(url: &Url) -> Option<IpAddr> {
    match url.host()? {
        Host::Ipv4(v4) => Some(IpAddr::V4(v4)),
        Host::Ipv6(v6) => Some(IpAddr::V6(v6)),
        Host::Domain(_) => None,
    }
}

/// Value for the adapter-owned `Host` header: logical host, with an explicit
/// port only when it differs from the scheme default.
pub(crate) fn host_header_value(url: &Url) -> Result<String, TransportError> {
    let host = url
        .host_str()
        .ok_or_else(|| TransportError::InvalidRequest("request URL has no host".to_string()))?;
    let default_port = match url.scheme() {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    };
    let port_part = match url.port() {
        Some(explicit) if Some(explicit) != default_port => format!(":{explicit}"),
        _ => String::new(),
    };
    Ok(format!("{host}{port_part}"))
}

/// Rewrite the wire URL host to an approved IP literal, preserving
/// scheme/port/path/query/fragment. The connector therefore resolves only
/// the authorized address; the logical hostname travels via `Host`/SNI.
pub(crate) fn pin_wire_url(logical: &Url, approved: IpAddr) -> Result<Url, TransportError> {
    // `Url::set_host` takes IPv6 literals bracketed (bare colons are
    // rejected as invalid domain characters).
    let host = if approved.is_ipv6() {
        format!("[{approved}]")
    } else {
        approved.to_string()
    };
    let mut wire = logical.clone();
    wire.set_host(Some(&host)).map_err(|e| {
        TransportError::InvalidRequest(format!("failed to pin approved address: {e}"))
    })?;
    Ok(wire)
}

/// Map the contract timeout to an `eggfetch-core` per-hop timeout.
///
/// `total` is the caller budget shrunk by already-elapsed time (the redirect
/// loop passes the remainder); `connect` mirrors the optional connect
/// timeout. Pool/read/write phases stay unset: the contract carries no such
/// values and parity configures none.
pub(crate) fn eggfetch_timeout(
    policy: &TimeoutPolicy,
    remaining_total: Duration,
) -> eggfetch_core::Timeout {
    eggfetch_core::Timeout {
        pool: None,
        connect: policy.connect_timeout,
        write: None,
        read: None,
        total: Some(remaining_total),
    }
}

/// Classify a backend failure. Timeouts keep their phase so callers can
/// distinguish connect/read/total without parsing strings; everything else
/// is an opaque backend failure (eggfetch error types never cross the
/// transport boundary). The full source chain is folded into the message
/// so handshake/connect diagnostics survive the mapping.
pub(crate) fn map_backend_error(error: eggfetch_core::Error) -> TransportError {
    let mut message = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    TransportError::Backend(message)
}

/// Map a redirect-construction failure (bad `Location`, unsupported scheme,
/// userinfo, non-replayable body) to a validation error so callers can
/// distinguish malformed redirects from scope rejections (`PolicyDenied`).
pub(crate) fn map_redirect_build_error(error: eggfetch_core::Error) -> TransportError {
    TransportError::InvalidRequest(format!("malformed redirect: {error}"))
}

/// Convert a logical body to the backend body (both replayable; empty stays
/// empty so no `Content-Length: 0` surprises downstream).
pub(crate) fn eggfetch_body(body: &Bytes) -> eggfetch_core::RequestBody {
    if body.is_empty() {
        eggfetch_core::RequestBody::Empty
    } else {
        eggfetch_core::RequestBody::Bytes(body.clone())
    }
}

/// Extract bytes from a redirect-transformed backend request body. Streams
/// are unreachable (the adapter never builds them); encountering one fails
/// closed rather than dropping payload bytes silently.
pub(crate) fn extract_redirect_body(
    body: &eggfetch_core::RequestBody,
) -> Result<Bytes, TransportError> {
    match body {
        eggfetch_core::RequestBody::Empty => Ok(Bytes::new()),
        eggfetch_core::RequestBody::Bytes(b) => Ok(b.clone()),
        eggfetch_core::RequestBody::Stream { .. } => Err(TransportError::Backend(
            "redirect transformed a streaming body (unreachable: adapter never sends streams)"
                .to_string(),
        )),
    }
}

/// `true` when both URLs share scheme/host/port (redirect credential-scope
/// comparison, mirroring Eggfetch's origin check).
pub(crate) fn same_origin(a: &Url, b: &Url) -> bool {
    a.origin() == b.origin()
}

/// Logical hop state carried across the manual redirect loop.
#[derive(Debug, Clone)]
pub(crate) struct LogicalHop {
    /// HTTP method for this hop (rewritten per redirect semantics).
    pub(crate) method: Method,
    /// Logical (pre-pinning) URL for this hop.
    pub(crate) url: Url,
    /// Headers for this hop, excluding the adapter-owned `Host` header
    /// (set fresh at wire time).
    pub(crate) headers: http::HeaderMap,
    /// Body bytes (possibly empty; always replayable).
    pub(crate) body: Bytes,
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_transport::PolicyCheckpoint;

    #[test]
    fn host_header_omits_default_port_and_keeps_explicit() {
        let d = Url::parse("http://example.com/a").expect("url");
        assert_eq!(host_header_value(&d).expect("host"), "example.com");
        let e = Url::parse("http://example.com:8080/a").expect("url");
        assert_eq!(host_header_value(&e).expect("host"), "example.com:8080");
        // Explicit-but-default ports normalize to the bare host.
        let n = Url::parse("http://example.com:80/a").expect("url");
        assert_eq!(host_header_value(&n).expect("host"), "example.com");
        let v6 = Url::parse("http://[::1]:8080/a").expect("url");
        assert_eq!(host_header_value(&v6).expect("host"), "[::1]:8080");
    }

    #[test]
    fn pinning_preserves_everything_but_host() {
        let logical = Url::parse("https://example.com:8443/a/b?x=1#frag").expect("url");
        let ip: IpAddr = "93.184.216.34".parse().expect("ip");
        let wire = pin_wire_url(&logical, ip).expect("pin");
        assert_eq!(wire.scheme(), "https");
        assert_eq!(wire.port(), Some(8443));
        assert_eq!(wire.path(), "/a/b");
        assert_eq!(wire.query(), Some("x=1"));
        assert_eq!(wire.fragment(), Some("frag"));
        assert_eq!(wire.host_str(), Some("93.184.216.34"));
    }

    #[test]
    fn pinning_supports_ipv6_literals() {
        let logical = Url::parse("http://example.com/a").expect("url");
        let ip: IpAddr = "::1".parse().expect("ip");
        let wire = pin_wire_url(&logical, ip).expect("pin");
        assert_eq!(wire.host_str(), Some("[::1]"));
        assert_eq!(ip_literal(&wire), Some(ip));
    }

    #[test]
    fn literals_detected_without_bracket_hassle() {
        assert_eq!(
            ip_literal(&Url::parse("http://127.0.0.1/").expect("url")),
            Some("127.0.0.1".parse().expect("ip"))
        );
        assert_eq!(
            ip_literal(&Url::parse("http://[::1]/").expect("url")),
            Some("::1".parse().expect("ip"))
        );
        assert_eq!(
            ip_literal(&Url::parse("http://example.com/").expect("url")),
            None
        );
    }

    #[test]
    fn non_http_schemes_rejected() {
        let ftp = Url::parse("ftp://example.com/a").expect("url");
        assert!(ensure_http_scheme(&ftp).is_err());
        let http = Url::parse("http://example.com/a").expect("url");
        assert!(ensure_http_scheme(&http).is_ok());
    }

    #[test]
    fn backend_timeout_keeps_phase_and_never_denies() {
        let err = map_backend_error(eggfetch_core::Error::Timeout {
            phase: eggfetch_core::TimeoutPhase::Connect,
            elapsed: Duration::from_secs(2),
        });
        assert!(!err.is_denied());
        assert!(err.to_string().contains("connect"));
    }

    #[test]
    fn redirect_build_failures_are_validation_not_denial() {
        let err = map_redirect_build_error(eggfetch_core::Error::Unsupported("x".into()));
        assert!(matches!(err, TransportError::InvalidRequest(_)));
        assert!(!err.is_denied());
    }

    #[test]
    fn checkpoint_names_match_contract_order() {
        assert_eq!(PolicyCheckpoint::Dns.to_string(), "dns");
        assert_eq!(PolicyCheckpoint::Reresolution.to_string(), "reresolution");
    }
}
