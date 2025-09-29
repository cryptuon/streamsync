//! Shard manager for coordinating sharding operations

use crate::{
    hash_ring::ConsistentHashRing, migration::ShardMigrator, metrics::ShardMetrics,
    node::NodeInfo, replication::ReplicationManager, NodeId, NodeStatus, Result, ShardConfig,
    ShardError,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{interval, Instant};
use tracing::{debug, error, info, warn};

/// Main coordinator for shard operations
pub struct ShardManager {
    /// Configuration for sharding behavior
    config: ShardConfig,
    /// Consistent hash ring for key distribution
    hash_ring: Arc<RwLock<ConsistentHashRing>>,
    /// Information about all nodes in the cluster
    nodes: Arc<RwLock<HashMap<NodeId, NodeInfo>>>,
    /// Replication manager
    replication_manager: Arc<ReplicationManager>,
    /// Migration manager
    migrator: Arc<Mutex<ShardMigrator>>,
    /// Metrics collector
    metrics: Arc<ShardMetrics>,
    /// Whether the manager is running
    running: Arc<RwLock<bool>>,
    /// Last rebalance time
    last_rebalance: Arc<RwLock<Instant>>,
}

impl ShardManager {
    /// Create a new shard manager
    pub fn new(config: ShardConfig) -> Self {
        config.validate().expect("Invalid shard configuration");

        let hash_ring = Arc::new(RwLock::new(ConsistentHashRing::new(config.hash_function)));
        let nodes = Arc::new(RwLock::new(HashMap::new()));
        let replication_manager = Arc::new(ReplicationManager::new(config.clone()));
        let migrator = Arc::new(Mutex::new(ShardMigrator::new(config.clone())));
        let metrics = Arc::new(ShardMetrics::new(config.metrics_interval()));

        Self {
            config,
            hash_ring,
            nodes,
            replication_manager,
            migrator,
            metrics,
            running: Arc::new(RwLock::new(false)),
            last_rebalance: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Start the shard manager
    pub async fn start(self: &Arc<Self>) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(ShardError::InvalidState {
                reason: "Shard manager is already running".to_string(),
            });
        }

        *running = true;
        drop(running);

        // Start background tasks
        self.start_background_tasks().await;

        info!("Shard manager started");
        Ok(())
    }

    /// Stop the shard manager
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(ShardError::InvalidState {
                reason: "Shard manager is not running".to_string(),
            });
        }

        *running = false;

        info!("Shard manager stopped");
        Ok(())
    }

    /// Add a new node to the cluster
    pub async fn add_node(&self, node_id: NodeId, address: SocketAddr) -> Result<()> {
        let mut nodes = self.nodes.write().await;

        if nodes.contains_key(&node_id) {
            return Err(ShardError::NodeAlreadyExists {
                node_id: node_id.to_string(),
            });
        }

        // Create node info
        let node_info = NodeInfo::new(node_id.clone(), address);
        nodes.insert(node_id.clone(), node_info);
        drop(nodes);

        // Add to hash ring
        let mut hash_ring = self.hash_ring.write().await;
        let virtual_hashes = hash_ring.add_node(node_id.clone(), self.config.virtual_nodes)?;
        drop(hash_ring);

        // Update node info with virtual nodes
        let mut nodes = self.nodes.write().await;
        if let Some(node_info) = nodes.get_mut(&node_id) {
            for (i, hash) in virtual_hashes.into_iter().enumerate() {
                let virtual_node = crate::node::VirtualNode::new(hash, node_id.clone(), i);
                node_info.add_virtual_node(virtual_node);
            }
        }
        drop(nodes);

        // Trigger rebalancing if auto-rebalance is enabled
        if self.config.auto_rebalance {
            self.trigger_rebalance().await?;
        }

        info!("Added node {} at {}", node_id, address);
        self.metrics.record_node_added().await;

        Ok(())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &NodeId) -> Result<()> {
        // Check if node exists
        let mut nodes = self.nodes.write().await;
        if !nodes.contains_key(node_id) {
            return Err(ShardError::NodeNotFound {
                node_id: node_id.to_string(),
            });
        }

        // Mark node as leaving
        if let Some(node_info) = nodes.get_mut(node_id) {
            node_info.status = NodeStatus::Leaving;
        }
        drop(nodes);

        // Get affected ranges before removing from ring
        let hash_ring = self.hash_ring.read().await;
        let affected_ranges = hash_ring.get_node_ranges(node_id);
        drop(hash_ring);

        // Trigger migration for affected data
        if !affected_ranges.is_empty() {
            let migrator = self.migrator.lock().await;
            for (start, end) in affected_ranges {
                migrator.plan_range_migration(start, end, node_id.clone()).await?;
            }
            drop(migrator);
        }

        // Remove from hash ring
        let mut hash_ring = self.hash_ring.write().await;
        hash_ring.remove_node(node_id)?;
        drop(hash_ring);

        // Remove from nodes after successful migration
        let mut nodes = self.nodes.write().await;
        nodes.remove(node_id);
        drop(nodes);

        // Trigger rebalancing if auto-rebalance is enabled
        if self.config.auto_rebalance {
            self.trigger_rebalance().await?;
        }

        info!("Removed node {}", node_id);
        self.metrics.record_node_removed().await;

        Ok(())
    }

    /// Get the responsible nodes for a key
    pub async fn get_responsible_nodes(&self, key: &str) -> Vec<NodeId> {
        let hash_ring = self.hash_ring.read().await;

        match hash_ring.get_nodes(key, self.config.replication_factor) {
            Ok(nodes) => {
                // Filter out unhealthy nodes
                let nodes_guard = self.nodes.read().await;
                nodes
                    .into_iter()
                    .filter(|node_id| {
                        if let Some(node_info) = nodes_guard.get(node_id) {
                            node_info.status.is_available()
                        } else {
                            false
                        }
                    })
                    .collect()
            }
            Err(e) => {
                warn!("Failed to get responsible nodes for key {}: {}", key, e);
                Vec::new()
            }
        }
    }

    /// Get the primary responsible node for a key
    pub async fn get_primary_node(&self, key: &str) -> Result<NodeId> {
        let hash_ring = self.hash_ring.read().await;
        let primary = hash_ring.get_node(key)?;

        // Check if the primary node is healthy
        let nodes = self.nodes.read().await;
        if let Some(node_info) = nodes.get(&primary) {
            if node_info.status.is_available() {
                Ok(primary)
            } else {
                // Primary is unhealthy, try to get a backup
                let status = format!("{:?}", node_info.status);
                drop(nodes);
                drop(hash_ring);
                let responsible_nodes = self.get_responsible_nodes(key).await;
                responsible_nodes.into_iter().next().ok_or(ShardError::NodeUnhealthy {
                    node_id: primary.to_string(),
                    status,
                })
            }
        } else {
            Err(ShardError::NodeNotFound {
                node_id: primary.to_string(),
            })
        }
    }

    /// Get information about all nodes
    pub async fn get_nodes(&self) -> HashMap<NodeId, NodeInfo> {
        self.nodes.read().await.clone()
    }

    /// Get information about a specific node
    pub async fn get_node(&self, node_id: &NodeId) -> Option<NodeInfo> {
        self.nodes.read().await.get(node_id).cloned()
    }

    /// Update node status
    pub async fn update_node_status(&self, node_id: &NodeId, status: NodeStatus) -> Result<()> {
        let mut nodes = self.nodes.write().await;

        if let Some(node_info) = nodes.get_mut(node_id) {
            let old_status = node_info.status;
            node_info.status = status;

            info!("Node {} status changed from {:?} to {:?}", node_id, old_status, status);

            // If node became unhealthy, consider triggering rebalance
            if old_status.is_available() && !status.is_available() && self.config.auto_rebalance {
                drop(nodes);
                self.trigger_rebalance().await?;
            }

            Ok(())
        } else {
            Err(ShardError::NodeNotFound {
                node_id: node_id.to_string(),
            })
        }
    }

    /// Process heartbeat from a node
    pub async fn process_heartbeat(&self, node_id: &NodeId) -> Result<()> {
        let mut nodes = self.nodes.write().await;

        if let Some(node_info) = nodes.get_mut(node_id) {
            node_info.update_heartbeat();
            Ok(())
        } else {
            Err(ShardError::NodeNotFound {
                node_id: node_id.to_string(),
            })
        }
    }

    /// Trigger rebalancing of the cluster
    pub async fn trigger_rebalance(&self) -> Result<()> {
        let hash_ring = self.hash_ring.read().await;

        // Check if rebalancing is needed
        if !hash_ring.is_balanced(self.config.rebalance_threshold) {
            drop(hash_ring);

            info!("Triggering cluster rebalance");

            // TODO: Implement actual rebalancing logic
            // This would involve:
            // 1. Calculating optimal virtual node distribution
            // 2. Planning migrations to achieve balance
            // 3. Executing migrations gradually

            let mut last_rebalance = self.last_rebalance.write().await;
            *last_rebalance = Instant::now();

            self.metrics.record_rebalance_triggered().await;
        }

        Ok(())
    }

    /// Get cluster statistics
    pub async fn get_cluster_stats(&self) -> ClusterStats {
        let nodes = self.nodes.read().await;
        let hash_ring = self.hash_ring.read().await;

        let total_nodes = nodes.len();
        let healthy_nodes = nodes.values().filter(|n| n.status == NodeStatus::Healthy).count();
        let degraded_nodes = nodes.values().filter(|n| n.status == NodeStatus::Degraded).count();
        let failed_nodes = nodes.values().filter(|n| n.status == NodeStatus::Failed).count();

        let virtual_nodes = hash_ring.virtual_node_count();
        let is_balanced = hash_ring.is_balanced(self.config.rebalance_threshold);

        ClusterStats {
            total_nodes,
            healthy_nodes,
            degraded_nodes,
            failed_nodes,
            virtual_nodes,
            replication_factor: self.config.replication_factor,
            is_balanced,
        }
    }

    /// Get shard metrics
    pub async fn get_metrics(&self) -> Arc<ShardMetrics> {
        self.metrics.clone()
    }

    /// Start background maintenance tasks
    async fn start_background_tasks(self: &Arc<Self>) {
        // Health check task
        tokio::spawn(Self::health_check_task(self.clone()));

        // Metrics collection task
        tokio::spawn(Self::metrics_task(self.clone()));

        // Failure detection task
        tokio::spawn(Self::failure_detection_task(self.clone()));
    }

    /// Background task for health checks
    async fn health_check_task(manager: Arc<ShardManager>) {
        let mut interval = interval(manager.config.heartbeat_interval());

        loop {
            interval.tick().await;

            if !*manager.running.read().await {
                break;
            }

            // TODO: Implement actual health checks
            // This would involve sending heartbeat requests to all nodes
            debug!("Performing health checks");
        }
    }

    /// Background task for metrics collection
    async fn metrics_task(manager: Arc<ShardManager>) {
        let mut interval = interval(manager.config.metrics_interval());

        loop {
            interval.tick().await;

            if !*manager.running.read().await {
                break;
            }

            // Collect and update metrics
            let stats = manager.get_cluster_stats().await;
            manager.metrics.update_cluster_stats(stats).await;
        }
    }

    /// Background task for failure detection
    async fn failure_detection_task(manager: Arc<ShardManager>) {
        let mut interval = interval(manager.config.failure_detection_timeout());

        loop {
            interval.tick().await;

            if !*manager.running.read().await {
                break;
            }

            // Check for failed nodes
            let nodes = manager.nodes.read().await;
            let failed_nodes: Vec<NodeId> = nodes
                .iter()
                .filter(|(_, node_info)| {
                    node_info.is_failed(manager.config.failure_detection_timeout())
                        && node_info.status != NodeStatus::Failed
                })
                .map(|(node_id, _)| node_id.clone())
                .collect();
            drop(nodes);

            // Mark failed nodes
            for node_id in failed_nodes {
                warn!("Detected failed node: {}", node_id);
                if let Err(e) = manager.update_node_status(&node_id, NodeStatus::Failed).await {
                    error!("Failed to update node status for {}: {}", node_id, e);
                }
            }
        }
    }
}

/// Cluster statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub failed_nodes: usize,
    pub virtual_nodes: usize,
    pub replication_factor: usize,
    pub is_balanced: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_shard_manager_creation() {
        let config = ShardConfig::test_config();
        let manager = ShardManager::new(config);

        assert!(!*manager.running.read().await);

        let stats = manager.get_cluster_stats().await;
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.virtual_nodes, 0);
    }

    #[tokio::test]
    async fn test_start_stop() {
        use std::sync::Arc;

        let config = ShardConfig::test_config();
        let manager = Arc::new(ShardManager::new(config));

        // Start
        assert!(manager.start().await.is_ok());
        assert!(*manager.running.read().await);

        // Try to start again (should fail)
        assert!(manager.start().await.is_err());

        // Stop
        assert!(manager.stop().await.is_ok());
        assert!(!*manager.running.read().await);

        // Try to stop again (should fail)
        assert!(manager.stop().await.is_err());
    }

    #[tokio::test]
    async fn test_add_remove_nodes() {
        let config = ShardConfig::test_config();
        let manager = ShardManager::new(config);

        let node1 = NodeId::new("node1");
        let addr1: SocketAddr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8080);

        // Add node
        assert!(manager.add_node(node1.clone(), addr1).await.is_ok());

        let stats = manager.get_cluster_stats().await;
        assert_eq!(stats.total_nodes, 1);
        assert!(stats.virtual_nodes > 0);

        // Try to add same node again (should fail)
        assert!(manager.add_node(node1.clone(), addr1).await.is_err());

        // Remove node
        assert!(manager.remove_node(&node1).await.is_ok());

        let stats = manager.get_cluster_stats().await;
        assert_eq!(stats.total_nodes, 0);

        // Try to remove non-existent node (should fail)
        let node2 = NodeId::new("node2");
        assert!(manager.remove_node(&node2).await.is_err());
    }

    #[tokio::test]
    async fn test_key_routing() {
        let config = ShardConfig::test_config();
        let manager = ShardManager::new(config);

        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");
        let addr: SocketAddr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8080);

        // Add nodes
        manager.add_node(node1.clone(), addr).await.unwrap();
        manager.add_node(node2.clone(), SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8081)).await.unwrap();

        // Process heartbeats to make nodes healthy (from Joining status)
        manager.process_heartbeat(&node1).await.unwrap();
        manager.process_heartbeat(&node2).await.unwrap();

        // Test key routing
        let responsible_nodes = manager.get_responsible_nodes("test_key").await;
        assert!(!responsible_nodes.is_empty());

        let primary_node = manager.get_primary_node("test_key").await.unwrap();
        assert!(responsible_nodes.contains(&primary_node));
    }

    #[tokio::test]
    async fn test_node_status_updates() {
        let config = ShardConfig::test_config();
        let manager = ShardManager::new(config);

        let node1 = NodeId::new("node1");
        let addr: SocketAddr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8080);

        manager.add_node(node1.clone(), addr).await.unwrap();

        // Update status
        assert!(manager.update_node_status(&node1, NodeStatus::Degraded).await.is_ok());

        let node_info = manager.get_node(&node1).await.unwrap();
        assert_eq!(node_info.status, NodeStatus::Degraded);

        // Update non-existent node (should fail)
        let node2 = NodeId::new("node2");
        assert!(manager.update_node_status(&node2, NodeStatus::Healthy).await.is_err());
    }

    #[tokio::test]
    async fn test_heartbeat_processing() {
        let config = ShardConfig::test_config();
        let manager = ShardManager::new(config);

        let node1 = NodeId::new("node1");
        let addr: SocketAddr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8080);

        manager.add_node(node1.clone(), addr).await.unwrap();

        let node_info_before = manager.get_node(&node1).await.unwrap();

        // Process heartbeat
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(manager.process_heartbeat(&node1).await.is_ok());

        let node_info_after = manager.get_node(&node1).await.unwrap();
        assert!(node_info_after.last_heartbeat > node_info_before.last_heartbeat);
    }
}