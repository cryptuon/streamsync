//! Unstaking instructions with cooldown period

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::errors::StrmError;
use crate::state::{NodeStake, ProgramConfig};

/// Initiate unstaking process (starts cooldown)
pub fn begin_unstake(ctx: Context<BeginUnstake>, amount: u64) -> Result<()> {
    let stake_account = &mut ctx.accounts.stake_account;
    let clock = Clock::get()?;

    // Must be staked
    require!(stake_account.is_active, StrmError::NotStaked);

    // Cannot already be unstaking
    require!(!stake_account.is_unstaking(), StrmError::UnstakingInProgress);

    // Check sufficient stake
    require!(
        stake_account.staked_amount >= amount,
        StrmError::InsufficientStake
    );

    // Start cooldown
    stake_account.unstake_initiated_at = clock.unix_timestamp;
    stake_account.unstaking_amount = amount;

    msg!("Unstaking initiated for {} STRM", amount);
    msg!("Cooldown ends at: {}", clock.unix_timestamp + ctx.accounts.config.unstake_cooldown_seconds);

    Ok(())
}

/// Cancel pending unstake
pub fn cancel_unstake(ctx: Context<CancelUnstake>) -> Result<()> {
    let stake_account = &mut ctx.accounts.stake_account;

    // Must be unstaking
    require!(stake_account.is_unstaking(), StrmError::NotUnstaking);

    // Reset unstaking state
    stake_account.unstake_initiated_at = 0;
    stake_account.unstaking_amount = 0;

    msg!("Unstaking cancelled");

    Ok(())
}

/// Complete unstaking after cooldown
pub fn withdraw_stake(ctx: Context<WithdrawStake>) -> Result<()> {
    let config = &ctx.accounts.config;
    let stake_account = &mut ctx.accounts.stake_account;
    let clock = Clock::get()?;

    // Must be unstaking
    require!(stake_account.is_unstaking(), StrmError::NotUnstaking);

    // Check cooldown complete
    let cooldown_end = stake_account.unstake_initiated_at + config.unstake_cooldown_seconds;
    require!(
        clock.unix_timestamp >= cooldown_end,
        StrmError::CooldownNotComplete
    );

    let withdraw_amount = stake_account.unstaking_amount;

    // Transfer tokens back to user
    let config_key = ctx.accounts.config.key();
    let seeds = &[
        CONFIG_SEED,
        config_key.as_ref(),
        &[config.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.stake_vault.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.config.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    token::transfer(cpi_ctx, withdraw_amount)?;

    // Update stake account
    stake_account.staked_amount = stake_account.staked_amount.checked_sub(withdraw_amount)
        .ok_or(StrmError::MathOverflow)?;
    stake_account.unstake_initiated_at = 0;
    stake_account.unstaking_amount = 0;

    // If fully unstaked, mark inactive
    if stake_account.staked_amount == 0 {
        stake_account.is_active = false;

        // Update global stats
        let config = &mut ctx.accounts.config;
        config.active_node_count = config.active_node_count.saturating_sub(1);
    }

    // Update global stats
    let config = &mut ctx.accounts.config;
    config.total_staked = config.total_staked.saturating_sub(withdraw_amount);

    msg!("Withdrew {} STRM. Remaining stake: {}", withdraw_amount, stake_account.staked_amount);

    Ok(())
}

/// Slash a misbehaving node's stake (admin only)
pub fn slash_stake(ctx: Context<SlashStake>, amount: u64, reason: String) -> Result<()> {
    let config = &ctx.accounts.config;
    let stake_account = &mut ctx.accounts.stake_account;

    // Check slash doesn't exceed maximum
    let max_slash = (stake_account.staked_amount as u128 * MAX_SLASH_BPS as u128 / BPS_DENOMINATOR as u128) as u64;
    require!(amount <= max_slash, StrmError::SlashExceedsMaximum);

    // Reduce stake
    stake_account.staked_amount = stake_account.staked_amount.saturating_sub(amount);
    stake_account.slash_count += 1;

    // Reduce reputation
    stake_account.reputation_score = stake_account.reputation_score.saturating_sub(100);

    // If below minimum, mark inactive
    if stake_account.staked_amount < config.min_stake_amount {
        stake_account.is_active = false;

        let config = &mut ctx.accounts.config;
        config.active_node_count = config.active_node_count.saturating_sub(1);
    }

    // Update global stats
    let config = &mut ctx.accounts.config;
    config.total_staked = config.total_staked.saturating_sub(amount);

    msg!("Slashed {} STRM from node {} for: {}", amount, stake_account.node, reason);

    // Transfer slashed tokens to protocol treasury (could be implemented)
    // For now, they remain in the vault

    Ok(())
}

#[derive(Accounts)]
pub struct BeginUnstake<'info> {
    /// Node operator
    pub node: Signer<'info>,

    /// Program configuration
    #[account(
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
}

#[derive(Accounts)]
pub struct CancelUnstake<'info> {
    /// Node operator
    pub node: Signer<'info>,

    /// Node's stake account
    #[account(
        mut,
        seeds = [STAKE_SEED, node.key().as_ref()],
        bump = stake_account.bump,
        constraint = stake_account.node == node.key() @ StrmError::InvalidNodePubkey
    )]
    pub stake_account: Account<'info, NodeStake>,
}

#[derive(Accounts)]
pub struct WithdrawStake<'info> {
    /// Node operator
    #[account(mut)]
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

    /// Stake vault
    #[account(
        mut,
        constraint = stake_vault.mint == config.strm_mint @ StrmError::InvalidMint
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SlashStake<'info> {
    /// Admin authority
    #[account(
        constraint = admin.key() == config.admin @ StrmError::Unauthorized
    )]
    pub admin: Signer<'info>,

    /// Program configuration
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, ProgramConfig>,

    /// Node's stake account to slash
    #[account(mut)]
    pub stake_account: Account<'info, NodeStake>,
}
