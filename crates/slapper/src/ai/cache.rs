use chrono::{DateTime, Utc};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct CacheEntry {
    pub value: String,
    created_at: DateTime<Utc>,
    pub ttl: Duration,
    #[serde(default)]
    pub hit_count: u64,
}

impl<'de> Deserialize<'de> for CacheEntry {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CacheEntrySer {
            value: String,
            created_at: DateTime<Utc>,
            ttl_nanos: Option<u64>,
            hit_count: Option<u64>,
        }
        let ser = CacheEntrySer::deserialize(_deserializer)?;
        Ok(Self {
            value: ser.value,
            created_at: ser.created_at,
            ttl: Duration::from_nanos(ser.ttl_nanos.unwrap_or(0)),
            hit_count: ser.hit_count.unwrap_or(0),
        })
    }
}

impl Default for CacheEntry {
    fn default() -> Self {
        Self {
            value: String::new(),
            created_at: Utc::now(),
            ttl: Duration::default(),
            hit_count: 0,
        }
    }
}

impl CacheEntry {
    pub fn is_expired(&self) -> bool {
        Utc::now()
            .signed_duration_since(self.created_at)
            .to_std()
            .map(|d| d > self.ttl)
            .unwrap_or(true)
    }

    fn new(value: String, ttl: Duration) -> Self {
        Self {
            value,
            created_at: Utc::now(),
            ttl,
            hit_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "AiCacheSerialized", into = "AiCacheSerialized")]
pub struct AiCache {
    entries: Arc<RwLock<FxHashMap<String, CacheEntry>>>,
    max_entries: usize,
    default_ttl: Duration,
    persist_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiCacheSerialized {
    entries: FxHashMap<String, CacheEntrySer>,
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
        let entries: FxHashMap<String, CacheEntry> = serialized
            .entries
            .into_iter()
            .map(|(k, v)| {
                let ttl = Duration::from_nanos(v.ttl_nanos);
                (
                    k,
                    CacheEntry {
                        value: v.value,
                        created_at: v.created_at,
                        ttl,
                        hit_count: v.hit_count,
                    },
                )
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
        AiCacheSerialized {
            entries: FxHashMap::default(),
            max_entries: cache.max_entries,
            default_ttl_nanos: cache.default_ttl.as_nanos() as u64,
        }
    }
}

impl AiCache {
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(FxHashMap::default())),
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
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(key) {
            if !entry.is_expired() {
                entry.hit_count += 1;
                return Some(entry.value.clone());
            }
        }
        None
    }

    pub async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) {
        let should_persist;
        {
            let mut entries = self.entries.write().await;
            while entries.len() > self.max_entries {
                self.evict_expired(&mut entries);
                if entries.len() > self.max_entries {
                    if let Some((oldest_key, _)) = entries
                        .iter()
                        .min_by_key(|(_, v)| v.created_at)
                        .map(|(k, v)| (k.clone(), v.clone()))
                    {
                        entries.remove(&oldest_key);
                    }
                }
            }
            entries.insert(
                key.to_string(),
                CacheEntry::new(value.to_string(), ttl.unwrap_or(self.default_ttl)),
            );
            should_persist = self.persist_path.is_some();
        }
        if should_persist {
            self.persist().await;
        }
    }

    pub async fn remove(&self, key: &str) {
        let should_persist;
        {
            let mut entries = self.entries.write().await;
            entries.remove(key);
            should_persist = self.persist_path.is_some();
        }
        if should_persist {
            self.persist().await;
        }
    }

    pub async fn clear(&self) {
        {
            let mut entries = self.entries.write().await;
            entries.clear();
        }
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

    fn evict_expired(&self, entries: &mut FxHashMap<String, CacheEntry>) {
        entries.retain(|_, v| !v.is_expired());
    }

    pub async fn cleanup(&self) {
        {
            let mut entries = self.entries.write().await;
            self.evict_expired(&mut entries);
        }
        self.persist().await;
    }

    async fn persist(&self) {
        if let Some(ref path) = self.persist_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let entries = self.entries.read().await;
            let serialized_entries: FxHashMap<String, CacheEntrySer> = entries
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        CacheEntrySer {
                            value: v.value.clone(),
                            created_at: v.created_at,
                            ttl_nanos: v.ttl.as_nanos() as u64,
                            hit_count: v.hit_count,
                        },
                    )
                })
                .collect();

            let serialized = AiCacheSerialized {
                entries: serialized_entries,
                max_entries: self.max_entries,
                default_ttl_nanos: self.default_ttl.as_nanos() as u64,
            };

            if let Ok(json) = serde_json::to_string(&serialized) {
                if let Err(e) = std::fs::write(path, json) {
                    tracing::warn!("Failed to persist AI cache to {:?}: {}", path, e);
                }
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

/// Cache key builder for AI cache entries.
/// Uses null byte (`\x00`) separators to prevent collisions when input contains colons.
pub struct CacheKeyBuilder;

impl CacheKeyBuilder {
    pub fn for_payload_suggestion(vuln_type: &str, context: &str) -> String {
        format!("payload{}\x00{}\x00{}", vuln_type, context)
    }

    pub fn for_waf_bypass(waf: &str, blocked_payload: &str) -> String {
        format!("waf_bypass{}\x00{}\x00{}", waf, blocked_payload)
    }

    pub fn for_finding_analysis(findings_hash: &str) -> String {
        format!("analysis{}\x00{}", findings_hash)
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
        cache.get("key1").await;
        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_hits, 2);
    }

    #[tokio::test]
    async fn test_cache_hit_count_incremented() {
        let cache = AiCache::new(10, Duration::from_secs(60));
        cache.set("key1", "value1", None).await;
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));
        let stats = cache.stats().await;
        assert_eq!(stats.total_hits, 3);
    }

    #[test]
    fn test_cache_key_builder_no_collision_with_colons() {
        let key1 = CacheKeyBuilder::for_payload_suggestion("sql:inject", "context:with:colons");
        let key2 = CacheKeyBuilder::for_payload_suggestion("sql", "inject:context:with:colons");
        assert_ne!(key1, key2, "Keys with colons in values should not collide");
    }
}
