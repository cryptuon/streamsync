//! Main consensus engine implementation

use crate::{
    config::Config,
    error::{ConsensusError, Result},
    messages::{ConsensusMessage, MessageBuilder, MessageType, PrePrepareData, PrepareData, CommitData, ViewChangeData},
    state::ConsensusState,
    transport::{Transport, Message},
    types::{Proposal, ConsensusResult, ConsensusStats, Phase},
};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast, Mutex};
use tokio::time::{timeout, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Main consensus engine
pub struct ConsensusEngine<T: Transport> {
    /// Configuration
    config: Config,

    /// Transport layer
    transport: Arc<Mutex<T>>,

    /// Consensus state
    state: Arc<RwLock<ConsensusState>>,

    /// Message builder
    message_builder: MessageBuilder,

    /// Channel for receiving results
    result_tx: broadcast::Sender<ConsensusResult>,

    /// Channel for internal commands
    command_tx: mpsc::UnboundedSender<EngineCommand>,
    command_rx: Arc<Mutex<mpsc::UnboundedReceiver<EngineCommand>>>,

    /// Running state
    running: Arc<RwLock<bool>>,

    /// Start time for statistics
    start_time: Instant,
}

/// Internal commands for the consensus engine
#[derive(Debug)]
enum EngineCommand {
    Propose(Proposal, tokio::sync::oneshot::Sender<Result<ConsensusResult>>),
    Stop,
    TriggerViewChange,
    CreateCheckpoint,
}

impl<T: Transport + 'static> ConsensusEngine<T> {
    /// Create a new consensus engine
    pub async fn new(config: Config, transport: T) -> Result<Self> {
        config.validate()?;

        let (result_tx, _) = broadcast::channel(1000);
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let state = Arc::new(RwLock::new(ConsensusState::new(config.clone())));
        let message_builder = MessageBuilder::new(config.node_id);

        Ok(Self {
            config,
            transport: Arc::new(Mutex::new(transport)),
            state,
            message_builder,
            result_tx,
            command_tx,
            command_rx: Arc::new(Mutex::new(command_rx)),
            running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        })
    }

    /// Start the consensus engine
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(ConsensusError::AlreadyRunning);
        }

        info!("🏛️ Starting consensus engine for node {}", self.config.node_id);

        // Start transport
        self.transport.lock().await.start().await?;

        *running = true;

        // Start background tasks
        self.start_message_loop().await;
        self.start_command_loop().await;
        self.start_maintenance_loop().await;

        info!("✅ Consensus engine started");
        Ok(())
    }

    /// Stop the consensus engine
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(ConsensusError::NotRunning);
        }

        info!("🛑 Stopping consensus engine");

        // Send stop command
        let _ = self.command_tx.send(EngineCommand::Stop);

        // Stop transport
        self.transport.lock().await.stop().await?;

        *running = false;

        info!("✅ Consensus engine stopped");
        Ok(())
    }

    /// Propose a value for consensus
    pub async fn propose(&self, proposal: Proposal) -> Result<ConsensusResult> {
        if !*self.running.read().await {
            return Err(ConsensusError::NotRunning);
        }

        // Check if we're the primary
        let state = self.state.read().await;
        if !state.is_primary() {
            return Err(ConsensusError::NotPrimary {
                node_id: self.config.node_id,
                view: state.current_view,
            });
        }
        drop(state);

        // Send command and wait for result
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.command_tx.send(EngineCommand::Propose(proposal, tx))
            .map_err(|_| ConsensusError::Internal {
                message: "Failed to send propose command".to_string(),
            })?;

        // Wait for result with timeout
        timeout(self.config.request_timeout, rx)
            .await
            .map_err(|_| ConsensusError::Timeout {
                timeout_ms: self.config.request_timeout.as_millis() as u64,
            })?
            .map_err(|_| ConsensusError::Internal {
                message: "Propose command cancelled".to_string(),
            })?
    }

    /// Subscribe to consensus results
    pub fn subscribe(&self) -> broadcast::Receiver<ConsensusResult> {
        self.result_tx.subscribe()
    }

    /// Get current statistics
    pub async fn stats(&self) -> ConsensusStats {
        self.state.read().await.get_stats()
    }

    /// Trigger a view change
    pub async fn trigger_view_change(&self) -> Result<()> {
        if !*self.running.read().await {
            return Err(ConsensusError::NotRunning);
        }

        self.command_tx.send(EngineCommand::TriggerViewChange)
            .map_err(|_| ConsensusError::Internal {
                message: "Failed to send view change command".to_string(),
            })?;

        Ok(())
    }

    /// Force a checkpoint
    pub async fn create_checkpoint(&self) -> Result<()> {
        if !*self.running.read().await {
            return Err(ConsensusError::NotRunning);
        }

        self.command_tx.send(EngineCommand::CreateCheckpoint)
            .map_err(|_| ConsensusError::Internal {
                message: "Failed to send checkpoint command".to_string(),
            })?;

        Ok(())
    }

    /// Check if the engine is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get current view
    pub async fn current_view(&self) -> u64 {
        self.state.read().await.current_view
    }

    /// Get last committed sequence
    pub async fn last_committed(&self) -> u64 {
        self.state.read().await.last_committed
    }

    // Background task loops

    async fn start_message_loop(&self) {
        let transport = self.transport.clone();
        let state = self.state.clone();
        let result_tx = self.result_tx.clone();
        let running = self.running.clone();
        let config = self.config.clone();
        let message_builder = MessageBuilder::new(self.config.node_id);

        tokio::spawn(async move {
            while *running.read().await {
                // Receive message from transport
                let message = match transport.lock().await.receive().await {
                    Ok(msg) => msg,
                    Err(ConsensusError::Timeout { .. }) => continue, // Normal timeout
                    Err(e) => {
                        error!("Transport receive error: {}", e);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };

                // Process message
                if let Err(e) = Self::handle_message(
                    &state,
                    &transport,
                    &result_tx,
                    &config,
                    &message_builder,
                    message,
                ).await {
                    warn!("Failed to handle message: {}", e);
                }
            }
        });
    }

    async fn start_command_loop(&self) {
        let command_rx = self.command_rx.clone();
        let state = self.state.clone();
        let transport = self.transport.clone();
        let running = self.running.clone();
        let config = self.config.clone();
        let message_builder = MessageBuilder::new(self.config.node_id);

        tokio::spawn(async move {
            let mut rx = command_rx.lock().await;

            while let Some(command) = rx.recv().await {
                match command {
                    EngineCommand::Propose(proposal, response_tx) => {
                        let result = Self::handle_propose(
                            &state,
                            &transport,
                            &config,
                            &message_builder,
                            proposal,
                        ).await;

                        let _ = response_tx.send(result);
                    }
                    EngineCommand::Stop => {
                        info!("Received stop command");
                        break;
                    }
                    EngineCommand::TriggerViewChange => {
                        if let Err(e) = Self::handle_view_change(&state, &transport, &config, &message_builder).await {
                            warn!("Failed to trigger view change: {}", e);
                        }
                    }
                    EngineCommand::CreateCheckpoint => {
                        if let Err(e) = Self::handle_checkpoint(&state, &transport, &config, &message_builder).await {
                            warn!("Failed to create checkpoint: {}", e);
                        }
                    }
                }
            }
        });
    }

    async fn start_maintenance_loop(&self) {
        let state = self.state.clone();
        let running = self.running.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(Duration::from_secs(1)).await;

                // Cleanup expired proposals
                let expired = state.write().await.cleanup_expired_proposals();
                if !expired.is_empty() {
                    debug!("Cleaned up {} expired proposals", expired.len());
                }

                // Check if we need a checkpoint
                if state.read().await.needs_checkpoint() {
                    debug!("Automatic checkpoint needed");
                    // Trigger checkpoint via command
                }
            }
        });
    }

    // Message handling

    async fn handle_message(
        state: &Arc<RwLock<ConsensusState>>,
        transport: &Arc<Mutex<T>>,
        result_tx: &broadcast::Sender<ConsensusResult>,
        config: &Config,
        message_builder: &MessageBuilder,
        message: Message,
    ) -> Result<()> {
        let consensus_message = message.payload;

        debug!(
            "Handling {} message from {} (view: {}, seq: {})",
            format!("{:?}", consensus_message.message_type),
            message.from,
            consensus_message.view,
            consensus_message.sequence
        );

        // Verify signature
        if !consensus_message.verify_signature(message.from) {
            return Err(ConsensusError::InvalidSignature { node_id: message.from });
        }

        // Verify sender is a participant
        if !config.is_participant(message.from) {
            return Err(ConsensusError::NodeNotParticipant { node_id: message.from });
        }

        match consensus_message.message_type {
            MessageType::PrePrepare => {
                Self::handle_pre_prepare(state, transport, config, message_builder, consensus_message).await
            }
            MessageType::Prepare => {
                Self::handle_prepare(state, transport, config, message_builder, consensus_message).await
            }
            MessageType::Commit => {
                Self::handle_commit(state, transport, config, message_builder, result_tx, consensus_message).await
            }
            MessageType::ViewChange => {
                Self::handle_view_change_message(state, transport, config, message_builder, consensus_message).await
            }
            MessageType::NewView => {
                Self::handle_new_view(state, consensus_message).await
            }
            MessageType::Checkpoint => {
                Self::handle_checkpoint_message(state, consensus_message).await
            }
        }
    }

    async fn handle_propose(
        state: &Arc<RwLock<ConsensusState>>,
        transport: &Arc<Mutex<T>>,
        config: &Config,
        message_builder: &MessageBuilder,
        proposal: Proposal,
    ) -> Result<ConsensusResult> {
        let mut state_guard = state.write().await;

        // Start the proposal
        let sequence = state_guard.start_proposal(proposal.clone())?;
        let view = state_guard.current_view;

        // Create and send pre-prepare message
        let pre_prepare = message_builder.pre_prepare(view, sequence, proposal.clone());
        drop(state_guard);

        let message = Message::broadcast(config.node_id, pre_prepare);
        transport.lock().await.send(message).await?;

        // Process our own pre-prepare
        let pre_prepare_data = PrePrepareData::new(proposal.clone(), config.node_id);
        state.write().await.add_proposal(view, sequence, &pre_prepare_data)?;

        // Wait for consensus (simplified - in practice would use proper waiting mechanism)
        let timeout_duration = config.request_timeout;
        let start = Instant::now();

        while start.elapsed() < timeout_duration {
            tokio::time::sleep(Duration::from_millis(10)).await;

            let state_guard = state.read().await;
            if let Some(proposal_state) = state_guard.get_proposal(sequence) {
                if proposal_state.phase == Phase::Committed {
                    let result = ConsensusResult::new(
                        proposal,
                        sequence,
                        view,
                        config.participants.clone(),
                    );
                    return Ok(result);
                }
            } else {
                // Proposal was committed and removed
                let result = ConsensusResult::new(
                    proposal,
                    sequence,
                    view,
                    config.participants.clone(),
                );
                return Ok(result);
            }
        }

        Err(ConsensusError::Timeout { timeout_ms: timeout_duration.as_millis() as u64 })
    }

    async fn handle_pre_prepare(
        state: &Arc<RwLock<ConsensusState>>,
        transport: &Arc<Mutex<T>>,
        config: &Config,
        message_builder: &MessageBuilder,
        message: ConsensusMessage,
    ) -> Result<()> {
        let pre_prepare_data = PrePrepareData::from_bytes(&message.data)
            .ok_or_else(|| ConsensusError::Internal {
                message: "Failed to deserialize pre-prepare data".to_string(),
            })?;

        // Verify this came from the primary
        let expected_primary = config.primary_for_view(message.view)
            .ok_or_else(|| ConsensusError::InvalidView {
                expected: 0,
                actual: message.view,
            })?;

        if pre_prepare_data.primary != expected_primary {
            return Err(ConsensusError::NotPrimary {
                node_id: pre_prepare_data.primary,
                view: message.view,
            });
        }

        // Add to state
        state.write().await.add_proposal(message.view, message.sequence, &pre_prepare_data)?;

        // Send prepare message
        let prepare = message_builder.prepare(message.view, message.sequence, pre_prepare_data.digest);
        let prepare_message = Message::broadcast(config.node_id, prepare);
        transport.lock().await.send(prepare_message).await?;

        Ok(())
    }

    async fn handle_prepare(
        state: &Arc<RwLock<ConsensusState>>,
        transport: &Arc<Mutex<T>>,
        config: &Config,
        message_builder: &MessageBuilder,
        message: ConsensusMessage,
    ) -> Result<()> {
        let prepare_data = PrepareData::from_bytes(&message.data)
            .ok_or_else(|| ConsensusError::Internal {
                message: "Failed to deserialize prepare data".to_string(),
            })?;

        // Process prepare
        let should_commit = state.write().await.process_prepare(
            message.view,
            message.sequence,
            &prepare_data,
        )?;

        // Send commit if we have enough prepares
        if should_commit {
            let commit = message_builder.commit(message.view, message.sequence, prepare_data.digest);
            let commit_message = Message::broadcast(config.node_id, commit);
            transport.lock().await.send(commit_message).await?;
        }

        Ok(())
    }

    async fn handle_commit(
        state: &Arc<RwLock<ConsensusState>>,
        _transport: &Arc<Mutex<T>>,
        config: &Config,
        _message_builder: &MessageBuilder,
        result_tx: &broadcast::Sender<ConsensusResult>,
        message: ConsensusMessage,
    ) -> Result<()> {
        let commit_data = CommitData::from_bytes(&message.data)
            .ok_or_else(|| ConsensusError::Internal {
                message: "Failed to deserialize commit data".to_string(),
            })?;

        // Process commit
        if let Some(committed_proposal) = state.write().await.process_commit(
            message.view,
            message.sequence,
            &commit_data,
        )? {
            // Consensus reached!
            let result = ConsensusResult::new(
                committed_proposal,
                message.sequence,
                message.view,
                config.participants.clone(),
            );

            // Broadcast result
            let _ = result_tx.send(result);
        }

        Ok(())
    }

    async fn handle_view_change(
        state: &Arc<RwLock<ConsensusState>>,
        transport: &Arc<Mutex<T>>,
        config: &Config,
        message_builder: &MessageBuilder,
    ) -> Result<()> {
        let new_view = {
            let state_guard = state.read().await;
            state_guard.current_view + 1
        };

        // Start view change
        state.write().await.start_view_change(new_view)?;

        // Send view change message
        let view_change = message_builder.view_change(new_view, 0); // Simplified
        let message = Message::broadcast(config.node_id, view_change);
        transport.lock().await.send(message).await?;

        info!("🔄 Initiated view change to view {}", new_view);
        Ok(())
    }

    async fn handle_view_change_message(
        state: &Arc<RwLock<ConsensusState>>,
        _transport: &Arc<Mutex<T>>,
        _config: &Config,
        _message_builder: &MessageBuilder,
        message: ConsensusMessage,
    ) -> Result<()> {
        let view_change_data = ViewChangeData::from_bytes(&message.data)
            .ok_or_else(|| ConsensusError::Internal {
                message: "Failed to deserialize view change data".to_string(),
            })?;

        // Process view change vote
        let has_quorum = state.write().await.process_view_change_vote(
            view_change_data.new_view,
            view_change_data.node,
        )?;

        if has_quorum {
            // Complete view change
            state.write().await.complete_view_change(view_change_data.new_view)?;
            info!("✅ Completed view change to view {}", view_change_data.new_view);
        }

        Ok(())
    }

    async fn handle_new_view(
        state: &Arc<RwLock<ConsensusState>>,
        message: ConsensusMessage,
    ) -> Result<()> {
        // Simplified new view handling
        state.write().await.complete_view_change(message.view)?;
        Ok(())
    }

    async fn handle_checkpoint(
        state: &Arc<RwLock<ConsensusState>>,
        transport: &Arc<Mutex<T>>,
        config: &Config,
        message_builder: &MessageBuilder,
    ) -> Result<()> {
        if let Some((sequence, state_hash)) = state.write().await.create_checkpoint() {
            let checkpoint = message_builder.checkpoint(sequence, state_hash);
            let message = Message::broadcast(config.node_id, checkpoint);
            transport.lock().await.send(message).await?;

            info!("📍 Created checkpoint at sequence {}", sequence);
        }

        Ok(())
    }

    async fn handle_checkpoint_message(
        _state: &Arc<RwLock<ConsensusState>>,
        _message: ConsensusMessage,
    ) -> Result<()> {
        // Simplified checkpoint message handling
        debug!("Received checkpoint message");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{InMemoryTransport, MockTransport};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_consensus_engine_creation() {
        let config = Config::test_config(4);
        let transport = MockTransport::new(config.node_id, config.participants.clone());
        let engine = ConsensusEngine::new(config, transport).await.unwrap();

        assert!(!engine.is_running().await);
    }

    #[tokio::test]
    async fn test_start_stop() {
        let config = Config::test_config(4);
        let transport = MockTransport::new(config.node_id, config.participants.clone());
        let engine = ConsensusEngine::new(config, transport).await.unwrap();

        // Start engine
        assert!(engine.start().await.is_ok());
        assert!(engine.is_running().await);

        // Starting again should fail
        assert!(matches!(engine.start().await, Err(ConsensusError::AlreadyRunning)));

        // Stop engine
        assert!(engine.stop().await.is_ok());
        assert!(!engine.is_running().await);

        // Stopping again should fail
        assert!(matches!(engine.stop().await, Err(ConsensusError::NotRunning)));
    }

    #[tokio::test]
    async fn test_propose_when_not_primary() {
        let mut config = Config::test_config(4);
        config.node_id = config.participants[1]; // Not primary for view 0

        let transport = MockTransport::new(config.node_id, config.participants.clone());
        let engine = ConsensusEngine::new(config, transport).await.unwrap();

        engine.start().await.unwrap();

        let proposal = Proposal::new("test".to_string(), b"data".to_vec());

        // Should fail because we're not primary
        match engine.propose(proposal).await {
            Err(ConsensusError::NotPrimary { .. }) => {} // Expected
            _ => panic!("Should have failed with NotPrimary"),
        }

        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_subscribe_to_results() {
        let config = Config::test_config(4);
        let transport = MockTransport::new(config.node_id, config.participants.clone());
        let engine = ConsensusEngine::new(config, transport).await.unwrap();

        let mut receiver = engine.subscribe();

        engine.start().await.unwrap();

        // Try to receive (should timeout since no consensus)
        tokio::select! {
            _ = receiver.recv() => panic!("Should not receive anything"),
            _ = tokio::time::sleep(Duration::from_millis(100)) => {} // Expected timeout
        }

        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_statistics() {
        let config = Config::test_config(4);
        let transport = MockTransport::new(config.node_id, config.participants.clone());
        let engine = ConsensusEngine::new(config, transport).await.unwrap();

        let stats = engine.stats().await;
        assert_eq!(stats.current_view, 0);
        assert_eq!(stats.current_sequence, 0);
        assert_eq!(stats.total_committed, 0);
        assert_eq!(stats.participant_count, 4);
    }
}