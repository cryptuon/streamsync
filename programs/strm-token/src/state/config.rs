//! Program configuration PDA

use anchor_lang::prelude::*;

/// Global program configuration
/// PDA: ["config"]
#[account]
#[derive(Default)]
pub struct ProgramConfig {
    /// Admin authority that can update config and slash stakes
    pub admin: Pubkey,

    /// STRM token mint address
    pub strm_mint: Pubkey,

    /// Minimum stake required to operate a node
    pub min_stake_amount: u64,

    /// Unstaking cooldown period in seconds
    pub unstake_cooldown_seconds: i64,

    /// Winner reward percentage in basis points (7000 = 70%)
    pub winner_reward_bps: u16,

    /// Verifier reward percentage in basis points (1500 = 15%)
    pub verifier_reward_bps: u16,

    /// Protocol fee percentage in basis points
    pub protocol_fee_bps: u16,

    /// Settlement batch interval in seconds
    pub settlement_interval_seconds: i64,

    /// Total STRM staked across all nodes
    pub total_staked: u64,

    /// Total rewards distributed
    pub total_rewards_distributed: u64,

    /// Total number of active nodes
    pub active_node_count: u64,

    /// Oracle authority for recording query rewards
    pub oracle_authority: Pubkey,

    /// Whether the program is paused
    pub is_paused: bool,

    /// Bump seed for PDA derivation
    pub bump: u8,

    /// Reserved for future use
    pub _reserved: [u8; 64],
}

impl ProgramConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // admin
        32 + // strm_mint
        8 +  // min_stake_amount
        8 +  // unstake_cooldown_seconds
        2 +  // winner_reward_bps
        2 +  // verifier_reward_bps
        2 +  // protocol_fee_bps
        8 +  // settlement_interval_seconds
        8 +  // total_staked
        8 +  // total_rewards_distributed
        8 +  // active_node_count
        32 + // oracle_authority
        1 +  // is_paused
        1 +  // bump
        64;  // reserved

    /// Check if reward percentages are valid (should sum to 10000 or less)
    pub fn validate_reward_percentages(&self) -> bool {
        let total = self.winner_reward_bps as u32 +
                   (self.verifier_reward_bps as u32 * 2) +
                   self.protocol_fee_bps as u32;
        total <= 10000
    }
}
