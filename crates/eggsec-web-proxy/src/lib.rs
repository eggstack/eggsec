//! Web Proxy and MITM Interception Domain Crate (defense-lab surface).
//!
//! Standalone defense-lab surface for HTTP/HTTPS traffic interception, proxy pool management,
//! and MITM security testing in authorized lab environments. Gated behind feature flags.
//!
//! This crate owns domain execution logic, types, and tests, but does NOT decide
//! whether an operation is allowed. Enforcement stays in the main `eggsec` crate.

pub mod config;
pub mod error;
pub mod health;
pub mod http_connect;
pub mod intercept;
#[cfg(feature = "web-proxy-mcp")]
pub mod mcp;
pub mod pool;
pub mod rotator;
pub mod socks;
pub mod utils;

pub use config::{HealthCheckConfig, ProxyConfig, ProxyEntry, ProxyType};
pub use error::{Result, WebProxyError};
pub use health::{HealthChecker, ProxyHealth};
pub use pool::ProxyPool;
pub use rotator::ProxyRotator;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use socks::connect_through_with_domain;

/// Connection routed through a proxy chain.
#[derive(Debug, Clone)]
pub struct ProxiedConnection {
    pub proxy_chain: Vec<ProxyEntry>,
    pub local_addr: SocketAddr,
    pub target_addr: SocketAddr,
}

/// Central proxy orchestrator: pool + rotator + health checker.
pub struct ProxyManager {
    pool: Arc<RwLock<ProxyPool>>,
    rotator: ProxyRotator,
    health_checker: HealthChecker,
}

impl ProxyManager {
    pub fn new(config: ProxyConfig) -> Result<Self> {
        let pool = ProxyPool::new(config.clone());
        let rotator = ProxyRotator::new(config.rotation_strategy);
        let health_checker = HealthChecker::new(HealthCheckConfig::from(&config))?;

        Ok(Self {
            pool: Arc::new(RwLock::new(pool)),
            rotator,
            health_checker,
        })
    }

    pub async fn add_proxy(&self, proxy: ProxyEntry) -> Result<()> {
        let pool = self.pool.write().await;
        pool.add(proxy);
        Ok(())
    }

    pub async fn add_proxies_from_file(&self, path: &str) -> Result<usize> {
        let proxies = ProxyEntry::load_from_file(path)?;
        let count = proxies.len();

        let pool = self.pool.write().await;
        for proxy in proxies {
            pool.add(proxy);
        }

        Ok(count)
    }

    pub async fn get_next_proxy(&self) -> Option<ProxyEntry> {
        let pool = self.pool.read().await;
        let proxies = pool.get_all();
        let pool_ref = &pool;

        self.rotator
            .select_with_stats(&proxies, &|key| pool_ref.get_stats(key))
    }

    pub async fn get_healthy_proxy(&self) -> Option<ProxyEntry> {
        let pool = self.pool.read().await;
        let healthy = pool.get_healthy();
        let pool_ref = &pool;

        self.rotator
            .select_with_stats(&healthy, &|key| pool_ref.get_stats(key))
    }

    pub async fn get_all_healthy_proxies(&self) -> Vec<ProxyEntry> {
        let pool = self.pool.read().await;
        pool.get_healthy()
    }

    pub async fn get_highest_priority_proxy(&self, min_priority: u8) -> Option<ProxyEntry> {
        let pool = self.pool.read().await;
        let by_priority = pool.get_by_priority(min_priority);

        if by_priority.is_empty() {
            drop(pool);
            return self.get_healthy_proxy().await;
        }

        let pool_ref = &pool;
        self.rotator
            .select_with_stats(&by_priority, &|key| pool_ref.get_stats(key))
    }

    pub async fn check_health(&self) -> Result<ProxyHealth> {
        let pool = self.pool.read().await;
        self.health_checker.check_all(&pool.get_all()).await
    }

    pub async fn create_connection(&self, target: &str) -> Result<ProxiedConnection> {
        let proxy = self
            .get_healthy_proxy()
            .await
            .ok_or_else(|| WebProxyError::Proxy("No healthy proxies available".to_string()))?;

        let target_addr = resolve_target(target).await?;

        match proxy.proxy_type {
            ProxyType::Socks4 | ProxyType::Socks5 => {
                socks::connect_through(proxy, target_addr).await
            }
            ProxyType::Http | ProxyType::Https => {
                http_connect::connect_through(proxy, target_addr).await
            }
            ProxyType::Tor => socks::connect_through_tor(proxy, target_addr).await,
        }
    }

    pub async fn create_connection_to_domain(
        &self,
        domain: &str,
        port: u16,
    ) -> Result<ProxiedConnection> {
        let proxy = self
            .get_healthy_proxy()
            .await
            .ok_or_else(|| WebProxyError::Proxy("No healthy proxies available".to_string()))?;

        connect_through_with_domain(&proxy, domain, port)
            .await
            .map(|stream| {
                let local_addr = stream.local_addr().unwrap_or_else(|_| {
                    tracing::warn!(
                        "Failed to get local address for proxied connection to {}",
                        domain
                    );
                    std::net::SocketAddr::new(
                        std::net::IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
                        0,
                    )
                });
                ProxiedConnection {
                    proxy_chain: vec![proxy],
                    local_addr,
                    target_addr: std::net::SocketAddr::new(
                        std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
                        port,
                    ),
                }
            })
    }

    pub async fn create_chained_connection(
        &self,
        target: &str,
        chain_length: usize,
    ) -> Result<ProxiedConnection> {
        let pool = self.pool.read().await;
        let proxies = pool.get_healthy();

        if proxies.len() < chain_length {
            return Err(WebProxyError::Proxy(format!(
                "Not enough healthy proxies for chaining (have {}, need {})",
                proxies.len(),
                chain_length
            )));
        }

        let chain = self
            .rotator
            .select_chain(&proxies, chain_length)
            .ok_or_else(|| WebProxyError::Proxy("Failed to select proxy chain".to_string()))?;

        drop(pool);

        let target_addr = resolve_target(target).await?;

        let chain_vec: Vec<ProxyEntry> = chain.to_vec();

        let socks_only = chain_vec
            .iter()
            .all(|p| matches!(p.proxy_type, ProxyType::Socks5 | ProxyType::Tor));

        let final_local_addr = if chain_vec.len() > 1 {
            if !socks_only {
                return Err(WebProxyError::Proxy(
                    "Proxy chaining currently supports only SOCKS5/Tor proxy chains".to_string(),
                ));
            }

            let stream = socks::chain_connect(&chain_vec, target_addr).await?;
            stream.local_addr()?
        } else {
            let proxy = &chain_vec[0];
            let conn = match proxy.proxy_type {
                ProxyType::Socks4 | ProxyType::Socks5 => {
                    socks::connect_through(proxy.clone(), target_addr).await?
                }
                ProxyType::Http | ProxyType::Https => {
                    http_connect::connect_through(proxy.clone(), target_addr).await?
                }
                ProxyType::Tor => socks::connect_through_tor(proxy.clone(), target_addr).await?,
            };

            conn.local_addr
        };

        let proxy_chain: Vec<ProxyEntry> = chain.into_iter().collect();

        Ok(ProxiedConnection {
            proxy_chain,
            local_addr: final_local_addr,
            target_addr,
        })
    }

    pub async fn pool_size(&self) -> usize {
        self.pool.read().await.size()
    }

    pub async fn start_background_health_check(&self, interval_secs: u64) -> JoinHandle<()> {
        let pool = Arc::clone(&self.pool);
        let health_checker = self.health_checker.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                let proxies = {
                    let pool = pool.read().await;
                    pool.get_all()
                };

                if proxies.is_empty() {
                    continue;
                }

                tracing::debug!(
                    "Running background health check for {} proxies",
                    proxies.len()
                );

                match health_checker.check_concurrent(&proxies, 10).await {
                    Ok(results) => {
                        let pool = pool.write().await;
                        for result in results.results {
                            let key = result.proxy_url;
                            if result.is_healthy {
                                pool.mark_healthy(&key);
                            } else {
                                pool.mark_unhealthy(&key);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Background health check failed: {}", e);
                    }
                }
            }
        })
    }
}

/// Resolve a target string to a SocketAddr, blocking private IPs.
async fn resolve_target(target: &str) -> Result<SocketAddr> {
    if let Ok(addr) = target.parse::<SocketAddr>() {
        if is_private_ip(addr.ip()) {
            return Err(WebProxyError::Proxy(format!(
                "Target address is a private/internal IP: {}",
                addr
            )));
        }
        return Ok(addr);
    }

    if target.contains(':') {
        let parts: Vec<&str> = target.splitn(2, ':').collect();
        let host = parts[0];
        let port: u16 = parts[1].parse().map_err(|e| {
            WebProxyError::Proxy(format!("Invalid port in target '{}': {}", target, e))
        })?;

        let target_addr = format!("{}:{}", host, port);
        let mut addrs = tokio::net::lookup_host(&target_addr)
            .await
            .map_err(|e| WebProxyError::Proxy(format!("DNS lookup failed: {}", e)))?;

        if let Some(addr) = addrs.next() {
            if is_private_ip(addr.ip()) {
                return Err(WebProxyError::Proxy(format!(
                    "Resolved address is a private/internal IP: {}",
                    addr
                )));
            }
            Ok(addr)
        } else {
            Err(WebProxyError::Proxy(format!(
                "Failed to resolve {}",
                target
            )))
        }
    } else {
        Err(WebProxyError::Proxy(format!(
            "Target must include port: {}",
            target
        )))
    }
}

/// Check if an IP address is private/internal (RFC 1918, loopback, multicast, broadcast).
fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 10
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
                || octets[0] == 127
                || (octets[0] >= 224 && octets[0] <= 239)
                || octets.iter().all(|&o| o == 255)
        }
        std::net::IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            (segments[0] & 0xffc0) == 0xfe80
                || ((segments[0] & 0xfe00) == 0xfc00)
                || ipv6.is_loopback()
                || (segments[0] & 0xff00) == 0xff00
                || ipv6.is_unspecified()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_proxy_type_parsing() {
        assert!(matches!(
            ProxyType::from_str("socks5"),
            Ok(ProxyType::Socks5)
        ));
        assert!(matches!(
            ProxyType::from_str("socks4"),
            Ok(ProxyType::Socks4)
        ));
        assert!(matches!(ProxyType::from_str("http"), Ok(ProxyType::Http)));
        assert!(matches!(ProxyType::from_str("tor"), Ok(ProxyType::Tor)));
    }
}
