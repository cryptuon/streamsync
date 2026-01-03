//! # Consensus Core Library
//!
//! A production-ready Byzantine fault tolerant consensus library implementing
//! the PBFT (Practical Byzantine Fault Tolerance) algorithm.
//!
//! ## Features
//!
//! - **Byzantine Fault Tolerance**: Supports up to f < n/3 malicious nodes
//! - **High Performance**: Sub-100ms consensus latency
//! - **View Changes**: Automatic leader failure handling
//! - **Checkpointing**: Efficient state management and garbage collection
//! - **Pluggable Transport**: Abstract network layer for flexibility
//! - **Comprehensive Testing**: Unit, integration, and property-based tests
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use consensus_core::{ConsensusEngine, Config, Proposal};
//! use consensus_core::transport::MockTransport;
//! use uuid::Uuid;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let node_id = Uuid::new_v4();
//! let participants = vec![node_id, Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
//!
//! let config = Config {
//!     node_id,
//!     participants: participants.clone(),
//!     ..Default::default()
//! };
//!
//! let transport = MockTransport::new(node_id, participants);
//! let engine = ConsensusEngine::new(config, transport).await?;
//!
//! // Propose a value
//! let proposal = Proposal::new("example".to_string(), b"data".to_vec());
//! let result = engine.propose(proposal).await?;
//!
//! println!("Consensus reached: {:?}", result);
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod engine;
pub mod messages;
pub mod state;
pub mod transport;
pub mod types;
pub mod error;

// Re-export main types
pub use config::Config;
pub use engine::ConsensusEngine;
pub use messages::{ConsensusMessage, MessageType};
pub use types::{Proposal, ConsensusResult, NodeId, SequenceNumber, ViewNumber};
pub use error::{ConsensusError, Result};
pub use transport::{Transport, Message};

/// Current version of the consensus protocol
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum number of participants supported
pub const MAX_PARTICIPANTS: usize = 100;

/// Default request timeout in milliseconds
pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 5000;

/// Default view change timeout in milliseconds
pub const DEFAULT_VIEW_CHANGE_TIMEOUT_MS: u64 = 10000;