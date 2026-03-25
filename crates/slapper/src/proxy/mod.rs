
mod config;
mod health;
mod http_connect;
mod pool;
mod rotator;
mod socks;

pub use config::{HealthCheckConfig, ProxyConfig, ProxyEntry, ProxyType};
pub use health::{HealthChecker, ProxyHealth};
pub use pool::ProxyPool;
pub use rotator::ProxyRotator;

use socks::connect_through_with_domain;

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct ProxiedConnection {
    pub proxy_chain: Vec<ProxyEntry>,
    pub local_addr: SocketAddr,
    pub target_addr: SocketAddr,
}

pub struct ProxyManager {
    pool: Arc<RwLock<ProxyPool>>,
    rotator: ProxyRotator,
    health_checker: HealthChecker,
}

impl ProxyManager {
    pub fn new(config: ProxyConfig) -> Self {
        let pool = ProxyPool::new(config.clone());
        let rotator = ProxyRotator::new(config.rotation_strategy);
        let health_checker = HealthChecker::new(HealthCheckConfig::from(&config))
            .expect("Failed to create health checker HTTP client");

        Self {
            pool: Arc::new(RwLock::new(pool)),
            rotator,
            health_checker,
        }
    }

    pub async fn add_proxy(&self, proxy: ProxyEntry) -> Result<()> {
        let mut pool = self.pool.write().await;
        pool.add(proxy);
        Ok(())
    }

    pub async fn add_proxies_from_file(&self, path: &str) -> Result<usize> {
        let proxies = ProxyEntry::load_from_file(path)?;
        let count = proxies.len();

        let mut pool = self.pool.write().await;
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
            .ok_or_else(|| anyhow::anyhow!("No healthy proxies available"))?;

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
            .ok_or_else(|| anyhow::anyhow!("No healthy proxies available"))?;

        connect_through_with_domain(&proxy, domain, port)
            .await
            .map(|stream| ProxiedConnection {
                proxy_chain: vec![proxy],
                local_addr: stream
                    .local_addr()
                    .unwrap_or_else(|_| std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0)),
                target_addr: format!("{}:{}", domain, port)
                    .parse()
                    .unwrap_or_else(|_| std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0)),
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
            anyhow::bail!(
                "Not enough healthy proxies for chaining (have {}, need {})",
                proxies.len(),
                chain_length
            );
        }

        let chain = self
            .rotator
            .select_chain(&proxies, chain_length)
            .ok_or_else(|| anyhow::anyhow!("Failed to select proxy chain"))?;

        drop(pool);

        let target_addr = resolve_target(target).await?;

        let mut final_local_addr = "0.0.0.0:0".parse::<SocketAddr>().unwrap();

        for proxy in chain.iter() {
            let conn = match proxy.proxy_type {
                ProxyType::Socks4 | ProxyType::Socks5 => {
                    socks::connect_through(proxy.clone(), target_addr).await?
                }
                ProxyType::Http | ProxyType::Https => {
                    http_connect::connect_through(proxy.clone(), target_addr).await?
                }
                ProxyType::Tor => socks::connect_through_tor(proxy.clone(), target_addr).await?,
            };
            final_local_addr = conn.local_addr;
        }

        Ok(ProxiedConnection {
            proxy_chain: chain,
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

                match health_checker.check_all(&proxies).await {
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

async fn resolve_target(target: &str) -> Result<SocketAddr> {
    if let Ok(addr) = target.parse::<SocketAddr>() {
        return Ok(addr);
    }

    if target.contains(':') {
        let parts: Vec<&str> = target.splitn(2, ':').collect();
        let host = parts[0];
        let port: u16 = parts[1].parse()?;

        use std::net::ToSocketAddrs;

        let addrs: Vec<_> = (host, port).to_socket_addrs()?.collect();

        addrs
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve {}", target))
    } else {
        anyhow::bail!("Target must include port: {}", target);
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
