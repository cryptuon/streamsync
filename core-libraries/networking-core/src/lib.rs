//! # Networking Core Library
//!
//! A high-performance networking layer for distributed systems, providing
//! multiple transport implementations with consistent abstractions.
//!
//! ## Features
//!
//! - **Multiple Transports**: TCP, UDP, gRPC, and NNG support
//! - **Connection Management**: Automatic reconnection and health monitoring
//! - **Message Framing**: Efficient serialization and deserialization
//! - **Security**: TLS support and message authentication
//! - **Performance**: Zero-copy operations where possible
//! - **Observability**: Comprehensive metrics and logging
//!
//! ## Architecture
//!
//! The library is organized around these core abstractions:
//!
//! - `NetworkTransport`: Main transport trait for sending/receiving messages
//! - `ConnectionManager`: Manages peer connections and health
//! - `MessageCodec`: Handles message serialization/deserialization
//! - `SecurityProvider`: Manages encryption and authentication
//! - `NetworkConfig`: Configuration for network parameters
//!
//! ## Example
//!
//! ```rust,no_run
//! use networking_core::{NetworkTransport, TcpTransport, NetworkConfig};
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = NetworkConfig::default()
//!         .with_bind_address("127.0.0.1:8080".parse()?)
//!         .with_max_connections(100);
//!
//!     let mut transport = TcpTransport::new(config)?;
//!     transport.start().await?;
//!
//!     // Send message to peer
//!     use networking_core::NetworkMessage;
//!     let message = NetworkMessage::new(
//!         "127.0.0.1:8080".to_string(),
//!         Some("127.0.0.1:8081".to_string()),
//!         b"Hello, world!".to_vec(),
//!         "greeting".to_string()
//!     );
//!     transport.send_to("127.0.0.1:8081", message).await?;
//!
//!     // Receive messages
//!     let received_message = transport.receive().await?;
//!     println!("Received from {}: {:?}", received_message.source, received_message.payload);
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod transport;
pub mod connection;
pub mod codec;
pub mod security;
pub mod metrics;
pub mod discovery;
pub mod gossip;

pub use config::NetworkConfig;
pub use error::{NetworkError, Result};
pub use transport::{NetworkTransport, NetworkMessage, TcpTransport, UdpTransport, GrpcTransport};
pub use connection::{ConnectionManager, ConnectionId, PeerInfo};
pub use codec::{MessageCodec, BinaryCodec, JsonCodec};
pub use security::{SecurityProvider, TlsProvider};
pub use metrics::NetworkMetrics;
pub use discovery::{DiscoveryManager, DiscoveredPeer, DiscoveryMethod, NetworkTopology, DiscoveryEvent};
pub use gossip::{GossipManager, GossipConfig, GossipMessage, GossipPeerInfo, PeerStatus};

/// Re-export commonly used types
pub use std::net::SocketAddr;
pub use uuid::Uuid;