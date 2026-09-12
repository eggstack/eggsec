//! Outbound vs interception boundary (Phase D workstream 4).
//!
//! `eggsec-web-proxy` has two fundamentally different roles:
//!
//! ```text
//! A. inbound/listening/interception/MITM (remains owned here)
//! B. ordinary outbound HTTP helper/upstream requests (migrates where semantics fit)
//! ```
//!
//! **Side A — interception/server TLS (never migrates to the shared client
//! transport):**
//! - `intercept/` listener/server I/O, MITM `TlsAcceptor`, certificate
//!   generation (`rcgen`), CA handling
//! - HTTP/2 stream demux (`h2`), WebSocket interception
//!   (`tokio-tungstenite`), gRPC protobuf inspection (`prost`)
//! - Custom SOCKS5 handshake (`socks.rs`), transparent-proxy integration
//! - Raw `TcpStream` dialing with `TCP_NODELAY` (`utils::connect_with_nodelay_timeout`)
//! - `lib.rs` `lookup_host` (RAWNET DNS, remain specialized)
//!
//! Side A must never depend on `eggsec-transport` (scope-aware *client*
//! contract) — server termination and interception demux are not outbound
//! dispatches. The guard enforces this: `intercept/`, `socks.rs`,
//! `http_connect.rs` contain no `eggsec_transport::` references.
//!
//! **Side B — ordinary outbound (migrates where semantics fit):**
//! - `health.rs` proxy health probes (`HealthChecker`)
//! - `utils::create_insecure_client_with_options` (insecure factory)
//!
//! Current disposition: side B **remains on `reqwest`** because every
//! production probe exercises SOCKS/HTTP proxy routing
//! (`reqwest::Proxy::all` + `basic_auth`, per-proxy `test_url` over
//! `socks5`/`http`). The Phase C adapter fails closed on any non-`Direct`
//! `ProxyIntent` (proxy routing deferred past Phase C; remote-DNS SOCKS
//! semantics would hide the target IP from the local authority). Migrating
//! proxy-testing probes to a proxy-blind backend would silently change what
//! is tested. Direct (non-proxied) probes can use the helper below.
//!
//! This module provides the narrow transport-neutral builder for direct
//! outbound probes (no proxy, verified or explicit-insecure TLS, same-host
//! redirects). Proxy-testing paths stay on the specialized stack with an
//! explicit reason; `reqwest`/`rustls`/`tokio-rustls`/`rcgen`/`h2`/`http`
//! remain required for interception + proxy-routing and are not removed for
//! aesthetics.

use std::collections::HashMap;

use eggsec_transport::{RedirectPolicy, RequestBody, ScopedHttpRequest, TimeoutPolicy, TlsPolicy};

/// Maximum redirect hops for direct outbound probes (same-host only).
pub const OUTBOUND_MAX_REDIRECTS: u8 = 5;

/// Build a direct (non-proxied) outbound probe request.
///
/// - `url`: canonical URL (userinfo rejected fail-closed).
/// - `timeout_secs`: per-request timeout (`max(1)`).
/// - `insecure`: explicit opt-in insecure TLS (warn-logged by the caller;
///   authorization semantics never change).
/// - `headers`: applied overwrite; no cookie jar (request-scoped headers
///   only — global jar semantics would bypass per-hop authorization).
///
/// Redirects follow same-host targets only; cross-host 3xx surfaces.
/// Proxy intent is always `Direct`: proxied probes must use the specialized
/// `health.rs` path until the adapter supports authorized proxy routing.
pub fn build_direct_probe_request(
    url: &str,
    timeout_secs: u64,
    insecure: bool,
    headers: &HashMap<String, String>,
) -> Result<ScopedHttpRequest, String> {
    let mut request = ScopedHttpRequest::new_with_url(eggsec_transport::Method::GET, url)
        .map_err(|e| format!("invalid outbound probe URL '{url}': {e}"))?;
    if !headers.is_empty() {
        let map = eggsec_transport::header_map_from_pairs(headers)
            .map_err(|e| format!("invalid outbound probe headers: {e}"))?;
        for (name, value) in map {
            if let Some(name) = name {
                request.headers.insert(name, value);
            }
        }
    }
    request.body = RequestBody::Empty;
    request.timeout = TimeoutPolicy::with_request_timeout(timeout_secs.max(1));
    request.redirect = RedirectPolicy::SameHostOnly {
        max_redirects: OUTBOUND_MAX_REDIRECTS,
    };
    request.tls = if insecure {
        tracing::warn!(
            "web-proxy direct probe armed insecure TLS for '{url}'; \
             verification off for lab traffic only"
        );
        TlsPolicy::insecure()
    } else {
        TlsPolicy::verified()
    };
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_probe_policies() {
        let req =
            build_direct_probe_request("http://example.com/health", 5, false, &HashMap::new())
                .expect("builds");
        assert_eq!(req.method, eggsec_transport::Method::GET);
        assert_eq!(
            req.timeout.request_timeout,
            std::time::Duration::from_secs(5)
        );
        assert_eq!(
            req.redirect,
            RedirectPolicy::SameHostOnly {
                max_redirects: OUTBOUND_MAX_REDIRECTS
            }
        );
        assert!(!req.proxy.uses_proxy());
        assert!(req.tls.is_verified());
        assert!(req.body.is_replayable());
    }

    #[test]
    fn direct_probe_insecure_opt_in() {
        let req = build_direct_probe_request("http://example.com/health", 5, true, &HashMap::new())
            .expect("builds");
        assert!(!req.tls.is_verified());
    }

    #[test]
    fn direct_probe_rejects_userinfo() {
        let err =
            build_direct_probe_request("http://user:pass@example.com/", 5, false, &HashMap::new())
                .unwrap_err();
        assert!(err.contains("invalid outbound probe URL"), "{err}");
    }

    #[test]
    fn outbound_module_has_no_server_tls_types() {
        // Boundary: outbound helper names only client-contract types.
        // If a future edit pulls server/interception types here
        // (TlsAcceptor, ServerConfig, h2, tungstenite, rcgen), this scan fails.
        let src = include_str!("outbound.rs");
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
        for b in [
            ["Tls", "Acceptor"].concat(),
            ["Server", "Config"].concat(),
            ["tokio_", "tungstenite"].concat(),
            "rcgen".to_string(),
            "h2::".to_string(),
            ["tokio", "_rustls"].concat(),
        ] {
            assert!(
                !joined.contains(b.as_str()),
                "outbound.rs must not mention server/interception type '{b}'"
            );
        }
    }
}
