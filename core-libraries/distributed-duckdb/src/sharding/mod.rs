//! Automatic Data Distribution and Sharding for StreamSync
//!
//! This module implements intelligent data sharding and distribution across the
//! decentralized StreamSync network. It automatically manages data placement,
//! replication, load balancing, and fault tolerance.

pub mod strategy;
pub mod placement;
pub mod replication;
pub mod migration;
pub mod load_balancer;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Sharding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// Default replication factor
    pub default_replication_factor: u32,
    /// Maximum shard size in bytes
    pub max_shard_size_bytes: u64,
    /// Load balancing threshold (when to trigger rebalancing)
    pub load_threshold: f64,
    /// Minimum nodes required for sharding
    pub min_nodes: usize,
    /// Shard migration batch size
    pub migration_batch_size: u64,
    /// Health check interval for shards
    pub health_check_interval_secs: u64,
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            default_replication_factor: 3,
            max_shard_size_bytes: 1024 * 1024 * 1024, // 1GB
            load_threshold: 0.8, // 80% capacity
            min_nodes: 3,
            migration_batch_size: 10000,
            health_check_interval_secs: 30,
        }
    }
}

/// Shard metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    pub shard_id: String,
    pub key_range: KeyRange,
    pub assigned_nodes: Vec<Uuid>,
    pub primary_node: Uuid,
    pub size_bytes: u64,
    pub record_count: u64,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub health_status: ShardHealth,
    pub schema_hash: String,
}

/// Key range for sharding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRange {
    pub start: String,
    pub end: String,
    pub shard_key: String,
}

/// Shard health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShardHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Migrating,
    Offline,
}

/// Data distribution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributionStrategy {
    /// Hash-based sharding
    Hash {
        hash_function: HashFunction,
        num_buckets: u32,
    },
    /// Range-based sharding
    Range {
        partition_key: String,
        ranges: Vec<KeyRange>,
    },
    /// Directory-based sharding
    Directory {
        mapping: HashMap<String, Vec<Uuid>>,
    },
    /// Consistent hashing
    ConsistentHash {
        virtual_nodes: u32,
        hash_ring: Vec<HashRingEntry>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashFunction {
    Sha256,
    Xxhash,
    Murmur3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRingEntry {
    pub hash: u64,
    pub node_id: Uuid,
    pub virtual_node_id: u32,
}

/// Data placement manager
pub struct DataPlacementManager {
    config: ShardingConfig,
    strategy: DistributionStrategy,
    shard_registry: HashMap<String, ShardInfo>,
    node_capacities: HashMap<Uuid, NodeCapacity>,
    placement_engine: placement::PlacementEngine,
    replication_manager: replication::ReplicationManager,
    migration_engine: migration::MigrationEngine,
    load_balancer: load_balancer::LoadBalancer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapacity {
    pub node_id: Uuid,
    pub total_storage_bytes: u64,
    pub used_storage_bytes: u64,
    pub cpu_cores: u32,
    pub memory_bytes: u64,
    pub network_bandwidth_mbps: u32,
    pub current_load: f64,
    pub availability_zone: String,
    pub last_updated: DateTime<Utc>,
}

impl DataPlacementManager {
    /// Create a new data placement manager
    pub fn new(config: ShardingConfig, strategy: DistributionStrategy) -> Result<Self> {
        let placement_engine = placement::PlacementEngine::new(config.clone(), strategy.clone())?;
        let replication_manager = replication::ReplicationManager::new(config.clone())?;
        let migration_engine = migration::MigrationEngine::new(config.clone())?;
        let load_balancer = load_balancer::LoadBalancer::new(config.clone())?;

        Ok(Self {
            config,
            strategy,
            shard_registry: HashMap::new(),
            node_capacities: HashMap::new(),
            placement_engine,
            replication_manager,
            migration_engine,
            load_balancer,
        })
    }

    /// Register a new node in the cluster
    pub async fn register_node(&mut self, capacity: NodeCapacity) -> Result<()> {
        tracing::info!("📍 Registering node {} with capacity {:?}", capacity.node_id, capacity);

        self.node_capacities.insert(capacity.node_id, capacity.clone());

        // Update placement strategy if needed
        self.placement_engine.add_node(capacity).await?;

        // Trigger rebalancing if necessary
        if self.should_rebalance().await {
            self.trigger_rebalancing().await?;
        }

        Ok(())
    }

    /// Unregister a node from the cluster
    pub async fn unregister_node(&mut self, node_id: Uuid) -> Result<()> {
        tracing::info!("📤 Unregistering node {}", node_id);

        // Migrate shards away from this node
        let affected_shards = self.get_shards_on_node(node_id).await;
        for shard_info in affected_shards {
            self.migrate_shard_away_from_node(&shard_info.shard_id, node_id).await?;
        }

        self.node_capacities.remove(&node_id);
        self.placement_engine.remove_node(node_id).await?;

        Ok(())
    }

    /// Create a new shard for data
    pub async fn create_shard(&mut self, key_range: KeyRange, data: Vec<u8>) -> Result<String> {
        let shard_id = self.generate_shard_id(&key_range).await;

        tracing::info!("🆕 Creating shard {} for key range {:?}", shard_id, key_range);

        // Determine optimal placement
        let placement = self.placement_engine.determine_placement(&key_range, data.len() as u64).await?;

        // Create shard info
        let shard_info = ShardInfo {
            shard_id: shard_id.clone(),
            key_range,
            assigned_nodes: placement.assigned_nodes.clone(),
            primary_node: placement.primary_node,
            size_bytes: data.len() as u64,
            record_count: self.estimate_record_count(&data),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            health_status: ShardHealth::Healthy,
            schema_hash: self.compute_schema_hash(&data),
        };

        // Store shard info
        self.shard_registry.insert(shard_id.clone(), shard_info);

        // Replicate data to assigned nodes
        self.replication_manager.replicate_shard(&shard_id, &data, &placement.assigned_nodes).await?;

        tracing::info!("✅ Shard {} created and replicated to {} nodes", shard_id, placement.assigned_nodes.len());

        Ok(shard_id)
    }

    /// Get shard information
    pub async fn get_shard(&self, shard_id: &str) -> Option<&ShardInfo> {
        self.shard_registry.get(shard_id)
    }

    /// Find shards for a given key
    pub async fn find_shards_for_key(&self, key: &str) -> Vec<String> {
        match &self.strategy {
            DistributionStrategy::Hash { hash_function, num_buckets } => {
                let hash = self.compute_hash(key, hash_function);
                let bucket = hash % (*num_buckets as u64);
                vec![format!("shard_{}", bucket)]
            }
            DistributionStrategy::Range { ranges, .. } => {
                ranges.iter()
                    .filter(|range| key >= range.start.as_str() && key < range.end.as_str())
                    .map(|range| self.generate_shard_id_for_range(range))
                    .collect()
            }
            DistributionStrategy::Directory { mapping } => {
                mapping.keys()
                    .filter(|k| key.starts_with(*k))
                    .map(|k| k.clone())
                    .collect()
            }
            DistributionStrategy::ConsistentHash { hash_ring, .. } => {
                let hash = self.compute_hash(key, &HashFunction::Sha256);
                let position = hash_ring.iter()
                    .position(|entry| entry.hash >= hash)
                    .unwrap_or(0);

                if let Some(entry) = hash_ring.get(position) {
                    vec![format!("shard_{}_{}", entry.node_id, entry.virtual_node_id)]
                } else {
                    vec![]
                }
            }
        }
    }

    /// Get nodes for a shard
    pub async fn get_nodes_for_shard(&self, shard_id: &str) -> Option<Vec<Uuid>> {
        self.shard_registry.get(shard_id).map(|info| info.assigned_nodes.clone())
    }

    /// Update node capacity information
    pub async fn update_node_capacity(&mut self, capacity: NodeCapacity) -> Result<()> {
        self.node_capacities.insert(capacity.node_id, capacity);

        // Check if rebalancing is needed
        if self.should_rebalance().await {
            self.trigger_rebalancing().await?;
        }

        Ok(())
    }

    /// Get cluster statistics
    pub async fn get_cluster_stats(&self) -> ClusterStats {
        let total_nodes = self.node_capacities.len();
        let total_shards = self.shard_registry.len();

        let total_storage: u64 = self.node_capacities.values()
            .map(|c| c.total_storage_bytes)
            .sum();

        let used_storage: u64 = self.node_capacities.values()
            .map(|c| c.used_storage_bytes)
            .sum();

        let healthy_shards = self.shard_registry.values()
            .filter(|s| s.health_status == ShardHealth::Healthy)
            .count();

        ClusterStats {
            total_nodes,
            total_shards,
            healthy_shards,
            total_storage_bytes: total_storage,
            used_storage_bytes: used_storage,
            storage_utilization: used_storage as f64 / total_storage as f64,
            average_shard_size: if total_shards > 0 {
                self.shard_registry.values().map(|s| s.size_bytes).sum::<u64>() / total_shards as u64
            } else {
                0
            },
            replication_factor: self.config.default_replication_factor,
        }
    }

    /// Trigger manual rebalancing
    pub async fn trigger_rebalancing(&mut self) -> Result<()> {
        tracing::info!("⚖️ Triggering cluster rebalancing");

        let rebalancing_plan = self.load_balancer.create_rebalancing_plan(
            &self.shard_registry,
            &self.node_capacities
        ).await?;

        let migrations_count = rebalancing_plan.migrations.len();
        for migration in rebalancing_plan.migrations {
            self.migration_engine.migrate_shard(
                &migration.shard_id,
                migration.from_node,
                migration.to_node
            ).await?;

            // Update shard registry
            if let Some(shard_info) = self.shard_registry.get_mut(&migration.shard_id) {
                if let Some(pos) = shard_info.assigned_nodes.iter().position(|&x| x == migration.from_node) {
                    shard_info.assigned_nodes[pos] = migration.to_node;
                }
            }
        }

        tracing::info!("✅ Rebalancing completed with {} migrations", migrations_count);
        Ok(())
    }

    // Private helper methods

    async fn should_rebalance(&self) -> bool {
        // Check if any node is above the load threshold
        self.node_capacities.values()
            .any(|capacity| capacity.current_load > self.config.load_threshold)
    }

    async fn get_shards_on_node(&self, node_id: Uuid) -> Vec<ShardInfo> {
        self.shard_registry.values()
            .filter(|shard| shard.assigned_nodes.contains(&node_id))
            .cloned()
            .collect()
    }

    async fn migrate_shard_away_from_node(&mut self, shard_id: &str, from_node: Uuid) -> Result<()> {
        // Find a suitable target node
        let target_node = self.placement_engine.find_best_node_for_migration(shard_id, from_node).await?;

        // Perform migration
        self.migration_engine.migrate_shard(shard_id, from_node, target_node).await?;

        // Update shard registry
        if let Some(shard_info) = self.shard_registry.get_mut(shard_id) {
            if let Some(pos) = shard_info.assigned_nodes.iter().position(|&x| x == from_node) {
                shard_info.assigned_nodes[pos] = target_node;
            }
        }

        Ok(())
    }

    async fn generate_shard_id(&self, key_range: &KeyRange) -> String {
        format!("shard_{}_{}",
               self.compute_hash(&key_range.start, &HashFunction::Sha256),
               Utc::now().timestamp_millis())
    }

    fn generate_shard_id_for_range(&self, range: &KeyRange) -> String {
        format!("shard_{}_{}",
               self.compute_hash(&range.start, &HashFunction::Sha256),
               self.compute_hash(&range.end, &HashFunction::Sha256))
    }

    fn compute_hash(&self, key: &str, hash_function: &HashFunction) -> u64 {
        match hash_function {
            HashFunction::Sha256 => {
                use sha2::{Sha256, Digest};
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
                // Simplified hash for demo
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                hasher.finish()
            }
        }
    }

    fn estimate_record_count(&self, data: &[u8]) -> u64 {
        // Simplified record count estimation
        // In practice, this would analyze the data structure
        data.len() as u64 / 100 // Assume average record size of 100 bytes
    }

    fn compute_schema_hash(&self, data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

/// Cluster statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub total_shards: usize,
    pub healthy_shards: usize,
    pub total_storage_bytes: u64,
    pub used_storage_bytes: u64,
    pub storage_utilization: f64,
    pub average_shard_size: u64,
    pub replication_factor: u32,
}