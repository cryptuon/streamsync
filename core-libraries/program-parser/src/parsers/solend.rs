//! Solend parser
use crate::{error::ParseError, types::*};
use solana_sdk::pubkey::Pubkey;

pub struct SolendParser;
impl SolendParser {
    pub fn new() -> Self { Self }
    pub async fn parse(&self, _: &Pubkey, _: &[u8], _: &[Pubkey]) -> Result<crate::types::ParseResult, ParseError> {
        Ok(crate::types::ParseResult::Solend(SolendData {
            operation_type: SolendOperation::DepositReserveLiquidity,
            lending_market: Pubkey::new_unique(),
            reserve: Pubkey::new_unique(),
            collateral_mint: Pubkey::new_unique(),
            liquidity_mint: Pubkey::new_unique(),
            amount: 1000000,
            user_wallet: Pubkey::new_unique(),
            health_factor: Some(1.5),
            ltv: Some(0.75),
            liquidation_threshold: Some(0.85),
        }))
    }
}