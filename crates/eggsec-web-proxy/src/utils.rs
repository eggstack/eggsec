use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::error::{Result, WebProxyError};

/// Ensure a rustls crypto provider is installed before building TLS clients.
///
/// reqwest built with `rustls-no-provider` panics if no process-level provider
/// has been installed. Installation is idempotent; the error from a second
/// install attempt is intentionally ignored.
pub fn ensure_rustls_provider() {
    static INSTALL_PROVIDER: std::sync::Once = std::sync::Once::new();
    INSTALL_PROVIDER.call_once(|| {
        if rustls::crypto::ring::default_provider()
            .install_default()
            .is_err()
        {
            tracing::debug!("rustls crypto provider already installed");
        }
    });
}

/// Create an insecure (skip certificate verification) reqwest client
/// with a custom timeout and optional builder customization.
pub fn create_insecure_client_with_options<F>(
    timeout_secs: u64,
    builder_fn: F,
) -> Result<reqwest::Client>
where
    F: FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
{
    ensure_rustls_provider();
    let builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(timeout_secs));
    let builder = builder_fn(builder);
    builder
        .build()
        .map_err(|e| WebProxyError::Network(format!("Failed to build HTTP client: {}", e)))
}

/// Connect to a target address with TCP_NODELAY and a timeout.
pub async fn connect_with_nodelay_timeout(
    addr: &SocketAddr,
    timeout_duration: Duration,
) -> Result<TcpStream> {
    let stream = timeout(timeout_duration, TcpStream::connect(addr))
        .await
        .map_err(|_| WebProxyError::Network(format!("Connection to {} timed out", addr)))?
        .map_err(|e: std::io::Error| {
            WebProxyError::Network(format!("Failed to connect to {}: {}", addr, e))
        })?;

    stream.set_nodelay(true).map_err(|e: std::io::Error| {
        WebProxyError::Network(format!("Failed to set nodelay on {}: {}", addr, e))
    })?;

    Ok(stream)
}
