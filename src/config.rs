use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node: NodeSettings,
    pub networking: NetworkingSettings,
    pub consensus: ConsensusSettings,
    pub sharding: ShardingSettings,
    pub storage: StorageSettings,
    pub performance: PerformanceSettings,
    pub solana: SolanaSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSettings {
    /// Unique node identifier
    pub id: String,
    /// Node name for identification
    pub name: String,
    /// Node role (primary, secondary, observer)
    pub role: String,
    /// HTTP API port
    pub api_port: u16,
    /// Metrics collection port
    pub metrics_port: u16,
    /// Data directory
    pub data_dir: String,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingSettings {
    /// Node's listening address
    pub listen_addr: String,
    /// Port for node-to-node communication
    pub p2p_port: u16,
    /// Bootstrap nodes for initial connection
    pub bootstrap_nodes: Vec<String>,
    /// Maximum number of peer connections
    pub max_peers: usize,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Enable TLS for peer connections
    pub enable_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusSettings {
    /// PBFT view timeout in milliseconds
    pub view_timeout_ms: u64,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Maximum number of retries for failed requests
    pub max_retries: u32,
    /// Checkpoint interval (number of requests)
    pub checkpoint_interval: u64,
    /// Maximum pending requests
    pub max_pending_requests: usize,
    /// Enable fast recovery mode
    pub enable_fast_recovery: bool,
    /// Bootstrap consensus nodes for joining the network
    pub bootstrap_nodes: Vec<String>,
    /// Enable consensus for data indexing decisions
    pub enable_consensus: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingSettings {
    /// Number of virtual nodes per physical node
    pub virtual_nodes: usize,
    /// Replication factor for data
    pub replication_factor: usize,
    /// Hash function to use (ahash, sha256)
    pub hash_function: String,
    /// Enable automatic rebalancing
    pub auto_rebalance: bool,
    /// Rebalance threshold (0.0-1.0)
    pub rebalance_threshold: f64,
    /// Migration batch size
    pub migration_batch_size: usize,
    /// Migration timeout in milliseconds
    pub migration_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    /// Local storage backend (memory, disk, hybrid)
    pub backend: String,
    /// Maximum cache size in MB
    pub cache_size_mb: usize,
    /// Data retention period in days
    pub retention_days: u32,
    /// Compression algorithm (none, gzip, lz4)
    pub compression: String,
    /// Batch size for database writes
    pub batch_size: usize,
    /// Sync interval in seconds
    pub sync_interval_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    /// Number of worker threads
    pub worker_threads: usize,
    /// I/O thread pool size
    pub io_threads: usize,
    /// Buffer size for network messages
    pub network_buffer_size: usize,
    /// Query processing timeout in milliseconds
    pub query_timeout_ms: u64,
    /// Maximum concurrent queries
    pub max_concurrent_queries: usize,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Metrics collection interval in seconds
    pub metrics_interval_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaSettings {
    /// Solana RPC endpoint URL
    pub rpc_url: String,
    /// Solana WebSocket endpoint URL
    pub ws_url: String,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Maximum concurrent RPC requests
    pub max_concurrent_requests: usize,
    /// Number of recent slots to cache
    pub slot_cache_size: usize,
    /// Batch size for transaction fetching
    pub transaction_batch_size: usize,
    /// Block polling interval in milliseconds
    pub polling_interval_ms: u64,
    /// Programs to track (empty = all)
    pub tracked_programs: Vec<String>,
    /// Include failed transactions in indexing
    pub include_failed_transactions: bool,
    /// Enable real-time transaction indexing
    pub enable_indexing: bool,
}

impl NodeConfig {
    /// Load configuration from a TOML file
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let config: NodeConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content).await?;
        Ok(())
    }

    /// Create a default configuration file
    pub async fn create_default<P: AsRef<Path>>(path: P) -> Result<()> {
        let config = Self::default();
        config.save(path).await
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate ports
        if self.node.api_port == 0 || self.node.metrics_port == 0 || self.networking.p2p_port == 0 {
            return Err(anyhow::anyhow!("Invalid port configuration"));
        }

        // Validate replication factor
        if self.sharding.replication_factor == 0 || self.sharding.replication_factor > 10 {
            return Err(anyhow::anyhow!("Replication factor must be between 1 and 10"));
        }

        // Validate performance settings
        if self.performance.worker_threads == 0 || self.performance.io_threads == 0 {
            return Err(anyhow::anyhow!("Thread counts must be greater than 0"));
        }

        Ok(())
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node: NodeSettings {
                id: Uuid::new_v4().to_string(),
                name: "streamsync-node".to_string(),
                role: "secondary".to_string(),
                api_port: 8080,
                metrics_port: 9090,
                data_dir: "./data".to_string(),
                log_level: "info".to_string(),
            },
            networking: NetworkingSettings {
                listen_addr: "0.0.0.0".to_string(),
                p2p_port: 7777,
                bootstrap_nodes: vec![],
                max_peers: 50,
                connection_timeout_ms: 10_000,
                heartbeat_interval_ms: 5_000,
                enable_tls: false,
            },
            consensus: ConsensusSettings {
                view_timeout_ms: 30_000,
                request_timeout_ms: 5_000,
                max_retries: 3,
                checkpoint_interval: 100,
                max_pending_requests: 1000,
                enable_fast_recovery: true,
                bootstrap_nodes: vec![],
                enable_consensus: false, // Disabled by default
            },
            sharding: ShardingSettings {
                virtual_nodes: 150,
                replication_factor: 3,
                hash_function: "ahash".to_string(),
                auto_rebalance: true,
                rebalance_threshold: 0.2,
                migration_batch_size: 100,
                migration_timeout_ms: 30_000,
            },
            storage: StorageSettings {
                backend: "hybrid".to_string(),
                cache_size_mb: 512,
                retention_days: 30,
                compression: "gzip".to_string(),
                batch_size: 1000,
                sync_interval_s: 60,
            },
            performance: PerformanceSettings {
                worker_threads: num_cpus::get(),
                io_threads: 4,
                network_buffer_size: 65536,
                query_timeout_ms: 30_000,
                max_concurrent_queries: 100,
                enable_metrics: true,
                metrics_interval_s: 10,
            },
            solana: SolanaSettings {
                rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
                ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
                request_timeout_ms: 30_000,
                max_concurrent_requests: 10,
                slot_cache_size: 1000,
                transaction_batch_size: 100,
                polling_interval_ms: 1000,
                tracked_programs: vec![],
                include_failed_transactions: false,
                enable_indexing: true,
            },
        }
    }
}