//! Data Replication Management for StreamSync Sharding
//!
//! This module handles data replication across nodes, ensuring fault tolerance
//! and data consistency in the distributed system.

use super::ShardingConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::sync::{RwLock, mpsc};
use std::sync::Arc;

/// Replication status for a shard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationStatus {
    /// All replicas are healthy and synchronized
    Healthy,
    /// Some replicas are lagging but within acceptable bounds
    Degraded,
    /// Critical replication failure
    Critical,
    /// Replication in progress
    InProgress,
    /// Replication failed
    Failed,
}

/// Replica information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaInfo {
    pub node_id: Uuid,
    pub shard_id: String,
    pub status: ReplicaStatus,
    pub last_sync: DateTime<Utc>,
    pub sync_lag_ms: u64,
    pub data_version: u64,
    pub size_bytes: u64,
    pub checksum: String,
}

/// Status of individual replica
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicaStatus {
    Active,
    Syncing,
    Lagging,
    Failed,
    Maintenance,
}

/// Replication strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStrategy {
    pub strategy_type: ReplicationStrategyType,
    pub consistency_level: ConsistencyLevel,
    pub sync_timeout_ms: u64,
    pub max_lag_tolerance_ms: u64,
    pub auto_repair: bool,
    pub checksum_validation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationStrategyType {
    /// Synchronous replication - all replicas must acknowledge
    Synchronous,
    /// Asynchronous replication - fire and forget
    Asynchronous,
    /// Semi-synchronous - wait for a quorum
    SemiSynchronous { quorum_size: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    /// Strong consistency - all replicas synchronized
    Strong,
    /// Eventual consistency - replicas will converge
    Eventual,
    /// Causal consistency - causally related operations are ordered
    Causal,
}

/// Replication operation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationOperation {
    pub operation_id: String,
    pub shard_id: String,
    pub operation_type: OperationType,
    pub source_node: Uuid,
    pub target_nodes: Vec<Uuid>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ReplicationOperationStatus,
    pub error_message: Option<String>,
    pub bytes_transferred: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    InitialReplication,
    IncrementalSync,
    FullResync,
    RepairSync,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplicationOperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Main replication manager
pub struct ReplicationManager {
    config: ShardingConfig,
    strategy: ReplicationStrategy,
    replica_registry: Arc<RwLock<HashMap<String, Vec<ReplicaInfo>>>>,
    active_operations: Arc<RwLock<HashMap<String, ReplicationOperation>>>,
    operation_history: Arc<RwLock<Vec<ReplicationOperation>>>,

    // Communication channels
    replication_tx: mpsc::UnboundedSender<ReplicationTask>,

    // Monitoring
    metrics: Arc<RwLock<ReplicationMetrics>>,
}

/// Replication task for background processing
#[derive(Debug, Clone)]
pub struct ReplicationTask {
    pub task_id: String,
    pub shard_id: String,
    pub operation_type: OperationType,
    pub source_node: Uuid,
    pub target_nodes: Vec<Uuid>,
    pub data: Vec<u8>,
    pub priority: TaskPriority,
    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Emergency = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

/// Replication metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationMetrics {
    pub total_replicas: usize,
    pub healthy_replicas: usize,
    pub degraded_replicas: usize,
    pub failed_replicas: usize,
    pub total_shards_replicated: usize,
    pub average_sync_lag_ms: f64,
    pub replication_throughput_mbps: f64,
    pub success_rate: f64,
    pub last_updated: DateTime<Utc>,
}

impl ReplicationManager {
    /// Create a new replication manager
    pub fn new(config: ShardingConfig) -> Result<Self> {
        let strategy = ReplicationStrategy {
            strategy_type: ReplicationStrategyType::SemiSynchronous {
                quorum_size: ((config.default_replication_factor + 1) / 2) as usize
            },
            consistency_level: ConsistencyLevel::Strong,
            sync_timeout_ms: 30000, // 30 seconds
            max_lag_tolerance_ms: 5000, // 5 seconds
            auto_repair: true,
            checksum_validation: true,
        };

        let (replication_tx, _replication_rx) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            strategy,
            replica_registry: Arc::new(RwLock::new(HashMap::new())),
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            operation_history: Arc::new(RwLock::new(Vec::new())),
            replication_tx,
            metrics: Arc::new(RwLock::new(ReplicationMetrics {
                total_replicas: 0,
                healthy_replicas: 0,
                degraded_replicas: 0,
                failed_replicas: 0,
                total_shards_replicated: 0,
                average_sync_lag_ms: 0.0,
                replication_throughput_mbps: 0.0,
                success_rate: 1.0,
                last_updated: Utc::now(),
            })),
        })
    }

    /// Replicate data to specified nodes
    pub async fn replicate_shard(&self, shard_id: &str, data: &[u8], target_nodes: &[Uuid]) -> Result<()> {
        let operation_id = format!("repl_{}_{}", shard_id, Utc::now().timestamp_millis());

        tracing::info!("🔄 Starting replication of shard {} to {} nodes", shard_id, target_nodes.len());

        // Create replication operation record
        let operation = ReplicationOperation {
            operation_id: operation_id.clone(),
            shard_id: shard_id.to_string(),
            operation_type: OperationType::InitialReplication,
            source_node: Uuid::new_v4(), // This should be the current node
            target_nodes: target_nodes.to_vec(),
            started_at: Utc::now(),
            completed_at: None,
            status: ReplicationOperationStatus::InProgress,
            error_message: None,
            bytes_transferred: 0,
        };

        // Store operation
        self.active_operations.write().await.insert(operation_id.clone(), operation.clone());

        // Create replicas
        let mut replicas = Vec::new();
        for &node_id in target_nodes {
            let replica = ReplicaInfo {
                node_id,
                shard_id: shard_id.to_string(),
                status: ReplicaStatus::Syncing,
                last_sync: Utc::now(),
                sync_lag_ms: 0,
                data_version: 1,
                size_bytes: data.len() as u64,
                checksum: self.calculate_checksum(data),
            };
            replicas.push(replica);
        }

        // Store replicas
        self.replica_registry.write().await.insert(shard_id.to_string(), replicas);

        // Execute replication based on strategy
        match &self.strategy.strategy_type {
            ReplicationStrategyType::Synchronous => {
                self.synchronous_replication(&operation_id, shard_id, data, target_nodes).await?;
            }
            ReplicationStrategyType::Asynchronous => {
                self.asynchronous_replication(&operation_id, shard_id, data, target_nodes).await?;
            }
            ReplicationStrategyType::SemiSynchronous { quorum_size } => {
                self.semi_synchronous_replication(&operation_id, shard_id, data, target_nodes, *quorum_size).await?;
            }
        }

        tracing::info!("✅ Replication of shard {} completed successfully", shard_id);
        Ok(())
    }

    /// Synchronous replication - wait for all replicas
    async fn synchronous_replication(&self, operation_id: &str, shard_id: &str, data: &[u8], target_nodes: &[Uuid]) -> Result<()> {
        let mut successful_replicas = 0;
        let total_replicas = target_nodes.len();

        for &node_id in target_nodes {
            match self.replicate_to_node(node_id, shard_id, data).await {
                Ok(_) => {
                    successful_replicas += 1;
                    self.update_replica_status(shard_id, node_id, ReplicaStatus::Active).await;
                }
                Err(e) => {
                    tracing::error!("Failed to replicate to node {}: {}", node_id, e);
                    self.update_replica_status(shard_id, node_id, ReplicaStatus::Failed).await;
                }
            }
        }

        if successful_replicas < total_replicas {
            self.mark_operation_failed(operation_id, "Some replicas failed").await;
            return Err(anyhow::anyhow!("Synchronous replication failed: {}/{} replicas successful",
                                     successful_replicas, total_replicas));
        }

        self.mark_operation_completed(operation_id, data.len() as u64).await;
        Ok(())
    }

    /// Asynchronous replication - fire and forget
    async fn asynchronous_replication(&self, operation_id: &str, shard_id: &str, data: &[u8], target_nodes: &[Uuid]) -> Result<()> {
        // Create tasks for background processing
        for &node_id in target_nodes {
            let task = ReplicationTask {
                task_id: format!("task_{}_{}", shard_id, node_id),
                shard_id: shard_id.to_string(),
                operation_type: OperationType::InitialReplication,
                source_node: Uuid::new_v4(), // Current node
                target_nodes: vec![node_id],
                data: data.to_vec(),
                priority: TaskPriority::Normal,
                deadline: None,
            };

            if let Err(e) = self.replication_tx.send(task) {
                tracing::error!("Failed to queue replication task: {}", e);
            }
        }

        self.mark_operation_completed(operation_id, data.len() as u64).await;
        Ok(())
    }

    /// Semi-synchronous replication - wait for quorum
    async fn semi_synchronous_replication(&self, operation_id: &str, shard_id: &str, data: &[u8], target_nodes: &[Uuid], quorum_size: usize) -> Result<()> {
        let mut successful_replicas = 0;
        let required_replicas = quorum_size.min(target_nodes.len());

        // Try to replicate to all nodes but only wait for quorum
        let mut tasks = Vec::new();
        for &node_id in target_nodes {
            let _shard_id = shard_id.to_string();
            let _data = data.to_vec();

            let task = tokio::spawn(async move {
                // Simulate replication to node
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                Ok::<Uuid, anyhow::Error>(node_id)
            });

            tasks.push(task);
        }

        // Wait for quorum
        let mut completed = 0;
        for task in tasks {
            match task.await {
                Ok(Ok(node_id)) => {
                    successful_replicas += 1;
                    completed += 1;
                    self.update_replica_status(shard_id, node_id, ReplicaStatus::Active).await;

                    if completed >= required_replicas {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!("Replication task failed: {}", e);
                }
                Err(e) => {
                    tracing::error!("Task join failed: {}", e);
                }
            }
        }

        if successful_replicas < required_replicas {
            self.mark_operation_failed(operation_id, "Insufficient replicas for quorum").await;
            return Err(anyhow::anyhow!("Semi-synchronous replication failed: {}/{} replicas needed",
                                     successful_replicas, required_replicas));
        }

        self.mark_operation_completed(operation_id, data.len() as u64).await;
        Ok(())
    }

    /// Replicate data to a specific node
    async fn replicate_to_node(&self, node_id: Uuid, shard_id: &str, data: &[u8]) -> Result<()> {
        // In a real implementation, this would:
        // 1. Establish connection to the target node
        // 2. Transfer the data
        // 3. Verify the transfer
        // 4. Update replica status

        tracing::debug!("🔄 Replicating shard {} ({} bytes) to node {}",
                       shard_id, data.len(), node_id);

        // Simulate network transfer
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Simulate occasional failures
        if rand::random::<f64>() < 0.05 { // 5% failure rate
            return Err(anyhow::anyhow!("Simulated network failure"));
        }

        Ok(())
    }

    /// Update replica status
    async fn update_replica_status(&self, shard_id: &str, node_id: Uuid, status: ReplicaStatus) {
        let mut registry = self.replica_registry.write().await;
        if let Some(replicas) = registry.get_mut(shard_id) {
            for replica in replicas {
                if replica.node_id == node_id {
                    replica.status = status;
                    replica.last_sync = Utc::now();
                    break;
                }
            }
        }
    }

    /// Mark operation as completed
    async fn mark_operation_completed(&self, operation_id: &str, bytes_transferred: u64) {
        let mut active_ops = self.active_operations.write().await;
        if let Some(mut operation) = active_ops.remove(operation_id) {
            operation.status = ReplicationOperationStatus::Completed;
            operation.completed_at = Some(Utc::now());
            operation.bytes_transferred = bytes_transferred;

            // Move to history
            self.operation_history.write().await.push(operation);
        }
    }

    /// Mark operation as failed
    async fn mark_operation_failed(&self, operation_id: &str, error_message: &str) {
        let mut active_ops = self.active_operations.write().await;
        if let Some(mut operation) = active_ops.remove(operation_id) {
            operation.status = ReplicationOperationStatus::Failed;
            operation.completed_at = Some(Utc::now());
            operation.error_message = Some(error_message.to_string());

            // Move to history
            self.operation_history.write().await.push(operation);
        }
    }

    /// Get replication status for a shard
    pub async fn get_replication_status(&self, shard_id: &str) -> Option<ReplicationStatus> {
        let registry = self.replica_registry.read().await;
        if let Some(replicas) = registry.get(shard_id) {
            let total = replicas.len();
            let healthy = replicas.iter().filter(|r| r.status == ReplicaStatus::Active).count();
            let failed = replicas.iter().filter(|r| r.status == ReplicaStatus::Failed).count();

            if failed == 0 && healthy == total {
                Some(ReplicationStatus::Healthy)
            } else if healthy >= (total + 1) / 2 {
                Some(ReplicationStatus::Degraded)
            } else {
                Some(ReplicationStatus::Critical)
            }
        } else {
            None
        }
    }

    /// Get replica information for a shard
    pub async fn get_replica_info(&self, shard_id: &str) -> Vec<ReplicaInfo> {
        let registry = self.replica_registry.read().await;
        registry.get(shard_id).cloned().unwrap_or_default()
    }

    /// Repair failed replicas
    pub async fn repair_replicas(&self, shard_id: &str) -> Result<()> {
        let replicas = self.get_replica_info(shard_id).await;
        let failed_replicas: Vec<_> = replicas.iter()
            .filter(|r| r.status == ReplicaStatus::Failed)
            .collect();

        if failed_replicas.is_empty() {
            return Ok(());
        }

        tracing::info!("🔧 Repairing {} failed replicas for shard {}", failed_replicas.len(), shard_id);

        // Find a healthy replica to use as source
        let source_replica = replicas.iter()
            .find(|r| r.status == ReplicaStatus::Active)
            .ok_or_else(|| anyhow::anyhow!("No healthy replica found for repair"))?;

        // Create repair tasks
        for failed_replica in failed_replicas {
            let task = ReplicationTask {
                task_id: format!("repair_{}_{}", shard_id, failed_replica.node_id),
                shard_id: shard_id.to_string(),
                operation_type: OperationType::RepairSync,
                source_node: source_replica.node_id,
                target_nodes: vec![failed_replica.node_id],
                data: Vec::new(), // Would fetch from source
                priority: TaskPriority::High,
                deadline: Some(Utc::now() + chrono::Duration::minutes(30)),
            };

            if let Err(e) = self.replication_tx.send(task) {
                tracing::error!("Failed to queue repair task: {}", e);
            }
        }

        Ok(())
    }

    /// Remove replica from a node
    pub async fn remove_replica(&self, shard_id: &str, node_id: Uuid) -> Result<()> {
        let mut registry = self.replica_registry.write().await;
        if let Some(replicas) = registry.get_mut(shard_id) {
            replicas.retain(|r| r.node_id != node_id);

            if replicas.is_empty() {
                registry.remove(shard_id);
            }
        }

        tracing::info!("🗑️ Removed replica of shard {} from node {}", shard_id, node_id);
        Ok(())
    }

    /// Calculate data checksum
    fn calculate_checksum(&self, data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Update metrics
    pub async fn update_metrics(&self) -> Result<()> {
        let registry = self.replica_registry.read().await;
        let mut total_replicas = 0;
        let mut healthy_replicas = 0;
        let mut degraded_replicas = 0;
        let mut failed_replicas = 0;
        let mut total_lag_ms = 0u64;

        for replicas in registry.values() {
            for replica in replicas {
                total_replicas += 1;
                total_lag_ms += replica.sync_lag_ms;

                match replica.status {
                    ReplicaStatus::Active => healthy_replicas += 1,
                    ReplicaStatus::Syncing | ReplicaStatus::Lagging => degraded_replicas += 1,
                    ReplicaStatus::Failed => failed_replicas += 1,
                    ReplicaStatus::Maintenance => {} // Don't count in health metrics
                }
            }
        }

        let average_lag = if total_replicas > 0 {
            total_lag_ms as f64 / total_replicas as f64
        } else {
            0.0
        };

        let history = self.operation_history.read().await;
        let recent_ops: Vec<_> = history.iter()
            .filter(|op| op.started_at > Utc::now() - chrono::Duration::hours(1))
            .collect();

        let success_rate = if recent_ops.is_empty() {
            1.0
        } else {
            let successful = recent_ops.iter()
                .filter(|op| op.status == ReplicationOperationStatus::Completed)
                .count();
            successful as f64 / recent_ops.len() as f64
        };

        let mut metrics = self.metrics.write().await;
        metrics.total_replicas = total_replicas;
        metrics.healthy_replicas = healthy_replicas;
        metrics.degraded_replicas = degraded_replicas;
        metrics.failed_replicas = failed_replicas;
        metrics.total_shards_replicated = registry.len();
        metrics.average_sync_lag_ms = average_lag;
        metrics.success_rate = success_rate;
        metrics.last_updated = Utc::now();

        Ok(())
    }

    /// Get replication metrics
    pub async fn get_metrics(&self) -> ReplicationMetrics {
        self.metrics.read().await.clone()
    }

    /// Get active operations
    pub async fn get_active_operations(&self) -> Vec<ReplicationOperation> {
        self.active_operations.read().await.values().cloned().collect()
    }

    /// Get recent operation history
    pub async fn get_operation_history(&self, limit: usize) -> Vec<ReplicationOperation> {
        let history = self.operation_history.read().await;
        let start = if history.len() > limit {
            history.len() - limit
        } else {
            0
        };
        history[start..].to_vec()
    }
}