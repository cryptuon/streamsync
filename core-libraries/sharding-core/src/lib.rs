//! # Sharding Core Library
//!
//! A production-ready sharding library implementing consistent hashing for distributed systems.
//! This library provides efficient data distribution, automatic rebalancing, and fault tolerance
//! for large-scale distributed applications.
//!
//! ## Features
//!
//! - **Consistent Hashing**: Virtual nodes for uniform data distribution
//! - **Dynamic Rebalancing**: Automatic redistribution when nodes join/leave
//! - **Replication**: Configurable replication factor for fault tolerance
//! - **Virtual Nodes**: Multiple virtual nodes per physical node for better distribution
//! - **Pluggable Hash Functions**: Support for different hash algorithms
//! - **Metrics & Monitoring**: Built-in metrics for monitoring shard health
//! - **Serializable State**: Full state serialization for persistence and recovery
//!
//! ## Architecture
//!
//! The library is built around several core abstractions:
//! - `ShardManager`: Main coordinator for shard operations
//! - `ConsistentHashRing`: Consistent hashing implementation with virtual nodes
//! - `ShardMigrator`: Handles data migration during rebalancing
//! - `ReplicationManager`: Manages data replication across nodes
//!
//! ## Example
//!
//! ```rust,no_run
//! use sharding_core::{ShardManager, ShardConfig, NodeId};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ShardConfig::builder()
//!         .virtual_nodes(150)
//!         .replication_factor(3)
//!         .migration_timeout_ms(30000)
//!         .build()?;
//!
//!     let mut shard_manager = ShardManager::new(config);
//!
//!     // Add nodes to the cluster
//!     let node1 = NodeId::new("node-1");
//!     let node2 = NodeId::new("node-2");
//!     shard_manager.add_node(node1, "192.168.1.10:8080".parse()?).await?;
//!     shard_manager.add_node(node2, "192.168.1.11:8080".parse()?).await?;
//!
//!     // Get responsible nodes for a key
//!     let key = "user:12345";
//!     let nodes = shard_manager.get_responsible_nodes(key).await;
//!     println!("Key '{}' is handled by nodes: {:?}", key, nodes);
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod hash_ring;
pub mod manager;
pub mod migration;
pub mod replication;
pub mod metrics;
pub mod node;

pub use config::ShardConfig;
pub use error::{ShardError, Result};
pub use hash_ring::{ConsistentHashRing, HashFunction};
pub use manager::ShardManager;
pub use migration::{ShardMigrator, MigrationPlan};
pub use replication::ReplicationManager;
pub use metrics::ShardMetrics;
pub use node::{NodeId, NodeInfo, NodeStatus};

/// Re-export commonly used types
pub use std::net::SocketAddr;
pub use uuid::Uuid;