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
                .get(&p.to_url())
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
                .get(&a.to_url())
                .map(|s| s.success_rate())
                .unwrap_or(0.0);
            let rate_b = self
                .stats
                .get(&b.to_url())
                .map(|s| s.success_rate())
                .unwrap_or(0.0);
            rate_b
                .partial_cmp(&rate_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        proxies
    }

    pub fn record_success(&self, proxy: &ProxyEntry, latency_ms: u64) {
        let key = proxy.to_url();
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
        let key = proxy.to_url();
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
