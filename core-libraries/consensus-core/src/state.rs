//! Consensus state management

use crate::types::{NodeId, SequenceNumber, ViewNumber, Phase, Proposal, ConsensusStats, ConsensusResult};
use crate::messages::{MessageType, PrepareData, CommitData, PrePrepareData};
use crate::error::{ConsensusError, Result};
use crate::config::Config;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::sync::oneshot;

/// State of a single proposal in the consensus protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalState {
    /// The proposal being processed
    pub proposal: Proposal,
    /// Current phase of the consensus protocol
    pub phase: Phase,
    /// View number
    pub view: ViewNumber,
    /// Sequence number
    pub sequence: SequenceNumber,
    /// Proposal digest
    pub digest: String,
    /// Primary node for this proposal
    pub primary: NodeId,
    /// Nodes that sent prepare messages
    pub prepared_by: HashSet<NodeId>,
    /// Nodes that sent commit messages
    pub committed_by: HashSet<NodeId>,
    /// Timestamp when proposal was created
    pub created_at: DateTime<Utc>,
    /// Timeout for this proposal
    pub timeout_at: Option<DateTime<Utc>>,
}

impl ProposalState {
    /// Create a new proposal state from pre-prepare
    pub fn from_pre_prepare(
        view: ViewNumber,
        sequence: SequenceNumber,
        pre_prepare_data: &PrePrepareData,
        timeout: std::time::Duration,
    ) -> Self {
        let now = Utc::now();
        Self {
            proposal: pre_prepare_data.proposal.clone(),
            phase: Phase::PrePrepare,
            view,
            sequence,
            digest: pre_prepare_data.digest.clone(),
            primary: pre_prepare_data.primary,
            prepared_by: HashSet::new(),
            committed_by: HashSet::new(),
            created_at: now,
            timeout_at: Some(now + chrono::Duration::from_std(timeout).unwrap()),
        }
    }

    /// Add a prepare vote
    pub fn add_prepare(&mut self, node_id: NodeId) -> bool {
        if self.phase == Phase::PrePrepare || self.phase == Phase::Prepare {
            self.prepared_by.insert(node_id)
        } else {
            false
        }
    }

    /// Add a commit vote
    pub fn add_commit(&mut self, node_id: NodeId) -> bool {
        if self.phase == Phase::Prepare || self.phase == Phase::Commit {
            self.committed_by.insert(node_id)
        } else {
            false
        }
    }

    /// Check if we have enough prepares for the next phase
    pub fn has_prepare_quorum(&self, quorum_size: usize) -> bool {
        self.prepared_by.len() >= quorum_size
    }

    /// Check if we have enough commits to finalize
    pub fn has_commit_quorum(&self, quorum_size: usize) -> bool {
        self.committed_by.len() >= quorum_size
    }

    /// Advance to the next phase
    pub fn advance_to_prepare(&mut self) {
        if self.phase == Phase::PrePrepare {
            self.phase = Phase::Prepare;
        }
    }

    /// Advance to commit phase
    pub fn advance_to_commit(&mut self) {
        if self.phase == Phase::Prepare {
            self.phase = Phase::Commit;
        }
    }

    /// Mark as committed
    pub fn mark_committed(&mut self) {
        self.phase = Phase::Committed;
    }

    /// Check if this proposal has expired
    pub fn is_expired(&self) -> bool {
        self.timeout_at.map_or(false, |timeout| Utc::now() > timeout)
    }

    /// Get prepare count
    pub fn prepare_count(&self) -> usize {
        self.prepared_by.len()
    }

    /// Get commit count
    pub fn commit_count(&self) -> usize {
        self.committed_by.len()
    }
}

/// Completion notifier for proposal consensus
pub struct ProposalNotifier {
    pub tx: oneshot::Sender<ConsensusResult>,
}

/// Overall consensus state
pub struct ConsensusState {
    /// Configuration
    config: Config,

    /// Current view number
    pub current_view: ViewNumber,

    /// Current sequence number (next to be assigned)
    pub current_sequence: SequenceNumber,

    /// Last committed sequence number
    pub last_committed: SequenceNumber,

    /// Last stable checkpoint
    pub last_checkpoint: SequenceNumber,

    /// Active proposals being processed
    pub active_proposals: HashMap<SequenceNumber, ProposalState>,

    /// View change in progress
    pub view_change_in_progress: bool,

    /// View change votes received
    pub view_change_votes: HashMap<ViewNumber, HashSet<NodeId>>,

    /// Statistics
    stats: ConsensusStats,

    /// Start time for uptime calculation
    start_time: DateTime<Utc>,

    /// Message counts for statistics
    message_counts: HashMap<MessageType, u64>,

    /// Commit times for average calculation
    recent_commit_times: Vec<u64>,

    /// Completion notifiers for pending proposals (not serializable)
    #[allow(dead_code)]
    completion_notifiers: HashMap<SequenceNumber, ProposalNotifier>,
}

impl ConsensusState {
    /// Create a new consensus state
    pub fn new(config: Config) -> Self {
        let now = Utc::now();
        Self {
            current_view: 0,
            current_sequence: 1,
            last_committed: 0,
            last_checkpoint: 0,
            active_proposals: HashMap::new(),
            view_change_in_progress: false,
            view_change_votes: HashMap::new(),
            stats: ConsensusStats {
                participant_count: config.participant_count(),
                ..Default::default()
            },
            start_time: now,
            message_counts: HashMap::new(),
            recent_commit_times: Vec::new(),
            completion_notifiers: HashMap::new(),
            config,
        }
    }

    /// Register a completion notifier for a proposal
    pub fn register_completion_notifier(&mut self, sequence: SequenceNumber, tx: oneshot::Sender<ConsensusResult>) {
        self.completion_notifiers.insert(sequence, ProposalNotifier { tx });
    }

    /// Notify completion of a proposal (called when consensus is reached)
    pub fn notify_completion(&mut self, sequence: SequenceNumber, result: ConsensusResult) {
        if let Some(notifier) = self.completion_notifiers.remove(&sequence) {
            // Ignore send errors - receiver might have dropped
            let _ = notifier.tx.send(result);
        }
    }

    /// Cancel a pending completion notifier (e.g., on timeout or view change)
    pub fn cancel_notifier(&mut self, sequence: SequenceNumber) {
        self.completion_notifiers.remove(&sequence);
    }

    /// Clear all completion notifiers (e.g., on view change)
    pub fn clear_notifiers(&mut self) {
        self.completion_notifiers.clear();
    }

    /// Get current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Check if this node is the primary for the current view
    pub fn is_primary(&self) -> bool {
        self.config.is_primary(self.current_view)
    }

    /// Get the primary node for the current view
    pub fn current_primary(&self) -> Option<NodeId> {
        self.config.primary_for_view(self.current_view)
    }

    /// Start a new proposal (primary only)
    pub fn start_proposal(&mut self, _proposal: Proposal) -> Result<SequenceNumber> {
        if !self.is_primary() {
            return Err(ConsensusError::NotPrimary {
                node_id: self.config.node_id,
                view: self.current_view,
            });
        }

        if self.view_change_in_progress {
            return Err(ConsensusError::ViewChangeFailed {
                new_view: self.current_view,
                reason: "View change in progress".to_string(),
            });
        }

        let sequence = self.current_sequence;
        self.current_sequence += 1;

        // Create proposal state (will be added when pre-prepare is processed)
        Ok(sequence)
    }

    /// Add a proposal from pre-prepare message
    pub fn add_proposal(&mut self,
        view: ViewNumber,
        sequence: SequenceNumber,
        pre_prepare_data: &PrePrepareData,
    ) -> Result<()> {
        // Validate view
        if view != self.current_view {
            return Err(ConsensusError::InvalidView {
                expected: self.current_view,
                actual: view,
            });
        }

        // Validate sequence
        if sequence <= self.last_committed {
            return Err(ConsensusError::InvalidSequence {
                sequence,
                last_committed: self.last_committed,
            });
        }

        // Check if proposal already exists
        if self.active_proposals.contains_key(&sequence) {
            return Err(ConsensusError::DuplicateProposal {
                proposal_id: pre_prepare_data.proposal.id.clone(),
                sequence,
            });
        }

        // Verify proposal digest
        if pre_prepare_data.digest != pre_prepare_data.proposal.digest() {
            return Err(ConsensusError::DigestMismatch {
                expected: pre_prepare_data.proposal.digest(),
                actual: pre_prepare_data.digest.clone(),
            });
        }

        // Create proposal state
        let proposal_state = ProposalState::from_pre_prepare(
            view,
            sequence,
            pre_prepare_data,
            self.config.request_timeout,
        );

        self.active_proposals.insert(sequence, proposal_state);
        self.record_message(MessageType::PrePrepare);

        Ok(())
    }

    /// Process a prepare message
    pub fn process_prepare(&mut self,
        view: ViewNumber,
        sequence: SequenceNumber,
        prepare_data: &PrepareData,
    ) -> Result<bool> {
        // Validate view
        if view != self.current_view {
            return Ok(false);
        }

        // Validate and update proposal state
        let quorum_size = self.config.quorum_size();
        let should_advance = {
            // If we don't have the proposal yet (PrePrepare not received), ignore this Prepare
            let proposal = match self.active_proposals.get_mut(&sequence) {
                Some(p) => p,
                None => return Ok(false), // Haven't received PrePrepare yet, ignore
            };

            // Validate digest
            if prepare_data.digest != proposal.digest {
                return Err(ConsensusError::DigestMismatch {
                    expected: proposal.digest.clone(),
                    actual: prepare_data.digest.clone(),
                });
            }

            // Add prepare vote
            let added = proposal.add_prepare(prepare_data.node);
            if added {
                // Ensure we're in Prepare phase (transition from PrePrepare if needed)
                proposal.advance_to_prepare();

                // Check if we have quorum and can advance to commit phase
                let has_quorum = proposal.has_prepare_quorum(quorum_size);
                if has_quorum {
                    proposal.advance_to_commit();
                }
                (true, has_quorum)
            } else {
                (false, false)
            }
        };

        if should_advance.0 {
            self.record_message(MessageType::Prepare);
            if should_advance.1 {
                return Ok(true); // Signal that we should send commit
            }
        }

        Ok(false)
    }

    /// Process a commit message
    pub fn process_commit(&mut self,
        view: ViewNumber,
        sequence: SequenceNumber,
        commit_data: &CommitData,
    ) -> Result<Option<Proposal>> {
        // Validate view
        if view != self.current_view {
            return Ok(None);
        }

        // Validate and update proposal state
        let quorum_size = self.config.quorum_size();
        let (should_record, commit_time_opt, should_commit) = {
            // If we don't have the proposal yet, ignore this Commit message
            let proposal = match self.active_proposals.get_mut(&sequence) {
                Some(p) => p,
                None => return Ok(None), // Haven't received PrePrepare yet, ignore
            };

            // Validate digest
            if commit_data.digest != proposal.digest {
                return Err(ConsensusError::DigestMismatch {
                    expected: proposal.digest.clone(),
                    actual: commit_data.digest.clone(),
                });
            }

            // Add commit vote
            let added = proposal.add_commit(commit_data.node);
            if added {
                // Check if we have quorum and can commit
                if proposal.has_commit_quorum(quorum_size) {
                    proposal.mark_committed();
                    let commit_time = (Utc::now() - proposal.created_at).num_milliseconds() as u64;
                    (true, Some(commit_time), true)
                } else {
                    (true, None, false)
                }
            } else {
                (false, None, false)
            }
        };

        if should_record {
            self.record_message(MessageType::Commit);
        }

        if should_commit {
            // Update last committed sequence
            if sequence > self.last_committed {
                self.last_committed = sequence;
            }

            // Update statistics
            self.stats.total_committed += 1;
            self.stats.current_sequence = self.current_sequence;

            // Record commit time
            if let Some(commit_time) = commit_time_opt {
                self.recent_commit_times.push(commit_time);
                if self.recent_commit_times.len() > 100 {
                    self.recent_commit_times.remove(0);
                }
            }

            // Remove from active proposals
            let committed_proposal = self.active_proposals.remove(&sequence).unwrap();
            return Ok(Some(committed_proposal.proposal));
        }

        Ok(None)
    }

    /// Start a view change
    pub fn start_view_change(&mut self, new_view: ViewNumber) -> Result<()> {
        if new_view <= self.current_view {
            return Err(ConsensusError::InvalidView {
                expected: self.current_view + 1,
                actual: new_view,
            });
        }

        self.view_change_in_progress = true;
        self.view_change_votes.clear();

        // Add our own vote
        self.view_change_votes
            .entry(new_view)
            .or_insert_with(HashSet::new)
            .insert(self.config.node_id);

        self.stats.view_changes += 1;

        Ok(())
    }

    /// Process a view change vote
    pub fn process_view_change_vote(&mut self, new_view: ViewNumber, node_id: NodeId) -> Result<bool> {
        if !self.view_change_in_progress {
            return Ok(false);
        }

        if new_view <= self.current_view {
            return Ok(false);
        }

        // Add vote
        let votes = self.view_change_votes
            .entry(new_view)
            .or_insert_with(HashSet::new);
        votes.insert(node_id);

        // Check if we have enough votes
        Ok(votes.len() >= self.config.quorum_size())
    }

    /// Complete view change
    pub fn complete_view_change(&mut self, new_view: ViewNumber) -> Result<()> {
        if new_view <= self.current_view {
            return Err(ConsensusError::InvalidView {
                expected: self.current_view + 1,
                actual: new_view,
            });
        }

        self.current_view = new_view;
        self.view_change_in_progress = false;
        self.view_change_votes.clear();

        // Clear active proposals (they will need to be re-proposed)
        self.active_proposals.clear();

        // Clear completion notifiers (proposals are no longer valid)
        self.clear_notifiers();

        self.stats.current_view = new_view;

        Ok(())
    }

    /// Create a checkpoint
    pub fn create_checkpoint(&mut self) -> Option<(SequenceNumber, String)> {
        if self.last_committed >= self.last_checkpoint + self.config.checkpoint_interval {
            let checkpoint_sequence = self.last_committed;
            let state_hash = self.compute_state_hash();

            self.last_checkpoint = checkpoint_sequence;
            self.stats.last_checkpoint = checkpoint_sequence;

            // Clean up old proposals
            self.active_proposals.retain(|&seq, _| seq > checkpoint_sequence);

            Some((checkpoint_sequence, state_hash))
        } else {
            None
        }
    }

    /// Cleanup expired proposals
    pub fn cleanup_expired_proposals(&mut self) -> Vec<SequenceNumber> {
        let mut expired = Vec::new();

        self.active_proposals.retain(|&sequence, proposal| {
            if proposal.is_expired() {
                expired.push(sequence);
                false
            } else {
                true
            }
        });

        if !expired.is_empty() {
            self.stats.failed_proposals += expired.len() as u64;
        }

        expired
    }

    /// Get current statistics
    pub fn get_stats(&self) -> ConsensusStats {
        let mut stats = self.stats.clone();

        // Update uptime
        stats.uptime_seconds = (Utc::now() - self.start_time).num_seconds() as u64;

        // Update average commit time
        if !self.recent_commit_times.is_empty() {
            stats.average_commit_time_ms = self.recent_commit_times.iter().sum::<u64>() as f64 / self.recent_commit_times.len() as f64;
        }

        // Update success rate
        let total_attempts = stats.total_committed + stats.failed_proposals;
        if total_attempts > 0 {
            stats.success_rate = stats.total_committed as f64 / total_attempts as f64;
        }

        stats
    }

    /// Get proposal state
    pub fn get_proposal(&self, sequence: SequenceNumber) -> Option<&ProposalState> {
        self.active_proposals.get(&sequence)
    }

    /// Check if we need a checkpoint
    pub fn needs_checkpoint(&self) -> bool {
        self.last_committed >= self.last_checkpoint + self.config.checkpoint_interval
    }

    /// Get active proposal count
    pub fn active_proposal_count(&self) -> usize {
        self.active_proposals.len()
    }

    /// Record message for statistics
    fn record_message(&mut self, message_type: MessageType) {
        *self.message_counts.entry(message_type).or_insert(0) += 1;
    }

    /// Compute state hash for checkpoint
    fn compute_state_hash(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&self.last_committed.to_le_bytes());
        hasher.update(&self.current_view.to_le_bytes());
        hasher.update(&self.current_sequence.to_le_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use uuid::Uuid;

    fn create_test_state() -> ConsensusState {
        let config = Config::test_config(4);
        ConsensusState::new(config)
    }

    #[test]
    fn test_proposal_state_creation() {
        let proposal = Proposal::new("test".to_string(), b"data".to_vec());
        let node_id = Uuid::new_v4();
        let pre_prepare_data = PrePrepareData::new(proposal, node_id);

        let state = ProposalState::from_pre_prepare(
            0,
            1,
            &pre_prepare_data,
            std::time::Duration::from_secs(5),
        );

        assert_eq!(state.phase, Phase::PrePrepare);
        assert_eq!(state.view, 0);
        assert_eq!(state.sequence, 1);
        assert_eq!(state.primary, node_id);
        assert_eq!(state.prepare_count(), 0);
        assert_eq!(state.commit_count(), 0);
    }

    #[test]
    fn test_proposal_state_voting() {
        let proposal = Proposal::new("test".to_string(), b"data".to_vec());
        let node_id = Uuid::new_v4();
        let pre_prepare_data = PrePrepareData::new(proposal, node_id);

        let mut state = ProposalState::from_pre_prepare(
            0,
            1,
            &pre_prepare_data,
            std::time::Duration::from_secs(5),
        );

        // Add prepare votes
        assert!(state.add_prepare(Uuid::new_v4()));
        assert!(state.add_prepare(Uuid::new_v4()));
        assert_eq!(state.prepare_count(), 2);

        // Check quorum (need 3 for 4 nodes)
        assert!(!state.has_prepare_quorum(3));
        state.add_prepare(Uuid::new_v4());
        assert!(state.has_prepare_quorum(3));

        // Advance to prepare phase first, then commit phase
        state.advance_to_prepare();
        assert_eq!(state.phase, Phase::Prepare);
        state.advance_to_commit();
        assert_eq!(state.phase, Phase::Commit);

        // Add commit votes
        assert!(state.add_commit(Uuid::new_v4()));
        assert!(state.add_commit(Uuid::new_v4()));
        assert!(state.add_commit(Uuid::new_v4()));
        assert!(state.has_commit_quorum(3));
    }

    #[test]
    fn test_consensus_state_proposal_flow() {
        let mut state = create_test_state();

        // Start proposal (should work as we're primary for view 0)
        let proposal = Proposal::new("test".to_string(), b"data".to_vec());
        let sequence = state.start_proposal(proposal.clone()).unwrap();
        assert_eq!(sequence, 1);

        // Add the proposal via pre-prepare
        let pre_prepare_data = PrePrepareData::new(proposal, state.config.node_id);
        assert!(state.add_proposal(0, sequence, &pre_prepare_data).is_ok());

        // Process prepare messages (including primary's prepare)
        let prepare_data_primary = PrepareData::new(pre_prepare_data.digest.clone(), state.config.node_id);
        assert!(!state.process_prepare(0, sequence, &prepare_data_primary).unwrap()); // Primary's prepare

        let prepare_data1 = PrepareData::new(pre_prepare_data.digest.clone(), Uuid::new_v4());
        assert!(!state.process_prepare(0, sequence, &prepare_data1).unwrap()); // Not enough yet

        let prepare_data2 = PrepareData::new(pre_prepare_data.digest.clone(), Uuid::new_v4());
        assert!(state.process_prepare(0, sequence, &prepare_data2).unwrap()); // Now we have quorum (3 total)

        // Process commit messages (including primary's commit)
        let commit_data_primary = CommitData::new(pre_prepare_data.digest.clone(), state.config.node_id);
        assert!(state.process_commit(0, sequence, &commit_data_primary).unwrap().is_none()); // Primary's commit

        let commit_data1 = CommitData::new(pre_prepare_data.digest.clone(), Uuid::new_v4());
        assert!(state.process_commit(0, sequence, &commit_data1).unwrap().is_none()); // Not enough yet

        let commit_data2 = CommitData::new(pre_prepare_data.digest.clone(), Uuid::new_v4());
        let committed_proposal = state.process_commit(0, sequence, &commit_data2).unwrap();
        assert!(committed_proposal.is_some()); // Now we have consensus (3 total)

        assert_eq!(state.last_committed, sequence);
    }

    #[test]
    fn test_view_change() {
        let mut state = create_test_state();

        // Start view change
        assert!(state.start_view_change(1).is_ok());
        assert!(state.view_change_in_progress);

        // Process view change votes
        // With 4 nodes, quorum is 3 (2f+1 where f=1)
        // start_view_change already added our own vote (1 vote)
        assert!(!state.process_view_change_vote(1, Uuid::new_v4()).unwrap()); // 2 votes - not enough
        assert!(state.process_view_change_vote(1, Uuid::new_v4()).unwrap()); // 3 votes - quorum reached

        // Complete view change
        assert!(state.complete_view_change(1).is_ok());
        assert_eq!(state.current_view, 1);
        assert!(!state.view_change_in_progress);
    }

    #[test]
    fn test_checkpoint() {
        let mut state = create_test_state();

        // Set up state to need checkpoint
        state.last_committed = 105; // Past checkpoint interval of 100
        state.last_checkpoint = 0;

        assert!(state.needs_checkpoint());

        let checkpoint = state.create_checkpoint();
        assert!(checkpoint.is_some());

        let (sequence, _hash) = checkpoint.unwrap();
        assert_eq!(sequence, 105);
        assert_eq!(state.last_checkpoint, 105);
    }

    #[test]
    fn test_error_conditions() {
        let mut state = create_test_state();

        // Try to start proposal when not primary
        state.current_view = 1; // Node 0 is not primary for view 1
        let proposal = Proposal::new("test".to_string(), b"data".to_vec());

        match state.start_proposal(proposal) {
            Err(ConsensusError::NotPrimary { .. }) => {} // Expected
            _ => panic!("Should have failed with NotPrimary"),
        }

        // Try to add proposal with wrong view
        let pre_prepare_data = PrePrepareData::new(
            Proposal::new("test".to_string(), b"data".to_vec()),
            state.config.node_id,
        );

        match state.add_proposal(0, 1, &pre_prepare_data) {
            Err(ConsensusError::InvalidView { .. }) => {} // Expected
            _ => panic!("Should have failed with InvalidView"),
        }
    }
}