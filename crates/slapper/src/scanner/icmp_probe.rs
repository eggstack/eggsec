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
    let mut results = Vec::with_capacity(count as usize);

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
    let addrs: Vec<_> = tokio::net::lookup_host((hostname, 0))
        .await
        .map_err(|e| SlapperError::Network(format!("DNS lookup failed: {}", e)))?
        .collect();

    addrs
        .first()
        .map(|s| s.ip())
        .ok_or_else(|| SlapperError::Network("No addresses found for hostname".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_stats_empty_results() {
        let results: Vec<PingResult> = vec![];
        let stats = calculate_stats(&results, 3);

        assert_eq!(stats.sent, 3);
        assert_eq!(stats.received, 0);
        assert_eq!(stats.lost, 3);
        assert!(stats.min_rtt.is_none());
        assert!(stats.max_rtt.is_none());
        assert!(stats.avg_rtt.is_none());
    }

    #[test]
    fn test_calculate_stats_single_result() {
        let results = vec![PingResult {
            target: IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            rtt: Duration::from_millis(10),
            ttl: 64,
            sequence: 1,
            identifier: 1234,
            payload_size: 56,
        }];
        let stats = calculate_stats(&results, 1);

        assert_eq!(stats.sent, 1);
        assert_eq!(stats.received, 1);
        assert_eq!(stats.lost, 0);
        assert_eq!(stats.min_rtt, Some(Duration::from_millis(10)));
        assert_eq!(stats.max_rtt, Some(Duration::from_millis(10)));
        assert_eq!(stats.avg_rtt, Some(Duration::from_millis(10)));
    }

    #[test]
    fn test_calculate_stats_multiple_results() {
        let results = vec![
            PingResult {
                target: IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                rtt: Duration::from_millis(10),
                ttl: 64,
                sequence: 1,
                identifier: 1234,
                payload_size: 56,
            },
            PingResult {
                target: IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                rtt: Duration::from_millis(20),
                ttl: 64,
                sequence: 2,
                identifier: 1234,
                payload_size: 56,
            },
            PingResult {
                target: IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                rtt: Duration::from_millis(30),
                ttl: 64,
                sequence: 3,
                identifier: 1234,
                payload_size: 56,
            },
        ];
        let stats = calculate_stats(&results, 5);

        assert_eq!(stats.sent, 5);
        assert_eq!(stats.received, 3);
        assert_eq!(stats.lost, 2);
        assert_eq!(stats.min_rtt, Some(Duration::from_millis(10)));
        assert_eq!(stats.max_rtt, Some(Duration::from_millis(30)));
        assert_eq!(stats.avg_rtt, Some(Duration::from_millis(20)));
    }

    #[test]
    fn test_calculate_stats_partial_responses() {
        let results = vec![PingResult {
            target: IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            rtt: Duration::from_millis(50),
            ttl: 128,
            sequence: 1,
            identifier: 4321,
            payload_size: 64,
        }];
        let stats = calculate_stats(&results, 10);

        assert_eq!(stats.sent, 10);
        assert_eq!(stats.received, 1);
        assert_eq!(stats.lost, 9);
    }

    #[test]
    fn test_ping_result_fields() {
        let result = PingResult {
            target: IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
            rtt: Duration::from_millis(5),
            ttl: 255,
            sequence: 42,
            identifier: 9999,
            payload_size: 128,
        };
        assert_eq!(result.ttl, 255);
        assert_eq!(result.sequence, 42);
        assert_eq!(result.identifier, 9999);
        assert_eq!(result.payload_size, 128);
    }

    #[test]
    fn test_ping_stats_clone() {
        let stats = PingStats {
            sent: 5,
            received: 3,
            lost: 2,
            min_rtt: Some(Duration::from_millis(10)),
            max_rtt: Some(Duration::from_millis(50)),
            avg_rtt: Some(Duration::from_millis(30)),
        };
        let cloned = stats.clone();
        assert_eq!(cloned.sent, stats.sent);
        assert_eq!(cloned.received, stats.received);
    }

    #[test]
    fn test_ping_result_serialize() {
        let result = PingResult {
            target: IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            rtt: Duration::from_millis(10),
            ttl: 64,
            sequence: 1,
            identifier: 1234,
            payload_size: 56,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("127.0.0.1"));
        assert!(json.contains("ttl"));
    }

    #[test]
    fn test_resolve_target_ip() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(resolve_target("127.0.0.1"));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
        );
    }

    #[test]
    fn test_resolve_target_ipv6() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(resolve_target("::1"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_target_invalid_ip() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(resolve_target("999.999.999.999"));
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_target_localhost_hostname() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(resolve_target("localhost"));
        assert!(result.is_ok());
    }
}
