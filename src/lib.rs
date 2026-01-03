//! StreamSync Node Library
//!
//! Core modules for the StreamSync distributed Solana transaction indexing node.
//!
//! ## Architecture
//!
//! StreamSync is a decentralized Solana transaction indexing network with:
//! - Distributed query processing with racing competition
//! - ZK-based account reconstruction for compressed data
//! - Economic incentives via $STRM token staking and rewards
//! - Node specializations for different query types

pub mod node;
pub mod config;
pub mod consensus;
pub mod query_router;
pub mod economics;
pub mod gateway;
pub mod light_client;
pub mod rate_limiter;
pub mod revenue_sharing;
pub mod wallet_manager;
pub mod settlement;
pub mod node_specializations;

pub use node::StreamSyncNode;
pub use config::NodeConfig;
pub use settlement::{SettlementEngine, SettlementConfig};
pub use node_specializations::{NodeSpecialization, SpecializationConfig, NodeCapabilities};
