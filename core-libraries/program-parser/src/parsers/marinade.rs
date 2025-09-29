//! Marinade parser
use crate::{error::ParseError, types::*};
use solana_sdk::pubkey::Pubkey;

pub struct MarinadeParser;
impl MarinadeParser {
    pub fn new() -> Self { Self }
    pub async fn parse(&self, _: &Pubkey, _: &[u8], _: &[Pubkey]) -> Result<crate::types::ParseResult, ParseError> {
        Ok(crate::types::ParseResult::Marinade(MarinadeData {
            operation_type: MarinadeOperation::Deposit,
            marinade_state: Pubkey::new_unique(),
            msol_mint: Pubkey::new_unique(),
            sol_amount: Some(1000000000),
            msol_amount: Some(950000000),
            exchange_rate: 0.95,
            user_wallet: Pubkey::new_unique(),
            validator_list: None,
        }))
    }
}