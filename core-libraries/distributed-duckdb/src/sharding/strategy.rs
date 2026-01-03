//! Data Distribution Strategies for StreamSync Sharding
//!
//! This module implements various data distribution strategies for optimal
//! data placement across the decentralized network.

use super::{DistributionStrategy, HashFunction, HashRingEntry, KeyRange, NodeCapacity};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use sha2::{Sha256, Digest};

/// Sharding strategy implementation
pub struct ShardingStrategy {
    strategy: DistributionStrategy,
    node_ring: Vec<HashRingEntry>,
    node_capacities: HashMap<Uuid, NodeCapacity>,
}

/// Placement result for a data shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementResult {
    pub primary_node: Uuid,
    pub assigned_nodes: Vec<Uuid>,
    pub placement_score: f64,
    pub load_distribution: HashMap<Uuid, f64>,
}

impl ShardingStrategy {
    /// Create a new sharding strategy
    pub fn new(strategy: DistributionStrategy) -> Self {
        Self {
            strategy,
            node_ring: Vec::new(),
            node_capacities: HashMap::new(),
        }
    }

    /// Update node capacities
    pub fn update_node_capacities(&mut self, capacities: HashMap<Uuid, NodeCapacity>) {
        self.node_capacities = capacities;

        // Rebuild hash ring for consistent hashing
        if let DistributionStrategy::ConsistentHash { virtual_nodes, .. } = &self.strategy {
            let virtual_nodes = *virtual_nodes;
            self.rebuild_hash_ring(virtual_nodes);
        }
    }

    /// Add a new node to the strategy
    pub fn add_node(&mut self, node_id: Uuid, capacity: NodeCapacity) -> Result<()> {
        self.node_capacities.insert(node_id, capacity);

        match &mut self.strategy {
            DistributionStrategy::ConsistentHash { virtual_nodes, hash_ring } => {
                let virtual_nodes_count = *virtual_nodes;
                // Generate new ring entries inline to avoid borrow issues
                for i in 0..virtual_nodes_count {
                    let key = format!("{}:{}", node_id, i);
                    let hash = {
                        use sha2::{Sha256, Digest};
                        let mut hasher = Sha256::new();
                        hasher.update(key.as_bytes());
                        let result = hasher.finalize();
                        u64::from_be_bytes([
                            result[0], result[1], result[2], result[3],
                            result[4], result[5], result[6], result[7]
                        ])
                    };
                    hash_ring.push(HashRingEntry {
                        hash,
                        node_id,
                        virtual_node_id: i,
                    });
                }
            }
            DistributionStrategy::Directory { mapping } => {
                // Nodes can be added to directory dynamically
                mapping.insert(node_id.to_string(), vec![node_id]);
            }
            _ => {
                // Other strategies may require full rebuild
            }
        }

        Ok(())
    }

    /// Remove a node from the strategy
    pub fn remove_node(&mut self, node_id: Uuid) -> Result<()> {
        self.node_capacities.remove(&node_id);

        match &mut self.strategy {
            DistributionStrategy::ConsistentHash { hash_ring, .. } => {
                hash_ring.retain(|entry| entry.node_id != node_id);
            }
            DistributionStrategy::Directory { mapping } => {
                mapping.retain(|_, nodes| {
                    nodes.retain(|&n| n != node_id);
                    !nodes.is_empty()
                });
            }
            _ => {
                // Other strategies may require full rebuild
            }
        }

        Ok(())
    }

    /// Determine optimal placement for a key range
    pub fn determine_placement(&self, key_range: &KeyRange, data_size: u64, replication_factor: u32) -> Result<PlacementResult> {
        match &self.strategy {
            DistributionStrategy::Hash { hash_function, num_buckets } => {
                self.hash_based_placement(key_range, data_size, replication_factor, hash_function, *num_buckets)
            }
            DistributionStrategy::Range { partition_key, ranges } => {
                self.range_based_placement(key_range, data_size, replication_factor, partition_key, ranges)
            }
            DistributionStrategy::Directory { mapping } => {
                self.directory_based_placement(key_range, data_size, replication_factor, mapping)
            }
            DistributionStrategy::ConsistentHash { virtual_nodes: _, hash_ring } => {
                self.consistent_hash_placement(key_range, data_size, replication_factor, hash_ring)
            }
        }
    }

    /// Hash-based placement strategy
    fn hash_based_placement(
        &self,
        key_range: &KeyRange,
        data_size: u64,
        replication_factor: u32,
        hash_function: &HashFunction,
        num_buckets: u32,
    ) -> Result<PlacementResult> {
        let hash = self.compute_hash(&key_range.start, hash_function);
        let _bucket = (hash % num_buckets as u64) as u32;

        // Get available nodes sorted by capacity
        let mut candidates: Vec<_> = self.node_capacities.iter()
            .map(|(id, _capacity)| (*id, self.calculate_node_score(*id, data_size)))
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let num_replicas = (replication_factor as usize).min(candidates.len());
        let assigned_nodes: Vec<Uuid> = candidates[..num_replicas]
            .iter()
            .map(|(id, _)| *id)
            .collect();

        let primary_node = assigned_nodes[0];

        let load_distribution = self.calculate_load_distribution(&assigned_nodes, data_size);

        Ok(PlacementResult {
            primary_node,
            assigned_nodes,
            placement_score: self.calculate_placement_score(&candidates[..num_replicas]),
            load_distribution,
        })
    }

    /// Range-based placement strategy
    fn range_based_placement(
        &self,
        key_range: &KeyRange,
        data_size: u64,
        replication_factor: u32,
        _partition_key: &str,
        ranges: &[KeyRange],
    ) -> Result<PlacementResult> {
        // Find the range that contains this key range
        let _matching_range = ranges.iter()
            .find(|range| key_range.start >= range.start && key_range.end <= range.end);

        // For range-based, we prefer nodes in the same availability zone
        let mut candidates: Vec<_> = self.node_capacities.iter()
            .map(|(id, capacity)| {
                let score = self.calculate_node_score(*id, data_size);
                let zone_bonus = if capacity.availability_zone == "primary" { 0.2 } else { 0.0 };
                (*id, score + zone_bonus)
            })
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let num_replicas = (replication_factor as usize).min(candidates.len());
        let assigned_nodes: Vec<Uuid> = candidates[..num_replicas]
            .iter()
            .map(|(id, _)| *id)
            .collect();

        let primary_node = assigned_nodes[0];
        let load_distribution = self.calculate_load_distribution(&assigned_nodes, data_size);

        Ok(PlacementResult {
            primary_node,
            assigned_nodes,
            placement_score: self.calculate_placement_score(&candidates[..num_replicas]),
            load_distribution,
        })
    }

    /// Directory-based placement strategy
    fn directory_based_placement(
        &self,
        key_range: &KeyRange,
        data_size: u64,
        replication_factor: u32,
        mapping: &HashMap<String, Vec<Uuid>>,
    ) -> Result<PlacementResult> {
        // Find the best matching directory entry
        let best_match = mapping.keys()
            .filter(|key| key_range.start.starts_with(*key))
            .max_by_key(|key| key.len())
            .and_then(|key| mapping.get(key))
            .cloned()
            .unwrap_or_else(|| self.node_capacities.keys().cloned().collect());

        // Score nodes based on capacity and directory preference
        let mut candidates: Vec<_> = best_match.iter()
            .filter_map(|&node_id| {
                self.node_capacities.get(&node_id).map(|_| {
                    let score = self.calculate_node_score(node_id, data_size);
                    (node_id, score + 0.3) // Directory preference bonus
                })
            })
            .collect();

        // Add other nodes as fallback
        for (&node_id, _) in &self.node_capacities {
            if !best_match.contains(&node_id) {
                let score = self.calculate_node_score(node_id, data_size);
                candidates.push((node_id, score));
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let num_replicas = (replication_factor as usize).min(candidates.len());
        let assigned_nodes: Vec<Uuid> = candidates[..num_replicas]
            .iter()
            .map(|(id, _)| *id)
            .collect();

        let primary_node = assigned_nodes[0];
        let load_distribution = self.calculate_load_distribution(&assigned_nodes, data_size);

        Ok(PlacementResult {
            primary_node,
            assigned_nodes,
            placement_score: self.calculate_placement_score(&candidates[..num_replicas]),
            load_distribution,
        })
    }

    /// Consistent hashing placement strategy
    fn consistent_hash_placement(
        &self,
        key_range: &KeyRange,
        data_size: u64,
        replication_factor: u32,
        hash_ring: &[HashRingEntry],
    ) -> Result<PlacementResult> {
        let hash = self.compute_hash(&key_range.start, &HashFunction::Sha256);

        // Find position in ring
        let position = hash_ring.iter()
            .position(|entry| entry.hash >= hash)
            .unwrap_or(0);

        // Collect nodes clockwise from position
        let mut assigned_nodes = Vec::new();
        let mut seen_nodes = std::collections::HashSet::new();
        let ring_len = hash_ring.len();

        for i in 0..ring_len {
            let idx = (position + i) % ring_len;
            let entry = &hash_ring[idx];

            if seen_nodes.insert(entry.node_id) {
                assigned_nodes.push(entry.node_id);
                if assigned_nodes.len() >= replication_factor as usize {
                    break;
                }
            }
        }

        if assigned_nodes.is_empty() {
            return Err(anyhow::anyhow!("No nodes available in hash ring"));
        }

        let primary_node = assigned_nodes[0];
        let load_distribution = self.calculate_load_distribution(&assigned_nodes, data_size);

        // Calculate placement score based on ring distribution
        let placement_score = self.calculate_ring_placement_score(&assigned_nodes, hash_ring);

        Ok(PlacementResult {
            primary_node,
            assigned_nodes,
            placement_score,
            load_distribution,
        })
    }

    /// Calculate node score based on capacity and current load
    fn calculate_node_score(&self, node_id: Uuid, data_size: u64) -> f64 {
        if let Some(capacity) = self.node_capacities.get(&node_id) {
            let storage_ratio = (capacity.total_storage_bytes - capacity.used_storage_bytes) as f64
                / capacity.total_storage_bytes as f64;
            let cpu_ratio = capacity.cpu_cores as f64 / 32.0; // Normalize to 32 cores
            let load_factor = 1.0 - capacity.current_load;
            let size_factor = if capacity.total_storage_bytes >= data_size { 1.0 } else { 0.1 };

            storage_ratio * 0.4 + cpu_ratio * 0.2 + load_factor * 0.3 + size_factor * 0.1
        } else {
            0.0
        }
    }

    /// Calculate load distribution across assigned nodes
    fn calculate_load_distribution(&self, nodes: &[Uuid], data_size: u64) -> HashMap<Uuid, f64> {
        let mut distribution = HashMap::new();
        let data_per_node = data_size as f64 / nodes.len() as f64;

        for &node_id in nodes {
            if let Some(capacity) = self.node_capacities.get(&node_id) {
                let new_load = (capacity.used_storage_bytes as f64 + data_per_node)
                    / capacity.total_storage_bytes as f64;
                distribution.insert(node_id, new_load);
            }
        }

        distribution
    }

    /// Calculate placement score for a set of candidates
    fn calculate_placement_score(&self, candidates: &[(Uuid, f64)]) -> f64 {
        if candidates.is_empty() {
            return 0.0;
        }

        let total_score: f64 = candidates.iter().map(|(_, score)| score).sum();
        total_score / candidates.len() as f64
    }

    /// Calculate placement score for hash ring
    fn calculate_ring_placement_score(&self, nodes: &[Uuid], hash_ring: &[HashRingEntry]) -> f64 {
        // Score based on even distribution around the ring
        if nodes.len() < 2 {
            return 1.0;
        }

        let mut positions = Vec::new();
        for &node_id in nodes {
            if let Some(entry) = hash_ring.iter().find(|e| e.node_id == node_id) {
                positions.push(entry.hash);
            }
        }

        positions.sort();

        // Calculate distribution evenness
        let mut gaps = Vec::new();
        for i in 1..positions.len() {
            gaps.push(positions[i] - positions[i-1]);
        }

        // Add wrap-around gap
        if !positions.is_empty() {
            gaps.push((u64::MAX - positions.last().unwrap()) + positions[0]);
        }

        if gaps.is_empty() {
            return 1.0;
        }

        let mean_gap = gaps.iter().sum::<u64>() as f64 / gaps.len() as f64;
        let variance = gaps.iter()
            .map(|&gap| (gap as f64 - mean_gap).powi(2))
            .sum::<f64>() / gaps.len() as f64;

        // Score is higher for more even distribution (lower variance)
        1.0 / (1.0 + variance / (mean_gap * mean_gap))
    }

    /// Rebuild hash ring for consistent hashing
    fn rebuild_hash_ring(&mut self, virtual_nodes: u32) {
        let mut hash_ring = Vec::new();

        for &node_id in self.node_capacities.keys() {
            self.add_to_hash_ring(node_id, virtual_nodes, &mut hash_ring);
        }

        hash_ring.sort_by_key(|entry| entry.hash);

        if let DistributionStrategy::ConsistentHash { hash_ring: ring, .. } = &mut self.strategy {
            *ring = hash_ring;
        }
    }

    /// Add node to hash ring
    fn add_to_hash_ring(&self, node_id: Uuid, virtual_nodes: u32, hash_ring: &mut Vec<HashRingEntry>) {
        for i in 0..virtual_nodes {
            let key = format!("{}:{}", node_id, i);
            let hash = self.compute_hash(&key, &HashFunction::Sha256);

            hash_ring.push(HashRingEntry {
                hash,
                node_id,
                virtual_node_id: i,
            });
        }
    }

    /// Add node to hash ring vector (helper for borrowing issues)
    fn add_to_hash_ring_vec(&self, node_id: Uuid, virtual_nodes: u32, ring_entries: &mut Vec<HashRingEntry>) {
        for i in 0..virtual_nodes {
            let key = format!("{}:{}", node_id, i);
            let hash = self.compute_hash(&key, &HashFunction::Sha256);

            ring_entries.push(HashRingEntry {
                hash,
                node_id,
                virtual_node_id: i,
            });
        }
    }

    /// Compute hash using specified function
    fn compute_hash(&self, key: &str, hash_function: &HashFunction) -> u64 {
        match hash_function {
            HashFunction::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(key.as_bytes());
                let result = hasher.finalize();
                u64::from_be_bytes([
                    result[0], result[1], result[2], result[3],
                    result[4], result[5], result[6], result[7]
                ])
            }
            HashFunction::Xxhash => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                hasher.finish()
            }
            HashFunction::Murmur3 => {
                // Simplified murmur3 implementation
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                format!("murmur3:{}", key).hash(&mut hasher);
                hasher.finish()
            }
        }
    }

    /// Get strategy statistics
    pub fn get_stats(&self) -> StrategyStats {
        match &self.strategy {
            DistributionStrategy::Hash { num_buckets, .. } => {
                StrategyStats {
                    strategy_type: "Hash".to_string(),
                    total_nodes: self.node_capacities.len(),
                    virtual_nodes: 0,
                    buckets: *num_buckets as usize,
                    load_balance_score: self.calculate_load_balance_score(),
                }
            }
            DistributionStrategy::Range { ranges, .. } => {
                StrategyStats {
                    strategy_type: "Range".to_string(),
                    total_nodes: self.node_capacities.len(),
                    virtual_nodes: 0,
                    buckets: ranges.len(),
                    load_balance_score: self.calculate_load_balance_score(),
                }
            }
            DistributionStrategy::Directory { mapping } => {
                StrategyStats {
                    strategy_type: "Directory".to_string(),
                    total_nodes: self.node_capacities.len(),
                    virtual_nodes: 0,
                    buckets: mapping.len(),
                    load_balance_score: self.calculate_load_balance_score(),
                }
            }
            DistributionStrategy::ConsistentHash { virtual_nodes, hash_ring } => {
                StrategyStats {
                    strategy_type: "ConsistentHash".to_string(),
                    total_nodes: self.node_capacities.len(),
                    virtual_nodes: *virtual_nodes as usize,
                    buckets: hash_ring.len(),
                    load_balance_score: self.calculate_load_balance_score(),
                }
            }
        }
    }

    /// Calculate load balance score across all nodes
    fn calculate_load_balance_score(&self) -> f64 {
        if self.node_capacities.is_empty() {
            return 1.0;
        }

        let loads: Vec<f64> = self.node_capacities.values()
            .map(|capacity| capacity.current_load)
            .collect();

        if loads.is_empty() {
            return 1.0;
        }

        let mean_load = loads.iter().sum::<f64>() / loads.len() as f64;
        let variance = loads.iter()
            .map(|load| (load - mean_load).powi(2))
            .sum::<f64>() / loads.len() as f64;

        // Score is higher for better load balance (lower variance)
        1.0 / (1.0 + variance)
    }
}

/// Statistics for sharding strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStats {
    pub strategy_type: String,
    pub total_nodes: usize,
    pub virtual_nodes: usize,
    pub buckets: usize,
    pub load_balance_score: f64,
}