//! Phase C — `eggfetch` adapter interop through the canonical engine policy.
//!
//! Runs a focused slice of the Phase A behaviors through
//! [`eggsec::config::ScopeAuthority`] + [`EggfetchTransport`] against local
//! wiremock fixtures (no Internet). The exhaustive adapter-mechanics suite
//! lives in `crates/eggsec-transport-eggfetch/tests/parity.rs` with stub
//! authorities; this file proves the same checkpoints hold under the real
//! scope model (mixed answers deny, redirects stop, proxy stays distinct).

use eggsec::config::{Scope, ScopeAuthority, ScopeRule};
use eggsec_transport::{HttpTransport, InMemoryResolver, ProxyIntent, ScopedHttpRequest};
use eggsec_transport_eggfetch::EggfetchTransport;
use http::{Method, StatusCode};
use std::sync::Arc;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn scope_with_cidr(cidr: &str) -> Scope {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::with_cidr(cidr.to_string()).expect("cidr"));
    scope
}

fn scope_with_patterns(patterns: &[&str]) -> Scope {
    let mut scope = Scope::new();
    for p in patterns {
        scope.allowed_targets.push(ScopeRule::new(p.to_string()));
    }
    scope
}

fn get(url: &str) -> ScopedHttpRequest {
    ScopedHttpRequest::new_with_url(Method::GET, url).expect("request builds")
}

/// Rewrite a wiremock `http://127.0.0.1:PORT` URI to the logical `host`.
fn logical(uri: &str, host: &str) -> String {
    let port = Url::parse(uri).expect("uri").port().expect("port");
    format!("http://{host}:{port}")
}

#[tokio::test]
async fn scope_authority_allows_loopback_dispatch_through_adapter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;
    let scope = scope_with_cidr("127.0.0.0/8");
    let auth = ScopeAuthority::new(&scope);
    let resolver: Arc<dyn eggsec_transport::TransportResolver> =
        Arc::new(InMemoryResolver::new().with("test.local", vec!["127.0.0.1"]));
    let transport = EggfetchTransport::new(resolver);
    let url = format!("{}/a", logical(&server.uri(), "test.local"));
    let response = transport.execute(&auth, get(&url)).await.expect("allowed");
    assert_eq!(response.status, StatusCode::OK);
    let conn = response.connection.expect("connection");
    assert_eq!(
        conn.remote_addr.expect("addr").ip().to_string(),
        "127.0.0.1"
    );
}

#[tokio::test]
async fn scope_authority_denies_out_of_scope_dns() {
    let scope = scope_with_cidr("93.184.216.0/24");
    let auth = ScopeAuthority::new(&scope);
    let resolver: Arc<dyn eggsec_transport::TransportResolver> =
        Arc::new(InMemoryResolver::new().with("example.com", vec!["203.0.113.5"]));
    let transport = EggfetchTransport::new(resolver);
    let err = transport
        .execute(&auth, get("http://example.com/a"))
        .await
        .unwrap_err();
    assert!(err.is_denied());
}

#[tokio::test]
async fn scope_authority_denies_mixed_dns_answers() {
    let scope = scope_with_cidr("127.0.0.0/8");
    let auth = ScopeAuthority::new(&scope);
    let resolver: Arc<dyn eggsec_transport::TransportResolver> =
        Arc::new(InMemoryResolver::new().with("mixed.local", vec!["127.0.0.1", "203.0.113.99"]));
    let transport = EggfetchTransport::new(resolver);
    let err = transport
        .execute(&auth, get("http://mixed.local:80/"))
        .await
        .unwrap_err();
    assert!(err.is_denied());
}

#[tokio::test]
async fn same_host_only_surfaces_cross_host_redirect_under_scope() {
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    let landing = format!("{}/landing", logical(&second.uri(), "other.local"));
    Mock::given(method("GET"))
        .and(path("/go"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", landing.as_str()))
        .mount(&first)
        .await;
    let scope = scope_with_patterns(&["test.local", "other.local"]);
    let auth = ScopeAuthority::new(&scope);
    let resolver: Arc<dyn eggsec_transport::TransportResolver> = Arc::new(
        InMemoryResolver::new()
            .with("test.local", vec!["127.0.0.1"])
            .with("other.local", vec!["127.0.0.1"]),
    );
    let transport = EggfetchTransport::new(resolver);
    let url = format!("{}/go", logical(&first.uri(), "test.local"));
    let response = transport
        .execute(&auth, get(&url))
        .await
        .expect("redirect surfaces");
    // Default policy is same-host-only: the cross-host hop stops even
    // though scope would allow the target.
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(response.final_url.as_str(), url);
    assert!(response.redirect_history.is_empty());
}

#[tokio::test]
async fn proxy_intent_denied_even_when_scope_allows_both_ends() {
    let server = MockServer::start().await;
    let scope = scope_with_cidr("127.0.0.0/8");
    let auth = ScopeAuthority::new(&scope);
    let resolver: Arc<dyn eggsec_transport::TransportResolver> =
        Arc::new(InMemoryResolver::new().with("test.local", vec!["127.0.0.1"]));
    let transport = EggfetchTransport::new(resolver);
    let request = get(&logical(&server.uri(), "test.local")).with_proxy(ProxyIntent::Http {
        endpoint: Url::parse("http://127.0.0.1:9/").expect("proxy url"),
        credential: None,
    });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(
            err,
            eggsec_transport::TransportError::PolicyDenied {
                checkpoint: eggsec_transport::PolicyCheckpoint::Proxy,
                ..
            }
        ),
        "expected Proxy denial, got {err:?}"
    );
}
