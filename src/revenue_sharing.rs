//! Revenue sharing and reward distribution for StreamSync network

use crate::economics::{PaymentToken, EconomicsEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use anyhow::Result;

/// Node performance metrics for revenue calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePerformance {
    pub node_id: Uuid,
    pub requests_served: u64,
    pub data_transferred_mb: f64,
    pub uptime_percentage: f64,
    pub average_response_time_ms: f64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub consensus_participation: f64,
    pub storage_provided_gb: f64,
    pub bandwidth_provided_mbps: f64,
}

/// Revenue distribution models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevenueModel {
    /// Equal distribution among all nodes
    EqualShare,
    /// Performance-based distribution
    PerformanceBased {
        request_weight: f64,
        uptime_weight: f64,
        response_time_weight: f64,
        consensus_weight: f64,
        storage_weight: f64,
    },
    /// Stake-based distribution (if using staking)
    StakeBased,
    /// Hybrid model combining multiple factors
    Hybrid {
        performance_ratio: f64,
        stake_ratio: f64,
        equal_ratio: f64,
    },
}

impl Default for RevenueModel {
    fn default() -> Self {
        Self::PerformanceBased {
            request_weight: 0.4,
            uptime_weight: 0.2,
            response_time_weight: 0.15,
            consensus_weight: 0.15,
            storage_weight: 0.1,
        }
    }
}

/// Staking information for nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStake {
    pub node_id: Uuid,
    pub staked_amount: f64,
    pub token: PaymentToken,
    pub stake_start_time: i64,
    pub lock_duration_days: u32,
    pub slashing_conditions: Vec<String>,
}

/// Revenue distribution period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionPeriod {
    pub period_id: Uuid,
    pub start_time: i64,
    pub end_time: i64,
    pub total_revenue: HashMap<PaymentToken, f64>,
    pub participating_nodes: Vec<Uuid>,
    pub distribution_model: RevenueModel,
}

/// Node reward calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReward {
    pub node_id: Uuid,
    pub period_id: Uuid,
    pub performance_score: f64,
    pub stake_weight: f64,
    pub reward_amount: f64,
    pub token: PaymentToken,
    pub bonus_multipliers: HashMap<String, f64>,
}

/// Revenue sharing events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevenueEvent {
    PerformanceUpdated {
        node_id: Uuid,
        performance: NodePerformance,
        timestamp: i64,
    },
    RewardDistributed {
        period_id: Uuid,
        node_rewards: Vec<NodeReward>,
        timestamp: i64,
    },
    StakeUpdated {
        node_id: Uuid,
        stake: NodeStake,
        timestamp: i64,
    },
    SlashingEvent {
        node_id: Uuid,
        reason: String,
        amount_slashed: f64,
        timestamp: i64,
    },
}

/// Revenue sharing manager
pub struct RevenueSharingManager {
    node_performance: Arc<RwLock<HashMap<Uuid, NodePerformance>>>,
    node_stakes: Arc<RwLock<HashMap<Uuid, NodeStake>>>,
    distribution_periods: Arc<RwLock<Vec<DistributionPeriod>>>,
    pending_rewards: Arc<RwLock<HashMap<Uuid, Vec<NodeReward>>>>,
    distribution_model: RevenueModel,
    event_sender: broadcast::Sender<RevenueEvent>,
    economics_receiver: broadcast::Receiver<EconomicsEvent>,
}

impl RevenueSharingManager {
    pub fn new(
        distribution_model: RevenueModel,
        economics_receiver: broadcast::Receiver<EconomicsEvent>,
    ) -> (Self, broadcast::Receiver<RevenueEvent>) {
        let (event_sender, event_receiver) = broadcast::channel(1000);

        let manager = Self {
            node_performance: Arc::new(RwLock::new(HashMap::new())),
            node_stakes: Arc::new(RwLock::new(HashMap::new())),
            distribution_periods: Arc::new(RwLock::new(Vec::new())),
            pending_rewards: Arc::new(RwLock::new(HashMap::new())),
            distribution_model,
            event_sender,
            economics_receiver,
        };

        (manager, event_receiver)
    }

    /// Start the revenue sharing manager
    pub async fn start(&mut self) -> Result<()> {
        // Start listening to economics events
        let node_performance = self.node_performance.clone();
        let event_sender = self.event_sender.clone();
        let mut economics_receiver = self.economics_receiver.resubscribe();

        tokio::spawn(async move {
            while let Ok(event) = economics_receiver.recv().await {
                match event {
                    EconomicsEvent::RevenueDistributed { total_amount, token, .. } => {
                        // Trigger revenue distribution
                        // This would be handled by the main distribution logic
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Update node performance metrics
    pub async fn update_performance(&self, node_id: Uuid, performance: NodePerformance) -> Result<()> {
        let mut performances = self.node_performance.write().await;
        performances.insert(node_id, performance.clone());

        let _ = self.event_sender.send(RevenueEvent::PerformanceUpdated {
            node_id,
            performance,
            timestamp: chrono::Utc::now().timestamp(),
        });

        Ok(())
    }

    /// Update node stake
    pub async fn update_stake(&self, node_id: Uuid, stake: NodeStake) -> Result<()> {
        let mut stakes = self.node_stakes.write().await;
        stakes.insert(node_id, stake.clone());

        let _ = self.event_sender.send(RevenueEvent::StakeUpdated {
            node_id,
            stake,
            timestamp: chrono::Utc::now().timestamp(),
        });

        Ok(())
    }

    /// Calculate performance score for a node
    fn calculate_performance_score(&self, performance: &NodePerformance, model: &RevenueModel) -> f64 {
        match model {
            RevenueModel::EqualShare => 1.0,
            RevenueModel::PerformanceBased {
                request_weight,
                uptime_weight,
                response_time_weight,
                consensus_weight,
                storage_weight,
            } => {
                let request_score = performance.requests_served as f64;
                let uptime_score = performance.uptime_percentage / 100.0;
                let response_score = 1.0 / (performance.average_response_time_ms / 100.0 + 1.0);
                let consensus_score = performance.consensus_participation;
                let storage_score = performance.storage_provided_gb;

                // Normalize and weight scores
                let normalized_request = (request_score / 10000.0).min(1.0);
                let normalized_storage = (storage_score / 1000.0).min(1.0);

                normalized_request * request_weight
                    + uptime_score * uptime_weight
                    + response_score * response_time_weight
                    + consensus_score * consensus_weight
                    + normalized_storage * storage_weight
            }
            RevenueModel::StakeBased => 1.0, // Will be weighted by stake
            RevenueModel::Hybrid { .. } => {
                // Calculate base performance score
                self.calculate_performance_score(performance, &RevenueModel::default())
            }
        }
    }

    /// Calculate stake weight for a node
    fn calculate_stake_weight(&self, node_id: &Uuid, stakes: &HashMap<Uuid, NodeStake>) -> f64 {
        if let Some(stake) = stakes.get(node_id) {
            // Apply time-based multiplier for long-term staking
            let stake_duration_days = (chrono::Utc::now().timestamp() - stake.stake_start_time) / 86400;
            let duration_multiplier = 1.0 + (stake_duration_days as f64 / 365.0) * 0.1; // 10% bonus per year

            stake.staked_amount * duration_multiplier
        } else {
            0.0
        }
    }

    /// Distribute revenue for a period
    pub async fn distribute_revenue(
        &self,
        period_id: Uuid,
        total_revenue: HashMap<PaymentToken, f64>,
    ) -> Result<Vec<NodeReward>> {
        let performances = self.node_performance.read().await;
        let stakes = self.node_stakes.read().await;

        let mut rewards = Vec::new();

        for (token, amount) in total_revenue {
            let node_rewards = self.calculate_rewards_sync(
                &token,
                amount,
                &performances,
                &stakes,
                &self.distribution_model,
            )?;

            for reward in node_rewards {
                rewards.push(NodeReward {
                    period_id,
                    ..reward
                });
            }
        }

        // Store pending rewards
        let mut pending = self.pending_rewards.write().await;
        for reward in &rewards {
            pending.entry(reward.node_id).or_insert_with(Vec::new).push(reward.clone());
        }

        let _ = self.event_sender.send(RevenueEvent::RewardDistributed {
            period_id,
            node_rewards: rewards.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        });

        Ok(rewards)
    }

    /// Calculate rewards for all nodes
    fn calculate_rewards_sync(
        &self,
        token: &PaymentToken,
        total_amount: f64,
        performances: &HashMap<Uuid, NodePerformance>,
        stakes: &HashMap<Uuid, NodeStake>,
        model: &RevenueModel,
    ) -> Result<Vec<NodeReward>> {
        let mut rewards = Vec::new();

        match model {
            RevenueModel::EqualShare => {
                let per_node_amount = total_amount / performances.len() as f64;
                for (&node_id, performance) in performances {
                    rewards.push(NodeReward {
                        node_id,
                        period_id: Uuid::new_v4(), // Will be set by caller
                        performance_score: 1.0,
                        stake_weight: 0.0,
                        reward_amount: per_node_amount,
                        token: token.clone(),
                        bonus_multipliers: HashMap::new(),
                    });
                }
            }
            RevenueModel::PerformanceBased { .. } => {
                let mut total_score = 0.0;
                let mut node_scores = HashMap::new();

                // Calculate scores for all nodes
                for (&node_id, performance) in performances {
                    let score = self.calculate_performance_score(performance, model);
                    node_scores.insert(node_id, score);
                    total_score += score;
                }

                // Distribute based on scores
                for (&node_id, &score) in &node_scores {
                    let reward_amount = (score / total_score) * total_amount;
                    rewards.push(NodeReward {
                        node_id,
                        period_id: Uuid::new_v4(),
                        performance_score: score,
                        stake_weight: 0.0,
                        reward_amount,
                        token: token.clone(),
                        bonus_multipliers: HashMap::new(),
                    });
                }
            }
            RevenueModel::StakeBased => {
                let mut total_stake = 0.0;
                let mut node_stakes_map = HashMap::new();

                // Calculate total stake
                for (&node_id, _performance) in performances {
                    let stake_weight = self.calculate_stake_weight(&node_id, stakes);
                    node_stakes_map.insert(node_id, stake_weight);
                    total_stake += stake_weight;
                }

                // Distribute based on stakes
                for (&node_id, &stake_weight) in &node_stakes_map {
                    let reward_amount = if total_stake > 0.0 {
                        (stake_weight / total_stake) * total_amount
                    } else {
                        total_amount / performances.len() as f64
                    };

                    rewards.push(NodeReward {
                        node_id,
                        period_id: Uuid::new_v4(),
                        performance_score: 0.0,
                        stake_weight,
                        reward_amount,
                        token: token.clone(),
                        bonus_multipliers: HashMap::new(),
                    });
                }
            }
            RevenueModel::Hybrid { performance_ratio, stake_ratio, equal_ratio } => {
                // Calculate performance-based distribution
                let perf_rewards = self.calculate_rewards_sync(
                    token,
                    total_amount * performance_ratio,
                    performances,
                    stakes,
                    &RevenueModel::default(),
                )?;

                // Calculate stake-based distribution
                let stake_rewards = self.calculate_rewards_sync(
                    token,
                    total_amount * stake_ratio,
                    performances,
                    stakes,
                    &RevenueModel::StakeBased,
                )?;

                // Calculate equal distribution
                let equal_rewards = self.calculate_rewards_sync(
                    token,
                    total_amount * equal_ratio,
                    performances,
                    stakes,
                    &RevenueModel::EqualShare,
                )?;

                // Combine rewards
                let mut combined_rewards = HashMap::new();
                for reward in perf_rewards.into_iter().chain(stake_rewards).chain(equal_rewards) {
                    let entry = combined_rewards.entry(reward.node_id).or_insert(NodeReward {
                        node_id: reward.node_id,
                        period_id: Uuid::new_v4(),
                        performance_score: 0.0,
                        stake_weight: 0.0,
                        reward_amount: 0.0,
                        token: token.clone(),
                        bonus_multipliers: HashMap::new(),
                    });

                    entry.reward_amount += reward.reward_amount;
                    entry.performance_score += reward.performance_score;
                    entry.stake_weight += reward.stake_weight;
                }

                rewards = combined_rewards.into_values().collect();
            }
        }

        // Apply bonus multipliers
        for reward in &mut rewards {
            if let Some(performance) = performances.get(&reward.node_id) {
                // High uptime bonus
                if performance.uptime_percentage >= 99.5 {
                    reward.bonus_multipliers.insert("high_uptime".to_string(), 1.1);
                    reward.reward_amount *= 1.1;
                }

                // Fast response bonus
                if performance.average_response_time_ms < 100.0 {
                    reward.bonus_multipliers.insert("fast_response".to_string(), 1.05);
                    reward.reward_amount *= 1.05;
                }

                // High consensus participation bonus
                if performance.consensus_participation >= 0.95 {
                    reward.bonus_multipliers.insert("consensus_leader".to_string(), 1.15);
                    reward.reward_amount *= 1.15;
                }
            }
        }

        Ok(rewards)
    }

    /// Get pending rewards for a node
    pub async fn get_pending_rewards(&self, node_id: &Uuid) -> Vec<NodeReward> {
        let pending = self.pending_rewards.read().await;
        pending.get(node_id).cloned().unwrap_or_default()
    }

    /// Claim rewards for a node (mark as paid)
    pub async fn claim_rewards(&self, node_id: Uuid) -> Result<Vec<NodeReward>> {
        let mut pending = self.pending_rewards.write().await;
        let rewards = pending.remove(&node_id).unwrap_or_default();

        // In a real implementation, this would:
        // 1. Initiate actual token transfers
        // 2. Update on-chain records
        // 3. Send notifications

        Ok(rewards)
    }

    /// Slash a node (reduce stake due to malicious behavior)
    pub async fn slash_node(&self, node_id: Uuid, reason: String, percentage: f64) -> Result<f64> {
        let mut stakes = self.node_stakes.write().await;

        if let Some(stake) = stakes.get_mut(&node_id) {
            let slash_amount = stake.staked_amount * (percentage / 100.0);
            stake.staked_amount -= slash_amount;

            let _ = self.event_sender.send(RevenueEvent::SlashingEvent {
                node_id,
                reason,
                amount_slashed: slash_amount,
                timestamp: chrono::Utc::now().timestamp(),
            });

            Ok(slash_amount)
        } else {
            Ok(0.0)
        }
    }

    /// Get network statistics for revenue sharing
    pub async fn get_network_stats(&self) -> HashMap<String, serde_json::Value> {
        let performances = self.node_performance.read().await;
        let stakes = self.node_stakes.read().await;
        let pending = self.pending_rewards.read().await;

        let mut stats = HashMap::new();

        let total_nodes = performances.len();
        let total_requests: u64 = performances.values().map(|p| p.requests_served).sum();
        let average_uptime: f64 = performances.values().map(|p| p.uptime_percentage).sum::<f64>() / total_nodes as f64;
        let total_stake: f64 = stakes.values().map(|s| s.staked_amount).sum();
        let pending_reward_count: usize = pending.values().map(|v| v.len()).sum();

        stats.insert("total_nodes".to_string(), serde_json::Value::Number(total_nodes.into()));
        stats.insert("total_requests_served".to_string(), serde_json::Value::Number(total_requests.into()));
        stats.insert("average_network_uptime".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(average_uptime).unwrap_or_else(|| serde_json::Number::from(0))));
        stats.insert("total_stake".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(total_stake).unwrap_or_else(|| serde_json::Number::from(0))));
        stats.insert("pending_rewards".to_string(), serde_json::Value::Number(pending_reward_count.into()));

        stats
    }
}