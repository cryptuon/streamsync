//! Node Specialization System for StreamSync
//!
//! Nodes can specialize in different types of queries and data handling:
//! - SpeedRunner: Optimized for low-latency real-time queries
//! - ReconstructionSpec: Specialized in ZK account reconstruction
//! - CacheOptimizer: Optimized for frequently accessed data
//! - ArchiveNode: Historical data with long retention periods

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Node specialization types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeSpecialization {
    /// General purpose node (default)
    General,
    /// Optimized for low-latency queries
    SpeedRunner,
    /// Specialized in ZK account reconstruction
    ReconstructionSpec,
    /// Optimized for cache hit rates on hot data
    CacheOptimizer,
    /// Long-term historical data storage
    ArchiveNode,
}

impl Default for NodeSpecialization {
    fn default() -> Self {
        Self::General
    }
}

impl std::fmt::Display for NodeSpecialization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General => write!(f, "General"),
            Self::SpeedRunner => write!(f, "SpeedRunner"),
            Self::ReconstructionSpec => write!(f, "ReconstructionSpec"),
            Self::CacheOptimizer => write!(f, "CacheOptimizer"),
            Self::ArchiveNode => write!(f, "ArchiveNode"),
        }
    }
}

/// Query type classification for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueryCategory {
    /// Real-time data queries requiring low latency
    RealTime,
    /// Historical data queries
    Historical,
    /// Compressed account reconstruction
    CompressedAccount,
    /// Aggregation and analytics
    Aggregation,
    /// Frequently accessed data
    HotData,
    /// General purpose queries
    General,
}

/// Configuration for SpeedRunner specialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedRunnerConfig {
    /// Target latency in milliseconds
    pub target_latency_ms: u32,
    /// Maximum query queue depth
    pub max_queue_depth: usize,
    /// Enable connection pooling
    pub connection_pooling: bool,
    /// Number of worker threads
    pub worker_threads: usize,
    /// Cache strategy
    pub cache_strategy: CacheStrategy,
}

impl Default for SpeedRunnerConfig {
    fn default() -> Self {
        Self {
            target_latency_ms: 50,
            max_queue_depth: 1000,
            connection_pooling: true,
            worker_threads: 8,
            cache_strategy: CacheStrategy::LRU,
        }
    }
}

/// Configuration for ReconstructionSpec specialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructionSpecConfig {
    /// ZK capability level
    pub zk_capability: ZKCapability,
    /// Merkle tree cache size (number of trees)
    pub merkle_cache_size: usize,
    /// Enable proof generation
    pub proof_generation: bool,
    /// Maximum concurrent reconstructions
    pub max_concurrent: usize,
}

impl Default for ReconstructionSpecConfig {
    fn default() -> Self {
        Self {
            zk_capability: ZKCapability::Full,
            merkle_cache_size: 1000,
            proof_generation: true,
            max_concurrent: 16,
        }
    }
}

/// Configuration for CacheOptimizer specialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheOptimizerConfig {
    /// Hot data access threshold (requests per minute)
    pub hot_data_threshold: usize,
    /// Cache eviction policy
    pub eviction_policy: EvictionPolicy,
    /// Maximum cache size in MB
    pub max_cache_size_mb: usize,
    /// Enable predictive caching
    pub predictive_caching: bool,
}

impl Default for CacheOptimizerConfig {
    fn default() -> Self {
        Self {
            hot_data_threshold: 100,
            eviction_policy: EvictionPolicy::LRU,
            max_cache_size_mb: 1024,
            predictive_caching: true,
        }
    }
}

/// Configuration for ArchiveNode specialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveNodeConfig {
    /// Data retention period in days
    pub retention_days: u32,
    /// Compression level (0-9)
    pub compression_level: u8,
    /// Enable cold storage tiering
    pub cold_storage_enabled: bool,
    /// Archive query timeout in seconds
    pub query_timeout_secs: u64,
}

impl Default for ArchiveNodeConfig {
    fn default() -> Self {
        Self {
            retention_days: 365,
            compression_level: 6,
            cold_storage_enabled: true,
            query_timeout_secs: 30,
        }
    }
}

/// Cache strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheStrategy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Time-based expiration
    TTL,
    /// Adaptive caching
    Adaptive,
}

impl Default for CacheStrategy {
    fn default() -> Self {
        Self::LRU
    }
}

/// ZK capability levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZKCapability {
    /// No ZK support
    None,
    /// Basic Merkle proofs only
    Basic,
    /// Full ZK reconstruction with proofs
    Full,
    /// Advanced with custom circuit support
    Advanced,
}

impl Default for ZKCapability {
    fn default() -> Self {
        Self::Full
    }
}

/// Cache eviction policies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// First In First Out
    FIFO,
    /// Random eviction
    Random,
    /// Weighted LRU (considers size)
    WeightedLRU,
}

impl Default for EvictionPolicy {
    fn default() -> Self {
        Self::LRU
    }
}

/// Complete node specialization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializationConfig {
    /// Primary specialization type
    pub specialization: NodeSpecialization,
    /// SpeedRunner config (if applicable)
    pub speed_runner: Option<SpeedRunnerConfig>,
    /// ReconstructionSpec config (if applicable)
    pub reconstruction: Option<ReconstructionSpecConfig>,
    /// CacheOptimizer config (if applicable)
    pub cache_optimizer: Option<CacheOptimizerConfig>,
    /// ArchiveNode config (if applicable)
    pub archive: Option<ArchiveNodeConfig>,
}

impl SpecializationConfig {
    /// Create a SpeedRunner configuration
    pub fn speed_runner(config: SpeedRunnerConfig) -> Self {
        Self {
            specialization: NodeSpecialization::SpeedRunner,
            speed_runner: Some(config),
            reconstruction: None,
            cache_optimizer: None,
            archive: None,
        }
    }

    /// Create a ReconstructionSpec configuration
    pub fn reconstruction_spec(config: ReconstructionSpecConfig) -> Self {
        Self {
            specialization: NodeSpecialization::ReconstructionSpec,
            speed_runner: None,
            reconstruction: Some(config),
            cache_optimizer: None,
            archive: None,
        }
    }

    /// Create a CacheOptimizer configuration
    pub fn cache_optimizer(config: CacheOptimizerConfig) -> Self {
        Self {
            specialization: NodeSpecialization::CacheOptimizer,
            speed_runner: None,
            reconstruction: None,
            cache_optimizer: Some(config),
            archive: None,
        }
    }

    /// Create an ArchiveNode configuration
    pub fn archive_node(config: ArchiveNodeConfig) -> Self {
        Self {
            specialization: NodeSpecialization::ArchiveNode,
            speed_runner: None,
            reconstruction: None,
            cache_optimizer: None,
            archive: Some(config),
        }
    }

    /// Create a General configuration
    pub fn general() -> Self {
        Self {
            specialization: NodeSpecialization::General,
            speed_runner: None,
            reconstruction: None,
            cache_optimizer: None,
            archive: None,
        }
    }
}

impl Default for SpecializationConfig {
    fn default() -> Self {
        Self::general()
    }
}

/// Node capabilities based on specialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Supported query categories
    pub supported_categories: Vec<QueryCategory>,
    /// Maximum concurrent queries
    pub max_concurrent_queries: usize,
    /// Target latency for queries
    pub target_latency_ms: u32,
    /// Data retention period
    pub data_retention_days: u32,
    /// ZK capability level
    pub zk_capability: ZKCapability,
    /// Custom capabilities
    pub custom: HashMap<String, String>,
}

impl NodeCapabilities {
    /// Create capabilities from specialization config
    pub fn from_config(config: &SpecializationConfig) -> Self {
        match config.specialization {
            NodeSpecialization::General => Self {
                supported_categories: vec![QueryCategory::General, QueryCategory::RealTime],
                max_concurrent_queries: 100,
                target_latency_ms: 200,
                data_retention_days: 30,
                zk_capability: ZKCapability::Basic,
                custom: HashMap::new(),
            },
            NodeSpecialization::SpeedRunner => {
                let default_sr = SpeedRunnerConfig::default();
                let sr = config.speed_runner.as_ref().unwrap_or(&default_sr);
                Self {
                    supported_categories: vec![QueryCategory::RealTime, QueryCategory::HotData],
                    max_concurrent_queries: sr.max_queue_depth,
                    target_latency_ms: sr.target_latency_ms,
                    data_retention_days: 7,
                    zk_capability: ZKCapability::None,
                    custom: HashMap::new(),
                }
            }
            NodeSpecialization::ReconstructionSpec => {
                let default_rc = ReconstructionSpecConfig::default();
                let rc = config.reconstruction.as_ref().unwrap_or(&default_rc);
                Self {
                    supported_categories: vec![QueryCategory::CompressedAccount],
                    max_concurrent_queries: rc.max_concurrent,
                    target_latency_ms: 500,
                    data_retention_days: 90,
                    zk_capability: rc.zk_capability,
                    custom: HashMap::new(),
                }
            }
            NodeSpecialization::CacheOptimizer => {
                let default_co = CacheOptimizerConfig::default();
                let co = config.cache_optimizer.as_ref().unwrap_or(&default_co);
                Self {
                    supported_categories: vec![QueryCategory::HotData, QueryCategory::Aggregation],
                    max_concurrent_queries: 500,
                    target_latency_ms: 20,
                    data_retention_days: 14,
                    zk_capability: ZKCapability::None,
                    custom: {
                        let mut m = HashMap::new();
                        m.insert("cache_size_mb".to_string(), co.max_cache_size_mb.to_string());
                        m
                    },
                }
            }
            NodeSpecialization::ArchiveNode => {
                let default_an = ArchiveNodeConfig::default();
                let an = config.archive.as_ref().unwrap_or(&default_an);
                Self {
                    supported_categories: vec![QueryCategory::Historical],
                    max_concurrent_queries: 50,
                    target_latency_ms: 5000,
                    data_retention_days: an.retention_days,
                    zk_capability: ZKCapability::Basic,
                    custom: HashMap::new(),
                }
            }
        }
    }

    /// Check if this node supports a query category
    pub fn supports_category(&self, category: QueryCategory) -> bool {
        self.supported_categories.contains(&category)
    }
}

/// Specialization registry for managing node types
#[derive(Debug, Default)]
pub struct SpecializationRegistry {
    /// Nodes by specialization type
    nodes: HashMap<NodeSpecialization, Vec<Uuid>>,
    /// Node configurations
    configs: HashMap<Uuid, SpecializationConfig>,
}

impl SpecializationRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a node with its specialization
    pub fn register(&mut self, node_id: Uuid, config: SpecializationConfig) {
        self.nodes
            .entry(config.specialization)
            .or_default()
            .push(node_id);
        self.configs.insert(node_id, config);
    }

    /// Unregister a node
    pub fn unregister(&mut self, node_id: &Uuid) {
        if let Some(config) = self.configs.remove(node_id) {
            if let Some(nodes) = self.nodes.get_mut(&config.specialization) {
                nodes.retain(|id| id != node_id);
            }
        }
    }

    /// Get nodes by specialization
    pub fn get_nodes(&self, specialization: NodeSpecialization) -> &[Uuid] {
        self.nodes.get(&specialization).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get best nodes for a query category
    pub fn get_nodes_for_category(&self, category: QueryCategory) -> Vec<Uuid> {
        let preferred_specs = match category {
            QueryCategory::RealTime => vec![NodeSpecialization::SpeedRunner, NodeSpecialization::General],
            QueryCategory::Historical => vec![NodeSpecialization::ArchiveNode, NodeSpecialization::General],
            QueryCategory::CompressedAccount => vec![NodeSpecialization::ReconstructionSpec],
            QueryCategory::Aggregation => vec![NodeSpecialization::CacheOptimizer, NodeSpecialization::General],
            QueryCategory::HotData => vec![NodeSpecialization::CacheOptimizer, NodeSpecialization::SpeedRunner],
            QueryCategory::General => vec![NodeSpecialization::General],
        };

        let mut result = Vec::new();
        for spec in preferred_specs {
            if let Some(nodes) = self.nodes.get(&spec) {
                result.extend(nodes.iter().cloned());
            }
        }
        result
    }

    /// Get node configuration
    pub fn get_config(&self, node_id: &Uuid) -> Option<&SpecializationConfig> {
        self.configs.get(node_id)
    }

    /// Get total node count
    pub fn node_count(&self) -> usize {
        self.configs.len()
    }

    /// Get node count by specialization
    pub fn count_by_specialization(&self) -> HashMap<NodeSpecialization, usize> {
        self.nodes.iter().map(|(k, v)| (*k, v.len())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialization_config() {
        let config = SpecializationConfig::speed_runner(SpeedRunnerConfig::default());
        assert_eq!(config.specialization, NodeSpecialization::SpeedRunner);
        assert!(config.speed_runner.is_some());
    }

    #[test]
    fn test_node_capabilities() {
        let config = SpecializationConfig::speed_runner(SpeedRunnerConfig {
            target_latency_ms: 25,
            ..Default::default()
        });

        let caps = NodeCapabilities::from_config(&config);
        assert!(caps.supports_category(QueryCategory::RealTime));
        assert_eq!(caps.target_latency_ms, 25);
    }

    #[test]
    fn test_registry() {
        let mut registry = SpecializationRegistry::new();

        let node1 = Uuid::new_v4();
        let node2 = Uuid::new_v4();
        let node3 = Uuid::new_v4();

        registry.register(node1, SpecializationConfig::speed_runner(SpeedRunnerConfig::default()));
        registry.register(node2, SpecializationConfig::speed_runner(SpeedRunnerConfig::default()));
        registry.register(node3, SpecializationConfig::archive_node(ArchiveNodeConfig::default()));

        assert_eq!(registry.get_nodes(NodeSpecialization::SpeedRunner).len(), 2);
        assert_eq!(registry.get_nodes(NodeSpecialization::ArchiveNode).len(), 1);

        let real_time_nodes = registry.get_nodes_for_category(QueryCategory::RealTime);
        assert_eq!(real_time_nodes.len(), 2);
    }
}
