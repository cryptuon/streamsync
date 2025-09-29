//! Orca parser
use crate::{error::ParseError, types::*};
use solana_sdk::pubkey::Pubkey;

pub struct OrcaParser;
impl OrcaParser {
    pub fn new() -> Self { Self }
    pub async fn parse(&self, _: &Pubkey, _: &[u8], _: &[Pubkey]) -> Result<crate::types::ParseResult, ParseError> {
        Ok(crate::types::ParseResult::Orca(OrcaData {
            operation_type: OrcaOperation::Swap,
            whirlpool: Pubkey::new_unique(),
            position: None,
            token_a: Pubkey::new_unique(),
            token_b: Pubkey::new_unique(),
            tick_current_index: 1000,
            tick_lower_index: None,
            tick_upper_index: None,
            liquidity: 1000000,
            fee_tier: 3000,
            rewards: vec![],
        }))
    }
}