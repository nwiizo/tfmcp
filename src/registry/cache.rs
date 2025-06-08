use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
struct CachedItem<T> {
    value: T,
    inserted_at: Instant,
}

/// Simple in-memory cache with TTL support
pub struct SimpleCache<T: Clone> {
    storage: Arc<RwLock<HashMap<String, CachedItem<T>>>>,
    ttl: Duration,
}

impl<T: Clone> SimpleCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Get value from cache if it exists and hasn't expired
    pub async fn get(&self, key: &str) -> Option<T> {
        let storage = self.storage.read().await;

        if let Some(item) = storage.get(key) {
            if item.inserted_at.elapsed() < self.ttl {
                return Some(item.value.clone());
            }
        }

        None
    }

    /// Set value in cache
    pub async fn set(&self, key: String, value: T) {
        let mut storage = self.storage.write().await;
        storage.insert(
            key,
            CachedItem {
                value,
                inserted_at: Instant::now(),
            },
        );
    }

    /// Remove expired entries from cache
    #[allow(dead_code)]
    pub async fn cleanup_expired(&self) {
        let mut storage = self.storage.write().await;
        storage.retain(|_, item| item.inserted_at.elapsed() < self.ttl);
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub async fn stats(&self) -> CacheStats {
        let storage = self.storage.read().await;
        let total_entries = storage.len();
        let valid_entries = storage
            .values()
            .filter(|item| item.inserted_at.elapsed() < self.ttl)
            .count();
        let expired_entries = total_entries - valid_entries;

        CacheStats {
            total_entries,
            valid_entries,
            expired_entries,
        }
    }

    /// Clear all cache entries
    #[allow(dead_code)]
    pub async fn clear(&self) {
        let mut storage = self.storage.write().await;
        storage.clear();
    }

    /// Get cache hit rate (requires tracking hits and misses)
    #[allow(dead_code)]
    pub async fn size(&self) -> usize {
        let storage = self.storage.read().await;
        storage.len()
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

impl<T: Clone> Default for SimpleCache<T> {
    fn default() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minutes default TTL
    }
}

/// Specialized cache for documentation content
pub type DocumentationCache = SimpleCache<String>;

/// Specialized cache for provider information
pub type ProvidersCache = SimpleCache<String>;

/// Cache manager that handles multiple cache types
pub struct CacheManager {
    pub documentation_cache: DocumentationCache,
    pub providers_cache: ProvidersCache,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            documentation_cache: SimpleCache::new(Duration::from_secs(1800)), // 30 minutes for docs
            providers_cache: SimpleCache::new(Duration::from_secs(600)), // 10 minutes for providers
        }
    }

    /// Get comprehensive cache statistics
    #[allow(dead_code)]
    pub async fn global_stats(&self) -> HashMap<String, CacheStats> {
        let mut stats = HashMap::new();
        stats.insert(
            "documentation".to_string(),
            self.documentation_cache.stats().await,
        );
        stats.insert("providers".to_string(), self.providers_cache.stats().await);
        stats
    }

    /// Cleanup expired entries from all caches
    #[allow(dead_code)]
    pub async fn cleanup_all(&self) {
        self.documentation_cache.cleanup_expired().await;
        self.providers_cache.cleanup_expired().await;
    }

    /// Clear all caches
    #[allow(dead_code)]
    pub async fn clear_all(&self) {
        self.documentation_cache.clear().await;
        self.providers_cache.clear().await;
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = SimpleCache::new(Duration::from_millis(100));

        // Test set and get
        cache.set("key1".to_string(), "value1".to_string()).await;
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));

        // Test non-existent key
        assert_eq!(cache.get("nonexistent").await, None);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = SimpleCache::new(Duration::from_millis(50));

        cache.set("key1".to_string(), "value1".to_string()).await;
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));

        // Wait for expiration
        sleep(Duration::from_millis(60)).await;
        assert_eq!(cache.get("key1").await, None);
    }
}
