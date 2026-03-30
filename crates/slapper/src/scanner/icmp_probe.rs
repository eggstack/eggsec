#![cfg(feature = "stress-testing")]

use std::net::IpAddr;
use std::time::Duration;

use crate::error::{Result, SlapperError};
use surge_ping::{IcmpPacket, PingIdentifier, PingSequence};

#[derive(Debug, Clone, serde::Serialize)]
pub struct PingResult {
    pub target: IpAddr,
    pub rtt: Duration,
    pub ttl: u8,
    pub sequence: u16,
    pub identifier: u16,
    pub payload_size: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PingStats {
    pub sent: u32,
    pub received: u32,
    pub lost: u32,
    pub min_rtt: Option<Duration>,
    pub max_rtt: Option<Duration>,
    pub avg_rtt: Option<Duration>,
}

pub async fn ping_host(
    target: &str,
    count: u32,
    _timeout: Duration,
    interval: Duration,
) -> Result<(Vec<PingResult>, PingStats)> {
    let target_ip = resolve_target(target).await?;

    let identifier = PingIdentifier(rand::random::<u16>());
    let mut results = Vec::new();

    for i in 0..count {
        let payload = [0u8; 56];
        let sequence = PingSequence(i as u16 + 1);

        match surge_ping::ping(target_ip, &payload).await {
            Ok((packet, rtt)) => {
                let ttl = match &packet {
                    IcmpPacket::V4(p) => p.get_ttl().unwrap_or(64),
                    IcmpPacket::V6(_) => 64,
                };

                results.push(PingResult {
                    target: target_ip,
                    rtt,
                    ttl,
                    sequence: sequence.0,
                    identifier: identifier.0,
                    payload_size: payload.len(),
                });
            }
            Err(e) => {
                tracing::debug!("Ping timeout for sequence {}: {}", sequence.0, e);
            }
        }

        if i < count - 1 && interval > Duration::ZERO {
            tokio::time::sleep(interval).await;
        }
    }

    let stats = calculate_stats(&results, count);

    Ok((results, stats))
}

fn calculate_stats(results: &[PingResult], sent: u32) -> PingStats {
    let received = results.len() as u32;
    let lost = sent - received;

    let rtts: Vec<Duration> = results.iter().map(|r| r.rtt).collect();

    let min_rtt = rtts.iter().min().copied();
    let max_rtt = rtts.iter().max().copied();
    let avg_rtt = if rtts.is_empty() {
        None
    } else {
        let total: Duration = rtts.iter().sum();
        Some(total / rtts.len() as u32)
    };

    PingStats {
        sent,
        received,
        lost,
        min_rtt,
        max_rtt,
        avg_rtt,
    }
}

async fn resolve_target(target: &str) -> Result<IpAddr> {
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(ip);
    }

    resolve_hostname(target).await
}

async fn resolve_hostname(hostname: &str) -> Result<IpAddr> {
    use std::net::ToSocketAddrs;

    let addrs: Vec<_> = (hostname, 0)
        .to_socket_addrs()
        .map_err(|e| SlapperError::Network(format!("DNS lookup failed: {}", e)))?
        .collect();

    addrs
        .first()
        .map(|s| s.ip())
        .ok_or_else(|| SlapperError::Network("No addresses found for hostname".to_string()))
}
