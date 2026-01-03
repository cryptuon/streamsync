//! Node discovery and peer management

use super::{NetworkConfig, protocol::{NetworkMessage, PeerAddress}};
use anyhow::Result;
use nng::{Socket, Protocol};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use uuid::Uuid;
use tracing::{info, debug};

/// Node discovery service for finding and managing peers
pub struct NodeDiscovery {
    config: NetworkConfig,
    known_peers: Arc<RwLock<HashMap<Uuid, PeerAddress>>>,
    discovery_socket: Option<Socket>,
    running: Arc<RwLock<bool>>,
}

impl NodeDiscovery {
    /// Create a new node discovery service
    pub fn new(config: NetworkConfig) -> Result<Self> {
        Ok(Self {
            config,
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            discovery_socket: None,
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the node discovery service
    pub async fn start(&mut self) -> Result<()> {
        info!("🔍 Starting node discovery service");

        // Create discovery socket for broadcasting
        let socket = Socket::new(Protocol::Pub0)?;
        socket.listen(&format!("tcp://{}", self.config.listen_addr))?;
        self.discovery_socket = Some(socket);

        *self.running.write().await = true;

        // Start periodic peer discovery
        self.start_peer_discovery().await;

        // Start peer cleanup task
        self.start_peer_cleanup().await;

        // Announce ourselves to the network
        self.announce_self().await?;

        info!("✅ Node discovery service started");
        Ok(())
    }

    /// Stop the node discovery service
    pub async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping node discovery service");

        *self.running.write().await = false;

        if let Some(socket) = &self.discovery_socket {
            socket.close();
        }

        info!("✅ Node discovery service stopped");
        Ok(())
    }

    /// Add a known peer
    pub async fn add_peer(&self, peer: PeerAddress) {
        let mut peers = self.known_peers.write().await;
        peers.insert(peer.node_id, peer);
    }

    /// Remove a peer
    pub async fn remove_peer(&self, node_id: Uuid) {
        let mut peers = self.known_peers.write().await;
        peers.remove(&node_id);
    }

    /// Get all known peers
    pub async fn get_peers(&self) -> Vec<PeerAddress> {
        self.known_peers.read().await.values().cloned().collect()
    }

    /// Get peers suitable for connection (not already connected)
    pub async fn get_candidate_peers(&self, max_count: usize) -> Vec<PeerAddress> {
        let peers = self.known_peers.read().await;

        // Sort by last seen time and take the most recent
        let mut candidates: Vec<_> = peers.values().cloned().collect();
        candidates.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
        candidates.truncate(max_count);

        candidates
    }

    /// Request peer list from a specific node
    pub async fn request_peers_from(&self, target_addr: SocketAddr) -> Result<Vec<PeerAddress>> {
        let socket = Socket::new(Protocol::Req0)?;
        socket.dial(&format!("tcp://{}", target_addr))?;

        let request = NetworkMessage::PeerDiscovery {
            requesting_node: self.config.node_id,
            known_peers: self.get_peers().await,
        };

        let request_data = bincode::serialize(&request)?;
        socket.send(&request_data).map_err(|(_msg, e)| anyhow::anyhow!("Send failed: {:?}", e))?;

        // Wait for response
        let response_data = socket.recv()?;
        let response: NetworkMessage = bincode::deserialize(&response_data)?;

        match response {
            NetworkMessage::PeerDiscoveryResponse { peers, .. } => {
                // Add discovered peers to our known peers
                for peer in &peers {
                    self.add_peer(peer.clone()).await;
                }
                Ok(peers)
            }
            _ => Err(anyhow::anyhow!("Unexpected response to peer discovery request")),
        }
    }

    /// Handle incoming peer discovery requests
    pub async fn handle_peer_request(&self, requesting_node: Uuid) -> Vec<PeerAddress> {
        debug!("📥 Handling peer discovery request from {}", requesting_node);

        // Return a subset of our known peers
        let peers = self.get_candidate_peers(20).await;

        debug!("📤 Sending {} peers to {}", peers.len(), requesting_node);
        peers
    }

    /// Announce this node to the network
    pub async fn announce_self(&self) -> Result<()> {
        let announcement = NetworkMessage::NodeAnnouncement {
            node_id: self.config.node_id,
            listen_addr: self.config.listen_addr.to_string(),
            capabilities: vec![
                "zk-reconstruction".to_string(),
                "program-parsing".to_string(),
                "distributed-query".to_string(),
                "pbft-consensus".to_string(),
            ],
            version: self.config.protocol_version,
        };

        if let Some(socket) = &self.discovery_socket {
            let data = bincode::serialize(&announcement)?;
            socket.send(&data).map_err(|(_msg, e)| anyhow::anyhow!("Send failed: {:?}", e))?;
        }

        info!("📢 Announced node {} to network", self.config.node_id);
        Ok(())
    }

    /// Get discovery statistics
    pub async fn get_stats(&self) -> DiscoveryStats {
        let peers = self.known_peers.read().await;
        let now = chrono::Utc::now();

        let active_peers = peers.values()
            .filter(|p| now.signed_duration_since(p.last_seen).num_minutes() < 10)
            .count();

        DiscoveryStats {
            total_known_peers: peers.len(),
            active_peers,
            discovery_enabled: *self.running.read().await,
            last_announcement: now, // TODO: Track actual last announcement
        }
    }

    // Private methods

    async fn start_peer_discovery(&self) {
        let known_peers = self.known_peers.clone();
        let running = self.running.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Discover peers every 30 seconds

            while *running.read().await {
                interval.tick().await;

                // Try to discover peers from bootstrap nodes
                for &bootstrap_addr in &config.bootstrap_peers {
                    if let Ok(discovery) = NodeDiscovery::new(config.clone()) {
                        match discovery.request_peers_from(bootstrap_addr).await {
                            Ok(new_peers) => {
                                debug!("🔍 Discovered {} peers from {}", new_peers.len(), bootstrap_addr);
                                for peer in new_peers {
                                    known_peers.write().await.insert(peer.node_id, peer);
                                }
                            }
                            Err(e) => {
                                debug!("Failed to discover peers from {}: {}", bootstrap_addr, e);
                            }
                        }
                    }
                }

                // Try to discover peers from known peers
                let current_peers: Vec<_> = known_peers.read().await.values().cloned().collect();
                for peer in current_peers.iter().take(5) { // Limit to 5 peers per round
                    if let Ok(addr) = peer.addr.parse::<SocketAddr>() {
                        if let Ok(discovery) = NodeDiscovery::new(config.clone()) {
                            if let Ok(new_peers) = discovery.request_peers_from(addr).await {
                                debug!("🔍 Discovered {} peers from {}", new_peers.len(), peer.node_id);
                                for new_peer in new_peers {
                                    known_peers.write().await.insert(new_peer.node_id, new_peer);
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    async fn start_peer_cleanup(&self) {
        let known_peers = self.known_peers.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Cleanup every minute

            while *running.read().await {
                interval.tick().await;

                let now = chrono::Utc::now();
                let mut peers = known_peers.write().await;

                // Remove peers that haven't been seen for more than 10 minutes
                peers.retain(|_, peer| {
                    now.signed_duration_since(peer.last_seen).num_minutes() < 10
                });
            }
        });
    }
}

/// Discovery service statistics
#[derive(Debug, Clone)]
pub struct DiscoveryStats {
    pub total_known_peers: usize,
    pub active_peers: usize,
    pub discovery_enabled: bool,
    pub last_announcement: chrono::DateTime<chrono::Utc>,
}