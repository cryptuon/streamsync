//! Data Migration Engine for StreamSync Sharding
//!
//! This module handles shard migration between nodes for load balancing,
//! node maintenance, and cluster optimization.

use super::ShardingConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::sync::{RwLock, mpsc};
use std::sync::Arc;

/// Migration operation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationStatus {
    Planned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Rollback,
}

/// Migration strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStrategy {
    /// Copy data then switch (zero downtime)
    CopyAndSwitch,
    /// Move data with brief downtime
    MoveWithDowntime,
    /// Gradual migration with load balancing
    Gradual { batch_size: u64 },
    /// Emergency migration (fastest possible)
    Emergency,
}

/// Migration operation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationOperation {
    pub operation_id: String,
    pub shard_id: String,
    pub from_node: Uuid,
    pub to_node: Uuid,
    pub strategy: MigrationStrategy,
    pub status: MigrationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress_percentage: f64,
    pub bytes_migrated: u64,
    pub total_bytes: u64,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub rollback_plan: Option<RollbackPlan>,
}

/// Rollback plan for failed migrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    pub rollback_steps: Vec<RollbackStep>,
    pub rollback_timeout: DateTime<Utc>,
    pub data_backup_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackStep {
    pub step_type: RollbackStepType,
    pub description: String,
    pub automated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackStepType {
    RestoreData,
    UpdateRouting,
    NotifyClients,
    CleanupTarget,
}

/// Migration plan for multiple shards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub plan_id: String,
    pub migrations: Vec<MigrationRequest>,
    pub total_operations: usize,
    pub estimated_duration: chrono::Duration,
    pub created_at: DateTime<Utc>,
    pub priority: MigrationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRequest {
    pub shard_id: String,
    pub from_node: Uuid,
    pub to_node: Uuid,
    pub reason: MigrationReason,
    pub urgency: MigrationUrgency,
    pub constraints: MigrationConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationReason {
    LoadBalancing,
    NodeMaintenance,
    NodeFailure,
    PerformanceOptimization,
    StorageRebalancing,
    NetworkOptimization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationUrgency {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationPriority {
    Emergency,
    High,
    Normal,
    Background,
}

/// Migration constraints and preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConstraints {
    pub max_downtime_ms: Option<u64>,
    pub bandwidth_limit_mbps: Option<u32>,
    pub maintenance_window: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub preserve_data_locality: bool,
    pub require_checksum_validation: bool,
}

impl Default for MigrationConstraints {
    fn default() -> Self {
        Self {
            max_downtime_ms: Some(5000), // 5 seconds
            bandwidth_limit_mbps: None,
            maintenance_window: None,
            preserve_data_locality: true,
            require_checksum_validation: true,
        }
    }
}

/// Migration engine for orchestrating shard movements
pub struct MigrationEngine {
    config: ShardingConfig,
    active_migrations: Arc<RwLock<HashMap<String, MigrationOperation>>>,
    migration_history: Arc<RwLock<Vec<MigrationOperation>>>,
    migration_queue: Arc<RwLock<Vec<MigrationRequest>>>,

    // Communication
    migration_tx: mpsc::UnboundedSender<MigrationTask>,

    // Metrics
    metrics: Arc<RwLock<MigrationMetrics>>,
}

/// Internal migration task
#[derive(Debug, Clone)]
struct MigrationTask {
    operation_id: String,
    task_type: MigrationTaskType,
    shard_id: String,
    from_node: Uuid,
    to_node: Uuid,
    data_size: u64,
}

#[derive(Debug, Clone)]
enum MigrationTaskType {
    PrepareTarget,
    CopyData,
    SwitchRouting,
    VerifyMigration,
    CleanupSource,
    Rollback,
}

/// Migration metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationMetrics {
    pub active_migrations: usize,
    pub completed_migrations: usize,
    pub failed_migrations: usize,
    pub total_bytes_migrated: u64,
    pub average_migration_time_ms: f64,
    pub success_rate: f64,
    pub current_throughput_mbps: f64,
    pub queue_length: usize,
    pub last_updated: DateTime<Utc>,
}

impl MigrationEngine {
    /// Create a new migration engine
    pub fn new(config: ShardingConfig) -> Result<Self> {
        let (migration_tx, _migration_rx) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            active_migrations: Arc::new(RwLock::new(HashMap::new())),
            migration_history: Arc::new(RwLock::new(Vec::new())),
            migration_queue: Arc::new(RwLock::new(Vec::new())),
            migration_tx,
            metrics: Arc::new(RwLock::new(MigrationMetrics {
                active_migrations: 0,
                completed_migrations: 0,
                failed_migrations: 0,
                total_bytes_migrated: 0,
                average_migration_time_ms: 0.0,
                success_rate: 1.0,
                current_throughput_mbps: 0.0,
                queue_length: 0,
                last_updated: Utc::now(),
            })),
        })
    }

    /// Migrate a shard from one node to another
    pub async fn migrate_shard(&self, shard_id: &str, from_node: Uuid, to_node: Uuid) -> Result<String> {
        let operation_id = format!("mig_{}_{}", shard_id, Utc::now().timestamp_millis());

        tracing::info!("🚚 Starting migration of shard {} from {} to {}", shard_id, from_node, to_node);

        // Create migration operation
        let operation = MigrationOperation {
            operation_id: operation_id.clone(),
            shard_id: shard_id.to_string(),
            from_node,
            to_node,
            strategy: MigrationStrategy::CopyAndSwitch,
            status: MigrationStatus::Planned,
            started_at: Utc::now(),
            completed_at: None,
            progress_percentage: 0.0,
            bytes_migrated: 0,
            total_bytes: 0, // Will be determined during migration
            estimated_completion: None,
            error_message: None,
            rollback_plan: Some(self.create_rollback_plan(shard_id, from_node, to_node)),
        };

        // Store operation
        self.active_migrations.write().await.insert(operation_id.clone(), operation);

        // Execute migration
        self.execute_migration(&operation_id).await?;

        Ok(operation_id)
    }

    /// Execute a migration operation
    async fn execute_migration(&self, operation_id: &str) -> Result<()> {
        // Update status to in progress
        self.update_migration_status(operation_id, MigrationStatus::InProgress).await;

        let operation = {
            let migrations = self.active_migrations.read().await;
            migrations.get(operation_id).cloned()
                .ok_or_else(|| anyhow::anyhow!("Migration operation not found: {}", operation_id))?
        };

        match operation.strategy {
            MigrationStrategy::CopyAndSwitch => {
                self.execute_copy_and_switch(&operation).await
            }
            MigrationStrategy::MoveWithDowntime => {
                self.execute_move_with_downtime(&operation).await
            }
            MigrationStrategy::Gradual { batch_size } => {
                self.execute_gradual_migration(&operation, batch_size).await
            }
            MigrationStrategy::Emergency => {
                self.execute_emergency_migration(&operation).await
            }
        }
    }

    /// Execute copy-and-switch migration
    async fn execute_copy_and_switch(&self, operation: &MigrationOperation) -> Result<()> {
        let operation_id = &operation.operation_id;

        // Step 1: Prepare target node
        self.update_migration_progress(operation_id, 10.0, "Preparing target node").await;
        self.prepare_target_node(&operation.shard_id, operation.to_node).await?;

        // Step 2: Copy data to target
        self.update_migration_progress(operation_id, 30.0, "Copying data").await;
        let data_size = self.copy_shard_data(&operation.shard_id, operation.from_node, operation.to_node).await?;

        // Step 3: Synchronize any changes
        self.update_migration_progress(operation_id, 70.0, "Synchronizing changes").await;
        self.synchronize_changes(&operation.shard_id, operation.from_node, operation.to_node).await?;

        // Step 4: Switch routing (atomic operation)
        self.update_migration_progress(operation_id, 90.0, "Switching routing").await;
        self.switch_shard_routing(&operation.shard_id, operation.from_node, operation.to_node).await?;

        // Step 5: Cleanup source
        self.update_migration_progress(operation_id, 95.0, "Cleaning up source").await;
        self.cleanup_source_data(&operation.shard_id, operation.from_node).await?;

        // Step 6: Verify migration
        self.update_migration_progress(operation_id, 100.0, "Verifying migration").await;
        self.verify_migration(&operation.shard_id, operation.to_node).await?;

        self.complete_migration(operation_id, data_size).await;
        Ok(())
    }

    /// Execute move with downtime migration
    async fn execute_move_with_downtime(&self, operation: &MigrationOperation) -> Result<()> {
        // This would involve:
        // 1. Mark shard as unavailable
        // 2. Copy data quickly
        // 3. Update routing
        // 4. Mark shard as available

        self.complete_migration(&operation.operation_id, 0).await;
        Ok(())
    }

    /// Execute gradual migration
    async fn execute_gradual_migration(&self, operation: &MigrationOperation, _batch_size: u64) -> Result<()> {
        // This would involve:
        // 1. Migrate data in batches
        // 2. Update routing incrementally
        // 3. Monitor performance impact

        self.complete_migration(&operation.operation_id, 0).await;
        Ok(())
    }

    /// Execute emergency migration
    async fn execute_emergency_migration(&self, operation: &MigrationOperation) -> Result<()> {
        // This would involve:
        // 1. Skip non-essential validation
        // 2. Use maximum bandwidth
        // 3. Parallelize operations

        self.complete_migration(&operation.operation_id, 0).await;
        Ok(())
    }

    /// Prepare target node for receiving shard
    async fn prepare_target_node(&self, shard_id: &str, target_node: Uuid) -> Result<()> {
        tracing::debug!("📦 Preparing target node {} for shard {}", target_node, shard_id);

        // In a real implementation:
        // 1. Check node capacity
        // 2. Create shard directory
        // 3. Set up replication streams
        // 4. Prepare metadata structures

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    /// Copy shard data from source to target
    async fn copy_shard_data(&self, shard_id: &str, from_node: Uuid, to_node: Uuid) -> Result<u64> {
        tracing::debug!("📋 Copying shard {} from {} to {}", shard_id, from_node, to_node);

        // Simulate data copy with progress updates
        let total_size = 1024 * 1024 * 100; // 100MB
        let chunk_size = 1024 * 1024; // 1MB chunks
        let mut copied = 0;

        while copied < total_size {
            // Simulate copying a chunk
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            copied += chunk_size;

            let _progress = (copied as f64 / total_size as f64) * 40.0 + 30.0; // 30-70%
            // Update progress would be called here in real implementation
        }

        Ok(total_size)
    }

    /// Synchronize any changes that occurred during copy
    async fn synchronize_changes(&self, shard_id: &str, from_node: Uuid, to_node: Uuid) -> Result<()> {
        tracing::debug!("🔄 Synchronizing changes for shard {} from {} to {}", shard_id, from_node, to_node);

        // In a real implementation:
        // 1. Apply transaction log since copy started
        // 2. Ensure data consistency
        // 3. Prepare for atomic switch

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        Ok(())
    }

    /// Switch routing from source to target (atomic operation)
    async fn switch_shard_routing(&self, shard_id: &str, from_node: Uuid, to_node: Uuid) -> Result<()> {
        tracing::debug!("🔀 Switching routing for shard {} from {} to {}", shard_id, from_node, to_node);

        // In a real implementation:
        // 1. Update routing tables atomically
        // 2. Notify all nodes of the change
        // 3. Wait for acknowledgments

        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        Ok(())
    }

    /// Cleanup source data after successful migration
    async fn cleanup_source_data(&self, shard_id: &str, source_node: Uuid) -> Result<()> {
        tracing::debug!("🗑️ Cleaning up source data for shard {} on node {}", shard_id, source_node);

        // In a real implementation:
        // 1. Remove shard data from source
        // 2. Update local metadata
        // 3. Free up storage space

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        Ok(())
    }

    /// Verify migration completed successfully
    async fn verify_migration(&self, shard_id: &str, target_node: Uuid) -> Result<()> {
        tracing::debug!("✅ Verifying migration for shard {} on node {}", shard_id, target_node);

        // In a real implementation:
        // 1. Verify data integrity
        // 2. Check routing is working
        // 3. Validate performance

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        Ok(())
    }

    /// Update migration progress
    async fn update_migration_progress(&self, operation_id: &str, percentage: f64, message: &str) {
        let mut migrations = self.active_migrations.write().await;
        if let Some(operation) = migrations.get_mut(operation_id) {
            operation.progress_percentage = percentage;

            // Estimate completion time
            if percentage > 0.0 {
                let elapsed = Utc::now() - operation.started_at;
                let total_estimated = elapsed.num_milliseconds() as f64 / (percentage / 100.0);
                let remaining_ms = total_estimated - elapsed.num_milliseconds() as f64;

                if remaining_ms > 0.0 {
                    operation.estimated_completion = Some(Utc::now() + chrono::Duration::milliseconds(remaining_ms as i64));
                }
            }
        }

        tracing::debug!("📊 Migration {} progress: {:.1}% - {}", operation_id, percentage, message);
    }

    /// Update migration status
    async fn update_migration_status(&self, operation_id: &str, status: MigrationStatus) {
        let mut migrations = self.active_migrations.write().await;
        if let Some(operation) = migrations.get_mut(operation_id) {
            operation.status = status;
        }
    }

    /// Complete a migration operation
    async fn complete_migration(&self, operation_id: &str, bytes_migrated: u64) {
        let mut migrations = self.active_migrations.write().await;
        if let Some(mut operation) = migrations.remove(operation_id) {
            operation.status = MigrationStatus::Completed;
            operation.completed_at = Some(Utc::now());
            operation.bytes_migrated = bytes_migrated;
            operation.progress_percentage = 100.0;

            // Move to history
            self.migration_history.write().await.push(operation);
        }

        tracing::info!("✅ Migration {} completed successfully", operation_id);
    }

    /// Fail a migration operation
    async fn fail_migration(&self, operation_id: &str, error_message: &str) {
        let mut migrations = self.active_migrations.write().await;
        if let Some(mut operation) = migrations.remove(operation_id) {
            operation.status = MigrationStatus::Failed;
            operation.completed_at = Some(Utc::now());
            operation.error_message = Some(error_message.to_string());

            // Move to history
            self.migration_history.write().await.push(operation);
        }

        tracing::error!("❌ Migration {} failed: {}", operation_id, error_message);
    }

    /// Create rollback plan for migration
    fn create_rollback_plan(&self, shard_id: &str, from_node: Uuid, to_node: Uuid) -> RollbackPlan {
        RollbackPlan {
            rollback_steps: vec![
                RollbackStep {
                    step_type: RollbackStepType::RestoreData,
                    description: format!("Restore shard {} data on node {}", shard_id, from_node),
                    automated: true,
                },
                RollbackStep {
                    step_type: RollbackStepType::UpdateRouting,
                    description: format!("Revert routing for shard {} to node {}", shard_id, from_node),
                    automated: true,
                },
                RollbackStep {
                    step_type: RollbackStepType::CleanupTarget,
                    description: format!("Clean up incomplete data on node {}", to_node),
                    automated: true,
                },
                RollbackStep {
                    step_type: RollbackStepType::NotifyClients,
                    description: "Notify clients of rollback completion".to_string(),
                    automated: false,
                },
            ],
            rollback_timeout: Utc::now() + chrono::Duration::hours(1),
            data_backup_location: Some(format!("backup/shard_{}", shard_id)),
        }
    }

    /// Get migration status
    pub async fn get_migration_status(&self, operation_id: &str) -> Option<MigrationOperation> {
        self.active_migrations.read().await.get(operation_id).cloned()
    }

    /// Get all active migrations
    pub async fn get_active_migrations(&self) -> Vec<MigrationOperation> {
        self.active_migrations.read().await.values().cloned().collect()
    }

    /// Get migration history
    pub async fn get_migration_history(&self, limit: usize) -> Vec<MigrationOperation> {
        let history = self.migration_history.read().await;
        let start = if history.len() > limit {
            history.len() - limit
        } else {
            0
        };
        history[start..].to_vec()
    }

    /// Cancel a migration
    pub async fn cancel_migration(&self, operation_id: &str) -> Result<()> {
        let mut migrations = self.active_migrations.write().await;
        if let Some(mut operation) = migrations.remove(operation_id) {
            operation.status = MigrationStatus::Cancelled;
            operation.completed_at = Some(Utc::now());

            // Move to history
            self.migration_history.write().await.push(operation);

            tracing::info!("🚫 Migration {} cancelled", operation_id);
        }

        Ok(())
    }

    /// Get migration metrics
    pub async fn get_metrics(&self) -> MigrationMetrics {
        self.metrics.read().await.clone()
    }

    /// Update metrics
    pub async fn update_metrics(&self) -> Result<()> {
        let active_count = self.active_migrations.read().await.len();
        let queue_length = self.migration_queue.read().await.len();

        let history = self.migration_history.read().await;
        let completed_count = history.iter()
            .filter(|op| op.status == MigrationStatus::Completed)
            .count();
        let failed_count = history.iter()
            .filter(|op| op.status == MigrationStatus::Failed)
            .count();

        let success_rate = if (completed_count + failed_count) > 0 {
            completed_count as f64 / (completed_count + failed_count) as f64
        } else {
            1.0
        };

        let total_bytes = history.iter()
            .map(|op| op.bytes_migrated)
            .sum();

        let mut metrics = self.metrics.write().await;
        metrics.active_migrations = active_count;
        metrics.completed_migrations = completed_count;
        metrics.failed_migrations = failed_count;
        metrics.total_bytes_migrated = total_bytes;
        metrics.success_rate = success_rate;
        metrics.queue_length = queue_length;
        metrics.last_updated = Utc::now();

        Ok(())
    }
}