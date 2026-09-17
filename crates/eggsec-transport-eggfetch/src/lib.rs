//! Scope-aware outbound HTTP transport over `eggfetch-core` (Phase C + corrective pass).
//!
//! [`EggfetchTransport`] implements [`eggsec_transport::HttpTransport`] on top
//! of the published `eggfetch-core` client using only stable public APIs. No
//! upstream Eggfetch change was required; the TOCTOU-closed binding is
//! achieved by the adapter itself:
//!
//! - Every hostname hop is resolved through the caller-supplied
//!   [`eggsec_transport::TransportResolver`], approved by the mandatory
//!   [`eggsec_transport::NetworkAuthority`], validated with
//!   [`eggsec_transport::validate_binding`], and then **pinned**. Direct hops
//!   rewrite the wire URL host to the approved IP literal, so the connector's
//!   own resolution can only return the authorized address. Proxied hops keep
//!   the logical URL and pin both legs via `Proxy::resolved_addresses`
//!   (proxy peer) + `proxy_target_addresses` (ultimate target, where
//!   enforceable). The logical hostname is preserved on the wire via the
//!   `Host` header and TLS SNI in both cases.
//! - Eggfetch automatic redirect following stays disabled
//!   (`follow_redirects(false)` + per-request `RedirectPolicy::new(false, 0)`).
//!   Each hop is computed with Eggfetch's public redirect primitive
//!   (`eggfetch_core::redirect::build_redirect_request`, which owns method
//!   rewrite, sensitive-header stripping, and body-replay rules) and is
//!   authorized through the full checkpoint sequence before dispatch.
//! - HTTP/3 is never constructed (the `http3` feature is off and both
//!   clients use [`eggfetch_core::HttpVersionPolicy::Auto`] with
//!   `allow_http3: false`; H1/H2 negotiate via ALPN).
//! - Proxy routing uses the qualified `eggfetch-core 0.1.5` pinning release:
//!   proxy peers and ultimate targets are independently resolved/authorized
//!   through `authorize_proxy_resolved` / `authorize_proxy_socket` plus the
//!   ultimate checkpoints. Supported: direct, HTTP/HTTPS proxy → HTTPS origin
//!   (CONNECT) with both pins, SOCKS5 local-resolution → HTTP/HTTPS with both
//!   pins. Fail-closed: SOCKS5H remote-DNS and plaintext HTTP forward-proxy
//!   ultimate pinning (the local process cannot constrain the ultimate peer).
//! - Retries are disabled in the backend (EggSec owns retry above the
//!   transport); every body in the contract is replayable by construction.
//! - Automatic decompression is off (parity: the current stack configures
//!   no decompression); response bytes are returned verbatim.
//! - Verified TLS uses Mozilla/WebPKI roots only (parity with the current
//!   Reqwest default); insecure mode disables certificate **and** hostname
//!   verification and is armed once at construction (logged) while every
//!   dispatch still requires authority approval.
//!
//! Production direct load-test traffic uses this transport (first production
//! backend). Proxied load testing uses it where the route matrix above
//! allows; otherwise it fails closed with an explicit policy error (no
//! Reqwest fallback).
//!
//! Start here: [`EggfetchTransport`].

mod adapter;
mod mapping;

pub use adapter::EggfetchTransport;
