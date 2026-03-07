#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub expires_at: Instant,
}

pub struct ApiCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry<serde_json::Value>>>>,
    ttl: Duration,
    max_entries: usize,
}

impl ApiCache {
    pub fn new(ttl_secs: u64, max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_secs),
            max_entries,
        }
    }

    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        let entries = self.entries.read().await;
        let entry = entries.get(key)?;
        
        if Instant::now() > entry.expires_at {
            None
        } else {
            Some(entry.value.clone())
        }
    }

    pub async fn set(&self, key: String, value: serde_json::Value) {
        let mut entries = self.entries.write().await;
        
        if entries.len() >= self.max_entries {
            self.evict_oldest(&mut entries).await;
        }
        
        let expires_at = Instant::now() + self.ttl;
        entries.insert(key, CacheEntry { value, expires_at });
    }

    pub async fn set_ttl(&self, key: String, value: serde_json::Value, ttl_secs: u64) {
        let mut entries = self.entries.write().await;
        
        if entries.len() >= self.max_entries {
            self.evict_oldest(&mut entries).await;
        }
        
        let expires_at = Instant::now() + Duration::from_secs(ttl_secs);
        entries.insert(key, CacheEntry { value, expires_at });
    }

    async fn evict_oldest(&self, entries: &mut HashMap<String, CacheEntry<serde_json::Value>>) {
        let oldest_key = entries.iter()
            .min_by_key(|(_, v)| v.expires_at)
            .map(|(k, _)| k.clone());
        
        if let Some(key) = oldest_key {
            entries.remove(&key);
        }
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let now = Instant::now();
        entries.retain(|_, v| now < v.expires_at);
    }
}

impl Default for ApiCache {
    fn default() -> Self {
        Self::new(3600, 10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache = ApiCache::new(3600, 10);
        
        cache.set("key1".to_string(), serde_json::json!("value1")).await;
        
        let value = cache.get("key1").await;
        assert!(value.is_some());
        assert_eq!(value.unwrap(), "value1");
    }

    #[tokio::test]
    async fn test_cache_expired() {
        let cache = ApiCache::new(0, 10);
        
        cache.set("key1".to_string(), serde_json::json!("value1")).await;
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let value = cache.get("key1").await;
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = ApiCache::new(3600, 10);
        
        let value = cache.get("nonexistent").await;
        assert!(value.is_none());
    }
}
