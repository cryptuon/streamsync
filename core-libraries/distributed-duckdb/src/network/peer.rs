//! Peer connection management using nng

use super::{NetworkConfig, protocol::NetworkMessage};
use anyhow::{Result, Context};
use nng::{Socket, Protocol, options::Options};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use uuid::Uuid;
use tracing::{debug, warn, error};

/// Represents a peer connection in the network
pub struct Peer {
    id: Uuid,
    addr: SocketAddr,
    socket: Arc<RwLock<Socket>>,
    status: Arc<RwLock<PeerStatus>>,
    message_tx: mpsc::UnboundedSender<NetworkMessage>,
    config: NetworkConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PeerStatus {
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: Uuid,
    pub addr: SocketAddr,
    pub status: String,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub version: u32,
    pub capabilities: Vec<String>,
}

impl Peer {
    /// Connect to a remote peer
    pub async fn connect(addr: SocketAddr, config: NetworkConfig) -> Result<Self> {
        let socket = Socket::new(Protocol::Req0)
            .context("Failed to create nng socket")?;

        // Configure socket options
        socket.set_opt::<nng::options::RecvTimeout>(Some(Duration::from_millis(config.connection_timeout_ms)))
            .context("Failed to set receive timeout")?;

        socket.set_opt::<nng::options::SendTimeout>(Some(Duration::from_millis(config.connection_timeout_ms)))
            .context("Failed to set send timeout")?;

        // Connect to the peer
        let connect_addr = format!("tcp://{}", addr);
        socket.dial(&connect_addr)
            .with_context(|| format!("Failed to connect to peer at {}", addr))?;

        let peer_id = Uuid::new_v4(); // In practice, this would be exchanged during handshake
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        let peer = Self {
            id: peer_id,
            addr,
            socket: Arc::new(RwLock::new(socket)),
            status: Arc::new(RwLock::new(PeerStatus::Connected)),
            message_tx,
            config,
        };

        // Start message handling task
        peer.start_message_handler(message_rx).await;

        // Start heartbeat task
        peer.start_heartbeat().await;

        debug!("✅ Connected to peer {} at {}", peer_id, addr);
        Ok(peer)
    }

    /// Create a peer from an incoming connection
    pub async fn from_incoming(socket: Socket, addr: SocketAddr, config: NetworkConfig) -> Result<Self> {
        let peer_id = Uuid::new_v4(); // Exchange during handshake
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        let peer = Self {
            id: peer_id,
            addr,
            socket: Arc::new(RwLock::new(socket)),
            status: Arc::new(RwLock::new(PeerStatus::Connected)),
            message_tx,
            config,
        };

        // Start message handling
        peer.start_message_handler(message_rx).await;
        peer.start_heartbeat().await;

        debug!("✅ Accepted incoming peer {} from {}", peer_id, addr);
        Ok(peer)
    }

    /// Get peer ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get peer address
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get current peer status
    pub async fn status(&self) -> PeerStatus {
        self.status.read().await.clone()
    }

    /// Send a message to this peer
    pub async fn send_message(&self, message: NetworkMessage) -> Result<()> {
        self.message_tx.send(message)
            .map_err(|e| anyhow::anyhow!("Failed to queue message: {}", e))?;
        Ok(())
    }

    /// Disconnect from this peer
    pub async fn disconnect(&self) -> Result<()> {
        *self.status.write().await = PeerStatus::Disconnecting;

        // Close the socket
        let socket = self.socket.read().await;
        socket.close();

        *self.status.write().await = PeerStatus::Disconnected;
        debug!("👋 Disconnected from peer {}", self.id);
        Ok(())
    }

    /// Get peer information
    pub async fn get_info(&self) -> PeerInfo {
        let status = self.status().await;

        PeerInfo {
            id: self.id,
            addr: self.addr,
            status: format!("{:?}", status),
            last_seen: chrono::Utc::now(),
            version: self.config.protocol_version,
            capabilities: vec![
                "zk-reconstruction".to_string(),
                "program-parsing".to_string(),
                "distributed-query".to_string(),
            ],
        }
    }

    /// Check if peer is healthy
    pub async fn is_healthy(&self) -> bool {
        matches!(self.status().await, PeerStatus::Connected)
    }

    // Private methods

    async fn start_message_handler(&self, mut message_rx: mpsc::UnboundedReceiver<NetworkMessage>) {
        let socket = self.socket.clone();
        let status = self.status.clone();
        let peer_id = self.id;

        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                // Check if we're still connected
                if !matches!(*status.read().await, PeerStatus::Connected) {
                    break;
                }

                // Serialize and send message
                match bincode::serialize(&message) {
                    Ok(data) => {
                        let socket = socket.read().await;
                        if let Err((_msg, err)) = socket.send(&data) {
                            error!("Failed to send message to peer {}: {:?}", peer_id, err);
                            *status.write().await = PeerStatus::Failed(format!("{:?}", err));
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize message for peer {}: {}", peer_id, e);
                    }
                }
            }
        });
    }

    async fn start_heartbeat(&self) {
        let message_tx = self.message_tx.clone();
        let status = self.status.clone();
        let peer_id = self.id;
        let heartbeat_interval = Duration::from_millis(self.config.heartbeat_interval_ms);

        tokio::spawn(async move {
            let mut interval = interval(heartbeat_interval);

            loop {
                interval.tick().await;

                // Check if we're still connected
                if !matches!(*status.read().await, PeerStatus::Connected) {
                    break;
                }

                // Send heartbeat
                let heartbeat = NetworkMessage::Heartbeat {
                    node_id: peer_id,
                    timestamp: chrono::Utc::now(),
                };

                if message_tx.send(heartbeat).is_err() {
                    warn!("Failed to send heartbeat to peer {}", peer_id);
                    break;
                }
            }

            debug!("Heartbeat task stopped for peer {}", peer_id);
        });
    }

    /// Start receiving messages from this peer
    pub async fn start_receiving<F>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(NetworkMessage) + Send + 'static,
    {
        let socket = self.socket.clone();
        let status = self.status.clone();
        let peer_id = self.id;

        tokio::spawn(async move {
            loop {
                // Check if we're still connected
                if !matches!(*status.read().await, PeerStatus::Connected) {
                    break;
                }

                // Receive message
                let socket = socket.read().await;
                match socket.recv() {
                    Ok(data) => {
                        match bincode::deserialize::<NetworkMessage>(&data) {
                            Ok(message) => {
                                handler(message);
                            }
                            Err(e) => {
                                warn!("Failed to deserialize message from peer {}: {}", peer_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to receive from peer {}: {}", peer_id, e);
                        *status.write().await = PeerStatus::Failed(e.to_string());
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}