
use crate::error::Result;
use std::time::{Duration, Instant};

use super::config::{HealthCheckConfig, ProxyEntry};
use crate::utils::create_insecure_http_client;

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub proxy_url: String,
    pub is_healthy: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub checked_at: Instant,
}

#[derive(Debug, Clone)]
pub struct ProxyHealth {
    pub total: usize,
    pub healthy: usize,
    pub unhealthy: usize,
    pub results: Vec<HealthCheckResult>,
}

impl ProxyHealth {
    pub fn healthy_percentage(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.healthy as f64 / self.total as f64) * 100.0
    }
}

pub struct HealthChecker {
    config: HealthCheckConfig,
    client: reqwest::Client,
}

impl HealthChecker {
    pub fn new(config: HealthCheckConfig) -> Result<Self> {
        let timeout_secs = (config.timeout_ms / 1000).max(1);
        let client = create_insecure_http_client(timeout_secs)?;

        Ok(Self { config, client })
    }

    pub async fn check(&self, proxy: &ProxyEntry) -> HealthCheckResult {
        let proxy_url = proxy.to_url();
        let start = Instant::now();

        let result = self.check_proxy(proxy).await;
        let latency = start.elapsed();

        match result {
            Ok(true) => HealthCheckResult {
                proxy_url: proxy_url.clone(),
                is_healthy: true,
                latency_ms: Some(latency.as_millis() as u64),
                error: None,
                checked_at: Instant::now(),
            },
            Ok(false) => HealthCheckResult {
                proxy_url: proxy_url.clone(),
                is_healthy: false,
                latency_ms: Some(latency.as_millis() as u64),
                error: Some("Proxy returned unsuccessful response".to_string()),
                checked_at: Instant::now(),
            },
            Err(e) => HealthCheckResult {
                proxy_url: proxy_url.clone(),
                is_healthy: false,
                latency_ms: None,
                error: Some(e.to_string()),
                checked_at: Instant::now(),
            },
        }
    }

    async fn check_proxy(&self, proxy: &ProxyEntry) -> Result<bool> {
        let proxy_url = format!(
            "{}://{}:{}",
            match proxy.proxy_type {
                super::config::ProxyType::Socks4 | super::config::ProxyType::Socks5 => "socks5",
                super::config::ProxyType::Http | super::config::ProxyType::Https => "http",
                super::config::ProxyType::Tor => "socks5",
            },
            proxy.address,
            proxy.port
        );

        let reqwest_proxy = if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
            reqwest::Proxy::all(&proxy_url)?.basic_auth(user, pass.expose_secret())
        } else {
            reqwest::Proxy::all(&proxy_url)?
        };

        let client = reqwest::Client::builder()
            .proxy(reqwest_proxy)
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .danger_accept_invalid_certs(true)
            .build()?;

        let response = client.get(&self.config.test_url).send().await?;

        Ok(response.status().is_success())
    }

    pub async fn check_all(&self, proxies: &[ProxyEntry]) -> Result<ProxyHealth> {
        let mut results = Vec::with_capacity(proxies.len());

        for proxy in proxies {
            if proxy.enabled {
                let result = self.check(proxy).await;
                results.push(result);
            }
        }

        let healthy = results.iter().filter(|r| r.is_healthy).count();

        Ok(ProxyHealth {
            total: proxies.len(),
            healthy,
            unhealthy: proxies.len() - healthy,
            results,
        })
    }

    pub async fn check_concurrent(
        &self,
        proxies: &[ProxyEntry],
        concurrency: usize,
    ) -> Result<ProxyHealth> {
        use futures::future::join_all;
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::new();

        for proxy in proxies {
            if !proxy.enabled {
                continue;
            }

            let permit = semaphore.clone().acquire_owned().await?;
            let checker = self.clone();
            let proxy = proxy.clone();

            let handle = tokio::spawn(async move {
                let result = checker.check(&proxy).await;
                drop(permit);
                result
            });

            handles.push(handle);
        }

        let results = join_all(handles)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        let healthy = results.iter().filter(|r| r.is_healthy).count();

        Ok(ProxyHealth {
            total: proxies.len(),
            healthy,
            unhealthy: proxies.len() - healthy,
            results,
        })
    }
}

impl Clone for HealthChecker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: self.client.clone(),
        }
    }
}
