//! StreamSync STRM Token Program
//!
//! This Anchor program manages the $STRM token economics for the StreamSync network:
//! - Node staking and unstaking with cooldown periods
//! - Query reward distribution (racing competition)
//! - Batch settlement of micro-transactions
//! - Slashing for misbehavior
//!
//! ## Architecture
//!
//! The program uses PDAs for state management:
//! - `ProgramConfig` - Global configuration (admin, rates, etc.)
//! - `NodeStake` - Per-node staking account
//! - `SettlementBatch` - Batch of pending rewards
//!
//! ## Reward Flow
//!
//! 1. Query is executed by racing nodes
//! 2. Winner (first correct) gets 70% of query fee
//! 3. Two verifiers each get 15%
//! 4. Rewards are batched and settled every 5 minutes
//! 5. Nodes can claim accumulated rewards anytime

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;
use state::NodeSpecialization;

declare_id!("STRMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");

#[program]
pub mod strm_token {
    use super::*;

    /// Initialize the program configuration
    /// Can only be called once by the deployer
    pub fn initialize(ctx: Context<InitializeConfig>, oracle_authority: Pubkey) -> Result<()> {
        instructions::initialize::initialize_config(ctx, oracle_authority)
    }

    /// Update program configuration (admin only)
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_min_stake: Option<u64>,
        new_cooldown_seconds: Option<i64>,
        new_winner_bps: Option<u16>,
        new_verifier_bps: Option<u16>,
        new_protocol_fee_bps: Option<u16>,
        new_oracle: Option<Pubkey>,
        pause: Option<bool>,
    ) -> Result<()> {
        instructions::initialize::update_config(
            ctx,
            new_min_stake,
            new_cooldown_seconds,
            new_winner_bps,
            new_verifier_bps,
            new_protocol_fee_bps,
            new_oracle,
            pause,
        )
    }

    /// Transfer admin authority
    pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::initialize::transfer_admin(ctx, new_admin)
    }

    /// Stake STRM tokens to become a node operator
    pub fn stake(
        ctx: Context<StakeTokens>,
        amount: u64,
        specialization: NodeSpecialization,
    ) -> Result<()> {
        instructions::stake::stake_tokens(ctx, amount, specialization)
    }

    /// Add more stake to existing node
    pub fn add_stake(ctx: Context<AddStake>, amount: u64) -> Result<()> {
        instructions::stake::add_stake(ctx, amount)
    }

    /// Begin unstaking process (starts cooldown)
    pub fn begin_unstake(ctx: Context<BeginUnstake>, amount: u64) -> Result<()> {
        instructions::unstake::begin_unstake(ctx, amount)
    }

    /// Cancel pending unstake
    pub fn cancel_unstake(ctx: Context<CancelUnstake>) -> Result<()> {
        instructions::unstake::cancel_unstake(ctx)
    }

    /// Complete unstaking after cooldown
    pub fn withdraw(ctx: Context<WithdrawStake>) -> Result<()> {
        instructions::unstake::withdraw_stake(ctx)
    }

    /// Slash a misbehaving node's stake (admin only)
    pub fn slash(ctx: Context<SlashStake>, amount: u64, reason: String) -> Result<()> {
        instructions::unstake::slash_stake(ctx, amount, reason)
    }

    /// Claim accumulated rewards
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        instructions::claim::claim_rewards(ctx)
    }

    /// Record a query reward (oracle only)
    pub fn record_reward(
        ctx: Context<RecordQueryReward>,
        amount: u64,
        query_hash: [u8; 32],
        is_winner: bool,
    ) -> Result<()> {
        instructions::claim::record_query_reward(ctx, amount, query_hash, is_winner)
    }

    /// Create a new settlement batch
    pub fn create_batch(ctx: Context<CreateSettlementBatch>) -> Result<()> {
        instructions::settle::create_settlement_batch(ctx)
    }

    /// Add a reward to the current batch (oracle only)
    pub fn add_batch_reward(
        ctx: Context<AddBatchReward>,
        node: Pubkey,
        amount: u64,
        query_hash: [u8; 32],
        reward_type: state::RewardType,
    ) -> Result<()> {
        instructions::settle::add_batch_reward(ctx, node, amount, query_hash, reward_type)
    }

    /// Process a settlement batch
    pub fn process_batch(ctx: Context<ProcessSettlementBatch>) -> Result<()> {
        instructions::settle::process_settlement_batch(ctx)
    }

    /// Apply batch rewards to a node
    pub fn apply_reward(ctx: Context<ApplyBatchReward>, reward_index: u16) -> Result<()> {
        instructions::settle::apply_batch_reward(ctx, reward_index)
    }
}
