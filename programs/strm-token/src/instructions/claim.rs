//! Reward claiming instructions

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::errors::StrmError;
use crate::state::{NodeStake, ProgramConfig};

/// Claim accumulated rewards
pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
    let config = &ctx.accounts.config;
    let stake_account = &mut ctx.accounts.stake_account;

    // Check there are rewards to claim
    require!(
        stake_account.pending_rewards > 0,
        StrmError::NoRewardsAvailable
    );

    let claim_amount = stake_account.pending_rewards;

    // Transfer rewards to user
    let config_key = ctx.accounts.config.key();
    let seeds = &[
        CONFIG_SEED,
        config_key.as_ref(),
        &[config.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.reward_vault.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.config.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    token::transfer(cpi_ctx, claim_amount)?;

    // Update stake account
    stake_account.pending_rewards = 0;
    stake_account.total_rewards_claimed = stake_account.total_rewards_claimed
        .checked_add(claim_amount)
        .ok_or(StrmError::MathOverflow)?;

    // Update global stats
    let config = &mut ctx.accounts.config;
    config.total_rewards_distributed = config.total_rewards_distributed
        .checked_add(claim_amount)
        .ok_or(StrmError::MathOverflow)?;

    msg!("Claimed {} STRM in rewards", claim_amount);
    msg!("Total rewards claimed: {}", stake_account.total_rewards_claimed);

    Ok(())
}

/// Record a query reward (oracle only)
/// This is called by the off-chain oracle after a query is completed
pub fn record_query_reward(
    ctx: Context<RecordQueryReward>,
    amount: u64,
    query_hash: [u8; 32],
    is_winner: bool,
) -> Result<()> {
    let stake_account = &mut ctx.accounts.stake_account;
    let clock = Clock::get()?;

    // Add to pending rewards
    stake_account.pending_rewards = stake_account.pending_rewards
        .checked_add(amount)
        .ok_or(StrmError::MathOverflow)?;

    // Update stats
    stake_account.queries_answered = stake_account.queries_answered
        .checked_add(1)
        .ok_or(StrmError::MathOverflow)?;

    if is_winner {
        stake_account.queries_won = stake_account.queries_won
            .checked_add(1)
            .ok_or(StrmError::MathOverflow)?;
    } else {
        stake_account.queries_verified = stake_account.queries_verified
            .checked_add(1)
            .ok_or(StrmError::MathOverflow)?;
    }

    stake_account.last_activity_at = clock.unix_timestamp;

    // Update reputation based on activity
    if stake_account.reputation_score < 1000 {
        stake_account.reputation_score = stake_account.reputation_score.saturating_add(1);
    }

    msg!("Recorded {} STRM reward for query {:?}", amount, query_hash);
    msg!("Node pending rewards: {}", stake_account.pending_rewards);

    Ok(())
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    /// Node operator
    pub node: Signer<'info>,

    /// Program configuration
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, ProgramConfig>,

    /// Node's stake account
    #[account(
        mut,
        seeds = [STAKE_SEED, node.key().as_ref()],
        bump = stake_account.bump,
        constraint = stake_account.node == node.key() @ StrmError::InvalidNodePubkey
    )]
    pub stake_account: Account<'info, NodeStake>,

    /// User's STRM token account
    #[account(
        mut,
        constraint = user_token_account.mint == config.strm_mint @ StrmError::InvalidMint,
        constraint = user_token_account.owner == node.key() @ StrmError::InvalidOwner
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Reward vault (PDA-owned)
    #[account(
        mut,
        constraint = reward_vault.mint == config.strm_mint @ StrmError::InvalidMint
    )]
    pub reward_vault: Account<'info, TokenAccount>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RecordQueryReward<'info> {
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

    /// Node's stake account to credit
    #[account(mut)]
    pub stake_account: Account<'info, NodeStake>,
}
