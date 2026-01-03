//! PBFT Consensus Implementation for StreamSync
//!
//! This module implements Practical Byzantine Fault Tolerance (PBFT) consensus
//! for coordinating distributed operations across the StreamSync network.
//! It ensures consistency and fault tolerance in the presence of up to f Byzantine nodes
//! out of 3f+1 total nodes.

pub mod pbft;
pub mod state;
pub mod message_log;
pub mod view_change;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Node ID of this consensus participant
    pub node_id: Uuid,
    /// List of all consensus participants
    pub participants: Vec<Uuid>,
    /// Maximum Byzantine faults tolerated (f)
    pub max_faults: usize,
    /// View change timeout in milliseconds
    pub view_change_timeout_ms: u64,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Checkpoint interval (number of operations)
    pub checkpoint_interval: u64,
    /// Maximum sequence number gap before triggering state transfer
    pub max_sequence_gap: u64,
}

impl ConsensusConfig {
    /// Create a new consensus configuration
    pub fn new(node_id: Uuid, participants: Vec<Uuid>) -> Self {
        let n = participants.len();
        let f = (n - 1) / 3; // Maximum Byzantine faults: f = floor((n-1)/3)

        Self {
            node_id,
            participants,
            max_faults: f,
            view_change_timeout_ms: 10000,
            request_timeout_ms: 5000,
            checkpoint_interval: 100,
            max_sequence_gap: 50,
        }
    }

    /// Check if we have enough nodes for consensus
    pub fn is_valid(&self) -> bool {
        self.participants.len() >= 3 * self.max_faults + 1
    }

    /// Get the primary node for a given view
    pub fn primary_for_view(&self, view: u64) -> Option<Uuid> {
        let index = (view as usize) % self.participants.len();
        self.participants.get(index).copied()
    }

    /// Check if this node is the primary for a given view
    pub fn is_primary(&self, view: u64) -> bool {
        self.primary_for_view(view) == Some(self.node_id)
    }

    /// Get quorum size (2f + 1)
    pub fn quorum_size(&self) -> usize {
        2 * self.max_faults + 1
    }
}

/// Consensus proposal that nodes agree on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusProposal {
    /// IDL update proposal
    IdlUpdate {
        program_id: String,
        new_idl: String,
        proposer: Uuid,
        timestamp: DateTime<Utc>,
    },

    /// Data shard assignment proposal
    ShardAssignment {
        shard_id: String,
        assigned_nodes: Vec<Uuid>,
        replication_factor: u32,
        timestamp: DateTime<Utc>,
    },

    /// Node management proposal (add/remove nodes)
    NodeManagement {
        action: NodeAction,
        target_node: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    /// Configuration update proposal
    ConfigUpdate {
        parameter: String,
        old_value: String,
        new_value: String,
        timestamp: DateTime<Utc>,
    },

    /// Emergency network action
    EmergencyAction {
        action_type: EmergencyActionType,
        details: String,
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeAction {
    Add,
    Remove,
    Suspend,
    Reactivate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmergencyActionType {
    NetworkHalt,
    ForceViewChange,
    StateReset,
}

/// Result of consensus operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub sequence: u64,
    pub view: u64,
    pub proposal: ConsensusProposal,
    pub committed_at: DateTime<Utc>,
    pub participating_nodes: Vec<Uuid>,
}

/// Consensus error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusError {
    InsufficientNodes,
    ViewChangeFailed,
    TimeoutExpired,
    InvalidProposal,
    NodeNotInView,
    SequenceGap,
    StateTransferFailed,
    CryptographicError,
}

impl std::fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusError::InsufficientNodes => write!(f, "Insufficient nodes for consensus"),
            ConsensusError::ViewChangeFailed => write!(f, "View change failed"),
            ConsensusError::TimeoutExpired => write!(f, "Consensus timeout expired"),
            ConsensusError::InvalidProposal => write!(f, "Invalid consensus proposal"),
            ConsensusError::NodeNotInView => write!(f, "Node not participating in current view"),
            ConsensusError::SequenceGap => write!(f, "Sequence number gap detected"),
            ConsensusError::StateTransferFailed => write!(f, "State transfer failed"),
            ConsensusError::CryptographicError => write!(f, "Cryptographic verification failed"),
        }
    }
}

impl std::error::Error for ConsensusError {}

/// Consensus statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStats {
    pub current_view: u64,
    pub current_sequence: u64,
    pub total_committed: u64,
    pub view_changes: u64,
    pub failed_proposals: u64,
    pub average_commit_time_ms: f64,
    pub participating_nodes: usize,
    pub last_checkpoint: u64,
    pub uptime_seconds: u64,
}

/// Consensus node role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Primary,
    Backup,
}

/// Re-export main consensus engine
pub use pbft::PBFTConsensus;