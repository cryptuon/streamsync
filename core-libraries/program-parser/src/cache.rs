//! Parsing cache for performance optimization

use crate::types::ParseResult;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// High-performance cache for parsed results
pub struct ParseCache {
    cache: DashMap<String, CacheEntry>,
    max_size: usize,
    default_ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    result: ParseResult,
    created_at: Instant,
    ttl: Duration,
    access_count: u64,
}

impl ParseCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: DashMap::new(),
            max_size,
            default_ttl: Duration::from_secs(300), // 5 minutes default
        }
    }

    pub fn get(&self, key: &str) -> Option<ParseResult> {
        if let Some(mut entry) = self.cache.get_mut(key) {
            // Check if entry is still valid
            if entry.created_at.elapsed() < entry.ttl {
                entry.access_count += 1;
                Some(entry.result.clone())
            } else {
                // Entry expired, remove it
                drop(entry);
                self.cache.remove(key);
                None
            }
        } else {
            None
        }
    }

    pub fn insert(&self, key: String, result: ParseResult) {
        self.insert_with_ttl(key, result, self.default_ttl);
    }

    pub fn insert_with_ttl(&self, key: String, result: ParseResult, ttl: Duration) {
        // If cache is full, remove least recently used entries
        if self.cache.len() >= self.max_size {
            self.evict_lru();
        }

        let entry = CacheEntry {
            result,
            created_at: Instant::now(),
            ttl,
            access_count: 0,
        };

        self.cache.insert(key, entry);
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }

    pub fn stats(&self) -> CacheStats {
        let total_entries = self.cache.len();
        let mut expired_entries = 0;
        let mut total_access_count = 0;

        for entry in self.cache.iter() {
            if entry.created_at.elapsed() >= entry.ttl {
                expired_entries += 1;
            }
            total_access_count += entry.access_count;
        }

        CacheStats {
            total_entries,
            expired_entries,
            hit_rate: if total_access_count > 0 {
                (total_access_count as f64) / (total_entries as f64)
            } else {
                0.0
            },
            memory_usage_mb: (total_entries * std::mem::size_of::<CacheEntry>()) / (1024 * 1024),
        }
    }

    fn evict_lru(&self) {
        // Find entry with lowest access count and oldest timestamp
        let mut oldest_key: Option<String> = None;
        let mut oldest_time = Instant::now();
        let mut lowest_access_count = u64::MAX;

        for entry in self.cache.iter() {
            if entry.access_count < lowest_access_count ||
               (entry.access_count == lowest_access_count && entry.created_at < oldest_time) {
                oldest_key = Some(entry.key().clone());
                oldest_time = entry.created_at;
                lowest_access_count = entry.access_count;
            }
        }

        if let Some(key) = oldest_key {
            self.cache.remove(&key);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub hit_rate: f64,
    pub memory_usage_mb: usize,
}