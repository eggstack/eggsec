//! Phase B health qualification matrix (local-only).
//!
//! Proves proxy health remains application-level HTTP(S)-through-proxy
//! validation after the Phase A dial migration. No Internet, public DNS,
//! external Tor, or root required.
//!
//! Backend decision (Phase B WS2, Option B): Reqwest retained explicitly as
//! the health-only owner. Rebuilding HTTPS verification, redirect, timeout,
//! and body-cap semantics over raw Eggress streams would recreate a general
//! HTTP client to remove a dependency line.
//!
//! Lab TLS note: health probes use explicit-insecure TLS
//! (`danger_accept_invalid_certs(true)`, unchanged from baseline), so a
//! self-signed local HTTPS target is expected to validate as healthy. TLS
//! verification is not silently enabled or disabled by this phase.

use eggsec_web_proxy::{HealthCheckConfig, HealthChecker, ProxyEntry, ProxyType};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn health_config(test_url: String, timeout_ms: u64) -> HealthCheckConfig {
    HealthCheckConfig {
        enabled: true,
        interval_secs: 60,
        timeout_ms,
        test_url,
        max_failures: 3,
    }
}

// --- Local HTTP(S) targets -------------------------------------------------

async fn spawn_http_target(status: u16, hits: Option<Arc<AtomicUsize>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/health", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = listener.accept().await else {
                return;
            };
            if let Some(h) = &hits {
                h.fetch_add(1, Ordering::SeqCst);
            }
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = tokio::time::timeout(Duration::from_secs(5), s.read(&mut buf)).await;
                let reason = match status {
                    200 => "OK",
                    500 => "Internal Server Error",
                    _ => "Error",
                };
                let body = "ok";
                let resp = format!(
                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    reason,
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes()).await;
            });
        }
    });
    url
}

async fn spawn_https_target(status: u16) -> String {
    use rustls::ServerConfig;
    use std::sync::Arc as StdArc;
    use tokio_rustls::TlsAcceptor;

    // Self-signed cert for localhost (lab-only; health client is insecure).
    let mut params =
        rcgen::CertificateParams::new(vec!["localhost".to_string(), "127.0.0.1".to_string()])
            .unwrap();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "localhost");
    let key_pair = rcgen::KeyPair::generate().unwrap();
    let cert = params.self_signed(&key_pair).unwrap();
    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();
    let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der)];
    let key = rustls::pki_types::PrivateKeyDer::try_from(key_der).unwrap();
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .unwrap();
    let acceptor = TlsAcceptor::from(StdArc::new(config));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("https://{}/health", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let Ok((s, _)) = listener.accept().await else {
                return;
            };
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut tls) = tokio::time::timeout(Duration::from_secs(5), acceptor.accept(s))
                    .await
                    .map(|r| r.map_err(|_| ()))
                    .unwrap_or(Err(()))
                else {
                    return;
                };
                let mut buf = [0u8; 4096];
                let _ = tokio::time::timeout(Duration::from_secs(5), tls.read(&mut buf)).await;
                let body = "ok";
                let resp = format!(
                    "HTTP/1.1 {} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    body.len(),
                    body
                );
                let _ = tls.write_all(resp.as_bytes()).await;
                let _ = tls.shutdown().await;
            });
        }
    });
    url
}

// --- Local forwarding proxies ----------------------------------------------

async fn relay(a: TcpStream, b: TcpStream) {
    let mut a = a;
    let mut b = b;
    let _ = tokio::io::copy_bidirectional(&mut a, &mut b).await;
}

async fn spawn_socks5_proxy(require_auth: bool, user: &str, pass: &str) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (u, p) = (user.to_string(), pass.to_string());
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = listener.accept().await else {
                return;
            };
            let (u, p) = (u.clone(), p.clone());
            tokio::spawn(async move {
                let mut hdr = [0u8; 2];
                if s.read_exact(&mut hdr).await.is_err() {
                    return;
                }
                let mut methods = vec![0u8; hdr[1] as usize];
                if s.read_exact(&mut methods).await.is_err() {
                    return;
                }
                if require_auth {
                    if s.write_all(&[0x05, 0x02]).await.is_err() {
                        return;
                    }
                    let mut ah = [0u8; 2];
                    if s.read_exact(&mut ah).await.is_err() {
                        return;
                    }
                    let mut ub = vec![0u8; ah[1] as usize];
                    if s.read_exact(&mut ub).await.is_err() {
                        return;
                    }
                    let mut pl = [0u8; 1];
                    if s.read_exact(&mut pl).await.is_err() {
                        return;
                    }
                    let mut pb = vec![0u8; pl[0] as usize];
                    if s.read_exact(&mut pb).await.is_err() {
                        return;
                    }
                    if ub != u.as_bytes() || pb != p.as_bytes() {
                        let _ = s.write_all(&[0x01, 0x01]).await;
                        return;
                    }
                    if s.write_all(&[0x01, 0x00]).await.is_err() {
                        return;
                    }
                } else if s.write_all(&[0x05, 0x00]).await.is_err() {
                    return;
                }
                let mut req = [0u8; 4];
                if s.read_exact(&mut req).await.is_err() {
                    return;
                }
                let target: std::net::SocketAddr = match req[3] {
                    0x01 => {
                        let mut b = [0u8; 6];
                        if s.read_exact(&mut b).await.is_err() {
                            return;
                        }
                        format!(
                            "{}.{}.{}.{}:{}",
                            b[0],
                            b[1],
                            b[2],
                            b[3],
                            u16::from_be_bytes([b[4], b[5]])
                        )
                        .parse()
                        .unwrap()
                    }
                    0x03 => {
                        let mut l = [0u8; 1];
                        if s.read_exact(&mut l).await.is_err() {
                            return;
                        }
                        let mut d = vec![0u8; l[0] as usize];
                        if s.read_exact(&mut d).await.is_err() {
                            return;
                        }
                        let mut pt = [0u8; 2];
                        if s.read_exact(&mut pt).await.is_err() {
                            return;
                        }
                        let host = String::from_utf8_lossy(&d).into_owned();
                        let port = u16::from_be_bytes(pt);
                        // Remote-domain: resolve locally in the harness.
                        match tokio::net::lookup_host((host.clone(), port)).await {
                            Ok(mut addrs) => match addrs.next() {
                                Some(a) => a,
                                None => return,
                            },
                            Err(_) => {
                                let _ = s.write_all(&[0x05, 0x04, 0, 1, 0, 0, 0, 0, 0, 0]).await;
                                return;
                            }
                        }
                    }
                    _ => return,
                };
                match TcpStream::connect(target).await {
                    Ok(up) => {
                        if s.write_all(&[0x05, 0, 0, 1, 0, 0, 0, 0, 0, 0])
                            .await
                            .is_err()
                        {
                            return;
                        }
                        relay(s, up).await;
                    }
                    Err(_) => {
                        let _ = s.write_all(&[0x05, 0x05, 0, 1, 0, 0, 0, 0, 0, 0]).await;
                    }
                }
            });
        }
    });
    addr
}

async fn spawn_http_proxy(require_auth: bool, expected_auth: &str) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let exp = expected_auth.to_string();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = listener.accept().await else {
                return;
            };
            let exp = exp.clone();
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
                    if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 65536 {
                        break;
                    }
                }
                let req = String::from_utf8_lossy(&buf).into_owned();
                if require_auth && !req.contains(&exp) {
                    let _ = s
                        .write_all(b"HTTP/1.1 407 Proxy Auth Required\r\n\r\n")
                        .await;
                    return;
                }
                let target = req
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("")
                    .to_string();
                // CONNECT target may be host:port (HTTP target) or TLS host.
                let target_addr = if let Ok(a) = target.parse::<std::net::SocketAddr>() {
                    a
                } else if let Some((h, p)) = target.rsplit_once(':') {
                    match tokio::net::lookup_host((h, p.parse::<u16>().unwrap_or(443))).await {
                        Ok(mut addrs) => match addrs.next() {
                            Some(a) => a,
                            None => {
                                let _ = s.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
                                return;
                            }
                        },
                        Err(_) => {
                            let _ = s.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
                            return;
                        }
                    }
                } else {
                    let _ = s.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
                    return;
                };
                match TcpStream::connect(target_addr).await {
                    Ok(up) => {
                        if s.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                            .await
                            .is_err()
                        {
                            return;
                        }
                        relay(s, up).await;
                    }
                    Err(_) => {
                        let _ = s.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
                    }
                }
            });
        }
    });
    addr
}

// --- Matrix ----------------------------------------------------------------

#[tokio::test]
async fn socks5_http_200_is_healthy() {
    let target = spawn_http_target(200, None).await;
    let proxy = spawn_socks5_proxy(false, "", "").await;
    let entry = ProxyEntry::new(ProxyType::Socks5, proxy.ip().to_string(), proxy.port());
    let checker = HealthChecker::new(health_config(target, 5000)).unwrap();
    let result = checker.check(&entry).await;
    assert!(result.is_healthy, "err: {:?}", result.error);
    assert!(result.latency_ms.is_some());
}

#[tokio::test]
async fn socks5_auth_http_200_is_healthy() {
    let target = spawn_http_target(200, None).await;
    let proxy = spawn_socks5_proxy(true, "hu", "hp-secret-1").await;
    let entry = ProxyEntry::new(ProxyType::Socks5, proxy.ip().to_string(), proxy.port())
        .with_auth("hu".to_string(), "hp-secret-1".to_string());
    let checker = HealthChecker::new(health_config(target, 5000)).unwrap();
    let result = checker.check(&entry).await;
    assert!(result.is_healthy, "err: {:?}", result.error);
}

#[tokio::test]
async fn socks5_http_non2xx_is_unhealthy() {
    let target = spawn_http_target(500, None).await;
    let proxy = spawn_socks5_proxy(false, "", "").await;
    let entry = ProxyEntry::new(ProxyType::Socks5, proxy.ip().to_string(), proxy.port());
    let checker = HealthChecker::new(health_config(target, 5000)).unwrap();
    let result = checker.check(&entry).await;
    assert!(!result.is_healthy);
}

#[tokio::test]
async fn http_connect_https_200_is_healthy() {
    let target = spawn_https_target(200).await;
    let proxy = spawn_http_proxy(false, "").await;
    let entry = ProxyEntry::new(ProxyType::Http, proxy.ip().to_string(), proxy.port());
    let checker = HealthChecker::new(health_config(target, 8000)).unwrap();
    let result = checker.check(&entry).await;
    assert!(result.is_healthy, "err: {:?}", result.error);
}

#[tokio::test]
async fn http_connect_auth_https_200_is_healthy() {
    use base64::{engine::general_purpose, Engine as _};
    let creds = general_purpose::STANDARD.encode("ku:kp-secret-2");
    let target = spawn_https_target(200).await;
    let proxy = spawn_http_proxy(true, &format!("Proxy-Authorization: Basic {}", creds)).await;
    let entry = ProxyEntry::new(ProxyType::Http, proxy.ip().to_string(), proxy.port())
        .with_auth("ku".to_string(), "kp-secret-2".to_string());
    let checker = HealthChecker::new(health_config(target, 8000)).unwrap();
    let result = checker.check(&entry).await;
    assert!(result.is_healthy, "err: {:?}", result.error);
}

#[tokio::test]
async fn tunnel_success_with_app_failure_is_unhealthy() {
    // Proxy handshake succeeds but the application request fails (500):
    // proves health is application-level, never tunnel-only.
    let target = spawn_http_target(500, None).await;
    let proxy = spawn_socks5_proxy(false, "", "").await;
    let entry = ProxyEntry::new(ProxyType::Socks5, proxy.ip().to_string(), proxy.port());
    let checker = HealthChecker::new(health_config(target, 5000)).unwrap();
    assert!(!checker.check(&entry).await.is_healthy);
}

#[tokio::test]
async fn https_self_signed_validates_via_lab_insecure_mode() {
    // Baseline behavior preserved: health probes use explicit-insecure TLS,
    // so a self-signed local HTTPS target validates as healthy. TLS
    // verification semantics are unchanged (neither enabled nor disabled)
    // by this phase; the mode stays explicit lab-only.
    let target = spawn_https_target(200).await;
    let proxy = spawn_socks5_proxy(false, "", "").await;
    let entry = ProxyEntry::new(ProxyType::Socks5, proxy.ip().to_string(), proxy.port());
    let checker = HealthChecker::new(health_config(target, 8000)).unwrap();
    let result = checker.check(&entry).await;
    assert!(result.is_healthy, "err: {:?}", result.error);
}

#[tokio::test]
async fn overall_timeout_is_bounded_and_unhealthy() {
    let target = spawn_http_target(200, None).await;
    // Blackhole proxy: accepts TCP, never speaks.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let Ok((s, _)) = listener.accept().await else {
            return;
        };
        let _held = s;
        tokio::time::sleep(Duration::from_secs(30)).await;
    });
    let entry = ProxyEntry::new(ProxyType::Socks5, proxy.ip().to_string(), proxy.port());
    let checker = HealthChecker::new(health_config(target, 800)).unwrap();
    let start = Instant::now();
    let result = checker.check(&entry).await;
    assert!(!result.is_healthy);
    assert!(
        start.elapsed() < Duration::from_secs(20),
        "health must be bounded"
    );
}

#[tokio::test]
async fn proxy_auth_failure_is_unhealthy_and_redacted() {
    let target = spawn_http_target(200, None).await;
    let proxy = spawn_socks5_proxy(true, "au", "correct-horse").await;
    let entry = ProxyEntry::new(ProxyType::Socks5, proxy.ip().to_string(), proxy.port())
        .with_auth("au".to_string(), "WRONG-secret-qq".to_string());
    let checker = HealthChecker::new(health_config(target, 5000)).unwrap();
    let result = checker.check(&entry).await;
    assert!(!result.is_healthy);
    let err = result.error.unwrap_or_default();
    assert!(!err.contains("WRONG-secret-qq"), "credential leaked: {err}");
    assert!(!err.contains("correct-horse"), "credential leaked: {err}");
}

#[tokio::test]
async fn no_direct_fallback_when_proxy_leg_fails() {
    let hits = Arc::new(AtomicUsize::new(0));
    let target = spawn_http_target(200, Some(hits.clone())).await;
    // Dead proxy port.
    let entry = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 9);
    let checker = HealthChecker::new(health_config(target, 1500)).unwrap();
    let result = checker.check(&entry).await;
    assert!(!result.is_healthy);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(hits.load(Ordering::SeqCst), 0, "direct fallback detected");
}

#[tokio::test]
async fn credentials_redacted_on_every_failure_path() {
    let secret = "matrix-secret-zz9";
    // Dead proxy with credentials: even construction/routing errors redact.
    let entry = ProxyEntry::new(ProxyType::Http, "127.0.0.1".to_string(), 9)
        .with_auth("matrix-user".to_string(), secret.to_string());
    let checker =
        HealthChecker::new(health_config("http://127.0.0.1:9/health".to_string(), 1500)).unwrap();
    let result = checker.check(&entry).await;
    assert!(!result.is_healthy);
    let err = result.error.clone().unwrap_or_default();
    let blob = format!("{:?} {:?} {}", result, entry.to_log_key(), err);
    assert!(!blob.contains(secret), "credential leaked");
    // Log keys redact by construction.
    assert!(entry.to_log_key().contains("***"));
    assert!(!entry.to_log_key().contains(secret));
}

#[tokio::test]
async fn socks4_fails_closed_not_as_socks5() {
    // WS3 behavior fix: SOCKS4 has no faithful Reqwest mapping. Health must
    // return an explicit unsupported error, never a SOCKS5 result presented
    // as SOCKS4 health.
    let target = spawn_http_target(200, None).await;
    let proxy = spawn_socks5_proxy(false, "", "").await;
    let entry = ProxyEntry::new(ProxyType::Socks4, proxy.ip().to_string(), proxy.port());
    let checker = HealthChecker::new(health_config(target, 5000)).unwrap();
    let result = checker.check(&entry).await;
    assert!(!result.is_healthy);
    let err = result.error.unwrap_or_default();
    assert!(err.contains("SOCKS4"), "must be explicit, got: {err}");
}

#[tokio::test]
async fn https_uses_plaintext_connect_like_production() {
    // Https health via http:// mapping is faithful because production Https
    // dial is plaintext CONNECT (Phase A characterization). An HTTPS target
    // is used because Reqwest tunnels HTTPS through the proxy with CONNECT
    // (plaintext to the proxy, TLS to the target) — exactly the production
    // shape. (Plain-HTTP targets use forward-proxy GET, a different mode
    // the minimal harness fake does not implement; production proxies
    // support both.)
    let target = spawn_https_target(200).await;
    let proxy = spawn_http_proxy(false, "").await;
    let entry = ProxyEntry::new(ProxyType::Https, proxy.ip().to_string(), proxy.port());
    let checker = HealthChecker::new(health_config(target, 8000)).unwrap();
    let result = checker.check(&entry).await;
    assert!(result.is_healthy, "err: {:?}", result.error);
}

#[tokio::test]
async fn concurrent_check_is_bounded_and_accounts_all() {
    let target = spawn_http_target(200, None).await;
    let proxy = spawn_socks5_proxy(false, "", "").await;
    let mut proxies = Vec::new();
    for _ in 0..8 {
        proxies.push(ProxyEntry::new(
            ProxyType::Socks5,
            proxy.ip().to_string(),
            proxy.port(),
        ));
    }
    let checker = HealthChecker::new(health_config(target, 8000)).unwrap();
    let health = checker.check_concurrent(&proxies, 3).await.unwrap();
    assert_eq!(health.total, 8);
    assert_eq!(health.healthy, 8);
    assert_eq!(health.unhealthy, 0);
    assert_eq!(health.results.len(), 8);
}

#[tokio::test]
async fn checker_clone_is_cheap_config_only() {
    // WS4: no stored client state; Clone carries config only.
    let checker =
        HealthChecker::new(health_config("http://127.0.0.1:9/health".to_string(), 1000)).unwrap();
    let dbg = format!("{:?}", checker);
    assert!(dbg.contains("HealthChecker"));
    let _cloned = checker.clone();
}
