//! Corrective 2026-09-19 WS2 — supported local-resolution SOCKS5 qualification.
//!
//! Exercises Eggsec authorization plus the production Eggfetch proxy path for
//! the already-supported local-resolution route (`socks5://` with pinned
//! proxy peer + pinned ultimate target). The fixture is test-only and limited
//! to the protocol subset the adapter emits (RFC 1928 CONNECT, no-auth).
//!
//! Proves: exactly one authorized proxy peer + one authorized ultimate target
//! are used, the proxy sees the selected target IP/port (never a hostname),
//! the target receives the request, both selected-socket checkpoints are
//! recorded, and failing selected peers/targets never fall back to another
//! candidate. Existing remote-resolution (`socks5h`) and plaintext
//! forward-proxy fail-closed coverage in `parity.rs` is rerun unchanged.

mod common;

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{AllowAll, Fixture};
use eggsec_transport::{
    HttpTransport, InMemoryResolver, NetworkAuthority, PolicyCheckpoint, ProxyIntent,
    TimeoutPolicy, TransportError, TransportResolver,
};
use eggsec_transport_eggfetch::EggfetchTransport;
use http::Method;
use url::Url;

/// Ordered resolver: returns addresses in the given order (no sorting), so
/// fallback tests control which candidate is primary.
struct OrderedResolver {
    map: HashMap<String, Vec<IpAddr>>,
}

impl OrderedResolver {
    fn new(pairs: Vec<(&str, Vec<&str>)>) -> Self {
        let mut map = HashMap::new();
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

/// Authority that approves every candidate but records selected-socket
/// checkpoints distinctly, plus proxy-decision observations.
#[derive(Debug, Default)]
struct SocksRecorder {
    socket_calls: Mutex<Vec<IpAddr>>,
    proxy_socket_calls: Mutex<Vec<IpAddr>>,
    proxy_calls: Mutex<Vec<String>>,
}

impl SocksRecorder {
    fn socket_calls(&self) -> Vec<IpAddr> {
        self.socket_calls.lock().expect("lock").clone()
    }

    fn proxy_socket_calls(&self) -> Vec<IpAddr> {
        self.proxy_socket_calls.lock().expect("lock").clone()
    }

    fn proxy_calls(&self) -> Vec<String> {
        self.proxy_calls.lock().expect("lock").clone()
    }
}

impl NetworkAuthority for SocksRecorder {
    fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError> {
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

    fn authorize_redirect(&self, _from: &Url, _to: &Url) -> Result<(), TransportError> {
        Ok(())
    }

    fn authorize_proxy(&self, proxy_endpoint: &Url, ultimate: &Url) -> Result<(), TransportError> {
        self.proxy_calls.lock().expect("lock").push(format!(
            "{} -> {}",
            proxy_endpoint.as_str(),
            ultimate.as_str()
        ));
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

/// One SOCKS5 CONNECT destination observed by the fixture.
#[derive(Debug, Clone)]
struct SocksTarget {
    atyp: u8,
    host: String,
    port: u16,
}

/// Minimal loopback SOCKS5 (RFC 1928 CONNECT, no-auth) fixture.
///
/// - Accepts TCP on 127.0.0.1, records each accepted peer (`hits`) and each
///   CONNECT destination (`targets` with ATYP + host + port).
/// - Dials the requested destination: success tunnels bytes via
///   `copy_bidirectional`; failure replies `0x05` (refused) and closes.
/// - Nothing leaves the host.
struct Socks5Proxy {
    addr: SocketAddr,
    hits: Arc<Mutex<usize>>,
    targets: Arc<Mutex<Vec<SocksTarget>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Socks5Proxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Socks5Proxy {
    async fn start() -> Self {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback binds");
        let addr = listener.local_addr().expect("local addr");
        let hits = Arc::new(Mutex::new(0usize));
        let targets = Arc::new(Mutex::new(Vec::new()));
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
                        // Greeting: VER, NMETHODS, METHODS.
                        let mut hdr = [0u8; 2];
                        if inbound.read_exact(&mut hdr).await.is_err() {
                            return;
                        }
                        if hdr[0] != 0x05 {
                            return;
                        }
                        let nmethods = hdr[1] as usize;
                        let mut methods = vec![0u8; nmethods];
                        if inbound.read_exact(&mut methods).await.is_err() {
                            return;
                        }
                        // No-auth only.
                        if inbound.write_all(&[0x05, 0x00]).await.is_err() {
                            return;
                        }
                        // Request: VER, CMD, RSV, ATYP, DST, PORT.
                        let mut req_hdr = [0u8; 4];
                        if inbound.read_exact(&mut req_hdr).await.is_err() {
                            return;
                        }
                        if req_hdr[0] != 0x05 || req_hdr[1] != 0x01 {
                            return;
                        }
                        let atyp = req_hdr[3];
                        let (host, port) = match atyp {
                            0x01 => {
                                let mut ip = [0u8; 4];
                                if inbound.read_exact(&mut ip).await.is_err() {
                                    return;
                                }
                                let mut port_bytes = [0u8; 2];
                                if inbound.read_exact(&mut port_bytes).await.is_err() {
                                    return;
                                }
                                let port = u16::from_be_bytes(port_bytes);
                                let host = std::net::Ipv4Addr::from(ip).to_string();
                                (host, port)
                            }
                            0x03 => {
                                let mut len = [0u8; 1];
                                if inbound.read_exact(&mut len).await.is_err() {
                                    return;
                                }
                                let len = len[0] as usize;
                                let mut domain = vec![0u8; len];
                                if inbound.read_exact(&mut domain).await.is_err() {
                                    return;
                                }
                                let mut port_bytes = [0u8; 2];
                                if inbound.read_exact(&mut port_bytes).await.is_err() {
                                    return;
                                }
                                let port = u16::from_be_bytes(port_bytes);
                                let host = String::from_utf8_lossy(&domain).to_string();
                                (host, port)
                            }
                            0x04 => {
                                let mut ip = [0u8; 16];
                                if inbound.read_exact(&mut ip).await.is_err() {
                                    return;
                                }
                                let mut port_bytes = [0u8; 2];
                                if inbound.read_exact(&mut port_bytes).await.is_err() {
                                    return;
                                }
                                let port = u16::from_be_bytes(port_bytes);
                                let host = std::net::Ipv6Addr::from(ip).to_string();
                                (host, port)
                            }
                            _ => return,
                        };
                        targets.lock().expect("lock").push(SocksTarget {
                            atyp,
                            host: host.clone(),
                            port,
                        });
                        // Dial the requested destination (loopback only in tests).
                        let dial_target = format!("{host}:{port}");
                        match tokio::net::TcpStream::connect(dial_target.as_str()).await {
                            Err(_) => {
                                let resp =
                                    [0x05, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
                                let _ = inbound.write_all(&resp).await;
                            }
                            Ok(mut upstream) => {
                                let resp =
                                    [0x05, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
                                if inbound.write_all(&resp).await.is_err() {
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

    fn targets(&self) -> Vec<SocksTarget> {
        self.targets.lock().expect("lock").clone()
    }
}

fn socks_request(
    origin_host: &str,
    origin_port: u16,
    proxy_port: u16,
) -> eggsec_transport::ScopedHttpRequest {
    eggsec_transport::ScopedHttpRequest::new_with_url(
        Method::GET,
        &format!("http://{origin_host}:{origin_port}/"),
    )
    .expect("request builds")
    .with_proxy(ProxyIntent::All {
        endpoint: Url::parse(&format!("socks5://127.0.0.1:{proxy_port}/")).expect("proxy url"),
        credential: None,
    })
    .with_timeout(TimeoutPolicy {
        request_timeout: Duration::from_secs(10),
        connect_timeout: Some(Duration::from_secs(5)),
    })
}

fn memory_resolver() -> Arc<dyn TransportResolver> {
    Arc::new(
        InMemoryResolver::new()
            .with("origin.local", vec!["127.0.0.1"])
            .with("proxy.local", vec!["127.0.0.1"]),
    )
}

#[tokio::test]
async fn socks5_local_success_uses_authorized_peers_and_ip_target() {
    // Success path: one authorized proxy peer + one authorized ultimate
    // target, proxy sees the selected IP/port (not a hostname), the HTTP
    // target receives the request, both selected-socket checkpoints recorded.
    let target = Fixture::echo().await;
    let proxy = Socks5Proxy::start().await;
    let resolver: Arc<dyn TransportResolver> = Arc::new(
        InMemoryResolver::new()
            .with("proxy.local", vec!["127.0.0.1"])
            .with("origin.local", vec!["127.0.0.1"]),
    );
    let transport = EggfetchTransport::new(resolver);
    let auth = SocksRecorder::default();
    let request = eggsec_transport::ScopedHttpRequest::new_with_url(
        Method::GET,
        &format!("http://origin.local:{}/", target.addr.port()),
    )
    .expect("request")
    .with_proxy(ProxyIntent::All {
        endpoint: Url::parse(&format!("socks5://proxy.local:{}/", proxy.addr.port()))
            .expect("proxy url"),
        credential: None,
    })
    .with_timeout(TimeoutPolicy {
        request_timeout: Duration::from_secs(10),
        connect_timeout: Some(Duration::from_secs(5)),
    });
    let response = transport.execute(&auth, request).await.expect("dispatch");
    assert_eq!(response.status, http::StatusCode::OK);
    // Connection reports the socket-authorized proxy peer.
    let conn = response.connection.expect("connection");
    let expected_peer = SocketAddr::new("127.0.0.1".parse().expect("ip"), proxy.addr.port());
    assert_eq!(conn.remote_addr, Some(expected_peer));
    // Exactly one checkpoint per leg.
    assert_eq!(
        auth.proxy_socket_calls().len(),
        1,
        "one proxy socket checkpoint"
    );
    assert_eq!(
        auth.socket_calls().len(),
        1,
        "one ultimate socket checkpoint"
    );
    assert_eq!(
        auth.proxy_socket_calls()[0].to_string(),
        "127.0.0.1",
        "proxy peer must be loopback"
    );
    assert_eq!(
        auth.socket_calls()[0].to_string(),
        "127.0.0.1",
        "ultimate must be loopback"
    );
    assert_eq!(auth.proxy_calls().len(), 1, "proxy decision observed");
    // Proxy saw the selected IP/port, not a hostname.
    let targets = proxy.targets();
    assert_eq!(targets.len(), 1, "one SOCKS destination: {targets:?}");
    assert_eq!(
        targets[0].atyp, 0x01,
        "local-resolution must send IPv4, got ATYP {:#x}",
        targets[0].atyp
    );
    assert_eq!(targets[0].host, "127.0.0.1");
    assert_eq!(targets[0].port, target.addr.port());
    assert_eq!(proxy.hits(), 1, "one proxy TCP connection");
    assert_eq!(target.received().len(), 1, "target receives the request");
}

#[tokio::test]
async fn socks5_proxy_peer_fallback_is_forbidden() {
    // Proxy candidates: primary 127.0.0.2 (refused) is the only
    // socket-authorized peer; secondary 127.0.0.1 (real SOCKS5) is
    // DNS-approved but never passes authorize_proxy_socket. Must fail on the
    // primary, never touch the secondary.
    let proxy = Socks5Proxy::start().await;
    let bad: IpAddr = "127.0.0.2".parse().expect("ip");
    let good: IpAddr = "127.0.0.1".parse().expect("ip");
    let resolver: Arc<dyn TransportResolver> = Arc::new(OrderedResolver::new(vec![
        ("proxy.local", vec!["127.0.0.2", "127.0.0.1"]),
        ("origin.local", vec!["127.0.0.1"]),
    ]));
    let transport = EggfetchTransport::new(resolver);
    let auth = SocksRecorder::default();
    let request = eggsec_transport::ScopedHttpRequest::new_with_url(
        Method::GET,
        &format!("http://origin.local:{}/", 9),
    )
    .expect("request")
    .with_proxy(ProxyIntent::All {
        endpoint: Url::parse(&format!("socks5://proxy.local:{}/", proxy.addr.port()))
            .expect("proxy url"),
        credential: None,
    })
    .with_timeout(TimeoutPolicy {
        request_timeout: Duration::from_secs(10),
        connect_timeout: Some(Duration::from_secs(3)),
    });
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "primary proxy failure must be Backend, got {err:?}"
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
    assert!(proxy.targets().is_empty());
}

#[tokio::test]
async fn socks5_ultimate_target_fallback_is_forbidden() {
    // Ultimate candidates: primary 127.0.0.2 (no listener; SOCKS dial fails)
    // is the only socket-authorized target; secondary 127.0.0.1 (real HTTP
    // target) is DNS-approved but never passes authorize_socket. Must fail on
    // the primary, never relay to the secondary.
    let target = Fixture::echo().await;
    let proxy = Socks5Proxy::start().await;
    let bad: IpAddr = "127.0.0.2".parse().expect("ip");
    let good: IpAddr = "127.0.0.1".parse().expect("ip");
    let resolver: Arc<dyn TransportResolver> = Arc::new(OrderedResolver::new(vec![
        ("proxy.local", vec!["127.0.0.1"]),
        ("origin.local", vec!["127.0.0.2", "127.0.0.1"]),
    ]));
    let transport = EggfetchTransport::new(resolver);
    let auth = SocksRecorder::default();
    // Use a closed port on 127.0.0.2 as the primary ultimate port so the
    // SOCKS dial deterministically fails; the secondary would be the live
    // target port but must never be attempted.
    let closed_port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.2:0")
            .await
            .expect("bind 127.0.0.2");
        let port = listener.local_addr().expect("port").port();
        drop(listener);
        port
    };
    let request = socks_request("origin.local", closed_port, proxy.addr.port());
    // Override resolver port mapping: the request port is closed_port; the
    // live target port is irrelevant because the secondary must never dial.
    let err = transport.execute(&auth, request).await.unwrap_err();
    assert!(
        matches!(err, TransportError::Backend(_)),
        "primary ultimate failure must be Backend, got {err:?}"
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
        "exactly one SOCKS destination: {targets:?}"
    );
    assert_eq!(targets[0].host, "127.0.0.2");
    assert_eq!(targets[0].port, closed_port);
    assert!(
        target.received().is_empty(),
        "secondary ultimate must receive no bytes"
    );
}

#[tokio::test]
async fn socks5_rerun_fail_closed_shapes_stay_green() {
    // Rerun the existing unsupported-shape coverage through the same
    // production adapter path: SOCKS5H remote-DNS + plaintext forward-proxy
    // remain fail closed at the Proxy checkpoint.
    let server = Fixture::echo().await;
    let transport = EggfetchTransport::new(memory_resolver());
    let auth = AllowAll;
    let request = eggsec_transport::ScopedHttpRequest::new_with_url(
        Method::GET,
        &format!("http://origin.local:{}/", server.addr.port()),
    )
    .expect("request")
    .with_proxy(ProxyIntent::All {
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
    assert!(server.received().is_empty());

    let plain = eggsec_transport::ScopedHttpRequest::new_with_url(
        Method::GET,
        &format!("http://origin.local:{}/", server.addr.port()),
    )
    .expect("request")
    .with_proxy(ProxyIntent::All {
        endpoint: Url::parse("http://127.0.0.1:8080").expect("proxy url"),
        credential: None,
    });
    let err = transport.execute(&auth, plain).await.unwrap_err();
    assert!(
        matches!(
            err,
            TransportError::PolicyDenied {
                checkpoint: PolicyCheckpoint::Proxy,
                ..
            }
        ),
        "plaintext forward-proxy must fail at proxy checkpoint, got {err:?}"
    );
}
