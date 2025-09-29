//! Core types used throughout the consensus library

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Node identifier type
pub type NodeId = Uuid;

/// Sequence number for ordering proposals
pub type SequenceNumber = u64;

/// View number for leader epochs
pub type ViewNumber = u64;

/// A proposal to be agreed upon by the consensus group
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique identifier for this proposal
    pub id: String,
    /// Proposer's node ID
    pub proposer: NodeId,
    /// Proposal data
    pub data: Vec<u8>,
    /// Timestamp when proposal was created
    pub timestamp: DateTime<Utc>,
}

impl Proposal {
    /// Create a new proposal
    pub fn new(id: String, data: Vec<u8>) -> Self {
        Self {
            id,
            proposer: Uuid::new_v4(), // Will be set by the consensus engine
            data,
            timestamp: Utc::now(),
        }
    }

    /// Set the proposer
    pub fn with_proposer(mut self, proposer: NodeId) -> Self {
        self.proposer = proposer;
        self
    }

    /// Get the size of the proposal in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Compute a digest of the proposal for integrity checking
    pub fn digest(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        hasher.update(self.id.as_bytes());
        hasher.update(self.proposer.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Result of a successful consensus operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// The agreed-upon proposal
    pub proposal: Proposal,
    /// Sequence number assigned to this result
    pub sequence: SequenceNumber,
    /// View in which consensus was reached
    pub view: ViewNumber,
    /// Nodes that participated in the consensus
    pub participants: Vec<NodeId>,
    /// Timestamp when consensus was reached
    pub committed_at: DateTime<Utc>,
}

impl ConsensusResult {
    /// Create a new consensus result
    pub fn new(
        proposal: Proposal,
        sequence: SequenceNumber,
        view: ViewNumber,
        participants: Vec<NodeId>,
    ) -> Self {
        Self {
            proposal,
            sequence,
            view,
            participants,
            committed_at: Utc::now(),
        }
    }
}

/// Statistics about consensus performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStats {
    /// Current view number
    pub current_view: ViewNumber,
    /// Current sequence number
    pub current_sequence: SequenceNumber,
    /// Total number of committed proposals
    pub total_committed: u64,
    /// Number of view changes that have occurred
    pub view_changes: u64,
    /// Number of failed proposals
    pub failed_proposals: u64,
    /// Average time to reach consensus (in milliseconds)
    pub average_commit_time_ms: f64,
    /// Number of participating nodes
    pub participant_count: usize,
    /// Last stable checkpoint sequence
    pub last_checkpoint: SequenceNumber,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
}

impl Default for ConsensusStats {
    fn default() -> Self {
        Self {
            current_view: 0,
            current_sequence: 0,
            total_committed: 0,
            view_changes: 0,
            failed_proposals: 0,
            average_commit_time_ms: 0.0,
            participant_count: 0,
            last_checkpoint: 0,
            uptime_seconds: 0,
            success_rate: 1.0,
        }
    }
}

/// Phase of the consensus protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Waiting for pre-prepare message
    PrePrepare,
    /// Waiting for prepare messages
    Prepare,
    /// Waiting for commit messages
    Commit,
    /// Proposal has been committed
    Committed,
    /// View change in progress
    ViewChange,
}

/// Role of a node in the current view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    /// Primary node (leader) for current view
    Primary,
    /// Backup node (follower) for current view
    Backup,
}

/// Cryptographic signature
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// Node that created the signature
    pub signer: NodeId,
    /// The signature data (simplified for this implementation)
    pub data: String,
    /// Timestamp when signature was created
    pub timestamp: DateTime<Utc>,
}

impl Signature {
    /// Create a new signature
    pub fn new(signer: NodeId, data: String) -> Self {
        Self {
            signer,
            data,
            timestamp: Utc::now(),
        }
    }

    /// Create a mock signature for testing
    pub fn mock(signer: NodeId) -> Self {
        Self::new(signer, format!("mock_signature_{}", signer))
    }

    /// Verify the signature (simplified implementation)
    pub fn verify(&self, expected_signer: NodeId) -> bool {
        self.signer == expected_signer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_creation() {
        let proposal = Proposal::new("test".to_string(), b"data".to_vec());
        assert_eq!(proposal.id, "test");
        assert_eq!(proposal.data, b"data");
        assert_eq!(proposal.size(), 4);
    }

    #[test]
    fn test_proposal_digest() {
        let proposal1 = Proposal::new("test".to_string(), b"data".to_vec());
        let proposal2 = Proposal::new("test".to_string(), b"data".to_vec());

        // Different proposers should have different digests
        assert_ne!(proposal1.digest(), proposal2.digest());
    }

    #[test]
    fn test_signature_verification() {
        let node_id = Uuid::new_v4();
        let signature = Signature::mock(node_id);

        assert!(signature.verify(node_id));
        assert!(!signature.verify(Uuid::new_v4()));
    }

    #[test]
    fn test_consensus_result() {
        let proposal = Proposal::new("test".to_string(), b"data".to_vec());
        let participants = vec![Uuid::new_v4(), Uuid::new_v4()];

        let result = ConsensusResult::new(proposal.clone(), 1, 0, participants.clone());

        assert_eq!(result.proposal, proposal);
        assert_eq!(result.sequence, 1);
        assert_eq!(result.view, 0);
        assert_eq!(result.participants, participants);
    }
}