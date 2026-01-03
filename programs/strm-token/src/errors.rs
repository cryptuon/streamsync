//! Custom error codes for StreamSync STRM Token Program

use anchor_lang::prelude::*;

#[error_code]
pub enum StrmError {
    #[msg("Program config already initialized")]
    AlreadyInitialized,

    #[msg("Unauthorized: only admin can perform this action")]
    Unauthorized,

    #[msg("Stake amount below minimum required")]
    StakeBelowMinimum,

    #[msg("Insufficient stake balance")]
    InsufficientStake,

    #[msg("Node is not currently staked")]
    NotStaked,

    #[msg("Unstaking already in progress")]
    UnstakingInProgress,

    #[msg("Unstaking cooldown not yet complete")]
    CooldownNotComplete,

    #[msg("No unstaking in progress")]
    NotUnstaking,

    #[msg("No rewards available to claim")]
    NoRewardsAvailable,

    #[msg("Settlement batch is full")]
    BatchFull,

    #[msg("Settlement batch is empty")]
    BatchEmpty,

    #[msg("Invalid reward distribution percentages")]
    InvalidRewardDistribution,

    #[msg("Slash amount exceeds maximum allowed")]
    SlashExceedsMaximum,

    #[msg("Invalid node public key")]
    InvalidNodePubkey,

    #[msg("Arithmetic overflow")]
    MathOverflow,

    #[msg("Settlement interval not yet elapsed")]
    SettlementIntervalNotElapsed,

    #[msg("Oracle signature verification failed")]
    OracleVerificationFailed,

    #[msg("Query ID already processed")]
    QueryAlreadyProcessed,

    #[msg("Invalid token mint")]
    InvalidMint,

    #[msg("Account not owned by program")]
    InvalidOwner,
}
