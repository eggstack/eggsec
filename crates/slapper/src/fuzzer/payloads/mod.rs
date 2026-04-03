pub mod cache;
pub mod cmd;
pub mod compression;
pub mod csv;
pub mod deser;
pub mod graphql;
pub mod grpc;
pub mod headers;
pub mod host;
pub mod idor;
pub mod jwt;
pub mod ldap;
#[macro_use]
pub mod macros;
pub mod oauth;
pub mod redirect;
pub mod redos;
pub mod soap;
pub mod sqli;
pub mod ssrf;
pub mod ssti;
pub mod traversal;
pub mod websocket;
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
        use std::sync::LazyLock;
        static VARIANTS: LazyLock<Vec<PayloadType>> = LazyLock::new(|| PayloadType::iter().collect());
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

pub use crate::types::Severity;

static PAYLOAD_CACHE: LazyLock<std::collections::HashMap<PayloadType, Vec<Payload>>> =
    LazyLock::new(|| {
        let mut map = std::collections::HashMap::new();
        for pt in PayloadType::all_variants() {
            map.insert(*pt, get_payloads(*pt));
        }
        map
    });

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
        PayloadType::GraphQL => graphql::get_payloads(),
        PayloadType::OAuth => oauth::get_payloads(),
        PayloadType::Jwt => jwt::get_payloads(),
        PayloadType::Idor => idor::get_payloads(),
        PayloadType::Ssti => ssti::get_payloads(),
        PayloadType::Grpc => grpc::get_payloads(),
        PayloadType::Xxe => xxe::get_payloads(),
        PayloadType::Ldap => ldap::get_payloads(),
        PayloadType::Cmd => cmd::get_payloads(),
        PayloadType::Deser => deser::get_payloads(),
        PayloadType::Host => host::get_payloads(),
        PayloadType::Cache => cache::get_payloads(),
        PayloadType::Csv => csv::get_payloads(),
        PayloadType::Soap => soap::get_payloads(),
        PayloadType::Websocket => websocket::get_payloads(),
    }
}

pub fn get_payloads_cached(payload_type: PayloadType) -> &'static Vec<Payload> {
    PAYLOAD_CACHE.get(&payload_type).unwrap_or_else(|| {
        static EMPTY: LazyLock<Vec<Payload>> = LazyLock::new(Vec::new);
        &EMPTY
    })
}

#[deprecated(since = "0.1.0", note = "Use get_all_payloads_cached instead")]
pub fn get_all_payloads() -> Vec<Payload> {
    get_all_payloads_cached()
}

pub fn get_all_payloads_cached() -> Vec<Payload> {
    PAYLOAD_CACHE.values().flatten().cloned().collect()
}
