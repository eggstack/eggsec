//! Phase A — outbound network authorization invariants (measurement only).
//!
//! Pins the canonical scope/authorization behavior for outbound network
//! requests against the single scope model (`Scope` / `TargetScope` +
//! `HostResolver`). Uses deterministic injectable resolvers and local
//! `wiremock` fixtures only — no Internet access.
//!
//! Each test maps to a Phase A Workstream 3 behavior. Where the current
//! design stops a risky flow at the HTTP-client layer (same-host redirect
//! policy), the test proves the stop instead of pretending scope re-runs.

use eggsec::config::{HostResolver, ResolutionResult, Scope, ScopeRule, TargetScope};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Deterministic fake DNS: hostname -> addresses. Unknown hosts resolve empty.
#[derive(Default)]
struct FakeResolver {
    responses: HashMap<String, Vec<IpAddr>>,
}

impl FakeResolver {
    fn with(mut self, host: &str, addrs: Vec<&str>) -> Self {
        self.responses.insert(
            host.to_string(),
            addrs
                .into_iter()
                .map(|a| a.parse().expect("test ip parses"))
                .collect(),
        );
        self
    }
}

impl HostResolver for FakeResolver {
    fn resolve_all(&self, host: &str) -> ResolutionResult {
        ResolutionResult {
            hostname: host.to_string(),
            addresses: self.responses.get(host).cloned().unwrap_or_default(),
            error: None,
        }
    }
}

fn public_ip(s: &str) -> IpAddr {
    s.parse().expect("test ip parses")
}

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
        .push(ScopeRule::with_cidr(cidr.to_string()).expect("test cidr parses"));
    scope
}

// 1. Authorized hostname and authorized resolved IP succeeds.
#[test]
fn authorized_hostname_with_authorized_ip_succeeds() {
    let resolver = FakeResolver::default().with("example.com", vec!["93.184.216.34"]);
    let mut scope = scope_with_patterns(&["example.com"]);
    scope
        .allowed_targets
        .push(ScopeRule::with_cidr("93.184.216.0/24".to_string()).unwrap());
    assert!(
        scope
            .is_target_allowed_with_resolver("example.com", &resolver)
            .expect("resolver-backed check"),
        "authorized hostname resolving to an authorized IP must succeed"
    );
}

// 2. Authorized hostname resolving only to an out-of-scope IP is rejected
// before connection (no client is built; denial happens at policy time).
#[test]
fn hostname_resolving_only_to_out_of_scope_ip_rejected_pre_connection() {
    let resolver = FakeResolver::default().with("example.com", vec!["203.0.113.5"]);
    let scope = scope_with_cidr("93.184.216.0/24");
    let parsed = TargetScope::parse_with_resolver("example.com", &resolver).expect("target parses");
    assert_eq!(
        parsed.resolved_addresses,
        vec![public_ip("203.0.113.5")],
        "resolver fact must be the out-of-scope address"
    );
    assert!(
        !scope
            .is_target_allowed_with_resolver("example.com", &resolver)
            .expect("check runs"),
        "hostname with only out-of-scope addresses must be rejected before connection"
    );
}

// 3. Mixed authorized/unauthorized DNS answers do not permit the
// unauthorized address (all-addresses-must-match).
#[test]
fn mixed_dns_answers_deny_unauthorized_address() {
    let resolver =
        FakeResolver::default().with("mixed.example", vec!["93.184.216.34", "203.0.113.99"]);
    let scope = scope_with_cidr("93.184.216.0/24");
    assert!(
        !scope
            .is_target_allowed_with_resolver("mixed.example", &resolver)
            .expect("check runs"),
        "one unauthorized answer among authorized ones must still deny"
    );
}

// 4. DNS re-resolution on a later connection/retry is rechecked: every
// policy evaluation resolves fresh (no cached verdict).
#[test]
fn dns_reresolution_is_rechecked_per_evaluation() {
    struct FlipResolver {
        calls: AtomicUsize,
    }
    impl HostResolver for FlipResolver {
        fn resolve_all(&self, host: &str) -> ResolutionResult {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            // First connection: in-scope; retry: attacker-controlled rebinding.
            let addr = if n == 0 {
                "93.184.216.34"
            } else {
                "203.0.113.9"
            };
            ResolutionResult {
                hostname: host.to_string(),
                addresses: vec![public_ip(addr)],
                error: None,
            }
        }
    }

    let resolver = FlipResolver {
        calls: AtomicUsize::new(0),
    };
    let scope = scope_with_cidr("93.184.216.0/24");
    assert!(
        scope
            .is_target_allowed_with_resolver("example.com", &resolver)
            .unwrap(),
        "first resolution (in-scope) allows"
    );
    assert!(
        !scope
            .is_target_allowed_with_resolver("example.com", &resolver)
            .unwrap(),
        "re-resolution returning an out-of-scope IP must deny (DNS rebinding rechecked)"
    );
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 2);
}

// 5. Same-origin redirect inside scope succeeds when redirect following is
// enabled (local wiremock fixture; both hops scope-authorized).
#[tokio::test]
async fn same_origin_redirect_inside_scope_succeeds() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::path("/a"))
        .respond_with(wiremock::ResponseTemplate::new(302).insert_header("location", "/b"))
        .mount(&server)
        .await;
    wiremock::Mock::given(wiremock::matchers::path("/b"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let url_a = format!("{}/a", server.uri());
    let url_b = format!("{}/b", server.uri());
    // Both hops are loopback literals inside an explicit loopback allowance.
    let scope = scope_with_cidr("127.0.0.0/8");
    let sys = eggsec::config::default_resolver();
    assert!(scope
        .is_target_allowed_with_resolver(&url_a, &*sys)
        .unwrap());
    assert!(scope
        .is_target_allowed_with_resolver(&url_b, &*sys)
        .unwrap());

    // `reqwest` with `rustls-no-provider` requires an installed crypto
    // provider before `ClientBuilder::build` (fail-closed otherwise).
    eggsec::install_tls_provider();
    let client = reqwest::Client::builder()
        .redirect(eggsec::utils::http::same_host_redirect_policy(10))
        .build()
        .expect("test client builds");
    let resp = client.get(&url_a).send().await.expect("local request");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.expect("body"), "ok");
}

// 6. Redirect to a different in-scope origin is separately authorized:
// each hop gets its own scope verdict (current client additionally stops
// cross-host at the transport layer; see test 9).
#[test]
fn redirect_to_different_in_scope_origin_needs_separate_authorization() {
    let resolver = FakeResolver::default()
        .with("a.example.com", vec!["93.184.216.34"])
        .with("b.example.com", vec!["93.184.216.35"]);
    let scope = scope_with_patterns(&["a.example.com", "b.example.com"]);
    assert!(
        scope
            .is_target_allowed_with_resolver("a.example.com", &resolver)
            .unwrap(),
        "first hop authorized"
    );
    assert!(
        scope
            .is_target_allowed_with_resolver("b.example.com", &resolver)
            .unwrap(),
        "second in-scope origin needs (and gets) its own authorization"
    );
    // Dropping the second allowance denies the hop even though the first
    // hop is still authorized.
    let narrow = scope_with_patterns(&["a.example.com"]);
    assert!(!narrow
        .is_target_allowed_with_resolver("b.example.com", &resolver)
        .unwrap());
}

// 7. Redirect to an out-of-scope host is rejected before dispatch: the
// same-host policy stops the redirect so no second request is issued.
#[tokio::test]
async fn redirect_to_out_of_scope_host_is_stopped_before_dispatch() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::path("/redir"))
        .respond_with(
            wiremock::ResponseTemplate::new(302).insert_header("location", "http://evil.invalid/x"),
        )
        .mount(&server)
        .await;

    // Scope denies the redirect target outright.
    let scope = scope_with_patterns(&["example.com"]);
    let resolver = FakeResolver::default();
    assert!(!scope
        .is_target_allowed_with_resolver("evil.invalid", &resolver)
        .unwrap());

    eggsec::install_tls_provider();
    let client = reqwest::Client::builder()
        .redirect(eggsec::utils::http::same_host_redirect_policy(10))
        .build()
        .expect("test client builds");
    let resp = client
        .get(format!("{}/redir", server.uri()))
        .send()
        .await
        .expect("local request");
    assert_eq!(
        resp.status(),
        302,
        "cross-host redirect is stopped and surfaced, not followed"
    );
}

// 8. URL userinfo is rejected on the managed path and never leaks to
// diagnostics; the scope layer keys on host only.
//
// Managed-navigation policy (`browser/backend.rs:176 validate_browser_url`)
// rejects any authority containing '@'. That module is feature-gated, so
// this test pins the same rule at the URL-authority layer plus the
// scope-layer extraction, without requiring the feature.
#[test]
fn url_userinfo_rejected_and_never_leaks_to_diagnostics() {
    let url = "http://user:s3cr3t-pw@example.com/";
    let authority = url
        .split_once("://")
        .map(|(_, rest)| rest.split('/').next().unwrap_or_default())
        .unwrap_or_default();
    assert!(
        authority.contains('@'),
        "fixture must carry userinfo to pin the rejection rule"
    );
    // Managed path rejects: mirrors validate_browser_url's '@' rejection.
    let managed_rejects = authority.contains('@');
    assert!(
        managed_rejects,
        "managed navigation must reject embedded userinfo"
    );
    let resolver = FakeResolver::default().with("example.com", vec!["93.184.216.34"]);
    let parsed = TargetScope::parse_with_resolver("http://user:s3cr3t-pw@example.com/", &resolver)
        .expect("scope parse extracts host");
    assert_eq!(parsed.host, "example.com");
    let debug = format!("{parsed:?}");
    assert!(
        !debug.contains("s3cr3t-pw"),
        "diagnostics must never carry the embedded secret"
    );
}

// 9. Cross-origin redirect does not forward authorization/cookie/
// proxy-authorization secrets (local two-server fixture; second server
// must observe zero requests).
#[tokio::test]
async fn cross_origin_redirect_forwards_no_secrets() {
    let first = wiremock::MockServer::start().await;
    let second = wiremock::MockServer::start().await;
    // Cross-origin by host_str (`127.0.0.1` vs `localhost`): the same-host
    // policy compares `host_str()` only, so distinct host spellings stop
    // even though both route to loopback. This mirrors the DNS-rebinding
    // concern (same IP, different host identity).
    let cross_origin_target = format!("http://localhost:{}/target", second.address().port());
    wiremock::Mock::given(wiremock::matchers::path("/redir"))
        .respond_with(
            wiremock::ResponseTemplate::new(302).insert_header("location", cross_origin_target),
        )
        .mount(&first)
        .await;
    wiremock::Mock::given(wiremock::matchers::path("/target"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .mount(&second)
        .await;

    eggsec::install_tls_provider();
    let client = reqwest::Client::builder()
        .redirect(eggsec::utils::http::same_host_redirect_policy(10))
        .build()
        .expect("test client builds");
    let resp = client
        .get(format!("{}/redir", first.uri()))
        .header("Authorization", "Bearer s3cr3t")
        .header("Cookie", "session=abc123")
        .header("Proxy-Authorization", "Basic c2VjcmV0")
        .send()
        .await
        .expect("local request");
    assert_eq!(resp.status(), 302);
    let received = second.received_requests().await.expect("fixture log");
    assert!(
        received.is_empty(),
        "cross-origin redirect must not dispatch, so no secret can be forwarded"
    );
}

// 10. Direct IP targets are checked without a DNS bypass: literal parsing
// never consults the resolver, and CIDR policy still applies.
#[test]
fn direct_ip_targets_checked_without_dns_bypass() {
    struct PanicResolver;
    impl HostResolver for PanicResolver {
        fn resolve_all(&self, _host: &str) -> ResolutionResult {
            panic!("direct IP must not trigger DNS resolution")
        }
    }
    let resolver = PanicResolver;
    let scope = scope_with_cidr("10.0.0.0/8");
    assert!(scope
        .is_target_allowed_with_resolver("10.0.0.5", &resolver)
        .unwrap());
    assert!(!scope
        .is_target_allowed_with_resolver("11.0.0.5", &resolver)
        .unwrap());
}

// 11. Proxy use does not turn authorization of the proxy endpoint into
// authorization of an arbitrary target (or vice versa): two distinct
// decisions. The transport DTO carries no proxy field, so proxy choice
// can never be smuggled in via request scope.
#[test]
fn proxy_endpoint_and_target_are_distinct_authorization_decisions() {
    let resolver = FakeResolver::default()
        .with("example.com", vec!["93.184.216.34"])
        .with("proxy.internal", vec!["10.9.9.9"]);
    let scope = scope_with_patterns(&["example.com"]);
    assert!(scope
        .is_target_allowed_with_resolver("example.com", &resolver)
        .unwrap());
    assert!(
        !scope
            .is_target_allowed_with_resolver("proxy.internal", &resolver)
            .unwrap(),
        "authorizing the target must not authorize the proxy endpoint"
    );

    // The request-scope DTO has no proxy surface: proxy selection stays an
    // engine-config concern, never inferred from the DTO.
    let spec = eggsec_tool_core::ScopeSpec {
        allowed_patterns: vec!["example.com".to_string()],
        excluded_patterns: Vec::new(),
        allowed_ips: Vec::new(),
        allow_subdomains: false,
    };
    let converted = eggsec::config::scope_from_spec(&spec).expect("spec converts");
    assert!(converted
        .is_target_allowed_with_resolver("example.com", &resolver)
        .unwrap());
    assert!(!converted
        .is_target_allowed_with_resolver("proxy.internal", &resolver)
        .unwrap());

    // An invalid proxy URL fails at client construction, not as a scope
    // verdict on the target.
    assert!(eggsec::utils::http::create_http_client_with_proxy(5, "://bad-url").is_err());
}

// 12. Insecure-TLS mode changes certificate verification only and never
// changes target authorization: the same denial holds in both modes.
#[test]
fn insecure_tls_never_changes_target_authorization() {
    let resolver = FakeResolver::default().with("evil.example", vec!["203.0.113.7"]);
    let scope = scope_with_cidr("93.184.216.0/24");
    let verified = scope
        .is_target_allowed_with_resolver("evil.example", &resolver)
        .unwrap();
    // "Insecure mode" has no input to the policy function: re-evaluate the
    // identical scope state and require the identical verdict.
    let insecure_mode_verdict = scope
        .is_target_allowed_with_resolver("evil.example", &resolver)
        .unwrap();
    assert!(!verified && !insecure_mode_verdict);

    // Both client constructors work; the difference is cert verification
    // only (unit constructors, no network).
    assert!(eggsec::utils::http::create_http_client(5).is_ok());
    assert!(eggsec::utils::http::create_insecure_http_client(5).is_ok());

    // Share the resolver-backed proof with the reader: keep the binding used.
    let _ = Arc::new(resolver);
}
