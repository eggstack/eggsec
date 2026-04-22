
use std::pin::Pin;
#[cfg(feature = "insecure-tls")]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::utils::connect_with_nodelay;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::server::ServerConfig;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream as ServerTlsStream;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::TlsConnector;

pub enum StreamWrapper {
    Plain(InnerStream),
    TlsClient(TlsStreamClient),
    TlsServer(TlsStreamServer),
}

pub type InnerStream = TcpStream;
pub type TlsStreamClient = tokio_rustls::client::TlsStream<TcpStream>;
pub type TlsStreamServer = ServerTlsStream<TcpStream>;

impl StreamWrapper {
    pub async fn accept_tls(
        acceptor: &TlsAcceptor,
        stream: TcpStream,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let tls_stream = acceptor.accept(stream).await?;
        Ok(StreamWrapper::TlsServer(tls_stream))
    }

    pub async fn connect_tls(
        connector: &TlsConnector,
        domain: &str,
        stream: TcpStream,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let domain: ServerName<'static> = domain.to_owned().try_into()?;
        let tls_stream = connector.connect(domain, stream).await?;
        Ok(StreamWrapper::TlsClient(tls_stream))
    }

    pub fn plain(stream: TcpStream) -> Self {
        StreamWrapper::Plain(stream)
    }

    pub fn is_tls(&self) -> bool {
        matches!(self, StreamWrapper::TlsClient(_) | StreamWrapper::TlsServer(_))
    }
}

impl AsyncRead for StreamWrapper {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            StreamWrapper::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            StreamWrapper::TlsClient(stream) => Pin::new(stream).poll_read(cx, buf),
            StreamWrapper::TlsServer(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for StreamWrapper {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match &mut *self {
            StreamWrapper::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            StreamWrapper::TlsClient(stream) => Pin::new(stream).poll_write(cx, buf),
            StreamWrapper::TlsServer(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            StreamWrapper::Plain(stream) => Pin::new(stream).poll_flush(cx),
            StreamWrapper::TlsClient(stream) => Pin::new(stream).poll_flush(cx),
            StreamWrapper::TlsServer(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            StreamWrapper::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            StreamWrapper::TlsClient(stream) => Pin::new(stream).poll_shutdown(cx),
            StreamWrapper::TlsServer(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

pub struct TlsServer {
    acceptor: TlsAcceptor,
}

impl TlsServer {
    pub fn from_pem<P: AsRef<std::path::Path>>(
        cert_path: P,
        key_path: P,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cert_data = std::fs::read(&cert_path)?;
        let pem_file = pem::parse_many(&cert_data)?;
        let certs: Vec<CertificateDer<'_>> = pem_file
            .iter()
            .filter(|p| p.tag() == "CERTIFICATE")
            .map(|p| CertificateDer::from(p.contents().to_vec()))
            .collect();

        if certs.is_empty() {
            return Err("No certificates found in PEM file".into());
        }

        let key_data = std::fs::read(&key_path)?;
        let pem_file = pem::parse_many(&key_data)?;
        let key_pem = pem_file
            .iter()
            .find(|p| p.tag() == "PRIVATE KEY" || p.tag() == "RSA PRIVATE KEY")
            .ok_or("No private key found in PEM file")?;

        use rustls::pki_types::{PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer};
        let key_der = if key_pem.tag() == "RSA PRIVATE KEY" {
            PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(key_pem.contents().to_vec()))
        } else {
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pem.contents().to_vec()))
        };
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key_der)
            .map_err(|e| format!("Failed to build server config: {}", e))?;

        Ok(Self {
            acceptor: TlsAcceptor::from(Arc::new(config)),
        })
    }

    pub fn acceptor(&self) -> &TlsAcceptor {
        &self.acceptor
    }

    pub fn clone_acceptor(&self) -> TlsAcceptor {
        self.acceptor.clone()
    }
}

pub struct TlsClient {
    connector: TlsConnector,
    domain: String,
    #[cfg(feature = "insecure-tls")]
    warn_on_use: bool,
    #[cfg(feature = "insecure-tls")]
    insecure_connection_count: Arc<AtomicUsize>,
}

impl TlsClient {
    #[cfg(feature = "insecure-tls")]
    pub fn new(domain: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();

        Ok(Self {
            connector: TlsConnector::from(Arc::new(config)),
            domain: domain.to_string(),
            warn_on_use: true,
            insecure_connection_count: Arc::new(AtomicUsize::new(0)),
        })
    }

    #[cfg(not(feature = "insecure-tls"))]
    pub fn new(_domain: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Err("TLS client requires the 'insecure-tls' feature flag. This feature disables certificate verification and should only be used for testing.".into())
    }

    pub fn connector(&self) -> &TlsConnector {
        &self.connector
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn clone_connector(&self) -> TlsConnector {
        self.connector.clone()
    }

    #[cfg(feature = "insecure-tls")]
    pub fn should_warn_and_consume(&mut self) -> bool {
        if self.warn_on_use {
            self.warn_on_use = false;
            true
        } else {
            false
        }
    }

    #[cfg(feature = "insecure-tls")]
    pub fn insecure_connection_count(&self) -> usize {
        self.insecure_connection_count.load(Ordering::Relaxed)
    }

    #[cfg(feature = "insecure-tls")]
    pub fn increment_insecure_connection(&self) {
        self.insecure_connection_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// WARNING: This verifier accepts ALL certificates without validation.
///
/// This is EXTREMELY INSECURE and should only be used in isolated testing
/// environments where certificate verification is not required. Using this
/// verifier in production exposes connections to man-in-the-middle attacks,
/// as any certificate—including self-signed, expired, or fraudulent certificates—
/// will be accepted as valid.
///
/// # Security Implications
///
/// - No verification that the server certificate is trusted
/// - No verification that the certificate matches the requested hostname
/// - No verification of certificate expiration or validity period
/// - Susceptible to DNS spoofing attacks where an attacker presents any certificate
///
/// # When to Use
///
/// Only use in:
/// - Local testing with self-signed certificates
/// - Isolated lab environments with no external network access
/// - Development environments where TLS is required but cert verification is not
///
/// # Alternatives
///
/// For production use, configure proper certificate verification or use
/// a custom `ServerCertVerifier` that performs appropriate validation based
/// on your security requirements.
#[derive(Debug)]
#[cfg(feature = "insecure-tls")]
struct NoVerifier;

#[cfg(feature = "insecure-tls")]
impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub struct LineWriter {
    stream: StreamWrapper,
}

impl LineWriter {
    pub fn new(stream: StreamWrapper) -> Self {
        Self { stream }
    }

    pub async fn write_line(&mut self, line: &str) -> std::io::Result<usize> {
        let mut data = line.as_bytes();
        let mut written = 0;

        while !data.is_empty() {
            let n = self.stream.write(data).await?;
            written += n;
            data = &data[n..];
        }

        self.stream.write_all(b"\n").await?;
        self.stream.flush().await?;

        Ok(written)
    }

    pub async fn read_line(&mut self) -> std::io::Result<Option<String>> {
        let mut reader = TokioBufReader::new(&mut self.stream);
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(line)),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncBufReadExt;
    use tokio::net::TcpListener;

    #[test]
    fn test_stream_wrapper_enum_variants() {
        let _ = StreamWrapper::Plain;
        let _ = StreamWrapper::TlsClient;
        let _ = StreamWrapper::TlsServer;
    }

    #[tokio::test]
    async fn test_line_writer_plain() -> std::io::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut writer = LineWriter::new(StreamWrapper::plain(stream));
            writer.write_line("hello").await.unwrap();
        });

        let client = connect_with_nodelay(&addr).await?;
        let mut reader = TokioBufReader::new(client);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        assert_eq!(line.trim(), "hello");
        server_handle.await.unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn test_line_writer_roundtrip() -> std::io::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut writer = LineWriter::new(StreamWrapper::plain(stream));

            while let Ok(Some(line)) = writer.read_line().await {
                writer.write_line(&format!("echo: {}", line)).await.unwrap();
            }
        });

        let client = connect_with_nodelay(&addr).await?;
        let mut writer = LineWriter::new(StreamWrapper::plain(client));

        writer.write_line("test").await?;

        let response = writer.read_line().await?.unwrap();
        assert_eq!(response.trim(), "echo: test");

        drop(writer);
        server_handle.await.unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn test_tcp_plaintext_e2e() -> std::io::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut writer = LineWriter::new(StreamWrapper::plain(stream));
            writer.write_line("welcome").await.unwrap();

            if let Ok(Some(msg)) = writer.read_line().await {
                writer.write_line(&format!("got: {}", msg)).await.unwrap();
            }
        });

        let stream = TcpStream::connect(addr).await?;
        let mut writer = LineWriter::new(StreamWrapper::plain(stream));

        let welcome = writer.read_line().await?.unwrap();
        assert_eq!(welcome.trim(), "welcome");

        writer.write_line("hello server").await?;

        let response = writer.read_line().await?.unwrap();
        assert_eq!(response.trim(), "got: hello server");

        server_handle.await.unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tls_tests {
    use super::*;

    #[tokio::test]
    async fn test_tls_server_from_pem_invalid() {
        let server = TlsServer::from_pem("nonexistent.pem", "nonexistent.key");
        assert!(server.is_err());
    }

    #[test]
    fn test_stream_wrapper_enum_variants() {
        let _ = StreamWrapper::Plain;
        let _ = StreamWrapper::TlsClient;
        let _ = StreamWrapper::TlsServer;
    }
}
