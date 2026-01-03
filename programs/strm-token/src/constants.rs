//! Program constants for StreamSync STRM Token

/// Seed for the program config PDA
pub const CONFIG_SEED: &[u8] = b"config";

/// Seed for node stake accounts
pub const STAKE_SEED: &[u8] = b"stake";

/// Seed for settlement batch accounts
pub const SETTLEMENT_SEED: &[u8] = b"settlement";

/// Seed for reward vault
pub const REWARD_VAULT_SEED: &[u8] = b"reward_vault";

/// Minimum stake required to operate a node (1000 STRM with 9 decimals)
pub const MIN_STAKE_AMOUNT: u64 = 1_000_000_000_000;

/// Unstaking cooldown period in seconds (7 days)
pub const UNSTAKE_COOLDOWN_SECONDS: i64 = 7 * 24 * 60 * 60;

/// Maximum number of rewards per settlement batch
pub const MAX_REWARDS_PER_BATCH: usize = 100;

/// Settlement batch interval in seconds (5 minutes)
pub const SETTLEMENT_INTERVAL_SECONDS: i64 = 5 * 60;

/// Winner reward percentage (70%)
pub const WINNER_REWARD_BPS: u16 = 7000;

/// Verifier reward percentage (15% each, 2 verifiers = 30%)
pub const VERIFIER_REWARD_BPS: u16 = 1500;

/// Protocol fee percentage (0% initially, can be set by admin)
pub const PROTOCOL_FEE_BPS: u16 = 0;

/// Basis points denominator
pub const BPS_DENOMINATOR: u16 = 10000;

/// Maximum slash percentage (50% of stake)
pub const MAX_SLASH_BPS: u16 = 5000;

/// Token decimals for STRM
pub const STRM_DECIMALS: u8 = 9;
