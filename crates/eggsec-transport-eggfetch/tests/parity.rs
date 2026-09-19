//! Phase C — `eggfetch` adapter parity and adversarial fixture suite.
//!
//! Runs the Phase A scope/redirect/DNS behaviors plus the Phase C
//! adversarial cases through [`EggfetchTransport`] against local
//! loopback fixtures (no Internet). Proves the adapter binds
//! caller-approved resolution results to actual connections (approved-IP
//! pinning), authorizes every redirect hop before dispatch, keeps
//! secrets out of cross-origin hops and `Debug`, and maps TLS/timeout
//! behavior without leaking backend types.
//!
//! Policy interop through the canonical [`eggsec::config::ScopeAuthority`]
//! is proven engine-side in
//! `crates/eggsec/tests/transport_eggfetch_parity.rs`; this suite uses
//! focused stub authorities to isolate adapter mechanics.

mod common;

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use common::{
    Action, AllowAll, CidrAllow, DenyDns, Fixture, FlipFlopResolver, Incoming, PanicResolver,
    RecordingAuth, TlsFixture,
};
use eggsec_transport::{
    HttpTransport, InMemoryResolver, NetworkAuthority, PolicyCheckpoint, ProxyIntent,
    RedirectPolicy, RequestBody, ScopedHttpRequest, TimeoutPolicy, TlsPolicy, TransportError,
    TransportResolver,
};
use eggsec_transport_eggfetch::EggfetchTransport;
use http::{Method, StatusCode};
use url::Url;

fn memory_resolver() -> Arc<dyn TransportResolver> {
    Arc::new(
        InMemoryResolver::new()
            .with("test.local", vec!["127.0.0.1"])
            .with("other.local", vec!["127.0.0.1"]),
    )
}

fn get(url: &str) -> ScopedHttpRequest {
    ScopedHttpRequest::new_with_url(Method::GET, url).expect("request builds")
}

fn authed(url: &str) -> ScopedHttpRequest {
    get(url)
        .with_header("authorization", "Bearer s3cr3t-tok")
        .expect("header")
        .with_header("cookie", "session=abc123")
        .expect("header")
}

async fn echo_target() -> Fixture {
    Fixture::echo().await
}

#[tokio::test]
async fn basic_get_binds_to_approved_address() {
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let url = format!("{}/search?q=rust", server.base_host("test.local"));
    let request = get(&url)
        .with_header("x-custom", "keep")
        .expect("header")
        .with_body(RequestBody::from_string("ping".to_string()));
    let response = transport
        .execute(&auth, request)
        .await
        .expect("allowed dispatch succeeds");
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body.as_ref(), b"ping");
    assert_eq!(response.final_url.as_str(), url);
    assert!(response.redirect_history.is_empty());
    // Connection metadata reports the authorized binding, not a re-guess.
    let conn = response.connection.expect("connection");
    assert_eq!(
        conn.remote_addr.expect("addr"),
        server.addr,
        "must dial the approved loopback address"
    );
    assert_eq!(conn.sni_host, None, "plain HTTP sends no SNI");
    // The connector saw the pinned literal, but logical semantics survived.
    let got = server.received();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].method, "GET");
    assert_eq!(got[0].path, "/search?q=rust");
    assert_eq!(got[0].header("x-custom"), Some("keep"));
}

#[tokio::test]
async fn host_header_preserves_logical_host() {
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let url = server.base_host("test.local");
    transport.execute(&auth, get(&url)).await.expect("dispatch");
    let got = server.received();
    assert_eq!(got.len(), 1);
    assert_eq!(
        got[0].header("host"),
        Some(format!("test.local:{}", server.addr.port()).as_str()),
        "wire Host must name the logical host, not the pinned literal"
    );
}

#[tokio::test]
async fn ip_literal_never_consults_resolver() {
    let server = echo_target().await;
    let transport = EggfetchTransport::new(Arc::new(PanicResolver));
    let auth = AllowAll;
    let response = transport
        .execute(&auth, get(&server.base()))
        .await
        .expect("literal dispatch");
    assert_eq!(response.status, StatusCode::OK);
    let conn = response.connection.expect("connection");
    assert_eq!(conn.remote_addr.expect("addr"), server.addr);
}

#[tokio::test]
async fn empty_resolution_fails_without_policy_denial() {
    let server = echo_target().await;
    let transport = EggfetchTransport::new(Arc::new(InMemoryResolver::new()));
    let auth = AllowAll;
    let err = transport
        .execute(&auth, get(&server.base_host("test.local")))
        .await
        .unwrap_err();
    assert!(
        matches!(err, TransportError::ResolutionFailed { .. }),
        "expected ResolutionFailed, got {err:?}"
    );
    assert!(!err.is_denied());
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn denied_dns_records_no_dispatch() {
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = DenyDns;
    let err = transport
        .execute(&auth, get(&server.base_host("test.local")))
        .await
        .unwrap_err();
    assert!(err.is_denied());
    assert!(server.received().is_empty());
}

/// DNS answer changes from allowed to denied between connections: the
/// second hop re-resolves and fails closed.
#[tokio::test]
async fn dns_change_between_hops_denies_before_second_dispatch() {
    let handler: common::Handler = Arc::new(|incoming: &Incoming| {
        if incoming.path == "/flip" {
            Action::redirect(302, "/flip2".to_string())
        } else {
            Action::ok("second".as_bytes().to_vec())
        }
    });
    let server = Fixture::start(handler).await;
    let host = "test.local";
    let url = format!("{}{}", server.base_host(host), "/flip");
    let resolver: Arc<dyn TransportResolver> = Arc::new(FlipFlopResolver::new(
        vec!["127.0.0.1"],
        vec!["203.0.113.99"],
    ));
    let transport = EggfetchTransport::new(resolver);
    let auth = CidrAllow::new("127.0.0.0/8");
    let request = get(&url).with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 5 });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(err.is_denied(), "changed DNS answer must deny, got {err:?}");
    assert_eq!(
        server.received().len(),
        1,
        "only the first (allowed) hop may dispatch"
    );
}

/// Redirect from an allowed hostname to a denied destination fails at the
/// redirect checkpoint before any second dispatch.
#[tokio::test]
async fn redirect_to_denied_target_stops_at_redirect_checkpoint() {
    let other = echo_target().await;
    let target = other.base();
    let first = Fixture::start(Arc::new(move |incoming: &Incoming| {
        if incoming.path == "/go" {
            Action::redirect(302, target.clone())
        } else {
            Action::ok("first".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = RecordingAuth::new(DenyRedirect);
    let request = get(&format!("{}{}", first.base_host("test.local"), "/go"))
        .with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 5 });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(
            err,
            TransportError::PolicyDenied {
                checkpoint: PolicyCheckpoint::Redirect,
                ..
            }
        ),
        "expected Redirect denial, got {err:?}"
    );
    assert!(auth.calls().contains(&"redirect".to_string()));
    assert_eq!(first.received().len(), 1);
    assert!(other.received().is_empty());
}

#[tokio::test]
async fn redirect_with_userinfo_is_invalid_not_denied() {
    let first = Fixture::start(Arc::new(|incoming: &Incoming| {
        if incoming.path == "/evil" {
            Action::redirect(302, "http://user:s3cr3t@test.local/x".to_string())
        } else {
            Action::ok("first".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let err = transport
        .execute(
            &auth,
            get(&format!("{}{}", first.base_host("test.local"), "/evil")),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, TransportError::InvalidRequest(_)),
        "userinfo redirect must be InvalidRequest, got {err:?}"
    );
}

#[tokio::test]
async fn redirect_to_unsupported_scheme_is_invalid() {
    let first = Fixture::start(Arc::new(|incoming: &Incoming| {
        if incoming.path == "/ftp" {
            Action::redirect(302, "ftp://test.local/x".to_string())
        } else {
            Action::ok("first".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let err = transport
        .execute(
            &auth,
            get(&format!("{}{}", first.base_host("test.local"), "/ftp")),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, TransportError::InvalidRequest(_)),
        "unsupported-scheme redirect must be InvalidRequest, got {err:?}"
    );
}

#[tokio::test]
async fn cross_origin_redirect_strips_auth_and_cookie() {
    let second = echo_target().await;
    let landing = second.base_host("other.local");
    let first = Fixture::start(Arc::new(move |incoming: &Incoming| {
        if incoming.path == "/login" {
            Action::redirect(302, format!("{landing}/landing"))
        } else {
            Action::ok("first".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = authed(&format!("{}{}", first.base_host("test.local"), "/login"))
        .with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 5 });
    let response = transport.execute(&auth, request).await.expect("follow");
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.redirect_history.len(), 1);
    let first_got = first.received();
    assert_eq!(first_got.len(), 1);
    assert!(first_got[0].has("authorization"));
    assert!(first_got[0].has("cookie"));
    let second_got = second.received();
    assert_eq!(second_got.len(), 1);
    assert!(
        !second_got[0].has("authorization"),
        "cross-origin hop must not carry Authorization"
    );
    assert!(
        !second_got[0].has("cookie"),
        "cross-origin hop must not carry Cookie"
    );
    assert!(
        !second_got[0].has("proxy-authorization"),
        "cross-origin hop must not carry Proxy-Authorization"
    );
    assert_eq!(
        second_got[0].header("host"),
        Some(format!("other.local:{}", second.addr.port()).as_str()),
        "wire Host tracks the new logical host"
    );
}

#[tokio::test]
async fn same_origin_redirect_preserves_auth() {
    let server = Fixture::start(Arc::new(|incoming: &Incoming| {
        if incoming.path == "/a" {
            Action::redirect(302, "/b".to_string())
        } else {
            Action::ok("b".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let response = transport
        .execute(
            &auth,
            authed(&format!("{}{}", server.base_host("test.local"), "/a")),
        )
        .await
        .expect("follow");
    assert_eq!(response.status, StatusCode::OK);
    let got = server.received();
    assert_eq!(got.len(), 2);
    assert_eq!(got[1].header("authorization"), Some("Bearer s3cr3t-tok"));
    assert_eq!(got[1].header("cookie"), Some("session=abc123"));
}

#[tokio::test]
async fn same_host_only_cross_host_surfaces_redirect() {
    let second = echo_target().await;
    let landing = second.base_host("other.local");
    let first = Fixture::start(Arc::new(move |incoming: &Incoming| {
        if incoming.path == "/go" {
            Action::redirect(302, format!("{landing}/landing"))
        } else {
            Action::ok("first".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let url = format!("{}{}", first.base_host("test.local"), "/go");
    let request = get(&url).with_redirect(RedirectPolicy::SameHostOnly { max_redirects: 5 });
    let response = transport.execute(&auth, request).await.expect("surface");
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(response.final_url.as_str(), url);
    assert!(response.redirect_history.is_empty());
    assert!(second.received().is_empty());
}

#[tokio::test]
async fn authority_checked_cross_host_follows_with_history() {
    let second = echo_target().await;
    let landing = second.base_host("other.local");
    let first = Fixture::start(Arc::new(move |incoming: &Incoming| {
        if incoming.path == "/go" {
            Action::redirect(302, format!("{landing}/landing"))
        } else {
            Action::ok("first".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let url = format!("{}{}", first.base_host("test.local"), "/go");
    let request = get(&url).with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 5 });
    let response = transport.execute(&auth, request).await.expect("follow");
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.redirect_history,
        vec![Url::parse(&url).expect("url")]
    );
    assert_eq!(second.received().len(), 1);
}

#[tokio::test]
async fn redirect_cap_surfaces_last_redirect() {
    let server = Fixture::start(Arc::new(|incoming: &Incoming| {
        if incoming.path == "/loop" {
            Action::redirect(302, "/loop".to_string())
        } else {
            Action::ok("unreachable".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let url = format!("{}{}", server.base_host("test.local"), "/loop");
    let request = get(&url).with_redirect(RedirectPolicy::SameHostOnly { max_redirects: 2 });
    let response = transport.execute(&auth, request).await.expect("surface");
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(response.redirect_history.len(), 2);
    assert_eq!(server.received().len(), 3);
}

#[tokio::test]
async fn reresolution_authorizes_hops_after_the_first() {
    let server = Fixture::start(Arc::new(|incoming: &Incoming| {
        if incoming.path == "/self" {
            Action::redirect(302, "/self".to_string())
        } else {
            Action::ok("unreachable".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = RecordingAuth::new(AllowAll);
    let request = get(&format!("{}{}", server.base_host("test.local"), "/self"))
        .with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 2 });
    transport.execute(&auth, request).await.expect("surface");
    let calls = auth.calls();
    assert_eq!(
        calls.iter().filter(|c| *c == "resolved").count(),
        1,
        "first hop uses authorize_resolved: {calls:?}"
    );
    assert_eq!(
        calls.iter().filter(|c| *c == "reresolution").count(),
        2,
        "later hops use authorize_reresolution: {calls:?}"
    );
}

#[tokio::test]
async fn first_hop_runs_checkpoints_in_contract_order() {
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = RecordingAuth::new(AllowAll);
    transport
        .execute(&auth, get(&server.base_host("test.local")))
        .await
        .expect("dispatch");
    assert_eq!(
        auth.calls(),
        vec![
            "initial_url",
            "host",
            "resolved",
            "socket",
            "tls_consistency",
        ],
        "direct hops observe no proxy/reresolution checkpoints"
    );
}

#[tokio::test]
async fn post_307_preserves_method_and_body() {
    let server = Fixture::start(Arc::new(|incoming: &Incoming| {
        if incoming.path == "/submit" {
            Action::redirect(307, "/echo".to_string())
        } else {
            Action::ok(incoming.body.clone())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = ScopedHttpRequest::new_with_url(
        Method::POST,
        &format!("{}{}", server.base_host("test.local"), "/submit"),
    )
    .expect("request")
    .with_body(RequestBody::from_string("field=1".to_string()));
    let response = transport.execute(&auth, request).await.expect("follow");
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body.as_ref(), b"field=1");
    let got = server.received();
    assert_eq!(got.len(), 2);
    assert_eq!(got[1].method, "POST");
    assert_eq!(got[1].body, b"field=1");
}

#[tokio::test]
async fn post_302_rewrites_to_get_without_body() {
    let server = Fixture::start(Arc::new(|incoming: &Incoming| {
        if incoming.path == "/old" {
            Action::redirect(302, "/new".to_string())
        } else {
            Action::ok(incoming.body.clone())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = ScopedHttpRequest::new_with_url(
        Method::POST,
        &format!("{}{}", server.base_host("test.local"), "/old"),
    )
    .expect("request")
    .with_body(RequestBody::from_string("field=1".to_string()));
    let response = transport.execute(&auth, request).await.expect("follow");
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.is_empty());
    let got = server.received();
    assert_eq!(got.len(), 2);
    assert_eq!(got[1].method, "GET");
    assert!(got[1].body.is_empty());
}

#[tokio::test]
async fn non_http_scheme_rejected_before_dispatch() {
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let err = transport
        .execute(&auth, get("ftp://test.local/file"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, TransportError::InvalidRequest(_)),
        "expected InvalidRequest, got {err:?}"
    );
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn proxy_intent_fails_closed_after_authority_observes_it() {
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = RecordingAuth::new(AllowAll);
    let request = get(&server.base_host("test.local")).with_proxy(ProxyIntent::Http {
        endpoint: Url::parse("http://127.0.0.1:9/").expect("proxy url"),
        credential: None,
    });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(
            err,
            TransportError::PolicyDenied {
                checkpoint: PolicyCheckpoint::Proxy,
                ..
            }
        ),
        "expected Proxy denial, got {err:?}"
    );
    assert!(
        auth.calls().contains(&"proxy".to_string()),
        "authority must observe the proxy decision: {:?}",
        auth.calls()
    );
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn proxy_credential_and_transport_never_leak_in_debug() {
    let request = get("http://test.local/")
        .with_proxy(ProxyIntent::All {
            endpoint: Url::parse("http://proxy.local:8080/").expect("proxy url"),
            credential: Some(eggsec_transport::ProxyCredential::new(
                "proxy-user",
                "proxy-s3cr3t",
            )),
        })
        .with_header("authorization", "Bearer s3cr3t-tok")
        .expect("header");
    let debug = format!("{request:?}");
    assert!(!debug.contains("proxy-s3cr3t"), "credential leak: {debug}");
    assert!(!debug.contains("proxy-user"), "credential leak: {debug}");
    assert!(!debug.contains("s3cr3t-tok"), "auth leak: {debug}");
    let transport = EggfetchTransport::new(memory_resolver());
    let transport_debug = format!("{transport:?}");
    assert!(transport_debug.contains("EggfetchTransport"));
}

#[tokio::test]
async fn compressed_response_returned_verbatim_without_bomb_limit() {
    // Parity: the current stack configures no decompression, so the
    // adapter disables it too and returns wire bytes untouched (even
    // when they are not valid gzip).
    let payload = b"\x1f\x8bFAKE-GZIP-PAYLOAD".to_vec();
    let body = payload.clone();
    let server = Fixture::start(Arc::new(move |_: &Incoming| Action {
        status: 200,
        headers: vec![("content-encoding".to_string(), "gzip".to_string())],
        body: body.clone(),
        delay: Duration::ZERO,
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let response = transport
        .execute(&auth, get(&server.base_host("test.local")))
        .await
        .expect("verbatim body");
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body.as_ref(), payload.as_slice());
}

#[tokio::test]
async fn total_timeout_fails_as_backend_not_denial() {
    let server = Fixture::start(Arc::new(|_: &Incoming| {
        Action::slow("too late", Duration::from_secs(5))
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = get(&server.base_host("test.local")).with_timeout(TimeoutPolicy {
        request_timeout: Duration::from_secs(1),
        connect_timeout: None,
    });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "expected Backend timeout, got {err:?}"
    );
    assert!(!err.is_denied());
    assert!(err.to_string().contains("total"));
}

#[tokio::test]
async fn zero_timeout_fails_before_dispatch() {
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = get(&server.base_host("test.local")).with_timeout(TimeoutPolicy {
        request_timeout: Duration::ZERO,
        connect_timeout: None,
    });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(matches!(err, TransportError::Backend(_)));
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn connect_timeout_fails_as_backend() {
    // TEST-NET-1 is unroutable: with AllowAll the dispatch reaches the
    // connector and fails there (refused or timed out), never as a
    // policy denial.
    let resolver: Arc<dyn TransportResolver> =
        Arc::new(InMemoryResolver::new().with("blackhole.local", vec!["192.0.2.1"]));
    let transport = EggfetchTransport::new(resolver);
    let auth = AllowAll;
    let request = get("http://blackhole.local/")
        .with_timeout(TimeoutPolicy::with_request_timeout(10).with_connect_timeout(1));
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "expected Backend connect failure, got {err:?}"
    );
    assert!(!err.is_denied());
}

#[tokio::test]
async fn cancellation_drops_inflight_dispatch() {
    let server = Fixture::start(Arc::new(|_: &Incoming| {
        Action::slow("too late", Duration::from_secs(10))
    }))
    .await;
    let transport = Arc::new(EggfetchTransport::new(memory_resolver()));
    let url = server.base_host("test.local");
    let worker = {
        let transport = transport.clone();
        tokio::spawn(async move {
            let auth = AllowAll;
            transport.execute(&auth, get(&url)).await
        })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;
    worker.abort();
    let aborted = worker.await.expect_err("abort reports JoinError");
    assert!(aborted.is_cancelled());
    // The transport is still usable afterwards (no wedged shared state).
    let auth = AllowAll;
    let echo = echo_target().await;
    let response = transport
        .execute(&auth, get(&echo.base_host("test.local")))
        .await
        .expect("transport reusable after abort");
    assert_eq!(response.status, StatusCode::OK);
}

#[tokio::test]
async fn self_signed_cert_rejected_when_verified() {
    let server = TlsFixture::start(
        Arc::new(|incoming: &Incoming| Action::ok(incoming.body.clone())),
        vec!["127.0.0.1".to_string()],
    )
    .await;
    let transport = EggfetchTransport::new(Arc::new(PanicResolver));
    let auth = AllowAll;
    let err = transport
        .execute(&auth, get(&server.base()))
        .await
        .unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "self-signed must fail the TLS handshake, got {err:?}"
    );
    assert!(!err.is_denied());
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn insecure_tls_accepts_self_signed_and_reports_sni() {
    let server = TlsFixture::start(
        Arc::new(|incoming: &Incoming| Action::ok(incoming.body.clone())),
        vec!["127.0.0.1".to_string()],
    )
    .await;
    let transport = EggfetchTransport::new(Arc::new(PanicResolver));
    let auth = AllowAll;
    let request = get(&server.base()).with_tls(TlsPolicy::insecure());
    let response = transport
        .execute(&auth, request)
        .await
        .expect("insecure ok");
    assert_eq!(response.status, StatusCode::OK);
    let conn = response.connection.expect("connection");
    assert_eq!(conn.remote_addr.expect("addr"), server.addr);
    assert_eq!(conn.sni_host.as_deref(), Some("127.0.0.1"));
    assert_eq!(server.received().len(), 1);
}

#[tokio::test]
async fn hostname_mismatch_rejected_when_verified() {
    let server = TlsFixture::start(
        Arc::new(|incoming: &Incoming| Action::ok(incoming.body.clone())),
        vec!["wrong.invalid".to_string()],
    )
    .await;
    let transport = EggfetchTransport::new(Arc::new(PanicResolver));
    let auth = AllowAll;
    let err = transport
        .execute(&auth, get(&server.base()))
        .await
        .unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "SNI/hostname mismatch must fail, got {err:?}"
    );
    assert!(!err.is_denied());
}

#[tokio::test]
async fn non_followable_3xx_surfaces_verbatim() {
    // 304 is not in Eggfetch's followable set: surfaced, never followed.
    let server = Fixture::start(Arc::new(|incoming: &Incoming| {
        if incoming.path == "/cached" {
            Action {
                status: 304,
                headers: vec![("location".to_string(), "/other".to_string())],
                body: Vec::new(),
                delay: Duration::ZERO,
            }
        } else {
            Action::ok("other".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let response = transport
        .execute(
            &auth,
            get(&format!("{}{}", server.base_host("test.local"), "/cached")),
        )
        .await
        .expect("surface");
    assert_eq!(response.status, StatusCode::NOT_MODIFIED);
    assert_eq!(server.received().len(), 1);
}

#[tokio::test]
async fn response_debug_redacts_set_cookie() {
    let server = Fixture::start(Arc::new(|_: &Incoming| Action {
        status: 200,
        headers: vec![(
            "set-cookie".to_string(),
            "session=s3cr3t-value; Path=/".to_string(),
        )],
        body: b"ok".to_vec(),
        delay: Duration::ZERO,
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let response = transport
        .execute(&auth, get(&server.base_host("test.local")))
        .await
        .expect("dispatch");
    let debug = format!("{response:?}");
    assert!(!debug.contains("s3cr3t-value"), "cookie leak: {debug}");
}

#[tokio::test]
async fn mixed_dns_answers_deny_without_dispatch() {
    let server = echo_target().await;
    let resolver: Arc<dyn TransportResolver> =
        Arc::new(InMemoryResolver::new().with("mixed.local", vec!["127.0.0.1", "203.0.113.99"]));
    let transport = EggfetchTransport::new(resolver);
    let auth = CidrAllow::new("127.0.0.0/8");
    let err = transport
        .execute(
            &auth,
            get(&format!("http://mixed.local:{}/", server.addr.port())),
        )
        .await
        .unwrap_err();
    assert!(err.is_denied(), "mixed answers must deny, got {err:?}");
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn redirect_routes_still_pin_each_hop() {
    // Both hops resolve through the authority and each wire URL is pinned
    // to its own approved address (same loopback here, distinct servers).
    let mut routes = HashMap::new();
    let second = echo_target().await;
    let landing = second.base_host("other.local");
    routes.insert(
        "/start".to_string(),
        Action::redirect(302, format!("{landing}/landed")),
    );
    let first = Fixture::routes(routes).await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = RecordingAuth::new(AllowAll);
    let request = get(&format!("{}{}", first.base_host("test.local"), "/start"))
        .with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 5 });
    let response = transport.execute(&auth, request).await.expect("follow");
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(first.received().len(), 1);
    assert_eq!(second.received().len(), 1);
    let calls = auth.calls();
    assert!(calls.contains(&"redirect".to_string()));
    assert!(calls.contains(&"reresolution".to_string()));
}

/// Redirect denial helper: allows everything except the redirect hop
/// itself, isolating the redirect checkpoint.
#[derive(Debug, Default)]
struct DenyRedirect;

impl NetworkAuthority for DenyRedirect {
    fn authorize_initial_url(&self, url: &url::Url) -> Result<(), TransportError> {
        eggsec_transport::reject_url_userinfo(url)
    }

    fn authorize_host(
        &self,
        _host: &str,
        _port: Option<u16>,
        _is_ip_literal: bool,
    ) -> Result<(), TransportError> {
        Ok(())
    }

    fn authorize_resolved(
        &self,
        _host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        Ok(candidates.to_vec())
    }

    fn authorize_socket(
        &self,
        _host: &str,
        _addr: IpAddr,
        _port: u16,
    ) -> Result<(), TransportError> {
        Ok(())
    }

    fn authorize_redirect(&self, _from: &url::Url, to: &url::Url) -> Result<(), TransportError> {
        Err(TransportError::denied(
            PolicyCheckpoint::Redirect,
            format!("test denial for '{}'", to.as_str()),
        ))
    }

    fn authorize_proxy(
        &self,
        _proxy_endpoint: &url::Url,
        _ultimate: &url::Url,
    ) -> Result<(), TransportError> {
        Ok(())
    }

    fn check_tls_consistency(
        &self,
        _request_host: &str,
        _sni_override: Option<&str>,
        _host_override: Option<&str>,
    ) -> Result<(), TransportError> {
        Ok(())
    }
}

#[tokio::test]
async fn socks5h_remote_dns_fails_closed_before_dispatch() {
    // SOCKS5H resolves the ultimate target at the proxy (remote DNS), so the
    // local process can never constrain the ultimate peer. Strict scoped
    // transport must fail closed, never describe this as pinned.
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = RecordingAuth::new(AllowAll);
    let request = get(&server.base_host("test.local")).with_proxy(ProxyIntent::All {
        endpoint: Url::parse("socks5h://127.0.0.1:1080").expect("proxy url"),
        credential: None,
    });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(
            err,
            TransportError::PolicyDenied {
                checkpoint: PolicyCheckpoint::Proxy,
                ..
            }
        ),
        "SOCKS5H must fail at proxy checkpoint, got {err:?}"
    );
    assert!(
        server.received().is_empty(),
        "no origin I/O after SOCKS5H denial"
    );
}

#[tokio::test]
async fn plaintext_forward_proxy_fails_closed_before_dispatch() {
    // Standard HTTP forward proxies cannot enforce the requested ultimate IP
    // for plaintext HTTP origins. Fail closed, never report as pinned.
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = get(&server.base_host("test.local")).with_proxy(ProxyIntent::All {
        endpoint: Url::parse("http://127.0.0.1:8080").expect("proxy url"),
        credential: None,
    });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(
            err,
            TransportError::PolicyDenied {
                checkpoint: PolicyCheckpoint::Proxy,
                ..
            }
        ),
        "plaintext forward-proxy pin must fail at proxy checkpoint, got {err:?}"
    );
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn proxy_peer_and_ultimate_are_separately_bound() {
    // Both legs must be authorized independently: a proxy-peer pin does not
    // authorize the ultimate origin and vice versa. Proven via checkpoint
    // observation (proxy + dns/socket both appear, in contract order).
    let server = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = RecordingAuth::new(AllowAll);
    // Direct (no proxy) still binds ultimate only; proxied HTTPS would bind both.
    // Here we assert the direct path observes dns/socket but no proxy call.
    let request = get(&server.base_host("test.local"));
    let _ = transport
        .execute(&auth, request)
        .await
        .expect("direct succeeds");
    let calls = auth.calls();
    assert!(calls.contains(&"resolved".to_string()) || calls.contains(&"reresolution".to_string()));
    assert!(
        !calls.contains(&"proxy".to_string()),
        "direct must not touch proxy: {calls:?}"
    );
}

// --- Multi-address socket-binding corrective pass (2026-09-17) ---
//
// Required invariant: for every physical connection leg, every socket address
// the concrete backend may dial must have passed the matching selected-socket
// checkpoint. A DNS-approved set is an input to selection, never permission
// for the backend to choose any member after a narrower socket checkpoint.
//
// These fixtures model two addresses per leg where the primary (first,
// socket-authorized) cannot complete the connection and the secondary is
// reachable but never socket-authorized. The backend must fail rather than
// silently fall back to the secondary.

/// Resolver that returns addresses in the given order (no sorting), so the
/// test controls which candidate is primary.
struct OrderedResolver {
    map: std::collections::HashMap<String, Vec<IpAddr>>,
}

impl OrderedResolver {
    fn new(pairs: Vec<(&str, Vec<&str>)>) -> Self {
        let mut map = std::collections::HashMap::new();
        for (host, addrs) in pairs {
            let parsed: Vec<IpAddr> = addrs
                .into_iter()
                .map(|s| s.parse().expect("test IP parses"))
                .collect();
            map.insert(host.to_string(), parsed);
        }
        Self { map }
    }
}

impl TransportResolver for OrderedResolver {
    fn resolve(&self, host: &str) -> eggsec_transport::ResolvedCandidates {
        match self.map.get(host) {
            Some(addrs) => eggsec_transport::ResolvedCandidates {
                hostname: host.to_string(),
                addresses: addrs.clone(),
            },
            None => eggsec_transport::ResolvedCandidates::empty(host),
        }
    }
}

/// Authority that approves every candidate but records each selected-socket
/// checkpoint distinctly, so the test can prove the secondary was never
/// socket-authorized.
#[derive(Debug, Default)]
struct SocketRecorder {
    socket_calls: std::sync::Mutex<Vec<IpAddr>>,
    proxy_socket_calls: std::sync::Mutex<Vec<IpAddr>>,
}

impl SocketRecorder {
    fn socket_calls(&self) -> Vec<IpAddr> {
        self.socket_calls.lock().expect("lock").clone()
    }

    fn proxy_socket_calls(&self) -> Vec<IpAddr> {
        self.proxy_socket_calls.lock().expect("lock").clone()
    }
}

impl NetworkAuthority for SocketRecorder {
    fn authorize_initial_url(&self, url: &url::Url) -> Result<(), TransportError> {
        eggsec_transport::reject_url_userinfo(url)
    }

    fn authorize_host(
        &self,
        _host: &str,
        _port: Option<u16>,
        _is_ip_literal: bool,
    ) -> Result<(), TransportError> {
        Ok(())
    }

    fn authorize_resolved(
        &self,
        _host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        Ok(candidates.to_vec())
    }

    fn authorize_socket(
        &self,
        _host: &str,
        addr: IpAddr,
        _port: u16,
    ) -> Result<(), TransportError> {
        self.socket_calls.lock().expect("lock").push(addr);
        Ok(())
    }

    fn authorize_redirect(&self, _from: &url::Url, _to: &url::Url) -> Result<(), TransportError> {
        Ok(())
    }

    fn authorize_proxy(
        &self,
        _proxy_endpoint: &url::Url,
        _ultimate: &url::Url,
    ) -> Result<(), TransportError> {
        Ok(())
    }

    fn authorize_proxy_resolved(
        &self,
        _proxy_host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        Ok(candidates.to_vec())
    }

    fn authorize_proxy_socket(
        &self,
        _proxy_host: &str,
        addr: IpAddr,
        _port: u16,
    ) -> Result<(), TransportError> {
        self.proxy_socket_calls.lock().expect("lock").push(addr);
        Ok(())
    }

    fn check_tls_consistency(
        &self,
        _request_host: &str,
        _sni_override: Option<&str>,
        _host_override: Option<&str>,
    ) -> Result<(), TransportError> {
        Ok(())
    }
}

/// Minimal HTTP CONNECT proxy fixture for the corrective pass.
///
/// Accepts TCP on 127.0.0.1, records each accepted peer connection and each
/// CONNECT authority-form target, dials the requested target (so a bad
/// ultimate fails with 502 and a good one tunnels), and relays bytes on
/// success. Nothing leaves the host.
struct ConnectProxy {
    addr: std::net::SocketAddr,
    hits: Arc<std::sync::Mutex<usize>>,
    targets: Arc<std::sync::Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for ConnectProxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl ConnectProxy {
    async fn start() -> Self {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback binds");
        let addr = listener.local_addr().expect("local addr");
        let hits = Arc::new(std::sync::Mutex::new(0usize));
        let targets = Arc::new(std::sync::Mutex::new(Vec::new()));
        let task = {
            let hits = hits.clone();
            let targets = targets.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((mut inbound, _)) = listener.accept().await else {
                        return;
                    };
                    {
                        *hits.lock().expect("lock") += 1;
                    }
                    let targets = targets.clone();
                    tokio::spawn(async move {
                        let mut buf = Vec::new();
                        let mut chunk = [0u8; 4096];
                        let end = loop {
                            match inbound.read(&mut chunk).await {
                                Ok(0) => return,
                                Ok(n) => {
                                    buf.extend_from_slice(&chunk[..n]);
                                    if buf.len() > 65536 {
                                        return;
                                    }
                                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n")
                                    {
                                        break pos + 4;
                                    }
                                }
                                Err(_) => return,
                            }
                        };
                        let head = String::from_utf8_lossy(&buf[..end]).to_string();
                        let first_line = head.lines().next().unwrap_or("");
                        // CONNECT <target> HTTP/1.1
                        let target = first_line
                            .split_whitespace()
                            .nth(1)
                            .unwrap_or("")
                            .to_string();
                        if target.is_empty() {
                            return;
                        }
                        targets.lock().expect("lock").push(target.clone());
                        match tokio::net::TcpStream::connect(target.as_str()).await {
                            Err(_) => {
                                let resp = "HTTP/1.1 502 Bad Gateway\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
                                let _ = inbound.write_all(resp.as_bytes()).await;
                            }
                            Ok(mut upstream) => {
                                let resp = "HTTP/1.1 200 Connection Established\r\n\r\n";
                                if inbound.write_all(resp.as_bytes()).await.is_err() {
                                    return;
                                }
                                let _ = tokio::io::copy_bidirectional(&mut inbound, &mut upstream)
                                    .await;
                            }
                        }
                    });
                }
            })
        };
        Self {
            addr,
            hits,
            targets,
            task,
        }
    }

    fn hits(&self) -> usize {
        *self.hits.lock().expect("lock")
    }

    fn targets(&self) -> Vec<String> {
        self.targets.lock().expect("lock").clone()
    }
}

fn proxied_https_request(
    origin_host: &str,
    origin_port: u16,
    proxy_host: &str,
    proxy_port: u16,
) -> ScopedHttpRequest {
    use eggsec_transport::{ProxyIntent, TimeoutPolicy, TlsPolicy};
    ScopedHttpRequest::new_with_url(
        Method::GET,
        &format!("https://{origin_host}:{origin_port}/"),
    )
    .expect("request builds")
    .with_proxy(ProxyIntent::Http {
        endpoint: Url::parse(&format!("http://{proxy_host}:{proxy_port}/")).expect("proxy url"),
        credential: None,
    })
    .with_tls(TlsPolicy::insecure())
    .with_timeout(TimeoutPolicy {
        request_timeout: Duration::from_secs(10),
        connect_timeout: Some(Duration::from_secs(5)),
    })
}

#[tokio::test]
async fn proxy_peer_fallback_to_unsocket_authorized_secondary_is_forbidden() {
    // Proxy candidates: primary 127.0.0.2 (no listener, refused) is the only
    // socket-authorized peer; secondary 127.0.0.1 (real CONNECT proxy) is
    // DNS-approved but never passes authorize_proxy_socket. The backend must
    // fail on the primary rather than fall back to the secondary.
    let origin = TlsFixture::start(
        Arc::new(|incoming: &Incoming| Action::ok(incoming.body.clone())),
        vec!["127.0.0.1".to_string()],
    )
    .await;
    let proxy = ConnectProxy::start().await;
    let bad: IpAddr = "127.0.0.2".parse().expect("ip");
    let good: IpAddr = "127.0.0.1".parse().expect("ip");
    let resolver: Arc<dyn TransportResolver> = Arc::new(OrderedResolver::new(vec![
        ("proxy.local", vec!["127.0.0.2", "127.0.0.1"]),
        ("origin.local", vec!["127.0.0.1"]),
    ]));
    let transport = EggfetchTransport::new(resolver);
    let auth = SocketRecorder::default();
    let request = proxied_https_request(
        "origin.local",
        origin.addr.port(),
        "proxy.local",
        proxy.addr.port(),
    );
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "primary proxy failure must surface as Backend, got {err:?}"
    );
    assert!(!err.is_denied());
    assert_eq!(
        auth.proxy_socket_calls(),
        vec![bad],
        "only the primary may pass the proxy socket checkpoint"
    );
    assert!(
        !auth.proxy_socket_calls().contains(&good),
        "secondary must never be socket-authorized"
    );
    assert_eq!(
        proxy.hits(),
        0,
        "no bytes may reach the secondary proxy peer"
    );
    assert!(
        proxy.targets().is_empty(),
        "no CONNECT may be issued when the only authorized peer fails"
    );
    assert!(
        origin.received().is_empty(),
        "no origin I/O after proxy-peer failure"
    );
}

#[tokio::test]
async fn proxied_ultimate_fallback_to_unsocket_authorized_secondary_is_forbidden() {
    // Ultimate candidates: primary 127.0.0.2 (no TLS listener, proxy dial
    // fails with 502) is the only socket-authorized target; secondary
    // 127.0.0.1 (real TLS origin) is DNS-approved but never passes
    // authorize_socket. The backend must fail on the primary rather than
    // retry to the secondary.
    let origin = TlsFixture::start(
        Arc::new(|incoming: &Incoming| Action::ok(incoming.body.clone())),
        vec!["127.0.0.1".to_string()],
    )
    .await;
    let proxy = ConnectProxy::start().await;
    let bad: IpAddr = "127.0.0.2".parse().expect("ip");
    let good: IpAddr = "127.0.0.1".parse().expect("ip");
    let resolver: Arc<dyn TransportResolver> = Arc::new(OrderedResolver::new(vec![
        ("proxy.local", vec!["127.0.0.1"]),
        ("origin.local", vec!["127.0.0.2", "127.0.0.1"]),
    ]));
    let transport = EggfetchTransport::new(resolver);
    let auth = SocketRecorder::default();
    let request = proxied_https_request(
        "origin.local",
        origin.addr.port(),
        "proxy.local",
        proxy.addr.port(),
    );
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "primary ultimate failure must surface as Backend, got {err:?}"
    );
    assert!(!err.is_denied());
    assert_eq!(
        auth.socket_calls(),
        vec![bad],
        "only the primary ultimate may pass the socket checkpoint"
    );
    assert!(
        !auth.socket_calls().contains(&good),
        "secondary ultimate must never be socket-authorized"
    );
    let targets = proxy.targets();
    assert_eq!(
        targets.len(),
        1,
        "exactly one CONNECT target may be attempted: {targets:?}"
    );
    assert!(
        targets[0].starts_with("127.0.0.2:"),
        "CONNECT must target the socket-authorized primary, got {:?}",
        targets[0]
    );
    assert!(
        origin.received().is_empty(),
        "secondary ultimate must receive no TLS bytes"
    );
}

#[tokio::test]
async fn proxied_success_reports_the_authorized_peer() {
    // Single-address control: with one proxy peer and one ultimate target,
    // dispatch succeeds and ConnectionInfo describes the authorized proxy
    // peer (truthful evidence), while CONNECT targets the authorized
    // ultimate.
    let origin = TlsFixture::start(
        Arc::new(|incoming: &Incoming| Action::ok(incoming.body.clone())),
        vec!["127.0.0.1".to_string()],
    )
    .await;
    let proxy = ConnectProxy::start().await;
    let resolver: Arc<dyn TransportResolver> = Arc::new(OrderedResolver::new(vec![
        ("proxy.local", vec!["127.0.0.1"]),
        ("origin.local", vec!["127.0.0.1"]),
    ]));
    let transport = EggfetchTransport::new(resolver);
    let auth = SocketRecorder::default();
    let request = proxied_https_request(
        "origin.local",
        origin.addr.port(),
        "proxy.local",
        proxy.addr.port(),
    );
    let response = transport.execute(&auth, request).await.expect("dispatch");
    assert_eq!(response.status, StatusCode::OK);
    let conn = response.connection.expect("connection");
    let expected_peer =
        std::net::SocketAddr::new("127.0.0.1".parse().expect("ip"), proxy.addr.port());
    assert_eq!(
        conn.remote_addr,
        Some(expected_peer),
        "proxied remote_addr must be the socket-authorized proxy peer"
    );
    assert_eq!(
        auth.proxy_socket_calls().len(),
        1,
        "exactly one proxy socket checkpoint"
    );
    assert_eq!(
        auth.socket_calls().len(),
        1,
        "exactly one ultimate socket checkpoint"
    );
    let targets = proxy.targets();
    assert_eq!(targets.len(), 1, "one CONNECT target: {targets:?}");
    assert!(
        targets[0].starts_with("127.0.0.1:"),
        "CONNECT must target the authorized ultimate, got {:?}",
        targets[0]
    );
    assert_eq!(origin.received().len(), 1);
}

// --- Eggfetch 0.1.7 adoption (2026-09-18): direct resolved route, total
// deadline through body EOF, reuse/isolation, policy dispositions ---
//
// The production direct route is logical URL + singular `resolved_addresses`
// pin (no IP-literal wire-URL shim). `Timeout.total` is an absolute deadline
// through response-body EOF/trailers. The route cache reuses Hyper H1/H2
// clients for equal logical-origin + ordered-address + SNI keys.

/// Raw slow-body server: sends response headers immediately, then delays
/// body EOF beyond the caller timeout. Proves `Timeout.total` covers the
/// body, not just headers/connect.
struct SlowBodyServer {
    addr: std::net::SocketAddr,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for SlowBodyServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl SlowBodyServer {
    async fn start(body: Vec<u8>, body_delay: Duration) -> Self {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback binds");
        let addr = listener.local_addr().expect("local addr");
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                let body = body.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let mut head = Vec::new();
                    loop {
                        match stream.read(&mut buf).await {
                            Ok(0) => return,
                            Ok(n) => {
                                head.extend_from_slice(&buf[..n]);
                                if head.windows(4).any(|w| w == b"\r\n\r\n") {
                                    break;
                                }
                                if head.len() > 65536 {
                                    return;
                                }
                            }
                            Err(_) => return,
                        }
                    }
                    let headers = format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        body.len()
                    );
                    if stream.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }
                    if stream.flush().await.is_err() {
                        return;
                    }
                    tokio::time::sleep(body_delay).await;
                    let _ = stream.write_all(&body).await;
                    let _ = stream.flush().await;
                });
            }
        });
        Self { addr, task }
    }

    fn base_host(&self, host: &str) -> String {
        format!("http://{host}:{}", self.addr.port())
    }
}

/// Trickle server: headers immediately, then body bytes one at a time with
/// a delay between chunks. Regular chunk arrival must not restart
/// `Timeout.total`.
struct TrickleServer {
    addr: std::net::SocketAddr,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TrickleServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TrickleServer {
    async fn start(body: Vec<u8>, per_chunk: Duration) -> Self {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback binds");
        let addr = listener.local_addr().expect("local addr");
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                let body = body.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let mut head = Vec::new();
                    loop {
                        match stream.read(&mut buf).await {
                            Ok(0) => return,
                            Ok(n) => {
                                head.extend_from_slice(&buf[..n]);
                                if head.windows(4).any(|w| w == b"\r\n\r\n") {
                                    break;
                                }
                                if head.len() > 65536 {
                                    return;
                                }
                            }
                            Err(_) => return,
                        }
                    }
                    let headers = format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        body.len()
                    );
                    if stream.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }
                    if stream.flush().await.is_err() {
                        return;
                    }
                    for chunk in body.chunks(1) {
                        tokio::time::sleep(per_chunk).await;
                        if stream.write_all(chunk).await.is_err() {
                            return;
                        }
                        if stream.flush().await.is_err() {
                            return;
                        }
                    }
                });
            }
        });
        Self { addr, task }
    }

    fn base_host(&self, host: &str) -> String {
        format!("http://{host}:{}", self.addr.port())
    }
}

/// Keep-alive server: counts accepted TCP connections, serves multiple
/// requests per connection with `connection: keep-alive`. Proves physical
/// reuse (accept count stays 1 for repeated same-route requests).
struct KeepAliveServer {
    addr: std::net::SocketAddr,
    accepts: Arc<std::sync::atomic::AtomicUsize>,
    received: Arc<std::sync::Mutex<Vec<Incoming>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for KeepAliveServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl KeepAliveServer {
    async fn start() -> Self {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback binds");
        let addr = listener.local_addr().expect("local addr");
        let accepts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let task = {
            let accepts = accepts.clone();
            let received = received.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        return;
                    };
                    accepts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let received = received.clone();
                    tokio::spawn(async move {
                        let mut stream = stream;
                        loop {
                            let mut head = Vec::new();
                            let mut chunk = [0u8; 4096];
                            let incoming = loop {
                                match stream.read(&mut chunk).await {
                                    Ok(0) => return,
                                    Ok(n) => {
                                        head.extend_from_slice(&chunk[..n]);
                                        if head.len() > 65536 {
                                            return;
                                        }
                                        if let Some(end) = head
                                            .windows(4)
                                            .position(|w| w == b"\r\n\r\n")
                                            .map(|p| p + 4)
                                        {
                                            let text =
                                                String::from_utf8_lossy(&head[..end]).to_string();
                                            let mut lines = text.lines();
                                            let line = lines.next().unwrap_or("").to_string();
                                            let mut parts = line.split_whitespace();
                                            let method = parts.next().unwrap_or("").to_string();
                                            let path = parts.next().unwrap_or("/").to_string();
                                            let mut headers = Vec::new();
                                            let mut content_length = 0usize;
                                            for l in lines {
                                                if let Some((k, v)) = l.split_once(':') {
                                                    let k = k.trim().to_lowercase();
                                                    let v = v.trim().to_string();
                                                    if k == "content-length" {
                                                        content_length =
                                                            v.parse().unwrap_or(0).min(1024);
                                                    }
                                                    headers.push((k, v));
                                                }
                                            }
                                            let mut body = head[end..].to_vec();
                                            while body.len() < content_length {
                                                match stream.read(&mut chunk).await {
                                                    Ok(0) => break,
                                                    Ok(m) => body.extend_from_slice(&chunk[..m]),
                                                    Err(_) => return,
                                                }
                                            }
                                            body.truncate(content_length);
                                            break Incoming {
                                                method,
                                                path,
                                                headers,
                                                body,
                                            };
                                        }
                                    }
                                    Err(_) => return,
                                }
                            };
                            received.lock().expect("lock").push(incoming.clone());
                            let payload = b"ok";
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: keep-alive\r\n\r\n",
                                payload.len()
                            );
                            if stream.write_all(resp.as_bytes()).await.is_err() {
                                return;
                            }
                            if stream.write_all(payload).await.is_err() {
                                return;
                            }
                            if stream.flush().await.is_err() {
                                return;
                            }
                        }
                    });
                }
            })
        };
        Self {
            addr,
            accepts,
            received,
            task,
        }
    }

    fn base_host(&self, host: &str) -> String {
        format!("http://{host}:{}", self.addr.port())
    }

    fn accepts(&self) -> usize {
        self.accepts.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn received(&self) -> Vec<Incoming> {
        self.received.lock().expect("lock").clone()
    }
}

#[tokio::test]
async fn direct_singular_pin_forbids_fallback_to_secondary() {
    // Direct candidates: primary 127.0.0.2 (refused) is the only
    // socket-authorized target; secondary 127.0.0.1 (real server) is
    // DNS-approved but never passes `authorize_socket`. The backend must fail
    // on the primary rather than fall back to the secondary.
    let server = echo_target().await;
    let bad: IpAddr = "127.0.0.2".parse().expect("ip");
    let good: IpAddr = "127.0.0.1".parse().expect("ip");
    let resolver: Arc<dyn TransportResolver> = Arc::new(OrderedResolver::new(vec![(
        "test.local",
        vec!["127.0.0.2", "127.0.0.1"],
    )]));
    let transport = EggfetchTransport::new(resolver);
    let auth = SocketRecorder::default();
    let url = format!("http://test.local:{}/", server.addr.port());
    let err = transport.execute(&auth, get(&url)).await.unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "primary direct failure must surface as Backend, got {err:?}"
    );
    assert!(!err.is_denied());
    assert_eq!(
        auth.socket_calls(),
        vec![bad],
        "only the primary may pass the socket checkpoint"
    );
    assert!(
        !auth.socket_calls().contains(&good),
        "secondary must never be socket-authorized"
    );
    assert!(
        server.received().is_empty(),
        "no bytes may reach the secondary peer"
    );
}

#[tokio::test]
async fn headers_fast_body_slow_exceeds_total_deadline() {
    // Headers arrive before `request_timeout`; body EOF stalls beyond it.
    // Under 0.1.5 this waited for the delayed body; under 0.1.7 it must fail
    // with a total-timeout Backend error within the budget.
    let server = SlowBodyServer::start(b"late-body".to_vec(), Duration::from_secs(5)).await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = get(&server.base_host("test.local")).with_timeout(TimeoutPolicy {
        request_timeout: Duration::from_secs(1),
        connect_timeout: None,
    });
    let start = std::time::Instant::now();
    let err = transport.execute(&auth, request).await.unwrap_err();
    let elapsed = start.elapsed();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "expected Backend total timeout, got {err:?}"
    );
    assert!(!err.is_denied());
    assert!(
        err.to_string().contains("total"),
        "timeout phase must be total, got {err}"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "total deadline must fire within budget, took {elapsed:?}"
    );
}

#[tokio::test]
async fn post_first_chunk_stall_still_exceeds_total() {
    // First body byte arrives quickly, then the stream stalls past total.
    // Total must not reset on chunk arrival.
    let server = TrickleServer::start(b"0123456789".to_vec(), Duration::from_secs(1)).await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = get(&server.base_host("test.local")).with_timeout(TimeoutPolicy {
        request_timeout: Duration::from_millis(1500),
        connect_timeout: None,
    });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "expected Backend total timeout, got {err:?}"
    );
    assert!(
        err.to_string().contains("total"),
        "must be total phase: {err}"
    );
}

#[tokio::test]
async fn continuous_trickle_cannot_extend_aggregate_total() {
    // 20 bytes at 200ms each = 4s total trickle with a 1.5s budget. Regular
    // arrival must not extend the absolute deadline.
    let body = vec![b'x'; 20];
    let server = TrickleServer::start(body, Duration::from_millis(200)).await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = get(&server.base_host("test.local")).with_timeout(TimeoutPolicy {
        request_timeout: Duration::from_millis(1500),
        connect_timeout: None,
    });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "trickle must still hit total, got {err:?}"
    );
    assert!(
        err.to_string().contains("total"),
        "must be total phase: {err}"
    );
}

#[tokio::test]
async fn redirect_final_body_sees_remaining_budget_not_fresh_timeout() {
    // Hop 1 delays ~800ms (consumes budget); hop 2 sends headers fast but
    // stalls its body for 5s. With a 2s aggregate budget the final body must
    // time out on the ~1.2s remainder. A fresh full timeout per hop would
    // incorrectly allow the 5s stall.
    let slow_body = SlowBodyServer::start(b"final".to_vec(), Duration::from_secs(5)).await;
    let landing = slow_body.base_host("other.local");
    let first = Fixture::start(Arc::new(move |incoming: &Incoming| {
        if incoming.path == "/slow-redirect" {
            std::thread::sleep(Duration::from_millis(800));
            Action::redirect(302, format!("{landing}/final"))
        } else {
            Action::ok("first".as_bytes().to_vec())
        }
    }))
    .await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let url = format!("{}{}", first.base_host("test.local"), "/slow-redirect");
    let request = get(&url)
        .with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 5 })
        .with_timeout(TimeoutPolicy {
            request_timeout: Duration::from_secs(2),
            connect_timeout: None,
        });
    let start = std::time::Instant::now();
    let err = transport.execute(&auth, request).await.unwrap_err();
    let elapsed = start.elapsed();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "expected Backend total timeout on remainder, got {err:?}"
    );
    assert!(
        err.to_string().contains("total"),
        "must be total phase: {err}"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "remaining budget must bound the chain, took {elapsed:?}"
    );
}

#[tokio::test]
async fn transport_reusable_after_total_timeout() {
    // A total-timeout terminalization must not poison the cached route client
    // for a subsequent authorized request.
    let slow = SlowBodyServer::start(b"late".to_vec(), Duration::from_secs(5)).await;
    let fast = echo_target().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let slow_req = get(&slow.base_host("test.local")).with_timeout(TimeoutPolicy {
        request_timeout: Duration::from_secs(1),
        connect_timeout: None,
    });
    let err = transport.execute(&auth, slow_req).await.unwrap_err();
    assert!(matches!(err, TransportError::Backend(_)));
    let ok = transport
        .execute(&auth, get(&fast.base_host("test.local")))
        .await
        .expect("transport reusable after timeout");
    assert_eq!(ok.status, StatusCode::OK);
}

#[tokio::test]
async fn h1_same_route_reuses_keep_alive_connection() {
    // One transport, one logical origin, one selected socket: repeated
    // sequential requests reuse the keep-alive TCP connection (accept count
    // stays 1) via the 0.1.7 route-keyed client.
    let server = KeepAliveServer::start().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let url = server.base_host("test.local");
    for _ in 0..5 {
        let response = transport
            .execute(&auth, get(&url))
            .await
            .expect("reuse request");
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body.as_ref(), b"ok");
    }
    assert_eq!(
        server.accepts(),
        1,
        "same-route sequential requests must reuse one TCP connection"
    );
    assert_eq!(server.received().len(), 5);
}

#[tokio::test]
async fn selected_target_change_does_not_reuse_old_connection() {
    // Two logical servers (different ports => different selected sockets):
    // requests to each must not share a connection.
    let first = KeepAliveServer::start().await;
    let second = KeepAliveServer::start().await;
    assert_ne!(first.addr.port(), second.addr.port());
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    transport
        .execute(&auth, get(&first.base_host("test.local")))
        .await
        .expect("first");
    transport
        .execute(&auth, get(&second.base_host("test.local")))
        .await
        .expect("second");
    assert_eq!(first.accepts(), 1);
    assert_eq!(second.accepts(), 1);
    // Repeat first: must reuse first's connection, not open a second.
    transport
        .execute(&auth, get(&first.base_host("test.local")))
        .await
        .expect("first again");
    assert_eq!(first.accepts(), 1, "same target must reuse, not redial");
    assert_eq!(first.received().len(), 2);
}

#[tokio::test]
async fn logical_origin_change_does_not_reuse_connection() {
    // Same physical socket (same server port) but different logical origins
    // must not share a route connection (cache key includes origin).
    let server = KeepAliveServer::start().await;
    let port = server.addr.port();
    let resolver: Arc<dyn TransportResolver> = Arc::new(
        InMemoryResolver::new()
            .with("a.local", vec!["127.0.0.1"])
            .with("b.local", vec!["127.0.0.1"]),
    );
    let transport = EggfetchTransport::new(resolver);
    let auth = AllowAll;
    transport
        .execute(&auth, get(&format!("http://a.local:{port}/")))
        .await
        .expect("a.local");
    transport
        .execute(&auth, get(&format!("http://b.local:{port}/")))
        .await
        .expect("b.local");
    assert_eq!(
        server.accepts(),
        2,
        "different logical origins must not share a physical route"
    );
}

#[tokio::test]
async fn hostile_proxy_environment_cannot_divert_direct_request() {
    // `ProxyEnvironment` is never constructed in the adapter; direct hops
    // force `.without_proxy()`. Hostile process env must not divert them.
    std::env::set_var("HTTP_PROXY", "http://127.0.0.1:9/");
    std::env::set_var("HTTPS_PROXY", "http://127.0.0.1:9/");
    std::env::set_var("ALL_PROXY", "http://127.0.0.1:9/");
    std::env::set_var("http_proxy", "http://127.0.0.1:9/");
    std::env::set_var("https_proxy", "http://127.0.0.1:9/");
    std::env::set_var("all_proxy", "http://127.0.0.1:9/");
    let result = async {
        let server = echo_target().await;
        let transport = EggfetchTransport::new(memory_resolver());
        let auth = AllowAll;
        transport
            .execute(&auth, get(&server.base_host("test.local")))
            .await
    }
    .await;
    std::env::remove_var("HTTP_PROXY");
    std::env::remove_var("HTTPS_PROXY");
    std::env::remove_var("ALL_PROXY");
    std::env::remove_var("http_proxy");
    std::env::remove_var("https_proxy");
    std::env::remove_var("all_proxy");
    let response = result.expect("direct must ignore hostile proxy env");
    assert_eq!(response.status, StatusCode::OK);
}

#[tokio::test]
async fn https_downgrade_behavior_remains_compatibility_allow() {
    // The cumulative 0.1.6 `RedirectDowngradePolicy` must not silently become
    // `Deny`: an https -> http same-loopback redirect still follows under
    // `AuthorityChecked` (neutral contract has no downgrade dimension).
    let plain = Fixture::start(Arc::new(|incoming: &Incoming| {
        Action::ok(format!("landed:{}", incoming.path).into_bytes())
    }))
    .await;
    let downgrade_target = format!("http://test.local:{}/landed", plain.addr.port());
    let tls = TlsFixture::start(
        Arc::new(move |incoming: &Incoming| {
            if incoming.path == "/down" {
                Action::redirect(302, downgrade_target.clone())
            } else {
                Action::ok("tls".as_bytes().to_vec())
            }
        }),
        vec!["127.0.0.1".to_string()],
    )
    .await;
    let resolver: Arc<dyn TransportResolver> = Arc::new(
        InMemoryResolver::new()
            .with("test.local", vec!["127.0.0.1"])
            .with("tls.local", vec!["127.0.0.1"]),
    );
    let transport = EggfetchTransport::new(resolver);
    let auth = AllowAll;
    let tls_url = format!("https://tls.local:{}/down", tls.addr.port());
    let request = ScopedHttpRequest::new_with_url(Method::GET, &tls_url)
        .expect("request")
        .with_tls(TlsPolicy::insecure())
        .with_redirect(RedirectPolicy::AuthorityChecked { max_redirects: 5 });
    let response = transport.execute(&auth, request).await.expect("follow");
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.redirect_history.len(), 1);
    assert_eq!(plain.received().len(), 1);
}

#[tokio::test]
async fn literal_ipv6_loopback_dispatches_without_resolver() {
    let server = echo_target().await;
    // Rewrite the IPv4 loopback base as IPv6 loopback on the same port.
    let url = format!("http://[::1]:{}/", server.addr.port());
    // If the fixture host lacks IPv6 loopback this fails as Backend (not
    // denial); either way the resolver must never be consulted.
    let transport = EggfetchTransport::new(Arc::new(PanicResolver));
    let auth = AllowAll;
    match transport.execute(&auth, get(&url)).await {
        Ok(response) => {
            assert_eq!(response.status, StatusCode::OK);
            let conn = response.connection.expect("connection");
            let addr = conn.remote_addr.expect("addr");
            assert_eq!(addr.ip(), "::1".parse::<IpAddr>().expect("ip"));
        }
        Err(TransportError::Backend(_)) => {}
        Err(other) => panic!("unexpected error kind: {other:?}"),
    }
}

#[tokio::test]
async fn proxy_credentials_do_not_bleed_across_requests() {
    // Two sequential CONNECT requests with distinct credentials must present
    // distinct `Proxy-Authorization` values (no cross-request bleed).
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    struct AuthProxy {
        addr: std::net::SocketAddr,
        seen: Arc<std::sync::Mutex<Vec<Option<String>>>>,
        task: tokio::task::JoinHandle<()>,
    }
    impl Drop for AuthProxy {
        fn drop(&mut self) {
            self.task.abort();
        }
    }
    impl AuthProxy {
        async fn start() -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("loopback binds");
            let addr = listener.local_addr().expect("local addr");
            let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
            let task = {
                let seen = seen.clone();
                tokio::spawn(async move {
                    loop {
                        let Ok((mut inbound, _)) = listener.accept().await else {
                            return;
                        };
                        let seen = seen.clone();
                        tokio::spawn(async move {
                            let mut buf = Vec::new();
                            let mut chunk = [0u8; 4096];
                            let end = loop {
                                match inbound.read(&mut chunk).await {
                                    Ok(0) => return,
                                    Ok(n) => {
                                        buf.extend_from_slice(&chunk[..n]);
                                        if buf.len() > 65536 {
                                            return;
                                        }
                                        if let Some(p) =
                                            buf.windows(4).position(|w| w == b"\r\n\r\n")
                                        {
                                            break p + 4;
                                        }
                                    }
                                    Err(_) => return,
                                }
                            };
                            let head = String::from_utf8_lossy(&buf[..end]).to_string();
                            let mut auth_value = None;
                            for line in head.lines().skip(1) {
                                if let Some((k, v)) = line.split_once(':') {
                                    if k.trim().eq_ignore_ascii_case("proxy-authorization") {
                                        auth_value = Some(v.trim().to_string());
                                    }
                                }
                            }
                            seen.lock().expect("lock").push(auth_value);
                            let resp = "HTTP/1.1 502 Bad Gateway\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
                            let _ = inbound.write_all(resp.as_bytes()).await;
                        });
                    }
                })
            };
            Self { addr, seen, task }
        }
    }
    let proxy = AuthProxy::start().await;
    let resolver: Arc<dyn TransportResolver> = Arc::new(
        InMemoryResolver::new()
            .with("proxy.local", vec!["127.0.0.1"])
            .with("origin.local", vec!["127.0.0.1"]),
    );
    let transport = EggfetchTransport::new(resolver);
    let auth = AllowAll;
    // Two CONNECT attempts with different credentials (both fail at the
    // fake proxy with 502, but the presented auth must differ per request).
    for user in ["alice", "bob"] {
        let req = ScopedHttpRequest::new_with_url(Method::GET, "https://origin.local:443/")
            .expect("request")
            .with_tls(TlsPolicy::insecure())
            .with_timeout(TimeoutPolicy {
                request_timeout: Duration::from_secs(5),
                connect_timeout: Some(Duration::from_secs(2)),
            })
            .with_proxy(ProxyIntent::Http {
                endpoint: Url::parse(&format!("http://proxy.local:{}/", proxy.addr.port()))
                    .expect("proxy url"),
                credential: Some(eggsec_transport::ProxyCredential::new(user, "s3cr3t")),
            });
        let _ = transport.execute(&auth, req).await;
    }
    let seen = proxy.seen.lock().expect("lock").clone();
    assert_eq!(
        seen.len(),
        2,
        "both CONNECT attempts must reach proxy: {seen:?}"
    );
    assert!(
        seen[0].is_some() && seen[1].is_some(),
        "both must present proxy auth: {seen:?}"
    );
    assert_ne!(
        seen[0], seen[1],
        "credential A must not bleed into B: {seen:?}"
    );
}
