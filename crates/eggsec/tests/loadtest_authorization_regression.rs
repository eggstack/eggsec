//! Load-test authorization + transport corrective-pass regressions (WS0).
//!
//! Fails on the pre-pass baseline (wildcard facade scope, Reqwest fallbacks,
//! missing proxy-peer checkpoints) and passes after WS1-WS3.
//!
//! Uses deterministic local fixtures + recording fakes. No public DNS.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use eggsec_transport::{
    CannedResponse, HttpTransport, InMemoryResolver, NetworkAuthority, PolicyCheckpoint,
    RecordingFakeTransport, TransportError,
};
use url::Url;

fn loopback_scope_for_port(port: u16) -> eggsec::config::Scope {
    let mut scope = eggsec::config::Scope::new();
    scope.allowed_targets.push(
        eggsec::config::ScopeRule::with_cidr("127.0.0.0/8".to_string()).expect("loopback cidr"),
    );
    scope.allowed_ports = Some(vec![port]);
    scope
}

fn loopback_scope_any_port() -> eggsec::config::Scope {
    let mut scope = eggsec::config::Scope::new();
    scope.allowed_targets.push(
        eggsec::config::ScopeRule::with_cidr("127.0.0.0/8".to_string()).expect("loopback cidr"),
    );
    scope
}

fn load_test_policy() -> eggsec::config::ExecutionPolicy {
    eggsec::config::ExecutionPolicy {
        allow_load_testing: true,
        allowed_capabilities: vec![eggsec::config::Capability::LoadTest],
        ..Default::default()
    }
}

fn port_of(uri: &str) -> u16 {
    Url::parse(uri)
        .expect("uri")
        .port_or_known_default()
        .expect("port")
}

// ── WS0 scope regressions ──

#[tokio::test]
async fn redirect_to_out_of_scope_never_reaches_target() {
    // A redirects to B; B's port is outside the approval scope. B must see no request.
    let resolver = Arc::new(
        InMemoryResolver::new()
            .with("a.local", vec!["127.0.0.1"])
            .with("b.local", vec!["127.0.0.1"]),
    );
    let fake = Arc::new(RecordingFakeTransport::new(resolver).with_canned(
        "http://a.local/",
        CannedResponse::redirect(http::StatusCode::FOUND, "http://b.local:9/"),
    ));
    // Scope allows a.local (port 80) but not b.local:9 (port 9 blocked via allowed_ports).
    let mut scope = eggsec::config::Scope::new();
    scope
        .allowed_targets
        .push(eggsec::config::ScopeRule::with_cidr("127.0.0.0/8".to_string()).expect("cidr"));
    scope.allowed_ports = Some(vec![80]);
    let authority = eggsec::loadtest::OwnedScopeAuthority::new(scope);
    let req =
        eggsec_transport::ScopedHttpRequest::new_with_url(http::Method::GET, "http://a.local/")
            .expect("req")
            .with_redirect(eggsec_transport::RedirectPolicy::AuthorityChecked { max_redirects: 5 });
    let err = fake.execute(&authority, req).await.unwrap_err();
    assert!(err.is_denied(), "redirect outside scope must deny: {err:?}");
    assert_eq!(fake.hop_count(), 1, "out-of-scope hop must not dispatch");
}

#[tokio::test]
async fn dns_change_to_out_of_scope_denies_before_dispatch() {
    // Resolver returns an out-of-scope address after approval; transport must deny.
    let resolver = Arc::new(InMemoryResolver::new().with("victim.local", vec!["203.0.113.99"]));
    let fake = RecordingFakeTransport::new(resolver);
    let mut scope = eggsec::config::Scope::new();
    scope
        .allowed_targets
        .push(eggsec::config::ScopeRule::with_cidr("127.0.0.0/8".to_string()).expect("cidr"));
    let authority = eggsec::loadtest::OwnedScopeAuthority::new(scope);
    let req = eggsec_transport::ScopedHttpRequest::new_with_url(
        http::Method::GET,
        "http://victim.local/",
    )
    .expect("req");
    let err = fake.execute(&authority, req).await.unwrap_err();
    assert!(err.is_denied(), "out-of-scope DNS must deny: {err:?}");
    assert!(fake.hops().is_empty(), "denied hop must not record");
}

#[tokio::test]
async fn canonical_approved_execution_uses_approval_scope() {
    // Canonical load-test via ApprovedExecution must enforce the approval scope:
    // redirect from allowed port to denied port fails, and the denied listener
    // sees no request (proven via response error + no success).
    use eggsec::dispatch::{execute_approved_execution, CanonicalOperationRequest, ExecutionSink};
    use eggsec::operation_request::LoadTestRequest;

    let server_a = wiremock::MockServer::start().await;
    let server_b = wiremock::MockServer::start().await;
    let port_a = port_of(&server_a.uri());
    let port_b = port_of(&server_b.uri());
    assert_ne!(port_a, port_b);

    // A redirects to B.
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/r"))
        .respond_with(
            wiremock::ResponseTemplate::new(302)
                .append_header("Location", format!("http://127.0.0.1:{port_b}/final"))
                .set_body_string("go"),
        )
        .mount(&server_a)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/final"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("B"))
        .mount(&server_b)
        .await;

    // Scope allows only port A.
    let scope = loopback_scope_for_port(port_a);
    let loaded = eggsec::config::LoadedScope::explicit(
        scope.clone(),
        eggsec::config::ScopeSource::ConfigFile,
        None,
    );
    let enforcement =
        eggsec::config::EnforcementContext::manual_permissive(load_test_policy(), loaded);
    // Descriptor target is server A /r.
    let target = format!("http://127.0.0.1:{port_a}/r");
    let metadata = eggsec::config::metadata_for_tool_id("load-test").expect("metadata");
    let descriptor = metadata
        .try_descriptor_for_target(Some(target.as_str()))
        .expect("descriptor");
    let execution = enforcement
        .approve_manual_execution(
            eggsec::config::ExecutionSurface::CliManual,
            descriptor,
            Some(&eggsec::config::ManualOverride {
                assume_yes: false,
                allow_out_of_scope: false,
                allow_explicit_exclusion: false,
                allow_high_risk: true,
                allow_db_pentest: false,
                allow_web_proxy: false,
                allow_nonbaseline_capability: true,
                allow_private_resolution: true,
                allow_cross_host_redirect: true,
                reason: Some("regression test override".to_string()),
            }),
        )
        .expect("approve");
    // Approval scope must be the enforcement scope (not wildcard).
    assert!(
        execution
            .scope()
            .allowed_ports
            .as_ref()
            .is_some_and(|p| p.contains(&port_a)),
        "execution scope must carry approval snapshot"
    );

    let raw = LoadTestRequest {
        target: target.clone(),
        method: None,
        requests: Some(3),
        connections: Some(1),
        duration_secs: Some(5),
        rate_limit: None,
    };
    let canonical = CanonicalOperationRequest::LoadTest(raw);
    let (tx, _rx) = tokio::sync::mpsc::channel(16);
    let sink = ExecutionSink::detached(tx);
    let outcome = execute_approved_execution(&execution, canonical, &sink).await;
    // Redirect to denied port must fail (policy denied at redirect/proxy/host),
    // never succeed via wildcard.
    match outcome {
        Ok(task_result) => {
            // If the executor surfaced a result, all requests must have failed
            // with policy_denied (no success via out-of-scope hop).
            match task_result {
                eggsec::dispatch::TaskResult::LoadTest(results) => {
                    assert_eq!(
                        results.successful_requests, 0,
                        "out-of-scope redirect must not succeed"
                    );
                    assert!(
                        results
                            .error_kinds
                            .get("policy_denied")
                            .copied()
                            .unwrap_or(0)
                            > 0
                            || results.failed_requests == results.total_requests,
                        "failures must be policy-denied: {:?}",
                        results.error_kinds
                    );
                }
                other => panic!("expected load-test result, got {other:?}"),
            }
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("denied") || msg.contains("scope") || msg.contains("port"),
                "expected scope denial, got: {msg}"
            );
        }
    }
}

#[tokio::test]
async fn strict_tool_dispatch_uses_approval_scope() {
    // EnforcedDispatcher::dispatch_execution must use the bundle scope, not wildcard.
    use eggsec::tool::{
        create_default_registry, EnforcedDispatcher, Target, ToolDispatcher, ToolRequest,
    };

    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;
    let port = port_of(&server.uri());

    // Scope allows the mock port; the bundle carries it.
    let scope = loopback_scope_for_port(port);
    let loaded =
        eggsec::config::LoadedScope::explicit(scope, eggsec::config::ScopeSource::ConfigFile, None);
    let enforcement = eggsec::config::EnforcementContext::mcp_strict(load_test_policy(), loaded);
    let target = server.uri();
    let metadata = eggsec::config::metadata_for_tool_id("load-test").expect("metadata");
    let descriptor = metadata
        .try_descriptor_for_target(Some(target.as_str()))
        .expect("descriptor");
    let execution = enforcement
        .approve(eggsec::config::ExecutionSurface::RestApi, descriptor)
        .map(|approved| {
            // Re-bind via execution API to snapshot scope (approve() alone has no scope).
            enforcement
                .approve_execution(
                    eggsec::config::ExecutionSurface::RestApi,
                    approved.descriptor().clone(),
                )
                .expect("execution")
        })
        .expect("approve");

    let dispatcher = EnforcedDispatcher::new(ToolDispatcher::new(create_default_registry()));
    let request = ToolRequest::new("load", Target::url(target.clone()))
        .with_params(serde_json::json!({"target": target, "requests": 2, "concurrency": 1}));
    let resp = dispatcher
        .dispatch_execution(&execution, request)
        .await
        .expect("strict dispatch with scope succeeds");
    assert_eq!(resp.status, eggsec::tool::ResponseStatus::Success);
}

#[tokio::test]
async fn facade_without_scope_fails_before_io() {
    let runner = eggsec::loadtest::LoadTestRunner::new(
        "http://127.0.0.1:9/".to_string(),
        2,
        1,
        Duration::from_secs(2),
    )
    .expect("runner builds");
    assert!(runner.scope().is_none(), "no wildcard default");
    let err = runner.run().await.unwrap_err();
    assert!(
        err.to_string().contains("execution scope is missing"),
        "must fail closed before I/O: {err}"
    );
}

#[tokio::test]
async fn explicit_authority_path_remains_usable() {
    // run_with(transport, authority, ...) with caller-supplied authority stays supported.
    use eggsec_transport::{CannedResponse, StatusCode};
    let resolver = Arc::new(InMemoryResolver::new().with("example.com", vec!["93.184.216.34"]));
    let transport = Arc::new(RecordingFakeTransport::new(resolver).with_canned(
        "http://example.com/",
        CannedResponse {
            status: StatusCode::OK,
            location: None,
            body: b"ok".to_vec(),
        },
    ));
    #[derive(Debug)]
    struct AllowAll;
    impl NetworkAuthority for AllowAll {
        fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError> {
            eggsec_transport::reject_url_userinfo(url)
        }
        fn authorize_host(&self, _: &str, _: Option<u16>, _: bool) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_resolved(&self, _: &str, c: &[IpAddr]) -> Result<Vec<IpAddr>, TransportError> {
            Ok(c.to_vec())
        }
        fn authorize_socket(&self, _: &str, _: IpAddr, _: u16) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_redirect(&self, _: &Url, t: &Url) -> Result<(), TransportError> {
            self.authorize_initial_url(t)
        }
        fn authorize_proxy(&self, _: &Url, _: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn check_tls_consistency(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }
    let runner = eggsec::loadtest::LoadTestRunner::new(
        "http://example.com/".to_string(),
        3,
        1,
        Duration::from_secs(5),
    )
    .expect("runner")
    .with_scope(loopback_scope_any_port());
    let authority: Arc<dyn NetworkAuthority> = Arc::new(AllowAll);
    let results = runner
        .run_with(
            transport,
            authority,
            tokio_util::sync::CancellationToken::new(),
            &eggsec::loadtest::NoopSink,
        )
        .await
        .expect("explicit authority path works");
    assert_eq!(results.successful_requests, 3);
}

// ── WS2 backend regressions ──

#[tokio::test]
async fn proxied_request_never_falls_back_to_direct() {
    // Origin would succeed direct (200), but the requested proxy is closed.
    // Fail-closed means the run errors (or all requests fail), never 200 via direct.
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("direct-ok"))
        .mount(&server)
        .await;

    let mut runner =
        eggsec::loadtest::LoadTestRunner::new(server.uri(), 2, 1, Duration::from_secs(5))
            .expect("runner")
            .with_scope(loopback_scope_any_port());
    runner.set_common(eggsec::types::CommonHttpArgs {
        proxy: Some("http://127.0.0.1:9".to_string()),
        ..Default::default()
    });
    // run() now uses the pinned Eggfetch backend: closed proxy must fail, never
    // succeed direct. (Reqwest fallback removal is proven by the same shape in
    // the Reqwest unit tests + no-direct-fallback guard.)
    let results = runner
        .run()
        .await
        .expect("run surfaces transport failures as results");
    assert_eq!(
        results.successful_requests, 0,
        "proxied run via closed proxy must not succeed direct"
    );
    assert_eq!(results.failed_requests, 2);
}

#[test]
fn invalid_proxy_fails_before_origin_io() {
    // Plan rejects an invalid proxy URL before any transport/dispatch.
    let input = eggsec::loadtest::adapter::AdapterInput {
        url: "http://127.0.0.1:80/",
        requests: 1,
        concurrency: 1,
        timeout: Duration::from_secs(2),
        method: "GET",
        body: None,
        headers: &[],
        insecure: false,
        proxy: Some("ht!tp://bad proxy"),
        proxy_auth: None,
        auth: None,
        bearer: None,
        cookie: None,
        api_key: None,
        user_agent: None,
        rate_limit: None,
    };
    let res = eggsec::loadtest::adapter::plan_from_adapter(
        input,
        None,
        true,
        None,
        None,
        None,
        None,
        &std::collections::HashMap::new(),
    );
    assert!(
        res.is_err(),
        "invalid proxy must fail before origin I/O: {res:?}"
    );
}

#[test]
fn proxy_userinfo_rejected_before_dispatch() {
    let proxy = Url::parse("http://user:pass@127.0.0.1:8080").expect("url");
    let ultimate = Url::parse("http://127.0.0.1/").expect("url");
    let scope = loopback_scope_any_port();
    let auth = eggsec::loadtest::OwnedScopeAuthority::new(scope);
    // userinfo in either leg must deny at the proxy checkpoint (no secret on wire).
    let err = auth.authorize_proxy(&proxy, &ultimate).unwrap_err();
    assert!(
        matches!(
            err,
            TransportError::PolicyDenied {
                checkpoint: PolicyCheckpoint::Proxy,
                ..
            }
        ),
        "proxy userinfo must deny at proxy checkpoint: {err:?}"
    );
}
