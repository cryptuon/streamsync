//! Settlement batch account for micro-transaction aggregation

use anchor_lang::prelude::*;

/// Maximum rewards in a single batch
pub const MAX_BATCH_REWARDS: usize = 100;

/// Settlement batch for aggregating micro-rewards
/// PDA: ["settlement", batch_id]
#[account]
pub struct SettlementBatch {
    /// Unique batch ID
    pub batch_id: u64,

    /// Number of rewards in this batch
    pub reward_count: u16,

    /// Total amount to be distributed in this batch
    pub total_amount: u64,

    /// Timestamp when batch was created
    pub created_at: i64,

    /// Timestamp when batch was settled (0 if pending)
    pub settled_at: i64,

    /// Whether the batch has been processed
    pub is_settled: bool,

    /// Bump seed for PDA derivation
    pub bump: u8,

    /// Individual rewards in this batch
    pub rewards: [BatchReward; MAX_BATCH_REWARDS],
}

impl SettlementBatch {
    pub const LEN: usize = 8 + // discriminator
        8 +   // batch_id
        2 +   // reward_count
        8 +   // total_amount
        8 +   // created_at
        8 +   // settled_at
        1 +   // is_settled
        1 +   // bump
        (BatchReward::LEN * MAX_BATCH_REWARDS); // rewards array

    /// Add a reward to the batch
    pub fn add_reward(&mut self, reward: BatchReward) -> Result<()> {
        if self.reward_count as usize >= MAX_BATCH_REWARDS {
            return err!(crate::errors::StrmError::BatchFull);
        }
        self.rewards[self.reward_count as usize] = reward;
        self.reward_count += 1;
        self.total_amount = self.total_amount.checked_add(reward.amount)
            .ok_or(crate::errors::StrmError::MathOverflow)?;
        Ok(())
    }

    /// Check if batch is full
    pub fn is_full(&self) -> bool {
        self.reward_count as usize >= MAX_BATCH_REWARDS
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.reward_count == 0
    }
}

impl Default for SettlementBatch {
    fn default() -> Self {
        Self {
            batch_id: 0,
            reward_count: 0,
            total_amount: 0,
            created_at: 0,
            settled_at: 0,
            is_settled: false,
            bump: 0,
            rewards: [BatchReward::default(); MAX_BATCH_REWARDS],
        }
    }
}

/// Individual reward within a batch
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct BatchReward {
    /// Recipient node
    pub node: Pubkey,

    /// Reward amount in STRM lamports
    pub amount: u64,

    /// Query ID this reward is for (hash of query)
    pub query_hash: [u8; 32],

    /// Reward type
    pub reward_type: RewardType,
}

impl BatchReward {
    pub const LEN: usize = 32 + // node
        8 +  // amount
        32 + // query_hash
        1;   // reward_type
}

/// Type of reward
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, PartialEq, Eq)]
pub enum RewardType {
    /// Winner of racing competition (70%)
    #[default]
    Winner,
    /// Verifier of correct answer (15%)
    Verifier,
    /// Protocol fee
    Protocol,
}
