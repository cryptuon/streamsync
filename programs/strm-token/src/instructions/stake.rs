//! Staking instructions for node operators

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::errors::StrmError;
use crate::state::{NodeSpecialization, NodeStake, ProgramConfig};

/// Stake STRM tokens to become a node operator
pub fn stake_tokens(
    ctx: Context<StakeTokens>,
    amount: u64,
    specialization: NodeSpecialization,
) -> Result<()> {
    let config = &ctx.accounts.config;
    let stake_account = &mut ctx.accounts.stake_account;
    let clock = Clock::get()?;

    // Check program is not paused
    require!(!config.is_paused, StrmError::Unauthorized);

    // Check minimum stake
    let new_total = stake_account.staked_amount.checked_add(amount)
        .ok_or(StrmError::MathOverflow)?;
    require!(
        new_total >= config.min_stake_amount,
        StrmError::StakeBelowMinimum
    );

    // Transfer tokens to stake vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.stake_vault.to_account_info(),
        authority: ctx.accounts.node.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Update stake account
    stake_account.node = ctx.accounts.node.key();
    stake_account.staked_amount = new_total;
    stake_account.specialization = specialization;
    stake_account.last_activity_at = clock.unix_timestamp;

    // If this is first stake, mark as active and update config
    if !stake_account.is_active {
        stake_account.is_active = true;
        stake_account.reputation_score = 500; // Start with neutral reputation

        // Update global stats
        let config = &mut ctx.accounts.config;
        config.active_node_count = config.active_node_count.checked_add(1)
            .ok_or(StrmError::MathOverflow)?;
    }

    // Update total staked
    let config = &mut ctx.accounts.config;
    config.total_staked = config.total_staked.checked_add(amount)
        .ok_or(StrmError::MathOverflow)?;

    msg!("Staked {} STRM tokens for node {}", amount, ctx.accounts.node.key());
    msg!("Specialization: {:?}", specialization);
    msg!("Total staked: {}", new_total);

    Ok(())
}

/// Add more stake to existing node
pub fn add_stake(ctx: Context<AddStake>, amount: u64) -> Result<()> {
    let config = &ctx.accounts.config;
    let stake_account = &mut ctx.accounts.stake_account;
    let clock = Clock::get()?;

    // Check program is not paused
    require!(!config.is_paused, StrmError::Unauthorized);

    // Must be an active staker
    require!(stake_account.is_active, StrmError::NotStaked);

    // Cannot add stake while unstaking
    require!(!stake_account.is_unstaking(), StrmError::UnstakingInProgress);

    // Transfer tokens
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.stake_vault.to_account_info(),
        authority: ctx.accounts.node.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Update stake account
    stake_account.staked_amount = stake_account.staked_amount.checked_add(amount)
        .ok_or(StrmError::MathOverflow)?;
    stake_account.last_activity_at = clock.unix_timestamp;

    // Update global stats
    let config = &mut ctx.accounts.config;
    config.total_staked = config.total_staked.checked_add(amount)
        .ok_or(StrmError::MathOverflow)?;

    msg!("Added {} STRM to stake. New total: {}", amount, stake_account.staked_amount);

    Ok(())
}

#[derive(Accounts)]
pub struct StakeTokens<'info> {
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

    /// Node's stake account PDA
    #[account(
        init_if_needed,
        payer = node,
        space = NodeStake::LEN,
        seeds = [STAKE_SEED, node.key().as_ref()],
        bump
    )]
    pub stake_account: Account<'info, NodeStake>,

    /// User's STRM token account
    #[account(
        mut,
        constraint = user_token_account.mint == config.strm_mint @ StrmError::InvalidMint,
        constraint = user_token_account.owner == node.key() @ StrmError::InvalidOwner
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Stake vault (PDA-owned token account)
    #[account(
        mut,
        constraint = stake_vault.mint == config.strm_mint @ StrmError::InvalidMint
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddStake<'info> {
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

    /// Node's stake account PDA
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
