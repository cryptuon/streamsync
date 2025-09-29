//! Metrics collection for sharding operations

use crate::manager::ClusterStats;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// Comprehensive metrics for shard operations
#[derive(Debug)]
pub struct ShardMetrics {
    /// Node operations metrics
    node_ops: NodeOperationsMetrics,
    /// Migration metrics
    migration: MigrationMetrics,
    /// Replication metrics
    replication: ReplicationMetrics,
    /// Performance metrics
    performance: PerformanceMetrics,
    /// Error metrics
    errors: ErrorMetrics,
    /// Cluster state metrics
    cluster: Arc<RwLock<ClusterMetrics>>,
    /// Metrics collection interval
    collection_interval: Duration,
}

impl ShardMetrics {
    /// Create new metrics collector
    pub fn new(collection_interval: Duration) -> Self {
        Self {
            node_ops: NodeOperationsMetrics::new(),
            migration: MigrationMetrics::new(),
            replication: ReplicationMetrics::new(),
            performance: PerformanceMetrics::new(),
            errors: ErrorMetrics::new(),
            cluster: Arc::new(RwLock::new(ClusterMetrics::new())),
            collection_interval,
        }
    }

    /// Record a node being added
    pub async fn record_node_added(&self) {
        self.node_ops.nodes_added.fetch_add(1, Ordering::Relaxed);
        self.record_event("node_added").await;
    }

    /// Record a node being removed
    pub async fn record_node_removed(&self) {
        self.node_ops.nodes_removed.fetch_add(1, Ordering::Relaxed);
        self.record_event("node_removed").await;
    }

    /// Record a rebalance being triggered
    pub async fn record_rebalance_triggered(&self) {
        self.node_ops.rebalances_triggered.fetch_add(1, Ordering::Relaxed);
        self.record_event("rebalance_triggered").await;
    }

    /// Record a migration starting
    pub async fn record_migration_started(&self) {
        self.migration.migrations_started.fetch_add(1, Ordering::Relaxed);
        self.migration.active_migrations.fetch_add(1, Ordering::Relaxed);
        self.record_event("migration_started").await;
    }

    /// Record a migration completing successfully
    pub async fn record_migration_completed(&self, duration: Duration) {
        self.migration.migrations_completed.fetch_add(1, Ordering::Relaxed);
        self.migration.active_migrations.fetch_sub(1, Ordering::Relaxed);
        self.migration.update_duration(duration).await;
        self.record_event("migration_completed").await;
    }

    /// Record a migration failing
    pub async fn record_migration_failed(&self, duration: Duration) {
        self.migration.migrations_failed.fetch_add(1, Ordering::Relaxed);
        self.migration.active_migrations.fetch_sub(1, Ordering::Relaxed);
        self.migration.update_duration(duration).await;
        self.record_event("migration_failed").await;
    }

    /// Record keys migrated
    pub async fn record_keys_migrated(&self, count: u64) {
        self.migration.keys_migrated.fetch_add(count, Ordering::Relaxed);
    }

    /// Record bytes migrated
    pub async fn record_bytes_migrated(&self, bytes: u64) {
        self.migration.bytes_migrated.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a replication operation
    pub async fn record_replication_write(&self, replicas: usize, successful: usize) {
        self.replication.write_operations.fetch_add(1, Ordering::Relaxed);
        self.replication.total_write_replicas.fetch_add(replicas as u64, Ordering::Relaxed);
        self.replication.successful_write_replicas.fetch_add(successful as u64, Ordering::Relaxed);
    }

    /// Record a replication read
    pub async fn record_replication_read(&self, replicas: usize, successful: usize) {
        self.replication.read_operations.fetch_add(1, Ordering::Relaxed);
        self.replication.total_read_replicas.fetch_add(replicas as u64, Ordering::Relaxed);
        self.replication.successful_read_replicas.fetch_add(successful as u64, Ordering::Relaxed);
    }

    /// Record quorum not achieved
    pub async fn record_quorum_failure(&self) {
        self.replication.quorum_failures.fetch_add(1, Ordering::Relaxed);
        self.record_error("quorum_failure").await;
    }

    /// Record performance metrics
    pub async fn record_operation_latency(&self, operation: &str, latency: Duration) {
        self.performance.record_latency(operation, latency).await;
    }

    /// Record throughput
    pub async fn record_throughput(&self, operations_per_second: f64) {
        self.performance.update_throughput(operations_per_second).await;
    }

    /// Record an error
    pub async fn record_error(&self, error_type: &str) {
        self.errors.record_error(error_type).await;
    }

    /// Update cluster statistics
    pub async fn update_cluster_stats(&self, stats: ClusterStats) {
        let mut cluster = self.cluster.write().await;
        cluster.update(stats).await;
    }

    /// Get current metrics snapshot
    pub async fn get_snapshot(&self) -> MetricsSnapshot {
        let cluster = self.cluster.read().await;

        MetricsSnapshot {
            timestamp: SystemTime::now(),
            node_operations: self.node_ops.get_snapshot(),
            migration: self.migration.get_snapshot().await,
            replication: self.replication.get_snapshot(),
            performance: self.performance.get_snapshot().await,
            errors: self.errors.get_snapshot().await,
            cluster: cluster.get_snapshot(),
        }
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        self.node_ops.reset();
        self.migration.reset().await;
        self.replication.reset();
        self.performance.reset().await;
        self.errors.reset().await;

        let mut cluster = self.cluster.write().await;
        cluster.reset();
    }

    /// Record a generic event
    async fn record_event(&self, _event_type: &str) {
        // In a real implementation, this might:
        // - Send to a metrics aggregation service
        // - Write to logs
        // - Update time-series databases
        // - Trigger alerts
    }
}

/// Node operations metrics
#[derive(Debug)]
struct NodeOperationsMetrics {
    nodes_added: AtomicU64,
    nodes_removed: AtomicU64,
    rebalances_triggered: AtomicU64,
}

impl NodeOperationsMetrics {
    fn new() -> Self {
        Self {
            nodes_added: AtomicU64::new(0),
            nodes_removed: AtomicU64::new(0),
            rebalances_triggered: AtomicU64::new(0),
        }
    }

    fn get_snapshot(&self) -> NodeOperationsSnapshot {
        NodeOperationsSnapshot {
            nodes_added: self.nodes_added.load(Ordering::Relaxed),
            nodes_removed: self.nodes_removed.load(Ordering::Relaxed),
            rebalances_triggered: self.rebalances_triggered.load(Ordering::Relaxed),
        }
    }

    fn reset(&self) {
        self.nodes_added.store(0, Ordering::Relaxed);
        self.nodes_removed.store(0, Ordering::Relaxed);
        self.rebalances_triggered.store(0, Ordering::Relaxed);
    }
}

/// Migration metrics
#[derive(Debug)]
struct MigrationMetrics {
    migrations_started: AtomicU64,
    migrations_completed: AtomicU64,
    migrations_failed: AtomicU64,
    active_migrations: AtomicUsize,
    keys_migrated: AtomicU64,
    bytes_migrated: AtomicU64,
    migration_durations: Arc<RwLock<Vec<Duration>>>,
}

impl MigrationMetrics {
    fn new() -> Self {
        Self {
            migrations_started: AtomicU64::new(0),
            migrations_completed: AtomicU64::new(0),
            migrations_failed: AtomicU64::new(0),
            active_migrations: AtomicUsize::new(0),
            keys_migrated: AtomicU64::new(0),
            bytes_migrated: AtomicU64::new(0),
            migration_durations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn update_duration(&self, duration: Duration) {
        let mut durations = self.migration_durations.write().await;
        durations.push(duration);

        // Keep only recent durations
        const MAX_DURATIONS: usize = 1000;
        if durations.len() > MAX_DURATIONS {
            durations.remove(0);
        }
    }

    async fn get_snapshot(&self) -> MigrationSnapshot {
        let durations = self.migration_durations.read().await;
        let avg_duration = if durations.is_empty() {
            Duration::from_secs(0)
        } else {
            let total_ms: u64 = durations.iter().map(|d| d.as_millis() as u64).sum();
            Duration::from_millis(total_ms / durations.len() as u64)
        };

        MigrationSnapshot {
            migrations_started: self.migrations_started.load(Ordering::Relaxed),
            migrations_completed: self.migrations_completed.load(Ordering::Relaxed),
            migrations_failed: self.migrations_failed.load(Ordering::Relaxed),
            active_migrations: self.active_migrations.load(Ordering::Relaxed),
            keys_migrated: self.keys_migrated.load(Ordering::Relaxed),
            bytes_migrated: self.bytes_migrated.load(Ordering::Relaxed),
            average_duration: avg_duration,
        }
    }

    async fn reset(&self) {
        self.migrations_started.store(0, Ordering::Relaxed);
        self.migrations_completed.store(0, Ordering::Relaxed);
        self.migrations_failed.store(0, Ordering::Relaxed);
        self.active_migrations.store(0, Ordering::Relaxed);
        self.keys_migrated.store(0, Ordering::Relaxed);
        self.bytes_migrated.store(0, Ordering::Relaxed);

        let mut durations = self.migration_durations.write().await;
        durations.clear();
    }
}

/// Replication metrics
#[derive(Debug)]
struct ReplicationMetrics {
    write_operations: AtomicU64,
    read_operations: AtomicU64,
    total_write_replicas: AtomicU64,
    successful_write_replicas: AtomicU64,
    total_read_replicas: AtomicU64,
    successful_read_replicas: AtomicU64,
    quorum_failures: AtomicU64,
}

impl ReplicationMetrics {
    fn new() -> Self {
        Self {
            write_operations: AtomicU64::new(0),
            read_operations: AtomicU64::new(0),
            total_write_replicas: AtomicU64::new(0),
            successful_write_replicas: AtomicU64::new(0),
            total_read_replicas: AtomicU64::new(0),
            successful_read_replicas: AtomicU64::new(0),
            quorum_failures: AtomicU64::new(0),
        }
    }

    fn get_snapshot(&self) -> ReplicationSnapshot {
        let write_ops = self.write_operations.load(Ordering::Relaxed);
        let read_ops = self.read_operations.load(Ordering::Relaxed);
        let total_write_replicas = self.total_write_replicas.load(Ordering::Relaxed);
        let successful_write_replicas = self.successful_write_replicas.load(Ordering::Relaxed);
        let total_read_replicas = self.total_read_replicas.load(Ordering::Relaxed);
        let successful_read_replicas = self.successful_read_replicas.load(Ordering::Relaxed);

        let write_success_rate = if total_write_replicas > 0 {
            successful_write_replicas as f64 / total_write_replicas as f64
        } else {
            0.0
        };

        let read_success_rate = if total_read_replicas > 0 {
            successful_read_replicas as f64 / total_read_replicas as f64
        } else {
            0.0
        };

        ReplicationSnapshot {
            write_operations: write_ops,
            read_operations: read_ops,
            write_success_rate,
            read_success_rate,
            quorum_failures: self.quorum_failures.load(Ordering::Relaxed),
        }
    }

    fn reset(&self) {
        self.write_operations.store(0, Ordering::Relaxed);
        self.read_operations.store(0, Ordering::Relaxed);
        self.total_write_replicas.store(0, Ordering::Relaxed);
        self.successful_write_replicas.store(0, Ordering::Relaxed);
        self.total_read_replicas.store(0, Ordering::Relaxed);
        self.successful_read_replicas.store(0, Ordering::Relaxed);
        self.quorum_failures.store(0, Ordering::Relaxed);
    }
}

/// Performance metrics
#[derive(Debug)]
struct PerformanceMetrics {
    latencies: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    throughput_samples: Arc<RwLock<Vec<f64>>>,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            latencies: Arc::new(RwLock::new(HashMap::new())),
            throughput_samples: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn record_latency(&self, operation: &str, latency: Duration) {
        let mut latencies = self.latencies.write().await;
        let operation_latencies = latencies.entry(operation.to_string()).or_insert_with(Vec::new);
        operation_latencies.push(latency);

        // Keep only recent latencies
        const MAX_LATENCIES: usize = 1000;
        if operation_latencies.len() > MAX_LATENCIES {
            operation_latencies.remove(0);
        }
    }

    async fn update_throughput(&self, throughput: f64) {
        let mut samples = self.throughput_samples.write().await;
        samples.push(throughput);

        // Keep only recent samples
        const MAX_SAMPLES: usize = 100;
        if samples.len() > MAX_SAMPLES {
            samples.remove(0);
        }
    }

    async fn get_snapshot(&self) -> PerformanceSnapshot {
        let latencies = self.latencies.read().await;
        let throughput_samples = self.throughput_samples.read().await;

        let mut operation_latencies = HashMap::new();

        for (operation, durations) in latencies.iter() {
            if !durations.is_empty() {
                let total_ms: u64 = durations.iter().map(|d| d.as_millis() as u64).sum();
                let avg_latency = Duration::from_millis(total_ms / durations.len() as u64);

                let mut sorted_durations = durations.clone();
                sorted_durations.sort();
                let p95_index = (sorted_durations.len() as f64 * 0.95) as usize;
                let p95_latency = sorted_durations.get(p95_index.min(sorted_durations.len() - 1))
                    .copied()
                    .unwrap_or_else(|| Duration::from_secs(0));

                operation_latencies.insert(operation.clone(), OperationLatency {
                    average: avg_latency,
                    p95: p95_latency,
                    sample_count: durations.len(),
                });
            }
        }

        let avg_throughput = if throughput_samples.is_empty() {
            0.0
        } else {
            throughput_samples.iter().sum::<f64>() / throughput_samples.len() as f64
        };

        PerformanceSnapshot {
            operation_latencies,
            average_throughput: avg_throughput,
        }
    }

    async fn reset(&self) {
        let mut latencies = self.latencies.write().await;
        latencies.clear();

        let mut throughput_samples = self.throughput_samples.write().await;
        throughput_samples.clear();
    }
}

/// Error metrics
#[derive(Debug)]
struct ErrorMetrics {
    error_counts: Arc<RwLock<HashMap<String, u64>>>,
    last_errors: Arc<RwLock<Vec<ErrorEvent>>>,
}

impl ErrorMetrics {
    fn new() -> Self {
        Self {
            error_counts: Arc::new(RwLock::new(HashMap::new())),
            last_errors: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn record_error(&self, error_type: &str) {
        let mut counts = self.error_counts.write().await;
        *counts.entry(error_type.to_string()).or_insert(0) += 1;

        let mut last_errors = self.last_errors.write().await;
        last_errors.push(ErrorEvent {
            error_type: error_type.to_string(),
            timestamp: SystemTime::now(),
        });

        // Keep only recent errors
        const MAX_ERRORS: usize = 1000;
        if last_errors.len() > MAX_ERRORS {
            last_errors.remove(0);
        }
    }

    async fn get_snapshot(&self) -> ErrorSnapshot {
        let counts = self.error_counts.read().await;
        let last_errors = self.last_errors.read().await;

        let total_errors = counts.values().sum();

        ErrorSnapshot {
            error_counts: counts.clone(),
            total_errors,
            recent_errors: last_errors.clone(),
        }
    }

    async fn reset(&self) {
        let mut counts = self.error_counts.write().await;
        counts.clear();

        let mut last_errors = self.last_errors.write().await;
        last_errors.clear();
    }
}

/// Cluster metrics
#[derive(Debug)]
struct ClusterMetrics {
    stats_history: Vec<ClusterStatsEntry>,
    last_update: Option<Instant>,
}

impl ClusterMetrics {
    fn new() -> Self {
        Self {
            stats_history: Vec::new(),
            last_update: None,
        }
    }

    async fn update(&mut self, stats: ClusterStats) {
        let now = Instant::now();

        self.stats_history.push(ClusterStatsEntry {
            timestamp: now,
            stats,
        });

        // Keep only recent history
        const MAX_HISTORY: usize = 100;
        if self.stats_history.len() > MAX_HISTORY {
            self.stats_history.remove(0);
        }

        self.last_update = Some(now);
    }

    fn get_snapshot(&self) -> ClusterSnapshot {
        let current_stats = self.stats_history.last().map(|entry| entry.stats.clone());

        ClusterSnapshot {
            current_stats,
            stats_history: self.stats_history.clone(),
            last_update: self.last_update,
        }
    }

    fn reset(&mut self) {
        self.stats_history.clear();
        self.last_update = None;
    }
}

/// Snapshot data structures for metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: SystemTime,
    pub node_operations: NodeOperationsSnapshot,
    pub migration: MigrationSnapshot,
    pub replication: ReplicationSnapshot,
    pub performance: PerformanceSnapshot,
    pub errors: ErrorSnapshot,
    pub cluster: ClusterSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOperationsSnapshot {
    pub nodes_added: u64,
    pub nodes_removed: u64,
    pub rebalances_triggered: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSnapshot {
    pub migrations_started: u64,
    pub migrations_completed: u64,
    pub migrations_failed: u64,
    pub active_migrations: usize,
    pub keys_migrated: u64,
    pub bytes_migrated: u64,
    pub average_duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationSnapshot {
    pub write_operations: u64,
    pub read_operations: u64,
    pub write_success_rate: f64,
    pub read_success_rate: f64,
    pub quorum_failures: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub operation_latencies: HashMap<String, OperationLatency>,
    pub average_throughput: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLatency {
    pub average: Duration,
    pub p95: Duration,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSnapshot {
    pub error_counts: HashMap<String, u64>,
    pub total_errors: u64,
    pub recent_errors: Vec<ErrorEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub error_type: String,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSnapshot {
    pub current_stats: Option<ClusterStats>,
    pub stats_history: Vec<ClusterStatsEntry>,
    #[serde(skip)]
    pub last_update: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct ClusterStatsEntry {
    pub timestamp: Instant,
    pub stats: ClusterStats,
}

impl Serialize for ClusterStatsEntry {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.stats.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ClusterStatsEntry {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let stats = ClusterStats::deserialize(deserializer)?;
        Ok(Self {
            timestamp: Instant::now(),
            stats,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_creation() {
        let metrics = ShardMetrics::new(Duration::from_secs(1));
        let snapshot = metrics.get_snapshot().await;

        assert_eq!(snapshot.node_operations.nodes_added, 0);
        assert_eq!(snapshot.migration.migrations_started, 0);
        assert_eq!(snapshot.replication.write_operations, 0);
    }

    #[tokio::test]
    async fn test_node_operations_metrics() {
        let metrics = ShardMetrics::new(Duration::from_secs(1));

        metrics.record_node_added().await;
        metrics.record_node_removed().await;
        metrics.record_rebalance_triggered().await;

        let snapshot = metrics.get_snapshot().await;
        assert_eq!(snapshot.node_operations.nodes_added, 1);
        assert_eq!(snapshot.node_operations.nodes_removed, 1);
        assert_eq!(snapshot.node_operations.rebalances_triggered, 1);
    }

    #[tokio::test]
    async fn test_migration_metrics() {
        let metrics = ShardMetrics::new(Duration::from_secs(1));

        metrics.record_migration_started().await;
        metrics.record_keys_migrated(100).await;
        metrics.record_bytes_migrated(1024).await;
        metrics.record_migration_completed(Duration::from_millis(500)).await;

        let snapshot = metrics.get_snapshot().await;
        assert_eq!(snapshot.migration.migrations_started, 1);
        assert_eq!(snapshot.migration.migrations_completed, 1);
        assert_eq!(snapshot.migration.active_migrations, 0);
        assert_eq!(snapshot.migration.keys_migrated, 100);
        assert_eq!(snapshot.migration.bytes_migrated, 1024);
        assert!(snapshot.migration.average_duration.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_replication_metrics() {
        let metrics = ShardMetrics::new(Duration::from_secs(1));

        metrics.record_replication_write(3, 2).await;
        metrics.record_replication_read(3, 3).await;
        metrics.record_quorum_failure().await;

        let snapshot = metrics.get_snapshot().await;
        assert_eq!(snapshot.replication.write_operations, 1);
        assert_eq!(snapshot.replication.read_operations, 1);
        assert!((snapshot.replication.write_success_rate - 0.666).abs() < 0.01);
        assert!((snapshot.replication.read_success_rate - 1.0).abs() < 0.01);
        assert_eq!(snapshot.replication.quorum_failures, 1);
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let metrics = ShardMetrics::new(Duration::from_secs(1));

        metrics.record_operation_latency("read", Duration::from_millis(10)).await;
        metrics.record_operation_latency("read", Duration::from_millis(20)).await;
        metrics.record_operation_latency("write", Duration::from_millis(30)).await;
        metrics.record_throughput(100.0).await;
        metrics.record_throughput(200.0).await;

        let snapshot = metrics.get_snapshot().await;

        assert!(snapshot.performance.operation_latencies.contains_key("read"));
        assert!(snapshot.performance.operation_latencies.contains_key("write"));

        let read_latency = &snapshot.performance.operation_latencies["read"];
        assert_eq!(read_latency.sample_count, 2);
        assert_eq!(read_latency.average.as_millis(), 15);

        assert!((snapshot.performance.average_throughput - 150.0).abs() < 1.0);
    }

    #[tokio::test]
    async fn test_error_metrics() {
        let metrics = ShardMetrics::new(Duration::from_secs(1));

        metrics.record_error("network_error").await;
        metrics.record_error("timeout_error").await;
        metrics.record_error("network_error").await;

        let snapshot = metrics.get_snapshot().await;
        assert_eq!(snapshot.errors.total_errors, 3);
        assert_eq!(*snapshot.errors.error_counts.get("network_error").unwrap(), 2);
        assert_eq!(*snapshot.errors.error_counts.get("timeout_error").unwrap(), 1);
        assert_eq!(snapshot.errors.recent_errors.len(), 3);
    }

    #[tokio::test]
    async fn test_cluster_metrics() {
        let metrics = ShardMetrics::new(Duration::from_secs(1));

        let stats = ClusterStats {
            total_nodes: 5,
            healthy_nodes: 4,
            degraded_nodes: 1,
            failed_nodes: 0,
            virtual_nodes: 150,
            replication_factor: 3,
            is_balanced: true,
        };

        metrics.update_cluster_stats(stats.clone()).await;

        let snapshot = metrics.get_snapshot().await;
        let current = snapshot.cluster.current_stats.unwrap();
        assert_eq!(current.total_nodes, 5);
        assert_eq!(current.healthy_nodes, 4);
        assert!(current.is_balanced);
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let metrics = ShardMetrics::new(Duration::from_secs(1));

        metrics.record_node_added().await;
        metrics.record_migration_started().await;
        metrics.record_error("test_error").await;

        let snapshot_before = metrics.get_snapshot().await;
        assert!(snapshot_before.node_operations.nodes_added > 0);

        metrics.reset().await;

        let snapshot_after = metrics.get_snapshot().await;
        assert_eq!(snapshot_after.node_operations.nodes_added, 0);
        assert_eq!(snapshot_after.migration.migrations_started, 0);
        assert_eq!(snapshot_after.errors.total_errors, 0);
    }

    #[tokio::test]
    async fn test_metrics_serialization() {
        let metrics = ShardMetrics::new(Duration::from_secs(1));

        metrics.record_node_added().await;
        metrics.record_migration_started().await;

        let snapshot = metrics.get_snapshot().await;

        // Test JSON serialization
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: MetricsSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(snapshot.node_operations.nodes_added, deserialized.node_operations.nodes_added);
        assert_eq!(snapshot.migration.migrations_started, deserialized.migration.migrations_started);
    }
}