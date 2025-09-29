//! Caching system for reconstruction results and patterns

use crate::{
    types::{TruncatedData, CompressionParams, ReconstructedAccount},
    error::ReconstructionResult,
};

use dashmap::DashMap;
use lru::LruCache;
use std::sync::{Arc, RwLock};
use std::time::{Instant, Duration};
use blake3::Hasher;

/// High-performance cache for reconstruction results
pub struct ReconstructionCache {
    // Fast lookup cache for exact matches
    exact_cache: DashMap<CacheKey, CachedReconstruction>,

    // LRU cache for pattern-based lookups
    pattern_cache: Arc<RwLock<LruCache<PatternKey, PatternCacheEntry>>>,

    // Statistics for cache performance
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub data_hash: [u8; 32],
    pub compression_hash: [u8; 32],
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct PatternKey {
    pattern_signature: u64,
    data_size: usize,
    compression_type: String,
}

#[derive(Debug, Clone)]
struct CachedReconstruction {
    result: ReconstructedAccount,
    cache_time: Instant,
    access_count: u64,
    last_access: Instant,
}

#[derive(Debug, Clone)]
struct PatternCacheEntry {
    pattern_data: Vec<u8>,
    reconstruction_template: Vec<u8>,
    success_rate: f64,
    usage_count: u64,
}

#[derive(Debug, Clone, Default)]
struct CacheStats {
    total_requests: u64,
    cache_hits: u64,
    cache_misses: u64,
    evictions: u64,
    pattern_matches: u64,
}

impl ReconstructionCache {
    /// Create a new reconstruction cache
    pub fn new(capacity: usize) -> Self {
        Self {
            exact_cache: DashMap::new(),
            pattern_cache: Arc::new(RwLock::new(LruCache::new(std::num::NonZero::new(capacity).unwrap()))),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get cached reconstruction if available
    pub async fn get_reconstruction(
        &self,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
    ) -> Option<ReconstructedAccount> {

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_requests += 1;
        }

        // Try exact cache first
        let cache_key = self.compute_cache_key(truncated_data, compression_params);

        if let Some(mut cached) = self.exact_cache.get_mut(&cache_key) {
            // Check if cache entry is still valid
            if self.is_cache_entry_valid(&cached, &cache_key) {
                cached.access_count += 1;
                cached.last_access = Instant::now();

                // Update stats
                {
                    let mut stats = self.stats.write().unwrap();
                    stats.cache_hits += 1;
                }

                return Some(cached.result.clone());
            } else {
                // Remove expired entry
                drop(cached);
                self.exact_cache.remove(&cache_key);
            }
        }

        // Try pattern-based cache
        if let Some(pattern_result) = self.try_pattern_cache(truncated_data).await {
            {
                let mut stats = self.stats.write().unwrap();
                stats.pattern_matches += 1;
            }
            return Some(pattern_result);
        }

        // Cache miss
        {
            let mut stats = self.stats.write().unwrap();
            stats.cache_misses += 1;
        }

        None
    }

    /// Store reconstruction result in cache
    pub async fn store_reconstruction(
        &self,
        truncated_data: &TruncatedData,
        compression_params: &CompressionParams,
        result: &ReconstructedAccount,
    ) {
        let cache_key = self.compute_cache_key(truncated_data, compression_params);

        let cached_entry = CachedReconstruction {
            result: result.clone(),
            cache_time: Instant::now(),
            access_count: 1,
            last_access: Instant::now(),
        };

        self.exact_cache.insert(cache_key, cached_entry);

        // Also store in pattern cache if it's a pattern-based reconstruction
        if let crate::types::ReconstructionMethod::PatternMatching { pattern_id } = &result.reconstruction_method {
            self.store_pattern_cache_entry(truncated_data, result, pattern_id).await;
        }
    }

    /// Try to find a match in the pattern cache
    async fn try_pattern_cache(&self, truncated_data: &TruncatedData) -> Option<ReconstructedAccount> {
        let pattern_key = self.compute_pattern_key(truncated_data);

        let pattern_cache = self.pattern_cache.read().unwrap();
        if let Some(pattern_entry) = pattern_cache.peek(&pattern_key) {
            // Try to apply the cached pattern
            if let Ok(reconstructed) = self.apply_cached_pattern(truncated_data, pattern_entry) {
                return Some(reconstructed);
            }
        }

        None
    }

    /// Store a pattern-based cache entry
    async fn store_pattern_cache_entry(
        &self,
        truncated_data: &TruncatedData,
        result: &ReconstructedAccount,
        _pattern_id: &str,
    ) {
        let pattern_key = self.compute_pattern_key(truncated_data);

        let pattern_entry = PatternCacheEntry {
            pattern_data: truncated_data.data.clone(),
            reconstruction_template: result.account_data.clone(),
            success_rate: result.confidence_score,
            usage_count: 1,
        };

        let mut pattern_cache = self.pattern_cache.write().unwrap();
        pattern_cache.put(pattern_key, pattern_entry);
    }

    /// Apply a cached pattern to reconstruct data
    fn apply_cached_pattern(
        &self,
        _truncated_data: &TruncatedData,
        pattern_entry: &PatternCacheEntry,
    ) -> ReconstructionResult<ReconstructedAccount> {

        // For this skeleton, use a simple pattern application
        // Real implementation would have sophisticated pattern matching

        let confidence = pattern_entry.success_rate * 0.9; // Slightly lower confidence for cached patterns

        Ok(ReconstructedAccount {
            account_data: pattern_entry.reconstruction_template.clone(),
            reconstruction_method: crate::types::ReconstructionMethod::PatternMatching {
                pattern_id: "cached_pattern".to_string(),
            },
            confidence_score: confidence,
            verification_proof: None,
            reconstruction_time: Duration::from_micros(100), // Very fast for cached patterns
            cache_hint: crate::types::CacheHint {
                cache_key: "pattern_cache".to_string(),
                ttl: Duration::from_secs(300),
                pattern_category: Some("cached".to_string()),
                reuse_probability: 0.95,
            },
        })
    }

    /// Compute cache key for exact matching
    pub fn compute_cache_key(&self, truncated_data: &TruncatedData, compression_params: &CompressionParams) -> CacheKey {
        let data_hash = blake3::hash(&truncated_data.data);

        // Hash compression parameters
        let mut compression_hasher = Hasher::new();
        compression_hasher.update(&compression_params.root_hash);
        compression_hasher.update(&compression_params.merkle_tree_height.to_le_bytes());
        compression_hasher.update(&compression_params.leaf_count.to_le_bytes());
        let compression_hash = compression_hasher.finalize();

        CacheKey {
            data_hash: *data_hash.as_bytes(),
            compression_hash: *compression_hash.as_bytes(),
        }
    }

    /// Compute pattern key for pattern-based matching
    fn compute_pattern_key(&self, truncated_data: &TruncatedData) -> PatternKey {
        // Compute pattern signature from data characteristics
        let mut signature_hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};

        // Hash first and last few bytes, data size, and compression type
        if !truncated_data.data.is_empty() {
            truncated_data.data[0].hash(&mut signature_hasher);
            if truncated_data.data.len() > 1 {
                truncated_data.data[truncated_data.data.len() - 1].hash(&mut signature_hasher);
            }
        }
        truncated_data.data.len().hash(&mut signature_hasher);

        PatternKey {
            pattern_signature: signature_hasher.finish(),
            data_size: truncated_data.data.len(),
            compression_type: format!("{:?}", truncated_data.metadata.compression_type),
        }
    }

    /// Check if a cache entry is still valid
    fn is_cache_entry_valid(&self, cached: &CachedReconstruction, _cache_key: &CacheKey) -> bool {
        // Check TTL from cache hint
        let ttl = cached.result.cache_hint.ttl;
        cached.cache_time.elapsed() < ttl
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }

    /// Calculate cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let stats = self.stats.read().unwrap();
        if stats.total_requests == 0 {
            0.0
        } else {
            stats.cache_hits as f64 / stats.total_requests as f64
        }
    }

    /// Clear expired entries from cache
    pub async fn cleanup_expired(&self) {
        let _now = Instant::now();
        let mut expired_keys = Vec::new();

        // Collect expired keys
        for entry in self.exact_cache.iter() {
            if !self.is_cache_entry_valid(entry.value(), entry.key()) {
                expired_keys.push(entry.key().clone());
            }
        }

        // Remove expired entries
        for key in expired_keys {
            self.exact_cache.remove(&key);
            let mut stats = self.stats.write().unwrap();
            stats.evictions += 1;
        }
    }

    /// Get cache size information
    pub fn cache_info(&self) -> CacheInfo {
        let exact_count = self.exact_cache.len();
        let pattern_count = self.pattern_cache.read().unwrap().len();
        let stats = self.get_stats();

        CacheInfo {
            exact_cache_entries: exact_count,
            pattern_cache_entries: pattern_count,
            total_requests: stats.total_requests,
            cache_hits: stats.cache_hits,
            hit_rate: self.hit_rate(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheInfo {
    pub exact_cache_entries: usize,
    pub pattern_cache_entries: usize,
    pub total_requests: u64,
    pub cache_hits: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TruncationMetadata, CompressionType};
    use solana_sdk::pubkey::Pubkey;
    use std::time::SystemTime;

    fn create_test_truncated_data() -> TruncatedData {
        TruncatedData {
            data: vec![1, 2, 3, 4, 5],
            original_size_hint: Some(1000),
            truncation_point: 5,
            metadata: TruncationMetadata {
                slot: 12345,
                account: Pubkey::new_unique(),
                program_id: Pubkey::new_unique(),
                compression_type: CompressionType::Standard,
                truncation_timestamp: SystemTime::now(),
            },
        }
    }

    fn create_test_compression_params() -> CompressionParams {
        CompressionParams {
            compression_type: CompressionType::Standard,
            merkle_tree_height: 20,
            leaf_count: 1000,
            root_hash: [1u8; 32],
            compression_program: Pubkey::new_unique(),
            additional_params: std::collections::HashMap::new(),
        }
    }

    fn create_test_reconstructed_account() -> ReconstructedAccount {
        ReconstructedAccount {
            account_data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            reconstruction_method: crate::types::ReconstructionMethod::MerkleTreeReconstruction,
            confidence_score: 0.95,
            verification_proof: None,
            reconstruction_time: Duration::from_millis(10),
            cache_hint: crate::types::CacheHint {
                cache_key: "test".to_string(),
                ttl: Duration::from_secs(300),
                pattern_category: None,
                reuse_probability: 0.8,
            },
        }
    }

    #[tokio::test]
    async fn test_cache_creation() {
        let cache = ReconstructionCache::new(1000);
        let stats = cache.get_stats();

        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[tokio::test]
    async fn test_cache_store_and_retrieve() {
        let cache = ReconstructionCache::new(1000);
        let truncated_data = create_test_truncated_data();
        let compression_params = create_test_compression_params();
        let reconstructed = create_test_reconstructed_account();

        // Should be cache miss initially
        let result = cache.get_reconstruction(&truncated_data, &compression_params).await;
        assert!(result.is_none());

        // Store result
        cache.store_reconstruction(&truncated_data, &compression_params, &reconstructed).await;

        // Should be cache hit now
        let result = cache.get_reconstruction(&truncated_data, &compression_params).await;
        assert!(result.is_some());

        let cached_result = result.unwrap();
        assert_eq!(cached_result.account_data, reconstructed.account_data);
        assert_eq!(cached_result.confidence_score, reconstructed.confidence_score);

        // Check stats
        let stats = cache.get_stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(cache.hit_rate(), 0.5);
    }

    #[test]
    fn test_cache_key_computation() {
        let cache = ReconstructionCache::new(1000);
        let truncated_data = create_test_truncated_data();
        let compression_params = create_test_compression_params();

        let key1 = cache.compute_cache_key(&truncated_data, &compression_params);
        let key2 = cache.compute_cache_key(&truncated_data, &compression_params);

        assert_eq!(key1, key2); // Should be deterministic

        // Different data should produce different keys
        let mut different_data = truncated_data.clone();
        different_data.data[0] = 99;

        let key3 = cache.compute_cache_key(&different_data, &compression_params);
        assert_ne!(key1, key3);
    }
}