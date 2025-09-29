//! Data Placement Engine for StreamSync Sharding
//!
//! This module implements intelligent data placement decisions,
//! considering node capacity, network topology, and performance characteristics.

use super::{ShardingConfig, DistributionStrategy, KeyRange, NodeCapacity};
use super::strategy::{ShardingStrategy, PlacementResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Placement constraints for data placement decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementConstraints {
    /// Minimum replication factor
    pub min_replication: u32,
    /// Maximum replication factor
    pub max_replication: u32,
    /// Preferred availability zones
    pub preferred_zones: Vec<String>,
    /// Excluded nodes (for maintenance, etc.)
    pub excluded_nodes: Vec<Uuid>,
    /// Maximum distance between replicas (for latency)
    pub max_replica_distance: Option<u32>,
    /// Require cross-zone distribution
    pub require_cross_zone: bool,
    /// Performance requirements
    pub performance_class: PerformanceClass,
}

/// Performance class for placement decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceClass {
    /// High performance requirements (SSDs, high CPU)
    HighPerformance,
    /// Standard performance requirements
    Standard,
    /// Archive/cold storage (optimize for cost)
    Archive,
}

impl Default for PlacementConstraints {
    fn default() -> Self {
        Self {
            min_replication: 2,
            max_replication: 5,
            preferred_zones: Vec::new(),
            excluded_nodes: Vec::new(),
            max_replica_distance: None,
            require_cross_zone: false,
            performance_class: PerformanceClass::Standard,
        }
    }
}

/// Placement context with additional metadata
#[derive(Debug, Clone)]
pub struct PlacementContext {
    pub key_range: KeyRange,
    pub data_size: u64,
    pub constraints: PlacementConstraints,
    pub access_pattern: AccessPattern,
    pub priority: PlacementPriority,
    pub deadline: Option<DateTime<Utc>>,
}

/// Data access pattern information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPattern {
    pub read_frequency: AccessFrequency,
    pub write_frequency: AccessFrequency,
    pub access_localities: Vec<String>, // Geographic regions
    pub temporal_pattern: TemporalPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessFrequency {
    VeryHigh, // > 1000 ops/sec
    High,     // 100-1000 ops/sec
    Medium,   // 10-100 ops/sec
    Low,      // 1-10 ops/sec
    Rare,     // < 1 op/sec
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalPattern {
    Realtime,    // Continuous access
    Batch,       // Periodic batch processing
    Sporadic,    // Irregular access
    Archive,     // Rare access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlacementPriority {
    Critical,    // Must succeed
    High,        // Should succeed
    Normal,      // Best effort
    Background,  // Can be delayed
}

/// Advanced placement engine with intelligent decision making
pub struct PlacementEngine {
    config: ShardingConfig,
    strategy: ShardingStrategy,
    node_metrics: HashMap<Uuid, NodeMetrics>,
    placement_history: Vec<PlacementRecord>,
    performance_tracker: PerformanceTracker,
}

/// Extended node metrics for placement decisions
#[derive(Debug, Clone)]
struct NodeMetrics {
    capacity: NodeCapacity,
    performance_score: f64,
    reliability_score: f64,
    latency_map: HashMap<Uuid, u64>, // Latency to other nodes
    recent_failures: u32,
    maintenance_window: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

/// Historical placement record
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlacementRecord {
    shard_id: String,
    placement_time: DateTime<Utc>,
    assigned_nodes: Vec<Uuid>,
    constraints: PlacementConstraints,
    success: bool,
    performance_outcome: Option<f64>,
}

/// Performance tracking for placement optimization
#[derive(Debug, Clone)]
struct PerformanceTracker {
    success_rate: f64,
    average_placement_time: u64,
    load_balance_quality: f64,
    recent_adjustments: Vec<PlacementAdjustment>,
}

#[derive(Debug, Clone)]
struct PlacementAdjustment {
    timestamp: DateTime<Utc>,
    adjustment_type: AdjustmentType,
    impact_score: f64,
}

#[derive(Debug, Clone)]
enum AdjustmentType {
    NodeAdded,
    NodeRemoved,
    RebalanceTriggered,
    ConstraintUpdated,
}

impl PlacementEngine {
    /// Create a new placement engine
    pub fn new(config: ShardingConfig, strategy: DistributionStrategy) -> Result<Self> {
        Ok(Self {
            config,
            strategy: ShardingStrategy::new(strategy),
            node_metrics: HashMap::new(),
            placement_history: Vec::new(),
            performance_tracker: PerformanceTracker {
                success_rate: 1.0,
                average_placement_time: 0,
                load_balance_quality: 1.0,
                recent_adjustments: Vec::new(),
            },
        })
    }

    /// Add a node to the placement engine
    pub async fn add_node(&mut self, capacity: NodeCapacity) -> Result<()> {
        let node_id = capacity.node_id;

        // Create node metrics
        let metrics = NodeMetrics {
            capacity: capacity.clone(),
            performance_score: self.calculate_initial_performance_score(&capacity),
            reliability_score: 1.0, // Start with perfect reliability
            latency_map: HashMap::new(),
            recent_failures: 0,
            maintenance_window: None,
        };

        self.node_metrics.insert(node_id, metrics);
        self.strategy.add_node(node_id, capacity)?;

        // Record adjustment
        self.performance_tracker.recent_adjustments.push(PlacementAdjustment {
            timestamp: Utc::now(),
            adjustment_type: AdjustmentType::NodeAdded,
            impact_score: 1.0,
        });

        tracing::info!("📍 Added node {} to placement engine", node_id);
        Ok(())
    }

    /// Remove a node from the placement engine
    pub async fn remove_node(&mut self, node_id: Uuid) -> Result<()> {
        self.node_metrics.remove(&node_id);
        self.strategy.remove_node(node_id)?;

        // Record adjustment
        self.performance_tracker.recent_adjustments.push(PlacementAdjustment {
            timestamp: Utc::now(),
            adjustment_type: AdjustmentType::NodeRemoved,
            impact_score: -1.0,
        });

        tracing::info!("📤 Removed node {} from placement engine", node_id);
        Ok(())
    }

    /// Determine optimal placement for data
    pub async fn determine_placement(&self, key_range: &KeyRange, data_size: u64) -> Result<PlacementResult> {
        let context = PlacementContext {
            key_range: key_range.clone(),
            data_size,
            constraints: PlacementConstraints::default(),
            access_pattern: AccessPattern {
                read_frequency: AccessFrequency::Medium,
                write_frequency: AccessFrequency::Low,
                access_localities: Vec::new(),
                temporal_pattern: TemporalPattern::Realtime,
            },
            priority: PlacementPriority::Normal,
            deadline: None,
        };

        self.determine_placement_with_context(&context).await
    }

    /// Determine placement with full context
    pub async fn determine_placement_with_context(&self, context: &PlacementContext) -> Result<PlacementResult> {
        let start_time = Utc::now();

        // Get base placement from strategy
        let mut placement = self.strategy.determine_placement(
            &context.key_range,
            context.data_size,
            context.constraints.max_replication,
        )?;

        // Apply intelligent optimizations
        placement = self.optimize_placement_for_context(placement, context).await?;

        // Validate constraints
        self.validate_constraints(&placement, &context.constraints)?;

        // Record placement
        let record = PlacementRecord {
            shard_id: format!("shard_{}", Utc::now().timestamp_millis()),
            placement_time: start_time,
            assigned_nodes: placement.assigned_nodes.clone(),
            constraints: context.constraints.clone(),
            success: true,
            performance_outcome: Some(placement.placement_score),
        };

        // Store record (in practice, this would be limited in size)
        let mut history = self.placement_history.clone();
        history.push(record);
        if history.len() > 1000 {
            history.remove(0);
        }

        tracing::debug!("📍 Determined placement for key range {:?}: {} nodes",
                       context.key_range, placement.assigned_nodes.len());

        Ok(placement)
    }

    /// Optimize placement based on context
    async fn optimize_placement_for_context(
        &self,
        mut placement: PlacementResult,
        context: &PlacementContext,
    ) -> Result<PlacementResult> {
        // Apply performance class optimizations
        match context.constraints.performance_class {
            PerformanceClass::HighPerformance => {
                placement = self.optimize_for_performance(placement).await?;
            }
            PerformanceClass::Archive => {
                placement = self.optimize_for_cost(placement).await?;
            }
            PerformanceClass::Standard => {
                placement = self.optimize_for_balance(placement).await?;
            }
        }

        // Apply access pattern optimizations
        placement = self.optimize_for_access_pattern(placement, &context.access_pattern).await?;

        // Apply zone distribution if required
        if context.constraints.require_cross_zone {
            placement = self.ensure_cross_zone_distribution(placement).await?;
        }

        // Filter excluded nodes
        if !context.constraints.excluded_nodes.is_empty() {
            placement = self.filter_excluded_nodes(placement, &context.constraints.excluded_nodes).await?;
        }

        Ok(placement)
    }

    /// Optimize placement for high performance
    async fn optimize_for_performance(&self, mut placement: PlacementResult) -> Result<PlacementResult> {
        // Prefer nodes with high performance scores
        let mut performance_candidates: Vec<_> = placement.assigned_nodes.iter()
            .filter_map(|&node_id| {
                self.node_metrics.get(&node_id).map(|metrics| (node_id, metrics.performance_score))
            })
            .collect();

        performance_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Keep only top performers
        let min_replicas = 2.max(placement.assigned_nodes.len() / 2);
        placement.assigned_nodes = performance_candidates[..min_replicas.min(performance_candidates.len())]
            .iter()
            .map(|(id, _)| *id)
            .collect();

        placement.primary_node = placement.assigned_nodes[0];
        placement.placement_score *= 1.1; // Bonus for performance optimization

        Ok(placement)
    }

    /// Optimize placement for cost (archive storage)
    async fn optimize_for_cost(&self, mut placement: PlacementResult) -> Result<PlacementResult> {
        // Prefer nodes with lower cost (higher storage, lower CPU requirements)
        let mut cost_candidates: Vec<_> = placement.assigned_nodes.iter()
            .filter_map(|&node_id| {
                self.node_metrics.get(&node_id).map(|metrics| {
                    let capacity = &metrics.capacity;
                    // Cost score: prefer high storage, low current load
                    let cost_score = (capacity.total_storage_bytes as f64 / 1e12) * (1.0 - capacity.current_load);
                    (node_id, cost_score)
                })
            })
            .collect();

        cost_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        placement.assigned_nodes = cost_candidates.iter().map(|(id, _)| *id).collect();
        placement.primary_node = placement.assigned_nodes[0];

        Ok(placement)
    }

    /// Optimize placement for balanced performance and cost
    async fn optimize_for_balance(&self, placement: PlacementResult) -> Result<PlacementResult> {
        // Keep the original placement but adjust scores
        // This represents a balanced approach between performance and cost
        Ok(placement)
    }

    /// Optimize placement based on access patterns
    async fn optimize_for_access_pattern(
        &self,
        mut placement: PlacementResult,
        access_pattern: &AccessPattern,
    ) -> Result<PlacementResult> {
        match access_pattern.read_frequency {
            AccessFrequency::VeryHigh | AccessFrequency::High => {
                // For high read frequency, prefer geographically distributed replicas
                placement = self.optimize_for_read_distribution(placement).await?;
            }
            AccessFrequency::Low | AccessFrequency::Rare => {
                // For low frequency, consolidate to reduce overhead
                placement = self.optimize_for_consolidation(placement).await?;
            }
            _ => {}
        }

        Ok(placement)
    }

    /// Optimize for read distribution
    async fn optimize_for_read_distribution(&self, mut placement: PlacementResult) -> Result<PlacementResult> {
        // Try to distribute across different zones for better read locality
        let mut zone_map: HashMap<String, Vec<Uuid>> = HashMap::new();

        for &node_id in &placement.assigned_nodes {
            if let Some(metrics) = self.node_metrics.get(&node_id) {
                let zone = &metrics.capacity.availability_zone;
                zone_map.entry(zone.clone()).or_default().push(node_id);
            }
        }

        // If we have nodes in multiple zones, prefer one per zone
        if zone_map.len() > 1 {
            let mut distributed_nodes = Vec::new();
            for (_, mut nodes) in zone_map {
                if let Some(node) = nodes.pop() {
                    distributed_nodes.push(node);
                }
            }

            if distributed_nodes.len() >= 2 {
                placement.assigned_nodes = distributed_nodes;
                placement.primary_node = placement.assigned_nodes[0];
                placement.placement_score *= 1.05; // Bonus for distribution
            }
        }

        Ok(placement)
    }

    /// Optimize for consolidation
    async fn optimize_for_consolidation(&self, mut placement: PlacementResult) -> Result<PlacementResult> {
        // For low-frequency access, prefer to consolidate in fewer zones to reduce overhead
        let target_replicas = 2.max(placement.assigned_nodes.len() / 2);
        placement.assigned_nodes.truncate(target_replicas);
        placement.primary_node = placement.assigned_nodes[0];

        Ok(placement)
    }

    /// Ensure cross-zone distribution
    async fn ensure_cross_zone_distribution(&self, mut placement: PlacementResult) -> Result<PlacementResult> {
        let mut zones: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut zone_nodes: HashMap<String, Uuid> = HashMap::new();

        for &node_id in &placement.assigned_nodes {
            if let Some(metrics) = self.node_metrics.get(&node_id) {
                let zone = &metrics.capacity.availability_zone;
                if zones.insert(zone.clone()) {
                    zone_nodes.insert(zone.clone(), node_id);
                }
            }
        }

        if zones.len() < 2 {
            // Try to find nodes in different zones
            for (node_id, metrics) in &self.node_metrics {
                let zone = &metrics.capacity.availability_zone;
                if !zones.contains(zone) && !placement.assigned_nodes.contains(node_id) {
                    placement.assigned_nodes.push(*node_id);
                    zones.insert(zone.clone());
                    if zones.len() >= 2 {
                        break;
                    }
                }
            }
        }

        if zones.len() < 2 {
            return Err(anyhow::anyhow!("Cannot satisfy cross-zone distribution requirement"));
        }

        Ok(placement)
    }

    /// Filter out excluded nodes
    async fn filter_excluded_nodes(&self, mut placement: PlacementResult, excluded: &[Uuid]) -> Result<PlacementResult> {
        placement.assigned_nodes.retain(|node_id| !excluded.contains(node_id));

        if placement.assigned_nodes.is_empty() {
            return Err(anyhow::anyhow!("All candidate nodes are excluded"));
        }

        // Update primary if it was excluded
        if excluded.contains(&placement.primary_node) {
            placement.primary_node = placement.assigned_nodes[0];
        }

        Ok(placement)
    }

    /// Validate placement against constraints
    fn validate_constraints(&self, placement: &PlacementResult, constraints: &PlacementConstraints) -> Result<()> {
        let replica_count = placement.assigned_nodes.len() as u32;

        if replica_count < constraints.min_replication {
            return Err(anyhow::anyhow!(
                "Insufficient replicas: {} < {}",
                replica_count,
                constraints.min_replication
            ));
        }

        if replica_count > constraints.max_replication {
            return Err(anyhow::anyhow!(
                "Too many replicas: {} > {}",
                replica_count,
                constraints.max_replication
            ));
        }

        // Validate excluded nodes
        for &excluded in &constraints.excluded_nodes {
            if placement.assigned_nodes.contains(&excluded) {
                return Err(anyhow::anyhow!("Placement includes excluded node: {}", excluded));
            }
        }

        Ok(())
    }

    /// Find best node for migration
    pub async fn find_best_node_for_migration(&self, _shard_id: &str, from_node: Uuid) -> Result<Uuid> {
        // Find the best alternative node that's not the source
        let mut candidates: Vec<_> = self.node_metrics.iter()
            .filter(|(&id, _)| id != from_node)
            .map(|(&id, metrics)| (id, metrics.performance_score * metrics.reliability_score))
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        candidates.first()
            .map(|(id, _)| *id)
            .ok_or_else(|| anyhow::anyhow!("No suitable migration target found"))
    }

    /// Calculate initial performance score for a node
    fn calculate_initial_performance_score(&self, capacity: &NodeCapacity) -> f64 {
        let storage_score = (capacity.total_storage_bytes as f64 / 1e12).min(1.0);
        let cpu_score = (capacity.cpu_cores as f64 / 64.0).min(1.0);
        let memory_score = (capacity.memory_bytes as f64 / 1e11).min(1.0); // 100GB baseline
        let bandwidth_score = (capacity.network_bandwidth_mbps as f64 / 10000.0).min(1.0); // 10Gbps baseline

        (storage_score + cpu_score + memory_score + bandwidth_score) / 4.0
    }

    /// Update node performance metrics
    pub async fn update_node_metrics(&mut self, node_id: Uuid, performance_score: f64, reliability_score: f64) {
        if let Some(metrics) = self.node_metrics.get_mut(&node_id) {
            metrics.performance_score = performance_score;
            metrics.reliability_score = reliability_score;
        }
    }

    /// Get placement engine statistics
    pub fn get_stats(&self) -> PlacementEngineStats {
        PlacementEngineStats {
            total_nodes: self.node_metrics.len(),
            total_placements: self.placement_history.len(),
            success_rate: self.performance_tracker.success_rate,
            average_placement_time: self.performance_tracker.average_placement_time,
            load_balance_quality: self.performance_tracker.load_balance_quality,
        }
    }
}

/// Placement engine statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementEngineStats {
    pub total_nodes: usize,
    pub total_placements: usize,
    pub success_rate: f64,
    pub average_placement_time: u64,
    pub load_balance_quality: f64,
}