//! Phase B — scoped transport contract closure over Phase A fixtures.
//!
//! Runs the Phase A scope/redirect/DNS behaviors through the
//! `eggsec-transport` fake + canonical [`eggsec::config::ScopeAuthority`]
//! binding (no Internet, deterministic fixtures). Proves the contract binds
//! authorized DNS results to the connection path, enforces per-hop redirect
//! and proxy-distinctness checks, keeps TLS orthogonal, and never leaks
//! secrets via `Debug`.

use eggsec::config::{Scope, ScopeAuthority, ScopeRule};
use eggsec_transport::{
    CannedResponse, HttpTransport, InMemoryResolver, NetworkAuthority, PolicyCheckpoint,
    RecordingFakeTransport, RedirectPolicy, ScopedHttpRequest, TlsPolicy, TransportError,
};
use http::{Method, StatusCode};
use std::net::IpAddr;
use std::sync::Arc;
use url::Url;

fn scope_with_patterns(patterns: &[&str]) -> Scope {
    let mut scope = Scope::new();
    for p in patterns {
        scope.allowed_targets.push(ScopeRule::new(p.to_string()));
    }
    scope
}

fn scope_with_cidr(cidr: &str) -> Scope {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::with_cidr(cidr.to_string()).expect("cidr"));
    scope
}

fn resolver() -> Arc<dyn eggsec_transport::TransportResolver> {
    Arc::new(
        InMemoryResolver::new()
            .with("example.com", vec!["93.184.216.34"])
            .with("a.example.com", vec!["93.184.216.34"])
            .with("b.example.com", vec!["93.184.216.35"])
            .with("proxy.internal", vec!["10.9.9.9"]),
    )
}

fn get(url: &str) -> ScopedHttpRequest {
    ScopedHttpRequest::new_with_url(Method::GET, url).expect("request builds")
}

#[tokio::test]
async fn authorized_destination_binds_to_approved_address() {
    let scope = scope_with_cidr("93.184.216.0/24");
    let auth = ScopeAuthority::new(&scope);
    let fake = RecordingFakeTransport::new(resolver());
    let resp = fake
        .execute(&auth, get("http://example.com/a"))
        .await
        .expect("allowed");
    assert_eq!(resp.status, StatusCode::OK);
    let hops = fake.hops();
    assert_eq!(hops.len(), 1);
    assert_eq!(hops[0].host, "example.com");
    assert_eq!(
        hops[0].selected_addr,
        Some("93.184.216.34".parse::<IpAddr>().expect("ip"))
    );
    assert_eq!(
        hops[0].checkpoint_order,
        vec![
            PolicyCheckpoint::InitialUrl,
            PolicyCheckpoint::Host,
            PolicyCheckpoint::Dns,
            PolicyCheckpoint::Socket,
            PolicyCheckpoint::TlsConsistency,
        ]
    );
    // Connection metadata reports the authorized binding, not a re-guess.
    let conn = resp.connection.expect("connection");
    assert_eq!(
        conn.remote_addr.expect("addr").ip(),
        "93.184.216.34".parse::<IpAddr>().expect("ip")
    );
}

#[tokio::test]
async fn out_of_scope_dns_denies_before_connect() {
    let scope = scope_with_cidr("93.184.216.0/24");
    let auth = ScopeAuthority::new(&scope);
    let r: Arc<dyn eggsec_transport::TransportResolver> =
        Arc::new(InMemoryResolver::new().with("example.com", vec!["203.0.113.5"]));
    let fake = RecordingFakeTransport::new(r);
    let err = fake
        .execute(&auth, get("http://example.com/a"))
        .await
        .unwrap_err();
    assert!(err.is_denied());
    assert!(fake.hops().is_empty());
}

#[tokio::test]
async fn mixed_dns_answers_deny() {
    let scope = scope_with_cidr("93.184.216.0/24");
    let auth = ScopeAuthority::new(&scope);
    let r: Arc<dyn eggsec_transport::TransportResolver> = Arc::new(
        InMemoryResolver::new().with("mixed.example", vec!["93.184.216.34", "203.0.113.99"]),
    );
    let fake = RecordingFakeTransport::new(r);
    let err = fake
        .execute(&auth, get("http://mixed.example/"))
        .await
        .unwrap_err();
    assert!(err.is_denied());
}

#[tokio::test]
async fn same_host_redirect_follows_and_cross_host_stops() {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::with_cidr("93.184.216.0/24".to_string()).expect("cidr"));
    let auth = ScopeAuthority::new(&scope);
    let fake = RecordingFakeTransport::new(resolver())
        .with_canned(
            "http://a.example.com/a",
            CannedResponse::redirect(StatusCode::FOUND, "/b"),
        )
        .with_canned("http://a.example.com/b", CannedResponse::ok("done"));
    let req = get("http://a.example.com/a")
        .with_redirect(RedirectPolicy::SameHostOnly { max_redirects: 5 });
    let resp = fake.execute(&auth, req).await.expect("same-host follows");
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.redirect_history.len(), 1);
    assert_eq!(fake.hop_count(), 2);

    let fake = RecordingFakeTransport::new(resolver()).with_canned(
        "http://a.example.com/a",
        CannedResponse::redirect(StatusCode::FOUND, "http://b.example.com/b"),
    );
    let req = get("http://a.example.com/a")
        .with_redirect(RedirectPolicy::SameHostOnly { max_redirects: 5 });
    let resp = fake.execute(&auth, req).await.expect("cross-host surfaces");
    assert_eq!(resp.status, StatusCode::FOUND);
    assert_eq!(fake.hop_count(), 1, "cross-host must not dispatch");
}

#[test]
fn redirect_to_out_of_scope_host_denied_by_authority() {
    let scope = scope_with_patterns(&["a.example.com"]);
    let auth = ScopeAuthority::new(&scope);
    let from = Url::parse("http://a.example.com/a").expect("url");
    let to = Url::parse("http://b.example.com/b").expect("url");
    assert!(auth.authorize_redirect(&from, &to).is_err());
    let to_in = Url::parse("http://a.example.com/b").expect("url");
    assert!(auth.authorize_redirect(&from, &to_in).is_ok());
}

#[test]
fn userinfo_rejected_and_redacted() {
    let scope = Scope::new();
    let auth = ScopeAuthority::new(&scope);
    let url = Url::parse("http://user:s3cr3t-pw@example.com/").expect("url");
    let err = auth.authorize_initial_url(&url).unwrap_err();
    assert!(matches!(err, TransportError::PolicyDenied { .. }));
    assert!(ScopedHttpRequest::new(Method::GET, url).is_err());
    let redacted = eggsec_transport::redact_url_for_debug(
        &Url::parse("http://user:s3cr3t-pw@example.com/").expect("url"),
    );
    assert!(!redacted.contains("s3cr3t-pw"));
}

#[tokio::test]
async fn secrets_never_leak_via_debug() {
    let scope = scope_with_cidr("93.184.216.0/24");
    let auth = ScopeAuthority::new(&scope);
    let fake = RecordingFakeTransport::new(resolver());
    let mut req = get("http://example.com/a");
    req.headers.insert(
        http::header::AUTHORIZATION,
        "Bearer s3cr3t".parse().expect("header"),
    );
    req.headers.insert(
        http::header::COOKIE,
        "session=abc123".parse().expect("header"),
    );
    let dbg = format!("{req:?}");
    assert!(!dbg.contains("s3cr3t"));
    assert!(!dbg.contains("abc123"));
    let resp = fake.execute(&auth, req).await.expect("exec");
    let resp_dbg = format!("{resp:?}");
    assert!(!resp_dbg.contains("s3cr3t"));
    let hops = fake.hops();
    assert!(hops[0].had_authorization);
    assert!(hops[0].had_cookie);
}

#[test]
fn direct_ip_checked_without_dns_bypass() {
    let scope = scope_with_cidr("10.0.0.0/8");
    let auth = ScopeAuthority::new(&scope);
    assert!(auth.authorize_host("10.0.0.5", Some(80), true).is_ok());
    assert!(auth.authorize_host("11.0.0.5", Some(80), true).is_err());
}

#[tokio::test]
async fn proxy_endpoint_and_target_are_distinct() {
    let scope = scope_with_patterns(&["example.com"]);
    let auth = ScopeAuthority::new(&scope);
    let proxy = Url::parse("http://proxy.internal:8080").expect("proxy");
    let ultimate = Url::parse("http://example.com/").expect("url");
    assert!(auth.authorize_proxy(&proxy, &ultimate).is_err());

    // Fake binds proxy IPs too: unauthorized proxy denies before dispatch.
    let fake = RecordingFakeTransport::new(resolver());
    let req = get("http://example.com/a").with_proxy(eggsec_transport::ProxyIntent::Http {
        endpoint: proxy,
        credential: None,
    });
    assert!(fake.execute(&auth, req).await.is_err());
    assert!(fake.hops().is_empty());
}

#[test]
fn insecure_tls_never_changes_authorization() {
    let scope = scope_with_cidr("93.184.216.0/24");
    let auth = ScopeAuthority::new(&scope);
    let evil = vec!["203.0.113.7".parse::<IpAddr>().expect("ip")];
    assert!(auth.authorize_resolved("evil.example", &evil).is_err());
    // TLS policy is orthogonal: verified vs insecure differ only in
    // verification flags, never in the verdict above.
    assert!(TlsPolicy::verified().is_verified());
    assert!(!TlsPolicy::insecure().is_verified());
    assert!(auth
        .check_tls_consistency("evil.example", None, None)
        .is_ok());
}

#[test]
fn authority_binding_rejects_invented_addresses() {
    struct EvilAuthority;
    impl NetworkAuthority for EvilAuthority {
        fn authorize_initial_url(&self, _u: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_host(
            &self,
            _h: &str,
            _p: Option<u16>,
            _l: bool,
        ) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_resolved(
            &self,
            _h: &str,
            _c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Ok(vec!["203.0.113.99".parse().expect("ip")])
        }
        fn authorize_socket(&self, _h: &str, _a: IpAddr, _p: u16) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_redirect(&self, _f: &Url, _t: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_proxy(&self, _p: &Url, _u: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn check_tls_consistency(
            &self,
            _h: &str,
            _s: Option<&str>,
            _o: Option<&str>,
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }
    let fake = RecordingFakeTransport::new(resolver());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt");
    let err = rt
        .block_on(fake.execute(&EvilAuthority, get("http://example.com/")))
        .unwrap_err();
    assert!(err.is_denied(), "invented address must fail closed: {err}");
}
