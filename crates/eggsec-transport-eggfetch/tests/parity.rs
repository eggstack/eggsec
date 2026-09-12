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
