//! Connection management and peer handling

use crate::{NetworkError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Unique identifier for a connection
pub type ConnectionId = Uuid;

/// Connection manager for handling peer connections
pub struct ConnectionManager {
    /// Active connections
    connections: Arc<DashMap<ConnectionId, Connection>>,
    /// Peer information indexed by address
    peers: Arc<DashMap<SocketAddr, PeerInfo>>,
    /// Connection statistics
    stats: Arc<ConnectionStats>,
    /// Configuration
    config: ConnectionConfig,
    /// Health checker
    health_checker: Option<HealthChecker>,
}

/// Individual connection information
#[derive(Debug, Clone)]
pub struct Connection {
    /// Unique connection identifier
    pub id: ConnectionId,
    /// Peer address
    pub peer_address: SocketAddr,
    /// Connection state
    pub state: ConnectionState,
    /// When the connection was established
    pub established_at: Instant,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Number of messages sent on this connection
    pub messages_sent: u64,
    /// Number of messages received on this connection
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Connection metadata
    pub metadata: ConnectionMetadata,
}

/// Connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connecting to peer
    Connecting,
    /// Connection established and active
    Connected,
    /// Connection is being closed
    Closing,
    /// Connection is closed
    Closed,
    /// Connection failed
    Failed(String),
}

/// Connection metadata
#[derive(Debug, Clone, Default)]
pub struct ConnectionMetadata {
    /// Peer protocol version
    pub protocol_version: Option<String>,
    /// Peer user agent
    pub user_agent: Option<String>,
    /// Additional key-value metadata
    pub custom: std::collections::HashMap<String, String>,
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer address
    pub address: SocketAddr,
    /// Peer identifier
    pub peer_id: Option<String>,
    /// When we first connected to this peer
    pub first_seen: Instant,
    /// Last successful connection
    pub last_connected: Option<Instant>,
    /// Number of successful connections
    pub connection_count: u64,
    /// Number of failed connection attempts
    pub failed_attempts: u64,
    /// Current connection status
    pub status: PeerStatus,
    /// Peer capabilities
    pub capabilities: Vec<String>,
    /// Average latency to this peer (in milliseconds)
    pub average_latency_ms: f64,
    /// Reliability score (0.0 to 1.0)
    pub reliability_score: f64,
}

/// Peer status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    /// Peer is reachable and responding
    Healthy,
    /// Peer is unreachable or not responding
    Unhealthy,
    /// Peer status is unknown
    Unknown,
    /// Peer is temporarily banned
    Banned { until: std::time::SystemTime },
}

/// Connection statistics
#[derive(Debug, Default)]
pub struct ConnectionStats {
    /// Total number of connections ever created
    pub total_connections: AtomicU64,
    /// Current number of active connections
    pub active_connections: AtomicU64,
    /// Total connection failures
    pub failed_connections: AtomicU64,
    /// Total bytes sent across all connections
    pub total_bytes_sent: AtomicU64,
    /// Total bytes received across all connections
    pub total_bytes_received: AtomicU64,
    /// Average connection duration
    pub average_connection_duration_ms: AtomicU64,
}

/// Connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    /// Maximum idle time before closing connection
    pub max_idle_time: Duration,
    /// Enable health checking
    pub health_check_enabled: bool,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Number of failed health checks before marking unhealthy
    pub health_check_failure_threshold: usize,
    /// Automatic reconnection enabled
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: usize,
    /// Initial reconnection delay
    pub reconnect_delay: Duration,
    /// Maximum reconnection delay
    pub max_reconnect_delay: Duration,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(60),
            max_idle_time: Duration::from_secs(300),
            health_check_enabled: true,
            health_check_interval: Duration::from_secs(30),
            health_check_failure_threshold: 3,
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_delay: Duration::from_secs(1),
            max_reconnect_delay: Duration::from_secs(30),
        }
    }
}

/// Health checker for monitoring connection health
pub struct HealthChecker {
    config: ConnectionConfig,
    connections: Arc<DashMap<ConnectionId, Connection>>,
    peers: Arc<DashMap<SocketAddr, PeerInfo>>,
    running: Arc<RwLock<bool>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(config: ConnectionConfig) -> Self {
        let connections = Arc::new(DashMap::new());
        let peers = Arc::new(DashMap::new());
        let stats = Arc::new(ConnectionStats::default());

        let health_checker = if config.health_check_enabled {
            Some(HealthChecker::new(
                config.clone(),
                connections.clone(),
                peers.clone(),
            ))
        } else {
            None
        };

        Self {
            connections,
            peers,
            stats,
            config,
            health_checker,
        }
    }

    /// Start the connection manager
    pub async fn start(&self) -> Result<()> {
        if let Some(health_checker) = &self.health_checker {
            health_checker.start().await?;
        }

        info!("Connection manager started");
        Ok(())
    }

    /// Stop the connection manager
    pub async fn stop(&self) -> Result<()> {
        if let Some(health_checker) = &self.health_checker {
            health_checker.stop().await?;
        }

        // Close all connections
        for mut connection in self.connections.iter_mut() {
            connection.state = ConnectionState::Closing;
        }

        info!("Connection manager stopped");
        Ok(())
    }

    /// Add a new connection
    pub async fn add_connection(&self, peer_address: SocketAddr) -> Result<ConnectionId> {
        // Check connection limits
        let active_count = self.stats.active_connections.load(Ordering::Relaxed) as usize;
        if active_count >= self.config.max_connections {
            return Err(NetworkError::ConnectionLimitExceeded {
                current: active_count,
                max: self.config.max_connections,
            });
        }

        let connection_id = ConnectionId::new_v4();
        let now = Instant::now();

        let connection = Connection {
            id: connection_id,
            peer_address,
            state: ConnectionState::Connecting,
            established_at: now,
            last_activity: now,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            metadata: ConnectionMetadata::default(),
        };

        self.connections.insert(connection_id, connection);

        // Update peer information
        self.peers.entry(peer_address).or_insert_with(|| PeerInfo {
            address: peer_address,
            peer_id: None,
            first_seen: now,
            last_connected: None,
            connection_count: 0,
            failed_attempts: 0,
            status: PeerStatus::Unknown,
            capabilities: Vec::new(),
            average_latency_ms: 0.0,
            reliability_score: 1.0,
        });

        // Update statistics
        self.stats.total_connections.fetch_add(1, Ordering::Relaxed);
        self.stats.active_connections.fetch_add(1, Ordering::Relaxed);

        debug!("Added connection {} to {}", connection_id, peer_address);
        Ok(connection_id)
    }

    /// Remove a connection
    pub async fn remove_connection(&self, connection_id: ConnectionId) -> Result<()> {
        if let Some((_, connection)) = self.connections.remove(&connection_id) {
            // Update peer statistics
            if let Some(mut peer) = self.peers.get_mut(&connection.peer_address) {
                if connection.state == ConnectionState::Connected {
                    peer.connection_count += 1;
                    peer.last_connected = Some(connection.established_at);
                } else {
                    peer.failed_attempts += 1;
                }

                // Update reliability score
                let total_attempts = peer.connection_count + peer.failed_attempts;
                if total_attempts > 0 {
                    peer.reliability_score = peer.connection_count as f64 / total_attempts as f64;
                }
            }

            // Update connection duration statistics
            let duration = connection.established_at.elapsed().as_millis() as u64;
            self.stats.average_connection_duration_ms.store(duration, Ordering::Relaxed);

            // Update active connection count
            self.stats.active_connections.fetch_sub(1, Ordering::Relaxed);

            debug!("Removed connection {} from {}", connection_id, connection.peer_address);
            Ok(())
        } else {
            Err(NetworkError::InvalidPeerAddress {
                address: format!("connection_id:{}", connection_id),
            })
        }
    }

    /// Update connection state
    pub async fn update_connection_state(&self, connection_id: ConnectionId, state: ConnectionState) -> Result<()> {
        if let Some(mut connection) = self.connections.get_mut(&connection_id) {
            let old_state = connection.state.clone();
            connection.state = state.clone();
            connection.last_activity = Instant::now();

            // Update peer status based on connection state
            if let Some(mut peer) = self.peers.get_mut(&connection.peer_address) {
                match state {
                    ConnectionState::Connected => {
                        peer.status = PeerStatus::Healthy;
                        if old_state == ConnectionState::Connecting {
                            peer.last_connected = Some(Instant::now());
                        }
                    },
                    ConnectionState::Failed(_) | ConnectionState::Closed => {
                        if matches!(peer.status, PeerStatus::Healthy) {
                            peer.status = PeerStatus::Unhealthy;
                        }
                    },
                    _ => {}
                }
            }

            debug!("Updated connection {} state: {:?} -> {:?}", connection_id, old_state, state);
            Ok(())
        } else {
            Err(NetworkError::InvalidPeerAddress {
                address: format!("connection_id:{}", connection_id),
            })
        }
    }

    /// Update connection activity
    pub async fn update_activity(&self, connection_id: ConnectionId, bytes_sent: u64, bytes_received: u64) -> Result<()> {
        if let Some(mut connection) = self.connections.get_mut(&connection_id) {
            connection.last_activity = Instant::now();
            connection.bytes_sent += bytes_sent;
            connection.bytes_received += bytes_received;

            if bytes_sent > 0 {
                connection.messages_sent += 1;
            }
            if bytes_received > 0 {
                connection.messages_received += 1;
            }

            // Update global statistics
            if bytes_sent > 0 {
                self.stats.total_bytes_sent.fetch_add(bytes_sent, Ordering::Relaxed);
            }
            if bytes_received > 0 {
                self.stats.total_bytes_received.fetch_add(bytes_received, Ordering::Relaxed);
            }

            Ok(())
        } else {
            Err(NetworkError::InvalidPeerAddress {
                address: format!("connection_id:{}", connection_id),
            })
        }
    }

    /// Get connection information
    pub fn get_connection(&self, connection_id: ConnectionId) -> Option<Connection> {
        self.connections.get(&connection_id).map(|c| c.clone())
    }

    /// Get all active connections
    pub fn get_active_connections(&self) -> Vec<Connection> {
        self.connections
            .iter()
            .filter(|c| c.state == ConnectionState::Connected)
            .map(|c| c.clone())
            .collect()
    }

    /// Get peer information
    pub fn get_peer(&self, address: SocketAddr) -> Option<PeerInfo> {
        self.peers.get(&address).map(|p| p.clone())
    }

    /// Get all known peers
    pub fn get_all_peers(&self) -> Vec<PeerInfo> {
        self.peers.iter().map(|p| p.clone()).collect()
    }

    /// Get healthy peers
    pub fn get_healthy_peers(&self) -> Vec<PeerInfo> {
        self.peers
            .iter()
            .filter(|p| p.status == PeerStatus::Healthy)
            .map(|p| p.clone())
            .collect()
    }

    /// Ban a peer temporarily
    pub async fn ban_peer(&self, address: SocketAddr, duration: Duration) -> Result<()> {
        if let Some(mut peer) = self.peers.get_mut(&address) {
            let ban_until = std::time::SystemTime::now() + duration;
            peer.status = PeerStatus::Banned { until: ban_until };

            info!("Banned peer {} for {:?}", address, duration);
            Ok(())
        } else {
            Err(NetworkError::InvalidPeerAddress {
                address: address.to_string(),
            })
        }
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> ConnectionStats {
        ConnectionStats {
            total_connections: AtomicU64::new(self.stats.total_connections.load(Ordering::Relaxed)),
            active_connections: AtomicU64::new(self.stats.active_connections.load(Ordering::Relaxed)),
            failed_connections: AtomicU64::new(self.stats.failed_connections.load(Ordering::Relaxed)),
            total_bytes_sent: AtomicU64::new(self.stats.total_bytes_sent.load(Ordering::Relaxed)),
            total_bytes_received: AtomicU64::new(self.stats.total_bytes_received.load(Ordering::Relaxed)),
            average_connection_duration_ms: AtomicU64::new(self.stats.average_connection_duration_ms.load(Ordering::Relaxed)),
        }
    }
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(
        config: ConnectionConfig,
        connections: Arc<DashMap<ConnectionId, Connection>>,
        peers: Arc<DashMap<SocketAddr, PeerInfo>>,
    ) -> Self {
        Self {
            config,
            connections,
            peers,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start health checking
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;

        let connections = self.connections.clone();
        let peers = self.peers.clone();
        let running_flag = self.running.clone();
        let check_interval = self.config.health_check_interval;
        let max_idle = self.config.max_idle_time;

        tokio::spawn(async move {
            while *running_flag.read().await {
                let now = Instant::now();

                // Check connection health
                for mut connection in connections.iter_mut() {
                    let idle_time = now.duration_since(connection.last_activity);

                    if idle_time > max_idle && connection.state == ConnectionState::Connected {
                        warn!("Connection {} to {} is idle for {:?}, marking as unhealthy",
                              connection.id, connection.peer_address, idle_time);
                        connection.state = ConnectionState::Closing;

                        // Update peer status
                        if let Some(mut peer) = peers.get_mut(&connection.peer_address) {
                            peer.status = PeerStatus::Unhealthy;
                        }
                    }
                }

                // Check peer bans
                for mut peer in peers.iter_mut() {
                    if let PeerStatus::Banned { until } = peer.status {
                        if std::time::SystemTime::now() > until {
                            peer.status = PeerStatus::Unknown;
                            info!("Unbanned peer {}", peer.address);
                        }
                    }
                }

                tokio::time::sleep(check_interval).await;
            }
        });

        info!("Health checker started");
        Ok(())
    }

    /// Stop health checking
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        info!("Health checker stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);
        assert_eq!(manager.get_active_connections().len(), 0);
    }

    #[tokio::test]
    async fn test_add_remove_connection() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        // Add connection
        let conn_id = manager.add_connection(address).await.unwrap();
        // New connections start in Connecting state, not Connected
        assert_eq!(manager.get_active_connections().len(), 0);

        // Update to connected state
        manager.update_connection_state(conn_id, ConnectionState::Connected).await.unwrap();
        // Now should have 1 active connection
        assert_eq!(manager.get_active_connections().len(), 1);

        let connection = manager.get_connection(conn_id).unwrap();
        assert_eq!(connection.state, ConnectionState::Connected);

        // Remove connection
        manager.remove_connection(conn_id).await.unwrap();
        assert_eq!(manager.get_active_connections().len(), 0);
    }

    #[tokio::test]
    async fn test_connection_limits() {
        let mut config = ConnectionConfig::default();
        config.max_connections = 2;
        let manager = ConnectionManager::new(config);

        let address1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let address2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);
        let address3 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082);

        // Add two connections (should succeed)
        assert!(manager.add_connection(address1).await.is_ok());
        assert!(manager.add_connection(address2).await.is_ok());

        // Try to add third connection (should fail)
        assert!(manager.add_connection(address3).await.is_err());
    }

    #[tokio::test]
    async fn test_peer_management() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        // Add connection to create peer
        let conn_id = manager.add_connection(address).await.unwrap();

        // Check peer was created
        let peer = manager.get_peer(address).unwrap();
        assert_eq!(peer.address, address);
        assert_eq!(peer.status, PeerStatus::Unknown);

        // Update connection state
        manager.update_connection_state(conn_id, ConnectionState::Connected).await.unwrap();

        // Check peer status updated
        let peer = manager.get_peer(address).unwrap();
        assert_eq!(peer.status, PeerStatus::Healthy);
    }

    #[tokio::test]
    async fn test_activity_tracking() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let conn_id = manager.add_connection(address).await.unwrap();

        // Update activity
        manager.update_activity(conn_id, 100, 200).await.unwrap();

        let connection = manager.get_connection(conn_id).unwrap();
        assert_eq!(connection.bytes_sent, 100);
        assert_eq!(connection.bytes_received, 200);
        assert_eq!(connection.messages_sent, 1);
        assert_eq!(connection.messages_received, 1);
    }

    #[tokio::test]
    async fn test_peer_banning() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        // Add connection to create peer
        manager.add_connection(address).await.unwrap();

        // Ban peer
        manager.ban_peer(address, Duration::from_millis(100)).await.unwrap();

        let peer = manager.get_peer(address).unwrap();
        assert!(matches!(peer.status, PeerStatus::Banned { .. }));
    }
}