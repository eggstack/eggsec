
use super::config::{ProxyEntry, RotationStrategy};
use super::pool::ProxyStats;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct ProxyRotator {
    strategy: RotationStrategy,
    round_robin_index: Arc<AtomicU64>,
}

impl ProxyRotator {
    pub fn new(strategy: RotationStrategy) -> Self {
        Self {
            strategy,
            round_robin_index: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn select(&self, proxies: &[ProxyEntry]) -> Option<ProxyEntry> {
        if proxies.is_empty() {
            return None;
        }

        match self.strategy {
            RotationStrategy::RoundRobin => self.select_round_robin(proxies),
            RotationStrategy::Random => self.select_random(proxies),
            RotationStrategy::Weighted => self.select_weighted(proxies),
            RotationStrategy::LeastUsed => {
                let no_stats = |_: &str| -> Option<ProxyStats> { None };
                self.select_least_used(proxies, Some(&no_stats))
            }
            RotationStrategy::LowestLatency => {
                let no_stats = |_: &str| -> Option<ProxyStats> { None };
                self.select_lowest_latency(proxies, Some(&no_stats))
            }
        }
    }

    pub fn select_with_stats(
        &self,
        proxies: &[ProxyEntry],
        stats: &impl Fn(&str) -> Option<ProxyStats>,
    ) -> Option<ProxyEntry> {
        if proxies.is_empty() {
            return None;
        }

        match self.strategy {
            RotationStrategy::RoundRobin => self.select_round_robin(proxies),
            RotationStrategy::Random => self.select_random(proxies),
            RotationStrategy::Weighted => self.select_weighted(proxies),
            RotationStrategy::LeastUsed => self.select_least_used(proxies, Some(stats)),
            RotationStrategy::LowestLatency => self.select_lowest_latency(proxies, Some(stats)),
        }
    }

    fn select_round_robin(&self, proxies: &[ProxyEntry]) -> Option<ProxyEntry> {
        let index = self.round_robin_index.fetch_add(1, Ordering::Relaxed) as usize;
        let proxy = proxies[index % proxies.len()].clone();
        Some(proxy)
    }

    fn select_random(&self, proxies: &[ProxyEntry]) -> Option<ProxyEntry> {
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..proxies.len());
        proxies.get(idx).cloned()
    }

    fn select_weighted(&self, proxies: &[ProxyEntry]) -> Option<ProxyEntry> {
        let total_weight: u32 = proxies.iter().map(|p| p.weight).sum();
        if total_weight == 0 {
            return self.select_random(proxies);
        }

        let mut rng = rand::thread_rng();
        let mut target = rng.gen_range(1..=total_weight);

        for proxy in proxies {
            if target <= proxy.weight {
                return Some(proxy.clone());
            }
            target -= proxy.weight;
        }

        proxies.last().cloned()
    }

    fn select_least_used(
        &self,
        proxies: &[ProxyEntry],
        stats: Option<&impl Fn(&str) -> Option<ProxyStats>>,
    ) -> Option<ProxyEntry> {
        let stats_fn = stats?;

        let mut least_used_proxy: Option<(&ProxyEntry, u64)> = None;

        for proxy in proxies {
            let key = proxy.to_url();
            if let Some(proxy_stats) = stats_fn(&key) {
                let usage = proxy_stats.total_requests;
                match least_used_proxy {
                    None => {
                        least_used_proxy = Some((proxy, usage));
                    }
                    Some((_, min_usage)) if usage < min_usage => {
                        least_used_proxy = Some((proxy, usage));
                    }
                    _ => {}
                }
            } else if least_used_proxy.is_none() {
                least_used_proxy = Some((proxy, 0));
            }
        }

        least_used_proxy.map(|(proxy, _)| proxy.clone())
    }

    fn select_lowest_latency(
        &self,
        proxies: &[ProxyEntry],
        stats: Option<&impl Fn(&str) -> Option<ProxyStats>>,
    ) -> Option<ProxyEntry> {
        let stats_fn = stats?;

        let mut lowest_latency_proxy: Option<(&ProxyEntry, u64)> = None;

        for proxy in proxies {
            let key = proxy.to_url();
            if let Some(proxy_stats) = stats_fn(&key) {
                let latency = proxy_stats.avg_latency_ms();
                match lowest_latency_proxy {
                    None => {
                        lowest_latency_proxy = Some((proxy, latency));
                    }
                    Some((_, min_latency)) if latency < min_latency && latency > 0 => {
                        lowest_latency_proxy = Some((proxy, latency));
                    }
                    Some((_, 0)) => {
                        // Keep current if it has 0 latency (no data yet)
                    }
                    _ => {}
                }
            } else if lowest_latency_proxy.is_none() {
                lowest_latency_proxy = Some((proxy, 0));
            }
        }

        lowest_latency_proxy.map(|(proxy, _)| proxy.clone())
    }

    pub fn select_chain(
        &self,
        proxies: &[ProxyEntry],
        chain_length: usize,
    ) -> Option<Vec<ProxyEntry>> {
        if proxies.len() < chain_length {
            return None;
        }

        let mut chain = Vec::with_capacity(chain_length);
        let mut available = proxies.to_vec();

        for _ in 0..chain_length {
            if let Some(proxy) = self.select(&available) {
                available.retain(|p| p.to_url() != proxy.to_url());
                chain.push(proxy);
            }
        }

        if chain.len() == chain_length {
            Some(chain)
        } else {
            None
        }
    }
}

impl Default for ProxyRotator {
    fn default() -> Self {
        Self::new(RotationStrategy::default())
    }
}
