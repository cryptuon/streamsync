//! Core types for program parsing

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Result of parsing program data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParseResult {
    /// SPL Token program operations
    SplToken(SplTokenData),
    /// Metaplex NFT operations
    Metaplex(MetaplexData),
    /// Jupiter aggregator swaps
    Jupiter(JupiterData),
    /// Raydium AMM operations
    Raydium(RaydiumData),
    /// Orca concentrated liquidity
    Orca(OrcaData),
    /// Serum DEX operations
    Serum(SerumData),
    /// Solend lending operations
    Solend(SolendData),
    /// Marinade liquid staking
    Marinade(MarinadeData),
    /// Anchor framework programs
    Anchor(AnchorData),
    /// Unknown program (with raw data)
    Unknown(UnknownData),
}

/// SPL Token program data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplTokenData {
    pub operation_type: SplTokenOperation,
    pub mint: Pubkey,
    pub amount: u64,
    pub decimals: u8,
    pub from: Option<Pubkey>,
    pub to: Option<Pubkey>,
    pub authority: Option<Pubkey>,
    pub multisig_signers: Vec<Pubkey>,
    pub parsed_amount: Decimal,
    pub symbol: Option<String>,
    pub metadata: SplTokenMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SplTokenOperation {
    Transfer,
    Mint,
    Burn,
    Approve,
    Revoke,
    CreateAccount,
    CloseAccount,
    InitializeMint,
    SetAuthority,
    MintTo,
    BurnChecked,
    TransferChecked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplTokenMetadata {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub logo_uri: Option<String>,
    pub total_supply: Option<u64>,
    pub is_initialized: bool,
    pub freeze_authority: Option<Pubkey>,
    pub mint_authority: Option<Pubkey>,
}

/// Metaplex NFT/cNFT data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaplexData {
    pub operation_type: MetaplexOperation,
    pub mint: Pubkey,
    pub metadata_account: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creators: Vec<Creator>,
    pub seller_fee_basis_points: u16,
    pub collection: Option<Collection>,
    pub uses: Option<Uses>,
    pub is_compressed: bool,
    pub tree_authority: Option<Pubkey>,
    pub leaf_id: Option<u32>,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetaplexOperation {
    CreateMetadata,
    UpdateMetadata,
    MintNft,
    TransferNft,
    BurnNft,
    VerifyCreator,
    UnverifyCreator,
    VerifyCollection,
    UnverifyCollection,
    // Compressed NFT operations
    MintToCollectionV1,
    TransferV1,
    BurnV1,
    DecompressV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    pub address: Pubkey,
    pub verified: bool,
    pub share: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub verified: bool,
    pub key: Pubkey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uses {
    pub use_method: UseMethod,
    pub remaining: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UseMethod {
    Burn,
    Multiple,
    Single,
}

/// Jupiter aggregator data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterData {
    pub operation_type: JupiterOperation,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub input_amount: u64,
    pub output_amount: u64,
    pub minimum_output_amount: u64,
    pub slippage_bps: u16,
    pub route: Vec<RouteStep>,
    pub fees: Vec<FeeInfo>,
    pub price_impact_pct: Option<f64>,
    pub user_wallet: Pubkey,
    pub referrer: Option<Pubkey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JupiterOperation {
    Swap,
    ExactInSwap,
    ExactOutSwap,
    SetTokenLedger,
    Route,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStep {
    pub program_id: Pubkey,
    pub program_name: String,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub input_amount: u64,
    pub output_amount: u64,
    pub fee_amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeInfo {
    pub fee_type: FeeType,
    pub amount: u64,
    pub mint: Pubkey,
    pub recipient: Pubkey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeeType {
    Platform,
    Referrer,
    Protocol,
    Router,
}

/// Raydium AMM data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumData {
    pub operation_type: RaydiumOperation,
    pub amm_id: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub pool_token_amount: Option<u64>,
    pub user_wallet: Pubkey,
    pub liquidity_info: LiquidityInfo,
    pub fees: RaydiumFees,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaydiumOperation {
    Swap,
    AddLiquidity,
    RemoveLiquidity,
    CreatePool,
    Harvest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityInfo {
    pub total_supply: u64,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub price_a_to_b: f64,
    pub price_b_to_a: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumFees {
    pub trade_fee_rate: u64,
    pub protocol_fee_rate: u64,
    pub fund_fee_rate: u64,
}

/// Orca concentrated liquidity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaData {
    pub operation_type: OrcaOperation,
    pub whirlpool: Pubkey,
    pub position: Option<Pubkey>,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub tick_current_index: i32,
    pub tick_lower_index: Option<i32>,
    pub tick_upper_index: Option<i32>,
    pub liquidity: u128,
    pub fee_tier: u16,
    pub rewards: Vec<RewardInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrcaOperation {
    Swap,
    OpenPosition,
    ClosePosition,
    IncreaseLiquidity,
    DecreaseLiquidity,
    CollectFees,
    CollectReward,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardInfo {
    pub mint: Pubkey,
    pub amount: u64,
    pub decimals: u8,
}

/// Serum DEX data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerumData {
    pub operation_type: SerumOperation,
    pub market: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub side: OrderSide,
    pub price: u64,
    pub quantity: u64,
    pub order_id: Option<u128>,
    pub client_order_id: Option<u64>,
    pub fees: SerumFees,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerumOperation {
    PlaceOrder,
    CancelOrder,
    MatchOrders,
    ConsumeEvents,
    SettleFunds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerumFees {
    pub base_fee: u64,
    pub quote_fee: u64,
    pub referrer_rebate: Option<u64>,
}

/// Solend lending data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolendData {
    pub operation_type: SolendOperation,
    pub lending_market: Pubkey,
    pub reserve: Pubkey,
    pub collateral_mint: Pubkey,
    pub liquidity_mint: Pubkey,
    pub amount: u64,
    pub user_wallet: Pubkey,
    pub health_factor: Option<f64>,
    pub ltv: Option<f64>,
    pub liquidation_threshold: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolendOperation {
    DepositReserveLiquidity,
    WithdrawReserveLiquidity,
    BorrowReserveLiquidity,
    RepayReserveLiquidity,
    LiquidateObligation,
    RefreshObligation,
}

/// Marinade liquid staking data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarinadeData {
    pub operation_type: MarinadeOperation,
    pub marinade_state: Pubkey,
    pub msol_mint: Pubkey,
    pub sol_amount: Option<u64>,
    pub msol_amount: Option<u64>,
    pub exchange_rate: f64,
    pub user_wallet: Pubkey,
    pub validator_list: Option<Vec<Pubkey>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarinadeOperation {
    Deposit,
    DepositStakeAccount,
    LiquidUnstake,
    DelayedUnstake,
    Claim,
    UpdateState,
}

/// Anchor framework data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorData {
    pub program_id: Pubkey,
    pub instruction_name: String,
    pub instruction_discriminator: Vec<u8>,
    pub accounts: Vec<AnchorAccount>,
    pub instruction_data: HashMap<String, serde_json::Value>,
    pub idl_version: Option<String>,
    pub program_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorAccount {
    pub name: String,
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
    pub is_optional: bool,
}

/// Unknown program data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownData {
    pub program_id: Pubkey,
    pub instruction_data: Vec<u8>,
    pub accounts: Vec<Pubkey>,
    pub suspected_program_type: Option<String>,
    pub confidence: f64,
    pub raw_bytes: Vec<u8>,
    pub parsing_hints: Vec<String>,
}

/// Parsing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseConfig {
    pub enable_metadata_lookup: bool,
    pub enable_price_lookup: bool,
    pub cache_results: bool,
    pub max_cache_size: usize,
    pub cache_ttl_seconds: u64,
    pub parallel_parsing: bool,
    pub max_retries: u32,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            enable_metadata_lookup: true,
            enable_price_lookup: false,
            cache_results: true,
            max_cache_size: 10000,
            cache_ttl_seconds: 300,
            parallel_parsing: true,
            max_retries: 3,
        }
    }
}

/// Program detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub program_type: ProgramType,
    pub confidence: f64,
    pub detection_method: DetectionMethod,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProgramType {
    SplToken,
    Metaplex,
    Jupiter,
    Raydium,
    Orca,
    Serum,
    Solend,
    Marinade,
    Anchor,
    System,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    ProgramId,
    InstructionPattern,
    AccountStructure,
    DataSignature,
    Heuristic,
}

/// Parsing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseStats {
    pub total_parsed: u64,
    pub successful_parses: u64,
    pub failed_parses: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub average_parse_time_ms: f64,
    pub programs_detected: HashMap<ProgramType, u64>,
    pub last_updated: DateTime<Utc>,
}