//! Query routing and load balancing for distributed queries
//!
//! This module implements intelligent query routing that can distribute queries
//! across the network based on various strategies including load balancing,
//! data locality, node capabilities, and racing competition.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{interval, timeout};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Query routing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Route to least loaded node
    LeastLoaded,
    /// Route based on data locality/sharding
    DataLocality,
    /// Route to node with specific capability
    CapabilityBased { required_capability: String },
    /// Route to geographically closest node
    GeographicProximity { region: String },
    /// Route based on response time
    LowestLatency,
    /// Broadcast to multiple nodes for redundancy
    Broadcast { replica_count: usize },
}

/// Query types supported by the router
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    /// Transaction lookup by signature
    TransactionLookup { signature: String },
    /// Block data query
    BlockQuery { slot: u64 },
    /// Account information query
    AccountQuery { pubkey: String },
    /// Program data query
    ProgramQuery { program_id: String },
    /// Time-range query
    TimeRangeQuery { start_time: u64, end_time: u64 },
    /// Aggregate/analytics query
    AggregateQuery { query_type: String, parameters: HashMap<String, String> },
    /// Custom SQL query
    SqlQuery { sql: String },
}

/// Query request with routing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// Unique request identifier
    pub id: Uuid,
    /// Type of query
    pub query_type: QueryType,
    /// Preferred routing strategy
    pub routing_strategy: Option<RoutingStrategy>,
    /// Query timeout
    pub timeout_ms: u64,
    /// Priority (0 = lowest, 100 = highest)
    pub priority: u8,
    /// Requester identifier
    pub requester_id: Uuid,
    /// When the request was created (seconds since epoch)
    #[serde(with = "instant_as_secs")]
    pub created_at: Instant,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// Request ID this responds to
    pub request_id: Uuid,
    /// Node that processed the query
    pub processor_node: Uuid,
    /// Query results
    pub results: serde_json::Value,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Whether the query succeeded
    pub success: bool,
    /// Error message if query failed
    pub error: Option<String>,
    /// Response metadata
    pub metadata: HashMap<String, String>,
}

/// Node information for routing decisions
#[derive(Debug, Clone)]
pub struct RouteTarget {
    /// Node identifier
    pub node_id: Uuid,
    /// Node's capabilities
    pub capabilities: Vec<String>,
    /// Current load (0.0 = idle, 1.0 = fully loaded)
    pub current_load: f64,
    /// Average response time
    pub avg_response_time_ms: f64,
    /// Geographic region
    pub region: Option<String>,
    /// Whether node is currently available
    pub available: bool,
    /// Last health check timestamp
    pub last_health_check: Instant,
    /// Number of queries currently being processed
    pub active_queries: usize,
    /// Historical success rate
    pub success_rate: f64,
}

/// Load balancing algorithms
pub struct LoadBalancer {
    /// Current round-robin position
    round_robin_index: usize,
    /// Node performance history
    performance_history: HashMap<Uuid, VecDeque<f64>>,
}

/// Query routing and load balancing manager
pub struct QueryRouter {
    /// Available route targets (nodes)
    targets: Arc<RwLock<HashMap<Uuid, RouteTarget>>>,
    /// Load balancer instance
    load_balancer: Arc<RwLock<LoadBalancer>>,
    /// Default routing strategy
    default_strategy: RoutingStrategy,
    /// Query timeout configuration
    default_timeout_ms: u64,
    /// Event broadcaster for routing events
    event_sender: broadcast::Sender<QueryRoutingEvent>,
    /// Router statistics
    stats: Arc<RwLock<QueryRouterStats>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

/// Query routing events
#[derive(Debug, Clone)]
pub enum QueryRoutingEvent {
    /// Query routed to a node
    QueryRouted { request_id: Uuid, target_node: Uuid },
    /// Query completed
    QueryCompleted { request_id: Uuid, success: bool, processing_time_ms: u64 },
    /// Node added to routing pool
    NodeAdded { node_id: Uuid },
    /// Node removed from routing pool
    NodeRemoved { node_id: Uuid },
    /// Node health status changed
    NodeHealthChanged { node_id: Uuid, available: bool },
    /// Load balancing reconfigured
    LoadBalancingReconfigured { strategy: String },
}

/// Router statistics
#[derive(Debug, Clone)]
pub struct QueryRouterStats {
    pub total_queries_routed: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub average_response_time_ms: f64,
    pub active_targets: usize,
    pub queries_by_strategy: HashMap<String, u64>,
    pub queries_by_type: HashMap<String, u64>,
    pub load_balance_decisions: u64,
    pub circuit_breaker_trips: u64,
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            round_robin_index: 0,
            performance_history: HashMap::new(),
        }
    }

    /// Select best target based on strategy
    pub fn select_target(
        &mut self,
        targets: &[RouteTarget],
        strategy: &RoutingStrategy,
        query_type: &QueryType,
    ) -> Option<Uuid> {
        let available_targets: Vec<_> = targets.iter()
            .filter(|t| t.available)
            .collect();

        if available_targets.is_empty() {
            return None;
        }

        match strategy {
            RoutingStrategy::RoundRobin => {
                let target = &available_targets[self.round_robin_index % available_targets.len()];
                self.round_robin_index += 1;
                Some(target.node_id)
            }
            RoutingStrategy::LeastLoaded => {
                available_targets
                    .iter()
                    .min_by(|a, b| a.current_load.partial_cmp(&b.current_load).unwrap())
                    .map(|t| t.node_id)
            }
            RoutingStrategy::LowestLatency => {
                available_targets
                    .iter()
                    .min_by(|a, b| a.avg_response_time_ms.partial_cmp(&b.avg_response_time_ms).unwrap())
                    .map(|t| t.node_id)
            }
            RoutingStrategy::DataLocality => {
                // For now, use capability-based routing as proxy for data locality
                self.select_by_capability(&available_targets, query_type)
            }
            RoutingStrategy::CapabilityBased { required_capability } => {
                available_targets
                    .iter()
                    .find(|t| t.capabilities.contains(required_capability))
                    .map(|t| t.node_id)
            }
            RoutingStrategy::GeographicProximity { region } => {
                available_targets
                    .iter()
                    .find(|t| t.region.as_ref() == Some(region))
                    .or_else(|| available_targets.first())
                    .map(|t| t.node_id)
            }
            RoutingStrategy::Broadcast { replica_count: _ } => {
                // For broadcast, return first available target
                // In real implementation, would return multiple targets
                available_targets.first().map(|t| t.node_id)
            }
        }
    }

    fn select_by_capability(&self, targets: &[&RouteTarget], query_type: &QueryType) -> Option<Uuid> {
        let required_capability = match query_type {
            QueryType::TransactionLookup { .. } => "transaction_indexing",
            QueryType::BlockQuery { .. } => "block_data",
            QueryType::AccountQuery { .. } => "account_data",
            QueryType::ProgramQuery { .. } => "program_data",
            QueryType::AggregateQuery { .. } => "analytics",
            QueryType::SqlQuery { .. } => "sql_query",
            _ => "general",
        };

        targets
            .iter()
            .find(|t| t.capabilities.contains(&required_capability.to_string()))
            .or_else(|| targets.first())
            .map(|t| t.node_id)
    }

    /// Update node performance metrics
    pub fn update_performance(&mut self, node_id: Uuid, response_time_ms: f64) {
        let history = self.performance_history.entry(node_id).or_insert_with(VecDeque::new);

        history.push_back(response_time_ms);
        if history.len() > 100 {
            history.pop_front();
        }
    }

    /// Get average performance for a node
    pub fn get_avg_performance(&self, node_id: &Uuid) -> f64 {
        self.performance_history
            .get(node_id)
            .and_then(|history| {
                if history.is_empty() {
                    None
                } else {
                    Some(history.iter().sum::<f64>() / history.len() as f64)
                }
            })
            .unwrap_or(0.0)
    }
}

impl QueryRouter {
    /// Create a new query router
    pub async fn new(
        default_strategy: RoutingStrategy,
        default_timeout_ms: u64,
    ) -> Result<Self> {
        let (event_sender, _) = broadcast::channel(1000);

        let stats = QueryRouterStats {
            total_queries_routed: 0,
            successful_queries: 0,
            failed_queries: 0,
            average_response_time_ms: 0.0,
            active_targets: 0,
            queries_by_strategy: HashMap::new(),
            queries_by_type: HashMap::new(),
            load_balance_decisions: 0,
            circuit_breaker_trips: 0,
        };

        Ok(Self {
            targets: Arc::new(RwLock::new(HashMap::new())),
            load_balancer: Arc::new(RwLock::new(LoadBalancer::new())),
            default_strategy,
            default_timeout_ms,
            event_sender,
            stats: Arc::new(RwLock::new(stats)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the query router
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(anyhow!("Query router is already running"));
        }

        info!("Starting query router");

        // Start health monitoring
        self.start_health_monitoring().await;

        // Start metrics collection
        self.start_metrics_collection().await;

        *running = true;
        info!("Query router started");
        Ok(())
    }

    /// Stop the query router
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        info!("Stopping query router");
        *running = false;
        Ok(())
    }

    /// Add a target node for routing
    pub async fn add_target(&self, target: RouteTarget) -> Result<()> {
        let mut targets = self.targets.write().await;
        let node_id = target.node_id;

        targets.insert(node_id, target);
        drop(targets);

        let _ = self.event_sender.send(QueryRoutingEvent::NodeAdded { node_id });

        // Update stats
        let mut stats = self.stats.write().await;
        stats.active_targets = self.targets.read().await.len();

        info!("Added routing target: {}", node_id);
        Ok(())
    }

    /// Remove a target node
    pub async fn remove_target(&self, node_id: &Uuid) -> Result<()> {
        let mut targets = self.targets.write().await;
        if targets.remove(node_id).is_some() {
            drop(targets);

            let _ = self.event_sender.send(QueryRoutingEvent::NodeRemoved { node_id: *node_id });

            // Update stats
            let mut stats = self.stats.write().await;
            stats.active_targets = self.targets.read().await.len();

            info!("Removed routing target: {}", node_id);
        }
        Ok(())
    }

    /// Route a query to the best available node
    pub async fn route_query(&self, request: QueryRequest) -> Result<Uuid> {
        let strategy = request.routing_strategy.as_ref()
            .unwrap_or(&self.default_strategy).clone();

        let targets = self.targets.read().await;
        let target_list: Vec<RouteTarget> = targets.values().cloned().collect();
        drop(targets);

        let mut load_balancer = self.load_balancer.write().await;
        let selected_node = load_balancer
            .select_target(&target_list, &strategy, &request.query_type)
            .ok_or_else(|| anyhow!("No available targets for query routing"))?;

        drop(load_balancer);

        // Update statistics
        self.update_routing_stats(&request, &selected_node).await;

        // Send routing event
        let _ = self.event_sender.send(QueryRoutingEvent::QueryRouted {
            request_id: request.id,
            target_node: selected_node,
        });

        debug!("Routed query {} to node {}", request.id, selected_node);
        Ok(selected_node)
    }

    /// Record query completion for metrics
    pub async fn record_query_completion(
        &self,
        request_id: Uuid,
        node_id: Uuid,
        success: bool,
        processing_time_ms: u64,
    ) -> Result<()> {
        // Update load balancer performance metrics
        let mut load_balancer = self.load_balancer.write().await;
        load_balancer.update_performance(node_id, processing_time_ms as f64);
        drop(load_balancer);

        // Update node metrics
        let mut targets = self.targets.write().await;
        if let Some(target) = targets.get_mut(&node_id) {
            target.avg_response_time_ms = (target.avg_response_time_ms + processing_time_ms as f64) / 2.0;
            if target.active_queries > 0 {
                target.active_queries -= 1;
            }
        }
        drop(targets);

        // Update router statistics
        let mut stats = self.stats.write().await;
        if success {
            stats.successful_queries += 1;
        } else {
            stats.failed_queries += 1;
        }

        let total_queries = stats.successful_queries + stats.failed_queries;
        if total_queries > 0 {
            stats.average_response_time_ms = (stats.average_response_time_ms * (total_queries - 1) as f64
                + processing_time_ms as f64) / total_queries as f64;
        }

        drop(stats);

        // Send completion event
        let _ = self.event_sender.send(QueryRoutingEvent::QueryCompleted {
            request_id,
            success,
            processing_time_ms,
        });

        debug!("Recorded completion for query {}: success={}, time={}ms",
               request_id, success, processing_time_ms);
        Ok(())
    }

    /// Get router statistics
    pub async fn get_stats(&self) -> QueryRouterStats {
        self.stats.read().await.clone()
    }

    /// Subscribe to routing events
    pub fn subscribe(&self) -> broadcast::Receiver<QueryRoutingEvent> {
        self.event_sender.subscribe()
    }

    /// Update node health status
    pub async fn update_node_health(&self, node_id: Uuid, available: bool) -> Result<()> {
        let mut targets = self.targets.write().await;
        if let Some(target) = targets.get_mut(&node_id) {
            target.available = available;
            target.last_health_check = Instant::now();

            let _ = self.event_sender.send(QueryRoutingEvent::NodeHealthChanged {
                node_id,
                available,
            });

            debug!("Updated health for node {}: available={}", node_id, available);
        }
        Ok(())
    }

    /// Get recommended targets for a query
    pub async fn get_recommended_targets(&self, request: &QueryRequest) -> Result<Vec<Uuid>> {
        let strategy = request.routing_strategy
            .as_ref()
            .unwrap_or(&self.default_strategy);

        match strategy {
            RoutingStrategy::Broadcast { replica_count } => {
                let targets = self.targets.read().await;
                let available_targets: Vec<Uuid> = targets.values()
                    .filter(|t| t.available)
                    .take(*replica_count)
                    .map(|t| t.node_id)
                    .collect();
                Ok(available_targets)
            }
            _ => {
                // For non-broadcast strategies, return single target
                let target = self.route_query(request.clone()).await?;
                Ok(vec![target])
            }
        }
    }

    async fn update_routing_stats(&self, request: &QueryRequest, target_node: &Uuid) {
        let mut stats = self.stats.write().await;

        stats.total_queries_routed += 1;
        stats.load_balance_decisions += 1;

        // Update strategy stats
        let strategy_name = format!("{:?}", request.routing_strategy.as_ref().unwrap_or(&self.default_strategy));
        *stats.queries_by_strategy.entry(strategy_name).or_insert(0) += 1;

        // Update query type stats
        let query_type_name = format!("{:?}", request.query_type);
        *stats.queries_by_type.entry(query_type_name).or_insert(0) += 1;

        // Update target load
        drop(stats);
        let mut targets = self.targets.write().await;
        if let Some(target) = targets.get_mut(target_node) {
            target.active_queries += 1;
        }
    }

    async fn start_health_monitoring(&self) {
        let targets = self.targets.clone();
        let event_sender = self.event_sender.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                debug!("Running health monitoring cycle");

                let targets_guard = targets.read().await;
                let now = Instant::now();

                for (node_id, target) in targets_guard.iter() {
                    // Check if node hasn't been checked recently
                    if now.duration_since(target.last_health_check) > Duration::from_secs(120) {
                        // Mark as potentially unhealthy
                        let _ = event_sender.send(QueryRoutingEvent::NodeHealthChanged {
                            node_id: *node_id,
                            available: false,
                        });
                    }
                }
            }
        });
    }

    async fn start_metrics_collection(&self) {
        let stats = self.stats.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Collect every minute

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                debug!("Collecting query router metrics");

                let stats_guard = stats.read().await;
                if stats_guard.total_queries_routed > 0 {
                    let success_rate = stats_guard.successful_queries as f64 / stats_guard.total_queries_routed as f64;
                    debug!("Query router metrics: total={}, success_rate={:.2}, avg_response_time={:.2}ms",
                           stats_guard.total_queries_routed,
                           success_rate,
                           stats_guard.average_response_time_ms);
                }
            }
        });
    }

    /// Execute a racing query - multiple nodes compete to answer first
    ///
    /// Racing competition rules:
    /// - 3-5 nodes are selected to race
    /// - First correct answer wins (70% of reward)
    /// - Two verifiers each get 15% of reward
    /// - Query is considered successful when winner + 2 verifiers agree
    pub async fn execute_racing_query<F, Fut>(
        &self,
        request: QueryRequest,
        execute_on_node: F,
    ) -> Result<RacingResult>
    where
        F: Fn(Uuid, QueryRequest) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<QueryResponse>> + Send + 'static,
    {
        info!("Starting racing query for request {}", request.id);

        // Select 3-5 racing candidates
        let candidates = self.select_racing_candidates(3, 5).await?;
        if candidates.len() < 3 {
            return Err(anyhow!("Not enough nodes for racing (need at least 3, have {})", candidates.len()));
        }

        let candidate_count = candidates.len();
        info!("Selected {} racing candidates", candidate_count);

        // Create channel for receiving results
        let (tx, mut rx) = mpsc::channel::<(Uuid, QueryResponse)>(candidate_count);

        // Spawn parallel queries
        let request_clone = request.clone();
        for node_id in &candidates {
            let tx = tx.clone();
            let node_id = *node_id;
            let req = request_clone.clone();
            let execute_fn = execute_on_node.clone();

            tokio::spawn(async move {
                let start = Instant::now();
                match execute_fn(node_id, req).await {
                    Ok(mut response) => {
                        response.processing_time_ms = start.elapsed().as_millis() as u64;
                        let _ = tx.send((node_id, response)).await;
                    }
                    Err(e) => {
                        warn!("Racing query failed on node {}: {}", node_id, e);
                    }
                }
            });
        }

        // Drop our sender so channel closes when all senders done
        drop(tx);

        // Wait for first correct answer with timeout
        let query_timeout = Duration::from_millis(request.timeout_ms);
        let mut winner: Option<(Uuid, QueryResponse)> = None;
        let mut verifiers: Vec<(Uuid, QueryResponse)> = Vec::new();
        let mut all_responses: Vec<(Uuid, QueryResponse)> = Vec::new();

        let receive_start = Instant::now();
        while let Ok(Some((node_id, response))) = timeout(
            query_timeout.saturating_sub(receive_start.elapsed()),
            rx.recv()
        ).await {
            if !response.success {
                debug!("Ignoring failed response from node {}", node_id);
                continue;
            }

            all_responses.push((node_id, response.clone()));

            if winner.is_none() {
                // First successful response is tentative winner
                winner = Some((node_id, response));
                info!("Tentative winner: node {}", node_id);
            } else if verifiers.len() < 2 {
                // Check if this matches the winner's result
                let winner_response = &winner.as_ref().unwrap().1;
                if self.verify_result_match(&winner_response.results, &response.results) {
                    verifiers.push((node_id, response));
                    info!("Verifier {} confirmed result (total: {})", node_id, verifiers.len());

                    // If we have winner + 2 verifiers, we're done
                    if verifiers.len() >= 2 {
                        break;
                    }
                } else {
                    warn!("Node {} disagrees with winner - potential inconsistency", node_id);
                }
            }
        }

        // Validate we have enough consensus
        if winner.is_none() {
            return Err(anyhow!("No successful responses in racing query"));
        }

        let (winner_id, winner_response) = winner.unwrap();

        if verifiers.len() < 2 {
            warn!("Insufficient verifiers ({}/2) for racing query", verifiers.len());
            // Still return result but with lower confidence
        }

        // Record query completion for all participating nodes
        for (node_id, response) in &all_responses {
            let success = *node_id == winner_id || verifiers.iter().any(|(v, _)| v == node_id);
            self.record_query_completion(
                request.id,
                *node_id,
                success,
                response.processing_time_ms,
            ).await?;
        }

        let verifier_ids: Vec<Uuid> = verifiers.iter().map(|(id, _)| *id).collect();

        info!(
            "Racing query complete: winner={}, verifiers={:?}, time={}ms",
            winner_id,
            verifier_ids,
            winner_response.processing_time_ms
        );

        Ok(RacingResult {
            request_id: request.id,
            winner_id,
            winner_response,
            verifier_ids,
            total_participants: all_responses.len(),
            consensus_reached: verifiers.len() >= 2,
        })
    }

    /// Select candidates for racing competition
    async fn select_racing_candidates(&self, min_count: usize, max_count: usize) -> Result<Vec<Uuid>> {
        let targets = self.targets.read().await;

        let available: Vec<Uuid> = targets.values()
            .filter(|t| t.available && t.success_rate > 0.8)
            .map(|t| t.node_id)
            .collect();

        if available.len() < min_count {
            return Err(anyhow!(
                "Not enough available nodes for racing: need {}, have {}",
                min_count, available.len()
            ));
        }

        // Select up to max_count nodes, preferring those with lower latency
        let mut candidates: Vec<_> = targets.values()
            .filter(|t| available.contains(&t.node_id))
            .cloned()
            .collect();

        // Sort by latency (lower is better)
        candidates.sort_by(|a, b| {
            a.avg_response_time_ms.partial_cmp(&b.avg_response_time_ms).unwrap()
        });

        Ok(candidates.into_iter()
            .take(max_count)
            .map(|t| t.node_id)
            .collect())
    }

    /// Verify that two query results match
    fn verify_result_match(&self, result1: &serde_json::Value, result2: &serde_json::Value) -> bool {
        // Simple equality check - in production would use hash or merkle comparison
        result1 == result2
    }

    /// Select verifier nodes (different from the racing winner)
    pub async fn select_verifiers(&self, count: usize, exclude: &[Uuid]) -> Result<Vec<Uuid>> {
        let targets = self.targets.read().await;

        let verifiers: Vec<Uuid> = targets.values()
            .filter(|t| t.available && !exclude.contains(&t.node_id))
            .take(count)
            .map(|t| t.node_id)
            .collect();

        if verifiers.len() < count {
            return Err(anyhow!(
                "Not enough verifier nodes: need {}, have {}",
                count, verifiers.len()
            ));
        }

        Ok(verifiers)
    }
}

/// Result of a racing query
#[derive(Debug, Clone)]
pub struct RacingResult {
    /// Original request ID
    pub request_id: Uuid,
    /// Winning node ID
    pub winner_id: Uuid,
    /// Winner's response
    pub winner_response: QueryResponse,
    /// Verifier node IDs
    pub verifier_ids: Vec<Uuid>,
    /// Total number of nodes that responded
    pub total_participants: usize,
    /// Whether consensus was reached (winner + 2 verifiers)
    pub consensus_reached: bool,
}

impl RacingResult {
    /// Calculate reward distribution
    ///
    /// Returns (winner_amount, verifier_amount) as basis points of total
    pub fn reward_distribution(&self) -> (u16, u16) {
        // Winner gets 70%, each verifier gets 15%
        (7000, 1500)
    }
}

impl Default for QueryRouterStats {
    fn default() -> Self {
        Self {
            total_queries_routed: 0,
            successful_queries: 0,
            failed_queries: 0,
            average_response_time_ms: 0.0,
            active_targets: 0,
            queries_by_strategy: HashMap::new(),
            queries_by_type: HashMap::new(),
            load_balance_decisions: 0,
            circuit_breaker_trips: 0,
        }
    }
}

// Serde helper module for Instant serialization
mod instant_as_secs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(_instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert to system time approximation for serialization
        let approx_duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        approx_duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _secs = u64::deserialize(deserializer)?;
        // Since Instant is relative to program start, we'll just use now
        // In a real implementation, you'd want a better approach
        Ok(Instant::now())
    }
}