#![allow(dead_code)]

use crate::error::Result;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::config::{ProxyConfig, ProxyEntry, ProxyType};

#[derive(Debug, Clone)]
pub struct ProxyStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_latency_ms: u64,
    pub last_used: Option<std::time::Instant>,
    pub last_failure: Option<std::time::Instant>,
    pub consecutive_failures: u32,
    pub is_healthy: bool,
}

impl Default for ProxyStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_latency_ms: 0,
            last_used: None,
            last_failure: None,
            consecutive_failures: 0,
            is_healthy: true,
        }
    }
}

impl ProxyStats {
    pub fn avg_latency_ms(&self) -> u64 {
        if self.successful_requests == 0 {
            return 0;
        }
        self.total_latency_ms / self.successful_requests
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        self.successful_requests as f64 / self.total_requests as f64
    }
}

pub struct ProxyPool {
    proxies: DashMap<String, ProxyEntry>,
    stats: DashMap<String, ProxyStats>,
    config: ProxyConfig,
    round_robin_index: Arc<AtomicU64>,
}

impl ProxyPool {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            proxies: DashMap::new(),
            stats: DashMap::new(),
            config,
            round_robin_index: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn add(&mut self, proxy: ProxyEntry) {
        let key = proxy.to_url();
        self.stats.insert(key.clone(), ProxyStats::default());
        self.proxies.insert(key, proxy);
    }

    pub fn remove(&self, key: &str) -> Option<ProxyEntry> {
        self.stats.remove(key);
        self.proxies.remove(key).map(|(_, v)| v)
    }

    pub fn get(&self, key: &str) -> Option<ProxyEntry> {
        self.proxies.get(key).map(|r| r.clone())
    }

    pub fn size(&self) -> usize {
        self.proxies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.proxies.is_empty()
    }

    pub fn get_all(&self) -> Vec<ProxyEntry> {
        self.proxies.iter().map(|r| r.clone()).collect()
    }

    pub fn get_healthy(&self) -> Vec<ProxyEntry> {
        self.proxies
            .iter()
            .filter(|r| match self.stats.get(r.key()) {
                Some(stats) => stats.is_healthy && r.enabled,
                _ => r.enabled,
            })
            .map(|r| r.clone())
            .collect()
    }

    pub fn get_by_priority(&self, min_priority: u8) -> Vec<ProxyEntry> {
        self.proxies
            .iter()
            .filter(|r| r.priority >= min_priority && r.enabled)
            .map(|r| r.clone())
            .collect()
    }

    pub fn get_sorted_by_priority(&self) -> Vec<ProxyEntry> {
        let mut proxies: Vec<_> = self.get_healthy();
        proxies.sort_by(|a, b| b.priority.cmp(&a.priority));
        proxies
    }

    pub fn get_by_type(&self, proxy_type: ProxyType) -> Vec<ProxyEntry> {
        self.proxies
            .iter()
            .filter(|r| r.proxy_type == proxy_type)
            .map(|r| r.clone())
            .collect()
    }

    pub fn get_sorted_by_latency(&self) -> Vec<ProxyEntry> {
        let mut proxies: Vec<_> = self.get_healthy();
        proxies.sort_by_key(|p| {
            self.stats
                .get(&p.to_log_key())
                .map(|s| s.avg_latency_ms())
                .unwrap_or(u64::MAX)
        });
        proxies
    }

    pub fn get_sorted_by_success_rate(&self) -> Vec<ProxyEntry> {
        let mut proxies: Vec<_> = self.get_healthy();
        proxies.sort_by(|a, b| {
            let rate_a = self
                .stats
                .get(&a.to_log_key())
                .map(|s| s.success_rate())
                .unwrap_or(0.0);
            let rate_b = self
                .stats
                .get(&b.to_log_key())
                .map(|s| s.success_rate())
                .unwrap_or(0.0);
            rate_b
                .partial_cmp(&rate_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        proxies
    }

    pub fn record_success(&self, proxy: &ProxyEntry, latency_ms: u64) {
        let key = proxy.to_log_key();
        if let Some(mut stats) = self.stats.get_mut(&key) {
            stats.total_requests += 1;
            stats.successful_requests += 1;
            stats.total_latency_ms += latency_ms;
            stats.last_used = Some(std::time::Instant::now());
            stats.consecutive_failures = 0;
            stats.is_healthy = true;
        }
    }

    pub fn record_failure(&self, proxy: &ProxyEntry) {
        let key = proxy.to_log_key();
        if let Some(mut stats) = self.stats.get_mut(&key) {
            stats.total_requests += 1;
            stats.failed_requests += 1;
            stats.last_failure = Some(std::time::Instant::now());
            stats.consecutive_failures += 1;

            if stats.consecutive_failures >= self.config.max_failures_before_disable {
                stats.is_healthy = false;
                tracing::warn!(
                    proxy = %key,
                    consecutive_failures = stats.consecutive_failures,
                    "Proxy marked as unhealthy"
                );
            }
        }
    }

    pub fn mark_healthy(&self, key: &str) {
        if let Some(mut stats) = self.stats.get_mut(key) {
            stats.is_healthy = true;
            stats.consecutive_failures = 0;
        }
    }

    pub fn mark_unhealthy(&self, key: &str) {
        if let Some(mut stats) = self.stats.get_mut(key) {
            stats.is_healthy = false;
        }
    }

    pub fn get_stats(&self, key: &str) -> Option<ProxyStats> {
        self.stats.get(key).map(|s| s.clone())
    }

    pub fn get_round_robin_index(&self) -> u64 {
        self.round_robin_index.fetch_add(1, Ordering::Relaxed)
    }

    pub fn clear(&self) {
        self.proxies.clear();
        self.stats.clear();
    }
}

pub struct ProxyPoolBuilder {
    proxies: Vec<ProxyEntry>,
    config: ProxyConfig,
}

impl ProxyPoolBuilder {
    pub fn new() -> Self {
        Self {
            proxies: Vec::new(),
            config: ProxyConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ProxyConfig) -> Self {
        self.config = config;
        self
    }

    pub fn add_proxy(mut self, proxy: ProxyEntry) -> Self {
        self.proxies.push(proxy);
        self
    }

    pub fn add_proxies(mut self, proxies: Vec<ProxyEntry>) -> Self {
        self.proxies.extend(proxies);
        self
    }

    pub fn add_from_file(self, path: &str) -> Result<Self> {
        let proxies = ProxyEntry::load_from_file(path)?;
        Ok(self.add_proxies(proxies))
    }

    pub fn build(self) -> ProxyPool {
        let mut pool = ProxyPool::new(self.config);
        for proxy in self.proxies {
            pool.add(proxy);
        }
        pool
    }
}

impl Default for ProxyPoolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proxy(addr: &str, port: u16) -> ProxyEntry {
        ProxyEntry::new(ProxyType::Socks5, addr.to_string(), port)
    }

    #[test]
    fn test_proxy_stats_default() {
        let stats = ProxyStats::default();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.failed_requests, 0);
        assert_eq!(stats.total_latency_ms, 0);
        assert!(stats.is_healthy);
        assert_eq!(stats.consecutive_failures, 0);
    }

    #[test]
    fn test_proxy_stats_avg_latency_zero_requests() {
        let stats = ProxyStats::default();
        assert_eq!(stats.avg_latency_ms(), 0);
    }

    #[test]
    fn test_proxy_stats_avg_latency_with_data() {
        let mut stats = ProxyStats::default();
        stats.total_latency_ms = 300;
        stats.successful_requests = 3;
        assert_eq!(stats.avg_latency_ms(), 100);
    }

    #[test]
    fn test_proxy_stats_success_rate_zero_requests() {
        let stats = ProxyStats::default();
        assert_eq!(stats.success_rate(), 1.0);
    }

    #[test]
    fn test_proxy_stats_success_rate_with_data() {
        let mut stats = ProxyStats::default();
        stats.total_requests = 10;
        stats.successful_requests = 7;
        stats.failed_requests = 3;
        assert!((stats.success_rate() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pool_new_is_empty() {
        let config = ProxyConfig::default();
        let pool = ProxyPool::new(config);
        assert!(pool.is_empty());
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_pool_add_and_get() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let proxy = make_proxy("127.0.0.1", 1080);
        let key = proxy.to_url();
        pool.add(proxy.clone());

        assert_eq!(pool.size(), 1);
        let got = pool.get(&key).unwrap();
        assert_eq!(got.address, "127.0.0.1");
        assert_eq!(got.port, 1080);
    }

    #[test]
    fn test_pool_remove() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let proxy = make_proxy("127.0.0.1", 1080);
        let key = proxy.to_url();
        pool.add(proxy);

        let removed = pool.remove(&key).unwrap();
        assert_eq!(removed.address, "127.0.0.1");
        assert!(pool.is_empty());
        assert!(pool.get(&key).is_none());
    }

    #[test]
    fn test_pool_remove_nonexistent() {
        let config = ProxyConfig::default();
        let pool = ProxyPool::new(config);
        assert!(pool.remove("nonexistent").is_none());
    }

    #[test]
    fn test_pool_get_all() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        pool.add(make_proxy("1.1.1.1", 1080));
        pool.add(make_proxy("2.2.2.2", 1080));
        pool.add(make_proxy("3.3.3.3", 1080));

        let all = pool.get_all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_pool_get_healthy_filters_unhealthy() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let p1 = make_proxy("1.1.1.1", 1080);
        let p2 = make_proxy("2.2.2.2", 1080);
        let k1 = p1.to_url();
        pool.add(p1);
        pool.add(p2);

        pool.mark_unhealthy(&k1);
        let healthy = pool.get_healthy();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].address, "2.2.2.2");
    }

    #[test]
    fn test_pool_get_healthy_filters_disabled() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let mut p1 = make_proxy("1.1.1.1", 1080);
        p1.enabled = false;
        pool.add(p1);
        pool.add(make_proxy("2.2.2.2", 1080));

        let healthy = pool.get_healthy();
        assert_eq!(healthy.len(), 1);
    }

    #[test]
    fn test_pool_get_by_priority() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let mut p1 = make_proxy("1.1.1.1", 1080);
        p1.priority = 5;
        let mut p2 = make_proxy("2.2.2.2", 1080);
        p2.priority = 10;
        pool.add(p1);
        pool.add(p2);

        let high = pool.get_by_priority(10);
        assert_eq!(high.len(), 1);
        assert_eq!(high[0].address, "2.2.2.2");
    }

    #[test]
    fn test_pool_get_sorted_by_priority() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let mut p1 = make_proxy("1.1.1.1", 1080);
        p1.priority = 5;
        let mut p2 = make_proxy("2.2.2.2", 1080);
        p2.priority = 10;
        pool.add(p1);
        pool.add(p2);

        let sorted = pool.get_sorted_by_priority();
        assert_eq!(sorted[0].address, "2.2.2.2");
        assert_eq!(sorted[1].address, "1.1.1.1");
    }

    #[test]
    fn test_pool_get_by_type() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        pool.add(ProxyEntry::new(
            ProxyType::Socks5,
            "1.1.1.1".to_string(),
            1080,
        ));
        pool.add(ProxyEntry::new(
            ProxyType::Http,
            "2.2.2.2".to_string(),
            8080,
        ));

        let socks = pool.get_by_type(ProxyType::Socks5);
        assert_eq!(socks.len(), 1);
        assert_eq!(socks[0].address, "1.1.1.1");
    }

    #[test]
    fn test_pool_record_success_and_failure() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let proxy = make_proxy("1.1.1.1", 1080);
        let key = proxy.to_url();
        pool.add(proxy.clone());

        pool.record_success(&proxy, 50);
        pool.record_success(&proxy, 100);
        pool.record_failure(&proxy);

        let stats = pool.get_stats(&key).unwrap();
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.successful_requests, 2);
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.total_latency_ms, 150);
    }

    #[test]
    fn test_pool_consecutive_failures_marks_unhealthy() {
        let mut config = ProxyConfig::default();
        config.max_failures_before_disable = 2;
        let mut pool = ProxyPool::new(config);
        let proxy = make_proxy("1.1.1.1", 1080);
        let key = proxy.to_url();
        pool.add(proxy.clone());

        pool.record_failure(&proxy);
        let stats = pool.get_stats(&key).unwrap();
        assert!(stats.is_healthy);

        pool.record_failure(&proxy);
        let stats = pool.get_stats(&key).unwrap();
        assert!(!stats.is_healthy);
    }

    #[test]
    fn test_pool_mark_healthy_resets_failures() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let proxy = make_proxy("1.1.1.1", 1080);
        let key = proxy.to_url();
        pool.add(proxy);

        pool.mark_unhealthy(&key);
        pool.mark_healthy(&key);

        let stats = pool.get_stats(&key).unwrap();
        assert!(stats.is_healthy);
        assert_eq!(stats.consecutive_failures, 0);
    }

    #[test]
    fn test_pool_clear() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        pool.add(make_proxy("1.1.1.1", 1080));
        pool.add(make_proxy("2.2.2.2", 1080));

        pool.clear();
        assert!(pool.is_empty());
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_pool_round_robin_index_increments() {
        let config = ProxyConfig::default();
        let pool = ProxyPool::new(config);

        let a = pool.get_round_robin_index();
        let b = pool.get_round_robin_index();
        assert_eq!(b, a + 1);
    }

    #[test]
    fn test_pool_builder_default() {
        let builder = ProxyPoolBuilder::default();
        let pool = builder.build();
        assert!(pool.is_empty());
    }

    #[test]
    fn test_pool_builder_with_proxies() {
        let pool = ProxyPoolBuilder::new()
            .add_proxy(make_proxy("1.1.1.1", 1080))
            .add_proxy(make_proxy("2.2.2.2", 1080))
            .build();

        assert_eq!(pool.size(), 2);
    }

    #[test]
    fn test_pool_builder_with_config() {
        let mut config = ProxyConfig::default();
        config.max_failures_before_disable = 1;
        let pool = ProxyPoolBuilder::new()
            .with_config(config)
            .add_proxy(make_proxy("1.1.1.1", 1080))
            .build();

        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_pool_get_sorted_by_latency() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let p1 = make_proxy("1.1.1.1", 1080);
        let p2 = make_proxy("2.2.2.2", 1080);
        pool.add(p1.clone());
        pool.add(p2.clone());

        pool.record_success(&p1, 200);
        pool.record_success(&p2, 50);

        let sorted = pool.get_sorted_by_latency();
        assert_eq!(sorted[0].address, "2.2.2.2");
        assert_eq!(sorted[1].address, "1.1.1.1");
    }

    #[test]
    fn test_pool_get_sorted_by_success_rate() {
        let config = ProxyConfig::default();
        let mut pool = ProxyPool::new(config);
        let p1 = make_proxy("1.1.1.1", 1080);
        let p2 = make_proxy("2.2.2.2", 1080);
        pool.add(p1.clone());
        pool.add(p2.clone());

        pool.record_success(&p1, 10);
        pool.record_failure(&p1);
        pool.record_success(&p2, 10);

        let sorted = pool.get_sorted_by_success_rate();
        assert_eq!(sorted[0].address, "2.2.2.2");
        assert_eq!(sorted[1].address, "1.1.1.1");
    }
}
