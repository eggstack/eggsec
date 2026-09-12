//! Transport-neutral response DTOs.

use crate::headers;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use std::fmt;
use std::net::SocketAddr;
use url::Url;

/// Connection metadata for the established (authorized) connection.
///
/// Carried so scanner envelopes and audit logs can record the destination
/// actually used — the binding the authority approved, not a re-resolved
/// guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    /// Remote socket address actually connected.
    pub remote_addr: Option<SocketAddr>,
    /// SNI host actually sent (if TLS).
    pub sni_host: Option<String>,
}

/// Scope-aware HTTP response.
#[derive(Clone, PartialEq, Eq)]
pub struct ScopedHttpResponse {
    /// HTTP status.
    pub status: StatusCode,
    /// Response headers.
    pub headers: HeaderMap,
    /// Response body bytes.
    pub body: Bytes,
    /// Final URL after redirects.
    pub final_url: Url,
    /// Redirect chain (each followed hop, in order; empty when none).
    pub redirect_history: Vec<Url>,
    /// Connection actually used (when the backend reports it).
    pub connection: Option<ConnectionInfo>,
}

impl ScopedHttpResponse {
    /// Minimal 200 response for `url` (fake/test helper).
    pub fn ok(url: Url, body: impl Into<Bytes>) -> Self {
        Self {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: body.into(),
            final_url: url,
            redirect_history: Vec::new(),
            connection: None,
        }
    }

    /// Response body as UTF-8 lossy text.
    pub fn text_lossy(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
}

impl fmt::Debug for ScopedHttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopedHttpResponse")
            .field("status", &self.status)
            .field("headers", &headers::redacted_headers_debug(&self.headers))
            .field("body_len", &self.body.len())
            .field(
                "final_url",
                &crate::request::redact_url_for_debug(&self.final_url),
            )
            .field("redirect_hops", &self.redirect_history.len())
            .field("connection", &self.connection)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    #[test]
    fn response_debug_redacts_set_cookie() {
        let url = Url::parse("http://example.com/").expect("url");
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::SET_COOKIE,
            HeaderValue::from_static("session=s3cr3t; Path=/"),
        );
        let resp = ScopedHttpResponse {
            status: StatusCode::OK,
            headers,
            body: Bytes::from_static(b"ok"),
            final_url: url,
            redirect_history: Vec::new(),
            connection: None,
        };
        let dbg = format!("{resp:?}");
        assert!(!dbg.contains("s3cr3t"), "cookie leak: {dbg}");
    }
}
