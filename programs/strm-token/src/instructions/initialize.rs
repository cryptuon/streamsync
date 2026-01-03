//! Initialize program configuration

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

use crate::constants::*;
use crate::errors::StrmError;
use crate::state::ProgramConfig;

/// Initialize the program configuration
/// Can only be called once by the deployer
pub fn initialize_config(
    ctx: Context<InitializeConfig>,
    oracle_authority: Pubkey,
) -> Result<()> {
    let config = &mut ctx.accounts.config;

    // Initialize config with default values
    config.admin = ctx.accounts.admin.key();
    config.strm_mint = ctx.accounts.strm_mint.key();
    config.min_stake_amount = MIN_STAKE_AMOUNT;
    config.unstake_cooldown_seconds = UNSTAKE_COOLDOWN_SECONDS;
    config.winner_reward_bps = WINNER_REWARD_BPS;
    config.verifier_reward_bps = VERIFIER_REWARD_BPS;
    config.protocol_fee_bps = PROTOCOL_FEE_BPS;
    config.settlement_interval_seconds = SETTLEMENT_INTERVAL_SECONDS;
    config.total_staked = 0;
    config.total_rewards_distributed = 0;
    config.active_node_count = 0;
    config.oracle_authority = oracle_authority;
    config.is_paused = false;
    config.bump = ctx.bumps.config;

    msg!("StreamSync STRM Token Program initialized");
    msg!("Admin: {}", config.admin);
    msg!("STRM Mint: {}", config.strm_mint);
    msg!("Oracle: {}", config.oracle_authority);

    Ok(())
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
    let config = &mut ctx.accounts.config;

    if let Some(min_stake) = new_min_stake {
        config.min_stake_amount = min_stake;
    }

    if let Some(cooldown) = new_cooldown_seconds {
        config.unstake_cooldown_seconds = cooldown;
    }

    if let Some(winner_bps) = new_winner_bps {
        config.winner_reward_bps = winner_bps;
    }

    if let Some(verifier_bps) = new_verifier_bps {
        config.verifier_reward_bps = verifier_bps;
    }

    if let Some(fee_bps) = new_protocol_fee_bps {
        config.protocol_fee_bps = fee_bps;
    }

    // Validate reward percentages after update
    require!(
        config.validate_reward_percentages(),
        StrmError::InvalidRewardDistribution
    );

    if let Some(oracle) = new_oracle {
        config.oracle_authority = oracle;
    }

    if let Some(paused) = pause {
        config.is_paused = paused;
    }

    msg!("Program configuration updated");
    Ok(())
}

/// Transfer admin authority
pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let old_admin = config.admin;
    config.admin = new_admin;

    msg!("Admin transferred from {} to {}", old_admin, new_admin);
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    /// Admin who initializes the program (must be deployer)
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Program configuration PDA
    #[account(
        init,
        payer = admin,
        space = ProgramConfig::LEN,
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, ProgramConfig>,

    /// STRM token mint
    pub strm_mint: Account<'info, Mint>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    /// Admin authority
    #[account(
        constraint = admin.key() == config.admin @ StrmError::Unauthorized
    )]
    pub admin: Signer<'info>,

    /// Program configuration PDA
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, ProgramConfig>,
}

#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    /// Current admin authority
    #[account(
        constraint = admin.key() == config.admin @ StrmError::Unauthorized
    )]
    pub admin: Signer<'info>,

    /// Program configuration PDA
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump
    )]
    pub config: Account<'info, ProgramConfig>,
}
