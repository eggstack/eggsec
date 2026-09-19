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
//!   [`eggsec_transport::validate_binding`], and then **pinned** to the
//!   singular socket-authorized address. Direct hops keep the logical URL as
//!   the request URL and supply exactly that address via
//!   `RequestBuilder::resolved_addresses` (no origin DNS inside Eggfetch;
//!   `eggfetch-core 0.1.7` reuses the Hyper H1/H2 client for equal
//!   logical-origin + ordered-address + SNI route keys). Proxied hops keep
//!   the logical URL and pin both legs via `Proxy::resolved_addresses`
//!   (proxy peer) + `proxy_target_addresses` (ultimate target, where
//!   enforceable). The logical hostname is preserved on the wire via the
//!   adapter-owned `Host` header and TLS SNI in both cases.
//! - Eggfetch automatic redirect following stays disabled
//!   (`follow_redirects(false)` + per-request `RedirectPolicy::new(false, 0)`).
//!   Each hop is computed with Eggfetch's public redirect primitive
//!   (`eggfetch_core::redirect::build_redirect_request`, which owns method
//!   rewrite, sensitive-header stripping, and body-replay rules) and is
//!   authorized through the full checkpoint sequence before dispatch.
//! - HTTP/3 is never constructed (the `http3` feature is off and both
//!   clients use [`eggfetch_core::HttpVersionPolicy::Auto`] with
//!   `allow_http3: false`; H1/H2 negotiate via ALPN).
//! - Timeouts map the remaining Eggsec aggregate request budget to
//!   `eggfetch_core::Timeout { total, connect }` per hop; under 0.1.7 `total`
//!   spans response-body EOF/trailers (never reset by chunks), so the manual
//!   redirect loop enforces one aggregate deadline without per-hop restart.
//! - Environment proxy discovery is never used (direct hops force
//!   `.without_proxy()`; proxied hops originate only from explicit
//!   `ProxyIntent`); environment discovery APIs do not appear
//!   in the adapter.
//! - Proxy routing uses the qualified `eggfetch-core 0.1.7` pinning release:
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
