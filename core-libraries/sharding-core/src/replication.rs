//! Replication management for fault tolerance

use crate::{NodeId, Result, ShardConfig, ShardError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use tracing::{info, warn};
use uuid::Uuid;

/// Unique identifier for a replica
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplicaId(Uuid);

impl ReplicaId {
    /// Create a new replica ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ReplicaId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ReplicaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a replica
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaStatus {
    /// Replica is healthy and in sync
    Healthy,
    /// Replica is lagging behind
    Lagging,
    /// Replica is failed or unreachable
    Failed,
    /// Replica is being synchronized
    Syncing,
    /// Replica is being removed
    Removing,
}

impl Default for ReplicaStatus {
    fn default() -> Self {
        Self::Healthy
    }
}

/// Information about a replica
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaInfo {
    /// Unique replica identifier
    pub id: ReplicaId,
    /// Node hosting this replica
    pub node_id: NodeId,
    /// Current status
    pub status: ReplicaStatus,
    /// Last successful sync time
    pub last_sync: std::time::SystemTime,
    /// Lag in milliseconds behind primary
    pub lag_ms: u64,
    /// Number of operations applied
    pub operations_applied: u64,
    /// Checksum of replica data (for consistency checking)
    pub checksum: Option<String>,
}

impl ReplicaInfo {
    /// Create new replica info
    pub fn new(node_id: NodeId) -> Self {
        Self {
            id: ReplicaId::new(),
            node_id,
            status: ReplicaStatus::Syncing,
            last_sync: std::time::SystemTime::now(),
            lag_ms: 0,
            operations_applied: 0,
            checksum: None,
        }
    }

    /// Update sync status
    pub fn update_sync(&mut self, operations_applied: u64, lag_ms: u64) {
        self.last_sync = std::time::SystemTime::now();
        self.operations_applied = operations_applied;
        self.lag_ms = lag_ms;

        // Update status based on lag
        self.status = if lag_ms < 1000 {
            ReplicaStatus::Healthy
        } else if lag_ms < 10000 {
            ReplicaStatus::Lagging
        } else {
            ReplicaStatus::Failed
        };
    }

    /// Check if replica is available for reads
    pub fn is_readable(&self) -> bool {
        matches!(self.status, ReplicaStatus::Healthy | ReplicaStatus::Lagging)
    }

    /// Check if replica is available for writes
    pub fn is_writable(&self) -> bool {
        self.status == ReplicaStatus::Healthy
    }

    /// Get time since last sync
    pub fn time_since_sync(&self) -> Result<Duration> {
        self.last_sync
            .elapsed()
            .map_err(|e| ShardError::InternalError {
                reason: format!("Failed to calculate time since sync: {}", e),
            })
    }
}

/// Replication group for a key range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationGroup {
    /// Hash range this group is responsible for
    pub range: (u64, u64),
    /// Primary replica
    pub primary: ReplicaInfo,
    /// Secondary replicas
    pub secondaries: Vec<ReplicaInfo>,
    /// Replication factor
    pub replication_factor: usize,
    /// Created timestamp
    pub created_at: std::time::SystemTime,
    /// Last modification
    pub modified_at: std::time::SystemTime,
}

impl ReplicationGroup {
    /// Create a new replication group
    pub fn new(range: (u64, u64), primary_node: NodeId, replication_factor: usize) -> Self {
        let now = std::time::SystemTime::now();
        let primary = ReplicaInfo::new(primary_node);

        Self {
            range,
            primary,
            secondaries: Vec::new(),
            replication_factor,
            created_at: now,
            modified_at: now,
        }
    }

    /// Add a secondary replica
    pub fn add_secondary(&mut self, node_id: NodeId) -> Result<ReplicaId> {
        if self.secondaries.len() >= self.replication_factor - 1 {
            return Err(ShardError::InvalidConfiguration {
                reason: format!(
                    "Cannot add more replicas, already have {} secondaries for replication factor {}",
                    self.secondaries.len(),
                    self.replication_factor
                ),
            });
        }

        // Check if node already has a replica in this group
        if self.primary.node_id == node_id {
            return Err(ShardError::InvalidConfiguration {
                reason: "Node already hosts the primary replica".to_string(),
            });
        }

        if self.secondaries.iter().any(|r| r.node_id == node_id) {
            return Err(ShardError::InvalidConfiguration {
                reason: "Node already hosts a secondary replica".to_string(),
            });
        }

        let replica = ReplicaInfo::new(node_id);
        let replica_id = replica.id.clone();
        self.secondaries.push(replica);
        self.modified_at = std::time::SystemTime::now();

        Ok(replica_id)
    }

    /// Remove a secondary replica
    pub fn remove_secondary(&mut self, replica_id: &ReplicaId) -> Result<()> {
        let initial_len = self.secondaries.len();
        self.secondaries.retain(|r| r.id != *replica_id);

        if self.secondaries.len() == initial_len {
            return Err(ShardError::ReplicaNotFound {
                replica_id: replica_id.to_string(),
            });
        }

        self.modified_at = std::time::SystemTime::now();
        Ok(())
    }

    /// Get all replicas (primary + secondaries)
    pub fn all_replicas(&self) -> Vec<&ReplicaInfo> {
        let mut replicas = vec![&self.primary];
        replicas.extend(self.secondaries.iter());
        replicas
    }

    /// Get healthy replicas
    pub fn healthy_replicas(&self) -> Vec<&ReplicaInfo> {
        self.all_replicas()
            .into_iter()
            .filter(|r| r.is_readable())
            .collect()
    }

    /// Get writable replicas
    pub fn writable_replicas(&self) -> Vec<&ReplicaInfo> {
        self.all_replicas()
            .into_iter()
            .filter(|r| r.is_writable())
            .collect()
    }

    /// Check if group has quorum
    pub fn has_quorum(&self) -> bool {
        let healthy_count = self.healthy_replicas().len();
        let required_quorum = (self.replication_factor / 2) + 1;
        healthy_count >= required_quorum
    }

    /// Get quorum size
    pub fn quorum_size(&self) -> usize {
        (self.replication_factor / 2) + 1
    }

    /// Check if replication is complete
    pub fn is_fully_replicated(&self) -> bool {
        self.all_replicas().len() == self.replication_factor
    }

    /// Promote a secondary to primary
    pub fn promote_secondary(&mut self, replica_id: &ReplicaId) -> Result<()> {
        // Find the secondary to promote
        let secondary_index = self
            .secondaries
            .iter()
            .position(|r| r.id == *replica_id)
            .ok_or_else(|| ShardError::ReplicaNotFound {
                replica_id: replica_id.to_string(),
            })?;

        // Remove the secondary and make it primary
        let new_primary = self.secondaries.remove(secondary_index);
        let old_primary = std::mem::replace(&mut self.primary, new_primary);

        // Add old primary as secondary
        self.secondaries.push(old_primary);
        self.modified_at = std::time::SystemTime::now();

        info!("Promoted replica {} to primary for range {:?}", replica_id, self.range);
        Ok(())
    }
}

/// Consistency level for read/write operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyLevel {
    /// Read/write from/to primary only
    Strong,
    /// Read from any healthy replica, write to quorum
    Eventual,
    /// Read/write from/to quorum
    Quorum,
    /// Read/write from/to all replicas
    All,
}

/// Replication operation result
#[derive(Debug, Clone)]
pub struct ReplicationResult {
    /// Number of successful operations
    pub successful: usize,
    /// Number of failed operations
    pub failed: usize,
    /// Nodes that succeeded
    pub successful_nodes: Vec<NodeId>,
    /// Nodes that failed
    pub failed_nodes: Vec<NodeId>,
}

impl ReplicationResult {
    /// Check if operation achieved quorum
    pub fn has_quorum(&self, quorum_size: usize) -> bool {
        self.successful >= quorum_size
    }

    /// Check if all replicas succeeded
    pub fn is_unanimous(&self) -> bool {
        self.failed == 0
    }
}

/// Manages replication across the cluster
pub struct ReplicationManager {
    /// Configuration
    config: ShardConfig,
    /// Replication groups by hash range
    replication_groups: Arc<RwLock<HashMap<(u64, u64), ReplicationGroup>>>,
    /// Replica health tracking
    replica_health: Arc<RwLock<HashMap<ReplicaId, ReplicaHealth>>>,
}

impl ReplicationManager {
    /// Create a new replication manager
    pub fn new(config: ShardConfig) -> Self {
        Self {
            config,
            replication_groups: Arc::new(RwLock::new(HashMap::new())),
            replica_health: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a replication group for a hash range
    pub async fn create_replication_group(
        &self,
        range: (u64, u64),
        primary_node: NodeId,
        secondary_nodes: Vec<NodeId>,
    ) -> Result<()> {
        let mut groups = self.replication_groups.write().await;

        if groups.contains_key(&range) {
            return Err(ShardError::InvalidState {
                reason: format!("Replication group already exists for range {:?}", range),
            });
        }

        let mut group = ReplicationGroup::new(range, primary_node, self.config.replication_factor);

        // Add secondary nodes
        for node_id in secondary_nodes {
            group.add_secondary(node_id)?;
        }

        groups.insert(range, group);
        info!("Created replication group for range {:?}", range);

        Ok(())
    }

    /// Get replication group for a hash range
    pub async fn get_replication_group(&self, range: &(u64, u64)) -> Option<ReplicationGroup> {
        let groups = self.replication_groups.read().await;
        groups.get(range).cloned()
    }

    /// Find replication group containing a hash value
    pub async fn find_replication_group(&self, hash: u64) -> Option<ReplicationGroup> {
        let groups = self.replication_groups.read().await;

        for group in groups.values() {
            if hash >= group.range.0 && hash <= group.range.1 {
                return Some(group.clone());
            }
        }

        None
    }

    /// Add a replica to an existing group
    pub async fn add_replica(&self, range: (u64, u64), node_id: NodeId) -> Result<ReplicaId> {
        let mut groups = self.replication_groups.write().await;

        if let Some(group) = groups.get_mut(&range) {
            let replica_id = group.add_secondary(node_id)?;
            info!("Added replica {} to group for range {:?}", replica_id, range);
            Ok(replica_id)
        } else {
            Err(ShardError::InvalidState {
                reason: format!("No replication group found for range {:?}", range),
            })
        }
    }

    /// Remove a replica from a group
    pub async fn remove_replica(&self, range: (u64, u64), replica_id: &ReplicaId) -> Result<()> {
        let mut groups = self.replication_groups.write().await;

        if let Some(group) = groups.get_mut(&range) {
            group.remove_secondary(replica_id)?;
            info!("Removed replica {} from group for range {:?}", replica_id, range);
            Ok(())
        } else {
            Err(ShardError::InvalidState {
                reason: format!("No replication group found for range {:?}", range),
            })
        }
    }

    /// Promote a secondary replica to primary
    pub async fn promote_replica(&self, range: (u64, u64), replica_id: &ReplicaId) -> Result<()> {
        let mut groups = self.replication_groups.write().await;

        if let Some(group) = groups.get_mut(&range) {
            group.promote_secondary(replica_id)?;
            info!("Promoted replica {} to primary for range {:?}", replica_id, range);
            Ok(())
        } else {
            Err(ShardError::InvalidState {
                reason: format!("No replication group found for range {:?}", range),
            })
        }
    }

    /// Perform a replicated write operation
    pub async fn replicated_write(
        &self,
        key: &str,
        data: &[u8],
        consistency_level: ConsistencyLevel,
    ) -> Result<ReplicationResult> {
        let key_hash = self.hash_key(key);
        let group = self.find_replication_group(key_hash).await.ok_or_else(|| ShardError::InvalidState {
            reason: format!("No replication group found for key: {}", key),
        })?;

        let target_replicas = match consistency_level {
            ConsistencyLevel::Strong => vec![&group.primary],
            ConsistencyLevel::Eventual => group.writable_replicas(),
            ConsistencyLevel::Quorum => {
                let mut replicas = group.writable_replicas();
                replicas.truncate(group.quorum_size());
                replicas
            }
            ConsistencyLevel::All => group.all_replicas().into_iter().filter(|r| r.is_writable()).collect(),
        };

        // Simulate write operations
        let mut successful = 0;
        let mut failed = 0;
        let mut successful_nodes = Vec::new();
        let mut failed_nodes = Vec::new();

        for replica in target_replicas {
            // In real implementation, this would send the write to the replica node
            if self.simulate_write_to_replica(&replica.node_id, key, data).await {
                successful += 1;
                successful_nodes.push(replica.node_id.clone());
            } else {
                failed += 1;
                failed_nodes.push(replica.node_id.clone());
            }
        }

        let result = ReplicationResult {
            successful,
            failed,
            successful_nodes,
            failed_nodes,
        };

        // Check if write achieved required consistency
        match consistency_level {
            ConsistencyLevel::Strong => {
                if result.successful == 0 {
                    return Err(ShardError::ReplicationFailed {
                        reason: "Primary write failed".to_string(),
                    });
                }
            }
            ConsistencyLevel::Quorum => {
                if !result.has_quorum(group.quorum_size()) {
                    return Err(ShardError::QuorumNotAchieved {
                        actual: result.successful,
                        required: group.quorum_size(),
                    });
                }
            }
            ConsistencyLevel::All => {
                if !result.is_unanimous() {
                    return Err(ShardError::ReplicationFailed {
                        reason: format!("Failed to write to all replicas: {}/{}", result.successful, result.successful + result.failed),
                    });
                }
            }
            ConsistencyLevel::Eventual => {
                // Eventually consistent, so any success is acceptable
                if result.successful == 0 {
                    return Err(ShardError::ReplicationFailed {
                        reason: "No replicas accepted the write".to_string(),
                    });
                }
            }
        }

        Ok(result)
    }

    /// Perform a replicated read operation
    pub async fn replicated_read(
        &self,
        key: &str,
        consistency_level: ConsistencyLevel,
    ) -> Result<(Vec<u8>, ReplicationResult)> {
        let key_hash = self.hash_key(key);
        let group = self.find_replication_group(key_hash).await.ok_or_else(|| ShardError::InvalidState {
            reason: format!("No replication group found for key: {}", key),
        })?;

        let target_replicas = match consistency_level {
            ConsistencyLevel::Strong => vec![&group.primary],
            ConsistencyLevel::Eventual => {
                // Read from any healthy replica
                let healthy = group.healthy_replicas();
                if healthy.is_empty() {
                    vec![]
                } else {
                    vec![healthy[0]] // Just pick the first one
                }
            }
            ConsistencyLevel::Quorum => {
                let mut replicas = group.healthy_replicas();
                replicas.truncate(group.quorum_size());
                replicas
            }
            ConsistencyLevel::All => group.healthy_replicas(),
        };

        if target_replicas.is_empty() {
            return Err(ShardError::ReplicationFailed {
                reason: "No healthy replicas available for read".to_string(),
            });
        }

        // Simulate read operations
        let mut data_values = HashMap::new();
        let mut successful = 0;
        let mut failed = 0;
        let mut successful_nodes = Vec::new();
        let mut failed_nodes = Vec::new();

        for replica in target_replicas {
            if let Some(data) = self.simulate_read_from_replica(&replica.node_id, key).await {
                successful += 1;
                successful_nodes.push(replica.node_id.clone());

                // Count data values for consistency checking
                *data_values.entry(data).or_insert(0) += 1;
            } else {
                failed += 1;
                failed_nodes.push(replica.node_id.clone());
            }
        }

        let result = ReplicationResult {
            successful,
            failed,
            successful_nodes,
            failed_nodes,
        };

        if successful == 0 {
            return Err(ShardError::ReplicationFailed {
                reason: "No replicas returned data".to_string(),
            });
        }

        // Find the most common data value (simple consistency resolution)
        let (data, count) = data_values
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .unwrap();

        // Check for consistency issues
        if consistency_level == ConsistencyLevel::All && count != successful {
            warn!("Inconsistent data detected for key: {}", key);
        }

        Ok((data, result))
    }

    /// Check replication health
    pub async fn check_replication_health(&self) -> ReplicationHealthReport {
        let groups = self.replication_groups.read().await;
        let mut total_groups = 0;
        let mut healthy_groups = 0;
        let mut groups_with_quorum = 0;
        let mut under_replicated_groups = 0;

        for group in groups.values() {
            total_groups += 1;

            let healthy_replicas = group.healthy_replicas().len();
            let total_replicas = group.all_replicas().len();

            if healthy_replicas == total_replicas {
                healthy_groups += 1;
            }

            if group.has_quorum() {
                groups_with_quorum += 1;
            }

            if total_replicas < group.replication_factor {
                under_replicated_groups += 1;
            }
        }

        ReplicationHealthReport {
            total_groups,
            healthy_groups,
            groups_with_quorum,
            under_replicated_groups,
        }
    }

    /// Hash a key to determine its location
    fn hash_key(&self, key: &str) -> u64 {
        // Simple hash function for demonstration
        // In practice, would use the same hash function as the consistent hash ring
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Simulate writing to a replica (placeholder)
    async fn simulate_write_to_replica(&self, _node_id: &NodeId, _key: &str, _data: &[u8]) -> bool {
        // Simulate 95% success rate
        rand::random::<f64>() < 0.95
    }

    /// Simulate reading from a replica (placeholder)
    async fn simulate_read_from_replica(&self, _node_id: &NodeId, _key: &str) -> Option<Vec<u8>> {
        // Simulate 95% success rate, return dummy data
        if rand::random::<f64>() < 0.95 {
            Some(b"dummy_data".to_vec())
        } else {
            None
        }
    }
}

/// Health information for a replica
#[derive(Debug, Clone)]
struct ReplicaHealth {
    last_health_check: Instant,
    consecutive_failures: usize,
    is_healthy: bool,
}

/// Replication health report
#[derive(Debug, Clone)]
pub struct ReplicationHealthReport {
    pub total_groups: usize,
    pub healthy_groups: usize,
    pub groups_with_quorum: usize,
    pub under_replicated_groups: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replica_info() {
        let node_id = NodeId::new("test-node");
        let mut replica = ReplicaInfo::new(node_id.clone());

        assert_eq!(replica.node_id, node_id);
        assert_eq!(replica.status, ReplicaStatus::Syncing);
        assert_eq!(replica.operations_applied, 0);

        // Update sync
        replica.update_sync(100, 500);
        assert_eq!(replica.operations_applied, 100);
        assert_eq!(replica.lag_ms, 500);
        assert_eq!(replica.status, ReplicaStatus::Healthy);

        // Test high lag
        replica.update_sync(200, 15000);
        assert_eq!(replica.status, ReplicaStatus::Failed);
    }

    #[test]
    fn test_replication_group() {
        let primary_node = NodeId::new("primary");
        let secondary1 = NodeId::new("secondary1");
        let secondary2 = NodeId::new("secondary2");

        let mut group = ReplicationGroup::new((100, 200), primary_node.clone(), 3);

        assert_eq!(group.range, (100, 200));
        assert_eq!(group.primary.node_id, primary_node);
        assert_eq!(group.replication_factor, 3);
        assert!(!group.is_fully_replicated());

        // Add secondaries
        assert!(group.add_secondary(secondary1.clone()).is_ok());
        assert!(group.add_secondary(secondary2.clone()).is_ok());
        assert!(group.is_fully_replicated());

        // Mark replicas as healthy for quorum test
        group.primary.status = ReplicaStatus::Healthy;
        for secondary in &mut group.secondaries {
            secondary.status = ReplicaStatus::Healthy;
        }

        // Try to add duplicate
        assert!(group.add_secondary(secondary1).is_err());

        // Check quorum
        assert!(group.has_quorum());
        assert_eq!(group.quorum_size(), 2);

        // Test promotion
        let replica_id = group.secondaries[0].id.clone();
        assert!(group.promote_secondary(&replica_id).is_ok());
        assert_eq!(group.secondaries.len(), 2);
    }

    #[tokio::test]
    async fn test_replication_manager() {
        let mut config = ShardConfig::test_config();
        config.replication_factor = 3; // Allow primary + 2 secondaries
        let manager = ReplicationManager::new(config);

        let primary = NodeId::new("primary");
        let secondary = NodeId::new("secondary");
        let range = (100, 200);

        // Create replication group
        assert!(manager.create_replication_group(range, primary.clone(), vec![secondary.clone()]).await.is_ok());

        // Get group
        let group = manager.get_replication_group(&range).await.unwrap();
        assert_eq!(group.primary.node_id, primary);
        assert_eq!(group.secondaries.len(), 1);

        // Add another replica
        let node3 = NodeId::new("node3");
        assert!(manager.add_replica(range, node3).await.is_ok());

        // Check health
        let health = manager.check_replication_health().await;
        assert_eq!(health.total_groups, 1);
    }

    #[tokio::test]
    async fn test_replicated_operations() {
        let mut config = ShardConfig::test_config();
        config.replication_factor = 2; // Allow primary + 1 secondary
        let manager = ReplicationManager::new(config);

        let primary = NodeId::new("primary");
        let secondary = NodeId::new("secondary");
        let range = (0, u64::MAX); // Cover all possible hash values

        // Create replication group
        manager.create_replication_group(range, primary, vec![secondary]).await.unwrap();

        // Mark replicas as healthy for operations
        {
            let mut groups = manager.replication_groups.write().await;
            if let Some(group) = groups.get_mut(&range) {
                group.primary.status = ReplicaStatus::Healthy;
                for secondary in &mut group.secondaries {
                    secondary.status = ReplicaStatus::Healthy;
                }
            }
        }

        // Test write
        let write_result = manager.replicated_write("test_key", b"test_data", ConsistencyLevel::Eventual).await.unwrap();
        assert!(write_result.successful > 0);

        // Test read
        let (data, read_result) = manager.replicated_read("test_key", ConsistencyLevel::Eventual).await.unwrap();
        assert!(!data.is_empty());
        assert!(read_result.successful > 0);
    }

    #[test]
    fn test_consistency_levels() {
        // Test that all consistency levels are covered
        let levels = [
            ConsistencyLevel::Strong,
            ConsistencyLevel::Eventual,
            ConsistencyLevel::Quorum,
            ConsistencyLevel::All,
        ];

        for level in levels {
            // Just check that we can create them
            let _ = level;
        }
    }

    #[test]
    fn test_replication_result() {
        let result = ReplicationResult {
            successful: 2,
            failed: 1,
            successful_nodes: vec![NodeId::new("node1"), NodeId::new("node2")],
            failed_nodes: vec![NodeId::new("node3")],
        };

        assert!(result.has_quorum(2));
        assert!(!result.has_quorum(3));
        assert!(!result.is_unanimous());

        let unanimous_result = ReplicationResult {
            successful: 3,
            failed: 0,
            successful_nodes: vec![NodeId::new("node1"), NodeId::new("node2"), NodeId::new("node3")],
            failed_nodes: vec![],
        };

        assert!(unanimous_result.is_unanimous());
    }
}