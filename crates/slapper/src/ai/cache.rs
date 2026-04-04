use crate::ai::errors::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub value: String,
    #[serde(skip)]
    created_at: Instant,
    #[serde(default)]
    created_at_ser: Option<DateTime<Utc>>,
    pub ttl: Duration,
    #[serde(default)]
    hit_count: u64,
}

impl CacheEntry {
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    fn new(value: String, ttl: Duration) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            created_at_ser: None,
            ttl,
            hit_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "AiCacheSerialized", into = "AiCacheSerialized")]
pub struct AiCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_entries: usize,
    default_ttl: Duration,
    persist_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiCacheSerialized {
    entries: HashMap<String, CacheEntrySer>,
    max_entries: usize,
    default_ttl_nanos: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntrySer {
    value: String,
    created_at: DateTime<Utc>,
    ttl_nanos: u64,
    hit_count: u64,
}

impl From<AiCacheSerialized> for AiCache {
    fn from(serialized: AiCacheSerialized) -> Self {
        let entries: HashMap<String, CacheEntry> = serialized
            .entries
            .into_iter()
            .map(|(k, v)| {
                let created_at = Instant::now() - Duration::from_nanos(v.created_at.signed_duration_since(Utc::now()).abs().num_nanoseconds() as u64);
                let ttl = Duration::from_nanos(v.ttl_nanos);
                (k, CacheEntry {
                    value: v.value,
                    created_at,
                    created_at_ser: Some(v.created_at),
                    ttl,
                    hit_count: v.hit_count,
                })
            })
            .collect();
        
        AiCache {
            entries: Arc::new(RwLock::new(entries)),
            max_entries: serialized.max_entries,
            default_ttl: Duration::from_nanos(serialized.default_ttl_nanos),
            persist_path: None,
        }
    }
}

impl From<AiCache> for AiCacheSerialized {
    fn from(cache: AiCache) -> Self {
        let entries: HashMap<String, CacheEntrySer> = HashMap::new();
        AiCacheSerialized {
            entries,
            max_entries: cache.max_entries,
            default_ttl_nanos: cache.default_ttl.as_nanos() as u64,
        }
    }
}

impl AiCache {
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            default_ttl,
            persist_path: None,
        }
    }

    pub fn with_persistence(mut self, path: PathBuf) -> Self {
        self.persist_path = Some(path);
        if let Some(ref path) = self.persist_path {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    if let Ok(serialized) = serde_json::from_str::<AiCacheSerialized>(&contents) {
                        let cache: AiCache = serialized.into();
                        self.entries = cache.entries;
                    }
                }
            }
        }
        self
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
        entries.insert(key.to_string(), CacheEntry::new(value.to_string(), ttl.unwrap_or(self.default_ttl)));
        drop(entries);
        self.persist().await;
    }

    pub async fn remove(&self, key: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
        drop(entries);
        self.persist().await;
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        drop(entries);
        self.persist().await;
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
        drop(entries);
        self.persist().await;
    }

    async fn persist(&self) {
        if let Some(ref path) = self.persist_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            
            let entries = self.entries.read().await;
            let serialized_entries: HashMap<String, CacheEntrySer> = entries
                .iter()
                .map(|(k, v)| {
                    let created_at_ser = v.created_at_ser.unwrap_or_else(|| {
                        Utc::now() - Duration::from_nanos(0)
                    });
                    (k.clone(), CacheEntrySer {
                        value: v.value.clone(),
                        created_at: created_at_ser,
                        ttl_nanos: v.ttl.as_nanos() as u64,
                        hit_count: v.hit_count,
                    })
                })
                .collect();
            
            let serialized = AiCacheSerialized {
                entries: serialized_entries,
                max_entries: self.max_entries,
                default_ttl_nanos: self.default_ttl.as_nanos() as u64,
            };
            
            if let Ok(json) = serde_json::to_string_pretty(&serialized) {
                let _ = std::fs::write(path, json);
            }
        }
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
