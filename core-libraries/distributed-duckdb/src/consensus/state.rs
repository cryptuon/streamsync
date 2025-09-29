//! Consensus State Management for PBFT
//!
//! This module manages the internal state of the PBFT consensus protocol,
//! including view numbers, sequence numbers, and protocol phases.

use super::{ConsensusConfig, ConsensusProposal};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Current phase of the consensus protocol
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusPhase {
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

/// State of a specific proposal in the consensus protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalState {
    pub view: u64,
    pub sequence: u64,
    pub digest: String,
    pub proposal: ConsensusProposal,
    pub phase: ConsensusPhase,
    pub prepare_count: usize,
    pub commit_count: usize,
    pub prepared_nodes: Vec<Uuid>,
    pub committed_nodes: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub phase_timeout: Option<DateTime<Utc>>,
}

impl ProposalState {
    /// Create a new proposal state
    pub fn new(view: u64, sequence: u64, digest: String, proposal: ConsensusProposal) -> Self {
        Self {
            view,
            sequence,
            digest,
            proposal,
            phase: ConsensusPhase::PrePrepare,
            prepare_count: 0,
            commit_count: 0,
            prepared_nodes: Vec::new(),
            committed_nodes: Vec::new(),
            created_at: Utc::now(),
            phase_timeout: None,
        }
    }

    /// Add a prepare vote from a node
    pub fn add_prepare(&mut self, node_id: Uuid) {
        if !self.prepared_nodes.contains(&node_id) {
            self.prepared_nodes.push(node_id);
            self.prepare_count += 1;
        }
    }

    /// Add a commit vote from a node
    pub fn add_commit(&mut self, node_id: Uuid) {
        if !self.committed_nodes.contains(&node_id) {
            self.committed_nodes.push(node_id);
            self.commit_count += 1;
        }
    }

    /// Check if we have enough prepares for the next phase
    pub fn has_prepare_quorum(&self, quorum_size: usize) -> bool {
        self.prepare_count >= quorum_size
    }

    /// Check if we have enough commits to finalize
    pub fn has_commit_quorum(&self, quorum_size: usize) -> bool {
        self.commit_count >= quorum_size
    }

    /// Advance to the next phase
    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            ConsensusPhase::PrePrepare => ConsensusPhase::Prepare,
            ConsensusPhase::Prepare => ConsensusPhase::Commit,
            ConsensusPhase::Commit => ConsensusPhase::Committed,
            ConsensusPhase::Committed => ConsensusPhase::Committed,
            ConsensusPhase::ViewChange => ConsensusPhase::PrePrepare,
        };
    }

    /// Check if this proposal is expired
    pub fn is_expired(&self) -> bool {
        if let Some(timeout) = self.phase_timeout {
            Utc::now() > timeout
        } else {
            false
        }
    }

    /// Set phase timeout
    pub fn set_timeout(&mut self, duration_ms: u64) {
        self.phase_timeout = Some(Utc::now() + chrono::Duration::milliseconds(duration_ms as i64));
    }
}

/// Overall consensus state for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusState {
    /// Current view number
    pub current_view: u64,
    /// Current sequence number
    pub current_sequence: u64,
    /// Last committed sequence number
    pub last_committed: u64,
    /// Last stable checkpoint
    pub last_checkpoint: u64,
    /// Active proposals being processed
    pub active_proposals: HashMap<u64, ProposalState>,
    /// View change in progress
    pub view_change_in_progress: bool,
    /// Node that initiated current view change
    pub view_change_initiator: Option<Uuid>,
    /// Timer for view change timeout
    pub view_change_timer: Option<DateTime<Utc>>,
    /// Committed proposals awaiting execution
    pub committed_queue: Vec<(u64, ConsensusProposal)>,
    /// Watermark for garbage collection
    pub low_watermark: u64,
    pub high_watermark: u64,
    /// Configuration snapshot
    config: ConsensusConfig,
}

impl ConsensusState {
    /// Create a new consensus state
    pub fn new(config: ConsensusConfig) -> Self {
        Self {
            current_view: 0,
            current_sequence: 0,
            last_committed: 0,
            last_checkpoint: 0,
            active_proposals: HashMap::new(),
            view_change_in_progress: false,
            view_change_initiator: None,
            view_change_timer: None,
            committed_queue: Vec::new(),
            low_watermark: 0,
            high_watermark: 100, // Default window size
            config,
        }
    }

    /// Start a new proposal
    pub fn start_proposal(&mut self, view: u64, sequence: u64, digest: String, proposal: ConsensusProposal) -> Result<()> {
        // Validate sequence number
        if sequence <= self.last_committed {
            return Err(anyhow::anyhow!("Sequence number {} is too old (last committed: {})", sequence, self.last_committed));
        }

        if sequence < self.low_watermark || sequence > self.high_watermark {
            return Err(anyhow::anyhow!("Sequence number {} outside watermark range [{}, {}]", sequence, self.low_watermark, self.high_watermark));
        }

        // Create proposal state
        let mut proposal_state = ProposalState::new(view, sequence, digest, proposal);
        proposal_state.set_timeout(self.config.request_timeout_ms);

        self.active_proposals.insert(sequence, proposal_state);
        self.current_sequence = sequence.max(self.current_sequence);

        Ok(())
    }

    /// Add a prepare vote to a proposal
    pub fn add_prepare(&mut self, sequence: u64, node_id: Uuid) -> Result<bool> {
        if let Some(proposal) = self.active_proposals.get_mut(&sequence) {
            proposal.add_prepare(node_id);

            // Check if we can advance to commit phase
            if proposal.phase == ConsensusPhase::Prepare && proposal.has_prepare_quorum(self.config.quorum_size()) {
                proposal.advance_phase();
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Add a commit vote to a proposal
    pub fn add_commit(&mut self, sequence: u64, node_id: Uuid) -> Result<Option<ConsensusProposal>> {
        if let Some(proposal) = self.active_proposals.get_mut(&sequence) {
            proposal.add_commit(node_id);

            // Check if we can commit the proposal
            if proposal.phase == ConsensusPhase::Commit && proposal.has_commit_quorum(self.config.quorum_size()) {
                proposal.advance_phase();

                // Move to committed queue
                let committed_proposal = proposal.proposal.clone();
                self.committed_queue.push((sequence, committed_proposal.clone()));
                self.last_committed = sequence.max(self.last_committed);

                // Remove from active proposals
                self.active_proposals.remove(&sequence);

                return Ok(Some(committed_proposal));
            }
        }
        Ok(None)
    }

    /// Get a proposal by sequence number
    pub fn get_proposal(&self, sequence: u64) -> Option<&ProposalState> {
        self.active_proposals.get(&sequence)
    }

    /// Get mutable proposal by sequence number
    pub fn get_proposal_mut(&mut self, sequence: u64) -> Option<&mut ProposalState> {
        self.active_proposals.get_mut(&sequence)
    }

    /// Start a view change
    pub fn start_view_change(&mut self, new_view: u64, initiator: Uuid) {
        self.view_change_in_progress = true;
        self.view_change_initiator = Some(initiator);
        self.view_change_timer = Some(Utc::now() + chrono::Duration::milliseconds(self.config.view_change_timeout_ms as i64));

        // Mark all active proposals as view change
        for proposal in self.active_proposals.values_mut() {
            proposal.phase = ConsensusPhase::ViewChange;
        }

        tracing::info!("🔄 Started view change to view {} initiated by {}", new_view, initiator);
    }

    /// Complete a view change
    pub fn complete_view_change(&mut self, new_view: u64) {
        self.current_view = new_view;
        self.view_change_in_progress = false;
        self.view_change_initiator = None;
        self.view_change_timer = None;

        // Reset proposal phases
        for proposal in self.active_proposals.values_mut() {
            if proposal.phase == ConsensusPhase::ViewChange {
                proposal.phase = ConsensusPhase::PrePrepare;
                proposal.view = new_view;
            }
        }

        tracing::info!("✅ Completed view change to view {}", new_view);
    }

    /// Check if view change timer has expired
    pub fn is_view_change_expired(&self) -> bool {
        if let Some(timer) = self.view_change_timer {
            Utc::now() > timer
        } else {
            false
        }
    }

    /// Get next committed proposal from queue
    pub fn pop_committed(&mut self) -> Option<(u64, ConsensusProposal)> {
        if !self.committed_queue.is_empty() {
            Some(self.committed_queue.remove(0))
        } else {
            None
        }
    }

    /// Update checkpoint
    pub fn update_checkpoint(&mut self, sequence: u64) {
        self.last_checkpoint = sequence;
        self.low_watermark = sequence;
        self.high_watermark = sequence + 100; // Update window

        // Clean up old proposals
        self.active_proposals.retain(|&seq, _| seq > sequence);
        self.committed_queue.retain(|(seq, _)| *seq > sequence);

        tracing::info!("📍 Updated checkpoint to sequence {}, watermarks: [{}, {}]",
                      sequence, self.low_watermark, self.high_watermark);
    }

    /// Cleanup expired proposals
    pub fn cleanup_expired(&mut self) -> Vec<u64> {
        let mut expired = Vec::new();

        self.active_proposals.retain(|&seq, proposal| {
            if proposal.is_expired() {
                expired.push(seq);
                false
            } else {
                true
            }
        });

        if !expired.is_empty() {
            tracing::warn!("🗑️ Cleaned up {} expired proposals: {:?}", expired.len(), expired);
        }

        expired
    }

    /// Get state summary for diagnostics
    pub fn get_summary(&self) -> StateSnapshot {
        StateSnapshot {
            current_view: self.current_view,
            current_sequence: self.current_sequence,
            last_committed: self.last_committed,
            last_checkpoint: self.last_checkpoint,
            active_proposals_count: self.active_proposals.len(),
            committed_queue_length: self.committed_queue.len(),
            view_change_in_progress: self.view_change_in_progress,
            low_watermark: self.low_watermark,
            high_watermark: self.high_watermark,
        }
    }

    /// Check if we need to trigger a checkpoint
    pub fn should_checkpoint(&self) -> bool {
        self.last_committed >= self.last_checkpoint + self.config.checkpoint_interval
    }

    /// Check if sequence gap is too large
    pub fn has_sequence_gap(&self) -> bool {
        if self.active_proposals.is_empty() {
            return false;
        }

        let min_active = *self.active_proposals.keys().min().unwrap();
        min_active > self.last_committed + self.config.max_sequence_gap
    }

    /// Validate proposal can be processed
    pub fn can_process_proposal(&self, view: u64, sequence: u64) -> bool {
        // Check view
        if view != self.current_view {
            return false;
        }

        // Check sequence bounds
        if sequence <= self.last_committed ||
           sequence < self.low_watermark ||
           sequence > self.high_watermark {
            return false;
        }

        // Check if already processing
        if self.active_proposals.contains_key(&sequence) {
            return false;
        }

        // Check view change
        if self.view_change_in_progress {
            return false;
        }

        true
    }
}

/// Snapshot of consensus state for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub current_view: u64,
    pub current_sequence: u64,
    pub last_committed: u64,
    pub last_checkpoint: u64,
    pub active_proposals_count: usize,
    pub committed_queue_length: usize,
    pub view_change_in_progress: bool,
    pub low_watermark: u64,
    pub high_watermark: u64,
}