//! Distributed coordination for DuckDB instances
//!
//! This module provides the main coordinator for distributed query execution
//! across the StreamSync network. It handles query planning, distribution,
//! parallel execution, and result aggregation.

use crate::consensus::{ConsensusConfig, PBFTConsensus};
use crate::network::{NetworkConfig, P2PNetwork};
use crate::network::protocol::NetworkMessage;
use crate::query::{Query, QueryResult};
use crate::sharding::{DataPlacementManager, DistributionStrategy, ShardingConfig};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for the distributed coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Node ID for this coordinator
    pub node_id: Uuid,
    /// Network configuration
    pub network: NetworkConfig,
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    /// Sharding configuration
    pub sharding: ShardingConfig,
    /// Query execution timeout in milliseconds
    pub query_timeout_ms: u64,
    /// Maximum concurrent queries
    pub max_concurrent_queries: usize,
    /// Enable query caching
    pub enable_query_cache: bool,
    /// Query cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Number of nodes to query in parallel for racing
    pub racing_parallelism: usize,
    /// Verification quorum (how many agreeing results needed)
    pub verification_quorum: usize,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        let node_id = Uuid::new_v4();
        Self {
            node_id,
            network: NetworkConfig::default(),
            consensus: ConsensusConfig::new(node_id, vec![node_id]),
            sharding: ShardingConfig::default(),
            query_timeout_ms: 30000,
            max_concurrent_queries: 100,
            enable_query_cache: true,
            cache_ttl_secs: 300,
            racing_parallelism: 3,
            verification_quorum: 2,
        }
    }
}

/// Query execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    /// Unique plan ID
    pub plan_id: String,
    /// Original query
    pub query: Query,
    /// Target shards for execution
    pub target_shards: Vec<String>,
    /// Nodes assigned to each shard
    pub shard_nodes: HashMap<String, Vec<Uuid>>,
    /// Aggregation strategy
    pub aggregation: AggregationStrategy,
    /// Estimated cost
    pub estimated_cost: QueryCost,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Strategy for aggregating partial results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationStrategy {
    /// No aggregation needed (single shard query)
    None,
    /// Union all results
    Union,
    /// Merge sorted results
    MergeSorted { sort_keys: Vec<String> },
    /// Aggregate with grouping
    GroupAggregate {
        group_by: Vec<String>,
        aggregates: Vec<AggregateFunction>,
    },
    /// Custom aggregation function
    Custom { function_name: String },
}

/// Aggregate function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateFunction {
    pub name: String,
    pub column: String,
    pub alias: String,
}

/// Query cost estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCost {
    /// Estimated rows to scan
    pub rows_to_scan: u64,
    /// Estimated bytes to transfer
    pub bytes_to_transfer: u64,
    /// Estimated execution time in ms
    pub estimated_time_ms: u64,
    /// Number of shards involved
    pub shard_count: usize,
    /// Network hops required
    pub network_hops: usize,
}

/// Partial result from a single node/shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialResult {
    /// Node that produced this result
    pub node_id: Uuid,
    /// Shard ID
    pub shard_id: String,
    /// Query plan ID
    pub plan_id: String,
    /// Result data
    pub data: Vec<u8>,
    /// Row count in this partial
    pub row_count: u64,
    /// Execution time in ms
    pub execution_time_ms: u64,
    /// Result hash for verification
    pub result_hash: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Query execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryStatus {
    /// Query is being planned
    Planning,
    /// Query is being distributed to nodes
    Distributing,
    /// Query is executing on nodes
    Executing {
        completed_shards: usize,
        total_shards: usize,
    },
    /// Results are being aggregated
    Aggregating,
    /// Query completed successfully
    Completed {
        execution_time_ms: u64,
        rows_returned: u64,
    },
    /// Query failed
    Failed { error: String },
    /// Query was cancelled
    Cancelled,
}

/// Cached query result
#[derive(Debug, Clone)]
struct CachedResult {
    result: QueryResult,
    created_at: DateTime<Utc>,
    hit_count: u64,
}

/// Main coordinator for distributed DuckDB operations
pub struct DistributedCoordinator {
    config: CoordinatorConfig,
    network: Option<P2PNetwork>,
    consensus: Option<Arc<RwLock<PBFTConsensus>>>,
    placement_manager: Option<Arc<RwLock<DataPlacementManager>>>,
    query_cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    active_queries: Arc<RwLock<HashMap<String, QueryStatus>>>,
    query_results: Arc<RwLock<HashMap<String, Vec<PartialResult>>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl DistributedCoordinator {
    /// Create a new distributed coordinator with default configuration
    pub fn new() -> Self {
        Self::with_config(CoordinatorConfig::default())
    }

    /// Create a new distributed coordinator with custom configuration
    pub fn with_config(config: CoordinatorConfig) -> Self {
        Self {
            config,
            network: None,
            consensus: None,
            placement_manager: None,
            query_cache: Arc::new(RwLock::new(HashMap::new())),
            active_queries: Arc::new(RwLock::new(HashMap::new())),
            query_results: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
        }
    }

    /// Initialize the coordinator with all subsystems
    pub async fn initialize(&mut self) -> Result<()> {
        info!("🚀 Initializing distributed coordinator for node {}", self.config.node_id);

        // Initialize network layer
        let network = P2PNetwork::new(self.config.network.clone())?;
        self.network = Some(network);

        // Create message channel for consensus
        let (message_tx, _message_rx) = mpsc::unbounded_channel::<NetworkMessage>();

        // Initialize consensus
        let consensus = PBFTConsensus::new(self.config.consensus.clone(), message_tx)?;
        self.consensus = Some(Arc::new(RwLock::new(consensus)));

        // Initialize placement manager
        let placement_manager = DataPlacementManager::new(
            self.config.sharding.clone(),
            DistributionStrategy::ConsistentHash {
                virtual_nodes: 100,
                hash_ring: vec![],
            },
        )?;
        self.placement_manager = Some(Arc::new(RwLock::new(placement_manager)));

        // Create shutdown channel
        let (shutdown_tx, _shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        info!("✅ Distributed coordinator initialized");
        Ok(())
    }

    /// Start the coordinator (network, consensus, etc.)
    pub async fn start(&mut self) -> Result<()> {
        info!("🌟 Starting distributed coordinator");

        // Start network
        if let Some(ref mut network) = self.network {
            network.start().await?;
        }

        // Start consensus
        if let Some(ref consensus) = self.consensus {
            consensus.write().await.start().await?;
        }

        // Start cache cleanup task
        self.start_cache_cleanup_task();

        info!("✅ Distributed coordinator started");
        Ok(())
    }

    /// Stop the coordinator
    pub async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping distributed coordinator");

        // Signal shutdown
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Stop network
        if let Some(ref mut network) = self.network {
            network.stop().await?;
        }

        // Stop consensus
        if let Some(ref consensus) = self.consensus {
            consensus.write().await.stop().await?;
        }

        info!("✅ Distributed coordinator stopped");
        Ok(())
    }

    /// Execute a distributed query
    pub async fn execute_query(&self, query: Query) -> Result<QueryResult> {
        let query_id = self.generate_query_id(&query);
        info!("📊 Executing distributed query: {}", query_id);

        // Check cache first
        if self.config.enable_query_cache {
            if let Some(cached) = self.check_cache(&query).await {
                debug!("📦 Cache hit for query {}", query_id);
                return Ok(cached);
            }
        }

        // Update status
        self.update_query_status(&query_id, QueryStatus::Planning).await;

        // Create execution plan
        let plan = self.create_query_plan(&query).await?;
        debug!("📋 Query plan created: {:?}", plan);

        // Distribute query
        self.update_query_status(&query_id, QueryStatus::Distributing).await;

        // Execute on target nodes with racing
        self.update_query_status(
            &query_id,
            QueryStatus::Executing {
                completed_shards: 0,
                total_shards: plan.target_shards.len(),
            },
        )
        .await;

        let partial_results = self.execute_query_plan(&plan).await?;

        // Aggregate results
        self.update_query_status(&query_id, QueryStatus::Aggregating).await;
        let result = self.aggregate_results(&plan, partial_results).await?;

        // Update final status
        self.update_query_status(
            &query_id,
            QueryStatus::Completed {
                execution_time_ms: 0, // Would track actual time
                rows_returned: result.data.len() as u64,
            },
        )
        .await;

        // Cache result
        if self.config.enable_query_cache {
            self.cache_result(&query, result.clone()).await;
        }

        info!("✅ Query {} completed", query_id);
        Ok(result)
    }

    /// Create a query execution plan
    async fn create_query_plan(&self, query: &Query) -> Result<QueryPlan> {
        let plan_id = self.generate_query_id(query);

        // Analyze query to determine affected shards
        let (target_shards, shard_nodes) = self.analyze_query_shards(query).await?;

        // Determine aggregation strategy
        let aggregation = self.determine_aggregation_strategy(query)?;

        // Estimate cost
        let estimated_cost = self.estimate_query_cost(query, &target_shards).await?;

        Ok(QueryPlan {
            plan_id,
            query: query.clone(),
            target_shards,
            shard_nodes,
            aggregation,
            estimated_cost,
            created_at: Utc::now(),
        })
    }

    /// Analyze query to determine which shards need to be queried
    async fn analyze_query_shards(
        &self,
        query: &Query,
    ) -> Result<(Vec<String>, HashMap<String, Vec<Uuid>>)> {
        // In a real implementation, this would parse the SQL and determine
        // which tables/partitions are involved

        // For now, return a simplified analysis
        let mut target_shards = Vec::new();
        let mut shard_nodes = HashMap::new();

        // Extract table name from simple SELECT queries
        let sql_lower = query.sql.to_lowercase();
        if sql_lower.contains("from ") {
            // Simple table extraction
            if let Some(placement_manager) = &self.placement_manager {
                let manager = placement_manager.read().await;

                // For demo, assume we need to query all shards
                // Real implementation would extract WHERE clause predicates
                let shards = manager.find_shards_for_key("*").await;

                for shard_id in shards {
                    if let Some(nodes) = manager.get_nodes_for_shard(&shard_id).await {
                        target_shards.push(shard_id.clone());
                        shard_nodes.insert(shard_id, nodes);
                    }
                }
            }
        }

        // If no specific shards found, use default shard
        if target_shards.is_empty() {
            target_shards.push("default".to_string());
            shard_nodes.insert("default".to_string(), vec![self.config.node_id]);
        }

        Ok((target_shards, shard_nodes))
    }

    /// Determine how to aggregate partial results based on query type
    fn determine_aggregation_strategy(&self, query: &Query) -> Result<AggregationStrategy> {
        let sql_lower = query.sql.to_lowercase();

        // Check for GROUP BY
        if sql_lower.contains("group by") {
            // Extract group by columns (simplified)
            let group_by = Vec::new();
            let mut aggregates = Vec::new();

            // Simple parsing for demo
            if sql_lower.contains("count(") {
                aggregates.push(AggregateFunction {
                    name: "count".to_string(),
                    column: "*".to_string(),
                    alias: "count".to_string(),
                });
            }
            if sql_lower.contains("sum(") {
                aggregates.push(AggregateFunction {
                    name: "sum".to_string(),
                    column: "amount".to_string(),
                    alias: "total".to_string(),
                });
            }

            return Ok(AggregationStrategy::GroupAggregate {
                group_by,
                aggregates,
            });
        }

        // Check for ORDER BY
        if sql_lower.contains("order by") {
            let sort_keys = vec!["id".to_string()]; // Simplified
            return Ok(AggregationStrategy::MergeSorted { sort_keys });
        }

        // Default to union for multiple shards
        Ok(AggregationStrategy::Union)
    }

    /// Estimate query execution cost
    async fn estimate_query_cost(
        &self,
        _query: &Query,
        target_shards: &[String],
    ) -> Result<QueryCost> {
        // Simplified cost estimation
        let shard_count = target_shards.len();

        Ok(QueryCost {
            rows_to_scan: 10000 * shard_count as u64,
            bytes_to_transfer: 1024 * 1024 * shard_count as u64,
            estimated_time_ms: 100 + (50 * shard_count as u64),
            shard_count,
            network_hops: 1,
        })
    }

    /// Execute query plan across nodes
    async fn execute_query_plan(&self, plan: &QueryPlan) -> Result<Vec<PartialResult>> {
        let mut all_results = Vec::new();

        // Execute on each shard in parallel
        let mut handles = Vec::new();

        for shard_id in &plan.target_shards {
            let shard_id = shard_id.clone();
            let plan_id = plan.plan_id.clone();
            let query = plan.query.clone();
            let nodes = plan
                .shard_nodes
                .get(&shard_id)
                .cloned()
                .unwrap_or_default();

            let racing_parallelism = self.config.racing_parallelism;
            let verification_quorum = self.config.verification_quorum;

            let handle = tokio::spawn(async move {
                execute_on_shard_with_racing(
                    shard_id,
                    plan_id,
                    query,
                    nodes,
                    racing_parallelism,
                    verification_quorum,
                )
                .await
            });

            handles.push(handle);
        }

        // Collect results
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => all_results.push(result),
                Ok(Err(e)) => warn!("Shard query failed: {}", e),
                Err(e) => warn!("Task join error: {}", e),
            }
        }

        Ok(all_results)
    }

    /// Aggregate partial results from multiple shards
    async fn aggregate_results(
        &self,
        plan: &QueryPlan,
        partial_results: Vec<PartialResult>,
    ) -> Result<QueryResult> {
        if partial_results.is_empty() {
            return Ok(QueryResult { data: vec![] });
        }

        match &plan.aggregation {
            AggregationStrategy::None => {
                // Single result, no aggregation needed
                Ok(QueryResult {
                    data: partial_results[0].data.clone(),
                })
            }
            AggregationStrategy::Union => {
                // Concatenate all results
                let mut combined = Vec::new();
                for partial in partial_results {
                    combined.extend(partial.data);
                }
                Ok(QueryResult { data: combined })
            }
            AggregationStrategy::MergeSorted { sort_keys: _ } => {
                // Merge sorted results (simplified - just concatenate for demo)
                let mut combined = Vec::new();
                for partial in partial_results {
                    combined.extend(partial.data);
                }
                Ok(QueryResult { data: combined })
            }
            AggregationStrategy::GroupAggregate {
                group_by: _,
                aggregates: _,
            } => {
                // Aggregate grouped results (simplified)
                let mut combined = Vec::new();
                for partial in partial_results {
                    combined.extend(partial.data);
                }
                Ok(QueryResult { data: combined })
            }
            AggregationStrategy::Custom { function_name } => {
                Err(anyhow!(
                    "Custom aggregation function '{}' not implemented",
                    function_name
                ))
            }
        }
    }

    /// Check query cache
    async fn check_cache(&self, query: &Query) -> Option<QueryResult> {
        let cache_key = self.generate_query_id(query);
        let mut cache = self.query_cache.write().await;

        if let Some(cached) = cache.get_mut(&cache_key) {
            let age = Utc::now()
                .signed_duration_since(cached.created_at)
                .num_seconds();
            if age < self.config.cache_ttl_secs as i64 {
                cached.hit_count += 1;
                return Some(cached.result.clone());
            } else {
                cache.remove(&cache_key);
            }
        }

        None
    }

    /// Cache a query result
    async fn cache_result(&self, query: &Query, result: QueryResult) {
        let cache_key = self.generate_query_id(query);
        let mut cache = self.query_cache.write().await;

        cache.insert(
            cache_key,
            CachedResult {
                result,
                created_at: Utc::now(),
                hit_count: 0,
            },
        );
    }

    /// Update query status
    async fn update_query_status(&self, query_id: &str, status: QueryStatus) {
        let mut active = self.active_queries.write().await;
        active.insert(query_id.to_string(), status);
    }

    /// Get query status
    pub async fn get_query_status(&self, query_id: &str) -> Option<QueryStatus> {
        let active = self.active_queries.read().await;
        active.get(query_id).cloned()
    }

    /// Cancel a running query
    pub async fn cancel_query(&self, query_id: &str) -> Result<()> {
        let mut active = self.active_queries.write().await;
        if active.contains_key(query_id) {
            active.insert(query_id.to_string(), QueryStatus::Cancelled);
            Ok(())
        } else {
            Err(anyhow!("Query {} not found", query_id))
        }
    }

    /// Get coordinator statistics
    pub async fn get_stats(&self) -> CoordinatorStats {
        let cache = self.query_cache.read().await;
        let active = self.active_queries.read().await;

        let cache_hits: u64 = cache.values().map(|c| c.hit_count).sum();
        let completed_queries = active
            .values()
            .filter(|s| matches!(s, QueryStatus::Completed { .. }))
            .count();

        CoordinatorStats {
            node_id: self.config.node_id,
            active_queries: active
                .values()
                .filter(|s| !matches!(s, QueryStatus::Completed { .. } | QueryStatus::Failed { .. } | QueryStatus::Cancelled))
                .count(),
            completed_queries,
            failed_queries: active
                .values()
                .filter(|s| matches!(s, QueryStatus::Failed { .. }))
                .count(),
            cache_size: cache.len(),
            cache_hits,
            uptime_secs: 0, // Would track actual uptime
        }
    }

    /// Generate a unique query ID based on query content
    fn generate_query_id(&self, query: &Query) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.sql.as_bytes());
        let hash = hasher.finalize();
        format!("q_{:x}", hash)[..24].to_string()
    }

    /// Start background cache cleanup task
    fn start_cache_cleanup_task(&self) {
        let cache = self.query_cache.clone();
        let ttl = self.config.cache_ttl_secs;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                let mut cache_guard = cache.write().await;
                let now = Utc::now();

                cache_guard.retain(|_, cached| {
                    now.signed_duration_since(cached.created_at).num_seconds() < ttl as i64
                });
            }
        });
    }
}

/// Execute query on a shard with racing competition
async fn execute_on_shard_with_racing(
    shard_id: String,
    plan_id: String,
    query: Query,
    nodes: Vec<Uuid>,
    racing_parallelism: usize,
    verification_quorum: usize,
) -> Result<PartialResult> {
    if nodes.is_empty() {
        return Err(anyhow!("No nodes available for shard {}", shard_id));
    }

    // Select nodes for racing (up to racing_parallelism)
    let racing_nodes: Vec<Uuid> = nodes
        .into_iter()
        .take(racing_parallelism)
        .collect();

    debug!(
        "Racing query on shard {} across {} nodes",
        shard_id,
        racing_nodes.len()
    );

    // Spawn racing execution on each node
    let (tx, mut rx) = mpsc::channel::<Result<PartialResult>>(racing_nodes.len());

    for node_id in racing_nodes {
        let tx = tx.clone();
        let shard_id = shard_id.clone();
        let plan_id = plan_id.clone();
        let query = query.clone();

        tokio::spawn(async move {
            // Simulate query execution on node
            // In real implementation, this would send RPC to the node
            let start = std::time::Instant::now();

            // Simulate execution delay
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let result = PartialResult {
                node_id,
                shard_id,
                plan_id,
                data: vec![], // Would contain actual results
                row_count: 0,
                execution_time_ms: start.elapsed().as_millis() as u64,
                result_hash: compute_result_hash(&query.sql),
                timestamp: Utc::now(),
            };

            let _ = tx.send(Ok(result)).await;
        });
    }

    drop(tx); // Drop original sender so receiver knows when all senders are done

    // Collect results until we have quorum or all failed
    let mut results = Vec::new();

    while let Some(result) = rx.recv().await {
        match result {
            Ok(partial) => {
                results.push(partial);

                // Check if we have verification quorum
                if results.len() >= verification_quorum {
                    // Verify results match (simplified - just compare hashes)
                    let first_hash = &results[0].result_hash;
                    let matching = results
                        .iter()
                        .filter(|r| &r.result_hash == first_hash)
                        .count();

                    if matching >= verification_quorum {
                        debug!(
                            "Shard {} query completed with {} matching results",
                            shard_id, matching
                        );
                        return Ok(results.remove(0));
                    }
                }
            }
            Err(e) => {
                warn!("Node query failed: {}", e);
            }
        }
    }

    // Return best result we have
    if !results.is_empty() {
        Ok(results.remove(0))
    } else {
        Err(anyhow!("All nodes failed for shard {}", shard_id))
    }
}

/// Compute hash of query result for verification
fn compute_result_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

impl Default for DistributedCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Coordinator statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorStats {
    pub node_id: Uuid,
    pub active_queries: usize,
    pub completed_queries: usize,
    pub failed_queries: usize,
    pub cache_size: usize,
    pub cache_hits: u64,
    pub uptime_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() {
        let coordinator = DistributedCoordinator::new();
        assert!(coordinator.network.is_none());
        assert!(coordinator.consensus.is_none());
    }

    #[test]
    fn test_coordinator_with_config() {
        let config = CoordinatorConfig {
            racing_parallelism: 5,
            verification_quorum: 3,
            ..Default::default()
        };
        let coordinator = DistributedCoordinator::with_config(config.clone());
        assert_eq!(coordinator.config.racing_parallelism, 5);
        assert_eq!(coordinator.config.verification_quorum, 3);
    }

    #[test]
    fn test_query_id_generation() {
        let coordinator = DistributedCoordinator::new();
        let query1 = Query {
            sql: "SELECT * FROM transactions".to_string(),
        };
        let query2 = Query {
            sql: "SELECT * FROM transactions".to_string(),
        };
        let query3 = Query {
            sql: "SELECT * FROM accounts".to_string(),
        };

        let id1 = coordinator.generate_query_id(&query1);
        let id2 = coordinator.generate_query_id(&query2);
        let id3 = coordinator.generate_query_id(&query3);

        assert_eq!(id1, id2); // Same query should have same ID
        assert_ne!(id1, id3); // Different queries should have different IDs
    }

    #[tokio::test]
    async fn test_aggregation_strategy_detection() {
        let coordinator = DistributedCoordinator::new();

        // Test GROUP BY detection
        let query1 = Query {
            sql: "SELECT COUNT(*) FROM tx GROUP BY account".to_string(),
        };
        let strategy1 = coordinator.determine_aggregation_strategy(&query1).unwrap();
        assert!(matches!(
            strategy1,
            AggregationStrategy::GroupAggregate { .. }
        ));

        // Test ORDER BY detection
        let query2 = Query {
            sql: "SELECT * FROM tx ORDER BY timestamp".to_string(),
        };
        let strategy2 = coordinator.determine_aggregation_strategy(&query2).unwrap();
        assert!(matches!(strategy2, AggregationStrategy::MergeSorted { .. }));

        // Test default (Union)
        let query3 = Query {
            sql: "SELECT * FROM transactions".to_string(),
        };
        let strategy3 = coordinator.determine_aggregation_strategy(&query3).unwrap();
        assert!(matches!(strategy3, AggregationStrategy::Union));
    }

    #[tokio::test]
    async fn test_query_status_tracking() {
        let coordinator = DistributedCoordinator::new();

        coordinator
            .update_query_status("test_query", QueryStatus::Planning)
            .await;

        let status = coordinator.get_query_status("test_query").await;
        assert!(matches!(status, Some(QueryStatus::Planning)));

        coordinator
            .update_query_status(
                "test_query",
                QueryStatus::Completed {
                    execution_time_ms: 100,
                    rows_returned: 50,
                },
            )
            .await;

        let status = coordinator.get_query_status("test_query").await;
        assert!(matches!(status, Some(QueryStatus::Completed { .. })));
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let mut config = CoordinatorConfig::default();
        config.enable_query_cache = true;
        config.cache_ttl_secs = 60;

        let coordinator = DistributedCoordinator::with_config(config);

        let query = Query {
            sql: "SELECT * FROM test".to_string(),
        };
        let result = QueryResult {
            data: vec![1, 2, 3, 4],
        };

        // Cache should be empty initially
        assert!(coordinator.check_cache(&query).await.is_none());

        // Add to cache
        coordinator.cache_result(&query, result.clone()).await;

        // Should find in cache
        let cached = coordinator.check_cache(&query).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().data, result.data);
    }

    #[tokio::test]
    async fn test_cancel_query() {
        let coordinator = DistributedCoordinator::new();

        coordinator
            .update_query_status(
                "cancel_test",
                QueryStatus::Executing {
                    completed_shards: 1,
                    total_shards: 5,
                },
            )
            .await;

        let result = coordinator.cancel_query("cancel_test").await;
        assert!(result.is_ok());

        let status = coordinator.get_query_status("cancel_test").await;
        assert!(matches!(status, Some(QueryStatus::Cancelled)));
    }
}
