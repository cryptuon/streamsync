use anyhow::{anyhow, Result};
use async_trait::async_trait;
use consensus_core::{
    Config as ConsensusConfig, ConsensusEngine, ConsensusResult, Proposal, Transport,
    ConsensusError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::NodeConfig;

/// Types of decisions that require consensus in StreamSync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusDecision {
    /// Decision about which Solana slot to start indexing from
    IndexingStart { slot: u64 },
    /// Decision about node membership changes
    MembershipChange { node_id: Uuid, action: MembershipAction },
    /// Decision about data retention policies
    RetentionPolicy { retention_days: u32 },
    /// Decision about sharding configuration changes
    ShardingUpdate { config_hash: String },
    /// Decision about network upgrades
    NetworkUpgrade { version: u32, features: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MembershipAction {
    Join,
    Leave,
    Promote,
    Demote,
}

/// Statistics about consensus operations
#[derive(Debug, Clone)]
pub struct ConsensusStats {
    pub total_proposals: u64,
    pub successful_proposals: u64,
    pub failed_proposals: u64,
    pub view_changes: u64,
    pub average_consensus_time_ms: f64,
    pub current_view: u64,
    pub is_leader: bool,
    pub participants: Vec<Uuid>,
    pub last_decision: Option<ConsensusDecision>,
}

/// StreamSync consensus coordinator
pub struct ConsensusCoordinator {
    node_id: Uuid,
    config: NodeConfig,
    engine: Option<Arc<ConsensusEngine<NetworkTransport>>>,
    stats: Arc<RwLock<ConsensusStats>>,
    decision_sender: broadcast::Sender<ConsensusDecision>,
    running: Arc<RwLock<bool>>,
    participants: Arc<RwLock<Vec<Uuid>>>,
}

/// Network transport adapter for consensus
#[derive(Debug)]
pub struct NetworkTransport {
    node_id: Uuid,
    peers: Arc<RwLock<HashMap<Uuid, String>>>,
    // In a real implementation, this would integrate with the networking-core library
}

impl NetworkTransport {
    pub fn new(node_id: Uuid) -> Self {
        Self {
            node_id,
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_peer(&self, peer_id: Uuid, address: String) {
        let mut peers = self.peers.write().await;
        info!("Added consensus peer: {} at {}", peer_id, address);
        peers.insert(peer_id, address);
    }

    pub async fn remove_peer(&self, peer_id: &Uuid) {
        let mut peers = self.peers.write().await;
        if peers.remove(peer_id).is_some() {
            info!("Removed consensus peer: {}", peer_id);
        }
    }
}

#[async_trait]
impl Transport for NetworkTransport {
    async fn start(&mut self) -> consensus_core::Result<()> {
        debug!("Starting consensus transport");
        Ok(())
    }

    async fn stop(&mut self) -> consensus_core::Result<()> {
        debug!("Stopping consensus transport");
        Ok(())
    }

    async fn send(&self, message: consensus_core::transport::Message) -> consensus_core::Result<()> {
        debug!("Sending consensus message from {} to {:?}: {:?}",
               self.node_id, message.to, message.payload);

        // In a real implementation, this would send over the network
        // For now, we'll simulate successful delivery
        Ok(())
    }

    async fn receive(&self) -> consensus_core::Result<consensus_core::transport::Message> {
        // In a real implementation, this would receive from the network
        // For now, return a timeout error to indicate no messages
        Err(ConsensusError::Timeout { timeout_ms: 5000 })
    }

    async fn connected_peers(&self) -> Vec<uuid::Uuid> {
        let peers = self.peers.read().await;
        peers.keys().cloned().collect()
    }

    async fn is_connected(&self, peer: uuid::Uuid) -> bool {
        let peers = self.peers.read().await;
        peers.contains_key(&peer)
    }

    async fn stats(&self) -> consensus_core::transport::TransportStats {
        consensus_core::transport::TransportStats {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            send_failures: 0,
            receive_failures: 0,
            average_latency_ms: 0.0,
            connected_peers: self.connected_peers().await.len(),
        }
    }
}

impl ConsensusCoordinator {
    pub async fn new(config: NodeConfig) -> Result<Self> {
        let node_id = Uuid::parse_str(&config.node.id)?;

        let stats = ConsensusStats {
            total_proposals: 0,
            successful_proposals: 0,
            failed_proposals: 0,
            view_changes: 0,
            average_consensus_time_ms: 0.0,
            current_view: 0,
            is_leader: false,
            participants: vec![node_id],
            last_decision: None,
        };

        let (decision_sender, _) = broadcast::channel(1000);

        Ok(Self {
            node_id,
            config,
            engine: None,
            stats: Arc::new(RwLock::new(stats)),
            decision_sender,
            running: Arc::new(RwLock::new(false)),
            participants: Arc::new(RwLock::new(vec![node_id])),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(anyhow!("Consensus coordinator is already running"));
        }

        if !self.config.consensus.enable_consensus {
            info!("Consensus is disabled in configuration");
            return Ok(());
        }

        info!("Starting consensus coordinator");

        // Initialize participants from bootstrap nodes
        let participants = self.initialize_participants().await?;

        // Only start consensus if we have enough nodes
        if participants.len() >= 4 {
            // Create consensus configuration
            let consensus_config = ConsensusConfig::new(self.node_id, participants.clone())
                .with_request_timeout(std::time::Duration::from_millis(
                    self.config.consensus.request_timeout_ms,
                ))
                .with_view_change_timeout(std::time::Duration::from_millis(
                    self.config.consensus.view_timeout_ms,
                ))
                .with_checkpoint_interval(self.config.consensus.checkpoint_interval)
                .with_debug_logs(true);

            // Validate configuration
            consensus_config.validate().map_err(|e| anyhow!("Consensus config validation failed: {}", e))?;

            // Create transport
            let transport = NetworkTransport::new(self.node_id);

            // Create and start consensus engine
            let engine = ConsensusEngine::new(consensus_config.clone(), transport).await
                .map_err(|e| anyhow!("Failed to create consensus engine: {}", e))?;

            engine.start().await
                .map_err(|e| anyhow!("Failed to start consensus engine: {}", e))?;

            self.engine = Some(Arc::new(engine));
            info!("Consensus engine started with {} participants", participants.len());

            // Update stats
            let mut stats = self.stats.write().await;
            stats.participants = participants;
            stats.is_leader = consensus_config.primary_for_view(0) == Some(self.node_id); // Check if we're primary for view 0
        } else {
            warn!("Insufficient nodes for consensus ({} < 4), running in single-node mode", participants.len());
        }

        *running = true;
        info!("Consensus coordinator started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        info!("Stopping consensus coordinator");

        if let Some(engine) = &self.engine {
            engine.stop().await
                .map_err(|e| anyhow!("Failed to stop consensus engine: {}", e))?;
        }

        *running = false;
        info!("Consensus coordinator stopped");
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Propose a decision that requires consensus
    pub async fn propose_decision(&self, decision: ConsensusDecision) -> Result<ConsensusResult> {
        if let Some(engine) = &self.engine {
            let proposal_data = serde_json::to_vec(&decision)?;
            let proposal = Proposal::new(
                format!("decision_{}", Uuid::new_v4()),
                proposal_data,
            );

            info!("Proposing consensus decision: {:?}", decision);

            let start_time = std::time::Instant::now();
            let result = engine.propose(proposal).await
                .map_err(|e| anyhow!("Consensus proposal failed: {}", e))?;

            let consensus_time = start_time.elapsed().as_millis() as f64;

            // Update statistics
            self.update_stats(true, consensus_time, Some(decision.clone())).await;

            // Broadcast decision to subscribers
            let _ = self.decision_sender.send(decision);

            info!("Consensus reached in {:.2}ms: {:?}", consensus_time, result);
            Ok(result)
        } else {
            // Single-node mode - automatically approve
            info!("Single-node mode: auto-approving decision {:?}", decision);
            let _ = self.decision_sender.send(decision.clone());
            self.update_stats(true, 0.0, Some(decision.clone())).await;

            Ok(ConsensusResult {
                proposal: Proposal::new("single_node".to_string(), serde_json::to_vec(&decision)?),
                sequence: 0,
                view: 0,
                participants: vec![self.node_id],
                committed_at: chrono::Utc::now(),
            })
        }
    }

    /// Subscribe to consensus decisions
    pub fn subscribe_to_decisions(&self) -> broadcast::Receiver<ConsensusDecision> {
        self.decision_sender.subscribe()
    }

    /// Get consensus statistics
    pub async fn get_stats(&self) -> ConsensusStats {
        self.stats.read().await.clone()
    }

    /// Add a new participant to the consensus group
    pub async fn add_participant(&self, node_id: Uuid, _address: String) -> Result<()> {
        let mut participants = self.participants.write().await;
        if !participants.contains(&node_id) {
            participants.push(node_id);
            info!("Added consensus participant: {}", node_id);

            // If we have a transport, add the peer
            if let Some(_engine) = &self.engine {
                // In a real implementation, we would notify the engine about the new participant
                debug!("Notified consensus engine about new participant");
            }
        }
        Ok(())
    }

    /// Remove a participant from the consensus group
    pub async fn remove_participant(&self, node_id: &Uuid) -> Result<()> {
        let mut participants = self.participants.write().await;
        if let Some(pos) = participants.iter().position(|&id| id == *node_id) {
            participants.remove(pos);
            info!("Removed consensus participant: {}", node_id);
        }
        Ok(())
    }

    async fn initialize_participants(&self) -> Result<Vec<Uuid>> {
        let mut participants = vec![self.node_id];

        // Add bootstrap nodes if configured
        for _bootstrap_addr in &self.config.consensus.bootstrap_nodes {
            // In a real implementation, we would resolve these addresses to node IDs
            // For now, generate placeholder node IDs
            let node_id = Uuid::new_v4(); // Use v4 instead of v5 for bootstrap nodes
            if !participants.contains(&node_id) {
                participants.push(node_id);
                debug!("Added bootstrap consensus node: {}", node_id);
            }
        }

        *self.participants.write().await = participants.clone();
        Ok(participants)
    }

    async fn update_stats(&self, success: bool, consensus_time_ms: f64, decision: Option<ConsensusDecision>) {
        let mut stats = self.stats.write().await;
        stats.total_proposals += 1;

        if success {
            stats.successful_proposals += 1;
        } else {
            stats.failed_proposals += 1;
        }

        // Update average consensus time
        let total_successful = stats.successful_proposals;
        if total_successful > 0 {
            stats.average_consensus_time_ms = (stats.average_consensus_time_ms * (total_successful - 1) as f64
                + consensus_time_ms) / total_successful as f64;
        }

        stats.last_decision = decision;
    }
}

// Convenience functions for common consensus decisions
impl ConsensusCoordinator {
    /// Propose starting indexing from a specific slot
    pub async fn propose_indexing_start(&self, slot: u64) -> Result<ConsensusResult> {
        let decision = ConsensusDecision::IndexingStart { slot };
        self.propose_decision(decision).await
    }

    /// Propose a node joining the network
    pub async fn propose_node_join(&self, node_id: Uuid) -> Result<ConsensusResult> {
        let decision = ConsensusDecision::MembershipChange {
            node_id,
            action: MembershipAction::Join,
        };
        self.propose_decision(decision).await
    }

    /// Propose a node leaving the network
    pub async fn propose_node_leave(&self, node_id: Uuid) -> Result<ConsensusResult> {
        let decision = ConsensusDecision::MembershipChange {
            node_id,
            action: MembershipAction::Leave,
        };
        self.propose_decision(decision).await
    }

    /// Propose updating data retention policy
    pub async fn propose_retention_update(&self, retention_days: u32) -> Result<ConsensusResult> {
        let decision = ConsensusDecision::RetentionPolicy { retention_days };
        self.propose_decision(decision).await
    }

    /// Propose a network upgrade
    pub async fn propose_network_upgrade(&self, version: u32, features: Vec<String>) -> Result<ConsensusResult> {
        let decision = ConsensusDecision::NetworkUpgrade { version, features };
        self.propose_decision(decision).await
    }
}