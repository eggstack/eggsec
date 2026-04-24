use crate::error::Result;
use crate::utils::create_insecure_client_with_options;
use std::time::Instant;

use super::config::{HealthCheckConfig, ProxyEntry};

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
        let client = create_insecure_client_with_options(timeout_secs, |builder| builder)?;

        Ok(Self { config, client })
    }

    pub async fn check(&self, proxy: &ProxyEntry) -> HealthCheckResult {
        let proxy_url = proxy.to_log_key();
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

        let timeout_secs = (self.config.timeout_ms as u64) / 1000;
        let client = create_insecure_client_with_options(timeout_secs, |builder| {
            builder.proxy(reqwest_proxy)
        })?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::config::{ProxyEntry, ProxyType};

    fn make_proxy(addr: &str, port: u16) -> ProxyEntry {
        ProxyEntry::new(ProxyType::Socks5, addr.to_string(), port)
    }

    #[test]
    fn test_health_check_result_fields() {
        let result = HealthCheckResult {
            proxy_url: "socks5://1.1.1.1:1080".to_string(),
            is_healthy: true,
            latency_ms: Some(42),
            error: None,
            checked_at: Instant::now(),
        };
        assert!(result.is_healthy);
        assert_eq!(result.latency_ms, Some(42));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_health_check_result_unhealthy() {
        let result = HealthCheckResult {
            proxy_url: "socks5://1.1.1.1:1080".to_string(),
            is_healthy: false,
            latency_ms: None,
            error: Some("timeout".to_string()),
            checked_at: Instant::now(),
        };
        assert!(!result.is_healthy);
        assert_eq!(result.error, Some("timeout".to_string()));
    }

    #[test]
    fn test_proxy_health_healthy_percentage() {
        let health = ProxyHealth {
            total: 10,
            healthy: 7,
            unhealthy: 3,
            results: vec![],
        };
        assert!((health.healthy_percentage() - 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_proxy_health_healthy_percentage_zero_total() {
        let health = ProxyHealth {
            total: 0,
            healthy: 0,
            unhealthy: 0,
            results: vec![],
        };
        assert_eq!(health.healthy_percentage(), 0.0);
    }

    #[test]
    fn test_proxy_health_all_healthy() {
        let health = ProxyHealth {
            total: 5,
            healthy: 5,
            unhealthy: 0,
            results: vec![],
        };
        assert!((health.healthy_percentage() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_proxy_health_none_healthy() {
        let health = ProxyHealth {
            total: 3,
            healthy: 0,
            unhealthy: 3,
            results: vec![],
        };
        assert!((health.healthy_percentage() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_health_checker_new() {
        let config = HealthCheckConfig {
            enabled: true,
            interval_secs: 30,
            timeout_ms: 5000,
            test_url: "https://example.com".to_string(),
            max_failures: 3,
        };
        let checker = HealthChecker::new(config);
        assert!(checker.is_ok());
    }

    #[test]
    fn test_health_checker_new_min_timeout() {
        let config = HealthCheckConfig {
            enabled: true,
            interval_secs: 30,
            timeout_ms: 100,
            test_url: "https://example.com".to_string(),
            max_failures: 3,
        };
        let checker = HealthChecker::new(config);
        assert!(checker.is_ok());
    }

    #[test]
    fn test_health_check_config_defaults_from_proxy_config() {
        let proxy_config = super::super::config::ProxyConfig::default();
        let hc: HealthCheckConfig = (&proxy_config).into();
        assert_eq!(hc.timeout_ms, 5000);
        assert_eq!(hc.interval_secs, 60);
    }

    #[tokio::test]
    async fn test_check_all_skips_disabled_proxies() {
        let config = HealthCheckConfig {
            enabled: true,
            interval_secs: 60,
            timeout_ms: 1000,
            test_url: "https://api.ipify.org".to_string(),
            max_failures: 3,
        };
        let checker = HealthChecker::new(config).unwrap();

        let mut disabled = make_proxy("127.0.0.1", 1080);
        disabled.enabled = false;

        let proxies = vec![disabled];
        let health = checker.check_all(&proxies).await.unwrap();
        assert_eq!(health.total, 1);
        assert_eq!(health.healthy, 0);
        assert_eq!(health.results.len(), 0);
    }

    #[tokio::test]
    async fn test_check_all_empty_proxies() {
        let config = HealthCheckConfig {
            enabled: true,
            interval_secs: 60,
            timeout_ms: 1000,
            test_url: "https://api.ipify.org".to_string(),
            max_failures: 3,
        };
        let checker = HealthChecker::new(config).unwrap();

        let health = checker.check_all(&[]).await.unwrap();
        assert_eq!(health.total, 0);
        assert_eq!(health.healthy, 0);
        assert_eq!(health.unhealthy, 0);
    }

    #[tokio::test]
    async fn test_check_concurrent_empty_proxies() {
        let config = HealthCheckConfig {
            enabled: true,
            interval_secs: 60,
            timeout_ms: 1000,
            test_url: "https://api.ipify.org".to_string(),
            max_failures: 3,
        };
        let checker = HealthChecker::new(config).unwrap();

        let health = checker.check_concurrent(&[], 5).await.unwrap();
        assert_eq!(health.total, 0);
        assert_eq!(health.results.len(), 0);
    }

    #[tokio::test]
    async fn test_check_concurrent_skips_disabled() {
        let config = HealthCheckConfig {
            enabled: true,
            interval_secs: 60,
            timeout_ms: 1000,
            test_url: "https://api.ipify.org".to_string(),
            max_failures: 3,
        };
        let checker = HealthChecker::new(config).unwrap();

        let mut disabled = make_proxy("127.0.0.1", 1080);
        disabled.enabled = false;

        let health = checker.check_concurrent(&[disabled], 5).await.unwrap();
        assert_eq!(health.total, 1);
        assert_eq!(health.results.len(), 0);
    }
}
