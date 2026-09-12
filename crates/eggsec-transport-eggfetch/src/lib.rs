//! Scope-aware outbound HTTP transport over `eggfetch-core` (Phase C).
//!
//! [`EggfetchTransport`] implements [`eggsec_transport::HttpTransport`] on top
//! of the published `eggfetch-core` client using only stable public APIs. No
//! upstream Eggfetch change was required; the TOCTOU-closed binding is
//! achieved by the adapter itself:
//!
//! - Every hostname hop is resolved through the caller-supplied
//!   [`eggsec_transport::TransportResolver`], approved by the mandatory
//!   [`eggsec_transport::NetworkAuthority`], validated with
//!   [`eggsec_transport::validate_binding`], and then **pinned**: the wire
//!   URL's host is rewritten to the approved IP literal, so the connector's
//!   own resolution can only return the authorized address. The logical
//!   hostname is preserved on the wire via the `Host` header and TLS SNI.
//! - Eggfetch automatic redirect following stays disabled
//!   (`follow_redirects(false)` + per-request `RedirectPolicy::new(false, 0)`).
//!   Each hop is computed with Eggfetch's public redirect primitive
//!   (`eggfetch_core::redirect::build_redirect_request`, which owns method
//!   rewrite, sensitive-header stripping, and body-replay rules) and is
//!   authorized through the full checkpoint sequence before dispatch.
//! - HTTP/3 is never constructed (the `http3` feature is off and both
//!   clients pin [`eggfetch_core::HttpVersionPolicy::Http1Only`]).
//! - Proxy/SOCKS execution is deferred: any non-`Direct`
//!   [`eggsec_transport::ProxyIntent`] fails closed at the `proxy`
//!   checkpoint after the authority has observed it. The `proxy` Cargo
//!   feature is enabled for one narrow reason only — without it the
//!   SNI-direct connector path builds no TLS connector, and the adapter
//!   needs that path (pinned-IP URL with SNI = logical hostname) for
//!   verified HTTPS. Proxy *routing* stays disabled by construction: no
//!   proxy is ever configured and every request sets `without_proxy`, so
//!   no environment or per-request proxy selection can divert a hop
//!   (remote-DNS SOCKS semantics remain unreachable).
//! - Retries are disabled in the backend (EggSec owns retry above the
//!   transport); every body in the contract is replayable by construction.
//! - Automatic decompression is off (parity: the current stack configures
//!   no decompression); response bytes are returned verbatim.
//! - Verified TLS uses Mozilla/WebPKI roots only (parity with the current
//!   Reqwest default); insecure mode disables certificate **and** hostname
//!   verification and is armed once at construction (logged) while every
//!   dispatch still requires authority approval.
//!
//! # Non-goals (Phase D)
//!
//! No production EggSec consumer uses this crate yet. Migration happens in
//! Phase D per consumer with focused parity tests; this crate only proves
//! the adapter against local fixtures here.
//!
//! Start here: [`EggfetchTransport`].

mod adapter;
mod mapping;

pub use adapter::EggfetchTransport;
