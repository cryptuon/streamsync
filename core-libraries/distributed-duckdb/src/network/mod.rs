//! P2P Network Layer for Distributed StreamSync
//!
//! This module implements a high-performance peer-to-peer network using nng (nanomsg-next-gen)
//! for communication between StreamSync nodes. It provides reliable message passing, node discovery,
//! and network topology management for the decentralized architecture.

pub mod peer;
pub mod discovery;
pub mod topology;
pub mod protocol;
pub mod gossip;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Network configuration for the P2P layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Local node ID
    pub node_id: Uuid,
    /// Local listening address
    pub listen_addr: SocketAddr,
    /// Bootstrap peers for initial connection
    pub bootstrap_peers: Vec<SocketAddr>,
    /// Maximum number of peer connections
    pub max_peers: usize,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    /// Heartbeat interval for peer health checks
    pub heartbeat_interval_ms: u64,
    /// Network protocol version
    pub protocol_version: u32,
    /// Enable gossip protocol
    pub enable_gossip: bool,
    /// Gossip fanout (number of peers to gossip to)
    pub gossip_fanout: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            node_id: Uuid::new_v4(),
            listen_addr: "127.0.0.1:8080".parse().unwrap(),
            bootstrap_peers: vec![],
            max_peers: 50,
            connection_timeout_ms: 5000,
            heartbeat_interval_ms: 10000,
            protocol_version: 1,
            enable_gossip: true,
            gossip_fanout: 3,
        }
    }
}

/// P2P Network Manager
pub struct P2PNetwork {
    config: NetworkConfig,
    peers: Arc<RwLock<HashMap<Uuid, peer::Peer>>>,
    topology: Arc<RwLock<topology::NetworkTopology>>,
    discovery: discovery::NodeDiscovery,
    gossip: gossip::GossipProtocol,
}

impl P2PNetwork {
    /// Create a new P2P network instance
    pub fn new(config: NetworkConfig) -> Result<Self> {
        let peers = Arc::new(RwLock::new(HashMap::new()));
        let topology = Arc::new(RwLock::new(topology::NetworkTopology::new()));
        let discovery = discovery::NodeDiscovery::new(config.clone())?;
        let gossip = gossip::GossipProtocol::new(config.clone(), peers.clone())?;

        Ok(Self {
            config,
            peers,
            topology,
            discovery,
            gossip,
        })
    }

    /// Start the P2P network
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("🌐 Starting P2P network on {}", self.config.listen_addr);

        // Start node discovery
        self.discovery.start().await?;

        // Start gossip protocol if enabled
        if self.config.enable_gossip {
            self.gossip.start().await?;
        }

        // Connect to bootstrap peers
        self.connect_to_bootstrap_peers().await?;

        tracing::info!("✅ P2P network started successfully");
        Ok(())
    }

    /// Stop the P2P network
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("🛑 Stopping P2P network");

        // Stop gossip protocol
        self.gossip.stop().await?;

        // Stop discovery
        self.discovery.stop().await?;

        // Disconnect all peers
        let mut peers = self.peers.write().await;
        for (_, peer) in peers.drain() {
            peer.disconnect().await?;
        }

        tracing::info!("✅ P2P network stopped");
        Ok(())
    }

    /// Connect to a specific peer
    pub async fn connect_peer(&self, addr: SocketAddr) -> Result<Uuid> {
        let peer = peer::Peer::connect(addr, self.config.clone()).await?;
        let peer_id = peer.id();

        self.peers.write().await.insert(peer_id, peer);
        self.topology.write().await.add_peer(peer_id, addr).await?;

        tracing::info!("🤝 Connected to peer {} at {}", peer_id, addr);
        Ok(peer_id)
    }

    /// Disconnect from a peer
    pub async fn disconnect_peer(&self, peer_id: Uuid) -> Result<()> {
        if let Some(peer) = self.peers.write().await.remove(&peer_id) {
            peer.disconnect().await?;
            self.topology.write().await.remove_peer(peer_id).await?;
            tracing::info!("👋 Disconnected from peer {}", peer_id);
        }
        Ok(())
    }

    /// Send a message to a specific peer
    pub async fn send_to_peer(&self, peer_id: Uuid, message: protocol::NetworkMessage) -> Result<()> {
        let peers = self.peers.read().await;
        if let Some(peer) = peers.get(&peer_id) {
            peer.send_message(message).await?;
        }
        Ok(())
    }

    /// Broadcast a message to all connected peers
    pub async fn broadcast(&self, message: protocol::NetworkMessage) -> Result<()> {
        let peers = self.peers.read().await;
        let futures: Vec<_> = peers.values()
            .map(|peer| peer.send_message(message.clone()))
            .collect();

        for result in futures::future::join_all(futures).await {
            if let Err(e) = result {
                tracing::warn!("Failed to send broadcast message: {}", e);
            }
        }

        Ok(())
    }

    /// Get list of connected peers
    pub async fn get_connected_peers(&self) -> Vec<Uuid> {
        self.peers.read().await.keys().cloned().collect()
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> NetworkStats {
        let peers = self.peers.read().await;
        let topology = self.topology.read().await;

        NetworkStats {
            node_id: self.config.node_id,
            connected_peers: peers.len(),
            total_nodes: topology.node_count(),
            network_diameter: topology.diameter(),
            cluster_coefficient: topology.clustering_coefficient(),
            uptime_seconds: 0, // TODO: Track uptime
        }
    }

    /// Get network topology information
    pub async fn get_topology(&self) -> topology::NetworkTopology {
        self.topology.read().await.clone()
    }

    // Private helper methods

    async fn connect_to_bootstrap_peers(&self) -> Result<()> {
        let bootstrap_peers = self.config.bootstrap_peers.clone();

        for addr in bootstrap_peers {
            match self.connect_peer(addr).await {
                Ok(peer_id) => {
                    tracing::info!("✅ Connected to bootstrap peer {} at {}", peer_id, addr);
                }
                Err(e) => {
                    tracing::warn!("❌ Failed to connect to bootstrap peer {}: {}", addr, e);
                }
            }
        }

        Ok(())
    }
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub node_id: Uuid,
    pub connected_peers: usize,
    pub total_nodes: usize,
    pub network_diameter: usize,
    pub cluster_coefficient: f64,
    pub uptime_seconds: u64,
}