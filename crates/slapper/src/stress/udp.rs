#[cfg(feature = "stress-testing")]
use crate::error::{Result, SlapperError};
#[cfg(feature = "stress-testing")]
use rand::Rng;
#[cfg(feature = "stress-testing")]
use std::net::{IpAddr, SocketAddr};
#[cfg(feature = "stress-testing")]
use std::sync::Arc;
#[cfg(feature = "stress-testing")]
use std::time::{Duration, Instant};
#[cfg(feature = "stress-testing")]
use tokio::net::UdpSocket;

#[cfg(feature = "stress-testing")]
use super::metrics::StressMetrics;
#[cfg(feature = "stress-testing")]
use super::{StressConfig, StressStats};

#[cfg(feature = "stress-testing")]
pub async fn run_udp_flood(config: &StressConfig, metrics: &StressMetrics) -> Result<StressStats> {
    let target_ip = resolve_target(&config.target).await?;
    let target_addr = SocketAddr::new(target_ip, config.port);

    let payload = generate_payload(config.payload_size);

    metrics.start();

    let start_time = Instant::now();
    let duration = Duration::from_secs(config.duration_secs);
    let interval = Duration::from_micros(1_000_000 / config.rate_pps.max(1));

    let semaphore = Arc::new(tokio::sync::Semaphore::new(config.concurrency));
    let metrics = Arc::new(metrics.clone());
    let payload = Arc::new(payload);
    let target_addr = Arc::new(target_addr);
    let random_port = config.random_source_port;

    let mut handles = Vec::new();

    while start_time.elapsed() < duration {
        let permit = semaphore.clone().acquire_owned().await?;
        let target = *target_addr;
        let payload = payload.clone();
        let metrics = metrics.clone();

        let port = if random_port {
            Some(rand::random::<u16>())
        } else {
            None
        };

        let handle = tokio::spawn(async move {
            let socket = match create_udp_socket(port).await {
                Ok(s) => s,
                Err(_) => {
                    metrics.record_error();
                    drop(permit);
                    return;
                }
            };

            match socket.send_to(&payload, target).await {
                Ok(_) => {
                    metrics.record_packet(payload.len() as u64);
                }
                Err(_) => {
                    metrics.record_error();
                }
            }

            drop(permit);
        });

        handles.push(handle);

        if interval > Duration::ZERO {
            tokio::time::sleep(interval).await;
        }
    }

    futures::future::join_all(handles).await;

    Ok(metrics.to_stats())
}

#[cfg(feature = "stress-testing")]
async fn resolve_target(target: &str) -> Result<IpAddr> {
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(ip);
    }

    use std::net::ToSocketAddrs;

    let addrs: Vec<_> = (target, 0).to_socket_addrs()?.collect();

    addrs
        .first()
        .map(|a| a.ip())
        .ok_or_else(|| SlapperError::Runtime(format!("Failed to resolve target: {}", target)))
}

#[cfg(feature = "stress-testing")]
async fn create_udp_socket(port: Option<u16>) -> Result<UdpSocket> {
    let socket = if let Some(port) = port {
        UdpSocket::bind(format!("0.0.0.0:{}", port)).await?
    } else {
        UdpSocket::bind("0.0.0.0:0").await?
    };

    socket.set_broadcast(true)?;

    Ok(socket)
}

#[cfg(feature = "stress-testing")]
fn generate_payload(size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut payload = vec![0u8; size];
    rng.fill(&mut payload[..]);
    payload
}

#[cfg(not(feature = "stress-testing"))]
pub async fn run_udp_flood(
    _config: &super::StressConfig,
    _metrics: &super::metrics::StressMetrics,
) -> crate::error::Result<super::StressStats> {
    Err(SlapperError::Runtime(
        "UDP flood requires 'stress-testing' feature enabled".to_string(),
    ))
}
