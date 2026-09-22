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

/// Application-level proxy health checker (Phase B, 2026-09-22).
///
/// Health means an HTTP(S) request through the selected proxy to the
/// configured `test_url` returning `2xx` — never mere proxy-connect
/// success. Reqwest is retained explicitly as the health-only owner: the
/// production dial path migrated to Eggress, but rebuilding HTTPS
/// verification, redirect, timeout, and body-cap semantics over raw Eggress
/// streams would recreate a general HTTP client to remove a dependency
/// line. See `architecture/egress_reuse_decision.md` and `outbound.rs`.
///
/// Protocol disposition (fixtures prove each; see `tests/health_matrix.rs`):
/// - `Socks5`/`Tor` -> SOCKS5 (faithful; Tor is SOCKS5 to the local daemon).
/// - `Http` -> HTTP CONNECT (faithful).
/// - `Https` -> HTTP CONNECT plaintext (faithful to current production dial
///   behavior, which is plaintext CONNECT under that enum; naming debt
///   recorded in Phase A, not a health misclassification).
/// - `Socks4` -> explicit unsupported error (fail closed). Reqwest cannot
///   represent SOCKS4; testing SOCKS5 instead would be false equivalence.
#[derive(Debug, Clone)]
pub struct HealthChecker {
    config: HealthCheckConfig,
}

impl HealthChecker {
    pub fn new(config: HealthCheckConfig) -> Result<Self> {
        Ok(Self { config })
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
        // Fail closed where the Reqwest backend cannot represent the type.
        // SOCKS4 has no faithful Reqwest mapping; testing SOCKS5 instead
        // would report health for a protocol never exercised (Phase B WS3
        // behavior fix with dedicated fixture, not invisible refactoring).
        if matches!(proxy.proxy_type, super::config::ProxyType::Socks4) {
            return Err(crate::error::WebProxyError::Proxy(
                "SOCKS4 health checks are not supported by the Reqwest health backend; \
                 configure a SOCKS5 proxy for application-level health validation"
                    .to_string(),
            ));
        }
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

        let timeout_secs = (self.config.timeout_ms / 1000).max(1);
        let client = create_insecure_client_with_options(timeout_secs, |builder| {
            builder.proxy(reqwest_proxy)
        })?;

        let response = client.get(&self.config.test_url).send().await?;

        Ok(response.status().is_success())
    }

    pub async fn check_all(&self, proxies: &[ProxyEntry]) -> Result<ProxyHealth> {
        let enabled_total = proxies.iter().filter(|p| p.enabled).count();
        let mut results = Vec::with_capacity(enabled_total);

        for proxy in proxies {
            if proxy.enabled {
                let result = self.check(proxy).await;
                results.push(result);
            }
        }

        let healthy = results.iter().filter(|r| r.is_healthy).count();

        Ok(ProxyHealth {
            total: enabled_total,
            healthy,
            unhealthy: enabled_total.saturating_sub(healthy),
            results,
        })
    }

    pub async fn check_concurrent(
        &self,
        proxies: &[ProxyEntry],
        concurrency: usize,
    ) -> Result<ProxyHealth> {
        use futures::stream::{self, StreamExt};

        // Bounded in-flight scheduler: O(concurrency) futures, one result
        // per enabled proxy, no spawn-per-proxy JoinHandle retention (Phase
        // B WS4). `check` never panics (all paths return data), so no
        // JoinError accounting is needed. `buffered` (not `buffer_unordered`)
        // preserves enabled-input ordering in the collected vector while
        // retaining the bound (corrective pass, 2026-09-22).
        let concurrency = concurrency.max(1);
        let enabled: Vec<ProxyEntry> = proxies.iter().filter(|p| p.enabled).cloned().collect();

        let results: Vec<HealthCheckResult> = stream::iter(enabled)
            .map(|proxy| {
                let checker = self.clone();
                async move { checker.check(&proxy).await }
            })
            .buffered(concurrency)
            .collect()
            .await;

        let checked_total = results.len();
        let healthy = results.iter().filter(|r| r.is_healthy).count();

        Ok(ProxyHealth {
            total: checked_total,
            healthy,
            unhealthy: checked_total.saturating_sub(healthy),
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProxyEntry, ProxyType};
    use std::time::Duration;

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
        assert_eq!(health.total, 0);
        assert_eq!(health.healthy, 0);
        assert_eq!(health.unhealthy, 0);
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
        assert_eq!(health.total, 0);
        assert_eq!(health.unhealthy, 0);
        assert_eq!(health.results.len(), 0);
    }

    #[tokio::test]
    async fn test_check_concurrent_preserves_enabled_input_order() {
        // Deterministic ordering proof (corrective pass, 2026-09-22):
        // proxy A is slow (blackhole holds the socket until the health
        // timeout), proxy B fails fast (closed port refuses immediately).
        // Completion order is B-then-A, but the returned vector must remain
        // A-then-B while staying bounded (no spawn-per-proxy).
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let slow_port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let Ok((s, _)) = listener.accept().await else {
                return;
            };
            let _held = s;
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        let config = HealthCheckConfig {
            enabled: true,
            interval_secs: 60,
            timeout_ms: 1500,
            test_url: "http://127.0.0.1:9/health".to_string(),
            max_failures: 3,
        };
        let checker = HealthChecker::new(config).unwrap();
        let slow = make_proxy("127.0.0.1", slow_port);
        let fast = make_proxy("127.0.0.1", 9);
        let slow_key = slow.to_log_key();
        let fast_key = fast.to_log_key();
        let health = checker.check_concurrent(&[slow, fast], 2).await.unwrap();
        assert_eq!(health.total, 2);
        assert_eq!(health.results.len(), 2);
        assert_eq!(health.results[0].proxy_url, slow_key);
        assert_eq!(health.results[1].proxy_url, fast_key);
        assert_eq!(health.healthy + health.unhealthy, 2);
    }
}
