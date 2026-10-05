//! Attack payload corpora (extracted from `eggsec::fuzzer::payloads` in Phase G).
//!
//! Injection, traversal, deserialization, and protocol-abuse payload sets, plus
//! the [`PayloadType`] enum and [`Payload`] DTO they share.
//!
//! # What this crate is
//!
//! Payload **data**: it builds strings and returns them. It opens no socket and
//! performs no probing. Its only workspace dependency is `eggsec-core`, for
//! [`Severity`]; guard 148 forbids any I/O, transport, frontend, or engine
//! dependency.
//!
//! # The six advanced types are engine-owned
//!
//! [`PayloadType::GraphQL`], [`PayloadType::OAuth`], [`PayloadType::Jwt`],
//! [`PayloadType::Idor`], [`PayloadType::Ssti`], and [`PayloadType::Grpc`]
//! generate their payloads by making live `reqwest` requests, so those modules
//! stay in the `eggsec` engine and the engine dispatches them. This crate owns
//! the other 34. [`PayloadType::is_advanced`] is exactly that split.
//!
//! Calling [`get_payloads`] with an advanced type **panics** rather than
//! returning an empty vector. An empty vector would read as "this payload type
//! has no payloads", which is false — the payloads exist, they just need a live
//! probe. That silent-empty shape is the trap this design exists to avoid.
//!
//! Because of that split, the cross-variant caches
//! (`get_payloads_cached` / `get_all_payloads_cached`) are **not** in this crate:
//! building them requires all 40 variants, including the engine's. They live in
//! the engine, which owns the union.
//!
//! # Laziness is load-bearing
//!
//! Payload construction is deliberately not cached here. The engine holds the
//! `LazyLock` caches that span all 40 variants, so a process that never asks
//! for a payload never pays to build one.

pub mod cache;
pub mod compression;
pub mod csv;
pub mod deser;
pub mod expression;
pub mod headers;
pub mod host;
pub mod ldap;
#[macro_use]
pub mod macros;
pub mod cmd;
pub mod css_inject;
pub mod dep_confusion;
pub mod dom_clobber;
pub mod html_inject;
pub mod latex;
pub mod mass_assign;
pub mod nosql;
pub mod oast;
pub mod prototype;
pub mod race;
pub mod redirect;
pub mod redos;
pub mod saml;
pub mod soap;
pub mod sqli;
pub mod ssi;
pub mod ssrf;
pub mod traversal;
pub mod viewstate;
pub mod websocket;
pub mod xpath;
pub mod xs_leak;
pub mod xslt;
pub mod xss;
pub mod xxe;

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum PayloadType {
    Sqli,
    Xss,
    Traversal,
    Ssrf,
    Redirect,
    Redos,
    Headers,
    Compression,
    GraphQL,
    OAuth,
    Jwt,
    Idor,
    Ssti,
    Grpc,
    Xxe,
    Ldap,
    Cmd,
    Deser,
    Host,
    Cache,
    Csv,
    Soap,
    Websocket,
    Nosql,
    Xpath,
    Expression,
    Prototype,
    Race,
    MassAssign,
    Oast,
    Saml,
    HtmlInject,
    CssInject,
    Ssi,
    DomClobber,
    Xslt,
    Viewstate,
    DepConfusion,
    XsLeak,
    Latex,
}

impl std::fmt::Display for PayloadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayloadType::Sqli => write!(f, "SQL Injection"),
            PayloadType::Xss => write!(f, "XSS"),
            PayloadType::Traversal => write!(f, "Path Traversal"),
            PayloadType::Ssrf => write!(f, "SSRF"),
            PayloadType::Redirect => write!(f, "Open Redirect"),
            PayloadType::Redos => write!(f, "ReDoS"),
            PayloadType::Headers => write!(f, "Header Expansion"),
            PayloadType::Compression => write!(f, "Compression Bomb"),
            PayloadType::GraphQL => write!(f, "GraphQL"),
            PayloadType::OAuth => write!(f, "OAuth/OIDC"),
            PayloadType::Jwt => write!(f, "JWT"),
            PayloadType::Idor => write!(f, "IDOR"),
            PayloadType::Ssti => write!(f, "SSTI"),
            PayloadType::Grpc => write!(f, "gRPC"),
            PayloadType::Xxe => write!(f, "XXE"),
            PayloadType::Ldap => write!(f, "LDAP Injection"),
            PayloadType::Cmd => write!(f, "Command Injection"),
            PayloadType::Deser => write!(f, "Deserialization"),
            PayloadType::Host => write!(f, "Host Header Injection"),
            PayloadType::Cache => write!(f, "Cache Poisoning"),
            PayloadType::Csv => write!(f, "CSV Injection"),
            PayloadType::Soap => write!(f, "SOAP/XML"),
            PayloadType::Websocket => write!(f, "WebSocket"),
            PayloadType::Nosql => write!(f, "NoSQL Injection"),
            PayloadType::Xpath => write!(f, "XPath Injection"),
            PayloadType::Expression => write!(f, "Expression Injection"),
            PayloadType::Prototype => write!(f, "Prototype Pollution"),
            PayloadType::Race => write!(f, "Race Condition"),
            PayloadType::MassAssign => write!(f, "Mass Assignment"),
            PayloadType::Oast => write!(f, "OAST"),
            PayloadType::Saml => write!(f, "SAML"),
            PayloadType::HtmlInject => write!(f, "HTML Injection"),
            PayloadType::CssInject => write!(f, "CSS Injection"),
            PayloadType::Ssi => write!(f, "SSI Injection"),
            PayloadType::DomClobber => write!(f, "DOM Clobbering"),
            PayloadType::Xslt => write!(f, "XSLT Injection"),
            PayloadType::Viewstate => write!(f, "ViewState Deserialization"),
            PayloadType::DepConfusion => write!(f, "Dependency Confusion"),
            PayloadType::XsLeak => write!(f, "XS-Leak"),
            PayloadType::Latex => write!(f, "LaTeX Injection"),
        }
    }
}

impl PayloadType {
    pub fn is_advanced(&self) -> bool {
        matches!(
            self,
            PayloadType::GraphQL
                | PayloadType::OAuth
                | PayloadType::Jwt
                | PayloadType::Idor
                | PayloadType::Ssti
                | PayloadType::Grpc
        )
    }

    pub fn all_variants() -> &'static [PayloadType] {
        static VARIANTS: LazyLock<Vec<PayloadType>> =
            LazyLock::new(|| PayloadType::iter().collect());
        &VARIANTS
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub payload_type: PayloadType,
    pub payload: String,
    pub description: String,
    pub severity: Severity,
    pub tags: Vec<String>,
}

pub use eggsec_core::types::Severity;

/// The six advanced payload types are **not** owned by this crate.
///
/// `GraphQL`, `OAuth`, `Jwt`, `Idor`, `Ssti`, and `Grpc` generate payloads by
/// performing live probing with `reqwest`, so those modules stay in the
/// `eggsec` engine and the engine dispatches them itself. This crate cannot
/// resolve them.
///
/// It fails loudly rather than returning an empty `Vec`, because a silent empty
/// result reads as "this payload type has no payloads" — which is false, and
/// would be a capability regression rather than an error. Filter with
/// [`PayloadType::is_advanced`] before calling [`get_payloads`] when iterating
/// [`PayloadType::all_variants`].
fn engine_owned_probe_payloads(payload_type: PayloadType) -> Vec<Payload> {
    unreachable!(
        "PayloadType::{payload_type:?} is engine-owned: its payloads are generated by live \
         reqwest probing and live in `eggsec::fuzzer::payloads`. Call the engine's \
         `get_payloads`, or skip advanced types with PayloadType::is_advanced()."
    )
}

/// Build the payloads for one [`PayloadType`].
///
/// Panics for the six advanced types; see [`engine_owned_probe_payloads`].
pub fn get_payloads(payload_type: PayloadType) -> Vec<Payload> {
    match payload_type {
        PayloadType::Sqli => sqli::get_payloads(),
        PayloadType::Xss => xss::get_payloads(),
        PayloadType::Traversal => traversal::get_payloads(),
        PayloadType::Ssrf => ssrf::get_payloads(),
        PayloadType::Redirect => redirect::get_payloads(),
        PayloadType::Redos => redos::get_payloads(),
        PayloadType::Headers => headers::get_payloads(),
        PayloadType::Compression => compression::get_payloads(),
        PayloadType::GraphQL => engine_owned_probe_payloads(PayloadType::GraphQL),
        PayloadType::OAuth => engine_owned_probe_payloads(PayloadType::OAuth),
        PayloadType::Jwt => engine_owned_probe_payloads(PayloadType::Jwt),
        PayloadType::Idor => engine_owned_probe_payloads(PayloadType::Idor),
        PayloadType::Ssti => engine_owned_probe_payloads(PayloadType::Ssti),
        PayloadType::Grpc => engine_owned_probe_payloads(PayloadType::Grpc),
        PayloadType::Xxe => xxe::get_payloads(),
        PayloadType::Ldap => ldap::get_payloads(),
        PayloadType::Cmd => cmd::get_payloads(),
        PayloadType::Deser => deser::get_payloads(),
        PayloadType::Host => host::get_payloads(),
        PayloadType::Cache => cache::get_payloads(),
        PayloadType::Csv => csv::get_payloads(),
        PayloadType::Soap => soap::get_payloads(),
        PayloadType::Websocket => websocket::get_payloads(),
        PayloadType::Nosql => nosql::get_payloads(),
        PayloadType::Xpath => xpath::get_payloads(),
        PayloadType::Expression => expression::get_payloads(),
        PayloadType::Prototype => prototype::get_payloads(),
        PayloadType::Race => race::get_payloads(),
        PayloadType::MassAssign => mass_assign::get_payloads(),
        PayloadType::Oast => oast::get_oast_payloads(),
        PayloadType::Saml => saml::get_payloads(),
        PayloadType::HtmlInject => html_inject::get_payloads(),
        PayloadType::CssInject => css_inject::get_payloads(),
        PayloadType::DomClobber => dom_clobber::get_payloads(),
        PayloadType::Ssi => ssi::get_payloads(),
        PayloadType::Xslt => xslt::get_payloads(),
        PayloadType::Viewstate => viewstate::get_payloads(),
        PayloadType::Latex => latex::get_payloads(),
        PayloadType::DepConfusion => dep_confusion::get_payloads(),
        PayloadType::XsLeak => xs_leak::get_payloads(),
    }
}
