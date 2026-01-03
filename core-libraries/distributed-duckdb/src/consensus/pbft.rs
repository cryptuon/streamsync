//! PBFT Consensus Engine Implementation

use super::{
    ConsensusConfig, ConsensusProposal, ConsensusResult, ConsensusError, ConsensusStats, NodeRole,
    state::ConsensusState,
    message_log::MessageLog,
    view_change::ViewChangeManager,
};
use crate::network::protocol::{ConsensusMessage, NetworkMessage, PreparedProof, ViewChangeProof};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio::time::{timeout, Duration, Instant};
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, warn, debug};

/// PBFT Consensus Engine
pub struct PBFTConsensus {
    config: ConsensusConfig,
    state: Arc<RwLock<ConsensusState>>,
    message_log: Arc<RwLock<MessageLog>>,
    view_change_manager: Arc<RwLock<ViewChangeManager>>,

    // Communication channels
    message_tx: mpsc::UnboundedSender<NetworkMessage>,
    proposal_tx: broadcast::Sender<ConsensusResult>,

    // Runtime state
    running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ConsensusStats>>,
    start_time: Instant,
}

impl PBFTConsensus {
    /// Create a new PBFT consensus engine
    pub fn new(
        config: ConsensusConfig,
        message_tx: mpsc::UnboundedSender<NetworkMessage>,
    ) -> Result<Self> {
        if !config.is_valid() {
            return Err(ConsensusError::InsufficientNodes.into());
        }

        let (proposal_tx, _) = broadcast::channel(1000);

        let state = Arc::new(RwLock::new(ConsensusState::new(config.clone())));
        let message_log = Arc::new(RwLock::new(MessageLog::new()));
        let view_change_manager = Arc::new(RwLock::new(ViewChangeManager::new(config.clone())));

        let stats = Arc::new(RwLock::new(ConsensusStats {
            current_view: 0,
            current_sequence: 0,
            total_committed: 0,
            view_changes: 0,
            failed_proposals: 0,
            average_commit_time_ms: 0.0,
            participating_nodes: config.participants.len(),
            last_checkpoint: 0,
            uptime_seconds: 0,
        }));

        Ok(Self {
            config,
            state,
            message_log,
            view_change_manager,
            message_tx,
            proposal_tx,
            running: Arc::new(RwLock::new(false)),
            stats,
            start_time: Instant::now(),
        })
    }

    /// Start the consensus engine
    pub async fn start(&mut self) -> Result<()> {
        info!("🏛️ Starting PBFT consensus engine for node {}", self.config.node_id);

        *self.running.write().await = true;

        // Start consensus processing tasks
        self.start_consensus_loop().await;
        self.start_view_change_timer().await;
        self.start_checkpoint_manager().await;
        self.start_stats_updater().await;

        info!("✅ PBFT consensus engine started");
        Ok(())
    }

    /// Stop the consensus engine
    pub async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping PBFT consensus engine");

        *self.running.write().await = false;

        info!("✅ PBFT consensus engine stopped");
        Ok(())
    }

    /// Submit a proposal for consensus
    pub async fn propose(&self, proposal: ConsensusProposal) -> Result<ConsensusResult> {
        let state = self.state.read().await;

        // Only primary can propose in current view
        if !self.config.is_primary(state.current_view) {
            return Err(ConsensusError::NodeNotInView.into());
        }

        let sequence = state.current_sequence + 1;
        let view = state.current_view;

        drop(state);

        // Create and send pre-prepare message
        self.send_pre_prepare(view, sequence, proposal.clone()).await?;

        // Wait for consensus to complete
        let result = self.wait_for_consensus(sequence).await?;
        Ok(result)
    }

    /// Handle incoming consensus message
    pub async fn handle_message(&self, message: ConsensusMessage) -> Result<()> {
        debug!("📨 Handling consensus message: {:?}", message);

        match message {
            ConsensusMessage::PrePrepare { view, sequence, digest, proposal, primary } => {
                self.handle_pre_prepare(view, sequence, digest, proposal, primary).await
            }
            ConsensusMessage::Prepare { view, sequence, digest, node_id } => {
                self.handle_prepare(view, sequence, digest, node_id).await
            }
            ConsensusMessage::Commit { view, sequence, digest, node_id } => {
                self.handle_commit(view, sequence, digest, node_id).await
            }
            ConsensusMessage::ViewChange { new_view, node_id, prepared_proofs } => {
                self.handle_view_change(new_view, node_id, prepared_proofs).await
            }
            ConsensusMessage::NewView { view, view_change_proofs, primary } => {
                self.handle_new_view(view, view_change_proofs, primary).await
            }
            ConsensusMessage::Checkpoint { sequence, state_hash, node_id } => {
                self.handle_checkpoint(sequence, state_hash, node_id).await
            }
        }
    }

    /// Get current consensus statistics
    pub async fn get_stats(&self) -> ConsensusStats {
        let mut stats = self.stats.read().await.clone();
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
        stats
    }

    /// Subscribe to consensus results
    pub fn subscribe_to_results(&self) -> broadcast::Receiver<ConsensusResult> {
        self.proposal_tx.subscribe()
    }

    /// Get current node role
    pub async fn get_role(&self) -> NodeRole {
        let state = self.state.read().await;
        if self.config.is_primary(state.current_view) {
            NodeRole::Primary
        } else {
            NodeRole::Backup
        }
    }

    /// Force a view change
    pub async fn trigger_view_change(&self) -> Result<()> {
        info!("🔄 Triggering view change from node {}", self.config.node_id);

        let mut state = self.state.write().await;
        let new_view = state.current_view + 1;

        // Create view change message
        let prepared_proofs = self.collect_prepared_proofs(&state).await;

        let view_change = ConsensusMessage::ViewChange {
            new_view,
            node_id: self.config.node_id,
            prepared_proofs,
        };

        self.send_consensus_message(view_change).await?;

        state.current_view = new_view;
        state.view_change_in_progress = true;

        Ok(())
    }

    // Private implementation methods

    async fn send_pre_prepare(&self, view: u64, sequence: u64, proposal: ConsensusProposal) -> Result<()> {
        let digest = self.compute_digest(&proposal).await;

        let pre_prepare = ConsensusMessage::PrePrepare {
            view,
            sequence,
            digest: digest.clone(),
            proposal: proposal.clone(),
            primary: self.config.node_id,
        };

        // Log the pre-prepare
        self.message_log.write().await.add_pre_prepare(view, sequence, digest, proposal);

        // Send to all backup nodes
        self.send_consensus_message(pre_prepare).await?;

        // Automatically send prepare message from primary
        self.send_prepare(view, sequence).await?;

        Ok(())
    }

    async fn send_prepare(&self, view: u64, sequence: u64) -> Result<()> {
        let message_log = self.message_log.read().await;
        if let Some(digest) = message_log.get_digest(view, sequence) {
            let prepare = ConsensusMessage::Prepare {
                view,
                sequence,
                digest,
                node_id: self.config.node_id,
            };

            self.send_consensus_message(prepare).await?;
        }

        Ok(())
    }

    async fn send_commit(&self, view: u64, sequence: u64) -> Result<()> {
        let message_log = self.message_log.read().await;
        if let Some(digest) = message_log.get_digest(view, sequence) {
            let commit = ConsensusMessage::Commit {
                view,
                sequence,
                digest,
                node_id: self.config.node_id,
            };

            self.send_consensus_message(commit).await?;
        }

        Ok(())
    }

    async fn handle_pre_prepare(&self, view: u64, sequence: u64, digest: String, proposal: ConsensusProposal, primary: Uuid) -> Result<()> {
        let state = self.state.write().await;

        // Validate view and sequence
        if view != state.current_view {
            warn!("Received pre-prepare for wrong view: {} (current: {})", view, state.current_view);
            return Ok(());
        }

        if sequence <= state.last_committed {
            warn!("Received pre-prepare for old sequence: {} (last committed: {})", sequence, state.last_committed);
            return Ok(());
        }

        // Verify primary
        if self.config.primary_for_view(view) != Some(primary) {
            warn!("Received pre-prepare from non-primary node: {}", primary);
            return Ok(());
        }

        // Verify digest
        let computed_digest = self.compute_digest(&proposal).await;
        if digest != computed_digest {
            warn!("Pre-prepare digest mismatch");
            return Ok(());
        }

        drop(state);

        // Store the pre-prepare
        self.message_log.write().await.add_pre_prepare(view, sequence, digest, proposal);

        // Send prepare message
        self.send_prepare(view, sequence).await?;

        Ok(())
    }

    async fn handle_prepare(&self, view: u64, sequence: u64, digest: String, node_id: Uuid) -> Result<()> {
        let state = self.state.read().await;

        // Validate view
        if view != state.current_view {
            return Ok(());
        }

        drop(state);

        // Add prepare to message log
        self.message_log.write().await.add_prepare(view, sequence, digest.clone(), node_id);

        // Check if we have enough prepares
        let message_log = self.message_log.read().await;
        let prepare_count = message_log.count_prepares(view, sequence, &digest);

        if prepare_count >= self.config.quorum_size() {
            drop(message_log);
            // Send commit
            self.send_commit(view, sequence).await?;
        }

        Ok(())
    }

    async fn handle_commit(&self, view: u64, sequence: u64, digest: String, node_id: Uuid) -> Result<()> {
        let state = self.state.read().await;

        // Validate view
        if view != state.current_view {
            return Ok(());
        }

        drop(state);

        // Add commit to message log
        self.message_log.write().await.add_commit(view, sequence, digest.clone(), node_id);

        // Check if we have enough commits
        let message_log = self.message_log.read().await;
        let commit_count = message_log.count_commits(view, sequence, &digest);

        if commit_count >= self.config.quorum_size() {
            // Execute the proposal
            if let Some(proposal) = message_log.get_proposal(view, sequence) {
                let result = ConsensusResult {
                    sequence,
                    view,
                    proposal,
                    committed_at: Utc::now(),
                    participating_nodes: self.config.participants.clone(),
                };

                drop(message_log);

                // Update state
                let mut state = self.state.write().await;
                state.last_committed = sequence;
                state.current_sequence = sequence;

                // Broadcast result
                let _ = self.proposal_tx.send(result);

                // Update stats
                let mut stats = self.stats.write().await;
                stats.total_committed += 1;
                stats.current_sequence = sequence;
            }
        }

        Ok(())
    }

    async fn handle_view_change(&self, new_view: u64, node_id: Uuid, prepared_proofs: Vec<PreparedProof>) -> Result<()> {
        info!("🔄 Handling view change to {} from {}", new_view, node_id);

        let mut view_change_manager = self.view_change_manager.write().await;
        view_change_manager.add_view_change(new_view, node_id, prepared_proofs);

        // Check if we have enough view changes
        if view_change_manager.has_quorum(new_view, self.config.quorum_size()) {
            // We are the new primary, send new-view message
            if self.config.is_primary(new_view) {
                let view_change_proofs = view_change_manager.get_view_change_proofs(new_view);

                let new_view_msg = ConsensusMessage::NewView {
                    view: new_view,
                    view_change_proofs,
                    primary: self.config.node_id,
                };

                self.send_consensus_message(new_view_msg).await?;
            }
        }

        Ok(())
    }

    async fn handle_new_view(&self, view: u64, _view_change_proofs: Vec<ViewChangeProof>, primary: Uuid) -> Result<()> {
        info!("🆕 Handling new view {} with primary {}", view, primary);

        // Verify primary
        if self.config.primary_for_view(view) != Some(primary) {
            warn!("Invalid primary for new view: {}", primary);
            return Ok(());
        }

        // Update state to new view
        let mut state = self.state.write().await;
        state.current_view = view;
        state.view_change_in_progress = false;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.current_view = view;
        stats.view_changes += 1;

        Ok(())
    }

    async fn handle_checkpoint(&self, sequence: u64, state_hash: String, node_id: Uuid) -> Result<()> {
        debug!("📍 Handling checkpoint at sequence {} from {}", sequence, node_id);

        // Add checkpoint to message log
        self.message_log.write().await.add_checkpoint(sequence, state_hash, node_id);

        // Check if we have enough checkpoint messages
        let message_log = self.message_log.read().await;
        let checkpoint_count = message_log.count_checkpoints(sequence);

        if checkpoint_count >= self.config.quorum_size() {
            // Stable checkpoint reached
            drop(message_log);

            let mut state = self.state.write().await;
            state.last_checkpoint = sequence;

            // Clean up old messages
            self.message_log.write().await.cleanup_before_sequence(sequence);

            let mut stats = self.stats.write().await;
            stats.last_checkpoint = sequence;

            info!("✅ Stable checkpoint reached at sequence {}", sequence);
        }

        Ok(())
    }

    async fn send_consensus_message(&self, message: ConsensusMessage) -> Result<()> {
        let network_message = NetworkMessage::Consensus(message);
        self.message_tx.send(network_message)
            .map_err(|e| anyhow::anyhow!("Failed to send consensus message: {}", e))?;
        Ok(())
    }

    async fn compute_digest(&self, proposal: &ConsensusProposal) -> String {
        use sha2::{Sha256, Digest};

        let data = bincode::serialize(proposal).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(&data);
        format!("{:x}", hasher.finalize())
    }

    async fn wait_for_consensus(&self, sequence: u64) -> Result<ConsensusResult> {
        let mut receiver = self.subscribe_to_results();

        let timeout_duration = Duration::from_millis(self.config.request_timeout_ms);

        match timeout(timeout_duration, async {
            while let Ok(result) = receiver.recv().await {
                if result.sequence == sequence {
                    return result;
                }
            }
            // This should never be reached
            unreachable!()
        }).await {
            Ok(result) => Ok(result),
            Err(_) => Err(ConsensusError::TimeoutExpired.into()),
        }
    }

    async fn collect_prepared_proofs(&self, _state: &ConsensusState) -> Vec<PreparedProof> {
        // Collect prepared proofs from message log
        // This is a simplified implementation
        vec![]
    }

    async fn start_consensus_loop(&self) {
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(Duration::from_millis(100)).await;
                // Main consensus processing loop
            }
        });
    }

    async fn start_view_change_timer(&self) {
        let running = self.running.clone();
        let view_change_timeout = Duration::from_millis(self.config.view_change_timeout_ms);

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(view_change_timeout).await;
                // Check if view change is needed
            }
        });
    }

    async fn start_checkpoint_manager(&self) {
        let running = self.running.clone();
        let _checkpoint_interval = self.config.checkpoint_interval;

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(Duration::from_secs(30)).await;
                // Periodic checkpoint creation
            }
        });
    }

    async fn start_stats_updater(&self) {
        let stats = self.stats.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(Duration::from_secs(10)).await;

                // Update statistics
                let _stats = stats.write().await;
                // Update rolling averages, etc.
            }
        });
    }
}