//! Eggress 1.0.8 Phase A protocol parity fixtures (local-only).
//!
//! Deterministic fixtures proving production `ProxyManager` execution via
//! the Eggress adapter matches legacy SOCKS/HTTP-CONNECT behavior. No
//! Internet, root, external Tor, or manually managed proxy process required.
//!
//! Targets use documentation IPs (`203.0.113.1`, `2001:db8::1`) for
//! `ProxyManager` paths (which retain private-IP rejection), and loopback
//! for adapter-direct paths (which bypass resolution policy by design).

use eggsec_web_proxy::eggress_outbound as adapter;
use eggsec_web_proxy::{ProxyConfig, ProxyEntry, ProxyManager, ProxyType};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const TIMEOUT: Duration = Duration::from_secs(5);
const SHORT_TIMEOUT: Duration = Duration::from_millis(800);

// ---------------------------------------------------------------------------
// Fake proxy servers (minimal, local-only)
// ---------------------------------------------------------------------------

struct SeenTarget {
    atyp: u8,
    host: String,
    port: u16,
}

async fn handle_socks5_one(
    mut stream: TcpStream,
    require_auth: bool,
    user: String,
    pass: String,
    fail_connect_code: Option<u8>,
    seen: Option<Arc<tokio::sync::Mutex<Option<SeenTarget>>>>,
    forward: bool,
) {
    // Handshake: VER NMETHODS METHODS
    let mut hdr = [0u8; 2];
    if stream.read_exact(&mut hdr).await.is_err() {
        return;
    }
    let nmethods = hdr[1] as usize;
    let mut methods = vec![0u8; nmethods];
    if stream.read_exact(&mut methods).await.is_err() {
        return;
    }
    if require_auth {
        if !methods.contains(&0x02) {
            let _ = stream.write_all(&[0x05, 0xFF]).await;
            return;
        }
        if stream.write_all(&[0x05, 0x02]).await.is_err() {
            return;
        }
        // Auth: VER ULEN U PASS...
        let mut ahdr = [0u8; 2];
        if stream.read_exact(&mut ahdr).await.is_err() {
            return;
        }
        let ulen = ahdr[1] as usize;
        let mut ubuf = vec![0u8; ulen];
        if stream.read_exact(&mut ubuf).await.is_err() {
            return;
        }
        let mut plen = [0u8; 1];
        if stream.read_exact(&mut plen).await.is_err() {
            return;
        }
        let mut pbuf = vec![0u8; plen[0] as usize];
        if stream.read_exact(&mut pbuf).await.is_err() {
            return;
        }
        if ubuf != user.as_bytes() || pbuf != pass.as_bytes() {
            let _ = stream.write_all(&[0x01, 0x01]).await;
            return;
        }
        if stream.write_all(&[0x01, 0x00]).await.is_err() {
            return;
        }
    } else if stream.write_all(&[0x05, 0x00]).await.is_err() {
        return;
    }
    // CONNECT request: VER CMD RSV ATYP ...
    let mut req = [0u8; 4];
    if stream.read_exact(&mut req).await.is_err() {
        return;
    }
    let atyp = req[3];
    let (host_str, port) = match atyp {
        0x01 => {
            let mut b = [0u8; 6];
            if stream.read_exact(&mut b).await.is_err() {
                return;
            }
            let ip = std::net::Ipv4Addr::new(b[0], b[1], b[2], b[3]);
            let port = u16::from_be_bytes([b[4], b[5]]);
            (ip.to_string(), port)
        }
        0x04 => {
            let mut b = [0u8; 18];
            if stream.read_exact(&mut b).await.is_err() {
                return;
            }
            let mut segs = [0u16; 8];
            for (i, chunk) in b[..16].chunks(2).enumerate() {
                segs[i] = u16::from_be_bytes([chunk[0], chunk[1]]);
            }
            let ip = std::net::Ipv6Addr::from(segs);
            let port = u16::from_be_bytes([b[16], b[17]]);
            (ip.to_string(), port)
        }
        0x03 => {
            let mut l = [0u8; 1];
            if stream.read_exact(&mut l).await.is_err() {
                return;
            }
            let mut d = vec![0u8; l[0] as usize];
            if stream.read_exact(&mut d).await.is_err() {
                return;
            }
            let mut p = [0u8; 2];
            if stream.read_exact(&mut p).await.is_err() {
                return;
            }
            (
                String::from_utf8_lossy(&d).into_owned(),
                u16::from_be_bytes(p),
            )
        }
        _ => return,
    };
    if let Some(seen) = seen {
        *seen.lock().await = Some(SeenTarget {
            atyp,
            host: host_str.clone(),
            port,
        });
    }
    if let Some(code) = fail_connect_code {
        let _ = stream
            .write_all(&[0x05, code, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
            .await;
        return;
    }
    if forward {
        // Dial the requested target and relay (used for chaining).
        let target: SocketAddr = format!("{}:{}", host_str, port).parse().unwrap();
        match TcpStream::connect(target).await {
            Ok(mut up) => {
                if stream
                    .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                    .await
                    .is_err()
                {
                    return;
                }
                let _ = tokio::io::copy_bidirectional(&mut stream, &mut up).await;
            }
            Err(_) => {
                let _ = stream
                    .write_all(&[0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                    .await;
            }
        }
        return;
    }
    // Success without dialing (parity harness: proxy need not reach target).
    let _ = stream
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await;
    // Hold open briefly so the client can observe establishment.
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        let mut buf = [0u8; 1];
        let _ = stream.read(&mut buf).await;
    })
    .await;
}

async fn spawn_socks5(
    require_auth: bool,
    user: &str,
    pass: &str,
    fail_code: Option<u8>,
    seen: Option<Arc<tokio::sync::Mutex<Option<SeenTarget>>>>,
    forward: bool,
) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (u, p) = (user.to_string(), pass.to_string());
    tokio::spawn(async move {
        loop {
            let Ok((s, _)) = listener.accept().await else {
                return;
            };
            let (u, p) = (u.clone(), p.clone());
            let seen = seen.clone();
            tokio::spawn(handle_socks5_one(
                s,
                require_auth,
                u,
                p,
                fail_code,
                seen,
                forward,
            ));
        }
    });
    addr
}

async fn handle_socks4_one(mut stream: TcpStream, forward: bool) {
    let mut req = [0u8; 8];
    if stream.read_exact(&mut req).await.is_err() {
        return;
    }
    if req[0] != 0x04 || req[1] != 0x01 {
        return;
    }
    let port = u16::from_be_bytes([req[2], req[3]]);
    let ip = std::net::Ipv4Addr::new(req[4], req[5], req[6], req[7]);
    // Consume userid terminator.
    loop {
        let mut b = [0u8; 1];
        if stream.read_exact(&mut b).await.is_err() {
            return;
        }
        if b[0] == 0x00 {
            break;
        }
    }
    if forward {
        let target: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(ip), port);
        match TcpStream::connect(target).await {
            Ok(mut up) => {
                if stream
                    .write_all(&[0x00, 0x5A, req[2], req[3], req[4], req[5], req[6], req[7]])
                    .await
                    .is_err()
                {
                    return;
                }
                let _ = tokio::io::copy_bidirectional(&mut stream, &mut up).await;
            }
            Err(_) => {
                let _ = stream.write_all(&[0x00, 0x5B, 0, 0, 0, 0, 0, 0]).await;
            }
        }
        return;
    }
    let _ = stream
        .write_all(&[0x00, 0x5A, req[2], req[3], req[4], req[5], req[6], req[7]])
        .await;
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        let mut buf = [0u8; 1];
        let _ = stream.read(&mut buf).await;
    })
    .await;
}

async fn spawn_socks4(forward: bool) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((s, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(handle_socks4_one(s, forward));
        }
    });
    addr
}

async fn handle_http_one(
    mut stream: TcpStream,
    require_auth: bool,
    expected_auth: String,
    status: u16,
    malformed: bool,
    oversized: bool,
) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    // Read until header terminator (bounded).
    let found = loop {
        match tokio::time::timeout(Duration::from_secs(5), stream.read(&mut tmp)).await {
            Ok(Ok(0)) | Ok(Err(_)) | Err(_) => break false,
            Ok(Ok(n)) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() > 128 * 1024 {
                    break false;
                }
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break true;
                }
                if buf.len() > 8192 && !malformed {
                    continue;
                }
            }
        }
    };
    if !found {
        return;
    }
    let req = String::from_utf8_lossy(&buf).into_owned();
    if require_auth && !req.contains(&expected_auth) {
        let _ = stream
            .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
            .await;
        return;
    }
    if malformed {
        let _ = stream.write_all(b"GARBAGE-NOT-HTTP\r\n\r\n").await;
        return;
    }
    if oversized {
        let big = "X-Fill: a".repeat(20000);
        let resp = format!("HTTP/1.1 200 OK\r\n{}...\r\n\r\n", &big[..64000]);
        let _ = stream.write_all(resp.as_bytes()).await;
        return;
    }
    let reason = match status {
        200 => "Connection Established",
        403 => "Forbidden",
        500 => "Internal Server Error",
        _ => "Error",
    };
    let resp = format!("HTTP/1.1 {} {}\r\n\r\n", status, reason);
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        let mut b = [0u8; 1];
        let _ = stream.read(&mut b).await;
    })
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn spawn_http(
    require_auth: bool,
    expected_auth: &str,
    status: u16,
    malformed: bool,
    oversized: bool,
) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let exp = expected_auth.to_string();
    tokio::spawn(async move {
        loop {
            let Ok((s, _)) = listener.accept().await else {
                return;
            };
            let exp = exp.clone();
            tokio::spawn(handle_http_one(
                s,
                require_auth,
                exp,
                status,
                malformed,
                oversized,
            ));
        }
    });
    addr
}

fn entry_with_addr(t: ProxyType, addr: SocketAddr) -> ProxyEntry {
    ProxyEntry::new(t, addr.ip().to_string(), addr.port())
}

async fn manager_with(entry: ProxyEntry) -> ProxyManager {
    let mut config = ProxyConfig::default();
    config.health_check_enabled = false;
    let mgr = ProxyManager::new(config).unwrap();
    mgr.add_proxy(entry).await.unwrap();
    mgr
}

// ---------------------------------------------------------------------------
// Parity tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn socks5_no_auth_success_via_manager() {
    let proxy_addr = spawn_socks5(false, "", "", None, None, false).await;
    let mgr = manager_with(entry_with_addr(ProxyType::Socks5, proxy_addr)).await;
    let conn = mgr.create_connection("203.0.113.1:80").await.unwrap();
    assert_eq!(conn.proxy_chain.len(), 1);
    assert_eq!(conn.target_addr.ip().to_string(), "203.0.113.1");
}

#[tokio::test]
async fn socks5_auth_success() {
    let proxy_addr = spawn_socks5(true, "bob", "s3cret-x1", None, None, false).await;
    let e = entry_with_addr(ProxyType::Socks5, proxy_addr)
        .with_auth("bob".to_string(), "s3cret-x1".to_string());
    let info = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, TIMEOUT)
        .await
        .unwrap();
    // Upstream 1.0.8 gap: local_addr is always None; hop_count/peer_addr prove establishment.
    assert_eq!(info.hop_count, 1);
    assert!(info.peer_addr.is_some());
}

#[tokio::test]
async fn socks5_auth_rejection_no_leak() {
    let proxy_addr = spawn_socks5(true, "bob", "right", None, None, false).await;
    let e = entry_with_addr(ProxyType::Socks5, proxy_addr)
        .with_auth("bob".to_string(), "WRONG-secret-zz".to_string());
    let err = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, TIMEOUT)
        .await
        .unwrap_err();
    let msg = format!("{:?} {}", err, err);
    assert!(!msg.contains("WRONG-secret-zz"), "credential leaked: {msg}");
    assert!(!msg.contains("right"));
}

#[tokio::test]
async fn socks5_destination_failure() {
    // 0x05 = connection refused from proxy.
    let proxy_addr = spawn_socks5(false, "", "", Some(0x05), None, false).await;
    let e = entry_with_addr(ProxyType::Socks5, proxy_addr);
    let err = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, TIMEOUT)
        .await
        .unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn socks5_domain_target_reaches_proxy_as_domain() {
    let seen = Arc::new(tokio::sync::Mutex::new(None));
    let proxy_addr = spawn_socks5(false, "", "", None, Some(seen.clone()), false).await;
    // Adapter-level remote-domain path.
    let e = entry_with_addr(ProxyType::Socks5, proxy_addr);
    let info = adapter::establish(std::slice::from_ref(&e), "example.com", 443, TIMEOUT)
        .await
        .unwrap();
    assert_eq!(info.hop_count, 1);
    let seen = seen.lock().await;
    let seen = seen.as_ref().unwrap();
    assert_eq!(seen.atyp, 0x03, "proxy must receive domain encoding");
    assert_eq!(seen.host, "example.com");
    assert_eq!(seen.port, 443);
}

#[tokio::test]
async fn socks5_manager_domain_path() {
    let seen = Arc::new(tokio::sync::Mutex::new(None));
    let proxy_addr = spawn_socks5(false, "", "", None, Some(seen.clone()), false).await;
    let mgr = manager_with(entry_with_addr(ProxyType::Socks5, proxy_addr)).await;
    let conn = mgr
        .create_connection_to_domain("example.com", 443)
        .await
        .unwrap();
    assert_eq!(conn.proxy_chain.len(), 1);
    let seen = seen.lock().await;
    assert_eq!(seen.as_ref().unwrap().host, "example.com");
}

#[tokio::test]
async fn socks5_ipv4_and_ipv6_encoding() {
    let proxy_addr = spawn_socks5(false, "", "", None, None, false).await;
    let e = entry_with_addr(ProxyType::Socks5, proxy_addr);
    // IPv4 literal.
    let v4 = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, TIMEOUT)
        .await
        .unwrap();
    assert!(v4.peer_addr.is_some());
    // IPv6 literal.
    let v6 = adapter::establish(std::slice::from_ref(&e), "::1", 80, TIMEOUT)
        .await
        .unwrap();
    assert!(v6.peer_addr.is_some());
}

#[tokio::test]
async fn socks4_ip_target_success() {
    let proxy_addr = spawn_socks4(false).await;
    let e = entry_with_addr(ProxyType::Socks4, proxy_addr);
    let info = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, TIMEOUT)
        .await
        .unwrap();
    assert_eq!(info.hop_count, 1);
}

#[tokio::test]
async fn socks4_domain_rejected_fail_closed() {
    let proxy_addr = spawn_socks4(false).await;
    let mgr = manager_with(entry_with_addr(ProxyType::Socks4, proxy_addr)).await;
    let err = mgr
        .create_connection_to_domain("example.com", 80)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("SOCKS4"));
}

#[tokio::test]
async fn http_connect_success() {
    let proxy_addr = spawn_http(false, "", 200, false, false).await;
    let mgr = manager_with(entry_with_addr(ProxyType::Http, proxy_addr)).await;
    let conn = mgr.create_connection("203.0.113.1:443").await.unwrap();
    assert_eq!(conn.proxy_chain.len(), 1);
}

#[tokio::test]
async fn http_connect_basic_auth() {
    use base64::{engine::general_purpose, Engine as _};
    let creds = general_purpose::STANDARD.encode("u1:p1");
    let proxy_addr = spawn_http(
        true,
        &format!("Proxy-Authorization: Basic {}", creds),
        200,
        false,
        false,
    )
    .await;
    let e =
        entry_with_addr(ProxyType::Http, proxy_addr).with_auth("u1".to_string(), "p1".to_string());
    let info = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 443, TIMEOUT)
        .await
        .unwrap();
    assert_eq!(info.hop_count, 1);
}

#[tokio::test]
async fn http_connect_failure_status() {
    for status in [403u16, 500] {
        let proxy_addr = spawn_http(false, "", status, false, false).await;
        let e = entry_with_addr(ProxyType::Http, proxy_addr);
        let err = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, TIMEOUT)
            .await
            .unwrap_err();
        assert!(
            !err.to_string().is_empty(),
            "status {} must surface an error",
            status
        );
    }
}

#[tokio::test]
async fn http_malformed_response_errors_bounded() {
    let proxy_addr = spawn_http(false, "", 200, true, false).await;
    let e = entry_with_addr(ProxyType::Http, proxy_addr);
    let err = tokio::time::timeout(
        Duration::from_secs(10),
        adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, TIMEOUT),
    )
    .await
    .expect("must resolve within bound")
    .unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn connect_timeout_is_bounded() {
    // Accept TCP but never speak proxy protocol (hold the stream open so
    // the client observes a timeout, not a reset).
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let Ok((s, _)) = listener.accept().await else {
            return;
        };
        // Hold the socket open without reading or responding so the client
        // observes the outer deadline, not a reset. `_held` keeps it alive.
        let _held = s;
        tokio::time::sleep(Duration::from_secs(30)).await;
    });
    let e = entry_with_addr(ProxyType::Socks5, addr);
    let start = std::time::Instant::now();
    let err = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, SHORT_TIMEOUT)
        .await
        .unwrap_err();
    assert!(start.elapsed() < Duration::from_secs(10), "must be bounded");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("timed out") || msg.contains("timeout"),
        "got: {msg}"
    );
}

#[tokio::test]
async fn cancellation_is_future_drop_safe() {
    let proxy_addr = spawn_socks5(false, "", "", None, None, false).await;
    let e = entry_with_addr(ProxyType::Socks5, proxy_addr);
    // Create the future and drop it immediately: must not panic or detach.
    let fut = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, TIMEOUT);
    drop(fut);
    // A fresh establishment still works after the drop.
    let info = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, TIMEOUT)
        .await
        .unwrap();
    assert_eq!(info.hop_count, 1);
}

#[tokio::test]
async fn two_hop_chain_ordering() {
    // Final echo target.
    let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = echo.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    let Ok(n) = s.read(&mut buf).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    if s.write_all(&buf[..n]).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    // Hop 1 (exit): forwards to echo.
    let seen_exit = Arc::new(tokio::sync::Mutex::new(None));
    let hop1 = spawn_socks5(false, "", "", None, Some(seen_exit.clone()), true).await;
    // Hop 0 (entry): forwards to hop1; record what it was asked to dial.
    let seen_entry = Arc::new(tokio::sync::Mutex::new(None));
    let hop0 = spawn_socks5(false, "", "", None, Some(seen_entry.clone()), true).await;

    let chain = vec![
        entry_with_addr(ProxyType::Socks5, hop0),
        entry_with_addr(ProxyType::Socks5, hop1),
    ];
    let info = adapter::establish(
        &chain,
        &echo_addr.ip().to_string(),
        echo_addr.port(),
        TIMEOUT,
    )
    .await
    .unwrap();
    assert_eq!(info.hop_count, 2);
    let entry_seen = seen_entry.lock().await;
    assert_eq!(entry_seen.as_ref().unwrap().host, hop1.ip().to_string());
    assert_eq!(entry_seen.as_ref().unwrap().port, hop1.port());
    let exit_seen = seen_exit.lock().await;
    assert_eq!(exit_seen.as_ref().unwrap().host, echo_addr.ip().to_string());
}

#[tokio::test]
async fn mixed_http_socks_chain_via_adapter() {
    // Eggress composition exercised internally; not exposed via public chain API.
    let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = echo.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let _ = tokio::time::timeout(Duration::from_secs(10), async {
                    let mut buf = [0u8; 1];
                    let _ = s.read(&mut buf).await;
                })
                .await;
            });
        }
    });
    // HTTP entry that forwards: minimal CONNECT-then-relay.
    let http_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let http_addr = http_listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = http_listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                loop {
                    let Ok(n) = s.read(&mut tmp).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                let req = String::from_utf8_lossy(&buf).into_owned();
                // Parse CONNECT host:port.
                let target = req
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("")
                    .to_string();
                match TcpStream::connect(&target).await {
                    Ok(mut up) => {
                        if s.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                            .await
                            .is_err()
                        {
                            return;
                        }
                        let _ = tokio::io::copy_bidirectional(&mut s, &mut up).await;
                    }
                    Err(_) => {
                        let _ = s.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
                    }
                }
            });
        }
    });
    let socks_exit = spawn_socks5(false, "", "", None, None, true).await;
    let chain = vec![
        entry_with_addr(ProxyType::Http, http_addr),
        entry_with_addr(ProxyType::Socks5, socks_exit),
    ];
    let info = adapter::establish(
        &chain,
        &echo_addr.ip().to_string(),
        echo_addr.port(),
        TIMEOUT,
    )
    .await
    .unwrap();
    assert_eq!(info.hop_count, 2);
}

#[tokio::test]
async fn hop_failure_diagnostics_redacted_and_stable() {
    // Hop 0 dead (closed port): stable error, no leak.
    let closed: SocketAddr = "127.0.0.1:9".parse().unwrap();
    let e = entry_with_addr(ProxyType::Socks5, closed)
        .with_auth("hopuser".to_string(), "hopsecret-abc".to_string());
    let err = adapter::establish(std::slice::from_ref(&e), "127.0.0.1", 80, SHORT_TIMEOUT)
        .await
        .unwrap_err();
    let msg = format!("{:?} {}", err, err);
    assert!(!msg.contains("hopsecret-abc"), "leak at hop0: {msg}");
    assert!(!msg.contains("hopuser") || !msg.contains("hopsecret"));

    // Later-hop failure: entry forwards to a failing exit.
    let failing_exit = spawn_socks5(false, "", "", Some(0x05), None, false).await;
    let entry = spawn_socks5(false, "", "", None, None, true).await;
    let chain = vec![
        entry_with_addr(ProxyType::Socks5, entry)
            .with_auth("cuser".to_string(), "cpass-xyz".to_string()),
        entry_with_addr(ProxyType::Socks5, failing_exit),
    ];
    let err = adapter::establish(&chain, "127.0.0.1", 80, TIMEOUT)
        .await
        .unwrap_err();
    let msg = format!("{:?} {}", err, err);
    assert!(!msg.contains("cpass-xyz"), "leak on later-hop: {msg}");
}

#[tokio::test]
async fn no_direct_fallback_on_proxy_failure() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_srv = hits.clone();
    let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_addr = target.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((_, _)) = target.accept().await else {
                return;
            };
            hits_srv.fetch_add(1, Ordering::SeqCst);
        }
    });
    // Dead proxy: connection must fail and target must see zero hits.
    let closed: SocketAddr = "127.0.0.1:9".parse().unwrap();
    let e = entry_with_addr(ProxyType::Socks5, closed);
    let err = adapter::establish(
        std::slice::from_ref(&e),
        &target_addr.ip().to_string(),
        target_addr.port(),
        SHORT_TIMEOUT,
    )
    .await
    .unwrap_err();
    assert!(!err.to_string().is_empty());
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(hits.load(Ordering::SeqCst), 0, "direct fallback detected");
}

#[tokio::test]
async fn https_preserves_plaintext_connect_behavior() {
    // Characterization: Https maps to plaintext CONNECT (tls=false) until audited.
    let e = entry_with_addr(ProxyType::Https, "127.0.0.1:8080".parse().unwrap());
    let hop = adapter::hop_from_entry(&e).unwrap();
    assert_eq!(hop.protocols, vec![eggress_uri::ProtocolSpec::Http]);
    assert!(!hop.tls);
    // And it works against a plaintext CONNECT fake.
    let proxy_addr = spawn_http(false, "", 200, false, false).await;
    let mgr = manager_with(entry_with_addr(ProxyType::Https, proxy_addr)).await;
    let conn = mgr.create_connection("203.0.113.1:443").await.unwrap();
    assert_eq!(conn.proxy_chain.len(), 1);
}

#[tokio::test]
async fn private_targets_rejected_before_eggress() {
    let proxy_addr = spawn_socks5(false, "", "", None, None, false).await;
    let mgr = manager_with(entry_with_addr(ProxyType::Socks5, proxy_addr)).await;
    for target in ["127.0.0.1:80", "10.0.0.1:80", "192.168.1.1:8080"] {
        let err = mgr.create_connection(target).await.unwrap_err();
        assert!(err.to_string().contains("private"), "got: {err}");
    }
}
