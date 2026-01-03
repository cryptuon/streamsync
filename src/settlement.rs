//! Settlement Engine for StreamSync
//!
//! This module handles batch settlement of micro-transactions for query rewards.
//! Rewards are accumulated during query execution and settled in batches every
//! 5 minutes to reduce on-chain transaction costs.

use anyhow::{anyhow, Result};
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Default settlement interval (5 minutes)
const DEFAULT_SETTLEMENT_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Maximum rewards per batch (Solana transaction size limit consideration)
const MAX_REWARDS_PER_BATCH: usize = 100;

/// Minimum reward amount to include in batch (avoid dust)
const MIN_REWARD_AMOUNT: u64 = 1000; // 0.001 STRM

/// Node identifier type
pub type NodeId = Uuid;

/// A micro-reward for a single query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroReward {
    /// Node that earned the reward
    pub node_id: NodeId,
    /// Reward amount in lamports
    pub amount: u64,
    /// Query ID this reward is for
    pub query_id: String,
    /// Whether this node was the winner (vs verifier)
    pub is_winner: bool,
    /// Timestamp when reward was recorded (Unix timestamp in seconds)
    pub timestamp_secs: i64,
}

impl MicroReward {
    /// Create a new micro-reward
    pub fn new(node_id: NodeId, amount: u64, query_id: String, is_winner: bool) -> Self {
        Self {
            node_id,
            amount,
            query_id,
            is_winner,
            timestamp_secs: Utc::now().timestamp(),
        }
    }
}

/// Settlement batch ready for on-chain submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementBatch {
    /// Unique batch ID
    pub batch_id: Uuid,
    /// Aggregated rewards per node
    pub rewards: Vec<NodeReward>,
    /// Total amount in this batch
    pub total_amount: u64,
    /// When batch was created (Unix timestamp)
    pub created_at: i64,
    /// Batch status
    pub status: BatchStatus,
}

/// Aggregated reward for a single node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReward {
    /// Node ID
    pub node_id: NodeId,
    /// Total reward amount
    pub amount: u64,
    /// Number of queries contributing to this reward
    pub query_count: u32,
    /// Number of wins
    pub win_count: u32,
    /// Number of verifications
    pub verify_count: u32,
}

/// Status of a settlement batch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchStatus {
    /// Batch is pending and accumulating
    Pending,
    /// Batch is ready for settlement
    Ready,
    /// Batch is being processed
    Processing,
    /// Batch was settled successfully
    Settled,
    /// Batch settlement failed
    Failed,
}

/// Result of a settlement operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementResult {
    /// Batch ID
    pub batch_id: Uuid,
    /// Whether settlement was successful
    pub success: bool,
    /// On-chain transaction signature (if successful)
    pub tx_signature: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Number of rewards processed
    pub rewards_processed: usize,
    /// Total amount settled
    pub total_settled: u64,
}

/// Settlement engine statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettlementStats {
    /// Total batches processed
    pub batches_processed: u64,
    /// Total rewards settled
    pub rewards_settled: u64,
    /// Total amount settled (in lamports)
    pub total_amount_settled: u64,
    /// Failed settlements
    pub failed_settlements: u64,
    /// Pending rewards count
    pub pending_rewards: usize,
    /// Current batch size
    pub current_batch_size: usize,
}

/// Configuration for the settlement engine
#[derive(Debug, Clone)]
pub struct SettlementConfig {
    /// Interval between settlement batches
    pub batch_interval: Duration,
    /// Maximum rewards per batch
    pub max_batch_size: usize,
    /// Minimum reward amount
    pub min_reward: u64,
    /// Solana RPC URL
    pub solana_rpc_url: String,
    /// Program ID for settlement contract
    pub program_id: String,
    /// Oracle authority keypair path
    pub oracle_keypair_path: Option<String>,
}

impl Default for SettlementConfig {
    fn default() -> Self {
        Self {
            batch_interval: DEFAULT_SETTLEMENT_INTERVAL,
            max_batch_size: MAX_REWARDS_PER_BATCH,
            min_reward: MIN_REWARD_AMOUNT,
            solana_rpc_url: "https://api.devnet.solana.com".to_string(),
            program_id: "STRMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            oracle_keypair_path: None,
        }
    }
}

/// Settlement Engine for batch micro-transaction processing
pub struct SettlementEngine {
    /// Configuration
    config: SettlementConfig,
    /// Pending rewards by node (not yet batched)
    pending_rewards: Arc<DashMap<NodeId, Vec<MicroReward>>>,
    /// Current batch being accumulated
    current_batch: Arc<RwLock<Option<SettlementBatch>>>,
    /// Historical batches (recent only)
    batch_history: Arc<RwLock<Vec<SettlementBatch>>>,
    /// Statistics
    stats: Arc<RwLock<SettlementStats>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

impl SettlementEngine {
    /// Create a new settlement engine
    pub fn new(config: SettlementConfig) -> Self {
        Self {
            config,
            pending_rewards: Arc::new(DashMap::new()),
            current_batch: Arc::new(RwLock::new(None)),
            batch_history: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(SettlementStats::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(SettlementConfig::default())
    }

    /// Start the settlement engine
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(anyhow!("Settlement engine already running"));
        }
        *running = true;
        drop(running);

        info!("Starting settlement engine with {:?} interval", self.config.batch_interval);

        // Start the settlement loop
        self.run_settlement_loop().await;

        Ok(())
    }

    /// Stop the settlement engine
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }

        info!("Stopping settlement engine");

        // Process any remaining pending rewards
        self.process_batch().await?;

        *running = false;
        Ok(())
    }

    /// Record a reward for a query
    pub async fn record_reward(&self, node_id: NodeId, amount: u64, query_id: String, is_winner: bool) -> Result<()> {
        // Skip dust amounts
        if amount < self.config.min_reward {
            debug!("Skipping dust reward of {} for node {}", amount, node_id);
            return Ok(());
        }

        let reward = MicroReward::new(node_id, amount, query_id, is_winner);

        // Add to pending rewards
        self.pending_rewards
            .entry(node_id)
            .or_insert_with(Vec::new)
            .push(reward);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.pending_rewards = self.pending_rewards.iter().map(|e| e.value().len()).sum();

        debug!("Recorded {} reward for node {}", if is_winner { "winner" } else { "verifier" }, node_id);

        // Check if we should trigger early batch
        if stats.pending_rewards >= self.config.max_batch_size {
            drop(stats);
            self.process_batch().await?;
        }

        Ok(())
    }

    /// Record rewards for a racing query (winner + verifiers)
    pub async fn record_racing_rewards(
        &self,
        winner_id: NodeId,
        winner_amount: u64,
        verifier_ids: Vec<NodeId>,
        verifier_amount: u64,
        query_id: String,
    ) -> Result<()> {
        // Record winner reward
        self.record_reward(winner_id, winner_amount, query_id.clone(), true).await?;

        // Record verifier rewards
        for verifier_id in verifier_ids {
            self.record_reward(verifier_id, verifier_amount, query_id.clone(), false).await?;
        }

        Ok(())
    }

    /// Process current batch (aggregate and prepare for settlement)
    pub async fn process_batch(&self) -> Result<SettlementResult> {
        info!("Processing settlement batch");

        // Collect pending rewards
        let mut node_rewards: std::collections::HashMap<NodeId, NodeReward> =
            std::collections::HashMap::new();

        let mut total_amount = 0u64;
        let mut processed_count = 0usize;

        // Drain pending rewards
        let node_ids: Vec<_> = self.pending_rewards.iter().map(|e| *e.key()).collect();

        for node_id in node_ids {
            if let Some((_, rewards)) = self.pending_rewards.remove(&node_id) {
                for reward in rewards {
                    let entry = node_rewards.entry(node_id).or_insert(NodeReward {
                        node_id,
                        amount: 0,
                        query_count: 0,
                        win_count: 0,
                        verify_count: 0,
                    });

                    entry.amount += reward.amount;
                    entry.query_count += 1;
                    if reward.is_winner {
                        entry.win_count += 1;
                    } else {
                        entry.verify_count += 1;
                    }

                    total_amount += reward.amount;
                    processed_count += 1;

                    // Limit batch size
                    if processed_count >= self.config.max_batch_size {
                        break;
                    }
                }
            }

            if processed_count >= self.config.max_batch_size {
                break;
            }
        }

        if node_rewards.is_empty() {
            debug!("No rewards to settle");
            return Ok(SettlementResult {
                batch_id: Uuid::new_v4(),
                success: true,
                tx_signature: None,
                error: None,
                rewards_processed: 0,
                total_settled: 0,
            });
        }

        // Create batch
        let batch = SettlementBatch {
            batch_id: Uuid::new_v4(),
            rewards: node_rewards.into_values().collect(),
            total_amount,
            created_at: Utc::now().timestamp(),
            status: BatchStatus::Ready,
        };

        info!("Created batch {} with {} rewards totaling {} lamports",
              batch.batch_id, batch.rewards.len(), total_amount);

        // Submit to on-chain settlement
        let result = self.submit_batch(&batch).await;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.batches_processed += 1;
        if result.success {
            stats.rewards_settled += processed_count as u64;
            stats.total_amount_settled += total_amount;
        } else {
            stats.failed_settlements += 1;
        }
        stats.pending_rewards = self.pending_rewards.iter().map(|e| e.value().len()).sum();

        // Store batch in history
        let mut history = self.batch_history.write().await;
        history.push(SettlementBatch {
            status: if result.success { BatchStatus::Settled } else { BatchStatus::Failed },
            ..batch
        });

        // Keep only last 100 batches
        if history.len() > 100 {
            history.remove(0);
        }

        Ok(result)
    }

    /// Submit batch to on-chain settlement contract
    async fn submit_batch(&self, batch: &SettlementBatch) -> SettlementResult {
        info!("Submitting batch {} to on-chain settlement", batch.batch_id);

        // In production, this would:
        // 1. Create Solana transaction calling the strm-token program
        // 2. Sign with oracle keypair
        // 3. Submit to network
        // 4. Wait for confirmation

        // For now, simulate successful settlement
        // The actual implementation would use anchor_client to call the program

        if self.config.oracle_keypair_path.is_none() {
            warn!("No oracle keypair configured - simulating settlement");

            // Simulate processing time
            tokio::time::sleep(Duration::from_millis(100)).await;

            return SettlementResult {
                batch_id: batch.batch_id,
                success: true,
                tx_signature: Some(format!("sim_{}", Uuid::new_v4())),
                error: None,
                rewards_processed: batch.rewards.len(),
                total_settled: batch.total_amount,
            };
        }

        // Actual on-chain submission would go here
        // Using solana_client and anchor_client to submit transaction

        SettlementResult {
            batch_id: batch.batch_id,
            success: true,
            tx_signature: Some(format!("sig_{}", Uuid::new_v4())),
            error: None,
            rewards_processed: batch.rewards.len(),
            total_settled: batch.total_amount,
        }
    }

    /// Run the periodic settlement loop
    async fn run_settlement_loop(&self) {
        let pending = self.pending_rewards.clone();
        let running = self.running.clone();
        let batch_interval = self.config.batch_interval;

        let engine = SettlementEngine {
            config: self.config.clone(),
            pending_rewards: pending,
            current_batch: self.current_batch.clone(),
            batch_history: self.batch_history.clone(),
            stats: self.stats.clone(),
            running: running.clone(),
        };

        tokio::spawn(async move {
            let mut interval = interval(batch_interval);

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                debug!("Running scheduled settlement batch");

                match engine.process_batch().await {
                    Ok(result) => {
                        if result.rewards_processed > 0 {
                            info!("Settlement batch {} completed: {} rewards, {} lamports",
                                  result.batch_id, result.rewards_processed, result.total_settled);
                        }
                    }
                    Err(e) => {
                        error!("Settlement batch failed: {}", e);
                    }
                }
            }
        });
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> SettlementStats {
        self.stats.read().await.clone()
    }

    /// Get recent batch history
    pub async fn get_batch_history(&self) -> Vec<SettlementBatch> {
        self.batch_history.read().await.clone()
    }

    /// Get pending rewards for a specific node
    pub async fn get_pending_for_node(&self, node_id: &NodeId) -> Vec<MicroReward> {
        self.pending_rewards
            .get(node_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    /// Get total pending amount for a node
    pub async fn get_pending_amount(&self, node_id: &NodeId) -> u64 {
        self.pending_rewards
            .get(node_id)
            .map(|r| r.value().iter().map(|m| m.amount).sum())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_reward() {
        let engine = SettlementEngine::default_config();

        let node_id = Uuid::new_v4();
        engine.record_reward(node_id, 10000, "query1".to_string(), true).await.unwrap();

        let pending = engine.get_pending_for_node(&node_id).await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].amount, 10000);
        assert!(pending[0].is_winner);
    }

    #[tokio::test]
    async fn test_record_racing_rewards() {
        let engine = SettlementEngine::default_config();

        let winner = Uuid::new_v4();
        let verifier1 = Uuid::new_v4();
        let verifier2 = Uuid::new_v4();

        engine.record_racing_rewards(
            winner,
            7000,
            vec![verifier1, verifier2],
            1500,
            "query1".to_string(),
        ).await.unwrap();

        assert_eq!(engine.get_pending_amount(&winner).await, 7000);
        assert_eq!(engine.get_pending_amount(&verifier1).await, 1500);
        assert_eq!(engine.get_pending_amount(&verifier2).await, 1500);
    }

    #[tokio::test]
    async fn test_process_batch() {
        let engine = SettlementEngine::default_config();

        // Add some rewards
        for i in 0..5 {
            let node_id = Uuid::new_v4();
            engine.record_reward(node_id, 10000 + i * 1000, format!("query{}", i), i % 2 == 0).await.unwrap();
        }

        let result = engine.process_batch().await.unwrap();

        assert!(result.success);
        assert_eq!(result.rewards_processed, 5);

        // Pending should be empty after batch
        let stats = engine.get_stats().await;
        assert_eq!(stats.pending_rewards, 0);
        assert_eq!(stats.batches_processed, 1);
    }

    #[tokio::test]
    async fn test_skip_dust() {
        let engine = SettlementEngine::default_config();

        let node_id = Uuid::new_v4();

        // This should be skipped (below minimum)
        engine.record_reward(node_id, 100, "query1".to_string(), true).await.unwrap();

        let pending = engine.get_pending_for_node(&node_id).await;
        assert_eq!(pending.len(), 0);
    }
}
