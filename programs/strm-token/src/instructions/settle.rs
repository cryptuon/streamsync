//! Batch settlement instructions

use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::StrmError;
use crate::state::{BatchReward, NodeStake, ProgramConfig, RewardType, SettlementBatch};

/// Create a new settlement batch
pub fn create_settlement_batch(ctx: Context<CreateSettlementBatch>) -> Result<()> {
    let batch = &mut ctx.accounts.settlement_batch;
    let clock = Clock::get()?;

    batch.batch_id = clock.unix_timestamp as u64; // Use timestamp as unique ID
    batch.reward_count = 0;
    batch.total_amount = 0;
    batch.created_at = clock.unix_timestamp;
    batch.settled_at = 0;
    batch.is_settled = false;
    batch.bump = ctx.bumps.settlement_batch;

    msg!("Created settlement batch {}", batch.batch_id);

    Ok(())
}

/// Add a reward to the current batch (oracle only)
pub fn add_batch_reward(
    ctx: Context<AddBatchReward>,
    node: Pubkey,
    amount: u64,
    query_hash: [u8; 32],
    reward_type: RewardType,
) -> Result<()> {
    let batch = &mut ctx.accounts.settlement_batch;

    // Check batch not already settled
    require!(!batch.is_settled, StrmError::BatchFull);

    let reward = BatchReward {
        node,
        amount,
        query_hash,
        reward_type,
    };

    batch.add_reward(reward)?;

    msg!("Added reward of {} to batch {} for node {}", amount, batch.batch_id, node);

    Ok(())
}

/// Process a settlement batch - distribute all rewards
pub fn process_settlement_batch(ctx: Context<ProcessSettlementBatch>) -> Result<()> {
    let config = &ctx.accounts.config;
    let batch = &mut ctx.accounts.settlement_batch;
    let clock = Clock::get()?;

    // Check batch not empty
    require!(!batch.is_empty(), StrmError::BatchEmpty);

    // Check batch not already settled
    require!(!batch.is_settled, StrmError::AlreadyInitialized);

    // Check settlement interval
    let time_since_creation = clock.unix_timestamp - batch.created_at;
    require!(
        time_since_creation >= config.settlement_interval_seconds,
        StrmError::SettlementIntervalNotElapsed
    );

    // Process rewards - update pending rewards for each node
    // In a full implementation, we'd iterate through remaining_accounts
    // For now, mark as settled and let nodes claim
    batch.is_settled = true;
    batch.settled_at = clock.unix_timestamp;

    msg!("Processed settlement batch {} with {} rewards totaling {} STRM",
         batch.batch_id, batch.reward_count, batch.total_amount);

    Ok(())
}

/// Apply batch rewards to a specific node's stake account
pub fn apply_batch_reward(ctx: Context<ApplyBatchReward>, reward_index: u16) -> Result<()> {
    let batch = &ctx.accounts.settlement_batch;
    let stake_account = &mut ctx.accounts.stake_account;

    // Check batch is settled
    require!(batch.is_settled, StrmError::BatchEmpty);

    // Check index valid
    require!(
        (reward_index as usize) < batch.reward_count as usize,
        StrmError::BatchEmpty
    );

    let reward = &batch.rewards[reward_index as usize];

    // Verify this is the correct node
    require!(
        reward.node == stake_account.node,
        StrmError::InvalidNodePubkey
    );

    // Apply reward
    stake_account.pending_rewards = stake_account.pending_rewards
        .checked_add(reward.amount)
        .ok_or(StrmError::MathOverflow)?;

    // Update stats based on reward type
    match reward.reward_type {
        RewardType::Winner => {
            stake_account.queries_won = stake_account.queries_won
                .checked_add(1)
                .ok_or(StrmError::MathOverflow)?;
        }
        RewardType::Verifier => {
            stake_account.queries_verified = stake_account.queries_verified
                .checked_add(1)
                .ok_or(StrmError::MathOverflow)?;
        }
        RewardType::Protocol => {
            // Protocol fees don't count as queries
        }
    }

    stake_account.queries_answered = stake_account.queries_answered
        .checked_add(1)
        .ok_or(StrmError::MathOverflow)?;

    msg!("Applied reward of {} STRM from batch {} to node {}",
         reward.amount, batch.batch_id, stake_account.node);

    Ok(())
}

#[derive(Accounts)]
pub struct CreateSettlementBatch<'info> {
    /// Oracle or admin authority
    #[account(
        mut,
        constraint =
            authority.key() == config.oracle_authority ||
            authority.key() == config.admin @ StrmError::Unauthorized
    )]
    pub authority: Signer<'info>,

    /// Program configuration
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, ProgramConfig>,

    /// New settlement batch PDA
    #[account(
        init,
        payer = authority,
        space = SettlementBatch::LEN,
        seeds = [SETTLEMENT_SEED, &Clock::get()?.unix_timestamp.to_le_bytes()],
        bump
    )]
    pub settlement_batch: Account<'info, SettlementBatch>,

    /// System program
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddBatchReward<'info> {
    /// Oracle authority
    #[account(
        constraint = oracle.key() == config.oracle_authority @ StrmError::OracleVerificationFailed
    )]
    pub oracle: Signer<'info>,

    /// Program configuration
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, ProgramConfig>,

    /// Settlement batch to add reward to
    #[account(mut)]
    pub settlement_batch: Account<'info, SettlementBatch>,
}

#[derive(Accounts)]
pub struct ProcessSettlementBatch<'info> {
    /// Oracle or admin authority
    #[account(
        constraint =
            authority.key() == config.oracle_authority ||
            authority.key() == config.admin @ StrmError::Unauthorized
    )]
    pub authority: Signer<'info>,

    /// Program configuration
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, ProgramConfig>,

    /// Settlement batch to process
    #[account(mut)]
    pub settlement_batch: Account<'info, SettlementBatch>,
}

#[derive(Accounts)]
pub struct ApplyBatchReward<'info> {
    /// Anyone can apply settled rewards
    pub payer: Signer<'info>,

    /// Settlement batch
    #[account(
        constraint = settlement_batch.is_settled @ StrmError::BatchEmpty
    )]
    pub settlement_batch: Account<'info, SettlementBatch>,

    /// Node's stake account to credit
    #[account(mut)]
    pub stake_account: Account<'info, NodeStake>,
}
