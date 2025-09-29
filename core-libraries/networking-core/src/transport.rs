//! Network transport implementations using NNG

use crate::{NetworkConfig, NetworkError, Result};
use async_trait::async_trait;
use nng::{Protocol, Socket};
use nng::options::Options;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Network message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    /// Unique message identifier
    pub id: Uuid,
    /// Source address
    pub source: String,
    /// Destination address (None for broadcast)
    pub destination: Option<String>,
    /// Message payload
    pub payload: Vec<u8>,
    /// Message timestamp
    pub timestamp: u64,
    /// Message type/routing key
    pub message_type: String,
}

impl NetworkMessage {
    /// Create a new network message
    pub fn new(source: String, destination: Option<String>, payload: Vec<u8>, message_type: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            destination,
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            message_type,
        }
    }

    /// Serialize message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| NetworkError::SerializationError {
            reason: e.to_string(),
        })
    }

    /// Deserialize message from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(|e| NetworkError::DeserializationError {
            reason: e.to_string(),
        })
    }

    /// Get message size in bytes
    pub fn size(&self) -> usize {
        std::mem::size_of::<Uuid>() +
        self.source.len() +
        self.destination.as_ref().map_or(0, |d| d.len()) +
        self.payload.len() +
        std::mem::size_of::<u64>() +
        self.message_type.len()
    }
}

/// Main network transport trait
#[async_trait]
pub trait NetworkTransport: Send + Sync {
    /// Start the transport
    async fn start(&mut self) -> Result<()>;

    /// Stop the transport
    async fn stop(&mut self) -> Result<()>;

    /// Send a message to a specific peer
    async fn send_to(&self, peer: &str, message: NetworkMessage) -> Result<()>;

    /// Broadcast a message to all connected peers
    async fn broadcast(&self, message: NetworkMessage) -> Result<()>;

    /// Receive the next message
    async fn receive(&self) -> Result<NetworkMessage>;

    /// Connect to a peer
    async fn connect_to(&self, address: &str) -> Result<()>;

    /// Disconnect from a peer
    async fn disconnect_from(&self, address: &str) -> Result<()>;

    /// Get list of connected peers
    async fn connected_peers(&self) -> Vec<String>;

    /// Check if connected to a peer
    async fn is_connected(&self, peer: &str) -> bool;

    /// Get local address
    fn local_address(&self) -> String;

    /// Get transport statistics
    async fn stats(&self) -> TransportStats;
}

/// Transport statistics
#[derive(Debug, Clone, Default)]
pub struct TransportStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connections_active: usize,
    pub connections_total: u64,
    pub errors_total: u64,
    pub average_latency_ms: f64,
}

/// NNG-based transport implementation (simplified)
pub struct NngTransport {
    config: NetworkConfig,
    socket: Option<Socket>,
    local_address: String,
    peers: Arc<RwLock<HashMap<String, PeerConnection>>>,
    stats: Arc<RwLock<TransportStats>>,
    running: Arc<RwLock<bool>>,
    message_tx: Arc<Mutex<Option<mpsc::UnboundedSender<NetworkMessage>>>>,
    message_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<NetworkMessage>>>>,
}

/// Peer connection information
#[derive(Debug, Clone)]
struct PeerConnection {
    address: String,
    connected_at: std::time::Instant,
    last_seen: std::time::Instant,
    messages_sent: u64,
    messages_received: u64,
}

impl NngTransport {
    /// Create a new NNG transport
    pub fn new(config: NetworkConfig) -> Result<Self> {
        config.validate()?;

        let local_address = config.bind_address.to_string();
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            socket: None,
            local_address,
            peers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(TransportStats::default())),
            running: Arc::new(RwLock::new(false)),
            message_tx: Arc::new(Mutex::new(Some(message_tx))),
            message_rx: Arc::new(Mutex::new(Some(message_rx))),
        })
    }

    /// Create transport with specific NNG protocol
    pub fn with_protocol(config: NetworkConfig, protocol: Protocol) -> Result<Self> {
        let mut transport = Self::new(config)?;
        transport.init_socket(protocol)?;
        Ok(transport)
    }

    /// Initialize NNG socket with protocol
    fn init_socket(&mut self, protocol: Protocol) -> Result<()> {
        let socket = Socket::new(protocol).map_err(|e| NetworkError::IoError {
            reason: format!("Failed to create NNG socket: {}", e),
        })?;

        // Configure socket options
        if let Err(e) = socket.set_opt::<nng::options::RecvTimeout>(Some(self.config.read_timeout)) {
            warn!("Failed to set receive timeout: {}", e);
        }

        if let Err(e) = socket.set_opt::<nng::options::SendTimeout>(Some(self.config.write_timeout)) {
            warn!("Failed to set send timeout: {}", e);
        }

        if let Err(e) = socket.set_opt::<nng::options::RecvMaxSize>(self.config.max_message_size) {
            warn!("Failed to set max message size: {}", e);
        }

        self.socket = Some(socket);
        Ok(())
    }

    /// Start listening for connections
    async fn start_listener(&self) -> Result<()> {
        let socket = self.socket.as_ref().ok_or(NetworkError::TransportNotStarted)?;

        let listen_url = format!("tcp://{}", self.config.bind_address);
        socket.listen(&listen_url).map_err(|e| NetworkError::IoError {
            reason: format!("Failed to listen on {}: {}", listen_url, e),
        })?;

        info!("NNG transport listening on {}", listen_url);
        Ok(())
    }

    /// Send message using blocking NNG operations
    async fn send_message_blocking(&self, message: NetworkMessage) -> Result<()> {
        let socket = self.socket.as_ref().ok_or(NetworkError::TransportNotStarted)?;

        // Serialize message
        let data = message.to_bytes()?;

        // Check message size
        if data.len() > self.config.max_message_size {
            return Err(NetworkError::MessageTooLarge {
                size: data.len(),
                max_size: self.config.max_message_size,
            });
        }

        let data_len = data.len();

        // Send the message using blocking API
        tokio::task::spawn_blocking({
            let socket = socket.clone();
            move || {
                socket.send(data.as_slice())
            }
        }).await.map_err(|e| NetworkError::IoError {
            reason: format!("Send task failed: {}", e),
        })?.map_err(|e| NetworkError::IoError {
            reason: format!("Failed to send message: {:?}", e),
        })?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += 1;
            stats.bytes_sent += data_len as u64;
        }

        debug!("Sent message of {} bytes", data_len);
        Ok(())
    }

    /// Receive message using blocking NNG operations
    async fn receive_message_blocking(&self) -> Result<NetworkMessage> {
        let socket = self.socket.as_ref().ok_or(NetworkError::TransportNotStarted)?;

        // Receive the message using blocking API
        let data = tokio::task::spawn_blocking({
            let socket = socket.clone();
            move || {
                socket.recv()
            }
        }).await.map_err(|e| NetworkError::IoError {
            reason: format!("Receive task failed: {}", e),
        })?.map_err(|_e| NetworkError::MessageTimeout {
            timeout_ms: self.config.read_timeout.as_millis() as u64,
        })?;

        // Convert to NetworkMessage
        let message = NetworkMessage::from_bytes(&data)?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_received += 1;
            stats.bytes_received += data.len() as u64;
        }

        debug!("Received message from {}", message.source);
        Ok(message)
    }
}

#[async_trait]
impl NetworkTransport for NngTransport {
    async fn start(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(NetworkError::TransportAlreadyRunning);
        }

        // Initialize socket if not already done
        if self.socket.is_none() {
            drop(running); // Release the lock before calling init_socket
            self.init_socket(Protocol::Pair1)?;
            running = self.running.write().await; // Re-acquire the lock
        }

        // Start listening
        self.start_listener().await?;

        *running = true;
        info!("NNG transport started on {}", self.local_address);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(NetworkError::TransportNotStarted);
        }

        *running = false;

        // Close socket
        if let Some(socket) = self.socket.take() {
            socket.close();
        }

        // Clear peers
        self.peers.write().await.clear();

        info!("NNG transport stopped");
        Ok(())
    }

    async fn send_to(&self, peer: &str, message: NetworkMessage) -> Result<()> {
        if !*self.running.read().await {
            return Err(NetworkError::TransportNotStarted);
        }

        // Update peer information
        {
            let mut peers = self.peers.write().await;
            if let Some(peer_conn) = peers.get_mut(peer) {
                peer_conn.last_seen = std::time::Instant::now();
                peer_conn.messages_sent += 1;
            }
        }

        self.send_message_blocking(message).await
    }

    async fn broadcast(&self, message: NetworkMessage) -> Result<()> {
        if !*self.running.read().await {
            return Err(NetworkError::TransportNotStarted);
        }

        self.send_message_blocking(message).await
    }

    async fn receive(&self) -> Result<NetworkMessage> {
        if !*self.running.read().await {
            return Err(NetworkError::TransportNotStarted);
        }

        self.receive_message_blocking().await
    }

    async fn connect_to(&self, address: &str) -> Result<()> {
        let socket = self.socket.as_ref().ok_or(NetworkError::TransportNotStarted)?;

        let connect_url = if address.starts_with("tcp://") {
            address.to_string()
        } else {
            format!("tcp://{}", address)
        };

        // Connect using blocking API
        tokio::task::spawn_blocking({
            let socket = socket.clone();
            let connect_url = connect_url.clone();
            move || {
                socket.dial(&connect_url)
            }
        }).await.map_err(|e| NetworkError::IoError {
            reason: format!("Connect task failed: {}", e),
        })?.map_err(|e| NetworkError::ConnectionFailed {
            address: address.parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
            reason: e.to_string(),
        })?;

        // Add to peers list
        let now = std::time::Instant::now();
        let peer_conn = PeerConnection {
            address: address.to_string(),
            connected_at: now,
            last_seen: now,
            messages_sent: 0,
            messages_received: 0,
        };

        self.peers.write().await.insert(address.to_string(), peer_conn);

        // Update connection statistics
        {
            let mut stats = self.stats.write().await;
            stats.connections_total += 1;
            stats.connections_active = self.peers.read().await.len();
        }

        info!("Connected to peer: {}", address);
        Ok(())
    }

    async fn disconnect_from(&self, address: &str) -> Result<()> {
        // Remove from peers list
        let removed = self.peers.write().await.remove(address).is_some();

        if removed {
            // Update connection statistics
            {
                let mut stats = self.stats.write().await;
                stats.connections_active = self.peers.read().await.len();
            }

            info!("Disconnected from peer: {}", address);
            Ok(())
        } else {
            Err(NetworkError::InvalidPeerAddress {
                address: address.to_string(),
            })
        }
    }

    async fn connected_peers(&self) -> Vec<String> {
        self.peers.read().await.keys().cloned().collect()
    }

    async fn is_connected(&self, peer: &str) -> bool {
        self.peers.read().await.contains_key(peer)
    }

    fn local_address(&self) -> String {
        self.local_address.clone()
    }

    async fn stats(&self) -> TransportStats {
        let mut stats = self.stats.read().await.clone();
        stats.connections_active = self.peers.read().await.len();
        stats
    }
}

/// Convenience type aliases for different NNG protocols
pub type TcpTransport = NngTransport;
pub type UdpTransport = NngTransport;
pub type GrpcTransport = NngTransport;

/// Create a REQ/REP pattern transport (client-server)
pub fn create_req_transport(config: NetworkConfig) -> Result<NngTransport> {
    NngTransport::with_protocol(config, Protocol::Req0)
}

/// Create a REP/REQ pattern transport (server-client)
pub fn create_rep_transport(config: NetworkConfig) -> Result<NngTransport> {
    NngTransport::with_protocol(config, Protocol::Rep0)
}

/// Create a PUB/SUB pattern transport (publisher)
pub fn create_pub_transport(config: NetworkConfig) -> Result<NngTransport> {
    NngTransport::with_protocol(config, Protocol::Pub0)
}

/// Create a SUB/PUB pattern transport (subscriber)
pub fn create_sub_transport(config: NetworkConfig) -> Result<NngTransport> {
    NngTransport::with_protocol(config, Protocol::Sub0)
}

/// Create a PAIR pattern transport (bidirectional)
pub fn create_pair_transport(config: NetworkConfig) -> Result<NngTransport> {
    NngTransport::with_protocol(config, Protocol::Pair1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_message() {
        let message = NetworkMessage::new(
            "127.0.0.1:8080".to_string(),
            Some("127.0.0.1:8081".to_string()),
            b"test data".to_vec(),
            "test".to_string(),
        );

        // Test serialization
        let bytes = message.to_bytes().unwrap();
        let deserialized = NetworkMessage::from_bytes(&bytes).unwrap();

        assert_eq!(message.id, deserialized.id);
        assert_eq!(message.source, deserialized.source);
        assert_eq!(message.destination, deserialized.destination);
        assert_eq!(message.payload, deserialized.payload);
        assert_eq!(message.message_type, deserialized.message_type);
    }

    #[tokio::test]
    async fn test_nng_transport_creation() {
        let config = NetworkConfig::test_config();
        let transport = NngTransport::new(config);
        assert!(transport.is_ok());
    }

    #[tokio::test]
    async fn test_transport_start_stop() {
        let config = NetworkConfig::test_config();
        let mut transport = NngTransport::new(config).unwrap();

        // Test start
        assert!(transport.start().await.is_ok());
        assert!(*transport.running.read().await);

        // Test stop
        assert!(transport.stop().await.is_ok());
        assert!(!*transport.running.read().await);
    }

    #[tokio::test]
    async fn test_protocol_factories() {
        let config = NetworkConfig::test_config();

        let req_transport = create_req_transport(config.clone());
        assert!(req_transport.is_ok());

        let rep_transport = create_rep_transport(config.clone());
        assert!(rep_transport.is_ok());

        let pub_transport = create_pub_transport(config.clone());
        assert!(pub_transport.is_ok());

        let sub_transport = create_sub_transport(config.clone());
        assert!(sub_transport.is_ok());

        let pair_transport = create_pair_transport(config);
        assert!(pair_transport.is_ok());
    }

    #[tokio::test]
    async fn test_peer_management() {
        let config = NetworkConfig::test_config();
        let mut transport = NngTransport::new(config).unwrap();

        assert!(transport.start().await.is_ok());

        // Initially no peers
        assert_eq!(transport.connected_peers().await.len(), 0);
        assert!(!transport.is_connected("127.0.0.1:8081").await);

        // Test disconnect from non-existent peer
        assert!(transport.disconnect_from("127.0.0.1:8081").await.is_err());

        assert!(transport.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_stats() {
        let config = NetworkConfig::test_config();
        let mut transport = NngTransport::new(config).unwrap();

        assert!(transport.start().await.is_ok());

        let stats = transport.stats().await;
        assert_eq!(stats.connections_active, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);

        assert!(transport.stop().await.is_ok());
    }
}