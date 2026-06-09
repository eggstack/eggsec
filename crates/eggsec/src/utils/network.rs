//! TCP connection utilities with Nagle's algorithm disabled.
//!
//! These functions create TCP connections with `TCP_NODELAY` enabled,
//! which disables Nagle's algorithm for lower latency. This is important
//! for security scanning tools where small packets need to be sent
//! immediately without waiting for additional data to fill the buffer.

use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;

/// Connect to a TCP address with Nagle's algorithm disabled.
///
/// This is useful for security scanning where you need low-latency
/// connections and want to send small packets immediately.
///
/// # Arguments
///
/// * `addr` - The socket address to connect to
///
/// # Returns
///
/// A connected `TcpStream` with `TCP_NODELAY` enabled.
pub async fn connect_with_nodelay(addr: &SocketAddr) -> std::io::Result<TcpStream> {
    let stream = TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    Ok(stream)
}

/// Connect to a TCP address with a timeout and Nagle's algorithm disabled.
///
/// This combines a connection timeout with `TCP_NODELAY` for scanning
/// operations that need both low latency and bounded wait times.
///
/// # Arguments
///
/// * `addr` - The socket address to connect to
/// * `timeout` - Maximum time to wait for the connection
///
/// # Returns
///
/// A connected `TcpStream` with `TCP_NODELAY` enabled, or an error if
/// the connection times out or fails.
pub async fn connect_with_nodelay_timeout(
    addr: &SocketAddr,
    timeout: Duration,
) -> std::io::Result<TcpStream> {
    let stream = tokio::time::timeout(timeout, TcpStream::connect(addr)).await??;
    stream.set_nodelay(true)?;
    Ok(stream)
}
