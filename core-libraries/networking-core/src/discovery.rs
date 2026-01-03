//! Peer discovery and network formation
//!
//! This module implements various peer discovery mechanisms for building
//! and maintaining the distributed network topology.

use crate::{NetworkError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Peer discovery mechanisms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    /// Bootstrap from a list of known peers
    Bootstrap(Vec<SocketAddr>),
    /// mDNS-based local network discovery
    Mdns { service_name: String },
    /// DHT-based distributed discovery
    Dht { bootstrap_nodes: Vec<SocketAddr> },
    /// Gossip-based peer propagation
    Gossip { fanout: usize },
    /// DNS-based discovery
    Dns { domain: String, srv_record: String },
}

/// Discovered peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    /// Unique peer identifier
    pub peer_id: Uuid,
    /// Network addresses where peer can be reached
    pub addresses: Vec<SocketAddr>,
    /// Peer capabilities and metadata
    pub metadata: PeerMetadata,
    /// When this peer was first discovered (seconds since epoch)
    #[serde(with = "instant_as_secs")]
    pub discovered_at: Instant,
    /// Last time we successfully communicated with this peer (seconds since epoch)
    #[serde(with = "option_instant_as_secs")]
    pub last_seen: Option<Instant>,
    /// Peer reputation score (0.0 = bad, 1.0 = excellent)
    pub reputation: f64,
}

/// Peer metadata and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerMetadata {
    /// Node type (primary, secondary, observer)
    pub node_type: String,
    /// Protocol version supported
    pub protocol_version: u32,
    /// Services provided by this peer
    pub services: HashSet<String>,
    /// Network region/zone for locality awareness
    pub region: Option<String>,
    /// Additional custom metadata
    pub extra: HashMap<String, String>,
}

/// Network formation strategy
#[derive(Debug, Clone)]
pub enum NetworkTopology {
    /// Full mesh - all nodes connect to all others
    FullMesh,
    /// Ring topology with configurable redundancy
    Ring { redundancy: usize },
    /// Star topology with designated hubs
    Star { hubs: Vec<Uuid> },
    /// Hierarchical with multiple levels
    Hierarchical { levels: usize, branching_factor: usize },
    /// Small world network (high clustering, low path length)
    SmallWorld { local_connections: usize, random_connections: usize },
}

/// Peer discovery and network formation manager
pub struct DiscoveryManager {
    /// Our node identifier
    node_id: Uuid,
    /// Discovery configuration
    config: DiscoveryConfig,
    /// Currently discovered peers
    peers: Arc<RwLock<HashMap<Uuid, DiscoveredPeer>>>,
    /// Active discovery methods
    methods: Vec<DiscoveryMethod>,
    /// Network formation strategy
    topology: NetworkTopology,
    /// Event broadcaster for peer updates
    event_sender: broadcast::Sender<DiscoveryEvent>,
    /// Running state
    running: Arc<RwLock<bool>>,
    /// Statistics
    stats: Arc<RwLock<DiscoveryStats>>,
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// How often to run discovery process
    pub discovery_interval: Duration,
    /// How often to gossip peer information
    pub gossip_interval: Duration,
    /// Maximum number of peers to maintain
    pub max_peers: usize,
    /// Minimum number of peers before considering network formed
    pub min_peers_for_formation: usize,
    /// Peer timeout duration
    pub peer_timeout: Duration,
    /// Bootstrap timeout
    pub bootstrap_timeout: Duration,
    /// Enable automatic network formation
    pub auto_form_network: bool,
}

/// Discovery events
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// New peer discovered
    PeerDiscovered(DiscoveredPeer),
    /// Peer information updated
    PeerUpdated(DiscoveredPeer),
    /// Peer lost or timed out
    PeerLost(Uuid),
    /// Network formation completed
    NetworkFormed { peer_count: usize },
    /// Network topology changed
    TopologyChanged { connected_peers: Vec<Uuid> },
}

/// Discovery statistics
#[derive(Debug, Clone)]
pub struct DiscoveryStats {
    pub total_peers_discovered: u64,
    pub active_peers: usize,
    pub failed_discovery_attempts: u64,
    pub network_formation_time: Option<Duration>,
    pub last_discovery_run: Option<Instant>,
    pub discovery_methods_active: usize,
}

impl DiscoveryManager {
    /// Create new discovery manager
    pub async fn new(
        node_id: Uuid,
        config: DiscoveryConfig,
        methods: Vec<DiscoveryMethod>,
        topology: NetworkTopology,
    ) -> Result<Self> {
        let (event_sender, _) = broadcast::channel(1000);

        let stats = DiscoveryStats {
            total_peers_discovered: 0,
            active_peers: 0,
            failed_discovery_attempts: 0,
            network_formation_time: None,
            last_discovery_run: None,
            discovery_methods_active: methods.len(),
        };

        Ok(Self {
            node_id,
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            methods,
            topology,
            event_sender,
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        })
    }

    /// Start the discovery manager
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(NetworkError::AlreadyRunning);
        }

        info!("Starting peer discovery manager with {} methods", self.methods.len());

        // Start discovery loop
        self.start_discovery_loop().await;

        // Start gossip loop
        self.start_gossip_loop().await;

        // Start peer maintenance
        self.start_peer_maintenance().await;

        *running = true;
        info!("Peer discovery manager started");
        Ok(())
    }

    /// Stop the discovery manager
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        info!("Stopping peer discovery manager");
        *running = false;
        Ok(())
    }

    /// Add a peer manually
    pub async fn add_peer(&self, peer: DiscoveredPeer) -> Result<()> {
        let mut peers = self.peers.write().await;
        let peer_id = peer.peer_id;

        peers.insert(peer_id, peer.clone());
        drop(peers);

        let _ = self.event_sender.send(DiscoveryEvent::PeerDiscovered(peer));

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_peers_discovered += 1;
        stats.active_peers = self.peers.read().await.len();

        info!("Manually added peer: {}", peer_id);
        Ok(())
    }

    /// Get all discovered peers
    pub async fn get_peers(&self) -> Vec<DiscoveredPeer> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Get peers by service capability
    pub async fn get_peers_by_service(&self, service: &str) -> Vec<DiscoveredPeer> {
        self.peers
            .read()
            .await
            .values()
            .filter(|peer| peer.metadata.services.contains(service))
            .cloned()
            .collect()
    }

    /// Subscribe to discovery events
    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveryEvent> {
        self.event_sender.subscribe()
    }

    /// Get discovery statistics
    pub async fn get_stats(&self) -> DiscoveryStats {
        self.stats.read().await.clone()
    }

    /// Check if network is formed according to strategy
    pub async fn is_network_formed(&self) -> bool {
        let peer_count = self.peers.read().await.len();
        peer_count >= self.config.min_peers_for_formation
    }

    /// Get recommended connections based on topology
    pub async fn get_recommended_connections(&self) -> Result<Vec<Uuid>> {
        let peers = self.get_peers().await;

        match &self.topology {
            NetworkTopology::FullMesh => {
                // Connect to all peers
                Ok(peers.into_iter().map(|p| p.peer_id).collect())
            }
            NetworkTopology::Ring { redundancy } => {
                // Connect to next N peers in ring
                self.calculate_ring_connections(&peers, *redundancy).await
            }
            NetworkTopology::Star { hubs } => {
                // Connect to hubs if we're not a hub, otherwise connect to all
                if hubs.contains(&self.node_id) {
                    Ok(peers.into_iter().map(|p| p.peer_id).collect())
                } else {
                    Ok(hubs.clone())
                }
            }
            NetworkTopology::SmallWorld { local_connections, random_connections } => {
                self.calculate_small_world_connections(&peers, *local_connections, *random_connections).await
            }
            NetworkTopology::Hierarchical { levels: _, branching_factor: _ } => {
                // Simplified hierarchical - connect to closest peers
                self.calculate_hierarchical_connections(&peers).await
            }
        }
    }

    async fn start_discovery_loop(&self) {
        let running = self.running.clone();
        let config = self.config.clone();
        let methods = self.methods.clone();
        let peers = self.peers.clone();
        let event_sender = self.event_sender.clone();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.discovery_interval);

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                debug!("Running peer discovery cycle");

                for method in &methods {
                    if let Err(e) = Self::run_discovery_method(
                        method,
                        &peers,
                        &event_sender,
                        &stats,
                    ).await {
                        warn!("Discovery method failed: {}", e);
                        let mut stats = stats.write().await;
                        stats.failed_discovery_attempts += 1;
                    }
                }

                let mut stats = stats.write().await;
                stats.last_discovery_run = Some(Instant::now());
            }
        });
    }

    async fn start_gossip_loop(&self) {
        let running = self.running.clone();
        let config = self.config.clone();
        let peers = self.peers.clone();
        let event_sender = self.event_sender.clone();
        let node_id = self.node_id;

        tokio::spawn(async move {
            let mut interval = interval(config.gossip_interval);

            // Initialize gossip manager
            use crate::gossip::{GossipManager, GossipConfig, GossipPeerInfo, PeerStatus as GossipStatus};

            let gossip_config = GossipConfig {
                fanout: 3,
                gossip_interval: config.gossip_interval,
                heartbeat_interval: Duration::from_secs(5),
                max_hops: 4,
                suspicion_timeout: Duration::from_secs(15),
                down_timeout: Duration::from_secs(60),
                use_push_pull: true,
                max_peers: config.max_peers,
            };

            let gossip_manager = GossipManager::new(node_id, gossip_config);

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                debug!("Running gossip cycle");

                // Get current peers and add to gossip manager
                let peers_guard = peers.read().await;
                let peer_count = peers_guard.len();

                if peer_count == 0 {
                    continue;
                }

                // Convert our peers to gossip format and sync
                for (peer_id, discovered_peer) in peers_guard.iter() {
                    let gossip_info = GossipPeerInfo {
                        peer_id: *peer_id,
                        addresses: discovered_peer.addresses.clone(),
                        services: discovered_peer.metadata.services.clone(),
                        protocol_version: discovered_peer.metadata.protocol_version,
                        region: discovered_peer.metadata.region.clone(),
                        status: GossipStatus::Healthy,
                        hop_count: 0,
                        last_updated: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    };
                    gossip_manager.add_peer(gossip_info).await;
                }
                drop(peers_guard);

                // Get healthy peers from gossip manager
                let healthy_peers = gossip_manager.get_healthy_peers().await;

                // Update our peer list with any new discoveries from gossip
                for gossip_peer in healthy_peers {
                    let mut peers_guard = peers.write().await;
                    if !peers_guard.contains_key(&gossip_peer.peer_id) {
                        let discovered = DiscoveredPeer {
                            peer_id: gossip_peer.peer_id,
                            addresses: gossip_peer.addresses,
                            metadata: PeerMetadata {
                                node_type: "secondary".to_string(),
                                protocol_version: gossip_peer.protocol_version,
                                services: gossip_peer.services,
                                region: gossip_peer.region,
                                extra: HashMap::new(),
                            },
                            discovered_at: Instant::now(),
                            last_seen: Some(Instant::now()),
                            reputation: 1.0,
                        };
                        peers_guard.insert(gossip_peer.peer_id, discovered.clone());
                        drop(peers_guard);

                        let _ = event_sender.send(DiscoveryEvent::PeerDiscovered(discovered));
                        debug!("Discovered peer {} via gossip sync", gossip_peer.peer_id);
                    }
                }

                debug!("Gossip cycle complete, tracking {} peers", peer_count);
            }
        });
    }

    async fn start_peer_maintenance(&self) {
        let running = self.running.clone();
        let config = self.config.clone();
        let peers = self.peers.clone();
        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                debug!("Running peer maintenance");

                let mut peers_to_remove = Vec::new();
                {
                    let peers_guard = peers.read().await;
                    let now = Instant::now();

                    for (peer_id, peer) in peers_guard.iter() {
                        if let Some(last_seen) = peer.last_seen {
                            if now.duration_since(last_seen) > config.peer_timeout {
                                peers_to_remove.push(*peer_id);
                            }
                        }
                    }
                }

                // Remove timed out peers
                if !peers_to_remove.is_empty() {
                    let mut peers_guard = peers.write().await;
                    for peer_id in &peers_to_remove {
                        peers_guard.remove(peer_id);
                        let _ = event_sender.send(DiscoveryEvent::PeerLost(*peer_id));
                    }
                    drop(peers_guard);

                    info!("Removed {} timed out peers", peers_to_remove.len());
                }
            }
        });
    }

    async fn run_discovery_method(
        method: &DiscoveryMethod,
        peers: &Arc<RwLock<HashMap<Uuid, DiscoveredPeer>>>,
        event_sender: &broadcast::Sender<DiscoveryEvent>,
        _stats: &Arc<RwLock<DiscoveryStats>>,
    ) -> Result<()> {
        match method {
            DiscoveryMethod::Bootstrap(addresses) => {
                Self::run_bootstrap_discovery(addresses, peers, event_sender).await
            }
            DiscoveryMethod::Mdns { service_name } => {
                Self::run_mdns_discovery(service_name, peers, event_sender).await
            }
            DiscoveryMethod::Dht { bootstrap_nodes } => {
                Self::run_dht_discovery(bootstrap_nodes, peers, event_sender).await
            }
            DiscoveryMethod::Gossip { fanout } => {
                Self::run_gossip_discovery(*fanout, peers, event_sender).await
            }
            DiscoveryMethod::Dns { domain, srv_record } => {
                Self::run_dns_discovery(domain, srv_record, peers, event_sender).await
            }
        }
    }

    async fn run_bootstrap_discovery(
        addresses: &[SocketAddr],
        peers: &Arc<RwLock<HashMap<Uuid, DiscoveredPeer>>>,
        event_sender: &broadcast::Sender<DiscoveryEvent>,
    ) -> Result<()> {
        debug!("Running bootstrap discovery with {} addresses", addresses.len());

        for &address in addresses {
            // In a real implementation, we would:
            // 1. Connect to the bootstrap address
            // 2. Request peer list
            // 3. Add discovered peers

            // For now, create a mock peer
            let peer = DiscoveredPeer {
                peer_id: Uuid::new_v4(),
                addresses: vec![address],
                metadata: PeerMetadata {
                    node_type: "secondary".to_string(),
                    protocol_version: 1,
                    services: ["consensus".to_string(), "storage".to_string()].into_iter().collect(),
                    region: Some("us-west-1".to_string()),
                    extra: HashMap::new(),
                },
                discovered_at: Instant::now(),
                last_seen: Some(Instant::now()),
                reputation: 1.0,
            };

            let mut peers_guard = peers.write().await;
            peers_guard.insert(peer.peer_id, peer.clone());
            drop(peers_guard);

            let _ = event_sender.send(DiscoveryEvent::PeerDiscovered(peer));
        }

        Ok(())
    }

    async fn run_mdns_discovery(
        _service_name: &str,
        _peers: &Arc<RwLock<HashMap<Uuid, DiscoveredPeer>>>,
        _event_sender: &broadcast::Sender<DiscoveryEvent>,
    ) -> Result<()> {
        debug!("mDNS discovery not implemented yet");
        Ok(())
    }

    async fn run_dht_discovery(
        _bootstrap_nodes: &[SocketAddr],
        _peers: &Arc<RwLock<HashMap<Uuid, DiscoveredPeer>>>,
        _event_sender: &broadcast::Sender<DiscoveryEvent>,
    ) -> Result<()> {
        debug!("DHT discovery not implemented yet");
        Ok(())
    }

    async fn run_gossip_discovery(
        fanout: usize,
        peers: &Arc<RwLock<HashMap<Uuid, DiscoveredPeer>>>,
        event_sender: &broadcast::Sender<DiscoveryEvent>,
    ) -> Result<()> {
        debug!("Running gossip discovery with fanout {}", fanout);

        let peers_guard = peers.read().await;
        let peer_list: Vec<_> = peers_guard.values().cloned().collect();
        drop(peers_guard);

        if peer_list.is_empty() {
            debug!("No peers available for gossip");
            return Ok(());
        }

        // Select random peers for gossip (up to fanout)
        let mut selected_indices: Vec<usize> = (0..peer_list.len()).collect();
        let mut gossip_targets = Vec::with_capacity(fanout.min(peer_list.len()));

        for _ in 0..fanout.min(peer_list.len()) {
            if selected_indices.is_empty() {
                break;
            }
            let idx = fastrand::usize(0..selected_indices.len());
            let peer_idx = selected_indices.swap_remove(idx);
            gossip_targets.push(peer_list[peer_idx].clone());
        }

        // Simulate gossip exchange - in real implementation this would:
        // 1. Send our peer list to selected targets
        // 2. Receive their peer lists
        // 3. Merge new peers into our list

        // For now, simulate discovering new peers through gossip
        for target in gossip_targets {
            debug!("Gossiping with peer {} at {:?}", target.peer_id, target.addresses);

            // Simulate receiving peer info from gossip target
            // In reality, this would be an RPC call
            let simulated_new_peer = DiscoveredPeer {
                peer_id: Uuid::new_v4(),
                addresses: vec![
                    format!("127.0.0.1:{}", 9000 + fastrand::u16(0..1000))
                        .parse()
                        .unwrap_or_else(|_| "127.0.0.1:9000".parse().unwrap()),
                ],
                metadata: PeerMetadata {
                    node_type: "secondary".to_string(),
                    protocol_version: 1,
                    services: ["query".to_string(), "storage".to_string()]
                        .into_iter()
                        .collect(),
                    region: target.metadata.region.clone(),
                    extra: HashMap::new(),
                },
                discovered_at: Instant::now(),
                last_seen: Some(Instant::now()),
                reputation: 0.8,
            };

            // Only add with some probability to simulate real gossip
            if fastrand::f32() < 0.3 {
                let mut peers_guard = peers.write().await;
                if !peers_guard.contains_key(&simulated_new_peer.peer_id) {
                    peers_guard.insert(simulated_new_peer.peer_id, simulated_new_peer.clone());
                    drop(peers_guard);
                    let _ = event_sender.send(DiscoveryEvent::PeerDiscovered(simulated_new_peer));
                    debug!("Discovered new peer via gossip");
                }
            }
        }

        Ok(())
    }

    async fn run_dns_discovery(
        _domain: &str,
        _srv_record: &str,
        _peers: &Arc<RwLock<HashMap<Uuid, DiscoveredPeer>>>,
        _event_sender: &broadcast::Sender<DiscoveryEvent>,
    ) -> Result<()> {
        debug!("DNS discovery not implemented yet");
        Ok(())
    }

    async fn calculate_ring_connections(&self, peers: &[DiscoveredPeer], redundancy: usize) -> Result<Vec<Uuid>> {
        // Sort peers by ID for consistent ring order
        let mut sorted_peers = peers.to_vec();
        sorted_peers.sort_by(|a, b| a.peer_id.cmp(&b.peer_id));

        // Find our position in the ring
        let self_pos = sorted_peers.iter().position(|p| p.peer_id == self.node_id);

        if let Some(pos) = self_pos {
            let mut connections = Vec::new();
            let peer_count = sorted_peers.len();

            // Connect to next `redundancy` peers in the ring
            for i in 1..=redundancy {
                let next_pos = (pos + i) % peer_count;
                connections.push(sorted_peers[next_pos].peer_id);
            }

            Ok(connections)
        } else {
            Ok(Vec::new())
        }
    }

    async fn calculate_small_world_connections(
        &self,
        peers: &[DiscoveredPeer],
        local_connections: usize,
        random_connections: usize,
    ) -> Result<Vec<Uuid>> {
        let mut connections = Vec::new();

        // Local connections (by region or proximity)
        let local_peers: Vec<_> = peers.iter()
            .filter(|p| {
                // Connect to peers in same region
                p.metadata.region.is_some() &&
                p.metadata.region == peers.iter().find(|peer| peer.peer_id == self.node_id)
                    .and_then(|our_peer| our_peer.metadata.region.as_ref()).cloned()
            })
            .take(local_connections)
            .collect();

        connections.extend(local_peers.iter().map(|p| p.peer_id));

        // Random connections for small world property
        let remaining_peers: Vec<_> = peers.iter()
            .filter(|p| !connections.contains(&p.peer_id))
            .collect();

        use fastrand;
        for _ in 0..random_connections.min(remaining_peers.len()) {
            let idx = fastrand::usize(0..remaining_peers.len());
            connections.push(remaining_peers[idx].peer_id);
        }

        Ok(connections)
    }

    async fn calculate_hierarchical_connections(&self, peers: &[DiscoveredPeer]) -> Result<Vec<Uuid>> {
        // Simplified hierarchical: connect to a subset based on node type
        let connections: Vec<Uuid> = peers.iter()
            .filter(|p| p.metadata.node_type == "primary")
            .take(3) // Connect to up to 3 primary nodes
            .map(|p| p.peer_id)
            .collect();

        Ok(connections)
    }
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            discovery_interval: Duration::from_secs(30),
            gossip_interval: Duration::from_secs(10),
            max_peers: 100,
            min_peers_for_formation: 3,
            peer_timeout: Duration::from_secs(300), // 5 minutes
            bootstrap_timeout: Duration::from_secs(30),
            auto_form_network: true,
        }
    }
}

// Serde helper modules for Instant serialization
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

mod option_instant_as_secs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(instant: &Option<Instant>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match instant {
            Some(_) => {
                let approx_duration = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(serde::ser::Error::custom)?;
                Some(approx_duration.as_secs()).serialize(serializer)
            }
            None => None::<u64>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Instant>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = Option::<u64>::deserialize(deserializer)?;
        Ok(secs.map(|_| Instant::now()))
    }
}