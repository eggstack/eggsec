//! Phase A ProxyPool baseline (manual, synthetic, informational).
//!
//! `#[ignore]`: run explicitly in release mode:
//!
//! ```bash
//! cargo test --release -p eggsec-web-proxy --test perf_pool_baseline -- --ignored --nocapture
//! ```

use eggsec_web_proxy::{ProxyConfig, ProxyEntry, ProxyPool, ProxyType};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Instant;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn checksum<I: Hash>(items: &[I]) -> u64 {
    let mut h = DefaultHasher::new();
    items.len().hash(&mut h);
    for item in items {
        item.hash(&mut h);
    }
    h.finish()
}

#[test]
#[ignore]
fn perf_pool_sort_baseline() {
    let trials = env_usize("EGGSEC_PERF_TRIALS", 5).max(1);
    let warmup = env_usize("EGGSEC_PERF_WARMUP", 1);
    let size = 5_000usize;

    let pool = ProxyPool::new(ProxyConfig::default());
    for i in 0..size {
        let mut entry = ProxyEntry::new(
            ProxyType::Http,
            format!("10.{}.{}.{}", (i >> 16) & 0xFF, (i >> 8) & 0xFF, i & 0xFF),
            8080,
        );
        entry.priority = (i % 256) as u8;
        pool.add(entry.clone());
        if i % 2 == 0 {
            pool.record_success(&entry, (i % 120) as u64);
        } else {
            pool.record_failure(&entry);
        }
    }

    let mut latency_walls = Vec::new();
    let mut rate_walls = Vec::new();
    let mut last_checksum = 0u64;
    for trial in 0..warmup + trials {
        let start = Instant::now();
        let by_latency = pool.get_sorted_by_latency();
        let latency_wall = start.elapsed();
        let start = Instant::now();
        let by_rate = pool.get_sorted_by_success_rate();
        let rate_wall = start.elapsed();
        assert_eq!(by_latency.len(), size);
        assert_eq!(by_rate.len(), size);
        let addresses: Vec<&str> = by_latency.iter().map(|p| p.address.as_str()).collect();
        last_checksum = checksum(&addresses);
        if trial >= warmup {
            latency_walls.push(latency_wall.as_secs_f64());
            rate_walls.push(rate_wall.as_secs_f64());
            println!(
                "perf pool trial={} size={} latency_ms={} rate_ms={} checksum={:#x}",
                trial,
                size,
                latency_wall.as_millis(),
                rate_wall.as_millis(),
                last_checksum
            );
        }
    }
    latency_walls.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    rate_walls.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    println!(
        "perf pool summary size={} trials={} warmup={} median_latency_s={:.4} median_rate_s={:.4} checksum={:#x}",
        size,
        trials,
        warmup,
        latency_walls[latency_walls.len() / 2],
        rate_walls[rate_walls.len() / 2],
        last_checksum
    );
}
