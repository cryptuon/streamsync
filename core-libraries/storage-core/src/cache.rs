use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::CacheConfig;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub data: serde_json::Value,
    pub inserted_at: Instant,
    pub access_count: u64,
    pub last_accessed: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f64,
    pub total_size_bytes: u64,
    pub evictions: u64,
    pub oldest_entry_age_seconds: u64,
}

pub struct StorageCache {
    cache: Arc<DashMap<String, CacheEntry>>,
    config: CacheConfig,
    stats: Arc<RwLock<CacheStats>>,
}

impl StorageCache {
    pub fn new(config: CacheConfig) -> Self {
        info!("Initializing storage cache with {}MB capacity", config.size_mb);

        Self {
            cache: Arc::new(DashMap::new()),
            config,
            stats: Arc::new(RwLock::new(CacheStats {
                total_entries: 0,
                cache_hits: 0,
                cache_misses: 0,
                hit_rate: 0.0,
                total_size_bytes: 0,
                evictions: 0,
                oldest_entry_age_seconds: 0,
            })),
        }
    }

    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        if !self.config.enabled {
            return None;
        }

        if let Some(mut entry) = self.cache.get_mut(key) {
            // Check if entry has expired
            if entry.inserted_at.elapsed() > Duration::from_secs(self.config.ttl_seconds) {
                drop(entry);
                self.cache.remove(key);
                self.increment_miss().await;
                return None;
            }

            // Update access statistics
            entry.access_count += 1;
            entry.last_accessed = Instant::now();

            let data = entry.data.clone();
            drop(entry);

            self.increment_hit().await;
            debug!("Cache hit for key: {}", key);
            Some(data)
        } else {
            self.increment_miss().await;
            debug!("Cache miss for key: {}", key);
            None
        }
    }

    pub async fn put(&self, key: String, value: serde_json::Value) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check cache size limits
        if self.cache.len() >= self.config.max_items {
            self.evict_oldest().await?;
        }

        let entry = CacheEntry {
            data: value,
            inserted_at: Instant::now(),
            access_count: 1,
            last_accessed: Instant::now(),
        };

        self.cache.insert(key.clone(), entry);
        self.increment_entry_count().await;

        debug!("Cached entry for key: {}", key);
        Ok(())
    }

    pub async fn remove(&self, key: &str) -> Option<serde_json::Value> {
        if let Some((_, entry)) = self.cache.remove(key) {
            self.decrement_entry_count().await;
            debug!("Removed cache entry for key: {}", key);
            Some(entry.data)
        } else {
            None
        }
    }

    pub async fn clear(&self) -> Result<()> {
        let count = self.cache.len();
        self.cache.clear();

        let mut stats = self.stats.write().await;
        stats.total_entries = 0;
        stats.total_size_bytes = 0;

        info!("Cleared {} entries from cache", count);
        Ok(())
    }

    pub async fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        let mut stats_copy = stats.clone();

        // Calculate current hit rate
        let total_requests = stats_copy.cache_hits + stats_copy.cache_misses;
        stats_copy.hit_rate = if total_requests > 0 {
            stats_copy.cache_hits as f64 / total_requests as f64 * 100.0
        } else {
            0.0
        };

        // Calculate oldest entry age
        stats_copy.oldest_entry_age_seconds = self.get_oldest_entry_age().await;
        stats_copy.total_entries = self.cache.len() as u64;
        stats_copy.total_size_bytes = self.estimate_cache_size().await;

        stats_copy
    }

    pub async fn cleanup_expired(&self) -> Result<u64> {
        let ttl = Duration::from_secs(self.config.ttl_seconds);
        let mut removed_count = 0;

        let expired_keys: Vec<String> = self.cache
            .iter()
            .filter_map(|entry| {
                if entry.value().inserted_at.elapsed() > ttl {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();

        for key in expired_keys {
            if self.cache.remove(&key).is_some() {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            self.update_eviction_count(removed_count).await;
            debug!("Cleaned up {} expired cache entries", removed_count);
        }

        Ok(removed_count)
    }

    async fn evict_oldest(&self) -> Result<()> {
        let oldest_key = self.cache
            .iter()
            .min_by_key(|entry| entry.value().inserted_at)
            .map(|entry| entry.key().clone());

        if let Some(key) = oldest_key {
            self.cache.remove(&key);
            self.increment_eviction_count().await;
            debug!("Evicted oldest cache entry: {}", key);
        }

        Ok(())
    }

    async fn increment_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.cache_hits += 1;
    }

    async fn increment_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.cache_misses += 1;
    }

    async fn increment_entry_count(&self) {
        let mut stats = self.stats.write().await;
        stats.total_entries = self.cache.len() as u64;
    }

    async fn decrement_entry_count(&self) {
        let mut stats = self.stats.write().await;
        stats.total_entries = self.cache.len() as u64;
    }

    async fn increment_eviction_count(&self) {
        let mut stats = self.stats.write().await;
        stats.evictions += 1;
    }

    async fn update_eviction_count(&self, count: u64) {
        let mut stats = self.stats.write().await;
        stats.evictions += count;
    }

    async fn get_oldest_entry_age(&self) -> u64 {
        self.cache
            .iter()
            .map(|entry| entry.value().inserted_at.elapsed().as_secs())
            .max()
            .unwrap_or(0)
    }

    async fn estimate_cache_size(&self) -> u64 {
        // Very rough estimate - in a real implementation, you'd want more accurate size calculation
        self.cache.len() as u64 * 1024 // Assume 1KB per entry on average
    }

    pub async fn start_cleanup_task(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let cache_clone = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Clean up every 5 minutes

            loop {
                interval.tick().await;
                if let Err(e) = cache_clone.cleanup_expired().await {
                    warn!("Cache cleanup failed: {}", e);
                }
            }
        });

        info!("Started cache cleanup task");
        Ok(())
    }
}

impl Clone for StorageCache {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
        }
    }
}