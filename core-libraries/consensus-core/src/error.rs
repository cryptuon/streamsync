//! Error types for the consensus library

use thiserror::Error;
use crate::types::{NodeId, ViewNumber, SequenceNumber};

/// Result type for consensus operations
pub type Result<T> = std::result::Result<T, ConsensusError>;

/// Errors that can occur during consensus operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ConsensusError {
    /// Insufficient nodes to form a consensus group
    #[error("Insufficient nodes: need at least {required}, got {actual}")]
    InsufficientNodes { required: usize, actual: usize },

    /// Node is not part of the consensus group
    #[error("Node {node_id} is not a participant in consensus")]
    NodeNotParticipant { node_id: NodeId },

    /// Only the primary can propose in the current view
    #[error("Node {node_id} cannot propose in view {view} (not primary)")]
    NotPrimary { node_id: NodeId, view: ViewNumber },

    /// Invalid view number received
    #[error("Invalid view number: expected {expected}, got {actual}")]
    InvalidView { expected: ViewNumber, actual: ViewNumber },

    /// Invalid sequence number received
    #[error("Invalid sequence number: {sequence} (last committed: {last_committed})")]
    InvalidSequence { sequence: SequenceNumber, last_committed: SequenceNumber },

    /// Proposal digest verification failed
    #[error("Proposal digest mismatch: expected {expected}, got {actual}")]
    DigestMismatch { expected: String, actual: String },

    /// Signature verification failed
    #[error("Invalid signature from node {node_id}")]
    InvalidSignature { node_id: NodeId },

    /// Timeout waiting for consensus
    #[error("Consensus timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// View change failed
    #[error("View change to {new_view} failed: {reason}")]
    ViewChangeFailed { new_view: ViewNumber, reason: String },

    /// State transfer required but failed
    #[error("State transfer failed: {reason}")]
    StateTransferFailed { reason: String },

    /// Sequence gap detected
    #[error("Sequence gap detected: expected {expected}, got {actual}")]
    SequenceGap { expected: SequenceNumber, actual: SequenceNumber },

    /// Proposal already exists
    #[error("Proposal {proposal_id} already exists in sequence {sequence}")]
    DuplicateProposal { proposal_id: String, sequence: SequenceNumber },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Transport layer error
    #[error("Transport error: {message}")]
    Transport { message: String },

    /// Internal consensus engine error
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Engine is not running
    #[error("Consensus engine is not running")]
    NotRunning,

    /// Engine is already running
    #[error("Consensus engine is already running")]
    AlreadyRunning,

    /// Operation was cancelled
    #[error("Operation was cancelled")]
    Cancelled,

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {resource} ({current}/{limit})")]
    ResourceLimit {
        resource: String,
        current: usize,
        limit: usize,
    },
}

impl ConsensusError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            ConsensusError::Timeout { .. } => true,
            ConsensusError::ViewChangeFailed { .. } => true,
            ConsensusError::Transport { .. } => true,
            ConsensusError::SequenceGap { .. } => true,
            ConsensusError::InvalidView { .. } => true,
            _ => false,
        }
    }

    /// Check if the error indicates a Byzantine fault
    pub fn is_byzantine_fault(&self) -> bool {
        match self {
            ConsensusError::DigestMismatch { .. } => true,
            ConsensusError::InvalidSignature { .. } => true,
            ConsensusError::DuplicateProposal { .. } => true,
            ConsensusError::InvalidSequence { .. } => true,
            _ => false,
        }
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            ConsensusError::InsufficientNodes { .. } => "configuration",
            ConsensusError::NodeNotParticipant { .. } => "configuration",
            ConsensusError::NotPrimary { .. } => "protocol",
            ConsensusError::InvalidView { .. } => "protocol",
            ConsensusError::InvalidSequence { .. } => "protocol",
            ConsensusError::DigestMismatch { .. } => "byzantine",
            ConsensusError::InvalidSignature { .. } => "byzantine",
            ConsensusError::Timeout { .. } => "timeout",
            ConsensusError::ViewChangeFailed { .. } => "view_change",
            ConsensusError::StateTransferFailed { .. } => "state_transfer",
            ConsensusError::SequenceGap { .. } => "protocol",
            ConsensusError::DuplicateProposal { .. } => "byzantine",
            ConsensusError::Configuration { .. } => "configuration",
            ConsensusError::Transport { .. } => "transport",
            ConsensusError::Internal { .. } => "internal",
            ConsensusError::NotRunning => "state",
            ConsensusError::AlreadyRunning => "state",
            ConsensusError::Cancelled => "cancelled",
            ConsensusError::ResourceLimit { .. } => "resource",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_error_categories() {
        let node_id = Uuid::new_v4();

        assert_eq!(
            ConsensusError::InsufficientNodes { required: 4, actual: 2 }.category(),
            "configuration"
        );

        assert_eq!(
            ConsensusError::DigestMismatch {
                expected: "abc".to_string(),
                actual: "def".to_string()
            }.category(),
            "byzantine"
        );

        assert_eq!(
            ConsensusError::Timeout { timeout_ms: 5000 }.category(),
            "timeout"
        );
    }

    #[test]
    fn test_recoverable_errors() {
        assert!(ConsensusError::Timeout { timeout_ms: 5000 }.is_recoverable());
        assert!(!ConsensusError::InsufficientNodes { required: 4, actual: 2 }.is_recoverable());
    }

    #[test]
    fn test_byzantine_faults() {
        assert!(ConsensusError::DigestMismatch {
            expected: "abc".to_string(),
            actual: "def".to_string()
        }.is_byzantine_fault());

        assert!(!ConsensusError::Timeout { timeout_ms: 5000 }.is_byzantine_fault());
    }
}