//! Local no-Internet fixtures for the `eggfetch` adapter parity suite.
//!
//! Hand-rolled HTTP/1.1 servers (plain + TLS) over loopback plus stub
//! authorities/resolvers. Nothing here leaves the host: hostnames resolve
//! through [`eggsec_transport::InMemoryResolver`], and every socket binds
//! `127.0.0.1`.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use eggsec_transport::{NetworkAuthority, PolicyCheckpoint, TransportError, TransportResolver};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;

/// One captured inbound request (values kept for presence/absence
/// assertions; tests must not log secrets on failure paths).
#[derive(Debug, Clone)]
pub struct Incoming {
    /// HTTP method (`GET`, `POST`, ...).
    pub method: String,
    /// Request target (path + query).
    pub path: String,
    /// Lowercased header names with raw values.
    pub headers: Vec<(String, String)>,
    /// Request body bytes.
    pub body: Vec<u8>,
}

impl Incoming {
    /// First value for `name` (case-insensitive), if present.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// `true` when the header is present (value never inspected).
    pub fn has(&self, name: &str) -> bool {
        self.header(name).is_some()
    }
}

/// Scripted response for one route.
#[derive(Debug, Clone)]
pub struct Action {
    /// HTTP status code.
    pub status: u16,
    /// Extra response headers.
    pub headers: Vec<(String, String)>,
    /// Response body.
    pub body: Vec<u8>,
    /// Optional artificial delay before responding (timeout fixtures).
    pub delay: Duration,
}

impl Action {
    /// `200` with a UTF-8 body.
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: body.into(),
            delay: Duration::ZERO,
        }
    }

    /// Redirect with a `Location` header.
    pub fn redirect(status: u16, location: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![("location".to_string(), location.into())],
            body: Vec::new(),
            delay: Duration::ZERO,
        }
    }

    /// `200` after `delay` (timeout fixtures).
    pub fn slow(body: &'static str, delay: Duration) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
            delay,
        }
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        404 => "Not Found",
        _ => "Response",
    }
}

/// Route table: path prefix → action factory.
pub type Handler = Arc<dyn Fn(&Incoming) -> Action + Send + Sync>;

async fn read_request<S>(stream: &mut S) -> std::io::Result<Option<Incoming>>
where
    S: AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(None);
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > 65536 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request head too large",
            ));
        }
        if let Some(end) = find_header_end(&buf) {
            let head = String::from_utf8_lossy(&buf[..end]).to_string();
            let mut lines = head.lines();
            let request_line = lines.next().unwrap_or("");
            let mut parts = request_line.split_whitespace();
            let method = parts.next().unwrap_or("").to_string();
            let path = parts.next().unwrap_or("/").to_string();
            let mut headers = Vec::new();
            let mut content_length = 0usize;
            for line in lines {
                if let Some((name, value)) = line.split_once(':') {
                    let name = name.trim().to_lowercase();
                    let value = value.trim().to_string();
                    if name == "content-length" {
                        content_length = value.parse().unwrap_or(0).min(8 * 1024 * 1024);
                    }
                    headers.push((name, value));
                }
            }
            let body_start = end;
            let mut body = buf[body_start..].to_vec();
            while body.len() < content_length {
                let n = stream.read(&mut chunk).await?;
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&chunk[..n]);
                if body.len() > 8 * 1024 * 1024 {
                    break;
                }
            }
            body.truncate(content_length);
            return Ok(Some(Incoming {
                method,
                path,
                headers,
                body,
            }));
        }
    }
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|pos| pos + 4)
}

async fn serve_conn<S>(
    mut stream: S,
    handler: Handler,
    received: Arc<Mutex<Vec<Incoming>>>,
) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // One request per connection is enough for the suite (fixtures answer
    // `connection: close`, and the backend opens fresh connections).
    let Some(incoming) = read_request(&mut stream).await? else {
        return Ok(());
    };
    received
        .lock()
        .expect("fixture log lock")
        .push(incoming.clone());
    let action = handler(&incoming);
    if !action.delay.is_zero() {
        tokio::time::sleep(action.delay).await;
    }
    let mut head = format!(
        "HTTP/1.1 {} {}\r\ncontent-length: {}\r\nconnection: close\r\n",
        action.status,
        reason(action.status),
        action.body.len()
    );
    for (name, value) in &action.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(&action.body).await?;
    stream.flush().await?;
    Ok(())
}

/// Plain-HTTP loopback fixture.
pub struct Fixture {
    /// Bound loopback address.
    pub addr: SocketAddr,
    received: Arc<Mutex<Vec<Incoming>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Fixture {
    /// Serve `handler` on `127.0.0.1:0`.
    pub async fn start(handler: Handler) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback binds");
        let addr = listener.local_addr().expect("local addr");
        let received = Arc::new(Mutex::new(Vec::new()));
        let task = {
            let handler = handler.clone();
            let received = received.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        return;
                    };
                    let handler = handler.clone();
                    let received = received.clone();
                    tokio::spawn(async move {
                        let _ = serve_conn(stream, handler, received).await;
                    });
                }
            })
        };
        Self {
            addr,
            received,
            task,
        }
    }

    /// Serve static `routes` (exact path match, `404` fallback).
    pub async fn routes(routes: HashMap<String, Action>) -> Self {
        let routes = Arc::new(routes);
        Self::start(Arc::new(move |incoming: &Incoming| {
            routes.get(&incoming.path).cloned().unwrap_or(Action {
                status: 404,
                headers: Vec::new(),
                body: b"no route".to_vec(),
                delay: Duration::ZERO,
            })
        }))
        .await
    }

    /// Echo fixture: every request answers `200` with its own body.
    pub async fn echo() -> Self {
        Self::start(Arc::new(|incoming: &Incoming| {
            Action::ok(incoming.body.clone())
        }))
        .await
    }

    /// Base URL for this fixture (`http://127.0.0.1:PORT`).
    pub fn base(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Base URL under a logical hostname (resolves via the test resolver).
    pub fn base_host(&self, host: &str) -> String {
        format!("http://{host}:{}", self.addr.port())
    }

    /// Requests received so far, in order.
    pub fn received(&self) -> Vec<Incoming> {
        self.received.lock().expect("lock").clone()
    }
}

/// TLS loopback fixture with a runtime-generated self-signed certificate.
///
/// `sans` controls the certificate subjectAltNames (e.g. `["127.0.0.1"]`
/// for the happy path, `["wrong.invalid"]` for mismatch fixtures).
pub struct TlsFixture {
    /// Bound loopback address.
    pub addr: SocketAddr,
    received: Arc<Mutex<Vec<Incoming>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TlsFixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TlsFixture {
    /// Start a TLS fixture serving `handler` with a self-signed cert.
    pub async fn start(handler: Handler, sans: Vec<String>) -> Self {
        let params = rcgen::CertificateParams::new(sans).expect("cert params");
        let key_pair = rcgen::KeyPair::generate().expect("key pair");
        let cert = params.self_signed(&key_pair).expect("self-signed");
        let cert_der = cert.der().to_vec();
        let key_der = key_pair.serialize_der();
        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![rustls::pki_types::CertificateDer::from(cert_der)],
                rustls::pki_types::PrivateKeyDer::try_from(key_der).expect("private key"),
            )
            .expect("server config");
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback binds");
        let addr = listener.local_addr().expect("local addr");
        let received = Arc::new(Mutex::new(Vec::new()));
        let task = {
            let handler = handler.clone();
            let received = received.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        return;
                    };
                    let acceptor = acceptor.clone();
                    let handler = handler.clone();
                    let received = received.clone();
                    tokio::spawn(async move {
                        let Ok(tls) = acceptor.accept(stream).await else {
                            return;
                        };
                        let _ = serve_conn(tls, handler, received).await;
                    });
                }
            })
        };
        Self {
            addr,
            received,
            task,
        }
    }

    /// `https://127.0.0.1:PORT` base URL.
    pub fn base(&self) -> String {
        format!("https://{}", self.addr)
    }

    /// Requests received so far, in order.
    pub fn received(&self) -> Vec<Incoming> {
        self.received.lock().expect("lock").clone()
    }
}

/// Permissive test authority: approves every candidate, records nothing.
///
/// Structural rules (userinfo rejection) already hold at request
/// construction; this authority exists to isolate adapter mechanics from
/// policy verdicts (policy interop is proven through `ScopeAuthority` in
/// the engine-level parity tests).
#[derive(Debug, Default)]
pub struct AllowAll;

impl NetworkAuthority for AllowAll {
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

    fn check_tls_consistency(
        &self,
        _request_host: &str,
        _sni_override: Option<&str>,
        _host_override: Option<&str>,
    ) -> Result<(), TransportError> {
        Ok(())
    }
}

/// Authority wrapper that records every checkpoint call in order.
pub struct RecordingAuth<A: NetworkAuthority> {
    inner: A,
    calls: Mutex<Vec<String>>,
}

impl<A: NetworkAuthority> RecordingAuth<A> {
    /// Wrap `inner`, recording checkpoint names.
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn record(&self, name: &str) {
        self.calls.lock().expect("lock").push(name.to_string());
    }

    /// Recorded checkpoint names in call order.
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("lock").clone()
    }
}

impl<A: NetworkAuthority> NetworkAuthority for RecordingAuth<A> {
    fn authorize_initial_url(&self, url: &url::Url) -> Result<(), TransportError> {
        self.record("initial_url");
        self.inner.authorize_initial_url(url)
    }

    fn authorize_host(
        &self,
        host: &str,
        port: Option<u16>,
        is_ip_literal: bool,
    ) -> Result<(), TransportError> {
        self.record("host");
        self.inner.authorize_host(host, port, is_ip_literal)
    }

    fn authorize_resolved(
        &self,
        host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        self.record("resolved");
        self.inner.authorize_resolved(host, candidates)
    }

    fn authorize_socket(&self, host: &str, addr: IpAddr, port: u16) -> Result<(), TransportError> {
        self.record("socket");
        self.inner.authorize_socket(host, addr, port)
    }

    fn authorize_redirect(&self, from: &url::Url, to: &url::Url) -> Result<(), TransportError> {
        self.record("redirect");
        self.inner.authorize_redirect(from, to)
    }

    fn authorize_reresolution(
        &self,
        host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        self.record("reresolution");
        self.inner.authorize_reresolution(host, candidates)
    }

    fn authorize_proxy(
        &self,
        proxy_endpoint: &url::Url,
        ultimate: &url::Url,
    ) -> Result<(), TransportError> {
        self.record("proxy");
        self.inner.authorize_proxy(proxy_endpoint, ultimate)
    }

    fn check_tls_consistency(
        &self,
        request_host: &str,
        sni_override: Option<&str>,
        host_override: Option<&str>,
    ) -> Result<(), TransportError> {
        self.record("tls_consistency");
        self.inner
            .check_tls_consistency(request_host, sni_override, host_override)
    }
}

/// Resolver that answers `first` on the first call and `rest` afterwards
/// (DNS-answer-change fixtures: allowed → denied between connections).
pub struct FlipFlopResolver {
    first: Vec<IpAddr>,
    rest: Vec<IpAddr>,
    calls: AtomicUsize,
}

impl FlipFlopResolver {
    /// New resolver with distinct first/subsequent answers.
    pub fn new(first: Vec<&str>, rest: Vec<&str>) -> Self {
        let parse = |list: Vec<&str>| {
            list.into_iter()
                .map(|s| s.parse().expect("test IP parses"))
                .collect()
        };
        Self {
            first: parse(first),
            rest: parse(rest),
            calls: AtomicUsize::new(0),
        }
    }
}

impl TransportResolver for FlipFlopResolver {
    fn resolve(&self, host: &str) -> eggsec_transport::ResolvedCandidates {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        let addrs = if n == 0 {
            self.first.clone()
        } else {
            self.rest.clone()
        };
        eggsec_transport::ResolvedCandidates::new(host, addrs)
    }
}

/// Resolver that panics when called (proves IP literals skip resolution).
#[derive(Debug, Default)]
pub struct PanicResolver;

impl TransportResolver for PanicResolver {
    fn resolve(&self, host: &str) -> eggsec_transport::ResolvedCandidates {
        panic!("resolver must not be consulted for '{host}' (IP literal path)")
    }
}

/// Deny-DNS authority: fails closed at the DNS checkpoint.
#[derive(Debug, Default)]
pub struct DenyDns;

impl NetworkAuthority for DenyDns {
    fn authorize_initial_url(&self, _url: &url::Url) -> Result<(), TransportError> {
        Ok(())
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
        host: &str,
        _candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        Err(TransportError::denied(
            PolicyCheckpoint::Dns,
            format!("test denial for '{host}'"),
        ))
    }

    fn authorize_socket(
        &self,
        _host: &str,
        _addr: IpAddr,
        _port: u16,
    ) -> Result<(), TransportError> {
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

    fn check_tls_consistency(
        &self,
        _request_host: &str,
        _sni_override: Option<&str>,
        _host_override: Option<&str>,
    ) -> Result<(), TransportError> {
        Ok(())
    }
}

/// CIDR-allow authority: only addresses inside `cidr` survive (mixed-answer
/// and DNS-change fixtures without the engine crate).
#[derive(Debug)]
pub struct CidrAllow {
    network: ipnetwork::IpNetwork,
}

impl CidrAllow {
    /// New authority allowing `cidr` (e.g. `"127.0.0.0/8"`).
    pub fn new(cidr: &str) -> Self {
        Self {
            network: cidr.parse().expect("test CIDR parses"),
        }
    }
}

impl NetworkAuthority for CidrAllow {
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
        host: &str,
        candidates: &[IpAddr],
    ) -> Result<Vec<IpAddr>, TransportError> {
        if candidates.iter().all(|ip| self.network.contains(*ip)) {
            Ok(candidates.to_vec())
        } else {
            Err(TransportError::denied(
                PolicyCheckpoint::Dns,
                format!("not all addresses for '{host}' are in {0}", self.network),
            ))
        }
    }

    fn authorize_socket(&self, host: &str, addr: IpAddr, _port: u16) -> Result<(), TransportError> {
        if self.network.contains(addr) {
            Ok(())
        } else {
            Err(TransportError::denied(
                PolicyCheckpoint::Socket,
                format!("socket address {addr} for '{host}' is out of scope"),
            ))
        }
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

    fn check_tls_consistency(
        &self,
        _request_host: &str,
        _sni_override: Option<&str>,
        _host_override: Option<&str>,
    ) -> Result<(), TransportError> {
        Ok(())
    }
}
