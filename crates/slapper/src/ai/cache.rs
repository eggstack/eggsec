use crate::ai::errors::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub value: String,
    pub created_at: Instant,
    pub ttl: Duration,
    pub hit_count: u64,
}

impl CacheEntry {
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_entries: usize,
    default_ttl: Duration,
}

impl AiCache {
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            default_ttl,
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if !entry.is_expired() {
                return Some(entry.value.clone());
            }
        }
        None
    }

    pub async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) {
        let mut entries = self.entries.write().await;
        if entries.len() >= self.max_entries {
            self.evict_expired(&mut entries);
            if entries.len() >= self.max_entries {
                if let Some((oldest_key, _)) = entries.iter()
                    .min_by_key(|(_, v)| v.created_at)
                    .map(|(k, v)| (k.clone(), v.clone()))
                {
                    entries.remove(&oldest_key);
                }
            }
        }
        entries.insert(key.to_string(), CacheEntry {
            value: value.to_string(),
            created_at: Instant::now(),
            ttl: ttl.unwrap_or(self.default_ttl),
            hit_count: 0,
        });
    }

    pub async fn remove(&self, key: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    pub async fn len(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    pub async fn is_empty(&self) -> bool {
        let entries = self.entries.read().await;
        entries.is_empty()
    }

    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let total_hits: u64 = entries.values().map(|e| e.hit_count).sum();
        let expired = entries.values().filter(|e| e.is_expired()).count();
        CacheStats {
            total_entries: entries.len(),
            expired_entries: expired,
            total_hits,
        }
    }

    fn evict_expired(&self, entries: &mut HashMap<String, CacheEntry>) {
        entries.retain(|_, v| !v.is_expired());
    }

    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        self.evict_expired(&mut entries);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub total_hits: u64,
}

pub struct CacheKeyBuilder;

impl CacheKeyBuilder {
    pub fn for_payload_suggestion(vuln_type: &str, context: &str) -> String {
        format!("payload:{}:{}", vuln_type, context)
    }

    pub fn for_waf_bypass(waf: &str, blocked_payload: &str) -> String {
        format!("waf_bypass:{}:{}", waf, blocked_payload)
    }

    pub fn for_finding_analysis(findings_hash: &str) -> String {
        format!("analysis:{}", findings_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache = AiCache::new(10, Duration::from_secs(60));
        cache.set("key1", "value1", None).await;
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        let cache = AiCache::new(10, Duration::from_millis(1));
        cache.set("key1", "value1", None).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert_eq!(cache.get("key1").await, None);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = AiCache::new(2, Duration::from_secs(60));
        cache.set("key1", "value1", None).await;
        cache.set("key2", "value2", None).await;
        cache.set("key3", "value3", None).await;
        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_some());
        assert!(cache.get("key3").await.is_some());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = AiCache::new(10, Duration::from_secs(60));
        cache.set("key1", "value1", None).await;
        cache.get("key1").await;
        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 1);
    }
}
