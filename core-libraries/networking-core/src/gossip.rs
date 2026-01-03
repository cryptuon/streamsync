//! Gossip Protocol Implementation
//!
//! Implements a push-pull gossip protocol for peer discovery and
//! state propagation in the distributed network.

use crate::{NetworkError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Gossip message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    /// Push our peer list to others
    Push(GossipPush),
    /// Request peer list from others
    Pull(GossipPull),
    /// Combined push-pull for efficiency
    PushPull(GossipPushPull),
    /// Heartbeat to indicate liveness
    Heartbeat(GossipHeartbeat),
    /// State synchronization message
    StateSync(GossipStateSync),
}

/// Push message containing peer information to share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPush {
    /// Sender's node ID
    pub sender_id: Uuid,
    /// Peers to share
    pub peers: Vec<GossipPeerInfo>,
    /// Message timestamp
    pub timestamp: u64,
    /// Digest of current state for consistency checks
    pub state_digest: u64,
}

/// Pull request for peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPull {
    /// Requester's node ID
    pub requester_id: Uuid,
    /// Services we're interested in
    pub service_filter: Option<HashSet<String>>,
    /// Maximum peers to return
    pub max_peers: usize,
    /// Our known peer IDs (to avoid duplicates)
    pub known_peers: HashSet<Uuid>,
}

/// Combined push-pull for efficiency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPushPull {
    /// Our peers to share
    pub push: GossipPush,
    /// What we're looking for
    pub pull: GossipPull,
}

/// Heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipHeartbeat {
    /// Sender's node ID
    pub node_id: Uuid,
    /// Current timestamp
    pub timestamp: u64,
    /// Node load/health metrics
    pub metrics: NodeMetrics,
}

/// State synchronization for specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipStateSync {
    /// State key (e.g., "shard_map", "config")
    pub key: String,
    /// Version number
    pub version: u64,
    /// State data (serialized)
    pub data: Vec<u8>,
    /// Hash of the data for verification
    pub hash: [u8; 32],
}

/// Peer information shared via gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPeerInfo {
    /// Peer's unique identifier
    pub peer_id: Uuid,
    /// Network addresses
    pub addresses: Vec<SocketAddr>,
    /// Services provided
    pub services: HashSet<String>,
    /// Protocol version
    pub protocol_version: u32,
    /// Region for locality awareness
    pub region: Option<String>,
    /// Last known status
    pub status: PeerStatus,
    /// Gossip hop count (for limiting propagation)
    pub hop_count: u8,
    /// When this info was last updated
    pub last_updated: u64,
}

/// Peer status reported via gossip
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    /// Peer is healthy and responsive
    Healthy,
    /// Peer is suspected to be failing
    Suspected,
    /// Peer is confirmed down
    Down,
    /// Peer status unknown
    Unknown,
}

/// Node metrics included in heartbeats
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeMetrics {
    /// CPU usage percentage (0-100)
    pub cpu_usage: f32,
    /// Memory usage percentage (0-100)
    pub memory_usage: f32,
    /// Active connections count
    pub connections: u32,
    /// Queries per second
    pub qps: f32,
    /// Average response latency in ms
    pub avg_latency_ms: f32,
}

/// Configuration for the gossip protocol
#[derive(Debug, Clone)]
pub struct GossipConfig {
    /// Number of peers to gossip with each round
    pub fanout: usize,
    /// Interval between gossip rounds
    pub gossip_interval: Duration,
    /// Interval between heartbeats
    pub heartbeat_interval: Duration,
    /// Maximum hop count for message propagation
    pub max_hops: u8,
    /// Time before considering a peer suspicious
    pub suspicion_timeout: Duration,
    /// Time before marking peer as down
    pub down_timeout: Duration,
    /// Enable push-pull optimization
    pub use_push_pull: bool,
    /// Maximum peers to track
    pub max_peers: usize,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            fanout: 3,
            gossip_interval: Duration::from_secs(1),
            heartbeat_interval: Duration::from_secs(5),
            max_hops: 4,
            suspicion_timeout: Duration::from_secs(15),
            down_timeout: Duration::from_secs(60),
            use_push_pull: true,
            max_peers: 1000,
        }
    }
}

/// Gossip protocol manager
pub struct GossipManager {
    /// Our node ID
    node_id: Uuid,
    /// Configuration
    config: GossipConfig,
    /// Known peers with their info
    peers: Arc<RwLock<HashMap<Uuid, GossipPeerEntry>>>,
    /// Running state
    running: Arc<RwLock<bool>>,
    /// Gossip statistics
    stats: Arc<RwLock<GossipStats>>,
    /// Pending outbound messages
    outbound_queue: Arc<RwLock<Vec<(SocketAddr, GossipMessage)>>>,
    /// State versions for sync
    state_versions: Arc<RwLock<HashMap<String, u64>>>,
}

/// Entry for a tracked peer
#[derive(Debug, Clone)]
pub struct GossipPeerEntry {
    /// Peer information
    pub info: GossipPeerInfo,
    /// Last time we received data from this peer
    pub last_received: Instant,
    /// Number of failed communication attempts
    pub failed_attempts: u32,
    /// Latest heartbeat metrics
    pub metrics: Option<NodeMetrics>,
}

/// Statistics for gossip protocol
#[derive(Debug, Clone, Default)]
pub struct GossipStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Peers discovered via gossip
    pub peers_discovered: u64,
    /// Peers marked as down
    pub peers_marked_down: u64,
    /// Gossip rounds completed
    pub rounds_completed: u64,
    /// State sync messages exchanged
    pub state_syncs: u64,
}

impl GossipManager {
    /// Create a new gossip manager
    pub fn new(node_id: Uuid, config: GossipConfig) -> Self {
        Self {
            node_id,
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(GossipStats::default())),
            outbound_queue: Arc::new(RwLock::new(Vec::new())),
            state_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the gossip protocol
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(NetworkError::AlreadyRunning);
        }
        *running = true;
        drop(running);

        info!("Starting gossip protocol for node {}", self.node_id);

        // Start gossip round loop
        self.start_gossip_loop().await;

        // Start heartbeat loop
        self.start_heartbeat_loop().await;

        // Start peer status check loop
        self.start_status_check_loop().await;

        Ok(())
    }

    /// Stop the gossip protocol
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        info!("Gossip protocol stopped");
        Ok(())
    }

    /// Add a peer manually (e.g., from bootstrap)
    pub async fn add_peer(&self, info: GossipPeerInfo) {
        let mut peers = self.peers.write().await;

        if peers.len() >= self.config.max_peers && !peers.contains_key(&info.peer_id) {
            debug!("Peer limit reached, not adding {}", info.peer_id);
            return;
        }

        let entry = GossipPeerEntry {
            info,
            last_received: Instant::now(),
            failed_attempts: 0,
            metrics: None,
        };

        let peer_id = entry.info.peer_id;
        peers.insert(peer_id, entry);

        let mut stats = self.stats.write().await;
        stats.peers_discovered += 1;

        debug!("Added peer {} via gossip", peer_id);
    }

    /// Get all known healthy peers
    pub async fn get_healthy_peers(&self) -> Vec<GossipPeerInfo> {
        self.peers
            .read()
            .await
            .values()
            .filter(|e| e.info.status == PeerStatus::Healthy)
            .map(|e| e.info.clone())
            .collect()
    }

    /// Get all peers with a specific service
    pub async fn get_peers_by_service(&self, service: &str) -> Vec<GossipPeerInfo> {
        self.peers
            .read()
            .await
            .values()
            .filter(|e| e.info.services.contains(service))
            .map(|e| e.info.clone())
            .collect()
    }

    /// Process an incoming gossip message
    pub async fn handle_message(&self, from: SocketAddr, message: GossipMessage) -> Result<Option<GossipMessage>> {
        let mut stats = self.stats.write().await;
        stats.messages_received += 1;
        drop(stats);

        match message {
            GossipMessage::Push(push) => {
                self.handle_push(push).await?;
                Ok(None)
            }
            GossipMessage::Pull(pull) => {
                let response = self.handle_pull(pull).await?;
                Ok(Some(response))
            }
            GossipMessage::PushPull(pp) => {
                let response = self.handle_push_pull(pp).await?;
                Ok(Some(response))
            }
            GossipMessage::Heartbeat(hb) => {
                self.handle_heartbeat(from, hb).await?;
                Ok(None)
            }
            GossipMessage::StateSync(sync) => {
                self.handle_state_sync(sync).await?;
                Ok(None)
            }
        }
    }

    /// Get pending outbound messages and clear queue
    pub async fn drain_outbound(&self) -> Vec<(SocketAddr, GossipMessage)> {
        let mut queue = self.outbound_queue.write().await;
        std::mem::take(&mut *queue)
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> GossipStats {
        self.stats.read().await.clone()
    }

    /// Trigger a state sync broadcast
    pub async fn broadcast_state(&self, key: String, data: Vec<u8>) -> Result<()> {
        let version = {
            let mut versions = self.state_versions.write().await;
            let v = versions.entry(key.clone()).or_insert(0);
            *v += 1;
            *v
        };

        let hash = self.compute_hash(&data);

        let sync = GossipStateSync {
            key,
            version,
            data,
            hash,
        };

        // Queue for broadcast to all peers
        let peers = self.peers.read().await;
        let mut queue = self.outbound_queue.write().await;

        for entry in peers.values() {
            if let Some(addr) = entry.info.addresses.first() {
                queue.push((*addr, GossipMessage::StateSync(sync.clone())));
            }
        }

        let mut stats = self.stats.write().await;
        stats.state_syncs += 1;

        Ok(())
    }

    // Internal methods

    async fn start_gossip_loop(&self) {
        let running = self.running.clone();
        let peers = self.peers.clone();
        let stats = self.stats.clone();
        let outbound = self.outbound_queue.clone();
        let config = self.config.clone();
        let node_id = self.node_id;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.gossip_interval);

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                // Select random peers for gossip
                let targets = Self::select_gossip_targets(&peers, config.fanout).await;

                if targets.is_empty() {
                    continue;
                }

                let target_count = targets.len();
                debug!("Gossip round: targeting {} peers", target_count);

                // Build our peer list to share
                let our_peers = Self::build_peer_list(&peers, config.max_hops).await;
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                for (addr, peer_id) in targets {
                    let message = if config.use_push_pull {
                        GossipMessage::PushPull(GossipPushPull {
                            push: GossipPush {
                                sender_id: node_id,
                                peers: our_peers.clone(),
                                timestamp,
                                state_digest: 0, // Simplified
                            },
                            pull: GossipPull {
                                requester_id: node_id,
                                service_filter: None,
                                max_peers: 10,
                                known_peers: peers.read().await.keys().cloned().collect(),
                            },
                        })
                    } else {
                        GossipMessage::Push(GossipPush {
                            sender_id: node_id,
                            peers: our_peers.clone(),
                            timestamp,
                            state_digest: 0,
                        })
                    };

                    outbound.write().await.push((addr, message));
                    debug!("Queued gossip message to peer {} at {}", peer_id, addr);
                }

                let mut stats = stats.write().await;
                stats.rounds_completed += 1;
                stats.messages_sent += target_count as u64;
            }
        });
    }

    async fn start_heartbeat_loop(&self) {
        let running = self.running.clone();
        let peers = self.peers.clone();
        let outbound = self.outbound_queue.clone();
        let config = self.config.clone();
        let node_id = self.node_id;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.heartbeat_interval);

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                let heartbeat = GossipHeartbeat {
                    node_id,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    metrics: NodeMetrics::default(), // Would be populated with real metrics
                };

                // Send heartbeat to all known peers
                let peers_guard = peers.read().await;
                let mut queue = outbound.write().await;

                for entry in peers_guard.values() {
                    if let Some(addr) = entry.info.addresses.first() {
                        queue.push((*addr, GossipMessage::Heartbeat(heartbeat.clone())));
                    }
                }

                debug!("Sent heartbeat to {} peers", peers_guard.len());
            }
        });
    }

    async fn start_status_check_loop(&self) {
        let running = self.running.clone();
        let peers = self.peers.clone();
        let stats = self.stats.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                let now = Instant::now();
                let mut peers_guard = peers.write().await;
                let mut marked_down = 0;

                for entry in peers_guard.values_mut() {
                    let elapsed = now.duration_since(entry.last_received);

                    let new_status = if elapsed < config.suspicion_timeout {
                        PeerStatus::Healthy
                    } else if elapsed < config.down_timeout {
                        PeerStatus::Suspected
                    } else {
                        marked_down += 1;
                        PeerStatus::Down
                    };

                    if entry.info.status != new_status {
                        info!(
                            "Peer {} status changed: {:?} -> {:?}",
                            entry.info.peer_id, entry.info.status, new_status
                        );
                        entry.info.status = new_status;
                    }
                }

                if marked_down > 0 {
                    let mut stats = stats.write().await;
                    stats.peers_marked_down += marked_down;
                }

                // Remove peers that have been down for too long
                let before_count = peers_guard.len();
                peers_guard.retain(|_, entry| {
                    if entry.info.status == PeerStatus::Down {
                        let elapsed = now.duration_since(entry.last_received);
                        elapsed < config.down_timeout * 3 // Keep for 3x down timeout
                    } else {
                        true
                    }
                });

                let removed = before_count - peers_guard.len();
                if removed > 0 {
                    info!("Removed {} dead peers", removed);
                }
            }
        });
    }

    async fn select_gossip_targets(
        peers: &Arc<RwLock<HashMap<Uuid, GossipPeerEntry>>>,
        fanout: usize,
    ) -> Vec<(SocketAddr, Uuid)> {
        let peers_guard = peers.read().await;
        let healthy: Vec<_> = peers_guard
            .values()
            .filter(|e| e.info.status == PeerStatus::Healthy || e.info.status == PeerStatus::Suspected)
            .collect();

        if healthy.is_empty() {
            return Vec::new();
        }

        // Random selection using fastrand
        let mut selected = Vec::with_capacity(fanout.min(healthy.len()));
        let mut indices: Vec<usize> = (0..healthy.len()).collect();

        for _ in 0..fanout.min(healthy.len()) {
            if indices.is_empty() {
                break;
            }
            let idx = fastrand::usize(0..indices.len());
            let peer_idx = indices.swap_remove(idx);
            let entry = &healthy[peer_idx];
            if let Some(addr) = entry.info.addresses.first() {
                selected.push((*addr, entry.info.peer_id));
            }
        }

        selected
    }

    async fn build_peer_list(
        peers: &Arc<RwLock<HashMap<Uuid, GossipPeerEntry>>>,
        max_hops: u8,
    ) -> Vec<GossipPeerInfo> {
        peers
            .read()
            .await
            .values()
            .filter(|e| e.info.hop_count < max_hops)
            .map(|e| {
                let mut info = e.info.clone();
                info.hop_count += 1; // Increment hop count
                info
            })
            .collect()
    }

    async fn handle_push(&self, push: GossipPush) -> Result<()> {
        debug!("Handling push from {} with {} peers", push.sender_id, push.peers.len());

        for peer_info in push.peers {
            if peer_info.peer_id == self.node_id {
                continue; // Skip ourselves
            }

            let mut peers = self.peers.write().await;

            if let Some(existing) = peers.get_mut(&peer_info.peer_id) {
                // Update if newer
                if peer_info.last_updated > existing.info.last_updated {
                    existing.info = peer_info;
                    existing.last_received = Instant::now();
                }
            } else if peers.len() < self.config.max_peers {
                // Add new peer
                peers.insert(peer_info.peer_id, GossipPeerEntry {
                    info: peer_info,
                    last_received: Instant::now(),
                    failed_attempts: 0,
                    metrics: None,
                });

                let mut stats = self.stats.write().await;
                stats.peers_discovered += 1;
            }
        }

        Ok(())
    }

    async fn handle_pull(&self, pull: GossipPull) -> Result<GossipMessage> {
        debug!("Handling pull from {}", pull.requester_id);

        let peers = self.peers.read().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut response_peers: Vec<GossipPeerInfo> = peers
            .values()
            .filter(|e| !pull.known_peers.contains(&e.info.peer_id))
            .filter(|e| {
                if let Some(ref filter) = pull.service_filter {
                    e.info.services.iter().any(|s| filter.contains(s))
                } else {
                    true
                }
            })
            .take(pull.max_peers)
            .map(|e| e.info.clone())
            .collect();

        // Increment hop count
        for peer in &mut response_peers {
            peer.hop_count += 1;
        }

        Ok(GossipMessage::Push(GossipPush {
            sender_id: self.node_id,
            peers: response_peers,
            timestamp,
            state_digest: 0,
        }))
    }

    async fn handle_push_pull(&self, pp: GossipPushPull) -> Result<GossipMessage> {
        // Handle the push part
        self.handle_push(pp.push).await?;

        // Handle the pull part and return response
        self.handle_pull(pp.pull).await
    }

    async fn handle_heartbeat(&self, from: SocketAddr, hb: GossipHeartbeat) -> Result<()> {
        let mut peers = self.peers.write().await;

        if let Some(entry) = peers.get_mut(&hb.node_id) {
            entry.last_received = Instant::now();
            entry.metrics = Some(hb.metrics);
            entry.info.status = PeerStatus::Healthy;
            debug!("Received heartbeat from {} at {}", hb.node_id, from);
        } else {
            // Unknown peer sent heartbeat, could add them
            debug!("Heartbeat from unknown peer {} at {}", hb.node_id, from);
        }

        Ok(())
    }

    async fn handle_state_sync(&self, sync: GossipStateSync) -> Result<()> {
        // Verify hash
        let computed_hash = self.compute_hash(&sync.data);
        if computed_hash != sync.hash {
            warn!("State sync hash mismatch for key {}", sync.key);
            return Ok(());
        }

        // Check version
        let mut versions = self.state_versions.write().await;
        let current_version = versions.get(&sync.key).cloned().unwrap_or(0);

        if sync.version > current_version {
            versions.insert(sync.key.clone(), sync.version);
            debug!("Updated state {} to version {}", sync.key, sync.version);
            // In a real implementation, you'd callback to apply the state
        }

        Ok(())
    }

    fn compute_hash(&self, data: &[u8]) -> [u8; 32] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();

        let mut result = [0u8; 32];
        result[..8].copy_from_slice(&hash.to_le_bytes());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gossip_manager_creation() {
        let node_id = Uuid::new_v4();
        let config = GossipConfig::default();
        let manager = GossipManager::new(node_id, config);

        assert_eq!(manager.node_id, node_id);
    }

    #[tokio::test]
    async fn test_add_peer() {
        let node_id = Uuid::new_v4();
        let manager = GossipManager::new(node_id, GossipConfig::default());

        let peer_info = GossipPeerInfo {
            peer_id: Uuid::new_v4(),
            addresses: vec!["127.0.0.1:8080".parse().unwrap()],
            services: ["query".to_string()].into_iter().collect(),
            protocol_version: 1,
            region: Some("us-west".to_string()),
            status: PeerStatus::Healthy,
            hop_count: 0,
            last_updated: 0,
        };

        manager.add_peer(peer_info.clone()).await;

        let peers = manager.get_healthy_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, peer_info.peer_id);
    }

    #[tokio::test]
    async fn test_get_peers_by_service() {
        let node_id = Uuid::new_v4();
        let manager = GossipManager::new(node_id, GossipConfig::default());

        let peer1 = GossipPeerInfo {
            peer_id: Uuid::new_v4(),
            addresses: vec!["127.0.0.1:8080".parse().unwrap()],
            services: ["query", "storage"].into_iter().map(String::from).collect(),
            protocol_version: 1,
            region: None,
            status: PeerStatus::Healthy,
            hop_count: 0,
            last_updated: 0,
        };

        let peer2 = GossipPeerInfo {
            peer_id: Uuid::new_v4(),
            addresses: vec!["127.0.0.1:8081".parse().unwrap()],
            services: ["consensus"].into_iter().map(String::from).collect(),
            protocol_version: 1,
            region: None,
            status: PeerStatus::Healthy,
            hop_count: 0,
            last_updated: 0,
        };

        manager.add_peer(peer1).await;
        manager.add_peer(peer2).await;

        let query_peers = manager.get_peers_by_service("query").await;
        assert_eq!(query_peers.len(), 1);

        let consensus_peers = manager.get_peers_by_service("consensus").await;
        assert_eq!(consensus_peers.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_push() {
        let node_id = Uuid::new_v4();
        let manager = GossipManager::new(node_id, GossipConfig::default());

        let push = GossipPush {
            sender_id: Uuid::new_v4(),
            peers: vec![
                GossipPeerInfo {
                    peer_id: Uuid::new_v4(),
                    addresses: vec!["127.0.0.1:8080".parse().unwrap()],
                    services: HashSet::new(),
                    protocol_version: 1,
                    region: None,
                    status: PeerStatus::Healthy,
                    hop_count: 0,
                    last_updated: 100,
                },
            ],
            timestamp: 100,
            state_digest: 0,
        };

        manager.handle_push(push).await.unwrap();

        let peers = manager.get_healthy_peers().await;
        assert_eq!(peers.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_pull() {
        let node_id = Uuid::new_v4();
        let manager = GossipManager::new(node_id, GossipConfig::default());

        // Add a peer first
        let peer_info = GossipPeerInfo {
            peer_id: Uuid::new_v4(),
            addresses: vec!["127.0.0.1:8080".parse().unwrap()],
            services: HashSet::new(),
            protocol_version: 1,
            region: None,
            status: PeerStatus::Healthy,
            hop_count: 0,
            last_updated: 0,
        };
        manager.add_peer(peer_info.clone()).await;

        let pull = GossipPull {
            requester_id: Uuid::new_v4(),
            service_filter: None,
            max_peers: 10,
            known_peers: HashSet::new(),
        };

        let response = manager.handle_pull(pull).await.unwrap();

        match response {
            GossipMessage::Push(push) => {
                assert_eq!(push.peers.len(), 1);
            }
            _ => panic!("Expected Push response"),
        }
    }

    #[test]
    fn test_gossip_config_defaults() {
        let config = GossipConfig::default();
        assert_eq!(config.fanout, 3);
        assert_eq!(config.gossip_interval, Duration::from_secs(1));
        assert!(config.use_push_pull);
    }

    #[test]
    fn test_gossip_message_serialization() {
        let push = GossipPush {
            sender_id: Uuid::new_v4(),
            peers: vec![],
            timestamp: 12345,
            state_digest: 0,
        };

        let message = GossipMessage::Push(push);
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: GossipMessage = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            GossipMessage::Push(p) => assert_eq!(p.timestamp, 12345),
            _ => panic!("Wrong message type"),
        }
    }
}
