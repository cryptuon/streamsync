//! Load Balancer for StreamSync Sharding
//!
//! This module implements intelligent load balancing across the distributed
//! cluster, monitoring resource utilization and triggering rebalancing operations.

use super::{ShardingConfig, ShardInfo, NodeCapacity};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Load balancing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Balance by storage utilization
    StorageBased { target_utilization: f64 },
    /// Balance by CPU usage
    CpuBased { target_utilization: f64 },
    /// Balance by network throughput
    NetworkBased { target_throughput: f64 },
    /// Composite balancing considering multiple factors
    Composite {
        storage_weight: f64,
        cpu_weight: f64,
        network_weight: f64,
        memory_weight: f64,
    },
    /// Custom balancing algorithm
    Custom { algorithm_name: String },
}

impl Default for LoadBalancingStrategy {
    fn default() -> Self {
        LoadBalancingStrategy::Composite {
            storage_weight: 0.4,
            cpu_weight: 0.3,
            network_weight: 0.2,
            memory_weight: 0.1,
        }
    }
}

/// Load balancing trigger conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingTriggers {
    /// Maximum acceptable load imbalance (0.0-1.0)
    pub max_load_imbalance: f64,
    /// Minimum time between rebalancing operations
    pub min_rebalance_interval_ms: u64,
    /// Maximum number of concurrent migrations
    pub max_concurrent_migrations: usize,
    /// Node failure threshold before emergency rebalancing
    pub node_failure_threshold: usize,
    /// Storage utilization threshold
    pub storage_threshold: f64,
    /// CPU utilization threshold
    pub cpu_threshold: f64,
}

impl Default for LoadBalancingTriggers {
    fn default() -> Self {
        Self {
            max_load_imbalance: 0.2, // 20% imbalance
            min_rebalance_interval_ms: 300_000, // 5 minutes
            max_concurrent_migrations: 3,
            node_failure_threshold: 1,
            storage_threshold: 0.8, // 80%
            cpu_threshold: 0.8, // 80%
        }
    }
}

/// Rebalancing plan with specific migrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancingPlan {
    pub plan_id: String,
    pub migrations: Vec<MigrationPlan>,
    pub estimated_duration: chrono::Duration,
    pub expected_improvement: f64,
    pub created_at: DateTime<Utc>,
    pub execution_priority: RebalancingPriority,
}

/// Individual migration in the rebalancing plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub shard_id: String,
    pub from_node: Uuid,
    pub to_node: Uuid,
    pub estimated_duration: chrono::Duration,
    pub load_reduction: f64,
    pub priority: MigrationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RebalancingPriority {
    Emergency = 0,
    High = 1,
    Normal = 2,
    Background = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

/// Node load assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLoadAssessment {
    pub node_id: Uuid,
    pub overall_load: f64,
    pub storage_load: f64,
    pub cpu_load: f64,
    pub network_load: f64,
    pub memory_load: f64,
    pub shard_count: usize,
    pub total_data_size: u64,
    pub predicted_load: f64,
    pub assessment_time: DateTime<Utc>,
}

/// Cluster load statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterLoadStats {
    pub total_nodes: usize,
    pub average_load: f64,
    pub load_standard_deviation: f64,
    pub max_load: f64,
    pub min_load: f64,
    pub load_imbalance_ratio: f64,
    pub overloaded_nodes: usize,
    pub underloaded_nodes: usize,
    pub total_shards: usize,
    pub total_data_size: u64,
}

/// Load balancer implementation
pub struct LoadBalancer {
    config: ShardingConfig,
    strategy: LoadBalancingStrategy,
    triggers: LoadBalancingTriggers,

    // State tracking
    last_rebalance: Option<DateTime<Utc>>,
    rebalancing_history: Vec<RebalancingRecord>,

    // Load prediction
    load_predictor: LoadPredictor,
}

/// Historical rebalancing record
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RebalancingRecord {
    timestamp: DateTime<Utc>,
    plan_id: String,
    migrations_count: usize,
    success_rate: f64,
    load_improvement: f64,
    duration: chrono::Duration,
}

/// Load prediction engine
#[derive(Debug, Clone)]
struct LoadPredictor {
    historical_loads: HashMap<Uuid, Vec<(DateTime<Utc>, f64)>>,
    prediction_window: chrono::Duration,
    trend_analysis: TrendAnalysis,
}

#[derive(Debug, Clone)]
struct TrendAnalysis {
    growth_rates: HashMap<Uuid, f64>,
    seasonality_patterns: HashMap<Uuid, Vec<f64>>,
    anomaly_detection: AnomalyDetector,
}

#[derive(Debug, Clone)]
struct AnomalyDetector {
    threshold_multiplier: f64,
    recent_anomalies: Vec<(DateTime<Utc>, Uuid, f64)>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(config: ShardingConfig) -> Result<Self> {
        Ok(Self {
            config,
            strategy: LoadBalancingStrategy::default(),
            triggers: LoadBalancingTriggers::default(),
            last_rebalance: None,
            rebalancing_history: Vec::new(),
            load_predictor: LoadPredictor {
                historical_loads: HashMap::new(),
                prediction_window: chrono::Duration::hours(1),
                trend_analysis: TrendAnalysis {
                    growth_rates: HashMap::new(),
                    seasonality_patterns: HashMap::new(),
                    anomaly_detection: AnomalyDetector {
                        threshold_multiplier: 2.0,
                        recent_anomalies: Vec::new(),
                    },
                },
            },
        })
    }

    /// Assess cluster load and create rebalancing plan if needed
    pub async fn create_rebalancing_plan(
        &mut self,
        shard_registry: &HashMap<String, ShardInfo>,
        node_capacities: &HashMap<Uuid, NodeCapacity>,
    ) -> Result<RebalancingPlan> {
        tracing::info!("📊 Analyzing cluster load for potential rebalancing");

        // Assess current load
        let node_assessments = self.assess_node_loads(shard_registry, node_capacities).await;
        let cluster_stats = self.calculate_cluster_stats(&node_assessments);

        // Check if rebalancing is needed
        if !self.should_rebalance(&cluster_stats).await {
            return Ok(RebalancingPlan {
                plan_id: format!("no_rebalance_{}", Utc::now().timestamp_millis()),
                migrations: Vec::new(),
                estimated_duration: chrono::Duration::zero(),
                expected_improvement: 0.0,
                created_at: Utc::now(),
                execution_priority: RebalancingPriority::Background,
            });
        }

        // Create migration plans
        let migrations = self.generate_migration_plans(&node_assessments, shard_registry).await?;

        // Calculate plan metrics
        let estimated_duration = migrations.iter()
            .map(|m| m.estimated_duration)
            .max()
            .unwrap_or(chrono::Duration::zero());

        let expected_improvement = self.calculate_expected_improvement(&migrations, &cluster_stats);

        let priority = self.determine_rebalancing_priority(&cluster_stats);

        let plan = RebalancingPlan {
            plan_id: format!("rebalance_{}", Utc::now().timestamp_millis()),
            migrations,
            estimated_duration,
            expected_improvement,
            created_at: Utc::now(),
            execution_priority: priority,
        };

        tracing::info!("📋 Created rebalancing plan with {} migrations, expected improvement: {:.2}%",
                      plan.migrations.len(), expected_improvement * 100.0);

        Ok(plan)
    }

    /// Assess load for all nodes
    async fn assess_node_loads(
        &mut self,
        shard_registry: &HashMap<String, ShardInfo>,
        node_capacities: &HashMap<Uuid, NodeCapacity>,
    ) -> Vec<NodeLoadAssessment> {
        let mut assessments = Vec::new();

        for (&node_id, capacity) in node_capacities {
            let assessment = self.assess_node_load(node_id, capacity, shard_registry).await;
            assessments.push(assessment);
        }

        assessments
    }

    /// Assess load for a single node
    async fn assess_node_load(
        &mut self,
        node_id: Uuid,
        capacity: &NodeCapacity,
        shard_registry: &HashMap<String, ShardInfo>,
    ) -> NodeLoadAssessment {
        // Count shards and calculate data size for this node
        let mut shard_count = 0;
        let mut total_data_size = 0;

        for shard in shard_registry.values() {
            if shard.assigned_nodes.contains(&node_id) {
                shard_count += 1;
                total_data_size += shard.size_bytes;
            }
        }

        // Calculate load metrics
        let storage_load = capacity.used_storage_bytes as f64 / capacity.total_storage_bytes as f64;
        let cpu_load = capacity.current_load;
        let network_load = 0.5; // Would be calculated from actual metrics
        let memory_load = 0.3; // Would be calculated from actual metrics

        // Calculate overall load based on strategy
        let overall_load = match &self.strategy {
            LoadBalancingStrategy::StorageBased { .. } => storage_load,
            LoadBalancingStrategy::CpuBased { .. } => cpu_load,
            LoadBalancingStrategy::NetworkBased { .. } => network_load,
            LoadBalancingStrategy::Composite { storage_weight, cpu_weight, network_weight, memory_weight } => {
                storage_load * storage_weight +
                cpu_load * cpu_weight +
                network_load * network_weight +
                memory_load * memory_weight
            }
            LoadBalancingStrategy::Custom { .. } => {
                // Would implement custom algorithm
                (storage_load + cpu_load + network_load + memory_load) / 4.0
            }
        };

        // Predict future load
        let predicted_load = self.predict_node_load(node_id, overall_load).await;

        // Update historical data
        self.update_historical_load(node_id, overall_load).await;

        NodeLoadAssessment {
            node_id,
            overall_load,
            storage_load,
            cpu_load,
            network_load,
            memory_load,
            shard_count,
            total_data_size,
            predicted_load,
            assessment_time: Utc::now(),
        }
    }

    /// Predict future load for a node
    async fn predict_node_load(&self, node_id: Uuid, current_load: f64) -> f64 {
        // Simple linear prediction based on historical trend
        if let Some(growth_rate) = self.load_predictor.trend_analysis.growth_rates.get(&node_id) {
            let prediction_hours = self.load_predictor.prediction_window.num_hours() as f64;
            (current_load * (1.0 + growth_rate * prediction_hours)).min(1.0)
        } else {
            current_load
        }
    }

    /// Update historical load data
    async fn update_historical_load(&mut self, node_id: Uuid, load: f64) {
        let history = self.load_predictor.historical_loads.entry(node_id).or_insert_with(Vec::new);
        history.push((Utc::now(), load));

        // Keep only recent history (last 24 hours)
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        history.retain(|(timestamp, _)| *timestamp > cutoff);

        // Update growth rate
        if history.len() >= 2 {
            let recent_loads: Vec<f64> = history.iter().map(|(_, load)| *load).collect();
            let growth_rate = self.calculate_growth_rate(&recent_loads);
            self.load_predictor.trend_analysis.growth_rates.insert(node_id, growth_rate);
        }
    }

    /// Calculate growth rate from historical data
    fn calculate_growth_rate(&self, loads: &[f64]) -> f64 {
        if loads.len() < 2 {
            return 0.0;
        }

        let n = loads.len() as f64;
        let sum_x: f64 = (0..loads.len()).map(|i| i as f64).sum();
        let sum_y: f64 = loads.iter().sum();
        let sum_xy: f64 = loads.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_x2: f64 = (0..loads.len()).map(|i| (i as f64).powi(2)).sum();

        // Linear regression slope
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        slope / n // Normalize by time period
    }

    /// Calculate cluster-wide load statistics
    fn calculate_cluster_stats(&self, assessments: &[NodeLoadAssessment]) -> ClusterLoadStats {
        if assessments.is_empty() {
            return ClusterLoadStats {
                total_nodes: 0,
                average_load: 0.0,
                load_standard_deviation: 0.0,
                max_load: 0.0,
                min_load: 0.0,
                load_imbalance_ratio: 0.0,
                overloaded_nodes: 0,
                underloaded_nodes: 0,
                total_shards: 0,
                total_data_size: 0,
            };
        }

        let loads: Vec<f64> = assessments.iter().map(|a| a.overall_load).collect();
        let average_load = loads.iter().sum::<f64>() / loads.len() as f64;
        let max_load = loads.iter().cloned().fold(0.0, f64::max);
        let min_load = loads.iter().cloned().fold(1.0, f64::min);

        // Calculate standard deviation
        let variance = loads.iter()
            .map(|load| (load - average_load).powi(2))
            .sum::<f64>() / loads.len() as f64;
        let std_deviation = variance.sqrt();

        // Calculate load imbalance ratio
        let load_imbalance_ratio = if max_load > 0.0 {
            (max_load - min_load) / max_load
        } else {
            0.0
        };

        // Count overloaded and underloaded nodes
        let overloaded_threshold = average_load + std_deviation;
        let underloaded_threshold = average_load - std_deviation;

        let overloaded_nodes = loads.iter().filter(|&&load| load > overloaded_threshold).count();
        let underloaded_nodes = loads.iter().filter(|&&load| load < underloaded_threshold).count();

        ClusterLoadStats {
            total_nodes: assessments.len(),
            average_load,
            load_standard_deviation: std_deviation,
            max_load,
            min_load,
            load_imbalance_ratio,
            overloaded_nodes,
            underloaded_nodes,
            total_shards: assessments.iter().map(|a| a.shard_count).sum(),
            total_data_size: assessments.iter().map(|a| a.total_data_size).sum(),
        }
    }

    /// Determine if rebalancing is needed
    async fn should_rebalance(&self, stats: &ClusterLoadStats) -> bool {
        // Check time constraint
        if let Some(last_rebalance) = self.last_rebalance {
            let time_since_last = Utc::now() - last_rebalance;
            if time_since_last.num_milliseconds() < self.triggers.min_rebalance_interval_ms as i64 {
                return false;
            }
        }

        // Check load imbalance
        if stats.load_imbalance_ratio > self.triggers.max_load_imbalance {
            tracing::info!("🔄 Rebalancing needed: load imbalance {:.2}% > {:.2}%",
                          stats.load_imbalance_ratio * 100.0,
                          self.triggers.max_load_imbalance * 100.0);
            return true;
        }

        // Check overloaded nodes
        if stats.overloaded_nodes > 0 {
            tracing::info!("🔄 Rebalancing needed: {} overloaded nodes", stats.overloaded_nodes);
            return true;
        }

        false
    }

    /// Generate migration plans to improve load balance
    async fn generate_migration_plans(
        &self,
        assessments: &[NodeLoadAssessment],
        shard_registry: &HashMap<String, ShardInfo>,
    ) -> Result<Vec<MigrationPlan>> {
        let mut migrations = Vec::new();

        // Sort nodes by load (descending for sources, ascending for targets)
        let mut source_candidates: Vec<_> = assessments.iter()
            .filter(|a| a.overall_load > 0.7) // Only consider nodes above 70% load
            .collect();
        source_candidates.sort_by(|a, b| b.overall_load.partial_cmp(&a.overall_load).unwrap());

        let mut target_candidates: Vec<_> = assessments.iter()
            .filter(|a| a.overall_load < 0.5) // Only consider nodes below 50% load
            .collect();
        target_candidates.sort_by(|a, b| a.overall_load.partial_cmp(&b.overall_load).unwrap());

        // Generate migrations from highest loaded to lowest loaded nodes
        for source in source_candidates.iter().take(3) { // Limit concurrent migrations
            if let Some(target) = target_candidates.iter().find(|t| t.node_id != source.node_id) {
                // Find a suitable shard to migrate
                if let Some(shard) = self.find_suitable_shard_for_migration(source.node_id, shard_registry) {
                    let migration = MigrationPlan {
                        shard_id: shard.shard_id.clone(),
                        from_node: source.node_id,
                        to_node: target.node_id,
                        estimated_duration: self.estimate_migration_duration(&shard),
                        load_reduction: self.calculate_load_reduction(source, target, &shard),
                        priority: self.determine_migration_priority(source.overall_load),
                    };

                    migrations.push(migration);

                    if migrations.len() >= self.triggers.max_concurrent_migrations {
                        break;
                    }
                }
            }
        }

        migrations.sort_by_key(|m| m.priority.clone());
        Ok(migrations)
    }

    /// Find a suitable shard for migration from an overloaded node
    fn find_suitable_shard_for_migration(&self, node_id: Uuid, shard_registry: &HashMap<String, ShardInfo>) -> Option<ShardInfo> {
        // Find shards where this node is not the primary (safer to migrate)
        let mut candidate_shards: Vec<_> = shard_registry.values()
            .filter(|shard| shard.assigned_nodes.contains(&node_id) && shard.primary_node != node_id)
            .collect();

        // Sort by size (prefer smaller shards for faster migration)
        candidate_shards.sort_by_key(|shard| shard.size_bytes);

        candidate_shards.first().map(|&shard| shard.clone())
    }

    /// Estimate migration duration based on shard size
    fn estimate_migration_duration(&self, shard: &ShardInfo) -> chrono::Duration {
        // Assume 100 MB/s transfer rate
        let transfer_rate_bytes_per_second = 100 * 1024 * 1024;
        let estimated_seconds = (shard.size_bytes / transfer_rate_bytes_per_second).max(30); // Minimum 30 seconds
        chrono::Duration::seconds(estimated_seconds as i64)
    }

    /// Calculate expected load reduction from migration
    fn calculate_load_reduction(&self, source: &NodeLoadAssessment, target: &NodeLoadAssessment, shard: &ShardInfo) -> f64 {
        let data_ratio = shard.size_bytes as f64 / source.total_data_size as f64;
        let load_reduction = source.overall_load * data_ratio * 0.8; // 80% efficiency
        load_reduction.min(source.overall_load - target.overall_load)
    }

    /// Determine migration priority based on source node load
    fn determine_migration_priority(&self, source_load: f64) -> MigrationPriority {
        if source_load > 0.9 {
            MigrationPriority::Critical
        } else if source_load > 0.8 {
            MigrationPriority::High
        } else if source_load > 0.7 {
            MigrationPriority::Normal
        } else {
            MigrationPriority::Low
        }
    }

    /// Calculate expected improvement from rebalancing plan
    fn calculate_expected_improvement(&self, migrations: &[MigrationPlan], stats: &ClusterLoadStats) -> f64 {
        let total_load_reduction: f64 = migrations.iter().map(|m| m.load_reduction).sum();
        total_load_reduction / (stats.average_load * stats.total_nodes as f64)
    }

    /// Determine rebalancing priority based on cluster state
    fn determine_rebalancing_priority(&self, stats: &ClusterLoadStats) -> RebalancingPriority {
        if stats.overloaded_nodes > 2 || stats.max_load > 0.95 {
            RebalancingPriority::Emergency
        } else if stats.overloaded_nodes > 0 || stats.load_imbalance_ratio > 0.4 {
            RebalancingPriority::High
        } else if stats.load_imbalance_ratio > 0.2 {
            RebalancingPriority::Normal
        } else {
            RebalancingPriority::Background
        }
    }

    /// Update configuration
    pub fn update_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.strategy = strategy;
    }

    /// Update triggers
    pub fn update_triggers(&mut self, triggers: LoadBalancingTriggers) {
        self.triggers = triggers;
    }

    /// Get load balancer statistics
    pub fn get_stats(&self) -> LoadBalancerStats {
        LoadBalancerStats {
            last_rebalance: self.last_rebalance,
            total_rebalances: self.rebalancing_history.len(),
            average_improvement: if !self.rebalancing_history.is_empty() {
                self.rebalancing_history.iter().map(|r| r.load_improvement).sum::<f64>() / self.rebalancing_history.len() as f64
            } else {
                0.0
            },
            strategy: self.strategy.clone(),
            triggers: self.triggers.clone(),
        }
    }
}

/// Load balancer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStats {
    pub last_rebalance: Option<DateTime<Utc>>,
    pub total_rebalances: usize,
    pub average_improvement: f64,
    pub strategy: LoadBalancingStrategy,
    pub triggers: LoadBalancingTriggers,
}