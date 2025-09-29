//! Node management and information

use crate::{Result, ShardError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Unique identifier for a node in the cluster
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    /// Create a new node ID with a custom identifier
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a random node ID
    pub fn random() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Get the underlying string identifier
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the underlying string identifier
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for NodeId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for NodeId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Current status of a node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is healthy and operational
    Healthy,
    /// Node is experiencing issues but still reachable
    Degraded,
    /// Node is unreachable or failed
    Failed,
    /// Node is temporarily unavailable (maintenance, etc.)
    Unavailable,
    /// Node is in the process of joining the cluster
    Joining,
    /// Node is in the process of leaving the cluster
    Leaving,
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self::Healthy
    }
}

impl NodeStatus {
    /// Check if the node is available for serving requests
    pub fn is_available(&self) -> bool {
        matches!(self, NodeStatus::Healthy | NodeStatus::Degraded)
    }

    /// Check if the node can participate in consensus operations
    pub fn can_participate(&self) -> bool {
        matches!(self, NodeStatus::Healthy)
    }
}

/// Comprehensive information about a node in the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub id: NodeId,
    /// Network address for communication
    pub address: SocketAddr,
    /// Current node status
    pub status: NodeStatus,
    /// When the node was first seen
    pub first_seen: SystemTime,
    /// Last successful health check
    pub last_heartbeat: SystemTime,
    /// Node capabilities and metadata
    pub metadata: NodeMetadata,
    /// Node performance metrics
    pub metrics: NodeMetrics,
    /// Virtual nodes assigned to this physical node
    pub virtual_nodes: Vec<VirtualNode>,
}

impl NodeInfo {
    /// Create new node information
    pub fn new(id: NodeId, address: SocketAddr) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            address,
            status: NodeStatus::Joining,
            first_seen: now,
            last_heartbeat: now,
            metadata: NodeMetadata::default(),
            metrics: NodeMetrics::default(),
            virtual_nodes: Vec::new(),
        }
    }

    /// Update the node's heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now();

        // Auto-update status based on heartbeat
        if self.status == NodeStatus::Joining {
            self.status = NodeStatus::Healthy;
        }
    }

    /// Check if the node has failed based on timeout
    pub fn is_failed(&self, timeout: Duration) -> bool {
        if let Ok(elapsed) = self.last_heartbeat.elapsed() {
            elapsed > timeout
        } else {
            true // Consider failed if we can't determine elapsed time
        }
    }

    /// Get the node's uptime
    pub fn uptime(&self) -> Result<Duration> {
        self.first_seen
            .elapsed()
            .map_err(|e| ShardError::InternalError {
                reason: format!("Failed to calculate uptime: {}", e),
            })
    }

    /// Get time since last heartbeat
    pub fn time_since_heartbeat(&self) -> Result<Duration> {
        self.last_heartbeat
            .elapsed()
            .map_err(|e| ShardError::InternalError {
                reason: format!("Failed to calculate time since heartbeat: {}", e),
            })
    }

    /// Add a virtual node to this physical node
    pub fn add_virtual_node(&mut self, virtual_node: VirtualNode) {
        self.virtual_nodes.push(virtual_node);
    }

    /// Remove a virtual node from this physical node
    pub fn remove_virtual_node(&mut self, hash: u64) -> bool {
        if let Some(pos) = self.virtual_nodes.iter().position(|vn| vn.hash == hash) {
            self.virtual_nodes.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all virtual node hashes for this physical node
    pub fn virtual_node_hashes(&self) -> Vec<u64> {
        self.virtual_nodes.iter().map(|vn| vn.hash).collect()
    }
}

/// Node metadata and capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// Node version or build identifier
    pub version: Option<String>,
    /// Geographic region or data center
    pub region: Option<String>,
    /// Available storage capacity in bytes
    pub storage_capacity: Option<u64>,
    /// Available memory in bytes
    pub memory_capacity: Option<usize>,
    /// CPU cores available
    pub cpu_cores: Option<usize>,
    /// Network bandwidth capacity
    pub bandwidth_capacity: Option<u64>,
    /// Custom metadata key-value pairs
    pub custom: HashMap<String, String>,
}

impl NodeMetadata {
    /// Set node version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set node region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set storage capacity
    pub fn with_storage_capacity(mut self, capacity: u64) -> Self {
        self.storage_capacity = Some(capacity);
        self
    }

    /// Set memory capacity
    pub fn with_memory_capacity(mut self, capacity: usize) -> Self {
        self.memory_capacity = Some(capacity);
        self
    }

    /// Set CPU cores
    pub fn with_cpu_cores(mut self, cores: usize) -> Self {
        self.cpu_cores = Some(cores);
        self
    }

    /// Add custom metadata
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }
}

/// Performance metrics for a node
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeMetrics {
    /// Average CPU utilization (0.0 to 1.0)
    pub cpu_utilization: f64,
    /// Memory utilization (0.0 to 1.0)
    pub memory_utilization: f64,
    /// Storage utilization (0.0 to 1.0)
    pub storage_utilization: f64,
    /// Network I/O rate in bytes per second
    pub network_io_rate: u64,
    /// Number of active connections
    pub active_connections: usize,
    /// Request processing latency in milliseconds
    pub avg_latency_ms: f64,
    /// Request throughput (requests per second)
    pub throughput_rps: f64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Number of keys stored on this node
    pub keys_stored: u64,
    /// Total data size in bytes
    pub data_size_bytes: u64,
}

impl NodeMetrics {
    /// Check if the node is under heavy load
    pub fn is_overloaded(&self) -> bool {
        self.cpu_utilization > 0.9
            || self.memory_utilization > 0.9
            || self.storage_utilization > 0.9
            || self.error_rate > 0.1
    }

    /// Get overall health score (0.0 to 1.0)
    pub fn health_score(&self) -> f64 {
        let cpu_score = 1.0 - self.cpu_utilization;
        let memory_score = 1.0 - self.memory_utilization;
        let storage_score = 1.0 - self.storage_utilization;
        let error_score = 1.0 - self.error_rate;

        (cpu_score + memory_score + storage_score + error_score) / 4.0
    }

    /// Update metrics with new values
    pub fn update(
        &mut self,
        cpu: f64,
        memory: f64,
        storage: f64,
        latency: f64,
        throughput: f64,
        error_rate: f64,
    ) {
        self.cpu_utilization = cpu.clamp(0.0, 1.0);
        self.memory_utilization = memory.clamp(0.0, 1.0);
        self.storage_utilization = storage.clamp(0.0, 1.0);
        self.avg_latency_ms = latency.max(0.0);
        self.throughput_rps = throughput.max(0.0);
        self.error_rate = error_rate.clamp(0.0, 1.0);
    }
}

/// Virtual node representation for consistent hashing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualNode {
    /// Hash value on the ring
    pub hash: u64,
    /// Physical node this virtual node belongs to
    pub node_id: NodeId,
    /// Virtual node index within the physical node
    pub index: usize,
}

impl VirtualNode {
    /// Create a new virtual node
    pub fn new(hash: u64, node_id: NodeId, index: usize) -> Self {
        Self {
            hash,
            node_id,
            index,
        }
    }
}

impl PartialEq for VirtualNode {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for VirtualNode {}

impl PartialOrd for VirtualNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VirtualNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_node_id_creation() {
        let id1 = NodeId::new("test-node");
        let id2 = NodeId::from("test-node");
        let id3 = NodeId::random();

        assert_eq!(id1, id2);
        assert_eq!(id1.as_str(), "test-node");
        assert_ne!(id1, id3);

        // Test display
        assert_eq!(format!("{}", id1), "test-node");
    }

    #[test]
    fn test_node_status() {
        assert!(NodeStatus::Healthy.is_available());
        assert!(NodeStatus::Degraded.is_available());
        assert!(!NodeStatus::Failed.is_available());
        assert!(!NodeStatus::Unavailable.is_available());

        assert!(NodeStatus::Healthy.can_participate());
        assert!(!NodeStatus::Degraded.can_participate());
        assert!(!NodeStatus::Failed.can_participate());
    }

    #[test]
    fn test_node_info_creation() {
        let id = NodeId::new("test-node");
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let node = NodeInfo::new(id.clone(), addr);

        assert_eq!(node.id, id);
        assert_eq!(node.address, addr);
        assert_eq!(node.status, NodeStatus::Joining);
        assert!(node.virtual_nodes.is_empty());
    }

    #[test]
    fn test_node_heartbeat() {
        let id = NodeId::new("test-node");
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut node = NodeInfo::new(id, addr);

        assert_eq!(node.status, NodeStatus::Joining);

        thread::sleep(Duration::from_millis(10));
        node.update_heartbeat();

        assert_eq!(node.status, NodeStatus::Healthy);
        assert!(node.time_since_heartbeat().unwrap() < Duration::from_millis(50));
    }

    #[test]
    fn test_node_failure_detection() {
        let id = NodeId::new("test-node");
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let node = NodeInfo::new(id, addr);

        // Node should not be considered failed immediately
        assert!(!node.is_failed(Duration::from_secs(1)));

        // Simulate old heartbeat
        let mut old_node = node;
        old_node.last_heartbeat = SystemTime::now() - Duration::from_secs(2);
        assert!(old_node.is_failed(Duration::from_secs(1)));
    }

    #[test]
    fn test_virtual_node_management() {
        let id = NodeId::new("test-node");
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut node = NodeInfo::new(id.clone(), addr);

        let vnode1 = VirtualNode::new(100, id.clone(), 0);
        let vnode2 = VirtualNode::new(200, id.clone(), 1);

        // Add virtual nodes
        node.add_virtual_node(vnode1);
        node.add_virtual_node(vnode2);

        assert_eq!(node.virtual_nodes.len(), 2);
        assert_eq!(node.virtual_node_hashes(), vec![100, 200]);

        // Remove virtual node
        assert!(node.remove_virtual_node(100));
        assert_eq!(node.virtual_nodes.len(), 1);
        assert!(!node.remove_virtual_node(300)); // Non-existent
    }

    #[test]
    fn test_node_metadata() {
        let metadata = NodeMetadata::default()
            .with_version("1.0.0")
            .with_region("us-west-2")
            .with_storage_capacity(1_000_000)
            .with_custom("environment", "production");

        assert_eq!(metadata.version, Some("1.0.0".to_string()));
        assert_eq!(metadata.region, Some("us-west-2".to_string()));
        assert_eq!(metadata.storage_capacity, Some(1_000_000));
        assert_eq!(metadata.custom.get("environment"), Some(&"production".to_string()));
    }

    #[test]
    fn test_node_metrics() {
        let mut metrics = NodeMetrics::default();

        // Test initial state
        assert!(!metrics.is_overloaded());
        assert_eq!(metrics.health_score(), 1.0);

        // Update with high load
        metrics.update(0.95, 0.85, 0.75, 100.0, 1000.0, 0.05);
        assert!(metrics.is_overloaded());
        assert!(metrics.health_score() < 0.5);

        // Test clamping
        metrics.update(-0.1, 1.5, 0.5, -10.0, -100.0, 1.5);
        assert_eq!(metrics.cpu_utilization, 0.0);
        assert_eq!(metrics.memory_utilization, 1.0);
        assert_eq!(metrics.avg_latency_ms, 0.0);
        assert_eq!(metrics.throughput_rps, 0.0);
        assert_eq!(metrics.error_rate, 1.0);
    }

    #[test]
    fn test_virtual_node_ordering() {
        let id = NodeId::new("test");
        let vnode1 = VirtualNode::new(100, id.clone(), 0);
        let vnode2 = VirtualNode::new(200, id.clone(), 1);
        let vnode3 = VirtualNode::new(100, id, 2);

        assert!(vnode1 < vnode2);
        assert_eq!(vnode1, vnode3); // Same hash
    }

    #[test]
    fn test_node_serialization() {
        let id = NodeId::new("test-node");
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let node = NodeInfo::new(id, addr);

        // Test JSON serialization
        let json = serde_json::to_string(&node).unwrap();
        let deserialized: NodeInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(node.id, deserialized.id);
        assert_eq!(node.address, deserialized.address);
        assert_eq!(node.status, deserialized.status);
    }
}