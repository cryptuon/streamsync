//! Caching system for IDL data

use crate::types::GeneratedIDL;

use solana_sdk::pubkey::Pubkey;
use dashmap::DashMap;
use std::time::{Instant, Duration};

/// High-performance cache for IDL data
pub struct IDLCache {
    // Main cache storage
    cache: DashMap<Pubkey, CachedIDL>,

    // Cache configuration
    max_entries: usize,
    default_ttl: Duration,
}

#[derive(Debug, Clone)]
struct CachedIDL {
    idl: GeneratedIDL,
    cached_at: Instant,
    access_count: u64,
    last_access: Instant,
}

impl IDLCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: DashMap::new(),
            max_entries,
            default_ttl: Duration::from_secs(300), // 5 minutes default TTL
        }
    }

    /// Store IDL in cache
    pub async fn store_idl(&self, program_id: &Pubkey, idl: &GeneratedIDL) {
        let cached_idl = CachedIDL {
            idl: idl.clone(),
            cached_at: Instant::now(),
            access_count: 0,
            last_access: Instant::now(),
        };

        // Check if we need to evict old entries
        if self.cache.len() >= self.max_entries {
            self.evict_old_entries().await;
        }

        self.cache.insert(*program_id, cached_idl);
    }

    /// Get IDL from cache
    pub async fn get_idl(&self, program_id: &Pubkey) -> Option<GeneratedIDL> {
        if let Some(mut cached) = self.cache.get_mut(program_id) {
            // Check if cache entry is still valid
            if self.is_cache_entry_valid(&cached) {
                cached.access_count += 1;
                cached.last_access = Instant::now();
                Some(cached.idl.clone())
            } else {
                // Remove expired entry
                drop(cached);
                self.cache.remove(program_id);
                None
            }
        } else {
            None
        }
    }

    /// Remove IDL from cache
    pub async fn remove_idl(&self, program_id: &Pubkey) -> bool {
        self.cache.remove(program_id).is_some()
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let total_entries = self.cache.len();
        let mut total_access_count = 0;

        for entry in self.cache.iter() {
            total_access_count += entry.access_count;
        }

        CacheStats {
            total_entries,
            max_entries: self.max_entries,
            total_access_count,
            hit_rate: 0.0, // Would need to track hits/misses for accurate calculation
        }
    }

    /// Check if cache entry is still valid
    fn is_cache_entry_valid(&self, cached: &CachedIDL) -> bool {
        cached.cached_at.elapsed() < self.default_ttl
    }

    /// Evict old entries when cache is full
    async fn evict_old_entries(&self) {
        // Simple LRU eviction - remove 25% of oldest entries
        let mut entries_to_remove = Vec::new();
        let eviction_count = self.max_entries / 4;

        // Collect entries with their last access times
        let mut access_times: Vec<(Pubkey, Instant)> = self.cache.iter()
            .map(|entry| (*entry.key(), entry.last_access))
            .collect();

        // Sort by last access time (oldest first)
        access_times.sort_by_key(|(_, last_access)| *last_access);

        // Take the oldest entries for eviction
        for (program_id, _) in access_times.into_iter().take(eviction_count) {
            entries_to_remove.push(program_id);
        }

        // Remove the selected entries
        for program_id in entries_to_remove {
            self.cache.remove(&program_id);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub max_entries: usize,
    pub total_access_count: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{IDLDefinition, IDLConfidence, NetworkConsensus, IDLMetadata, ConsensusMethod};
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_idl(program_id: &Pubkey) -> GeneratedIDL {
        GeneratedIDL {
            program_id: *program_id,
            idl: IDLDefinition {
                version: "0.1.0".to_string(),
                name: "test_program".to_string(),
                instructions: vec![],
                accounts: vec![],
                types: vec![],
                events: vec![],
                errors: vec![],
                constants: vec![],
            },
            confidence: IDLConfidence {
                overall_confidence: 0.9,
                instruction_confidence: HashMap::new(),
                account_confidence: HashMap::new(),
                type_confidence: HashMap::new(),
                confidence_factors: crate::types::ConfidenceFactors {
                    sample_size: 1000,
                    observation_period: chrono::Duration::days(7),
                    pattern_consistency: 0.9,
                    cross_validation_score: 0.8,
                    expert_validation_score: None,
                },
            },
            network_consensus: NetworkConsensus {
                agreement_score: 0.9,
                participating_nodes: 5,
                consensus_timestamp: Utc::now(),
                disagreement_areas: vec![],
                consensus_method: ConsensusMethod::Majority,
            },
            metadata: IDLMetadata {
                generation_timestamp: Utc::now(),
                generator_version: "0.1.0".to_string(),
                source_transactions: 1000,
                analysis_period: chrono::Duration::days(7),
                update_history: vec![],
                validation_results: vec![],
            },
        }
    }

    #[tokio::test]
    async fn test_cache_creation() {
        let cache = IDLCache::new(100);
        assert_eq!(cache.max_entries, 100);
        assert_eq!(cache.cache.len(), 0);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_idl() {
        let cache = IDLCache::new(100);
        let program_id = Pubkey::new_unique();
        let idl = create_test_idl(&program_id);

        // Should be empty initially
        assert!(cache.get_idl(&program_id).await.is_none());

        // Store IDL
        cache.store_idl(&program_id, &idl).await;

        // Should be retrievable now
        let retrieved = cache.get_idl(&program_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().program_id, program_id);
    }

    #[tokio::test]
    async fn test_cache_removal() {
        let cache = IDLCache::new(100);
        let program_id = Pubkey::new_unique();
        let idl = create_test_idl(&program_id);

        // Store and verify
        cache.store_idl(&program_id, &idl).await;
        assert!(cache.get_idl(&program_id).await.is_some());

        // Remove and verify
        let removed = cache.remove_idl(&program_id).await;
        assert!(removed);
        assert!(cache.get_idl(&program_id).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = IDLCache::new(100);
        let program_id1 = Pubkey::new_unique();
        let program_id2 = Pubkey::new_unique();
        let idl1 = create_test_idl(&program_id1);
        let idl2 = create_test_idl(&program_id2);

        // Store multiple IDLs
        cache.store_idl(&program_id1, &idl1).await;
        cache.store_idl(&program_id2, &idl2).await;

        assert_eq!(cache.cache.len(), 2);

        // Clear and verify
        cache.clear().await;
        assert_eq!(cache.cache.len(), 0);
        assert!(cache.get_idl(&program_id1).await.is_none());
        assert!(cache.get_idl(&program_id2).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = IDLCache::new(100);
        let program_id = Pubkey::new_unique();
        let idl = create_test_idl(&program_id);

        // Check initial stats
        let stats = cache.get_stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.max_entries, 100);

        // Store IDL and check stats
        cache.store_idl(&program_id, &idl).await;
        let stats = cache.get_stats();
        assert_eq!(stats.total_entries, 1);

        // Access IDL and check stats
        cache.get_idl(&program_id).await;
        let stats = cache.get_stats();
        assert!(stats.total_access_count > 0);
    }
}