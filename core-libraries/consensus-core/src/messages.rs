//! Consensus message types and handling

use crate::types::{NodeId, SequenceNumber, ViewNumber, Signature, Proposal};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Type of consensus message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    /// Pre-prepare message from primary
    PrePrepare,
    /// Prepare message from backup
    Prepare,
    /// Commit message from any node
    Commit,
    /// View change request
    ViewChange,
    /// New view message from new primary
    NewView,
    /// Checkpoint message
    Checkpoint,
}

/// Core consensus message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusMessage {
    /// Type of message
    pub message_type: MessageType,
    /// View number
    pub view: ViewNumber,
    /// Sequence number
    pub sequence: SequenceNumber,
    /// Message-specific data
    pub data: Vec<u8>,
    /// Digital signature
    pub signature: Option<Signature>,
}

impl ConsensusMessage {
    /// Create a new consensus message
    pub fn new(
        message_type: MessageType,
        view: ViewNumber,
        sequence: SequenceNumber,
        data: Vec<u8>,
    ) -> Self {
        Self {
            message_type,
            view,
            sequence,
            data,
            signature: None,
        }
    }

    /// Add signature to the message
    pub fn with_signature(mut self, signature: Signature) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Get the size of the message in bytes
    pub fn size(&self) -> usize {
        std::mem::size_of::<MessageType>() +
        std::mem::size_of::<ViewNumber>() +
        std::mem::size_of::<SequenceNumber>() +
        self.data.len() +
        self.signature.as_ref().map_or(0, |s| s.data.len())
    }

    /// Verify the message signature
    pub fn verify_signature(&self, expected_signer: NodeId) -> bool {
        match &self.signature {
            Some(sig) => sig.verify(expected_signer),
            None => false,
        }
    }

    /// Compute digest of the message for integrity checking
    pub fn digest(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&[self.message_type as u8]);
        hasher.update(&self.view.to_le_bytes());
        hasher.update(&self.sequence.to_le_bytes());
        hasher.update(&self.data);
        format!("{:x}", hasher.finalize())
    }
}

/// Pre-prepare message data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrePrepareData {
    /// The proposal being proposed
    pub proposal: Proposal,
    /// Digest of the proposal for quick verification
    pub digest: String,
    /// Primary node that sent this pre-prepare
    pub primary: NodeId,
    /// Timestamp when pre-prepare was created
    pub timestamp: DateTime<Utc>,
}

impl PrePrepareData {
    /// Create new pre-prepare data
    pub fn new(proposal: Proposal, primary: NodeId) -> Self {
        let digest = proposal.digest();
        Self {
            proposal,
            digest,
            primary,
            timestamp: Utc::now(),
        }
    }

    /// Serialize to bytes for message data
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        bincode::deserialize(data).ok()
    }
}

/// Prepare message data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrepareData {
    /// Digest of the proposal being prepared
    pub digest: String,
    /// Node that sent this prepare
    pub node: NodeId,
    /// Timestamp when prepare was created
    pub timestamp: DateTime<Utc>,
}

impl PrepareData {
    /// Create new prepare data
    pub fn new(digest: String, node: NodeId) -> Self {
        Self {
            digest,
            node,
            timestamp: Utc::now(),
        }
    }

    /// Serialize to bytes for message data
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        bincode::deserialize(data).ok()
    }
}

/// Commit message data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitData {
    /// Digest of the proposal being committed
    pub digest: String,
    /// Node that sent this commit
    pub node: NodeId,
    /// Timestamp when commit was created
    pub timestamp: DateTime<Utc>,
}

impl CommitData {
    /// Create new commit data
    pub fn new(digest: String, node: NodeId) -> Self {
        Self {
            digest,
            node,
            timestamp: Utc::now(),
        }
    }

    /// Serialize to bytes for message data
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        bincode::deserialize(data).ok()
    }
}

/// View change message data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewChangeData {
    /// New view being proposed
    pub new_view: ViewNumber,
    /// Last stable checkpoint
    pub last_checkpoint: SequenceNumber,
    /// Node requesting view change
    pub node: NodeId,
    /// Prepared messages since last checkpoint
    pub prepared_messages: Vec<PreparedProof>,
    /// Timestamp when view change was initiated
    pub timestamp: DateTime<Utc>,
}

impl ViewChangeData {
    /// Create new view change data
    pub fn new(new_view: ViewNumber, last_checkpoint: SequenceNumber, node: NodeId) -> Self {
        Self {
            new_view,
            last_checkpoint,
            node,
            prepared_messages: Vec::new(),
            timestamp: Utc::now(),
        }
    }

    /// Add prepared proof
    pub fn with_prepared_proof(mut self, proof: PreparedProof) -> Self {
        self.prepared_messages.push(proof);
        self
    }

    /// Serialize to bytes for message data
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        bincode::deserialize(data).ok()
    }
}

/// Proof that a message was prepared
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedProof {
    /// View number
    pub view: ViewNumber,
    /// Sequence number
    pub sequence: SequenceNumber,
    /// Proposal digest
    pub digest: String,
    /// Prepare messages from different nodes
    pub prepare_signatures: Vec<Signature>,
}

/// New view message data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewViewData {
    /// New view number
    pub view: ViewNumber,
    /// View change messages that justified this new view
    pub view_change_proofs: Vec<ViewChangeProof>,
    /// New primary for this view
    pub primary: NodeId,
    /// Pre-prepare messages for incomplete requests
    pub pre_prepares: Vec<PrePrepareData>,
    /// Timestamp when new view was created
    pub timestamp: DateTime<Utc>,
}

impl NewViewData {
    /// Create new view data
    pub fn new(view: ViewNumber, primary: NodeId) -> Self {
        Self {
            view,
            view_change_proofs: Vec::new(),
            primary,
            pre_prepares: Vec::new(),
            timestamp: Utc::now(),
        }
    }

    /// Serialize to bytes for message data
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        bincode::deserialize(data).ok()
    }
}

/// Proof of view change
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewChangeProof {
    /// View change message
    pub view_change: ViewChangeData,
    /// Signature of the view change
    pub signature: Signature,
}

/// Checkpoint message data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointData {
    /// Sequence number of the checkpoint
    pub sequence: SequenceNumber,
    /// Hash of the application state at this sequence
    pub state_hash: String,
    /// Node that created this checkpoint
    pub node: NodeId,
    /// Timestamp when checkpoint was created
    pub timestamp: DateTime<Utc>,
}

impl CheckpointData {
    /// Create new checkpoint data
    pub fn new(sequence: SequenceNumber, state_hash: String, node: NodeId) -> Self {
        Self {
            sequence,
            state_hash,
            node,
            timestamp: Utc::now(),
        }
    }

    /// Serialize to bytes for message data
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        bincode::deserialize(data).ok()
    }
}

/// Message builder for creating typed consensus messages
pub struct MessageBuilder {
    node_id: NodeId,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new(node_id: NodeId) -> Self {
        Self { node_id }
    }

    /// Build a pre-prepare message
    pub fn pre_prepare(
        &self,
        view: ViewNumber,
        sequence: SequenceNumber,
        proposal: Proposal,
    ) -> ConsensusMessage {
        let data = PrePrepareData::new(proposal, self.node_id);
        ConsensusMessage::new(MessageType::PrePrepare, view, sequence, data.to_bytes())
            .with_signature(Signature::mock(self.node_id))
    }

    /// Build a prepare message
    pub fn prepare(
        &self,
        view: ViewNumber,
        sequence: SequenceNumber,
        digest: String,
    ) -> ConsensusMessage {
        let data = PrepareData::new(digest, self.node_id);
        ConsensusMessage::new(MessageType::Prepare, view, sequence, data.to_bytes())
            .with_signature(Signature::mock(self.node_id))
    }

    /// Build a commit message
    pub fn commit(
        &self,
        view: ViewNumber,
        sequence: SequenceNumber,
        digest: String,
    ) -> ConsensusMessage {
        let data = CommitData::new(digest, self.node_id);
        ConsensusMessage::new(MessageType::Commit, view, sequence, data.to_bytes())
            .with_signature(Signature::mock(self.node_id))
    }

    /// Build a view change message
    pub fn view_change(
        &self,
        new_view: ViewNumber,
        last_checkpoint: SequenceNumber,
    ) -> ConsensusMessage {
        let data = ViewChangeData::new(new_view, last_checkpoint, self.node_id);
        ConsensusMessage::new(MessageType::ViewChange, new_view, 0, data.to_bytes())
            .with_signature(Signature::mock(self.node_id))
    }

    /// Build a checkpoint message
    pub fn checkpoint(
        &self,
        sequence: SequenceNumber,
        state_hash: String,
    ) -> ConsensusMessage {
        let data = CheckpointData::new(sequence, state_hash, self.node_id);
        ConsensusMessage::new(MessageType::Checkpoint, 0, sequence, data.to_bytes())
            .with_signature(Signature::mock(self.node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_message_creation() {
        let message = ConsensusMessage::new(
            MessageType::Prepare,
            1,
            10,
            vec![1, 2, 3],
        );

        assert_eq!(message.message_type, MessageType::Prepare);
        assert_eq!(message.view, 1);
        assert_eq!(message.sequence, 10);
        assert_eq!(message.data, vec![1, 2, 3]);
        assert!(message.signature.is_none());
    }

    #[test]
    fn test_message_with_signature() {
        let node_id = Uuid::new_v4();
        let signature = Signature::mock(node_id);

        let message = ConsensusMessage::new(
            MessageType::Commit,
            0,
            5,
            vec![],
        ).with_signature(signature);

        assert!(message.signature.is_some());
        assert!(message.verify_signature(node_id));
    }

    #[test]
    fn test_message_digest() {
        let message1 = ConsensusMessage::new(MessageType::Prepare, 1, 10, vec![1, 2, 3]);
        let message2 = ConsensusMessage::new(MessageType::Prepare, 1, 10, vec![1, 2, 3]);
        let message3 = ConsensusMessage::new(MessageType::Prepare, 1, 11, vec![1, 2, 3]);

        // Same messages should have same digest
        assert_eq!(message1.digest(), message2.digest());

        // Different messages should have different digests
        assert_ne!(message1.digest(), message3.digest());
    }

    #[test]
    fn test_pre_prepare_data() {
        let proposal = Proposal::new("test".to_string(), b"data".to_vec());
        let node_id = Uuid::new_v4();

        let pre_prepare = PrePrepareData::new(proposal.clone(), node_id);

        assert_eq!(pre_prepare.proposal, proposal);
        assert_eq!(pre_prepare.digest, proposal.digest());
        assert_eq!(pre_prepare.primary, node_id);

        // Test serialization
        let bytes = pre_prepare.to_bytes();
        let deserialized = PrePrepareData::from_bytes(&bytes).unwrap();
        assert_eq!(pre_prepare, deserialized);
    }

    #[test]
    fn test_message_builder() {
        let node_id = Uuid::new_v4();
        let builder = MessageBuilder::new(node_id);

        let proposal = Proposal::new("test".to_string(), b"data".to_vec());

        // Test pre-prepare
        let pre_prepare = builder.pre_prepare(1, 10, proposal);
        assert_eq!(pre_prepare.message_type, MessageType::PrePrepare);
        assert_eq!(pre_prepare.view, 1);
        assert_eq!(pre_prepare.sequence, 10);
        assert!(pre_prepare.verify_signature(node_id));

        // Test prepare
        let prepare = builder.prepare(1, 10, "digest".to_string());
        assert_eq!(prepare.message_type, MessageType::Prepare);

        // Test commit
        let commit = builder.commit(1, 10, "digest".to_string());
        assert_eq!(commit.message_type, MessageType::Commit);

        // Test view change
        let view_change = builder.view_change(2, 5);
        assert_eq!(view_change.message_type, MessageType::ViewChange);

        // Test checkpoint
        let checkpoint = builder.checkpoint(100, "state_hash".to_string());
        assert_eq!(checkpoint.message_type, MessageType::Checkpoint);
    }

    #[test]
    fn test_data_serialization() {
        let node_id = Uuid::new_v4();

        // Test PrepareData
        let prepare_data = PrepareData::new("digest".to_string(), node_id);
        let bytes = prepare_data.to_bytes();
        let deserialized = PrepareData::from_bytes(&bytes).unwrap();
        assert_eq!(prepare_data, deserialized);

        // Test CommitData
        let commit_data = CommitData::new("digest".to_string(), node_id);
        let bytes = commit_data.to_bytes();
        let deserialized = CommitData::from_bytes(&bytes).unwrap();
        assert_eq!(commit_data, deserialized);

        // Test CheckpointData
        let checkpoint_data = CheckpointData::new(100, "state_hash".to_string(), node_id);
        let bytes = checkpoint_data.to_bytes();
        let deserialized = CheckpointData::from_bytes(&bytes).unwrap();
        assert_eq!(checkpoint_data, deserialized);
    }
}