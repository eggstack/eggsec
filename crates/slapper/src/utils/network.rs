use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;

pub async fn connect_with_nodelay(addr: &SocketAddr) -> std::io::Result<TcpStream> {
    let stream = TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    Ok(stream)
}

pub async fn connect_with_nodelay_timeout(
    addr: &SocketAddr,
    timeout: Duration,
) -> std::io::Result<TcpStream> {
    let stream = tokio::time::timeout(timeout, TcpStream::connect(addr)).await??;
    stream.set_nodelay(true)?;
    Ok(stream)
}
