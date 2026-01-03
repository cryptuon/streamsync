//! Node stake account PDA

use anchor_lang::prelude::*;

/// Per-node staking account
/// PDA: ["stake", node_pubkey]
#[account]
#[derive(Default)]
pub struct NodeStake {
    /// The node operator's wallet address
    pub node: Pubkey,

    /// Amount of STRM currently staked
    pub staked_amount: u64,

    /// Accumulated rewards pending claim
    pub pending_rewards: u64,

    /// Total rewards claimed historically
    pub total_rewards_claimed: u64,

    /// Timestamp when unstaking was initiated (0 if not unstaking)
    pub unstake_initiated_at: i64,

    /// Amount being unstaked (0 if not unstaking)
    pub unstaking_amount: u64,

    /// Node specialization type
    pub specialization: NodeSpecialization,

    /// Node reputation score (0-1000)
    pub reputation_score: u16,

    /// Total queries answered
    pub queries_answered: u64,

    /// Total queries won (first correct answer)
    pub queries_won: u64,

    /// Total queries verified
    pub queries_verified: u64,

    /// Timestamp of last activity
    pub last_activity_at: i64,

    /// Whether the node is currently active
    pub is_active: bool,

    /// Number of times slashed
    pub slash_count: u8,

    /// Bump seed for PDA derivation
    pub bump: u8,

    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl NodeStake {
    pub const LEN: usize = 8 + // discriminator
        32 + // node
        8 +  // staked_amount
        8 +  // pending_rewards
        8 +  // total_rewards_claimed
        8 +  // unstake_initiated_at
        8 +  // unstaking_amount
        1 +  // specialization
        2 +  // reputation_score
        8 +  // queries_answered
        8 +  // queries_won
        8 +  // queries_verified
        8 +  // last_activity_at
        1 +  // is_active
        1 +  // slash_count
        1 +  // bump
        32;  // reserved

    /// Check if node is currently unstaking
    pub fn is_unstaking(&self) -> bool {
        self.unstake_initiated_at > 0
    }

    /// Check if node meets minimum stake requirement
    pub fn meets_minimum_stake(&self, min_stake: u64) -> bool {
        self.staked_amount >= min_stake
    }

    /// Calculate win rate as percentage (0-100)
    pub fn win_rate(&self) -> u8 {
        if self.queries_answered == 0 {
            return 0;
        }
        ((self.queries_won as u128 * 100) / self.queries_answered as u128) as u8
    }
}

/// Node specialization types matching the off-chain implementation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeSpecialization {
    /// General purpose node
    #[default]
    General,
    /// Optimized for low-latency queries
    SpeedRunner,
    /// Specialized in ZK account reconstruction
    ReconstructionSpec,
    /// Optimized for frequently accessed data
    CacheOptimizer,
    /// Historical data with long retention
    ArchiveNode,
}
