#![allow(dead_code)]

use std::path::Path;
use std::pin::Pin;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio_native_tls::{native_tls, TlsAcceptor, TlsConnector};

pub enum StreamWrapper {
    Plain(InnerStream),
    Tls(TlsStreamWrapper),
}

pub type InnerStream = TcpStream;
pub type TlsStreamWrapper = tokio_native_tls::TlsStream<TcpStream>;

impl StreamWrapper {
    pub async fn accept_tls(
        acceptor: &TlsAcceptor,
        stream: TcpStream,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let tls_stream = acceptor.accept(stream).await?;
        Ok(StreamWrapper::Tls(tls_stream))
    }

    pub async fn connect_tls(
        connector: &TlsConnector,
        domain: &str,
        stream: TcpStream,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let tls_stream = connector.connect(domain, stream).await?;
        Ok(StreamWrapper::Tls(tls_stream))
    }

    pub fn plain(stream: TcpStream) -> Self {
        StreamWrapper::Plain(stream)
    }

    pub fn is_tls(&self) -> bool {
        matches!(self, StreamWrapper::Tls(_))
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
            StreamWrapper::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
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
            StreamWrapper::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            StreamWrapper::Plain(stream) => Pin::new(stream).poll_flush(cx),
            StreamWrapper::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            StreamWrapper::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            StreamWrapper::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

pub struct TlsServer {
    acceptor: TlsAcceptor,
}

impl TlsServer {
    pub fn from_pkcs12<P: AsRef<Path>>(path: P, password: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let identity = native_tls::Identity::from_pkcs12(
            &std::fs::read(path)?,
            password,
        )?;
        let acceptor = native_tls::TlsAcceptor::new(identity)?;
        let acceptor = TlsAcceptor::from(acceptor);
        Ok(Self { acceptor })
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
}

impl TlsClient {
    pub fn new(domain: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let connector = native_tls::TlsConnector::new()?;
        let connector = TlsConnector::from(connector);
        Ok(Self {
            connector,
            domain: domain.to_string(),
        })
    }

    pub fn from_native(connector: native_tls::TlsConnector, domain: &str) -> Self {
        Self {
            connector: TlsConnector::from(connector),
            domain: domain.to_string(),
        }
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
        let mut reader = BufReader::new(&mut self.stream);
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
    use tokio::net::TcpListener;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    #[test]
    fn test_stream_wrapper_enum_variants() {
        let _ = StreamWrapper::Plain;
        let _ = StreamWrapper::Tls;
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

        let client = TcpStream::connect(addr).await?;
        let mut reader = BufReader::new(client);
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

        let client = TcpStream::connect(addr).await?;
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
    async fn test_tls_acceptor_creation() {
        let server = TlsServer::from_pkcs12("test", "password");
        assert!(server.is_err());
    }
    
    #[test]
    fn test_stream_wrapper_enum_variants() {
        let _ = StreamWrapper::Plain;
        let _ = StreamWrapper::Tls;
    }
}
