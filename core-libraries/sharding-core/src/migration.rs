//! Data migration for shard rebalancing

use crate::{NodeId, Result, ShardConfig, ShardError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Unique identifier for a migration operation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MigrationId(Uuid);

impl MigrationId {
    /// Create a new migration ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for MigrationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MigrationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a migration operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// Migration is planned but not started
    Planned,
    /// Migration is currently in progress
    InProgress,
    /// Migration completed successfully
    Completed,
    /// Migration failed
    Failed,
    /// Migration was cancelled
    Cancelled,
}

/// A single migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// Unique migration identifier
    pub id: MigrationId,
    /// Hash range to migrate (start, end)
    pub range: (u64, u64),
    /// Source node
    pub source_node: NodeId,
    /// Destination node
    pub destination_node: NodeId,
    /// Current status
    pub status: MigrationStatus,
    /// When the migration was created
    pub created_at: std::time::SystemTime,
    /// When the migration started
    pub started_at: Option<std::time::SystemTime>,
    /// When the migration completed
    pub completed_at: Option<std::time::SystemTime>,
    /// Number of keys migrated so far
    pub keys_migrated: u64,
    /// Total number of keys to migrate (if known)
    pub total_keys: Option<u64>,
    /// Error message if failed
    pub error_message: Option<String>,
}

impl Migration {
    /// Create a new migration
    pub fn new(
        range: (u64, u64),
        source_node: NodeId,
        destination_node: NodeId,
    ) -> Self {
        Self {
            id: MigrationId::new(),
            range,
            source_node,
            destination_node,
            status: MigrationStatus::Planned,
            created_at: std::time::SystemTime::now(),
            started_at: None,
            completed_at: None,
            keys_migrated: 0,
            total_keys: None,
            error_message: None,
        }
    }

    /// Mark migration as started
    pub fn start(&mut self) {
        self.status = MigrationStatus::InProgress;
        self.started_at = Some(std::time::SystemTime::now());
    }

    /// Mark migration as completed
    pub fn complete(&mut self) {
        self.status = MigrationStatus::Completed;
        self.completed_at = Some(std::time::SystemTime::now());
    }

    /// Mark migration as failed
    pub fn fail(&mut self, error: String) {
        self.status = MigrationStatus::Failed;
        self.completed_at = Some(std::time::SystemTime::now());
        self.error_message = Some(error);
    }

    /// Update migration progress
    pub fn update_progress(&mut self, keys_migrated: u64, total_keys: Option<u64>) {
        self.keys_migrated = keys_migrated;
        if let Some(total) = total_keys {
            self.total_keys = Some(total);
        }
    }

    /// Get migration progress as a percentage (0.0 to 1.0)
    pub fn progress(&self) -> Option<f64> {
        self.total_keys.map(|total| {
            if total == 0 {
                1.0
            } else {
                self.keys_migrated as f64 / total as f64
            }
        })
    }

    /// Get migration duration
    pub fn duration(&self) -> Option<Duration> {
        if let (Some(started), Some(completed)) = (self.started_at, self.completed_at) {
            completed.duration_since(started).ok()
        } else if let Some(started) = self.started_at {
            std::time::SystemTime::now().duration_since(started).ok()
        } else {
            None
        }
    }

    /// Check if migration is active (in progress)
    pub fn is_active(&self) -> bool {
        self.status == MigrationStatus::InProgress
    }

    /// Check if migration is finished (completed, failed, or cancelled)
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            MigrationStatus::Completed | MigrationStatus::Failed | MigrationStatus::Cancelled
        )
    }
}

/// Migration plan for rebalancing
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    /// List of migrations to execute
    pub migrations: Vec<Migration>,
    /// Estimated total time for all migrations
    pub estimated_duration: Duration,
    /// Priority of this plan (higher = more urgent)
    pub priority: u8,
}

impl MigrationPlan {
    /// Create a new migration plan
    pub fn new(migrations: Vec<Migration>) -> Self {
        // Estimate duration based on number of migrations
        let estimated_duration = Duration::from_secs(migrations.len() as u64 * 60); // 1 minute per migration estimate

        Self {
            migrations,
            estimated_duration,
            priority: 0,
        }
    }

    /// Set plan priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Check if plan is empty
    pub fn is_empty(&self) -> bool {
        self.migrations.is_empty()
    }

    /// Get total number of migrations
    pub fn migration_count(&self) -> usize {
        self.migrations.len()
    }
}

/// Manages shard migrations
pub struct ShardMigrator {
    /// Configuration
    config: ShardConfig,
    /// Active migrations
    active_migrations: Arc<RwLock<HashMap<MigrationId, Migration>>>,
    /// Migration history
    migration_history: Arc<RwLock<Vec<Migration>>>,
    /// Current migration plan
    current_plan: Arc<Mutex<Option<MigrationPlan>>>,
    /// Migration executor
    executor: Arc<Mutex<Option<MigrationExecutor>>>,
}

impl ShardMigrator {
    /// Create a new shard migrator
    pub fn new(config: ShardConfig) -> Self {
        Self {
            config,
            active_migrations: Arc::new(RwLock::new(HashMap::new())),
            migration_history: Arc::new(RwLock::new(Vec::new())),
            current_plan: Arc::new(Mutex::new(None)),
            executor: Arc::new(Mutex::new(None)),
        }
    }

    /// Plan migration for a hash range
    pub async fn plan_range_migration(
        &self,
        start: u64,
        end: u64,
        source_node: NodeId,
    ) -> Result<MigrationId> {
        // For now, create a simple migration plan
        // In a real implementation, this would:
        // 1. Find the best destination node
        // 2. Estimate the amount of data to migrate
        // 3. Schedule the migration appropriately

        let destination_node = self.find_best_destination_node(&source_node).await?;
        let migration = Migration::new((start, end), source_node, destination_node);
        let migration_id = migration.id.clone();

        let mut active = self.active_migrations.write().await;
        active.insert(migration_id.clone(), migration);

        info!("Planned migration {} for range {}-{}", migration_id, start, end);
        Ok(migration_id)
    }

    /// Create a migration plan for rebalancing
    pub async fn create_rebalance_plan(
        &self,
        node_distribution: &HashMap<NodeId, Vec<(u64, u64)>>,
        target_distribution: &HashMap<NodeId, Vec<(u64, u64)>>,
    ) -> Result<MigrationPlan> {
        let mut migrations = Vec::new();

        // Find ranges that need to be moved
        for (source_node, source_ranges) in node_distribution {
            for &(start, end) in source_ranges {
                // Check if this range should be on a different node
                if let Some(target_node) = self.find_target_node_for_range(start, end, target_distribution).await {
                    if target_node != *source_node {
                        let migration = Migration::new((start, end), source_node.clone(), target_node);
                        migrations.push(migration);
                    }
                }
            }
        }

        let plan = MigrationPlan::new(migrations);
        info!("Created rebalance plan with {} migrations", plan.migration_count());

        Ok(plan)
    }

    /// Execute a migration plan
    pub async fn execute_plan(self: &Arc<Self>, plan: MigrationPlan) -> Result<()> {
        if plan.is_empty() {
            return Ok(());
        }

        // Check if we're already executing a plan
        let mut current_plan = self.current_plan.lock().await;
        if current_plan.is_some() {
            return Err(ShardError::MigrationInProgress {
                start: 0,
                end: u64::MAX,
            });
        }

        *current_plan = Some(plan.clone());
        drop(current_plan);

        info!("Starting execution of migration plan with {} migrations", plan.migration_count());

        // Execute migrations with concurrency limit
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_migrations));
        let mut handles = Vec::new();

        for migration in plan.migrations {
            let semaphore = semaphore.clone();
            let migrator = self.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                migrator.execute_migration(migration).await
            });

            handles.push(handle);
        }

        // Wait for all migrations to complete
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!("Migration task panicked: {}", e);
                    results.push(Err(ShardError::MigrationFailed {
                        reason: format!("Task panicked: {}", e),
                    }));
                }
            }
        }

        // Clear current plan
        let mut current_plan = self.current_plan.lock().await;
        *current_plan = None;

        // Check results
        let failed_migrations = results.iter().filter(|r| r.is_err()).count();
        if failed_migrations > 0 {
            warn!("Migration plan completed with {} failures", failed_migrations);
            return Err(ShardError::MigrationFailed {
                reason: format!("{} migrations failed", failed_migrations),
            });
        }

        info!("Migration plan completed successfully");
        Ok(())
    }

    /// Execute a single migration
    async fn execute_migration(&self, mut migration: Migration) -> Result<()> {
        let migration_id = migration.id.clone();
        info!("Starting migration {}", migration_id);

        // Update migration status
        migration.start();
        self.update_migration_status(migration_id.clone(), migration.clone()).await;

        // Simulate migration execution
        // In a real implementation, this would:
        // 1. Connect to source and destination nodes
        // 2. Stream data from source to destination
        // 3. Verify data integrity
        // 4. Update routing tables
        // 5. Clean up source data

        let result = self.perform_migration_work(&mut migration).await;

        match &result {
            Ok(()) => {
                migration.complete();
                info!("Migration {} completed successfully", migration_id);
            }
            Err(e) => {
                migration.fail(e.to_string());
                error!("Migration {} failed: {}", migration_id, e);
            }
        }

        // Update final status and move to history
        self.finalize_migration(migration).await;

        result
    }

    /// Perform the actual migration work
    async fn perform_migration_work(&self, migration: &mut Migration) -> Result<()> {
        let batch_size = self.config.migration_batch_size;
        let timeout = self.config.migration_timeout();
        let start_time = Instant::now();

        // Simulate migrating data in batches
        let estimated_keys = 1000; // In real implementation, this would be calculated
        migration.total_keys = Some(estimated_keys);

        for batch in 0..(estimated_keys / batch_size as u64 + 1) {
            // Check timeout
            if start_time.elapsed() > timeout {
                return Err(ShardError::MigrationTimeout {
                    timeout_ms: timeout.as_millis() as u64,
                });
            }

            // Simulate batch migration
            tokio::time::sleep(Duration::from_millis(10)).await;

            let keys_in_batch = std::cmp::min(batch_size as u64, estimated_keys - batch * batch_size as u64);
            migration.keys_migrated += keys_in_batch;

            // Update progress periodically
            if batch % 10 == 0 {
                self.update_migration_status(migration.id.clone(), migration.clone()).await;
                debug!("Migration {} progress: {}/{}", migration.id, migration.keys_migrated, estimated_keys);
            }

            if migration.keys_migrated >= estimated_keys {
                break;
            }
        }

        Ok(())
    }

    /// Update migration status in active migrations
    async fn update_migration_status(&self, migration_id: MigrationId, migration: Migration) {
        let mut active = self.active_migrations.write().await;
        active.insert(migration_id, migration);
    }

    /// Move completed migration to history
    async fn finalize_migration(&self, migration: Migration) {
        let migration_id = migration.id.clone();

        // Remove from active migrations
        let mut active = self.active_migrations.write().await;
        active.remove(&migration_id);
        drop(active);

        // Add to history
        let mut history = self.migration_history.write().await;
        history.push(migration);

        // Limit history size
        const MAX_HISTORY_SIZE: usize = 1000;
        if history.len() > MAX_HISTORY_SIZE {
            history.remove(0);
        }
    }

    /// Get active migrations
    pub async fn get_active_migrations(&self) -> HashMap<MigrationId, Migration> {
        self.active_migrations.read().await.clone()
    }

    /// Get migration by ID
    pub async fn get_migration(&self, migration_id: &MigrationId) -> Option<Migration> {
        let active = self.active_migrations.read().await;
        if let Some(migration) = active.get(migration_id) {
            return Some(migration.clone());
        }

        // Check history
        let history = self.migration_history.read().await;
        history.iter().find(|m| m.id == *migration_id).cloned()
    }

    /// Cancel a migration
    pub async fn cancel_migration(&self, migration_id: &MigrationId) -> Result<()> {
        let mut active = self.active_migrations.write().await;

        if let Some(mut migration) = active.remove(migration_id) {
            migration.status = MigrationStatus::Cancelled;
            migration.completed_at = Some(std::time::SystemTime::now());

            // Move to history
            drop(active);
            let mut history = self.migration_history.write().await;
            history.push(migration);

            info!("Cancelled migration {}", migration_id);
            Ok(())
        } else {
            Err(ShardError::InvalidState {
                reason: format!("Migration {} not found or not active", migration_id),
            })
        }
    }

    /// Get migration statistics
    pub async fn get_migration_stats(&self) -> MigrationStats {
        let active = self.active_migrations.read().await;
        let history = self.migration_history.read().await;

        let active_count = active.len();
        let completed_count = history.iter().filter(|m| m.status == MigrationStatus::Completed).count();
        let failed_count = history.iter().filter(|m| m.status == MigrationStatus::Failed).count();
        let cancelled_count = history.iter().filter(|m| m.status == MigrationStatus::Cancelled).count();

        // Calculate average duration for completed migrations
        let completed_durations: Vec<Duration> = history
            .iter()
            .filter(|m| m.status == MigrationStatus::Completed)
            .filter_map(|m| m.duration())
            .collect();

        let avg_duration = if completed_durations.is_empty() {
            Duration::from_secs(0)
        } else {
            let total_secs: u64 = completed_durations.iter().map(|d| d.as_secs()).sum();
            Duration::from_secs(total_secs / completed_durations.len() as u64)
        };

        MigrationStats {
            active_count,
            completed_count,
            failed_count,
            cancelled_count,
            average_duration: avg_duration,
        }
    }

    /// Find the best destination node for a migration
    async fn find_best_destination_node(&self, _source_node: &NodeId) -> Result<NodeId> {
        // Placeholder implementation
        // In a real system, this would consider:
        // - Node capacity and load
        // - Network topology
        // - Geographic distribution
        // - Current migration load

        Ok(NodeId::new("destination-node"))
    }

    /// Find target node for a hash range in the target distribution
    async fn find_target_node_for_range(
        &self,
        start: u64,
        end: u64,
        target_distribution: &HashMap<NodeId, Vec<(u64, u64)>>,
    ) -> Option<NodeId> {
        for (node_id, ranges) in target_distribution {
            for &(range_start, range_end) in ranges {
                if start >= range_start && end <= range_end {
                    return Some(node_id.clone());
                }
            }
        }
        None
    }
}

/// Migration statistics
#[derive(Debug, Clone)]
pub struct MigrationStats {
    pub active_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub cancelled_count: usize,
    pub average_duration: Duration,
}

/// Migration executor for handling the actual data transfer
struct MigrationExecutor {
    // Placeholder for migration execution logic
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_creation() {
        let source = NodeId::new("source");
        let dest = NodeId::new("dest");
        let migration = Migration::new((100, 200), source.clone(), dest.clone());

        assert_eq!(migration.range, (100, 200));
        assert_eq!(migration.source_node, source);
        assert_eq!(migration.destination_node, dest);
        assert_eq!(migration.status, MigrationStatus::Planned);
        assert_eq!(migration.keys_migrated, 0);
        assert!(migration.total_keys.is_none());
    }

    #[test]
    fn test_migration_lifecycle() {
        let source = NodeId::new("source");
        let dest = NodeId::new("dest");
        let mut migration = Migration::new((100, 200), source, dest);

        // Start migration
        migration.start();
        assert_eq!(migration.status, MigrationStatus::InProgress);
        assert!(migration.started_at.is_some());
        assert!(migration.is_active());
        assert!(!migration.is_finished());

        // Update progress
        migration.update_progress(50, Some(100));
        assert_eq!(migration.keys_migrated, 50);
        assert_eq!(migration.total_keys, Some(100));
        assert_eq!(migration.progress(), Some(0.5));

        // Complete migration
        migration.complete();
        assert_eq!(migration.status, MigrationStatus::Completed);
        assert!(migration.completed_at.is_some());
        assert!(!migration.is_active());
        assert!(migration.is_finished());
        assert!(migration.duration().is_some());
    }

    #[test]
    fn test_migration_failure() {
        let source = NodeId::new("source");
        let dest = NodeId::new("dest");
        let mut migration = Migration::new((100, 200), source, dest);

        migration.start();
        migration.fail("Test error".to_string());

        assert_eq!(migration.status, MigrationStatus::Failed);
        assert_eq!(migration.error_message, Some("Test error".to_string()));
        assert!(migration.is_finished());
    }

    #[test]
    fn test_migration_plan() {
        let source = NodeId::new("source");
        let dest = NodeId::new("dest");
        let migration1 = Migration::new((100, 200), source.clone(), dest.clone());
        let migration2 = Migration::new((300, 400), source, dest);

        let plan = MigrationPlan::new(vec![migration1, migration2]).with_priority(5);

        assert_eq!(plan.migration_count(), 2);
        assert!(!plan.is_empty());
        assert_eq!(plan.priority, 5);
    }

    #[tokio::test]
    async fn test_migrator_creation() {
        let config = ShardConfig::test_config();
        let migrator = ShardMigrator::new(config);

        let active = migrator.get_active_migrations().await;
        assert!(active.is_empty());

        let stats = migrator.get_migration_stats().await;
        assert_eq!(stats.active_count, 0);
        assert_eq!(stats.completed_count, 0);
    }

    #[tokio::test]
    async fn test_range_migration_planning() {
        let config = ShardConfig::test_config();
        let migrator = ShardMigrator::new(config);

        let source_node = NodeId::new("source");
        let migration_id = migrator.plan_range_migration(100, 200, source_node.clone()).await.unwrap();

        let migration = migrator.get_migration(&migration_id).await.unwrap();
        assert_eq!(migration.range, (100, 200));
        assert_eq!(migration.source_node, source_node);
        assert_eq!(migration.status, MigrationStatus::Planned);

        let active = migrator.get_active_migrations().await;
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_migration_cancellation() {
        let config = ShardConfig::test_config();
        let migrator = ShardMigrator::new(config);

        let source_node = NodeId::new("source");
        let migration_id = migrator.plan_range_migration(100, 200, source_node).await.unwrap();

        // Cancel the migration
        assert!(migrator.cancel_migration(&migration_id).await.is_ok());

        // Check that it's no longer active
        let active = migrator.get_active_migrations().await;
        assert!(active.is_empty());

        // Check that it's in history with cancelled status
        let migration = migrator.get_migration(&migration_id).await.unwrap();
        assert_eq!(migration.status, MigrationStatus::Cancelled);
    }
}