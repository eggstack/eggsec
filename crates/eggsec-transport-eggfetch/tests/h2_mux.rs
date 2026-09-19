//! Corrective 2026-09-19 WS1 (strengthened by the final-qualification
//! deep-check pass) — Eggsec-local H2-over-TLS qualification.
//!
//! Exercises [`EggfetchTransport`] (never `eggfetch-core` directly) against a
//! deterministic loopback H2-over-TLS fixture. Proves ALPN `h2`, concurrent
//! multiplexing on one physical connection, sequential reuse, logical-origin
//! isolation, and selected-resolved-address isolation with an unchanged
//! logical origin — all through the production resolved-route path
//! (logical hostname + singular authorized loopback `SocketAddr`).
//!
//! The fixture uses an explicit insecure TLS policy: it qualifies
//! ALPN/multiplexing only. Verified-TLS/SNI semantics remain covered by the
//! existing `parity.rs` certificate fixtures, which stay green.

use std::net::{IpAddr, SocketAddr};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use eggsec_transport::{
    HttpTransport, InMemoryResolver, ResolvedCandidates, TlsPolicy, TransportResolver,
};
use eggsec_transport_eggfetch::EggfetchTransport;
use http::Method;

mod common;
use common::AllowAll;

/// Deterministic H2-over-TLS loopback fixture.
///
/// - Binds `127.0.0.1:0`, serves H2 over rustls with ALPN `h2` only.
/// - Counts accepted TCP connections (`accepts`), H2 request streams
///   (`streams`), and peak concurrent live streams (`max_concurrent`).
/// - Records negotiated ALPN per connection and `:authority` per stream.
/// - Each stream sleeps `delay` before responding so concurrent requests can
///   be proven overlapping (not merely sequential keep-alive).
struct H2TlsFixture {
    addr: SocketAddr,
    accepts: Arc<AtomicUsize>,
    streams: Arc<AtomicUsize>,
    current: Arc<AtomicUsize>,
    max_concurrent: Arc<AtomicUsize>,
    alpns: Arc<Mutex<Vec<Option<Vec<u8>>>>>,
    authorities: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for H2TlsFixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Guard that decrements the live-stream counter exactly once when the
/// response task exits, regardless of response-send success/failure.
/// Single-exit accounting: a future early return inside the task cannot
/// reintroduce a double-decrement.
struct LiveGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for LiveGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

impl H2TlsFixture {
    async fn start(delay: Duration) -> Self {
        Self::start_on(
            SocketAddr::new("127.0.0.1".parse().expect("loopback ip"), 0),
            delay,
        )
        .await
    }

    async fn start_on(bind: SocketAddr, delay: Duration) -> Self {
        let sans = vec![
            "127.0.0.1".to_string(),
            "127.0.0.2".to_string(),
            "h2.local".to_string(),
            "a.local".to_string(),
            "b.local".to_string(),
        ];
        let params = rcgen::CertificateParams::new(sans).expect("cert params");
        let key_pair = rcgen::KeyPair::generate().expect("key pair");
        let cert = params.self_signed(&key_pair).expect("self-signed");
        let cert_der = cert.der().to_vec();
        let key_der = key_pair.serialize_der();
        let mut server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![rustls::pki_types::CertificateDer::from(cert_der)],
                rustls::pki_types::PrivateKeyDer::try_from(key_der).expect("private key"),
            )
            .expect("server config");
        // Advertise H2 only so negotiation proves the client offers H2.
        server_config.alpn_protocols = vec![b"h2".to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

        let listener = tokio::net::TcpListener::bind(bind)
            .await
            .expect("loopback binds");
        let addr = listener.local_addr().expect("local addr");
        let accepts = Arc::new(AtomicUsize::new(0));
        let streams = Arc::new(AtomicUsize::new(0));
        let current = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));
        let alpns: Arc<Mutex<Vec<Option<Vec<u8>>>>> = Arc::new(Mutex::new(Vec::new()));
        let authorities: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        let task = {
            let accepts = accepts.clone();
            let streams = streams.clone();
            let current = current.clone();
            let max_concurrent = max_concurrent.clone();
            let alpns = alpns.clone();
            let authorities = authorities.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        return;
                    };
                    accepts.fetch_add(1, Ordering::SeqCst);
                    let acceptor = acceptor.clone();
                    let streams = streams.clone();
                    let current = current.clone();
                    let max_concurrent = max_concurrent.clone();
                    let alpns = alpns.clone();
                    let authorities = authorities.clone();
                    tokio::spawn(async move {
                        let Ok(tls) = acceptor.accept(stream).await else {
                            return;
                        };
                        let negotiated = tls.get_ref().1.alpn_protocol().map(|v| v.to_vec());
                        alpns.lock().expect("lock").push(negotiated);
                        let mut conn = match h2::server::handshake(tls).await {
                            Ok(c) => c,
                            Err(_) => return,
                        };
                        while let Some(result) = conn.accept().await {
                            let (request, mut respond) = match result {
                                Ok(pair) => pair,
                                Err(_) => return,
                            };
                            streams.fetch_add(1, Ordering::SeqCst);
                            let live = current.fetch_add(1, Ordering::SeqCst) + 1;
                            max_concurrent.fetch_max(live, Ordering::SeqCst);
                            let authority = request
                                .uri()
                                .authority()
                                .map(|a| a.to_string())
                                .unwrap_or_default();
                            authorities.lock().expect("lock").push(authority);
                            let current = current.clone();
                            tokio::spawn(async move {
                                let _live = LiveGuard {
                                    counter: current.clone(),
                                };
                                if !delay.is_zero() {
                                    tokio::time::sleep(delay).await;
                                }
                                let response = http::Response::builder()
                                    .status(200)
                                    .body(())
                                    .expect("response");
                                if let Ok(mut send) = respond.send_response(response, false) {
                                    let _ = send.send_data(bytes::Bytes::from_static(b"ok"), true);
                                }
                            });
                        }
                    });
                }
            })
        };
        Self {
            addr,
            accepts,
            streams,
            current,
            max_concurrent,
            alpns,
            authorities,
            task,
        }
    }

    fn accepts(&self) -> usize {
        self.accepts.load(Ordering::SeqCst)
    }

    fn streams(&self) -> usize {
        self.streams.load(Ordering::SeqCst)
    }

    fn max_concurrent(&self) -> usize {
        self.max_concurrent.load(Ordering::SeqCst)
    }

    fn live(&self) -> usize {
        self.current.load(Ordering::SeqCst)
    }

    /// Wait until no response task holds a live stream (bounded poll so a
    /// future accounting regression fails loudly instead of hanging the
    /// suite). Response tasks decrement after the client-visible body send,
    /// so callers must quiesce before asserting a zero count.
    async fn wait_quiescent(&self, timeout: Duration) {
        let start = std::time::Instant::now();
        while self.live() != 0 && start.elapsed() < timeout {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    fn alpns(&self) -> Vec<Option<Vec<u8>>> {
        self.alpns.lock().expect("lock").clone()
    }

    fn authorities(&self) -> Vec<String> {
        self.authorities.lock().expect("lock").clone()
    }
}

fn h2_resolver(hosts: &[&str]) -> Arc<dyn TransportResolver> {
    let mut resolver = InMemoryResolver::new();
    for host in hosts {
        resolver = resolver.with(*host, vec!["127.0.0.1"]);
    }
    Arc::new(resolver)
}

/// Resolver with a scripted per-call answer sequence (selected-address
/// rotation fixtures). Calls beyond the script reuse the final answer so a
/// revisit step can re-select an earlier snapshot. Test-local: facts only,
/// policy stays with the per-call authority.
struct ScriptedResolver {
    script: Vec<Vec<IpAddr>>,
    calls: AtomicUsize,
}

impl ScriptedResolver {
    fn new(script: Vec<Vec<&str>>) -> Self {
        let parse = |list: Vec<&str>| {
            list.into_iter()
                .map(|s| s.parse().expect("test IP parses"))
                .collect()
        };
        Self {
            script: script.into_iter().map(parse).collect(),
            calls: AtomicUsize::new(0),
        }
    }
}

impl TransportResolver for ScriptedResolver {
    fn resolve(&self, host: &str) -> ResolvedCandidates {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        let addrs = self
            .script
            .get(n)
            .or_else(|| self.script.last())
            .cloned()
            .unwrap_or_default();
        ResolvedCandidates::new(host, addrs)
    }
}

fn h2_get(url: &str) -> eggsec_transport::ScopedHttpRequest {
    eggsec_transport::ScopedHttpRequest::new_with_url(Method::GET, url)
        .expect("request builds")
        .with_tls(TlsPolicy::insecure())
        .with_timeout(eggsec_transport::TimeoutPolicy {
            request_timeout: Duration::from_secs(15),
            connect_timeout: Some(Duration::from_secs(5)),
        })
}

#[tokio::test]
async fn h2_concurrent_same_route_multiplexes_on_one_connection() {
    // Warm the route (establishes the single H2 connection), then run four
    // concurrent requests with a 300ms server hold: they must overlap on the
    // warmed connection (H1 could not do this on one connection), with
    // concurrent-phase stream count == request count and peak overlap >= 2.
    //
    // A cold-start burst may legitimately open parallel connections while the
    // first H2 handshake is still in flight (Hyper owns that race); warming
    // isolates the multiplexing proof from connection-establishment racing.
    let server = H2TlsFixture::start(Duration::from_millis(300)).await;
    let transport = Arc::new(EggfetchTransport::new(h2_resolver(&["h2.local"])));
    let url = format!("https://h2.local:{}/sync", server.addr.port());
    let warm_auth = AllowAll;
    let warm = transport
        .execute(&warm_auth, h2_get(&url))
        .await
        .expect("warm H2");
    assert_eq!(warm.status, http::StatusCode::OK);
    assert_eq!(server.accepts(), 1, "warm must establish one connection");
    let streams_after_warm = server.streams();
    assert_eq!(streams_after_warm, 1);
    let max_after_warm = server.max_concurrent();

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let transport = transport.clone();
            let url = url.clone();
            tokio::spawn(async move {
                let auth = AllowAll;
                transport.execute(&auth, h2_get(&url)).await
            })
        })
        .collect();
    let mut bodies = 0;
    for handle in handles {
        let response = handle.await.expect("task join").expect("dispatch");
        assert_eq!(response.status, http::StatusCode::OK);
        assert_eq!(response.body.as_ref(), b"ok");
        bodies += 1;
    }
    assert_eq!(bodies, 4);
    assert_eq!(
        server.accepts(),
        1,
        "concurrent same-route H2 must reuse the warmed physical connection"
    );
    assert_eq!(
        server.streams() - streams_after_warm,
        4,
        "concurrent-phase stream count must equal request count"
    );
    assert!(
        server.max_concurrent() >= 2,
        "at least two streams must be live concurrently, got {} (warm peak {})",
        server.max_concurrent(),
        max_after_warm
    );
    let alpns = server.alpns();
    assert_eq!(alpns.len(), 1, "one TLS connection, got {alpns:?}");
    assert_eq!(
        alpns[0].as_deref(),
        Some(b"h2".as_slice()),
        "ALPN must negotiate h2, got {alpns:?}"
    );
    // Logical :authority tracks the logical hostname, not the pinned literal.
    for authority in server.authorities() {
        assert!(
            authority.starts_with("h2.local:"),
            "authority must name logical host, got {authority}"
        );
    }
    server.wait_quiescent(Duration::from_secs(5)).await;
    assert_eq!(server.live(), 0, "no live streams after quiescence");
}

#[tokio::test]
async fn h2_sequential_same_route_reuses_one_connection() {
    // Five sequential requests: one accepted connection, five streams, ALPN h2.
    let server = H2TlsFixture::start(Duration::ZERO).await;
    let transport = EggfetchTransport::new(h2_resolver(&["h2.local"]));
    let auth = AllowAll;
    let url = format!("https://h2.local:{}/seq", server.addr.port());
    for _ in 0..5 {
        let response = transport
            .execute(&auth, h2_get(&url))
            .await
            .expect("sequential H2");
        assert_eq!(response.status, http::StatusCode::OK);
        assert_eq!(response.body.as_ref(), b"ok");
    }
    assert_eq!(
        server.accepts(),
        1,
        "sequential same-route H2 must reuse one connection"
    );
    assert_eq!(server.streams(), 5);
    let alpns = server.alpns();
    assert_eq!(alpns.len(), 1);
    assert_eq!(alpns[0].as_deref(), Some(b"h2".as_slice()));
    server.wait_quiescent(Duration::from_secs(5)).await;
    assert_eq!(server.live(), 0, "no live streams after quiescence");
}

#[tokio::test]
async fn h2_selected_socket_change_does_not_reuse_old_connection() {
    // Two H2 servers (different ports => different selected sockets AND
    // different logical origins): each must accept its own connection;
    // revisiting the first must reuse it. This proves route differentiation
    // across logical ports, not selected-address isolation on its own — see
    // h2_selected_address_change_with_same_origin_* for the address-only
    // proof with an unchanged logical origin.
    let first = H2TlsFixture::start(Duration::ZERO).await;
    let second = H2TlsFixture::start(Duration::ZERO).await;
    assert_ne!(first.addr.port(), second.addr.port());
    let transport = EggfetchTransport::new(h2_resolver(&["h2.local"]));
    let auth = AllowAll;
    transport
        .execute(
            &auth,
            h2_get(&format!("https://h2.local:{}/", first.addr.port())),
        )
        .await
        .expect("first H2");
    transport
        .execute(
            &auth,
            h2_get(&format!("https://h2.local:{}/", second.addr.port())),
        )
        .await
        .expect("second H2");
    assert_eq!(first.accepts(), 1);
    assert_eq!(second.accepts(), 1);
    assert_eq!(first.streams(), 1);
    assert_eq!(second.streams(), 1);
    transport
        .execute(
            &auth,
            h2_get(&format!("https://h2.local:{}/", first.addr.port())),
        )
        .await
        .expect("first again");
    assert_eq!(
        first.accepts(),
        1,
        "same selected socket must reuse, not redial"
    );
    assert_eq!(first.streams(), 2);
    assert_eq!(second.accepts(), 1);
    first.wait_quiescent(Duration::from_secs(5)).await;
    second.wait_quiescent(Duration::from_secs(5)).await;
    assert_eq!(first.live(), 0, "no live streams after quiescence");
    assert_eq!(second.live(), 0, "no live streams after quiescence");
}

#[tokio::test]
async fn h2_selected_address_change_with_same_origin_does_not_reuse_old_connection() {
    // Selected-address-only isolation: the logical origin (scheme, hostname,
    // port) is identical for every request while the authorized resolved
    // address rotates across authorization cycles. The route key must
    // include the selected physical address, not just the logical origin.
    //
    // Two H2-over-TLS listeners share one TCP port on different loopback
    // addresses (`127.0.0.1:P` and `127.0.0.2:P`); one logical URL such as
    // `https://h2.local:P/` addresses both. The scripted resolver returns
    // the first address, then the second, then the first again.
    let first = H2TlsFixture::start(Duration::ZERO).await;
    let port = first.addr.port();
    let second = H2TlsFixture::start_on(
        SocketAddr::new("127.0.0.2".parse().expect("loopback ip"), port),
        Duration::ZERO,
    )
    .await;
    assert_eq!(second.addr.port(), port, "same TCP port on 127.0.0.2");
    assert_ne!(first.addr.ip(), second.addr.ip());
    let resolver: Arc<dyn TransportResolver> = Arc::new(ScriptedResolver::new(vec![
        vec!["127.0.0.1"],
        vec!["127.0.0.2"],
        vec!["127.0.0.1"],
    ]));
    let transport = EggfetchTransport::new(resolver);
    let auth = AllowAll;
    let url = format!("https://h2.local:{port}/");
    transport
        .execute(&auth, h2_get(&url))
        .await
        .expect("first selected address");
    assert_eq!(first.accepts(), 1, "first snapshot dials its listener");
    assert_eq!(
        second.accepts(),
        0,
        "unselected listener must see no connection"
    );
    assert_eq!(first.streams(), 1);
    transport
        .execute(&auth, h2_get(&url))
        .await
        .expect("rotated selected address");
    assert_eq!(
        first.accepts(),
        1,
        "rotated selected address must not reuse the old connection"
    );
    assert_eq!(second.accepts(), 1, "rotated snapshot dials its listener");
    assert_eq!(second.streams(), 1);
    transport
        .execute(&auth, h2_get(&url))
        .await
        .expect("first snapshot again");
    assert_eq!(
        first.accepts(),
        1,
        "revisiting a cached selected-address snapshot must reuse, not redial"
    );
    assert_eq!(first.streams(), 2);
    assert_eq!(second.accepts(), 1);
    // Logical :authority tracks the logical hostname on every hop, never
    // the pinned loopback literal.
    for authority in first
        .authorities()
        .iter()
        .chain(second.authorities().iter())
    {
        assert!(
            authority.starts_with("h2.local:"),
            "authority must name logical host, got {authority}"
        );
    }
    // ALPN h2 was negotiated on both physical connections.
    for (label, alpns) in [("first", first.alpns()), ("second", second.alpns())] {
        assert_eq!(alpns.len(), 1, "{label} listener: one TLS connection");
        assert_eq!(
            alpns[0].as_deref(),
            Some(b"h2".as_slice()),
            "{label} listener: ALPN must negotiate h2"
        );
    }
    first.wait_quiescent(Duration::from_secs(5)).await;
    second.wait_quiescent(Duration::from_secs(5)).await;
    assert_eq!(first.live(), 0, "no live streams after quiescence");
    assert_eq!(second.live(), 0, "no live streams after quiescence");
}

#[tokio::test]
async fn h2_logical_origin_change_does_not_reuse_connection() {
    // Same physical socket, different logical origins: route-key isolation
    // must prevent cross-origin H2 reuse (2 accepts on one listener).
    let server = H2TlsFixture::start(Duration::ZERO).await;
    let port = server.addr.port();
    let transport = EggfetchTransport::new(h2_resolver(&["a.local", "b.local"]));
    let auth = AllowAll;
    transport
        .execute(&auth, h2_get(&format!("https://a.local:{port}/")))
        .await
        .expect("a.local H2");
    transport
        .execute(&auth, h2_get(&format!("https://b.local:{port}/")))
        .await
        .expect("b.local H2");
    assert_eq!(
        server.accepts(),
        2,
        "different logical origins must not share an H2 connection"
    );
    assert_eq!(server.streams(), 2);
    let authorities = server.authorities();
    assert_eq!(authorities.len(), 2);
    assert!(authorities.iter().any(|a| a.starts_with("a.local:")));
    assert!(authorities.iter().any(|a| a.starts_with("b.local:")));
    server.wait_quiescent(Duration::from_secs(5)).await;
    assert_eq!(server.live(), 0, "no live streams after quiescence");
}
