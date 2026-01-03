use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use axum::{
    extract::State,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use chrono;

// Import core libraries
use networking_core::{NetworkTransport, config::NetworkConfig};
use networking_core::transport::{NngTransport, TransportStats};
use networking_core::discovery::{DiscoveryManager, DiscoveryMethod, NetworkTopology};
use networking_core::discovery::DiscoveryConfig;
use sharding_core::{ShardManager, ShardConfig, config::HashFunctionType, manager::ClusterStats};
use solana_indexer::{TransactionIndexer, SolanaConfig, IndexingStats};
use storage_core::{StorageManager, StorageConfig};
use storage_core::manager::StorageStats;

use crate::config::NodeConfig;
use crate::consensus::{ConsensusCoordinator, ConsensusStats};
use crate::query_router::{QueryRouter, RoutingStrategy, RouteTarget};

/// Shared application state for API endpoints
#[derive(Clone)]
pub struct AppState {
    pub node_info: NodeInfo,
    pub transaction_indexer: Option<Arc<TransactionIndexer>>,
    pub storage_manager: Option<Arc<RwLock<StorageManager>>>,
    pub consensus_coordinator: Option<Arc<RwLock<ConsensusCoordinator>>>,
    pub discovery_manager: Option<Arc<RwLock<DiscoveryManager>>>,
    pub query_router: Option<Arc<RwLock<QueryRouter>>>,
    pub shard_manager: Option<Arc<ShardManager>>,
}

/// Main StreamSync node that orchestrates all components
pub struct StreamSyncNode {
    config: NodeConfig,
    role: String,
    node_id: Uuid,

    // Core components
    network_transport: Option<Arc<NngTransport>>,
    discovery_manager: Option<Arc<RwLock<DiscoveryManager>>>,
    shard_manager: Option<Arc<ShardManager>>,
    transaction_indexer: Option<Arc<TransactionIndexer>>,
    storage_manager: Option<Arc<RwLock<StorageManager>>>,
    consensus_coordinator: Option<Arc<RwLock<ConsensusCoordinator>>>,
    query_router: Option<Arc<RwLock<QueryRouter>>>,

    // API server
    api_server_handle: Option<tokio::task::JoinHandle<()>>,

    // State
    running: Arc<RwLock<bool>>,
}

impl StreamSyncNode {
    /// Create a new StreamSync node
    pub async fn new(config: NodeConfig, role: String) -> Result<Self> {
        // Validate configuration
        config.validate()?;

        let node_id = Uuid::parse_str(&config.node.id)?;

        info!("Creating StreamSync node: {}", node_id);
        info!("Role: {}", role);

        Ok(Self {
            config,
            role,
            node_id,
            network_transport: None,
            discovery_manager: None,
            shard_manager: None,
            transaction_indexer: None,
            storage_manager: None,
            consensus_coordinator: None,
            query_router: None,
            api_server_handle: None,
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the node and all its components
    pub async fn start(&mut self) -> Result<()> {
        {
            let running = self.running.read().await;
            if *running {
                return Err(anyhow::anyhow!("Node is already running"));
            }
        }

        info!("Starting StreamSync node components...");

        // Initialize networking
        self.init_networking().await?;

        // Initialize peer discovery
        self.init_peer_discovery().await?;

        // Initialize sharding
        self.init_sharding().await?;

        // Initialize Solana indexing
        self.init_solana_indexer().await?;

        // Initialize storage
        self.init_storage().await?;

        // Initialize consensus
        self.init_consensus().await?;

        // Initialize query router
        self.init_query_router().await?;

        // Start all components
        self.start_components().await?;

        // Start API server
        self.start_api_server().await?;

        {
            let mut running = self.running.write().await;
            *running = true;
        }
        info!("StreamSync node started successfully");
        Ok(())
    }

    /// Shutdown the node gracefully
    pub async fn shutdown(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        info!("Shutting down StreamSync node...");

        // Stop components in reverse order
        if let Some(query_router) = &self.query_router {
            if let Err(e) = query_router.read().await.stop().await {
                warn!("Error stopping query router: {}", e);
            }
        }

        if let Some(consensus_coordinator) = &self.consensus_coordinator {
            if let Err(e) = consensus_coordinator.read().await.stop().await {
                warn!("Error stopping consensus coordinator: {}", e);
            }
        }

        if let Some(discovery_manager) = &self.discovery_manager {
            if let Err(e) = discovery_manager.read().await.stop().await {
                warn!("Error stopping discovery manager: {}", e);
            }
        }

        if let Some(storage_manager) = &self.storage_manager {
            if let Err(e) = storage_manager.read().await.stop().await {
                warn!("Error stopping storage manager: {}", e);
            }
        }

        if let Some(transaction_indexer) = &self.transaction_indexer {
            if let Err(e) = transaction_indexer.stop().await {
                warn!("Error stopping transaction indexer: {}", e);
            }
        }

        if let Some(shard_manager) = &self.shard_manager {
            if let Err(e) = shard_manager.stop().await {
                warn!("Error stopping shard manager: {}", e);
            }
        }

        if let Some(_network_transport) = &self.network_transport {
            info!("Stopping network transport...");
            // NngTransport doesn't have a disconnect method, so we just log
        }

        *running = false;
        info!("StreamSync node shutdown complete");
        Ok(())
    }

    /// Initialize networking component
    async fn init_networking(&mut self) -> Result<()> {
        info!("Initializing networking component...");

        let bind_addr = format!("{}:{}", self.config.networking.listen_addr, self.config.networking.p2p_port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid bind address: {}", e))?;

        let network_config = NetworkConfig::new(bind_addr)
            .with_max_connections(self.config.networking.max_peers)
            .with_connection_timeout(std::time::Duration::from_millis(self.config.networking.connection_timeout_ms));

        let transport = NngTransport::new(network_config)?;
        self.network_transport = Some(Arc::new(transport));

        info!("Networking component initialized");
        Ok(())
    }

    /// Initialize peer discovery component
    async fn init_peer_discovery(&mut self) -> Result<()> {
        info!("Initializing peer discovery component...");

        // Configure discovery methods based on node configuration
        let mut discovery_methods = Vec::new();

        // Bootstrap from configured bootstrap nodes
        if !self.config.networking.bootstrap_nodes.is_empty() {
            let bootstrap_addresses: Result<Vec<std::net::SocketAddr>, _> =
                self.config.networking.bootstrap_nodes
                    .iter()
                    .map(|addr| addr.parse())
                    .collect();

            match bootstrap_addresses {
                Ok(addresses) => {
                    info!("Added bootstrap discovery with {} nodes", addresses.len());
                    discovery_methods.push(DiscoveryMethod::Bootstrap(addresses));
                }
                Err(e) => {
                    warn!("Failed to parse bootstrap addresses: {}", e);
                }
            }
        }

        // Add gossip discovery for peer propagation
        discovery_methods.push(DiscoveryMethod::Gossip { fanout: 3 });

        // Determine network topology based on role
        let topology = match self.role.as_str() {
            "primary" => NetworkTopology::Star {
                hubs: vec![self.node_id]
            },
            "observer" => NetworkTopology::Ring {
                redundancy: 2
            },
            _ => NetworkTopology::SmallWorld {
                local_connections: 5,
                random_connections: 3
            },
        };

        // Create discovery configuration
        let discovery_config = DiscoveryConfig {
            max_peers: self.config.networking.max_peers,
            min_peers_for_formation: 3,
            auto_form_network: true,
            ..Default::default()
        };

        // Create and initialize discovery manager
        let discovery_manager = DiscoveryManager::new(
            self.node_id,
            discovery_config,
            discovery_methods,
            topology,
        ).await?;

        self.discovery_manager = Some(Arc::new(RwLock::new(discovery_manager)));

        info!("Peer discovery component initialized");
        Ok(())
    }

    /// Initialize sharding component
    async fn init_sharding(&mut self) -> Result<()> {
        info!("Initializing sharding component...");

        let shard_config = ShardConfig::builder()
            .virtual_nodes(self.config.sharding.virtual_nodes)
            .replication_factor(self.config.sharding.replication_factor)
            .hash_function(self.parse_hash_function(&self.config.sharding.hash_function)?)
            .auto_rebalance(self.config.sharding.auto_rebalance)
            .rebalance_threshold(self.config.sharding.rebalance_threshold)
            .migration_batch_size(self.config.sharding.migration_batch_size)
            .migration_timeout_ms(self.config.sharding.migration_timeout_ms)
            .build()?;

        let shard_manager = ShardManager::new(shard_config);
        self.shard_manager = Some(Arc::new(shard_manager));

        info!("Sharding component initialized");
        Ok(())
    }

    /// Initialize Solana indexer component
    async fn init_solana_indexer(&mut self) -> Result<()> {
        if !self.config.solana.enable_indexing {
            info!("Solana indexing is disabled in configuration");
            return Ok(());
        }

        info!("Initializing Solana indexer component...");

        let solana_config = SolanaConfig::new(
            self.config.solana.rpc_url.clone(),
            self.config.solana.ws_url.clone(),
        )
        .with_timeout(self.config.solana.request_timeout_ms)
        .with_batch_size(self.config.solana.transaction_batch_size)
        .with_tracked_programs(self.config.solana.tracked_programs.clone());

        let transaction_indexer = TransactionIndexer::new(solana_config)?;
        self.transaction_indexer = Some(Arc::new(transaction_indexer));

        info!("Solana indexer component initialized");
        Ok(())
    }

    /// Initialize storage component
    async fn init_storage(&mut self) -> Result<()> {
        info!("Initializing storage component...");

        let storage_config = StorageConfig::new(
            std::path::PathBuf::from(&self.config.storage.backend)
                .join("streamsync.db")
        )
        .with_memory_limit(self.config.storage.cache_size_mb)
        .with_batch_size(self.config.storage.batch_size);

        let storage_manager = StorageManager::new(storage_config).await?;
        self.storage_manager = Some(Arc::new(RwLock::new(storage_manager)));

        info!("Storage component initialized");
        Ok(())
    }

    /// Initialize consensus component
    async fn init_consensus(&mut self) -> Result<()> {
        info!("Initializing consensus component...");

        let consensus_coordinator = ConsensusCoordinator::new(self.config.clone()).await?;
        self.consensus_coordinator = Some(Arc::new(RwLock::new(consensus_coordinator)));

        info!("Consensus component initialized");
        Ok(())
    }

    /// Initialize query router component
    async fn init_query_router(&mut self) -> Result<()> {
        info!("Initializing query router component...");

        // Determine routing strategy based on node role and configuration
        let default_strategy = match self.role.as_str() {
            "primary" => RoutingStrategy::LeastLoaded,
            "secondary" => RoutingStrategy::DataLocality,
            "observer" => RoutingStrategy::LowestLatency,
            _ => RoutingStrategy::RoundRobin,
        };

        // Create query router
        let query_router = QueryRouter::new(
            default_strategy,
            self.config.performance.query_timeout_ms,
        ).await?;

        // Add ourselves as a routing target if we're not an observer
        if self.role != "observer" {
            let our_target = RouteTarget {
                node_id: self.node_id,
                capabilities: self.determine_node_capabilities(),
                current_load: 0.0,
                avg_response_time_ms: 100.0, // Initial estimate
                region: Some("local".to_string()),
                available: true,
                last_health_check: std::time::Instant::now(),
                active_queries: 0,
                success_rate: 1.0,
            };

            query_router.add_target(our_target).await?;
        }

        self.query_router = Some(Arc::new(RwLock::new(query_router)));

        info!("Query router component initialized");
        Ok(())
    }

    /// Determine capabilities of this node
    fn determine_node_capabilities(&self) -> Vec<String> {
        let mut capabilities = vec!["general".to_string()];

        // Add capabilities based on enabled components
        if self.config.solana.enable_indexing {
            capabilities.extend_from_slice(&[
                "transaction_indexing".to_string(),
                "block_data".to_string(),
                "account_data".to_string(),
                "program_data".to_string(),
            ]);
        }

        if self.config.consensus.enable_consensus {
            capabilities.push("consensus".to_string());
        }

        // Storage capabilities
        capabilities.extend_from_slice(&[
            "sql_query".to_string(),
            "analytics".to_string(),
        ]);

        capabilities
    }

    /// Start all initialized components
    async fn start_components(&mut self) -> Result<()> {
        info!("Starting all components...");

        // Start networking
        if let Some(_network_transport) = &self.network_transport {
            info!("Network transport initialized");
        }

        // Start peer discovery
        if let Some(discovery_manager) = &self.discovery_manager {
            let discovery = discovery_manager.read().await;
            discovery.start().await?;
            info!("Peer discovery started");
        }

        // Start sharding
        if let Some(shard_manager) = &self.shard_manager {
            shard_manager.start().await?;
            info!("Shard manager started");
        }

        // Start Solana indexing
        if let Some(transaction_indexer) = &self.transaction_indexer {
            transaction_indexer.start().await?;
            info!("Transaction indexer started");
        }

        // Start storage
        if let Some(storage_manager) = &self.storage_manager {
            let mut storage = storage_manager.write().await;
            storage.start().await?;
            info!("Storage manager started");
        }

        // Start consensus
        if let Some(consensus_coordinator) = &self.consensus_coordinator {
            let mut consensus = consensus_coordinator.write().await;
            consensus.start().await?;
            info!("Consensus coordinator started");
        }

        // Start query router
        if let Some(query_router) = &self.query_router {
            let query_router = query_router.read().await;
            query_router.start().await?;
            info!("Query router started");
        }

        info!("All components started successfully");
        Ok(())
    }

    /// Start API server
    async fn start_api_server(&mut self) -> Result<()> {
        info!("Starting HTTP API server...");

        let bind_address = format!("{}:{}", "0.0.0.0", self.config.node.api_port);

        // Create full app state with all components
        let app_state = AppState {
            node_info: self.get_node_info(),
            transaction_indexer: self.transaction_indexer.clone(),
            storage_manager: self.storage_manager.clone(),
            consensus_coordinator: self.consensus_coordinator.clone(),
            discovery_manager: self.discovery_manager.clone(),
            query_router: self.query_router.clone(),
            shard_manager: self.shard_manager.clone(),
        };

        let bind_addr_clone = bind_address.clone();
        let handle = tokio::spawn(async move {
            let app = create_api_router(app_state);
            let listener = tokio::net::TcpListener::bind(&bind_addr_clone).await.unwrap();
            info!("HTTP API server listening on {}", bind_addr_clone);
            axum::serve(listener, app).await.unwrap();
        });

        self.api_server_handle = Some(handle);
        info!("HTTP API server started on {}", bind_address);
        Ok(())
    }

    /// Parse hash function string to enum
    fn parse_hash_function(&self, hash_func: &str) -> Result<HashFunctionType> {
        match hash_func.to_lowercase().as_str() {
            "ahash" => Ok(HashFunctionType::AHash),
            "sha256" => Ok(HashFunctionType::Sha256),
            _ => Err(anyhow::anyhow!("Unsupported hash function: {}", hash_func)),
        }
    }

    /// Get node status information
    pub async fn get_status(&self) -> Result<NodeStatus> {
        let running = *self.running.read().await;

        let network_status = if let Some(nt) = &self.network_transport {
            Some(nt.stats().await)
        } else {
            None
        };

        let shard_status = if let Some(sm) = &self.shard_manager {
            Some(sm.get_cluster_stats().await)
        } else {
            None
        };

        let indexing_status = if let Some(ti) = &self.transaction_indexer {
            Some(ti.get_stats().await)
        } else {
            None
        };

        let storage_status = if let Some(sm) = &self.storage_manager {
            match sm.read().await.get_storage_stats().await {
                Ok(stats) => Some(stats),
                Err(_) => None,
            }
        } else {
            None
        };

        let consensus_status = if let Some(cc) = &self.consensus_coordinator {
            Some(cc.read().await.get_stats().await)
        } else {
            None
        };

        Ok(NodeStatus {
            node_id: self.node_id,
            role: self.role.clone(),
            running,
            network_status,
            shard_status,
            indexing_status,
            storage_status,
            consensus_status,
        })
    }

    /// Check if node is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get basic node info
    pub fn get_node_info(&self) -> NodeInfo {
        NodeInfo {
            node_id: self.node_id,
            role: self.role.clone(),
        }
    }

    /// Get node configuration (read-only reference)
    pub fn get_config(&self) -> &NodeConfig {
        &self.config
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> Option<TransportStats> {
        if let Some(nt) = &self.network_transport {
            Some(nt.stats().await)
        } else {
            None
        }
    }

    /// Get shard statistics
    pub async fn get_shard_stats(&self) -> Option<ClusterStats> {
        if let Some(sm) = &self.shard_manager {
            Some(sm.get_cluster_stats().await)
        } else {
            None
        }
    }
}

/// Basic node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: Uuid,
    pub role: String,
}

/// Node status information
#[derive(Debug)]
pub struct NodeStatus {
    pub node_id: Uuid,
    pub role: String,
    pub running: bool,
    pub network_status: Option<TransportStats>,
    pub shard_status: Option<ClusterStats>,
    pub indexing_status: Option<IndexingStats>,
    pub storage_status: Option<StorageStats>,
    pub consensus_status: Option<ConsensusStats>,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "StreamSync Node Status")?;
        writeln!(f, "=====================")?;
        writeln!(f, "Node ID: {}", self.node_id)?;
        writeln!(f, "Role: {}", self.role)?;
        writeln!(f, "Running: {}", self.running)?;

        if let Some(net_stats) = &self.network_status {
            writeln!(f, "\nNetwork:")?;
            writeln!(f, "  Messages Sent: {}", net_stats.messages_sent)?;
            writeln!(f, "  Messages Received: {}", net_stats.messages_received)?;
            writeln!(f, "  Bytes Sent: {}", net_stats.bytes_sent)?;
            writeln!(f, "  Bytes Received: {}", net_stats.bytes_received)?;
        }


        if let Some(shard_stats) = &self.shard_status {
            writeln!(f, "\nSharding:")?;
            writeln!(f, "  Total Nodes: {}", shard_stats.total_nodes)?;
            writeln!(f, "  Healthy Nodes: {}", shard_stats.healthy_nodes)?;
        }

        if let Some(indexing_stats) = &self.indexing_status {
            writeln!(f, "\nSolana Indexing:")?;
            writeln!(f, "  Transactions Indexed: {}", indexing_stats.transactions_indexed)?;
            writeln!(f, "  Blocks Indexed: {}", indexing_stats.blocks_indexed)?;
            writeln!(f, "  Last Indexed Slot: {}", indexing_stats.last_indexed_slot)?;
            writeln!(f, "  Indexing Rate: {:.2}/sec", indexing_stats.indexing_rate_per_second)?;
            writeln!(f, "  Errors: {}", indexing_stats.errors_encountered)?;
        }

        if let Some(storage_stats) = &self.storage_status {
            writeln!(f, "\nStorage:")?;
            writeln!(f, "  Total Records: {}", storage_stats.total_records)?;
            writeln!(f, "  Total Queries: {}", storage_stats.total_queries)?;
            writeln!(f, "  Storage Size: {:.2} MB", storage_stats.storage_size_bytes as f64 / 1024.0 / 1024.0)?;
            writeln!(f, "  Compression Ratio: {:.2}", storage_stats.compression_ratio)?;
            writeln!(f, "  Uptime: {} seconds", storage_stats.uptime_seconds)?;
        }

        if let Some(consensus_stats) = &self.consensus_status {
            writeln!(f, "\nConsensus:")?;
            writeln!(f, "  Total Proposals: {}", consensus_stats.total_proposals)?;
            writeln!(f, "  Successful Proposals: {}", consensus_stats.successful_proposals)?;
            writeln!(f, "  Failed Proposals: {}", consensus_stats.failed_proposals)?;
            writeln!(f, "  View Changes: {}", consensus_stats.view_changes)?;
            writeln!(f, "  Current View: {}", consensus_stats.current_view)?;
            writeln!(f, "  Is Leader: {}", consensus_stats.is_leader)?;
            writeln!(f, "  Average Consensus Time: {:.2}ms", consensus_stats.average_consensus_time_ms)?;
            writeln!(f, "  Participants: {}", consensus_stats.participants.len())?;
        }

        Ok(())
    }
}

/// Create a simple API router for the node
fn create_api_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/status", get(node_status))
        .route("/info", get(node_info_endpoint))
        .route("/indexing/stats", get(indexing_stats))
        .route("/indexing/latest", get(latest_indexed_data))
        .route("/storage/stats", get(storage_stats))
        .route("/storage/query", get(storage_query))
        .route("/consensus/stats", get(consensus_stats))
        .route("/consensus/status", get(consensus_status))
        .route("/discovery/peers", get(discovery_peers))
        .route("/discovery/stats", get(discovery_stats))
        .route("/discovery/topology", get(discovery_topology))
        .route("/query/router/stats", get(query_router_stats))
        .route("/query/router/targets", get(query_router_targets))
        .route("/shards/stats", get(shard_stats))
        .with_state(app_state)
}

/// Health check endpoint
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Node status endpoint
async fn node_status(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "node_id": state.node_info.node_id,
        "role": state.node_info.role,
        "status": "running",
        "components": {
            "indexer": state.transaction_indexer.is_some(),
            "storage": state.storage_manager.is_some(),
            "consensus": state.consensus_coordinator.is_some(),
            "discovery": state.discovery_manager.is_some(),
            "query_router": state.query_router.is_some(),
            "sharding": state.shard_manager.is_some()
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Node info endpoint
async fn node_info_endpoint(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "node_id": state.node_info.node_id,
        "role": state.node_info.role,
        "version": "0.1.0",
        "capabilities": {
            "networking": true,
            "sharding": state.shard_manager.is_some(),
            "api": true,
            "solana_indexing": state.transaction_indexer.is_some(),
            "consensus": state.consensus_coordinator.is_some()
        }
    }))
}

/// Indexing statistics endpoint
async fn indexing_stats(State(state): State<AppState>) -> Json<Value> {
    match &state.transaction_indexer {
        Some(indexer) => {
            let stats = indexer.get_stats().await;
            Json(json!({
                "status": "active",
                "transactions_indexed": stats.transactions_indexed,
                "blocks_indexed": stats.blocks_indexed,
                "last_indexed_slot": stats.last_indexed_slot,
                "indexing_rate_per_second": stats.indexing_rate_per_second,
                "errors_encountered": stats.errors_encountered,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Transaction indexer not enabled",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Latest indexed data endpoint
async fn latest_indexed_data(State(state): State<AppState>) -> Json<Value> {
    match &state.transaction_indexer {
        Some(indexer) => {
            let stats = indexer.get_stats().await;
            Json(json!({
                "status": "active",
                "last_indexed_slot": stats.last_indexed_slot,
                "transactions_indexed": stats.transactions_indexed,
                "blocks_indexed": stats.blocks_indexed,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Transaction indexer not enabled",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Storage statistics endpoint
async fn storage_stats(State(state): State<AppState>) -> Json<Value> {
    match &state.storage_manager {
        Some(sm) => {
            let manager = sm.read().await;
            match manager.get_storage_stats().await {
                Ok(stats) => Json(json!({
                    "status": "active",
                    "total_records": stats.total_records,
                    "total_queries": stats.total_queries,
                    "storage_size_bytes": stats.storage_size_bytes,
                    "storage_size_mb": stats.storage_size_bytes as f64 / 1024.0 / 1024.0,
                    "compression_ratio": stats.compression_ratio,
                    "uptime_seconds": stats.uptime_seconds,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })),
                Err(e) => Json(json!({
                    "status": "error",
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            }
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Storage manager not initialized",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Storage query endpoint
async fn storage_query(State(state): State<AppState>) -> Json<Value> {
    match &state.storage_manager {
        Some(_) => Json(json!({
            "status": "active",
            "message": "Use POST /storage/query with SQL body for queries",
            "example_queries": [
                "SELECT * FROM transactions WHERE success = true LIMIT 10",
                "SELECT COUNT(*) FROM blocks WHERE block_time > '2024-01-01'",
                "SELECT * FROM transactions WHERE slot = 12345"
            ],
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
        None => Json(json!({
            "status": "disabled",
            "message": "Storage manager not initialized",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Consensus statistics endpoint
async fn consensus_stats(State(state): State<AppState>) -> Json<Value> {
    match &state.consensus_coordinator {
        Some(cc) => {
            let coordinator = cc.read().await;
            let stats = coordinator.get_stats().await;
            Json(json!({
                "status": "active",
                "total_proposals": stats.total_proposals,
                "successful_proposals": stats.successful_proposals,
                "failed_proposals": stats.failed_proposals,
                "view_changes": stats.view_changes,
                "current_view": stats.current_view,
                "is_leader": stats.is_leader,
                "average_consensus_time_ms": stats.average_consensus_time_ms,
                "participants": stats.participants,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Consensus not enabled",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Consensus status endpoint
async fn consensus_status(State(state): State<AppState>) -> Json<Value> {
    match &state.consensus_coordinator {
        Some(cc) => {
            let coordinator = cc.read().await;
            let stats = coordinator.get_stats().await;
            Json(json!({
                "status": "active",
                "consensus_enabled": true,
                "mode": if stats.participants.len() > 1 { "multi-node" } else { "single-node" },
                "current_view": stats.current_view,
                "is_leader": stats.is_leader,
                "participants_count": stats.participants.len(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "consensus_enabled": false,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Discovery peers endpoint
async fn discovery_peers(State(state): State<AppState>) -> Json<Value> {
    match &state.discovery_manager {
        Some(dm) => {
            let discovery = dm.read().await;
            let peers = discovery.get_peers().await;
            let active_count = peers.iter().filter(|p| p.reputation > 0.5).count();
            Json(json!({
                "status": "active",
                "total_peers": peers.len(),
                "active_peers": active_count,
                "peers": peers.iter().map(|p| json!({
                    "peer_id": p.peer_id,
                    "addresses": p.addresses.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                    "reputation": p.reputation,
                    "discovered_at": format!("{:?}", p.discovered_at)
                })).collect::<Vec<_>>(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Discovery manager not initialized",
            "peers": [],
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Discovery statistics endpoint
async fn discovery_stats(State(state): State<AppState>) -> Json<Value> {
    match &state.discovery_manager {
        Some(dm) => {
            let discovery = dm.read().await;
            let stats = discovery.get_stats().await;
            Json(json!({
                "status": "active",
                "total_peers_discovered": stats.total_peers_discovered,
                "active_peers": stats.active_peers,
                "failed_discovery_attempts": stats.failed_discovery_attempts,
                "network_formation_time": stats.network_formation_time.map(|t| format!("{:?}", t)),
                "discovery_methods_active": stats.discovery_methods_active,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Discovery manager not initialized",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Discovery topology endpoint
async fn discovery_topology(State(state): State<AppState>) -> Json<Value> {
    match &state.discovery_manager {
        Some(dm) => {
            let discovery = dm.read().await;
            let peers = discovery.get_peers().await;
            Json(json!({
                "status": "active",
                "node_count": peers.len(),
                "active_peers": peers.iter().filter(|p| p.reputation > 0.5).count(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Discovery manager not initialized",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Query router statistics endpoint
async fn query_router_stats(State(state): State<AppState>) -> Json<Value> {
    match &state.query_router {
        Some(qr) => {
            let router = qr.read().await;
            let stats = router.get_stats().await;
            Json(json!({
                "status": "active",
                "total_queries_routed": stats.total_queries_routed,
                "successful_queries": stats.successful_queries,
                "failed_queries": stats.failed_queries,
                "average_response_time_ms": stats.average_response_time_ms,
                "active_targets": stats.active_targets,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Query router not initialized",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Query router targets endpoint
async fn query_router_targets(State(state): State<AppState>) -> Json<Value> {
    match &state.query_router {
        Some(qr) => {
            let router = qr.read().await;
            let stats = router.get_stats().await;
            Json(json!({
                "status": "active",
                "total_targets": stats.active_targets,
                "available_targets": stats.active_targets,
                "total_queries_routed": stats.total_queries_routed,
                "successful_queries": stats.successful_queries,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Query router not initialized",
            "targets": [],
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Shard statistics endpoint
async fn shard_stats(State(state): State<AppState>) -> Json<Value> {
    match &state.shard_manager {
        Some(sm) => {
            let stats = sm.get_cluster_stats().await;
            Json(json!({
                "status": "active",
                "total_nodes": stats.total_nodes,
                "healthy_nodes": stats.healthy_nodes,
                "virtual_nodes": stats.virtual_nodes,
                "replication_factor": stats.replication_factor,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }
        None => Json(json!({
            "status": "disabled",
            "message": "Shard manager not initialized",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}