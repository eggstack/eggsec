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
            let key = proxy.to_log_key();
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
            let key = proxy.to_log_key();
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

        let saved_index = self.round_robin_index.load(Ordering::Relaxed);

        let mut chain = Vec::with_capacity(chain_length);
        let mut available = proxies.to_vec();

        for _ in 0..chain_length {
            if let Some(proxy) = self.select(&available) {
                available.retain(|p| p.to_log_key() != proxy.to_log_key());
                chain.push(proxy);
            }
        }

        if chain.len() == chain_length {
            Some(chain)
        } else {
            self.round_robin_index.store(saved_index, Ordering::Relaxed);
            None
        }
    }
}

impl Default for ProxyRotator {
    fn default() -> Self {
        Self::new(RotationStrategy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProxyType;

    fn make_proxy(addr: &str, port: u16) -> ProxyEntry {
        ProxyEntry::new(ProxyType::Socks5, addr.to_string(), port)
    }

    fn make_weighted_proxy(addr: &str, port: u16, weight: u32) -> ProxyEntry {
        ProxyEntry::new(ProxyType::Socks5, addr.to_string(), port).with_weight(weight)
    }

    #[test]
    fn test_rotator_default() {
        let rotator = ProxyRotator::default();
        assert!(matches!(rotator.strategy, RotationStrategy::RoundRobin));
    }

    #[test]
    fn test_select_empty_returns_none() {
        let rotator = ProxyRotator::new(RotationStrategy::RoundRobin);
        let proxies: Vec<ProxyEntry> = vec![];
        assert!(rotator.select(&proxies).is_none());
    }

    #[test]
    fn test_select_with_stats_empty_returns_none() {
        let rotator = ProxyRotator::new(RotationStrategy::RoundRobin);
        let proxies: Vec<ProxyEntry> = vec![];
        let no_stats = |_: &str| -> Option<ProxyStats> { None };
        assert!(rotator.select_with_stats(&proxies, &no_stats).is_none());
    }

    #[test]
    fn test_round_robin_cycles() {
        let rotator = ProxyRotator::new(RotationStrategy::RoundRobin);
        let proxies = vec![
            make_proxy("1.1.1.1", 1080),
            make_proxy("2.2.2.2", 1080),
            make_proxy("3.3.3.3", 1080),
        ];

        let first = rotator.select(&proxies).unwrap();
        let second = rotator.select(&proxies).unwrap();
        let third = rotator.select(&proxies).unwrap();
        let fourth = rotator.select(&proxies).unwrap();

        assert_ne!(first.address, second.address);
        assert_ne!(second.address, third.address);
        assert_eq!(fourth.address, first.address);
    }

    #[test]
    fn test_round_robin_single_proxy() {
        let rotator = ProxyRotator::new(RotationStrategy::RoundRobin);
        let proxies = vec![make_proxy("1.1.1.1", 1080)];

        for _ in 0..10 {
            let selected = rotator.select(&proxies).unwrap();
            assert_eq!(selected.address, "1.1.1.1");
        }
    }

    #[test]
    fn test_random_selects_from_pool() {
        let rotator = ProxyRotator::new(RotationStrategy::Random);
        let proxies = vec![
            make_proxy("1.1.1.1", 1080),
            make_proxy("2.2.2.2", 1080),
            make_proxy("3.3.3.3", 1080),
        ];

        for _ in 0..20 {
            let selected = rotator.select(&proxies).unwrap();
            assert!(proxies.iter().any(|p| p.address == selected.address));
        }
    }

    #[test]
    fn test_weighted_selects_from_pool() {
        let rotator = ProxyRotator::new(RotationStrategy::Weighted);
        let proxies = vec![
            make_weighted_proxy("1.1.1.1", 1080, 10),
            make_weighted_proxy("2.2.2.2", 1080, 5),
        ];

        for _ in 0..20 {
            let selected = rotator.select(&proxies).unwrap();
            assert!(proxies.iter().any(|p| p.address == selected.address));
        }
    }

    #[test]
    fn test_weighted_zero_weight_falls_back_to_random() {
        let rotator = ProxyRotator::new(RotationStrategy::Weighted);
        let proxies = vec![
            make_weighted_proxy("1.1.1.1", 1080, 0),
            make_weighted_proxy("2.2.2.2", 1080, 0),
        ];

        for _ in 0..10 {
            let selected = rotator.select(&proxies).unwrap();
            assert!(proxies.iter().any(|p| p.address == selected.address));
        }
    }

    #[test]
    fn test_weighted_distribution() {
        let rotator = ProxyRotator::new(RotationStrategy::Weighted);
        let proxies = vec![
            make_weighted_proxy("high", 1080, 90),
            make_weighted_proxy("low", 1080, 10),
        ];

        let mut high_count = 0;
        let iterations = 1000;
        for _ in 0..iterations {
            if rotator.select(&proxies).unwrap().address == "high" {
                high_count += 1;
            }
        }

        let pct = high_count as f64 / iterations as f64;
        assert!(
            pct > 0.8,
            "high weight proxy should be selected >80% of time, got {}",
            pct
        );
    }

    #[test]
    fn test_least_used_without_stats_returns_first() {
        let rotator = ProxyRotator::new(RotationStrategy::LeastUsed);
        let proxies = vec![make_proxy("1.1.1.1", 1080), make_proxy("2.2.2.2", 1080)];

        let selected = rotator.select(&proxies).unwrap();
        assert_eq!(selected.address, "1.1.1.1");
    }

    #[test]
    fn test_least_used_with_stats() {
        let rotator = ProxyRotator::new(RotationStrategy::LeastUsed);
        let proxies = vec![make_proxy("heavy", 1080), make_proxy("light", 1080)];

        let stats = |key: &str| -> Option<ProxyStats> {
            let mut s = ProxyStats::default();
            if key.contains("heavy") {
                s.total_requests = 100;
            } else {
                s.total_requests = 5;
            }
            Some(s)
        };

        let selected = rotator.select_with_stats(&proxies, &stats).unwrap();
        assert_eq!(selected.address, "light");
    }

    #[test]
    fn test_lowest_latency_without_stats_returns_first() {
        let rotator = ProxyRotator::new(RotationStrategy::LowestLatency);
        let proxies = vec![make_proxy("1.1.1.1", 1080), make_proxy("2.2.2.2", 1080)];

        let selected = rotator.select(&proxies).unwrap();
        assert_eq!(selected.address, "1.1.1.1");
    }

    #[test]
    fn test_lowest_latency_with_stats() {
        let rotator = ProxyRotator::new(RotationStrategy::LowestLatency);
        let proxies = vec![make_proxy("slow", 1080), make_proxy("fast", 1080)];

        let stats = |key: &str| -> Option<ProxyStats> {
            let mut s = ProxyStats::default();
            if key.contains("slow") {
                s.total_latency_ms = 5000;
                s.successful_requests = 10;
            } else {
                s.total_latency_ms = 100;
                s.successful_requests = 10;
            }
            Some(s)
        };

        let selected = rotator.select_with_stats(&proxies, &stats).unwrap();
        assert_eq!(selected.address, "fast");
    }

    #[test]
    fn test_select_chain_success() {
        let rotator = ProxyRotator::new(RotationStrategy::RoundRobin);
        let proxies = vec![
            make_proxy("1.1.1.1", 1080),
            make_proxy("2.2.2.2", 1080),
            make_proxy("3.3.3.3", 1080),
        ];

        let chain = rotator.select_chain(&proxies, 3).unwrap();
        assert_eq!(chain.len(), 3);

        let urls: Vec<_> = chain.iter().map(|p| p.to_log_key()).collect();
        let unique: std::collections::HashSet<_> = urls.iter().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn test_select_chain_too_long_returns_none() {
        let rotator = ProxyRotator::new(RotationStrategy::RoundRobin);
        let proxies = vec![make_proxy("1.1.1.1", 1080), make_proxy("2.2.2.2", 1080)];

        assert!(rotator.select_chain(&proxies, 5).is_none());
    }

    #[test]
    fn test_select_chain_zero_length() {
        let rotator = ProxyRotator::new(RotationStrategy::RoundRobin);
        let proxies = vec![make_proxy("1.1.1.1", 1080)];

        let chain = rotator.select_chain(&proxies, 0).unwrap();
        assert!(chain.is_empty());
    }

    #[test]
    fn test_select_chain_empty_proxies() {
        let rotator = ProxyRotator::new(RotationStrategy::RoundRobin);
        let proxies: Vec<ProxyEntry> = vec![];

        assert!(rotator.select_chain(&proxies, 1).is_none());
    }
}
